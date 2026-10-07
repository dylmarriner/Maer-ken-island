//! The regional island world: everything that needs engine code on top of
//! the `mk_island` domain contract (dependency direction `mk_engine →
//! mk_island → mk_core`).

pub mod boundary;
pub mod climate;
pub mod deposits;
pub mod ecology;
pub mod edge;
pub mod energy;
pub mod estate_layout;
pub mod geology;
pub mod geophysics;
pub mod hydrology;
pub mod levels;
pub mod local_vegetation;
pub mod ocean;
pub mod par;
pub mod physical;
pub mod property;
pub mod seismicity;
pub mod shape;
pub mod synoptic;
pub mod tectonics;
pub mod terrain;
pub mod tides;
pub mod volcanism;
pub mod weather;
pub mod zonal;
