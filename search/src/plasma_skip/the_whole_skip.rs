use super::charge_stone_tile::*;
use rng_core::lcg::{self, Lcg};
use rng_core::models::{FieldRange, GameTimeSpec, game_time_iterator};
use rng_core::models::DSConfig;
use rng_core::models::game_date::{GameDate};
use rng_core::initial_seed::SeedResultBase;
use rng_core::models::key_presses::KeyPresses;

#[derive(Debug, Clone)]
pub struct Cloud {
    pub trainer_tile: [i32; 2],
    pub cloud_tile: [i32; 2],
    pub frame: u32,
}

#[derive(Debug, Clone)]
pub struct PlasmaSkipSearchResult {
    pub seed_result_base: SeedResultBase,
    pub offset: u32,
    pub first_cloud: Vec<Cloud>,
    pub second_cloud: Vec<Cloud>,
    pub third_cloud: Vec<Cloud>,
    pub fourth_cloud: Vec<Cloud>,
    pub fifth_cloud: Vec<Cloud>,
}

pub fn plasma_skip_search_spring(ds_config: DSConfig, game_date: GameDate) -> Vec<PlasmaSkipSearchResult> {
    let mut results = Vec::new();

    let spec = GameTimeSpec {
        year: FieldRange { min: game_date.year, max: game_date.year },
        month: FieldRange { min: game_date.month, max: game_date.month },
        day: FieldRange { min: game_date.day, max: game_date.day },
        hour: FieldRange { min: 23, max: 23 },
        minute: FieldRange { min: 20, max: 45 },
        second: FieldRange { min: 5, max: 8 },
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
        let first_cloud = find_valid_clouds(
            seed_result_base.seed1,
            39,
            60,
            FIRST_SKIP_TILE,
            FIRST_CLOUD_TILE,
            CHARGE_STONE_B1F_VALID_TILES,
        );

        if first_cloud.is_empty() {
            continue;
        }

        let first_frame = first_cloud[0].frame;

        let second_cloud_left = find_valid_clouds(
            seed_result_base.seed1,
            first_frame + 49,
            first_frame + 85,
            SECOND_SKIP_TILE_LEFT,
            SECOND_CLOUD_TILE_LEFT,
            CHARGE_STONE_B1F_VALID_TILES,
        );

        let second_cloud_right = find_valid_clouds(
            seed_result_base.seed1,
            first_frame + 49,
            first_frame + 85,
            SECOND_SKIP_TILE_RIGHT,
            SECOND_CLOUD_TILE_RIGHT,
            CHARGE_STONE_B1F_VALID_TILES,
        );
        
        let second_cloud = [second_cloud_left, second_cloud_right].concat();
        if second_cloud.is_empty() {
            continue;
        }
        let second_frame = second_cloud[0].frame;

        let third_cloud = find_valid_clouds(
            seed_result_base.seed1,
            second_frame + 69,
            second_frame + 95,
            THIRD_SKIP_TILE,
            THIRD_CLOUD_TILE,
            CHARGE_STONE_B1F_VALID_TILES,
        );

        if third_cloud.is_empty() {
            continue;
        }

        let third_frame = third_cloud[0].frame;

        let fourth_cloud = find_valid_clouds(
            seed_result_base.seed1,
            third_frame + 54,
            third_frame + 95,
            FOURTH_SKIP_TILE,
            FOURTH_CLOUD_TILE,
            CHARGE_STONE_B1F_VALID_TILES,
        );

        if fourth_cloud.is_empty() {
            continue;
        }

        let fourth_frame = fourth_cloud[0].frame;

        let fifth_cloud = find_valid_clouds(
            seed_result_base.seed1,
            fourth_frame + 44,
            fourth_frame + 80,
            FIFTH_SKIP_TILE,
            FIFTH_CLOUD_TILE,
            CHARGE_STONE_B1F_VALID_TILES,
        );

        if fifth_cloud.is_empty() {
            continue;
        }
        
        let mut lcg = Lcg::new(seed_result_base.seed1);
        lcg.offset_seed1(lcg::OffsetType::Bw1Continue);
        let offset = lcg.step as u32;

        results.push(PlasmaSkipSearchResult {
            seed_result_base: seed_result_base.clone(),
            offset,
            first_cloud: first_cloud.clone(),
            second_cloud: second_cloud.clone(),
            third_cloud: third_cloud.clone(),
            fourth_cloud: fourth_cloud.clone(),
            fifth_cloud: fifth_cloud.clone(),
        });
    }
    
    results
}


fn find_valid_clouds(
    seed1: u64,
    frame_start: u32,
    frame_end: u32,
    trainer_tiles: &[[i32; 2]],
    target_tiles: &[[i32; 2]],
    valid_tiles: &[[i32; 2]],
) -> Vec<Cloud> {
    let mut lcg = Lcg::new(seed1);
    lcg.offset_seed1(lcg::OffsetType::Bw1Continue);
    let offset = lcg.step as u32;

    if frame_start > 1 {
        lcg.advance((frame_start - 1) as u64);
    }
    let mut clouds = Vec::new();

    for frame in frame_start..=frame_end {
        // if (lcg.step as u32) != frame + offset {
        //     panic!("LCG step does not match expected frame. Expected: {}, Actual: {}, Offset: {}", frame, lcg.step, offset);
        // }
        lcg.next();
        if (lcg.step as u32 - frame_start - offset) % 2 == 0 {
            continue;
        }
        let mut lcg_clone = lcg.clone();
        for trainer_tile in trainer_tiles {
            let cloud_tile = lcg_clone.get_cloud(trainer_tile, valid_tiles);
            if target_tiles.contains(&cloud_tile) {
                clouds.push(Cloud {
                    trainer_tile: *trainer_tile,
                    cloud_tile,
                    frame: (lcg.step as u32) - offset,
                });
            }
        }
    }

    clouds
}

#[cfg(test)]
mod tests {
    use rng_core::lcg::OffsetType::Bw1Continue;

use crate::plasma_skip::charge_stone_tile;
    use super::*;

    #[test]
    fn test_cloud_impl() {
        let mut lcg = Lcg::new(0x7E5A5625DFABC4C7);
        lcg.offset_seed1(Bw1Continue);
        lcg.advance(48);
        println!("{}", lcg.step);
        let cloud_tile = lcg.get_cloud(&[48, 34], charge_stone_tile::CHARGE_STONE_B1F_VALID_TILES);
        println!("cloud_tile: {:?}", cloud_tile);
        //assert_eq!(cloud_tile, [50, 33]);
    }

    #[test]
    fn test_exisitng_pskip() {
        let mut lcg = Lcg::new(0xabaa7978d5d90e58);
        lcg.advance(94);
        let cloud_first = lcg.get_cloud(&[48, 34], charge_stone_tile::CHARGE_STONE_B1F_VALID_TILES);
        assert_eq!(cloud_first, [49, 33]);
        println!("{}", lcg.step); // 97

        lcg.advance(162 - 94 - 3);

        let cloud_second = lcg.get_cloud(&[49, 23], charge_stone_tile::CHARGE_STONE_B1F_VALID_TILES);
        assert_eq!(cloud_second[1], 21);

        lcg.advance(240 - 162 - 3);

        let cloud_third = lcg.get_cloud(&[54, 13], charge_stone_tile::CHARGE_STONE_B1F_VALID_TILES);
        assert_eq!(cloud_third, [53, 14]);
    }
    
    #[test]
    #[ignore]
    fn test_plasma_skip_search_spring() {
        let ds_config = DSConfig {
            version: rng_core::models::GameVersion::Black,
            timer0: 0xc7a,
            mac_address: 0x0009bf6d93ce,
            is_dslite: false,
        };

        let game_date = GameDate {
            year: 95,
            month: 8,
            day: 31,
        };

        let results = plasma_skip_search_spring(ds_config, game_date);
        let len = results.len();
        for result in results {
            println!("seed1: {:016X}", result.seed_result_base.seed1);
            println!("offset: {}", result.offset);
            println!("time: {:02}:{:02}:{:02}", result.seed_result_base.game_time.hour, result.seed_result_base.game_time.minute, result.seed_result_base.game_time.second);
            println!("key presses: {}", result.seed_result_base.key_presses.pressed_keys_string());
            println!("first cloud: {:?}", result.first_cloud);
            println!("second cloud: {:?}", result.second_cloud);
            println!("third cloud: {:?}", result.third_cloud);
            println!("fourth cloud: {:?}", result.fourth_cloud);
            println!("fifth cloud: {:?}", result.fifth_cloud);
            println!();
        }
        println!("Total results: {}", len);
    }
}
