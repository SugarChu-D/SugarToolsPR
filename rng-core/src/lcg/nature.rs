use super::Lcg;

#[derive(Debug,Clone,PartialEq,Eq)]
pub struct Nature(u8);

const NATURE_NAMES: [&str; 25] = [
    "Hardy", "Lonely", "Brave", "Adamant", "Naughty",
    "Bold", "Docile", "Relaxed", "Impish", "Lax",
    "Timid", "Hasty", "Serious", "Jolly", "Naive",
    "Modest", "Mild", "Quiet", "Bashful", "Rash",
    "Calm", "Gentle", "Sassy", "Careful", "Quirky",
];


impl Nature {
    pub const MAX: u8 = 25;

    pub fn new (v: u8) -> Self {
        Nature(v)
    }

    pub fn id(&self) -> u8 {
        self.0
    }

    pub fn name(&self) -> &'static str {
        NATURE_NAMES[self.0 as usize]
    }
}

impl Lcg {
    pub fn get_nature(&mut self) -> Nature {
        Nature::new(self.rand(25) as u8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_nature() {
        let mut seed = Lcg::new(0xf9d9dd91248eecb0);
        seed.advance(213);
        let nature = seed.get_nature();
        assert_eq!(nature, Nature(4));
    }
}
