#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkgroupSize {
    W64,
    W128,
    W256
}

impl WorkgroupSize {
    pub const fn as_u32(self) -> u32 { match self { Self::W64 => 64, Self::W128 => 128, Self::W256 => 256 } }
    pub fn try_from_u32(value: u32) -> Result<Self, String> {
        match value {
            64 => Ok(Self::W64),
            128 => Ok(Self::W128),
            256 => Ok(Self::W256),
            _ => Err("workgroup_size must be one of 64, 128, or 256".into())
        }
    }
}
