use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;

use rng_core::models::ds_config::DSConfig;
use serde::Deserialize;

/// Profiles keyed by their names in config.toml, with the shared GPU setting.
#[derive(Debug, Deserialize)]
pub struct ProfileConfig {
    pub ds_configs: BTreeMap<String, DSConfig>,
    pub workgroup_size: Option<u32>,
}

/// Reads all profiles from a config.toml without selecting one.
pub fn load_profiles(path: &Path) -> io::Result<ProfileConfig> {
    let text = fs::read_to_string(path)?;
    toml::from_str(&text).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_all_named_profiles_and_shared_workgroup_size() {
        let file: ProfileConfig = toml::from_str(
            r#"
            workgroup_size = 64

            [ds_configs.secondary]
            version = "White2"
            region = "JPN"
            timer0 = 0x10fa
            is_dslite = false
            mac = 0x0009bf6d93ce

            [ds_configs."main console"]
            version = "White2"
            region = "JPN"
            timer0 = "0x10fb"
            is_dslite = true
            mac = "0x0009bf6d93cf"
            "#,
        )
        .unwrap();

        let names: Vec<_> = file.ds_configs.keys().map(String::as_str).collect();
        assert_eq!(names, ["main console", "secondary"]);
        assert_eq!(file.ds_configs["main console"].timer0, 0x10fb);
        assert_eq!(file.ds_configs["secondary"].timer0, 0x10fa);
        assert_eq!(file.workgroup_size, Some(64));
    }
}
