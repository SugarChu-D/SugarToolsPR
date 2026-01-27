use rng_core::models::{GameDate,KeyPresses};

#[derive(Debug, Clone)]
pub struct SearchResultBase {
    pub initial_seed0: u64,
    pub initial_seed1: u64,
    pub game_date: GameDate,
    pub key_presses: KeyPresses,
    pub ivs_values: [u8; 6],
}