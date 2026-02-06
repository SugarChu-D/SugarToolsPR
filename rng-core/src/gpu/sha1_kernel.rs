use infra::gpu::bind_group_builder::BindGroupBuilder;
use infra::gpu::buffer_pool::{BufferKind, BufferPool};
use infra::gpu::encoder_utils::EncoderUtils;
use infra::gpu::pipeline_factory::PipelineFactory;
use infra::gpu::readback::Readback;
use infra::gpu::shader_loader::ShaderLoader;

use crate::gpu::bind_layout::input_output_layout;
use crate::gpu::input_layout::{GpuInput};
use crate::gpu::staging_layout::GpuCandidate;
use crate::gpu::local_gpu_config::GpuKernelConfig;

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
    let output_len = kp_count * time_count * input.len();
    if output_len == 0 {
        return Ok(Vec::new());
    }

    let shader = ShaderLoader::from_wgsl(
        &ctx.device,
        Some("rng_core_sha1_expand"),
        include_str!("wgsl/sha-1.wgsl"),
    );

    let pool = BufferPool::new(&ctx.device);
    let input_buffer = pool
        .create_init(input, BufferKind::Input, "rng_core_sha1_expand_input_buffer")
        .buffer;

    let output_bytes = (output_len * std::mem::size_of::<GpuCandidate>()) as u64;
    let output_buffer = pool
        .create::<GpuCandidate>(output_bytes as usize, BufferKind::Output, "rng_core_sha1_expand_output_buffer")
        .buffer;

    let layout = input_output_layout(&ctx.device);
    let bind_group = BindGroupBuilder::new(&layout)
        .buffer(0, &input_buffer)
        .buffer(1, &output_buffer)
        .build(&ctx.device, Some("rng_core_sha1_expand_bind_group"));

    let pipeline = PipelineFactory::new(&ctx.device)
        .create_compute(&shader, &layout, "main");

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
            Some("rng_core_sha1_expand_readback"),
        )
        .await
}

pub async fn run_sha1_mt(
    ctx: &infra::gpu::context::GpuContext,
    input: &[GpuInput],
) -> Result<Vec<GpuCandidate>, wgpu::BufferAsyncError> {
    let sha1_out = run_sha1(ctx, input).await?;
    if input.is_empty() {
        return Ok(Vec::new());
    }

    #[cfg(debug_assertions)]
    {
        let first = &input[0];
        for other in input.iter().skip(1) {
            assert_eq!(other.iv_step, first.iv_step, "iv_step must match across inputs");
            assert_eq!(other.iv_min, first.iv_min, "iv_min must match across inputs");
            assert_eq!(other.iv_max, first.iv_max, "iv_max must match across inputs");
        }
    }

    // MT uses only IV config, which is expected to be the same across inputs.
    crate::gpu::mt_kernel::run_mt(ctx, &sha1_out, &input[0..1]).await
}

#[cfg(all(test, not(ci)))]
mod tests {
    use super::*;
    use crate::models::{DSConfig, GameTime, GameVersion};
    use crate::sha_1::generate_initial_seed0;
    use crate::models::KeyPresses;
    use crate::mt::mt_0;
    use infra::gpu::context::GpuContext;

    #[test]
    fn test_sha1_mt_matches_cpu_single_case() {
        pollster::block_on(async {
            let ctx = GpuContext::new().await;

            let ds_config = DSConfig::new(GameVersion::White2, 0x10F7, false, 0x0009bf6d93ce);
            
            let p = 2u32;
            let iv_min: [u32; 6] = [31u32, 31u32, 7u32, 7u32, 31u32, 31u32];
            let iv_max: [u32; 6] = [31u32; 6];
            
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
                hour_range:[1,1],
                minute_range: [41,41],
                second_range: [5,5],
                _pad0: 0,
                iv_step: 2,
                iv_min,
                iv_max,
            }];

            let game_time: GameTime = GameTime {
                year: 33,
                month: 8,
                day: 27,
                hour: 1,
                minute: 41,
                second: 5,
            };
            let results = run_sha1_mt(&ctx, &inputs).await.expect("run_sha1_mt failed");

            let out = results
                .iter()
                .find(|c| c.seed0 != 0)
                .expect("candidate not found");

            let key_presses = KeyPresses::new(out.key_presses as u16);
            let cpu_seed0 = generate_initial_seed0(&ds_config, &game_time, key_presses);
            let cpu_ivs = mt_0(cpu_seed0, p as u8);

            assert_eq!(out.seed0, cpu_seed0);
            for i in 0..6 {
                let iv = cpu_ivs[i] as u32;
                assert!(iv_min[i] <= iv && iv <= iv_max[i]);
            }
        });
    }
}
