use crate::models::{DSConfig, game_date::GameDate, game_date_iterator::GameDateSpec};

use bytemuck::{Pod, Zeroable};

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct GpuInput {
    pub nazo: [u32; 5],
    pub vcount_timer0_as_data5: u32,
    pub mac: u64,
    pub gxframe_xor_frame: u32,
    pub date_as_data8: u32,
    pub hour_range: [u32; 2],
    pub minute_range: [u32; 2],
    pub second_range: [u32; 2],
    pub _pad0: u32,
    pub iv_step: u32,
    pub iv_min: [u32; 6],
    pub iv_max: [u32; 6],
}

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct GpuIvConfig {
    pub iv_step: u32,
    pub _pad0: u32,
    pub iv_min: [u32; 6],
    pub iv_max: [u32; 6],
}


#[cfg(test)]
impl GpuInput {
    pub fn test_new(
        nazo: [u32; 5],
        vcount_timer0_as_data5: u32,
        mac: u64,
        gxframe_xor_frame: u32,
        date_as_data8: u32,
        hour_range: [u32; 2],
        minute_range: [u32; 2],
        second_range: [u32; 2],
        iv_step: u32,
        iv_min: [u32; 6],
        iv_max: [u32; 6],
    ) -> Self {
        Self {
            nazo,
            vcount_timer0_as_data5,
            mac,
            gxframe_xor_frame,
            date_as_data8,
            hour_range,
            minute_range,
            second_range,
            _pad0: 0,
            iv_step,
            iv_min,
            iv_max,
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::game_date_iterator::GameDateSpec;
    use crate::models::field_range::FieldRange;
    use crate::models::game_version::GameVersion;

    #[test]
    fn test_gpu_input_iterator_next_batch() {
        let ds_config = DSConfig::new(GameVersion::White2, 0x10F7, false, 0x0009bf6d93ce);
        let datespec = GameDateSpec {
            year: FieldRange { min: 33, max: 33 },
            month: FieldRange { min: 8, max: 8 },
            day: FieldRange { min: 27, max: 28 },
        };

        let mut it = GPUInputIterator::new(
            ds_config,
            datespec,
            [0, 23],
            [0, 59],
            [0, 59],
            2,
            [0; 6],
            [31; 6],
        );

        let batch1 = it.next_batch(1);
        assert_eq!(batch1.len(), 1);

        let batch2 = it.next_batch(4);
        assert_eq!(batch2.len(), 1);

        let batch3 = it.next_batch(1);
        assert!(batch3.is_empty());
    }
}

/**
 * イテレータ
 */
pub struct GPUInputIterator {
    ds_config: DSConfig,
    current_date: GameDate,
    datespec: GameDateSpec,
    hour_range: [u32; 2],
    minute_range: [u32; 2],
    second_range: [u32; 2],
    iv_step: u32,
    iv_min: [u32; 6],
    iv_max: [u32; 6],
    finished: bool,
}

impl Iterator for GPUInputIterator {
    type Item = GpuInput;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        let out = GpuInput {
            nazo: [
                self.ds_config.get_version_config().nazo_values.nazo1,
                self.ds_config.get_version_config().nazo_values.nazo2,
                self.ds_config.get_version_config().nazo_values.nazo3,
                self.ds_config.get_version_config().nazo_values.nazo4,
                self.ds_config.get_version_config().nazo_values.nazo5,
            ],
            vcount_timer0_as_data5: ((self.ds_config.get_version_config().vcount.0 as u32) << 16) | (self.ds_config.Timer0 as u32),
            mac: self.ds_config.MAC,
            gxframe_xor_frame: if self.ds_config.IsDSLite { 0x0600_0006} else {0x0600_0008},
            date_as_data8: self.current_date.get_date8_format(),
            hour_range: self.hour_range,
            minute_range: self.minute_range,
            second_range: self.second_range,
            _pad0: 0,
            iv_step: self.iv_step,
            iv_min: self.iv_min,
            iv_max: self.iv_max,
        };
        self.advance();
        Some(out)
    }
}

impl GPUInputIterator {
    pub fn new(
        ds_config: DSConfig,
        datespec: GameDateSpec,
        hour_range: [u32; 2],
        minute_range: [u32; 2],
        second_range: [u32; 2],
        iv_step: u32,
        iv_min: [u32; 6],
        iv_max: [u32; 6],
    ) -> Self {
        Self {
            ds_config,
            current_date: datespec.start(),
            datespec,
            hour_range,
            minute_range,
            second_range,
            iv_step,
            iv_min,
            iv_max,
            finished: false,
        }
    }

    pub fn next_batch(&mut self, n: usize) -> Vec<GpuInput> {
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            match self.next() {
                Some(item) => out.push(item),
                None => break,
            }
        }
        out
    }

    #[inline]
    fn advance(&mut self) {
        // 日
        self.current_date.day += 1;
        let dim = self.current_date.days_in_month();
        if self.current_date.day <= dim && self.datespec.day.contains(self.current_date.day) {
            return;
        }
        self.current_date.day = self.datespec.day.min;

        // 月
        self.current_date.month += 1;
        if self.current_date.month <= self.datespec.month.max {
            return;
        }
        self.current_date.month = self.datespec.month.min;

        // 年
        self.current_date.year += 1;
        if self.current_date.year <= self.datespec.year.max {
            return;
        }

        // 完全終了
        self.finished = true;
    }
}
