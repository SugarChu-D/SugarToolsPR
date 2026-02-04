pub struct GpuKernelConfig {
    pub workgroup_size: u32,
}

impl GpuKernelConfig {
    pub const SHA1_MT: Self = Self {
        workgroup_size: 256,
    };
}
