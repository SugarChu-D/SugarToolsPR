#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DateTimeError {
    #[error("Invalid game date {year:02}-{month:02}-{day:02} (years 00 through 99)")]
    InvalidDate { year: u8, month: u8, day: u8 },
    #[error("Invalid time {hour:02}:{minute:02}:{second:02}")]
    InvalidTime { hour: u8, minute: u8, second: u8 },
}
