//! `island` — Maer-Ken Island.
//!
//! `island serve [--data-dir DIR] [--seed HEX64] [--bind ADDR]` serves the
//! people dashboard and Human Creator.

mod serve;

use serve::auth::{ControlAuth, TOKEN_ENV};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

fn usage() -> ! {
    eprintln!("usage: island serve [--data-dir ./island-data] [--seed <64 hex chars>] [--bind 127.0.0.1:8080]");
    eprintln!("  writes need {TOKEN_ENV} as a bearer token if set; without it, only loopback binds accept writes");
    std::process::exit(2);
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some((command, rest)) = args.split_first() else {
        usage()
    };
    if command != "serve" {
        usage();
    }
    let mut data_dir = PathBuf::from("island-data");
    let mut seed = island_humans::DEFAULT_SEED;
    let mut bind: std::net::SocketAddr = "127.0.0.1:8080".parse().expect("valid default");
    let mut iter = rest.iter();
    while let Some(flag) = iter.next() {
        let value = iter.next().unwrap_or_else(|| usage());
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
    println!(
        "Maer-Ken Island dashboard: http://{bind}/  ({} people, writes: {})",
        population.len(),
        auth.describe()
    );
    println!(
        "Human data: {}  (back up humans/.secret_storage_key or set MK_STORAGE_KEY)",
        data_dir.display()
    );

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
