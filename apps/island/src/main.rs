//! `island` — Maer-Ken Island.
//!
//! `island serve` serves the dashboard: an overview of the population, the
//! roster with everything stored about each person, and the Human Creator.
//! `island run` advances the simulated island without a browser, and
//! `island inspect` describes a saved one.

use island::run as headless;
use island::serve;
use island::serve::auth::{ControlAuth, ReadAuth, READ_TOKEN_ENV, TOKEN_ENV};
use island::serve::server::ServeConfig;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

const USAGE: &str = "\
island serve   [--data-dir ./island-data] [--seed <64 hex>] [--bind 127.0.0.1:8080]
               [--scenario <path> | --snapshot <path>] [--speed real|max|<n>] [--log <path>]
               [--allow-origin <url>]...
island run     (--scenario <path> | --snapshot <path>) [--steps <n>] [--dt <seconds>]
               [--save <path>] [--save-root <dir>]
island replay  --scenario <path> --log <path> [--until <tick>]
island inspect --snapshot <path> [--scenario <path>]

serve    the dashboard: the overview at /, the roster at /people, the island
         itself at /island and the Human Creator at /creator, with the JSON
         behind them under /api.
  --data-dir DIR   where people are stored (default ./island-data)
  --seed HEX       64 hex characters; the same seed and the same creations
                   rebuild the same people (default all zeroes)
  --bind ADDR      address and port to listen on (default 127.0.0.1:8080)
  --scenario PATH  also simulate the island, served at /api/world
  --snapshot PATH  simulate it from a saved island instead
  --speed SPEED    real (one simulated second per real second, the default),
                   max, or a multiplier like 60. Speed changes how often a
                   step runs, never its size, so the island is the same at
                   any speed.
  --log PATH       record every command this dashboard applies, so the
                   session can be replayed with `island replay`
  --snapshot-dir D where a snapshot asked for from the dashboard is written.
                   Without it that request is refused rather than silently
                   doing nothing. Writing one takes about half a minute on
                   the full island and the island does not step meanwhile
  --import-0b DIR  carry the people stored in a Phase-0b data directory into
                   the world, once, as creations at the first step
  --allow-origin U a web frontend served from this address may call this
                   backend. Give it exactly as a browser sends it --
                   scheme, host and port, no path: https://island.example.
                   Repeat it for more than one. Without any, a browser may
                   only call this server from pages it served itself, which
                   is what you want when they are the same machine.

run      the simulated island, headless, printing its canonical state digest.
         Two runs of one scenario and seed print the same digest.
  --scenario PATH  start a new island from a scenario
  --snapshot PATH  carry on from a saved one instead
  --steps N        how many steps to advance (default 1440, a simulated day
                   at the default --dt)
  --dt SECONDS     simulated seconds per step (default 60, the human step)
  --save PATH      write the island to a file when the run ends
  --save-root DIR  keep a folder for every human under DIR/<run id>/humans
  --log PATH       write the commands this run applied, for `island replay`

replay   a recorded run, printing the digest it reaches. It should be the
         digest the original run printed.
  --scenario PATH  the scenario the log was recorded against
  --log PATH       the log to replay
  --until TICK     stop at this tick (default: the log's last command)

inspect  what is in a saved island, without running it.
  --snapshot PATH  the file to describe
  --scenario PATH  which canon it was written under (default the island
                   fixture canon)

Two tokens, answering two questions. ISLAND_CONTROL_TOKEN decides who may
change the island: set it and every write needs `Authorization: Bearer
<token>`; without it only a loopback bind accepts writes at all.
ISLAND_READ_TOKEN decides who may look. Reads are open without it, which is
right on a loopback bind and a decision anywhere else -- a backend bound to
0.0.0.0 with neither token is readable by anyone who can reach the port, and
this says so on startup rather than leaving it to be discovered.

Set ISLAND_ACCESS_LOG=off to silence the request log.";

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

/// The value after `name`, if it was given.
fn flag(args: &[String], name: &str) -> Option<String> {
    args.windows(2).find(|w| w[0] == name).map(|w| w[1].clone())
}

/// A numeric flag, or the default when it was not given.
fn number(args: &[String], name: &str, default: u64) -> u64 {
    match flag(args, name) {
        None => default,
        Some(value) => value.parse().unwrap_or_else(|_| {
            eprintln!("{name} {value:?} is not a number.\n");
            usage()
        }),
    }
}

/// Anything the headless commands fail on is a message, not a panic: these
/// are run from scripts, where an exit status and one line matter more than
/// a backtrace.
fn finish(result: Result<(), String>) {
    if let Err(message) = result {
        eprintln!("{message}");
        std::process::exit(1);
    }
}

fn headless_run(args: &[String]) {
    if args.iter().any(|a| matches!(a.as_str(), "--help" | "-h")) {
        help();
    }
    finish(headless::run(headless::RunArgs {
        scenario: flag(args, "--scenario").map(PathBuf::from),
        snapshot: flag(args, "--snapshot").map(PathBuf::from),
        steps: number(args, "--steps", 1_440),
        dt_seconds: number(args, "--dt", 60),
        save: flag(args, "--save").map(PathBuf::from),
        save_root: flag(args, "--save-root").map(PathBuf::from),
        log: flag(args, "--log").map(PathBuf::from),
    }));
}

fn headless_replay(args: &[String]) {
    if args.iter().any(|a| matches!(a.as_str(), "--help" | "-h")) {
        help();
    }
    let (Some(scenario), Some(log)) = (flag(args, "--scenario"), flag(args, "--log")) else {
        eprintln!("replay needs --scenario and --log.\n");
        usage()
    };
    finish(headless::replay(
        &PathBuf::from(scenario),
        &PathBuf::from(log),
        flag(args, "--until").map(|n| {
            n.parse().unwrap_or_else(|_| {
                eprintln!("--until {n:?} is not a number.\n");
                usage()
            })
        }),
    ));
}

fn headless_inspect(args: &[String]) {
    if args.iter().any(|a| matches!(a.as_str(), "--help" | "-h")) {
        help();
    }
    let Some(snapshot) = flag(args, "--snapshot") else {
        eprintln!("inspect needs --snapshot.\n");
        usage()
    };
    finish(headless::inspect(
        &PathBuf::from(snapshot),
        flag(args, "--scenario").map(PathBuf::from).as_deref(),
    ));
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some((command, rest)) = args.split_first() else {
        usage()
    };
    if matches!(command.as_str(), "help" | "--help" | "-h") {
        help();
    }
    match command.as_str() {
        "serve" => {}
        "run" => return headless_run(rest),
        "replay" => return headless_replay(rest),
        "inspect" => return headless_inspect(rest),
        other => {
            eprintln!("island has `serve`, `run` and `inspect`, not `{other}`.\n");
            usage();
        }
    }
    let mut data_dir = PathBuf::from("island-data");
    let mut seed = island_humans::DEFAULT_SEED;
    let mut bind: std::net::SocketAddr = "127.0.0.1:8080".parse().expect("valid default");
    let mut allowed_origins: Vec<String> = Vec::new();
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
            "--allow-origin" => {
                let origin = value.trim_end_matches('/').to_string();
                if !origin.starts_with("http://") && !origin.starts_with("https://") {
                    eprintln!(
                        "--allow-origin {value:?} is not an origin. A browser sends scheme, host \
                         and port and nothing else: https://island.example or \
                         http://192.168.1.20:3000.\n"
                    );
                    usage()
                }
                allowed_origins.push(origin);
            }
            // Read again below, where the world is opened; named here so
            // they are accepted rather than falling through to the usage
            // message as unknown flags.
            "--scenario" | "--snapshot" | "--speed" | "--log" | "--import-0b"
            | "--snapshot-dir" => {}
            _ => usage(),
        }
    }

    // The world, when one was asked for. It starts before the listener so a
    // failure to bootstrap is a clean exit rather than a server that serves
    // an island it never managed to build.
    let world = match (flag(rest, "--scenario"), flag(rest, "--snapshot")) {
        (Some(_), Some(_)) => {
            eprintln!("--scenario starts a new island and --snapshot carries one on; pick one.\n");
            usage()
        }
        (None, None) => None,
        (scenario, snapshot) => {
            let speed = match flag(rest, "--speed") {
                None => serve::sim::SimSpeed::RealTime,
                Some(text) => text.parse().unwrap_or_else(|e: String| {
                    eprintln!("{e}\n");
                    usage()
                }),
            };
            let life = headless::open_world(
                scenario.map(PathBuf::from).as_deref(),
                snapshot.map(PathBuf::from).as_deref(),
            )
            .unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            println!(
                "Simulating the island at {}. Its state is at /api/world.",
                speed.describe()
            );
            let log = flag(rest, "--log").map(PathBuf::from);
            if let Some(path) = &log {
                println!("Recording commands to {}.", path.display());
            }
            let snapshots = flag(rest, "--snapshot-dir").map(PathBuf::from);
            if let Some(dir) = &snapshots {
                println!(
                    "Snapshots on request go to {}. Writing one takes about half a minute on \
                     the full island, and the clock stops while it does.",
                    dir.display()
                );
            }
            let world = serve::sim::spawn_with(life, speed, log, snapshots);
            // Carried in as ordinary commands, so they are recorded in the
            // replay log and reproduced by a replay exactly like anybody
            // created later. The estate's General zone is where they land:
            // a stored person has no island location, because there was no
            // island when they were made.
            if let Some(dir) = flag(rest, "--import-0b") {
                let dir = PathBuf::from(dir);
                let commands = headless::commands_from_phase_0b(
                    &dir,
                    mk_engine::regional::estate_layout::SpaceId(0),
                )
                .unwrap_or_else(|e| {
                    eprintln!("{e}");
                    std::process::exit(1);
                });
                println!(
                    "Carrying {} stored {} from {} into the world.",
                    commands.len(),
                    if commands.len() == 1 {
                        "person"
                    } else {
                        "people"
                    },
                    dir.display()
                );
                for command in commands {
                    world.send(command);
                }
            }
            Some(world)
        }
    };

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
    let reads = ReadAuth::resolve(std::env::var(READ_TOKEN_ENV).ok());
    println!("Maer-Ken Island dashboard is up.");
    println!("  Overview        http://{bind}/");
    println!("  People          http://{bind}/people");
    // Listed only when there is a world: without `--scenario` the page
    // exists but has nothing to show, and sending an operator to it would
    // be sending them to a banner saying so.
    if world.is_some() {
        println!("  The island      http://{bind}/island");
    }
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
    println!("Reading: {}.", reads.describe());
    if !allowed_origins.is_empty() {
        println!(
            "A web frontend served from {} may call this backend.",
            allowed_origins.join(", ")
        );
    }
    // Said once, loudly, on the one configuration where it matters. A
    // backend reachable from another machine with neither token is open to
    // everyone who can reach the port, and that should be a choice rather
    // than something discovered later.
    if !bind.ip().is_loopback() && !reads.needs_token() {
        println!();
        println!(
            "  This backend is bound to {bind}, so anyone who can reach that port can read \
             everything on this island: every person's full record included. Set \
             {READ_TOKEN_ENV} before exposing it to a network you do not control."
        );
        println!();
    }
    println!("Press Ctrl-C to stop. Set ISLAND_ACCESS_LOG=off to silence the request log.");

    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    if let Err(err) = runtime.block_on(serve::server::run_with_config(
        Arc::new(Mutex::new(population)),
        bind,
        world,
        ServeConfig {
            control: auth,
            reads,
            allowed_origins,
        },
    )) {
        eprintln!("could not serve on {bind}: {err}");
        std::process::exit(1);
    }
}
