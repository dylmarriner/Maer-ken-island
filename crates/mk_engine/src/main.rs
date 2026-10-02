use tracing::{error, info, warn};

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    info!("═══════════════════════════════════════════════════════════════");
    info!("         MARR'KENA UNIVERSE SIMULATION ENGINE");
    info!("═══════════════════════════════════════════════════════════════");

    std::fs::create_dir_all("data").unwrap_or_else(|e| {
        warn!("Could not create data directory: {}", e);
    });

    std::fs::create_dir_all("snapshots").unwrap_or_else(|e| {
        warn!("Could not create snapshots directory: {}", e);
    });

    let event_db = match mk_engine::io::EventDatabase::new("data/universe_events.db") {
        Ok(db) => {
            info!("Event database initialized: data/universe_events.db");
            db
        }
        Err(e) => {
            error!("Could not initialize event database: {}", e);
            std::process::exit(1);
        }
    };

    let boot_config = serde_json::json!({
        "version": "0.1.0",
        "engine": "mk_engine",
        "start_time": chrono::Utc::now().to_rfc3339(),
    });
    if let Err(e) = event_db.log_world_boot(0, &boot_config.to_string()) {
        warn!("Could not log world boot event: {}", e);
    }

    if let Err(e) = mk_engine::io::init_event_database(event_db) {
        warn!("Global event database already initialized: {}", e);
    }

    if let Some(db) = mk_engine::io::get_event_database() {
        mk_engine::io::print_universe_summary(db);
    }

    let canon = std::sync::Arc::new(mk_core::canon::CanonLocked::default());
    let mut world = mk_engine::world_integration::WorldState::new(canon, [0; 32]);
    match world.enable_persistent_humans("data/humans") {
        Ok(seed_errors) => {
            for e in seed_errors {
                error!(
                    "Founder folder could not be written (founder still exists): {}",
                    e
                );
            }
        }
        Err(e) => {
            error!("Could not initialize persistent human profiles: {}", e);
            std::process::exit(1);
        }
    }

    info!("Initial State: Tick {}", world.tick);

    let mut tick_counter = 0u64;
    let mut last_snapshot_tick = 0u64;
    let snapshot_interval = 1000u64;

    loop {
        match world.step_world(world.canon.step_seconds()) {
            Ok(_) => {
                tick_counter += 1;

                if world.tick - last_snapshot_tick >= snapshot_interval {
                    let snapshot_path = format!("snapshots/universe_tick_{}.bin", world.tick);
                    match mk_engine::io::save_snapshot(&world, std::path::Path::new(&snapshot_path))
                    {
                        Ok(_) => {
                            info!("Snapshot saved: {}", snapshot_path);
                            last_snapshot_tick = world.tick;
                        }
                        Err(e) => {
                            warn!("Failed to save snapshot: {}", e);
                        }
                    }
                }

                if tick_counter.is_multiple_of(100) {
                    info!(
                        tick = world.tick,
                        energy_closure = world.audit_trail.energy_closure,
                        "Tick success"
                    );
                }
            }
            Err(e) => {
                error!(tick = world.tick, error = ?e, "Tick failed");
                std::thread::sleep(std::time::Duration::from_secs(1));
                continue;
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
