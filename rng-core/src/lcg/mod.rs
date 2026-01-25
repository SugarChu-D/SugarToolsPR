pub mod offset_impl;
pub mod TID_impl;
pub use offset_impl::OffsetType;

// lcg定数
const LCG_MULTIPLIER: u64 = 0x5D588B656C078965u64;
const LCG_INCREMENT: u64 = 0x269EC3u64;

#[derive(Clone, Copy, Debug)]
pub struct lcg {
    pub state: u64,
    pub step: u64,
}
impl lcg {
    pub fn new(seed: u64) -> Self {
        Self { 
            state: seed,
            step: 0,
        }
    }

    pub fn next(&mut self) -> u64 {
        self.state = self.state.wrapping_mul(LCG_MULTIPLIER).wrapping_add(LCG_INCREMENT);
        self.step += 1;
        self.state
    }

    pub fn advance(&mut self, steps: u64) -> u64 {
        for _ in 0..steps {
            self.next();
        }
        self.state
    }

    /*
    fn calc_advance_params(a: u64, c: u64, n: u64) -> (u64, u64) {
        let mult = Self::pow_mod(a, n);
        let add = Self::calc_geometric_sum(a, c, n);
        (mult, add)
    }

    
    // 幾何級数和の計算: c * (a^0 + a^1 + ... + a^(n-1))
    fn calc_geometric_sum(a: u64, c: u64, mut k: u64) -> u64 {
        if k == 0 {
            return 0;
        }
    
        let mut sum = 0u64;
        let mut term = c;
        let mut power = 1u64;
    
        while k > 0 {
            if k & 1 == 1 {
                sum = sum.wrapping_add(term.wrapping_mul(power));
            }
            power = power.wrapping_add(power.wrapping_mul(a));  // power *= (1 + a)
            term = term.wrapping_mul(1u64.wrapping_add(a));     // term *= (1 + a)
            k >>= 1;
        }
    
        sum
    }

    fn pow_mod(mut base: u64, mut exp: u64) -> u64 {
        let mut result = 1u64;
        while exp > 0 {
            if exp & 1 == 1 {
                result = result.wrapping_mul(base);
            }
            base = base.wrapping_mul(base);
            exp >>= 1;
        }
        result
    }*/
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_lcg_next() {
        let mut lcg = lcg::new(0x9B3E7C4BC185AE31);
        assert_eq!(lcg.next(), 0xA90C98ED53739118);
    }

    #[test]
    fn test_lcg_advance() {
        let mut lcg = lcg::new(0x9B3E7C4BC185AE31);
        assert_eq!(lcg.advance(3), 0x8C9900BCDBC3B20A, "after 3 steps: {:X}", lcg.state);
    }
}