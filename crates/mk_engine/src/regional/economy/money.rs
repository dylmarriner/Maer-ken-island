//! Money on the island (Phase 4b Task 7, first part): accounts, a bank that
//! takes deposits and lends, monthly bills with the owner's disconnection
//! rule, and dividends.
//!
//! **Money is conserved.** Every movement is a [`Transaction`] from one
//! account to another, so the total held never changes, with two kinds of
//! exception, both explicit and both in the log:
//! - **Issuance** at the scenario's start: each founder's opening deposit,
//!   NZ median household net worth (Stats NZ 2024, `industry/finance.json`).
//!   That figure is mostly houses and land, so as money in the bank it is a
//!   deviation (D61).
//! - **Lending.** As a real bank does, the island's bank creates the money
//!   it lends as a deposit in the borrower's account, and repaid principal
//!   leaves the money supply again. Interest is not destroyed: it is paid to
//!   the bank's own account, which is income like any other business's.
//!
//! So `total held == issued + principal outstanding` at every step, which
//! [`MoneyLedger::audit`] checks.
//!
//! **Months.** The local calendar has no months, but the climatology already
//! bins the year into twelve (D32), and the plan bills "each local month":
//! a month here is a twelfth of the local year (D61). The ledger itself is
//! calendar-free: callers pass times in seconds and the month's length.
//!
//! **Disconnection** (the owner, 2026-10-15): a household whose bills have
//! gone a month unpaid loses its power and water, unless it is still
//! making regular payments and trying to keep up. See [`BillingAccount`].
//!
//! **Dividends** (the owner, 2026-10-15): every enterprise is owned 50/50 by
//! Gem-D and Gem-K, and half of each month's profit is paid out; the rest
//! stays with the business. See [`pay_dividends`].
//!
//! Amounts are whole cents in `i64`: money is counted, never rounded away.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::validation::{ReferenceDomain, ReferenceLibrary};

/// Whole New Zealand cents.
pub type Cents = i64;

/// Cents in a dollar.
pub const CENTS_PER_DOLLAR: Cents = 100;

/// Share of a month's profit paid to the owners (the owner, 2026-10-15).
pub const PAYOUT_SHARE: f64 = 0.5;

/// Who holds an account.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Holder {
    /// A person, by agent id.
    Person(String),
    /// A business, by name ("Gem Hydro", "Gem Housing", ...).
    Enterprise(String),
    /// The bank's own account: its interest income.
    Bank,
    /// A person's KiwiSaver: retirement savings in their name that they
    /// cannot spend, which the employer contributes to with each pay.
    KiwiSaver(String),
}

/// Why money moved, or appeared, or left.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Movement {
    /// Money created at the scenario's start.
    Issued,
    /// One account paid another.
    Paid { from: Holder },
    /// The bank created a loan as a deposit in the borrower's account.
    Lent,
    /// Repaid principal, leaving the money supply.
    Repaid,
}

/// One entry in the money log.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transaction {
    pub at_s: u64,
    pub movement: Movement,
    /// The account credited, or for [`Movement::Repaid`] the borrower debited.
    pub to: Holder,
    pub cents: Cents,
    pub memo: String,
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum MoneyError {
    #[error("{holder:?} holds {held} cents and cannot pay {asked}")]
    Insufficient {
        holder: Holder,
        held: Cents,
        asked: Cents,
    },
    #[error("an amount of money is a positive number of cents, not {0}")]
    NotPositive(Cents),
    #[error("money is not conserved: {held} cents held against {expected} issued and lent")]
    NotConserved { held: Cents, expected: Cents },
    #[error("{0}")]
    Reference(String),
}

/// A loan from the bank: an annuity, repaid in equal monthly instalments of
/// interest and principal over its term.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Loan {
    pub borrower: Holder,
    pub outstanding: Cents,
    pub annual_rate: f64,
    /// The fixed monthly instalment (the last one may be smaller).
    pub instalment: Cents,
    /// Instalments due and not paid.
    pub missed: u32,
}

/// Every account, the bank's loans and the log.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MoneyLedger {
    /// Saved as a list of pairs: a holder is not a JSON map key.
    #[serde(with = "pairs")]
    balances: BTreeMap<Holder, Cents>,
    loans: Vec<Loan>,
    issued: Cents,
    log: Vec<Transaction>,
}

impl MoneyLedger {
    pub fn balance(&self, holder: &Holder) -> Cents {
        self.balances.get(holder).copied().unwrap_or(0)
    }

    /// Everything held in every account.
    pub fn total_held(&self) -> Cents {
        self.balances.values().sum()
    }

    /// Principal the bank has lent and not yet been repaid.
    pub fn principal_outstanding(&self) -> Cents {
        self.loans.iter().map(|l| l.outstanding).sum()
    }

    pub fn issued(&self) -> Cents {
        self.issued
    }

    pub fn loans(&self) -> &[Loan] {
        &self.loans
    }

    pub fn log(&self) -> &[Transaction] {
        &self.log
    }

    /// Money is held only as it was issued or lent.
    pub fn audit(&self) -> Result<(), MoneyError> {
        let held = self.total_held();
        let expected = self.issued + self.principal_outstanding();
        if held == expected {
            Ok(())
        } else {
            Err(MoneyError::NotConserved { held, expected })
        }
    }

    fn credit(&mut self, holder: &Holder, cents: Cents) {
        *self.balances.entry(holder.clone()).or_insert(0) += cents;
    }

    fn debit(&mut self, holder: &Holder, cents: Cents) -> Result<(), MoneyError> {
        let held = self.balance(holder);
        if held < cents {
            return Err(MoneyError::Insufficient {
                holder: holder.clone(),
                held,
                asked: cents,
            });
        }
        *self.balances.entry(holder.clone()).or_insert(0) -= cents;
        Ok(())
    }

    /// Create `cents` in `to`'s account: for opening balances only.
    pub fn issue(
        &mut self,
        at_s: u64,
        to: Holder,
        cents: Cents,
        memo: &str,
    ) -> Result<(), MoneyError> {
        if cents <= 0 {
            return Err(MoneyError::NotPositive(cents));
        }
        self.credit(&to, cents);
        self.issued += cents;
        self.log.push(Transaction {
            at_s,
            movement: Movement::Issued,
            to,
            cents,
            memo: memo.to_string(),
        });
        Ok(())
    }

    /// Move `cents` from one account to another, or nothing if the payer
    /// cannot cover it.
    pub fn transfer(
        &mut self,
        at_s: u64,
        from: &Holder,
        to: &Holder,
        cents: Cents,
        memo: &str,
    ) -> Result<(), MoneyError> {
        if cents <= 0 {
            return Err(MoneyError::NotPositive(cents));
        }
        self.debit(from, cents)?;
        self.credit(to, cents);
        self.log.push(Transaction {
            at_s,
            movement: Movement::Paid { from: from.clone() },
            to: to.clone(),
            cents,
            memo: memo.to_string(),
        });
        Ok(())
    }

    /// Lend `principal` to `borrower` at `annual_rate` over `months`,
    /// crediting their account with new money. Returns the loan's index.
    pub fn lend(
        &mut self,
        at_s: u64,
        borrower: Holder,
        principal: Cents,
        annual_rate: f64,
        months: u32,
    ) -> Result<usize, MoneyError> {
        if principal <= 0 {
            return Err(MoneyError::NotPositive(principal));
        }
        let instalment = annuity_instalment(principal, annual_rate, months.max(1));
        self.credit(&borrower, principal);
        self.loans.push(Loan {
            borrower: borrower.clone(),
            outstanding: principal,
            annual_rate,
            instalment,
            missed: 0,
        });
        self.log.push(Transaction {
            at_s,
            movement: Movement::Lent,
            to: borrower,
            cents: principal,
            memo: format!("loan at {:.2}% over {months} months", annual_rate * 100.0),
        });
        Ok(self.loans.len() - 1)
    }

    /// A month passes for every loan: each borrower owes a month's interest
    /// on what is outstanding and pays their instalment if they can. The
    /// interest goes to the bank's account; the rest of the instalment
    /// repays principal and leaves the money supply. A borrower who cannot
    /// pay the whole instalment pays nothing, the interest is added to what
    /// they owe (and so to the money that must one day be repaid), and the
    /// missed instalment is counted.
    pub fn service_loans(&mut self, at_s: u64) {
        for i in 0..self.loans.len() {
            let loan = &self.loans[i];
            if loan.outstanding <= 0 {
                continue;
            }
            let interest = monthly_interest(loan.outstanding, loan.annual_rate);
            // Never less than the month's interest, which capitalised
            // arrears can push above the original instalment.
            let due = loan
                .instalment
                .max(interest)
                .min(loan.outstanding + interest);
            let borrower = loan.borrower.clone();
            if self.balance(&borrower) >= due {
                let principal = due - interest;
                // Interest to the bank, as income.
                if interest > 0 {
                    self.transfer(at_s, &borrower, &Holder::Bank, interest, "loan interest")
                        .expect("balance checked");
                }
                if principal > 0 {
                    self.debit(&borrower, principal).expect("balance checked");
                    self.loans[i].outstanding -= principal;
                    self.log.push(Transaction {
                        at_s,
                        movement: Movement::Repaid,
                        to: borrower,
                        cents: principal,
                        memo: "loan principal".to_string(),
                    });
                }
            } else {
                // Capitalised: owed to the bank as more principal, which is
                // money the bank has in effect lent the borrower to cover it.
                self.loans[i].outstanding += interest;
                self.loans[i].missed += 1;
                self.credit(&Holder::Bank, interest);
                self.log.push(Transaction {
                    at_s,
                    movement: Movement::Lent,
                    to: Holder::Bank,
                    cents: interest,
                    memo: "missed instalment: interest capitalised".to_string(),
                });
            }
        }
    }
}

mod pairs {
    use std::collections::BTreeMap;

    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use super::{Cents, Holder};

    pub fn serialize<S: Serializer>(
        map: &BTreeMap<Holder, Cents>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        map.iter().collect::<Vec<_>>().serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<BTreeMap<Holder, Cents>, D::Error> {
        Ok(Vec::<(Holder, Cents)>::deserialize(deserializer)?
            .into_iter()
            .collect())
    }
}

/// A month's interest on `outstanding` at `annual_rate`, to the cent.
pub fn monthly_interest(outstanding: Cents, annual_rate: f64) -> Cents {
    (outstanding as f64 * annual_rate / 12.0).round() as Cents
}

/// The level monthly instalment that repays `principal` with interest at
/// `annual_rate` over `months` (the standard annuity formula).
pub fn annuity_instalment(principal: Cents, annual_rate: f64, months: u32) -> Cents {
    let r = annual_rate / 12.0;
    let n = months as f64;
    let p = principal as f64;
    let payment = if r.abs() < 1e-12 {
        p / n
    } else {
        p * r / (1.0 - (1.0 + r).powf(-n))
    };
    payment.ceil() as Cents
}

/// The island's sourced money figures (`industry/finance.json`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FinanceReference {
    /// Each founder's opening deposit, in cents.
    pub founder_opening_cents: Cents,
    /// The bank's lending rate, per year.
    pub lending_rate: f64,
}

impl FinanceReference {
    pub fn from_reference(library: &ReferenceLibrary) -> Result<Self, MoneyError> {
        let point = |key: &str| {
            library
                .item(ReferenceDomain::Industry, key)
                .and_then(|i| i.point())
                .ok_or_else(|| MoneyError::Reference(format!("industry/finance.json: {key}")))
        };
        Ok(Self {
            founder_opening_cents: (point("median_household_net_worth_2024")?
                * CENTS_PER_DOLLAR as f64)
                .round() as Cents,
            lending_rate: point("new_mortgage_rate_2y_fixed_2026_04")?,
        })
    }
}

/// A household's (or business's) bills for one supply, and whether it is
/// still connected.
///
/// The owner's rule (2026-10-15): a month of unpaid bills cuts the supply
/// off, unless the account is still making regular payments and trying to
/// keep up. So it is not the balance owed that decides but the payment
/// record: an account that owes money but has paid something within the
/// last month stays connected while it catches up; one that has owed
/// money for a month and paid nothing in that month is disconnected.
/// A disconnected account is reconnected when it is paid up.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BillingAccount {
    /// Billed and not yet paid.
    pub owed: Cents,
    /// When the oldest still-unpaid bill was issued.
    pub owed_since_s: Option<u64>,
    /// When anything was last paid on this account.
    pub last_payment_s: Option<u64>,
    pub connected: bool,
}

impl Default for BillingAccount {
    fn default() -> Self {
        Self::new()
    }
}

impl BillingAccount {
    pub fn new() -> Self {
        Self {
            owed: 0,
            owed_since_s: None,
            last_payment_s: None,
            connected: true,
        }
    }

    /// A bill arrives.
    pub fn bill(&mut self, at_s: u64, cents: Cents) {
        if cents <= 0 {
            return;
        }
        if self.owed == 0 {
            self.owed_since_s = Some(at_s);
        }
        self.owed += cents;
    }

    /// `payer` pays up to `offered` cents of what is owed, to `supplier`.
    /// Returns what was paid.
    pub fn pay(
        &mut self,
        ledger: &mut MoneyLedger,
        at_s: u64,
        payer: &Holder,
        supplier: &Holder,
        offered: Cents,
    ) -> Result<Cents, MoneyError> {
        let paid = offered.min(self.owed);
        if paid <= 0 {
            return Ok(0);
        }
        ledger.transfer(at_s, payer, supplier, paid, "bill")?;
        self.owed -= paid;
        self.last_payment_s = Some(at_s);
        if self.owed == 0 {
            self.owed_since_s = None;
            self.connected = true;
        } else {
            // What is still owed now dates from this payment: the account
            // is keeping up, and the month starts again.
            self.owed_since_s = Some(at_s);
        }
        Ok(paid)
    }

    /// Apply the owner's rule at `now_s`, a month being `month_s` seconds.
    /// Returns whether the supply is on.
    pub fn review(&mut self, now_s: u64, month_s: u64) -> bool {
        if let Some(since) = self.owed_since_s {
            let a_month_owing = now_s.saturating_sub(since) >= month_s;
            let paid_this_month = self
                .last_payment_s
                .is_some_and(|paid| now_s.saturating_sub(paid) < month_s);
            if a_month_owing && !paid_this_month {
                self.connected = false;
            }
        }
        self.connected
    }
}

/// Pay out an enterprise's month: [`PAYOUT_SHARE`] of `profit` to its
/// shareholders by share, the rest kept by the business. A loss pays
/// nothing (the business bears it from its own account). Shares are whole
/// cents to each holder in order, with any cent left by rounding kept by the
/// business. Returns what was paid to each.
pub fn pay_dividends(
    ledger: &mut MoneyLedger,
    at_s: u64,
    enterprise: &Holder,
    profit: Cents,
    shareholders: &[(Holder, f64)],
) -> Result<Vec<(Holder, Cents)>, MoneyError> {
    if profit <= 0 {
        return Ok(Vec::new());
    }
    let pool = (profit as f64 * PAYOUT_SHARE).floor() as Cents;
    let paid: Vec<(Holder, Cents)> = shareholders
        .iter()
        .map(|(holder, share)| (holder.clone(), (pool as f64 * share).floor() as Cents))
        .filter(|(_, cents)| *cents > 0)
        .collect();
    // All or nobody: a business that cannot pay every holder pays none.
    let total: Cents = paid.iter().map(|(_, c)| c).sum();
    let held = ledger.balance(enterprise);
    if held < total {
        return Err(MoneyError::Insufficient {
            holder: enterprise.clone(),
            held,
            asked: total,
        });
    }
    for (holder, cents) in &paid {
        ledger.transfer(at_s, enterprise, holder, *cents, "dividend")?;
    }
    Ok(paid)
}

/// The owner's default ownership of every enterprise: Gem-D and Gem-K,
/// half each (2026-10-15).
pub fn founders_shares() -> Vec<(Holder, f64)> {
    vec![
        (Holder::Person("Gem-D".to_string()), 0.5),
        (Holder::Person("Gem-K".to_string()), 0.5),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn person(id: &str) -> Holder {
        Holder::Person(id.to_string())
    }

    #[test]
    fn transfers_move_money_and_never_make_it() {
        let mut ledger = MoneyLedger::default();
        ledger.issue(0, person("a"), 10_000, "opening").unwrap();
        ledger
            .transfer(1, &person("a"), &person("b"), 2_500, "rent")
            .unwrap();
        assert_eq!(ledger.balance(&person("a")), 7_500);
        assert_eq!(ledger.balance(&person("b")), 2_500);
        assert_eq!(
            ledger.transfer(2, &person("b"), &person("a"), 2_501, "too much"),
            Err(MoneyError::Insufficient {
                holder: person("b"),
                held: 2_500,
                asked: 2_501
            })
        );
        assert_eq!(
            ledger.balance(&person("b")),
            2_500,
            "a failed payment moves nothing"
        );
        assert!(ledger
            .transfer(3, &person("a"), &person("b"), 0, "")
            .is_err());
        ledger.audit().unwrap();
    }

    #[test]
    fn a_loan_is_new_money_and_repaying_it_retires_it() {
        let mut ledger = MoneyLedger::default();
        let rate = 0.0482;
        // Savings to pay the interest from.
        ledger.issue(0, person("v"), 50_000, "savings").unwrap();
        ledger.lend(0, person("v"), 1_000_000, rate, 12).unwrap();
        assert_eq!(ledger.balance(&person("v")), 1_050_000);
        ledger.audit().unwrap();
        for month in 1..=12 {
            ledger.service_loans(month);
            ledger.audit().unwrap();
        }
        assert_eq!(
            ledger.principal_outstanding(),
            0,
            "a year's instalments repay it"
        );
        // The interest went to the bank: about 4.82% of the average balance.
        let interest = ledger.balance(&Holder::Bank);
        assert!((25_000..28_000).contains(&interest), "interest {interest}");
        assert!(ledger.loans()[0].missed == 0, "{:?}", ledger.loans());
        // The borrower spent nothing, so what they hold is their savings less
        // the interest: the principal came and went.
        assert_eq!(ledger.balance(&person("v")), 50_000 - interest);
    }

    #[test]
    fn the_instalment_is_the_annuity_formula() {
        // $100,000 over 24 months at 4.82%: r = 0.0040167; payment =
        // P r / (1 - (1+r)^-24) = $4,379.09 (to the cent, rounded up).
        assert_eq!(annuity_instalment(10_000_000, 0.0482, 24), 437_909);
        assert_eq!(annuity_instalment(1_200, 0.0, 12), 100);
    }

    #[test]
    fn a_missed_instalment_adds_its_interest_and_is_counted() {
        let mut ledger = MoneyLedger::default();
        ledger.lend(0, person("v"), 1_000_000, 0.12, 12).unwrap();
        ledger
            .transfer(0, &person("v"), &person("shop"), 1_000_000, "spent it")
            .unwrap();
        ledger.service_loans(1);
        assert_eq!(ledger.loans()[0].missed, 1);
        assert_eq!(
            ledger.loans()[0].outstanding,
            1_010_000,
            "a month at 12% a year"
        );
        ledger.audit().unwrap();
    }

    #[test]
    fn a_month_unpaid_cuts_the_supply_but_paying_something_keeps_it() {
        let month = 1_000;
        // Stopped paying: cut off once the oldest bill is a month old.
        let mut stopped = BillingAccount::new();
        stopped.bill(0, 5_000);
        assert!(stopped.review(month - 1, month), "not yet a month");
        assert!(!stopped.review(month, month), "a month owing, nothing paid");

        // Behind, but paying regularly: stays on.
        let mut ledger = MoneyLedger::default();
        let (payer, supplier) = (person("h"), Holder::Enterprise("Gem Hydro".into()));
        ledger.issue(0, payer.clone(), 10_000, "opening").unwrap();
        let mut keeping_up = BillingAccount::new();
        keeping_up.bill(0, 5_000);
        keeping_up.bill(month, 5_000);
        for t in [month / 2, month + month / 2, 2 * month + month / 2] {
            keeping_up
                .pay(&mut ledger, t, &payer, &supplier, 1_000)
                .unwrap();
            assert!(
                keeping_up.review(t + month / 2, month),
                "paying something every month"
            );
        }
        assert!(keeping_up.owed > 0, "still behind");

        // Paid up: reconnected.
        stopped
            .pay(&mut ledger, 2 * month, &payer, &supplier, 5_000)
            .unwrap();
        assert!(stopped.connected && stopped.review(2 * month, month));
        ledger.audit().unwrap();
    }

    #[test]
    fn dividends_are_half_the_profit_split_by_share() {
        let mut ledger = MoneyLedger::default();
        let hydro = Holder::Enterprise("Gem Hydro".into());
        ledger
            .issue(0, hydro.clone(), 1_000_000, "takings")
            .unwrap();
        let paid = pay_dividends(&mut ledger, 1, &hydro, 400_001, &founders_shares()).unwrap();
        // Half of 400,001 is 200,000 (the odd cent stays); each founder 100,000.
        assert_eq!(
            paid,
            vec![(person("Gem-D"), 100_000), (person("Gem-K"), 100_000)]
        );
        assert_eq!(ledger.balance(&hydro), 800_000);
        assert!(
            pay_dividends(&mut ledger, 2, &hydro, -5, &founders_shares())
                .unwrap()
                .is_empty()
        );
        ledger.audit().unwrap();
    }

    #[test]
    fn the_founders_open_with_the_sourced_net_worth_and_the_bank_lends_at_the_sourced_rate() {
        let library = ReferenceLibrary::load(&crate::validation::default_reference_dir()).unwrap();
        let finance = FinanceReference::from_reference(&library).unwrap();
        assert_eq!(finance.founder_opening_cents, 52_900_000);
        assert_eq!(finance.lending_rate, 0.0482);
    }

    /// Capitalised arrears can lift a month's interest above the fixed
    /// instalment; a borrower who holds the instalment but not the interest
    /// then misses the month instead of the books failing.
    #[test]
    fn arrears_past_the_instalment_are_missed_not_a_panic() {
        let mut ledger = MoneyLedger::default();
        // 10% a month makes one missed month enough.
        ledger.lend(0, person("v"), 1_000_000, 1.2, 360).unwrap();
        ledger
            .transfer(0, &person("v"), &person("x"), 1_000_000, "spent")
            .unwrap();
        ledger.service_loans(1);
        let loan = ledger.loans()[0].clone();
        assert_eq!(loan.missed, 1);
        let interest = monthly_interest(loan.outstanding, loan.annual_rate);
        assert!(
            interest > loan.instalment,
            "{interest} vs {}",
            loan.instalment
        );
        let between = (loan.instalment + interest) / 2;
        ledger.issue(1, person("v"), between, "wages").unwrap();
        ledger.service_loans(2);
        assert_eq!(ledger.loans()[0].missed, 2);
        assert_eq!(ledger.balance(&person("v")), between, "nothing was taken");
        ledger.audit().unwrap();
        // With the interest in hand, the month is paid.
        ledger
            .issue(2, person("v"), interest, "more wages")
            .unwrap();
        ledger.service_loans(3);
        assert_eq!(ledger.loans()[0].missed, 2);
        ledger.audit().unwrap();
    }

    /// A business that cannot pay every shareholder pays none of them.
    #[test]
    fn dividends_are_paid_to_everyone_or_no_one() {
        let mut ledger = MoneyLedger::default();
        let shop = Holder::Enterprise("shop".to_string());
        ledger.issue(0, shop.clone(), 150, "takings").unwrap();
        // A 400-cent profit pays 100 to each founder; the shop holds 150.
        assert!(pay_dividends(&mut ledger, 1, &shop, 400, &founders_shares()).is_err());
        assert_eq!(ledger.balance(&shop), 150);
        assert_eq!(ledger.balance(&person("Gem-D")), 0);
        ledger.audit().unwrap();
    }

    #[test]
    fn a_default_billing_account_is_connected() {
        assert_eq!(BillingAccount::default(), BillingAccount::new());
        assert!(BillingAccount::default().connected);
    }

    #[test]
    fn the_ledger_saves_to_json_and_back() {
        let mut ledger = MoneyLedger::default();
        ledger.issue(0, person("a"), 10_000, "opening").unwrap();
        ledger.lend(0, person("a"), 5_000, 0.05, 12).unwrap();
        let json = serde_json::to_string(&ledger).unwrap();
        let back: MoneyLedger = serde_json::from_str(&json).unwrap();
        assert_eq!(back, ledger);
    }
}
