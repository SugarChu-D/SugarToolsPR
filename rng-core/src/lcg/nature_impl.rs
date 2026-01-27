use super::Lcg;

#[derive(Debug,Clone,PartialEq,Eq)]
pub enum Nature {
    Hardy = 0,
    Lonely = 1,
    Brave = 2,
    Adamant = 3,
    Naughty = 4,
    Bold = 5,
    Docile = 6,
    Relaxed = 7,
    Impish = 8,
    Lax = 9,
    Timid = 10,
    Hasty = 11,
    Serious = 12,
    Jolly = 13,
    Naive = 14,
    Modest = 15,
    Mild = 16,
    Quiet = 17,
    Bashful = 18,
    Rash = 19,
    Calm = 20,
    Gentle = 21,
    Sassy = 22,
    Careful = 23,
    Quirky = 24
}

// todo structに書き換える

impl Lcg {
    pub fn get_nature(&mut self) -> Nature {
        self.rand(25) as Nature
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    
    fn test_get_nature() {
        let mut seed = Lcg::new(0xf9d9dd91248eecb0);
        seed.advance(213);
        let nature = seed.get_nature();
        assert_eq!(nature, Nature::Naughty);
    }
}
