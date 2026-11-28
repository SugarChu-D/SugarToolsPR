use rng_core::models::{
    DSConfig,
    game_date_iterator::{GameDateIterator, GameDateSpec},
};

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
    #[expect(
        clippy::too_many_arguments,
        reason = "GPU入力レイアウトの各フィールドをテストから個別に指定する"
    )]
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

/**
 * イテレータ
 */
pub struct GPUInputIterator {
    ds_config: DSConfig,
    dates: GameDateIterator,
    hour_range: [u32; 2],
    minute_range: [u32; 2],
    second_range: [u32; 2],
    iv_step: u32,
    iv_min: [u32; 6],
    iv_max: [u32; 6],
}

impl Iterator for GPUInputIterator {
    type Item = GpuInput;

    fn next(&mut self) -> Option<Self::Item> {
        let date = self.dates.next()?;

        let out = GpuInput {
            nazo: [
                self.ds_config.get_version_config().nazo_values.nazo1,
                self.ds_config.get_version_config().nazo_values.nazo2,
                self.ds_config.get_version_config().nazo_values.nazo3,
                self.ds_config.get_version_config().nazo_values.nazo4,
                self.ds_config.get_version_config().nazo_values.nazo5,
            ],
            vcount_timer0_as_data5: (self.ds_config.get_version_config().vcount << 16)
                | (self.ds_config.timer0 as u32),
            mac: self.ds_config.mac_address,
            gxframe_xor_frame: if self.ds_config.is_dslite {
                0x0600_0006
            } else {
                0x0600_0008
            },
            date_as_data8: date.get_date8_format(),
            hour_range: self.hour_range,
            minute_range: self.minute_range,
            second_range: self.second_range,
            _pad0: 0,
            iv_step: self.iv_step,
            iv_min: self.iv_min,
            iv_max: self.iv_max,
        };
        Some(out)
    }
}

impl GPUInputIterator {
    #[expect(
        clippy::too_many_arguments,
        reason = "既存の公開コンストラクタの呼び出し形式を維持する"
    )]
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
            dates: GameDateIterator::new(datespec),
            hour_range,
            minute_range,
            second_range,
            iv_step,
            iv_min,
            iv_max,
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use rng_core::models::field_range::FieldRange;
    use rng_core::models::game_date_iterator::GameDateSpec;
    use rng_core::models::game_version::GameVersion;

    #[test]
    fn test_gpu_input_iterator_next_batch() {
        let ds_config = DSConfig::new(
            GameVersion::White2,
            rng_core::models::Region::JPN,
            0x10F7,
            false,
            0x0009bf6d93ce,
        );
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
