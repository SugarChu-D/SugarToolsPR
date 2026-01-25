use crate::models::{DSConfig, GameDate, KeyPresses};

#[derive(Debug, Clone)]
pub struct SearchConfigBase {
    pub DSConfig: DSConfig,
    pub GameDate: GameDate,
    pub KeyPresses: KeyPresses,
}

pub struct SearchResultBase {
    pub initial_seed0: u64,
    pub initial_seed1: u64,
    pub GameDate: GameDate,
    pub KeyPresses: KeyPresses,
    pub ivs_values: [u8; 6],
}