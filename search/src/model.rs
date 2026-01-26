use rng_core::models::KeyPresses;

#[derive(Debug, Clone)]
pub struct SearchResultBase {
    pub initial_seed: u64,
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub key_presses: KeyPresses,
    pub ivs: [u8; 6],
}