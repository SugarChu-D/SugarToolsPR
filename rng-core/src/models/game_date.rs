#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameDate {
    pub year: u8,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

impl GameDate {
    pub fn new(year: u8, month: u8, day: u8, hour: u8, minute: u8, second: u8) -> Self {
        Self {
            year,
            month,
            day,
            hour,
            minute,
            second,
        }
    }

    pub fn weekday(&self) -> u8 {
        // Zeller's Congruence algorithm to calculate the day of the week
        let mut m = self.month as i32;
        let mut y = self.year as i32;

        if m < 3 {
            m += 12;
            y = if y == 0 { 93 } else { y - 1 }; // Adjust for year wrap-around
        }

        let weekday = (self.day as i32 + ((13 * m + 8) / 5) + y + (y >> 2)) % 7;

        weekday as u8 // 0 = Sunday, 1 = Monday, ..., 6 = Saturday
    }

    pub fn get_days_in_month(month: u8, year: u8) -> u8 {
        match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => {
                if year % 4 == 0 && year % 100 != 0 {
                    29
                } else {
                    28
                }
            }
            _ => 0, // Invalid month
        }
    }

    pub fn add_days(&mut self, days: u32) {
        let mut total_days = days as u8;
        while total_days > 0 {
            let days_in_month = GameDate::get_days_in_month(self.month, self.year);
            if self.day + total_days <= days_in_month {
                self.day += total_days;
                break;
            } else {
                total_days -= days_in_month - self.day + 1;
                self.day = 1;
                if self.month == 12 {
                    self.month = 1;
                    self.year = (self.year + 1) % 100; // Wrap around after year 99
                } else {
                    self.month += 1;
                }
            }
        }
    }

    pub fn get_date8_format(&self) -> u32 {
        // 10進数を16進数に変換する処理を直接ビット演算で行う
        let hex_year = ((self.year / 10) << 4) | (self.year % 10);
        let hex_month = ((self.month / 10) << 4) | (self.month % 10);
        let hex_day = ((self.day / 10) << 4) | (self.day % 10);

        ((hex_year as u32) << 24) | ((hex_month as u32) << 16) | ((hex_day as u32) << 8) | self.weekday() as u32
    }

    pub fn get_time9_format(&self) -> u32 {
        // もし午後なら、時間に40を加える
        let adjusted_hour: u8 = if self.hour >= 12 { self.hour + 40 } else { self.hour };

        // 各値を10進数から16進数として解釈
        let hex_hour: u8 = ((adjusted_hour / 10) << 4) | (adjusted_hour % 10);
        let hex_minute: u8 = ((self.minute / 10) << 4) | (self.minute % 10);
        let hex_second: u8 = ((self.second / 10) << 4) | (self.second % 10);

        ((hex_hour as u32) << 24) | ((hex_minute as u32) << 16) | ((hex_second as u32) << 8) | 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_game_date() {
        let mut date = GameDate::new(26, 1, 24, 23, 59, 59);
        assert_eq!(date.weekday(), 6); // Saturday
        assert_eq!(date.get_date8_format(), 0x26012406);
        date.add_days(1);
        assert_eq!(date.year, 26);
    }
}
