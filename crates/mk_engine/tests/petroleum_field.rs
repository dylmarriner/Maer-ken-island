//! Phase 4b Task 4: oil and gas fields (`regional::industry::petroleum`).

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use mk_core::canon::CanonLocked;
use mk_core::flux::{FluxKind, Ledger, Reservoir};
use mk_engine::regional::deposits::DepositKind;
use mk_engine::regional::industry::petroleum::{PetroleumField, PetroleumModel, BARREL_M3};
use mk_engine::regional::physical::RegionalPhysicalState;
use mk_island::{IslandDomain, IslandProfile};

struct Base {
    seed: [u8; 32],
    physical: RegionalPhysicalState,
    model: PetroleumModel,
}

fn repo(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

fn base() -> &'static Base {
    static BASE: OnceLock<Base> = OnceLock::new();
    BASE.get_or_init(|| {
        let canon = Arc::new(CanonLocked::load(&repo("fixtures/island/canon.json")).unwrap());
        let profile = IslandProfile::load(&repo("fixtures/island/default_profile.json")).unwrap();
        let seed = profile.seed_bytes().unwrap();
        let domain = IslandDomain::from_profile(profile).unwrap();
        let physical = RegionalPhysicalState::bootstrap(&canon, &domain, seed).unwrap();
        Base {
            seed,
            physical,
            model: PetroleumModel::load_default().unwrap(),
        }
    })
}

fn fields() -> Vec<PetroleumField> {
    let b = base();
    PetroleumField::survey(&b.physical.geophysics.deposits, &b.model, &b.seed)
}

/// Net flow of carbon into `reservoir` in `ledger` (kg C).
fn net_carbon(ledger: &Ledger, reservoir: Reservoir) -> f64 {
    ledger
        .entries()
        .iter()
        .filter(|e| e.kind == FluxKind::Carbon)
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
fn the_model_is_the_packs_numbers() {
    let m = &base().model;
    // Höök et al. 2009, Table 2: onshore giants' median decline.
    assert!((m.decline_per_year - 0.044).abs() < 1e-12);
    // IPCC 2006: 42.3 GJ/t x 20.0 kg C/GJ; gas 15.3 kg C/GJ.
    assert!((m.crude_carbon_per_kg - 0.846).abs() < 1e-12);
    assert_eq!(m.gas_carbon_kg_per_gj, 15.3);
    // Alvarez et al. 2018.
    assert!((m.methane_loss_fraction - 0.023).abs() < 1e-12);
    // New Zealand's nine fields with oil, all named once.
    assert_eq!(m.analogues.len(), 9);
    let maui = m.analogues.iter().find(|a| a.name == "Maui").unwrap();
    assert!((maui.ultimate_oil_m3 - 227e6 * BARREL_M3).abs() < 1.0);
    assert!((maui.ultimate_gas_gj - 4_331e6).abs() < 1.0);
    let tui = m.analogues.iter().find(|a| a.name == "Tui").unwrap();
    assert_eq!(tui.ultimate_gas_gj, 0.0);
}

#[test]
fn every_petroleum_deposit_is_one_field_sized_like_a_real_one() {
    let b = base();
    let petroleum = b
        .physical
        .geophysics
        .deposits
        .iter()
        .filter(|d| d.kind == DepositKind::Petroleum)
        .count();
    let fields = fields();
    println!("{petroleum} petroleum deposits");
    assert!(petroleum > 0, "the owner's island has petroleum");
    assert_eq!(fields.len(), petroleum);
    for f in &fields {
        let a = b
            .model
            .analogues
            .iter()
            .find(|a| a.name == f.analogue)
            .unwrap();
        assert_eq!(f.ultimate_oil_m3, a.ultimate_oil_m3);
        assert_eq!(f.ultimate_gas_gj, a.ultimate_gas_gj);
        println!(
            "field {} at {:?}: like {}, {:.0} Mbbl oil, {:.0} PJ gas",
            f.deposit_id,
            f.cell,
            f.analogue,
            f.ultimate_oil_m3 / BARREL_M3 / 1e6,
            f.ultimate_gas_gj / 1e6
        );
    }
    assert_eq!(
        fields,
        self::fields(),
        "the same seed gives the same fields"
    );
}

#[test]
fn production_follows_the_decline_whatever_the_step() {
    let m = &base().model;
    let field = fields().into_iter().next().unwrap();
    let years = 10.0;
    let (mut once, mut monthly) = (field.clone(), field.clone());
    once.produce(years, m, &mut Ledger::new());
    for _ in 0..120 {
        monthly.produce(years / 120.0, m, &mut Ledger::new());
    }
    let expected = field.ultimate_oil_m3 * (1.0 - (-m.decline_per_year * years).exp());
    for f in [&once, &monthly] {
        assert!((f.produced_oil_m3 - expected).abs() <= 1e-9 * expected.max(1.0));
    }
    let (oil_rate, gas_rate) = once.rates_per_year(m);
    assert!((oil_rate - m.decline_per_year * once.remaining_oil_m3()).abs() < 1e-6);
    assert!((gas_rate - m.decline_per_year * once.remaining_gas_gj()).abs() < 1e-6);
}

#[test]
fn a_field_never_gives_more_than_it_holds() {
    let m = &base().model;
    for mut f in fields() {
        for _ in 0..100 {
            f.produce(50.0, m, &mut Ledger::new());
        }
        assert!(f.produced_oil_m3 <= f.ultimate_oil_m3);
        assert!(f.produced_gas_gj <= f.ultimate_gas_gj);
        assert!(f.remaining_oil_m3() <= 1e-6 * f.ultimate_oil_m3.max(1.0));
    }
}

#[test]
fn carbon_comes_out_of_the_crust_and_some_leaks_as_methane() {
    let m = &base().model;
    let mut field = fields()
        .into_iter()
        .max_by(|a, b| a.ultimate_gas_gj.total_cmp(&b.ultimate_gas_gj))
        .unwrap();
    let mut ledger = Ledger::new();
    let p = field.produce(1.0, m, &mut ledger);
    let (crust, material, methane) = (
        net_carbon(&ledger, Reservoir::CrustCarbon),
        net_carbon(&ledger, Reservoir::MaterialCarbon),
        net_carbon(&ledger, Reservoir::AtmosCH4),
    );
    assert!(
        (crust + material + methane).abs() < 1e-6,
        "the ledger balances"
    );
    assert!((material - field.tank_kgc(m)).abs() <= 1e-9 * material);
    assert!((material - p.tanked_kgc).abs() <= 1e-9 * material);
    let gas_kgc = p.gas_gj * m.gas_carbon_kg_per_gj;
    if gas_kgc > 0.0 {
        assert!((methane / gas_kgc - m.methane_loss_fraction).abs() < 1e-12);
    }
    println!(
        "first year: {:.0} m3 oil, {:.2e} GJ gas, {:.2e} kg C tanked, {:.2e} kg C leaked",
        p.oil_m3, p.gas_gj, p.tanked_kgc, p.leaked_kgc
    );
}

#[test]
fn no_time_no_production() {
    let m = &base().model;
    let mut f = fields().into_iter().next().unwrap();
    let mut ledger = Ledger::new();
    for dt in [0.0, -1.0, f64::NAN] {
        assert_eq!(f.produce(dt, m, &mut ledger).oil_m3, 0.0);
    }
    assert!(ledger.entries().is_empty());
}

#[test]
fn a_field_drills_the_wells_its_rate_needs_and_counts_their_blowouts() {
    let m = &base().model;
    // EIA 2025: the average US oil well, 29 b/d in 2024; BOEM 2014, US Gulf
    // of Mexico: 1.71e-3 blowouts per development well, 9.88e-5 per
    // oil-producing well-year.
    assert_eq!(m.well_bbl_per_day, 29.0);
    assert_eq!(m.blowout_per_well_drilled, 0.00171);
    assert_eq!(m.blowout_per_well_year, 9.88e-5);
    let mut field = fields()
        .into_iter()
        .max_by(|a, b| a.ultimate_oil_m3.total_cmp(&b.ultimate_oil_m3))
        .unwrap();
    // Choked to the refinery's 500 bbl a day for 30 years.
    let year_m3 = 500.0 * BARREL_M3 * 365.25;
    for _ in 0..30 {
        field.produce_up_to(1.0, year_m3, m, &mut Ledger::new());
    }
    assert_eq!(field.wells, 18, "500 / 29 rounded up");
    let expected = 18.0 * 0.00171 + 18.0 * 30.0 * 9.88e-5;
    assert!((field.blowout_expectation - expected).abs() < 1e-12);
    assert_eq!(field.blowouts, 0);
    // Bailey et al. 2000: three barrels of water for each of oil, sent
    // back down.
    assert_eq!(m.water_oil_ratio, 3.0);
    assert!((field.produced_water_m3 - 3.0 * field.produced_oil_m3).abs() < 1e-6 * field.produced_water_m3);
    // A well is never abandoned, and the count only passes whole numbers.
    field.drill_for(0.0, 600.0, m);
    assert_eq!(field.wells, 18);
    assert_eq!(field.blowouts, field.blowout_expectation.floor() as u32);
    assert!(field.blowouts >= 1);
    // No time, no wells.
    let mut idle = fields().into_iter().next().unwrap();
    idle.produce_up_to(0.0, year_m3, m, &mut Ledger::new());
    assert_eq!((idle.wells, idle.blowouts), (0, 0));
}
