use super::{date_time_error::DateTimeError, game_date::GameDate};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameTime {
    date: GameDate,
    hour: u8,
    minute: u8,
    second: u8,
}

impl GameTime {
    pub fn try_new(
        year: u8,
        month: u8,
        day: u8,
        hour: u8,
        minute: u8,
        second: u8,
    ) -> Result<Self, DateTimeError> {
        Self::from_date(GameDate::try_new(year, month, day)?, hour, minute, second)
    }

    pub fn from_date(
        date: GameDate,
        hour: u8,
        minute: u8,
        second: u8,
    ) -> Result<Self, DateTimeError> {
        if hour > 23 || minute > 59 || second > 59 {
            return Err(DateTimeError::InvalidTime {
                hour,
                minute,
                second,
            });
        }
        Ok(Self {
            date,
            hour,
            minute,
            second,
        })
    }

    pub fn date(self) -> GameDate {
        self.date
    }

    pub fn year(self) -> u8 {
        self.date.year()
    }

    pub fn month(self) -> u8 {
        self.date.month()
    }

    pub fn day(self) -> u8 {
        self.date.day()
    }

    pub fn weekday(self) -> u8 {
        self.date.weekday()
    }

    pub fn days_in_month(self) -> u8 {
        self.date.days_in_month()
    }

    pub fn get_date8_format(self) -> u32 {
        self.date.get_date8_format()
    }

    pub fn hour(self) -> u8 {
        self.hour
    }

    pub fn minute(self) -> u8 {
        self.minute
    }

    pub fn second(self) -> u8 {
        self.second
    }

    pub fn add_second(&mut self) -> Result<(), DateTimeError> {
        if self.second == 59 {
            self.add_minute()?;
            self.second = 0;
        } else {
            self.second += 1;
        }
        Ok(())
    }

    pub fn add_minute(&mut self) -> Result<(), DateTimeError> {
        if self.minute == 59 {
            self.add_hour()?;
            self.minute = 0;
        } else {
            self.minute += 1;
        }
        Ok(())
    }

    pub fn add_hour(&mut self) -> Result<(), DateTimeError> {
        if self.hour == 23 {
            self.add_day()?;
            self.hour = 0;
        } else {
            self.hour += 1;
        }
        Ok(())
    }

    pub fn add_day(&mut self) -> Result<(), DateTimeError> {
        self.date.add_day()
    }

    pub fn add_month(&mut self) -> Result<(), DateTimeError> {
        self.date.add_month()
    }

    pub fn get_time9_format(&self) -> u32 {
        // もし午後なら、時間に40を加える
        let adjusted_hour: u8 = if self.hour >= 12 {
            self.hour + 40
        } else {
            self.hour
        };

        // 各値を10進数から16進数として解釈
        let hex_hour: u8 = ((adjusted_hour / 10) << 4) | (adjusted_hour % 10);
        let hex_minute: u8 = ((self.minute / 10) << 4) | (self.minute % 10);
        let hex_second: u8 = ((self.second / 10) << 4) | (self.second % 10);

        ((hex_hour as u32) << 24) | ((hex_minute as u32) << 16) | ((hex_second as u32) << 8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_game_date() {
        let date = GameTime::try_new(26, 1, 24, 23, 59, 59).unwrap();
        assert_eq!(date.weekday(), 6); // Saturday
        assert_eq!(date.get_date8_format(), 0x26012406);
        assert_eq!(date.year(), 26);
    }

    #[test]
    fn rejects_invalid_clock_fields() {
        for (hour, minute, second) in [(24, 0, 0), (0, 60, 0), (0, 0, 60)] {
            assert_eq!(
                GameTime::try_new(26, 1, 1, hour, minute, second),
                Err(DateTimeError::InvalidTime {
                    hour,
                    minute,
                    second
                })
            );
        }
    }

    #[test]
    fn carries_across_midnight_and_preserves_the_last_instant_on_overflow() {
        let mut time = GameTime::try_new(98, 12, 31, 23, 59, 59).unwrap();
        time.add_second().unwrap();
        assert_eq!(time, GameTime::try_new(99, 1, 1, 0, 0, 0).unwrap());
        let last = GameTime::try_new(99, 12, 31, 23, 59, 59).unwrap();
        let mut time = last;
        assert!(time.add_second().is_err());
        assert_eq!(time, last);
    }
}
