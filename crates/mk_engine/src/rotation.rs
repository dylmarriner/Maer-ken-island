// mk_engine::rotation - Planetary rotation for MARR'KENA
// Deterministic rotation angle and subsolar point calculation

use mk_core::canon::CanonLocked;
use serde::{Deserialize, Serialize};

const TAU: f64 = 2.0 * std::f64::consts::PI;
const PI: f64 = std::f64::consts::PI;

/// Rotation state at a given tick
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationState {
    /// Current rotation angle (radians, 0 to 2π)
    pub rotation_angle: f64,
    /// Subsolar longitude (radians, -π to π)
    pub subsolar_longitude: f64,
    /// Subsolar latitude (radians, -π/2 to π/2)
    pub subsolar_latitude: f64,
}

/// Planetary rotation calculator
pub struct RotationCalculator {
    /// Axial tilt (radians)
    axial_tilt: f64,
    /// Rotation rate (radians per second)
    rotation_rate: f64,
}

impl RotationCalculator {
    /// Create new rotation calculator from canon values
    pub fn from_canon(canon: &CanonLocked) -> Self {
        // Day length in seconds
        let day_length = canon.rotation_period_s;

        // Axial tilt from canon (degrees)
        let axial_tilt = canon.obliquity_deg * PI / 180.0;

        // Rotation rate: 2π / day_length (radians per second)
        let rotation_rate = TAU / day_length;

        Self {
            axial_tilt,
            rotation_rate,
        }
    }

    /// Calculate rotation state for given tick and orbital phase
    /// Rotation state `time_seconds` into the simulation, with the planet
    /// `orbital_angle_rad` (radians) around its orbit. The subsolar latitude
    /// (solar declination) swings between ±axial tilt over the year.
    pub fn calculate_at(&self, time_seconds: f64, orbital_angle_rad: f64) -> RotationState {
        let rotation_angle = (self.rotation_rate * time_seconds.max(0.0)).rem_euclid(TAU);
        let subsolar_longitude = (-rotation_angle + PI).rem_euclid(TAU) - PI;
        let subsolar_latitude = self.axial_tilt * orbital_angle_rad.sin();

        RotationState {
            rotation_angle,
            subsolar_longitude,
            subsolar_latitude,
        }
    }

    pub fn local_solar_time(&self, rotation_angle: f64, longitude: f64) -> f64 {
        // Solar noon occurs when sun is at local meridian
        let solar_hour_angle = rotation_angle + longitude;
        ((solar_hour_angle + PI) % TAU) - PI
    }

    /// Check if a location is in daylight
    pub fn is_daylight(
        &self,
        subsolar_longitude: f64,
        subsolar_latitude: f64,
        longitude: f64,
        latitude: f64,
    ) -> bool {
        // Calculate solar zenith angle
        let hour_angle = longitude - subsolar_longitude;
        let cos_zenith = latitude.sin() * subsolar_latitude.sin()
            + latitude.cos() * subsolar_latitude.cos() * hour_angle.cos();

        // Daylight if sun is above horizon (zenith angle < 90°)
        cos_zenith > 0.0
    }
}

/// Step rotation for one tick
/// Rotation state at `time_seconds` of simulated time with the planet at
/// `orbital_angle_rad` around its orbit.
pub fn step_rotation(
    time_seconds: f64,
    orbital_angle_rad: f64,
    canon: &CanonLocked,
) -> RotationState {
    RotationCalculator::from_canon(canon).calculate_at(time_seconds, orbital_angle_rad)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rotation_determinism() {
        let canon = CanonLocked::default();
        let calculator = RotationCalculator::from_canon(&canon);

        // Same time should always produce same result
        let state1 = calculator.calculate_at(1234.5, 0.5);
        let state2 = calculator.calculate_at(1234.5, 0.5);

        assert_eq!(state1.rotation_angle, state2.rotation_angle);
        assert_eq!(state1.subsolar_longitude, state2.subsolar_longitude);
        assert_eq!(state1.subsolar_latitude, state2.subsolar_latitude);
    }

    #[test]
    fn test_day_wraparound() {
        let canon = CanonLocked::default();
        let calculator = RotationCalculator::from_canon(&canon);

        // After one full day, rotation should return to start
        let state_start = calculator.calculate_at(0.0, 0.0);
        let state_end = calculator.calculate_at(canon.rotation_period_s, 0.0);
        let delta = (state_end.rotation_angle - state_start.rotation_angle).rem_euclid(TAU);
        assert!(delta < 1e-6 || TAU - delta < 1e-6);
    }

    #[test]
    fn test_is_daylight() {
        let canon = CanonLocked::default();
        let calculator = RotationCalculator::from_canon(&canon);

        // At subsolar point, it should be day
        assert!(calculator.is_daylight(0.0, 0.0, 0.0, 0.0));

        // At antipodal point, it should be night
        assert!(!calculator.is_daylight(0.0, 0.0, PI, 0.0));
    }

    #[test]
    fn declination_reaches_both_tropics_over_an_orbit() {
        let canon = CanonLocked::default();
        let calculator = RotationCalculator::from_canon(&canon);
        let north = calculator.calculate_at(0.0, PI / 2.0).subsolar_latitude;
        let south = calculator
            .calculate_at(0.0, 3.0 * PI / 2.0)
            .subsolar_latitude;
        assert!(north > 0.0 && south < 0.0);
        assert!((north + south).abs() < 1e-12);
    }
}
