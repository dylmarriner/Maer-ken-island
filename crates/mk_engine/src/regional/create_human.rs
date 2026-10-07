//! Phase 4 Task 6: putting a person into the running island.
//!
//! Creating a human from the dashboard used to mean storing one: a complete
//! canonical record in a folder, with nowhere to be. This puts them in the
//! world — on the estate in a named room, or on a cell of the island — so
//! that from the next step they age, breathe and sleep alongside the
//! founders.
//!
//! Every rule here refuses rather than guesses. A room that does not exist,
//! a cell in the sea, an age no human has reached: each is an error naming
//! the field, because a creation that silently lands somewhere other than
//! where it was asked for is worse than one that does not happen.

use mk_core::human::BiologicalSex;
use mk_interventions::HumanSpawnProfile;

use super::estate_layout::{Space, SpaceId};
use super::life::IslandLife;
use crate::humans::spawn::{agent_id_for_name, build_authored_human};
use mk_island::DomainLevel;

/// Where a new person starts.
#[derive(Debug, Clone, PartialEq)]
pub enum CreateLocation {
    /// A room, zone or building of the founders' estate, by the layout's
    /// own id. They stand at its centre.
    EstateSpace(SpaceId),
    /// A cell of the island's medium grid. Must be land.
    IslandCell { row: usize, col: usize },
}

/// A request to create one person in the world.
#[derive(Debug, Clone)]
pub struct IslandCreateHuman {
    pub name: String,
    pub biological_sex: BiologicalSex,
    /// RFC 3339.
    pub birth_timestamp: String,
    pub age_years: f64,
    pub height_cm: f64,
    pub build: String,
    pub hair_color: String,
    pub eye_color: String,
    pub skin_tone: String,
    pub location: CreateLocation,
    /// Take the birthplace from the chosen location rather than from the
    /// coordinates below. Somebody may be born off the island, so this is a
    /// choice rather than an assumption.
    pub birthplace_here: bool,
    pub birth_latitude: f64,
    pub birth_longitude: f64,
}

/// Why a creation did not happen, or what happened alongside it.
#[derive(Debug)]
pub enum CreateHumanError {
    /// One message per problem, each naming its field.
    Invalid(Vec<String>),
    /// Upstream's spawn refused the profile.
    Spawn(String),
}

impl std::fmt::Display for CreateHumanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(problems) => write!(f, "{}", problems.join("; ")),
            Self::Spawn(why) => write!(f, "{why}"),
        }
    }
}

impl std::error::Error for CreateHumanError {}

/// Who was created, and where they ended up.
#[derive(Debug, Clone)]
pub struct CreatedIslander {
    pub agent_id: String,
    /// The layout's own label for where they are, when they are on the
    /// estate.
    pub space: Option<String>,
    pub position_m: Option<(f64, f64)>,
    pub cell: (usize, usize),
    pub birth_latitude: f64,
    pub birth_longitude: f64,
    /// A folder that could not be written. The person still exists: losing
    /// the record of somebody is not a reason to refuse them, and silently
    /// dropping the error would leave nobody able to tell.
    pub storage_error: Option<String>,
}

/// The oldest age this accepts, in years.
///
/// The reference packs' `maximum_verified_lifespan` is 122.45 years
/// (Jeanne Calment), which `human_realism` already holds the model to.
/// Creating somebody older than anyone has lived is a typo, not a wish.
const OLDEST_YEARS: f64 = 122.45;

impl IslandLife {
    /// Create a person and put them in the world.
    ///
    /// They are added through the registry, so a folder is written when the
    /// run keeps them (Task 3b), and they appear in the next step's
    /// projection. The simulation is not advanced here: the caller owns the
    /// clock.
    pub fn create_human(
        &mut self,
        request: IslandCreateHuman,
    ) -> Result<CreatedIslander, CreateHumanError> {
        let placement = self.check(&request)?;

        let agent_id = agent_id_for_name(&self.humans.registry, &request.name);
        let (latitude, longitude) = if request.birthplace_here {
            placement.lat_lon
        } else {
            (request.birth_latitude, request.birth_longitude)
        };
        let profile = HumanSpawnProfile {
            name: request.name.trim().to_string(),
            birth_timestamp: request.birth_timestamp.clone(),
            birth_latitude: latitude,
            birth_longitude: longitude,
            age_years: request.age_years,
            height_cm: request.height_cm,
            build: request.build.clone(),
            hair_color: request.hair_color.clone(),
            eye_color: request.eye_color.clone(),
            skin_tone: request.skin_tone.clone(),
        };

        // Keyed on the tick, as every other creation in the world is, so a
        // replay at the same tick rebuilds the same person.
        let human = build_authored_human(
            &self.rng,
            self.tick,
            agent_id.clone(),
            request.biological_sex,
            &profile,
        )
        .map_err(|e| CreateHumanError::Spawn(e.to_string()))?;

        // The material ledger needs a body before anybody can breathe. The
        // founders are registered at bootstrap; without the same call here a
        // created person would be in the world, visibly, and never respire —
        // no body carbon, no meals, outside the flows entirely. They would
        // look alive on the page and not be.
        let weight_kg = human.body.weight_kg;
        let mut storage_errors = Vec::new();
        if let Err(e) = self.humans.registry.add_human(human) {
            storage_errors.push(e.to_string());
        }
        self.materials.register_human(&agent_id, weight_kg);
        let event = serde_json::json!({
            "kind": "created",
            "by": "dashboard",
            "tick": self.tick,
            "sim_time_s": self.sim_time_s,
        });
        if let Err(e) = self.humans.registry.record_event(&agent_id, &event) {
            storage_errors.push(e.to_string());
        }

        // Two separate things, and the first version of this set only the
        // second: `positions` is the metre-level side table for people
        // inside the estate patch, while the runtime position is where the
        // human runtime itself thinks they are. Without the runtime
        // position they are nowhere the step recognises, and the next step
        // drops their estate entry — which is exactly what
        // `somebody_created_in_a_room_is_in_that_room_and_in_the_world`
        // caught. The founders are placed by both, so a created person is
        // placed by both.
        if let Some(human) = self.humans.registry.get_human_mut(&agent_id) {
            human.set_runtime_position(placement.grid);
        }
        if let Some(position) = placement.estate {
            self.positions.0.insert(agent_id.clone(), position);
        }

        Ok(CreatedIslander {
            agent_id,
            space: placement.space_label,
            position_m: placement.estate.map(|p| p.position_m),
            cell: placement.cell,
            birth_latitude: latitude,
            birth_longitude: longitude,
            storage_error: (!storage_errors.is_empty()).then(|| storage_errors.join("; ")),
        })
    }

    /// Everything that must be true before anybody is built.
    ///
    /// Collected rather than returned one at a time: somebody filling in a
    /// form wants every problem at once, not to be sent round again for the
    /// next one.
    fn check(&self, request: &IslandCreateHuman) -> Result<Placement, CreateHumanError> {
        let mut problems = Vec::new();

        if request.name.trim().is_empty() {
            problems.push("name: somebody needs a name.".to_string());
        }
        if !request.age_years.is_finite() || request.age_years < 0.0 {
            problems.push("age_years: an age cannot be negative.".to_string());
        } else if request.age_years > OLDEST_YEARS {
            problems.push(format!(
                "age_years: {:.1} is older than anyone has verifiably lived ({OLDEST_YEARS:.2}).",
                request.age_years
            ));
        }
        if !request.height_cm.is_finite() || !(50.0..=272.0).contains(&request.height_cm) {
            problems.push(
                "height_cm: must be between 50 and 272 — the tallest verified human was 272 cm."
                    .to_string(),
            );
        }
        if !request.birthplace_here
            && (!(-90.0..=90.0).contains(&request.birth_latitude)
                || !(-180.0..=180.0).contains(&request.birth_longitude))
        {
            problems.push("birth_latitude/birth_longitude: not a place on a planet.".to_string());
        }

        let placement = match self.place(&request.location) {
            Ok(placement) => Some(placement),
            Err(problem) => {
                problems.push(problem);
                None
            }
        };

        match (problems.is_empty(), placement) {
            (true, Some(placement)) => Ok(placement),
            _ => Err(CreateHumanError::Invalid(problems)),
        }
    }

    /// Turn a requested location into a cell, a coordinate and, on the
    /// estate, a position inside the patch.
    fn place(&self, location: &CreateLocation) -> Result<Placement, String> {
        match location {
            CreateLocation::EstateSpace(id) => {
                let space = self
                    .placed
                    .layout
                    .spaces
                    .iter()
                    .find(|s| s.id == *id)
                    .ok_or_else(|| {
                        format!(
                            "location: the estate has no space {}. It has {}.",
                            id.0,
                            self.placed
                                .layout
                                .spaces
                                .iter()
                                .map(|s| s.label.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        )
                    })?;
                let rect = space.rect_m;
                let position_m = ((rect.x0 + rect.x1) / 2.0, (rect.y0 + rect.y1) / 2.0);
                Ok(Placement {
                    cell: self.placed.location,
                    lat_lon: self.patch_lat_lon(position_m),
                    grid: super::humans::medium_cell_of(&self.domain, position_m),
                    estate: Some(super::humans::EstatePosition {
                        space: Space::Inside(*id),
                        position_m,
                    }),
                    space_label: Some(space.label.clone()),
                })
            }
            CreateLocation::IslandCell { row, col } => {
                let (rows, cols) = (
                    self.domain.rows(DomainLevel::Medium),
                    self.domain.cols(DomainLevel::Medium),
                );
                if *row >= rows || *col >= cols {
                    return Err(format!(
                        "location: cell ({row}, {col}) is off the island's {rows} by {cols} grid."
                    ));
                }
                if !*self.physical.geophysics.land_mask.get(*row, *col) {
                    return Err(format!("location: cell ({row}, {col}) is in the sea."));
                }
                let size = self.domain.cell_size_m(DomainLevel::Medium);
                let (x, y) = ((*col as f64 + 0.5) * size, (*row as f64 + 0.5) * size);
                Ok(Placement {
                    cell: (*row, *col),
                    lat_lon: self.domain.lat_lon_at_m(x, y),
                    grid: crate::agents::GridPosition::new(*row as i32, *col as i32),
                    // Only the estate's own patch has metre positions.
                    estate: None,
                    space_label: None,
                })
            }
        }
    }

    /// Where a point on the estate patch is on the planet.
    fn patch_lat_lon(&self, position_m: (f64, f64)) -> (f64, f64) {
        let size = self.domain.cell_size_m(DomainLevel::Medium);
        let (row, col) = self.placed.location;
        self.domain.lat_lon_at_m(
            col as f64 * size + position_m.0,
            row as f64 * size + position_m.1,
        )
    }
}

/// A validated location.
struct Placement {
    cell: (usize, usize),
    lat_lon: (f64, f64),
    /// Where the human runtime itself places them.
    grid: crate::agents::GridPosition,
    estate: Option<super::humans::EstatePosition>,
    space_label: Option<String>,
}
