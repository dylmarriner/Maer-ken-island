//! What the inspector panels say, worked out without a window (Phase 5
//! Task 5).
//!
//! Every figure here comes from what the backend last published -- the
//! world's clock, the terrain and a cell the backend described -- and from
//! nothing the renderer made up. The one presentation choice is which cell
//! "the view" is on: the one under the camera's focus, which is the middle
//! of the screen whatever the zoom.

use mk_island_api::{Cell, Clock, Terrain};

use crate::camera::Orbit;
use crate::scene::REGIONAL_METRES_PER_UNIT;

/// The speeds the panel offers, as the control request spells them, in
/// the order shown: real time, then multipliers, then flat out.
pub const SPEEDS: [&str; 6] = ["real", "10", "60", "360", "1440", "max"];

/// How a speed reads on its button.
pub fn speed_label(speed: &str) -> String {
    match speed {
        "real" => "real time".into(),
        "max" => "max".into(),
        n => format!("{n}x"),
    }
}

/// A moment on the island's own calendar: local years of
/// `year_length_days` local days, each `day_length_hours` long.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocalDate {
    /// Counted from 1, as people count years and days.
    pub year: u64,
    pub day: u64,
    pub hour: u32,
    pub minute: u32,
}

/// Where the clock is on the local calendar. `None` from a backend too old
/// to say how long its year is, rather than a guess at it.
pub fn local_date(clock: &Clock) -> Option<LocalDate> {
    let year_days = clock.year_length_days.filter(|d| *d > 0.0)?;
    if !(clock.days.is_finite() && clock.days >= 0.0) {
        return None;
    }
    let year = (clock.days / year_days).floor();
    let day = (clock.days - year * year_days).floor();
    let minutes = (clock.hour_of_day * 60.0).floor().max(0.0) as u64;
    Some(LocalDate {
        year: year as u64 + 1,
        day: day as u64 + 1,
        hour: (minutes / 60) as u32,
        minute: (minutes % 60) as u32,
    })
}

/// The local date as the panel prints it, with the length of the day so
/// nobody reads 30:00 as a mistake.
pub fn local_date_line(clock: &Clock) -> String {
    match local_date(clock) {
        Some(d) => format!(
            "Year {}, day {} of {:.0}, {:02}:{:02} of a {:.0}-hour day",
            d.year,
            d.day,
            clock.year_length_days.unwrap_or_default().floor(),
            d.hour,
            d.minute,
            clock.day_length_hours
        ),
        None => format!(
            "Day {:.2}, hour {:.1} of {:.0}",
            clock.days, clock.hour_of_day, clock.day_length_hours
        ),
    }
}

const EARTH_DAY_S: f64 = 86_400.0;
const EARTH_YEAR_S: f64 = 365.25 * EARTH_DAY_S;

/// The simulated time so far in Earth's units, for anyone comparing the
/// island with a calendar they know.
pub fn earth_elapsed_line(sim_time_s: u64) -> String {
    let s = sim_time_s as f64;
    if s >= EARTH_YEAR_S {
        format!(
            "{:.2} Earth years ({:.0} Earth days) since the start",
            s / EARTH_YEAR_S,
            s / EARTH_DAY_S
        )
    } else if s >= EARTH_DAY_S {
        format!("{:.2} Earth days since the start", s / EARTH_DAY_S)
    } else {
        format!("{:.1} Earth hours since the start", s / 3_600.0)
    }
}

/// The medium-grid cell under the camera's focus, or `None` when the focus
/// is off the grid.
pub fn cell_under(orbit: &Orbit, terrain: &Terrain) -> Option<(usize, usize)> {
    let (width_m, height_m) = terrain.extent_m();
    // The inverse of `scene::regional_of`: x east, z south, centred.
    let x_m = f64::from(orbit.focus.x) * REGIONAL_METRES_PER_UNIT + width_m / 2.0;
    let y_m = -f64::from(orbit.focus.z) * REGIONAL_METRES_PER_UNIT + height_m / 2.0;
    let size = f64::from(terrain.cell_size_m);
    if !(x_m >= 0.0 && y_m >= 0.0 && size > 0.0) {
        return None;
    }
    let (row, col) = ((y_m / size) as usize, (x_m / size) as usize);
    (row < terrain.rows && col < terrain.cols).then_some((row, col))
}

/// What the cell inspector prints, one line each, from a cell the backend
/// described. Fields an older backend does not send are left out rather
/// than shown as zero.
pub fn cell_lines(cell: &Cell) -> Vec<String> {
    let mut lines = vec![
        format!("Cell {}, {}", cell.row, cell.col),
        format!("{:.3}°, {:.3}°", cell.latitude, cell.longitude),
        if cell.land {
            format!(
                "Land, {:.0} m up{}",
                cell.elevation_m,
                if cell.buildable { ", buildable" } else { "" }
            )
        } else {
            format!("Sea, {:.0} m", cell.elevation_m)
        },
    ];
    if let Some(biome) = &cell.biome {
        lines.push(biome_words(biome));
    }
    if let (Some(t), Some(rain)) = (cell.mean_temperature_c, cell.annual_rain_mm) {
        lines.push(format!(
            "Climate: {t:.1} °C mean, {rain:.0} mm of rain a year"
        ));
    }
    if let Some(npp) = cell.npp_kgc_m2_yr.filter(|n| *n > 0.0) {
        lines.push(format!(
            "Plants grow {:.0} g C per m² a year",
            npp * 1_000.0
        ));
    }
    lines
}

/// `TemperateForest` as "Temperate forest".
fn biome_words(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 4);
    for (i, ch) in name.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            out.push(' ');
            out.extend(ch.to_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{regional_of, Point};

    fn clock(sim_time_s: u64) -> Clock {
        let day_s = 129_600.0;
        Clock {
            sim_time_s,
            tick: sim_time_s / 60,
            days: sim_time_s as f64 / day_s,
            hour_of_day: (sim_time_s as f64 % day_s) / 3_600.0,
            day_length_hours: 36.0,
            achieved_speed: None,
            requested_speed: "real".into(),
            year_length_days: Some(37_349_219.0 / day_s),
        }
    }

    fn terrain() -> Terrain {
        let (rows, cols) = (4, 5);
        Terrain {
            rows,
            cols,
            cell_size_m: 2_000.0,
            elevation_m: vec![10.0; rows * cols],
            land: vec![true; rows * cols],
        }
    }

    #[test]
    fn the_calendar_counts_local_days_and_years_from_one() {
        let start = local_date(&clock(0)).unwrap();
        assert_eq!(
            start,
            LocalDate {
                year: 1,
                day: 1,
                hour: 0,
                minute: 0
            }
        );
        // 30 hours and 15 minutes into the first day: past an Earth day,
        // still the same local one.
        let late = local_date(&clock(30 * 3_600 + 15 * 60)).unwrap();
        assert_eq!((late.day, late.hour, late.minute), (1, 30, 15));
        // A whole local year and a day on.
        let year_s = 37_349_219.0;
        let next = local_date(&clock((year_s + 129_600.0) as u64 + 1)).unwrap();
        assert_eq!((next.year, next.day), (2, 2));
        assert!(local_date_line(&clock(0)).contains("of a 36-hour day"));
    }

    #[test]
    fn an_older_backend_gets_the_plain_clock_not_a_guessed_calendar() {
        let mut c = clock(200_000);
        c.year_length_days = None;
        assert_eq!(local_date(&c), None);
        assert!(local_date_line(&c).starts_with("Day "));
    }

    #[test]
    fn earth_time_is_said_in_the_unit_that_reads() {
        assert_eq!(earth_elapsed_line(5_400), "1.5 Earth hours since the start");
        assert_eq!(
            earth_elapsed_line(129_600),
            "1.50 Earth days since the start"
        );
        assert!(earth_elapsed_line(2 * 31_557_600).starts_with("2.00 Earth years"));
    }

    #[test]
    fn the_speeds_are_the_planned_set_in_order() {
        assert_eq!(SPEEDS, ["real", "10", "60", "360", "1440", "max"]);
        assert_eq!(speed_label("1440"), "1440x");
        assert_eq!(speed_label("real"), "real time");
    }

    /// The cell under the view is the cell whose centre the camera was
    /// pointed at, through the same frame the scene draws in.
    #[test]
    fn the_cell_under_the_view_inverts_the_scene_frame() {
        let t = terrain();
        for (row, col) in [(0, 0), (2, 3), (3, 4)] {
            let (x, y) = ((col as f64 + 0.5) * 2_000.0, (row as f64 + 0.5) * 2_000.0);
            let mut orbit = Orbit::default();
            orbit.look_at(regional_of(&t, x, y, 10.0), 5.0);
            assert_eq!(cell_under(&orbit, &t), Some((row, col)));
        }
        let mut off = Orbit::default();
        off.look_at(Point::new(1_000.0, 0.0, 0.0), 5.0);
        assert_eq!(cell_under(&off, &t), None);
    }

    #[test]
    fn a_cell_says_only_what_the_backend_sent() {
        let mut cell = Cell {
            row: 3,
            col: 7,
            latitude: -41.2,
            longitude: 174.5,
            land: true,
            elevation_m: 312.4,
            buildable: true,
            biome: None,
            npp_kgc_m2_yr: None,
            mean_temperature_c: None,
            annual_rain_mm: None,
        };
        let old = cell_lines(&cell);
        assert_eq!(old.len(), 3);
        assert_eq!(old[2], "Land, 312 m up, buildable");
        cell.biome = Some("TemperateForest".into());
        cell.mean_temperature_c = Some(11.84);
        cell.annual_rain_mm = Some(1_234.0);
        cell.npp_kgc_m2_yr = Some(0.567);
        let new = cell_lines(&cell);
        assert_eq!(new[3], "Temperate forest");
        assert_eq!(new[4], "Climate: 11.8 °C mean, 1234 mm of rain a year");
        assert_eq!(new[5], "Plants grow 567 g C per m² a year");
    }
}
