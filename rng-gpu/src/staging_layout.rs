use bytemuck::{Pod, Zeroable};
use rng_core::models::GameTime;
use rng_core::models::date_time_error::DateTimeError;

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct GpuCandidate {
    pub seed0: u64,
    pub game_date: u32,
    pub game_time: u32,
    pub timer0: u32,
    pub key_presses: u32,
}

fn bcd_to_u8(v: u8) -> u8 {
    ((v >> 4) * 10) + (v & 0x0f)
}

pub fn decode_date8(date8: u32) -> (u8, u8, u8) {
    let year = bcd_to_u8(((date8 >> 24) & 0xff) as u8);
    let month = bcd_to_u8(((date8 >> 16) & 0xff) as u8);
    let day = bcd_to_u8(((date8 >> 8) & 0xff) as u8);
    (year, month, day)
}

pub fn decode_time9(time9: u32) -> (u8, u8, u8) {
    let mut hour = bcd_to_u8(((time9 >> 24) & 0xff) as u8);
    if hour >= 40 {
        hour -= 40;
    }
    let minute = bcd_to_u8(((time9 >> 16) & 0xff) as u8);
    let second = bcd_to_u8(((time9 >> 8) & 0xff) as u8);
    (hour, minute, second)
}

pub fn candidate_game_time(candidate: &GpuCandidate) -> Result<GameTime, DateTimeError> {
    let (year, month, day) = decode_date8(candidate.game_date);
    let (hour, minute, second) = decode_time9(candidate.game_time);
    GameTime::try_new(year, month, day, hour, minute, second)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_a_candidate_with_an_impossible_date_or_time() {
        let mut candidate = GpuCandidate {
            game_date: 0x26043100,
            ..GpuCandidate::zeroed()
        };
        assert!(matches!(
            candidate_game_time(&candidate),
            Err(DateTimeError::InvalidDate { .. })
        ));
        candidate.game_date = 0x26043000;
        candidate.game_time = 0x00600000;
        assert!(matches!(
            candidate_game_time(&candidate),
            Err(DateTimeError::InvalidTime { .. })
        ));
    }
}
