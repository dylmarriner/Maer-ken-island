use chrono::{DateTime, Utc};
use island_humans::{CreateHumanError, CreateHumanRequest, IslandHumanPopulation, DEFAULT_SEED};
use std::path::PathBuf;

fn usage() -> ! {
    eprintln!("usage:");
    eprintln!("  island_humans founders [--data-dir DIR]");
    eprintln!(
        "  island_humans create <name> <male|female> <birth-rfc3339> <lat> <lon> \
         [--age YEARS] [--height CM] [--build B] [--hair H] [--eyes E] [--skin S] [--data-dir DIR]"
    );
    eprintln!(
        "  --data-dir defaults to ./island-data; every human gets a folder under DIR/humans/"
    );
    std::process::exit(2);
}

/// Split `--flag value` pairs off the end of the arguments.
fn take_flags(args: &[String]) -> (Vec<String>, Vec<(String, String)>) {
    let mut positional = Vec::new();
    let mut flags = Vec::new();
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if let Some(flag) = arg.strip_prefix("--") {
            let value = iter.next().unwrap_or_else(|| usage());
            flags.push((flag.to_string(), value.clone()));
        } else {
            positional.push(arg.clone());
        }
    }
    (positional, flags)
}

fn flag<'a>(flags: &'a [(String, String)], name: &str) -> Option<&'a str> {
    flags
        .iter()
        .rev()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.as_str())
}

fn open(flags: &[(String, String)]) -> IslandHumanPopulation {
    let dir = PathBuf::from(flag(flags, "data-dir").unwrap_or("island-data"));
    match IslandHumanPopulation::open(&dir, DEFAULT_SEED) {
        Ok((population, warnings)) => {
            for warning in warnings {
                eprintln!("warning: {warning}");
            }
            population
        }
        Err(err) => {
            eprintln!("could not open {}: {err}", dir.display());
            std::process::exit(1);
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (positional, flags) = take_flags(&args);

    match positional.as_slice() {
        [] => usage(),
        [cmd] if cmd == "founders" => {
            let population = open(&flags);
            println!(
                "{}",
                serde_json::to_string_pretty(&population.summaries()).unwrap()
            );
        }
        [cmd, name, sex, birth, lat, lon] if cmd == "create" => {
            let birth_instant: DateTime<Utc> = birth.parse().unwrap_or_else(|_| usage());
            let number = |value: &str| value.parse::<f64>().unwrap_or_else(|_| usage());
            // Default age: time lived since the birth instant, recorded in the
            // request so replaying the creation reproduces the same person.
            let age_years = match flag(&flags, "age") {
                Some(age) => number(age),
                None => (Utc::now() - birth_instant).num_seconds() as f64 / (365.25 * 86_400.0),
            };
            let request = CreateHumanRequest {
                name: name.clone(),
                biological_sex: sex.clone(),
                birth_timestamp: birth.clone(),
                birth_latitude: number(lat),
                birth_longitude: number(lon),
                age_years,
                height_cm: flag(&flags, "height").map(number).unwrap_or(170.0),
                build: flag(&flags, "build").unwrap_or("average").to_string(),
                hair_color: flag(&flags, "hair").unwrap_or("brown").to_string(),
                eye_color: flag(&flags, "eyes").unwrap_or("brown").to_string(),
                skin_tone: flag(&flags, "skin").unwrap_or("medium").to_string(),
            };
            let mut population = open(&flags);
            match population.create_human(request, "cli") {
                Ok(created) => {
                    if let Some(err) = &created.storage_error {
                        eprintln!("warning: {err}");
                    }
                    println!("{}", serde_json::to_string_pretty(&created).unwrap());
                }
                Err(CreateHumanError::Invalid(errors)) => {
                    for error in errors {
                        eprintln!("invalid: {error}");
                    }
                    std::process::exit(2);
                }
            }
        }
        _ => usage(),
    }
}
