// Gameversion
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameVersion {
    Black,
    White,
    Black2,
    White2,
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
pub struct VCount(pub u8);

#[derive(Debug, Clone, Copy)]
pub struct VersionConfig {
    pub game_version: GameVersion,
    pub nazo_values: NazoValues,
    pub vcount: VCount,
}

impl VersionConfig {
    pub const fn from_version(version: GameVersion) -> Self {
        match version {
            GameVersion::Black => Self {
                game_version: version,
                nazo_values: NazoValues {
                    nazo1: 0x02215F10,
                    nazo2: 0x0221600C,
                    nazo3: 0x0221600C,
                    nazo4: 0x02216058,
                    nazo5: 0x02216058,
                },
                vcount: VCount(0x60),
            },
            GameVersion::White => Self {
                game_version: version,
                nazo_values: NazoValues {
                    nazo1: 0x02215f30,
                    nazo2: 0x0221602C,
                    nazo3: 0x0221602C,
                    nazo4: 0x02216078,
                    nazo5: 0x02216078,
                },
                vcount: VCount(0x5f),
            },
            GameVersion::Black2 => Self {
                game_version: version,
                nazo_values: NazoValues {
                    nazo1: 0x0209A8DC,
                    nazo2: 0x02039AC9,
                    nazo3: 0x021FF9B0,
                    nazo4: 0x021FFA04,
                    nazo5: 0x021FFA04,
                },
                vcount: VCount(0x82),
            },
            GameVersion::White2 => Self {
                game_version: version,
                nazo_values: NazoValues {
                    nazo1: 0x0209A8FC,
                    nazo2: 0x02039AF5,
                    nazo3: 0x021FF9D0,
                    nazo4: 0x021FFA24,
                    nazo5: 0x021FFA24,
                },
                vcount: VCount(0x82),
            },
        }
    }
}