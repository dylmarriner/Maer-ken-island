//! Water at home (realism programme R1, `WORK_REMAINING.md` section 8).
//!
//! The founders' house takes its water as a New Zealand house off the mains
//! does: rain on the roof runs into a tank, and the household draws on the
//! tank. Before this, a founder who drank took water the moment they drank
//! from any wet cell within reach of the household's stream, kilometres
//! away, and nobody washed, flushed or cooked with any.
//!
//! - **Inflow** follows Harrison Grierson's model for Greater Wellington
//!   (2011, equation 1): each day the roof delivers `(rain − loss) × area`,
//!   the loss (first flush, spouting, inefficiency) taken from the packs.
//!   Rain is counted over a rolling 24 hours, the period the source's loss
//!   is per.
//! - **Use** is ESR's Table 1 (2021) per person per day, less the drinking
//!   row: what each body drinks is booked from what it turns over, as
//!   before, but now from the tank. When the tank holds less than one more
//!   day of normal use the household cuts back to ESR's emergency ration.
//! - **Size** is not a guessed volume. Like the micro-hydro, it is chosen
//!   from the island's own climate at the house: the storage that carries
//!   the household's normal use through the driest run of the estate's
//!   climatology (sequent-peak over two years of its monthly means),
//!   installed as whole tanks of the largest common size (BRANZ: up to
//!   30,000 L each) and never fewer than one. Monthly means hide the dry
//!   days between storms, which is what the one-tank floor is for; a
//!   deviation row records it.

use serde::{Deserialize, Serialize};

use crate::validation::{ReferenceDomain, ReferenceLibrary};

/// Seconds in the period the roof's loss is stated per (an Earth day).
const LOSS_PERIOD_S: f64 = 86_400.0;

/// The figures the household runs on, read from the packs.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HouseholdWaterSpec {
    /// Normal use per person per day, drinking excluded (L, = kg).
    pub use_l_per_person_day: f64,
    /// Normal use per person per day, drinking included: what the tank is
    /// sized for (L).
    pub total_l_per_person_day: f64,
    /// What a household cuts back to when water is short (L/person/day).
    pub rationed_l_per_person_day: f64,
    /// Rain a roof loses per day before the tank (mm).
    pub roof_loss_mm_day: f64,
    /// The largest common tank, the unit tanks are installed in (L).
    pub tank_unit_l: f64,
}

impl HouseholdWaterSpec {
    pub fn from_reference(library: &ReferenceLibrary) -> Result<Self, String> {
        let row = |key: &str, label: &str| {
            library
                .item(ReferenceDomain::Hydrology, key)
                .and_then(|i| i.table())
                .and_then(|t| t.get(label, "litres"))
                .ok_or_else(|| format!("{key}/{label}"))
        };
        let total = row("household_water_use_per_person", "total")?;
        let drinking = row("household_water_use_per_person", "drinking")?;
        let loss = library
            .item(ReferenceDomain::Hydrology, "roof_water_first_flush_loss")
            .and_then(|i| i.point())
            .ok_or_else(|| "roof_water_first_flush_loss".to_string())?;
        Ok(Self {
            use_l_per_person_day: total - drinking,
            total_l_per_person_day: total,
            rationed_l_per_person_day: row("emergency_water_per_person", "rationed")?,
            roof_loss_mm_day: loss,
            tank_unit_l: library
                .item(
                    ReferenceDomain::Hydrology,
                    "largest_standard_rainwater_tank",
                )
                .and_then(|i| i.point())
                .ok_or_else(|| "largest_standard_rainwater_tank".to_string())?,
        })
    }
}

/// The tanks to install (kg): [`storage_needed_kg`] in whole tanks of the
/// spec's unit, never fewer than one.
pub fn installed_capacity_kg(spec: &HouseholdWaterSpec, needed_kg: f64) -> f64 {
    let tanks = (needed_kg / spec.tank_unit_l).ceil().max(1.0);
    tanks * spec.tank_unit_l
}

/// The storage (kg) that never runs dry under `monthly_rain_mm_day`
/// for `people` using the spec's full amount: the sequent-peak method,
/// run over two cycles of the year so a deficit that straddles its end is
/// seen. `days_per_bin` is the length of one climatology bin.
pub fn storage_needed_kg(
    spec: &HouseholdWaterSpec,
    roof_area_m2: f64,
    people: usize,
    monthly_rain_mm_day: &[f64],
    days_per_bin: f64,
) -> f64 {
    let demand = spec.total_l_per_person_day * people as f64 * days_per_bin;
    let mut deficit: f64 = 0.0;
    let mut largest: f64 = 0.0;
    for rain in monthly_rain_mm_day.iter().chain(monthly_rain_mm_day) {
        let inflow = (rain - spec.roof_loss_mm_day).max(0.0) * roof_area_m2 * days_per_bin;
        deficit = (deficit + demand - inflow).max(0.0);
        largest = largest.max(deficit);
    }
    largest
}

/// The house's roof, tank and what is in it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HouseholdWater {
    pub spec: HouseholdWaterSpec,
    /// Plan area of the house roof (m²).
    pub roof_area_m2: f64,
    pub capacity_kg: f64,
    pub stored_kg: f64,
    /// Rain on the roof in the current 24-hour period (mm), and how much of
    /// it has already been passed to the tank (mm).
    pub rain_in_period_mm: f64,
    pub credited_in_period_mm: f64,
    pub period_elapsed_s: f64,
    /// Running totals, for the dashboard and the tests (kg).
    pub caught_kg: f64,
    pub overflowed_kg: f64,
    pub used_kg: f64,
    /// Simulated seconds spent rationed.
    pub rationed_s: f64,
}

impl HouseholdWater {
    pub fn new(spec: HouseholdWaterSpec, roof_area_m2: f64, capacity_kg: f64) -> Self {
        Self {
            spec,
            roof_area_m2,
            capacity_kg,
            stored_kg: 0.0,
            rain_in_period_mm: 0.0,
            credited_in_period_mm: 0.0,
            period_elapsed_s: 0.0,
            caught_kg: 0.0,
            overflowed_kg: 0.0,
            used_kg: 0.0,
            rationed_s: 0.0,
        }
    }

    /// Rain on the roof over `dt_s` at `rain_mm_day`: the kg the roof
    /// delivers this step, before the tank's room is considered. The
    /// period's loss is taken from its first rain, as a diverter does.
    pub fn roof_delivers_kg(&mut self, rain_mm_day: f64, dt_s: f64) -> f64 {
        if self.period_elapsed_s >= LOSS_PERIOD_S {
            self.period_elapsed_s = 0.0;
            self.rain_in_period_mm = 0.0;
            self.credited_in_period_mm = 0.0;
        }
        self.period_elapsed_s += dt_s;
        self.rain_in_period_mm += rain_mm_day.max(0.0) * dt_s / LOSS_PERIOD_S;
        let deliverable = (self.rain_in_period_mm - self.spec.roof_loss_mm_day).max(0.0);
        let new_mm = deliverable - self.credited_in_period_mm;
        self.credited_in_period_mm = deliverable;
        new_mm.max(0.0) * self.roof_area_m2
    }

    /// Room left in the tank (kg).
    pub fn room_kg(&self) -> f64 {
        (self.capacity_kg - self.stored_kg).max(0.0)
    }

    /// Book `kg` into the tank, of `delivered_kg` the roof brought; the rest
    /// overflowed.
    pub fn fill(&mut self, kg: f64, delivered_kg: f64) {
        self.stored_kg += kg;
        self.caught_kg += kg;
        self.overflowed_kg += (delivered_kg - kg).max(0.0);
    }

    /// What `people` use over `dt_s` besides drinking (kg), and whether
    /// they are rationing: they cut back once the tank holds less than one
    /// more day of normal use.
    pub fn household_use_kg(&self, people: usize, dt_s: f64) -> (f64, bool) {
        let day = self.spec.use_l_per_person_day * people as f64;
        let rationed = self.stored_kg < day;
        let per_day = if rationed {
            self.spec.rationed_l_per_person_day * people as f64
        } else {
            day
        };
        (per_day * dt_s / LOSS_PERIOD_S, rationed)
    }

    /// Take up to `kg` out of the tank; returns what was taken.
    pub fn draw(&mut self, kg: f64) -> f64 {
        let taken = kg.max(0.0).min(self.stored_kg);
        self.stored_kg -= taken;
        self.used_kg += taken;
        taken
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec() -> HouseholdWaterSpec {
        HouseholdWaterSpec {
            use_l_per_person_day: 278.0,
            total_l_per_person_day: 280.0,
            rationed_l_per_person_day: 20.0,
            roof_loss_mm_day: 1.0,
            tank_unit_l: 30_000.0,
        }
    }

    #[test]
    fn the_packs_give_the_household_its_figures() {
        let library = ReferenceLibrary::load(&crate::validation::default_reference_dir()).unwrap();
        let s = HouseholdWaterSpec::from_reference(&library).unwrap();
        assert_eq!(s, spec());
    }

    /// Equation 1 of the source, over a whole day in many small steps: the
    /// first millimetre of the day is lost, the rest arrives.
    #[test]
    fn a_day_of_rain_delivers_rain_less_the_loss_times_the_roof() {
        let mut w = HouseholdWater::new(spec(), 150.0, 1e9);
        let mut delivered = 0.0;
        for _ in 0..1_440 {
            delivered += w.roof_delivers_kg(6.0, 60.0);
        }
        assert!(
            (delivered - (6.0 - 1.0) * 150.0).abs() < 1e-6,
            "{delivered}"
        );
        // A day of drizzle under the loss delivers nothing.
        let mut w = HouseholdWater::new(spec(), 150.0, 1e9);
        let mut d = 0.0;
        for _ in 0..1_440 {
            d += w.roof_delivers_kg(0.8, 60.0);
        }
        assert_eq!(d, 0.0);
    }

    #[test]
    fn a_full_tank_overflows_and_a_low_one_is_rationed() {
        let mut w = HouseholdWater::new(spec(), 150.0, 1_000.0);
        let room = w.room_kg();
        w.fill(room, 1_500.0);
        assert_eq!(w.stored_kg, 1_000.0);
        assert_eq!(w.overflowed_kg, 500.0);
        // 1,000 L is more than a day of one person's 278 L.
        let (_, rationed) = w.household_use_kg(1, 60.0);
        assert!(!rationed);
        let (_, rationed) = w.household_use_kg(4, 60.0);
        assert!(rationed, "1,000 L is under a day of four people's 1,112 L");
        let (per_minute, _) = w.household_use_kg(4, 60.0);
        assert!((per_minute - 4.0 * 20.0 / 1_440.0).abs() < 1e-12);
    }

    #[test]
    fn a_tank_is_sized_for_the_driest_run_and_not_for_a_wet_climate() {
        // Wet all year: the roof always out-supplies two people.
        let wet = [10.0; 12];
        assert_eq!(storage_needed_kg(&spec(), 150.0, 2, &wet, 24.0), 0.0);
        // Three dry bins in a row: the tank carries their whole deficit.
        let mut dry = [10.0; 12];
        dry[5] = 0.0;
        dry[6] = 0.0;
        dry[7] = 0.0;
        let need = 3.0 * 24.0 * 2.0 * 280.0;
        assert!((storage_needed_kg(&spec(), 150.0, 2, &dry, 24.0) - need).abs() < 1e-6);
        // A dry spell across the year's end is seen too.
        let mut wrap = [10.0; 12];
        wrap[11] = 0.0;
        wrap[0] = 0.0;
        let need = 2.0 * 24.0 * 2.0 * 280.0;
        assert!((storage_needed_kg(&spec(), 150.0, 2, &wrap, 24.0) - need).abs() < 1e-6);
    }

    #[test]
    fn tanks_come_whole_and_there_is_always_one() {
        assert_eq!(installed_capacity_kg(&spec(), 0.0), 30_000.0);
        assert_eq!(installed_capacity_kg(&spec(), 30_000.0), 30_000.0);
        assert_eq!(installed_capacity_kg(&spec(), 30_001.0), 60_000.0);
    }
}
