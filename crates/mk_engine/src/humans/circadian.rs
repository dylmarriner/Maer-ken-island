//! Circadian clock and sleep/wake regulation (two-process model).
//!
//! Process C is the light-driven circadian pacemaker of Forger, Jewett &
//! Kronauer (1999), "A simpler model of the human circadian pacemaker",
//! J. Biol. Rhythms 14:532-537: a van der Pol oscillator (`x`, `xc`) driven
//! by a photoreceptor activation process (`n`). It free-runs near 24.2 h,
//! entrains to light-dark cycles only within the narrow human range, and
//! responds to light with the published phase-response curve, so on
//! Marr'Kena's 36 h day the body clock does not lock to the day.
//!
//! Process S is the homeostatic sleep pressure, `NeedsSnapshot::fatigue`
//! (Borbély 1982; Daan, Beersma & Borbély 1984): it rises while awake and
//! dissipates during sleep. Sleep begins when melatonin (the clock's
//! night signal, suppressed by light) plus weighted sleep pressure crosses
//! an onset threshold, and ends when it falls below a lower wake threshold.

use serde::{Deserialize, Serialize};

/// Oscillator stiffness. Forger, Jewett & Kronauer (1999), Table 1.
const MU: f64 = 0.23;
/// Light sensitivity of `xc`. Forger et al. (1999).
const Q: f64 = 1.0 / 3.0;
/// Light modulation of the intrinsic frequency. Forger et al. (1999).
const K: f64 = 0.55;
/// Photic drive gain. Forger et al. (1999).
const G: f64 = 33.75;
/// Photoreceptor activation rate scale (per minute). Forger et al. (1999).
const ALPHA_0: f64 = 0.05;
/// Photoreceptor recovery rate (per minute). Forger et al. (1999).
const BETA: f64 = 0.0075;
/// Light-response exponent. Forger et al. (1999).
const P: f64 = 0.5;
/// Reference illuminance (lux). Forger et al. (1999).
const I_0: f64 = 9500.0;

/// Mean intrinsic period parameter `tau_x` (hours). With the model's
/// amplitude-dependent frequency, `tau_x = 24.0` free-runs at 24.14 h,
/// matching the measured human mean of 24.15 h (Duffy et al. 2011, PNAS
/// 108:15602; fixtures/reference/humans/physiology.json).
pub const MEAN_TAU_X_HOURS: f64 = 24.0;
/// Between-person standard deviation of the intrinsic period (hours).
/// Duffy et al. (2011): SD ~0.2 h.
pub const TAU_X_SD_HOURS: f64 = 0.2;

/// Daylight illuminance at full sun on the eye (lux): overcast-to-clear
/// daylight spans ~10^3-10^5 lux; 10,000 lux is the bright-light reference
/// used in human phase-response studies (Khalsa et al. 2003).
pub const OUTDOOR_DAYLIGHT_LUX: f64 = 10_000.0;
/// Illuminance giving half-maximal melatonin suppression (lux). Zeitzer et
/// al. (2000), J. Physiol. 526:695: ED50 ~ 100 lux.
const MELATONIN_SUPPRESSION_HALF_LUX: f64 = 100.0;
/// Time constant of plasma melatonin (hours): elimination half-life
/// ~40 min (Claustrat et al. 2005, Sleep Med. Rev. 9:11), tau = t½/ln2.
const MELATONIN_TIME_CONSTANT_HOURS: f64 = 1.0;
/// Phase lead of melatonin secretion over the oscillator's `-x` peak
/// (radians, 2 h). Calibrated so that on a 16 h light / 8 h dark day sleep
/// begins about an hour after dark and lasts ~8 h (Hirshkowitz et al.
/// 2015), matching dim-light melatonin onset ~2 h before habitual sleep.
const MELATONIN_PHASE_LEAD_RAD: f64 = std::f64::consts::PI / 6.0;

/// Sleep pressure rise time constant while awake (hours). Daan, Beersma &
/// Borbély (1984), Am. J. Physiol. 246:R161.
pub const SLEEP_PRESSURE_RISE_HOURS: f64 = 18.2;
/// Sleep pressure decay time constant during sleep (hours). Daan et al.
/// (1984).
pub const SLEEP_PRESSURE_DECAY_HOURS: f64 = 4.2;
/// Weight of sleep pressure against melatonin in the sleep gate.
/// Calibrated with the two thresholds below (see `MELATONIN_PHASE_LEAD_RAD`).
const SLEEP_PRESSURE_WEIGHT: f64 = 0.6;
/// Sleep begins when `melatonin + weight * pressure` exceeds this.
const SLEEP_ONSET_THRESHOLD: f64 = 0.9;
/// Sleep ends when `melatonin + weight * pressure` falls below this.
const WAKE_THRESHOLD: f64 = 0.6;
/// Sleep pressure at which sleep can no longer be resisted, whatever the
/// clock says: about 45 h of continuous wakefulness from a rested start.
const INVOLUNTARY_SLEEP_PRESSURE: f64 = 0.93;

/// Longest step whose sleep and wake are resolved (hours). A longer step
/// (a day-long decision period) contains a whole night: the human is
/// treated as a regular sleeper, decides the step awake, and carries the
/// day-mean sleep pressure.
pub const MAX_RESOLVED_STEP_HOURS: f64 = 6.0;
/// Day-mean sleep pressure of a regular sleeper (16 h awake, 8 h asleep;
/// the two-process cycle with the time constants above averages ~0.35).
pub const DAY_MEAN_SLEEP_PRESSURE: f64 = 0.35;
/// Sleep pressure after a normal waking day (~17 h awake from rested):
/// pressure beyond this is sleep deprivation.
pub const NORMAL_WAKING_SLEEP_PRESSURE: f64 = 0.65;
/// Longest integration sub-step (hours). RK4 at 6 min is well inside the
/// oscillator's stability limit and resolves the photoreceptor process.
const MAX_SUBSTEP_HOURS: f64 = 0.1;
/// Longest stretch of an unresolved (long) step whose pacemaker phase is
/// integrated (hours): beyond two days only the cost grows.
const MAX_INTEGRATED_HOURS: f64 = 48.0;

/// One human's circadian pacemaker state and sleep/wake state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CircadianClock {
    /// Oscillator state (dimensionless, amplitude ~1). `x` is lowest near
    /// the core-body-temperature minimum, late in the biological night.
    pub x: f64,
    pub xc: f64,
    /// Fraction of activated photoreceptors (0..1).
    pub n: f64,
    /// Intrinsic period parameter of this human (hours).
    pub tau_x_hours: f64,
    /// Plasma melatonin, 0 (daytime) .. 1 (night peak in darkness).
    pub melatonin: f64,
    pub asleep: bool,
}

impl Default for CircadianClock {
    fn default() -> Self {
        Self::with_tau(MEAN_TAU_X_HOURS)
    }
}

impl CircadianClock {
    /// A clock with intrinsic period parameter `tau_x_hours`, started on its
    /// limit cycle at the start of the biological day.
    pub fn with_tau(tau_x_hours: f64) -> Self {
        Self {
            x: 1.0,
            xc: 0.0,
            n: 0.0,
            tau_x_hours,
            melatonin: 0.0,
            asleep: false,
        }
    }

    /// A clock whose intrinsic period is drawn deterministically for the
    /// human with this numeric id: mean `MEAN_TAU_X_HOURS`, SD
    /// `TAU_X_SD_HOURS` (an Irwin-Hall approximation to a normal, clipped
    /// at ±3 SD).
    pub fn for_human(human_id: u64) -> Self {
        let digest = blake3::hash(&[b"circadian-tau".as_slice(), &human_id.to_le_bytes()].concat());
        let bytes = digest.as_bytes();
        // Sum of 12 uniforms on [0,1) minus 6 is ~N(0,1).
        let z: f64 = (0..12)
            .map(|i| u16::from_le_bytes([bytes[2 * i], bytes[2 * i + 1]]) as f64 / 65536.0)
            .sum::<f64>()
            - 6.0;
        Self::with_tau(MEAN_TAU_X_HOURS + TAU_X_SD_HOURS * z.clamp(-3.0, 3.0))
    }

    /// Phase angle of the oscillator (radians, `atan2(xc, x)`).
    pub fn phase(&self) -> f64 {
        self.xc.atan2(self.x)
    }

    /// Illuminance reaching this human's eyes: daylight when awake, none
    /// with eyes closed.
    pub fn eye_lux(&self, daylight_fraction: f64) -> f64 {
        if self.asleep {
            0.0
        } else {
            daylight_fraction.clamp(0.0, 1.0) * OUTDOOR_DAYLIGHT_LUX
        }
    }

    fn derivatives(&self, state: [f64; 3], lux: f64) -> [f64; 3] {
        let [x, xc, n] = state;
        let alpha = if lux > 0.0 {
            ALPHA_0 * (lux / I_0).powf(P)
        } else {
            0.0
        };
        let b_hat = G * (1.0 - n) * alpha;
        let b = b_hat * (1.0 - 0.4 * x) * (1.0 - 0.4 * xc);
        let w = std::f64::consts::PI / 12.0;
        let freq = 24.0 / (0.99729 * self.tau_x_hours);
        let dx = w * (xc + MU * (x / 3.0 + 4.0 / 3.0 * x.powi(3) - 256.0 / 105.0 * x.powi(7)) + b);
        let dxc = w * (Q * b * xc - x * (freq * freq + K * b));
        let dn = 60.0 * (alpha * (1.0 - n) - BETA * n);
        [dx, dxc, dn]
    }

    fn rk4(&mut self, lux: f64, h: f64) {
        let s = [self.x, self.xc, self.n];
        let add =
            |a: [f64; 3], k: [f64; 3], f: f64| [a[0] + f * k[0], a[1] + f * k[1], a[2] + f * k[2]];
        let k1 = self.derivatives(s, lux);
        let k2 = self.derivatives(add(s, k1, h / 2.0), lux);
        let k3 = self.derivatives(add(s, k2, h / 2.0), lux);
        let k4 = self.derivatives(add(s, k3, h), lux);
        self.x = s[0] + h / 6.0 * (k1[0] + 2.0 * k2[0] + 2.0 * k3[0] + k4[0]);
        self.xc = s[1] + h / 6.0 * (k1[1] + 2.0 * k2[1] + 2.0 * k3[1] + k4[1]);
        self.n = (s[2] + h / 6.0 * (k1[2] + 2.0 * k2[2] + 2.0 * k3[2] + k4[2])).clamp(0.0, 1.0);
    }

    /// Melatonin the pineal gland secretes now: the clock's night signal,
    /// suppressed by light.
    fn melatonin_target(&self, lux: f64) -> f64 {
        let (s, c) = MELATONIN_PHASE_LEAD_RAD.sin_cos();
        // Rotating (x, xc) advances the secretion window relative to -x.
        let night = -(self.x * c + self.xc * s);
        let suppression = lux / (lux + MELATONIN_SUPPRESSION_HALF_LUX);
        night.clamp(0.0, 1.0) * (1.0 - suppression)
    }

    /// Advance the clock and the sleep pressure `fatigue` (0..1) by
    /// `dt_hours` under constant `daylight_fraction`, switching between sleep
    /// and wake as the two processes dictate. Returns the new fatigue.
    ///
    /// `pressure_rate_factor` scales how fast pressure builds while awake
    /// (individual sleep need, `NeedsSnapshot::sleep_pressure_rate_factor`);
    /// `sleep_quality` (0..1, from shelter) scales how fast it dissipates
    /// in sleep: sleeping rough is less restorative.
    pub fn step(
        &mut self,
        daylight_fraction: f64,
        fatigue: f64,
        dt_hours: f64,
        pressure_rate_factor: f64,
        sleep_quality: f64,
    ) -> f64 {
        let rise_hours = SLEEP_PRESSURE_RISE_HOURS / pressure_rate_factor.clamp(0.5, 2.0);
        // Poor shelter fragments sleep: at worst it restores at half speed.
        let decay_hours = SLEEP_PRESSURE_DECAY_HOURS / (0.5 + 0.5 * sleep_quality.clamp(0.0, 1.0));
        let mut fatigue = fatigue.clamp(0.0, 1.0);
        let total = dt_hours.max(0.0);
        if total == 0.0 {
            return fatigue;
        }
        if total > MAX_RESOLVED_STEP_HOURS {
            // A step this long holds a whole night. Keep the pacemaker's
            // phase running (in darkness, the light of one instant cannot
            // stand for a whole day), then hand back an awake human at the
            // day-mean sleep pressure.
            let mut remaining = total.min(MAX_INTEGRATED_HOURS);
            while remaining > 1e-12 {
                let h = remaining.min(MAX_SUBSTEP_HOURS);
                self.rk4(0.0, h);
                remaining -= h;
            }
            self.asleep = false;
            self.melatonin = 0.0;
            return DAY_MEAN_SLEEP_PRESSURE;
        }
        let mut remaining = total;
        while remaining > 1e-12 {
            let h = remaining.min(MAX_SUBSTEP_HOURS);
            let lux = self.eye_lux(daylight_fraction);
            self.rk4(lux, h);
            let target = self.melatonin_target(lux);
            self.melatonin +=
                (target - self.melatonin) * -(-h / MELATONIN_TIME_CONSTANT_HOURS).exp_m1();
            fatigue = if self.asleep {
                fatigue * (-h / decay_hours).exp()
            } else {
                1.0 - (1.0 - fatigue) * (-h / rise_hours).exp()
            };
            let propensity = self.melatonin + SLEEP_PRESSURE_WEIGHT * fatigue;
            if !self.asleep
                && (propensity > SLEEP_ONSET_THRESHOLD || fatigue >= INVOLUNTARY_SLEEP_PRESSURE)
            {
                self.asleep = true;
            } else if self.asleep && propensity < WAKE_THRESHOLD {
                self.asleep = false;
            }
            remaining -= h;
        }
        fatigue
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intrinsic_periods_spread_around_the_human_mean() {
        let taus: Vec<f64> = (0..500)
            .map(|id| CircadianClock::for_human(id).tau_x_hours)
            .collect();
        let mean = taus.iter().sum::<f64>() / taus.len() as f64;
        let sd = (taus.iter().map(|t| (t - mean).powi(2)).sum::<f64>() / taus.len() as f64).sqrt();
        assert!((mean - MEAN_TAU_X_HOURS).abs() < 0.03, "{mean}");
        assert!((sd - TAU_X_SD_HOURS).abs() < 0.04, "{sd}");
        assert_eq!(CircadianClock::for_human(7), CircadianClock::for_human(7));
    }

    #[test]
    fn long_steps_cost_at_most_two_days_of_integration_and_stay_finite() {
        let mut clock = CircadianClock::default();
        let fatigue = clock.step(0.5, 0.2, 24.0 * 365.0, 1.0, 1.0);
        assert!(clock.x.is_finite() && clock.xc.is_finite());
        assert!((0.0..=1.0).contains(&fatigue));
    }

    #[test]
    fn a_zero_step_changes_nothing() {
        let mut clock = CircadianClock::default();
        let before = clock.clone();
        assert_eq!(clock.step(1.0, 0.4, 0.0, 1.0, 1.0), 0.4);
        assert_eq!(clock, before);
    }
}
