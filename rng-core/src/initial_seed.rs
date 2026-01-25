
use sha1::{Sha1, Digest};
use crate::models::{GameDate, KeyPresses, DSConfig};
use crate::lcg::lcg;

pub fn generate_initial_seed0(config: &DSConfig, game_date: &GameDate, key_presses_value: &u16) -> u64 {
    let mut hasher = Sha1::new();

    // ゲームバージョンのnazo値をリトルエンディアンで追加 data[0]-data[4]に対応
    let version_config = crate::models::VersionConfig::from_version(config.Version);
    #[cfg(debug_assertions)]
    {
        println!("nazo1: 0x{:08X}", version_config.nazo_values.nazo1);
        println!("nazo2: 0x{:08X}", version_config.nazo_values.nazo2);
        println!("nazo3: 0x{:08X}", version_config.nazo_values.nazo3);
        println!("nazo4: 0x{:08X}", version_config.nazo_values.nazo4);
        println!("nazo5: 0x{:08X}", version_config.nazo_values.nazo5);
    }
    hasher.update(version_config.nazo_values.nazo1.to_le_bytes());
    hasher.update(version_config.nazo_values.nazo2.to_le_bytes());
    hasher.update(version_config.nazo_values.nazo3.to_le_bytes());
    hasher.update(version_config.nazo_values.nazo4.to_le_bytes());
    hasher.update(version_config.nazo_values.nazo5.to_le_bytes());

    // VCountとTimer0をリトルエンディアンで追加 data[5]に対応
    let vcount_timer0 = ((version_config.vcount.0 as u32) << 16) | (config.Timer0 as u32);
    #[cfg(debug_assertions)]
    {
        println!("vcount: 0x{:02X}, Timer0: 0x{:04X}", version_config.vcount.0, config.Timer0);
        println!("vcount_timer0: 0x{:08X}", vcount_timer0);
    }
    hasher.update(vcount_timer0.to_le_bytes());

    // MACアドレスの下位16bitをビッグエンディアンで追加 data[6]に対応
    let mac_lower_16 = (config.MAC & 0xFFFF) as u32;
    #[cfg(debug_assertions)]
    {
        println!("MAC: 0x{:012X}", config.MAC);
        println!("mac_lower_16: 0x{:08X}", mac_lower_16);
    }
    hasher.update(mac_lower_16.to_be_bytes());

    // data[7]
    // GxFrame XOR frame の結果をビッグエンディアンで取得し、
    // MACアドレスの中間32itとXORを取ってビッグエンディアンで追加
    const GxFrame: u32 = 0x0600_0000;
    let frame: u32 = if config.IsDSLite { 6 } else { 8 };
    let gxframe_xor_frame = GxFrame ^ frame;
    let gxframe_xor_frame_le = u32::from_be(gxframe_xor_frame);
    let mac_middle_16 = ((config.MAC >> 16) & 0xFFFFFFFF) as u32;
    let data7 = gxframe_xor_frame_le ^ mac_middle_16;
    #[cfg(debug_assertions)]
    {
        println!("GxFrame: 0x{:08X}, frame: 0x{:08X}, gxframe_xor_frame: 0x{:08X}", GxFrame, frame, gxframe_xor_frame);
        println!("gxframe_xor_frame_le: 0x{:08X}, mac_middle_16: 0x{:08X}", gxframe_xor_frame_le, mac_middle_16);
        println!("data7: 0x{:08X}", data7);
    }
    hasher.update(data7.to_be_bytes());

    // data[8] に対応
    // 日付情報をdata8形式で取得してビッグエンディアンで追加
    let data8 = game_date.get_date8_format();
    #[cfg(debug_assertions)]
    println!("data8 (date): 0x{:08X} (year:{} month:{} day:{} weekday:{})", 
        data8, game_date.year, game_date.month, game_date.day, game_date.weekday());
    hasher.update(data8.to_be_bytes());

    // data[9] に対応
    // 時刻情報をdata9形式で取得してビッグエンディアンで追加
    let time9 = game_date.get_time9_format();
    #[cfg(debug_assertions)]
    println!("data9 (time): 0x{:08X} (hour:{} minute:{} second:{})", 
        time9, game_date.hour, game_date.minute, game_date.second);
    hasher.update(time9.to_be_bytes());

    // data[10]とdata[11]は0で固定
    #[cfg(debug_assertions)]
    println!("data10: 0x00000000, data11: 0x00000000");
    hasher.update(0u32.to_le_bytes());
    hasher.update(0u32.to_le_bytes());

    // data[12]に対応
    // キー入力状態をリトルエンディアンで追加
    #[cfg(debug_assertions)]
    println!("key_presses: 0x{:04X}", key_presses_value);
    hasher.update((*key_presses_value as u32).to_le_bytes());

    // ハッシュを計算
    let result = hasher.finalize();

    // 最初の8バイトをu64として返す
    let initial_seed = u64::from_le_bytes([
        result[0], result[1], result[2], result[3],
        result[4], result[5], result[6], result[7],
    ]);

    #[cfg(debug_assertions)]
    println!("\nFinal hash (first 8 bytes): 0x{:016X}", initial_seed);

    initial_seed
}

pub fn generate_initial_seed1(config: &DSConfig, game_date: &GameDate, key_presses_value: &u16) -> u64 {
    let seed0 = generate_initial_seed0(config, game_date, key_presses_value);
    // LCGでseed1を生成
    let seed1 = lcg::new(seed0).next();
    #[cfg(debug_assertions)]
    println!("Generated initial seed1: 0x{:016X}", seed1);
    seed1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::GameVersion;

    #[test]
    fn test_generate_initial_seed0() {
        // テスト用のデータを作成
        let config = DSConfig {
            Version: GameVersion::White2,
            Timer0: 0x10FA,
            MAC: 0x0009bf6d93ceu64,
            IsDSLite: false,
        };

        let game_date = GameDate::new(
            21,  // year
            4,   // month
            24,   // day
            2,  // hour
            38,  // minute
            5,   // second
        );

        let key_presses = KeyPresses {
            keys: 0x2eec, // A,B,右,Rが押されている状態
        };

        // 関数を実行
        let result0 = generate_initial_seed0(&config, &game_date, &key_presses.keys);
        let result1 = generate_initial_seed1(&config, &game_date, &key_presses.keys);

        // 結果を確認
        assert_eq!(result0, 0x9B3E7C4BC185AE31, "generated is {:X}", result0);
        assert_eq!(result1, 0xA90C98ED53739118, "generated is {:X}", result1);
    }
}
