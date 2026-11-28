struct GpuInput {
    nazo: array<u32, 5>,
    vcount_timer0_as_data5: u32,
    mac: u64,
    gxframe_xor_frame: u32,
    date_as_data8: u32,
    hour_range: array<u32, 2>,
    minute_range: array<u32, 2>,
    second_range: array<u32, 2>,
    _pad0: u32,
    iv_step: u32,
    iv_min: array<u32, 6>,
    iv_max: array<u32, 6>,
}

struct GpuIvConfig {
    iv_step: u32,
    _pad0: u32,
    iv_min: array<u32, 6>,
    iv_max: array<u32, 6>,
}

struct GpuCandidate {
    seed0: u64,
    game_date: u32,
    game_time: u32,
    timer0: u32,
    key_presses: u32,
}

const MAX_RESULTS: u32 = 1048576u; // 1 << 20
