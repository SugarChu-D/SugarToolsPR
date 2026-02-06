pub struct BindGroupBuilder<'a> {
    layout: &'a wgpu::BindGroupLayout,
    entries: Vec<wgpu::BindGroupEntry<'a>>,
}

impl<'a> BindGroupBuilder<'a> {
    pub fn new(layout: &'a wgpu::BindGroupLayout) -> Self {
        Self {
            layout,
            entries: Vec::new(),
        }
    }

    pub fn buffer(mut self, binding: u32, buffer: &'a wgpu::Buffer) -> Self {
        self.entries.push(wgpu::BindGroupEntry {
            binding,
            resource: buffer.as_entire_binding(),
        });
        self
    }

    pub fn build(self, device: &wgpu::Device, label: Option<&str>) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label,
            layout: self.layout,
            entries: &self.entries,
        })
    }
}