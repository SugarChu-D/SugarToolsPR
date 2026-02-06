use infra::gpu::bind_group_builder::BindGroupBuilder;
use infra::gpu::buffer_pool::{BufferKind, BufferPool};
use infra::gpu::encoder_utils::EncoderUtils;
use infra::gpu::pipeline_factory::PipelineFactory;
use infra::gpu::readback::Readback;
use infra::gpu::shader_loader::ShaderLoader;

use crate::gpu::local_gpu_config::GpuKernelConfig;

use crate::gpu::bind_layout::input_output_layout;
use crate::gpu::input_layout::GpuInput;
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

            let input = GpuInput::test_new(
                nazo,
                vcount_timer0_as_data5,
                mac,
                gxframe_xor_frame,
                date_as_data8,
                timespec,
            );

            let results = run_test_io(&ctx, &[input]).await.expect("run_test_io failed");
            assert_eq!(results.len(), 1);

            let out = results[0];
            assert_eq!(out.seed0, mac);
            assert_eq!(out.game_date, date_as_data8);
            assert_eq!(out.game_time, timespec[0][0]);
            assert_eq!(out.timer0, vcount_timer0_as_data5);
            assert_eq!(out.key_presses, nazo[0]);
        });
    }
}
