use infra::gpu::bind_group_builder::BindGroupBuilder;
use bytemuck;
use infra::gpu::buffer_pool::{BufferKind, BufferPool};
use infra::gpu::encoder_utils::EncoderUtils;
use infra::gpu::pipeline_factory::PipelineFactory;
use infra::gpu::readback::Readback;
use infra::gpu::shader_loader::ShaderLoader;
use wgpu::util::DeviceExt;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::gpu::bind_layout::{candidate_config_output_layout, candidate_config_output_counter_layout, config_output_counter_params_layout};
use crate::gpu::input_layout::{GpuInput, GpuIvConfig};
use crate::gpu::local_gpu_config::GpuKernelConfig;
use crate::gpu::staging_layout::GpuCandidate;

pub const MAX_RESULTS: usize = 1 << 20;

#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
struct DispatchParams {
    base_index: u64,
    total_len: u64,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct IvKey {
    iv_step: u32,
    iv_min: [u32; 6],
    iv_max: [u32; 6],
}

static SEED_HIGH_CACHE: OnceLock<Mutex<HashMap<IvKey, Vec<u32>>>> = OnceLock::new();

pub async fn run_mt(
    ctx: &infra::gpu::context::GpuContext,
    candidates: &[GpuCandidate],
    configs: &[GpuInput],
) -> Result<Vec<GpuCandidate>, wgpu::BufferAsyncError> {
    assert!(
        configs.len() == 1 || configs.len() == candidates.len(),
        "configs must have length 1 or match candidates length"
    );

    let shader = ShaderLoader::from_wgsl(
        &ctx.device,
        Some("rng_core_mt"),
        include_str!("wgsl/mt.wgsl"),
    );

    let pool = BufferPool::new(&ctx.device);
    let candidate_buffer = pool
        .create_init(candidates, BufferKind::Input, "rng_core_mt_candidate_buffer")
        .buffer;
    let config_buffer = pool
        .create_init(configs, BufferKind::Input, "rng_core_mt_config_buffer")
        .buffer;

    let output_len = candidates.len();
    let output_bytes = (output_len * std::mem::size_of::<GpuCandidate>()) as u64;
    let output_buffer = pool
        .create::<GpuCandidate>(output_bytes as usize, BufferKind::Output, "rng_core_mt_output_buffer")
        .buffer;

    let layout = candidate_config_output_layout(&ctx.device);
    let bind_group = BindGroupBuilder::new(&layout)
        .buffer(0, &candidate_buffer)
        .buffer(1, &config_buffer)
        .buffer(2, &output_buffer)
        .build(&ctx.device, Some("rng_core_mt_bind_group"));

    let pipeline = PipelineFactory::new(&ctx.device)
        .create_compute(&shader, &layout, "main");

    let mut encoder = ctx.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("rng_core_mt_encoder"),
    });
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("rng_core_mt_pass"),
            timestamp_writes: None,
        });
        EncoderUtils::dispatch_1d(
            &mut pass,
            &pipeline,
            &bind_group,
            output_len as u32,
            GpuKernelConfig::SHA1_MT.workgroup_size,
        );
    }
    ctx.queue.submit(Some(encoder.finish()));

    let readback = Readback::new(ctx);
    readback
        .read_buffer_with_pool::<GpuCandidate>(
            &pool,
            &output_buffer,
            output_len,
            Some("rng_core_mt_readback"),
        )
        .await
}

pub async fn run_mt_compact(
    ctx: &infra::gpu::context::GpuContext,
    candidates: &[GpuCandidate],
    configs: &[GpuInput],
) -> Result<Vec<GpuCandidate>, wgpu::BufferAsyncError> {
    assert!(
        configs.len() == 1 || configs.len() == candidates.len(),
        "configs must have length 1 or match candidates length"
    );

    let shader = ShaderLoader::from_wgsl(
        &ctx.device,
        Some("rng_core_mt_compact"),
        include_str!("wgsl/mt_compact.wgsl"),
    );

    let pool = BufferPool::new(&ctx.device);
    let candidate_buffer = pool
        .create_init(candidates, BufferKind::Input, "rng_core_mt_candidate_buffer")
        .buffer;
    let config_buffer = pool
        .create_init(configs, BufferKind::Input, "rng_core_mt_config_buffer")
        .buffer;

    let output_bytes = (MAX_RESULTS * std::mem::size_of::<GpuCandidate>()) as u64;
    let output_buffer = pool
        .create::<GpuCandidate>(output_bytes as usize, BufferKind::Output, "rng_core_mt_output_buffer")
        .buffer;

    let counter_buffer = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("rng_core_mt_counter_buffer"),
        contents: bytemuck::bytes_of(&0u32),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
    });

    let layout = candidate_config_output_counter_layout(&ctx.device);
    let bind_group = BindGroupBuilder::new(&layout)
        .buffer(0, &candidate_buffer)
        .buffer(1, &config_buffer)
        .buffer(2, &output_buffer)
        .buffer(3, &counter_buffer)
        .build(&ctx.device, Some("rng_core_mt_bind_group"));

    let pipeline = PipelineFactory::new(&ctx.device)
        .create_compute(&shader, &layout, "main");

    let mut encoder = ctx.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("rng_core_mt_encoder"),
    });
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("rng_core_mt_pass"),
            timestamp_writes: None,
        });
        EncoderUtils::dispatch_1d(
            &mut pass,
            &pipeline,
            &bind_group,
            candidates.len() as u32,
            GpuKernelConfig::SHA1_MT.workgroup_size,
        );
    }
    ctx.queue.submit(Some(encoder.finish()));

    let readback = Readback::new(ctx);
    let count_vec = readback
        .read_buffer::<u32>(&counter_buffer, 1, Some("rng_core_mt_count_readback"))
        .await?;
    let count = count_vec[0] as usize;
    if count == 0 {
        return Ok(Vec::new());
    }

    let read_len = count.min(MAX_RESULTS);
    readback
        .read_buffer_with_pool::<GpuCandidate>(
            &pool,
            &output_buffer,
            read_len,
            Some("rng_core_mt_readback"),
        )
        .await
}

pub async fn run_mt_seedhigh_candidates(
    ctx: &infra::gpu::context::GpuContext,
    config: &GpuIvConfig,
) -> Result<Vec<u32>, wgpu::BufferAsyncError> {
    let shader = ShaderLoader::from_wgsl(
        &ctx.device,
        Some("rng_core_mt_seedhigh_compact"),
        include_str!("wgsl/mt_seedhigh_compact.wgsl"),
    );

    let pool = BufferPool::new(&ctx.device);
    let config_buffer = pool
        .create_init(std::slice::from_ref(config), BufferKind::Input, "rng_core_mt_seedhigh_config_buffer")
        .buffer;

    let output_bytes = (MAX_RESULTS * std::mem::size_of::<u32>()) as u64;
    let output_buffer = pool
        .create::<u32>(output_bytes as usize, BufferKind::Output, "rng_core_mt_seedhigh_output_buffer")
        .buffer;

    let layout = config_output_counter_params_layout(&ctx.device);
    let pipeline = PipelineFactory::new(&ctx.device)
        .create_compute(&shader, &layout, "main");

    let readback = Readback::new(ctx);

    let mut results = Vec::new();
    let total_len = (u32::MAX as u64) + 1;
    let mut base: u64 = 0;
    let wg = GpuKernelConfig::SHA1_MT.workgroup_size as u64;
    let max_groups = ctx
        .device
        .limits()
        .max_compute_workgroups_per_dimension
        .min(65535) as u64;
    let max_dispatch = max_groups.saturating_mul(wg);
    while base < total_len {
        let remaining = total_len - base;
        let chunk_len = remaining.min(max_dispatch) as u32;

        let counter_buffer = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("rng_core_mt_seedhigh_counter_buffer"),
            contents: bytemuck::bytes_of(&0u32),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        });

        let params = DispatchParams {
            base_index: base,
            total_len,
        };
        let params_buffer = pool
            .create_init(std::slice::from_ref(&params), BufferKind::Input, "rng_core_mt_seedhigh_params_buffer")
            .buffer;

        let bind_group = BindGroupBuilder::new(&layout)
            .buffer(0, &config_buffer)
            .buffer(1, &output_buffer)
            .buffer(2, &counter_buffer)
            .buffer(3, &params_buffer)
            .build(&ctx.device, Some("rng_core_mt_seedhigh_bind_group"));

        let mut encoder = ctx.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("rng_core_mt_seedhigh_encoder"),
        });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("rng_core_mt_seedhigh_pass"),
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
            .read_buffer::<u32>(&counter_buffer, 1, Some("rng_core_mt_seedhigh_count"))
            .await?;
        let count = count_vec[0] as usize;
        if count > 0 {
            let read_len = count.min(MAX_RESULTS);
            let mut chunk = readback
                .read_buffer_with_pool::<u32>(
                    &pool,
                    &output_buffer,
                    read_len,
                    Some("rng_core_mt_seedhigh_readback"),
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

pub async fn run_mt_seedhigh_candidates_cached(
    ctx: &infra::gpu::context::GpuContext,
    config: &GpuIvConfig,
) -> Result<Vec<u32>, wgpu::BufferAsyncError> {
    let key = IvKey {
        iv_step: config.iv_step,
        iv_min: config.iv_min,
        iv_max: config.iv_max,
    };

    if let Some(cache) = SEED_HIGH_CACHE.get() {
        if let Ok(map) = cache.lock() {
            if let Some(existing) = map.get(&key) {
                //println!("seed_high cache hit (len={})", existing.len());
                return Ok(existing.clone());
            }
        }
    }

    //println!("seed_high cache miss; computing");
    let computed = run_mt_seedhigh_candidates(ctx, config).await?;
    //println!("seed_high computed len={}", computed.len());

    let cache = SEED_HIGH_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(mut map) = cache.lock() {
        map.entry(key).or_insert_with(|| computed.clone());
    }

    Ok(computed)
}

pub async fn run_mt_seedhigh_candidates_cached_multi(
    ctx: &infra::gpu::context::GpuContext,
    configs: &[GpuIvConfig],
) -> Result<Vec<u32>, wgpu::BufferAsyncError> {
    if configs.is_empty() {
        return Ok(Vec::new());
    }

    let mut combined: Vec<u32> = Vec::new();
    for cfg in configs {
        let mut chunk = run_mt_seedhigh_candidates_cached(ctx, cfg).await?;
        combined.append(&mut chunk);
    }

    if combined.is_empty() {
        return Ok(Vec::new());
    }

    combined.sort_unstable();
    combined.dedup();
    Ok(combined)
}

#[cfg(all(test, not(ci)))]
mod tests {
    use super::*;
    use infra::gpu::context::GpuContext;

    #[test]
    fn test_mt_seedhigh_speed() {
        pollster::block_on(async {
            let ctx = GpuContext::new().await;

            let iv_min: [u32; 6] = [31u32, 31u32, 31u32, 8, 31u32, 31u32];
            let iv_max: [u32; 6] = [31u32, 31u32, 31u32, 31, 31u32, 31u32];
            let cfg = GpuIvConfig {
                iv_step: 2,
                _pad0: 0,
                iv_min,
                iv_max,
            };

            let start = std::time::Instant::now();
            let candidates = run_mt_seedhigh_candidates(&ctx, &cfg)
                .await
                .expect("run_mt_seedhigh_candidates failed");
            let elapsed = start.elapsed();

            println!("seed_high candidates: {}", candidates.len());
            println!("elapsed: {:?}", elapsed);
        });
    }
}
