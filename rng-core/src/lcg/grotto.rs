use super::Lcg;

#[derive(Debug, Clone, Copy, Default)]
pub struct Grotto {
    sub_slot: Option<u32>,
    slot: Option<u32>,
    gender: Option<u32>
}

#[derive(Debug, Clone)]
pub struct Grottos {
    grottos: [Grotto; 20]
}

impl Grottos {
    pub fn new () -> Self {
        Self { grottos: [Grotto::default(); 20] }
    }

    pub fn new_game () -> Self {
        Self { grottos: Self::new_game_grottos() }
    }

    pub fn reset_newgame(&mut self) {
        self.grottos = Self::new_game_grottos();
    }

    pub fn fill_grottos(&mut self, lcg: &Lcg) {
        let mut lcg_local = lcg.clone();
        for i in 0..20 {
            // もし埋まっているならスキップ
            if self.grottos[i].sub_slot.is_some() {continue;}
            // lcgが5以上なら埋めないでスキップ
            if lcg_local.rand(100) >= 5 {continue;}

            self.grottos[i].sub_slot = Some(lcg_local.rand(4));
            self.grottos[i].slot = Some(lcg_local.rand(100));
            self.grottos[i].gender = Some(lcg_local.rand(100));
        }
    }

    fn new_game_grottos() -> [Grotto; 20] {
        [
            Grotto::default(),
            Grotto{
                sub_slot: Some(1),
                slot: Some(0),
                gender: Some(0)
                // 5番道路のチラーミィはデフォルトで埋まってる
            },
            Grotto::default(),
            Grotto::default(),
            Grotto::default(),
            Grotto::default(),
            Grotto::default(),
            Grotto::default(),
            Grotto::default(),
            Grotto::default(),
            Grotto::default(),
            Grotto::default(),
            Grotto::default(),
            Grotto::default(),
            Grotto::default(),
            Grotto::default(),
            Grotto::default(),
            Grotto::default(),
            Grotto::default(),
            Grotto::default(),
        ]
    }
}

impl Grotto {
    pub fn sub_slot(&self) -> Option<u32> {
        self.sub_slot
    }

    pub fn slot(&self) -> Option<u32> {
        self.slot
    }

    pub fn gender(&self) -> Option<u32> {
        self.gender
    }
}

impl Grottos {
    pub fn get(&self, index: usize) -> Option<Grotto> {
        self.grottos.get(index).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_candy() {
        let mut seed = Lcg::new(0xf9d9dd91248eecb0);
        seed.advance(385);
        let mut grottos = Grottos::new_game();
        grottos.fill_grottos(&seed);

        for i in 0..20 {
            println!(
                "{}: sub_slot={:?}, slot={:?}",
                i, grottos.grottos[i].sub_slot, grottos.grottos[i].slot
            );
        }
    }

    #[test]
    fn test_get_candy_dragonite() {
        let mut seed = Lcg::new(0x113E10468C85C156);
        seed.advance(395);
        let mut grottos = Grottos::new();
        grottos.fill_grottos(&seed);

        for i in 0..20 {
            println!(
                "{}: sub_slot={:?}, slot={:?}, gender={:?}",
                i, grottos.grottos[i].sub_slot, grottos.grottos[i].slot, grottos.grottos[i].gender
            );
        }
    }
}
