//! `island` — Maer-Ken Island.
//!
//! `island serve [--data-dir DIR] [--seed HEX64] [--bind ADDR]` serves the
//! people dashboard and Human Creator.

mod serve;

use serve::auth::{ControlAuth, TOKEN_ENV};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

fn usage() -> ! {
    eprintln!("{}", island_runner::RUNNER_USAGE);
    eprintln!("  writes need {TOKEN_ENV} as a bearer token if set; without it, only loopback binds accept writes");
    std::process::exit(2);
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(result) = island_runner::run_cli(&args) {
        let command = args.first().map_or("", String::as_str);
        match result {
            Ok(text) => {
                println!("{text}");
                return;
            }
            Err(message) => {
                eprintln!("island {command}: {message}");
                std::process::exit(1);
            }
        }
    }
    let Some((command, rest)) = args.split_first() else {
        usage()
    };
    if command != "serve" {
        usage();
    }
    if rest.iter().any(|a| a == "--scenario" || a == "--snapshot") {
        serve_island(rest);
        return;
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

/// `island serve --scenario|--snapshot ...`: the live island.
fn serve_island(rest: &[String]) {
    let flag = |name: &str| rest.windows(2).find(|w| w[0] == name).map(|w| w[1].clone());
    let bind: std::net::SocketAddr = flag("--bind")
        .unwrap_or_else(|| "127.0.0.1:8080".into())
        .parse()
        .unwrap_or_else(|_| usage());
    // `--snapshot` is the file to resume from, as `island run --load`.
    let mut world = match island_runner::open_world(
        &rest
            .iter()
            .map(|a| {
                if a == "--snapshot" {
                    "--load".to_string()
                } else {
                    a.clone()
                }
            })
            .collect::<Vec<_>>(),
        "--load",
    ) {
        Ok(world) => world,
        Err(message) => {
            eprintln!("island serve: {message}");
            std::process::exit(1);
        }
    };
    if let Some(root) = flag("--save-root") {
        match world.enable_human_store(std::path::Path::new(&root)) {
            Ok(run) => println!("run {run}: human folders in {root}/{run}/humans"),
            Err(e) => {
                eprintln!("island serve: cannot open the human store: {e}");
                std::process::exit(1);
            }
        }
    }
    let auth = ControlAuth::resolve(std::env::var(TOKEN_ENV).ok(), bind.ip());
    println!(
        "Maer-Ken Island (live): http://{bind}/  ({} people, writes: {})",
        world.life.humans.registry.iter().count(),
        auth.describe()
    );
    let sim = serve::island_sim::spawn(world, flag("--snapshot-out").map(PathBuf::from));
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    if let Err(err) = runtime.block_on(serve::island_api::run(sim, auth, bind)) {
        eprintln!("could not serve on {bind}: {err}");
        std::process::exit(1);
    }
}
