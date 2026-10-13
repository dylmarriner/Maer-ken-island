//! The island's industry (Phase 4b): plants built from the physical world.
//!
//! - [`hydro`]: small hydropower design and output from a river's flow and
//!   head, after the European Small Hydropower Association's guide and
//!   RETScreen's turbine efficiency curves (`fixtures/reference/industry`).
//! - [`flow_duration`]: each perennial river cell's flow-duration curve from
//!   a simulated year.
//! - [`hydro_sites`]: run-of-river sites that support a plant.
//! - [`grid`]: overhead lines, their losses and a radial grid's dispatch.

pub mod flow_duration;
pub mod grid;
pub mod hydro;
pub mod hydro_sites;
