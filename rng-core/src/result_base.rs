use crate::models::{DSConfig, GameTime, KeyPresses};

#[derive(Debug, Clone)]
pub struct ResultBase {
    pub ds_config: DSConfig,
    pub seed0: u64,
    pub seed1: u64,
    pub game_time: GameTime,
    pub key_presses: KeyPresses,
    pub ivs: [u8; 6],
}
