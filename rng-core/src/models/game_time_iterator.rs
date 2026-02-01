use super::game_time::*;
use super::Field_Range::*;

#[derive(Debug, Clone, Copy)]
pub struct GameTimeSpec {
    pub year: FieldRange<u8>,
    pub month: FieldRange<u8>,
    pub day: FieldRange<u8>,
    pub hour: FieldRange<u8>,
    pub minute: FieldRange<u8>,
    pub second: FieldRange<u8>,
}

/**
 * 検索範囲の最小を定める
 */
impl GameTimeSpec {
    pub fn start(&self) -> GameTime {
        GameTime {
            year: self.year.min,
            month: self.month.min,
            day: self.day.min,
            hour: self.hour.min,
            minute: self.minute.min,
            second: self.second.min,
        }
    }
}


// イテレータ実装
pub struct GameTimeIterator {
    current: GameTime,
    spec: GameTimeSpec,
    finished: bool,
}

impl Iterator for GameTimeIterator {
    type Item = GameTime;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        let out = self.current;
        self.advance();
        Some(out)
    }
}


impl GameTimeIterator {
    pub fn new(spec: GameTimeSpec) -> Self {
        Self {
            current: spec.start(),
            spec,
            finished: false,
        }
    }

    #[inline]
    fn advance(&mut self) {
        // 秒
        self.current.second += 1;
        if self.current.second <= self.spec.second.max {
            return;
        }
        self.current.second = self.spec.second.min;

        // 分
        self.current.minute += 1;
        if self.current.minute <= self.spec.minute.max {
            return;
        }
        self.current.minute = self.spec.minute.min;

        // 時
        self.current.hour += 1;
        if self.current.hour <= self.spec.hour.max {
            return;
        }
        self.current.hour = self.spec.hour.min;

        // 日
        self.current.day += 1;
        let dim = self.current.days_in_month();
        if self.current.day <= dim && self.spec.day.contains(self.current.day) {
            return;
        }
        self.current.day = self.spec.day.min;

        // 月
        self.current.month += 1;
        if self.current.month <= self.spec.month.max {
            return;
        }
        self.current.month = self.spec.month.min;

        // 年
        self.current.year += 1;
        if self.current.year <= self.spec.year.max {
            return;
        }

        // 完全終了
        self.finished = true;
    }
}




#[cfg(test)]
mod tests{
    use super::*;

    #[test]
    fn iter_single_value() {
        let spec = GameTimeSpec {
            year: FieldRange { min: 26, max: 26 },
            month: FieldRange { min: 1, max: 1 },
            day: FieldRange { min: 1, max: 1 },
            hour: FieldRange { min: 0, max: 0 },
            minute: FieldRange { min: 0, max: 0 },
            second: FieldRange { min: 0, max: 0 },
        };

        let mut it = GameTimeIterator::new(spec);
        let v = it.next().unwrap();

        assert_eq!(v.year, 26);
        assert_eq!(v.month, 1);
        assert_eq!(v.day, 1);
        assert_eq!(v.hour, 0);
        assert_eq!(v.minute, 0);
        assert_eq!(v.second, 0);

        assert!(it.next().is_none());
    }


    #[test]
    fn iter_second_carry() {
            let spec = GameTimeSpec {
            year: FieldRange { min: 26, max: 26 },
            month: FieldRange { min: 1, max: 1 },
            day: FieldRange { min: 1, max: 1 },
            hour: FieldRange { min: 0, max: 0 },
            minute: FieldRange { min: 0, max: 1 },
            second: FieldRange { min: 58, max: 59 },
        };

        let mut it = GameTimeIterator::new(spec);

        let a = it.next().unwrap();
        let b = it.next().unwrap();
        let c = it.next().unwrap();
        let d = it.next().unwrap();

        assert_eq!((a.minute, a.second), (0, 58));
        assert_eq!((b.minute, b.second), (0, 59));
        assert_eq!((c.minute, c.second), (1, 58));
        assert_eq!((d.minute, d.second), (1, 59));

        assert!(it.next().is_none());
    }

    #[test]
    fn iter_month_end() {
        let spec = GameTimeSpec {
            year: FieldRange { min: 26, max: 26 },
            month: FieldRange { min: 1, max: 2 },
            day: FieldRange { min: 31, max: 31 },
            hour: FieldRange { min: 0, max: 0 },
            minute: FieldRange { min: 0, max: 0 },
            second: FieldRange { min: 0, max: 0 },
        };

        let mut it = GameTimeIterator::new(spec);

        let jan = it.next().unwrap();
        let feb = it.next().unwrap();

        assert_eq!((jan.month, jan.day), (1, 31));
        assert_eq!((feb.month, feb.day), (2, 31)); // ← ここで止まる設計なら OK

        assert!(it.next().is_none());
    }
}