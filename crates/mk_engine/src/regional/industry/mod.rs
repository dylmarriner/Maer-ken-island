//! The island's industry (Phase 4b): plants built from the physical world.
//!
//! - [`hydro`]: small hydropower design and output from a river's flow and
//!   head, after the European Small Hydropower Association's guide and
//!   RETScreen's turbine efficiency curves (`fixtures/reference/industry`).
//! - [`chain`]: the fields and refinery run as one plant, its carbon auditable.
//! - [`construction`]: the reference house's timber wall frames, built from
//!   the mill's sawn timber with its carbon.
//! - [`farm`]: dairy and crop land yielding New Zealand's average per
//!   hectare.
//! - [`fishery`]: a fish stock in Schaefer surplus production, fished no
//!   faster than it grows.
//! - [`flow_duration`]: each perennial river cell's flow-duration curve from
//!   a simulated year.
//! - [`food_processing`]: flour, malt, rolled oats and dairy products at
//!   New Zealand's extraction rates.
//! - [`forestry`]: even-aged radiata pine stands clearfelled at New
//!   Zealand's average age and yield, never cutting more than they grow.
//! - [`hydro_sites`]: run-of-river sites that support a plant.
//! - [`grid`]: overhead lines, their losses and a radial grid's dispatch.
//! - [`livestock`]: sheep and beef breeding stock, lambs and steers
//!   finished at New Zealand's carcass weights.
//! - [`petroleum`]: oil and gas fields sized from New Zealand's own, in
//!   exponential decline, their carbon out of the crust.
//! - [`refinery`]: a topping refinery that distils crude into the cuts of a
//!   published assay and burns its own gas and residue.
//! - [`sawmill`]: softwood logs sawn at New Zealand's recovery, the rest
//!   shared as residues.

pub mod chain;
pub mod construction;
pub mod farm;
pub mod fishery;
pub mod flow_duration;
pub mod food_processing;
pub mod forestry;
pub mod grid;
pub mod hydro;
pub mod hydro_sites;
pub mod livestock;
pub mod petroleum;
pub mod refinery;
pub mod sawmill;
