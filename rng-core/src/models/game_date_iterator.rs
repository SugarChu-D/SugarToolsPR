use super::field_range::FieldRange;
use super::game_date::GameDate;

#[derive(Debug, Clone, Copy)]
pub struct GameDateSpec {
    pub year: FieldRange<u8>,
    pub month: FieldRange<u8>,
    pub day: FieldRange<u8>,
}

impl GameDateSpec {
    pub fn start(self) -> Option<GameDate> {
        GameDateIterator::new(self).next()
    }
}

pub struct GameDateIterator {
    dates: Box<dyn Iterator<Item = GameDate> + Send + Sync>,
}

impl GameDateIterator {
    pub fn new(spec: GameDateSpec) -> Self {
        let dates = (spec.year.min..=spec.year.max.min(99)).flat_map(move |year| {
            (spec.month.min.max(1)..=spec.month.max.min(12)).flat_map(move |month| {
                (spec.day.min.max(1)..=spec.day.max.min(31))
                    .filter_map(move |day| GameDate::try_new(year, month, day).ok())
            })
        });
        Self {
            dates: Box::new(dates),
        }
    }
}

impl Iterator for GameDateIterator {
    type Item = GameDate;
    fn next(&mut self) -> Option<GameDate> {
        self.dates.next()
    }
}
