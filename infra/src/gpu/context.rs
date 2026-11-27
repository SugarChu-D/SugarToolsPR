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

pub const DEFAULT_WORKGROUP_SIZE: u32 = 256;

pub struct GpuContext {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub workgroup_size: u32,
}

impl GpuContext {
    pub async fn new() -> Result<Self, GpuInitError> {
        Self::new_with_workgroup_size(DEFAULT_WORKGROUP_SIZE).await
    }

    /// Creates a context whose shaders are compiled with `workgroup_size`.
    ///
    /// This value is fixed for each compute pipeline, but does not require a
    /// separate application build. Typical values to benchmark are 64, 128,
    /// and 256.
    pub async fn new_with_workgroup_size(workgroup_size: u32) -> Result<Self, GpuInitError> {
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .map_err(GpuInitError::Adapter)?;
        if !adapter.features().contains(wgpu::Features::SHADER_INT64) {
            return Err(GpuInitError::UnsupportedShaderInt64);
        }

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::SHADER_INT64,
                required_limits: wgpu::Limits::default(),
                ..Default::default()
            })
            .await
            .map_err(GpuInitError::Device)?;

        let limits = device.limits();
        let max = limits
            .max_compute_workgroup_size_x
            .min(limits.max_compute_invocations_per_workgroup);
        if !(1..=max).contains(&workgroup_size) {
            return Err(GpuInitError::WorkgroupSize {
                size: workgroup_size,
                max,
            });
        }

        Ok(Self {
            device,
            queue,
            workgroup_size,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires a GPU adapter"]
    fn gpu_context_can_be_created() {
        pollster::block_on(async {
            let _ctx = GpuContext::new().await.expect("GPU setup failed");
        });
    }
}
