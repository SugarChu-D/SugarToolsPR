#[derive (Debug, Clone, Copy, PartialEq, Eq)]
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

    fn weekday(&self) -> u8 {
        // Zeller's Congruence algorithm to calculate the day of the week
        let mut m = self.month as i32;
        let mut y = self.year as i32;

        if m < 3 {
            m += 12;
            y = if y == 0 { 93 } else { y-1 }; // Adjust for year wrap-around
        }

        let k = y % 100;
        let j = y / 100;

        let f = self.day as i32 + ((13 * (m + 1)) / 5) + k + (k / 4) + (j / 4) - (2 * j);
        let weekday = ((f % 7) + 7) % 7; // Ensure non-negative result

        weekday as u8 // 0 = Saturday, 1 = Sunday, ..., 6 = Friday
    }
}
