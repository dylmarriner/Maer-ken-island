//! Body growth from birth to adulthood.
//!
//! A person's sampled body is their *adult* body. Growth is expressed as the
//! fraction of that adult size reached at each age, taken from the WHO
//! median length/height-for-age and weight-for-age tables
//! (`fixtures/reference/humans/life_history.json`, WHO 2006/2007), and applied
//! as a ratio from one age to the next, so no extra state is stored and an
//! adult (age 18 or more, fraction exactly 1) is never changed.
//!
//! Limits: medians only, so no individual growth spurts or catch-up growth;
//! weight past 5 years follows height with a fitted power law (the pack's
//! weight table ends at 5 years); the adult reference weights are derived
//! from the 18-year median BMI (about 20.5 boys, 20.4 girls) and height.

use mk_core::human::BiologicalSex;

/// Age at which the sampled adult size is reached.
pub const ADULT_AGE_YEARS: f64 = 18.0;

/// (age in years, median height in cm): WHO 2006 / 2007.
const HEIGHT_BOYS: [(f64, f64); 6] = [
    (0.0, 49.9),
    (1.0, 75.7),
    (2.0, 87.8),
    (5.0, 110.0),
    (10.0, 137.8),
    (18.0, 176.1),
];
const HEIGHT_GIRLS: [(f64, f64); 6] = [
    (0.0, 49.1),
    (1.0, 74.0),
    (2.0, 86.4),
    (5.0, 109.4),
    (10.0, 138.6),
    (18.0, 163.1),
];
/// (age in years, median weight in kg), birth to 5 years: WHO 2006.
const WEIGHT_BOYS: [(f64, f64); 4] = [(0.0, 3.3), (1.0, 9.6), (2.0, 12.2), (5.0, 18.3)];
const WEIGHT_GIRLS: [(f64, f64); 4] = [(0.0, 3.2), (1.0, 8.9), (2.0, 11.5), (5.0, 18.2)];
/// Adult reference weight: median BMI at 18 years times the 18-year height squared.
const ADULT_WEIGHT_BOYS_KG: f64 = 20.5 * 1.761 * 1.761;
const ADULT_WEIGHT_GIRLS_KG: f64 = 20.4 * 1.631 * 1.631;

fn interpolate(table: &[(f64, f64)], age: f64) -> f64 {
    let age = age.max(table[0].0);
    for pair in table.windows(2) {
        let ((a0, v0), (a1, v1)) = (pair[0], pair[1]);
        if age <= a1 {
            return v0 + (v1 - v0) * (age - a0) / (a1 - a0);
        }
    }
    table[table.len() - 1].1
}

/// One sex's growth tables.
struct Tables {
    height: &'static [(f64, f64)],
    weight: &'static [(f64, f64)],
    adult_weight_kg: f64,
}

fn tables(sex: BiologicalSex) -> Tables {
    match sex {
        BiologicalSex::Male => Tables {
            height: &HEIGHT_BOYS,
            weight: &WEIGHT_BOYS,
            adult_weight_kg: ADULT_WEIGHT_BOYS_KG,
        },
        _ => Tables {
            height: &HEIGHT_GIRLS,
            weight: &WEIGHT_GIRLS,
            adult_weight_kg: ADULT_WEIGHT_GIRLS_KG,
        },
    }
}

/// The adult reference weight (kg) the weight fractions are relative to.
pub fn adult_reference_weight_kg(sex: BiologicalSex) -> f64 {
    tables(sex).adult_weight_kg
}

/// Height at `age_years` as a fraction of adult height (1 from 18 years).
pub fn height_fraction(sex: BiologicalSex, age_years: f64) -> f64 {
    if age_years >= ADULT_AGE_YEARS {
        return 1.0;
    }
    let height = tables(sex).height;
    interpolate(height, age_years) / height[height.len() - 1].1
}

/// Weight at `age_years` as a fraction of adult weight (1 from 18 years).
pub fn weight_fraction(sex: BiologicalSex, age_years: f64) -> f64 {
    if age_years >= ADULT_AGE_YEARS {
        return 1.0;
    }
    let Tables {
        weight,
        adult_weight_kg: adult_kg,
        ..
    } = tables(sex);
    if age_years <= 5.0 {
        return interpolate(weight, age_years) / adult_kg;
    }
    // Past 5 years weight follows height by a power law fitted so that the
    // curve is continuous at 5 years and reaches 1 at 18.
    let g5 = weight[weight.len() - 1].1 / adult_kg;
    let h5 = height_fraction(sex, 5.0);
    let exponent = (1.0 / g5).ln() / (1.0 / h5).ln();
    (g5 * (height_fraction(sex, age_years) / h5).powf(exponent)).min(1.0)
}

/// The factor by which height and weight change when a person ages from
/// `from_years` to `to_years`: `(height, weight)`. Exactly `(1, 1)` once both
/// ages are adult.
pub fn growth_ratio(sex: BiologicalSex, from_years: f64, to_years: f64) -> (f64, f64) {
    if from_years >= ADULT_AGE_YEARS && to_years >= ADULT_AGE_YEARS {
        return (1.0, 1.0);
    }
    (
        height_fraction(sex, to_years) / height_fraction(sex, from_years),
        weight_fraction(sex, to_years) / weight_fraction(sex, from_years),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fractions_reproduce_the_who_tables_for_a_median_adult() {
        for (sex, heights, weights, adult_kg) in [
            (
                BiologicalSex::Male,
                &HEIGHT_BOYS[..],
                &WEIGHT_BOYS[..],
                ADULT_WEIGHT_BOYS_KG,
            ),
            (
                BiologicalSex::Female,
                &HEIGHT_GIRLS[..],
                &WEIGHT_GIRLS[..],
                ADULT_WEIGHT_GIRLS_KG,
            ),
        ] {
            let adult_cm = heights[heights.len() - 1].1;
            for &(age, cm) in heights {
                let got = adult_cm * height_fraction(sex, age);
                assert!(
                    (got - cm).abs() / cm < 1e-9,
                    "{sex:?} height at {age}: {got} vs {cm}"
                );
            }
            for &(age, kg) in weights {
                let got = adult_kg * weight_fraction(sex, age);
                assert!(
                    (got - kg).abs() / kg < 1e-9,
                    "{sex:?} weight at {age}: {got} vs {kg}"
                );
            }
        }
    }

    #[test]
    fn weight_is_continuous_monotone_and_ends_at_adult_size() {
        for sex in [BiologicalSex::Male, BiologicalSex::Female] {
            let mut last = (0.0, 0.0);
            for step in 0..=360 {
                let age = f64::from(step) * 0.05;
                let (h, w) = (height_fraction(sex, age), weight_fraction(sex, age));
                assert!(h >= last.0 && w >= last.1, "{sex:?} not monotone at {age}");
                assert!(h <= 1.0 && w <= 1.0);
                last = (h, w);
            }
            assert_eq!(weight_fraction(sex, 18.0), 1.0);
            // A ten-year-old is roughly 30-35 kg when the adult is 55-65 kg.
            let ten = weight_fraction(sex, 10.0)
                * if sex == BiologicalSex::Male {
                    63.6
                } else {
                    54.3
                };
            assert!(
                (26.0..=38.0).contains(&ten),
                "{sex:?} 10-year-old weighs {ten:.1} kg"
            );
        }
    }

    #[test]
    fn adults_are_never_changed() {
        assert_eq!(growth_ratio(BiologicalSex::Male, 25.0, 25.001), (1.0, 1.0));
        assert_eq!(growth_ratio(BiologicalSex::Female, 18.0, 40.0), (1.0, 1.0));
    }
}
