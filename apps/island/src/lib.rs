//! Maer-Ken Island's dashboard.
//!
//! [`serve`] is everything the `island serve` command runs: the pages, the
//! read-only JSON API, the one write, and the rules about who may make it.
//! It lives in the library so the binary and the integration tests drive the
//! same code.

pub mod serve;
