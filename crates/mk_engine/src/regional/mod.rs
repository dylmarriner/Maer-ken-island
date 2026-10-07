//! The regional island world: everything that needs engine code on top of
//! the `mk_island` domain contract (dependency direction `mk_engine →
//! mk_island → mk_core`).

pub mod boundary;
pub mod climate;
pub mod deposits;
pub mod edge;
pub mod geology;
pub mod geophysics;
pub mod levels;
pub mod par;
pub mod seismicity;
pub mod shape;
pub mod tectonics;
pub mod terrain;
pub mod volcanism;
pub mod weather;
pub mod zonal;
