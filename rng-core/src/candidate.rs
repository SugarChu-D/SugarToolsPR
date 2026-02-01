use crate::models::{DSConfig, GameTime, GameTimeSpec, KeyPresses};

pub trait CandidateGenerator {
    /// seed生成+MT計算+IV抽出まで実行
    fn generate_candidates(
        &self,
        ds_config: DSConfig,
        time_spec: GameTimeSpec,
        mt_step: u8,
    ) -> Box<dyn Iterator<Item = Candidate>>;
}

#[derive(Debug, Clone)]
pub struct Candidate {
    pub ds_config: DSConfig,
    pub seed0: u64,
    pub seed1: u64,
    pub game_time: GameTime,
    pub key_presses: KeyPresses,
    pub ivs: [u8; 6],
}

//CPU版
pub struct CpuCandidateGenerator;
impl CpuCandidateGenerator for CandidateGenerator {
    fn generate_candidates(...) -> Box<dyn Iterator<Item = Candidate>> {
        Box::new(
            InitialSeed
        )
    }
}
