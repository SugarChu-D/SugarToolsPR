use core::panic;

use rng_core::lcg::{self, Nature};
use rng_core::lcg::Nature::Nature as Nature;
use rng_core::initial_seed as initial_seed;
use rng_core::models::DSConfig as DSConfig;
use rng_core::models::GameDate as GameDate;
use rng_core::models::key_presses as key_presses;

use crate::model::SearchResultBase;

#[derive(Debug,Clone)]
pub struct TepigSearchResult {
    pub search_result_base: SearchResultBase,
    pub tepig_iv_step: u8,
    pub tepig_frame: Box<u32>,
    pub candy_frame: Box<u32>,
    pub pidove_frame: Box<u32>,
    pub psyduck_frame: Box<u32>,
}

const FRAME_ENTERING_ROUTE20: u32 = 430;
const FRAME_EXITING_ROUTE20: u32 = 490;
const FRAME_ENTERING_RANCH: u32 = 550;
const FRAME_EXITING_RANCH: u32 = 630;
const FRAME_MIN_FOR_CANDY: u32 = 360;
const FRAME_MAX_FOR_CANDY: u32 = 440;

const MIN_TEPIG_NATURE: u32 = 215;
const MAX_TEPIG_NATURE: u32 = 270;

/**
やんちゃポカブに合う個体値かどうかを確かめる
*/
fn tepig_iv_check_naughty(ivs: [u8; 6]) -> bool {
    (27..=31).contains(&ivs[0]) // HP
    && (29..=31).contains(&ivs[1]) // A
    && (29..=31).contains(&ivs[2]) // B
    && (29..=31).contains(&ivs[3]) // C
    && &iv[5] == 25 // S
} 

/**
うっかりやに合う個体値かどうかを確かめる
*/
fn tepig_iv_check_rash(ivs: [u8; 6]) -> bool {
    (28..=31).contains(&ivs[0]) // HP
    && (29..=31).contains(&ivs[1]) // A
    && (30..=31).contains(&ivs[2]) // B
    && (30..=31).contains(&ivs[3]) // C
    && (30..=31).contains(&ivs[5]) // S
} 

pub fn white2_tepig_search(config: DSConfig, year: u8, month: u8, day: u8, nature: rng_core::lcg::Nature::Nature)
    -> Vec<TepigSearchResult>
{
    if (year >= 100 || month > 12 || day > 31){
        panic!("Invalid Date!")
    };

    if month % 4 == 2{
        panic!("SUMMER MONTH!")
    };

    let mut results = Vec::new();

    for hour in 0..24{
        for minute in 0..60{
            for second in 0..60{
                for key_presses_value in key_presses::KeyPresses::valid_key_inputs(){
                    let game_date = GameDate{
                        year,
                        month,
                        day,
                        hour,
                        minute,
                        second,
                    };

                    let seed0 = initial_seed::generate_initial_seed0(&config, &game_date, &key_presses_value);

                    let mut rng: lcg::Lcg = rng_core::lcg::Lcg::new(seed0);

                    let seed1: u64 = rng.next();

                    let mut iv_frame:u8;

                    let ivs_15: [u8; 6] = rng_core::MT::mt_1(seed1, 15);

                    let ivs_16: [u8; 6] = rng_core::MT::mt_1(seed1, 16);

                    let mut tepig_iv_frame: u32 = 0;

                    match nature {
                        Nature::Rash => {
                            if tepig_iv_check_rash(ivs_15) {
                                tepig_iv_frame = 15;
                            }
                            else if tepig_iv_check_rash(ivs_16) {
                                tepig_iv_frame = 16
                            }
                        },
                        Nature::Naughty => {
                            if tepig_iv_check_rash(ivs_15) {
                                tepig_iv_frame = 15;
                            }
                            else if tepig_iv_check_rash(ivs_16) {
                                tepig_iv_frame = 16
                            }
                        }
                        _ => {
                            panic!("Invalid Nature!")
                        }
                    }

                    if tepig_iv_frame != 0 {
                        let mut tepig_frame: Box<u32>;
                        for frame in MIN_TEPIG_NATURE..=MAX_TEPIG_NATURE{
                            
                        }
                    }

                };
            }
        }
    }

    results
}