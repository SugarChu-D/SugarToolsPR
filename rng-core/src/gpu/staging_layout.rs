use bytemuck::{Pod, Zeroable};
use crate::models::GameTime;

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

pub fn candidate_game_time(candidate: &GpuCandidate) -> GameTime {
    let (year, month, day) = decode_date8(candidate.game_date);
    let (hour, minute, second) = decode_time9(candidate.game_time);
    GameTime::new(year, month, day, hour, minute, second)
}
