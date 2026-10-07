//! Memory and per-human step cost of the human runtime at island-scale
//! populations (Phase 5 evidence for the 8-16 GB budget). Slow tier:
//! `cargo test --release -p mk_engine --test human_population_memory -- --ignored slow_ --nocapture`.

use mk_core::grid::{Grid2, GridSpec};
use mk_core::human::BiologicalSex;
use mk_core::rng::RngRegistry;
use mk_engine::humans::{AgentWorldObservation, GridPosition, HumanBeing, HumanSystem};
use mk_engine::resource_economy::ResourceEconomyState;
use std::time::Instant;

const DT_YEARS: f64 = 60.0 / (365.25 * 86_400.0);

/// Peak resident memory of this process so far (kB), from /proc.
fn peak_rss_kb() -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("VmHWM:"))
                .and_then(|l| l.split_whitespace().nth(1)?.parse().ok())
        })
        .unwrap_or(0)
}

#[test]
#[ignore = "slow: population memory and cost scaling"]
fn slow_the_human_runtime_stays_small_and_roughly_linear_to_five_thousand_people() {
    let grid = GridSpec::new(32, 64);
    let elevation = Grid2::new(&grid, 0.0_f64);
    let rng = RngRegistry::new([11u8; 32]);
    let mut per_human_us = Vec::new();
    for &n in &[200usize, 1_000, 5_000] {
        let mut system = HumanSystem::new();
        for i in 0..n {
            let sex = if i % 2 == 0 {
                BiologicalSex::Female
            } else {
                BiologicalSex::Male
            };
            let mut human = HumanBeing::new(format!("adult-{i:05}"), sex);
            human.development.age_years = 25.0;
            // Spread over the grid, as a settled population would be.
            human.set_runtime_position(GridPosition::new((i % 32) as i32, (i / 32 % 64) as i32));
            system.registry.add_human_no_storage(human);
        }
        let mut economy = ResourceEconomyState::new();
        let steps = 4u64;
        let started = Instant::now();
        for tick in 0..steps {
            system.step(
                DT_YEARS,
                tick,
                &rng,
                &grid,
                &mut economy,
                &elevation,
                |_, _| AgentWorldObservation {
                    caloric_access: 0.9,
                    hydration_access: 0.9,
                    shelter_quality: 0.9,
                    ..AgentWorldObservation::default()
                },
                None,
            );
        }
        let us = started.elapsed().as_secs_f64() * 1e6 / (n as f64 * steps as f64);
        println!(
            "{n:>5} humans: {us:.0} us per human-step, peak RSS so far {} MB",
            peak_rss_kb() / 1024
        );
        per_human_us.push(us);
    }
    // Roughly linear: 25x the people costs at most 4x more per person.
    assert!(
        per_human_us[2] <= 4.0 * per_human_us[0].max(1.0),
        "per-human cost {per_human_us:?} us grew superlinearly"
    );
    // Five thousand people fit comfortably in an 8 GB machine's budget
    // (the whole process, including test harness, under 2 GB).
    assert!(
        peak_rss_kb() < 2 * 1024 * 1024,
        "peak RSS {} kB",
        peak_rss_kb()
    );
}
