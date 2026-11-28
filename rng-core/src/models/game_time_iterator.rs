use super::field_range::*;
use super::game_date_iterator::{GameDateIterator, GameDateSpec};
use super::game_time::*;

#[derive(Debug, Clone, Copy)]
pub struct GameTimeSpec {
    pub year: FieldRange<u8>,
    pub month: FieldRange<u8>,
    pub day: FieldRange<u8>,
    pub hour: FieldRange<u8>,
    pub minute: FieldRange<u8>,
    pub second: FieldRange<u8>,
}

impl GameTimeSpec {
    pub fn start(self) -> Option<GameTime> {
        GameTimeIterator::new(self).next()
    }
}

pub struct GameTimeIterator {
    times: Box<dyn Iterator<Item = GameTime> + Send + Sync>,
}

impl GameTimeIterator {
    pub fn new(spec: GameTimeSpec) -> Self {
        let dates = GameDateIterator::new(GameDateSpec {
            year: spec.year,
            month: spec.month,
            day: spec.day,
        });
        let hours = spec.hour.min..=spec.hour.max.min(23);
        let minutes = spec.minute.min..=spec.minute.max.min(59);
        let seconds = spec.second.min..=spec.second.max.min(59);
        if hours.is_empty() || minutes.is_empty() || seconds.is_empty() {
            return Self {
                times: Box::new(std::iter::empty()),
            };
        }
        let times = dates.flat_map(move |date| {
            let minutes = minutes.clone();
            let seconds = seconds.clone();
            hours.clone().flat_map(move |hour| {
                let seconds = seconds.clone();
                minutes.clone().flat_map(move |minute| {
                    seconds.clone().map(move |second| {
                        GameTime::from_date(date, hour, minute, second)
                            .expect("clock ranges are bounded above")
                    })
                })
            })
        });
        Self {
            times: Box::new(times),
        }
    }
}

impl Iterator for GameTimeIterator {
    type Item = GameTime;
    fn next(&mut self) -> Option<GameTime> {
        self.times.next()
    }
}

#[cfg(test)]
mod tests {
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

        assert_eq!(v.year(), 26);
        assert_eq!(v.month(), 1);
        assert_eq!(v.day(), 1);
        assert_eq!(v.hour(), 0);
        assert_eq!(v.minute(), 0);
        assert_eq!(v.second(), 0);

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

        assert_eq!((a.minute(), a.second()), (0, 58));
        assert_eq!((b.minute(), b.second()), (0, 59));
        assert_eq!((c.minute(), c.second()), (1, 58));
        assert_eq!((d.minute(), d.second()), (1, 59));

        assert!(it.next().is_none());
    }

    #[test]
    fn iter_month_end() {
        let spec = GameTimeSpec {
            year: FieldRange { min: 26, max: 26 },
            month: FieldRange { min: 2, max: 3 },
            day: FieldRange { min: 31, max: 31 },
            hour: FieldRange { min: 0, max: 0 },
            minute: FieldRange { min: 0, max: 0 },
            second: FieldRange { min: 0, max: 0 },
        };

        let mut it = GameTimeIterator::new(spec);

        let march = it.next().unwrap();
        assert_eq!((march.month(), march.day()), (3, 31));
        assert!(it.next().is_none());
    }
}
