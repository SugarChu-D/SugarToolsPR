
use infra::gpu::context::GpuContext;
use rng_core::{gpu::helpers::{GpuInputParams, run_result_base_seedhigh_by_dates}, lcg::{self, OffsetType, cloud_impl::{find_cloud_exists_advances, find_cloud_poke_advances}, nature::Nature, offset_impl}, models::KeyPresses, result_base::ResultBase};

static GOOD_DATES: &[&[u32]] =&[
    &[], // 0月 存在しないため空白
    &[1,2,6,7,8,9,10,14,15,16,17,18,19,22,23,24,25,26,30,31], // 1月
    &[], // 2月
    &[1,2,3,6,7,8,9,10,11,14,15,16,17,18,19,22,23,24,25,26,27,30,31], // 3月
    &[6,7,8,12,15,16,17,23,24,26,27,29], // 4月
    &[1,2,6,7,8,9,10,14,15,16,17,18,19,22,23,24,25,26,30,31], // 5月
    &[], // 6月
    &[1,2,3,6,7,8,9,10,11,14,15,16,17,18,19,22,23,24,25,26,27,30,31], // 7月
    &[4,6,7,12,13,15,16,17,20,23,24,26,27,29], // 8月
    &[1,2,6,7,8,9,10,14,15,16,17,18,19,22,23,24,25,26,30], // 9月
    &[], // 10月
    &[1,2,3,6,7,8,9,10,11,14,15,16,17,18,19,22,23,24,25,26,27,30], // 11月
    &[4,6,7,10,12,13,15,16,17,19,20,23,24,26,27,29] // 12月
];

#[derive(Debug, Clone)]
pub struct DrilburSearchResult {
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
    pub offset: u32,
    pub cloud_advances: Vec<u32>,
    pub wild_advances: Vec<u32>,
}

const BATCH_DATES: usize = 256;

pub async fn drilbur_search(ds_config: rng_core::models::DSConfig) -> Vec<DrilburSearchResult> {
    let ctx = GpuContext::new().await;
    let mut results = Vec::new();
    let mut seen_seed0: std::collections::HashSet<u64> = std::collections::HashSet::new();

    let iv_min: [u32; 6] = [31, 31, 31, 7, 31, 31];
    let iv_max: [u32; 6] = [31, 31, 31, 31, 31, 31];

    let params = GpuInputParams::new(
        ds_config,
        [0, 23],
        [0, 59],
        [5, 7],
        2,
        iv_min,
        iv_max,
    );
    
    let mut dates = Vec::with_capacity(BATCH_DATES);
    for year in 0..=99u8 {
        for month in 1..=12u8 {
            if month % 4 == 2 {
                continue; // 夏月は対象外
            }
            for day in GOOD_DATES[month as usize] {
                dates.push(rng_core::models::game_date::GameDate { year, month, day: *day as u8 });
                if dates.len() >= BATCH_DATES {
                    collect_gpu_results(
                        &ctx,
                        ds_config,
                        &params,
                        &dates,
                        &mut results,
                        &mut seen_seed0,
                    ).await;
                    dates.clear();
                }
            }
        }
    }

    if !dates.is_empty() {
        collect_gpu_results(
            &ctx,
            ds_config,
            &params,
            &dates,
            &mut results,
            &mut seen_seed0,
        ).await;
    }

    results
}

async fn collect_gpu_results(
    ctx: &GpuContext,
    ds_config: rng_core::models::DSConfig,
    params: &GpuInputParams,
    dates: &[rng_core::models::game_date::GameDate],
    results: &mut Vec<DrilburSearchResult>,
    seen_seed0: &mut std::collections::HashSet<u64>,
) {
    let base_results = match run_result_base_seedhigh_by_dates(
        ctx,
        ds_config,
        params,
        dates,
        BATCH_DATES
    )
    .await
    {
        Ok(res) => res,
        Err(e) => {
            eprintln!("Error running GPU search: {:?}", e);
            return;
        }
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
            continue; // Skip if seed0 has already been seen
        }

        let cloud_advances =
            find_cloud_exists_advances(
                seed0,
                26,
                40,
                OffsetType::BW2Continue,
            );

        if cloud_advances.is_empty() {
            continue; // Skip if no valid cloud advances found
        }

        
        let mut lcg = lcg::Lcg::new(seed0);
        lcg.offset_seed0(OffsetType::BW2Continue);
        let offset = lcg.step as u32;

        let wild_advances =
            find_cloud_poke_advances(
                seed0,
                cloud_advances[0] + 4 - offset,
                cloud_advances[cloud_advances.len() - 1] + 60 - offset,
                &is_target_drilbur,
                OffsetType::BW2Continue,
            );

        if wild_advances.is_empty() {
            continue; // Skip if no valid wild advances found
        }

        results.push(DrilburSearchResult {
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
            offset,
            cloud_advances,
            wild_advances,
        });
    }
}

fn is_target_drilbur(poke: &rng_core::lcg::wild_poke::WildPoke) -> bool {
    let slot_ok = poke.slot.is_some_and(|s| s >= 80);
    let nature_ok = poke.nature.as_ref() == Some(&Nature::new(3));
    let ability_ok = poke.ability().is_some_and(|v| v == 1);
    let gender_ok = poke.gender().is_some_and(|g| g < 127);

    slot_ok && nature_ok && ability_ok && gender_ok
}

#[cfg(test)]
mod tests {
    use std::time::Instant;
    use rng_core::models::{DSConfig, GameVersion};

use super::*;

    #[test]
    #[ignore]
    fn test_drilbur_search() {
        let ds_config = DSConfig {
            version : GameVersion::White2,
            timer0 : 0x10FA,
            is_dslite : false,
            mac_address : 0x9bf6d93ce,
        };

        let start = Instant::now();
        let results = pollster::block_on(async {
            drilbur_search(ds_config).await
        });
        let elapsed = start.elapsed();
        println!("Elapsed: {:?}", elapsed);
        println!("Total results: {}", results.len());
        println!("seed0, seed1, year, month, day, hour, minute, second, key_presses, h, a, b, c, d, s, offset, clouds, advances");
        for r in results.iter() {
            println!(
                "{:016X}, {:016X}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}",
                r.seed0,
                r.seed1,
                r.year,
                r.month,
                r.day,
                r.hour,
                r.minute,
                r.second,
                r.key_presses.pressed_keys_string(),
                r.ivs[0],
                r.ivs[1],
                r.ivs[2],
                r.ivs[3],
                r.ivs[4],
                r.ivs[5],
                r.offset,
                r.cloud_advances.iter().map(|a| a.to_string()).collect::<Vec<_>>().join("|"),
                r.wild_advances.iter().map(|a| a.to_string()).collect::<Vec<_>>().join("|"),
            );
        }
    }
}
