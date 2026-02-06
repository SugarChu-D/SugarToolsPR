pub struct EncoderUtils;

impl EncoderUtils {
    pub fn ceil_div(x: u32, divisor: u32) -> u32 {
        (x + divisor - 1) / divisor
    }

    pub fn dispatch_1d(
        pass: &mut wgpu::ComputePass<'_>,
        pipeline: &wgpu::ComputePipeline,
        bind_group: &wgpu::BindGroup,
        x: u32,
        workgroup_size: u32,
    ) {
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, bind_group, &[]);
        pass.dispatch_workgroups(Self::ceil_div(x, workgroup_size), 1, 1);
    }
}
