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
               [--allow-origin <url>]... [--backend <url>] [--allow-backend <url>]...
island serve   --frontend-only --backend <url> [--bind 0.0.0.0:3000]
island export  --to <dir> [--backend <url>]
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
  --backend URL    the pages this server sends read *that* island instead
                   of this one. This is the other half of --allow-origin:
                   run one process with --frontend-only --backend
                   https://island.example on the machine people open, and
                   the island itself with --allow-origin pointing back.
  --allow-backend U another island the pages may be pointed at by hand,
                   through the server control at the foot of every page.
                   The backend given above is always allowed; this is for
                   offering a choice. A browser refuses any island the page
                   does not name, which is what keeps a script that gets
                   onto the page from posting the island somewhere else.
  --frontend-only  serve the pages and nothing else: no API, no island, no
                   stored population. Needs --backend. This is a web server
                   for the dashboard, and it can sit anywhere.

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

export   write the dashboard out as plain files, for any web server. The
         pages, the stylesheet, the scripts and a config.js naming the
         island they read. Nothing in it needs this binary afterwards.
  --to DIR         where to write them. Created if it is not there; files
                   already in it with these names are overwritten
  --backend URL    the island those pages read. Without it they read
                   whatever server ends up hosting them, which is right
                   only if that server is also the island

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

/// Whether an island was asked for, so `--frontend-only` can refuse to be
/// both halves at once.
fn world_asked(args: &[String]) -> bool {
    flag(args, "--scenario").is_some() || flag(args, "--snapshot").is_some()
}

/// An origin as a browser sends it: scheme, host and port, nothing else.
///
/// Checked rather than taken as given, because every one of these ends up
/// in a header a browser compares byte for byte, and a trailing slash or a
/// path on the end of one is a rule that silently never matches.
fn origin(value: &str, flag: &str) -> String {
    let tidy = value.trim_end_matches('/');
    let is_origin =
        (tidy.starts_with("http://") || tidy.starts_with("https://")) && !tidy[8..].contains('/');
    if !is_origin {
        eprintln!(
            "{flag} {value:?} is not an origin. A browser sends scheme, host and port and \
             nothing else: https://island.example or http://192.168.1.20:3000.\n"
        );
        usage()
    }
    tidy.to_string()
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

/// Write the dashboard out as files, for a web server that is not this one.
///
/// `--frontend-only` is the same frontend served by this binary; this is
/// the same frontend as bytes on disk, for the many places that already
/// have a way to host static files and no reason to run a Rust process to
/// do it. Both put the island's address in `config.js` rather than in the
/// pages, so the pages are identical either way.
fn export_frontend(args: &[String]) {
    if args.iter().any(|a| matches!(a.as_str(), "--help" | "-h")) {
        help();
    }
    let Some(to) = flag(args, "--to") else {
        eprintln!("export needs --to, a directory to write the dashboard into.\n");
        usage()
    };
    let backend = flag(args, "--backend").map(|value| origin(&value, "--backend"));
    let to = PathBuf::from(to);
    match serve::pages::export(&to, backend.clone()) {
        Err(err) => {
            eprintln!("could not write the dashboard to {}: {err}", to.display());
            std::process::exit(1);
        }
        Ok(written) => {
            println!(
                "Wrote {written} files to {}. Serve that directory from anything.",
                to.display()
            );
            match &backend {
                Some(backend) => println!(
                    "Those pages read {backend}, which has to have been started with \
                     --allow-origin naming the address people will open this from, or a browser \
                     will refuse every request they make."
                ),
                None => println!(
                    "No --backend, so those pages read whichever server hosts them. That is \
                     right only if that server is also the island."
                ),
            }
        }
    }
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
        "export" => return export_frontend(rest),
        "replay" => return headless_replay(rest),
        "inspect" => return headless_inspect(rest),
        other => {
            eprintln!(
                "island has `serve`, `run`, `replay`, `inspect` and `export`, not `{other}`.\n"
            );
            usage();
        }
    }
    let mut data_dir = PathBuf::from("island-data");
    let mut seed = island_humans::DEFAULT_SEED;
    let mut bind: std::net::SocketAddr = "127.0.0.1:8080".parse().expect("valid default");
    let mut allowed_origins: Vec<String> = Vec::new();
    let mut backend: Option<String> = None;
    let mut allow_backends: Vec<String> = Vec::new();
    let frontend_only = rest.iter().any(|a| a == "--frontend-only");
    let mut iter = rest.iter();
    while let Some(flag) = iter.next() {
        if matches!(flag.as_str(), "--help" | "-h") {
            help();
        }
        if flag == "--frontend-only" {
            continue;
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
            "--allow-origin" => allowed_origins.push(origin(value, "--allow-origin")),
            "--backend" => backend = Some(origin(value, "--backend")),
            "--allow-backend" => allow_backends.push(origin(value, "--allow-backend")),
            // Read again below, where the world is opened; named here so
            // they are accepted rather than falling through to the usage
            // message as unknown flags.
            "--scenario" | "--snapshot" | "--speed" | "--log" | "--import-0b"
            | "--snapshot-dir" => {}
            _ => usage(),
        }
    }

    let connect_origins = {
        let mut origins = allow_backends.clone();
        if let Some(backend) = &backend {
            if !origins.contains(backend) {
                origins.push(backend.clone());
            }
        }
        origins
    };

    // A frontend and nothing else: the pages, pointed at an island on
    // another machine. It opens no data directory and bootstraps no world,
    // because it has neither and should not pretend to.
    if frontend_only {
        let Some(backend) = backend.clone() else {
            eprintln!(
                "--frontend-only serves the pages for an island somewhere else, so it needs \
                 --backend to say which one.\n"
            );
            usage()
        };
        if world_asked(rest) {
            eprintln!(
                "--frontend-only serves pages and nothing else, so it cannot also run an \
                 island. Run the island in its own process and point this at it.\n"
            );
            usage()
        }
        println!("Maer-Ken Island frontend is up, serving the dashboard for {backend}.");
        println!("  Overview        http://{bind}/");
        println!("  People          http://{bind}/people");
        println!("  The island      http://{bind}/island");
        println!("  Create a human  http://{bind}/creator");
        println!(
            "No island and no stored people here: every request these pages make goes to \
             {backend}, which has to have been started with --allow-origin http://{bind} (or \
             whatever address people actually open) or a browser will refuse them."
        );
        println!("Press Ctrl-C to stop.");
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
        if let Err(err) = runtime.block_on(serve::server::run_frontend(
            bind,
            ServeConfig {
                control: ControlAuth::Disabled,
                reads: ReadAuth::Open,
                allowed_origins,
                backend: Some(backend),
                connect_origins,
            },
        )) {
            eprintln!("could not serve on {bind}: {err}");
            std::process::exit(1);
        }
        return;
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
            backend,
            connect_origins,
        },
    )) {
        eprintln!("could not serve on {bind}: {err}");
        std::process::exit(1);
    }
}
