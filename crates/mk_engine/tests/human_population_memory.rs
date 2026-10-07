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

/// What `formal_predictive_processing::step` costs, as a share of a whole
/// human step.
///
/// `docs/canon/HUMAN_SYSTEM_STATUS.md` records that this subsystem runs for
/// every human every tick and that nothing outside its own module reads the
/// action it selects. Whether to wire that action into behaviour or stop
/// running it is a decision about the model, not about this test — but it is
/// a decision worth making on a number, and this is the number. Slow tier:
/// `cargo test --release -p mk_engine --test human_population_memory -- --ignored slow_ --nocapture`.
#[test]
#[ignore = "slow: predictive-processing share of a human step"]
fn slow_formal_predictive_processing_costs_this_share_of_a_human_step() {
    const N: usize = 1_000;
    const STEPS: u64 = 4;

    let grid = GridSpec::new(32, 64);
    let elevation = Grid2::new(&grid, 0.0_f64);
    let rng = RngRegistry::new([11u8; 32]);
    let observation = || AgentWorldObservation {
        caloric_access: 0.9,
        hydration_access: 0.9,
        shelter_quality: 0.9,
        ..AgentWorldObservation::default()
    };

    let mut system = HumanSystem::new();
    for i in 0..N {
        let sex = if i % 2 == 0 {
            BiologicalSex::Female
        } else {
            BiologicalSex::Male
        };
        let mut human = HumanBeing::new(format!("adult-{i:05}"), sex);
        human.development.age_years = 25.0;
        human.set_runtime_position(GridPosition::new((i % 32) as i32, (i / 32 % 64) as i32));
        system.registry.add_human_no_storage(human);
    }
    let mut economy = ResourceEconomyState::new();

    // One warm-up step, so neither measurement pays first-touch costs.
    system.step(
        DT_YEARS,
        0,
        &rng,
        &grid,
        &mut economy,
        &elevation,
        |_, _| observation(),
        None,
    );

    let whole = Instant::now();
    for tick in 1..=STEPS {
        system.step(
            DT_YEARS,
            tick,
            &rng,
            &grid,
            &mut economy,
            &elevation,
            |_, _| observation(),
            None,
        );
    }
    let whole_us = whole.elapsed().as_secs_f64() * 1e6;

    // The same subsystem, the same number of times, on the same humans and
    // the same inputs the lifecycle hands it.
    let humans: Vec<_> = system.registry.iter().cloned().collect();
    let observation = observation();
    let part = Instant::now();
    // Summed so the optimiser cannot drop the calls it is here to time.
    let mut sink = 0.0_f64;
    for tick in 1..=STEPS {
        for human in &humans {
            let after = human.formal_predictive_processing.step(
                human.profile.human_id,
                tick,
                &observation,
                &human.needs,
                &human.core_systems,
                &rng,
                DT_YEARS,
            );
            sink += after.subjective_confidence;
        }
    }
    let part_us = part.elapsed().as_secs_f64() * 1e6;
    assert!(sink.is_finite(), "the timed calls really ran");

    let share = part_us / whole_us;
    println!(
        "{N} humans x {STEPS} steps: whole step {:.0} us/human-step, predictive processing {:.1} us/human-step, {:.0}% of the step",
        whole_us / (N as f64 * STEPS as f64),
        part_us / (N as f64 * STEPS as f64),
        share * 100.0
    );
    assert!(
        share > 0.0 && share < 1.0,
        "a part of the step cannot cost {share} of it"
    );
}
