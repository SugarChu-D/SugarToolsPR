use crate::lcg::LCG;

impl LCG {
    pub fn offset_extra(&mut self) {
        self.PT(2);
        self.advance(1);
    }
}