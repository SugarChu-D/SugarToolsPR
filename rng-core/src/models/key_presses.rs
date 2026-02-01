use crate::models::eval_mode::{EvalMode, EvalResult, Evaluator};

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
    1u16 << bit
}

// 定数定義
const KEY_RANGE_START: u16 = 0x2000;
const KEY_RANGE_END: u16 = 0x2fff;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyPresses {
    keys: u16,
}

impl KeyPresses {
    /**
    生のキー値から生成する(有効評価は行わない)
     */
    pub const fn new(keys: u16) -> Self {
        Self { keys }
    }

    // キーの値のゲッタ
    pub const fn raw(&self) -> u16 {
        self.keys
    }

    /// キーが押されているかチェック（ビットが0であることを確認）
    pub fn is_pressed(&self, bit: u32) -> bool {
        (self.keys & key_mask(bit)) == 0
    }

    /// 有効なキー入力かチェック
    /// 無効な組み合わせ：上下同時、左右同時、L・R・Start・Select同時
    const fn is_valid_raw(keys: u16) -> bool {
        // 上下同時
        if (keys & key_mask(KEY_UP_BIT)) == 0 && (keys & key_mask(KEY_DOWN_BIT)) == 0 {
            return false;
        }
        // 左右同時
        if (keys & key_mask(KEY_LEFT_BIT)) == 0 && (keys & key_mask(KEY_RIGHT_BIT)) == 0 {
            return false;
        }
        // L R START SELECT 同時
        if (keys & key_mask(KEY_L_BIT)) == 0
            && (keys & key_mask(KEY_R_BIT)) == 0
            && (keys & key_mask(KEY_START_BIT)) == 0
            && (keys & key_mask(KEY_SELECT_BIT)) == 0
        {
            return false;
        }
        true
    }

    /// 押されているキーを文字列で返す
    pub fn pressed_keys_string(&self) -> String {
        let mut keys = Vec::new();

        macro_rules! push {
            ($bit:ident, $name:expr) => {
                if (self.keys & key_mask($bit)) == 0 {
                    keys.push($name);
                }
            };
        }

        push!(KEY_A_BIT, "A");
        push!(KEY_B_BIT, "B");
        push!(KEY_X_BIT, "X");
        push!(KEY_Y_BIT, "Y");
        push!(KEY_UP_BIT, "UP");
        push!(KEY_DOWN_BIT, "DOWN");
        push!(KEY_LEFT_BIT, "LEFT");
        push!(KEY_RIGHT_BIT, "RIGHT");
        push!(KEY_L_BIT, "L");
        push!(KEY_R_BIT, "R");
        push!(KEY_START_BIT, "START");
        push!(KEY_SELECT_BIT, "SELECT");

        if keys.is_empty() {
            "none".to_string()
        } else {
            keys.join("+")
        }
    }
    
    /// 有効なすべてのキー入力値の配列を返す（全探索用）
    pub fn iter_valid() -> impl Iterator<Item = KeyPresses> {
        (KEY_RANGE_START..=KEY_RANGE_END)
            .filter(|&keys| Self::is_valid_raw(keys))
            .map(KeyPresses::new)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
}
