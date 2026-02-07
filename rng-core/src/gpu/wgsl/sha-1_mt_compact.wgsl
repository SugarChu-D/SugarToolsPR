// SHA-1 + MT19937 IVS filter with compacted output (atomic counter)
//
// Input:
// - GpuInput array (date + ranges)
// - DispatchParams (base index, total length)
//
// Output:
// - Only candidates that pass IV range are written to output buffer.
// - Counter indicates number of outputs (capped at MAX_RESULTS).

const KEY_RANGE_START: u32 = 0x2000u;
const KEY_RANGE_END: u32 = 0x2FFFu;
const KP_COUNT: u32 = KEY_RANGE_END - KEY_RANGE_START + 1u;

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

const MAX_RESULTS: u32 = 1048576u; // 2^20

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

fn rotl32(x: u32, n: u32) -> u32 {
    return (x << n) | (x >> (32u - n));
}

fn bswap32(x: u32) -> u32 {
    return ((x & 0x000000FFu) << 24u)
        | ((x & 0x0000FF00u) << 8u)
        | ((x & 0x00FF0000u) >> 8u)
        | ((x & 0xFF000000u) >> 24u);
}

fn write_le(bytes: ptr<function, array<u32, 64>>, index: u32, value: u32) {
    (*bytes)[index + 0u] = (value >> 0u) & 0xFFu;
    (*bytes)[index + 1u] = (value >> 8u) & 0xFFu;
    (*bytes)[index + 2u] = (value >> 16u) & 0xFFu;
    (*bytes)[index + 3u] = (value >> 24u) & 0xFFu;
}

fn write_be(bytes: ptr<function, array<u32, 64>>, index: u32, value: u32) {
    (*bytes)[index + 0u] = (value >> 24u) & 0xFFu;
    (*bytes)[index + 1u] = (value >> 16u) & 0xFFu;
    (*bytes)[index + 2u] = (value >> 8u) & 0xFFu;
    (*bytes)[index + 3u] = (value >> 0u) & 0xFFu;
}

fn time9_from_hms(hour: u32, minute: u32, second: u32) -> u32 {
    let adjusted_hour = select(hour, hour + 40u, hour >= 12u);
    let hex_hour = ((adjusted_hour / 10u) << 4u) | (adjusted_hour % 10u);
    let hex_min = ((minute / 10u) << 4u) | (minute % 10u);
    let hex_sec = ((second / 10u) << 4u) | (second % 10u);
    return (hex_hour << 24u) | (hex_min << 16u) | (hex_sec << 8u);
}

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
    let input_len = arrayLength(&input_buf.data);
    if (input_len == 0u) { return; }

    let cfg0 = input_buf.data[0];
    let h_min = cfg0.hour_range[0];
    let h_max = cfg0.hour_range[1];
    let m_min = cfg0.minute_range[0];
    let m_max = cfg0.minute_range[1];
    let s_min = cfg0.second_range[0];
    let s_max = cfg0.second_range[1];
    if (h_max < h_min || m_max < m_min || s_max < s_min) { return; }

    let h_count = h_max - h_min + 1u;
    let m_count = m_max - m_min + 1u;
    let s_count = s_max - s_min + 1u;
    let time_count = h_count * m_count * s_count;
    let per_input = u64(KP_COUNT) * u64(time_count);

    let input_idx = global / per_input;
    if (input_idx >= u64(input_len)) { return; }
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

    // Build message bytes (52 bytes)
    var bytes: array<u32, 64>;
    for (var j: u32 = 0u; j < 64u; j = j + 1u) {
        bytes[j] = 0u;
    }

    var off: u32 = 0u;
    write_le(&bytes, off, input.nazo[0]); off = off + 4u;
    write_le(&bytes, off, input.nazo[1]); off = off + 4u;
    write_le(&bytes, off, input.nazo[2]); off = off + 4u;
    write_le(&bytes, off, input.nazo[3]); off = off + 4u;
    write_le(&bytes, off, input.nazo[4]); off = off + 4u;
    write_le(&bytes, off, input.vcount_timer0_as_data5); off = off + 4u;

    let mac_lower_16: u32 = u32(input.mac & u64(0xFFFFu));
    write_be(&bytes, off, mac_lower_16); off = off + 4u;

    let gxframe_xor_frame_le = bswap32(input.gxframe_xor_frame);
    let mac_middle_16: u32 = u32((input.mac >> 16u) & u64(0xFFFFFFFFu));
    let data7: u32 = gxframe_xor_frame_le ^ mac_middle_16;
    write_be(&bytes, off, data7); off = off + 4u;

    write_be(&bytes, off, input.date_as_data8); off = off + 4u;
    write_be(&bytes, off, time9); off = off + 4u;

    write_le(&bytes, off, 0u); off = off + 4u; // data10
    write_le(&bytes, off, 0u); off = off + 4u; // data11

    write_le(&bytes, off, key_presses); off = off + 4u; // data12

    // Padding
    bytes[52] = 0x80u;
    bytes[56] = 0u; bytes[57] = 0u; bytes[58] = 0u; bytes[59] = 0u;
    bytes[60] = 0u; bytes[61] = 0u; bytes[62] = 0x01u; bytes[63] = 0xA0u;

    var w: array<u32, 80>;
    for (var t: u32 = 0u; t < 16u; t = t + 1u) {
        let b0 = bytes[t * 4u + 0u];
        let b1 = bytes[t * 4u + 1u];
        let b2 = bytes[t * 4u + 2u];
        let b3 = bytes[t * 4u + 3u];
        w[t] = (b0 << 24u) | (b1 << 16u) | (b2 << 8u) | b3;
    }
    for (var t: u32 = 16u; t < 80u; t = t + 1u) {
        w[t] = rotl32(w[t - 3u] ^ w[t - 8u] ^ w[t - 14u] ^ w[t - 16u], 1u);
    }

    var a: u32 = 0x67452301u;
    var b: u32 = 0xEFCDAB89u;
    var c: u32 = 0x98BADCFEu;
    var d: u32 = 0x10325476u;
    var e: u32 = 0xC3D2E1F0u;

    for (var t: u32 = 0u; t < 80u; t = t + 1u) {
        var f: u32;
        var k: u32;
        if (t < 20u) {
            f = (b & c) | ((~b) & d);
            k = 0x5A827999u;
        } else if (t < 40u) {
            f = b ^ c ^ d;
            k = 0x6ED9EBA1u;
        } else if (t < 60u) {
            f = (b & c) | (b & d) | (c & d);
            k = 0x8F1BBCDCu;
        } else {
            f = b ^ c ^ d;
            k = 0xCA62C1D6u;
        }

        let temp = rotl32(a, 5u) + f + e + k + w[t];
        e = d;
        d = c;
        c = rotl32(b, 30u);
        b = a;
        a = temp;
    }

    let h0 = 0x67452301u + a;
    let h1 = 0xEFCDAB89u + b;
    let h2 = 0x98BADCFEu + c;
    let h3 = 0x10325476u + d;
    let h4 = 0xC3D2E1F0u + e;

    let b0 = (h0 >> 24u) & 0xFFu;
    let b1 = (h0 >> 16u) & 0xFFu;
    let b2 = (h0 >> 8u) & 0xFFu;
    let b3 = (h0 >> 0u) & 0xFFu;
    let b4 = (h1 >> 24u) & 0xFFu;
    let b5 = (h1 >> 16u) & 0xFFu;
    let b6 = (h1 >> 8u) & 0xFFu;
    let b7 = (h1 >> 0u) & 0xFFu;

    let seed0: u64 =
        (u64(b0) << 0u) |
        (u64(b1) << 8u) |
        (u64(b2) << 16u) |
        (u64(b3) << 24u) |
        (u64(b4) << 32u) |
        (u64(b5) << 40u) |
        (u64(b6) << 48u) |
        (u64(b7) << 56u);

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
