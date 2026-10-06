//! `island_preview` — headless previews of the island region.
//!
//! ```text
//! island_preview geophysics --profile <path> [--seed <64 hex>] --out <dir>
//! island_preview gallery --profile <path> --seeds <n> --out <dir>
//! ```

use std::path::PathBuf;
use std::process::ExitCode;

use island_preview::{generate, render_gallery, render_geophysics, Output};
use mk_island::IslandProfile;

const USAGE: &str = "usage:
  island_preview geophysics --profile <path> [--seed <64 hex chars>] --out <dir>
  island_preview gallery --profile <path> --seeds <n> --out <dir>";

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
