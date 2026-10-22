//! The timber chain from plantation to house (Phase 4b Task 7b): harvest
//! never exceeds regrowth, and every tonne of carbon felled is in a mill
//! residue, the builder's stock or a house's frame.

use mk_engine::regional::industry::construction::{Builder, HouseModel};
use mk_engine::regional::industry::forestry::{Plantation, PlantationModel};
use mk_engine::regional::industry::sawmill::SawmillModel;

#[test]
fn a_century_of_forest_mill_and_builder_loses_no_carbon_and_overcuts_nothing() {
    let forest_model = PlantationModel::load_default().expect("the forestry pack loads");
    let mill = SawmillModel::load_default().expect("the construction pack loads");
    let house = HouseModel::load_default().expect("the construction pack loads");
    let mut forest = Plantation::even_aged_classes(&forest_model, 500.0);
    let mut builder = Builder::default();
    let mut residues_c = 0.0;

    let years = 100;
    let (mut logs_m3, mut sawn_m3, mut felled_c, mut products_c) = (0.0, 0.0, 0.0, 0.0);
    for year in 1..=years {
        let cut = forest.clearfell(&forest_model, year as f64);
        let carbon = cut * forest_model.carbon_t_per_m3();
        let out = mill.saw(cut);
        logs_m3 += cut;
        sawn_m3 += out[0];
        felled_c += carbon;
        let products = mill.carbon(carbon);
        products_c += products.iter().sum::<f64>();
        residues_c += products[1..].iter().sum::<f64>();
        builder.receive(out[0], products[0]);
        builder.build(&house);
    }

    let grown = forest.area_ha() * forest_model.mean_annual_increment() * years as f64;
    println!("{logs_m3:.0} m³ of logs, {sawn_m3:.0} m³ sawn, {felled_c:.0} t C");
    assert!(logs_m3 > 0.0 && logs_m3 <= grown);
    assert!((sawn_m3 * mill.roundwood_per_sawnwood - logs_m3).abs() < 1e-6 * logs_m3);
    assert!((products_c - felled_c).abs() < 1e-9 * felled_c);
    println!("{} houses framed", builder.houses);
    assert!(builder.houses > 0);
    let held = residues_c + builder.timber_carbon_t + builder.houses_carbon_t;
    assert!((held - felled_c).abs() < 1e-9 * felled_c);
}
