const KEY_RANGE_START: u32 = 0x2000u;
const KEY_RANGE_END: u32 = 0x2FFFu;
const KP_COUNT: u32 = KEY_RANGE_END - KEY_RANGE_START + 1u;

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

fn sha1_seed0(input: GpuInput, time9: u32, key_presses: u32) -> u64 {
    var bytes: array<u32, 64>;

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
    write_le(&bytes, off, 0u); off = off + 4u;
    write_le(&bytes, off, 0u); off = off + 4u;
    write_le(&bytes, off, key_presses); off = off + 4u;

    bytes[52u] = 0x80u;
    bytes[56u] = 0u; bytes[57u] = 0u; bytes[58u] = 0u; bytes[59u] = 0u;
    bytes[60u] = 0u; bytes[61u] = 0u; bytes[62u] = 0x01u; bytes[63u] = 0xA0u;

    var w: array<u32, 80>;
    w[0u] = (bytes[0u] << 24u) | (bytes[1u] << 16u) | (bytes[2u] << 8u) | bytes[3u];
    w[1u] = (bytes[4u] << 24u) | (bytes[5u] << 16u) | (bytes[6u] << 8u) | bytes[7u];
    w[2u] = (bytes[8u] << 24u) | (bytes[9u] << 16u) | (bytes[10u] << 8u) | bytes[11u];
    w[3u] = (bytes[12u] << 24u) | (bytes[13u] << 16u) | (bytes[14u] << 8u) | bytes[15u];
    w[4u] = (bytes[16u] << 24u) | (bytes[17u] << 16u) | (bytes[18u] << 8u) | bytes[19u];
    w[5u] = (bytes[20u] << 24u) | (bytes[21u] << 16u) | (bytes[22u] << 8u) | bytes[23u];
    w[6u] = (bytes[24u] << 24u) | (bytes[25u] << 16u) | (bytes[26u] << 8u) | bytes[27u];
    w[7u] = (bytes[28u] << 24u) | (bytes[29u] << 16u) | (bytes[30u] << 8u) | bytes[31u];
    w[8u] = (bytes[32u] << 24u) | (bytes[33u] << 16u) | (bytes[34u] << 8u) | bytes[35u];
    w[9u] = (bytes[36u] << 24u) | (bytes[37u] << 16u) | (bytes[38u] << 8u) | bytes[39u];
    w[10u] = (bytes[40u] << 24u) | (bytes[41u] << 16u) | (bytes[42u] << 8u) | bytes[43u];
    w[11u] = (bytes[44u] << 24u) | (bytes[45u] << 16u) | (bytes[46u] << 8u) | bytes[47u];
    w[12u] = (bytes[48u] << 24u) | (bytes[49u] << 16u) | (bytes[50u] << 8u) | bytes[51u];
    w[13u] = (bytes[52u] << 24u) | (bytes[53u] << 16u) | (bytes[54u] << 8u) | bytes[55u];
    w[14u] = (bytes[56u] << 24u) | (bytes[57u] << 16u) | (bytes[58u] << 8u) | bytes[59u];
    w[15u] = (bytes[60u] << 24u) | (bytes[61u] << 16u) | (bytes[62u] << 8u) | bytes[63u];
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

    let temp0 = rotl32(a, 5u) + ((b & c) | ((~b) & d)) + e + 0x5A827999u + w[0u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp0;

    let temp1 = rotl32(a, 5u) + ((b & c) | ((~b) & d)) + e + 0x5A827999u + w[1u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp1;

    let temp2 = rotl32(a, 5u) + ((b & c) | ((~b) & d)) + e + 0x5A827999u + w[2u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp2;

    let temp3 = rotl32(a, 5u) + ((b & c) | ((~b) & d)) + e + 0x5A827999u + w[3u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp3;

    let temp4 = rotl32(a, 5u) + ((b & c) | ((~b) & d)) + e + 0x5A827999u + w[4u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp4;

    let temp5 = rotl32(a, 5u) + ((b & c) | ((~b) & d)) + e + 0x5A827999u + w[5u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp5;

    let temp6 = rotl32(a, 5u) + ((b & c) | ((~b) & d)) + e + 0x5A827999u + w[6u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp6;

    let temp7 = rotl32(a, 5u) + ((b & c) | ((~b) & d)) + e + 0x5A827999u + w[7u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp7;

    let temp8 = rotl32(a, 5u) + ((b & c) | ((~b) & d)) + e + 0x5A827999u + w[8u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp8;

    let temp9 = rotl32(a, 5u) + ((b & c) | ((~b) & d)) + e + 0x5A827999u + w[9u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp9;

    let temp10 = rotl32(a, 5u) + ((b & c) | ((~b) & d)) + e + 0x5A827999u + w[10u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp10;

    let temp11 = rotl32(a, 5u) + ((b & c) | ((~b) & d)) + e + 0x5A827999u + w[11u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp11;

    let temp12 = rotl32(a, 5u) + ((b & c) | ((~b) & d)) + e + 0x5A827999u + w[12u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp12;

    let temp13 = rotl32(a, 5u) + ((b & c) | ((~b) & d)) + e + 0x5A827999u + w[13u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp13;

    let temp14 = rotl32(a, 5u) + ((b & c) | ((~b) & d)) + e + 0x5A827999u + w[14u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp14;

    let temp15 = rotl32(a, 5u) + ((b & c) | ((~b) & d)) + e + 0x5A827999u + w[15u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp15;

    let temp16 = rotl32(a, 5u) + ((b & c) | ((~b) & d)) + e + 0x5A827999u + w[16u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp16;

    let temp17 = rotl32(a, 5u) + ((b & c) | ((~b) & d)) + e + 0x5A827999u + w[17u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp17;

    let temp18 = rotl32(a, 5u) + ((b & c) | ((~b) & d)) + e + 0x5A827999u + w[18u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp18;

    let temp19 = rotl32(a, 5u) + ((b & c) | ((~b) & d)) + e + 0x5A827999u + w[19u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp19;

    let temp20 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0x6ED9EBA1u + w[20u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp20;

    let temp21 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0x6ED9EBA1u + w[21u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp21;

    let temp22 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0x6ED9EBA1u + w[22u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp22;

    let temp23 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0x6ED9EBA1u + w[23u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp23;

    let temp24 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0x6ED9EBA1u + w[24u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp24;

    let temp25 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0x6ED9EBA1u + w[25u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp25;

    let temp26 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0x6ED9EBA1u + w[26u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp26;

    let temp27 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0x6ED9EBA1u + w[27u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp27;

    let temp28 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0x6ED9EBA1u + w[28u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp28;

    let temp29 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0x6ED9EBA1u + w[29u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp29;

    let temp30 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0x6ED9EBA1u + w[30u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp30;

    let temp31 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0x6ED9EBA1u + w[31u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp31;

    let temp32 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0x6ED9EBA1u + w[32u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp32;

    let temp33 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0x6ED9EBA1u + w[33u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp33;

    let temp34 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0x6ED9EBA1u + w[34u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp34;

    let temp35 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0x6ED9EBA1u + w[35u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp35;

    let temp36 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0x6ED9EBA1u + w[36u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp36;

    let temp37 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0x6ED9EBA1u + w[37u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp37;

    let temp38 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0x6ED9EBA1u + w[38u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp38;

    let temp39 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0x6ED9EBA1u + w[39u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp39;

    let temp40 = rotl32(a, 5u) + ((b & c) | (b & d) | (c & d)) + e + 0x8F1BBCDCu + w[40u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp40;

    let temp41 = rotl32(a, 5u) + ((b & c) | (b & d) | (c & d)) + e + 0x8F1BBCDCu + w[41u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp41;

    let temp42 = rotl32(a, 5u) + ((b & c) | (b & d) | (c & d)) + e + 0x8F1BBCDCu + w[42u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp42;

    let temp43 = rotl32(a, 5u) + ((b & c) | (b & d) | (c & d)) + e + 0x8F1BBCDCu + w[43u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp43;

    let temp44 = rotl32(a, 5u) + ((b & c) | (b & d) | (c & d)) + e + 0x8F1BBCDCu + w[44u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp44;

    let temp45 = rotl32(a, 5u) + ((b & c) | (b & d) | (c & d)) + e + 0x8F1BBCDCu + w[45u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp45;

    let temp46 = rotl32(a, 5u) + ((b & c) | (b & d) | (c & d)) + e + 0x8F1BBCDCu + w[46u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp46;

    let temp47 = rotl32(a, 5u) + ((b & c) | (b & d) | (c & d)) + e + 0x8F1BBCDCu + w[47u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp47;

    let temp48 = rotl32(a, 5u) + ((b & c) | (b & d) | (c & d)) + e + 0x8F1BBCDCu + w[48u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp48;

    let temp49 = rotl32(a, 5u) + ((b & c) | (b & d) | (c & d)) + e + 0x8F1BBCDCu + w[49u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp49;

    let temp50 = rotl32(a, 5u) + ((b & c) | (b & d) | (c & d)) + e + 0x8F1BBCDCu + w[50u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp50;

    let temp51 = rotl32(a, 5u) + ((b & c) | (b & d) | (c & d)) + e + 0x8F1BBCDCu + w[51u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp51;

    let temp52 = rotl32(a, 5u) + ((b & c) | (b & d) | (c & d)) + e + 0x8F1BBCDCu + w[52u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp52;

    let temp53 = rotl32(a, 5u) + ((b & c) | (b & d) | (c & d)) + e + 0x8F1BBCDCu + w[53u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp53;

    let temp54 = rotl32(a, 5u) + ((b & c) | (b & d) | (c & d)) + e + 0x8F1BBCDCu + w[54u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp54;

    let temp55 = rotl32(a, 5u) + ((b & c) | (b & d) | (c & d)) + e + 0x8F1BBCDCu + w[55u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp55;

    let temp56 = rotl32(a, 5u) + ((b & c) | (b & d) | (c & d)) + e + 0x8F1BBCDCu + w[56u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp56;

    let temp57 = rotl32(a, 5u) + ((b & c) | (b & d) | (c & d)) + e + 0x8F1BBCDCu + w[57u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp57;

    let temp58 = rotl32(a, 5u) + ((b & c) | (b & d) | (c & d)) + e + 0x8F1BBCDCu + w[58u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp58;

    let temp59 = rotl32(a, 5u) + ((b & c) | (b & d) | (c & d)) + e + 0x8F1BBCDCu + w[59u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp59;

    let temp60 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0xCA62C1D6u + w[60u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp60;

    let temp61 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0xCA62C1D6u + w[61u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp61;

    let temp62 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0xCA62C1D6u + w[62u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp62;

    let temp63 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0xCA62C1D6u + w[63u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp63;

    let temp64 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0xCA62C1D6u + w[64u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp64;

    let temp65 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0xCA62C1D6u + w[65u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp65;

    let temp66 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0xCA62C1D6u + w[66u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp66;

    let temp67 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0xCA62C1D6u + w[67u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp67;

    let temp68 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0xCA62C1D6u + w[68u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp68;

    let temp69 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0xCA62C1D6u + w[69u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp69;

    let temp70 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0xCA62C1D6u + w[70u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp70;

    let temp71 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0xCA62C1D6u + w[71u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp71;

    let temp72 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0xCA62C1D6u + w[72u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp72;

    let temp73 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0xCA62C1D6u + w[73u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp73;

    let temp74 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0xCA62C1D6u + w[74u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp74;

    let temp75 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0xCA62C1D6u + w[75u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp75;

    let temp76 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0xCA62C1D6u + w[76u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp76;

    let temp77 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0xCA62C1D6u + w[77u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp77;

    let temp78 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0xCA62C1D6u + w[78u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp78;

    let temp79 = rotl32(a, 5u) + (b ^ c ^ d) + e + 0xCA62C1D6u + w[79u];
    e = d;
    d = c;
    c = rotl32(b, 30u);
    b = a;
    a = temp79;

    let h0 = 0x67452301u + a;
    let h1 = 0xEFCDAB89u + b;

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
    return seed0;
}
