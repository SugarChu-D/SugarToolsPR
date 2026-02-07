use infra::gpu::bind_group_builder::BindGroupBuilder;
use infra::gpu::buffer_pool::{BufferKind, BufferPool};
use infra::gpu::encoder_utils::EncoderUtils;
use infra::gpu::pipeline_factory::PipelineFactory;
use infra::gpu::readback::Readback;
use infra::gpu::shader_loader::ShaderLoader;
use wgpu::util::DeviceExt;

use crate::gpu::bind_layout::{input_output_layout, input_output_counter_params_layout, input_list_output_counter_params_layout};
use crate::gpu::input_layout::{GpuInput, GpuIvConfig, GPUInputIterator};
use crate::gpu::staging_layout::GpuCandidate;
use crate::gpu::local_gpu_config::GpuKernelConfig;
use crate::models::game_date_iterator::GameDateSpec;
use crate::models::{DSConfig, KeyPresses};
use crate::gpu::mt_kernel;

#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
struct DispatchParams {
    base_index: u64,
    total_len: u64,
}

pub const MAX_RESULTS: usize = 1 << 20;

#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
struct ListDispatchParams {
    base_index: u64,
    total_len: u64,
    list_len: u32,
    keypress_len: u32,
}

pub async fn run_sha1(
    ctx: &infra::gpu::context::GpuContext,
    input: &[GpuInput],
) -> Result<Vec<GpuCandidate>, wgpu::BufferAsyncError> {

    if input.is_empty() {
        return Ok(Vec::new());
    }

    let first = &input[0];
    if first.hour_range[1] < first.hour_range[0]
        || first.minute_range[1] < first.minute_range[0]
        || first.second_range[1] < first.second_range[0] {
        return Ok(Vec::new());
    }

    #[cfg(debug_assertions)]
    {
        for other in input.iter().skip(1) {
            assert_eq!(other.hour_range, first.hour_range, "hour_range must match across inputs");
            assert_eq!(other.minute_range, first.minute_range, "minute_range must match across inputs");
            assert_eq!(other.second_range, first.second_range, "second_range must match across inputs");
        }
    }

    let kp_count = 0x1000usize;
    let h_count = (first.hour_range[1] - first.hour_range[0] + 1) as usize;
    let m_count = (first.minute_range[1] - first.minute_range[0] + 1) as usize;
    let s_count = (first.second_range[1] - first.second_range[0] + 1) as usize;
    let time_count = h_count * m_count * s_count;
    let output_len_u64 = (kp_count as u64)
        * (time_count as u64)
        * (input.len() as u64);
    if output_len_u64 == 0 {
        return Ok(Vec::new());
    }
    if output_len_u64 > (usize::MAX as u64) {
        return Ok(Vec::new());
    }
    let output_len = output_len_u64 as usize;

    let shader = ShaderLoader::from_wgsl(
        &ctx.device,
        Some("rng_core_sha1_expand"),
        include_str!("wgsl/sha-1.wgsl"),
    );

    let pool = BufferPool::new(&ctx.device);
    let input_buffer = pool
        .create_init(input, BufferKind::Input, "rng_core_sha1_expand_input_buffer")
        .buffer;

    let layout = input_output_layout(&ctx.device);
    let pipeline = PipelineFactory::new(&ctx.device)
        .create_compute(&shader, &layout, "main");

    let readback = Readback::new(ctx);
    let limits = ctx.device.limits();
    let max_bytes = limits
        .max_buffer_size
        .min(limits.max_storage_buffer_binding_size as u64);
    let elem_size = std::mem::size_of::<GpuCandidate>() as u64;
    let mut max_elems = (max_bytes / elem_size) as usize;
    max_elems = max_elems.min(u32::MAX as usize);
    if max_elems == 0 {
        return Ok(Vec::new());
    }

    let mut results = Vec::with_capacity(output_len);
    let mut base = 0usize;
    while base < output_len {
        let chunk_len = (output_len - base).min(max_elems);
        let output_bytes = (chunk_len * std::mem::size_of::<GpuCandidate>()) as u64;
        let output_buffer = pool
            .create::<GpuCandidate>(output_bytes as usize, BufferKind::Output, "rng_core_sha1_expand_output_buffer")
            .buffer;

        let params = DispatchParams {
            base_index: base as u64,
            total_len: output_len_u64,
        };
        let params_buffer = pool
            .create_init(std::slice::from_ref(&params), BufferKind::Input, "rng_core_sha1_expand_params_buffer")
            .buffer;

        let bind_group = BindGroupBuilder::new(&layout)
            .buffer(0, &input_buffer)
            .buffer(1, &output_buffer)
            .buffer(2, &params_buffer)
            .build(&ctx.device, Some("rng_core_sha1_expand_bind_group"));

        let mut encoder = ctx.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("rng_core_sha1_expand_encoder"),
        });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("rng_core_sha1_expand_pass"),
                timestamp_writes: None,
            });
            EncoderUtils::dispatch_1d(
                &mut pass,
                &pipeline,
                &bind_group,
                chunk_len as u32,
                GpuKernelConfig::SHA1_MT.workgroup_size,
            );
        }
        ctx.queue.submit(Some(encoder.finish()));

        let mut chunk = readback
            .read_buffer_with_pool::<GpuCandidate>(
                &pool,
                &output_buffer,
                chunk_len,
                Some("rng_core_sha1_expand_readback"),
            )
            .await?;
        results.append(&mut chunk);
        base += chunk_len;
    }

    Ok(results)
}

pub async fn run_sha1_mt(
    ctx: &infra::gpu::context::GpuContext,
    input: &[GpuInput],
) -> Result<Vec<GpuCandidate>, wgpu::BufferAsyncError> {
    if input.is_empty() {
        return Ok(Vec::new());
    }

    let first = &input[0];
    if first.hour_range[1] < first.hour_range[0]
        || first.minute_range[1] < first.minute_range[0]
        || first.second_range[1] < first.second_range[0] {
        return Ok(Vec::new());
    }

    #[cfg(debug_assertions)]
    {
        for other in input.iter().skip(1) {
            assert_eq!(other.hour_range, first.hour_range, "hour_range must match across inputs");
            assert_eq!(other.minute_range, first.minute_range, "minute_range must match across inputs");
            assert_eq!(other.second_range, first.second_range, "second_range must match across inputs");
        }
    }

    #[cfg(debug_assertions)]
    {
        for other in input.iter().skip(1) {
            assert_eq!(other.iv_step, first.iv_step, "iv_step must match across inputs");
            assert_eq!(other.iv_min, first.iv_min, "iv_min must match across inputs");
            assert_eq!(other.iv_max, first.iv_max, "iv_max must match across inputs");
        }
    }

    let kp_count = 0x1000usize;
    let h_count = (first.hour_range[1] - first.hour_range[0] + 1) as usize;
    let m_count = (first.minute_range[1] - first.minute_range[0] + 1) as usize;
    let s_count = (first.second_range[1] - first.second_range[0] + 1) as usize;
    let time_count = h_count * m_count * s_count;
    let output_len_u64 = (kp_count as u64)
        * (time_count as u64)
        * (input.len() as u64);
    if output_len_u64 == 0 {
        return Ok(Vec::new());
    }
    if output_len_u64 > (usize::MAX as u64) {
        return Ok(Vec::new());
    }
    let output_len = output_len_u64 as usize;

    let shader = ShaderLoader::from_wgsl(
        &ctx.device,
        Some("rng_core_sha1_expand"),
        include_str!("wgsl/sha-1.wgsl"),
    );

    let pool = BufferPool::new(&ctx.device);
    let input_buffer = pool
        .create_init(input, BufferKind::Input, "rng_core_sha1_expand_input_buffer")
        .buffer;

    let layout = input_output_layout(&ctx.device);
    let pipeline = PipelineFactory::new(&ctx.device)
        .create_compute(&shader, &layout, "main");

    let readback = Readback::new(ctx);
    let limits = ctx.device.limits();
    let max_bytes = limits
        .max_buffer_size
        .min(limits.max_storage_buffer_binding_size as u64);
    let elem_size = std::mem::size_of::<GpuCandidate>() as u64;
    let mut max_elems = (max_bytes / elem_size) as usize;
    max_elems = max_elems.min(u32::MAX as usize);
    if max_elems == 0 {
        return Ok(Vec::new());
    }

    let mut results = Vec::new();
    let mut base = 0usize;
    while base < output_len {
        let chunk_len = (output_len - base).min(max_elems);
        let output_bytes = (chunk_len * std::mem::size_of::<GpuCandidate>()) as u64;
        let output_buffer = pool
            .create::<GpuCandidate>(output_bytes as usize, BufferKind::Output, "rng_core_sha1_expand_output_buffer")
            .buffer;

        let params = DispatchParams {
            base_index: base as u64,
            total_len: output_len_u64,
        };
        let params_buffer = pool
            .create_init(std::slice::from_ref(&params), BufferKind::Input, "rng_core_sha1_expand_params_buffer")
            .buffer;

        let bind_group = BindGroupBuilder::new(&layout)
            .buffer(0, &input_buffer)
            .buffer(1, &output_buffer)
            .buffer(2, &params_buffer)
            .build(&ctx.device, Some("rng_core_sha1_expand_bind_group"));

        let mut encoder = ctx.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("rng_core_sha1_expand_encoder"),
        });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("rng_core_sha1_expand_pass"),
                timestamp_writes: None,
            });
            EncoderUtils::dispatch_1d(
                &mut pass,
                &pipeline,
                &bind_group,
                chunk_len as u32,
                GpuKernelConfig::SHA1_MT.workgroup_size,
            );
        }
        ctx.queue.submit(Some(encoder.finish()));

        let sha1_chunk = readback
            .read_buffer_with_pool::<GpuCandidate>(
                &pool,
                &output_buffer,
                chunk_len,
                Some("rng_core_sha1_expand_readback"),
            )
            .await?;

        // MT uses only IV config, which is expected to be the same across inputs.
        let mut mt_chunk = crate::gpu::mt_kernel::run_mt_compact(ctx, &sha1_chunk, &input[0..1]).await?;
        results.append(&mut mt_chunk);
        if results.len() >= crate::gpu::mt_kernel::MAX_RESULTS {
            results.truncate(crate::gpu::mt_kernel::MAX_RESULTS);
            break;
        }
        base += chunk_len;
    }

    Ok(results)
}

pub async fn run_sha1_mt_compact(
    ctx: &infra::gpu::context::GpuContext,
    input: &[GpuInput],
) -> Result<Vec<GpuCandidate>, wgpu::BufferAsyncError> {
    if input.is_empty() {
        return Ok(Vec::new());
    }

    let first = &input[0];
    if first.hour_range[1] < first.hour_range[0]
        || first.minute_range[1] < first.minute_range[0]
        || first.second_range[1] < first.second_range[0] {
        return Ok(Vec::new());
    }

    #[cfg(debug_assertions)]
    {
        for other in input.iter().skip(1) {
            assert_eq!(other.hour_range, first.hour_range, "hour_range must match across inputs");
            assert_eq!(other.minute_range, first.minute_range, "minute_range must match across inputs");
            assert_eq!(other.second_range, first.second_range, "second_range must match across inputs");
            assert_eq!(other.iv_step, first.iv_step, "iv_step must match across inputs");
            assert_eq!(other.iv_min, first.iv_min, "iv_min must match across inputs");
            assert_eq!(other.iv_max, first.iv_max, "iv_max must match across inputs");
        }
    }

    let kp_count = 0x1000usize;
    let h_count = (first.hour_range[1] - first.hour_range[0] + 1) as usize;
    let m_count = (first.minute_range[1] - first.minute_range[0] + 1) as usize;
    let s_count = (first.second_range[1] - first.second_range[0] + 1) as usize;
    let time_count = h_count * m_count * s_count;
    let output_len_u64 = (kp_count as u64)
        * (time_count as u64)
        * (input.len() as u64);
    if output_len_u64 == 0 {
        return Ok(Vec::new());
    }

    let shader = ShaderLoader::from_wgsl(
        &ctx.device,
        Some("rng_core_sha1_mt_compact"),
        include_str!("wgsl/sha-1_mt_compact.wgsl"),
    );

    let pool = BufferPool::new(&ctx.device);
    let input_buffer = pool
        .create_init(input, BufferKind::Input, "rng_core_sha1_mt_compact_input_buffer")
        .buffer;

    let layout = input_output_counter_params_layout(&ctx.device);
    let pipeline = PipelineFactory::new(&ctx.device)
        .create_compute(&shader, &layout, "main");

    let readback = Readback::new(ctx);

    let mut results = Vec::new();
    let mut base: u64 = 0;
    let wg = GpuKernelConfig::SHA1_MT.workgroup_size as u64;
    let max_groups = ctx
        .device
        .limits()
        .max_compute_workgroups_per_dimension
        .min(65535) as u64;
    let max_dispatch = max_groups.saturating_mul(wg);
    while base < output_len_u64 {
        let remaining = output_len_u64 - base;
        let chunk_len = remaining.min(max_dispatch) as u32;

        let output_bytes = (MAX_RESULTS * std::mem::size_of::<GpuCandidate>()) as u64;
        let output_buffer = pool
            .create::<GpuCandidate>(output_bytes as usize, BufferKind::Output, "rng_core_sha1_mt_compact_output_buffer")
            .buffer;

        let counter_buffer = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("rng_core_sha1_mt_compact_counter_buffer"),
            contents: bytemuck::bytes_of(&0u32),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        });

        let params = DispatchParams {
            base_index: base,
            total_len: output_len_u64,
        };
        let params_buffer = pool
            .create_init(std::slice::from_ref(&params), BufferKind::Input, "rng_core_sha1_mt_compact_params_buffer")
            .buffer;

        let bind_group = BindGroupBuilder::new(&layout)
            .buffer(0, &input_buffer)
            .buffer(1, &output_buffer)
            .buffer(2, &counter_buffer)
            .buffer(3, &params_buffer)
            .build(&ctx.device, Some("rng_core_sha1_mt_compact_bind_group"));

        let mut encoder = ctx.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("rng_core_sha1_mt_compact_encoder"),
        });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("rng_core_sha1_mt_compact_pass"),
                timestamp_writes: None,
            });
            EncoderUtils::dispatch_1d(
                &mut pass,
                &pipeline,
                &bind_group,
                chunk_len,
                GpuKernelConfig::SHA1_MT.workgroup_size,
            );
        }
        ctx.queue.submit(Some(encoder.finish()));

        let count_vec = readback
            .read_buffer::<u32>(&counter_buffer, 1, Some("rng_core_sha1_mt_compact_count"))
            .await?;
        let count = count_vec[0] as usize;
        if count > 0 {
            let read_len = count.min(MAX_RESULTS);
            let mut chunk = readback
                .read_buffer_with_pool::<GpuCandidate>(
                    &pool,
                    &output_buffer,
                    read_len,
                    Some("rng_core_sha1_mt_compact_readback"),
                )
                .await?;
            results.append(&mut chunk);
            if results.len() >= MAX_RESULTS {
                results.truncate(MAX_RESULTS);
                break;
            }
        }

        base += chunk_len as u64;
    }

    Ok(results)
}

pub async fn run_sha1_seedhigh_filter(
    ctx: &infra::gpu::context::GpuContext,
    input: &[GpuInput],
    seed_high_list: &[u32],
) -> Result<Vec<GpuCandidate>, wgpu::BufferAsyncError> {
    let keypress_list: Vec<u32> = KeyPresses::iter_valid()
        .map(|k| k.raw() as u32)
        .collect();
    let keypress_count = keypress_list.len() as u32;
    if keypress_count == 0 {
        return Ok(Vec::new());
    }

    if input.is_empty() {
        return Ok(Vec::new());
    }
    if seed_high_list.is_empty() {
        return Ok(Vec::new());
    }

    let first = &input[0];
    if first.hour_range[1] < first.hour_range[0]
        || first.minute_range[1] < first.minute_range[0]
        || first.second_range[1] < first.second_range[0] {
        return Ok(Vec::new());
    }

    #[cfg(debug_assertions)]
    {
        for other in input.iter().skip(1) {
            assert_eq!(other.hour_range, first.hour_range, "hour_range must match across inputs");
            assert_eq!(other.minute_range, first.minute_range, "minute_range must match across inputs");
            assert_eq!(other.second_range, first.second_range, "second_range must match across inputs");
        }
    }

    let mut list = seed_high_list.to_vec();
    list.sort_unstable();
    list.dedup();
    if list.is_empty() {
        return Ok(Vec::new());
    }

    let kp_count = keypress_list.len();
    let h_count = (first.hour_range[1] - first.hour_range[0] + 1) as usize;
    let m_count = (first.minute_range[1] - first.minute_range[0] + 1) as usize;
    let s_count = (first.second_range[1] - first.second_range[0] + 1) as usize;
    let time_count = h_count * m_count * s_count;
    let output_len_u64 = (kp_count as u64)
        * (time_count as u64)
        * (input.len() as u64);
    if output_len_u64 == 0 {
        return Ok(Vec::new());
    }

    let shader = ShaderLoader::from_wgsl(
        &ctx.device,
        Some("rng_core_sha1_seedhigh_filter"),
        include_str!("wgsl/sha-1_seedhigh_filter.wgsl"),
    );

    let pool = BufferPool::new(&ctx.device);
    let input_buffer = pool
        .create_init(input, BufferKind::Input, "rng_core_sha1_seedhigh_input_buffer")
        .buffer;
    let list_buffer = pool
        .create_init(&list, BufferKind::Input, "rng_core_sha1_seedhigh_list_buffer")
        .buffer;
    let keypress_buffer = pool
        .create_init(&keypress_list, BufferKind::Input, "rng_core_sha1_seedhigh_keypress_buffer")
        .buffer;

    let layout = input_list_output_counter_params_layout(&ctx.device);
    let pipeline = PipelineFactory::new(&ctx.device)
        .create_compute(&shader, &layout, "main");

    let readback = Readback::new(ctx);

    let mut results = Vec::new();
    let mut base: u64 = 0;
    let wg = GpuKernelConfig::SHA1_MT.workgroup_size as u64;
    let max_groups = ctx
        .device
        .limits()
        .max_compute_workgroups_per_dimension
        .min(65535) as u64;
    let max_dispatch = max_groups.saturating_mul(wg);
    while base < output_len_u64 {
        let remaining = output_len_u64 - base;
        let chunk_len = remaining.min(max_dispatch) as u32;

        let output_bytes = (MAX_RESULTS * std::mem::size_of::<GpuCandidate>()) as u64;
        let output_buffer = pool
            .create::<GpuCandidate>(output_bytes as usize, BufferKind::Output, "rng_core_sha1_seedhigh_output_buffer")
            .buffer;

        let counter_buffer = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("rng_core_sha1_seedhigh_counter_buffer"),
            contents: bytemuck::bytes_of(&0u32),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        });

        let params = ListDispatchParams {
            base_index: base,
            total_len: output_len_u64,
            list_len: list.len() as u32,
            keypress_len: keypress_count,
        };
        let params_buffer = pool
            .create_init(std::slice::from_ref(&params), BufferKind::Input, "rng_core_sha1_seedhigh_params_buffer")
            .buffer;

        let bind_group = BindGroupBuilder::new(&layout)
            .buffer(0, &input_buffer)
            .buffer(1, &list_buffer)
            .buffer(2, &keypress_buffer)
            .buffer(3, &output_buffer)
            .buffer(4, &counter_buffer)
            .buffer(5, &params_buffer)
            .build(&ctx.device, Some("rng_core_sha1_seedhigh_bind_group"));

        let mut encoder = ctx.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("rng_core_sha1_seedhigh_encoder"),
        });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("rng_core_sha1_seedhigh_pass"),
                timestamp_writes: None,
            });
            EncoderUtils::dispatch_1d(
                &mut pass,
                &pipeline,
                &bind_group,
                chunk_len,
                GpuKernelConfig::SHA1_MT.workgroup_size,
            );
        }
        ctx.queue.submit(Some(encoder.finish()));

        let count_vec = readback
            .read_buffer::<u32>(&counter_buffer, 1, Some("rng_core_sha1_seedhigh_count"))
            .await?;
        let count = count_vec[0] as usize;
        if count > 0 {
            let read_len = count.min(MAX_RESULTS);
            let mut chunk = readback
                .read_buffer_with_pool::<GpuCandidate>(
                    &pool,
                    &output_buffer,
                    read_len,
                    Some("rng_core_sha1_seedhigh_readback"),
                )
                .await?;
            results.append(&mut chunk);
            if results.len() >= MAX_RESULTS {
                results.truncate(MAX_RESULTS);
                break;
            }
        }

        base += chunk_len as u64;
    }

    Ok(results)
}

pub async fn run_sha1_seedhigh_search(
    ctx: &infra::gpu::context::GpuContext,
    ds_config: DSConfig,
    datespec: GameDateSpec,
    hour_range: [u32; 2],
    minute_range: [u32; 2],
    second_range: [u32; 2],
    iv_step: u32,
    iv_min: [u32; 6],
    iv_max: [u32; 6],
    batch_days: usize,
) -> Result<Vec<GpuCandidate>, wgpu::BufferAsyncError> {
    let cfg = GpuIvConfig {
        iv_step,
        _pad0: 0,
        iv_min,
        iv_max,
    };

    let seed_start = std::time::Instant::now();
    let seed_highs = mt_kernel::run_mt_seedhigh_candidates_cached(ctx, &cfg).await?;
    println!("seed_high elapsed: {:?}", seed_start.elapsed());
    if seed_highs.is_empty() {
        return Ok(Vec::new());
    }

    let mut it = GPUInputIterator::new(
        ds_config,
        datespec,
        hour_range,
        minute_range,
        second_range,
        iv_step,
        iv_min,
        iv_max,
    );

    let mut results = Vec::new();
    let batch = batch_days.max(1);
    let sha_start = std::time::Instant::now();
    loop {
        let inputs = it.next_batch(batch);
        if inputs.is_empty() {
            break;
        }

        let mut chunk = run_sha1_seedhigh_filter(ctx, &inputs, &seed_highs).await?;
        results.append(&mut chunk);
        if results.len() >= MAX_RESULTS {
            results.truncate(MAX_RESULTS);
            break;
        }
    }

    println!("sha1 filter elapsed: {:?}", sha_start.elapsed());
    Ok(results)
}

#[cfg(all(test, not(ci)))]
mod tests {
    use super::*;
    use crate::models::{DSConfig, GameVersion};
    use crate::gpu::input_layout::GpuIvConfig;
    use crate::gpu::mt_kernel;
    use crate::models::field_range::FieldRange;
    use crate::models::game_date_iterator::GameDateSpec;
    use infra::gpu::context::GpuContext;

    #[test]
    #[ignore]
    fn test_sha1_mt_matches_cpu_single_case() {
        pollster::block_on(async {
            let ctx = GpuContext::new().await;
            let start = std::time::Instant::now();

            let ds_config = DSConfig::new(GameVersion::White2, 0x10F7, false, 0x0009bf6d93ce);
            
            let p = 2u32;
            let iv_min: [u32; 6] = [31u32, 31u32, 31u32, 8, 31u32, 31u32];
            let iv_max: [u32; 6] = [31u32, 31u32, 31u32, 8, 31u32, 31u32];
            
            let inputs = [GpuInput {
                nazo: [
                ds_config.get_version_config().nazo_values.nazo1,
                ds_config.get_version_config().nazo_values.nazo2,
                ds_config.get_version_config().nazo_values.nazo3,
                ds_config.get_version_config().nazo_values.nazo4,
                ds_config.get_version_config().nazo_values.nazo5,
            ],
                vcount_timer0_as_data5: ((ds_config.get_version_config().vcount.0 as u32) << 16) | (ds_config.Timer0 as u32),
                mac: ds_config.MAC,
                gxframe_xor_frame: 0x600_0008,
                date_as_data8: 0x33082706,
                hour_range:[0,23],
                minute_range: [0,59],
                second_range: [0,59],
                _pad0: 0,
                iv_step: 2,
                iv_min,
                iv_max,
            }];

            let iv_cfg = GpuIvConfig {
                iv_step: p,
                _pad0: 0,
                iv_min,
                iv_max,
            };

            let seed_start = std::time::Instant::now();
            let seed_highs = mt_kernel::run_mt_seedhigh_candidates(&ctx, &iv_cfg)
                .await
                .expect("run_mt_seedhigh_candidates failed");
            let seed_elapsed = seed_start.elapsed();

            let sha_start = std::time::Instant::now();
            let results = run_sha1_seedhigh_filter(&ctx, &inputs, &seed_highs)
                .await
                .expect("run_sha1_seedhigh_filter failed");
            let sha_elapsed = sha_start.elapsed();

            println!("seed_high candidates: {}", seed_highs.len());
            println!("final candidates: {}", results.len());
            println!("seed_high elapsed: {:?}", seed_elapsed);
            println!("sha1 filter elapsed: {:?}", sha_elapsed);
            println!("total elapsed: {:?}", start.elapsed());
        });
    }

    #[test]
    #[ignore]
    fn test_sha1_seedhigh_search_smoke() {
        pollster::block_on(async {
            let ctx = GpuContext::new().await;
            let ds_config = DSConfig::new(GameVersion::White2, 0x10f7, false, 0x0009bf6d93ce);
            let datespec = GameDateSpec {
                year: FieldRange { min: 33, max:  33 },
                month: FieldRange { min: 8, max: 8 },
                day: FieldRange { min: 27, max: 27 },
            };

            let results = run_sha1_seedhigh_search(
                &ctx,
                ds_config,
                datespec,
                [1, 1],
                [41, 41],
                [5, 5],
                2,
                [31u32, 31u32, 31u32, 8, 31u32, 31u32],
                [31u32, 31u32, 31u32, 8, 31u32, 31u32],
                64,
            )
            .await
            .expect("run_sha1_seedhigh_search failed");

            println!("seedhigh search results: {}", results.len());
        });
    }
}
