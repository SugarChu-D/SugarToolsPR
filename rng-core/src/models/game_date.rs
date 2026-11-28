use super::date_time_error::DateTimeError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct GameDate {
    year: u8,
    month: u8,
    day: u8,
}

impl GameDate {
    pub fn try_new(year: u8, month: u8, day: u8) -> Result<Self, DateTimeError> {
        if year > 99 || !(1..=days_in_month(year, month)).contains(&day) {
            return Err(DateTimeError::InvalidDate { year, month, day });
        }
        Ok(Self { year, month, day })
    }

    pub fn year(self) -> u8 {
        self.year
    }

    pub fn month(self) -> u8 {
        self.month
    }

    pub fn day(self) -> u8 {
        self.day
    }

    pub fn weekday(&self) -> u8 {
        // Zeller's Congruence algorithm to calculate the day of the week
        let mut m = self.month as i32;
        let mut y = self.year as i32;

        if m < 3 {
            m += 12;
            y = if y == 0 { 94 } else { y - 1 }; // Adjust for year wrap-around
        }

        let weekday = (self.day as i32 + ((13 * m + 8) / 5) + y + (y >> 2)) % 7;

        weekday as u8 // 0 = Sunday, 1 = Monday, ..., 6 = Saturday
    }

    pub fn days_in_month(&self) -> u8 {
        days_in_month(self.year, self.month)
    }

    pub fn add_day(&mut self) -> Result<(), DateTimeError> {
        if self.day < self.days_in_month() {
            self.day += 1;
        } else {
            *self = self.next_month(1)?;
        }
        Ok(())
    }

    pub fn add_month(&mut self) -> Result<(), DateTimeError> {
        *self = self.next_month(self.day)?;
        Ok(())
    }

    fn next_month(self, day: u8) -> Result<Self, DateTimeError> {
        if self.month == 12 {
            Self::try_new(self.year + 1, 1, day)
        } else {
            Self::try_new(self.year, self.month + 1, day)
        }
    }

    pub fn get_date8_format(&self) -> u32 {
        // 10進数を16進数に変換する処理を直接ビット演算で行う
        let hex_year = ((self.year / 10) << 4) | (self.year % 10);
        let hex_month = ((self.month / 10) << 4) | (self.month % 10);
        let hex_day = ((self.day / 10) << 4) | (self.day % 10);

        ((hex_year as u32) << 24)
            | ((hex_month as u32) << 16)
            | ((hex_day as u32) << 8)
            | self.weekday() as u32
    }
}

// The DS calendar covers 2000..=2099, including the leap year 2000.
fn days_in_month(year: u8, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year.is_multiple_of(4) => 29,
        2 => 28,
        _ => 0,
    }
}

pub fn build_autumn_and_winter() -> Vec<GameDate> {
    let mut dates = Vec::new();
    for year in 0..=99u8 {
        for month in 1..=12u8 {
            if month % 4 == 2 || month % 4 == 1 {
                continue;
            }
            let days = days_in_month(year, month);
            for day in 1..=days {
                dates.push(GameDate { year, month, day });
            }
        }
    }
    dates
}

pub fn build_date_except_summer() -> Vec<GameDate> {
    let mut dates = Vec::new();
    for year in 0..=99u8 {
        for month in 1..=12u8 {
            if month % 4 == 2 {
                continue;
            }
            let days = days_in_month(year, month);
            for day in 1..=days {
                dates.push(GameDate { year, month, day });
            }
        }
    }
    dates
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_dates_in_the_ds_calendar() {
        for (year, month, day) in [
            (100, 1, 1),
            (0, 0, 1),
            (0, 13, 1),
            (0, 1, 0),
            (26, 4, 31),
            (1, 2, 29),
        ] {
            assert_eq!(
                GameDate::try_new(year, month, day),
                Err(DateTimeError::InvalidDate { year, month, day })
            );
        }
        for year in [0, 4, 96] {
            let mut date = GameDate::try_new(year, 2, 28).unwrap();
            date.add_day().unwrap();
            assert_eq!(date, GameDate::try_new(year, 2, 29).unwrap());
            date.add_day().unwrap();
            assert_eq!(date, GameDate::try_new(year, 3, 1).unwrap());
        }
    }

    #[test]
    fn adding_a_month_rejects_a_missing_day() {
        let jan = GameDate::try_new(26, 1, 31).unwrap();
        let mut date = jan;
        assert!(date.add_month().is_err());
        assert_eq!(date, jan);
    }
}
