//! Astrological calculation engine for human trait determination.
//!
//! Computes birth chart from date/time/location:
//! - Sun sign (zodiac position)
//! - Moon sign (lunar position)
//! - Ascendant/Rising (eastern horizon at birth)
//! - Elemental balance (fire, earth, air, water)
//! - Modality balance (cardinal, fixed, mutable)
//!
//! These astrological factors determine base personality traits per canon.

use chrono::{DateTime, Datelike, Timelike, Utc};
use serde::{Deserialize, Serialize};

/// Zodiac signs in tropical astrology order
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ZodiacSign {
    Aries,       // Mar 21 - Apr 19 (Fire, Cardinal)
    Taurus,      // Apr 20 - May 20 (Earth, Fixed)
    Gemini,      // May 21 - Jun 20 (Air, Mutable)
    Cancer,      // Jun 21 - Jul 22 (Water, Cardinal)
    Leo,         // Jul 23 - Aug 22 (Fire, Fixed)
    Virgo,       // Aug 23 - Sep 22 (Earth, Mutable)
    Libra,       // Sep 23 - Oct 22 (Air, Cardinal)
    Scorpio,     // Oct 23 - Nov 21 (Water, Fixed)
    Sagittarius, // Nov 22 - Dec 21 (Fire, Mutable)
    Capricorn,   // Dec 22 - Jan 19 (Earth, Cardinal)
    Aquarius,    // Jan 20 - Feb 18 (Air, Fixed)
    Pisces,      // Feb 19 - Mar 20 (Water, Mutable)
}

impl ZodiacSign {
    /// Get the element for this sign
    pub fn element(&self) -> Element {
        match self {
            ZodiacSign::Aries | ZodiacSign::Leo | ZodiacSign::Sagittarius => Element::Fire,
            ZodiacSign::Taurus | ZodiacSign::Virgo | ZodiacSign::Capricorn => Element::Earth,
            ZodiacSign::Gemini | ZodiacSign::Libra | ZodiacSign::Aquarius => Element::Air,
            ZodiacSign::Cancer | ZodiacSign::Scorpio | ZodiacSign::Pisces => Element::Water,
        }
    }

    /// Get the modality for this sign
    pub fn modality(&self) -> Modality {
        match self {
            ZodiacSign::Aries | ZodiacSign::Cancer | ZodiacSign::Libra | ZodiacSign::Capricorn => {
                Modality::Cardinal
            }
            ZodiacSign::Taurus | ZodiacSign::Leo | ZodiacSign::Scorpio | ZodiacSign::Aquarius => {
                Modality::Fixed
            }
            ZodiacSign::Gemini
            | ZodiacSign::Virgo
            | ZodiacSign::Sagittarius
            | ZodiacSign::Pisces => Modality::Mutable,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Element {
    Fire,
    Earth,
    Air,
    Water,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modality {
    Cardinal,
    Fixed,
    Mutable,
}

/// Geographical coordinates for ascendant calculation
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GeoCoordinates {
    pub latitude: f64,  // -90 to 90
    pub longitude: f64, // -180 to 180
}

impl GeoCoordinates {
    /// Validates coordinates are in valid range
    pub fn validate(&self) -> Result<(), String> {
        if self.latitude < -90.0 || self.latitude > 90.0 {
            return Err(format!("Latitude {} out of range [-90, 90]", self.latitude));
        }
        if self.longitude < -180.0 || self.longitude > 180.0 {
            return Err(format!(
                "Longitude {} out of range [-180, 180]",
                self.longitude
            ));
        }
        Ok(())
    }
}

/// Complete birth chart with astrological positions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BirthChart {
    pub sun: ZodiacSign,
    pub moon: ZodiacSign,
    pub ascendant: ZodiacSign,
    pub element_balance: ElementalBalance,
    pub modality_balance: ModalityBalance,
    pub coordinates: GeoCoordinates,
    pub birth_timestamp: String, // ISO 8601
}

/// Elemental balance as proportions (sum = 1.0)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ElementalBalance {
    pub fire: f64,
    pub earth: f64,
    pub air: f64,
    pub water: f64,
}

impl ElementalBalance {
    /// Validates proportions sum to approximately 1.0
    pub fn validate(&self) -> Result<(), String> {
        let sum = self.fire + self.earth + self.air + self.water;
        if (sum - 1.0).abs() > 0.001 {
            return Err(format!("Elemental balance sums to {}, not 1.0", sum));
        }
        if self.fire < 0.0 || self.earth < 0.0 || self.air < 0.0 || self.water < 0.0 {
            return Err("Elemental balance cannot have negative values".to_string());
        }
        Ok(())
    }

    /// Dominant element (highest proportion)
    pub fn dominant(&self) -> Element {
        let mut max_val = self.fire;
        let mut dominant = Element::Fire;
        if self.earth > max_val {
            max_val = self.earth;
            dominant = Element::Earth;
        }
        if self.air > max_val {
            max_val = self.air;
            dominant = Element::Air;
        }
        if self.water > max_val {
            dominant = Element::Water;
        }
        dominant
    }
}

/// Modality balance as proportions (sum = 1.0)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ModalityBalance {
    pub cardinal: f64,
    pub fixed: f64,
    pub mutable: f64,
}

impl ModalityBalance {
    /// Validates proportions sum to approximately 1.0
    pub fn validate(&self) -> Result<(), String> {
        let sum = self.cardinal + self.fixed + self.mutable;
        if (sum - 1.0).abs() > 0.001 {
            return Err(format!("Modality balance sums to {}, not 1.0", sum));
        }
        if self.cardinal < 0.0 || self.fixed < 0.0 || self.mutable < 0.0 {
            return Err("Modality balance cannot have negative values".to_string());
        }
        Ok(())
    }
}

/// Astrology engine for calculating birth charts
pub struct AstrologyEngine;

impl AstrologyEngine {
    /// Sun sign from the sun's apparent ecliptic longitude at the birth
    /// instant (Meeus, *Astronomical Algorithms* ch. 25, accurate to
    /// ~0.01°), so cusp births land on the correct side of the boundary for
    /// their actual year rather than a fixed calendar table.
    pub fn calculate_sun_sign(date: DateTime<Utc>) -> ZodiacSign {
        let longitude = Self::calculate_sun_longitude(Self::utc_to_julian_day(date));
        Self::index_to_sign((longitude / 30.0).floor() as i32)
    }

    /// Apparent geocentric ecliptic longitude of the sun, degrees `0..360`
    /// (Meeus ch. 25, low-accuracy method).
    fn calculate_sun_longitude(julian_day: f64) -> f64 {
        let t = (julian_day - 2451545.0) / 36525.0;
        let t2 = t * t;
        let mean_longitude = 280.46646 + 36000.76983 * t + 0.0003032 * t2;
        let mean_anomaly = (357.52911 + 35999.05029 * t - 0.0001537 * t2).to_radians();
        let equation_of_centre = (1.914602 - 0.004817 * t - 0.000014 * t2) * mean_anomaly.sin()
            + (0.019993 - 0.000101 * t) * (2.0 * mean_anomaly).sin()
            + 0.000289 * (3.0 * mean_anomaly).sin();
        let true_longitude = mean_longitude + equation_of_centre;
        // Nutation and aberration correction to apparent longitude.
        let omega = (125.04 - 1934.136 * t).to_radians();
        (true_longitude - 0.00569 - 0.00478 * omega.sin()).rem_euclid(360.0)
    }

    /// Mean obliquity of the ecliptic at `julian_day`, degrees (Meeus
    /// eq. 22.2).
    fn mean_obliquity_degrees(julian_day: f64) -> f64 {
        let t = (julian_day - 2451545.0) / 36525.0;
        23.439_291_111 - 0.013_004_167 * t - 1.639e-7 * t * t + 5.036e-7 * t * t * t
    }

    /// Moon sign from the moon's apparent ecliptic longitude (Meeus ch. 47,
    /// about 10 arcseconds), so only births within seconds of arc of a cusp
    /// are uncertain.
    pub fn calculate_moon_sign(date: DateTime<Utc>) -> ZodiacSign {
        let julian_day = Self::utc_to_julian_day(date);
        let moon_longitude = Self::calculate_moon_longitude(julian_day);
        let sign_index = (moon_longitude / 30.0).floor() as i32 % 12;
        Self::index_to_sign(sign_index)
    }

    /// Convert UTC datetime to Julian Day Number
    /// Algorithm from Meeus' Astronomical Algorithms
    fn utc_to_julian_day(date: DateTime<Utc>) -> f64 {
        let year = date.year();
        let month = date.month() as i32;
        let day = date.day() as f64;
        let hour = date.hour() as f64 + date.minute() as f64 / 60.0 + date.second() as f64 / 3600.0;

        let (y, m) = if month <= 2 {
            (year - 1, month + 12)
        } else {
            (year, month)
        };

        let a = (y as f64 / 100.0).floor();
        let b = 2.0 - a + (a / 4.0).floor();

        let day_fraction = day + hour / 24.0;

        (365.25 * (y as f64 + 4716.0)).floor()
            + (30.6001 * (m as f64 + 1.0)).floor()
            + day_fraction
            + b
            - 1524.5
    }

    /// Mean elongation of the moon from the sun, in degrees — Meeus'
    /// term `D` in the lunar longitude perturbation series.
    fn mean_lunar_elongation_degrees(t: f64) -> f64 {
        let t2 = t * t;
        let t3 = t2 * t;
        297.8501921 + 445267.1114034 * t - 0.0018819 * t2 + t3 / 545868.0 - t3 * t / 113065000.0
    }

    /// Astronomical season at the birth instant and place: the quarter of
    /// the sun's ecliptic longitude (equinox to solstice), mirrored for the
    /// southern hemisphere, where the seasons are reversed.
    pub fn calculate_birth_season(date: DateTime<Utc>, latitude: f64) -> String {
        let longitude = Self::calculate_sun_longitude(Self::utc_to_julian_day(date));
        let northern = ["Spring", "Summer", "Autumn", "Winter"];
        let quarter = (longitude / 90.0).floor() as usize % 4;
        let quarter = if latitude < 0.0 {
            (quarter + 2) % 4
        } else {
            quarter
        };
        northern[quarter].to_string()
    }

    /// Lunar phase name from the moon's true elongation from the sun (moon
    /// minus sun ecliptic longitude, both from the Meeus series above),
    /// bucketed into the 8 standard phase names at 45-degree intervals
    /// centred on each name.
    pub fn calculate_lunar_phase(date: DateTime<Utc>) -> String {
        let julian_day = Self::utc_to_julian_day(date);
        let d = (Self::calculate_moon_longitude(julian_day)
            - Self::calculate_sun_longitude(julian_day))
        .rem_euclid(360.0);
        let phase_index = ((d + 22.5) / 45.0).floor() as i32 % 8;
        match phase_index {
            0 => "New Moon",
            1 => "Waxing Crescent",
            2 => "First Quarter",
            3 => "Waxing Gibbous",
            4 => "Full Moon",
            5 => "Waning Gibbous",
            6 => "Last Quarter",
            _ => "Waning Crescent",
        }
        .to_string()
    }

    /// Calculate lunar ecliptic longitude using Meeus' algorithm
    /// Returns longitude in degrees (0-360)
    /// Apparent geocentric ecliptic longitude of the moon at a UT Julian
    /// day, degrees `0..360`.
    ///
    /// Meeus, *Astronomical Algorithms* (2nd ed.) ch. 47: the 60 periodic
    /// terms of Table 47.A with the eccentricity factor `E`, the additive
    /// terms for Venus (A1), Jupiter (A2) and the Earth's flattening, then
    /// nutation in longitude (ch. 22). Accurate to about 10 arcseconds. The
    /// series is in dynamical time, so the UT instant is shifted by ΔT.
    fn calculate_moon_longitude(julian_day: f64) -> f64 {
        let jde = julian_day + delta_t_seconds(julian_day) / 86_400.0;
        let (geometric, t) = Self::geometric_moon_longitude_tt(jde);
        (geometric + nutation_in_longitude_degrees(t)).rem_euclid(360.0)
    }

    /// Geometric (mean-equinox) lunar longitude in degrees at a dynamical
    /// Julian ephemeris day, with the Julian centuries it used.
    fn geometric_moon_longitude_tt(jde: f64) -> (f64, f64) {
        let t = (jde - 2451545.0) / 36525.0;
        let t2 = t * t;
        let t3 = t2 * t;
        let t4 = t3 * t;

        // Mean longitude, elongation, anomalies and argument of latitude
        // (Meeus 47.1-47.5), degrees.
        let l_prime =
            218.3164477 + 481267.88123421 * t - 0.0015786 * t2 + t3 / 538841.0 - t4 / 65194000.0;
        let d = Self::mean_lunar_elongation_degrees(t);
        let m = 357.5291092 + 35999.0502909 * t - 0.0001536 * t2 + t3 / 24490000.0;
        let m_prime =
            134.9633964 + 477198.8675055 * t + 0.0087414 * t2 + t3 / 69699.0 - t4 / 14712000.0;
        let f =
            93.2720950 + 483202.0175233 * t - 0.0036539 * t2 - t3 / 3526000.0 + t4 / 863310000.0;

        // Eccentricity of the Earth's orbit, which scales terms in M.
        let e = 1.0 - 0.002516 * t - 0.0000074 * t2;

        let (d_r, m_r, mp_r, f_r) = (
            d.to_radians(),
            m.to_radians(),
            m_prime.to_radians(),
            f.to_radians(),
        );
        let mut sigma_l: f64 = MOON_LONGITUDE_TERMS
            .iter()
            .map(|&(cd, cm, cmp, cf, coefficient)| {
                let argument =
                    cd as f64 * d_r + cm as f64 * m_r + cmp as f64 * mp_r + cf as f64 * f_r;
                let eccentricity = match cm.abs() {
                    1 => e,
                    2 => e * e,
                    _ => 1.0,
                };
                coefficient as f64 * eccentricity * argument.sin()
            })
            .sum();

        let a1 = (119.75 + 131.849 * t).to_radians();
        let a2 = (53.09 + 479264.290 * t).to_radians();
        sigma_l += 3958.0 * a1.sin() + 1962.0 * (l_prime - f).to_radians().sin() + 318.0 * a2.sin();

        ((l_prime + sigma_l / 1_000_000.0).rem_euclid(360.0), t)
    }

    /// Calculate ascendant (rising sign) from date/time/location
    /// Ascendant = intersection of ecliptic with eastern horizon
    /// Uses proper astronomical calculation with obliquity of the ecliptic
    /// Based on Meeus' Astronomical Algorithms
    pub fn calculate_ascendant(date: DateTime<Utc>, coords: GeoCoordinates) -> ZodiacSign {
        let julian_day = Self::utc_to_julian_day(date);
        let lst = Self::calculate_local_sidereal_time(julian_day, coords.longitude);
        let ascendant_degrees = Self::calculate_ascendant_longitude(
            lst,
            coords.latitude,
            Self::mean_obliquity_degrees(julian_day),
        );
        let sign_index = (ascendant_degrees / 30.0).floor() as i32 % 12;
        Self::index_to_sign(sign_index)
    }

    /// Calculate Local Sidereal Time in degrees
    /// Uses Meeus' algorithm for apparent sidereal time at Greenwich
    fn calculate_local_sidereal_time(julian_day: f64, longitude: f64) -> f64 {
        // Julian centuries since J2000.0
        let t = (julian_day - 2451545.0) / 36525.0;
        let t2 = t * t;
        let t3 = t2 * t;

        // Mean sidereal time at Greenwich (degrees)
        let theta = 280.46061837 + 360.98564736629 * (julian_day - 2451545.0) + 0.000387933 * t2
            - t3 / 38710000.0;

        // Normalize to 0-360
        let theta_gst = theta.rem_euclid(360.0);

        // Local sidereal time = GST + longitude (east positive)
        let lst = theta_gst + longitude;
        lst.rem_euclid(360.0)
    }

    /// Ecliptic longitude of the ascendant, degrees `0..360`, from local
    /// sidereal time, latitude and the obliquity of date (Meeus ch. 14:
    /// `tan λ = cos θ / −(sin θ cos ε + tan φ sin ε)`, resolved to the
    /// eastern-horizon intersection by `atan2`).
    fn calculate_ascendant_longitude(lst: f64, latitude: f64, obliquity_degrees: f64) -> f64 {
        let epsilon = obliquity_degrees.to_radians();
        let lat_rad = latitude.to_radians();
        let lst_rad = lst.to_radians();
        let numerator = lst_rad.cos();
        let denominator = -(lst_rad.sin() * epsilon.cos() + lat_rad.tan() * epsilon.sin());
        numerator.atan2(denominator).to_degrees().rem_euclid(360.0)
    }

    /// Calculate elemental balance from sun, moon, and ascendant
    /// Weights: Sun = 40%, Moon = 35%, Ascendant = 25%
    pub fn calculate_elemental_balance(
        sun: ZodiacSign,
        moon: ZodiacSign,
        ascendant: ZodiacSign,
    ) -> ElementalBalance {
        let mut fire = 0.0;
        let mut earth = 0.0;
        let mut air = 0.0;
        let mut water = 0.0;

        // Sun weight: 40%
        match sun.element() {
            Element::Fire => fire += 0.40,
            Element::Earth => earth += 0.40,
            Element::Air => air += 0.40,
            Element::Water => water += 0.40,
        }

        // Moon weight: 35%
        match moon.element() {
            Element::Fire => fire += 0.35,
            Element::Earth => earth += 0.35,
            Element::Air => air += 0.35,
            Element::Water => water += 0.35,
        }

        // Ascendant weight: 25%
        match ascendant.element() {
            Element::Fire => fire += 0.25,
            Element::Earth => earth += 0.25,
            Element::Air => air += 0.25,
            Element::Water => water += 0.25,
        }

        ElementalBalance {
            fire,
            earth,
            air,
            water,
        }
    }

    /// Calculate modality balance from sun, moon, and ascendant
    /// Same weights as elemental balance
    pub fn calculate_modality_balance(
        sun: ZodiacSign,
        moon: ZodiacSign,
        ascendant: ZodiacSign,
    ) -> ModalityBalance {
        let mut cardinal = 0.0;
        let mut fixed = 0.0;
        let mut mutable = 0.0;

        // Sun weight: 40%
        match sun.modality() {
            Modality::Cardinal => cardinal += 0.40,
            Modality::Fixed => fixed += 0.40,
            Modality::Mutable => mutable += 0.40,
        }

        // Moon weight: 35%
        match moon.modality() {
            Modality::Cardinal => cardinal += 0.35,
            Modality::Fixed => fixed += 0.35,
            Modality::Mutable => mutable += 0.35,
        }

        // Ascendant weight: 25%
        match ascendant.modality() {
            Modality::Cardinal => cardinal += 0.25,
            Modality::Fixed => fixed += 0.25,
            Modality::Mutable => mutable += 0.25,
        }

        ModalityBalance {
            cardinal,
            fixed,
            mutable,
        }
    }

    /// Compute complete birth chart from birth data
    pub fn compute_birth_chart(
        birth_datetime: DateTime<Utc>,
        coordinates: GeoCoordinates,
    ) -> Result<BirthChart, String> {
        coordinates.validate()?;

        let sun = Self::calculate_sun_sign(birth_datetime);
        let moon = Self::calculate_moon_sign(birth_datetime);
        let ascendant = Self::calculate_ascendant(birth_datetime, coordinates);
        let element_balance = Self::calculate_elemental_balance(sun, moon, ascendant);
        let modality_balance = Self::calculate_modality_balance(sun, moon, ascendant);

        element_balance.validate()?;
        modality_balance.validate()?;

        Ok(BirthChart {
            sun,
            moon,
            ascendant,
            element_balance,
            modality_balance,
            coordinates,
            birth_timestamp: birth_datetime.to_rfc3339(),
        })
    }

    /// Convert index 0-11 to ZodiacSign
    fn index_to_sign(index: i32) -> ZodiacSign {
        match index.rem_euclid(12) {
            0 => ZodiacSign::Aries,
            1 => ZodiacSign::Taurus,
            2 => ZodiacSign::Gemini,
            3 => ZodiacSign::Cancer,
            4 => ZodiacSign::Leo,
            5 => ZodiacSign::Virgo,
            6 => ZodiacSign::Libra,
            7 => ZodiacSign::Scorpio,
            8 => ZodiacSign::Sagittarius,
            9 => ZodiacSign::Capricorn,
            10 => ZodiacSign::Aquarius,
            11 => ZodiacSign::Pisces,
            _ => ZodiacSign::Aries, // Should never happen with rem_euclid
        }
    }
}

/// ΔT = TT − UT in seconds at a UT Julian day, from the Espenak & Meeus
/// (2006) polynomial fits (NASA eclipse canon). Outside 1900–2150 it uses
/// their long-term parabola.
fn delta_t_seconds(julian_day: f64) -> f64 {
    let y = 2000.0 + (julian_day - 2451545.0) / 365.25;
    let long_term = |y: f64| {
        let u = (y - 1820.0) / 100.0;
        -20.0 + 32.0 * u * u
    };
    if y < 1900.0 {
        long_term(y)
    } else if y < 1920.0 {
        let t = y - 1900.0;
        -2.79 + 1.494119 * t - 0.0598939 * t.powi(2) + 0.0061966 * t.powi(3) - 0.000197 * t.powi(4)
    } else if y < 1941.0 {
        let t = y - 1920.0;
        21.20 + 0.84493 * t - 0.076100 * t.powi(2) + 0.0020936 * t.powi(3)
    } else if y < 1961.0 {
        let t = y - 1950.0;
        29.07 + 0.407 * t - t.powi(2) / 233.0 + t.powi(3) / 2547.0
    } else if y < 1986.0 {
        let t = y - 1975.0;
        45.45 + 1.067 * t - t.powi(2) / 260.0 - t.powi(3) / 718.0
    } else if y < 2005.0 {
        let t = y - 2000.0;
        63.86 + 0.3345 * t - 0.060374 * t.powi(2)
            + 0.0017275 * t.powi(3)
            + 0.000651814 * t.powi(4)
            + 0.00002373599 * t.powi(5)
    } else if y < 2050.0 {
        let t = y - 2000.0;
        62.92 + 0.32217 * t + 0.005589 * t.powi(2)
    } else if y < 2150.0 {
        long_term(y) - 0.5628 * (2150.0 - y)
    } else {
        long_term(y)
    }
}

/// Nutation in longitude Δψ in degrees at `t` Julian centuries of TT from
/// J2000.0 (Meeus ch. 22, the four-term form accurate to 0.5").
fn nutation_in_longitude_degrees(t: f64) -> f64 {
    let omega = (125.04452 - 1934.136261 * t).to_radians();
    let l_sun = (280.4665 + 36000.7698 * t).to_radians();
    let l_moon = (218.3165 + 481267.8813 * t).to_radians();
    let arcseconds =
        -17.20 * omega.sin() - 1.32 * (2.0 * l_sun).sin() - 0.23 * (2.0 * l_moon).sin()
            + 0.21 * (2.0 * omega).sin();
    arcseconds / 3600.0
}

/// Meeus Table 47.A, longitude column: multiples of (D, M, M', F) and the
/// coefficient of the sine in millionths of a degree.
const MOON_LONGITUDE_TERMS: [(i8, i8, i8, i8, i32); 60] = [
    (0, 0, 1, 0, 6288774),
    (2, 0, -1, 0, 1274027),
    (2, 0, 0, 0, 658314),
    (0, 0, 2, 0, 213618),
    (0, 1, 0, 0, -185116),
    (0, 0, 0, 2, -114332),
    (2, 0, -2, 0, 58793),
    (2, -1, -1, 0, 57066),
    (2, 0, 1, 0, 53322),
    (2, -1, 0, 0, 45758),
    (0, 1, -1, 0, -40923),
    (1, 0, 0, 0, -34720),
    (0, 1, 1, 0, -30383),
    (2, 0, 0, -2, 15327),
    (0, 0, 1, 2, -12528),
    (0, 0, 1, -2, 10980),
    (4, 0, -1, 0, 10675),
    (0, 0, 3, 0, 10034),
    (4, 0, -2, 0, 8548),
    (2, 1, -1, 0, -7888),
    (2, 1, 0, 0, -6766),
    (1, 0, -1, 0, -5163),
    (1, 1, 0, 0, 4987),
    (2, -1, 1, 0, 4036),
    (2, 0, 2, 0, 3994),
    (4, 0, 0, 0, 3861),
    (2, 0, -3, 0, 3665),
    (0, 1, -2, 0, -2689),
    (2, 0, -1, 2, -2602),
    (2, -1, -2, 0, 2390),
    (1, 0, 1, 0, -2348),
    (2, -2, 0, 0, 2236),
    (0, 1, 2, 0, -2120),
    (0, 2, 0, 0, -2069),
    (2, -2, -1, 0, 2048),
    (2, 0, 1, -2, -1773),
    (2, 0, 0, 2, -1595),
    (4, -1, -1, 0, 1215),
    (0, 0, 2, 2, -1110),
    (3, 0, -1, 0, -892),
    (2, 1, 1, 0, -810),
    (4, -1, -2, 0, 759),
    (0, 2, -1, 0, -713),
    (2, 2, -1, 0, -700),
    (2, 1, -2, 0, 691),
    (2, -1, 0, -2, 596),
    (4, 0, 1, 0, 549),
    (0, 0, 4, 0, 537),
    (4, -1, 0, 0, 520),
    (1, 0, -2, 0, -487),
    (2, 1, 0, -2, -399),
    (0, 0, 2, -2, -381),
    (1, 1, 1, 0, 351),
    (3, 0, -2, 0, -340),
    (4, 0, -3, 0, 330),
    (2, -1, 2, 0, 327),
    (0, 2, 1, 0, -323),
    (1, 1, -1, 0, 299),
    (2, 0, 3, 0, 294),
    (2, 0, -1, -2, 0),
];

/// Trait mapping from astrological factors to personality baselines
/// Per HumanReplicationSchema canon
pub struct AstrologyTraitMapper;

impl AstrologyTraitMapper {
    /// Map elemental balance to temperament traits
    /// Fire: passion, energy, extroversion
    /// Earth: stability, practicality, groundedness
    /// Air: intellect, communication, detachment
    /// Water: emotion, intuition, sensitivity
    pub fn map_elemental_to_temperament(balance: &ElementalBalance) -> ElementalTemperament {
        ElementalTemperament {
            emotional_intensity: (balance.water * 0.7 + balance.fire * 0.3) as f32,
            emotional_stability: (balance.earth * 0.8 + balance.water * 0.2) as f32,
            empathy: (balance.water * 0.6 + balance.air * 0.2 + balance.earth * 0.2) as f32,
            extroversion: (balance.fire * 0.6 + balance.air * 0.4) as f32,
            openness_to_experience: (balance.air * 0.5 + balance.fire * 0.3 + balance.water * 0.2)
                as f32,
            conscientiousness: (balance.earth * 0.7 + balance.fire * 0.2 + balance.air * 0.1)
                as f32,
        }
    }

    /// Map modality balance to behavioral patterns
    /// Cardinal: initiative, leadership, action-oriented
    /// Fixed: persistence, determination, resistance to change
    /// Mutable: adaptability, flexibility, changeable
    pub fn map_modality_to_behavior(balance: &ModalityBalance) -> ModalityBehavior {
        ModalityBehavior {
            initiative: balance.cardinal as f32,
            persistence: balance.fixed as f32,
            adaptability: balance.mutable as f32,
        }
    }

    /// Generate neurocognitive baseline from birth chart
    /// Combines sun (core self), moon (emotional nature), ascendant (outward expression)
    pub fn map_birth_chart_to_neurocognitive(chart: &BirthChart) -> NeurocognitiveBaseline {
        let temp = Self::map_elemental_to_temperament(&chart.element_balance);
        let behavior = Self::map_modality_to_behavior(&chart.modality_balance);

        // ADHD profile influenced by mutable dominance (high adaptability = distractibility risk)
        let adhd_risk = if chart.modality_balance.mutable > 0.45 {
            Some(AdhdRiskProfile {
                inattentive_tendency: (chart.modality_balance.mutable * 0.6
                    + chart.element_balance.air * 0.4) as f32,
                combined_tendency: (chart.modality_balance.mutable * 0.4
                    + chart.element_balance.fire * 0.4) as f32,
            })
        } else {
            None
        };

        // Autism spectrum risk inversely correlated with air dominance
        let autism_risk = if chart.element_balance.air > 0.35 {
            Some(AutismRiskProfile {
                level: if chart.element_balance.air > 0.50 {
                    1
                } else {
                    0
                },
                sensory_sensitivity: (chart.element_balance.water * 0.6) as f32,
            })
        } else {
            None
        };

        NeurocognitiveBaseline {
            temperament: temp,
            behavior,
            adhd_risk,
            autism_risk,
            sun_sign: chart.sun,
            moon_sign: chart.moon,
            ascendant: chart.ascendant,
        }
    }
}

/// Elemental-derived temperament traits
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElementalTemperament {
    pub emotional_intensity: f32,    // 0-1, high in fire/water
    pub emotional_stability: f32,    // 0-1, high in earth
    pub empathy: f32,                // 0-1, high in water
    pub extroversion: f32,           // 0-1, high in fire/air
    pub openness_to_experience: f32, // 0-1, high in air/fire
    pub conscientiousness: f32,      // 0-1, high in earth
}

/// Modality-derived behavioral patterns
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModalityBehavior {
    pub initiative: f32,   // 0-1, cardinal dominance
    pub persistence: f32,  // 0-1, fixed dominance
    pub adaptability: f32, // 0-1, mutable dominance
}

/// ADHD risk profile from astrological factors
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdhdRiskProfile {
    pub inattentive_tendency: f32,
    pub combined_tendency: f32,
}

/// Autism spectrum risk profile
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutismRiskProfile {
    pub level: u8, // 0-3 per HumanReplicationSchema
    pub sensory_sensitivity: f32,
}

/// Complete neurocognitive baseline from astrology
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NeurocognitiveBaseline {
    pub temperament: ElementalTemperament,
    pub behavior: ModalityBehavior,
    pub adhd_risk: Option<AdhdRiskProfile>,
    pub autism_risk: Option<AutismRiskProfile>,
    pub sun_sign: ZodiacSign,
    pub moon_sign: ZodiacSign,
    pub ascendant: ZodiacSign,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moon_longitude_matches_meeus_example_47a() {
        // Meeus example 47.a: 1992 April 12, 0h TD (JDE 2448724.5).
        let (geometric, t) = AstrologyEngine::geometric_moon_longitude_tt(2448724.5);
        assert!((geometric - 133.162655).abs() < 1e-5, "{geometric}");
        let apparent = geometric + nutation_in_longitude_degrees(t);
        assert!((apparent - 133.167265).abs() < 2e-4, "{apparent}");
    }

    #[test]
    fn delta_t_matches_observed_values() {
        // Observed ΔT: 1950 ≈ 29.1 s, 1975 ≈ 45.5 s, 2000 ≈ 63.8 s.
        for (jd, observed) in [(2433282.5, 29.1), (2442413.5, 45.5), (2451544.5, 63.8)] {
            assert!((delta_t_seconds(jd) - observed).abs() < 0.5, "{jd}");
        }
    }
    use chrono::TimeZone;

    #[test]
    fn sun_sign_calculation_march_aries() {
        let date = Utc.with_ymd_and_hms(2024, 3, 21, 12, 0, 0).unwrap();
        assert_eq!(AstrologyEngine::calculate_sun_sign(date), ZodiacSign::Aries);
    }

    #[test]
    fn sun_sign_calculation_april_taurus() {
        let date = Utc.with_ymd_and_hms(2024, 4, 25, 12, 0, 0).unwrap();
        assert_eq!(
            AstrologyEngine::calculate_sun_sign(date),
            ZodiacSign::Taurus
        );
    }

    #[test]
    fn birth_season_covers_all_months_deterministically() {
        // Mid-month 2024 in the northern hemisphere. Astronomical seasons
        // turn at the equinoxes/solstices (~20th-22nd), so the 15th of
        // March, June, September and December is still the prior season.
        let cases = [
            (1, "Winter"),
            (2, "Winter"),
            (3, "Winter"),
            (4, "Spring"),
            (5, "Spring"),
            (6, "Spring"),
            (7, "Summer"),
            (8, "Summer"),
            (9, "Summer"),
            (10, "Autumn"),
            (11, "Autumn"),
            (12, "Autumn"),
        ];
        for (month, expected) in cases {
            let date = Utc.with_ymd_and_hms(2024, month, 15, 0, 0, 0).unwrap();
            assert_eq!(
                AstrologyEngine::calculate_birth_season(date, 45.0),
                expected
            );
        }
        // Southern hemisphere seasons are reversed.
        let july = Utc.with_ymd_and_hms(2024, 7, 15, 0, 0, 0).unwrap();
        assert_eq!(
            AstrologyEngine::calculate_birth_season(july, -41.3),
            "Winter"
        );
    }

    #[test]
    fn sun_sign_follows_the_actual_cusp_for_the_year() {
        // The 2024 March equinox was 2024-03-20 03:06 UTC: the sun enters
        // Aries then, a day earlier than a fixed "March 21" table claims.
        let before = Utc.with_ymd_and_hms(2024, 3, 20, 2, 0, 0).unwrap();
        let after = Utc.with_ymd_and_hms(2024, 3, 20, 5, 0, 0).unwrap();
        assert_eq!(
            AstrologyEngine::calculate_sun_sign(before),
            ZodiacSign::Pisces
        );
        assert_eq!(
            AstrologyEngine::calculate_sun_sign(after),
            ZodiacSign::Aries
        );
    }

    #[test]
    fn lunar_phase_is_deterministic_for_same_date() {
        let date = Utc.with_ymd_and_hms(2024, 6, 1, 0, 0, 0).unwrap();
        assert_eq!(
            AstrologyEngine::calculate_lunar_phase(date),
            AstrologyEngine::calculate_lunar_phase(date)
        );
    }

    #[test]
    fn lunar_phase_new_moon_at_known_date() {
        // 2024-01-11 ~11:57 UTC was an actual new moon.
        let date = Utc.with_ymd_and_hms(2024, 1, 11, 12, 0, 0).unwrap();
        assert_eq!(AstrologyEngine::calculate_lunar_phase(date), "New Moon");
    }

    #[test]
    fn lunar_phase_full_moon_at_known_date() {
        // 2024-01-25 ~17:54 UTC was an actual full moon.
        let date = Utc.with_ymd_and_hms(2024, 1, 25, 18, 0, 0).unwrap();
        assert_eq!(AstrologyEngine::calculate_lunar_phase(date), "Full Moon");
    }

    #[test]
    fn elemental_balance_sums_to_one() {
        let balance = AstrologyEngine::calculate_elemental_balance(
            ZodiacSign::Aries,  // Fire
            ZodiacSign::Taurus, // Earth
            ZodiacSign::Gemini, // Air
        );
        let sum = balance.fire + balance.earth + balance.air + balance.water;
        assert!(
            (sum - 1.0).abs() < 0.001,
            "Elemental balance should sum to 1.0, got {}",
            sum
        );
    }

    #[test]
    fn gem_d_birth_chart_reproducible() {
        // gem-d: 1998-03-03 14:10 NZT, Pukekohe NZ
        // NZT is UTC+13 (with DST on March 3)
        let birth = Utc.with_ymd_and_hms(1998, 3, 3, 1, 10, 0).unwrap(); // 14:10 NZT = 01:10 UTC
        let coords = GeoCoordinates {
            latitude: -37.203,
            longitude: 174.938,
        };

        let chart = AstrologyEngine::compute_birth_chart(birth, coords).unwrap();

        // Sun should be Pisces (Feb 19 - Mar 20)
        assert_eq!(chart.sun, ZodiacSign::Pisces);

        // Validate elemental balance
        chart.element_balance.validate().unwrap();

        // Water dominant for Pisces sun + Cancer moon pattern
        assert!(chart.element_balance.water > 0.3);
    }

    #[test]
    fn coordinates_validation_out_of_bounds() {
        let invalid = GeoCoordinates {
            latitude: 95.0,
            longitude: 0.0,
        };
        assert!(invalid.validate().is_err());

        let invalid_lon = GeoCoordinates {
            latitude: 0.0,
            longitude: 200.0,
        };
        assert!(invalid_lon.validate().is_err());
    }

    #[test]
    fn trait_mapper_produces_valid_baselines() {
        let chart = BirthChart {
            sun: ZodiacSign::Pisces,
            moon: ZodiacSign::Cancer,
            ascendant: ZodiacSign::Scorpio,
            element_balance: ElementalBalance {
                fire: 0.0,
                earth: 0.25,
                air: 0.0,
                water: 0.75,
            },
            modality_balance: ModalityBalance {
                cardinal: 0.25,
                fixed: 0.75,
                mutable: 0.0,
            },
            coordinates: GeoCoordinates {
                latitude: -37.0,
                longitude: 175.0,
            },
            birth_timestamp: "1998-03-03T01:10:00Z".to_string(),
        };

        let baseline = AstrologyTraitMapper::map_birth_chart_to_neurocognitive(&chart);

        // High water = high empathy (0.75*0.6 + 0.25*0.2 = 0.5) and emotional intensity
        assert!(baseline.temperament.empathy >= 0.5);
        assert!(baseline.temperament.emotional_intensity > 0.5);

        // High fixed = high persistence
        assert!(baseline.behavior.persistence > 0.5);
    }

    #[test]
    fn ascendant_closed_form_matches_known_equatorial_values() {
        // At the equator, with 0° Aries culminating (LST 0°), the rising
        // ecliptic point is 0° Cancer (90°); with LST 90° it is 0° Libra.
        let rising_at_lst_0 = AstrologyEngine::calculate_ascendant_longitude(0.0, 0.0, 23.4397);
        let rising_at_lst_90 = AstrologyEngine::calculate_ascendant_longitude(90.0, 0.0, 23.4397);
        assert!((rising_at_lst_0 - 90.0).abs() < 1e-9);
        assert!((rising_at_lst_90 - 180.0).abs() < 1e-9);
    }
}
