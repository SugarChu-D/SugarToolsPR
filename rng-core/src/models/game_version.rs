// Gameversion
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameVersion {
    Black,
    White,
    Black2,
    White2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Region {
    JPN,
    KOR,
    USA,
    DEU,
    FRA,
    ESP,
    ITA,
}

impl Default for Region {
    fn default() -> Self {
        Self::JPN
    }
}

#[derive(Debug, Clone, Copy)]
pub struct NazoValues {
    pub nazo1: u32,
    pub nazo2: u32,
    pub nazo3: u32,
    pub nazo4: u32,
    pub nazo5: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct VersionConfig {
    pub game_version: GameVersion,
    pub nazo_values: NazoValues,
    pub vcount: u32,
}

impl VersionConfig {
    pub fn from_version(version: GameVersion) -> Self {
        Self::from_version_and_region(version, Region::JPN)
    }

    pub fn from_version_and_region(version: GameVersion, region: Region) -> Self {
        let version_name = version.to_string();
        let region_name = region.to_string();
        let row = include_str!("params/nazo_vcount.csv")
            .lines()
            .skip(1)
            .find(|line| {
                let mut fields = line.split(',');
                fields.next() == Some(version_name.as_str())
                    && fields.next() == Some(region_name.as_str())
            })
            .unwrap_or_else(|| panic!("missing nazo_vcount.csv row for {version_name}/{region_name}"));

        let mut fields = row.split(',');
        fields.next();
        fields.next();
        let nazo_values = NazoValues {
            nazo1: parse_hex_field(fields.next()),
            nazo2: parse_hex_field(fields.next()),
            nazo3: parse_hex_field(fields.next()),
            nazo4: parse_hex_field(fields.next()),
            nazo5: parse_hex_field(fields.next()),
        };
        let vcount = parse_hex_field(fields.next());

        Self {
            game_version: version,
            nazo_values,
            vcount,
        }
    }
}

fn parse_hex_field(field: Option<&str>) -> u32 {
    let field = field.expect("missing field in nazo_vcount.csv").trim();
    let digits = field.strip_prefix("0x").unwrap_or(field);
    u32::from_str_radix(digits, 16).expect("invalid hexadecimal field in nazo_vcount.csv")
}

impl std::fmt::Display for GameVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Black => "Black",
            Self::White => "White",
            Self::Black2 => "Black2",
            Self::White2 => "White2",
        })
    }
}

impl std::fmt::Display for Region {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::JPN => "JPN",
            Self::KOR => "KOR",
            Self::USA => "USA",
            Self::DEU => "DEU",
            Self::FRA => "FRA",
            Self::ESP => "ESP",
            Self::ITA => "ITA",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_version_config_from_csv() {
        let config = VersionConfig::from_version_and_region(GameVersion::White2, Region::USA);

        assert_eq!(config.nazo_values.nazo1, 0x0209AF28);
        assert_eq!(config.nazo_values.nazo2, 0x02039E15);
        assert_eq!(config.nazo_values.nazo3, 0x02200050);
        assert_eq!(config.nazo_values.nazo4, 0x022000A4);
        assert_eq!(config.nazo_values.nazo5, 0x022000A4);
        assert_eq!(config.vcount, 0x82);
    }
}