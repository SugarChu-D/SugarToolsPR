#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct GameDate {
    pub year: u8,
    pub month: u8,
    pub day: u8,
}

impl GameDate {
    pub fn new(year: u8, month: u8, day: u8) -> Self{
        Self { year, month, day }
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
        match self.month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => {
                if self.year % 4 == 0 && self.year % 100 != 0 {
                    29
                } else {
                    28
                }
            }
            _ => 0, // Invalid month
        }
    }

    pub fn add_day(&mut self) {
        self.day += 1;
        if self.day > self.days_in_month() {
            self.day = 1;
            self.add_month();
        }
    }

    pub fn add_month(&mut self) {
        self.month += 1;
        if self.month > 12 {
            self.month = 1;
            self.year += 1;
        }
    }

    pub fn get_date8_format(&self) -> u32 {
        // 10進数を16進数に変換する処理を直接ビット演算で行う
        let hex_year = ((self.year / 10) << 4) | (self.year % 10);
        let hex_month = ((self.month / 10) << 4) | (self.month % 10);
        let hex_day = ((self.day / 10) << 4) | (self.day % 10);

        ((hex_year as u32) << 24) | ((hex_month as u32) << 16) | ((hex_day as u32) << 8) | self.weekday() as u32
    }
}
