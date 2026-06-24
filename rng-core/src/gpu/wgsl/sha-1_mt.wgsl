struct InputBuffer {
    data: array<GpuInput>,
}

struct OutputBuffer {
    data: array<GpuCandidate>,
}

struct CounterBuffer {
    value: atomic<u32>,
}

struct DispatchParams {
    base_index: u64,
    total_len: u64,
}

@group(0) @binding(0)
var<storage, read> input_buf: InputBuffer;

@group(0) @binding(1)
var<storage, read_write> output_buf: OutputBuffer;

@group(0) @binding(2)
var<storage, read_write> counter_buf: CounterBuffer;

@group(0) @binding(3)
var<storage, read> params: DispatchParams;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let global = params.base_index + u64(gid.x);
    if (global >= params.total_len) {
        return;
    }
    let input_len = arrayLength(&input_buf.data);
    if (input_len == 0u) {
        return;
    }

    let cfg0 = input_buf.data[0];
    let h_min = cfg0.hour_range[0];
    let h_max = cfg0.hour_range[1];
    let m_min = cfg0.minute_range[0];
    let m_max = cfg0.minute_range[1];
    let s_min = cfg0.second_range[0];
    let s_max = cfg0.second_range[1];
    if (h_max < h_min || m_max < m_min || s_max < s_min) {
        return;
    }

    let h_count = h_max - h_min + 1u;
    let m_count = m_max - m_min + 1u;
    let s_count = s_max - s_min + 1u;
    let time_count = h_count * m_count * s_count;
    let per_input = u64(KP_COUNT) * u64(time_count);

    let input_idx = global / per_input;
    if (input_idx >= u64(input_len)) {
        return;
    }
    let local = global - input_idx * per_input;

    let kp_idx = local / u64(time_count);
    let time_idx = local - kp_idx * u64(time_count);

    let h_idx = time_idx / u64(m_count * s_count);
    let rem = time_idx - h_idx * u64(m_count * s_count);
    let m_idx = rem / u64(s_count);
    let s_idx = rem - m_idx * u64(s_count);

    let hour = h_min + u32(h_idx);
    let minute = m_min + u32(m_idx);
    let second = s_min + u32(s_idx);
    let time9 = time9_from_hms(hour, minute, second);

    let key_presses = KEY_RANGE_START + u32(kp_idx);
    let input = input_buf.data[u32(input_idx)];
    let seed0: u64 = sha1_seed0(input, time9, key_presses);

    let mult: u64 = (u64(LCG_MULTIPLIER_HI) << 32u) | u64(LCG_MULTIPLIER_LO);
    let inc: u64 = (u64(LCG_INCREMENT_HI) << 32u) | u64(LCG_INCREMENT_LO);
    let seed1: u64 = seed0 * mult + inc;
    let seed_high: u32 = u32(seed1 >> 32u);

    let p = input.iv_step;
    let init_range = p + 6u + M;
    var table: array<u32, 423>;
    init_table(&table, seed_high, init_range);
    let ivs = generate_ivs(&table, p);

    if (ivs_in_range(ivs, input.iv_min, input.iv_max)) {
        let idx = atomicAdd(&counter_buf.value, 1u);
        if (idx < MAX_RESULTS) {
            var out: GpuCandidate;
            out.seed0 = seed0;
            out.game_date = input.date_as_data8;
            out.game_time = time9;
            out.timer0 = input.vcount_timer0_as_data5;
            out.key_presses = key_presses;
            output_buf.data[idx] = out;
        }
    }
}
