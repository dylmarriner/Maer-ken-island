//! `island` — Maer-Ken Island.
//!
//! `island serve [--data-dir DIR] [--seed HEX64] [--bind ADDR]` serves the
//! dashboard: an overview of the population, the roster with everything
//! stored about each person, and the Human Creator.

mod serve;

use serve::auth::{ControlAuth, TOKEN_ENV};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

const USAGE: &str = "\
island serve [--data-dir ./island-data] [--seed <64 hex chars>] [--bind 127.0.0.1:8080]

Serves the island dashboard: the overview at /, the roster at /people and the
Human Creator at /creator, with the JSON behind them under /api.

  --data-dir DIR   where people are stored (default ./island-data)
  --seed HEX       64 hex characters; the same seed and the same creations
                   rebuild the same people (default all zeroes)
  --bind ADDR      address and port to listen on (default 127.0.0.1:8080)

Reading never needs a token. Creating people needs ISLAND_CONTROL_TOKEN as a
bearer token when it is set; without one, only a loopback bind accepts
creations. Set ISLAND_ACCESS_LOG=off to silence the request log.";

/// Printed when someone asks for help: that is not an error, so it goes to
/// stdout and exits cleanly.
fn help() -> ! {
    println!("{USAGE}");
    std::process::exit(0);
}

fn usage() -> ! {
    eprintln!("{USAGE}");
    std::process::exit(2);
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some((command, rest)) = args.split_first() else {
        usage()
    };
    if matches!(command.as_str(), "help" | "--help" | "-h") {
        help();
    }
    if command != "serve" {
        eprintln!("island has one command, `serve`, not `{command}`.\n");
        usage();
    }
    let mut data_dir = PathBuf::from("island-data");
    let mut seed = island_humans::DEFAULT_SEED;
    let mut bind: std::net::SocketAddr = "127.0.0.1:8080".parse().expect("valid default");
    let mut iter = rest.iter();
    while let Some(flag) = iter.next() {
        if matches!(flag.as_str(), "--help" | "-h") {
            help();
        }
        let value = iter.next().unwrap_or_else(|| {
            eprintln!("{flag} needs a value.\n");
            usage()
        });
        match flag.as_str() {
            "--data-dir" => data_dir = PathBuf::from(value),
            "--seed" => {
                let bytes = hex::decode(value).unwrap_or_else(|_| usage());
                seed = bytes.try_into().unwrap_or_else(|_| usage());
            }
            "--bind" => bind = value.parse().unwrap_or_else(|_| usage()),
            _ => usage(),
        }
    }

    let (population, warnings) = match island_humans::IslandHumanPopulation::open(&data_dir, seed) {
        Ok(opened) => opened,
        Err(err) => {
            eprintln!("could not open {}: {err}", data_dir.display());
            std::process::exit(1);
        }
    };
    for warning in warnings {
        eprintln!("warning: {warning}");
    }
    let auth = ControlAuth::resolve(std::env::var(TOKEN_ENV).ok(), bind.ip());
    println!("Maer-Ken Island dashboard is up.");
    println!("  Overview        http://{bind}/");
    println!("  People          http://{bind}/people");
    println!("  Create a human  http://{bind}/creator");
    println!(
        "{} on the island. Creating people: {}.",
        match population.len() {
            1 => "1 person".to_string(),
            n => format!("{n} people"),
        },
        auth.describe()
    );
    println!(
        "Their records are in {}. Back up humans/.secret_storage_key, or set MK_STORAGE_KEY \
         yourself — without it the records cannot be read again.",
        data_dir.display()
    );
    println!("Press Ctrl-C to stop. Set ISLAND_ACCESS_LOG=off to silence the request log.");

    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    if let Err(err) = runtime.block_on(serve::server::run(
        Arc::new(Mutex::new(population)),
        auth,
        bind,
    )) {
        eprintln!("could not serve on {bind}: {err}");
        std::process::exit(1);
    }
}
