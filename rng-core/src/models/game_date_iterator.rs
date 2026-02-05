use super::game_date::GameDate;
use super::field_range::FieldRange;

#[derive(Debug, Clone, Copy)]
pub struct GameDateSpec {
    pub year: FieldRange<u8>,
    pub month: FieldRange<u8>,
    pub day: FieldRange<u8>,
}

/**
 * 検索範囲の最小を定める
 */
impl GameDateSpec {
    pub fn start(&self) -> GameDate {
        GameDate {
            year: self.year.min,
            month: self.month.min,
            day: self.day.min,
        }
    }
}

// イテレータ実装
pub struct GameDateIterator {
    current: GameDate,
    spec: GameDateSpec,
    finished: bool,
}

impl Iterator for GameDateIterator {
    type Item = GameDate;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        let out = self.current;
        self.advance();
        Some(out)
    }
}

impl GameDateIterator {
    pub fn new(spec: GameDateSpec) -> Self {
        Self {
            current: spec.start(),
            spec,
            finished: false,
        }
    }

    #[inline]
    fn advance(&mut self) {
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
