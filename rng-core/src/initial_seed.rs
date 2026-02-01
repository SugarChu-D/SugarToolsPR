use crate::sha_1::generate_initial_seed0;
use crate::models::{DSConfig, GameTime, KeyPresses, game_time, key_presses};
use crate::lcg::{Lcg, lcg_next};

pub struct SeedResultBase {
    pub ds_config: DSConfig,
    pub seed0: u64,
    pub seed1: u64,
    pub game_time: GameTime,
    pub key_presses: KeyPresses,
}

pub struct SeedIter<'a, I>
where I:Iterator<Item = (GameTime, KeyPresses)>,
{
    config: &'a DSConfig,
    inner: I,
}

impl<'a, T> Iterator for SeedIter<'a, T>
where T: Iterator<Item = (GameTime, KeyPresses)>,
{
    type Item = SeedResultBase;

    fn next(&mut self) -> Option<Self::Item> {
        let (game_time, key_presses) = self.inner.next()?;

        let seed0 = generate_initial_seed0(self.config, &game_time, key_presses);
        let seed1 = lcg_next(seed0);

        Some(SeedResultBase { ds_config:*self.config, seed0, seed1, game_time, key_presses })
    }
}


#[cfg(test)]
mod tests {
    use crate::models::GameVersion;

    use super::*;

    #[test]
    fn seed0_iterator_basic() {
        // --- DSConfig ダミー ---
        let config = DSConfig {
            Version: GameVersion::Black,
            Timer0: 0xc7a,
            MAC: 0x0009bf6d93ce,
            IsDSLite: false,
            // 他のフィールドがあれば適当に固定
        };

        // --- GameTime ダミー ---
        let t1 = GameTime::new(26, 1, 24, 12, 0, 0);
        let t2 = GameTime::new(26, 1, 24, 12, 0, 1);

        // --- KeyPresses ダミー ---
        let k1 = KeyPresses::new(0x2fff);
        let k2 = KeyPresses::new(0x2ffe);

        // --- inner iterator（直積を模倣）---
        let inner = vec![
            (t1, k1),
            (t1, k2),
            (t2, k1),
            (t2, k2),
        ].into_iter();

        let mut iter: SeedIter<'_, std::vec::IntoIter<(GameTime, KeyPresses)>> = SeedIter{
            config: &config,
            inner,
        };

        // --- 1個目 ---
        let r1 = iter.next().expect("first item");
        assert_eq!(r1.game_time, t1);
        assert_eq!(r1.key_presses.raw(), 0x2fff);

        // --- 2個目 ---
        let r2 = iter.next().expect("second item");
        assert_eq!(r2.game_time, t1);
        assert_eq!(r2.key_presses.raw(), 0x2ffe);

        // --- 全件消費 ---
        let rest: Vec<_> = iter.collect();
        assert_eq!(rest.len(), 2);

        // --- seed が毎回計算されていることだけ確認 ---
        assert_ne!(r1.seed0, 0);
        assert_ne!(r2.seed0, 0);
    }
}
