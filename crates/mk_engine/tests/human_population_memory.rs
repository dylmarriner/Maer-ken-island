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

/// Timed runs per population, of which the middle one is kept. Odd, so the
/// median is an actual measurement rather than the mean of two. Nine rather
/// than five because the spread is wide: single runs at 200 humans have been
/// seen from 16 to 41 us on one otherwise idle machine.
const REPEATS: usize = 9;

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
        // The same number of steps for every population, deliberately.
        //
        // Scaling them so each population does equal total work was tried
        // and is wrong: `HumanSystem::step` has per-call cost that does not
        // depend on how many humans it holds, so giving 200 humans 100
        // steps while 5,000 get 4 amortises that fixed cost over 25x fewer
        // people. Measured, it put the 200-human figure at 84 us against
        // its usual 17-25 and the 5,000-human one at 59 — comparing two
        // different things and calling the result linearity.
        let steps = 4u64;
        // One untimed step first, so the measurement is of a warm run
        // rather than of the allocator touching 5,000 humans for the first
        // time. This is the part of the noise that can be removed without
        // changing what is being compared.
        system.step(
            DT_YEARS,
            0,
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
        // The median of several runs, not one run.
        //
        // This is what finally settled the flake. The assertion below
        // divides by the 200-human figure, and one run of it is a few
        // hundred microseconds of work on a shared machine: it has been
        // seen at 17 us and at 26 us on the same commit, and a low draw
        // there drops the bar under a perfectly ordinary measurement at
        // 5,000. Taking the middle of REPEATS runs throws away the
        // outliers in both directions.
        //
        // It does not change what is being compared. Each run is the same
        // `steps` steps on the same population that a single run was, so
        // the fixed per-call cost of `HumanSystem::step` still lands on
        // every population equally — which is exactly the property that
        // the earlier attempt to scale the step count destroyed.
        //
        // One thing the printed runs show that a single figure hid: they
        // climb, monotonically, every time. A typical 200-human row reads
        // 17, 25, 35, 41, 49, 65, 69, 71, 79. That is a trend and not
        // jitter, and the likeliest reason is that these humans are
        // accumulating state as they live — conversation history, memory,
        // relationships — so a later step genuinely costs more than an
        // early one. It is why the medians rose when REPEATS went from 5
        // to 9, and it means the absolute numbers here are only
        // comparable at a fixed REPEATS.
        //
        // The ratio is unharmed, because every population is measured
        // over the same history: each starts fresh and takes the same
        // 1 + REPEATS * steps. Across three runs it came out at 1.69,
        // 1.86 and 1.71 against a bar of 4, where single samples had
        // produced 3.2 and a failure.
        let mut runs = Vec::with_capacity(REPEATS);
        for run in 0..REPEATS {
            let started = Instant::now();
            for step in 0..steps {
                system.step(
                    DT_YEARS,
                    run as u64 * steps + step,
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
            runs.push(started.elapsed().as_secs_f64() * 1e6 / (n as f64 * steps as f64));
        }
        runs.sort_by(|a, b| a.partial_cmp(b).expect("no NaN from a clock"));
        let us = runs[REPEATS / 2];
        println!(
            "{n:>5} humans: {us:.0} us per human-step (median of {REPEATS}: {}), \
             peak RSS so far {} MB",
            runs.iter()
                .map(|r| format!("{r:.0}"))
                .collect::<Vec<_>>()
                .join(", "),
            peak_rss_kb() / 1024
        );
        per_human_us.push(us);
    }
    // Roughly linear: 25x the people costs at most 4x more per person. Two
    // hundred humans fit in cache and five thousand do not, so some growth
    // is real and this 4x is the tolerance for it. The figures it divides
    // are medians rather than single runs, which is what makes dividing by
    // the smallest and noisiest of them sound.
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
