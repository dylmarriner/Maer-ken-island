//! The food chain from the estate's farms (Phase 4b Task 7b): a year's
//! produce is the land's cited yield, however finely the year is stepped.

use mk_engine::regional::industry::farm::{Crop, Farm, FarmModel, CROPS};
use mk_engine::regional::industry::food_processing::{ProcessingModel, Product};

#[test]
fn a_year_of_farming_in_daily_steps_yields_the_cited_year() {
    let model = FarmModel::load_default().expect("the agriculture pack loads");
    let farm = Farm {
        dairy_ha: 150.0,
        crops_ha: CROPS.iter().map(|&c| (c, 4.0)).collect(),
    };

    let whole = farm.produce(&model, 1.0);
    let (mut milk, mut crops) = (0.0, vec![0.0; CROPS.len()]);
    for _ in 0..365 {
        let day = farm.produce(&model, 1.0 / 365.0);
        milk += day.milksolids_kg;
        for (sum, (_, kg)) in crops.iter_mut().zip(&day.crops_kg) {
            *sum += kg;
        }
    }

    assert!((milk - whole.milksolids_kg).abs() < 1e-6 * whole.milksolids_kg);
    for ((crop, kg), sum) in whole.crops_kg.iter().zip(&crops) {
        assert!((kg - 4.0 * model.crop_kg_per_ha(*crop)).abs() < 1e-9 * kg);
        assert!((sum - kg).abs() < 1e-6 * kg, "{crop:?}");
    }
    assert!(whole.crops_kg.iter().any(|(c, _)| *c == Crop::Potatoes));
    assert_eq!(farm.area_ha(), 150.0 + 4.0 * CROPS.len() as f64);
}

#[test]
fn the_farm_s_wheat_and_milk_become_flour_and_butter_at_fao_s_rates() {
    let farm_model = FarmModel::load_default().expect("the agriculture pack loads");
    let mills = ProcessingModel::load_default().expect("the processing pack loads");
    let farm = Farm {
        dairy_ha: 100.0,
        crops_ha: vec![(Crop::Wheat, 10.0)],
    };
    let year = farm.produce(&farm_model, 1.0);

    let wheat = year.crops_kg[0].1;
    let milled = mills.mill(Crop::Wheat, wheat);
    let flour = milled
        .iter()
        .find(|p| p.0 == Product::WheatFlour)
        .unwrap()
        .1;
    assert!((flour - 0.78 * wheat).abs() < 1e-6 * wheat);
    assert!((milled.iter().map(|p| p.1).sum::<f64>() - wheat).abs() < 1e-6 * wheat);

    assert!((year.milk_kg - 276.0 * 4_593.0).abs() < 1e-6 * year.milk_kg);
    let butter = mills.dairy(Product::Butter, year.milk_kg);
    assert!((butter - 0.061 * year.milk_kg).abs() < 1e-6 * butter);
}
