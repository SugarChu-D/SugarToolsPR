#[derive(Debug, Clone, Copy)]
pub struct FieldRange<T> {
    pub min: T,
    pub max: T,
}

impl<T: Copy + Ord> FieldRange<T> {
    pub fn contains(&self, v: T) -> bool {
        self.min <= v && v <= self.max
    }
}
