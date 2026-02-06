use infra::gpu::bind_group_builder::BindGroupBuilder;
use infra::gpu::buffer_pool::{BufferKind, BufferPool};
use infra::gpu::encoder_utils::EncoderUtils;
use infra::gpu::pipeline_factory::PipelineFactory;
use infra::gpu::readback::Readback;
use infra::gpu::shader_loader::ShaderLoader;

use crate::gpu::bind_layout::input_output_layout;
use crate::gpu::input_layout::{GpuInput, build_inputs_for_keypresses};
use crate::gpu::staging_layout::GpuCandidate;
use crate::gpu::local_gpu_config::GpuKernelConfig;

pub async fn run_sha1(
    ctx: &infra::gpu::context::GpuContext,
    inputs: &[GpuInput],
) -> Result<Vec<GpuCandidate>, wgpu::BufferAsyncError> {
    let shader = ShaderLoader::from_wgsl(
        &ctx.device,
        Some("rng_core_sha1"),
        include_str!("wgsl/sha-1.wgsl"),
    );

    let pool = BufferPool::new(&ctx.device);
    let input_buffer = pool
        .create_init(inputs, BufferKind::Input, "rng_core_sha1_input_buffer")
        .buffer;

    let output_len = inputs.len();
    let output_bytes = (output_len * std::mem::size_of::<GpuCandidate>()) as u64;
    let output_buffer = pool
        .create::<GpuCandidate>(output_bytes as usize, BufferKind::Output, "rng_core_sha1_output_buffer")
        .buffer;

    let layout = input_output_layout(&ctx.device);
    let bind_group = BindGroupBuilder::new(&layout)
        .buffer(0, &input_buffer)
        .buffer(1, &output_buffer)
        .build(&ctx.device, Some("rng_core_sha1_bind_group"));

    let pipeline = PipelineFactory::new(&ctx.device)
        .create_compute(&shader, &layout, "main");

    let mut encoder = ctx.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("rng_core_sha1_encoder"),
    });
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("rng_core_sha1_pass"),
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
            Some("rng_core_sha1_readback"),
        )
        .await
}

pub async fn run_sha1_for_keypresses(
    ctx: &infra::gpu::context::GpuContext,
    ds_config: crate::models::DSConfig,
    datespec: crate::models::game_date_iterator::GameDateSpec,
    timespec: [[u32; 2]; 3],
    p: u32,
    iv_min: [u32; 6],
    iv_max: [u32; 6],
) -> Result<Vec<GpuCandidate>, wgpu::BufferAsyncError> {
    let inputs = build_inputs_for_keypresses(ds_config, datespec, timespec, p, iv_min, iv_max);
    run_sha1(ctx, &inputs).await
}

pub async fn run_sha1_mt(
    ctx: &infra::gpu::context::GpuContext,
    inputs: &[GpuInput],
) -> Result<Vec<GpuCandidate>, wgpu::BufferAsyncError> {
    let sha1_out = run_sha1(ctx, inputs).await?;
    crate::gpu::mt_kernel::run_mt(ctx, &sha1_out, inputs).await
}

#[cfg(all(test, not(ci)))]
mod tests {
    use super::*;
    use crate::models::{DSConfig, FieldRange, GameTime, GameVersion};
    use crate::sha_1::generate_initial_seed0;
    use crate::models::KeyPresses;
    use crate::models::game_date_iterator::GameDateSpec;
    use crate::mt::mt_0;
    use infra::gpu::context::GpuContext;

    #[test]
    fn test_sha1_with_keypresses_print_seed0_and_keypresses() {
        pollster::block_on(async {
            let ctx = GpuContext::new().await;

            let ds_config = DSConfig::new(GameVersion::White2, 0x10FA, false, 0x0009bf6d93ce);
            let datespec = GameDateSpec {
                year: FieldRange { min: 26, max: 26 },
                month: FieldRange { min: 2, max: 2 },
                day: FieldRange { min: 6, max: 6 },
            };

            let game_time = GameTime::new(26, 2, 6, 12, 34, 56);
            let time9 = game_time.get_time9_format();
            let timespec = [[time9, 0], [0, 0], [0, 0]];

            let p = 0u32;
            let iv_min = [0u32; 6];
            let iv_max = [31u32; 6];

            let results = run_sha1_for_keypresses(
                &ctx,
                ds_config,
                datespec,
                timespec,
                p,
                iv_min,
                iv_max,
            )
            .await
            .expect("run_sha1_for_keypresses failed");

            for out in results {
                println!("seed0=0x{:016X} key_presses=0x{:04X}", out.seed0, out.key_presses);
            }
        });
    }

    #[test]
    fn test_sha1_matches_cpu_single_case() {
        pollster::block_on(async {
            let ctx = GpuContext::new().await;

            let ds_config = DSConfig::new(GameVersion::Black, 0x1F0A, false, 0x1122_3344_5566_7788);
            let game_time = GameTime::new(26, 2, 6, 12, 34, 56);
            let time9 = game_time.get_time9_format();
            let timespec = [[time9, 0], [0, 0], [0, 0]];

            let key_presses = 0x2000u32;
            let p = 5u32;
            let iv_min = [0u32; 6];
            let iv_max = [255u32; 6];

            let version_cfg = ds_config.get_version_config();
            let nazo = [
                version_cfg.nazo_values.nazo1,
                version_cfg.nazo_values.nazo2,
                version_cfg.nazo_values.nazo3,
                version_cfg.nazo_values.nazo4,
                version_cfg.nazo_values.nazo5,
            ];
            let vcount_timer0_as_data5 =
                ((version_cfg.vcount.0 as u32) << 16) | (ds_config.Timer0 as u32);

            let input = GpuInput::test_new(
                nazo,
                vcount_timer0_as_data5,
                ds_config.MAC,
                if ds_config.IsDSLite { 0x0600_0006 } else { 0x0600_0008 },
                game_time.get_date8_format(),
                timespec,
                key_presses,
                p,
                iv_min,
                iv_max,
            );

            let results = run_sha1(&ctx, &[input]).await.expect("run_sha1 failed");
            assert_eq!(results.len(), 1);

            let cpu_seed0 = generate_initial_seed0(
                &ds_config,
                &game_time,
                KeyPresses::new(key_presses as u16),
            );
            assert_eq!(results[0].seed0, cpu_seed0);
        });
    }

    #[test]
    fn test_sha1_mt_matches_cpu_single_case() {
        pollster::block_on(async {
            let ctx = GpuContext::new().await;

            let ds_config = DSConfig::new(GameVersion::White2, 0x10F7, false, 0x0009bf6d93ce);
            let game_time = GameTime::new(33, 8, 27, 1, 41, 5);
            let time9 = game_time.get_time9_format();
            let timespec = [[time9, 0], [0, 0], [0, 0]];

            let key_presses = 0x2CDFu32;
            let p = 2u32;
            let iv_min: [u32; 6] = [31u32, 31u32, 7u32, 7u32, 31u32, 31u32];
            let iv_max: [u32; 6] = [31u32; 6];

            let version_cfg = ds_config.get_version_config();
            let nazo: [u32; 5] = [
                version_cfg.nazo_values.nazo1,
                version_cfg.nazo_values.nazo2,
                version_cfg.nazo_values.nazo3,
                version_cfg.nazo_values.nazo4,
                version_cfg.nazo_values.nazo5,
            ];
            let vcount_timer0_as_data5 =
                ((version_cfg.vcount.0 as u32) << 16) | (ds_config.Timer0 as u32);

            let input = GpuInput::test_new(
                nazo,
                vcount_timer0_as_data5,
                ds_config.MAC,
                if ds_config.IsDSLite { 0x0600_0006 } else { 0x0600_0008 },
                game_time.get_date8_format(),
                timespec,
                key_presses,
                p,
                iv_min,
                iv_max,
            );

            let results = run_sha1_mt(&ctx, &[input]).await.expect("run_sha1_mt failed");
            assert_eq!(results.len(), 1);

            let cpu_seed0 = generate_initial_seed0(
                &ds_config,
                &game_time,
                KeyPresses::new(key_presses as u16),
            );
            let cpu_ivs = mt_0(cpu_seed0, p as u8);

            assert_eq!(results[0].seed0, cpu_seed0);
            for i in 0..6 {
                let iv = cpu_ivs[i] as u32;
                assert!(iv_min[i] <= iv && iv <= iv_max[i]);
            }
        });
    }
}
