use wgpu::{self};

pub const DEFAULT_WORKGROUP_SIZE: u32 = 256;

pub struct GpuContext {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub workgroup_size: u32,
}

impl GpuContext {
    pub async fn new() -> Self {
        Self::new_with_workgroup_size(DEFAULT_WORKGROUP_SIZE).await
    }

    /// Creates a context whose shaders are compiled with `workgroup_size`.
    ///
    /// This value is fixed for each compute pipeline, but does not require a
    /// separate application build. Typical values to benchmark are 64, 128,
    /// and 256.
    pub async fn new_with_workgroup_size(workgroup_size: u32) -> Self {
        assert!(workgroup_size > 0, "workgroup size must be at least 1");
        pollster::block_on(async {
            let instance = wgpu::Instance::default();
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    compatible_surface: None,
                    force_fallback_adapter: false,
                })
                .await
                .expect("No suitable GPU adapter found");

            let (device, queue) = adapter
                .request_device(&wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::SHADER_INT64,
                    required_limits: wgpu::Limits::default(),
                    ..Default::default()
                })
                .await
                .expect("Failed to create device");

            let limits = device.limits();
            assert!(
                workgroup_size <= limits.max_compute_workgroup_size_x
                    && workgroup_size <= limits.max_compute_invocations_per_workgroup,
                "workgroup size {workgroup_size} exceeds this GPU's compute limits (max x: {}, max invocations: {})",
                limits.max_compute_workgroup_size_x,
                limits.max_compute_invocations_per_workgroup,
            );

            Self {
                device,
                queue,
                workgroup_size,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpu_context_can_be_created(){
        pollster::block_on(async {
            let _ctx = GpuContext::new().await;
        });
    }
}
