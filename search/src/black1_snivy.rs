use rng_core::{lcg::{Lcg, OffsetType::Bw1Start, nature::Nature}, models::{FieldRange, GameTime, ds_config::DSConfig, game_time_iterator, key_presses::KeyPresses}, mt::mt_0, result_base::ResultBase};

#[derive(Debug, Clone)]
pub struct SnivySearchResult {
    pub seed0: u64,
    pub seed1: u64,
    pub game_time: GameTime,
    pub key_presses: KeyPresses,
    pub ivs: [u8; 6],
    pub advance: u64,
    pub nature: String,
}

pub fn search_snivy(
    ds_config: DSConfig,
    GameTime { hour, minute, second, year, month, day }: GameTime,
) -> Vec<SnivySearchResult> {
    let mut results: Vec<SnivySearchResult> = Vec::new();

    let spec = rng_core::models::GameTimeSpec {
        year: FieldRange { min: year, max: year },
        month: FieldRange { min: month, max: month },
        day: FieldRange { min: day, max: day },
        hour: FieldRange { min: hour, max: hour },
        minute: FieldRange { min: minute, max: minute },
        second: FieldRange { min: second, max: second },
    };

    let time_iter = game_time_iterator::GameTimeIterator::new(spec);
    let valid_key_presses: Vec<_> = KeyPresses::iter_valid().collect();
    
    let inner = time_iter.flat_map(|game_time| {
        valid_key_presses.clone().into_iter().map(move |key_presses| (game_time, key_presses))
    }).collect::<Vec<_>>().into_iter();

    let mut seed_iter = rng_core::initial_seed::SeedIter {
        config: &ds_config,
        inner,
    };

    while let Some(seed_result_base) = seed_iter.next() {
        let ivs = mt_0(seed_result_base.seed0, 10);

        // Aが30以上かつSが24以上でなければスキップ
        if ivs[1] < 30 || ivs[5] < 24 {
            continue;
        }

        let mut lcg = Lcg::new(seed_result_base.seed0);
        lcg.offset_seed0(Bw1Start);
        lcg.next(); // TID/SID
        lcg.pt(4);
        lcg.advance(14); // 15回目のadvanceで性格が決まる
        let advance = lcg.step;
        let nature: Nature = lcg.get_nature();

        if nature != Nature::new(11) && nature != Nature::new(13) && nature != Nature::new(14) {
            continue;
        }

        let result = SnivySearchResult {
            seed0: seed_result_base.seed0,
            seed1: seed_result_base.seed1,
            game_time: seed_result_base.game_time,
            key_presses: seed_result_base.key_presses,
            ivs,
            advance,
            nature: nature.name().to_string(),
        };
        results.push(result);
    }

    results
}

#[cfg(test)]
mod tests {
    use rng_core::models::GameVersion::Black;

use super::*;

    #[test]
    #[ignore]
    fn test_snivy_search() {
        let ds_config = DSConfig {
            version: Black,
            timer0: 0xc7a,
            is_dslite: false,
            mac_address: 0x9bf6d93ce,
        };

        let game_time = GameTime {
            year: 95,
            month: 8,
            day: 30,
            hour: 22,
            minute: 58,
            second: 9,
        };

        let results = search_snivy(ds_config, game_time);
        for r in results {
            println!(
                "seed0={:016X} seed1={:016X} time={:02}:{:02}:{:02} kp={:?} ivs={:?} nature={}, advances={}",
                r.seed0,
                r.seed1,
                r.game_time.hour,
                r.game_time.minute,
                r.game_time.second,
                r.key_presses.pressed_keys_string(),
                r.ivs,
                r.nature,
                r.advance
            );
        }
    }
}
