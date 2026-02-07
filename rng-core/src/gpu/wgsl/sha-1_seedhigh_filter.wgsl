// SHA-1 with seed1_high filtering (binary search in candidate list)
// Writes only matching candidates into output buffer using atomic counter.

const KEY_RANGE_START: u32 = 0x2000u;

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

struct ListBuffer {
    data: array<u32>,
}

struct KeypressBuffer {
    data: array<u32>,
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
    list_len: u32,
    keypress_len: u32,
}

@group(0) @binding(0)
var<storage, read> input_buf: InputBuffer;

@group(0) @binding(1)
var<storage, read> list_buf: ListBuffer;

@group(0) @binding(2)
var<storage, read> keypress_buf: KeypressBuffer;

@group(0) @binding(3)
var<storage, read_write> output_buf: OutputBuffer;

@group(0) @binding(4)
var<storage, read_write> counter_buf: CounterBuffer;

@group(0) @binding(5)
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

fn time9_from_hms(hour: u32, minute: u32, second: u32) -> u32 {
    let adjusted_hour = select(hour, hour + 40u, hour >= 12u);
    let hex_hour = ((adjusted_hour / 10u) << 4u) | (adjusted_hour % 10u);
    let hex_min = ((minute / 10u) << 4u) | (minute % 10u);
    let hex_sec = ((second / 10u) << 4u) | (second % 10u);
    return (hex_hour << 24u) | (hex_min << 16u) | (hex_sec << 8u);
}

fn list_contains(list_len: u32, value: u32) -> bool {
    var lo: u32 = 0u;
    var hi: u32 = list_len;
    loop {
        if (lo >= hi) { break; }
        let mid = (lo + hi) >> 1u;
        let v = list_buf.data[mid];
        if (v == value) {
            return true;
        }
        if (v < value) {
            lo = mid + 1u;
        } else {
            hi = mid;
        }
    }
    return false;
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let global = params.base_index + u64(gid.x);
    if (global >= params.total_len) { return; }
    let input_len = arrayLength(&input_buf.data);
    if (input_len == 0u) { return; }
    if (params.list_len == 0u) { return; }

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
    let kp_count = params.keypress_len;
    if (kp_count == 0u) { return; }
    let per_input = u64(kp_count) * u64(time_count);

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

    let key_presses = keypress_buf.data[u32(kp_idx)];
    let input = input_buf.data[u32(input_idx)];

    let mac_lower_16: u32 = u32(input.mac & u64(0xFFFFu));
    let gxframe_xor_frame_le = bswap32(input.gxframe_xor_frame);
    let mac_middle_16: u32 = u32((input.mac >> 16u) & u64(0xFFFFFFFFu));
    let data7: u32 = gxframe_xor_frame_le ^ mac_middle_16;

    var w: array<u32, 80>;
    w[0] = bswap32(input.nazo[0]);
    w[1] = bswap32(input.nazo[1]);
    w[2] = bswap32(input.nazo[2]);
    w[3] = bswap32(input.nazo[3]);
    w[4] = bswap32(input.nazo[4]);
    w[5] = bswap32(input.vcount_timer0_as_data5);
    w[6] = mac_lower_16;
    w[7] = data7;
    w[8] = input.date_as_data8;
    w[9] = time9;
    w[10] = 0u;
    w[11] = 0u;
    w[12] = bswap32(key_presses);
    w[13] = 0x80000000u;
    w[14] = 0u;
    w[15] = 0x000001A0u;
    w[16u] = rotl32(w[13u] ^ w[8u] ^ w[2u] ^ w[0u], 1u);
    w[17u] = rotl32(w[14u] ^ w[9u] ^ w[3u] ^ w[1u], 1u);
    w[18u] = rotl32(w[15u] ^ w[10u] ^ w[4u] ^ w[2u], 1u);
    w[19u] = rotl32(w[16u] ^ w[11u] ^ w[5u] ^ w[3u], 1u);
    w[20u] = rotl32(w[17u] ^ w[12u] ^ w[6u] ^ w[4u], 1u);
    w[21u] = rotl32(w[18u] ^ w[13u] ^ w[7u] ^ w[5u], 1u);
    w[22u] = rotl32(w[19u] ^ w[14u] ^ w[8u] ^ w[6u], 1u);
    w[23u] = rotl32(w[20u] ^ w[15u] ^ w[9u] ^ w[7u], 1u);
    w[24u] = rotl32(w[21u] ^ w[16u] ^ w[10u] ^ w[8u], 1u);
    w[25u] = rotl32(w[22u] ^ w[17u] ^ w[11u] ^ w[9u], 1u);
    w[26u] = rotl32(w[23u] ^ w[18u] ^ w[12u] ^ w[10u], 1u);
    w[27u] = rotl32(w[24u] ^ w[19u] ^ w[13u] ^ w[11u], 1u);
    w[28u] = rotl32(w[25u] ^ w[20u] ^ w[14u] ^ w[12u], 1u);
    w[29u] = rotl32(w[26u] ^ w[21u] ^ w[15u] ^ w[13u], 1u);
    w[30u] = rotl32(w[27u] ^ w[22u] ^ w[16u] ^ w[14u], 1u);
    w[31u] = rotl32(w[28u] ^ w[23u] ^ w[17u] ^ w[15u], 1u);
    w[32u] = rotl32(w[29u] ^ w[24u] ^ w[18u] ^ w[16u], 1u);
    w[33u] = rotl32(w[30u] ^ w[25u] ^ w[19u] ^ w[17u], 1u);
    w[34u] = rotl32(w[31u] ^ w[26u] ^ w[20u] ^ w[18u], 1u);
    w[35u] = rotl32(w[32u] ^ w[27u] ^ w[21u] ^ w[19u], 1u);
    w[36u] = rotl32(w[33u] ^ w[28u] ^ w[22u] ^ w[20u], 1u);
    w[37u] = rotl32(w[34u] ^ w[29u] ^ w[23u] ^ w[21u], 1u);
    w[38u] = rotl32(w[35u] ^ w[30u] ^ w[24u] ^ w[22u], 1u);
    w[39u] = rotl32(w[36u] ^ w[31u] ^ w[25u] ^ w[23u], 1u);
    w[40u] = rotl32(w[37u] ^ w[32u] ^ w[26u] ^ w[24u], 1u);
    w[41u] = rotl32(w[38u] ^ w[33u] ^ w[27u] ^ w[25u], 1u);
    w[42u] = rotl32(w[39u] ^ w[34u] ^ w[28u] ^ w[26u], 1u);
    w[43u] = rotl32(w[40u] ^ w[35u] ^ w[29u] ^ w[27u], 1u);
    w[44u] = rotl32(w[41u] ^ w[36u] ^ w[30u] ^ w[28u], 1u);
    w[45u] = rotl32(w[42u] ^ w[37u] ^ w[31u] ^ w[29u], 1u);
    w[46u] = rotl32(w[43u] ^ w[38u] ^ w[32u] ^ w[30u], 1u);
    w[47u] = rotl32(w[44u] ^ w[39u] ^ w[33u] ^ w[31u], 1u);
    w[48u] = rotl32(w[45u] ^ w[40u] ^ w[34u] ^ w[32u], 1u);
    w[49u] = rotl32(w[46u] ^ w[41u] ^ w[35u] ^ w[33u], 1u);
    w[50u] = rotl32(w[47u] ^ w[42u] ^ w[36u] ^ w[34u], 1u);
    w[51u] = rotl32(w[48u] ^ w[43u] ^ w[37u] ^ w[35u], 1u);
    w[52u] = rotl32(w[49u] ^ w[44u] ^ w[38u] ^ w[36u], 1u);
    w[53u] = rotl32(w[50u] ^ w[45u] ^ w[39u] ^ w[37u], 1u);
    w[54u] = rotl32(w[51u] ^ w[46u] ^ w[40u] ^ w[38u], 1u);
    w[55u] = rotl32(w[52u] ^ w[47u] ^ w[41u] ^ w[39u], 1u);
    w[56u] = rotl32(w[53u] ^ w[48u] ^ w[42u] ^ w[40u], 1u);
    w[57u] = rotl32(w[54u] ^ w[49u] ^ w[43u] ^ w[41u], 1u);
    w[58u] = rotl32(w[55u] ^ w[50u] ^ w[44u] ^ w[42u], 1u);
    w[59u] = rotl32(w[56u] ^ w[51u] ^ w[45u] ^ w[43u], 1u);
    w[60u] = rotl32(w[57u] ^ w[52u] ^ w[46u] ^ w[44u], 1u);
    w[61u] = rotl32(w[58u] ^ w[53u] ^ w[47u] ^ w[45u], 1u);
    w[62u] = rotl32(w[59u] ^ w[54u] ^ w[48u] ^ w[46u], 1u);
    w[63u] = rotl32(w[60u] ^ w[55u] ^ w[49u] ^ w[47u], 1u);
    w[64u] = rotl32(w[61u] ^ w[56u] ^ w[50u] ^ w[48u], 1u);
    w[65u] = rotl32(w[62u] ^ w[57u] ^ w[51u] ^ w[49u], 1u);
    w[66u] = rotl32(w[63u] ^ w[58u] ^ w[52u] ^ w[50u], 1u);
    w[67u] = rotl32(w[64u] ^ w[59u] ^ w[53u] ^ w[51u], 1u);
    w[68u] = rotl32(w[65u] ^ w[60u] ^ w[54u] ^ w[52u], 1u);
    w[69u] = rotl32(w[66u] ^ w[61u] ^ w[55u] ^ w[53u], 1u);
    w[70u] = rotl32(w[67u] ^ w[62u] ^ w[56u] ^ w[54u], 1u);
    w[71u] = rotl32(w[68u] ^ w[63u] ^ w[57u] ^ w[55u], 1u);
    w[72u] = rotl32(w[69u] ^ w[64u] ^ w[58u] ^ w[56u], 1u);
    w[73u] = rotl32(w[70u] ^ w[65u] ^ w[59u] ^ w[57u], 1u);
    w[74u] = rotl32(w[71u] ^ w[66u] ^ w[60u] ^ w[58u], 1u);
    w[75u] = rotl32(w[72u] ^ w[67u] ^ w[61u] ^ w[59u], 1u);
    w[76u] = rotl32(w[73u] ^ w[68u] ^ w[62u] ^ w[60u], 1u);
    w[77u] = rotl32(w[74u] ^ w[69u] ^ w[63u] ^ w[61u], 1u);
    w[78u] = rotl32(w[75u] ^ w[70u] ^ w[64u] ^ w[62u], 1u);
    w[79u] = rotl32(w[76u] ^ w[71u] ^ w[65u] ^ w[63u], 1u);

    var a: u32 = 0x67452301u;
    var b: u32 = 0xEFCDAB89u;
    var c: u32 = 0x98BADCFEu;
    var d: u32 = 0x10325476u;
    var e: u32 = 0xC3D2E1F0u;

    // t=0
    let temp_0 = rotl32(a, 5u) + ((b & c) | ((~b) & d)) + e + 0x5A827999u + w[0u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_0;
    // t=1
    let temp_1 = rotl32(a, 5u) + ((b & c) | ((~b) & d)) + e + 0x5A827999u + w[1u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_1;
    // t=2
    let temp_2 = rotl32(a, 5u) + ((b & c) | ((~b) & d)) + e + 0x5A827999u + w[2u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_2;
    // t=3
    let temp_3 = rotl32(a, 5u) + ((b & c) | ((~b) & d)) + e + 0x5A827999u + w[3u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_3;
    // t=4
    let temp_4 = rotl32(a, 5u) + ((b & c) | ((~b) & d)) + e + 0x5A827999u + w[4u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_4;
    // t=5
    let temp_5 = rotl32(a, 5u) + ((b & c) | ((~b) & d)) + e + 0x5A827999u + w[5u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_5;
    // t=6
    let temp_6 = rotl32(a, 5u) + ((b & c) | ((~b) & d)) + e + 0x5A827999u + w[6u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_6;
    // t=7
    let temp_7 = rotl32(a, 5u) + ((b & c) | ((~b) & d)) + e + 0x5A827999u + w[7u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_7;
    // t=8
    let temp_8 = rotl32(a, 5u) + ((b & c) | ((~b) & d)) + e + 0x5A827999u + w[8u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_8;
    // t=9
    let temp_9 = rotl32(a, 5u) + ((b & c) | ((~b) & d)) + e + 0x5A827999u + w[9u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_9;
    // t=10
    let temp_10 = rotl32(a, 5u) + ((b & c) | ((~b) & d)) + e + 0x5A827999u + w[10u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_10;
    // t=11
    let temp_11 = rotl32(a, 5u) + ((b & c) | ((~b) & d)) + e + 0x5A827999u + w[11u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_11;
    // t=12
    let temp_12 = rotl32(a, 5u) + ((b & c) | ((~b) & d)) + e + 0x5A827999u + w[12u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_12;
    // t=13
    let temp_13 = rotl32(a, 5u) + ((b & c) | ((~b) & d)) + e + 0x5A827999u + w[13u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_13;
    // t=14
    let temp_14 = rotl32(a, 5u) + ((b & c) | ((~b) & d)) + e + 0x5A827999u + w[14u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_14;
    // t=15
    let temp_15 = rotl32(a, 5u) + ((b & c) | ((~b) & d)) + e + 0x5A827999u + w[15u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_15;
    // t=16
    let temp_16 = rotl32(a, 5u) + ((b & c) | ((~b) & d)) + e + 0x5A827999u + w[16u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_16;
    // t=17
    let temp_17 = rotl32(a, 5u) + ((b & c) | ((~b) & d)) + e + 0x5A827999u + w[17u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_17;
    // t=18
    let temp_18 = rotl32(a, 5u) + ((b & c) | ((~b) & d)) + e + 0x5A827999u + w[18u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_18;
    // t=19
    let temp_19 = rotl32(a, 5u) + ((b & c) | ((~b) & d)) + e + 0x5A827999u + w[19u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_19;
    // t=20
    let temp_20 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0x6ED9EBA1u + w[20u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_20;
    // t=21
    let temp_21 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0x6ED9EBA1u + w[21u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_21;
    // t=22
    let temp_22 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0x6ED9EBA1u + w[22u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_22;
    // t=23
    let temp_23 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0x6ED9EBA1u + w[23u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_23;
    // t=24
    let temp_24 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0x6ED9EBA1u + w[24u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_24;
    // t=25
    let temp_25 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0x6ED9EBA1u + w[25u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_25;
    // t=26
    let temp_26 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0x6ED9EBA1u + w[26u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_26;
    // t=27
    let temp_27 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0x6ED9EBA1u + w[27u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_27;
    // t=28
    let temp_28 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0x6ED9EBA1u + w[28u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_28;
    // t=29
    let temp_29 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0x6ED9EBA1u + w[29u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_29;
    // t=30
    let temp_30 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0x6ED9EBA1u + w[30u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_30;
    // t=31
    let temp_31 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0x6ED9EBA1u + w[31u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_31;
    // t=32
    let temp_32 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0x6ED9EBA1u + w[32u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_32;
    // t=33
    let temp_33 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0x6ED9EBA1u + w[33u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_33;
    // t=34
    let temp_34 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0x6ED9EBA1u + w[34u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_34;
    // t=35
    let temp_35 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0x6ED9EBA1u + w[35u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_35;
    // t=36
    let temp_36 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0x6ED9EBA1u + w[36u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_36;
    // t=37
    let temp_37 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0x6ED9EBA1u + w[37u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_37;
    // t=38
    let temp_38 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0x6ED9EBA1u + w[38u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_38;
    // t=39
    let temp_39 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0x6ED9EBA1u + w[39u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_39;
    // t=40
    let temp_40 = rotl32(a, 5u) + ((b & c) | (b & d) | (c & d)) + e + 0x8F1BBCDCu + w[40u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_40;
    // t=41
    let temp_41 = rotl32(a, 5u) + ((b & c) | (b & d) | (c & d)) + e + 0x8F1BBCDCu + w[41u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_41;
    // t=42
    let temp_42 = rotl32(a, 5u) + ((b & c) | (b & d) | (c & d)) + e + 0x8F1BBCDCu + w[42u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_42;
    // t=43
    let temp_43 = rotl32(a, 5u) + ((b & c) | (b & d) | (c & d)) + e + 0x8F1BBCDCu + w[43u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_43;
    // t=44
    let temp_44 = rotl32(a, 5u) + ((b & c) | (b & d) | (c & d)) + e + 0x8F1BBCDCu + w[44u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_44;
    // t=45
    let temp_45 = rotl32(a, 5u) + ((b & c) | (b & d) | (c & d)) + e + 0x8F1BBCDCu + w[45u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_45;
    // t=46
    let temp_46 = rotl32(a, 5u) + ((b & c) | (b & d) | (c & d)) + e + 0x8F1BBCDCu + w[46u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_46;
    // t=47
    let temp_47 = rotl32(a, 5u) + ((b & c) | (b & d) | (c & d)) + e + 0x8F1BBCDCu + w[47u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_47;
    // t=48
    let temp_48 = rotl32(a, 5u) + ((b & c) | (b & d) | (c & d)) + e + 0x8F1BBCDCu + w[48u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_48;
    // t=49
    let temp_49 = rotl32(a, 5u) + ((b & c) | (b & d) | (c & d)) + e + 0x8F1BBCDCu + w[49u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_49;
    // t=50
    let temp_50 = rotl32(a, 5u) + ((b & c) | (b & d) | (c & d)) + e + 0x8F1BBCDCu + w[50u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_50;
    // t=51
    let temp_51 = rotl32(a, 5u) + ((b & c) | (b & d) | (c & d)) + e + 0x8F1BBCDCu + w[51u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_51;
    // t=52
    let temp_52 = rotl32(a, 5u) + ((b & c) | (b & d) | (c & d)) + e + 0x8F1BBCDCu + w[52u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_52;
    // t=53
    let temp_53 = rotl32(a, 5u) + ((b & c) | (b & d) | (c & d)) + e + 0x8F1BBCDCu + w[53u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_53;
    // t=54
    let temp_54 = rotl32(a, 5u) + ((b & c) | (b & d) | (c & d)) + e + 0x8F1BBCDCu + w[54u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_54;
    // t=55
    let temp_55 = rotl32(a, 5u) + ((b & c) | (b & d) | (c & d)) + e + 0x8F1BBCDCu + w[55u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_55;
    // t=56
    let temp_56 = rotl32(a, 5u) + ((b & c) | (b & d) | (c & d)) + e + 0x8F1BBCDCu + w[56u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_56;
    // t=57
    let temp_57 = rotl32(a, 5u) + ((b & c) | (b & d) | (c & d)) + e + 0x8F1BBCDCu + w[57u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_57;
    // t=58
    let temp_58 = rotl32(a, 5u) + ((b & c) | (b & d) | (c & d)) + e + 0x8F1BBCDCu + w[58u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_58;
    // t=59
    let temp_59 = rotl32(a, 5u) + ((b & c) | (b & d) | (c & d)) + e + 0x8F1BBCDCu + w[59u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_59;
    // t=60
    let temp_60 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0xCA62C1D6u + w[60u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_60;
    // t=61
    let temp_61 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0xCA62C1D6u + w[61u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_61;
    // t=62
    let temp_62 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0xCA62C1D6u + w[62u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_62;
    // t=63
    let temp_63 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0xCA62C1D6u + w[63u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_63;
    // t=64
    let temp_64 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0xCA62C1D6u + w[64u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_64;
    // t=65
    let temp_65 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0xCA62C1D6u + w[65u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_65;
    // t=66
    let temp_66 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0xCA62C1D6u + w[66u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_66;
    // t=67
    let temp_67 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0xCA62C1D6u + w[67u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_67;
    // t=68
    let temp_68 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0xCA62C1D6u + w[68u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_68;
    // t=69
    let temp_69 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0xCA62C1D6u + w[69u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_69;
    // t=70
    let temp_70 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0xCA62C1D6u + w[70u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_70;
    // t=71
    let temp_71 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0xCA62C1D6u + w[71u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_71;
    // t=72
    let temp_72 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0xCA62C1D6u + w[72u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_72;
    // t=73
    let temp_73 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0xCA62C1D6u + w[73u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_73;
    // t=74
    let temp_74 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0xCA62C1D6u + w[74u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_74;
    // t=75
    let temp_75 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0xCA62C1D6u + w[75u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_75;
    // t=76
    let temp_76 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0xCA62C1D6u + w[76u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_76;
    // t=77
    let temp_77 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0xCA62C1D6u + w[77u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_77;
    // t=78
    let temp_78 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0xCA62C1D6u + w[78u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_78;
    // t=79
    let temp_79 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0xCA62C1D6u + w[79u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp_79;

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

    if (list_contains(params.list_len, seed_high)) {
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
