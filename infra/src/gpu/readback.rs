use bytemuck::Pod;

use super::context::GpuContext;

pub struct Readback<'a> {
    device: &'a wgpu::Device,
    queue: &'a wgpu::Queue,
}

impl<'a> Readback<'a> {
    pub fn new(ctx: &'a GpuContext) -> Self {
        Self {
            device: &ctx.device,
            queue: &ctx.queue,
        }
    }

    /// Read back a GPU buffer into a Vec<T>.
    ///
    /// Requirements:
    /// - `buffer` must have `COPY_SRC` usage.
    /// - `len` is number of T elements to read.
    pub async fn read_buffer<T: Pod>(
        &self,
        buffer: &wgpu::Buffer,
        len: usize,
        label: Option<&str>,
    ) -> Result<Vec<T>, wgpu::BufferAsyncError> {
        let byte_len = (len * std::mem::size_of::<T>()) as u64;

        let staging_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label,
            size: byte_len,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label,
        });
        encoder.copy_buffer_to_buffer(buffer, 0, &staging_buffer, 0, byte_len);
        self.queue.submit(Some(encoder.finish()));

        let buffer_slice = staging_buffer.slice(..);
        let (tx, rx) = futures_intrusive::channel::shared::oneshot_channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).ok();
        });
        self.device.poll(wgpu::Maintain::Wait);

        match rx.receive().await {
            Some(Ok(())) => {
                let data = buffer_slice.get_mapped_range();
                let result: Vec<T> = bytemuck::cast_slice(&data).to_vec();
                drop(data);
                staging_buffer.unmap();
                Ok(result)
            }
            Some(Err(err)) => Err(err),
            None => Err(wgpu::BufferAsyncError::MapAborted),
        }
    }
}
