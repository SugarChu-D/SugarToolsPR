use infra::gpu::bind_group_builder::BindGroupBuilder;
use infra::gpu::buffer_pool::{BufferKind, BufferPool};
use infra::gpu::encoder_utils::EncoderUtils;
use infra::gpu::pipeline_factory::PipelineFactory;
use infra::gpu::readback::Readback;
use infra::gpu::shader_loader::ShaderLoader;

use crate::gpu::bind_layout::candidate_config_output_layout;
use crate::gpu::input_layout::GpuInput;
use crate::gpu::local_gpu_config::GpuKernelConfig;
use crate::gpu::staging_layout::GpuCandidate;

pub async fn run_mt(
    ctx: &infra::gpu::context::GpuContext,
    candidates: &[GpuCandidate],
    configs: &[GpuInput],
) -> Result<Vec<GpuCandidate>, wgpu::BufferAsyncError> {
    assert_eq!(candidates.len(), configs.len());

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
