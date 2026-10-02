//! Execution of typed operator interventions against live world state.
//!
//! `mk_interventions` defines *what* an intervention is (the typed action,
//! its author, its provenance, its validation and its permission gate). This
//! module is the other half: it takes a validated, permitted
//! [`InterventionAction`] and actually applies it to a [`WorldState`], or —
//! for the actions that are properties of the *run loop* rather than of world
//! state — returns a typed [`ControlDirective`] for the host to carry out.
//!
//! Before this module existed, `transparent_ui::attempt_intervention`
//! returned `LoggedAndGated` on every path including the `Allowed` branch,
//! and `mk_interventions::validate_intervention` had no call sites anywhere
//! in the workspace, so its bounds checks provided no runtime guarantee.
//!
//! # Ordering guarantees
//!
//! Every mutation here is a pure function of the action and the current world
//! state — no wall-clock reads, no thread RNG, no iteration over unordered
//! collections. Two identical interventions applied to identical world states
//! produce identical results, so an intervention does not break the
//! hash-chain reproducibility the verification tooling depends on.
//!
//! # Conservation
//!
//! Injections introduce matter or energy that did not come from anywhere
//! inside the simulated system. Every such injection is recorded as a flux
//! from [`Reservoir::OperatorIntervention`], which is a *boundary* reservoir,
//! and is passed through to a second boundary reservoir where the receiving
//! store is itself interior. That keeps [`mk_core::flux::Ledger::audit`]
//! closing on an intervention tick while leaving the inflow visible and
//! attributable in the ledger, rather than either silently breaking closure
//! or going unrecorded.

use std::path::{Path, PathBuf};

use mk_core::flux::{FluxEntry, FluxKind, Reservoir};
use mk_core::time::Tick;
use mk_interventions::{
    validate_intervention, BiomassType, ClimateParameter, DisturbanceType, EnergyType,
    HumanSpawnProfile, InterventionAction, InterventionPermissions, Location, Region, ResourceType,
    ScenarioModification, ScenarioValue, StructureKind, ValidationResult,
};

use crate::agents::GridPosition;
use crate::biosphere::evolution::SpeciesCategory;
use crate::disturbance::{ActiveDisturbance, DisturbanceKind};
use crate::resource_economy::{RecipeId, ResourceNode, ResourceNodeKind};
use crate::world_integration::WorldState;

/// Why an intervention could not be applied.
#[derive(Debug)]
pub enum InterventionError {
    /// The action failed `mk_interventions::validate_intervention`.
    Validation(ValidationResult),
    /// The caller's permissions do not cover this action.
    PermissionDenied { action: String },
    /// Refused by the Reverence Veto (see [`crate::governance`]).
    ReverenceVeto(crate::governance::ReverenceVetoViolation),
    /// Snapshot I/O failed while branching, forking or scrubbing.
    Snapshot(crate::io::snapshot::SnapshotError),
    /// The action requires a capability the executor was not configured with.
    NotConfigured { action: String, reason: String },
    /// The action named something that does not exist.
    NotFound { what: String },
    /// The action named a scenario parameter outside the supported set.
    UnknownScenarioParameter {
        parameter: String,
        supported: &'static [&'static str],
    },
    /// The action named a scenario parameter with the wrong value type.
    ScenarioTypeMismatch {
        parameter: String,
        expected: &'static str,
    },
    /// The world's physical state forbids the action at the requested place
    /// (for example, construction on terrain too steep to build on).
    PhysicallyBlocked { action: String, reason: String },
}

impl std::fmt::Display for InterventionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InterventionError::Validation(result) => {
                write!(
                    f,
                    "intervention failed validation: {}",
                    result.errors.join("; ")
                )
            }
            InterventionError::PermissionDenied { action } => {
                write!(f, "permission denied for intervention '{action}'")
            }
            InterventionError::ReverenceVeto(violation) => {
                write!(f, "refused by Reverence Veto: {violation}")
            }
            InterventionError::Snapshot(err) => write!(f, "snapshot operation failed: {err}"),
            InterventionError::NotConfigured { action, reason } => {
                write!(f, "intervention '{action}' is not available: {reason}")
            }
            InterventionError::NotFound { what } => write!(f, "not found: {what}"),
            InterventionError::UnknownScenarioParameter {
                parameter,
                supported,
            } => write!(
                f,
                "unknown scenario parameter '{parameter}'; supported: {}",
                supported.join(", ")
            ),
            InterventionError::ScenarioTypeMismatch {
                parameter,
                expected,
            } => write!(
                f,
                "scenario parameter '{parameter}' expects a {expected} value"
            ),
            InterventionError::PhysicallyBlocked { action, reason } => {
                write!(f, "'{action}' blocked by world state: {reason}")
            }
        }
    }
}

impl std::error::Error for InterventionError {}

impl From<crate::io::snapshot::SnapshotError> for InterventionError {
    fn from(err: crate::io::snapshot::SnapshotError) -> Self {
        InterventionError::Snapshot(err)
    }
}

impl From<crate::governance::ReverenceVetoViolation> for InterventionError {
    fn from(err: crate::governance::ReverenceVetoViolation) -> Self {
        InterventionError::ReverenceVeto(err)
    }
}

/// An instruction for the *host run loop*, which owns stepping and state
/// replacement and therefore cannot be driven from inside `WorldState`.
///
/// This is not a deferral of the work: the executor has already validated the
/// action, checked permissions, and (for the snapshot-backed variants)
/// performed the snapshot I/O, so the host's remaining job is mechanical.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlDirective {
    /// Stop advancing the simulation.
    Pause,
    /// Resume advancing the simulation.
    Resume,
    /// Advance exactly `ticks` ticks, then pause.
    Step { ticks: u64 },
    /// Replace world state with the snapshot at `path`, which the executor
    /// has already confirmed exists and covers `target_tick`.
    Scrub { target_tick: Tick, path: PathBuf },
    /// A snapshot of the pre-branch state has been written to `path`; the
    /// host should register `name` as a branch rooted there.
    Branch { name: String, path: PathBuf },
    /// As [`ControlDirective::Branch`], and the listed modifications have
    /// already been applied to the live state after the snapshot was taken.
    Fork {
        name: String,
        path: PathBuf,
        applied: Vec<String>,
    },
}

/// What an intervention actually did.
#[derive(Debug, Clone)]
pub enum AppliedOutcome {
    /// World state was mutated in place.
    Mutated {
        /// Concrete description of the change, suitable for an audit trail.
        summary: String,
        /// Number of grid cells or entities touched.
        touched: usize,
    },
    /// The action is a run-loop control; `directive` tells the host what to do.
    Control { directive: ControlDirective },
}

impl AppliedOutcome {
    /// Audit-trail description of the outcome.
    pub fn summary(&self) -> String {
        match self {
            AppliedOutcome::Mutated { summary, .. } => summary.clone(),
            AppliedOutcome::Control { directive } => format!("control directive: {directive:?}"),
        }
    }
}

/// Scenario parameters `ModifyScenario` can set.
///
/// An explicit allowlist rather than a free-form string map: an unrecognised
/// parameter is an error, not a silently-accepted no-op.
pub const SUPPORTED_SCENARIO_PARAMETERS: &[&str] = &[
    "climate.co2_concentration_ppm",
    "biosphere.current_time_myr",
    "resource_economy.regeneration_multiplier",
];

/// Applies interventions to a world.
///
/// Holds the caller's permissions and the snapshot directory the
/// snapshot-backed actions need. Construct one per operator session so the
/// permission set travels with the caller rather than being re-derived.
#[derive(Debug, Clone)]
pub struct InterventionExecutor {
    permissions: InterventionPermissions,
    snapshot_dir: Option<PathBuf>,
}

impl InterventionExecutor {
    /// Executor with the given permissions and no snapshot directory.
    ///
    /// The snapshot-backed actions (`Branch`, `Fork`, `Scrub`) will return
    /// [`InterventionError::NotConfigured`] until
    /// [`InterventionExecutor::with_snapshot_dir`] supplies one.
    pub fn new(permissions: InterventionPermissions) -> Self {
        Self {
            permissions,
            snapshot_dir: None,
        }
    }

    /// Set the directory snapshots are written to and read from.
    pub fn with_snapshot_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.snapshot_dir = Some(dir.into());
        self
    }

    pub fn permissions(&self) -> &InterventionPermissions {
        &self.permissions
    }

    /// Validate, permission-check and apply `action`.
    ///
    /// The order is deliberate and is the guarantee this function offers:
    /// validation before permission before mutation, so a malformed action is
    /// rejected the same way regardless of who sent it, and no partial
    /// mutation is ever applied from an action that fails either gate.
    pub fn execute(
        &self,
        world: &mut WorldState,
        action: &InterventionAction,
    ) -> Result<AppliedOutcome, InterventionError> {
        let validation = validate_intervention(action);
        if !validation.valid {
            return Err(InterventionError::Validation(validation));
        }

        if !self.permissions.can_execute(action) {
            return Err(InterventionError::PermissionDenied {
                action: action_name(action).to_string(),
            });
        }

        let outcome = self.apply(world, action)?;
        if let AppliedOutcome::Mutated { summary, .. } = &outcome {
            world.chronicle.record(
                world.tick,
                crate::chronicle::ChronicleKind::Intervention,
                format!("operator {summary}"),
                0.5,
            );
        }
        Ok(outcome)
    }

    fn apply(
        &self,
        world: &mut WorldState,
        action: &InterventionAction,
    ) -> Result<AppliedOutcome, InterventionError> {
        match action {
            InterventionAction::Pause => Ok(control(ControlDirective::Pause)),
            InterventionAction::Resume => Ok(control(ControlDirective::Resume)),
            InterventionAction::Step { ticks } => {
                Ok(control(ControlDirective::Step { ticks: *ticks }))
            }
            InterventionAction::Scrub { target_tick } => self.scrub(*target_tick),
            InterventionAction::Branch {
                name,
                source_snapshot,
            } => self.branch(world, name, source_snapshot),
            InterventionAction::Fork {
                name,
                modifications,
            } => self.fork(world, name, modifications),
            InterventionAction::InjectResource {
                resource_type,
                amount,
                location,
            } => inject_resource(world, resource_type, *amount, location),
            InterventionAction::InjectBiomass {
                biomass_type,
                amount,
                region,
            } => inject_biomass(world, biomass_type, *amount, region),
            InterventionAction::InjectEnergy {
                energy_type,
                amount,
                location,
            } => inject_energy(world, energy_type, *amount, location),
            InterventionAction::ModifyClimate {
                parameter,
                value,
                region,
            } => modify_climate(world, parameter, *value, region.as_ref()),
            InterventionAction::TriggerDisturbance {
                disturbance_type,
                intensity,
                location,
            } => trigger_disturbance(world, disturbance_type, *intensity, location),
            InterventionAction::SculptTerrain {
                elevation_delta_m,
                region,
            } => sculpt_terrain(world, *elevation_delta_m, region),
            InterventionAction::SmoothTerrain { strength, region } => {
                smooth_terrain(world, *strength, region)
            }
            InterventionAction::ModifyScenario { parameter, value } => {
                modify_scenario(world, parameter, value)
            }
            InterventionAction::SpawnHuman {
                template_id,
                location,
                profile,
            } => spawn_human(world, template_id, location, profile.as_ref()),
            InterventionAction::RemoveHuman { human_id } => remove_human(world, human_id),
            InterventionAction::ConstructStructure {
                structure,
                location,
            } => construct_structure(world, *structure, location),
        }
    }

    fn snapshot_dir(&self, action: &'static str) -> Result<&Path, InterventionError> {
        self.snapshot_dir
            .as_deref()
            .ok_or_else(|| InterventionError::NotConfigured {
                action: action.to_string(),
                reason: "no snapshot directory configured; \
                         build the executor with `with_snapshot_dir`"
                    .to_string(),
            })
    }

    fn scrub(&self, target_tick: Tick) -> Result<AppliedOutcome, InterventionError> {
        let dir = self.snapshot_dir("Scrub")?;
        let path = snapshot_path(dir, &format!("tick_{target_tick}"));
        if !path.exists() {
            return Err(InterventionError::NotFound {
                what: format!("snapshot for tick {target_tick} at {}", path.display()),
            });
        }
        // Confirm the file is loadable and really is the requested tick
        // before handing the host a directive it would fail on.
        let (_, state) = crate::io::snapshot::load_snapshot(&path)?;
        if state.tick != target_tick {
            return Err(InterventionError::NotFound {
                what: format!(
                    "snapshot at {} holds tick {}, not {target_tick}",
                    path.display(),
                    state.tick
                ),
            });
        }
        Ok(control(ControlDirective::Scrub { target_tick, path }))
    }

    fn branch(
        &self,
        world: &WorldState,
        name: &str,
        source_snapshot: &str,
    ) -> Result<AppliedOutcome, InterventionError> {
        let dir = self.snapshot_dir("Branch")?;
        // `source_snapshot` names the lineage root the operator believes they
        // are branching from; record it in the branch file name so the
        // written snapshot carries its own provenance.
        let label = if source_snapshot.is_empty() {
            format!("branch_{name}_tick_{}", world.tick)
        } else {
            format!("branch_{name}_from_{source_snapshot}_tick_{}", world.tick)
        };
        let path = snapshot_path(dir, &sanitize_label(&label));
        write_snapshot(world, &path)?;
        Ok(control(ControlDirective::Branch {
            name: name.to_string(),
            path,
        }))
    }

    fn fork(
        &self,
        world: &mut WorldState,
        name: &str,
        modifications: &[ScenarioModification],
    ) -> Result<AppliedOutcome, InterventionError> {
        let dir = self.snapshot_dir("Fork")?;
        let path = snapshot_path(
            dir,
            &sanitize_label(&format!("fork_{name}_tick_{}", world.tick)),
        );
        // Snapshot the pre-fork state first: if a modification fails, the
        // operator still has the exact state the fork was attempted from.
        write_snapshot(world, &path)?;

        let mut applied = Vec::with_capacity(modifications.len());
        for modification in modifications {
            let outcome = modify_scenario(world, &modification.parameter, &modification.new_value)?;
            applied.push(outcome.summary());
        }

        Ok(control(ControlDirective::Fork {
            name: name.to_string(),
            path,
            applied,
        }))
    }
}

fn control(directive: ControlDirective) -> AppliedOutcome {
    AppliedOutcome::Control { directive }
}

fn mutated(summary: String, touched: usize) -> AppliedOutcome {
    AppliedOutcome::Mutated { summary, touched }
}

fn snapshot_path(dir: &Path, label: &str) -> PathBuf {
    dir.join(format!("{label}.mkss"))
}

/// Restrict a caller-supplied label to characters safe in a file name, so an
/// operator-chosen branch name cannot escape the snapshot directory.
fn sanitize_label(label: &str) -> String {
    let cleaned: String = label
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if cleaned.is_empty() {
        "unnamed".to_string()
    } else {
        cleaned
    }
}

fn write_snapshot(world: &WorldState, path: &Path) -> Result<(), InterventionError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| InterventionError::NotConfigured {
            action: "snapshot".to_string(),
            reason: format!(
                "cannot create snapshot directory {}: {err}",
                parent.display()
            ),
        })?;
    }
    crate::io::snapshot::save_snapshot(world, path)?;
    Ok(())
}

fn action_name(action: &InterventionAction) -> &'static str {
    match action {
        InterventionAction::Pause => "Pause",
        InterventionAction::Resume => "Resume",
        InterventionAction::Step { .. } => "Step",
        InterventionAction::Scrub { .. } => "Scrub",
        InterventionAction::Branch { .. } => "Branch",
        InterventionAction::Fork { .. } => "Fork",
        InterventionAction::InjectResource { .. } => "InjectResource",
        InterventionAction::InjectBiomass { .. } => "InjectBiomass",
        InterventionAction::InjectEnergy { .. } => "InjectEnergy",
        InterventionAction::ModifyClimate { .. } => "ModifyClimate",
        InterventionAction::TriggerDisturbance { .. } => "TriggerDisturbance",
        InterventionAction::SculptTerrain { .. } => "SculptTerrain",
        InterventionAction::SmoothTerrain { .. } => "SmoothTerrain",
        InterventionAction::ModifyScenario { .. } => "ModifyScenario",
        InterventionAction::SpawnHuman { .. } => "SpawnHuman",
        InterventionAction::RemoveHuman { .. } => "RemoveHuman",
        InterventionAction::ConstructStructure { .. } => "ConstructStructure",
    }
}

/// Resolve an intervention [`Location`] to a grid cell of `world`.
fn cell_for(world: &WorldState, location: &Location) -> (usize, usize) {
    world
        .grid_spec
        .cell_for_lat_lon(location.latitude as f64, location.longitude as f64)
}

/// Cells covered by a [`Region`], derived from its real `radius_km` against
/// the planet's actual circumference — not a fixed cell count.
fn cells_for_region(world: &WorldState, region: &Region) -> Vec<(usize, usize)> {
    let spec = &world.grid_spec;
    let (row, col) = cell_for(world, &region.center);

    // Planet radius as used by `GridSpec::cell_area_m2`.
    const PLANET_RADIUS_M: f64 = 1.9113e7;
    let metres_per_row = std::f64::consts::PI * PLANET_RADIUS_M / spec.nlat() as f64;
    let radius_m = (region.radius_km as f64).max(0.0) * 1000.0;
    let radius_rows = (radius_m / metres_per_row).floor() as isize;

    let mut cells = Vec::new();
    for drow in -radius_rows..=radius_rows {
        let r = row as isize + drow;
        if r < 0 || r >= spec.nlat() as isize {
            continue;
        }
        // Longitudinal extent grows towards the poles for a fixed ground
        // distance, because the cells get narrower.
        let lat = spec.lat_rad(r as usize);
        let metres_per_col =
            (2.0 * std::f64::consts::PI * PLANET_RADIUS_M * lat.cos()).abs() / spec.nlon() as f64;
        let radius_cols = if metres_per_col > 0.0 {
            (radius_m / metres_per_col).floor() as isize
        } else {
            0
        };
        // Never wrap more than once around the planet.
        let radius_cols = radius_cols.min(spec.nlon() as isize / 2);
        for dcol in -radius_cols..=radius_cols {
            let c = (col as isize + dcol).rem_euclid(spec.nlon() as isize);
            cells.push((r as usize, c as usize));
        }
    }
    if cells.is_empty() {
        cells.push((row, col));
    }
    cells.sort_unstable();
    cells.dedup();
    cells
}

/// Node kinds a given resource type materialises as in the resource economy.
fn node_kinds_for(resource_type: &ResourceType) -> &'static [ResourceNodeKind] {
    match resource_type {
        ResourceType::Minerals => &[
            ResourceNodeKind::StoneOutcrop,
            ResourceNodeKind::IronVein,
            ResourceNodeKind::CopperVein,
            ResourceNodeKind::ClayDeposit,
            ResourceNodeKind::SandBank,
        ],
        ResourceType::Organic => &[
            ResourceNodeKind::Tree,
            ResourceNodeKind::FibrePatch,
            ResourceNodeKind::FoodPatch,
        ],
        // Bioavailable nutrients show up in-world as forageable food and
        // fibre, which is what agents actually consume.
        ResourceType::Nutrients => &[ResourceNodeKind::FoodPatch, ResourceNodeKind::FibrePatch],
        // Water and Energy have no node representation; they are applied to
        // the physical stores instead (see `inject_resource`).
        ResourceType::Water | ResourceType::Energy => &[],
    }
}

fn inject_resource(
    world: &mut WorldState,
    resource_type: &ResourceType,
    amount: f32,
    location: &Location,
) -> Result<AppliedOutcome, InterventionError> {
    let (row, col) = cell_for(world, location);
    let amount = amount as f64;

    match resource_type {
        ResourceType::Water => {
            // Water goes to the real hydrology store, overflowing to surface
            // water past field capacity exactly as a flood does.
            const FIELD_CAPACITY_MM: f64 = 300.0;
            let Some(soil) = world.hydrology_state.soil_water.get_mut_safe(row, col) else {
                return Err(InterventionError::NotFound {
                    what: format!("grid cell ({row}, {col})"),
                });
            };
            let headroom = (FIELD_CAPACITY_MM - soil.storage_mm).max(0.0);
            let into_soil = amount.min(headroom);
            soil.storage_mm += into_soil;
            soil.moisture_fraction = (soil.storage_mm / FIELD_CAPACITY_MM).clamp(0.0, 1.0);
            let overflow = amount - into_soil;
            if overflow > 0.0 {
                if let Some(surface) = world.hydrology_state.surface_water.get_mut_safe(row, col) {
                    *surface += overflow;
                }
            }
            // 1 mm over 1 m² is 1 kg. What the soil keeps enters the soil
            // store; the overflow stands as surface water.
            let area = world
                .grid_spec
                .cell_area_at_row_m2(row, world.canon.planet_radius_m)
                .unwrap_or(0.0);
            for (sink, kg) in [
                (Reservoir::SoilWater, into_soil * area),
                (Reservoir::SurfaceWater, overflow.max(0.0) * area),
            ] {
                if kg > 0.0 {
                    world.ledger.push(FluxEntry::new(
                        Reservoir::OperatorIntervention,
                        sink,
                        kg,
                        FluxKind::Water,
                    ));
                }
            }
            Ok(mutated(
                format!("injected {amount:.3} mm of water at cell ({row}, {col})"),
                1,
            ))
        }
        ResourceType::Energy => inject_energy_at_cell(world, row, col, amount, "resource"),
        kind => {
            let kinds = node_kinds_for(kind);
            let added = add_to_nodes(world, row, col, kinds, amount);
            Ok(mutated(
                format!("added {added} units of {kind:?} to resource nodes at cell ({row}, {col})"),
                1,
            ))
        }
    }
}

/// Top up existing nodes of the given kinds at a cell, creating one if the
/// cell has none.
///
/// Returns the number of units actually added, which is bounded by node
/// capacity — so this reports the real effect, not the requested amount.
fn add_to_nodes(
    world: &mut WorldState,
    row: usize,
    col: usize,
    kinds: &[ResourceNodeKind],
    amount: f64,
) -> u32 {
    let requested = amount.clamp(0.0, u32::MAX as f64) as u32;
    if requested == 0 || kinds.is_empty() {
        return 0;
    }
    let position = GridPosition::new(row as i32, col as i32);

    let mut matching: Vec<&mut ResourceNode> = world
        .resource_economy_state
        .nodes
        .iter_mut()
        .filter(|node| node.position == position && kinds.contains(&node.kind))
        .collect();

    if matching.is_empty() {
        // No node of a compatible kind here: materialise one. Capacity is the
        // injected amount, so the node reflects exactly what was placed
        // rather than an invented ceiling.
        let id = world
            .resource_economy_state
            .nodes
            .iter()
            .map(|node| node.id)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        world.resource_economy_state.nodes.push(ResourceNode {
            id,
            position,
            kind: kinds[0],
            available: requested,
            capacity: requested,
            regeneration_per_tick: 0,
        });
        // Keep node order stable so the world hash does not depend on
        // injection history.
        world
            .resource_economy_state
            .nodes
            .sort_by_key(|node| (node.position.row, node.position.col, node.id));
        return requested;
    }

    // Spread the injection evenly, then give any remainder to the first node.
    matching.sort_by_key(|node| node.id);
    let share = requested / matching.len() as u32;
    let mut remainder = requested % matching.len() as u32;
    let mut added = 0u32;
    for node in matching.iter_mut() {
        let mut want = share;
        if remainder > 0 {
            want += 1;
            remainder -= 1;
        }
        // Raise capacity to admit the injection rather than silently
        // discarding it at the old ceiling.
        node.capacity = node.capacity.saturating_add(want);
        let before = node.available;
        node.available = node.available.saturating_add(want).min(node.capacity);
        added += node.available - before;
    }
    added
}

fn inject_energy(
    world: &mut WorldState,
    energy_type: &EnergyType,
    amount: f32,
    location: &Location,
) -> Result<AppliedOutcome, InterventionError> {
    let (row, col) = cell_for(world, location);
    let label = match energy_type {
        EnergyType::Solar => "solar",
        EnergyType::Thermal => "thermal",
        EnergyType::Chemical => "chemical",
    };
    inject_energy_at_cell(world, row, col, amount as f64, label)
}

/// Convert injected joules into a surface-temperature rise at one cell.
///
/// Uses the cell's true spherical area and the climate model's areal heat
/// capacity for its surface (ocean mixed layer or land), so the temperature
/// change is derived from the injected energy rather than being a free
/// parameter.
fn inject_energy_at_cell(
    world: &mut WorldState,
    row: usize,
    col: usize,
    joules: f64,
    label: &str,
) -> Result<AppliedOutcome, InterventionError> {
    let area = world
        .grid_spec
        .cell_area_at_row_m2(row, world.canon.planet_radius_m)
        .filter(|area| *area > 0.0)
        .ok_or_else(|| InterventionError::NotFound {
            what: format!("grid cell ({row}, {col}) with a non-zero area"),
        })?;
    let height = world
        .elevation_grid
        .get_safe(row, col)
        .copied()
        .unwrap_or(0.0);
    let delta_k = joules / (area * crate::climate::surface_heat_capacity_j_m2_k(height));

    let Some(temp) = world
        .climate_state
        .surface_temperature
        .get_mut_safe(row, col)
    else {
        return Err(InterventionError::NotFound {
            what: format!("grid cell ({row}, {col})"),
        });
    };
    *temp += delta_k;

    // The energy enters the surface layer it warms. The climate step then
    // radiates it away and books that loss against the cooling it measures.
    world.ledger.push(FluxEntry::new(
        Reservoir::OperatorIntervention,
        crate::conservation::heat_reservoir(height),
        joules,
        FluxKind::Energy,
    ));

    Ok(mutated(
        format!(
            "injected {joules:.3} J of {label} energy at cell ({row}, {col}), \
             warming it {delta_k:.6} K"
        ),
        1,
    ))
}

/// Species categories that make up each trophic group.
fn categories_for(biomass_type: &BiomassType) -> &'static [SpeciesCategory] {
    match biomass_type {
        BiomassType::Producers => &[SpeciesCategory::Tree, SpeciesCategory::Flower],
        BiomassType::Consumers => &[
            SpeciesCategory::Herbivore,
            SpeciesCategory::Omnivore,
            SpeciesCategory::Insect,
            SpeciesCategory::Bee,
            SpeciesCategory::Bird,
            SpeciesCategory::Aquatic,
            SpeciesCategory::Amphibious,
            SpeciesCategory::Flying,
        ],
        BiomassType::Apex => &[SpeciesCategory::Carnivore],
        BiomassType::Decomposers => &[SpeciesCategory::Fungoid],
    }
}

/// Terrain affinities compatible with a biome, used to keep a regional
/// biomass injection from landing on species that could not live there.
fn affinities_for_biome(
    biome: mk_core::biomes::BiomeType,
) -> &'static [crate::biosphere::evolution::TerrainAffinity] {
    use crate::biosphere::evolution::TerrainAffinity as TA;
    use mk_core::biomes::BiomeType as B;
    match biome {
        B::DeepOcean | B::ShallowOcean | B::CoastalWaters => &[TA::OpenOcean],
        B::ReefSea => &[TA::ReefSea, TA::OpenOcean],
        B::River => &[TA::River, TA::Wetland],
        B::Wetland => &[TA::Wetland, TA::River],
        B::Volcanic => &[TA::Volcanic, TA::Mountain],
        B::Alpine | B::MontaneForest => &[TA::Mountain, TA::Cliff, TA::Hill],
        B::TropicalRainforest
        | B::TropicalDryForest
        | B::TemperateForest
        | B::BorealForest
        | B::Woodland => &[TA::ForestCanopy, TA::Hill, TA::Plains],
        B::Desert | B::SemiDesert | B::Savanna | B::Grassland | B::Shrubland | B::Tundra => {
            &[TA::Plains, TA::Hill]
        }
        // Permanent ice supports no terrestrial or marine affinity in this
        // model, so a regional injection targeting only ice sheets finds no
        // eligible species and is reported as such rather than applied.
        B::IceSheet => &[],
    }
}

fn inject_biomass(
    world: &mut WorldState,
    biomass_type: &BiomassType,
    amount: f32,
    region: &Region,
) -> Result<AppliedOutcome, InterventionError> {
    let cells = cells_for_region(world, region);
    let categories = categories_for(biomass_type);

    // Which terrain affinities the region can actually support, from the real
    // biome classification of its cells.
    let mut affinities: Vec<crate::biosphere::evolution::TerrainAffinity> = Vec::new();
    for (row, col) in &cells {
        if let Some(biome) = world.biome_grid.get_safe(*row, *col) {
            for affinity in affinities_for_biome(*biome) {
                if !affinities.contains(affinity) {
                    affinities.push(affinity.clone());
                }
            }
        }
    }

    let mut eligible: Vec<usize> = world
        .biosphere_state
        .species
        .iter()
        .enumerate()
        .filter(|(_, species)| {
            categories.contains(&species.category) && affinities.contains(&species.terrain_affinity)
        })
        .map(|(index, _)| index)
        .collect();

    if eligible.is_empty() {
        return Err(InterventionError::NotFound {
            what: format!(
                "no {biomass_type:?} species whose terrain affinity is supported by \
                 the target region's biomes"
            ),
        });
    }

    // Stable order by species_id so the split does not depend on Vec order.
    eligible.sort_by_key(|index| world.biosphere_state.species[*index].species_id);

    let carbon_before = crate::conservation::biomass_carbon_kg(&world.biosphere_state.species);
    let nutrients_before =
        crate::conservation::biomass_nutrients_kg(&world.biosphere_state.species, &world.canon);
    let total = amount.max(0.0) as u64;
    let share = total / eligible.len() as u64;
    let mut remainder = total % eligible.len() as u64;
    let mut added_total = 0u64;
    for index in &eligible {
        let mut want = share;
        if remainder > 0 {
            want += 1;
            remainder -= 1;
        }
        let species = &mut world.biosphere_state.species[*index];
        species.population_size = species.population_size.saturating_add(want);
        added_total += want;
    }

    // Biomass introduced from outside the system carries its carbon (kg C
    // in the new individuals' bodies) with it, and stays in the biomass.
    let carbon_added =
        crate::conservation::biomass_carbon_kg(&world.biosphere_state.species) - carbon_before;
    if carbon_added > 0.0 {
        world.ledger.push(FluxEntry::new(
            Reservoir::OperatorIntervention,
            Reservoir::BiomassCarbon,
            carbon_added,
            FluxKind::Carbon,
        ));
    }
    // ...and the nitrogen and phosphorus in their tissue.
    let (nitrogen, phosphorus) =
        crate::conservation::biomass_nutrients_kg(&world.biosphere_state.species, &world.canon);
    for (reservoir, kind, added) in [
        (
            Reservoir::BiomassNitrogen,
            FluxKind::Nitrogen,
            nitrogen - nutrients_before.0,
        ),
        (
            Reservoir::BiomassPhosphorus,
            FluxKind::Phosphorus,
            phosphorus - nutrients_before.1,
        ),
    ] {
        if added > 0.0 {
            world.ledger.push(FluxEntry::new(
                Reservoir::OperatorIntervention,
                reservoir,
                added,
                kind,
            ));
        }
    }

    Ok(mutated(
        format!(
            "added {added_total} {biomass_type:?} individuals across {} species \
             in {} cells",
            eligible.len(),
            cells.len()
        ),
        eligible.len(),
    ))
}

fn modify_climate(
    world: &mut WorldState,
    parameter: &ClimateParameter,
    value: f32,
    region: Option<&Region>,
) -> Result<AppliedOutcome, InterventionError> {
    let cells = match region {
        Some(region) => cells_for_region(world, region),
        None => {
            // No region means globally: every cell.
            let spec = &world.grid_spec;
            (0..spec.nlat())
                .flat_map(|row| (0..spec.nlon()).map(move |col| (row, col)))
                .collect()
        }
    };
    let value = value as f64;
    let mut touched = 0usize;

    match parameter {
        ClimateParameter::Temperature => {
            let radius = world.canon.planet_radius_m;
            let (mut land_j, mut ocean_j) = (0.0, 0.0);
            for (row, col) in &cells {
                let height = world
                    .elevation_grid
                    .get_safe(*row, *col)
                    .copied()
                    .unwrap_or(0.0);
                let area = world
                    .grid_spec
                    .cell_area_at_row_m2(*row, radius)
                    .unwrap_or(0.0);
                if let Some(temp) = world
                    .climate_state
                    .surface_temperature
                    .get_mut_safe(*row, *col)
                {
                    // Absolute set, in kelvin: the parameter names a target
                    // temperature, not an offset.
                    let joules = (value - *temp)
                        * area
                        * crate::climate::surface_heat_capacity_j_m2_k(height);
                    *temp = value;
                    touched += 1;
                    match crate::conservation::heat_reservoir(height) {
                        Reservoir::OceanHeat => ocean_j += joules,
                        _ => land_j += joules,
                    }
                }
            }
            // The operator adds or removes that heat from outside the system.
            for (reservoir, joules) in [
                (Reservoir::SurfaceEnergy, land_j),
                (Reservoir::OceanHeat, ocean_j),
            ] {
                crate::conservation::book_signed(
                    &mut world.ledger,
                    FluxKind::Energy,
                    Reservoir::OperatorIntervention,
                    reservoir,
                    Reservoir::OperatorIntervention,
                    joules,
                );
            }
        }
        ClimateParameter::Precipitation => {
            for (row, col) in &cells {
                if let Some(precip) = world.weather_state.precipitation.get_mut_safe(*row, *col) {
                    *precip = value.max(0.0);
                    touched += 1;
                }
            }
        }
        ClimateParameter::Humidity => {
            for (row, col) in &cells {
                if let Some(moisture) = world.weather_state.moisture.get_mut_safe(*row, *col) {
                    *moisture = value.max(0.0);
                    touched += 1;
                }
            }
        }
        ClimateParameter::WindSpeed => {
            for (row, col) in &cells {
                if let Some(wind) = world.weather_state.wind.get_mut_safe(*row, *col) {
                    // Rescale the existing direction to the requested speed,
                    // so setting a speed does not silently also set a
                    // direction. A calm cell has no direction to preserve, so
                    // it becomes a due-east wind.
                    let speed = (wind.u_east * wind.u_east + wind.v_north * wind.v_north).sqrt();
                    if speed > 0.0 {
                        let scale = value / speed;
                        wind.u_east *= scale;
                        wind.v_north *= scale;
                    } else {
                        wind.u_east = value;
                        wind.v_north = 0.0;
                    }
                    touched += 1;
                }
            }
        }
    }

    Ok(mutated(
        format!("set {parameter:?} to {value} across {touched} cells"),
        touched,
    ))
}

fn trigger_disturbance(
    world: &mut WorldState,
    disturbance_type: &DisturbanceType,
    intensity: f32,
    location: &Location,
) -> Result<AppliedOutcome, InterventionError> {
    let (row, col) = cell_for(world, location);
    let kind = match disturbance_type {
        DisturbanceType::Fire => DisturbanceKind::Fire,
        DisturbanceType::Flood => DisturbanceKind::Flood,
        DisturbanceType::VolcanicEruption => DisturbanceKind::VolcanicEruption,
        DisturbanceType::MeteorImpact => DisturbanceKind::MeteorImpact,
        DisturbanceType::Disease => DisturbanceKind::Disease,
        DisturbanceType::Drought => DisturbanceKind::Drought,
    };
    let disturbance = ActiveDisturbance::new(kind, row, col, intensity as f64, world.tick);
    let expiry = disturbance.expiry_tick;
    world.disturbance_state.push(disturbance);

    Ok(mutated(
        format!(
            "registered {} disturbance at cell ({row}, {col}) with intensity \
             {intensity:.3}, active until tick {expiry}",
            kind.as_str()
        ),
        1,
    ))
}

/// Great-circle distance (m) between two cells' centres.
fn cell_distance_m(world: &WorldState, a: (usize, usize), b: (usize, usize)) -> f64 {
    const PLANET_RADIUS_M: f64 = 1.9113e7;
    let spec = &world.grid_spec;
    let (lat1, lon1) = (spec.lat_rad(a.0), spec.lon_rad(a.1));
    let (lat2, lon2) = (spec.lat_rad(b.0), spec.lon_rad(b.1));
    let h = ((lat2 - lat1) / 2.0).sin().powi(2)
        + lat1.cos() * lat2.cos() * ((lon2 - lon1) / 2.0).sin().powi(2);
    2.0 * PLANET_RADIUS_M * h.sqrt().min(1.0).asin()
}

/// Raised-cosine brush weight for `cell`: 1 at the region centre, easing
/// smoothly to 0 at the region edge, so edits read as landforms rather than
/// stepped plateaus.
fn brush_weight(
    world: &WorldState,
    centre: (usize, usize),
    cell: (usize, usize),
    region: &Region,
) -> f64 {
    let radius_m = (region.radius_km as f64) * 1000.0;
    let distance = cell_distance_m(world, centre, cell);
    if radius_m > 0.0 && distance < radius_m {
        0.5 * (1.0 + (std::f64::consts::PI * distance / radius_m).cos())
    } else if cell == centre {
        1.0
    } else {
        0.0
    }
}

fn smooth_terrain(
    world: &mut WorldState,
    strength: f32,
    region: &Region,
) -> Result<AppliedOutcome, InterventionError> {
    let centre = cell_for(world, &region.center);
    let (nlat, nlon) = (world.elevation_grid.nlat(), world.elevation_grid.nlon());
    // Compute every change from the pre-edit surface first, so the result
    // does not depend on the order cells are visited.
    let elevation = world.tectonics_state.get_elevation_grid();
    let changes: Vec<((usize, usize), f64)> = cells_for_region(world, region)
        .into_iter()
        .filter_map(|cell| {
            let weight = brush_weight(world, centre, cell, region) * strength as f64;
            if weight <= 0.0 {
                return None;
            }
            let (row, col) = cell;
            let mut neighbours = vec![
                *elevation.get(row, (col + 1) % nlon),
                *elevation.get(row, (col + nlon - 1) % nlon),
            ];
            if row > 0 {
                neighbours.push(*elevation.get(row - 1, col));
            }
            if row + 1 < nlat {
                neighbours.push(*elevation.get(row + 1, col));
            }
            let mean = neighbours.iter().sum::<f64>() / neighbours.len() as f64;
            Some((cell, weight * (mean - *elevation.get(row, col))))
        })
        .collect();

    let mut touched = 0usize;
    let mut largest = 0.0f64;
    for ((row, col), delta) in changes {
        if world.tectonics_state.add_surface_relief(row, col, delta) {
            touched += 1;
            largest = largest.max(delta.abs());
        }
    }
    world.elevation_grid = world.tectonics_state.get_elevation_grid();

    Ok(mutated(
        format!(
            "smoothed terrain across {touched} cells around cell ({}, {}), largest change {largest:.0} m",
            centre.0, centre.1
        ),
        touched,
    ))
}

fn sculpt_terrain(
    world: &mut WorldState,
    elevation_delta_m: f32,
    region: &Region,
) -> Result<AppliedOutcome, InterventionError> {
    let centre = cell_for(world, &region.center);
    let peak = elevation_delta_m as f64;
    let mut touched = 0usize;
    for cell in cells_for_region(world, region) {
        let weight = brush_weight(world, centre, cell, region);
        if weight > 0.0
            && world
                .tectonics_state
                .add_surface_relief(cell.0, cell.1, peak * weight)
        {
            touched += 1;
        }
    }
    // Refresh the cached surface so movement/building physics see the new
    // terrain immediately, not only after the next biome pass.
    world.elevation_grid = world.tectonics_state.get_elevation_grid();

    Ok(mutated(
        format!(
            "{} terrain by up to {:.0} m across {touched} cells around cell ({}, {})",
            if peak >= 0.0 { "raised" } else { "lowered" },
            peak.abs(),
            centre.0,
            centre.1
        ),
        touched,
    ))
}

fn modify_scenario(
    world: &mut WorldState,
    parameter: &str,
    value: &ScenarioValue,
) -> Result<AppliedOutcome, InterventionError> {
    let as_float = |expected: &'static str| -> Result<f64, InterventionError> {
        match value {
            ScenarioValue::Float(v) => Ok(*v as f64),
            ScenarioValue::Int(v) => Ok(*v as f64),
            _ => Err(InterventionError::ScenarioTypeMismatch {
                parameter: parameter.to_string(),
                expected,
            }),
        }
    };

    match parameter {
        "climate.co2_concentration_ppm" => {
            let ppm = as_float("numeric")?;
            if !ppm.is_finite() || ppm < 0.0 {
                return Err(InterventionError::Validation(ValidationResult::invalid(
                    vec!["CO2 concentration must be finite and non-negative".to_string()],
                )));
            }
            let previous = world.climate_state.co2_concentration;
            // CO2 is carried state that the carbon cycle steps from its
            // previous value, so the new level persists.
            world.climate_state.co2_concentration = ppm;
            crate::conservation::book_atmospheric_co2_change(
                &mut world.ledger,
                Reservoir::OperatorIntervention,
                crate::conservation::atmospheric_carbon_kg_per_ppm(&world.canon),
                previous,
                ppm,
            );
            Ok(mutated(
                format!("set atmospheric CO2 from {previous} ppm to {ppm} ppm"),
                1,
            ))
        }
        "biosphere.current_time_myr" => {
            let myr = as_float("numeric")?;
            if !myr.is_finite() || myr < 0.0 {
                return Err(InterventionError::Validation(ValidationResult::invalid(
                    vec!["Biosphere time must be finite and non-negative".to_string()],
                )));
            }
            let previous = world.biosphere_state.current_time_myr;
            world.biosphere_state.current_time_myr = myr;
            Ok(mutated(
                format!("set biosphere clock from {previous} Myr to {myr} Myr"),
                1,
            ))
        }
        "resource_economy.regeneration_multiplier" => {
            let multiplier = as_float("numeric")?;
            if !multiplier.is_finite() || multiplier < 0.0 {
                return Err(InterventionError::Validation(ValidationResult::invalid(
                    vec!["Regeneration multiplier must be finite and non-negative".to_string()],
                )));
            }
            let mut touched = 0usize;
            for node in &mut world.resource_economy_state.nodes {
                let scaled = (node.regeneration_per_tick as f64 * multiplier).round();
                node.regeneration_per_tick = scaled.clamp(0.0, u32::MAX as f64) as u32;
                touched += 1;
            }
            Ok(mutated(
                format!("scaled regeneration of {touched} resource nodes by {multiplier}"),
                touched,
            ))
        }
        other => Err(InterventionError::UnknownScenarioParameter {
            parameter: other.to_string(),
            supported: SUPPORTED_SCENARIO_PARAMETERS,
        }),
    }
}

/// A registry-unique `agent_id` derived from an authored display name:
/// lowercase alphanumerics joined by `-`, suffixed `-2`, `-3`, … on collision.
fn agent_id_for_name(world: &WorldState, name: &str) -> String {
    let slug = name
        .split(|c: char| !c.is_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(str::to_lowercase)
        .collect::<Vec<_>>()
        .join("-");
    let registry = &world.humans_state.registry;
    if registry.get_human(&slug).is_none() {
        return slug;
    }
    (2u64..)
        .map(|n| format!("{slug}-{n}"))
        .find(|candidate| registry.get_human(candidate).is_none())
        .expect("an unbounded suffix range always yields a free id")
}

/// Keyed RNG epoch for sampling a spawned human's genome and traits,
/// distinct from every other `SubsystemId::Humans` consumer's epoch.
const SPAWN_HUMAN_EPOCH: u32 = 0x5350_574E;

/// Developmental age of a human spawned without an authored profile: an
/// adult, matching the engine's default for a profile with no recorded age.
const DEFAULT_SPAWN_AGE_YEARS: f64 = 25.0;

/// The in-world calendar instant. Every human's age advances by exactly the
/// simulated step, so a living human with a real birth timestamp is living
/// "now" at birth + age; the latest such instant is taken. With no living
/// human, the calendar's own anchor is used: the founders' world begins
/// when Gem-D, from the compiled founder fixture, is at their starting age,
/// and the world's elapsed simulated time is added to that.
fn world_calendar_now(world: &WorldState) -> chrono::DateTime<chrono::Utc> {
    world
        .humans_state
        .registry
        .get_all_humans()
        .iter()
        .filter(|h| matches!(h.profile.status, crate::humans::HumanStatus::Alive))
        .filter(|h| {
            chrono::DateTime::parse_from_rfc3339(&h.profile.core_identity.birth_timestamp).is_ok()
        })
        .map(crate::humans::lifecycle::current_instant_of)
        .max()
        .unwrap_or_else(|| {
            let founding = crate::humans::lifecycle::current_instant_of(
                &crate::humans::HumanBeing::gem_d_founder(),
            );
            let elapsed_s = (world.tick as f64 * world.canon.dt_seconds).round() as i64;
            founding + chrono::Duration::seconds(elapsed_s)
        })
}

fn spawn_human(
    world: &mut WorldState,
    template_id: &str,
    location: &Location,
    profile: Option<&HumanSpawnProfile>,
) -> Result<AppliedOutcome, InterventionError> {
    use mk_core::human::astrology::GeoCoordinates;
    use mk_core::human::BiologicalSex;
    use mk_core::rng::{RngKey, SubsystemId};

    // The template id selects biological sex; anything else is rejected
    // rather than silently defaulting, so a typo cannot quietly produce an
    // unintended human.
    let sex = match template_id.trim().to_ascii_lowercase().as_str() {
        "male" => BiologicalSex::Male,
        "female" => BiologicalSex::Female,
        other => {
            return Err(InterventionError::NotFound {
                what: format!("human template '{other}'; supported templates: male, female"),
            })
        }
    };
    let refused = |reason: String| InterventionError::NotConfigured {
        action: "SpawnHuman".to_string(),
        reason,
    };

    // Birth instant, birthplace and age: authored when a profile is given;
    // otherwise an adult born where they are placed, dated back from the
    // world's own calendar.
    let (birth, birthplace, age_years) = match profile {
        Some(profile) => {
            let birth = chrono::DateTime::parse_from_rfc3339(&profile.birth_timestamp)
                .map_err(|err| {
                    refused(format!(
                        "birth timestamp '{}' is not RFC 3339: {err}",
                        profile.birth_timestamp
                    ))
                })?
                .with_timezone(&chrono::Utc);
            let birthplace = GeoCoordinates {
                latitude: profile.birth_latitude,
                longitude: profile.birth_longitude,
            };
            (birth, birthplace, profile.age_years)
        }
        None => {
            let now = world_calendar_now(world);
            let lived_seconds = (DEFAULT_SPAWN_AGE_YEARS * 365.25 * 24.0 * 3600.0).round() as i64;
            let birthplace = GeoCoordinates {
                latitude: location.latitude as f64,
                longitude: location.longitude as f64,
            };
            (
                now - chrono::Duration::seconds(lived_seconds),
                birthplace,
                DEFAULT_SPAWN_AGE_YEARS,
            )
        }
    };

    let agent_id = match profile {
        Some(profile) => agent_id_for_name(world, &profile.name),
        None => world.humans_state.registry.allocate_agent_id(),
    };
    let place_name = format!("{:.4}, {:.4}", birthplace.latitude, birthplace.longitude);
    let mut rng = world.rng.stream(RngKey::new(
        SubsystemId::Humans,
        (crate::humans::deterministic_human_id(&agent_id) & 0xFFFF_FFFF) as u32,
        SPAWN_HUMAN_EPOCH,
        world.tick,
    ));
    let mut human = crate::humans::HumanBeing::sampled(
        agent_id.clone(),
        sex,
        birth,
        birthplace,
        &place_name,
        age_years,
        &mut rng,
    )
    .map_err(|err| refused(err.to_string()))?;

    // Place the new human at the requested cell, so the location argument
    // is honoured rather than ignored.
    let (row, col) = cell_for(world, location);
    human.set_runtime_position(GridPosition::new(row as i32, col as i32));
    human.development.age_years = age_years;
    if let Some(profile) = profile {
        // Keep the authored timestamp as written (its offset included); the
        // sampled chart already used the same instant.
        human.profile.core_identity.birth_timestamp = profile.birth_timestamp.clone();
        human.body.height_cm = profile.height_cm;
        human.body.build = profile.build.trim().to_lowercase();
        human.body.hair_color = profile.hair_color.trim().to_lowercase();
        human.body.eye_color = profile.eye_color.trim().to_lowercase();
        human.body.skin_tone = profile.skin_tone.trim().to_lowercase();
    }
    human.refresh_phase11_layers();

    // The human exists as soon as it is added; a failed folder write is a
    // warning, not a refusal. Only a duplicate id means it was not added.
    let storage_warning = match world.humans_state.registry.add_human(human) {
        Ok(()) => None,
        Err(err @ crate::io::HumanStorageError::DuplicateAgent(_)) => {
            return Err(refused(format!("the new human was not added: {err}")));
        }
        Err(err) => Some(err),
    };

    let mut summary = format!("spawned human '{agent_id}' ({sex:?}) at cell ({row}, {col})");
    if let Some(err) = storage_warning {
        summary.push_str(&format!(
            " (warning: the human's storage folder could not be written: {err})"
        ));
    }
    Ok(mutated(summary, 1))
}

fn construct_structure(
    world: &mut WorldState,
    structure: StructureKind,
    location: &Location,
) -> Result<AppliedOutcome, InterventionError> {
    let recipe = match structure {
        StructureKind::WoodenShelter => RecipeId::WoodenShelter,
        StructureKind::Workshop => RecipeId::Workshop,
        StructureKind::Storage => RecipeId::Storage,
        StructureKind::StoneHouse => RecipeId::StoneHouse,
    };
    let (row, col) = cell_for(world, location);
    let tick = world.tick;
    let id = world
        .resource_economy_state
        .construct_for_operator(
            "operator",
            recipe,
            GridPosition::new(row as i32, col as i32),
            tick,
            &world.elevation_grid,
        )
        .map_err(|reason| InterventionError::PhysicallyBlocked {
            action: "ConstructStructure".to_string(),
            reason: format!("cell ({row}, {col}): {reason}"),
        })?;
    Ok(mutated(
        format!(
            "constructed {} (structure #{id}) at cell ({row}, {col})",
            structure.display_name()
        ),
        1,
    ))
}

fn remove_human(
    world: &mut WorldState,
    human_id: &str,
) -> Result<AppliedOutcome, InterventionError> {
    // `remove_human` enforces the Reverence Veto for protected creator
    // entities; that refusal is propagated, never swallowed.
    match world.humans_state.registry.remove_human(human_id)? {
        Some(human) => Ok(mutated(format!("removed human '{}'", human.agent_id()), 1)),
        None => Err(InterventionError::NotFound {
            what: format!("human '{human_id}'"),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mk_core::canon::CanonLocked;
    use mk_interventions::{
        BiomassType, ClimateParameter, DisturbanceType, EnergyType, InterventionAction,
        InterventionPermissions, Location, Region, ResourceType, ScenarioValue,
    };
    use std::sync::Arc;

    fn world() -> WorldState {
        WorldState::new(Arc::new(CanonLocked::default()), [7u8; 32])
    }

    fn executor() -> InterventionExecutor {
        InterventionExecutor::new(InterventionPermissions::default())
    }

    /// Permissions that forbid everything, to prove the gate is real.
    fn no_permissions() -> InterventionPermissions {
        InterventionPermissions {
            can_pause: false,
            can_resume: false,
            can_branch: false,
            can_inject_resources: false,
            can_modify_climate: false,
            can_trigger_disturbances: false,
            can_modify_scenario: false,
            can_spawn_humans: false,
            can_construct_structures: false,
            can_sculpt_terrain: false,
        }
    }

    #[test]
    fn control_actions_return_a_directive_not_a_mutation() {
        let mut w = world();
        let ex = executor();
        for (action, expected) in [
            (InterventionAction::Pause, ControlDirective::Pause),
            (InterventionAction::Resume, ControlDirective::Resume),
            (
                InterventionAction::Step { ticks: 5 },
                ControlDirective::Step { ticks: 5 },
            ),
        ] {
            match ex.execute(&mut w, &action).expect("control is permitted") {
                AppliedOutcome::Control { directive } => assert_eq!(directive, expected),
                other => panic!("expected a control directive, got {other:?}"),
            }
        }
    }

    #[test]
    fn invalid_action_is_rejected_before_any_mutation() {
        let mut w = world();
        let before = w.climate_state.co2_concentration;
        // Negative amount fails `validate_intervention`.
        let action = InterventionAction::InjectResource {
            resource_type: ResourceType::Water,
            amount: -5.0,
            location: Location::new(0.0, 0.0),
        };
        let err = executor()
            .execute(&mut w, &action)
            .expect_err("negative amount must be rejected");
        assert!(matches!(err, InterventionError::Validation(_)));
        assert_eq!(w.climate_state.co2_concentration, before);
    }

    #[test]
    fn out_of_range_location_is_rejected() {
        let mut w = world();
        let action = InterventionAction::InjectResource {
            resource_type: ResourceType::Water,
            amount: 10.0,
            location: Location::new(120.0, 0.0),
        };
        assert!(matches!(
            executor().execute(&mut w, &action),
            Err(InterventionError::Validation(_))
        ));
    }

    #[test]
    fn permission_gate_blocks_a_valid_action() {
        let mut w = world();
        let ex = InterventionExecutor::new(no_permissions());
        let before = w.hydrology_state.soil_water.get(16, 32).storage_mm;
        let action = InterventionAction::InjectResource {
            resource_type: ResourceType::Water,
            amount: 10.0,
            location: Location::new(0.0, 0.0),
        };
        let err = ex
            .execute(&mut w, &action)
            .expect_err("permissions forbid injection");
        assert!(matches!(err, InterventionError::PermissionDenied { .. }));
        assert_eq!(
            w.hydrology_state.soil_water.get(16, 32).storage_mm,
            before,
            "a denied intervention must not mutate world state"
        );
    }

    #[test]
    fn water_injection_raises_soil_water_and_closes_the_audit() {
        let mut w = world();
        let (row, col) = w.grid_spec.cell_for_lat_lon(0.0, 0.0);
        // The injected water lands in soil up to field capacity and spills
        // into surface water beyond it, so the invariant that actually holds
        // for any starting saturation is that the cell's *total* water rose.
        let before = w.hydrology_state.soil_water.get(row, col).storage_mm
            + *w.hydrology_state.surface_water.get(row, col);
        w.ledger.clear();
        let stocks_before = crate::conservation::measure(&w);

        let outcome = executor()
            .execute(
                &mut w,
                &InterventionAction::InjectResource {
                    resource_type: ResourceType::Water,
                    amount: 50.0,
                    location: Location::new(0.0, 0.0),
                },
            )
            .expect("water injection succeeds");

        assert!(matches!(outcome, AppliedOutcome::Mutated { .. }));
        let after = w.hydrology_state.soil_water.get(row, col).storage_mm
            + *w.hydrology_state.surface_water.get(row, col);
        assert!(
            (after - before - 50.0).abs() < 1e-6,
            "all 50 mm must land somewhere: {before} -> {after}"
        );
        let report = crate::conservation::audit(
            &w.ledger,
            0,
            &stocks_before,
            &crate::conservation::measure(&w),
        );
        assert!(report.conserved, "{:?}", report.failure);
    }

    #[test]
    fn energy_injection_warms_the_target_cell_and_closes_the_audit() {
        let mut w = world();
        let (row, col) = w.grid_spec.cell_for_lat_lon(10.0, 20.0);
        let before = *w.climate_state.surface_temperature.get(row, col);
        w.ledger.clear();
        let stocks_before = crate::conservation::measure(&w);

        executor()
            .execute(
                &mut w,
                &InterventionAction::InjectEnergy {
                    energy_type: EnergyType::Thermal,
                    // Large enough that the derived temperature rise is
                    // representable at this cell area.
                    amount: 1.0e18,
                    location: Location::new(10.0, 20.0),
                },
            )
            .expect("energy injection succeeds");

        assert!(*w.climate_state.surface_temperature.get(row, col) > before);
        let report = crate::conservation::audit(
            &w.ledger,
            w.tick,
            &stocks_before,
            &crate::conservation::measure(&w),
        );
        assert!(report.conserved, "{:?}", report.failure);
    }

    #[test]
    fn mineral_injection_creates_a_node_when_the_cell_has_none() {
        let mut w = world();
        let mineral_kinds = node_kinds_for(&ResourceType::Minerals);
        // The world seeds nodes from its biomes, so pick a cell whose biome
        // yields none of the mineral kinds.
        let (row, col) = (0..w.grid_spec.nlat)
            .flat_map(|row| (0..w.grid_spec.nlon).map(move |col| (row, col)))
            .find(|&(row, col)| {
                let position = GridPosition::new(row as i32, col as i32);
                !w.resource_economy_state
                    .nodes
                    .iter()
                    .any(|n| n.position == position && mineral_kinds.contains(&n.kind))
            })
            .expect("some cell has no mineral node");
        let lat = w.grid_spec.lat_rad(row).to_degrees();
        let mut lon = w.grid_spec.lon_rad(col).to_degrees();
        if lon > 180.0 {
            lon -= 360.0;
        }
        assert_eq!(w.grid_spec.cell_for_lat_lon(lat, lon), (row, col));
        let position = GridPosition::new(row as i32, col as i32);

        executor()
            .execute(
                &mut w,
                &InterventionAction::InjectResource {
                    resource_type: ResourceType::Minerals,
                    amount: 250.0,
                    location: Location::new(lat as f32, lon as f32),
                },
            )
            .expect("mineral injection succeeds");

        let created: Vec<_> = w
            .resource_economy_state
            .nodes
            .iter()
            .filter(|n| n.position == position && mineral_kinds.contains(&n.kind))
            .collect();
        assert_eq!(created.len(), 1);
        assert_eq!(created[0].available, 250);
        assert_eq!(created[0].capacity, 250);
    }

    #[test]
    fn injected_node_ids_are_unique() {
        let mut w = world();
        for (lat, lon) in [(10.0, 10.0), (20.0, 20.0), (30.0, 30.0)] {
            executor()
                .execute(
                    &mut w,
                    &InterventionAction::InjectResource {
                        resource_type: ResourceType::Minerals,
                        amount: 10.0,
                        location: Location::new(lat as f32, lon as f32),
                    },
                )
                .expect("injection succeeds");
        }
        let mut ids: Vec<u64> = w
            .resource_economy_state
            .nodes
            .iter()
            .map(|n| n.id)
            .collect();
        let total = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), total, "resource node ids must stay unique");
    }

    #[test]
    fn biomass_injection_respects_regional_biome_affinity() {
        let mut w = world();
        let region = Region::new(Location::new(0.0, 0.0), 2000.0);
        let before: u64 = w
            .biosphere_state
            .species
            .iter()
            .map(|s| s.population_size)
            .sum();
        w.ledger.clear();
        let stocks_before = crate::conservation::measure(&w);

        let result = executor().execute(
            &mut w,
            &InterventionAction::InjectBiomass {
                biomass_type: BiomassType::Consumers,
                amount: 1000.0,
                region,
            },
        );

        match result {
            Ok(AppliedOutcome::Mutated { .. }) => {
                let after: u64 = w
                    .biosphere_state
                    .species
                    .iter()
                    .map(|s| s.population_size)
                    .sum();
                assert!(after > before);
                // The new individuals' carbon is booked into the biomass
                // stock, and the stock grew by exactly that much.
                assert!(
                    w.ledger.net_flow_by_reservoir(FluxKind::Carbon)[&Reservoir::BiomassCarbon]
                        > 0.0
                );
                let report = crate::conservation::audit(
                    &w.ledger,
                    0,
                    &stocks_before,
                    &crate::conservation::measure(&w),
                );
                assert!(report.conserved, "{:?}", report.failure);
            }
            // A region whose biomes support no consumer species is a real,
            // reported outcome — not a silent no-op.
            Err(InterventionError::NotFound { what }) => {
                assert!(what.contains("terrain affinity"), "unexpected: {what}");
            }
            other => panic!("unexpected outcome: {other:?}"),
        }
    }

    #[test]
    fn modify_climate_sets_temperature_globally_when_no_region_given() {
        let mut w = world();
        executor()
            .execute(
                &mut w,
                &InterventionAction::ModifyClimate {
                    parameter: ClimateParameter::Temperature,
                    value: 300.0,
                    region: None,
                },
            )
            .expect("global climate set succeeds");

        assert!(w
            .climate_state
            .surface_temperature
            .data()
            .iter()
            .all(|t| (*t - 300.0).abs() < 1e-9));
    }

    #[test]
    fn modify_climate_region_leaves_far_cells_untouched() {
        let mut w = world();
        let (row, col) = w.grid_spec.cell_for_lat_lon(0.0, 0.0);
        // A cell on the opposite side of the planet from the region centre.
        let far_col = (col + w.grid_spec.nlon() / 2) % w.grid_spec.nlon();
        let far_before = *w.climate_state.surface_temperature.get(row, far_col);

        executor()
            .execute(
                &mut w,
                &InterventionAction::ModifyClimate {
                    parameter: ClimateParameter::Temperature,
                    value: 310.0,
                    region: Some(Region::new(Location::new(0.0, 0.0), 100.0)),
                },
            )
            .expect("regional climate set succeeds");

        assert!((*w.climate_state.surface_temperature.get(row, col) - 310.0).abs() < 1e-9);
        assert_eq!(
            *w.climate_state.surface_temperature.get(row, far_col),
            far_before,
            "a 100 km region must not reach the antipode"
        );
    }

    #[test]
    fn modify_climate_wind_preserves_direction() {
        let mut w = world();
        let (row, col) = w.grid_spec.cell_for_lat_lon(0.0, 0.0);
        {
            let wind = w.weather_state.wind.get_mut(row, col);
            wind.u_east = 3.0;
            wind.v_north = 4.0;
        }

        executor()
            .execute(
                &mut w,
                &InterventionAction::ModifyClimate {
                    parameter: ClimateParameter::WindSpeed,
                    value: 10.0,
                    region: Some(Region::new(Location::new(0.0, 0.0), 100.0)),
                },
            )
            .expect("wind set succeeds");

        let wind = w.weather_state.wind.get(row, col);
        let speed = (wind.u_east * wind.u_east + wind.v_north * wind.v_north).sqrt();
        assert!(
            (speed - 10.0).abs() < 1e-6,
            "speed should be set to 10, got {speed}"
        );
        // Original direction was 3:4, so the ratio must be preserved.
        assert!((wind.u_east / wind.v_north - 0.75).abs() < 1e-6);
    }

    #[test]
    fn triggered_disturbance_is_registered_and_then_changes_the_world() {
        let mut w = world();
        // Flood a land cell: the sea has no land hydrology to change.
        let (row, col) = w
            .elevation_grid
            .indexed_iter()
            .find(|(_, _, height)| **height > 0.0)
            .map(|(row, col, _)| (row, col))
            .expect("the world has land");
        let lat = w.grid_spec.lat_rad(row).to_degrees();
        let lon = w.grid_spec.lon_rad(col).to_degrees();
        executor()
            .execute(
                &mut w,
                &InterventionAction::TriggerDisturbance {
                    disturbance_type: DisturbanceType::Flood,
                    intensity: 1.0,
                    location: Location::new(lat as f32, lon as f32),
                },
            )
            .expect("disturbance registration succeeds");

        assert_eq!(w.disturbance_state.len(), 1);
        assert_eq!(w.grid_spec.cell_for_lat_lon(lat, lon), (row, col));
        let before = w.hydrology_state.soil_water.get(row, col).storage_mm;
        w.step_world(1).expect("world steps");
        let after = w.hydrology_state.soil_water.get(row, col).storage_mm;

        assert!(
            after != before,
            "a registered flood must actually change hydrology on the next tick"
        );
    }

    #[test]
    fn scenario_parameter_allowlist_is_enforced() {
        let mut w = world();
        let err = executor()
            .execute(
                &mut w,
                &InterventionAction::ModifyScenario {
                    parameter: "not.a.real.parameter".to_string(),
                    value: ScenarioValue::Float(1.0),
                },
            )
            .expect_err("unknown parameters must be rejected");
        match err {
            InterventionError::UnknownScenarioParameter { supported, .. } => {
                assert_eq!(supported, SUPPORTED_SCENARIO_PARAMETERS);
            }
            other => panic!("expected UnknownScenarioParameter, got {other:?}"),
        }
    }

    #[test]
    fn scenario_co2_is_really_set() {
        let mut w = world();
        executor()
            .execute(
                &mut w,
                &InterventionAction::ModifyScenario {
                    parameter: "climate.co2_concentration_ppm".to_string(),
                    value: ScenarioValue::Float(500.0),
                },
            )
            .expect("co2 set succeeds");
        assert!((w.climate_state.co2_concentration - 500.0).abs() < 1e-9);
    }

    #[test]
    fn scenario_co2_persists_through_climate_steps() {
        let mut w = world();
        executor()
            .execute(
                &mut w,
                &InterventionAction::ModifyScenario {
                    parameter: "climate.co2_concentration_ppm".to_string(),
                    value: ScenarioValue::Float(900.0),
                },
            )
            .expect("co2 set succeeds");
        assert_eq!(w.climate_state.co2_concentration, 900.0);

        let dt = w.canon.step_seconds();
        for _ in 0..3 {
            w.step_world(dt).expect("step succeeds");
        }
        // The carbon cycle relaxes CO2 over ~10⁵ years, so a few steps
        // leave the operator's level in place.
        assert!((w.climate_state.co2_concentration - 900.0).abs() < 1.0);
    }

    #[test]
    fn scenario_type_mismatch_is_rejected() {
        let mut w = world();
        let err = executor()
            .execute(
                &mut w,
                &InterventionAction::ModifyScenario {
                    parameter: "climate.co2_concentration_ppm".to_string(),
                    value: ScenarioValue::Bool(true),
                },
            )
            .expect_err("a bool is not a co2 concentration");
        assert!(matches!(
            err,
            InterventionError::ScenarioTypeMismatch { .. }
        ));
    }

    #[test]
    fn negative_scenario_co2_is_rejected() {
        let mut w = world();
        let before = w.climate_state.co2_concentration;
        let err = executor()
            .execute(
                &mut w,
                &InterventionAction::ModifyScenario {
                    parameter: "climate.co2_concentration_ppm".to_string(),
                    value: ScenarioValue::Float(-1.0),
                },
            )
            .expect_err("negative concentration is invalid");
        assert!(matches!(err, InterventionError::Validation(_)));
        assert_eq!(w.climate_state.co2_concentration, before);
    }

    #[test]
    fn spawn_human_requires_a_known_template() {
        let mut w = world();
        let ex = InterventionExecutor::new(InterventionPermissions {
            can_spawn_humans: true,
            ..InterventionPermissions::default()
        });
        let err = ex
            .execute(
                &mut w,
                &InterventionAction::SpawnHuman {
                    template_id: "wizard".to_string(),
                    location: Location::new(0.0, 0.0),
                    profile: None,
                },
            )
            .expect_err("unknown templates must be rejected");
        assert!(matches!(err, InterventionError::NotFound { .. }));
    }

    /// Latitude/longitude (degrees) at the centre of `(row, col)`.
    fn cell_centre(w: &WorldState, row: usize, col: usize) -> Location {
        let lat = w.grid_spec.lat_rad(row).to_degrees();
        let mut lon = w.grid_spec.lon_rad(col).to_degrees();
        if lon > 180.0 {
            lon -= 360.0;
        }
        Location::new(lat as f32, lon as f32)
    }

    fn find_cell(w: &WorldState, buildable: bool) -> Option<(usize, usize)> {
        (0..w.grid_spec.nlat())
            .flat_map(|r| (0..w.grid_spec.nlon()).map(move |c| (r, c)))
            .find(|&(r, c)| {
                crate::physics::is_buildable(&w.elevation_grid, r as i32, c as i32) == buildable
            })
    }

    #[test]
    fn construct_structure_adds_a_real_economy_structure() {
        let mut w = world();
        let (row, col) = find_cell(&w, true).expect("default world has buildable terrain");
        let before = w.resource_economy_state.structures.len();
        let location = cell_centre(&w, row, col);
        let outcome = executor()
            .execute(
                &mut w,
                &InterventionAction::ConstructStructure {
                    structure: StructureKind::Workshop,
                    location,
                },
            )
            .expect("construction on buildable terrain succeeds");
        assert!(matches!(outcome, AppliedOutcome::Mutated { .. }));
        let economy = &w.resource_economy_state;
        assert_eq!(economy.structures.len(), before + 1);
        let built = economy.structures.last().expect("structure recorded");
        assert_eq!(built.recipe, RecipeId::Workshop);
        assert_eq!(built.position, GridPosition::new(row as i32, col as i32));
        assert!(economy.events.iter().any(|e| {
            e.kind == crate::resource_economy::EconomyEventKind::Constructed
                && e.agent_id == "operator"
        }));
    }

    #[test]
    fn construct_structure_refuses_unbuildable_terrain() {
        let mut w = world();
        let (row, col) = find_cell(&w, true).expect("default world has buildable terrain");
        // Raise the cell into a cliff far above the maximum climbable slope.
        *w.elevation_grid.get_mut(row, col) += 10.0 * crate::physics::MAX_CLIMB_HEIGHT_M + 1.0e4;
        assert!(!crate::physics::is_buildable(
            &w.elevation_grid,
            row as i32,
            col as i32
        ));
        let before = w.resource_economy_state.structures.len();
        let location = cell_centre(&w, row, col);
        let err = executor()
            .execute(
                &mut w,
                &InterventionAction::ConstructStructure {
                    structure: StructureKind::StoneHouse,
                    location,
                },
            )
            .expect_err("steep terrain must refuse construction");
        assert!(matches!(err, InterventionError::PhysicallyBlocked { .. }));
        assert_eq!(w.resource_economy_state.structures.len(), before);
    }

    #[test]
    fn construct_structure_respects_its_permission() {
        let mut w = world();
        let err = InterventionExecutor::new(no_permissions())
            .execute(
                &mut w,
                &InterventionAction::ConstructStructure {
                    structure: StructureKind::Storage,
                    location: Location::new(0.0, 0.0),
                },
            )
            .expect_err("permission gate applies");
        assert!(matches!(err, InterventionError::PermissionDenied { .. }));
    }

    #[test]
    fn sculpted_terrain_changes_elevation_and_survives_tectonic_steps() {
        let mut w = world();
        let location = Location::new(10.0, 40.0);
        let (row, col) = cell_for(&w, &location);
        let before = *w.elevation_grid.get(row, col);
        executor()
            .execute(
                &mut w,
                &InterventionAction::SculptTerrain {
                    elevation_delta_m: 1500.0,
                    region: Region::new(location, 1500.0),
                },
            )
            .expect("sculpting is permitted by default");
        let raised = *w.elevation_grid.get(row, col);
        assert!(
            (raised - before - 1500.0).abs() < 1e-6,
            "centre gets the full change"
        );

        let dt = w.canon.step_seconds();
        w.step_world(dt).expect("world steps");
        let after_step = *w.elevation_grid.get(row, col);
        assert!(
            after_step - before > 1000.0,
            "authored relief must persist across tectonic regeneration"
        );
    }

    #[test]
    fn smoothing_reduces_local_relief() {
        let mut w = world();
        let location = Location::new(-20.0, 100.0);
        let (row, col) = cell_for(&w, &location);
        // Raise a single-cell spike, then smooth it.
        w.tectonics_state.add_surface_relief(row, col, 3000.0);
        w.elevation_grid = w.tectonics_state.get_elevation_grid();
        let spike = *w.elevation_grid.get(row, col);
        executor()
            .execute(
                &mut w,
                &InterventionAction::SmoothTerrain {
                    strength: 1.0,
                    region: Region::new(location, 800.0),
                },
            )
            .expect("smoothing is permitted by default");
        let smoothed = *w.elevation_grid.get(row, col);
        assert!(
            smoothed < spike - 1000.0,
            "spike {spike} should relax, got {smoothed}"
        );
    }

    #[test]
    fn sculpt_terrain_rejects_absurd_changes() {
        let mut w = world();
        let err = executor()
            .execute(
                &mut w,
                &InterventionAction::SculptTerrain {
                    elevation_delta_m: 1.0e6,
                    region: Region::new(Location::new(0.0, 0.0), 100.0),
                },
            )
            .expect_err("a thousand-kilometre mountain is invalid");
        assert!(matches!(err, InterventionError::Validation(_)));
    }

    #[test]
    fn spawn_human_is_denied_by_default_permissions() {
        let mut w = world();
        // `can_spawn_humans` defaults to false in `mk_interventions`.
        let err = executor()
            .execute(
                &mut w,
                &InterventionAction::SpawnHuman {
                    template_id: "female".to_string(),
                    location: Location::new(0.0, 0.0),
                    profile: None,
                },
            )
            .expect_err("spawning is privileged");
        assert!(matches!(err, InterventionError::PermissionDenied { .. }));
    }

    #[test]
    fn spawn_human_places_the_human_at_the_requested_cell() {
        let mut w = world();
        let ex = InterventionExecutor::new(InterventionPermissions {
            can_spawn_humans: true,
            ..InterventionPermissions::default()
        });
        let before = w.humans_state.registry.count();
        let (row, col) = w.grid_spec.cell_for_lat_lon(30.0, 60.0);

        let outcome = ex
            .execute(
                &mut w,
                &InterventionAction::SpawnHuman {
                    template_id: "male".to_string(),
                    location: Location::new(30.0, 60.0),
                    profile: None,
                },
            )
            .expect("spawn succeeds");

        assert_eq!(w.humans_state.registry.count(), before + 1);
        let summary = outcome.summary();
        assert!(
            summary.contains(&format!("({row}, {col})")),
            "summary should name the real cell: {summary}"
        );
    }

    #[test]
    fn spawn_human_succeeds_with_a_warning_when_its_folder_cannot_be_written() {
        let mut w = world();
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("humans");
        w.humans_state
            .registry
            .set_storage(crate::io::HumanStorage::new_unencrypted(&root))
            .unwrap();
        std::fs::remove_dir_all(&root).unwrap();
        std::fs::write(&root, b"not a directory").unwrap();

        let ex = InterventionExecutor::new(InterventionPermissions {
            can_spawn_humans: true,
            ..InterventionPermissions::default()
        });
        let before = w.humans_state.registry.count();
        let outcome = ex
            .execute(
                &mut w,
                &InterventionAction::SpawnHuman {
                    template_id: "female".to_string(),
                    location: Location::new(0.0, 0.0),
                    profile: None,
                },
            )
            .expect("a storage failure must not refuse the spawn");

        assert_eq!(w.humans_state.registry.count(), before + 1);
        assert!(
            outcome.summary().contains("warning"),
            "the storage error is reported: {}",
            outcome.summary()
        );
    }

    #[test]
    fn spawned_human_is_a_sampled_individual_not_the_zero_template() {
        let mut w = world();
        let ex = InterventionExecutor::new(InterventionPermissions {
            can_spawn_humans: true,
            ..InterventionPermissions::default()
        });
        ex.execute(
            &mut w,
            &InterventionAction::SpawnHuman {
                template_id: "male".to_string(),
                location: Location::new(-36.85, 174.76),
                profile: None,
            },
        )
        .expect("spawn succeeds");

        let human = w
            .humans_state
            .registry
            .get_all_humans()
            .last()
            .expect("the spawned human is registered");
        let schema = human
            .profile
            .canonical_schema()
            .expect("a sampled human carries its canonical schema");
        let pair = &schema.genome.chromosomes.pair23;
        assert_eq!(
            (format!("{:?}", pair.A), format!("{:?}", pair.B)),
            ("X".to_string(), "Y".to_string()),
            "a male spawn carries an XY karyotype"
        );
        let t = &schema.temperament_matrix;
        assert!(t.empathy > 0.0 && t.openness_to_experience > 0.0);
        assert!(schema.drive_weights.curiosity > 0.0);

        // With no living human, the birth date comes from the founders'
        // calendar anchor: an adult of the default age at tick 0.
        let born =
            chrono::DateTime::parse_from_rfc3339(&human.profile.core_identity.birth_timestamp)
                .expect("the birth timestamp is RFC 3339");
        let founding = crate::humans::lifecycle::current_instant_of(
            &crate::humans::HumanBeing::gem_d_founder(),
        );
        let age_days = (founding - born.with_timezone(&chrono::Utc)).num_days();
        assert!((age_days as f64 / 365.25 - DEFAULT_SPAWN_AGE_YEARS).abs() < 0.01);
        assert!((human.development.age_years - DEFAULT_SPAWN_AGE_YEARS).abs() < 1e-9);
    }

    fn authored_profile(name: &str) -> HumanSpawnProfile {
        HumanSpawnProfile {
            name: name.to_string(),
            birth_timestamp: "1995-09-10T09:00:00Z".to_string(),
            birth_latitude: 48.85,
            birth_longitude: 2.35,
            age_years: 29.0,
            height_cm: 168.0,
            build: "Slim".to_string(),
            hair_color: "Brown".to_string(),
            eye_color: "Green".to_string(),
            skin_tone: "Fair".to_string(),
        }
    }

    #[test]
    fn spawn_human_applies_the_authored_profile() {
        let mut w = world();
        let ex = InterventionExecutor::new(InterventionPermissions {
            can_spawn_humans: true,
            ..InterventionPermissions::default()
        });
        let spawn = |w: &mut WorldState| {
            ex.execute(
                w,
                &InterventionAction::SpawnHuman {
                    template_id: "female".to_string(),
                    location: Location::new(10.0, 20.0),
                    profile: Some(authored_profile("Ada Lovelace")),
                },
            )
            .expect("profiled spawn succeeds")
        };
        spawn(&mut w);
        spawn(&mut w);

        let first = w
            .humans_state
            .registry
            .get_human("ada-lovelace")
            .expect("agent id derives from the authored name");
        assert_eq!(first.age(), 29);
        assert_eq!(
            first.profile.core_identity.birth_timestamp,
            "1995-09-10T09:00:00Z"
        );
        assert!((first.profile.core_identity.birthplace.coordinates.latitude - 48.85).abs() < 1e-9);
        assert_eq!(first.body.height_cm, 168.0);
        assert_eq!(first.body.eye_color, "green");
        assert_eq!(first.body.skin_tone, "fair");
        assert!(
            w.humans_state
                .registry
                .get_human("ada-lovelace-2")
                .is_some(),
            "a second human with the same name gets a unique id"
        );
    }

    #[test]
    fn spawn_human_rejects_an_invalid_profile() {
        let mut w = world();
        let ex = InterventionExecutor::new(InterventionPermissions {
            can_spawn_humans: true,
            ..InterventionPermissions::default()
        });
        let mut profile = authored_profile("   ");
        profile.age_years = -1.0;
        let err = ex
            .execute(
                &mut w,
                &InterventionAction::SpawnHuman {
                    template_id: "male".to_string(),
                    location: Location::new(0.0, 0.0),
                    profile: Some(profile),
                },
            )
            .expect_err("blank name and negative age are invalid");
        match err {
            InterventionError::Validation(result) => assert_eq!(result.errors.len(), 2),
            other => panic!("expected validation failure, got {other:?}"),
        }
    }

    #[test]
    fn removing_an_unknown_human_reports_not_found() {
        let mut w = world();
        let ex = InterventionExecutor::new(InterventionPermissions {
            can_spawn_humans: true,
            ..InterventionPermissions::default()
        });
        let err = ex
            .execute(
                &mut w,
                &InterventionAction::RemoveHuman {
                    human_id: "human_does_not_exist".to_string(),
                },
            )
            .expect_err("no such human");
        assert!(matches!(err, InterventionError::NotFound { .. }));
    }

    #[test]
    fn snapshot_actions_report_when_no_directory_is_configured() {
        let mut w = world();
        let err = executor()
            .execute(
                &mut w,
                &InterventionAction::Branch {
                    name: "b1".to_string(),
                    source_snapshot: String::new(),
                },
            )
            .expect_err("branching needs a snapshot directory");
        assert!(matches!(err, InterventionError::NotConfigured { .. }));
    }

    #[test]
    fn branch_writes_a_real_snapshot() {
        let dir =
            std::env::temp_dir().join(format!("mk_intervention_branch_{}", std::process::id()));
        let mut w = world();
        let ex = executor().with_snapshot_dir(&dir);

        let outcome = ex
            .execute(
                &mut w,
                &InterventionAction::Branch {
                    name: "trial".to_string(),
                    source_snapshot: "root".to_string(),
                },
            )
            .expect("branch succeeds");

        match outcome {
            AppliedOutcome::Control {
                directive: ControlDirective::Branch { path, name },
            } => {
                assert_eq!(name, "trial");
                assert!(path.exists(), "branch must write the snapshot file");
                let (_, loaded) =
                    crate::io::snapshot::load_snapshot(&path).expect("snapshot loads");
                assert_eq!(loaded.tick, w.tick);
            }
            other => panic!("expected a branch directive, got {other:?}"),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn branch_name_cannot_escape_the_snapshot_directory() {
        let dir =
            std::env::temp_dir().join(format!("mk_intervention_escape_{}", std::process::id()));
        let mut w = world();
        let ex = executor().with_snapshot_dir(&dir);

        let outcome = ex
            .execute(
                &mut w,
                &InterventionAction::Branch {
                    name: "../../etc/evil".to_string(),
                    source_snapshot: String::new(),
                },
            )
            .expect("branch succeeds");

        match outcome {
            AppliedOutcome::Control {
                directive: ControlDirective::Branch { path, .. },
            } => {
                assert_eq!(
                    path.parent(),
                    Some(dir.as_path()),
                    "a hostile branch name must stay inside the snapshot dir"
                );
            }
            other => panic!("expected a branch directive, got {other:?}"),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn scrub_to_a_missing_snapshot_reports_not_found() {
        let dir =
            std::env::temp_dir().join(format!("mk_intervention_scrub_{}", std::process::id()));
        let mut w = world();
        let ex = executor().with_snapshot_dir(&dir);
        let err = ex
            .execute(&mut w, &InterventionAction::Scrub { target_tick: 999 })
            .expect_err("no snapshot for that tick");
        assert!(matches!(err, InterventionError::NotFound { .. }));
    }

    #[test]
    fn identical_interventions_produce_identical_world_hashes() {
        let action = InterventionAction::InjectResource {
            resource_type: ResourceType::Minerals,
            amount: 123.0,
            location: Location::new(12.5, 77.25),
        };

        let run = || {
            let mut w = world();
            executor()
                .execute(&mut w, &action)
                .expect("injection succeeds");
            w.step_world(1).expect("world steps");
            w.hash_chain.current
        };

        assert_eq!(
            run(),
            run(),
            "an intervention must not introduce run-to-run divergence"
        );
    }
}
