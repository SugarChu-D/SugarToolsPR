#[repr(C)]
pub struct GpuCandidate {
    pub seed0: u64,
    pub game_date: u32,
    pub game_time: u32,
    pub timer0: u32,
    pub key_presses: u32,
}
