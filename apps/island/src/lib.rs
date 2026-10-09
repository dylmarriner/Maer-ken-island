//! Maer-Ken Island.
//!
//! [`serve`] is everything the `island serve` command runs: the pages, the
//! read-only JSON API, the one write, and the rules about who may make it.
//! [`run`] is the headless side — advancing the island, saving it and
//! describing a saved one. Both live in the library so the binary and the
//! integration tests drive the same code.

// warp builds one type per route and `and`/`or` nests them, so the whole
// route tree is a single deeply generic type. The backend's gate, its
// CORS wrapper and its recover arm added three more layers and tipped the
// trait solver over its default limit of 128 -- in release only, where
// `Unpin` is proved through more of the future machinery. This is a limit
// on type nesting, not on anything at run time.
#![recursion_limit = "256"]

pub mod run;
pub mod serve;
