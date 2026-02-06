pub struct ShaderLoader;

impl ShaderLoader {
    pub fn from_wgsl(
        device: &wgpu::Device,
        label: Option<&str>,
        source: &str,
    ) -> wgpu::ShaderModule {
        device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label,
            source: wgpu::ShaderSource::Wgsl(source.into()),
        })
    }
}