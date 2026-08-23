use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

use rng_core::models::ds_config::DSConfig;
use rng_core::models::game_version::GameVersion;
use search::white2_whack_a_mole::{drilbur_search, DrilburSearchResult};
use serde::Deserialize;

#[derive(Deserialize)]
struct DsConfigFile {
    ds_configs: HashMap<String, DSConfig>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let default_config = default_config_path();
    let config_path = prompt_path("ds_config.json path", &default_config)?;
    let ds_config = load_single_profile(&config_path)?;

    if ds_config.version != GameVersion::White2 {
        eprintln!(
            "warning: ds_config version is {:?}, expected White2",
            ds_config.version
        );
    }

    let results =
        pollster::block_on(async { drilbur_search(ds_config).await });

    let output_path = default_output_path();
    let text = build_output_text(&results);
    fs::write(&output_path, text)?;
    println!("wrote {}", output_path.display());

    Ok(())
}

fn load_config_file(path: &PathBuf) -> Result<DsConfigFile, Box<dyn Error>> {
    let text = fs::read_to_string(path)?;
    let file: DsConfigFile = serde_json::from_str(&text)?;
    Ok(file)
}

fn load_single_profile(path: &PathBuf) -> Result<DSConfig, Box<dyn Error>> {
    let file = load_config_file(path)?;
    let mut iter = file.ds_configs.iter();
    let (name, cfg) = iter
        .next()
        .ok_or("no profiles found in ds_config.json")?;
    if iter.next().is_some() {
        eprintln!("warning: multiple profiles found; using '{}'", name);
    }
    Ok(*cfg)
}

fn prompt(label: &str, default: &str) -> Result<String, Box<dyn Error>> {
    print!("{} [{}]: ", label, default);
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim();
    if input.is_empty() {
        Ok(default.to_string())
    } else {
        Ok(input.to_string())
    }
}

fn prompt_path(label: &str, default: &PathBuf) -> Result<PathBuf, Box<dyn Error>> {
    let default_str = default.to_string_lossy();
    let s = prompt(label, &default_str)?;
    Ok(PathBuf::from(s))
}

fn default_config_path() -> PathBuf {
    match std::env::current_exe() {
        Ok(exe) => exe
            .parent()
            .map(|dir| dir.join("ds_config.json"))
            .unwrap_or_else(|| PathBuf::from("ds_config.json")),
        Err(_) => PathBuf::from("ds_config.json"),
    }
}

fn default_output_path() -> PathBuf {
    match std::env::current_exe() {
        Ok(exe) => exe
            .parent()
            .map(|dir| dir.join("result.txt"))
            .unwrap_or_else(|| PathBuf::from("result.txt")),
        Err(_) => PathBuf::from("result.txt"),
    }
}

fn build_output_text(results: &[DrilburSearchResult]) -> String {
    let mut out = String::new();
    out.push_str(&format!("total_results:{}\n", results.len()));
    out.push_str("seed0, seed1, year, month, day, hour, minute, second, key_presses, h, a, b, c, d, s, clouds, advances\n");
    for result in results {
        let line = format!(
            "{:016X}, {:016X}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}\n",
            result.seed0,
            result.seed1,
            result.year,
            result.month,
            result.day,
            result.hour,
            result.minute,
            result.second,
            result.key_presses.pressed_keys_string(),
            result.ivs[0],
            result.ivs[1],
            result.ivs[2],
            result.ivs[3],
            result.ivs[4],
            result.ivs[5],
            result.cloud_advances.iter().map(|a| a.to_string()).collect::<Vec<_>>().join("|"),
            result.wild_advances.iter().map(|a| a.to_string()).collect::<Vec<_>>().join("|"),
        );
        out.push_str(&line);
    }
    out
}
