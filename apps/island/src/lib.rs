//! Maer-Ken Island.
//!
//! [`serve`] is everything the `island serve` command runs: the pages, the
//! read-only JSON API, the one write, and the rules about who may make it.
//! [`run`] is the headless side — advancing the island, saving it and
//! describing a saved one. Both live in the library so the binary and the
//! integration tests drive the same code.

pub mod run;
pub mod serve;
