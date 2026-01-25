// MT19937の定数
const M: usize = 397;
const MAX_P: usize = 20; // pの最大値 まれに変わるかもしれない
const TABLE_SIZE: usize = MAX_P + 6 + M;

// マスク定数
const UPPER_MASK: u32 = 0x80000000;
const LOWER_MASK: u32 = 0x7fffffff;
const MATRIX_A: u32 = 0x9908B0DF;

// テンパリング定数
const TEMPERING_MASK_B: u32 = 0x9D2C5680;
const TEMPERING_MASK_C: u32 = 0xEFC60000;

// 初期化用定数
const INIT_MULTIPLIER: u32 = 1812433253u32;

use crate::LCG::LCG;

// テンパリング処理
fn tempering(mut val: u32) -> u8 {
    val ^= val >> 11;
    val ^= (val << 7) & TEMPERING_MASK_B;
    val ^= (val << 15) & TEMPERING_MASK_C;
    val ^= val >> 18;
    ((val >> 27) & 0xFF) as u8
}

// テーブルの初期化
fn init_table(table: &mut [u32], seed: u32, init_range: usize) {
    table[0] = seed;
    
    let mut prev = table[0];
    for i in 1..=init_range {
        prev = INIT_MULTIPLIER
            .wrapping_mul(prev ^ (prev >> 30))
            .wrapping_add(i as u32);
        table[i] = prev;
    }
}

// IVSコード生成（テーブルとpから）
fn generate_ivs_code(table: &[u32], p: u8) -> [u8; 6] {
    let mut ivs_values = [0u8; 6];
    
    // メインループ: p から p+6 まで（6回）
    for j in 0..6 {
        let i = p as usize + j;
        
        let x = (table[i] & UPPER_MASK) | (table[i + 1] & LOWER_MASK);
        let x_a = (x >> 1) ^ (if x & 1 != 0 { MATRIX_A } else { 0 });
        let val = table[i + M] ^ x_a;
        
        // テンパリング処理
        let tempered = tempering(val);
        
        ivs_values[j] = tempered;
    }
    
    ivs_values
}

/// MT_1関数: seed1とpから6つの値を配列で生成
pub fn mt_1(seed1: u64, p: u8) -> [u8; 6] {
    let mut table = vec![0u32; TABLE_SIZE];
    let seed_high = (seed1 >> 32) as u32;
    
    init_table(&mut table, seed_high, p as usize + 6 + M);
    generate_ivs_code(&table, p)
}

/// MT_0関数: seed0からseed1をLCGで生成してMT_1を呼ぶ
pub fn mt_0(seed0: u64, p: u8) -> [u8; 6] {
    let seed1 = LCG::new(seed0).next();
    mt_1(seed1, p)
}

/// MT_32関数: 32ビットシードから6つの値を配列で生成
pub fn mt_32(seed: u32, p: u8) -> [u8; 6] {
    let mut table = vec![0u32; TABLE_SIZE];
    
    init_table(&mut table, seed, p as usize + 6 + M);
    generate_ivs_code(&table, p)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_mt_1() {
        let result = mt_0(0x9B3E7C4BC185AE31u64, 5);
        assert_eq!(result, [31, 19, 31, 31, 31, 31]);
    }
}