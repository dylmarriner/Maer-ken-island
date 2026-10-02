//! A crowded cell must not make the human runtime superlinear: 200 adults in
//! one cell may cost at most 3x per human what 200 adults spread across the
//! grid cost. Timing is relative (a ratio on the same machine in the same
//! run), never an absolute duration.

use mk_core::grid::{Grid2, GridSpec};
use mk_core::human::BiologicalSex;
use mk_core::rng::RngRegistry;
use mk_engine::humans::{AgentWorldObservation, GridPosition, HumanBeing, HumanSystem};
use mk_engine::resource_economy::ResourceEconomyState;
use std::time::{Duration, Instant};

const HUMANS: usize = 200;
const STEPS: u64 = 6;
const REPEATS: usize = 3;
const DT_YEARS: f64 = 60.0 / (365.25 * 86_400.0);

fn population(position_of: impl Fn(usize) -> GridPosition) -> HumanSystem {
    let mut system = HumanSystem::new();
    for n in 0..HUMANS {
        let sex = if n % 2 == 0 {
            BiologicalSex::Female
        } else {
            BiologicalSex::Male
        };
        let mut human = HumanBeing::new(format!("adult-{n:04}"), sex);
        human.development.age_years = 25.0;
        human.set_runtime_position(position_of(n));
        system.registry.add_human_no_storage(human);
    }
    system
}

/// Wall time of `STEPS` human steps, per human per step.
fn per_human_step(position_of: impl Fn(usize) -> GridPosition) -> Duration {
    let grid = GridSpec::new(32, 64);
    let elevation = Grid2::new(&grid, 0.0_f64);
    let rng = RngRegistry::new([11u8; 32]);
    let mut samples: Vec<Duration> = (0..REPEATS)
        .map(|_| {
            let mut system = population(&position_of);
            let mut economy = ResourceEconomyState::new();
            let started = Instant::now();
            for tick in 0..STEPS {
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
            started.elapsed() / (HUMANS as u32 * STEPS as u32)
        })
        .collect();
    samples.sort();
    samples[REPEATS / 2]
}

#[test]
#[ignore = "slow: wall-clock scaling comparison; run with: cargo test --workspace --release -- --ignored slow_"]
fn slow_a_crowded_cell_costs_at_most_three_times_a_spread_population() {
    let spread =
        per_human_step(|n| GridPosition::new(((n / 20) * 3) as i32, ((n % 20) * 3) as i32));
    let crowded = per_human_step(|_| GridPosition::new(10, 10));

    let ratio = crowded.as_secs_f64() / spread.as_secs_f64();
    eprintln!("spread {spread:?}/human/step, crowded {crowded:?}/human/step, ratio {ratio:.2}");
    assert!(
        ratio <= 3.0,
        "crowding makes each human {ratio:.2}x as expensive (spread {spread:?}, crowded {crowded:?})"
    );
}
