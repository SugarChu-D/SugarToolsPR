use crate::models::{
    VersionConfig,
    game_version::{GameVersion, Region, VersionConfigError},
};
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;

fn parse_hex_or_decimal_u64(s: &str) -> Result<u64, String> {
    let s = s.trim();
    if s.starts_with("0x") || s.starts_with("0X") {
        u64::from_str_radix(&s[2..], 16).map_err(|e| format!("invalid hex: {}", e))
    } else {
        s.parse::<u64>()
            .map_err(|e| format!("invalid number: {}", e))
    }
}

fn de_u64_hex_or_dec<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    struct V;
    impl<'de> serde::de::Visitor<'de> for V {
        type Value = u64;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "a u64 as number or hex string")
        }

        fn visit_u64<E>(self, v: u64) -> Result<u64, E>
        where
            E: serde::de::Error,
        {
            Ok(v)
        }

        fn visit_i64<E>(self, v: i64) -> Result<u64, E>
        where
            E: serde::de::Error,
        {
            if v < 0 {
                Err(E::custom("negative value for unsigned field"))
            } else {
                Ok(v as u64)
            }
        }

        fn visit_str<E>(self, s: &str) -> Result<u64, E>
        where
            E: serde::de::Error,
        {
            parse_hex_or_decimal_u64(s).map_err(E::custom)
        }
    }

    deserializer.deserialize_any(V)
}

fn de_u16_hex_or_dec<'de, D>(deserializer: D) -> Result<u16, D::Error>
where
    D: Deserializer<'de>,
{
    let v = de_u64_hex_or_dec(deserializer)?;
    if v <= u16::MAX as u64 {
        Ok(v as u16)
    } else {
        Err(serde::de::Error::custom(format!(
            "value {} out of range for u16",
            v
        )))
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct DSConfig {
    #[serde(rename = "version")]
    pub version: GameVersion,
    #[serde(rename = "region")]
    pub region: Region,
    #[serde(rename = "timer0", deserialize_with = "de_u16_hex_or_dec")]
    pub timer0: u16,
    #[serde(rename = "is_dslite")]
    pub is_dslite: bool,
    #[serde(rename = "mac", deserialize_with = "de_u64_hex_or_dec")]
    pub mac_address: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DSConfigError {
    MacAddressOutOfRange(u64),
    VersionConfig(VersionConfigError),
}

impl fmt::Display for DSConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MacAddressOutOfRange(mac) => write!(f, "MAC address exceeds 48 bits: {mac:#x}"),
            Self::VersionConfig(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for DSConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::MacAddressOutOfRange(_) => None,
            Self::VersionConfig(error) => Some(error),
        }
    }
}

impl DSConfig {
    pub fn new(
        version: GameVersion,
        region: Region,
        timer0: u16,
        is_dslite: bool,
        mac: u64,
    ) -> Self {
        Self {
            version,
            region,
            timer0,
            is_dslite,
            mac_address: mac,
        }
    }

    pub fn validate(&self) -> Result<(), DSConfigError> {
        self.try_get_version_config().map(|_| ())
    }

    pub fn try_get_version_config(&self) -> Result<VersionConfig, DSConfigError> {
        if self.mac_address >= (1 << 48) {
            return Err(DSConfigError::MacAddressOutOfRange(self.mac_address));
        }
        VersionConfig::try_from_version(self.version, self.region)
            .map_err(DSConfigError::VersionConfig)
    }

    pub fn get_version_config(&self) -> VersionConfig {
        VersionConfig::from_version(self.version, self.region)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::game_version::GameVersion;

    #[test]
    fn validates_mac_address_width() {
        let mut config = DSConfig::new(
            GameVersion::Black,
            Region::JPN,
            u16::MAX,
            false,
            (1 << 48) - 1,
        );
        assert_eq!(config.validate(), Ok(()));

        config.mac_address += 1;
        assert_eq!(
            config.validate(),
            Err(DSConfigError::MacAddressOutOfRange(1 << 48))
        );
    }

    #[test]
    fn test_deserialize_hex_mac_and_timer() {
        let j = r#"{ "version": "Black", "timer0": "0x1F", "is_dslite": false, "mac": "0x1234abcd", "region": "JPN" }"#;
        let cfg: DSConfig = serde_json::from_str(j).expect("parse hex fields");
        assert_eq!(cfg.timer0, 0x1F);
        assert_eq!(cfg.mac_address, 0x1234ABCDu64);
    }

    #[test]
    fn test_deserialize_timer_overflow_errors() {
        let j = r#"{ "version": "Black", "timer0": "0x10000", "is_dslite": false, "mac": 1, "region": "JPN" }"#;
        let r: Result<DSConfig, _> = serde_json::from_str(j);
        assert!(r.is_err());
    }

    #[test]
    fn test_dsconfig_new_and_fields() {
        let cfg = DSConfig::new(
            GameVersion::Black,
            Region::JPN,
            0x10FA,
            true,
            0x1234_ABCDu64,
        );
        assert_eq!(cfg.version, GameVersion::Black);
        assert_eq!(cfg.region, Region::JPN);
        assert_eq!(cfg.timer0, 0x10FA);
        assert!(cfg.is_dslite);
        assert_eq!(cfg.mac_address, 0x1234_ABCDu64);
    }

    #[test]
    fn test_serde_roundtrip() {
        let cfg = DSConfig::new(
            GameVersion::White2,
            Region::JPN,
            0x10FA,
            false,
            0xDEAD_BEEFu64,
        );
        let s = serde_json::to_string(&cfg).expect("serialize");
        let de: DSConfig = serde_json::from_str(&s).expect("deserialize");
        assert_eq!(de.mac_address, cfg.mac_address);
        assert_eq!(de.timer0, cfg.timer0);
        assert_eq!(de.is_dslite, cfg.is_dslite);
        assert_eq!(de.version, cfg.version);
        assert_eq!(de.region, cfg.region);
    }
}
