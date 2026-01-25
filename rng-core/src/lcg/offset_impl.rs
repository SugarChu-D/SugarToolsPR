use super::Lcg;

const PROBABILITY_TABLE: [[u8; 5]; 6] = [
    [50, 100, 100, 100, 100],
    [50,  50, 100, 100, 100],
    [30,  50, 100, 100, 100],
    [25,  30,  50, 100, 100],
    [20,  25,  33,  50, 100],
    [100, 100, 100, 100, 100],
];

#[derive(Clone, Copy, Debug)]
pub enum OffsetType {
    Bw1Start,
    Bw1Continue,
    BW2Start,
    BW2Continue,
    BW2ContinueWithLink,
}

impl Lcg {
    pub fn offset_seed0(&mut self, offset_type: OffsetType) -> u64 {
        self.next();
        self.offset_seed1(offset_type)
    }
    
    pub fn offset_seed1(&mut self, offset_type: OffsetType) -> u64 {
        match offset_type {
            // BW1はじめから
            OffsetType::Bw1Start => {
                self.pt(3);
                self.advance(3);
                // 以降はTID,SID
            },
            // BW1続きから
            OffsetType::Bw1Continue => {
                self.pt(5);
            },
            // BW2はじめから
            OffsetType::BW2Start => {
                self.pt(1);
                self.advance(2);
                self.pt(1);
                self.advance(4);
                self.pt(1);
                self.advance(2);
                // チラチーノ用pid
                self.next();
                // 以降はTID,SID
            },
            // BW2続きから
            OffsetType::BW2Continue => {
                self.pt(1);
                self.advance(3);
                self.pt(4);
                self.offset_extra();
            },
            // BW2続きから(おもいでリンクあり)
            OffsetType::BW2ContinueWithLink => {
                self.pt(1);
                self.advance(2);
                self.pt(4);
                self.offset_extra();
            },
        }
        
        self.step
    }

    pub fn pt(&mut self, counts: u32) {
        for _ in 0..counts {
            for i in 0..6 {
                for j in 0..5 {
                    if PROBABILITY_TABLE[i][j] == 100 {
                        // 100%なら無条件で
                        break;
                    }
                    let r: u32 = (((self.next() >> 32) * 101) >> 32) as u32;
                    if r <= PROBABILITY_TABLE[i][j] as u32 {
                        break;
                    }
                }
            }
        }
    }

    // BW2特有の追加オフセット処理
    pub fn offset_extra(&mut self) {
        loop{
            let r1: u32 = (((self.next() >> 32) * 15) >> 32) as u32;
            let r2: u32 = (((self.next() >> 32) * 15) >> 32) as u32;
            let r3: u32 = (((self.next() >> 32) * 15) >> 32) as u32;
            if !(r1 == r2 || r2 == r3 || r1 == r3) {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_offset_BW2Contiue() {
        let mut seed = Lcg::new(0x490CC591E17E7DB7);
        let offset = seed.offset_seed1(OffsetType::BW2Continue);
        assert_eq!(offset, 55);
    }

    #[test]
    fn test_offset_BW1Start() {
        let mut seed = Lcg::new(0x48B96278DC6233AB);
        let offset = seed.offset_seed1(OffsetType::Bw1Start);
        let (_tid, _sid) = seed.tid_sid(OffsetType::Bw1Start);
        assert_eq!(offset, 34);
    }
}