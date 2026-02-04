#[repr(C)]
struct GpuInput {
    nazo: [u32; 5],
    vcount_timer0_as_data5: u32,
    mac: u64,
    gxframe_xor_frame: u32,
    day_as_data8: u32,
    time_range: [(u8, u8); 3],
}
