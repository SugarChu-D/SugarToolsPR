use std::collections::HashSet;

use infra::gpu::context::GpuContext;
use rng_core::gpu::helpers::{GpuInputParams, run_result_base_seedhigh_by_dates};
use rng_core::lcg::{Lcg, OffsetType};
use rng_core::lcg::nature::Nature;
use rng_core::lcg::wild_poke::WildPoke;
use rng_core::models::game_date::GameDate;
use rng_core::models::*;
use rng_core::result_base::ResultBase;

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
    pub seed0: u64,
    pub seed1: u64,
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub key_presses: KeyPresses,
    pub ivs: [u8; 6],
    pub wild_advances: Vec<u32>,
}


const BATCH_DATES: usize = 256;

pub async fn search(ds_config: DSConfig, wild_max_advances: u32) -> Vec<PupSearchResult> {
    let ctx = GpuContext::new().await;
    let mut results = Vec::new();
    let mut seen_seed0: HashSet<u64> = HashSet::new();

    let iv_min: [u32; 6] = [30, 31, 30, 0, 30, 31];
    let iv_max: [u32; 6] = [31, 31, 31, 31, 31, 31];

    let params = GpuInputParams::new(
        ds_config,
        [0, 23],
        [0, 59],
        [0, 59],
        0,
        iv_min,
        iv_max,
    );

    let mut dates = Vec::with_capacity(BATCH_DATES);
    for year in 0..=99u8 {
        for &(month, day) in &TARGET_DATES {
            dates.push(GameDate { year, month, day });
            if dates.len() >= BATCH_DATES {
                collect_gpu_results(&ctx, ds_config, wild_max_advances, &params, &dates, &mut results, &mut seen_seed0).await;
                dates.clear();
            }
        }
    }
    if !dates.is_empty() {
        collect_gpu_results(&ctx, ds_config, wild_max_advances, &params, &dates, &mut results, &mut seen_seed0).await;
    }

    results
}

async fn collect_gpu_results(
    ctx: &GpuContext,
    ds_config: DSConfig,
    wild_max_advances: u32,
    params: &GpuInputParams,
    dates: &[GameDate],
    results: &mut Vec<PupSearchResult>,
    seen_seed0: &mut HashSet<u64>,
) {
    let base_results = match run_result_base_seedhigh_by_dates(ctx, ds_config, params, dates, BATCH_DATES).await {
        Ok(v) => v,
        Err(_) => return,
    };

    for base in base_results.into_iter() {
        let ResultBase {
            seed0,
            seed1,
            game_time,
            key_presses,
            ivs,
            ..
        } = base;
        if !seen_seed0.insert(seed0) {
            continue;
        }

        let wild_advances = find_wild_poke_advances(seed0, wild_max_advances);
        if wild_advances.is_empty() {
            continue;
        }
        results.push(PupSearchResult {
            seed0,
            seed1,
            year: game_time.year as u16,
            month: game_time.month,
            day: game_time.day,
            hour: game_time.hour,
            minute: game_time.minute,
            second: game_time.second,
            key_presses,
            ivs,
            wild_advances,
        });
    }
}

fn find_wild_poke_advances(seed0: u64, max_advances: u32) -> Vec<u32> {
    let mut seed = Lcg::new(seed0);
    seed.offset_seed0(OffsetType::Bw1Continue);
    let mut out = Vec::new();
    for i in 0..max_advances {
        seed.next();
        let pup = seed.get_wild_poke_bw1();
        if is_target_wild_poke(&pup) {
            out.push(i + 1);
        }
    }
    out
}

fn is_target_wild_poke(pup: &WildPoke) -> bool {
    let slot_ok = matches!(pup.slot, Some(94..=97) | Some(99));
    let nature_ok = pup.nature.as_ref() == Some(&Nature::new(3));
    let ability_ok = pup.ability().is_some_and(|v|v == 1);
    let gender_ok = pup.gender().is_some_and(|g| g >= 177);

    slot_ok && nature_ok && ability_ok && gender_ok
}

#[cfg(test)]
mod tests {
    use std::time::Instant;
    use super::*;

    #[test]
    #[ignore]
    fn test_black1_pups() {
        let ds_config = DSConfig{
            Version : GameVersion::Black,
            Timer0 : 0xc7a,
            IsDSLite : false,
            MAC : 0x9bf6d93ce,
        };
        let start = Instant::now();
        let results = pollster::block_on(async { search(ds_config, 70).await });
        let elapsed = start.elapsed();

        println!("Elapsed: {:?}", elapsed);
        println!("Total results: {}", results.len());
        for r in results.iter() {
            println!(
                "seed0={:016X} seed1={:016X} year={:02} date={:02}/{:02} time={:02}:{:02}:{:02} kp={:?} ivs={:?} advances={:?}",
                r.seed0,
                r.seed1,
                r.year,
                r.month,
                r.day,
                r.hour,
                r.minute,
                r.second,
                r.key_presses.pressed_keys_string(),
                r.ivs,
                r.wild_advances
            );
        }
    }
}
