use crate::models::gameVersion::GameVersion;

#[derive(Debug, Clone, Copy)]
pub struct DSConfig {
    pub Version: GameVersion,
    pub Timer0: u8,
    pub IsDSLite: bool,
    pub MAC: u64,
}

impl DSConfig {
    pub fn new(version: GameVersion, timer0: u8, is_dslite: bool, mac: u64) -> Self {
        Self {
            Version: version,
            Timer0: timer0,
            IsDSLite: is_dslite,
            MAC: mac,
        }
    }
}