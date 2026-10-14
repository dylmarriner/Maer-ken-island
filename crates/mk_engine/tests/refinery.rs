//! Phase 4b Task 5: a topping refinery (`regional::industry::refinery`).

use mk_core::flux::{FluxKind, Ledger, Reservoir};
use mk_engine::conservation::O2_PER_CARBON;
use mk_engine::regional::industry::petroleum::{PetroleumField, PetroleumModel, BARREL_M3};
use mk_engine::regional::industry::refinery::{
    products_kgc, refine, Product, RefineryModel, PRODUCTS,
};
use mk_engine::validation::{default_reference_dir, ReferenceDomain, ReferenceLibrary};

fn model() -> RefineryModel {
    RefineryModel::load_default().unwrap()
}

fn net(ledger: &Ledger, reservoir: Reservoir, kind: FluxKind) -> f64 {
    ledger
        .entries()
        .iter()
        .filter(|e| e.kind == kind)
        .map(|e| {
            if e.sink == reservoir {
                e.amount
            } else if e.source == reservoir {
                -e.amount
            } else {
                0.0
            }
        })
        .sum()
}

#[test]
fn cuts_are_products_by_the_fuel_specifications() {
    let m = model();
    for cut in &m.cuts {
        let product = m.product_of(cut);
        println!(
            "{:<40} {:>6.2}% -> {product:?}",
            cut.label,
            cut.weight_fraction * 100.0
        );
        match cut.end_c {
            None => assert_eq!(product, Product::RefineryGas),
            Some(end) if end <= 210.0 => assert_eq!(product, Product::Naphtha),
            Some(end) if end <= 360.0 => assert_eq!(product, Product::Distillate),
            Some(_) => assert_eq!(product, Product::Residue),
        }
    }
    let shares = m.distilled_fractions();
    assert!((shares.iter().sum::<f64>() - 1.0).abs() < 1e-12);
}

#[test]
fn mass_and_carbon_close_and_the_fuel_is_burned() {
    let m = model();
    let crude_m3 = 1_000.0 * BARREL_M3;
    let mut ledger = Ledger::new();
    let run = refine(&m, crude_m3, &mut ledger);
    let made: f64 = run.products_t.iter().sum();
    assert!((made + run.fuel_gas_t + run.fuel_residue_t - run.crude_t).abs() < 1e-9 * run.crude_t);
    let crude_kgc = run.crude_t * 1_000.0 * m.crude_carbon_per_kg;
    assert!((products_kgc(&m, &run) + run.burned_kgc - crude_kgc).abs() < 1e-6 * crude_kgc);
    assert!(
        (-net(&ledger, Reservoir::MaterialCarbon, FluxKind::Carbon) - run.burned_kgc).abs() < 1e-9
    );
    assert!(
        (net(&ledger, Reservoir::AtmosCO2, FluxKind::Oxygen) - run.burned_kgc * O2_PER_CARBON)
            .abs()
            < 1e-6
    );
    // One factor closes IPCC's carbon contents onto the crude's.
    println!("carbon scale {:.4}", run.carbon_scale);
    assert!((0.97..=1.03).contains(&run.carbon_scale));
    for p in PRODUCTS {
        println!("{p:?}: {:.1} t", run.product_t(p));
    }
}

#[test]
fn the_fuel_and_its_co2_are_what_refineries_use() {
    let m = model();
    let barrels = 1_000.0;
    let run = refine(&m, barrels * BARREL_M3, &mut Ledger::new());
    let fuel_share = (run.fuel_gas_t + run.fuel_residue_t) / run.crude_t;
    let co2_kg_per_bbl = run.burned_kgc * (1.0 + O2_PER_CARBON) / barrels;
    println!(
        "fuel {:.1}% of the crude by weight, {co2_kg_per_bbl:.1} kg CO2 a barrel",
        fuel_share * 100.0
    );
    let packs = ReferenceLibrary::load(&default_reference_dir()).unwrap();
    let industry = |key: &str| packs.item(ReferenceDomain::Industry, key).unwrap();
    // Downey: refinery fuel 8-12% of the crude by volume (illustrative);
    // compared by weight here, which differs from volume by the fuels'
    // densities -- an order-of-magnitude check.
    let (lo, hi) = industry("refinery_fuel_use_vol_pct").range().unwrap();
    assert!(
        (lo / 100.0..=hi / 100.0).contains(&fuel_share),
        "fuel {fuel_share}"
    );
    // Jing et al. 2020: 13.9-62.1 kg CO2e a barrel across countries.
    let (lo, hi) = industry("refinery_co2_per_bbl_global").range().unwrap();
    assert!((lo..=hi).contains(&co2_kg_per_bbl), "{co2_kg_per_bbl}");
}

#[test]
fn crude_from_a_field_is_refined_with_its_carbon_accounted() {
    let petroleum = PetroleumModel::load_default().unwrap();
    let refinery = model();
    // A field the size of Kapuni, made by hand: the test needs no island.
    let kapuni = petroleum
        .analogues
        .iter()
        .find(|a| a.name == "Kapuni")
        .unwrap();
    let mut field = PetroleumField {
        deposit_id: 0,
        cell: (0, 0),
        analogue: kapuni.name.clone(),
        ultimate_oil_m3: kapuni.ultimate_oil_m3,
        ultimate_gas_gj: kapuni.ultimate_gas_gj,
        produced_oil_m3: 0.0,
        produced_gas_gj: 0.0,
        tank_oil_m3: 0.0,
        tank_gas_gj: 0.0,
        leaked_kgc: 0.0,
    };
    let mut ledger = Ledger::new();
    field.produce(1.0, &petroleum, &mut ledger);
    let oil_kgc_before = field.tank_oil_m3 * petroleum.crude_carbon_kg_per_m3();
    let crude = field.take_oil(f64::INFINITY);
    assert_eq!(field.tank_oil_m3, 0.0);
    let run = refine(&refinery, crude, &mut ledger);
    let left = products_kgc(&refinery, &run);
    assert!((left + run.burned_kgc - oil_kgc_before).abs() < 1e-6 * oil_kgc_before);
    // The whole chain's carbon balances: crust out = tanks (gas) + products
    // + CO2 + methane.
    let total: f64 = [
        Reservoir::CrustCarbon,
        Reservoir::MaterialCarbon,
        Reservoir::AtmosCO2,
        Reservoir::AtmosCH4,
    ]
    .iter()
    .map(|r| net(&ledger, *r, FluxKind::Carbon))
    .sum();
    assert!(total.abs() < 1e-3);
}
