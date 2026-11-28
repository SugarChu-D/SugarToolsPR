use std::error::Error;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use rng_core::gpu::workgroup_size::WorkgroupSize;
use rng_core::lcg::nature::Nature;
use rng_core::lcg::tid_impl::get_frigate_pass;
use rng_core::models::ds_config::DSConfig;
use rng_core::models::game_version::GameVersion;
use search::profiles::load_profiles;
use search::white2_tepig::TepigSearchResult;
use search::white2_tepig::{BW2Mode, white2_tepig_dragonite_search};

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
    let results = pollster::block_on(async {
        white2_tepig_dragonite_search(ds_config, Nature::new(4), BW2Mode::Normal, workgroup_size)
            .await
    });

    let output_path = default_output_path();
    let text = build_text(&results);
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

fn build_text(results: &[TepigSearchResult]) -> String {
    let mut out = String::new();
    out.push_str(&format!("total_results:{}\n", results.len()));
    for r in results {
        out.push_str(&format!(
            "seed0: {:016X}\nseed1: {:016X}\n",
            r.seed0, r.seed1,
        ));
        out.push_str(&format!(
            "date: {:02}/{:02}/{:02} {:02}:{:02}:{:02} key: {}\n",
            r.year, r.month, r.day, r.hour, r.minute, r.second, r.key_presses
        ));
        out.push_str(&format!(
            "TID: {} Frigate: {}\n",
            r.tid,
            get_frigate_pass(r.tid)
        ));
        out.push_str(&format!("ivs=: {:?} iv_step: {}\n", r.ivs, r.tepig_iv_step));
        out.push_str(&format!("tepig_frames: {:?}\n", r.tepig_frames));

        out.push_str("pidove_frames:\n");
        for (frame, poke) in &r.pidove_frames {
            let nature = poke.nature.as_ref().map(|n| n.name()).unwrap_or("None");
            let lv = if poke.slot.is_some_and(|s| s < 20) {
                "Lv.2"
            } else {
                "Lv.4"
            };
            out.push_str(&format!("{}:{}:{} ", frame, lv, nature));
        }
        out.push('\n');

        out.push_str("psyduck_frames:\n");
        for (frame, poke) in &r.psyduck_frames {
            let nature = poke.nature.as_ref().map(|n| n.name()).unwrap_or("None");
            out.push_str(&format!("{}:{} ", frame, nature));
        }
        out.push('\n');

        out.push_str("candy_frames:\n");
        for (frame, grottos) in &r.candy_frames {
            out.push_str(&format!("  {}: ", frame));
            let mut any = false;
            for i in 0..grottos.grottos.len() {
                let g = grottos.get(i).unwrap_or_default();
                if g.slot().is_some() {
                    any = true;
                    out.push_str(&format!(
                        "#{}(sub={:?},slot={:?},gender={:?}) ",
                        i,
                        g.sub_slot(),
                        g.slot(),
                        g.gender()
                    ));
                }
            }
            if !any {
                out.push_str("(none)");
            }
            out.push('\n');
        }
        out.push('\n');
    }
    out
}
