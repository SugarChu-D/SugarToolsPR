use std::marker::PhantomData;

pub enum BufferKind {
    Input,
    Output,
    Staging,
}

pub struct BufferHandle<T> {
    pub buffer: wgpu::Buffer,
    _marker: PhantomData<T>,
}
pub struct BufferPool<'a> {
    device: &'a wgpu::Device,
}

impl<'a> BufferPool<'a> {
    pub fn new(device: &'a &wgpu::Device) -> Self {
        Self { device }
    }

    pub fn create<T>(
        &self,
        size: usize,
        kind: BufferKind,
        label: &str,
    ) -> BufferHandle<T> {
        let usage = match kind {
            BufferKind::Input =>
                wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            BufferKind::Output =>
                wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            BufferKind::Staging =>
                wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        };

        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: size as u64,
            usage,
            mapped_at_creation: false,
        });

        BufferHandle { buffer, _marker: PhantomData }
    }
}
