use super::Lcg;
use super::OffsetType;

impl Lcg {
    pub fn tid_sid(&mut self, offset_type: OffsetType) -> (u16, u16) {
        if self.step <= 1 {
            panic!("LCG state has not been advanced yet. Call offset_seed0 or offset_seed1 first.");
        }
        let next: u32 = (self.next() >> 32) as u32;
        // TIDはnext>>32の下16ビット
        // The game logic is effectively modulo 65536; avoid debug underflow on 0.
        let tid: u16 = ((next & 0xFFFF) as u16).wrapping_sub(1);
        // SIDはnext>>32の上16ビット
        let sid: u16 = ((next >> 16) & 0xFFFF) as u16;

        match offset_type {
            OffsetType::Bw1Start => {
                self.pt(4);
                // 表住人・裏住人を決定
                self.advance(13);
            },
            _ => {
                // 他のオフセットタイプでは特に追加処理なし
            },
        }

        (tid, sid)
    }
}

pub fn get_frigate_pass(tid: u16) -> &'static str {
    match tid % 256 % 5 {
        0 => "RESHIRAM",
        1 => "ZEKROM",
        2 => "9909",
        3 => "7707",
        4 => "2202",
        _ => " "
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_tid_sid() {
        let mut seed = Lcg::new(0x48B96278DC6233AB);
        seed.offset_seed1(OffsetType::Bw1Start);
        let (tid, _sid) = seed.tid_sid(OffsetType::Bw1Start);
        assert_eq!(tid, 5683);
        assert_eq!(_sid, 47868, "SID is {:X}", _sid);
    }
}
