//! The regional island world: everything that needs engine code on top of
//! the `mk_island` domain contract (dependency direction `mk_engine →
//! mk_island → mk_core`).

pub mod boundary;
pub mod par;
pub mod tectonics;
