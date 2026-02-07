use sha1::{Sha1, Digest};
use crate::models::{DSConfig, GameTime, KeyPresses};
use crate::lcg::lcg_next;

pub fn generate_initial_seed0(config: &DSConfig, game_time: &GameTime, key_presses: KeyPresses) -> u64 {
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
    const GX_FRAME: u32 = 0x0600_0000; // GxFrameはほぼ固定
    let frame: u32 = if config.IsDSLite { 6 } else { 8 };
    let gxframe_xor_frame = GX_FRAME ^ frame;
    let gxframe_xor_frame_le = u32::from_be(gxframe_xor_frame);
    let mac_middle_16 = ((config.MAC >> 16) & 0xFFFFFFFF) as u32;
    let data7 = gxframe_xor_frame_le ^ mac_middle_16;
    #[cfg(debug_assertions)]
    {
        println!("GxFrame: 0x{:08X}, frame: 0x{:08X}, gxframe_xor_frame: 0x{:08X}", GX_FRAME, frame, gxframe_xor_frame);
        println!("gxframe_xor_frame_le: 0x{:08X}, mac_middle_16: 0x{:08X}", gxframe_xor_frame_le, mac_middle_16);
        println!("data7: 0x{:08X}", data7);
    }
    hasher.update(data7.to_be_bytes());

    // data[8] に対応
    // 日付情報をdata8形式で取得してビッグエンディアンで追加
    let data8 = game_time.get_date8_format();
    #[cfg(debug_assertions)]
    println!("data8 (date): 0x{:08X} (year:{} month:{} day:{} weekday:{})", 
        data8, game_time.year, game_time.month, game_time.day, game_time.weekday());
    hasher.update(data8.to_be_bytes());

    // data[9] に対応
    // 時刻情報をdata9形式で取得してビッグエンディアンで追加
    let time9 = game_time.get_time9_format();
    #[cfg(debug_assertions)]
    println!("data9 (time): 0x{:08X} (hour:{} minute:{} second:{})", 
        time9, game_time.hour, game_time.minute, game_time.second);
    hasher.update(time9.to_be_bytes());

    // data[10]とdata[11]は0で固定
    #[cfg(debug_assertions)]
    println!("data10: 0x00000000, data11: 0x00000000");
    hasher.update(0u32.to_le_bytes());
    hasher.update(0u32.to_le_bytes());

    // data[12]に対応
    // キー入力状態をリトルエンディアンで追加
    #[cfg(debug_assertions)]
    println!("key_presses: 0x{:04X}", key_presses.raw());
    hasher.update((key_presses.raw() as u32).to_le_bytes());

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

pub fn generate_initial_seed1(config: &DSConfig, game_time: &GameTime, key_presses: KeyPresses) -> u64 {
    let seed0 = generate_initial_seed0(config, game_time, key_presses);
    // LCGでseed1を生成
    let seed1 = lcg_next(seed0);
    #[cfg(debug_assertions)]
    println!("Generated initial seed1: 0x{:016X}", seed1);
    seed1
}
