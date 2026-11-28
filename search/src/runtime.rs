use crate::SearchError;
use infra::gpu::context::{GpuContext, GpuInitError};
use rng_core::gpu::workgroup_size::WorkgroupSize;

pub use infra::gpu::context::GpuCapabilities;

/// Owns the device and queue shared by successive Tepig searches.
pub struct SearchRuntime {
    context: GpuContext,
    capabilities: GpuCapabilities,
}

impl SearchRuntime {
    /// Checks GPU availability and prepares the device for search.
    pub async fn new() -> Result<Self, SearchError> {
        let (context, capabilities) = GpuContext::initialize(WorkgroupSize::W32.as_u32()).await?;
        Ok(Self {
            context,
            capabilities,
        })
    }

    pub fn capabilities(&self) -> &GpuCapabilities {
        &self.capabilities
    }

    pub(crate) fn context(&self, size: WorkgroupSize) -> Result<GpuContext, SearchError> {
        let size = size.as_u32();
        let max = self.capabilities.max_workgroup_size;
        if size > max {
            return Err(GpuInitError::WorkgroupSize { size, max }.into());
        }
        Ok(GpuContext {
            device: self.context.device.clone(),
            queue: self.context.queue.clone(),
            workgroup_size: size,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires an adapter with SHADER_INT64"]
    fn different_workgroup_sizes_share_the_device_and_queue() {
        let runtime = pollster::block_on(SearchRuntime::new()).expect("GPU setup failed");
        let first = runtime.context(WorkgroupSize::W32).unwrap();
        let second = runtime.context(WorkgroupSize::W64).unwrap();
        assert_eq!(first.device, second.device);
        assert_eq!(first.queue, second.queue);
        assert_eq!((first.workgroup_size, second.workgroup_size), (32, 64));
        assert!(runtime.capabilities().supports_shader_int64);
        assert!(runtime.capabilities().max_workgroup_size >= second.workgroup_size);
    }
}
