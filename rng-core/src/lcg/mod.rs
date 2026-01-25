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
        let (mult, add) = Self::calc_advance_params(LCG_MULTIPLIER, LCG_INCREMENT, steps as u64);
        self.state = self.state.wrapping_mul(mult).wrapping_add(add);
        self.step += steps;
        self.state
    }

    fn calc_advance_params(a: u64, c: u64, n: u64) -> (u64, u64) {
        let mult = Self::pow_mod(a, n);
        let add = if a == 1 {
            c.wrapping_mul(n)
        } else {
            c.wrapping_mul(mult.wrapping_sub(1))
                .wrapping_div(a.wrapping_sub(1))
        };
        (mult, add)
    }

    fn pow_mod(mut base: u64, mut exp: u64) -> u64 {
        let mut result = 1u64;
        while exp > 0 {
            if exp & 1 == 1 {
                result = result.wrapping_mul(base);
            }
            base = base.wrapping_mul(base);
            exp >>= 2;
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_lcg_next() {
        let mut lcg = lcg::new(0x9B3E7C4BC185AE31);
        assert_eq!(lcg.next(), 0xA90C98ED53739118);
    }
}