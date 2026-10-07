//! `island_preview` — headless previews of the island region.
//!
//! ```text
//! island_preview geophysics --profile <path> [--seed <64 hex>] --out <dir>
//! island_preview gallery --profile <path> --seeds <n> --out <dir>
//! island_preview physical --profile <path> [--seed <64 hex>] --days <n> --out <dir>
//! island_preview life --scenario <path> [--days <n>] [--from <file>] [--save <file>] --out <dir>
//! ```

use std::path::PathBuf;
use std::process::ExitCode;

use island_preview::{
    generate, render_gallery, render_geophysics, render_physical, simulate_physical, Output,
};
use mk_island::IslandProfile;

const USAGE: &str = "usage:
  island_preview geophysics --profile <path> [--seed <64 hex chars>] --out <dir>
  island_preview gallery --profile <path> --seeds <n> --out <dir>
  island_preview physical --profile <path> [--seed <64 hex chars>] --days <n> --out <dir>
  island_preview life --scenario <path> [--days <n>] [--from <file>] [--save <file>] --out <dir>

`life` renders the island: biomes, resources, the estate plan and its trees.
  --days <n>     simulate n days before rendering (default 0, just bootstrapped)
  --from <file>  start from a saved island instead of bootstrapping it
  --save <file>  write the island to a file afterwards, to be read with --from

A saved island only loads against the canon it was written under, which is the
one the scenario names.";

fn flag(args: &[String], name: &str) -> Option<String> {
    args.windows(2).find(|w| w[0] == name).map(|w| w[1].clone())
}

fn write_all(out: &PathBuf, files: &[Output]) -> Result<(), String> {
    std::fs::create_dir_all(out).map_err(|e| format!("cannot create {}: {e}", out.display()))?;
    for (name, bytes) in files {
        let path = out.join(name);
        std::fs::write(&path, bytes)
            .map_err(|e| format!("cannot write {}: {e}", path.display()))?;
        println!("wrote {}", path.display());
    }
    Ok(())
}

fn parse_seed(hex: &str) -> Result<[u8; 32], String> {
    let probe = IslandProfile {
        seed: Some(hex.to_string()),
        ..IslandProfile::default_nz_scale()
    };
    probe
        .seed_bytes()
        .ok_or_else(|| format!("seed {hex:?} is not 64 hex characters"))
}

fn run(args: Vec<String>) -> Result<(), String> {
    let command = args.first().ok_or(USAGE)?;
    if command == "life" {
        let scenario_path = flag(&args, "--scenario").ok_or(USAGE)?;
        let out = PathBuf::from(flag(&args, "--out").ok_or(USAGE)?);
        let scenario = mk_island::IslandScenario::load(&PathBuf::from(&scenario_path))
            .map_err(|e| e.to_string())?;
        let days: u64 = match flag(&args, "--days") {
            Some(n) => n
                .parse()
                .map_err(|_| format!("--days {n:?} is not a number"))?,
            None => 0,
        };
        let canon = std::sync::Arc::new(island_preview::island_canon());
        let mut life = match flag(&args, "--from") {
            Some(path) => {
                let path = PathBuf::from(path);
                let life = mk_engine::io::load_island_snapshot(canon, &path)
                    .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
                println!(
                    "read {} ({:.1} days in)",
                    path.display(),
                    life.sim_time_s as f64 / 86_400.0
                );
                life
            }
            None => mk_engine::regional::life::IslandLife::bootstrap(scenario, canon)
                .map_err(|e| e.to_string())?,
        };
        if days > 0 {
            life.advance(days * 86_400).map_err(|e| e.to_string())?;
            println!("simulated {days} day(s)");
        }
        if let Some(path) = flag(&args, "--save") {
            let path = PathBuf::from(path);
            mk_engine::io::save_island_snapshot(&life, &path)
                .map_err(|e| format!("cannot write {}: {e}", path.display()))?;
            let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            println!("wrote {} ({} MB)", path.display(), size / 1_048_576);
        }
        return write_all(&out, &island_preview::life::render_life(&life));
    }
    let profile_path = flag(&args, "--profile").ok_or(USAGE)?;
    let out = PathBuf::from(flag(&args, "--out").ok_or(USAGE)?);
    let profile = IslandProfile::load(&PathBuf::from(&profile_path)).map_err(|e| e.to_string())?;
    match command.as_str() {
        "geophysics" => {
            let seed = match flag(&args, "--seed") {
                Some(hex) => parse_seed(&hex)?,
                None => profile
                    .seed_bytes()
                    .ok_or("the profile pins no seed; pass --seed")?,
            };
            let (domain, result) = generate(&profile, seed);
            let g = result.map_err(|e| e.to_string())?;
            write_all(&out, &render_geophysics(&domain, &g, &seed))?;
            let unmet = profile.shape.unmet(&g.shape);
            if !unmet.is_empty() {
                eprintln!(
                    "note: this island fails the shape requirements: {}",
                    unmet.join("; ")
                );
            }
            Ok(())
        }
        "gallery" => {
            let n: u32 = flag(&args, "--seeds")
                .ok_or(USAGE)?
                .parse()
                .map_err(|_| "--seeds must be a whole number")?;
            write_all(&out, &render_gallery(&profile, n))
        }
        "physical" => {
            let seed = match flag(&args, "--seed") {
                Some(hex) => parse_seed(&hex)?,
                None => profile
                    .seed_bytes()
                    .ok_or("the profile pins no seed; pass --seed")?,
            };
            let days: u64 = flag(&args, "--days")
                .ok_or(USAGE)?
                .parse()
                .map_err(|_| "--days must be a whole number")?;
            let run = simulate_physical(&profile, seed, days).map_err(|e| e.to_string())?;
            write_all(&out, &render_physical(&run))
        }
        _ => Err(USAGE.to_string()),
    }
}

fn main() -> ExitCode {
    match run(std::env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
