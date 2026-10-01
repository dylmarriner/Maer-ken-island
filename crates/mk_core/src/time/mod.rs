/**
 * Purpose
 * - World-level and planet-level time management for Maer'Ken simulation.
 * - Provides deterministic time progression and calendar conversions.
 *
 * Invariants
 * - Tick is always u64 and starts at 0.
 * - TimeBase advances by whole ticks (no system time).
 * - Scheduler describes the canonical phase order and cycles through it;
 *   `mk_engine::WorldState::step_world` runs its own pipeline and does not
 *   consult it.
 * - Calendar uses exact Rational arithmetic, so a sub-second canon step
 *   (0.1 s = 1/10) converts without rounding.
 *
 * Failure Modes
 * - Rational overflow (a result that does not fit i64 after reduction)
 *   → panic naming the operation; use the checked_* forms to handle it.
 *
 * Debug Notes
 * - Use Calendar::tick_to_days_at (or tick_to_days for whole-second steps)
 *   for planet-local time conversion.
 */
use crate::canon::CanonLocked;
use std::fmt;

/// World-level time counter (deterministic, no floats)
///
/// Represents discrete simulation steps.
/// MUST be u64 (no floats, no i64).
/// MUST start at 0 and increment by 1.
/// MUST NEVER reference planet-specific time units.
pub type Tick = u64;

/// World-level clock for deterministic time progression
///
/// Tracks the current simulation tick.
/// MUST be world-level (no planet-specific fields).
/// MUST be deterministic (same initial state → same progression).
/// MUST NEVER use system time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeBase {
    pub tick: Tick,
}

impl TimeBase {
    /// Create new time base starting at tick 0
    ///
    /// # Returns
    ///
    /// New TimeBase with tick = 0
    pub fn new() -> Self {
        Self { tick: 0 }
    }

    /// Advance time by one tick
    ///
    /// Increments tick by exactly 1.
    /// This is the only way time progresses in the simulation.
    pub fn advance(&mut self) {
        self.tick += 1;
    }

    /// Get current tick value
    ///
    /// # Returns
    ///
    /// Current tick as u64
    pub fn current(&self) -> Tick {
        self.tick
    }
}

impl Default for TimeBase {
    fn default() -> Self {
        Self::new()
    }
}

/// World-level phase sequence: Orbit → Insolation → … → HashCommit →
/// LedgerClear → TimeAdvance, then back to Orbit.
///
/// `next_phase` steps through that order. It records position only; it does
/// not stop code from running phases out of order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scheduler {
    pub current_phase: Phase,
}

/// Simulation phases in exact execution order
///
/// This defines the deterministic sequence of operations.
/// The order is fixed and cannot be changed without plan amendment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Phase {
    Orbit,
    Insolation,
    Planet,
    Tides,
    Climate,
    Weather,
    Hydrology,
    Ocean,
    Tectonics,
    Volcanism,
    Biosphere,
    Disturbance,
    Evolution,
    Extinction,
    Audit,
    HashCommit,
    LedgerClear,
    TimeAdvance,
}

impl Scheduler {
    /// Create new scheduler starting at Orbit phase
    ///
    /// # Returns
    ///
    /// New Scheduler with current_phase = Orbit
    pub fn new() -> Self {
        Self {
            current_phase: Phase::Orbit,
        }
    }

    /// Advance to next phase in deterministic order
    ///
    /// Follows the exact phase sequence.
    /// After HashCommit, wraps around to Orbit.
    ///
    /// # Returns
    ///
    /// Next phase in sequence
    pub fn next_phase(&mut self) -> Phase {
        use Phase::*;
        self.current_phase = match self.current_phase {
            Orbit => Insolation,
            Insolation => Planet,
            Planet => Tides,
            Tides => Climate,
            Climate => Weather,
            Weather => Hydrology,
            Hydrology => Ocean,
            Ocean => Tectonics,
            Tectonics => Volcanism,
            Volcanism => Biosphere,
            Biosphere => Disturbance,
            Disturbance => Evolution,
            Evolution => Extinction,
            Extinction => Audit,
            Audit => HashCommit,
            HashCommit => LedgerClear,
            LedgerClear => TimeAdvance,
            TimeAdvance => Orbit,
        };
        self.current_phase
    }

    /// Reset scheduler to Orbit phase
    ///
    /// Used to restart simulation or reset phase state.
    pub fn reset(&mut self) {
        self.current_phase = Phase::Orbit;
    }

    /// Check if current phase is the specified phase
    ///
    /// # Arguments
    ///
    /// * `phase` - Phase to check against
    ///
    /// # Returns
    ///
    /// True if current phase matches, false otherwise
    pub fn is_phase(&self, phase: Phase) -> bool {
        self.current_phase == phase
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for Phase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Phase::Orbit => write!(f, "Orbit"),
            Phase::Insolation => write!(f, "Insolation"),
            Phase::Planet => write!(f, "Planet"),
            Phase::Tides => write!(f, "Tides"),
            Phase::Climate => write!(f, "Climate"),
            Phase::Weather => write!(f, "Weather"),
            Phase::Hydrology => write!(f, "Hydrology"),
            Phase::Ocean => write!(f, "Ocean"),
            Phase::Tectonics => write!(f, "Tectonics"),
            Phase::Volcanism => write!(f, "Volcanism"),
            Phase::Biosphere => write!(f, "Biosphere"),
            Phase::Disturbance => write!(f, "Disturbance"),
            Phase::Evolution => write!(f, "Evolution"),
            Phase::Extinction => write!(f, "Extinction"),
            Phase::Audit => write!(f, "Audit"),
            Phase::HashCommit => write!(f, "HashCommit"),
            Phase::LedgerClear => write!(f, "LedgerClear"),
            Phase::TimeAdvance => write!(f, "TimeAdvance"),
        }
    }
}

/// Exact fraction arithmetic for planet-level time calculations.
///
/// Always stored in lowest terms with a positive denominator. Operations
/// compute in `i128` and reduce before narrowing back to `i64`, so an
/// intermediate product only overflows if the reduced result itself does
/// not fit. `to_f64()` is for display and logging.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rational {
    pub num: i64,
    pub den: i64,
}

impl Rational {
    /// Create new rational number
    ///
    /// Automatically reduces to lowest terms.
    /// Denominator cannot be zero.
    ///
    /// # Arguments
    ///
    /// * `num` - Numerator
    /// * `den` - Denominator (must be non-zero)
    ///
    /// # Panics
    ///
    /// Panics if den = 0
    pub fn new(num: i64, den: i64) -> Self {
        assert_ne!(den, 0, "Denominator cannot be zero");
        let mut result = Self { num, den };
        result.reduce();
        result
    }

    /// Convert to f64 for display/logging ONLY
    ///
    /// # Warning
    ///
    /// NEVER use for computation - display only!
    ///
    /// # Returns
    ///
    /// f64 representation
    pub fn to_f64(&self) -> f64 {
        self.num as f64 / self.den as f64
    }

    /// Build a reduced rational from an `i128` fraction, or `None` if the
    /// reduced value does not fit `i64` (or `den` is zero).
    fn from_wide(num: i128, den: i128) -> Option<Rational> {
        if den == 0 {
            return None;
        }
        let g = gcd_i128(num.unsigned_abs(), den.unsigned_abs()).max(1) as i128;
        let (mut num, mut den) = (num / g, den / g);
        if den < 0 {
            num = -num;
            den = -den;
        }
        Some(Rational {
            num: i64::try_from(num).ok()?,
            den: i64::try_from(den).ok()?,
        })
    }

    /// Sum, or `None` on overflow.
    pub fn checked_add(&self, other: &Rational) -> Option<Rational> {
        let (a, b, c, d) = (
            self.num as i128,
            self.den as i128,
            other.num as i128,
            other.den as i128,
        );
        Self::from_wide(a * d + c * b, b * d)
    }

    /// Difference, or `None` on overflow.
    pub fn checked_sub(&self, other: &Rational) -> Option<Rational> {
        let (a, b, c, d) = (
            self.num as i128,
            self.den as i128,
            other.num as i128,
            other.den as i128,
        );
        Self::from_wide(a * d - c * b, b * d)
    }

    /// Product, or `None` on overflow.
    pub fn checked_mul(&self, other: &Rational) -> Option<Rational> {
        Self::from_wide(
            self.num as i128 * other.num as i128,
            self.den as i128 * other.den as i128,
        )
    }

    /// Quotient, or `None` on overflow or division by zero.
    pub fn checked_div(&self, other: &Rational) -> Option<Rational> {
        if other.num == 0 {
            return None;
        }
        Self::from_wide(
            self.num as i128 * other.den as i128,
            self.den as i128 * other.num as i128,
        )
    }

    /// Add two rational numbers.
    ///
    /// # Panics
    ///
    /// If the reduced sum does not fit `i64`.
    pub fn add(&self, other: &Rational) -> Rational {
        self.checked_add(other).expect("Rational overflow in add")
    }

    /// Multiply two rational numbers.
    ///
    /// # Panics
    ///
    /// If the reduced product does not fit `i64`.
    pub fn mul(&self, other: &Rational) -> Rational {
        self.checked_mul(other).expect("Rational overflow in mul")
    }

    /// Subtract two rational numbers.
    ///
    /// # Panics
    ///
    /// If the reduced difference does not fit `i64`.
    pub fn sub(&self, other: &Rational) -> Rational {
        self.checked_sub(other).expect("Rational overflow in sub")
    }

    /// Divide two rational numbers.
    ///
    /// # Panics
    ///
    /// If `other` is zero, or the reduced quotient does not fit `i64`.
    pub fn div(&self, other: &Rational) -> Rational {
        assert_ne!(other.num, 0, "Cannot divide by zero");
        self.checked_div(other).expect("Rational overflow in div")
    }

    /// The exact value of the shortest decimal that round-trips to `value`,
    /// e.g. `0.1` becomes 1/10 rather than the binary approximation. Returns
    /// `None` for non-finite values or ones that don't fit.
    pub fn from_decimal_f64(value: f64) -> Option<Rational> {
        if !value.is_finite() {
            return None;
        }
        let text = format!("{value}");
        let (negative, digits) = match text.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, text.as_str()),
        };
        let (whole, frac) = digits.split_once('.').unwrap_or((digits, ""));
        let scale = 10i128.checked_pow(u32::try_from(frac.len()).ok()?)?;
        let mut num: i128 = format!("{whole}{frac}").parse().ok()?;
        if negative {
            num = -num;
        }
        Self::from_wide(num, scale)
    }

    /// Reduce fraction to lowest terms
    fn reduce(&mut self) {
        let gcd = Self::gcd(self.num.abs(), self.den.abs());
        if gcd > 0 {
            self.num /= gcd;
            self.den /= gcd;
        }

        // Ensure denominator is positive
        if self.den < 0 {
            self.num = -self.num;
            self.den = -self.den;
        }
    }

    /// Compute greatest common divisor using Euclidean algorithm
    fn gcd(a: i64, b: i64) -> i64 {
        if b == 0 {
            a
        } else {
            Self::gcd(b, a % b)
        }
    }

    /// Create rational from integer
    ///
    /// # Arguments
    ///
    /// * `value` - Integer value
    ///
    /// # Returns
    ///
    /// Rational with denominator = 1
    pub fn from_int(value: i64) -> Self {
        Self::new(value, 1)
    }

    /// Get numerator
    pub fn numerator(&self) -> i64 {
        self.num
    }

    /// Get denominator
    pub fn denominator(&self) -> i64 {
        self.den
    }
}

fn gcd_i128(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

impl Default for Rational {
    fn default() -> Self {
        Self::new(0, 1)
    }
}

impl fmt::Display for Rational {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.den == 1 {
            write!(f, "{}", self.num)
        } else {
            write!(f, "{}/{}", self.num, self.den)
        }
    }
}

/// Planet-level calendar: converts world ticks to planet-local days and
/// years as exact rationals.
///
/// Built from canon: the day is `rotation_period_s` (129600 s), the year is
/// `orbital_period_s / rotation_period_s` days (360), and the canon tick is
/// `dt_seconds` (0.1 s = 1/10).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Calendar {
    pub day_seconds: u64,
    pub year_days: Rational,
    /// Canon tick length in seconds, exactly.
    pub step_seconds: Rational,
}

impl Calendar {
    /// Create new calendar from canon parameters
    ///
    /// Extracts day_seconds and year_days from CanonLocked.
    ///
    /// # Arguments
    ///
    /// * `canon` - CanonLocked with time parameters
    ///
    /// # Returns
    ///
    /// New Calendar configured for the planet
    pub fn new(canon: &CanonLocked) -> Self {
        // rotation_period_s is the length of 1 day in seconds
        let rotation_period = canon.rotation_period_s as i64;
        // orbital_period_s is the length of 1 year in seconds
        let orbital_period = canon.orbital_period_s as i64;

        let year_days = Rational::new(orbital_period, rotation_period);

        // The canon dt_seconds (0.1) is a decimal; take it exactly.
        let step_seconds = Rational::from_decimal_f64(canon.dt_seconds)
            .expect("canon dt_seconds must be a finite decimal");

        Self {
            day_seconds: rotation_period as u64,
            year_days,
            step_seconds,
        }
    }

    /// Days elapsed after `tick` ticks of the canon step.
    pub fn canon_tick_to_days(&self, tick: Tick) -> Rational {
        self.tick_to_days_at(tick, self.step_seconds)
    }

    /// Days elapsed after `tick` ticks of `step_seconds` each, exactly.
    ///
    /// # Panics
    ///
    /// If the reduced result does not fit `i64`.
    pub fn tick_to_days_at(&self, tick: Tick, step_seconds: Rational) -> Rational {
        let ticks = Rational::from_wide(tick as i128, 1).expect("tick does not fit i64");
        ticks
            .mul(&step_seconds)
            .div(&Rational::from_int(self.day_seconds as i64))
    }

    /// Convert world ticks to planet days
    ///
    /// Uses exact rational arithmetic.
    ///
    /// # Arguments
    ///
    /// * `tick` - World tick count
    /// * `dt_seconds` - Whole seconds per tick. For the canon sub-second
    ///   step use [`Calendar::canon_tick_to_days`] or
    ///   [`Calendar::tick_to_days_at`].
    ///
    /// # Returns
    ///
    /// Days elapsed as Rational
    pub fn tick_to_days(&self, tick: Tick, dt_seconds: u64) -> Rational {
        self.tick_to_days_at(tick, Rational::from_int(dt_seconds as i64))
    }

    /// Convert world ticks to planet years
    ///
    /// Uses exact rational arithmetic.
    ///
    /// # Arguments
    ///
    /// * `tick` - World tick count
    /// * `dt_seconds` - Whole seconds per tick
    ///
    /// # Returns
    ///
    /// Years elapsed as Rational
    pub fn tick_to_years(&self, tick: Tick, dt_seconds: u64) -> Rational {
        let days = self.tick_to_days(tick, dt_seconds);
        days.div(&self.year_days)
    }

    /// Get day length in seconds
    ///
    /// # Returns
    ///
    /// Seconds per day from canon
    pub fn day_length(&self) -> u64 {
        self.day_seconds
    }

    /// Get year length as rational
    ///
    /// # Returns
    ///
    /// Days per year as Rational
    pub fn year_length(&self) -> Rational {
        self.year_days
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canon::CanonLocked;

    #[test]
    fn time_base_advances_deterministically() {
        let mut time = TimeBase::new();
        assert_eq!(time.current(), 0);

        time.advance();
        assert_eq!(time.current(), 1);

        time.advance();
        assert_eq!(time.current(), 2);
    }

    #[test]
    fn scheduler_cycles_in_phase_order() {
        let mut scheduler = Scheduler::new();
        assert_eq!(scheduler.current_phase, Phase::Orbit);

        let phases = [
            Phase::Insolation,
            Phase::Planet,
            Phase::Tides,
            Phase::Climate,
            Phase::Weather,
            Phase::Hydrology,
            Phase::Ocean,
            Phase::Tectonics,
            Phase::Volcanism,
            Phase::Biosphere,
            Phase::Disturbance,
            Phase::Evolution,
            Phase::Extinction,
            Phase::Audit,
            Phase::HashCommit,
            Phase::LedgerClear,
            Phase::TimeAdvance,
            Phase::Orbit, // Wrap around
        ];

        for &expected_phase in &phases {
            let next_phase = scheduler.next_phase();
            assert_eq!(next_phase, expected_phase);
        }
    }

    #[test]
    fn rational_arithmetic_exact() {
        let a = Rational::new(1, 2);
        let b = Rational::new(1, 4);

        let sum = a.add(&b);
        assert_eq!(sum, Rational::new(3, 4));

        let product = a.mul(&b);
        assert_eq!(product, Rational::new(1, 8));

        let difference = a.sub(&b);
        assert_eq!(difference, Rational::new(1, 4));

        let quotient = a.div(&b);
        assert_eq!(quotient, Rational::new(2, 1));
    }

    #[test]
    fn rational_survives_large_intermediates() {
        // (2^40 / 3) * (3 / 2^40) = 1, though the raw product overflows i64.
        let big = Rational::new(1 << 40, 3);
        let inv = Rational::new(3, 1 << 40);
        assert_eq!(big.mul(&inv), Rational::from_int(1));
        assert_eq!(
            Rational::from_int(i64::MAX).checked_add(&Rational::from_int(1)),
            None
        );
        assert_eq!(
            Rational::from_int(1).checked_div(&Rational::from_int(0)),
            None
        );
    }

    #[test]
    fn decimal_conversion_is_exact() {
        assert_eq!(Rational::from_decimal_f64(0.1), Some(Rational::new(1, 10)));
        assert_eq!(Rational::from_decimal_f64(-2.5), Some(Rational::new(-5, 2)));
        assert_eq!(
            Rational::from_decimal_f64(86400.0),
            Some(Rational::from_int(86400))
        );
        assert_eq!(Rational::from_decimal_f64(f64::NAN), None);
    }

    #[test]
    fn canon_step_converts_exactly() {
        let calendar = Calendar::new(&CanonLocked::default());
        assert_eq!(calendar.step_seconds, Rational::new(1, 10));
        assert_eq!(calendar.year_days, Rational::from_int(360));
        // One day is 129600 s = 1_296_000 canon ticks.
        assert_eq!(
            calendar.canon_tick_to_days(1_296_000),
            Rational::from_int(1)
        );
        assert_eq!(calendar.canon_tick_to_days(648_000), Rational::new(1, 2));
        assert_eq!(calendar.tick_to_days(129_600, 1), Rational::from_int(1));
    }

    #[test]
    fn rational_reduces_to_lowest_terms() {
        let r = Rational::new(4, 8);
        assert_eq!(r, Rational::new(1, 2));

        let r = Rational::new(-2, -4);
        assert_eq!(r, Rational::new(1, 2));

        let r = Rational::new(-2, 4);
        assert_eq!(r, Rational::new(-1, 2));
    }

    #[test]
    fn calendar_tick_to_days() {
        let canon = CanonLocked::default();
        let calendar = Calendar::new(&canon);

        // One day = 129600 seconds = 129600 ticks (dt_seconds = 1)
        let days = calendar.tick_to_days(129600, 1);
        assert_eq!(days, Rational::new(1, 1));

        // Half day
        let days = calendar.tick_to_days(129600 / 2, 1);
        assert_eq!(days, Rational::new(1, 2));
    }

    #[test]
    fn calendar_tick_to_years() {
        let canon = CanonLocked::default();
        let calendar = Calendar::new(&canon);

        // One year = 46656000 seconds / 129600 seconds per day = 360 days
        // In ticks: 46656000 ticks (dt_seconds = 1)
        let ticks_per_year = 46656000;
        let years = calendar.tick_to_years(ticks_per_year, 1);
        assert_eq!(years, Rational::new(1, 1));

        // Half year
        let years = calendar.tick_to_years(ticks_per_year / 2, 1);
        assert_eq!(years, Rational::new(1, 2));
    }

    #[test]
    fn calendar_uses_canon_parameters() {
        let canon = CanonLocked::default();
        let calendar = Calendar::new(&canon);

        assert_eq!(calendar.day_seconds, 129600);
        assert_eq!(calendar.year_days, Rational::new(46656000, 129600));
    }

    #[test]
    fn rational_to_f64_display_only() {
        let r = Rational::new(1, 2);
        assert_eq!(r.to_f64(), 0.5);

        let r = Rational::new(2, 1);
        assert_eq!(r.to_f64(), 2.0);
    }

    #[test]
    fn scheduler_reset() {
        let mut scheduler = Scheduler::new();

        // Advance several phases
        for _ in 0..5 {
            scheduler.next_phase();
        }

        // Reset should return to Orbit
        scheduler.reset();
        assert_eq!(scheduler.current_phase, Phase::Orbit);
    }

    #[test]
    fn scheduler_is_phase() {
        let scheduler = Scheduler::new();

        assert!(scheduler.is_phase(Phase::Orbit));
        assert!(!scheduler.is_phase(Phase::Insolation));
    }
}
