use std::marker::PhantomData;

use wgpu::{self,util::DeviceExt};

pub struct InputBuffer<T> {
    pub buffer: wgpu::Buffer,
    pub size: u64,
    _phantom: PhantomData<T>,
}

impl<T: bytemuck::Pod> InputBuffer<T> {
    pub fn new (device: &wgpu::Device, data: &[T]) -> Self {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Input Buffer"),
            contents: bytemuck::cast_slice(data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
        Self {
            buffer,
            size: (data.len() * std::mem::size_of::<T>()) as u64,
            _phantom: PhantomData,
        }
    }
}
