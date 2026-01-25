use rng_core::models::*;
use rng_core::*;

const TARGET_DATES: [(u8, u8); 6] = [
    (4, 29),
    (4, 30),
    (8, 30),
    (8, 31),
    (12, 30),
    (12, 31),
];

#[derive(Debug, Clone)]
pub struct PupSearchResult {
    pub initial_seed: u64,
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub key_presses: key_presses::KeyPresses,
    pub ivs: [u8; 6],
}

pub fn search(ds_config: DSConfig) -> Vec<PupSearchResult> {
    let mut results = Vec::new();
    
    for year in 0..=99 {
        for &(month, day) in &TARGET_DATES {
            for hour in 0..24 {
                for minute in 0..60 {
                    for second in 0..60 {
                        for key_presses_value in key_presses::KeyPresses::valid_key_inputs() {
                            
                            let game_date = GameDate {
                                year,
                                month,
                                day,
                                hour,
                                minute,
                                second,
                            };
                            
                            let seed = initial_seed::generate_initial_seed1(&ds_config, &game_date, &key_presses_value);
                            
                            let ivs: [u8; 6] = MT::mt_1(seed, 0);
                            
                            if (30..=31).contains(&ivs[0])
                                && ivs[1] == 31
                                && (30..=31).contains(&ivs[2])
                                && (30..=31).contains(&ivs[4])
                                && ivs[5] == 31
                            {
                                results.push(PupSearchResult {
                                    initial_seed: seed,
                                    year: game_date.year as u16,
                                    month: game_date.month,
                                    day: game_date.day,
                                    hour: game_date.hour,
                                    minute: game_date.minute,
                                    second: game_date.second,
                                    key_presses: key_presses::KeyPresses { keys: key_presses_value },
                                    ivs,
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    
    results
}

#[cfg(test)]
mod tests {
    use std::{time::Instant};
    use super::*;
    
    
    fn test_black1_pups() {
        let ds_config = DSConfig{
            Version : GameVersion::Black,
            Timer0 : 0xc7a,
            IsDSLite : false,
            MAC : 0x9bf6d93ce,
        };
        let start: Instant = Instant::now();
        let results: Vec<PupSearchResult> = search(ds_config);
        let elapsed: std::time::Duration = start.elapsed();

        println!("1年分の探索時間: {:?}", elapsed);
        println!("見つかった結果: {}件", results.len());
    }
}