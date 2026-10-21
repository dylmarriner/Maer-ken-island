//! A local year of the town's money (Phase 4b Task 7): the founders' power
//! and water company bills households from their meters every month, pays
//! its staff every 14 roster days and pays half of each month's profit to
//! Gem-D and Gem-K; a villager borrows from the bank; a household that stops
//! paying is cut off. Money is conserved at every step.

use std::path::PathBuf;

use mk_core::canon::CanonLocked;
use mk_engine::regional::economy::enterprise::{Enterprise, PayReference, PAY_PERIOD_S};
use mk_engine::regional::economy::market::{Meters, Tariffs};
use mk_engine::regional::economy::money::{
    founders_shares, BillingAccount, FinanceReference, Holder, MoneyLedger, PAYOUT_SHARE,
};
use mk_engine::validation::{default_reference_dir, ReferenceLibrary};

fn local_year_s() -> u64 {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/island/canon.json");
    CanonLocked::load(&path)
        .expect("island canon loads")
        .orbital_period_s
        .round() as u64
}

struct Household {
    payer: Holder,
    meters: Meters,
    power: BillingAccount,
    water: BillingAccount,
    /// Whether the household pays its bills.
    pays: bool,
}

#[test]
fn a_year_of_the_town_s_money_is_conserved_and_pays_out_by_the_owner_s_rules() {
    let library = ReferenceLibrary::load(&default_reference_dir()).unwrap();
    let finance = FinanceReference::from_reference(&library).unwrap();
    let pay = PayReference::from_reference(&library).unwrap();
    let tariffs = Tariffs::from_reference(&library).unwrap();
    let month_s = local_year_s() / 12;
    assert!(month_s > PAY_PERIOD_S, "a local month spans a pay period");

    let mut ledger = MoneyLedger::default();
    for founder in ["Gem-D", "Gem-K"] {
        ledger
            .issue(
                0,
                Holder::Person(founder.into()),
                finance.founder_opening_cents,
                "opening net worth",
            )
            .unwrap();
    }

    // The founders capitalise the utility out of their own deposits.
    let mut utility = Enterprise::new("Gem Utilities", founders_shares());
    for founder in ["Gem-D", "Gem-K"] {
        ledger
            .transfer(
                0,
                &Holder::Person(founder.into()),
                &utility.holder(),
                20_000_000,
                "capital",
            )
            .unwrap();
    }
    let lines_rate = pay
        .industry_rate("Electricity, Gas, Water and Waste Services")
        .unwrap();
    utility.hire(&pay, "worker", lines_rate).unwrap();

    // The worker borrows to buy a house from the founders.
    let worker = Holder::Person("worker".into());
    ledger
        .lend(0, worker.clone(), 30_000_000, finance.lending_rate, 360)
        .unwrap();
    ledger
        .transfer(
            0,
            &worker,
            &Holder::Person("Gem-D".into()),
            30_000_000,
            "house",
        )
        .unwrap();

    let mut households = vec![
        Household {
            payer: worker.clone(),
            meters: Meters::default(),
            power: BillingAccount::new(),
            water: BillingAccount::new(),
            pays: true,
        },
        Household {
            payer: Holder::Person("stopped".into()),
            meters: Meters::default(),
            power: BillingAccount::new(),
            water: BillingAccount::new(),
            pays: false,
        },
    ];
    ledger
        .issue(0, households[1].payer.clone(), 10_000, "savings")
        .unwrap();

    let year_s = 12 * month_s;
    let mut t = 0;
    let mut next_pay = PAY_PERIOD_S;
    let mut next_month = month_s;
    let mut payslips = 0;
    let mut founder_dividends = 0;
    let step_s = 86_400;
    while t < year_s {
        t += step_s;
        // A working day and a day of the model household's use.
        utility.record_hours(&pay, "worker", 8.0).unwrap();
        for h in &mut households {
            if h.power.connected {
                h.meters.record_electricity(8_000.0 / 365.0);
            }
            if h.water.connected {
                h.meters.record_water(0.5);
            }
        }

        if t >= next_pay {
            next_pay += PAY_PERIOD_S;
            let slips = utility.run_payroll(&mut ledger, &pay, t).unwrap();
            for slip in &slips {
                assert_eq!(slip.owed, 0, "the utility can always pay");
                assert_eq!(
                    slip.kiwisaver_employer,
                    (slip.gross as f64 * pay.kiwisaver_employer_rate).round() as i64
                );
            }
            payslips += slips.len();
        }

        if t >= next_month {
            next_month += month_s;
            for h in &mut households {
                let bill = h.meters.read(&tariffs);
                h.power.bill(t, bill.electricity);
                h.water.bill(t, bill.water);
                if h.pays {
                    let paid_power = h
                        .power
                        .pay(&mut ledger, t, &h.payer, &utility.holder(), h.power.owed)
                        .unwrap();
                    let paid_water = h
                        .water
                        .pay(&mut ledger, t, &h.payer, &utility.holder(), h.water.owed)
                        .unwrap();
                    utility.book_income(paid_power + paid_water);
                }
                h.power.review(t, month_s);
                h.water.review(t, month_s);
            }
            ledger.service_loans(t);

            let gem_d = Holder::Person("Gem-D".into());
            let before = ledger.balance(&gem_d);
            let close = utility.close_month(&mut ledger, t).unwrap();
            let paid_d = ledger.balance(&gem_d) - before;
            if close.profit > 0 {
                let expected = ((close.profit as f64 * PAYOUT_SHARE).floor() * 0.5).floor() as i64;
                assert_eq!(paid_d, expected, "dividend = profit x payout x share");
            } else {
                assert_eq!(paid_d, 0, "a loss pays nothing");
            }
            founder_dividends += paid_d;
        }
        ledger.audit().unwrap();
    }

    // Paid every 14 roster days through the year.
    let periods = (year_s / PAY_PERIOD_S) as usize;
    assert_eq!(payslips, periods);
    // The household that stopped paying lost power and water; the one that
    // pays kept them.
    assert!(!households[1].power.connected && !households[1].water.connected);
    assert!(households[0].power.connected && households[0].water.connected);
    // The worker kept up the mortgage from their wages.
    assert_eq!(ledger.loans()[0].missed, 0);
    assert!(
        ledger.balance(&Holder::Bank) > 0,
        "the bank earned interest"
    );
    assert!(ledger.balance(&Holder::KiwiSaver("worker".into())) > 0);
    // The utility sold power and water to one paying house and paid a
    // lines worker's wages: it makes a loss, and pays no dividends.
    assert_eq!(founder_dividends, 0);
}
