// キーのビット位置
const KEY_A_BIT: u32 = 0;
const KEY_B_BIT: u32 = 1;
const KEY_SELECT_BIT: u32 = 2;
const KEY_START_BIT: u32 = 3;
const KEY_RIGHT_BIT: u32 = 4;
const KEY_LEFT_BIT: u32 = 5;
const KEY_UP_BIT: u32 = 6;
const KEY_DOWN_BIT: u32 = 7;
const KEY_R_BIT: u32 = 8;
const KEY_L_BIT: u32 = 9;
const KEY_X_BIT: u32 = 10;
const KEY_Y_BIT: u32 = 11;

// ビットマスクヘルパー関数（定数関数）
const fn key_mask(bit: u32) -> u16 {
    1 << bit as u16
}

// 定数定義
const KEY_NONE: u16 = 0x2fff;
const KEY_RANGE_START: u16 = 0x2000;
const KEY_RANGE_END: u16 = 0x2fff;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyPresses {
    pub keys: u16,
}

impl KeyPresses {
    /// 新規作成（0x2FFF の初期状態）
    pub fn new() -> Self {
        KeyPresses { keys: KEY_NONE }
    }

    /// 指定したキーの状態で作成
    pub fn with_keys(keys: u16) -> Self {
        KeyPresses { keys }
    }

    /// キーを押す（特定の位置のビットを1から0に）
    pub fn press(&mut self, bit: u32) {
        self.keys &= !key_mask(bit);
    }

    /// キーを離す（特定の位置のビットを0から1に）
    pub fn release(&mut self, bit: u32) {
        self.keys |= key_mask(bit);
    }

    /// キーが押されているかチェック（ビットが0であることを確認）
    pub fn is_pressed(&self, bit: u32) -> bool {
        (self.keys & key_mask(bit)) == 0
    }

    /// すべてのキーをリセット（0x2FFF に戻す）
    pub fn clear(&mut self) {
        self.keys = KEY_NONE;
    }

    /// 有効なキー入力かチェック
    /// 無効な組み合わせ：上下同時、左右同時、L・R・Start・Select同時
    pub fn is_valid(&self) -> bool {
        // 上下同時押しをチェック
        if (self.keys & key_mask(KEY_UP_BIT)) == 0 && (self.keys & key_mask(KEY_DOWN_BIT)) == 0 {
            return false;
        }

        // 左右同時押しをチェック
        if (self.keys & key_mask(KEY_LEFT_BIT)) == 0 && (self.keys & key_mask(KEY_RIGHT_BIT)) == 0 {
            return false;
        }

        // L・R・Start・Select同時押しをチェック
        if (self.keys & key_mask(KEY_L_BIT)) == 0
            && (self.keys & key_mask(KEY_R_BIT)) == 0
            && (self.keys & key_mask(KEY_START_BIT)) == 0
            && (self.keys & key_mask(KEY_SELECT_BIT)) == 0
        {
            return false;
        }

        true
    }

    /// 押されているキーを文字列で返す
    pub fn pressed_keys_string(&self) -> String {
        let mut keys = Vec::new();

        if self.is_pressed(KEY_A_BIT) {
            keys.push("A");
        }
        if self.is_pressed(KEY_B_BIT) {
            keys.push("B");
        }
        if self.is_pressed(KEY_X_BIT) {
            keys.push("X");
        }
        if self.is_pressed(KEY_Y_BIT) {
            keys.push("Y");
        }
        if self.is_pressed(KEY_UP_BIT) {
            keys.push("UP");
        }
        if self.is_pressed(KEY_DOWN_BIT) {
            keys.push("DOWN");
        }
        if self.is_pressed(KEY_LEFT_BIT) {
            keys.push("LEFT");
        }
        if self.is_pressed(KEY_RIGHT_BIT) {
            keys.push("RIGHT");
        }
        if self.is_pressed(KEY_L_BIT) {
            keys.push("L");
        }
        if self.is_pressed(KEY_R_BIT) {
            keys.push("R");
        }
        if self.is_pressed(KEY_START_BIT) {
            keys.push("START");
        }
        if self.is_pressed(KEY_SELECT_BIT) {
            keys.push("SELECT");
        }

        if keys.is_empty() {
            "none".to_string()
        } else {
            keys.join("+")
        }
    }
    
    /// 有効なすべてのキー入力値の配列を返す（全探索用）
    pub fn valid_key_inputs() -> Vec<u16> {
        (KEY_RANGE_START..=KEY_RANGE_END)
            .filter_map(|keys| {
                let kp = KeyPresses { keys };
                if kp.is_valid() {
                    Some(keys)
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn default() -> Self {
        Self::new()
    }
}


impl Default for KeyPresses {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_and_initial_state() {
        let kp = KeyPresses::new();
        assert_eq!(kp.keys, KEY_NONE);
        assert_eq!(kp.pressed_keys_string(), "none");
    }

    #[test]
    fn test_with_keys() {
        let kp = KeyPresses::with_keys(0x2ffc); // A と B を押した状態
        assert_eq!(kp.keys, 0x2ffc);
        assert!(kp.is_pressed(KEY_A_BIT));
        assert!(kp.is_pressed(KEY_B_BIT));
        assert!(!kp.is_pressed(KEY_START_BIT));
    }

    #[test]
    fn test_press_and_release() {
        let mut kp = KeyPresses::new();
        
        // キーを押す
        kp.press(KEY_A_BIT);
        assert!(kp.is_pressed(KEY_A_BIT));
        assert!(!kp.is_pressed(KEY_B_BIT));
        
        // 別のキーを押す
        kp.press(KEY_B_BIT);
        assert!(kp.is_pressed(KEY_A_BIT));
        assert!(kp.is_pressed(KEY_B_BIT));
        
        // キーを離す
        kp.release(KEY_A_BIT);
        assert!(!kp.is_pressed(KEY_A_BIT));
        assert!(kp.is_pressed(KEY_B_BIT));
    }

    #[test]
    fn test_clear() {
        let mut kp = KeyPresses::new();
        kp.press(KEY_A_BIT);
        kp.press(KEY_B_BIT);
        kp.press(KEY_X_BIT);
        assert_eq!(kp.keys, 0x2bfc);
        
        kp.clear();
        assert_eq!(kp.keys, KEY_NONE);
        assert_eq!(kp.pressed_keys_string(), "none");
    }

    #[test]
    fn test_invalid_up_down_simultaneous() {
        let mut kp = KeyPresses::new();
        kp.press(KEY_UP_BIT);
        kp.press(KEY_DOWN_BIT);
        assert!(!kp.is_valid()); // 上下同時は無効
        
        kp.release(KEY_DOWN_BIT);
        assert!(kp.is_valid()); // DOWN を離すと有効
    }

    #[test]
    fn test_invalid_left_right_simultaneous() {
        let mut kp = KeyPresses::new();
        kp.press(KEY_LEFT_BIT);
        kp.press(KEY_RIGHT_BIT);
        assert!(!kp.is_valid()); // 左右同時は無効
        
        kp.release(KEY_RIGHT_BIT);
        assert!(kp.is_valid()); // RIGHT を離すと有効
    }

    #[test]
    fn test_invalid_lrss_simultaneous() {
        let mut kp = KeyPresses::new();
        kp.press(KEY_L_BIT);
        kp.press(KEY_R_BIT);
        kp.press(KEY_START_BIT);
        kp.press(KEY_SELECT_BIT);
        assert!(!kp.is_valid()); // L・R・Start・Select同時は無効
        
        kp.release(KEY_R_BIT);
        assert!(kp.is_valid()); // R を離すと有効
    }

    #[test]
    fn test_pressed_keys_string() {
        let mut kp = KeyPresses::new();
        assert_eq!(kp.pressed_keys_string(), "none");
        
        kp.press(KEY_A_BIT);
        assert_eq!(kp.pressed_keys_string(), "A");
        
        kp.press(KEY_B_BIT);
        kp.press(KEY_START_BIT);
        assert_eq!(kp.pressed_keys_string(), "A+B+START");
        
        kp.clear();
        assert_eq!(kp.pressed_keys_string(), "none");
    }

    #[test]
    fn test_all_buttons() {
        let mut kp = KeyPresses::new();
        
        let all_buttons = [
            (KEY_A_BIT, "A"),
            (KEY_B_BIT, "B"),
            (KEY_X_BIT, "X"),
            (KEY_Y_BIT, "Y"),
            (KEY_UP_BIT, "UP"),
            (KEY_DOWN_BIT, "DOWN"),
            (KEY_LEFT_BIT, "LEFT"),
            (KEY_RIGHT_BIT, "RIGHT"),
            (KEY_L_BIT, "L"),
            (KEY_R_BIT, "R"),
            (KEY_START_BIT, "START"),
            (KEY_SELECT_BIT, "SELECT"),
        ];
        
        for (bit, _name) in all_buttons {
            kp.clear();
            kp.press(bit);
            assert!(kp.is_pressed(bit));
        }
    }

    #[test]
    fn test_valid_key_inputs_count() {
        let valid_inputs = KeyPresses::valid_key_inputs();
        // 全範囲 0x2000-0x2fff は 4096 個
        // 上下同時、左右同時、L・R・Start・Select同時を除外
        assert!(!valid_inputs.is_empty());
        assert!(valid_inputs.len() < 4096);
        
        // すべてのキー入力が有効かチェック
        for &input in &valid_inputs {
            let kp = KeyPresses::with_keys(input);
            assert!(kp.is_valid(), "Key input 0x{:04x} should be valid", input);
        }
    }

    #[test]
    fn test_default_impl() {
        let kp: KeyPresses = Default::default();
        assert_eq!(kp.keys, KEY_NONE);
    }
}
