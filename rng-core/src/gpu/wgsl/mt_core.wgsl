const M: u32 = 397u;
const MAX_P: u32 = 20u;
const TABLE_SIZE: u32 = MAX_P + 6u + M;

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

fn seed_high_from_seed0(seed0: u64) -> u32 {
    let mult: u64 = (u64(LCG_MULTIPLIER_HI) << 32u) | u64(LCG_MULTIPLIER_LO);
    let inc: u64 = (u64(LCG_INCREMENT_HI) << 32u) | u64(LCG_INCREMENT_LO);
    let seed1: u64 = seed0 * mult + inc;
    return u32(seed1 >> 32u);
}

fn mt_matches_seed_high(seed_high: u32, p: u32, minv: array<u32, 6>, maxv: array<u32, 6>) -> bool {
    let init_range = p + 6u + M;
    var table: array<u32, 423>;
    init_table(&table, seed_high, init_range);
    let ivs = generate_ivs(&table, p);
    return ivs_in_range(ivs, minv, maxv);
}
