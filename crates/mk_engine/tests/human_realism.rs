//! Phase 0c Task 4: the human runtime compared with real-world data
//! (`fixtures/reference/humans/`). Each test states its tolerance and the
//! reference item it is checked against.

use mk_core::human::{BiologicalSex, HumanId, HumanProfile, HumanSchema};
use mk_core::rng::RngRegistry;
use mk_engine::humans::circadian::{CircadianClock, Light};
use mk_engine::humans::lifecycle::{
    step_lifecycle, GOMPERTZ_AGING_RATE_PER_YEAR, GOMPERTZ_BASELINE_HAZARD_PER_YEAR,
};
use mk_engine::humans::needs::{EffortFocus, NeedsSnapshot};
use mk_engine::humans::rates::HOUR_YEARS;
use mk_engine::humans::reproduction::GESTATION_YEARS;
use mk_engine::humans::AgentWorldObservation;
use mk_engine::humans::HumanBeing;
use mk_engine::validation::{default_reference_dir, ReferenceDomain, ReferenceLibrary};

fn reference() -> ReferenceLibrary {
    ReferenceLibrary::load(&default_reference_dir()).expect("reference packs load")
}

fn range(key: &str) -> (f64, f64) {
    reference()
        .item(ReferenceDomain::Humans, key)
        .and_then(|i| i.range())
        .unwrap_or_else(|| panic!("reference range {key}"))
}

fn adult_needs() -> NeedsSnapshot {
    let schema = HumanSchema::canonical_minimal("realism");
    NeedsSnapshot::from_profile(&HumanProfile::from_canonical_schema(
        HumanId::new(1),
        schema,
    ))
}

fn observation(caloric: f64, water: f64, temperature_c: f64) -> AgentWorldObservation {
    AgentWorldObservation {
        caloric_access: caloric,
        hydration_access: water,
        shelter_quality: 1.0,
        ambient_temperature_c: temperature_c,
        ..AgentWorldObservation::default()
    }
}

/// Days until `dead` holds, stepping hourly, or `None` within `max_days`.
fn days_until(
    mut needs: NeedsSnapshot,
    obs: &AgentWorldObservation,
    max_days: f64,
    dead: impl Fn(&NeedsSnapshot) -> bool,
) -> Option<f64> {
    let steps = (max_days * 24.0) as usize;
    for hour in 1..=steps {
        needs = needs.step(obs, 1.0, EffortFocus::none(), HOUR_YEARS);
        if dead(&needs) {
            return Some(hour as f64 / 24.0);
        }
    }
    None
}

#[test]
fn without_water_a_resting_adult_dies_in_days() {
    // Reference `survival_without_water`: 2-10 days, ~3-5 at rest in a
    // temperate climate. Tolerance: inside 3-6 days at 20 °C.
    let (lo, hi) = range("survival_without_water");
    let days = days_until(adult_needs(), &observation(1.0, 0.0, 20.0), 30.0, |n| {
        n.hydration <= 0.0
    })
    .expect("dies of dehydration");
    assert!(days >= lo && days <= hi, "{days} days");
    assert!((3.0..=6.0).contains(&days), "{days} days");
}

#[test]
fn heat_shortens_survival_without_water() {
    // Sweating at 40 °C more than halves the time (Adolph 1947).
    let temperate = days_until(adult_needs(), &observation(1.0, 0.0, 20.0), 30.0, |n| {
        n.hydration <= 0.0
    })
    .unwrap();
    let hot = days_until(adult_needs(), &observation(1.0, 0.0, 40.0), 30.0, |n| {
        n.hydration <= 0.0
    })
    .unwrap();
    assert!(hot < 0.5 * temperate, "hot {hot} vs temperate {temperate}");
    assert!(hot >= 1.0, "{hot} days");
}

#[test]
fn without_food_a_watered_adult_starves_in_weeks() {
    // Reference `survival_without_food`: 45-75 days. Tolerance: the range.
    let (lo, hi) = range("survival_without_food");
    let days = days_until(adult_needs(), &observation(0.0, 1.0, 20.0), 200.0, |n| {
        n.is_lethal()
    })
    .expect("starves");
    assert!(days >= lo && days <= hi, "{days} days");
}

#[test]
fn fasting_makes_a_human_hungry_within_a_day_but_not_dead() {
    let mut needs = adult_needs();
    let obs = observation(0.0, 1.0, 20.0);
    for _ in 0..24 {
        needs = needs.step(&obs, 1.0, EffortFocus::none(), HOUR_YEARS);
    }
    assert!(needs.hunger > 0.8, "hunger {}", needs.hunger);
    assert!(!needs.is_lethal());
    assert!(
        needs.energy_reserve > 0.95,
        "reserve {}",
        needs.energy_reserve
    );
}

#[test]
fn a_fed_and_watered_adult_lives_on_and_refeeding_restores_stores() {
    let mut needs = adult_needs();
    let fed = observation(1.0, 1.0, 20.0);
    for _ in 0..(365 * 24) {
        needs = needs.step(&fed, 1.0, EffortFocus::none(), HOUR_YEARS);
    }
    assert!(!needs.is_lethal());
    assert!(needs.energy_reserve > 0.99);

    // Three weeks of fasting draws the stores down; refeeding rebuilds them.
    let fasting = observation(0.0, 1.0, 20.0);
    for _ in 0..(21 * 24) {
        needs = needs.step(&fasting, 1.0, EffortFocus::none(), HOUR_YEARS);
    }
    let depleted = needs.energy_reserve;
    assert!(depleted < 0.8, "{depleted}");
    for _ in 0..(60 * 24) {
        needs = needs.step(&fed, 1.0, EffortFocus::none(), HOUR_YEARS);
    }
    assert!(needs.energy_reserve > depleted);
}

#[test]
fn step_length_does_not_change_survival() {
    // One-day steps and hourly steps reach the same state after 3 days.
    let obs = observation(0.3, 0.3, 20.0);
    let mut hourly = adult_needs();
    for _ in 0..72 {
        hourly = hourly.step(&obs, 1.0, EffortFocus::none(), HOUR_YEARS);
    }
    let mut daily = adult_needs();
    for _ in 0..3 {
        daily = daily.step(&obs, 1.0, EffortFocus::none(), 24.0 * HOUR_YEARS);
    }
    assert!((hourly.hydration - daily.hydration).abs() < 0.02);
    assert!((hourly.energy_reserve - daily.energy_reserve).abs() < 0.01);
}

// --- Circadian clock -------------------------------------------------------

/// Mean period (hours) of the clock over `days` local days of a
/// light-dark cycle, after a 20-day spin-up, measured from the unwrapped
/// oscillator phase. The human is held awake (no sleep gate), so light
/// reaches the eyes exactly as scheduled.
fn clock_period(tau: f64, day_hours: f64, light_hours: f64, lux_fraction: f64, days: f64) -> f64 {
    let mut clock = CircadianClock::with_tau(tau);
    let h = 0.1;
    let total = day_hours * days;
    let start = day_hours * 20.0;
    let (mut t, mut turned, mut prev) = (0.0, 0.0, None::<f64>);
    while t < total {
        let daylight = if t % day_hours < light_hours {
            lux_fraction
        } else {
            0.0
        };
        clock.asleep = false;
        clock.step(Light::outdoors(daylight), 0.0, h, 1.0, 1.0);
        clock.asleep = false;
        t += h;
        if t >= start {
            let phase = clock.phase();
            if let Some(p) = prev {
                let mut d = phase - p;
                while d > std::f64::consts::PI {
                    d -= std::f64::consts::TAU;
                }
                while d < -std::f64::consts::PI {
                    d += std::f64::consts::TAU;
                }
                turned += d;
            }
            prev = Some(phase);
        }
    }
    (total - start) / (turned.abs() / std::f64::consts::TAU)
}

#[test]
fn the_clock_free_runs_near_24_2_hours_in_darkness() {
    // Reference `intrinsic_circadian_period`: 23.6-24.6 h, mean 24.15 h.
    let (lo, hi) = range("intrinsic_circadian_period");
    let period = clock_period(
        mk_engine::humans::circadian::MEAN_TAU_X_HOURS,
        24.0,
        0.0,
        0.0,
        40.0,
    );
    assert!(period > lo && period < hi, "{period}");
    assert!((period - 24.15).abs() < 0.1, "{period}");
}

#[test]
fn the_clock_entrains_to_a_24_hour_day() {
    let period = clock_period(24.0, 24.0, 16.0, 1.0, 40.0);
    assert!((period - 24.0).abs() < 0.02, "{period}");
    // Dim (300 lux) light still entrains a 24 h day.
    let dim = clock_period(24.0, 24.0, 16.0, 0.03, 40.0);
    assert!((dim - 24.0).abs() < 0.05, "{dim}");
}

#[test]
fn the_clock_does_not_lock_to_marr_kenas_36_hour_day() {
    // Reference `circadian_entrainment_range` excludes 36 h: the clock must
    // keep running near 24 h under any light level.
    let (lo, hi) = range("circadian_entrainment_range");
    assert!(!(lo..=hi).contains(&36.0));
    for (light_hours, lux_fraction) in [(24.0, 1.0), (18.0, 1.0), (24.0, 0.03)] {
        let period = clock_period(24.0, 36.0, light_hours, lux_fraction, 40.0);
        assert!(
            (23.5..=24.7).contains(&period),
            "{light_hours} h light at {lux_fraction}: {period}"
        );
    }
}

#[test]
fn a_bright_light_pulse_shifts_the_clock_within_the_published_limits() {
    // Reference `max_light_phase_shift`: single 6.7 h ~10,000 lux pulses
    // shift the clock at most 3.4 h (delay) / 2.2 h (advance). Tolerance:
    // the largest shift over the cycle lies in 1-4.5 h.
    let tau = 24.2;
    let free_run = |hours: f64, pulse: Option<(f64, f64)>| {
        let mut clock = CircadianClock::with_tau(tau);
        let h = 0.05;
        let mut t = 0.0;
        // Spin up 10 days in darkness.
        while t < 240.0 {
            clock.step(Light::outdoors(0.0), 0.0, h, 1.0, 1.0);
            clock.asleep = false;
            t += h;
        }
        let mut s = 0.0;
        while s < hours {
            let lit = pulse.is_some_and(|(start, len)| s >= start && s < start + len);
            clock.step(
                Light::outdoors(if lit { 1.0 } else { 0.0 }),
                0.0,
                h,
                1.0,
                1.0,
            );
            clock.asleep = false;
            s += h;
        }
        clock.phase()
    };
    let reference_phase = free_run(96.0, None);
    let mut largest: f64 = 0.0;
    for start in (0..24).map(|hour| hour as f64) {
        let shifted = free_run(96.0, Some((start, 6.7)));
        let mut d = shifted - reference_phase;
        while d > std::f64::consts::PI {
            d -= std::f64::consts::TAU;
        }
        while d < -std::f64::consts::PI {
            d += std::f64::consts::TAU;
        }
        largest = largest.max(d.abs() / std::f64::consts::TAU * tau);
    }
    assert!((1.0..=4.5).contains(&largest), "largest shift {largest} h");
}

/// Mean hours asleep per 24 h over the last `days` of a run with the
/// sleep gate active, and the hour (relative to lights-off) of the last
/// sleep onset.
fn sleep_per_day(day_hours: f64, light_hours: f64, lux_fraction: f64, days: f64) -> (f64, f64) {
    sleep_per_day_in(day_hours, light_hours, lux_fraction, None, days)
}

/// As [`sleep_per_day`], for a human who stays indoors in a room with
/// `room = Some((daylight_factor, lamp_lux))`: daylight reaches them
/// through the windows, a lamp is there when they want it, and at bedtime
/// the room goes dark.
fn sleep_per_day_in(
    day_hours: f64,
    light_hours: f64,
    lux_fraction: f64,
    room: Option<(f64, f64)>,
    days: f64,
) -> (f64, f64) {
    let mut clock = CircadianClock::with_tau(24.0);
    let mut fatigue = 0.3;
    let h = 0.1;
    let total = day_hours * (days + 10.0);
    let measure_from = day_hours * 10.0;
    let (mut t, mut slept, mut onset) = (0.0, 0.0, f64::NAN);
    while t < total {
        let daylight = if t % day_hours < light_hours {
            lux_fraction
        } else {
            0.0
        };
        let was_asleep = clock.asleep;
        let light = match room {
            Some((daylight_factor, lamp_lux)) => Light {
                daylight_fraction: daylight,
                daylight_factor,
                lamp_lux,
                indoors: true,
            },
            None => Light::outdoors(daylight),
        };
        fatigue = clock.step(light, fatigue, h, 1.0, 1.0);
        if clock.asleep && !was_asleep {
            onset = t % day_hours - light_hours;
        }
        if t >= measure_from && clock.asleep {
            slept += h;
        }
        t += h;
    }
    (slept / ((total - measure_from) / 24.0), onset)
}

#[test]
fn adults_sleep_seven_to_nine_hours_a_night_starting_after_dark() {
    // Reference `sleep_need_by_age` adult_18_64_years: 7-9 h.
    let lib = reference();
    let table = lib
        .item(ReferenceDomain::Humans, "sleep_need_by_age")
        .unwrap()
        .table()
        .unwrap();
    let lo = table.get("adult_18_64_years", "min").unwrap();
    let hi = table.get("adult_18_64_years", "max").unwrap();
    for lux_fraction in [1.0, 0.03] {
        let (hours, onset) = sleep_per_day(24.0, 16.0, lux_fraction, 20.0);
        assert!(hours >= lo && hours <= hi, "{hours} h at {lux_fraction}");
        assert!((0.0..=3.0).contains(&onset), "onset {onset} h after dark");
    }
}

#[test]
fn a_36_hour_day_disrupts_sleep() {
    // Without an artificial schedule, humans on Marr'Kena sleep less and
    // more irregularly than on a 24 h day.
    let (normal, _) = sleep_per_day(24.0, 16.0, 1.0, 20.0);
    let (marrkena, _) = sleep_per_day(36.0, 24.0, 1.0, 20.0);
    assert!(
        marrkena < normal - 1.5,
        "36 h day {marrkena} h vs 24 h day {normal} h"
    );
}

#[test]
fn sleep_cannot_be_resisted_forever() {
    // Kept in constant bright light, a human still falls asleep within
    // about two days.
    let mut clock = CircadianClock::with_tau(24.0);
    let mut fatigue = 0.1;
    let mut hours = 0.0;
    while !clock.asleep && hours < 72.0 {
        fatigue = clock.step(Light::outdoors(1.0), fatigue, 0.1, 1.0, 1.0);
        hours += 0.1;
    }
    assert!(clock.asleep && hours <= 50.0, "awake for {hours} h");
}

#[test]
fn a_living_human_sleeps_at_night_through_the_whole_runtime() {
    // The full lifecycle pipeline, half-hourly steps for 16 days under a
    // 24 h day: after ten days to entrain (a new human's clock starts at an
    // arbitrary phase, like jet lag), the clock drives the brain into sleep
    // each night. Their mind decides each step, as the runtime's does: on
    // steps this short, eating and drinking are its choices.
    let mut human = HumanBeing::new("realism-sleeper".into(), BiologicalSex::Female);
    let rng = RngRegistry::new([7u8; 32]);
    let dt = 0.5 * HOUR_YEARS;
    let mut asleep_steps = 0usize;
    let mut asleep_in_light = 0usize;
    let steps = 16 * 48;
    for step in 0..steps {
        let hour = (step as f64 * 0.5) % 24.0;
        let daylight = if (6.0..22.0).contains(&hour) {
            1.0
        } else {
            0.0
        };
        let obs = AgentWorldObservation {
            caloric_access: 1.0,
            hydration_access: 1.0,
            shelter_quality: 1.0,
            ambient_temperature_c: 20.0,
            daylight_fraction: daylight,
            ..AgentWorldObservation::default()
        };
        human.decide(&obs, step as u64, &rng);
        step_lifecycle(&mut human, dt, step as u64, &obs, &rng, (0, 0));
        if step >= 10 * 48 && human.neurochemistry.asleep {
            asleep_steps += 1;
            if daylight > 0.0 && !(6.0..8.0).contains(&hour) {
                asleep_in_light += 1;
            }
        }
    }
    let hours_per_day = asleep_steps as f64 * 0.5 / 6.0;
    assert!(
        (6.5..=9.5).contains(&hours_per_day),
        "{hours_per_day} h/day"
    );
    assert!(
        asleep_in_light <= 6 * 2,
        "{asleep_in_light} half-hours asleep in daylight"
    );
    assert_eq!(human.death_reason, None);
}

// --- Life history -----------------------------------------------------------

#[test]
fn gestation_matches_the_natural_median() {
    // Reference `gestation_length`: median 268 days from ovulation, range
    // 250-287. Tolerance: within 3 days of the median.
    let days = GESTATION_YEARS * 365.25;
    let (lo, hi) = range("gestation_length");
    assert!(days >= lo && days <= hi);
    assert!((days - 268.0).abs() <= 3.0, "{days}");
}

#[test]
fn adult_mortality_follows_the_reference_life_table() {
    // Reference `annual_mortality_by_age` (US 2019) for ages 40-90 and
    // `mortality_rate_doubling_time`. Tolerance: within a factor of two of
    // each tabulated qx (the island has no modern medicine, so higher is
    // expected), and the doubling time inside the reference range.
    let lib = reference();
    let table = lib
        .item(ReferenceDomain::Humans, "annual_mortality_by_age")
        .unwrap()
        .table()
        .unwrap()
        .clone();
    for age in ["40", "60", "70", "80", "90"] {
        let x: f64 = age.parse().unwrap();
        let hazard = GOMPERTZ_BASELINE_HAZARD_PER_YEAR * (GOMPERTZ_AGING_RATE_PER_YEAR * x).exp();
        let q = 1.0 - (-hazard).exp();
        let reference_q = table.get(age, "qx").unwrap();
        let ratio = q / reference_q;
        assert!(
            (0.5..=2.0).contains(&ratio),
            "age {age}: model {q:.4} vs {reference_q:.4}"
        );
    }
    let doubling = std::f64::consts::LN_2 / GOMPERTZ_AGING_RATE_PER_YEAR;
    let (lo, hi) = range("mortality_rate_doubling_time");
    assert!(doubling >= lo && doubling <= hi, "{doubling}");
}

/// The metabolic equivalents `humans::lifecycle` charges for a chosen action
/// are the packs' own numbers, not numbers that once came from the packs.
/// A pack revision that moves a row should fail here rather than quietly
/// change how hungry walking makes someone.
#[test]
fn action_costs_match_the_compendium_rows_they_came_from() {
    use mk_engine::humans::lifecycle::{CARPENTRY_MET, HAND_MINING_MET, WALKING_MET};
    use mk_engine::regional::labour::LabourTable;

    let table = LabourTable::load_default().expect("the labour packs load");
    for (constant, row, name) in [
        (WALKING_MET, "walking_4_8_kmh_level", "walking"),
        (HAND_MINING_MET, "hand_mining", "mining by hand"),
        (CARPENTRY_MET, "carpentry_general", "carpentry"),
    ] {
        let packed = table.met(row).unwrap_or_else(|e| panic!("{row}: {e:?}"));
        assert!(
            (constant - packed).abs() < 1e-9,
            "{name} is charged at {constant} MET but the pack's {row} is {packed}"
        );
    }

    // Walking has to cost more than the baseline the needs model is
    // calibrated at, or none of this reaches the drain rates at all. Read
    // from the pack rather than from the constant above, so this stays a
    // statement about the Compendium and not about our copy of it.
    let walking = table.met("walking_4_8_kmh_level").expect("walking");
    let baseline = mk_engine::regional::labour::BASELINE_MET;
    assert!(
        walking > baseline,
        "walking at {walking} MET is below the needs model's {baseline} MET baseline, so charging it would change nothing"
    );
}

/// Walking somewhere now makes a person thirstier and hungrier than sitting
/// still does. This is the behaviour the MET constants exist for; without
/// it they would be decoration.
#[test]
fn walking_somewhere_costs_more_than_sitting_still() {
    use mk_core::human::BiologicalSex;
    use mk_core::rng::RngRegistry;
    use mk_engine::humans::{lifecycle::step_lifecycle, ActionKind, HumanBeing};

    let rng = RngRegistry::new([5u8; 32]);
    // Half a day with nothing to eat or drink: long enough for the drain to
    // separate them, short enough that neither has bottomed out, where a
    // floor would hide the difference.
    let world = observation(0.0, 0.0, 20.0);
    let hour = 1.0 / (365.25 * 24.0);

    let spend_the_morning = |action: ActionKind| {
        let mut human = HumanBeing::new("walker".into(), BiologicalSex::Female);
        human.development.age_years = 30.0;
        for tick in 0..12u64 {
            // The action is re-asserted each tick: the autonomous mind would
            // otherwise choose its own, and this test is about the cost of a
            // given action, not about which one a human picks.
            human.economy_action.kind = action;
            step_lifecycle(&mut human, hour, tick, &world, &rng, (0, 0));
        }
        human.needs
    };

    let walked = spend_the_morning(ActionKind::Move);
    let sat = spend_the_morning(ActionKind::Idle);
    assert!(
        walked.glucose > 0.0 && sat.glucose > 0.0,
        "both bottomed out, so this proves nothing: {} and {}",
        walked.glucose,
        sat.glucose
    );

    assert!(
        walked.glucose < sat.glucose,
        "walking left more glucose ({}) than sitting ({})",
        walked.glucose,
        sat.glucose
    );
    assert!(
        walked.hydration < sat.hydration,
        "walking left more water ({}) than sitting ({})",
        walked.hydration,
        sat.hydration
    );
}

/// Nobody outlives the longest life anyone has actually lived.
///
/// `maximum_verified_lifespan` has sat in the life-history pack unchecked;
/// this ties the runtime's hard biological ceiling to it.
#[test]
fn nobody_outlives_the_longest_verified_human_life() {
    use mk_core::human::BiologicalSex;
    use mk_core::rng::RngRegistry;
    use mk_engine::humans::{lifecycle::step_lifecycle, HumanBeing};
    use mk_engine::validation::ReferenceDomain;

    let verified = reference()
        .item(ReferenceDomain::Humans, "maximum_verified_lifespan")
        .and_then(|i| i.point())
        .expect("the pack records a longest verified life");

    let rng = RngRegistry::new([9u8; 32]);
    let world = observation(1.0, 1.0, 20.0);
    let mut human = HumanBeing::new("methuselah".into(), BiologicalSex::Female);
    // Fed, watered and sheltered, just past the record: only the ceiling
    // itself can be what ends this.
    human.development.age_years = verified + 0.5;
    step_lifecycle(&mut human, HOUR_YEARS, 1, &world, &rng, (0, 0));

    assert!(
        matches!(human.profile.status, mk_core::human::HumanStatus::Dead),
        "someone reached {:.2} years, past the verified record of {verified}",
        human.development.age_years
    );
}

/// A woman's fertility ends before menopause, not after it.
///
/// The pack's `menopause_age` was never checked against anything. The
/// runtime models the end of fertility rather than menopause itself — about
/// a decade earlier (te Velde & Pearson 2002, cited where the constants
/// live) — so the honest check is the ordering: fertile well before the
/// pack's earliest menopause, finished by its latest.
#[test]
fn fertility_ends_before_the_published_menopause_window_closes() {
    use mk_core::human::BiologicalSex;
    use mk_engine::humans::{lifecycle::reproductive_timeline, HumanBeing};
    use mk_engine::validation::ReferenceDomain;

    let (earliest, latest) = reference()
        .item(ReferenceDomain::Humans, "menopause_age")
        .and_then(|i| i.range())
        .expect("the pack records a menopause window");

    // Several women, because the age is drawn per individual.
    for i in 0..32 {
        let woman = HumanBeing::new(format!("woman-{i:02}"), BiologicalSex::Female);
        let timeline = reproductive_timeline(&woman);
        assert!(
            timeline.fecundity(25.0) > 0.0,
            "woman-{i:02} is infertile at 25"
        );
        assert_eq!(
            timeline.fecundity(latest),
            0.0,
            "woman-{i:02} is still fertile at {latest}, the latest published menopause"
        );
    }

    // And the end of fertility really does precede menopause rather than
    // coinciding with it: the average woman is finished before the earliest
    // published menopause age.
    let finished_early = (0..32)
        .filter(|i| {
            let woman = HumanBeing::new(format!("woman-{i:02}"), BiologicalSex::Female);
            reproductive_timeline(&woman).fecundity(earliest) == 0.0
        })
        .count();
    assert!(
        finished_early > 16,
        "only {finished_early} of 32 women were infertile by {earliest}, so fertility is not \
         ending the decade before menopause the constants claim"
    );
}

/// Survival holds for a person whose chosen action is Rest, not only for
/// one the test hands the baseline to directly.
///
/// The survival tests above all pass `EffortFocus::none()`, which is exactly
/// the 1.5 MET the drain rates are calibrated at, so they cannot see what
/// happens when a real activity level reaches the needs model. That left a
/// blind spot: a human whose action is Rest goes through the full lifecycle,
/// not through `NeedsSnapshot::step` directly. This walks one through it and
/// holds them to the same published range. It does not test sleep itself —
/// the circadian clock owns `asleep` and the action does not set it — which
/// is why the name says resting.
#[test]
fn a_resting_adult_still_dies_of_thirst_within_the_published_range() {
    use mk_core::human::BiologicalSex;
    use mk_core::rng::RngRegistry;
    use mk_engine::humans::{lifecycle::step_lifecycle, ActionKind, HumanBeing};

    let (lo, hi) = range("survival_without_water");
    let rng = RngRegistry::new([21u8; 32]);
    // Fed and sheltered at a temperate 20 °C, with nothing to drink.
    let world = observation(1.0, 0.0, 20.0);

    let mut human = HumanBeing::new("resting".into(), BiologicalSex::Female);
    human.development.age_years = 30.0;
    let mut died_on_day = None;
    for hour in 1..=(24 * 30) {
        human.economy_action.kind = ActionKind::Rest;
        step_lifecycle(&mut human, HOUR_YEARS, hour as u64, &world, &rng, (0, 0));
        if human.needs.hydration <= 0.0 {
            died_on_day = Some(hour as f64 / 24.0);
            break;
        }
    }

    let days = died_on_day.expect("a resting adult with no water runs dry within a month");
    assert!(
        days >= lo && days <= hi,
        "a resting adult ran dry after {days} days, outside the published {lo}-{hi}"
    );
}

#[test]
fn a_house_with_lamps_and_curtains_keeps_its_own_night_on_marr_kena() {
    // D23. Out of doors a body cannot keep a night on a 36 h day: the sun
    // is up when its clock says sleep, and nothing puts it out. In the
    // house, daylight comes through windows at the room's daylight factor,
    // a lamp gives room light after dark, and at bedtime the lamp goes out
    // and the curtains are drawn -- and sleep comes back to the adult
    // reference of 7-9 h.
    use mk_engine::regional::energy::ROOM_LIGHT_LUX;
    use mk_engine::regional::humans::{BEDROOM_DAYLIGHT_FACTOR, LIVING_ROOM_DAYLIGHT_FACTOR};
    let lib = reference();
    let table = lib
        .item(ReferenceDomain::Humans, "sleep_need_by_age")
        .unwrap()
        .table()
        .unwrap();
    let lo = table.get("adult_18_64_years", "min").unwrap();
    let hi = table.get("adult_18_64_years", "max").unwrap();
    let (outdoors, _) = sleep_per_day(36.0, 24.0, 1.0, 20.0);
    assert!(outdoors < lo, "outdoors on a 36 h day {outdoors} h");
    for factor in [BEDROOM_DAYLIGHT_FACTOR, LIVING_ROOM_DAYLIGHT_FACTOR] {
        for light_hours in [18.0, 24.0] {
            let (indoors, _) =
                sleep_per_day_in(36.0, light_hours, 1.0, Some((factor, ROOM_LIGHT_LUX)), 20.0);
            assert!(
                (lo..=hi).contains(&indoors),
                "indoors at daylight factor {factor}, {light_hours} h of daylight: {indoors} h"
            );
        }
    }
    // And the same house on a 24 h day takes nothing away.
    let (home, _) = sleep_per_day_in(
        24.0,
        16.0,
        1.0,
        Some((LIVING_ROOM_DAYLIGHT_FACTOR, ROOM_LIGHT_LUX)),
        20.0,
    );
    assert!((lo..=hi).contains(&home), "{home} h");
}

#[test]
fn the_lighting_figures_are_the_reference_packs() {
    use mk_engine::regional::energy::{
        LED_LAMP_LUMENS_PER_W, ROOM_LIGHT_LUX, ROOM_UTILISATION_FACTOR,
    };
    use mk_engine::regional::humans::{
        BEDROOM_DAYLIGHT_FACTOR, KITCHEN_DAYLIGHT_FACTOR, LIVING_ROOM_DAYLIGHT_FACTOR,
    };
    let lib = reference();
    let point = |domain, key| lib.item(domain, key).unwrap().point().unwrap();
    assert_eq!(
        ROOM_LIGHT_LUX,
        point(ReferenceDomain::Humans, "room_light_illuminance")
    );
    assert_eq!(
        LED_LAMP_LUMENS_PER_W,
        point(ReferenceDomain::Labour, "led_lamp_efficacy")
    );
    assert_eq!(
        Some(ROOM_UTILISATION_FACTOR),
        lib.item(ReferenceDomain::Labour, "room_utilisation_factor")
            .unwrap()
            .central()
    );
    let factors = lib
        .item(ReferenceDomain::Labour, "room_daylight_factor")
        .unwrap()
        .table()
        .unwrap();
    assert_eq!(
        factors.get("bedroom", "daylight_factor"),
        Some(BEDROOM_DAYLIGHT_FACTOR)
    );
    assert_eq!(
        factors.get("living_room", "daylight_factor"),
        Some(LIVING_ROOM_DAYLIGHT_FACTOR)
    );
    assert_eq!(
        factors.get("kitchen", "daylight_factor"),
        Some(KITCHEN_DAYLIGHT_FACTOR)
    );
}
