// MT19937 IVS filter over seed_high space (0..=0xFFFFFFFF) with compacted output.
// Writes matching seed_high values into output list using atomic counter.

const M: u32 = 397u;
const MAX_P: u32 = 20u;
const TABLE_SIZE: u32 = MAX_P + 6u + M; // 423

const UPPER_MASK: u32 = 0x80000000u;
const LOWER_MASK: u32 = 0x7fffffffu;
const MATRIX_A: u32 = 0x9908B0DFu;

const TEMPERING_MASK_B: u32 = 0x9D2C5680u;
const TEMPERING_MASK_C: u32 = 0xEFC60000u;

const INIT_MULTIPLIER: u32 = 1812433253u;

const MAX_RESULTS: u32 = 1048576u; // 2^20

struct GpuIvConfig {
    iv_step: u32,
    _pad0: u32,
    iv_min: array<u32, 6>,
    iv_max: array<u32, 6>,
}

struct ConfigBuffer {
    data: array<GpuIvConfig>,
}

struct OutputBuffer {
    data: array<u32>,
}

struct CounterBuffer {
    value: atomic<u32>,
}

struct DispatchParams {
    base_index: u64,
    total_len: u64,
}

@group(0) @binding(0)
var<storage, read> config_buf: ConfigBuffer;

@group(0) @binding(1)
var<storage, read_write> output_buf: OutputBuffer;

@group(0) @binding(2)
var<storage, read_write> counter_buf: CounterBuffer;

@group(0) @binding(3)
var<storage, read> params: DispatchParams;

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
    let global = params.base_index + u64(gid.x);
    if (global >= params.total_len) { return; }

    let cfg = config_buf.data[0];
    let seed_high = u32(global);

    let p = cfg.iv_step;
    let init_range = p + 6u + M;

    var table: array<u32, 423>;
    init_table(&table, seed_high, init_range);
    let ivs = generate_ivs(&table, p);

    if (ivs_in_range(ivs, cfg.iv_min, cfg.iv_max)) {
        let idx = atomicAdd(&counter_buf.value, 1u);
        if (idx < MAX_RESULTS) {
            output_buf.data[idx] = seed_high;
        }
    }
}
