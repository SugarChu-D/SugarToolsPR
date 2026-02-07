use crate::lcg::nature::Nature;

use super::Lcg;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WildPoke {
    pub slot: Option<u32>,
    pub poke_code: Option<u32>,
    pub nature: Option<Nature>,
    pub item: Option<u32>,
}

impl Lcg {
    pub fn get_wild_poke(&mut self) -> WildPoke {
        let mut lcg_local = self.clone();
        if lcg_local.rand(100) < 20 {
            return WildPoke::default();
        }
        lcg_local.next();
        let mut result = WildPoke::default();
        result.slot = Some(lcg_local.rand(100));
        lcg_local.next();
        result.poke_code = Some((lcg_local.next() >> 32) as u32);
        result.nature = Some(lcg_local.get_nature());
        result.item = Some(lcg_local.rand(100));
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_pup() {
        let mut seed = Lcg::new(0x45758423BB8FCDB8);
        seed.advance(83);
        let pup = seed.get_wild_poke();

        println!(
            "slot={:?}, poke_code={:?}, nature={:?}, item={:?}",
            pup.slot, pup.poke_code, pup.nature, pup.item
        );
    }
}
