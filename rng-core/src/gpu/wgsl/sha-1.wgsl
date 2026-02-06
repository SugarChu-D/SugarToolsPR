// SHA-1 implementation based on rng-core/src/sha_1.rs
//
// Notes:
// - Requires SHADER_INT64 (uses u64 for seed0).
// - This shader expects GpuInput to provide:
//   - nazo[0..4]
//   - vcount_timer0_as_data5
//   - mac (u64)
//   - gxframe_xor_frame (already computed)
//   - date_as_data8
//   - timespec[0].x = time9 (u32)
//   - key_presses (u32)
//
// The message is 13 * u32 = 52 bytes, padded to a single 64-byte block.

struct GpuInput {
    nazo: array<u32, 5>,
    vcount_timer0_as_data5: u32,
    mac: u64,
    gxframe_xor_frame: u32,
    date_as_data8: u32,
    timespec: array<vec2<u32>, 3>,
    key_presses: u32,
    _pad0: u32,
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

@group(0) @binding(0)
var<storage, read> input_buf: InputBuffer;

@group(0) @binding(1)
var<storage, read_write> output_buf: OutputBuffer;

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

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i >= arrayLength(&input_buf.data)) {
        return;
    }

    let input = input_buf.data[i];
    let time9 = input.timespec[0].x;
    let key_presses = input.key_presses;

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

    let mac_lower_16: u32 = u32(input.mac & 0xFFFFu);
    write_be(&bytes, off, mac_lower_16); off = off + 4u;

    let gxframe_xor_frame_le = bswap32(input.gxframe_xor_frame);
    let mac_middle_16: u32 = u32((input.mac >> 16u) & 0xFFFF_FFFFu);
    let data7: u32 = gxframe_xor_frame_le ^ mac_middle_16;
    write_be(&bytes, off, data7); off = off + 4u;

    write_be(&bytes, off, input.date_as_data8); off = off + 4u;
    write_be(&bytes, off, time9); off = off + 4u;

    write_le(&bytes, off, 0u); off = off + 4u; // data10
    write_le(&bytes, off, 0u); off = off + 4u; // data11

    write_le(&bytes, off, key_presses); off = off + 4u; // data12

    // Padding: 52 bytes message
    bytes[52] = 0x80u;
    // bytes[53..55] already 0
    // length in bits = 52 * 8 = 416 = 0x00000000000001A0 (big-endian)
    bytes[56] = 0u;
    bytes[57] = 0u;
    bytes[58] = 0u;
    bytes[59] = 0u;
    bytes[60] = 0u;
    bytes[61] = 0u;
    bytes[62] = 0x01u;
    bytes[63] = 0xA0u;

    // Prepare message schedule
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

    // Initial hash values
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

    // Seed0 is u64 from little-endian bytes of digest[0..7]
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

    var out: GpuCandidate;
    out.seed0 = seed0;
    out.game_date = input.date_as_data8;
    out.game_time = time9;
    out.timer0 = input.vcount_timer0_as_data5;
    out.key_presses = key_presses;

    output_buf.data[i] = out;
}
