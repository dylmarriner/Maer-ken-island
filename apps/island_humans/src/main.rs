use chrono::{DateTime, Utc};
use island_humans::IslandHumanPopulation;
use mk_core::human::{astrology::GeoCoordinates, BiologicalSex};

fn parse_sex(value: &str) -> Option<BiologicalSex> {
    match value.to_ascii_lowercase().as_str() {
        "male" | "m" => Some(BiologicalSex::Male),
        "female" | "f" => Some(BiologicalSex::Female),
        "neutral" | "n" => Some(BiologicalSex::Neutral),
        _ => None,
    }
}

fn usage() -> ! {
    eprintln!("usage:");
    eprintln!("  cargo run -p island_humans -- founders");
    eprintln!("  cargo run -p island_humans -- create <name> <male|female|neutral> <birth-rfc3339> <lat> <lon> [location]");
    std::process::exit(2);
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut population = IslandHumanPopulation::with_founders();

    match args.as_slice() {
        [] => {
            println!(
                "{}",
                serde_json::to_string_pretty(&population.summaries()).unwrap()
            );
        }
        [cmd] if cmd == "founders" => {
            println!(
                "{}",
                serde_json::to_string_pretty(&population.summaries()).unwrap()
            );
        }
        [cmd, name, sex, birth, lat, lon, rest @ ..] if cmd == "create" => {
            let sex = parse_sex(sex).unwrap_or_else(|| usage());
            let birth: DateTime<Utc> = birth.parse().unwrap_or_else(|_| usage());
            let latitude: f64 = lat.parse().unwrap_or_else(|_| usage());
            let longitude: f64 = lon.parse().unwrap_or_else(|_| usage());
            let location = rest
                .first()
                .map(String::as_str)
                .unwrap_or("Maer-Ken Island");
            let summary = population
                .create_human(
                    name,
                    sex,
                    birth,
                    GeoCoordinates {
                        latitude,
                        longitude,
                    },
                    location,
                )
                .unwrap_or_else(|error| panic!("could not create human: {error:?}"));
            println!("{}", serde_json::to_string_pretty(&summary).unwrap());
        }
        _ => usage(),
    }
}
