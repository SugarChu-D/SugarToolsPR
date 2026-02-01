use rng_core::models::{GameTime,KeyPresses};

#[derive(Debug, Clone)]
pub struct SearchResultBase {
    pub initial_seed0: u64,
    pub initial_seed1: u64,
    pub game_time: GameTime,
    pub key_presses: KeyPresses,
    pub ivs_values: [u8; 6],
}