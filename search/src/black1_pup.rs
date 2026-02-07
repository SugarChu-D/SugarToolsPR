use infra::gpu::context::GpuContext;
use rng_core::gpu::helpers::{GpuInputParams, run_result_base_seedhigh_by_dates};
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
}


const BATCH_DATES: usize = 256;

pub async fn search(ds_config: DSConfig) -> Vec<PupSearchResult> {
    let ctx = GpuContext::new().await;
    let mut results = Vec::new();

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
                collect_gpu_results(&ctx, ds_config, &params, &dates, &mut results).await;
                dates.clear();
            }
        }
    }
    if !dates.is_empty() {
        collect_gpu_results(&ctx, ds_config, &params, &dates, &mut results).await;
    }

    results
}

async fn collect_gpu_results(
    ctx: &GpuContext,
    ds_config: DSConfig,
    params: &GpuInputParams,
    dates: &[GameDate],
    results: &mut Vec<PupSearchResult>,
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
        });
    }
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
        let results = pollster::block_on(async { search(ds_config).await });
        let elapsed = start.elapsed();

        println!("Elapsed: {:?}", elapsed);
        println!("Total results: {}", results.len());
    }
}

