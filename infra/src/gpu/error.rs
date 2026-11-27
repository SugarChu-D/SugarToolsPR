#[derive(Debug, thiserror::Error)]
pub enum GpuInitError {
    #[error("No suitable GPU adapter found: {0}")]
    Adapter(#[source] wgpu::RequestAdapterError),
    #[error("GPU does not support 64-bit integer shaders")]
    UnsupportedShaderInt64,
    #[error("Failed to create GPU device: {0}")]
    Device(#[source] wgpu::RequestDeviceError),
    #[error("Workgroup size {size} is outside the supported range 1..={max}")]
    WorkgroupSize { size: u32, max: u32 },
}

#[derive(Debug, thiserror::Error)]
pub enum ReadbackError {
    #[error("GPU polling failed: {0}")]
    Poll(#[from] wgpu::PollError),
    #[error("GPU buffer mapping failed: {0}")]
    Map(#[from] wgpu::BufferAsyncError),
    #[error("GPU buffer mapping callback was dropped")]
    CallbackDropped,
}
