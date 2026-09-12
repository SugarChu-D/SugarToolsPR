use wgpu::PipelineLayoutDescriptor;

pub struct PipelineFactory<'a> {
    device: &'a wgpu::Device,
}

impl<'a> PipelineFactory<'a> {
    pub fn new(device: &'a wgpu::Device) -> Self {
        Self { device }
    }

    pub fn create_compute(
        &self,
        shader: &wgpu::ShaderModule,
        layout: &wgpu::BindGroupLayout,
        entry: &str,
    ) -> wgpu::ComputePipeline {
        let pipeline_layout =
            self.device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[layout],
                immediate_size: 0,
            });

        self.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            module: shader,
            entry_point: Some(entry),
            compilation_options: Default::default(),
            cache: None,
        })
    }

    /// Builds a pipeline using the WGSL `WORKGROUP_SIZE` override constant.
    pub fn create_compute_with_workgroup_size(
        &self,
        shader: &wgpu::ShaderModule,
        layout: &wgpu::BindGroupLayout,
        entry: &str,
        workgroup_size: u32,
    ) -> wgpu::ComputePipeline {
        let pipeline_layout =
            self.device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[layout],
                immediate_size: 0,
            });

        self.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            module: shader,
            entry_point: Some(entry),
            compilation_options: wgpu::PipelineCompilationOptions {
                constants: &[("WORKGROUP_SIZE", workgroup_size as f64)],
                ..Default::default()
            },
            cache: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::*;
    use crate::gpu::context::GpuContext;

    #[test]
    fn workgroup_size_override_creates_a_pipeline() {
        pollster::block_on(async {
            let ctx = GpuContext::new_with_workgroup_size(64).await;
            let shader = ctx.device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("workgroup_size_override_test"),
                source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(
                    "override WORKGROUP_SIZE: u32 = 256u;\n@compute @workgroup_size(WORKGROUP_SIZE) fn main() {}",
                )),
            });
            let layout = ctx.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[],
            });

            let _pipeline = PipelineFactory::new(&ctx.device).create_compute_with_workgroup_size(
                &shader,
                &layout,
                "main",
                ctx.workgroup_size,
            );
        });
    }
}
