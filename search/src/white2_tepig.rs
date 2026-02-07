use core::panic;
use std::collections::HashSet;
use std::thread;

use infra::gpu::context::GpuContext;
use rayon::prelude::*;
use rng_core::gpu::helpers::{GpuInputParams, run_result_base_seedhigh_by_dates_multi_iv};
use rng_core::gpu::input_layout::GpuIvConfig;
use rng_core::lcg::{Lcg, OffsetType};
use rng_core::lcg::grotto::Grottos;
use rng_core::lcg::nature::Nature as Nature;
use rng_core::lcg::wild_poke::WildPoke;
use rng_core::models::DSConfig as DSConfig;
use rng_core::models::game_date::GameDate;

#[derive(Debug,Clone)]
pub struct TepigSearchResult {
    pub seed0: u64,
    pub seed1: u64,
    pub year: u8,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub tid: u16,
    pub key_presses: String,
    pub ivs: [u8; 6],
    pub tepig_iv_step: u8,
    pub tepig_frames: Vec<u32>,
    pub candy_frames: Vec<(u32, Grottos)>,
    pub pidove_frames: Vec<(u32, WildPoke)>,
    pub psyduck_frames: Vec<(u32, WildPoke)>,
}

#[derive(Debug, Clone, Copy)]
pub enum BW2Mode {
    Normal,
    Challenge,
}

const FRAME_ENTERING_ROUTE20: u64 = 430;
const FRAME_EXITING_ROUTE20: u64 = 510;
const FRAME_ENTERING_RANCH: u64 = 550;
const FRAME_EXITING_RANCH: u64 = 630;
const FRAME_MIN_FOR_CANDY: u64 = 360;
const FRAME_MAX_FOR_CANDY: u64 = 440;

const MIN_TEPIG_NATURE: u64 = 190;
const MAX_TEPIG_NATURE: u64 = 240;
const GROTTO_INDEX: usize = 3;
const GROTTO_SUB_SLOT: u32 = 0;
const GROTTO_SLOT: u32 = 60;
const GROTTO_INDEX_2: usize = 19;
const GROTTO_SLOT_2: u32 = 0;

/**
やんちゃの個体値かどうかを確かめる
*/
fn tepig_iv_check_naughty(ivs: [u8; 6]) -> bool {
    (27..=31).contains(&ivs[0]) // HP
    && (29..=31).contains(&ivs[1]) // A
    && (29..=31).contains(&ivs[2]) // B
    && (29..=31).contains(&ivs[3]) // C
    && ivs[5] == 25 // S
} 

/**
うっかりやに適した個体値かどうかを確かめる
*/
fn tepig_iv_check_rash(ivs: [u8; 6]) -> bool {
    (28..=31).contains(&ivs[0]) // HP
    && (29..=31).contains(&ivs[1]) // A
    && (30..=31).contains(&ivs[2]) // B
    && (30..=31).contains(&ivs[3]) // C
    && (30..=31).contains(&ivs[5]) // S
}

fn tepig_iv_check(ivs: [u8; 6], nat: &Nature) -> bool {
    match nat.id() {
        4 => tepig_iv_check_naughty(ivs),
        19 => tepig_iv_check_rash(ivs),
        any => panic!("Invalid Nature: {}", any.to_string()),
    }
}

const BATCH_DATES: usize = 256;

pub async fn white2_tepig_dragonite_search(config: DSConfig, nat: Nature, mode: BW2Mode)
    -> Vec<TepigSearchResult> {
    let dates = build_all_dates();
    search_by_dates(config, nat, &dates, mode, find_grotto_advances_candy_dragonite).await
}

pub async fn white2_tepig_search(config: DSConfig, year: u8, month: u8, day: u8, nat: Nature, mode: BW2Mode)
    -> Vec<TepigSearchResult> {
    if year >= 100 || month > 12 || day > 31 {
        panic!("Invalid Date!")
    };

    if month % 4 == 2{
        panic!("SUMMER MONTH!")
    };

    let dates = [GameDate{ year, month, day }];
    search_by_dates(config, nat, &dates, mode, find_grotto_advances_candy).await
}

async fn search_by_dates(
    config: DSConfig,
    nat: Nature,
    dates: &[GameDate],
    mode: BW2Mode,
    find_grotto: fn(u64, u64, u64) -> Vec<(u32, Grottos)>,
)
    -> Vec<TepigSearchResult> {
    let ctx = GpuContext::new().await;
    let mut results = Vec::new();
    let mut seen_seed0: HashSet<u64> = HashSet::new();
    let mut pending_cpu: Option<thread::JoinHandle<Vec<TepigSearchResult>>> = None;

    let (iv_min, iv_max): ([u32; 6], [u32; 6]) = match nat.id() {
        4 => ([27, 29, 29, 29, 0, 25], [31, 31, 31, 31, 31, 25]), // Naughty
        19 => ([28, 29, 30, 30, 0, 30], [31, 31, 31, 31, 31, 31]), // Rash
        any => panic!("Invalid Nature: {}", any.to_string()),
    };

    let params = GpuInputParams::new(
        config,
        [0, 23],
        [0, 59],
        [0, 59],
        16,
        iv_min,
        iv_max,
    );
    let iv_cfgs = [
        GpuIvConfig {
            iv_step: 16,
            _pad0: 0,
            iv_min,
            iv_max,
        },
        GpuIvConfig {
            iv_step: 17,
            _pad0: 0,
            iv_min,
            iv_max,
        },
    ];

    let mut date_batch = Vec::with_capacity(BATCH_DATES);
    for &date in dates {
        date_batch.push(date);
        if date_batch.len() < BATCH_DATES {
            continue;
        }
        let base_results = match run_result_base_seedhigh_by_dates_multi_iv(
            &ctx,
            config,
            &params,
            &date_batch,
            BATCH_DATES,
            &iv_cfgs,
        ).await {
            Ok(v) => v,
            Err(_) => {
                date_batch.clear();
                continue;
            }
        };
        if let Some(handle) = pending_cpu.take() {
            let batch_results = handle.join().expect("CPU worker thread panicked");
            merge_results(batch_results, &mut results, &mut seen_seed0);
        }
        let nat_clone = nat.clone();
        pending_cpu = Some(thread::spawn(move || {
            process_base_results(base_results, mode, nat_clone, find_grotto)
        }));
        date_batch.clear();
    }
    if !date_batch.is_empty() {
        let base_results = match run_result_base_seedhigh_by_dates_multi_iv(
            &ctx,
            config,
            &params,
            &date_batch,
            BATCH_DATES,
            &iv_cfgs,
        ).await {
            Ok(v) => v,
            Err(_) => Vec::new(),
        };
        if let Some(handle) = pending_cpu.take() {
            let batch_results = handle.join().expect("CPU worker thread panicked");
            merge_results(batch_results, &mut results, &mut seen_seed0);
        }
        let nat_clone = nat.clone();
        pending_cpu = Some(thread::spawn(move || {
            process_base_results(base_results, mode, nat_clone, find_grotto)
        }));
    }
    if let Some(handle) = pending_cpu.take() {
        let batch_results = handle.join().expect("CPU worker thread panicked");
        merge_results(batch_results, &mut results, &mut seen_seed0);
    }

    results
}

fn merge_results(
    batch_results: Vec<TepigSearchResult>,
    results: &mut Vec<TepigSearchResult>,
    seen_seed0: &mut HashSet<u64>,
) {
    for candidate in batch_results {
        if !seen_seed0.insert(candidate.seed0) {
            continue;
        }
        results.push(candidate);
    }
}

fn process_base_results(
    base_results: Vec<rng_core::result_base::ResultBase>,
    mode: BW2Mode,
    nat: Nature,
    find_grotto: fn(u64, u64, u64) -> Vec<(u32, Grottos)>,
    )
    -> Vec<TepigSearchResult> {
    base_results
        .into_par_iter()
        .filter_map(|base| {
            let seed0 = base.seed0;
            let seed1 = base.seed1;

            let mut rng: Lcg = Lcg::new(seed0);
            let offset = match mode {
                BW2Mode::Normal => rng.offset_seed0(OffsetType::BW2Start),
                BW2Mode::Challenge => rng.offset_seed0(OffsetType::BW2StartChallengeMode),
            };

            let ivs_16: [u8; 6] = rng_core::mt::mt_1(seed1, 16);
            let ivs_17: [u8; 6] = rng_core::mt::mt_1(seed1, 17);
            let ivs: [u8; 6];
            let tepig_iv_frame: u8;

            if tepig_iv_check(ivs_16, &nat) {
                ivs = ivs_16;
                tepig_iv_frame = 16;
            } else if tepig_iv_check(ivs_17, &nat) {
                ivs = ivs_17;
                tepig_iv_frame = 17;
            } else {
                return None;
            }

            if tepig_iv_frame == 17 {
                rng.next();
            }

            let tid = rng.tid_sid(OffsetType::BW2Start).0;

            rng.advance(MIN_TEPIG_NATURE - 1);

            let mut tepig_frames = Vec::new();
            for frame in MIN_TEPIG_NATURE..=MAX_TEPIG_NATURE {
                if rng.get_nature() == nat {
                    tepig_frames.push((frame + offset & 0xFFFFFFFF) as u32);
                }
            }
            if tepig_frames.is_empty() {
                return None;
            }

            let pidove_frames = find_wild_advances_bw2(
                seed0,
                FRAME_ENTERING_ROUTE20,
                FRAME_EXITING_ROUTE20,
                is_target_pidove,
            );
            if pidove_frames.is_empty() {
                return None;
            }

            let psyduck_frames = find_wild_advances_bw2(
                seed0,
                FRAME_ENTERING_RANCH,
                FRAME_EXITING_RANCH,
                is_target_psyduck,
            );
            if psyduck_frames.is_empty() {
                return None;
            }

            let candy_frames = find_grotto(seed0, FRAME_MIN_FOR_CANDY, FRAME_MAX_FOR_CANDY);
            if candy_frames.is_empty() {
                return None;
            }

            Some(TepigSearchResult {
                seed0,
                seed1,
                year: base.game_time.year,
                month: base.game_time.month,
                day: base.game_time.day,
                hour: base.game_time.hour,
                minute: base.game_time.minute,
                second: base.game_time.second,
                tid,
                key_presses: base.key_presses.pressed_keys_string(),
                ivs,
                tepig_iv_step: tepig_iv_frame,
                tepig_frames,
                candy_frames,
                pidove_frames,
                psyduck_frames,
            })
        })
        .collect()
}

fn build_all_dates() -> Vec<GameDate> {
    let mut dates = Vec::new();
    for year in 0..=99u8 {
        for month in 1..=12u8 {
            if month % 4 == 2 {
                continue;
            }
            let days = GameDate::new(year, month, 1).days_in_month();
            if days == 0 {
                continue;
            }
            for day in 1..=days {
                dates.push(GameDate { year, month, day });
            }
        }
    }
    dates
}

fn find_wild_advances_bw2(
    seed0: u64,
    start: u64,
    end: u64,
    is_target: fn(&WildPoke) -> bool,
) -> Vec<(u32, WildPoke)> {
    let mut seed = Lcg::new(seed0);
    if start > 1 {
        seed.advance(start - 1);
    }
    let mut out: Vec<(u32, WildPoke)> = Vec::new();
    for frame in start..=end {
        seed.next();
        let pup = seed.get_wild_poke_bw2();
        if is_target(&pup) {
            out.push((frame as u32, pup));
        }
    }
    out
}

fn is_target_pidove(dov: &WildPoke) -> bool {
    matches!(dov.slot, Some(0..20) | Some(80..85))
}

fn is_target_psyduck(duck: &WildPoke) -> bool {
    let slot_ok = matches!(duck.slot, Some(70..80));
    let ability_ok = duck.ability().is_some_and(|v|v == 0);

    slot_ok && ability_ok
}

fn find_grotto_advances_candy(seed0: u64, start: u64, end: u64) -> Vec<(u32, Grottos)> {
    let mut out: Vec<(u32, Grottos)> = Vec::new();
    if start > end {
        return out;
    }

    let mut seed = Lcg::new(seed0);
    seed.advance(start);
    for frame in start..=end {
        let mut grottos = Grottos::new();
        grottos.fill_grottos(&seed);
        let index3_ok = grottos.get(GROTTO_INDEX).is_some_and(|grotto| {
            grotto.sub_slot() == Some(GROTTO_SUB_SLOT) && grotto.slot() == Some(GROTTO_SLOT)
        });
        if index3_ok {
            out.push((frame as u32, grottos));
        }
        if frame < end {
            seed.next();
        }
    }
    out
}

fn find_grotto_advances_candy_dragonite(seed0: u64, start: u64, end: u64) -> Vec<(u32, Grottos)> {
    let mut out: Vec<(u32, Grottos)> = Vec::new();
    if start > end {
        return out;
    }

    let mut seed = Lcg::new(seed0);
    seed.advance(start);
    for frame in start..=end {
        let mut grottos = Grottos::new();
        grottos.fill_grottos(&seed);
        let index3_ok = grottos.get(GROTTO_INDEX).is_some_and(|grotto| {
            grotto.sub_slot() == Some(GROTTO_SUB_SLOT) && grotto.slot() == Some(GROTTO_SLOT)
        });
        let index19_ok = grottos.get(GROTTO_INDEX_2).is_some_and(|grotto| {
            grotto.slot() == Some(GROTTO_SLOT_2)
        });
        if index3_ok && index19_ok {
            out.push((frame as u32, grottos));
        }
        if frame < end {
            seed.next();
        }
    }
    out
}


impl TepigSearchResult {
    #[cfg(test)]
    fn print(&self) {
        use rng_core::lcg::TID_impl::get_frigate_pass;

        println!(
            "seed0: {:016X} seed1: {:016X} ",
            self.seed0,
            self.seed1,
        );
        println!("date: {:02}/{:02}/{:02} {:02}:{:02}:{:02} key={}",
            self.year,
            self.month,
            self.day,
            self.hour,
            self.minute,
            self.second,
            self.key_presses,
        );
        println!("TID: {} Pass: {}", self.tid, get_frigate_pass(self.tid));
            println!("ivs={:?} iv_step={}", self.ivs, self.tepig_iv_step);
            println!("Tepig frame: {:?}", self.tepig_frames);
            println!("Pidove:");
            for pidove in &self.pidove_frames {
                println!("{}:{} {}",
                    pidove.0,
                    if pidove.1.slot.is_some_and(|s| s < 20) {"Lv.2"} else {"Lv.4"},
                    pidove.1.nature.clone().unwrap().name(),
                )
            }

            println!("PsyDuck:");
            for psyduck in &self.psyduck_frames {
                println!("{}:{}",
                    psyduck.0,
                    psyduck.1.nature.clone().unwrap().name(),
                )
            }

            print!("candy:");
            for candy in &self.candy_frames {
                println!("{}: ", candy.0);
                for i in 0..candy.1.grottos.len() {
                    if candy.1.grottos[i].slot().is_some() {
                        println!("#{}: {:?}", i, candy.1.grottos[i]);
                    }
                }
            }
            println!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    #[ignore]
    fn test_white2_tepig_single_date() {
        let ds_config = DSConfig{
            Version : rng_core::models::GameVersion::White2,
            Timer0 : 0x10FA,
            IsDSLite : false,
            MAC : 0x0009bf6d93ce,
        };

        let start = Instant::now();
        let results = pollster::block_on(async {
            white2_tepig_search(
                ds_config,
                33,
                8,
                27,
                Nature::new(4),
                BW2Mode::Normal
            ).await // 例: Rash
        });
        let elapsed = start.elapsed();

        println!("Elapsed: {:?}", elapsed);
        println!("Total results: {}", results.len());
        for r in results.iter() {
            r.print();
        }
    }
    #[test]
    #[ignore]
    fn test_white2_tepig_dragonite() {
        let ds_config = DSConfig{
            Version : rng_core::models::GameVersion::White2,
            Timer0 : 0x10FA,
            IsDSLite : false,
            MAC : 0x0009bf6d93ce,
        };

        let start = Instant::now();let results = pollster::block_on(async {
            white2_tepig_dragonite_search(
                ds_config,
                Nature::new(4),
                BW2Mode::Normal
            ).await // 例: Naughty
        });
        let elapsed = start.elapsed();

        println!("Elapsed: {:?}", elapsed);
        println!("Total results: {}", results.len());
        for r in results.iter() {
            r.print();
        }
    }
}
