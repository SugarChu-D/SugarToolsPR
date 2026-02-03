use wgpu::{self};

pub struct GpuContext {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

impl GpuContext {
    pub async fn new() -> Self {
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
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                }, 
                None)
                .await
                .expect("Failed to create device");

            Self { device, queue}
        })
    }
}

#[cfg(all(test, not(ci)))]
mod tests {
    use super::*;

    #[test]
    fn gpu_context_can_be_created(){
        pollster::block_on(async {
            let _ctx = GpuContext::new().await;
        });
    }
}
