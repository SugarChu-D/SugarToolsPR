pub struct EncoderUtils;

impl EncoderUtils {
    pub fn ceil_div(x: u32, divisor: u32) -> u32 {
        if divisor == 0 {
            return 0;
        }
        let x64 = x as u64;
        let d64 = divisor as u64;
        ((x64 + d64 - 1) / d64) as u32
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
