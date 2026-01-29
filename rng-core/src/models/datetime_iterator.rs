#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Date {
    pub month: u8,
    pub day: u8,
}

impl Date {
    pub fn next_day(&self) -> Option<Self> {
        let days_in_month = match self.month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => 29, // うるう年は次のイテレータで判断
            _ => return None,
        };
        
        if self.day < days_in_month {
            Some(Self { month: self.month, day: self.day + 1 })
        } else if self.month < 12 {
            Some(Self { month: self.month + 1, day: 1 })
        } else {
            None
        }
    }
}

pub fn weekday(year: u8, date: &Date ) -> u32 {
    // Zeller's Congruence algorithm to calculate the day of the week
    let (y, m) = if date.month < 3 {
        let temp_y = if year == 0 {94} else {year - 1};
        (temp_y as i32, date.month as i32 + 12)
    } else {
        (year as i32, date.month as i32)
    };


    let weekday = (date.day as i32 + ((13 * m + 8) / 5) + y + (y >> 2)) % 7;

    weekday as u32 // 0 = Sunday, 1 = Monday, ..., 6 = Saturday
}

#[derive(Debug, Clone, Copy)]
pub struct DateRange {
    pub year_range: (u8, u8),
    pub date_start: Date,
    pub date_end: Date,
    pub hour_range: (u8, u8),
    pub minute_range: (u8, u8),
    pub second_range: (u8, u8),
}

// イテレータ実装
pub struct DateTimeIterator {
    range: DateRange,
    current_year: u8,
    current_date: Date,
    current_hour: u8,
    current_minute: u8,
    current_second: u8,
}

impl Iterator for DateTimeIterator {
    type Item = u32;
    
    fn next(&mut self) -> Option<Self::Item> {
        if self.current_year > self.range.year_range.1 {
            return None;
        }
        let result = self.pack();
        self.step();
        Some(result)
    }
}

impl DateTimeIterator {
    pub fn new(
        range: DateRange,
    ) -> Self {
        Self {
            current_year: range.year_range.0,
            current_date: range.date_start,
            current_hour: range.hour_range.0,
            current_minute: range.minute_range.0,
            current_second: range.second_range.0,
            range,
        }
    }

    fn step(&mut self) {
        // return false when iteration should stop

        self.current_second += 1;
        if self.current_second <= self.range.second_range.1 {
            return;
        }
        self.current_second = self.range.second_range.0;

        self.current_minute += 1;
        if self.current_minute <= self.range.minute_range.1 {
            return;
        }
        self.current_minute = self.range.minute_range.0;

        self.current_hour += 1;
        if self.current_hour <= self.range.hour_range.1 {
            return;
        }
        self.current_hour = self.range.hour_range.0;

        if let Some(next_date) = self.current_date.next_day() {
            // うるう年未対応
            if next_date <= self.range.date_end {
                self.current_date = next_date;
                return;
            }
        }

        self.current_date = self.range.date_start;
        self.current_year += 1;
    }

    fn pack(&self) -> u32 {
        let hex_year = (((self.current_year / 10) << 4)
                            | (self.current_year % 10)) as u32;
        let hex_month = (((self.current_date.month / 10) << 4)
                            | (self.current_date.month % 10)) as u32;
        let hex_day = (((self.current_date.day / 10) << 4)
                            | (self.current_date.day % 10)) as u32;

        (hex_year << 24)
            | (hex_month << 16)
            | (hex_day << 8)
            | weekday(self.current_year, &self.current_date)
    }
}


#[cfg(test)]
mod tests{
    use super::*;

    #[test]
    fn test_weekday(){
        let date = Date{month:1,day:29};
        assert_eq!(weekday(26, &date), 4);
    }
    #[test]
    fn test_datetime_iterator_first_item(){
        let range = DateRange{
            year_range: (26, 26),
            date_start: Date{ month: 1, day: 30},
            date_end: Date {month: 1, day: 30},
            hour_range: (0,0),
            minute_range: (32, 32),
            second_range: (5, 5)
        };
        let it = DateTimeIterator::new(range);
        let first = it.into_iter().next().unwrap();

        assert_eq!(first, 0x26013005)
    }

    #[test]
    fn test_datetime_iterator_end() {
        let range = DateRange{
            year_range: (26, 26),
            date_start: Date{ month: 2, day: 28},
            date_end: Date {month: 3, day: 1},
            hour_range: (0,0),
            minute_range: (32, 32),
            second_range: (5, 5)
        };
        let it = DateTimeIterator::new(range);
        let v: Vec<_> = it.collect();

        assert_eq!(v.len(), 2);
    }
}