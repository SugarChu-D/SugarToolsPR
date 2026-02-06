use infra::gpu::bind_group_builder::BindGroupBuilder;
use infra::gpu::buffer_pool::{BufferKind, BufferPool};
use infra::gpu::encoder_utils::EncoderUtils;
use infra::gpu::pipeline_factory::PipelineFactory;
use infra::gpu::readback::Readback;
use infra::gpu::shader_loader::ShaderLoader;

use crate::gpu::local_gpu_config::GpuKernelConfig;

use crate::gpu::bind_layout::input_output_layout;
use crate::gpu::input_layout::{GpuInput, build_inputs_for_keypresses};
use crate::gpu::staging_layout::GpuCandidate;

pub async fn run_test_io(
    ctx: &infra::gpu::context::GpuContext,
    inputs: &[GpuInput],
) -> Result<Vec<GpuCandidate>, wgpu::BufferAsyncError> {
    let shader = ShaderLoader::from_wgsl(
        &ctx.device,
        Some("rng_core_test_io"),
        include_str!("wgsl/test_io.wgsl"),
    );

    let pool = BufferPool::new(&ctx.device);
    let input_buffer = pool
        .create_init(inputs, BufferKind::Input, "rng_core_input_buffer")
        .buffer;

    let output_len = inputs.len();
    let output_bytes = (output_len * std::mem::size_of::<GpuCandidate>()) as u64;
    let output_buffer = pool
        .create::<GpuCandidate>(output_bytes as usize, BufferKind::Output, "rng_core_output_buffer")
        .buffer;

    let layout = input_output_layout(&ctx.device);
    let bind_group = BindGroupBuilder::new(&layout)
        .buffer(0, &input_buffer)
        .buffer(1, &output_buffer)
        .build(&ctx.device, Some("rng_core_test_io_bind_group"));

    let pipeline = PipelineFactory::new(&ctx.device)
        .create_compute(&shader, &layout, "main");

    let mut encoder = ctx.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("rng_core_test_io_encoder"),
    });
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("rng_core_test_io_pass"),
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

    readback_candidates(ctx, &pool, &output_buffer, output_len).await
}

pub async fn run_test_io_for_keypresses(
    ctx: &infra::gpu::context::GpuContext,
    ds_config: crate::models::DSConfig,
    datespec: crate::models::game_date_iterator::GameDateSpec,
    timespec: [[u32; 2]; 3],
    p: u32,
    iv_min: [u32; 6],
    iv_max: [u32; 6],
) -> Result<Vec<GpuCandidate>, wgpu::BufferAsyncError> {
    let inputs = build_inputs_for_keypresses(ds_config, datespec, timespec, p, iv_min, iv_max);
    run_test_io(ctx, &inputs).await
}

fn readback_candidates<'a>(
    ctx: &'a infra::gpu::context::GpuContext,
    pool: &'a BufferPool<'a>,
    buffer: &'a wgpu::Buffer,
    len: usize,
) -> impl std::future::Future<Output = Result<Vec<GpuCandidate>, wgpu::BufferAsyncError>> + 'a {
    let readback = Readback::new(ctx);
    async move {
        readback
            .read_buffer_with_pool::<GpuCandidate>(
                pool,
                buffer,
                len,
                Some("rng_core_test_io_readback"),
            )
            .await
    }
}

#[cfg(all(test, not(ci)))]
mod tests {
    use super::*;
    use crate::gpu::input_layout::GpuInput;
    use crate::gpu::input_layout::GPUInputIterator;
    use crate::models::{DSConfig, FieldRange, GameVersion};
    use crate::models::game_date_iterator::GameDateSpec;
    use infra::gpu::context::GpuContext;

    #[test]
    fn test_io_kernel_roundtrip() {
        pollster::block_on(async {
            let ctx = GpuContext::new().await;

            let nazo = [10, 20, 30, 40, 50];
            let vcount_timer0_as_data5 = 0x1122_3344;
            let mac = 0x1122_3344_5566_7788;
            let gxframe_xor_frame = 0x0600_0008;
            let date_as_data8 = 0x2026_0206;
            let timespec = [[123, 456], [789, 1011], [1213, 1415]];
            let key_presses = 0x2000u32;
            let p = 5u32;
            let iv_min = [0u32; 6];
            let iv_max = [255u32; 6];

            let input = GpuInput::test_new(
                nazo,
                vcount_timer0_as_data5,
                mac,
                gxframe_xor_frame,
                date_as_data8,
                timespec,
                key_presses,
                p,
                iv_min,
                iv_max,
            );

            let results = run_test_io(&ctx, &[input]).await.expect("run_test_io failed");
            assert_eq!(results.len(), 1);

            let out = results[0];
            assert_eq!(out.seed0, mac);
            assert_eq!(out.game_date, date_as_data8);
            assert_eq!(out.game_time, timespec[0][0]);
            assert_eq!(out.timer0, vcount_timer0_as_data5);
            assert_eq!(out.key_presses, key_presses);
        });
    }

    #[test]
    fn test_io_kernel_with_gpu_input_iterator() {
        pollster::block_on(async {
            let ctx = GpuContext::new().await;

            let ds_config = DSConfig::new(GameVersion::Black, 0x1F0A, false, 0x1122_3344_5566_7788);
            let datespec = GameDateSpec {
                year: FieldRange { min: 26, max: 26 },
                month: FieldRange { min: 2, max: 2 },
                day: FieldRange { min: 6, max: 6 },
            };
            let timespec = [[123, 456], [789, 1011], [1213, 1415]];
            let key_presses = 0x2000u32;
            let p = 5u32;
            let iv_min = [0u32; 6];
            let iv_max = [255u32; 6];

            let mut iter = GPUInputIterator::new(ds_config, datespec, timespec, key_presses, p, iv_min, iv_max);
            let input = iter.next().expect("iterator should yield one item");

            let results = run_test_io(&ctx, &[input]).await.expect("run_test_io failed");
            assert_eq!(results.len(), 1);
            let out = results[0];

            let version_cfg = ds_config.get_version_config();
            let expected_vcount_timer0 =
                ((version_cfg.vcount.0 as u32) << 16) | (ds_config.Timer0 as u32);
            let expected_date = datespec.start().get_date8_format();

            assert_eq!(out.seed0, ds_config.MAC);
            assert_eq!(out.game_date, expected_date);
            assert_eq!(out.game_time, timespec[0][0]);
            assert_eq!(out.timer0, expected_vcount_timer0);
            assert_eq!(out.key_presses, key_presses);
        });
    }
}
