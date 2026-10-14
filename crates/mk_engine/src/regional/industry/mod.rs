//! The island's industry (Phase 4b): plants built from the physical world.
//!
//! - [`hydro`]: small hydropower design and output from a river's flow and
//!   head, after the European Small Hydropower Association's guide and
//!   RETScreen's turbine efficiency curves (`fixtures/reference/industry`).
//! - [`flow_duration`]: each perennial river cell's flow-duration curve from
//!   a simulated year.
//! - [`hydro_sites`]: run-of-river sites that support a plant.
//! - [`grid`]: overhead lines, their losses and a radial grid's dispatch.
//! - [`petroleum`]: oil and gas fields sized from New Zealand's own, in
//!   exponential decline, their carbon out of the crust.
//! - [`refinery`]: a topping refinery that distils crude into the cuts of a
//!   published assay and burns its own gas and residue.

pub mod flow_duration;
pub mod grid;
pub mod hydro;
pub mod hydro_sites;
pub mod petroleum;
pub mod refinery;
