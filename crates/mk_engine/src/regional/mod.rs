//! The regional island world: everything that needs engine code on top of
//! the `mk_island` domain contract (dependency direction `mk_engine →
//! mk_island → mk_core`).

pub mod boundary;
pub mod climate;
pub mod create_human;
pub mod deposits;
pub mod ecology;
pub mod edge;
pub mod energy;
pub mod estate_layout;
pub mod geology;
pub mod geophysics;
pub mod human_store;
pub mod humans;
pub mod hydrology;
pub mod labour;
pub mod levels;
pub mod life;
pub mod local_vegetation;
pub mod materials;
pub mod ocean;
pub mod par;
pub mod physical;
pub mod property;
pub mod scheduler;
pub mod seismicity;
pub mod shape;
pub mod synoptic;
pub mod tectonics;
pub mod terrain;
pub mod tides;
pub mod volcanism;
pub mod weather;
pub mod world;
pub mod zonal;
