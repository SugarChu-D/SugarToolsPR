use sha1::{Sha1, Digest};
use crate::models::{game_date::GameDate, gameVersion::GameVersion};
use crate::models::DSConfig::DSConfig;
use crate::utils::byte_utils::u32_to_bytes_le;

pub fn generate_initial_seed(config: &DSConfig, game_date: &GameDate) -> u32 {
}
