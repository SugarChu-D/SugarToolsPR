use crate::gpu::input_layout::{GpuInput, GpuIvConfig};
use crate::gpu::mt_kernel;
use crate::gpu::sha1_kernel;
use crate::gpu::staging_layout::candidate_game_time;
use crate::lcg::lcg_next;
use crate::models::game_date::GameDate;
use crate::models::{DSConfig, KeyPresses};
use crate::mt;
use crate::result_base::ResultBase;

#[derive(Clone, Copy)]
pub struct GpuInputParams {
    nazo: [u32; 5],
    vcount_timer0_as_data5: u32,
    mac: u64,
    gxframe_xor_frame: u32,
    hour_range: [u32; 2],
    minute_range: [u32; 2],
    second_range: [u32; 2],
    iv_step: u32,
    iv_min: [u32; 6],
    iv_max: [u32; 6],
}

impl GpuInputParams {
    pub fn new(
        ds_config: DSConfig,
        hour_range: [u32; 2],
        minute_range: [u32; 2],
        second_range: [u32; 2],
        iv_step: u32,
        iv_min: [u32; 6],
        iv_max: [u32; 6],
    ) -> Self {
        let vcfg = ds_config.get_version_config();
        let nazo = [
            vcfg.nazo_values.nazo1,
            vcfg.nazo_values.nazo2,
            vcfg.nazo_values.nazo3,
            vcfg.nazo_values.nazo4,
            vcfg.nazo_values.nazo5,
        ];
        let vcount_timer0_as_data5 = ((vcfg.vcount.0 as u32) << 16) | (ds_config.Timer0 as u32);
        let gxframe_xor_frame = if ds_config.IsDSLite { 0x0600_0006 } else { 0x0600_0008 };

        Self {
            nazo,
            vcount_timer0_as_data5,
            mac: ds_config.MAC,
            gxframe_xor_frame,
            hour_range,
            minute_range,
            second_range,
            iv_step,
            iv_min,
            iv_max,
        }
    }

    pub fn with_date(&self, date: GameDate) -> GpuInput {
        GpuInput {
            nazo: self.nazo,
            vcount_timer0_as_data5: self.vcount_timer0_as_data5,
            mac: self.mac,
            gxframe_xor_frame: self.gxframe_xor_frame,
            date_as_data8: date.get_date8_format(),
            hour_range: self.hour_range,
            minute_range: self.minute_range,
            second_range: self.second_range,
            _pad0: 0,
            iv_step: self.iv_step,
            iv_min: self.iv_min,
            iv_max: self.iv_max,
        }
    }

}

pub async fn run_sha1_mt_compact_by_dates(
    ctx: &infra::gpu::context::GpuContext,
    params: &GpuInputParams,
    dates: &[GameDate],
    batch_size: usize,
) -> Result<Vec<crate::gpu::staging_layout::GpuCandidate>, wgpu::BufferAsyncError> {
    if dates.is_empty() {
        return Ok(Vec::new());
    }

    let mut results = Vec::new();
    let batch = batch_size.max(1);
    let mut inputs = Vec::with_capacity(batch);
    for &date in dates {
        inputs.push(params.with_date(date));
        if inputs.len() >= batch {
            let mut chunk = sha1_kernel::run_sha1_mt_compact(ctx, &inputs).await?;
            results.append(&mut chunk);
            inputs.clear();
        }
    }
    if !inputs.is_empty() {
        let mut chunk = sha1_kernel::run_sha1_mt_compact(ctx, &inputs).await?;
        results.append(&mut chunk);
    }

    Ok(results)
}

pub async fn run_result_base_by_dates(
    ctx: &infra::gpu::context::GpuContext,
    ds_config: DSConfig,
    params: &GpuInputParams,
    dates: &[GameDate],
    batch_size: usize,
) -> Result<Vec<ResultBase>, wgpu::BufferAsyncError> {
    let candidates = run_sha1_mt_compact_by_dates(ctx, params, dates, batch_size).await?;
    Ok(build_result_base_from_candidates(ds_config, candidates))
}

pub async fn run_result_base_seedhigh_by_dates(
    ctx: &infra::gpu::context::GpuContext,
    ds_config: DSConfig,
    params: &GpuInputParams,
    dates: &[GameDate],
    batch_size: usize,
) -> Result<Vec<ResultBase>, wgpu::BufferAsyncError> {
    if dates.is_empty() {
        return Ok(Vec::new());
    }

    let iv_cfg = GpuIvConfig {
        iv_step: params.iv_step,
        _pad0: 0,
        iv_min: params.iv_min,
        iv_max: params.iv_max,
    };
    let seed_highs = mt_kernel::run_mt_seedhigh_candidates_cached(ctx, &iv_cfg).await?;
    if seed_highs.is_empty() {
        return Ok(Vec::new());
    }

    let mut results = Vec::new();
    let batch = batch_size.max(1);
    let mut inputs = Vec::with_capacity(batch);
    for &date in dates {
        inputs.push(params.with_date(date));
        if inputs.len() >= batch {
            let mut chunk = sha1_kernel::run_sha1_seedhigh_filter(ctx, &inputs, &seed_highs).await?;
            results.append(&mut build_result_base_from_candidates(ds_config, chunk));
            inputs.clear();
        }
    }
    if !inputs.is_empty() {
        let mut chunk = sha1_kernel::run_sha1_seedhigh_filter(ctx, &inputs, &seed_highs).await?;
        results.append(&mut build_result_base_from_candidates(ds_config, chunk));
    }

    Ok(results)
}

fn build_result_base_from_candidates(
    ds_config: DSConfig,
    candidates: Vec<crate::gpu::staging_layout::GpuCandidate>,
) -> Vec<ResultBase> {
    let mut results = Vec::with_capacity(candidates.len());
    for cand in candidates {
        let raw_kp = cand.key_presses as u16;
        if !KeyPresses::is_valid_raw(raw_kp) {
            continue;
        }

        let game_time = candidate_game_time(&cand);
        let seed0 = cand.seed0;
        let seed1 = lcg_next(seed0);
        let ivs: [u8; 6] = mt::mt_1(seed1, 0);

        results.push(ResultBase {
            ds_config,
            seed0,
            seed1,
            game_time,
            key_presses: KeyPresses::new(raw_kp),
            ivs,
        });
    }

    results
}
