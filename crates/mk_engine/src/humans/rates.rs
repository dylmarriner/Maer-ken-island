//! Turning per-unit-time rates into per-step changes.
//!
//! Human subsystems are stepped with the world's real `dt_years`, which is a
//! fraction of a second for a live run and can be days for a coarse one.
//! Each rate therefore names its own time unit (per hour, per day, …), and
//! a step converts the elapsed time into that unit exactly. First-order
//! relaxations use the closed form `1 − e^(−rate·t)`, so no step length can
//! overshoot the target.

/// One hour, in years.
pub const HOUR_YEARS: f64 = 1.0 / (365.25 * 24.0);
/// One day, in years.
pub const DAY_YEARS: f64 = 1.0 / 365.25;
/// One week, in years.
pub const WEEK_YEARS: f64 = 7.0 / 365.25;
/// One (mean Julian) month, in years.
pub const MONTH_YEARS: f64 = 1.0 / 12.0;

/// `dt_years` expressed in units of `unit_years` (never negative).
pub fn elapsed(dt_years: f64, unit_years: f64) -> f64 {
    dt_years.max(0.0) / unit_years
}

/// Fraction of the remaining gap to a target that first-order relaxation at
/// `rate` per `unit_years` closes in `dt_years`: `1 − e^(−rate·t)`.
pub fn relaxation_fraction(rate: f64, dt_years: f64, unit_years: f64) -> f64 {
    let exposure = rate.max(0.0) * elapsed(dt_years, unit_years);
    -(-exposure).exp_m1()
}

/// Probability that an event with hazard `rate` per `unit_years` happens at
/// least once in `dt_years`: `1 − e^(−rate·t)` (a Poisson process).
pub fn event_probability(rate: f64, dt_years: f64, unit_years: f64) -> f64 {
    relaxation_fraction(rate, dt_years, unit_years)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relaxation_composes_across_step_lengths() {
        // Two half-hour steps close exactly as much of the gap as one hour.
        let one = relaxation_fraction(0.4, HOUR_YEARS, HOUR_YEARS);
        let half = relaxation_fraction(0.4, HOUR_YEARS / 2.0, HOUR_YEARS);
        assert!(((1.0 - half) * (1.0 - half) - (1.0 - one)).abs() < 1e-12);
    }

    #[test]
    fn long_steps_saturate_instead_of_overshooting() {
        assert!(relaxation_fraction(5.0, 1.0, HOUR_YEARS) <= 1.0);
        assert_eq!(relaxation_fraction(5.0, 0.0, HOUR_YEARS), 0.0);
        assert_eq!(relaxation_fraction(5.0, -1.0, HOUR_YEARS), 0.0);
    }

    #[test]
    fn a_one_second_step_is_a_one_second_step() {
        let second = 1.0 / (365.25 * 24.0 * 3600.0);
        assert!((elapsed(second, HOUR_YEARS) - 1.0 / 3600.0).abs() < 1e-12);
    }
}
