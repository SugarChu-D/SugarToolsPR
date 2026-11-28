use std::error::Error;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use rng_core::gpu::workgroup_size::WorkgroupSize;
use rng_core::models::ds_config::DSConfig;
use rng_core::models::game_version::GameVersion;
use search::profiles::load_profiles;
use search::white2_whack_a_mole::{DrilburSearchResult, drilbur_search};

fn main() -> Result<(), Box<dyn Error>> {
    let default_config = default_config_path();
    let config_path = prompt_path("config.toml path", &default_config)?;
    let (ds_config, workgroup_size_opt) = select_profile(&config_path)?;

    if ds_config.version != GameVersion::White2 {
        eprintln!(
            "warning: ds_config version is {:?}, expected White2",
            ds_config.version
        );
    }

    let workgroup_size = WorkgroupSize::try_from_u32(workgroup_size_opt.unwrap_or(256))
        .map_err(|e| -> Box<dyn Error> { e.into() })?;
    println!("Workgroup size: {}", workgroup_size.as_u32());
    println!("Searching for Drilbur... ");

    let results = pollster::block_on(async { drilbur_search(ds_config, workgroup_size).await });

    let output_path = default_output_path();
    let text = build_output_text(&results);
    fs::write(&output_path, text)?;
    println!("wrote {}", output_path.display());

    Ok(())
}

fn select_profile(path: &Path) -> Result<(DSConfig, Option<u32>), Box<dyn Error>> {
    let file = load_profiles(path)?;
    let index = match file.ds_configs.len() {
        0 => return Err("no profiles found in config.toml".into()),
        1 => 0,
        _ => {
            for (index, name) in file.ds_configs.keys().enumerate() {
                println!("{}: {:?}", index + 1, name);
            }
            prompt("Profile number", "")?
                .parse::<usize>()
                .ok()
                .and_then(|number| number.checked_sub(1))
                .ok_or("invalid profile number")?
        }
    };
    let config = file
        .ds_configs
        .into_values()
        .nth(index)
        .ok_or("invalid profile number")?;
    Ok((config, file.workgroup_size))
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

fn prompt_path(label: &str, default: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let default_str = default.to_string_lossy();
    let s = prompt(label, &default_str)?;
    Ok(PathBuf::from(s))
}

fn default_config_path() -> PathBuf {
    match std::env::current_exe() {
        Ok(exe) => exe
            .parent()
            .map(|dir| dir.join("config.toml"))
            .unwrap_or_else(|| PathBuf::from("config.toml")),
        Err(_) => PathBuf::from("config.toml"),
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
            result
                .cloud_advances
                .iter()
                .map(|a| a.to_string())
                .collect::<Vec<_>>()
                .join("|"),
            result
                .wild_advances
                .iter()
                .map(|a| a.to_string())
                .collect::<Vec<_>>()
                .join("|"),
        );
        out.push_str(&line);
    }
    out
}
