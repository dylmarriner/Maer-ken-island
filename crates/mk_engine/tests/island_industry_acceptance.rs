//! Phase 4b Task 9's acceptance (slow tier): one local year of the island
//! with its oil chain and its money running. Every ledger closes as it
//! goes: carbon (with the chain's crude, products and leaked methane) and
//! material water inside every audited step, the island's water every hour,
//! and the money ledger at every month's close. Snapshotting the island
//! half way through the year and resuming from the snapshot ends the year
//! at the same state hash as running straight through.
//!
//! Run with `cargo test --release -p mk_engine --test island_industry_acceptance -- --ignored slow_`.

use std::path::PathBuf;
use std::sync::Arc;

use mk_core::canon::CanonLocked;
use mk_engine::regional::life::IslandLife;
use mk_island::IslandScenario;

fn repo(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

fn canon() -> Arc<CanonLocked> {
    Arc::new(CanonLocked::load(&repo("fixtures/island/canon.json")).expect("canon"))
}

fn bootstrap_life() -> IslandLife {
    let scenario = IslandScenario::load(&repo("fixtures/island/default_scenario.json")).unwrap();
    IslandLife::bootstrap(scenario, canon()).expect("the island bootstraps")
}

fn hex(d: [u8; 32]) -> String {
    d.iter().map(|b| format!("{b:02x}")).collect()
}

/// Advance `life` by `seconds` a month at a time, checking the money after
/// each: what is held is what was issued or lent, and every month that has
/// ended has been closed.
fn run_months(life: &mut IslandLife, seconds: u64) {
    let month_s = life.money.as_ref().expect("the books are open").month_s;
    let end_s = life.sim_time_s + seconds;
    while life.sim_time_s < end_s {
        let step = month_s.min(end_s - life.sim_time_s);
        life.advance(step)
            .unwrap_or_else(|e| panic!("at {} s: {e:?}", life.sim_time_s));
        let money = life.money.as_ref().unwrap();
        money
            .ledger
            .audit()
            .unwrap_or_else(|e| panic!("money at {} s: {e:?}", life.sim_time_s));
        assert!(
            money.next_month_s > life.sim_time_s,
            "a month ended at {} s without closing",
            money.next_month_s
        );
    }
}

#[test]
#[ignore = "slow: a local year of the island, about an hour in release"]
fn slow_a_year_of_industry_and_money_closes_every_ledger_and_resumes_exactly() {
    let mut life = bootstrap_life();
    assert!(life.industry.is_some(), "the oil chain is surveyed");
    assert!(life.money.is_some(), "the books are open");
    let year_s = life.canon.orbital_period_s.round() as u64;
    let start_s = life.sim_time_s;
    let issued_at_start = life.money.as_ref().unwrap().ledger.issued();

    let half_s = year_s / 2;
    run_months(&mut life, half_s);
    let snapshot = life.snapshot();
    let mut resumed = IslandLife::restore(canon(), snapshot).expect("the island restores");
    assert_eq!(
        hex(life.state_digest()),
        hex(resumed.state_digest()),
        "a restored island differs from the one it was taken from"
    );

    run_months(&mut life, year_s - half_s);
    run_months(&mut resumed, year_s - half_s);
    assert_eq!(life.sim_time_s, start_s + year_s);
    assert_eq!(
        hex(life.state_digest()),
        hex(resumed.state_digest()),
        "resuming from the mid-year snapshot changed the year"
    );

    // The year's audits all ran: one carbon audit an hour at the least, and
    // the island's water every hour.
    let hours = year_s / 3_600;
    assert!(
        life.audits_closed >= hours,
        "{} carbon audits in {hours} hours",
        life.audits_closed
    );
    assert!(
        life.water_audits_closed >= hours,
        "{} water audits in {hours} hours",
        life.water_audits_closed
    );

    // Twelve months closed, and money was only ever issued at the opening:
    // what changed hands since was paid, not printed.
    let money = life.money.as_ref().unwrap();
    assert!(money.next_month_s > start_s + 11 * money.month_s);
    assert_eq!(money.ledger.issued(), issued_at_start);

    // The chain ran: where the island has petroleum it refined crude, and
    // its leaked methane stays a finite, non-negative stock in the air.
    let chain = life.industry.as_ref().unwrap();
    if !chain.fields.is_empty() {
        assert!(
            chain.refined_m3 > 0.0,
            "a year with oil fields refined nothing"
        );
    }
    assert!(chain.atmos_ch4_kgc.is_finite() && chain.atmos_ch4_kgc >= 0.0);
    assert!(chain
        .store_t
        .iter()
        .chain(&chain.store_kgc)
        .all(|v| v.is_finite() && *v >= 0.0));
    eprintln!(
        "a year: {} carbon and {} water audits, {:.0} m3 refined, {} fields, digest {}",
        life.audits_closed,
        life.water_audits_closed,
        chain.refined_m3,
        chain.fields.len(),
        hex(life.state_digest())
    );
}
