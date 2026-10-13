//! Diagnostic: where people stand and what water is around them.
//! `ISLAND_SNAPSHOT=<file> cargo test --release -p mk_engine --test island_inspect_water -- --ignored --nocapture`

use std::path::PathBuf;

use mk_engine::regional::life::IslandLife;
use mk_engine::regional::materials::{cells_around, water_available_kg};
use mk_island::DomainLevel;

/// The island saved at `ISLAND_SNAPSHOT`, against the repository's canon.
fn load_island_snapshot(path: &std::path::Path) -> Result<IslandLife, String> {
    let canon = mk_core::canon::CanonLocked::load(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/island/canon.json"),
    )
    .map_err(|e| e.to_string())?;
    mk_engine::io::island_snapshot::load_island_snapshot(std::sync::Arc::new(canon), path)
        .map_err(|e| e.to_string())
}

#[test]
#[ignore = "diagnostic: needs ISLAND_SNAPSHOT"]
fn where_people_stand_and_what_water_is_around_them() {
    let path = PathBuf::from(std::env::var("ISLAND_SNAPSHOT").expect("set ISLAND_SNAPSHOT"));
    let life = load_island_snapshot(&path).unwrap();
    let life = &life;
    let m = DomainLevel::Medium;
    let (rows, cols) = (life.domain.rows(m), life.domain.cols(m));
    let area = life.domain.cell_area_m2(m);
    let home = life.placed.location;
    let mut dry = 0;
    for h in life.humans.registry.iter() {
        let p = h.position;
        let cell = (
            p.row.clamp(0, rows as i32 - 1) as usize,
            p.col.clamp(0, cols as i32 - 1) as usize,
        );
        let hy = &life.physical.hydrology;
        let soil = hy.soil_water.get(cell.0, cell.1);
        let liquid_in_reach = cells_around(cell, 2, rows, cols)
            .into_iter()
            .any(|c| water_available_kg(hy, c, area) > 0.0);
        let dist = cell.0.abs_diff(home.0).max(cell.1.abs_diff(home.1));
        let biome = life.ecology.biome_grid.get(cell.0, cell.1);
        if !liquid_in_reach && soil.moisture_fraction < 0.2 {
            dry += 1;
        }
        println!(
            "WHO {:<24} {:>5.1}y cell {:?} {:>3} cells from home biome {:?} soil {:.3} storage_mm {:.3} surface {:.5} runoff {:.5} liquid_in_reach {}",
            h.agent_id().chars().take(24).collect::<String>(),
            h.development.age_years,
            cell,
            dist,
            biome,
            soil.moisture_fraction,
            soil.storage_mm,
            hy.surface_water.get(cell.0, cell.1),
            hy.runoff.get(cell.0, cell.1),
            liquid_in_reach
        );
    }
    println!("WHO people with neither liquid water in reach nor damp soil: {dry}");
}

#[test]
#[ignore = "diagnostic: needs ISLAND_SNAPSHOT"]
fn who_fails_to_drink_at_the_next_six_hourly_firing() {
    let path = PathBuf::from(std::env::var("ISLAND_SNAPSHOT").expect("set ISLAND_SNAPSHOT"));
    let mut life = load_island_snapshot(&path).unwrap();
    let m = DomainLevel::Medium;
    let (rows, cols) = (life.domain.rows(m), life.domain.cols(m));
    let area = life.domain.cell_area_m2(m);
    for _ in 0..400 {
        let before = life.shortfalls.water;
        life.advance(60).unwrap();
        let after = life.shortfalls.water;
        if after > before {
            println!(
                "FAIL tick {} sim_time {} shortfalls {} -> {}",
                life.tick, life.sim_time_s, before, after
            );
            let hy = &life.physical.hydrology;
            for h in life.humans.registry.iter() {
                let p = h.position;
                let cell = (
                    p.row.clamp(0, rows as i32 - 1) as usize,
                    p.col.clamp(0, cols as i32 - 1) as usize,
                );
                let soil = hy.soil_water.get(cell.0, cell.1);
                let liquid = cells_around(cell, 2, rows, cols)
                    .into_iter()
                    .any(|c| water_available_kg(hy, c, area) > 0.0);
                if !liquid && soil.moisture_fraction < 0.2 || soil.storage_mm <= 0.0 {
                    println!(
                        "FAIL-WHO {} cell {:?} soil {:.3} storage {:.3} liquid {}",
                        h.agent_id(),
                        cell,
                        soil.moisture_fraction,
                        soil.storage_mm,
                        liquid
                    );
                }
            }
            return;
        }
    }
    println!("FAIL none within 400 steps");
}

#[test]
#[ignore = "diagnostic: needs ISLAND_SNAPSHOT"]
fn the_ledger_holds_a_body_for_exactly_the_living() {
    use mk_core::flux::Reservoir;
    use mk_core::human::HumanStatus;
    let path = PathBuf::from(std::env::var("ISLAND_SNAPSHOT").expect("set ISLAND_SNAPSHOT"));
    let life = load_island_snapshot(&path).unwrap();
    let life = &life;
    let (mut alive, mut dead, mut living_in_ledger, mut dead_in_ledger) = (0, 0, 0, 0);
    for h in life.humans.registry.iter() {
        let registered = life.materials.is_registered(h.agent_id());
        if matches!(h.profile.status, HumanStatus::Alive) {
            alive += 1;
            living_in_ledger += usize::from(registered);
        } else {
            dead += 1;
            dead_in_ledger += usize::from(registered);
        }
    }
    println!(
        "LEDGER alive {alive} (in ledger {living_in_ledger}), dead {dead} (still in ledger {dead_in_ledger}), human carbon {:.1} kg",
        life.materials.stock_of(Reservoir::HumanCarbon)
    );
}
