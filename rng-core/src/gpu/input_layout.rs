use crate::models::{DSConfig, KeyPresses, game_date::GameDate, game_date_iterator::GameDateSpec};

use bytemuck::{Pod, Zeroable};

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct GpuInput {
    nazo: [u32; 5],
    vcount_timer0_as_data5: u32,
    mac: u64,
    gxframe_xor_frame: u32,
    date_as_data8: u32,
    timespec: [[u32; 2]; 3],
    key_presses: u32,
    _pad0: u32,
}

#[cfg(test)]
impl GpuInput {
    pub fn test_new(
        nazo: [u32; 5],
        vcount_timer0_as_data5: u32,
        mac: u64,
        gxframe_xor_frame: u32,
        date_as_data8: u32,
        timespec: [[u32; 2]; 3],
        key_presses: u32,
    ) -> Self {
        Self {
            nazo,
            vcount_timer0_as_data5,
            mac,
            gxframe_xor_frame,
            date_as_data8,
            timespec,
            key_presses,
            _pad0: 0,
        }
    }
}

/**
 * イテレータ
 */
pub struct GPUInputIterator {
    ds_config: DSConfig,
    current_date: GameDate,
    datespec: GameDateSpec,
    timespec: [[u32; 2]; 3],
    key_presses: u32,
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
            timespec: self.timespec,
            key_presses: self.key_presses,
            _pad0: 0,
        };
        self.advance();
        Some(out)
    }
}

impl GPUInputIterator {
    pub fn new(
        ds_config: DSConfig,
        datespec: GameDateSpec,
        timespec: [[u32; 2]; 3],
        key_presses: u32,
    ) -> Self {
        
        Self {
            ds_config,
            current_date: datespec.start(),
            datespec,
            timespec,
            key_presses,
            finished: false,
        }
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

/// Build inputs for all valid key presses (2160 patterns).
/// This expands the date range for each key press.
pub fn build_inputs_for_keypresses(
    ds_config: DSConfig,
    datespec: GameDateSpec,
    timespec: [[u32; 2]; 3],
) -> Vec<GpuInput> {
    KeyPresses::iter_valid()
        .flat_map(|kp| {
            GPUInputIterator::new(ds_config, datespec, timespec, kp.raw() as u32)
        })
        .collect()
}
