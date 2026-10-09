//! Phase 4 Task 4: operator interventions, adapted to the island.
//!
//! `crate::interventions` applies a validated [`InterventionAction`] to a
//! planetary [`WorldState`](crate::world_integration::WorldState). This is
//! the same job against [`IslandLife`], and it is a different piece of work
//! rather than a generic one because the two worlds are not the same world:
//! the island has no species populations, no disturbance system and no
//! branch registry, and its terrain is canon rather than clay.
//!
//! So the adaptation is mostly a question of what to refuse. An action with
//! no island meaning returns
//! [`IslandInterventionError::NotSupportedOnIsland`] naming the action and
//! why — never a silent no-op, and never an invented effect. The refusals
//! are listed in `UPSTREAM.md` alongside the rest of the island's
//! divergence from upstream, and `every_action_is_supported_or_refused_by_name`
//! holds the two together by walking every variant.
//!
//! # Permissions
//!
//! Unlike [`crate::interventions::InterventionExecutor`], nothing here holds
//! an [`InterventionPermissions`]. The gate belongs at the edge, with the
//! session that holds the permissions, and
//! [`IslandLife::apply_command`](crate::regional::life::IslandLife) applies
//! what reaches it. That is deliberate: the replay log records commands that
//! were *applied*, so a replay must be able to re-apply them. A replay that
//! could be refused for permissions the original run had would not be a
//! replay of that run.
//!
//! Validation is a different matter and does live here, because it is a
//! property of the action rather than of who sent it: a malformed action is
//! refused identically every time, so it never reaches the log and never
//! has to replay.
//!
//! # Conservation
//!
//! An intervention introduces matter or energy from outside the simulated
//! system, or takes it out. Every such crossing is booked against
//! [`Reservoir::OperatorIntervention`], a boundary reservoir, so the
//! island's audit still closes on the tick it lands and the inflow stays
//! attributable rather than appearing as something the island made from
//! nothing.

use mk_core::flux::{FluxKind, Reservoir};
use mk_interventions::{
    validate_intervention, BiomassType, ClimateParameter, InterventionAction, Location, Region,
    ResourceType, StructureKind, ValidationResult,
};
use mk_island::DomainLevel;

use super::create_human::{CreateHumanError, CreateLocation, IslandCreateHuman};
use super::life::IslandLife;
use super::materials::Material;
use crate::resource_economy::RecipeId;
use crate::topology::GridTopology;

/// A run-loop control the island's host must carry out.
///
/// Mirrors [`crate::interventions::ControlDirective`]'s first three
/// variants and stops there: `Scrub`, `Branch` and `Fork` are snapshot-
/// backed, and the island runner has nowhere to put a branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IslandDirective {
    Pause,
    Resume,
    Step { ticks: u64 },
}

/// What an intervention did to the island.
#[derive(Debug, Clone)]
pub enum IslandApplied {
    /// The world changed. `summary` is the audit-trail line and `touched`
    /// counts the cells or people it reached.
    Mutated { summary: String, touched: usize },
    /// Nothing in the world changed; the host should act on `directive`.
    Control { directive: IslandDirective },
}

impl IslandApplied {
    pub fn summary(&self) -> String {
        match self {
            Self::Mutated { summary, .. } => summary.clone(),
            Self::Control { directive } => format!("control directive: {directive:?}"),
        }
    }
}

/// Why an intervention did not happen.
#[derive(Debug)]
pub enum IslandInterventionError {
    /// Failed `mk_interventions::validate_intervention`.
    Validation(ValidationResult),
    /// The action has no island meaning. `reason` says what is missing
    /// rather than only that something is.
    NotSupportedOnIsland {
        action: &'static str,
        reason: &'static str,
    },
    /// A latitude and longitude that is not on this island.
    OffIsland { latitude: f64, longitude: f64 },
    /// The action named something that does not exist here.
    NotFound { what: String },
    /// The island's physical state forbids it where it was asked for.
    PhysicallyBlocked { action: String, reason: String },
    /// The creation was refused, with a reason per problem.
    Create(CreateHumanError),
    /// Refused by the Reverence Veto (see [`crate::governance`]).
    ReverenceVeto(crate::governance::ReverenceVetoViolation),
    /// The material ledger refused the change.
    Material(super::materials::MaterialError),
    /// The island's conservation audit did not close after the change.
    ///
    /// The change has already happened: this is not a refusal but a report
    /// that the island is now in a state its own books do not balance, and
    /// it says so rather than rolling back a mutation the ledger has
    /// already been told about.
    Audit(String),
}

impl std::fmt::Display for IslandInterventionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Validation(result) => write!(
                f,
                "intervention failed validation: {}",
                result.errors.join("; ")
            ),
            Self::NotSupportedOnIsland { action, reason } => {
                write!(f, "'{action}' has no meaning on the island: {reason}")
            }
            Self::OffIsland {
                latitude,
                longitude,
            } => write!(f, "{latitude:.4}, {longitude:.4} is not on this island"),
            Self::NotFound { what } => write!(f, "not found: {what}"),
            Self::PhysicallyBlocked { action, reason } => {
                write!(f, "'{action}' blocked by the island: {reason}")
            }
            Self::Create(e) => write!(f, "{e}"),
            Self::ReverenceVeto(v) => write!(f, "refused by Reverence Veto: {v}"),
            Self::Material(e) => write!(f, "{e}"),
            Self::Audit(why) => write!(f, "conservation audit did not close: {why}"),
        }
    }
}

impl std::error::Error for IslandInterventionError {}

impl From<crate::governance::ReverenceVetoViolation> for IslandInterventionError {
    fn from(v: crate::governance::ReverenceVetoViolation) -> Self {
        Self::ReverenceVeto(v)
    }
}

/// The upstream action's name, as the refusals and the audit trail spell it.
pub fn action_name(action: &InterventionAction) -> &'static str {
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
        InterventionAction::ConstructStructure { .. } => "ConstructStructure",
        InterventionAction::SpawnHuman { .. } => "SpawnHuman",
        InterventionAction::RemoveHuman { .. } => "RemoveHuman",
    }
}

/// Why an action is refused on the island, or `None` if it is applied.
///
/// A table rather than arms scattered through the executor, because this is
/// the list `UPSTREAM.md` carries and a reader should be able to see all of
/// it at once. Two of the actions are refused only for some of their
/// arguments — `InjectResource` and `InjectBiomass` name vocabularies the
/// island only partly shares — so those are decided here too rather than
/// halfway down their own function.
pub fn refusal(action: &InterventionAction) -> Option<&'static str> {
    match action {
        // The island's land is canon. The domain is generated from the
        // canon-locked scenario, and the terrain's hash is part of the
        // state digest every snapshot and every replay is checked against.
        // An edited coastline would make an island disagree with its own
        // canon, which is a worse outcome than not being able to edit it.
        InterventionAction::SculptTerrain { .. } | InterventionAction::SmoothTerrain { .. } => {
            Some(
                "the island's terrain is generated from its canon-locked scenario and hashed \
                  into the state digest; editing it would put the island out of agreement with \
                  its own canon",
            )
        }
        // Upstream injects biomass by raising species populations. The
        // island has no species: `regional::ecology` carries standing
        // producer carbon and NPP, and animals are not seeded at island
        // scale (UPSTREAM.md item 13).
        InterventionAction::InjectBiomass { biomass_type, .. }
            if !matches!(biomass_type, BiomassType::Producers) =>
        {
            Some(
                "the island has no animal populations — its ecology carries standing producer \
                  carbon and NPP, and consumers, apex predators and decomposers are not seeded \
                  at island scale",
            )
        }
        // The island's material ledger holds nine named materials. Only
        // water names the same thing in both vocabularies; the rest would
        // have to be guessed at.
        InterventionAction::InjectResource { resource_type, .. }
            if !matches!(resource_type, ResourceType::Water) =>
        {
            Some(
                "the island's stores are named materials (wood, charcoal, coal, limestone, \
                  quicklime, plant food, meat, fibre, water); only water names the same thing \
                  in both vocabularies",
            )
        }
        // The island's energy is the estate's plant — fuel stores,
        // generators, solar arrays, batteries — not a field over the
        // ground. "Inject solar energy at a latitude" names nothing here.
        InterventionAction::InjectEnergy { .. } => Some(
            "the island's energy is the estate's own plant (fuel stores, generators, solar \
             arrays, batteries), not an energy field over the ground, so there is nowhere a \
             quantity of energy at a coordinate would land",
        ),
        // `crate::disturbance` is planetary and the island does not run it.
        InterventionAction::TriggerDisturbance { .. } => Some(
            "the island runs no disturbance system; fire, storm and the rest are planetary \
             state the island's physical step does not carry",
        ),
        // The three supported parameters are planetary
        // (`crate::interventions::SUPPORTED_SCENARIO_PARAMETERS`), and the
        // island's scenario is a different type whose digest the canon is
        // checked against.
        InterventionAction::ModifyScenario { .. } => Some(
            "the island's scenario is an `IslandScenario`, whose digest every snapshot and \
             replay is checked against; upstream's settable parameters are planetary and name \
             nothing in it",
        ),
        // Snapshot-backed. The island can save and load (`io::island_snapshot`)
        // but the runner keeps no registry of branches to root one in.
        InterventionAction::Scrub { .. }
        | InterventionAction::Branch { .. }
        | InterventionAction::Fork { .. } => Some(
            "the island can be saved and loaded, but its runner keeps no branch registry for a \
             fork to be rooted in",
        ),
        _ => None,
    }
}

impl IslandLife {
    /// Validate `action` and apply it to the island.
    ///
    /// Validation runs before anything is touched, so an action that fails
    /// it leaves the island exactly as it was, whoever sent it. See the
    /// module docs for why the permission gate is not here.
    pub fn intervene(
        &mut self,
        action: &InterventionAction,
    ) -> Result<IslandApplied, IslandInterventionError> {
        let validation = validate_intervention(action);
        if !validation.valid {
            return Err(IslandInterventionError::Validation(validation));
        }
        if let Some(reason) = refusal(action) {
            return Err(IslandInterventionError::NotSupportedOnIsland {
                action: action_name(action),
                reason,
            });
        }
        match action {
            InterventionAction::Pause => Ok(control(IslandDirective::Pause)),
            InterventionAction::Resume => Ok(control(IslandDirective::Resume)),
            InterventionAction::Step { ticks } => {
                Ok(control(IslandDirective::Step { ticks: *ticks }))
            }
            InterventionAction::ModifyClimate {
                parameter,
                value,
                region,
            } => self.modify_climate(parameter, *value, region.as_ref()),
            InterventionAction::InjectBiomass { amount, region, .. } => {
                self.inject_producers(*amount, region)
            }
            InterventionAction::InjectResource {
                amount, location, ..
            } => self.inject_water(*amount, location),
            InterventionAction::ConstructStructure {
                structure,
                location,
            } => self.construct_structure(*structure, location),
            InterventionAction::SpawnHuman {
                template_id,
                location,
                profile,
            } => self.spawn_human(template_id, location, profile.as_ref()),
            InterventionAction::RemoveHuman { human_id } => self.remove_human(human_id),
            // Everything else was refused by `refusal` above. Spelled out
            // rather than left to a wildcard so that adding a variant
            // upstream fails this match instead of silently falling into a
            // refusal it was never considered for.
            InterventionAction::SculptTerrain { .. }
            | InterventionAction::SmoothTerrain { .. }
            | InterventionAction::InjectEnergy { .. }
            | InterventionAction::TriggerDisturbance { .. }
            | InterventionAction::ModifyScenario { .. }
            | InterventionAction::Scrub { .. }
            | InterventionAction::Branch { .. }
            | InterventionAction::Fork { .. } => {
                // `refusal` has already returned `Some` for every one of
                // these, so this cannot be reached — but it refuses again
                // rather than asserting that. This runs on the thread that
                // owns the island, and a panic there takes the island down;
                // no operator action should be able to do that, least of
                // all through a line whose whole argument is that it never
                // runs.
                Err(IslandInterventionError::NotSupportedOnIsland {
                    action: action_name(action),
                    reason: "the island has no meaning for this action",
                })
            }
        }
    }
}

/// "1 cell" / "4 cells". These summaries are read by a person — they are
/// what the dashboard puts on screen after an intervention — and "over 1
/// coarse cells" is the sort of thing that makes software look like it is
/// talking to itself.
fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

fn control(directive: IslandDirective) -> IslandApplied {
    IslandApplied::Control { directive }
}

fn mutated(summary: String, touched: usize) -> IslandApplied {
    IslandApplied::Mutated { summary, touched }
}

impl IslandLife {
    /// The cell of a grid level containing a latitude and longitude.
    ///
    /// Refused rather than clamped when it is off the island. Upstream's
    /// planet has a cell for every coordinate, so clamping there costs
    /// nothing; here, clamping would quietly move an intervention aimed at
    /// the open sea onto the nearest coast.
    fn cell_for(
        &self,
        level: DomainLevel,
        location: &Location,
    ) -> Result<(usize, usize), IslandInterventionError> {
        let (latitude, longitude) = (f64::from(location.latitude), f64::from(location.longitude));
        GridTopology::regional(&self.domain, level)
            .cell_for_lat_lon(latitude, longitude)
            .ok_or(IslandInterventionError::OffIsland {
                latitude,
                longitude,
            })
    }

    /// Every cell of a level whose centre is within the region's radius.
    ///
    /// Straight-line metres, because the island's domain is a flat metre
    /// grid: upstream's version corrects the longitudinal extent for
    /// latitude because its cells narrow toward the poles, and these do not.
    /// The centre must be on the island; cells beyond its edge simply are
    /// not there, so a region overhanging the sea is clipped rather than
    /// refused.
    ///
    /// The reach is clamped to the grid before the walk, which is not
    /// tidiness: `validate_region` only requires a radius that is finite
    /// and positive, so `radius_km: 1e30` is a *valid* action. Without the
    /// clamp that is a walk over `(2 x isize::MAX + 1)^2` offsets on the
    /// thread that owns the island — an operator hanging the world with one
    /// accepted request. A region larger than the island is the island.
    fn cells_in_region(
        &self,
        level: DomainLevel,
        region: &Region,
    ) -> Result<Vec<(usize, usize)>, IslandInterventionError> {
        let (row, col) = self.cell_for(level, &region.center)?;
        let size_m = self.domain.cell_size_m(level);
        let radius_m = f64::from(region.radius_km).max(0.0) * 1000.0;
        let (rows, cols) = (self.domain.rows(level), self.domain.cols(level));
        let reach = ((radius_m / size_m).floor() as isize).clamp(0, rows.max(cols) as isize);
        let mut cells = Vec::new();
        for drow in -reach..=reach {
            for dcol in -reach..=reach {
                let (r, c) = (row as isize + drow, col as isize + dcol);
                if r < 0 || c < 0 || r >= rows as isize || c >= cols as isize {
                    continue;
                }
                let distance_m = ((drow * drow + dcol * dcol) as f64).sqrt() * size_m;
                if distance_m <= radius_m {
                    cells.push((r as usize, c as usize));
                }
            }
        }
        Ok(cells)
    }

    /// Set a coarse climate or weather field across a region, or the whole
    /// island when no region is named.
    ///
    /// Upstream's logic, against the island's own coarse grids. The value is
    /// absolute in every case — the parameter names a target, not an offset
    /// — and the heat a temperature change implies is booked from
    /// [`Reservoir::OperatorIntervention`].
    ///
    /// That booking lives for one step. The island's physical state rebuilds
    /// its ledger at the start of every physical step (it is a record of
    /// that step's fluxes, not a running total), and the island keeps no
    /// cumulative energy ledger to put it in instead. So the entry is
    /// readable on the step the intervention lands on and gone after the
    /// next one, which is said here rather than left for somebody to
    /// discover.
    fn modify_climate(
        &mut self,
        parameter: &ClimateParameter,
        value: f32,
        region: Option<&Region>,
    ) -> Result<IslandApplied, IslandInterventionError> {
        let level = DomainLevel::Coarse;
        let cells = match region {
            Some(region) => self.cells_in_region(level, region)?,
            None => {
                let (rows, cols) = (self.domain.rows(level), self.domain.cols(level));
                (0..rows)
                    .flat_map(|row| (0..cols).map(move |col| (row, col)))
                    .collect()
            }
        };
        let value = f64::from(value);
        let mut touched = 0usize;
        let mut booked = String::new();

        match parameter {
            ClimateParameter::Temperature => {
                let area = self.domain.cell_area_m2(level);
                let (mut land_j, mut ocean_j) = (0.0, 0.0);
                for (row, col) in &cells {
                    let height = self
                        .physical
                        .elevation_coarse_m()
                        .get_safe(*row, *col)
                        .copied()
                        .unwrap_or(0.0);
                    if let Some(temperature) = self
                        .physical
                        .climate
                        .surface_temperature
                        .get_mut_safe(*row, *col)
                    {
                        let joules = (value - *temperature)
                            * area
                            * crate::climate::surface_heat_capacity_j_m2_k(height);
                        *temperature = value;
                        touched += 1;
                        match crate::conservation::heat_reservoir(height) {
                            Reservoir::OceanHeat => ocean_j += joules,
                            _ => land_j += joules,
                        }
                    }
                }
                for (reservoir, joules) in [
                    (Reservoir::SurfaceEnergy, land_j),
                    (Reservoir::OceanHeat, ocean_j),
                ] {
                    crate::conservation::book_signed(
                        &mut self.physical.ledger,
                        FluxKind::Energy,
                        Reservoir::OperatorIntervention,
                        reservoir,
                        Reservoir::OperatorIntervention,
                        joules,
                    );
                }
                booked = format!(", {:.3e} J booked from outside", land_j + ocean_j);
            }
            ClimateParameter::Precipitation => {
                for (row, col) in &cells {
                    if let Some(precipitation) =
                        self.physical.weather.precipitation.get_mut_safe(*row, *col)
                    {
                        *precipitation = value.max(0.0);
                        touched += 1;
                    }
                }
            }
            ClimateParameter::Humidity => {
                for (row, col) in &cells {
                    if let Some(moisture) = self.physical.weather.moisture.get_mut_safe(*row, *col)
                    {
                        *moisture = value.max(0.0);
                        touched += 1;
                    }
                }
            }
            ClimateParameter::WindSpeed => {
                for (row, col) in &cells {
                    if let Some(wind) = self.physical.weather.wind.get_mut_safe(*row, *col) {
                        // Rescale the existing direction, so setting a speed
                        // does not silently also set a direction. A calm cell
                        // has none to preserve, so it becomes a due-east wind.
                        let speed = wind.u_east.hypot(wind.v_north);
                        if speed > 0.0 {
                            let scale = value.max(0.0) / speed;
                            wind.u_east *= scale;
                            wind.v_north *= scale;
                        } else {
                            wind.u_east = value.max(0.0);
                            wind.v_north = 0.0;
                        }
                        touched += 1;
                    }
                }
            }
        }

        Ok(mutated(
            format!(
                "set {parameter:?} to {value} over {}{booked}",
                plural(touched, "coarse cell", "coarse cells")
            ),
            touched,
        ))
    }

    /// Add standing producer carbon to a region's land cells.
    ///
    /// `amount` is kilograms of carbon in total, spread evenly over the
    /// region's land by area, because the island's biomass field is a
    /// density (kgC/m²) rather than a count. Sea cells carry no producer
    /// carbon and are skipped rather than quietly given some.
    ///
    /// Booked into the physical ledger, with the one-step lifetime
    /// [`IslandLife::modify_climate`] describes.
    fn inject_producers(
        &mut self,
        amount: f32,
        region: &Region,
    ) -> Result<IslandApplied, IslandInterventionError> {
        let level = DomainLevel::Medium;
        let cells: Vec<(usize, usize)> = self
            .cells_in_region(level, region)?
            .into_iter()
            .filter(|(row, col)| *self.physical.geophysics.land_mask.get(*row, *col))
            .collect();
        if cells.is_empty() {
            return Err(IslandInterventionError::NotFound {
                what: "land inside the region — every cell in it is sea".to_string(),
            });
        }
        let total_kgc = f64::from(amount).max(0.0);
        let area_m2 = self.domain.cell_area_m2(level);
        let per_cell_kgc_m2 = total_kgc / (cells.len() as f64 * area_m2);
        for (row, col) in &cells {
            if let Some(biomass) = self.ecology.biomass_kgc_m2.get_mut_safe(*row, *col) {
                *biomass += per_cell_kgc_m2;
            }
        }
        self.physical.ledger.push(mk_core::flux::FluxEntry::new(
            Reservoir::OperatorIntervention,
            Reservoir::BiomassCarbon,
            total_kgc,
            FluxKind::Carbon,
        ));
        Ok(mutated(
            format!(
                "added {total_kgc} kgC of standing producer carbon over {} \
                 ({per_cell_kgc_m2:.6} kgC/m² each)",
                plural(cells.len(), "land cell", "land cells")
            ),
            cells.len(),
        ))
    }

    /// Put water into the island's material stores.
    ///
    /// Through the island's own audit, which is why this one differs in
    /// shape from the two above: `MaterialWater` is one of the three stocks
    /// [`IslandLife`] reconciles against the ledger on every household step,
    /// so an injection that did not book its flux would break the next
    /// audit rather than merely going unrecorded.
    ///
    /// The location is checked even though the stores are not per-cell: an
    /// operator asking for water at a coordinate off the island has asked
    /// for something that cannot be honoured, and answering "done" would be
    /// a lie about where it went.
    fn inject_water(
        &mut self,
        amount: f32,
        location: &Location,
    ) -> Result<IslandApplied, IslandInterventionError> {
        let (row, col) = self.cell_for(DomainLevel::Medium, location)?;
        let kg = f64::from(amount).max(0.0);
        let mut added = 0.0;
        self.audited(|life, ledger| {
            added = life.materials.inject(Material::Water, kg, ledger);
        })
        .map_err(|e| IslandInterventionError::Audit(e.to_string()))?;
        Ok(mutated(
            format!("put {added} kg of water into the island's stores, asked for at cell ({row}, {col})"),
            1,
        ))
    }

    /// Build one of the resource economy's structures on a land cell.
    ///
    /// The island's economy is upstream's `ResourceEconomyState` and the
    /// recipes are the same, so the structure that appears is the same kind
    /// of world object an agent builds from gathered materials.
    ///
    /// The buildability gate is the island's, not upstream's. Upstream's
    /// `construct_for_operator` checks `physics::is_buildable`, an absolute
    /// 2 m of relief between neighbouring cells, which on a 2 km grid is a
    /// gradient of 0.1% and admits none of the island's 66,116 land cells —
    /// the estate's own included. So the cell is checked here first against
    /// [`geophysics::MAX_BUILD_GRADIENT`](super::geophysics::MAX_BUILD_GRADIENT),
    /// and refused with its real gradient when it is mountainside.
    ///
    /// So the economy is entered through `place_structure`, which is
    /// `construct_for_operator` without that gate in front of it — the
    /// recipe is still checked and the construction is still recorded in
    /// the event log, so a structure placed this way is as visible as any
    /// agent-built one. The only thing the island takes on is the terrain
    /// judgement, which is the one upstream's constant cannot make here.
    fn construct_structure(
        &mut self,
        structure: StructureKind,
        location: &Location,
    ) -> Result<IslandApplied, IslandInterventionError> {
        let (row, col) = self.cell_for(DomainLevel::Medium, location)?;
        let blocked = |reason: String| IslandInterventionError::PhysicallyBlocked {
            action: "ConstructStructure".to_string(),
            reason,
        };
        if !*self.physical.geophysics.land_mask.get(row, col) {
            return Err(blocked(format!("cell ({row}, {col}) is in the sea")));
        }
        let size_m = self.domain.cell_size_m(DomainLevel::Medium);
        if !super::geophysics::is_buildable_cell(
            &self.physical.geophysics.elevation_m,
            &self.physical.geophysics.land_mask,
            size_m,
            row,
            col,
        ) {
            let here = *self.physical.geophysics.elevation_m.get(row, col);
            let steepest = [(0i64, 1i64), (0, -1), (1, 0), (-1, 0)]
                .iter()
                .filter_map(|(drow, dcol)| {
                    let (r, c) = (row as i64 + drow, col as i64 + dcol);
                    let (rows, cols) = (
                        self.domain.rows(DomainLevel::Medium) as i64,
                        self.domain.cols(DomainLevel::Medium) as i64,
                    );
                    (r >= 0 && c >= 0 && r < rows && c < cols).then_some((r as usize, c as usize))
                })
                .filter(|(r, c)| *self.physical.geophysics.land_mask.get(*r, *c))
                .map(|(r, c)| {
                    (here - *self.physical.geophysics.elevation_m.get(r, c)).abs() / size_m
                })
                .fold(0.0f64, f64::max);
            return Err(blocked(format!(
                "cell ({row}, {col}) falls {:.1}% to a neighbour, and an ordinary footing \
                 wants no more than {:.1}%",
                steepest * 100.0,
                super::geophysics::MAX_BUILD_GRADIENT * 100.0
            )));
        }
        let recipe = match structure {
            StructureKind::WoodenShelter => RecipeId::WoodenShelter,
            StructureKind::Workshop => RecipeId::Workshop,
            StructureKind::Storage => RecipeId::Storage,
            StructureKind::StoneHouse => RecipeId::StoneHouse,
        };
        let tick = self.tick;
        let id = self
            .economy
            .place_structure(
                "operator",
                recipe,
                crate::agents::GridPosition::new(row as i32, col as i32),
                tick,
            )
            .map_err(|reason| blocked(format!("cell ({row}, {col}): {reason}")))?;
        Ok(mutated(
            format!(
                "constructed {} (structure #{id}) at cell ({row}, {col})",
                structure.display_name()
            ),
            1,
        ))
    }

    /// Put a person on the island at a coordinate.
    ///
    /// Through [`IslandLife::create_human`], so an operator spawn and a
    /// dashboard creation are the same creation: the same validation, the
    /// same body in the material ledger, the same runtime position, the same
    /// folder. Anything else would be a second way of making a person, and
    /// the two would drift.
    ///
    /// The profile is required here although upstream's is optional. Upstream
    /// falls back to its canonical newborn defaults; the island's creation
    /// takes an authored identity — a name, an age, a height — and there is
    /// nothing to default a name to.
    fn spawn_human(
        &mut self,
        template_id: &str,
        location: &Location,
        profile: Option<&mk_interventions::HumanSpawnProfile>,
    ) -> Result<IslandApplied, IslandInterventionError> {
        let profile = profile.ok_or_else(|| {
            IslandInterventionError::Create(CreateHumanError::Invalid(vec![
                "profile: creating somebody on the island needs an authored identity; there is \
                 no name to fall back on."
                    .to_string(),
            ]))
        })?;
        let biological_sex = match template_id.trim().to_ascii_lowercase().as_str() {
            "male" => mk_core::human::BiologicalSex::Male,
            "female" => mk_core::human::BiologicalSex::Female,
            other => {
                return Err(IslandInterventionError::Create(CreateHumanError::Invalid(
                    vec![format!(
                        "template_id: '{other}' is not a template; it is 'male' or 'female'."
                    )],
                )))
            }
        };
        let (row, col) = self.cell_for(DomainLevel::Medium, location)?;
        let created = self
            .create_human(IslandCreateHuman {
                name: profile.name.clone(),
                biological_sex,
                birth_timestamp: profile.birth_timestamp.clone(),
                age_years: profile.age_years,
                height_cm: profile.height_cm,
                build: profile.build.clone(),
                hair_color: profile.hair_color.clone(),
                eye_color: profile.eye_color.clone(),
                skin_tone: profile.skin_tone.clone(),
                location: CreateLocation::IslandCell { row, col },
                // The profile carries its own birthplace, and an operator
                // spawn is explicit about it, so it is taken as given
                // rather than overwritten with where they were put.
                birthplace_here: false,
                birth_latitude: profile.birth_latitude,
                birth_longitude: profile.birth_longitude,
            })
            .map_err(IslandInterventionError::Create)?;
        // A folder that could not be written does not stop the person
        // existing, but it must not vanish either: `CreatedIslander`
        // carries it for the dashboard, and swallowing it here would leave
        // an operator spawn as the one path where nobody is told.
        let storage = created
            .storage_error
            .map(|why| format!(" (their folder was not written: {why})"))
            .unwrap_or_default();
        Ok(mutated(
            format!(
                "created '{}' at cell ({row}, {col}){storage}",
                created.agent_id
            ),
            1,
        ))
    }

    /// Take a person out of the world.
    ///
    /// Four places, not one. Upstream removes a human from the registry and
    /// that is the whole of it; here a person is also an entry in the estate
    /// position table, a body in the material ledger, and two running
    /// accumulators (carbon respired since their last meal, and seconds
    /// asleep and awake). Leaving any of them behind would leave the island
    /// carrying a ghost — which is exactly the defect
    /// `the_dead_stop_accumulating_hours_and_meals` was written for, and a
    /// removal has the same shape as a death in that respect.
    ///
    /// The Reverence Veto refusal is propagated, never swallowed.
    fn remove_human(&mut self, human_id: &str) -> Result<IslandApplied, IslandInterventionError> {
        let removed = self
            .humans
            .registry
            .remove_human(human_id)?
            .ok_or_else(|| IslandInterventionError::NotFound {
                what: format!("human '{human_id}'"),
            })?;
        let agent_id = removed.agent_id().to_string();
        self.positions.0.remove(&agent_id);
        self.forget_accumulators(&agent_id);
        // The body leaves the ledger through the boundary, audited like any
        // other change to the island's stocks.
        let mut material = Ok(());
        self.audited(|life, ledger| {
            material = life.materials.remove_body(&agent_id, ledger);
        })
        .map_err(|e| IslandInterventionError::Audit(e.to_string()))?;
        // A person who was never registered with the ledger has no body to
        // take out; that is not a failure of the removal.
        if let Err(super::materials::MaterialError::NoSuchHuman(_)) = material {
            material = Ok(());
        }
        material.map_err(IslandInterventionError::Material)?;
        Ok(mutated(format!("removed human '{agent_id}'"), 1))
    }
}
