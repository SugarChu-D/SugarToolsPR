use crate::{SearchError, SearchRuntime};
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use infra::gpu::context::GpuContext;
use rayon::prelude::*;
use rng_core::gpu::helpers::GpuInputParams;
use rng_core::gpu::helpers::run_result_base_seedhigh_by_dates_multi_iv;
use rng_core::gpu::input_layout::GpuIvConfig;
use rng_core::gpu::workgroup_size::WorkgroupSize;
use rng_core::lcg::grotto::Grottos;
use rng_core::lcg::nature::Nature;
use rng_core::lcg::wild_poke::WildPoke;
use rng_core::lcg::{Lcg, OffsetType};
use rng_core::models::DSConfig;
use rng_core::models::game_date::{GameDate, build_date_except_summer};

#[derive(Debug, Clone)]
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
        _ => false,
    }
}

const BATCH_DATES: usize = 256;

/// A signal shared between a Tepig search and its caller.
#[derive(Debug, Clone, Default)]
pub struct TepigSearchCancellation(Arc<AtomicBool>);

impl TepigSearchCancellation {
    /// Requests cancellation for every search using this signal.
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

/// Cumulative counts after a batch has completed CPU processing and seed deduplication.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TepigSearchProgress {
    pub completed_batches: usize,
    pub total_batches: usize,
    pub unique_results: usize,
}

#[derive(Default)]
struct TepigSearchExecution<'a> {
    runtime: Option<&'a SearchRuntime>,
    cancellation: Option<&'a TepigSearchCancellation>,
    on_progress: Option<&'a mut (dyn FnMut(TepigSearchProgress) + Send)>,
}

fn check_cancelled(cancellation: Option<&TepigSearchCancellation>) -> Result<(), SearchError> {
    if cancellation.is_some_and(TepigSearchCancellation::is_cancelled) {
        return Err(SearchError::Cancelled);
    }
    Ok(())
}

pub async fn white2_tepig_dragonite_search(
    config: DSConfig,
    nat: Nature,
    mode: BW2Mode,
    workgroup_size: WorkgroupSize,
) -> Result<Vec<TepigSearchResult>, SearchError> {
    DragoniteSearchRequest {
        config,
        nature: nat,
        mode,
        workgroup_size,
    }
    .run()
    .await
}

/// Searches all eligible dates and reports cumulative progress after each completed batch.
///
/// The callback runs on the search task. Cancellation is cooperative, so an in-flight GPU batch
/// finishes and any CPU worker is joined before the search returns.
pub async fn white2_tepig_dragonite_search_with_control(
    config: DSConfig,
    nat: Nature,
    mode: BW2Mode,
    workgroup_size: WorkgroupSize,
    runtime: Option<&SearchRuntime>,
    cancellation: &TepigSearchCancellation,
    on_progress: impl FnMut(TepigSearchProgress) + Send,
) -> Result<Vec<TepigSearchResult>, SearchError> {
    DragoniteSearchRequest {
        config,
        nature: nat,
        mode,
        workgroup_size,
    }
    .run_with_control(runtime, cancellation, on_progress)
    .await
}

/// Inputs for the White 2 Tepig and Dragonite search across all non-summer dates.
#[derive(Debug, Clone)]
pub struct DragoniteSearchRequest {
    pub config: DSConfig,
    pub nature: Nature,
    pub mode: BW2Mode,
    pub workgroup_size: WorkgroupSize,
}

impl DragoniteSearchRequest {
    /// Returns matching seeds, or the input, GPU or worker error that stopped the search.
    pub async fn run(self) -> Result<Vec<TepigSearchResult>, SearchError> {
        self.run_using(TepigSearchExecution::default()).await
    }

    /// Runs the search with cancellation and cumulative batch progress.
    pub async fn run_with_control(
        self,
        runtime: Option<&SearchRuntime>,
        cancellation: &TepigSearchCancellation,
        mut on_progress: impl FnMut(TepigSearchProgress) + Send,
    ) -> Result<Vec<TepigSearchResult>, SearchError> {
        self.run_using(TepigSearchExecution {
            runtime,
            cancellation: Some(cancellation),
            on_progress: Some(&mut on_progress),
        })
        .await
    }

    async fn run_using(
        self,
        execution: TepigSearchExecution<'_>,
    ) -> Result<Vec<TepigSearchResult>, SearchError> {
        check_cancelled(execution.cancellation)?;
        let dates = build_date_except_summer();
        tepig_search_by_dates(
            self.config,
            self.nature,
            &dates,
            self.mode,
            self.workgroup_size,
            find_grotto_advances_candy_dragonite,
            execution,
        )
        .await
    }
}

pub async fn white2_tepig_search(
    config: DSConfig,
    year: u8,
    month: u8,
    day: u8,
    nat: Nature,
    mode: BW2Mode,
    workgroup_size: WorkgroupSize,
) -> Result<Vec<TepigSearchResult>, SearchError> {
    TepigSearchRequest {
        config,
        date: GameDate::try_new(year, month, day)?,
        nature: nat,
        mode,
        workgroup_size,
    }
    .run()
    .await
}

/// Inputs for the single-date White 2 Tepig search.
#[derive(Debug, Clone)]
pub struct TepigSearchRequest {
    pub config: DSConfig,
    pub date: GameDate,
    pub nature: Nature,
    pub mode: BW2Mode,
    pub workgroup_size: WorkgroupSize,
}

impl TepigSearchRequest {
    /// Returns matching seeds, or the input, GPU or worker error that stopped the search.
    pub async fn run(self) -> Result<Vec<TepigSearchResult>, SearchError> {
        self.run_using(TepigSearchExecution::default()).await
    }

    /// Runs the search using a previously initialized GPU device and queue.
    pub async fn run_with_runtime(
        self,
        runtime: &SearchRuntime,
    ) -> Result<Vec<TepigSearchResult>, SearchError> {
        self.run_using(TepigSearchExecution {
            runtime: Some(runtime),
            ..Default::default()
        })
        .await
    }

    /// Runs the search with cancellation and a per-batch progress callback.
    ///
    /// Pass an existing runtime to reuse its GPU device and queue. The callback runs on the search
    /// task. Cancellation returns `SearchError::Cancelled` once any in-flight GPU batch and CPU
    /// worker finish.
    pub async fn run_with_control(
        self,
        runtime: Option<&SearchRuntime>,
        cancellation: &TepigSearchCancellation,
        mut on_progress: impl FnMut(TepigSearchProgress) + Send,
    ) -> Result<Vec<TepigSearchResult>, SearchError> {
        self.run_using(TepigSearchExecution {
            runtime,
            cancellation: Some(cancellation),
            on_progress: Some(&mut on_progress),
        })
        .await
    }

    async fn run_using(
        self,
        execution: TepigSearchExecution<'_>,
    ) -> Result<Vec<TepigSearchResult>, SearchError> {
        let month = self.date.month();
        if month % 4 == 2 {
            return Err(SearchError::SummerMonth(month));
        }
        tepig_search_by_dates(
            self.config,
            self.nature,
            &[self.date],
            self.mode,
            self.workgroup_size,
            find_grotto_advances_candy,
            execution,
        )
        .await
    }
}

async fn tepig_search_by_dates(
    config: DSConfig,
    nat: Nature,
    dates: &[GameDate],
    mode: BW2Mode,
    workgroup_size: WorkgroupSize,
    find_grotto: fn(u64, u64, u64) -> Vec<(u32, Grottos)>,
    execution: TepigSearchExecution<'_>,
) -> Result<Vec<TepigSearchResult>, SearchError> {
    let batch_size = BATCH_DATES;
    let TepigSearchExecution {
        runtime,
        cancellation,
        on_progress,
    } = execution;
    let mut completed = CompletedTepigBatches::new(dates.len().div_ceil(batch_size), on_progress);
    let mut pending_cpu = None;

    let (iv_min, iv_max): ([u32; 6], [u32; 6]) = match nat.id() {
        4 => ([27, 29, 29, 29, 0, 25], [31, 31, 31, 31, 31, 25]), // Naughty
        19 => ([28, 29, 30, 30, 0, 30], [31, 31, 31, 31, 31, 31]), // Rash
        any => return Err(SearchError::UnsupportedNature(any)),
    };

    check_cancelled(cancellation)?;
    let ctx = match runtime {
        Some(runtime) => runtime.context(workgroup_size)?,
        None => GpuContext::new_with_workgroup_size(workgroup_size.as_u32()).await?,
    };
    let params = GpuInputParams::new(config, [0, 23], [0, 59], [0, 59], 16, iv_min, iv_max);
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

    for date_batch in dates.chunks(batch_size) {
        if check_cancelled(cancellation).is_err() {
            break;
        }
        let base_results = run_result_base_seedhigh_by_dates_multi_iv(
            &ctx, config, &params, date_batch, batch_size, &iv_cfgs,
        )
        .await;
        if let Some(handle) = pending_cpu.take() {
            completed.join(handle, cancellation)?;
        }
        check_cancelled(cancellation)?;
        let base_results = base_results?;
        let nat_clone = nat.clone();
        let worker_cancellation = cancellation.cloned();
        pending_cpu = Some(
            thread::Builder::new()
                .spawn(move || {
                    process_base_results(
                        base_results,
                        mode,
                        nat_clone,
                        find_grotto,
                        worker_cancellation.as_ref(),
                    )
                })
                .map_err(SearchError::WorkerStart)?,
        );
    }
    if let Some(handle) = pending_cpu.take() {
        completed.join(handle, cancellation)?;
    }

    check_cancelled(cancellation)?;
    Ok(completed.results)
}

struct CompletedTepigBatches<'a> {
    results: Vec<TepigSearchResult>,
    seen_seed0: HashSet<u64>,
    progress: TepigSearchProgress,
    on_progress: Option<&'a mut (dyn FnMut(TepigSearchProgress) + Send)>,
}

impl<'a> CompletedTepigBatches<'a> {
    fn new(
        total_batches: usize,
        on_progress: Option<&'a mut (dyn FnMut(TepigSearchProgress) + Send)>,
    ) -> Self {
        Self {
            results: Vec::new(),
            seen_seed0: HashSet::new(),
            progress: TepigSearchProgress {
                completed_batches: 0,
                total_batches,
                unique_results: 0,
            },
            on_progress,
        }
    }

    fn join(
        &mut self,
        worker: thread::JoinHandle<Vec<TepigSearchResult>>,
        cancellation: Option<&TepigSearchCancellation>,
    ) -> Result<(), SearchError> {
        let batch_results = worker.join().map_err(|_| SearchError::WorkerPanicked)?;
        check_cancelled(cancellation)?;
        for candidate in batch_results {
            if self.seen_seed0.insert(candidate.seed0) {
                self.results.push(candidate);
            }
        }
        self.progress.completed_batches += 1;
        self.progress.unique_results = self.results.len();
        if let Some(on_progress) = &mut self.on_progress {
            on_progress(self.progress);
        }
        Ok(())
    }
}

fn process_base_results(
    base_results: Vec<rng_core::result_base::ResultBase>,
    mode: BW2Mode,
    nat: Nature,
    find_grotto: fn(u64, u64, u64) -> Vec<(u32, Grottos)>,
    cancellation: Option<&TepigSearchCancellation>,
) -> Vec<TepigSearchResult> {
    base_results
        .into_par_iter()
        .map(|base| {
            if check_cancelled(cancellation).is_err() {
                None
            } else {
                Some(base)
            }
        })
        .while_some()
        .filter_map(|base| {
            let seed0 = base.seed0;
            let seed1 = base.seed1;

            let mut rng: Lcg = Lcg::new(seed0);
            let offset = match mode {
                BW2Mode::Normal => rng.offset_seed0(OffsetType::BW2Start),
                BW2Mode::Challenge => rng.offset_seed0(OffsetType::BW2StartChallengeMode),
            };

            let candy_frames = find_grotto(seed0, FRAME_MIN_FOR_CANDY, FRAME_MAX_FOR_CANDY);
            if candy_frames.is_empty() {
                return None;
            }

            let ivs_16: [u8; 6] = base.ivs;
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
                    tepig_frames.push(((frame + offset) & 0xFFFFFFFF) as u32);
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

            Some(TepigSearchResult {
                seed0,
                seed1,
                year: base.game_time.year(),
                month: base.game_time.month(),
                day: base.game_time.day(),
                hour: base.game_time.hour(),
                minute: base.game_time.minute(),
                second: base.game_time.second(),
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
    let ability_ok = duck.ability().is_some_and(|v| v == 0);

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
        let index19_ok = grottos
            .get(GROTTO_INDEX_2)
            .is_some_and(|grotto| grotto.slot() == Some(GROTTO_SLOT_2));
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
        use rng_core::lcg::tid_impl::get_frigate_pass;

        println!("seed0: {:016X} seed1: {:016X} ", self.seed0, self.seed1,);
        println!(
            "date: {:02}/{:02}/{:02} {:02}:{:02}:{:02} key={}",
            self.year, self.month, self.day, self.hour, self.minute, self.second, self.key_presses,
        );
        println!("TID: {} Pass: {}", self.tid, get_frigate_pass(self.tid));
        println!("ivs={:?} iv_step={}", self.ivs, self.tepig_iv_step);
        println!("Tepig frame: {:?}", self.tepig_frames);
        println!("Pidove:");
        for pidove in &self.pidove_frames {
            println!(
                "{}:{} {}",
                pidove.0,
                if pidove.1.slot.is_some_and(|s| s < 20) {
                    "Lv.2"
                } else {
                    "Lv.4"
                },
                pidove.1.nature.clone().unwrap().name(),
            )
        }

        println!("PsyDuck:");
        for psyduck in &self.psyduck_frames {
            println!("{}:{}", psyduck.0, psyduck.1.nature.clone().unwrap().name(),)
        }

        print!("candy:");
        for candy in &self.candy_frames {
            println!("{}: ", candy.0);
            for (i, grotto) in candy.1.grottos.iter().enumerate() {
                if grotto.slot().is_some() {
                    println!("#{}: {:?}", i, grotto);
                }
            }
        }
        println!();
    }
}

#[cfg(test)]
mod tests {
    use rng_core::gpu::workgroup_size::WorkgroupSize::W256;

    use super::*;
    use std::time::Instant;

    fn test_config() -> DSConfig {
        DSConfig::new(
            rng_core::models::GameVersion::White2,
            rng_core::models::Region::JPN,
            0x10fa,
            false,
            0x0009bf6d93ce,
        )
    }

    #[test]
    fn cancelled_search_exits_before_gpu_setup() {
        let cancellation = TepigSearchCancellation::default();
        cancellation.clone().cancel();
        let request = TepigSearchRequest {
            config: test_config(),
            date: GameDate::try_new(26, 3, 1).unwrap(),
            nature: Nature::new(4),
            mode: BW2Mode::Normal,
            workgroup_size: W256,
        };
        assert!(matches!(
            pollster::block_on(request.run_with_control(None, &cancellation, |_| {
                panic!("a cancelled search cannot complete a batch")
            })),
            Err(SearchError::Cancelled)
        ));
    }

    fn result_with_seed(seed0: u64) -> TepigSearchResult {
        TepigSearchResult {
            seed0,
            seed1: 0,
            year: 26,
            month: 3,
            day: 1,
            hour: 0,
            minute: 0,
            second: 0,
            tid: 0,
            key_presses: String::new(),
            ivs: [0; 6],
            tepig_iv_step: 16,
            tepig_frames: Vec::new(),
            candy_frames: Vec::new(),
            pidove_frames: Vec::new(),
            psyduck_frames: Vec::new(),
        }
    }

    #[test]
    fn progress_counts_unique_results_across_completed_batches() {
        let mut updates = Vec::new();
        let mut record = |progress| updates.push(progress);
        let mut completed = CompletedTepigBatches::new(3, Some(&mut record));
        completed
            .join(
                thread::spawn(|| vec![result_with_seed(1), result_with_seed(1)]),
                None,
            )
            .unwrap();
        completed
            .join(
                thread::spawn(|| vec![result_with_seed(1), result_with_seed(2)]),
                None,
            )
            .unwrap();
        completed.join(thread::spawn(Vec::new), None).unwrap();
        assert_eq!(
            updates,
            [
                TepigSearchProgress {
                    completed_batches: 1,
                    total_batches: 3,
                    unique_results: 1,
                },
                TepigSearchProgress {
                    completed_batches: 2,
                    total_batches: 3,
                    unique_results: 2,
                },
                TepigSearchProgress {
                    completed_batches: 3,
                    total_batches: 3,
                    unique_results: 2,
                },
            ]
        );
    }

    #[test]
    fn cancellation_joins_worker_before_returning() {
        let cancellation = TepigSearchCancellation::default();
        let worker_cancellation = cancellation.clone();
        let worker = thread::spawn(move || {
            worker_cancellation.cancel();
            vec![result_with_seed(1)]
        });
        let mut record = |_| panic!("a cancelled batch cannot report completion");
        let mut completed = CompletedTepigBatches::new(1, Some(&mut record));
        assert!(matches!(
            completed.join(worker, Some(&cancellation)),
            Err(SearchError::Cancelled)
        ));
        assert!(completed.results.is_empty());
    }

    #[test]
    fn cancelled_cpu_batch_skips_candidate_processing() {
        use rng_core::models::{GameTime, KeyPresses};
        use rng_core::result_base::ResultBase;

        let cancellation = TepigSearchCancellation::default();
        cancellation.cancel();
        let base = ResultBase {
            ds_config: test_config(),
            seed0: 1,
            seed1: 2,
            game_time: GameTime::try_new(26, 3, 1, 0, 0, 0).unwrap(),
            key_presses: KeyPresses::new(0x2fff),
            ivs: [0; 6],
        };
        assert!(
            process_base_results(
                vec![base; 8],
                BW2Mode::Normal,
                Nature::new(4),
                |_, _, _| panic!("cancelled candidates cannot reach filtering"),
                Some(&cancellation),
            )
            .is_empty()
        );
    }

    #[test]
    fn rejects_invalid_conditions_before_gpu_setup() {
        let config = test_config();
        let search = |year, month, day, nature| {
            pollster::block_on(white2_tepig_search(
                config,
                year,
                month,
                day,
                Nature::new(nature),
                BW2Mode::Normal,
                W256,
            ))
        };
        for (year, month, day) in [(100, 1, 1), (0, 0, 1), (0, 13, 1), (0, 1, 0), (0, 4, 31)] {
            assert!(matches!(
                search(year, month, day, 4),
                Err(SearchError::InvalidDateTime(_))
            ));
        }
        assert!(matches!(
            search(0, 6, 1, 4),
            Err(SearchError::SummerMonth(6))
        ));
        assert!(matches!(
            search(0, 3, 1, 25),
            Err(SearchError::UnsupportedNature(25))
        ));
    }

    #[test]
    #[ignore]
    fn test_white2_tepig_single_date() {
        let ds_config = DSConfig {
            version: rng_core::models::GameVersion::White2,
            region: rng_core::models::Region::JPN,
            timer0: 0x10FA,
            is_dslite: false,
            mac_address: 0x0009bf6d93ce,
        };

        let start = Instant::now();
        let results = pollster::block_on(async {
            white2_tepig_search(ds_config, 33, 8, 27, Nature::new(4), BW2Mode::Normal, W256).await // 例: Rash
        })
        .expect("search failed");
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
        let ds_config = DSConfig {
            version: rng_core::models::GameVersion::White2,
            region: rng_core::models::Region::JPN,
            timer0: 0x10FA,
            is_dslite: false,
            mac_address: 0x0009bf6d93ce,
        };

        let start = Instant::now();
        let results = pollster::block_on(async {
            white2_tepig_dragonite_search(ds_config, Nature::new(4), BW2Mode::Normal, W256).await // 例: Naughty
        })
        .expect("search failed");
        let elapsed = start.elapsed();

        println!("Elapsed: {:?}", elapsed);
        println!("Total results: {}", results.len());
        for r in results.iter() {
            r.print();
        }
    }
}
