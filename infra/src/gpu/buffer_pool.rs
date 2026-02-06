use std::{marker::PhantomData, sync::Mutex};

use bytemuck::Pod;
use wgpu::util::DeviceExt;

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
    staging_cache: Mutex<Option<StagingCache>>,
}

impl<'a> BufferPool<'a> {
    pub fn new(device: &'a wgpu::Device) -> Self {
        Self {
            device,
            staging_cache: Mutex::new(None),
        }
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

    pub fn create_init<T: Pod>(
        &self,
        contents: &[T],
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

        let buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: bytemuck::cast_slice(contents),
            usage,
        });

        BufferHandle { buffer, _marker: PhantomData }
    }

    /// Take a staging buffer from the cache (or create one).
    /// The caller should return it with `put_staging` after use.
    pub fn take_staging(&self, size: usize, label: &str) -> wgpu::Buffer {
        if let Ok(mut cache) = self.staging_cache.lock() {
            if let Some(cache) = cache.take() {
                if cache.size >= size {
                    return cache.buffer;
                }
            }
        }

        self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: size as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    /// Return a staging buffer to the cache for reuse.
    pub fn put_staging(&self, size: usize, buffer: wgpu::Buffer) {
        if let Ok(mut cache) = self.staging_cache.lock() {
            match cache.as_ref() {
                Some(existing) if existing.size >= size => {
                    // Keep the larger cached buffer.
                }
                _ => {
                    *cache = Some(StagingCache { size, buffer });
                }
            }
        }
    }
}

struct StagingCache {
    size: usize,
    buffer: wgpu::Buffer,
}
