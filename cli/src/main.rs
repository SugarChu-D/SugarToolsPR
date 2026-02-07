use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
use rng_core::lcg::grotto::Grottos;
use rng_core::lcg::nature::Nature;
use rng_core::lcg::wild_poke::WildPoke;
use rng_core::models::ds_config::DSConfig;
use rng_core::models::game_version::GameVersion;
use search::white2_tepig::{
    white2_tepig_dragonite_search, white2_tepig_search, TepigSearchResult,
};
use serde::Deserialize;
use serde::Serialize;

#[derive(Parser)]
#[command(name = "sugartools")]
#[command(author, version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// White2 Tepig search
    White2Tepig {
        /// Path to ds_config.json
        #[arg(long, default_value = "ds_config.json")]
        config: PathBuf,
        /// Profile name under ds_configs
        #[arg(long, default_value = "profile3")]
        profile: String,
        /// Date in YY-MM-DD (required for normal mode)
        #[arg(long)]
        date: Option<String>,
        /// Nature (naughty|rash|4|19)
        #[arg(long)]
        nature: String,
        /// Search mode
        #[arg(long, value_enum, default_value_t = TepigMode::Normal)]
        mode: TepigMode,
        /// Output format
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        output: OutputFormat,
    },
}

#[derive(Copy, Clone, ValueEnum)]
enum TepigMode {
    Normal,
    Dragonite,
}

#[derive(Copy, Clone, ValueEnum)]
enum OutputFormat {
    Text,
    Json,
}

#[derive(Deserialize)]
struct DsConfigFile {
    ds_configs: HashMap<String, DSConfig>,
}

#[derive(Serialize)]
struct OutputResult {
    seed0: u64,
    seed1: u64,
    year: u8,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
    key_presses: String,
    ivs: [u8; 6],
    tepig_iv_step: u8,
    tepig_frames: Vec<u32>,
    candy_frames: Vec<CandyFrame>,
    pidove_frames: Vec<WildFrame>,
    psyduck_frames: Vec<WildFrame>,
}

#[derive(Serialize)]
struct CandyFrame {
    frame: u32,
    grottos: Vec<GrottoEntry>,
}

#[derive(Serialize)]
struct GrottoEntry {
    index: usize,
    sub_slot: Option<u32>,
    slot: Option<u32>,
    gender: Option<u32>,
}

#[derive(Serialize)]
struct WildFrame {
    frame: u32,
    slot: Option<u32>,
    poke_code: Option<u32>,
    nature_id: Option<u8>,
    nature_name: Option<&'static str>,
    item: Option<u32>,
    ability: Option<u8>,
    gender: Option<u8>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    match cli.command {
        Command::White2Tepig {
            config,
            profile,
            date,
            nature,
            mode,
            output,
        } => run_white2_tepig(config, profile, date, nature, mode, output),
    }
}

fn run_white2_tepig(
    config_path: PathBuf,
    profile: String,
    date: Option<String>,
    nature: String,
    mode: TepigMode,
    output: OutputFormat,
) -> Result<(), Box<dyn Error>> {
    let ds_config = load_ds_config(&config_path, &profile)?;
    if ds_config.Version != GameVersion::White2 {
        eprintln!(
            "warning: profile '{}' is {:?}, expected White2",
            profile, ds_config.Version
        );
    }

    let nat = parse_nature(&nature)?;

    let results = match mode {
        TepigMode::Normal => {
            let (year, month, day) = parse_date(date)?;
            pollster::block_on(async { white2_tepig_search(ds_config, year, month, day, nat).await })
        }
        TepigMode::Dragonite => {
            pollster::block_on(async { white2_tepig_dragonite_search(ds_config, nat).await })
        }
    };

    match output {
        OutputFormat::Text => print_text(&results),
        OutputFormat::Json => print_json(&results)?,
    }

    Ok(())
}

fn load_ds_config(path: &PathBuf, profile: &str) -> Result<DSConfig, Box<dyn Error>> {
    let text = fs::read_to_string(path)?;
    let file: DsConfigFile = serde_json::from_str(&text)?;
    let cfg = file
        .ds_configs
        .get(profile)
        .ok_or_else(|| format!("profile '{}' not found in {}", profile, path.display()))?;
    Ok(*cfg)
}

fn parse_nature(s: &str) -> Result<Nature, Box<dyn Error>> {
    let lower = s.trim().to_lowercase();
    let id = match lower.as_str() {
        "naughty" => 4,
        "rash" => 19,
        _ => lower.parse::<u8>().map_err(|_| "invalid nature")?,
    };
    if id != 4 && id != 19 {
        return Err("nature must be naughty|rash|4|19".into());
    }
    Ok(Nature::new(id))
}

fn parse_date(date: Option<String>) -> Result<(u8, u8, u8), Box<dyn Error>> {
    let date = date.ok_or("date is required in normal mode (use --date YY-MM-DD)")?;
    let parts: Vec<&str> = date.split('-').collect();
    if parts.len() != 3 {
        return Err("date must be YY-MM-DD".into());
    }
    let year = parts[0].parse::<u8>()?;
    let month = parts[1].parse::<u8>()?;
    let day = parts[2].parse::<u8>()?;
    Ok((year, month, day))
}

fn print_text(results: &[TepigSearchResult]) {
    println!("total_results={}", results.len());
    for r in results {
        println!(
            "seed0={:016X} seed1={:016X} date={:02}/{:02} {:02}:{:02}:{:02} kp={}",
            r.seed0, r.seed1, r.month, r.day, r.hour, r.minute, r.second, r.key_presses
        );
        println!("ivs={:?} iv_step={}", r.ivs, r.tepig_iv_step);
        println!("tepig_frames={:?}", r.tepig_frames);

        print!("pidove_frames=");
        for (frame, poke) in &r.pidove_frames {
            let nature = poke.nature.as_ref().map(|n| n.name()).unwrap_or("None");
            let lv = if poke.slot.is_some_and(|s| s < 20) { "Lv.2" } else { "Lv.4" };
            print!("{}:{}:{} ", frame, lv, nature);
        }
        println!();

        print!("psyduck_frames=");
        for (frame, poke) in &r.psyduck_frames {
            let nature = poke.nature.as_ref().map(|n| n.name()).unwrap_or("None");
            print!("{}:{} ", frame, nature);
        }
        println!();

        println!("candy_frames:");
        for (frame, grottos) in &r.candy_frames {
            print!("  {}: ", frame);
            let mut any = false;
            for i in 0..grottos.grottos.len() {
                let g = grottos.get(i).unwrap_or_default();
                if g.slot().is_some() {
                    any = true;
                    print!(
                        "#{}(sub={:?},slot={:?},gender={:?}) ",
                        i,
                        g.sub_slot(),
                        g.slot(),
                        g.gender()
                    );
                }
            }
            if !any {
                print!("(none)");
            }
            println!();
        }
        println!();
    }
}

fn print_json(results: &[TepigSearchResult]) -> Result<(), Box<dyn Error>> {
    let out: Vec<OutputResult> = results.iter().map(to_output).collect();
    let text = serde_json::to_string_pretty(&out)?;
    println!("{text}");
    Ok(())
}

fn to_output(r: &TepigSearchResult) -> OutputResult {
    OutputResult {
        seed0: r.seed0,
        seed1: r.seed1,
        year: r.year,
        month: r.month,
        day: r.day,
        hour: r.hour,
        minute: r.minute,
        second: r.second,
        key_presses: r.key_presses.clone(),
        ivs: r.ivs,
        tepig_iv_step: r.tepig_iv_step,
        tepig_frames: r.tepig_frames.clone(),
        candy_frames: r
            .candy_frames
            .iter()
            .map(|(frame, grottos)| CandyFrame {
                frame: *frame,
                grottos: grottos_to_entries(grottos),
            })
            .collect(),
        pidove_frames: r.pidove_frames.iter().map(to_wild_frame).collect(),
        psyduck_frames: r.psyduck_frames.iter().map(to_wild_frame).collect(),
    }
}

fn grottos_to_entries(grottos: &Grottos) -> Vec<GrottoEntry> {
    let mut out = Vec::new();
    for i in 0..grottos.grottos.len() {
        if let Some(g) = grottos.get(i) {
            let entry = GrottoEntry {
                index: i,
                sub_slot: g.sub_slot(),
                slot: g.slot(),
                gender: g.gender(),
            };
            if entry.sub_slot.is_some() || entry.slot.is_some() || entry.gender.is_some() {
                out.push(entry);
            }
        }
    }
    out
}

fn to_wild_frame((frame, poke): &(u32, WildPoke)) -> WildFrame {
    WildFrame {
        frame: *frame,
        slot: poke.slot,
        poke_code: poke.poke_code,
        nature_id: poke.nature.as_ref().map(|n| n.id()),
        nature_name: poke.nature.as_ref().map(|n| n.name()),
        item: poke.item,
        ability: poke.ability(),
        gender: poke.gender(),
    }
}
