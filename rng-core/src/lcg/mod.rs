pub mod offset_impl;
pub mod TID_impl;
pub mod Nature_impl;
pub use offset_impl::OffsetType;

// lcg定数
const LCG_MULTIPLIER: u64 = 0x5D588B656C078965u64;
const LCG_INCREMENT: u64 = 0x269EC3u64;

#[derive(Clone, Copy)]
struct Mat {
    a11: u64, a12: u64,
    a21: u64, a22: u64,
}

impl Mat {
    fn mul(self, rhs: Mat) -> Mat {
        Mat {
            a11: self.a11.wrapping_mul(rhs.a11)
                .wrapping_add(self.a12.wrapping_mul(rhs.a21)),
            a12: self.a11.wrapping_mul(rhs.a12)
                .wrapping_add(self.a12.wrapping_mul(rhs.a22)),
            a21: self.a21.wrapping_mul(rhs.a11)
                .wrapping_add(self.a22.wrapping_mul(rhs.a21)),
            a22: self.a21.wrapping_mul(rhs.a12)
                .wrapping_add(self.a22.wrapping_mul(rhs.a22)),
        }
    }
}

fn mat_pow(mut base: Mat, mut exp: u64) -> Mat {
    let mut result = Mat {
        a11: 1, a12: 0,
        a21: 0, a22: 1,
    };

    while exp > 0 {
        if exp & 1 == 1 {
            result = result.mul(base);
        }
        base = base.mul(base);
        exp >>= 1;
    }
    result
}

#[derive(Clone, Copy, Debug)]
pub struct Lcg {
    pub state: u64,
    pub step: u64,
}

impl Lcg {
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
    
    pub fn advance(&mut self, n: u64) -> u64 {
        if n == 0 {
            return self.state;
        }

        let (mul, add) = Lcg::lcg_advance_params(LCG_MULTIPLIER, LCG_INCREMENT, n);
        self.state = self.state.wrapping_mul(mul).wrapping_add(add);
        self.step += n;
        self.state
    }

    /// x_n = mul * x_0 + add
    fn lcg_advance_params(a: u64, c: u64, n: u64) -> (u64, u64) {
        let m = Mat {
            a11: a, a12: c,
            a21: 0, a22: 1,
        };

        let p = mat_pow(m, n);
        (p.a11, p.a12)
    }

    pub fn rand(&mut self, max: u64) -> u32 {
        let current_state = self.state;
        let value = ((current_state >> 32)
        .wrapping_mul(max)
        >> 32) as u32;
        self.next();
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_lcg_next() {
        let mut seed = Lcg::new(0x9B3E7C4BC185AE31);
        assert_eq!(seed.next(), 0xA90C98ED53739118);
    }

    #[test]
    fn test_lcg_advance() {
        let mut seed = Lcg::new(0x9B3E7C4BC185AE31);
        assert_eq!(seed.advance(40), 0x20B7ACE1F983F819, "after 40 steps: {:X}", seed.state);
    }
}