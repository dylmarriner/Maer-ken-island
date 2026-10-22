//! The island's money as `IslandLife` runs it (Phase 4b Task 7, D61, D62):
//! the founders' deposits, the bank, the businesses the island has, and the
//! monthly round of loan instalments, month closes and dividends, with the
//! money audit closed after each month.

use serde::{Deserialize, Serialize};

use super::enterprise::{Enterprise, EnterpriseError, MonthClose};
use super::money::{founders_shares, Cents, FinanceReference, Holder, MoneyError, MoneyLedger};

/// The founders, who own every business (the owner, 2026-10-15).
pub const FOUNDERS: [&str; 2] = ["Gem-D", "Gem-K"];

/// The business that runs the oil fields and the refinery (Task 4, D58).
pub const PETROLEUM: &str = "Gem Petroleum";

#[derive(Debug, Clone, PartialEq)]
pub enum IslandEconomyError {
    Money(MoneyError),
    Enterprise(EnterpriseError),
}

impl From<MoneyError> for IslandEconomyError {
    fn from(e: MoneyError) -> Self {
        Self::Money(e)
    }
}

impl From<EnterpriseError> for IslandEconomyError {
    fn from(e: EnterpriseError) -> Self {
        Self::Enterprise(e)
    }
}

/// Every account and business on the island, and when the month ends.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IslandEconomy {
    pub ledger: MoneyLedger,
    pub enterprises: Vec<Enterprise>,
    /// A month is a twelfth of the local year (D61).
    pub month_s: u64,
    pub next_month_s: u64,
}

impl IslandEconomy {
    /// Open the books at `now_s`: each founder's sourced opening deposit, and
    /// the named businesses, owned 50/50 by the founders.
    pub fn open(
        finance: &FinanceReference,
        year_s: u64,
        now_s: u64,
        enterprises: &[&str],
    ) -> Result<Self, IslandEconomyError> {
        let mut ledger = MoneyLedger::default();
        for founder in FOUNDERS {
            ledger.issue(
                now_s,
                Holder::Person(founder.to_string()),
                finance.founder_opening_cents,
                "opening net worth",
            )?;
        }
        let month_s = (year_s / 12).max(1);
        Ok(Self {
            ledger,
            enterprises: enterprises
                .iter()
                .map(|name| Enterprise::new(name, founders_shares()))
                .collect(),
            month_s,
            next_month_s: now_s + month_s,
        })
    }

    pub fn enterprise(&self, name: &str) -> Option<&Enterprise> {
        self.enterprises.iter().find(|e| e.name == name)
    }

    /// The estate buys `cents` of diesel from the petroleum business, the
    /// founders paying half each (their household is both of theirs). Takes
    /// what the founders can pay; with no petroleum business nothing moves.
    pub fn sell_diesel(&mut self, at_s: u64, cents: Cents) -> Result<Cents, IslandEconomyError> {
        let Some(company) = self.enterprises.iter_mut().find(|e| e.name == PETROLEUM) else {
            return Ok(0);
        };
        let seller = company.holder();
        let mut paid = 0;
        for (i, founder) in FOUNDERS.iter().enumerate() {
            let share = if i == 0 { cents - cents / 2 } else { cents / 2 };
            let buyer = Holder::Person(founder.to_string());
            if share > 0 && self.ledger.balance(&buyer) >= share {
                self.ledger
                    .transfer(at_s, &buyer, &seller, share, "diesel")?;
                paid += share;
            }
        }
        company.book_income(paid);
        Ok(paid)
    }

    /// Run every month that has ended by `now_s`: the bank takes its
    /// instalments, each business closes its month and pays its dividends,
    /// and the ledger must still balance.
    pub fn advance(&mut self, now_s: u64) -> Result<Vec<MonthClose>, IslandEconomyError> {
        let mut closes = Vec::new();
        while now_s >= self.next_month_s {
            let at_s = self.next_month_s;
            self.ledger.service_loans(at_s);
            for enterprise in &mut self.enterprises {
                closes.push(enterprise.close_month(&mut self.ledger, at_s)?);
            }
            self.ledger.audit()?;
            self.next_month_s += self.month_s;
        }
        Ok(closes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validation::{default_reference_dir, ReferenceLibrary};

    fn finance() -> FinanceReference {
        let library = ReferenceLibrary::load(&default_reference_dir()).unwrap();
        FinanceReference::from_reference(&library).unwrap()
    }

    #[test]
    fn the_books_open_with_the_founders_deposits_and_close_each_month() {
        let finance = finance();
        let year_s = 12 * 1_000;
        let mut economy = IslandEconomy::open(&finance, year_s, 0, &[PETROLEUM]).unwrap();
        assert_eq!(economy.ledger.issued(), 2 * finance.founder_opening_cents);
        assert!(
            economy.advance(999).unwrap().is_empty(),
            "the month runs on"
        );

        // A month of takings: half the profit goes to the founders.
        let company = economy.enterprise(PETROLEUM).unwrap().holder();
        economy
            .ledger
            .issue(500, company, 10_000, "takings")
            .unwrap();
        economy.enterprises[0].book_income(10_000);
        let closes = economy.advance(2_500).unwrap();
        assert_eq!(closes.len(), 2, "two months ended");
        assert_eq!(closes[0].profit, 10_000);
        assert_eq!(
            closes[0].dividends,
            vec![
                (Holder::Person("Gem-D".into()), 2_500),
                (Holder::Person("Gem-K".into()), 2_500)
            ]
        );
        assert_eq!(closes[1].profit, 0);

        // The estate's diesel: half from each founder, takings for the month.
        let before = economy.ledger.balance(&Holder::Person("Gem-D".into()));
        assert_eq!(economy.sell_diesel(2_600, 1_001).unwrap(), 1_001);
        assert_eq!(
            economy.ledger.balance(&Holder::Person("Gem-D".into())),
            before - 501
        );
        assert_eq!(economy.enterprise(PETROLEUM).unwrap().month_income, 1_001);
        assert_eq!(economy.next_month_s, 3_000);
        economy.ledger.audit().unwrap();
    }
}
