//! `island_preview` — headless previews of the island region.
//!
//! ```text
//! island_preview geophysics --profile <path> [--seed <64 hex>] --out <dir>
//! island_preview gallery --profile <path> --seeds <n> --out <dir>
//! island_preview physical --profile <path> [--seed <64 hex>] --days <n> --out <dir>
//! island_preview life (--scenario <path> | --from <file>) [--days <n>]
//!                      [--save <file>] [--humans <dir>] --out <dir>
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
  island_preview life (--scenario <path> | --from <file>) [--days <n>]
                      [--save <file>] [--humans <dir>] --out <dir>

`life` renders the island: biomes, resources, the estate plan and its trees.
  --days <n>     simulate n days before rendering (default 0, just bootstrapped)
  --from <file>  start from a saved island instead of bootstrapping it, which
                 carries its own scenario, so --scenario is not needed with it
  --save <file>  write the island to a file afterwards, to be read with --from
  --humans <dir> keep a folder for every human under <dir>/<run id>/humans

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
        let out = PathBuf::from(flag(&args, "--out").ok_or(USAGE)?);
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
            // A saved island carries its own scenario, so `--scenario` is
            // only asked for when there is no island to read.
            None => {
                let path = PathBuf::from(flag(&args, "--scenario").ok_or(USAGE)?);
                let scenario = mk_island::IslandScenario::load(&path).map_err(|e| e.to_string())?;
                mk_engine::regional::life::IslandLife::bootstrap(scenario, canon)
                    .map_err(|e| e.to_string())?
            }
        };
        if let Some(root) = flag(&args, "--humans") {
            let root = PathBuf::from(root);
            let run = life
                .enable_human_store(&root)
                .map_err(|e| format!("cannot keep human folders in {}: {e}", root.display()))?;
            println!(
                "run {run}: folders in {}",
                root.join(run.as_str()).display()
            );
        }
        if days > 0 {
            life.advance(days * 86_400).map_err(|e| e.to_string())?;
            println!("simulated {days} day(s)");
        }
        if let Some(store) = life.human_store() {
            match store.failures() {
                (0, _) => println!("human folders written with no failures"),
                (n, reason) => println!(
                    "warning: {n} human folder write(s) failed, most recently: {}",
                    reason.unwrap_or("unknown")
                ),
            }
        }
        if let Some(path) = flag(&args, "--save") {
            let path = PathBuf::from(path);
            mk_engine::io::save_island_snapshot(&mut life, &path)
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
