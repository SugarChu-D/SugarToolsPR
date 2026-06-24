use crate::lcg::nature::Nature;

use super::{Lcg, OffsetType};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WildPoke {
    pub slot: Option<u32>,
    pub poke_code: Option<u32>,
    pub nature: Option<Nature>,
    pub item: Option<u32>,
}

impl WildPoke {
    pub fn ability(&self) -> Option<u8>{
        if self.poke_code.is_none() {return None}
        Some((((self.poke_code.unwrap() >> 16)^ 1) % 2) as u8)
    }

    pub fn gender(&self) -> Option<u8> {
        if self.poke_code.is_none() {return None}
        Some((self.poke_code.unwrap() & 0xFF) as u8)
    }
}

impl Lcg {
    pub fn get_wild_poke_bw1(&mut self) -> WildPoke {
        let mut lcg_local = self.clone();
        if lcg_local.rand(100) > 9 {
            return WildPoke::default()
        }
        let mut result = WildPoke::default();
        result.slot = Some(lcg_local.rand(100));
        lcg_local.next();
        result.poke_code = Some((lcg_local.next() >> 32) as u32);
        result.nature = Some(lcg_local.get_nature());
        result.item = Some(lcg_local.rand(100));
        result
    }

    pub fn get_wild_poke_bw2(&mut self) -> WildPoke {
        let mut lcg_local = self.clone();
        if lcg_local.rand(100) > 20 {
            return WildPoke::default()
        }
        let mut result = WildPoke::default();
        result.slot = Some(lcg_local.rand(100));
        lcg_local.next();
        result.poke_code = Some((lcg_local.next() >> 32) as u32);
        result.nature = Some(lcg_local.get_nature());
        result.item = Some(lcg_local.rand(100));
        result
    }
}

pub fn find_wild_advances_bw1(
    seed0: u64,
    min_advances: u32,
    max_advances: u32,
    is_target: &dyn Fn(&WildPoke) -> bool,
) -> Vec<u32> {
    let mut seed = Lcg::new(seed0);
    seed.offset_seed0(OffsetType::Bw1Continue);

    let mut out = Vec::new();
    seed.advance(min_advances.into());
    for advance in min_advances..max_advances {
        seed.next();
        let wild_poke = seed.get_wild_poke_bw1();
        if is_target(&wild_poke) {
            out.push(advance + 1);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_pup() {
        let mut seed = Lcg::new(0x45758423BB8FCDB8);
        seed.offset_seed1(crate::lcg::OffsetType::Bw1Continue);
        seed.advance(42);
        let pup = seed.get_wild_poke_bw1();

        println!(
            "slot={:?}, poke_code={:?}, nature={:?}, gender={:?}, ability={:?}, item={:?}",
            pup.slot, pup.poke_code, pup.nature, pup.gender(), pup.ability(), pup.item
        );
    }
}
