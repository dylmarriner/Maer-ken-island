//! Enterprises and payroll (Phase 4b Task 7, second part).
//!
//! An [`Enterprise`] is a business with an account in the [`MoneyLedger`],
//! shareholders (Gem-D and Gem-K, half each, by default), staff and a
//! month's takings and costs. It pays its staff **every 14 roster days**
//! (the plan; a roster day is 24 hours, not the island's 36-hour sun) and
//! closes each month by paying out half its profit ([`pay_dividends`]).
//!
//! **Pay** is a sourced hourly rate, never below the adult minimum wage
//! (Employment NZ, 1 April 2026, `industry/wages.json`), with the benefits
//! every New Zealand employee has by law and that the pack cites:
//! - the employer's **KiwiSaver** contribution, 3.5% of gross pay, paid into
//!   the worker's KiwiSaver account (`Holder::KiwiSaver`) on top of their pay;
//! - **four weeks' annual leave** a year. Leave accrues with hours worked, at
//!   4 weeks for every 48 worked (52 less the 4 on leave), and leave taken is
//!   paid at the worker's rate like hours worked.
//!
//! There is no income tax or ACC levy: the island has no government (D62).
//!
//! **Profit is counted on cash**: the month's takings less what the business
//! actually paid out, wages and KiwiSaver included. Wages a business cannot
//! cover are owed to the worker, paid first at the next pay run and counted
//! as a cost when they are paid.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::money::{pay_dividends, Cents, Holder, MoneyError, MoneyLedger, CENTS_PER_DOLLAR};
use crate::validation::{ReferenceDomain, ReferenceLibrary};

/// A roster day: shifts and pay run on a 24-hour clock (the plan, Task 6).
pub const ROSTER_DAY_S: u64 = 86_400;

/// Staff are paid every 14 roster days (the plan, Task 7).
pub const PAY_PERIOD_S: u64 = 14 * ROSTER_DAY_S;

/// The sourced pay figures (`industry/wages.json`).
#[derive(Debug, Clone, PartialEq)]
pub struct PayReference {
    pub minimum_hourly_cents: Cents,
    pub kiwisaver_employer_rate: f64,
    pub annual_leave_weeks: f64,
    /// Median ordinary-time hourly earnings by industry (Stats NZ QES).
    pub hourly_by_industry_cents: BTreeMap<String, Cents>,
    /// Median hourly earnings by ANZSCO major occupation group (Stats NZ HLFS).
    pub hourly_by_occupation_cents: BTreeMap<String, Cents>,
}

fn dollars_to_cents(dollars: f64) -> Cents {
    (dollars * CENTS_PER_DOLLAR as f64).round() as Cents
}

impl PayReference {
    pub fn from_reference(library: &ReferenceLibrary) -> Result<Self, MoneyError> {
        let item = |key: &str| {
            library
                .item(ReferenceDomain::Industry, key)
                .ok_or_else(|| MoneyError::Reference(format!("industry/wages.json: {key}")))
        };
        let point = |key: &str| {
            item(key)?
                .point()
                .ok_or_else(|| MoneyError::Reference(format!("industry/wages.json: {key}")))
        };
        let column = |key: &str| -> Result<BTreeMap<String, Cents>, MoneyError> {
            let table = item(key)?
                .table()
                .ok_or_else(|| MoneyError::Reference(format!("industry/wages.json: {key}")))?;
            Ok(table
                .rows
                .iter()
                .filter_map(|row| Some((row.label.clone(), dollars_to_cents(*row.values.first()?))))
                .collect())
        };
        Ok(Self {
            minimum_hourly_cents: dollars_to_cents(point("minimum_wage_adult_2026")?),
            kiwisaver_employer_rate: point("kiwisaver_employer_min_rate")?,
            annual_leave_weeks: point("annual_leave_weeks")?,
            hourly_by_industry_cents: column("qes_ordinary_hourly_by_industry_2026q2")?,
            hourly_by_occupation_cents: column("median_hourly_by_occupation_2026")?,
        })
    }

    /// Leave hours earned for each hour worked: the annual weeks of leave
    /// spread over the weeks actually worked.
    pub fn leave_per_hour_worked(&self) -> f64 {
        self.annual_leave_weeks / (52.0 - self.annual_leave_weeks)
    }

    /// The median rate for an industry, by its Stats NZ name.
    pub fn industry_rate(&self, industry: &str) -> Option<Cents> {
        self.hourly_by_industry_cents.get(industry).copied()
    }

    /// The median rate for an ANZSCO major group, by its Stats NZ name.
    pub fn occupation_rate(&self, occupation: &str) -> Option<Cents> {
        self.hourly_by_occupation_cents.get(occupation).copied()
    }
}

/// One member of staff.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Employee {
    pub person: String,
    pub hourly_cents: Cents,
    /// Hours worked since the last pay run.
    pub hours_unpaid: f64,
    /// Hours of leave taken since the last pay run, paid like hours worked.
    pub leave_hours_unpaid: f64,
    /// Leave earned and not yet taken, in hours.
    pub leave_balance_hours: f64,
    /// Pay (and KiwiSaver) earned in past pay runs that the business could
    /// not cover.
    pub wages_owed: Cents,
    pub kiwisaver_owed: Cents,
}

/// A record of one pay run for one worker.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Payslip {
    pub person: String,
    pub enterprise: String,
    pub period_end_s: u64,
    pub hours_worked: f64,
    pub leave_hours_taken: f64,
    pub hourly_cents: Cents,
    /// This period's gross pay, leave included.
    pub gross: Cents,
    /// The employer's KiwiSaver contribution on this period's gross.
    pub kiwisaver_employer: Cents,
    /// What actually reached the worker's account this run (arrears first).
    pub paid: Cents,
    pub kiwisaver_paid: Cents,
    /// Still owed to the worker after this run.
    pub owed: Cents,
    pub leave_balance_hours: f64,
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum EnterpriseError {
    #[error("{person} is not on {enterprise}'s staff")]
    NotStaff { person: String, enterprise: String },
    #[error("{person} is already on {enterprise}'s staff")]
    AlreadyStaff { person: String, enterprise: String },
    #[error("{offered} cents an hour is below the adult minimum wage of {minimum}")]
    BelowMinimumWage { offered: Cents, minimum: Cents },
    #[error("{person} has {balance:.1} hours of leave and asked for {asked:.1}")]
    NotEnoughLeave {
        person: String,
        balance: f64,
        asked: f64,
    },
    #[error("hours are a finite, non-negative number, not {0}")]
    BadHours(f64),
    #[error(transparent)]
    Money(#[from] MoneyError),
}

/// What one month's close did.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MonthClose {
    pub enterprise: String,
    pub at_s: u64,
    pub income: Cents,
    pub costs: Cents,
    pub profit: Cents,
    pub dividends: Vec<(Holder, Cents)>,
}

/// A business on the island.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Enterprise {
    pub name: String,
    pub shareholders: Vec<(Holder, f64)>,
    pub staff: Vec<Employee>,
    /// Takings since the month began.
    pub month_income: Cents,
    /// What the business paid out since the month began.
    pub month_costs: Cents,
    pub payslips: Vec<Payslip>,
    pub closes: Vec<MonthClose>,
}

impl Enterprise {
    pub fn new(name: &str, shareholders: Vec<(Holder, f64)>) -> Self {
        Self {
            name: name.to_string(),
            shareholders,
            staff: Vec::new(),
            month_income: 0,
            month_costs: 0,
            payslips: Vec::new(),
            closes: Vec::new(),
        }
    }

    pub fn holder(&self) -> Holder {
        Holder::Enterprise(self.name.clone())
    }

    fn employee_mut(&mut self, person: &str) -> Result<&mut Employee, EnterpriseError> {
        let enterprise = self.name.clone();
        self.staff
            .iter_mut()
            .find(|e| e.person == person)
            .ok_or_else(|| EnterpriseError::NotStaff {
                person: person.to_string(),
                enterprise,
            })
    }

    /// Take `person` on at `hourly_cents`, which may not be below the
    /// minimum wage.
    pub fn hire(
        &mut self,
        pay: &PayReference,
        person: &str,
        hourly_cents: Cents,
    ) -> Result<(), EnterpriseError> {
        if hourly_cents < pay.minimum_hourly_cents {
            return Err(EnterpriseError::BelowMinimumWage {
                offered: hourly_cents,
                minimum: pay.minimum_hourly_cents,
            });
        }
        if self.staff.iter().any(|e| e.person == person) {
            return Err(EnterpriseError::AlreadyStaff {
                person: person.to_string(),
                enterprise: self.name.clone(),
            });
        }
        self.staff.push(Employee {
            person: person.to_string(),
            hourly_cents,
            hours_unpaid: 0.0,
            leave_hours_unpaid: 0.0,
            leave_balance_hours: 0.0,
            wages_owed: 0,
            kiwisaver_owed: 0,
        });
        Ok(())
    }

    /// `person` leaves. What they are owed stays owed: the caller should run
    /// a final pay first.
    pub fn release(&mut self, person: &str) -> Result<Employee, EnterpriseError> {
        let index = self
            .staff
            .iter()
            .position(|e| e.person == person)
            .ok_or_else(|| EnterpriseError::NotStaff {
                person: person.to_string(),
                enterprise: self.name.clone(),
            })?;
        Ok(self.staff.remove(index))
    }

    /// `person` worked `hours`, which earns leave as well as pay.
    pub fn record_hours(
        &mut self,
        pay: &PayReference,
        person: &str,
        hours: f64,
    ) -> Result<(), EnterpriseError> {
        if !hours.is_finite() || hours < 0.0 {
            return Err(EnterpriseError::BadHours(hours));
        }
        let accrual = pay.leave_per_hour_worked();
        let employee = self.employee_mut(person)?;
        employee.hours_unpaid += hours;
        employee.leave_balance_hours += hours * accrual;
        Ok(())
    }

    /// `person` takes `hours` of leave from their balance, paid at the next
    /// pay run.
    pub fn take_leave(&mut self, person: &str, hours: f64) -> Result<(), EnterpriseError> {
        if !hours.is_finite() || hours < 0.0 {
            return Err(EnterpriseError::BadHours(hours));
        }
        let employee = self.employee_mut(person)?;
        if hours > employee.leave_balance_hours + 1e-9 {
            return Err(EnterpriseError::NotEnoughLeave {
                person: person.to_string(),
                balance: employee.leave_balance_hours,
                asked: hours,
            });
        }
        employee.leave_balance_hours = (employee.leave_balance_hours - hours).max(0.0);
        employee.leave_hours_unpaid += hours;
        Ok(())
    }

    /// A customer pays the business.
    pub fn sell(
        &mut self,
        ledger: &mut MoneyLedger,
        at_s: u64,
        customer: &Holder,
        cents: Cents,
        memo: &str,
    ) -> Result<(), EnterpriseError> {
        ledger.transfer(at_s, customer, &self.holder(), cents, memo)?;
        self.month_income += cents;
        Ok(())
    }

    /// Takings that reached the business's account by another route (a bill
    /// paid through [`super::money::BillingAccount::pay`], say).
    pub fn book_income(&mut self, cents: Cents) {
        self.month_income += cents;
    }

    /// The business pays a supplier.
    pub fn buy(
        &mut self,
        ledger: &mut MoneyLedger,
        at_s: u64,
        supplier: &Holder,
        cents: Cents,
        memo: &str,
    ) -> Result<(), EnterpriseError> {
        ledger.transfer(at_s, &self.holder(), supplier, cents, memo)?;
        self.month_costs += cents;
        Ok(())
    }

    /// Pay every member of staff for the period ending at `at_s`: arrears
    /// first, then this period's gross and the employer's KiwiSaver, as far
    /// as the business's account covers them. Returns the payslips, which
    /// are also kept.
    pub fn run_payroll(
        &mut self,
        ledger: &mut MoneyLedger,
        pay: &PayReference,
        at_s: u64,
    ) -> Result<Vec<Payslip>, EnterpriseError> {
        let business = self.holder();
        let mut slips = Vec::new();
        for employee in &mut self.staff {
            let paid_hours = employee.hours_unpaid + employee.leave_hours_unpaid;
            let gross = (paid_hours * employee.hourly_cents as f64).round() as Cents;
            let kiwisaver = (gross as f64 * pay.kiwisaver_employer_rate).round() as Cents;
            let wages_due = employee.wages_owed + gross;
            let kiwisaver_due = employee.kiwisaver_owed + kiwisaver;

            // Wages before KiwiSaver: what the worker lives on comes first.
            let available = ledger.balance(&business);
            let paid = wages_due.min(available);
            if paid > 0 {
                ledger.transfer(
                    at_s,
                    &business,
                    &Holder::Person(employee.person.clone()),
                    paid,
                    "wages",
                )?;
            }
            let kiwisaver_paid = kiwisaver_due.min(ledger.balance(&business));
            if kiwisaver_paid > 0 {
                ledger.transfer(
                    at_s,
                    &business,
                    &Holder::KiwiSaver(employee.person.clone()),
                    kiwisaver_paid,
                    "KiwiSaver employer contribution",
                )?;
            }
            self.month_costs += paid + kiwisaver_paid;

            employee.wages_owed = wages_due - paid;
            employee.kiwisaver_owed = kiwisaver_due - kiwisaver_paid;
            let slip = Payslip {
                person: employee.person.clone(),
                enterprise: self.name.clone(),
                period_end_s: at_s,
                hours_worked: employee.hours_unpaid,
                leave_hours_taken: employee.leave_hours_unpaid,
                hourly_cents: employee.hourly_cents,
                gross,
                kiwisaver_employer: kiwisaver,
                paid,
                kiwisaver_paid,
                owed: employee.wages_owed + employee.kiwisaver_owed,
                leave_balance_hours: employee.leave_balance_hours,
            };
            employee.hours_unpaid = 0.0;
            employee.leave_hours_unpaid = 0.0;
            slips.push(slip);
        }
        self.payslips.extend(slips.iter().cloned());
        Ok(slips)
    }

    /// Close the month: profit is takings less costs, half of a profit is
    /// paid to the shareholders, and the month starts again.
    pub fn close_month(
        &mut self,
        ledger: &mut MoneyLedger,
        at_s: u64,
    ) -> Result<MonthClose, EnterpriseError> {
        let profit = self.month_income - self.month_costs;
        let dividends = pay_dividends(ledger, at_s, &self.holder(), profit, &self.shareholders)?;
        let close = MonthClose {
            enterprise: self.name.clone(),
            at_s,
            income: self.month_income,
            costs: self.month_costs,
            profit,
            dividends,
        };
        self.month_income = 0;
        self.month_costs = 0;
        self.closes.push(close.clone());
        Ok(close)
    }
}

#[cfg(test)]
mod tests {
    use super::super::money::founders_shares;
    use super::*;

    fn pay() -> PayReference {
        let library = ReferenceLibrary::load(&crate::validation::default_reference_dir()).unwrap();
        PayReference::from_reference(&library).unwrap()
    }

    #[test]
    fn the_sourced_rates_load() {
        let pay = pay();
        assert_eq!(pay.minimum_hourly_cents, 2_395);
        assert_eq!(pay.kiwisaver_employer_rate, 0.035);
        assert_eq!(
            pay.industry_rate("Electricity, Gas, Water and Waste Services"),
            Some(5_369)
        );
        assert_eq!(pay.occupation_rate("Professionals"), Some(4_771));
        // Four weeks for every 48 worked.
        assert!((pay.leave_per_hour_worked() - 4.0 / 48.0).abs() < 1e-12);
    }

    #[test]
    fn nobody_is_hired_below_the_minimum_wage_or_twice() {
        let pay = pay();
        let mut shop = Enterprise::new("Gem Store", founders_shares());
        assert_eq!(
            shop.hire(&pay, "v1", 2_394),
            Err(EnterpriseError::BelowMinimumWage {
                offered: 2_394,
                minimum: 2_395
            })
        );
        shop.hire(&pay, "v1", 2_395).unwrap();
        assert!(matches!(
            shop.hire(&pay, "v1", 3_000),
            Err(EnterpriseError::AlreadyStaff { .. })
        ));
    }

    #[test]
    fn a_fortnight_pays_gross_plus_kiwisaver_and_earns_leave() {
        let pay = pay();
        let mut ledger = MoneyLedger::default();
        let mut hydro = Enterprise::new("Gem Hydro", founders_shares());
        ledger
            .issue(0, hydro.holder(), 1_000_000, "working capital")
            .unwrap();
        hydro.hire(&pay, "w", 5_369).unwrap();
        hydro.record_hours(&pay, "w", 80.0).unwrap();
        let slips = hydro.run_payroll(&mut ledger, &pay, PAY_PERIOD_S).unwrap();
        let slip = &slips[0];
        assert_eq!(slip.gross, 80 * 5_369);
        assert_eq!(
            slip.kiwisaver_employer,
            (429_520.0_f64 * 0.035).round() as Cents
        );
        assert_eq!(slip.paid, slip.gross);
        assert_eq!(slip.owed, 0);
        assert!((slip.leave_balance_hours - 80.0 / 12.0).abs() < 1e-9);
        assert_eq!(ledger.balance(&Holder::Person("w".into())), slip.gross);
        assert_eq!(
            ledger.balance(&Holder::KiwiSaver("w".into())),
            slip.kiwisaver_employer
        );
        assert_eq!(hydro.month_costs, slip.gross + slip.kiwisaver_employer);
        ledger.audit().unwrap();
    }

    #[test]
    fn leave_is_paid_from_the_balance_and_no_more() {
        let pay = pay();
        let mut ledger = MoneyLedger::default();
        let mut farm = Enterprise::new("Gem Farms", founders_shares());
        ledger
            .issue(0, farm.holder(), 1_000_000, "capital")
            .unwrap();
        farm.hire(&pay, "f", 3_000).unwrap();
        farm.record_hours(&pay, "f", 96.0).unwrap();
        assert!(matches!(
            farm.take_leave("f", 8.5),
            Err(EnterpriseError::NotEnoughLeave { .. })
        ));
        farm.take_leave("f", 8.0).unwrap();
        let slip = &farm.run_payroll(&mut ledger, &pay, PAY_PERIOD_S).unwrap()[0];
        assert_eq!(slip.gross, 104 * 3_000, "96 worked and 8 on leave");
        assert!(slip.leave_balance_hours.abs() < 1e-9);
    }

    #[test]
    fn wages_a_business_cannot_cover_are_owed_and_paid_first_next_time() {
        let pay = pay();
        let mut ledger = MoneyLedger::default();
        let mut cafe = Enterprise::new("Gem Cafe", founders_shares());
        ledger.issue(0, cafe.holder(), 100_000, "capital").unwrap();
        ledger
            .issue(0, Holder::Person("diner".into()), 1_000_000, "savings")
            .unwrap();
        cafe.hire(&pay, "c", 2_500).unwrap();
        cafe.record_hours(&pay, "c", 60.0).unwrap(); // $1,500 gross
        let slip = cafe.run_payroll(&mut ledger, &pay, PAY_PERIOD_S).unwrap()[0].clone();
        assert_eq!(slip.paid, 100_000);
        assert_eq!(slip.owed, 150_000 - 100_000 + slip.kiwisaver_employer);
        assert_eq!(ledger.balance(&cafe.holder()), 0);

        cafe.sell(
            &mut ledger,
            PAY_PERIOD_S + 1,
            &Holder::Person("diner".into()),
            500_000,
            "meals",
        )
        .unwrap();
        let next = cafe
            .run_payroll(&mut ledger, &pay, 2 * PAY_PERIOD_S)
            .unwrap()[0]
            .clone();
        assert_eq!(next.gross, 0);
        assert_eq!(next.paid, 50_000, "the arrears");
        assert_eq!(next.kiwisaver_paid, slip.kiwisaver_employer);
        assert_eq!(next.owed, 0);
        ledger.audit().unwrap();
    }

    #[test]
    fn a_month_closes_with_half_the_profit_to_the_founders() {
        let pay = pay();
        let mut ledger = MoneyLedger::default();
        let mut hydro = Enterprise::new("Gem Hydro", founders_shares());
        let town = Holder::Person("town".into());
        ledger
            .issue(0, town.clone(), 10_000_000, "savings")
            .unwrap();
        hydro.hire(&pay, "w", 4_000).unwrap();
        hydro
            .sell(&mut ledger, 1, &town, 1_000_000, "electricity")
            .unwrap();
        hydro.record_hours(&pay, "w", 80.0).unwrap();
        hydro.run_payroll(&mut ledger, &pay, PAY_PERIOD_S).unwrap();
        let costs = 320_000 + 11_200;
        let close = hydro.close_month(&mut ledger, 2 * PAY_PERIOD_S).unwrap();
        assert_eq!(close.costs, costs);
        assert_eq!(close.profit, 1_000_000 - costs);
        let half_each = (close.profit / 2) / 2;
        assert_eq!(
            close.dividends,
            vec![
                (Holder::Person("Gem-D".into()), half_each),
                (Holder::Person("Gem-K".into()), half_each)
            ]
        );
        assert_eq!(hydro.month_income, 0);
        ledger.audit().unwrap();
    }
}
