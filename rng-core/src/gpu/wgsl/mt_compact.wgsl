// MT19937 IVS generation + range filter with compacted output (atomic counter)
//
// Output:
// - If ivs are within range, write candidate into output buffer at atomic index.
// - Otherwise ignore.

const M: u32 = 397u;
const MAX_P: u32 = 20u;
const TABLE_SIZE: u32 = MAX_P + 6u + M; // 423

const UPPER_MASK: u32 = 0x80000000u;
const LOWER_MASK: u32 = 0x7fffffffu;
const MATRIX_A: u32 = 0x9908B0DFu;

const TEMPERING_MASK_B: u32 = 0x9D2C5680u;
const TEMPERING_MASK_C: u32 = 0xEFC60000u;

const INIT_MULTIPLIER: u32 = 1812433253u;

const LCG_MULTIPLIER_LO: u32 = 0x6C078965u;
const LCG_MULTIPLIER_HI: u32 = 0x5D588B65u;
const LCG_INCREMENT_LO: u32 = 0x00269EC3u;
const LCG_INCREMENT_HI: u32 = 0x00000000u;

const MAX_RESULTS: u32 = 1048576u;

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

struct GpuCandidate {
    seed0: u64,
    game_date: u32,
    game_time: u32,
    timer0: u32,
    key_presses: u32,
}

struct InputBuffer {
    data: array<GpuCandidate>,
}

struct ConfigBuffer {
    data: array<GpuInput>,
}

struct OutputBuffer {
    data: array<GpuCandidate>,
}

struct CounterBuffer {
    value: atomic<u32>,
}

@group(0) @binding(0)
var<storage, read> input_buf: InputBuffer;

@group(0) @binding(1)
var<storage, read> config_buf: ConfigBuffer;

@group(0) @binding(2)
var<storage, read_write> output_buf: OutputBuffer;

@group(0) @binding(3)
var<storage, read_write> counter_buf: CounterBuffer;

fn tempering(val_in: u32) -> u32 {
    var val = val_in;
    val = val ^ (val >> 11u);
    val = val ^ ((val << 7u) & TEMPERING_MASK_B);
    val = val ^ ((val << 15u) & TEMPERING_MASK_C);
    val = val ^ (val >> 18u);
    return (val >> 27u) & 0xFFu;
}

fn init_table(table: ptr<function, array<u32, 423>>, seed: u32, init_range: u32) {
    (*table)[0] = seed;
    var prev = (*table)[0];
    for (var i: u32 = 1u; i < TABLE_SIZE; i = i + 1u) {
        if (i <= init_range) {
            prev = INIT_MULTIPLIER * (prev ^ (prev >> 30u)) + i;
            (*table)[i] = prev;
        } else {
            (*table)[i] = 0u;
        }
    }
}

fn generate_ivs(table: ptr<function, array<u32, 423>>, p: u32) -> array<u32, 6> {
    var ivs: array<u32, 6>;
    for (var j: u32 = 0u; j < 6u; j = j + 1u) {
        let i = p + j;
        let x = ((*table)[i] & UPPER_MASK) | ((*table)[i + 1u] & LOWER_MASK);
        let x_a = (x >> 1u) ^ (select(0u, MATRIX_A, (x & 1u) != 0u));
        let val = (*table)[i + M] ^ x_a;
        ivs[j] = tempering(val);
    }
    return ivs;
}

fn ivs_in_range(ivs: array<u32, 6>, minv: array<u32, 6>, maxv: array<u32, 6>) -> bool {
    for (var i: u32 = 0u; i < 6u; i = i + 1u) {
        if (ivs[i] < minv[i] || ivs[i] > maxv[i]) {
            return false;
        }
    }
    return true;
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i >= arrayLength(&input_buf.data)) {
        return;
    }

    let in_candidate = input_buf.data[i];
    let cfg_len = arrayLength(&config_buf.data);
    let cfg_idx = select(i, 0u, cfg_len == 1u);
    let cfg = config_buf.data[cfg_idx];

    let mult: u64 = (u64(LCG_MULTIPLIER_HI) << 32u) | u64(LCG_MULTIPLIER_LO);
    let inc: u64 = (u64(LCG_INCREMENT_HI) << 32u) | u64(LCG_INCREMENT_LO);
    let seed1: u64 = in_candidate.seed0 * mult + inc;
    let seed_high: u32 = u32(seed1 >> 32u);

    let p = cfg.iv_step;
    let init_range = p + 6u + M;

    var table: array<u32, 423>;
    init_table(&table, seed_high, init_range);
    let ivs = generate_ivs(&table, p);

    if (ivs_in_range(ivs, cfg.iv_min, cfg.iv_max)) {
        let idx = atomicAdd(&counter_buf.value, 1u);
        if (idx < MAX_RESULTS) {
            output_buf.data[idx] = in_candidate;
        }
    }
}
