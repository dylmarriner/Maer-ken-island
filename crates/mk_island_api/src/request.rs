//! What a frontend sends the island, and what it gets back for having
//! sent it.
//!
//! Writes are asynchronous by design. The simulation owns its state on one
//! thread, so a command is queued and applied between steps; the reply is a
//! command id and a URL to poll, never the result. [`Outcome`] is what that
//! poll eventually answers, and it is a sum type rather than a success flag
//! because "refused, and here is why" is an ordinary answer on this island:
//! most upstream intervention variants are refused by name and reason.

use serde::{Deserialize, Serialize};

/// A request to create somebody *in the world*.
///
/// `space` names a room of the estate by its layout id; `row`/`col` put
/// them on a cell. Exactly one is required, because "somewhere" is not a
/// place and guessing one would put a person where nobody asked. The
/// server checks that; this type cannot, because whether a room exists and
/// whether a cell is in the sea are questions only the live island can
/// answer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateHumanRequest {
    pub name: String,
    /// `male` or `female`: the spawn templates the engine has.
    pub biological_sex: String,
    pub birth_timestamp: String,
    pub age_years: f64,
    pub height_cm: f64,
    pub build: String,
    pub hair_color: String,
    pub eye_color: String,
    pub skin_tone: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub space: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub row: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub col: Option<usize>,
    /// Default true: somebody created on the island was, as a rule, born
    /// there. A creator who means otherwise says so and gives coordinates.
    #[serde(default = "yes")]
    pub birthplace_here: bool,
    #[serde(default)]
    pub birth_latitude: f64,
    #[serde(default)]
    pub birth_longitude: f64,
}

fn yes() -> bool {
    true
}

impl CreateHumanRequest {
    /// Somebody created in a room of the estate.
    pub fn in_space(
        name: impl Into<String>,
        biological_sex: impl Into<String>,
        birth_timestamp: impl Into<String>,
        age_years: f64,
        space: u32,
    ) -> Self {
        Self {
            name: name.into(),
            biological_sex: biological_sex.into(),
            birth_timestamp: birth_timestamp.into(),
            age_years,
            height_cm: 170.0,
            build: "Average".to_string(),
            hair_color: "Brown".to_string(),
            eye_color: "Brown".to_string(),
            skin_tone: "Medium".to_string(),
            space: Some(space),
            row: None,
            col: None,
            birthplace_here: true,
            birth_latitude: 0.0,
            birth_longitude: 0.0,
        }
    }

    /// Somebody created on a cell of the island.
    pub fn on_cell(mut self, row: usize, col: usize) -> Self {
        self.space = None;
        self.row = Some(row);
        self.col = Some(col);
        self
    }
}

/// Running the island: pause it, resume it, step it, or change its speed.
///
/// Internally tagged on `command`, which is the shape the dashboard has
/// always posted and the shape this type now fixes for every client.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum ControlRequest {
    Pause,
    Resume,
    /// Advance exactly this many steps and then hold.
    Step {
        ticks: u64,
    },
    /// Write the whole island to the directory the server was started with.
    ///
    /// Slow enough that it is worth saying so: about half a minute on the
    /// full island, during which the simulation thread is writing rather
    /// than stepping.
    Snapshot,
    /// `real`, `max`, or a multiplier like `60`.
    SetSpeed {
        speed: String,
    },
}

/// Where an intervention is aimed, in degrees.
///
/// Degrees, not radians, because that is what the island's `Location` is
/// and what every form takes. The one time this was got wrong, a birth
/// came out 0.716 degrees from the equator.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct InterventionTarget {
    pub latitude: f64,
    pub longitude: f64,
}

impl InterventionTarget {
    pub fn new(latitude: f64, longitude: f64) -> Self {
        Self {
            latitude,
            longitude,
        }
    }

    /// The JSON an upstream `Location` deserializes from.
    ///
    /// `altitude` is sent explicitly as null. The island's `Location` has
    /// the field and does not default it, so leaving it out is a
    /// deserialization error rather than an omission.
    pub fn as_json(&self) -> serde_json::Value {
        serde_json::json!({
            "latitude": self.latitude,
            "longitude": self.longitude,
            "altitude": serde_json::Value::Null,
        })
    }

    /// A circle around this point, in kilometres: the island's `Region`.
    pub fn region(&self, radius_km: f64) -> serde_json::Value {
        serde_json::json!({ "center": self.as_json(), "radius_km": radius_km })
    }
}

/// An intervention, as JSON the island's own `InterventionAction` reads.
///
/// Deliberately **not** a copy of that enum. `InterventionAction` lives in
/// `mk_interventions` with the engine, has two dozen variants, and the
/// island refuses most of them by name and reason — terrain edits are canon
/// and hashed into the digest, there are no animal species at island scale,
/// the non-water resources are named materials instead. Re-declaring it
/// here would give a client a typed vocabulary that is mostly lies, and
/// would drift from the real one the first time upstream added a variant.
///
/// So this is the wire bytes, with constructors for the actions the island
/// actually applies. Anything else can still be sent — `from_json` takes
/// whatever you give it, and the island will answer with its own refusal,
/// which is more useful than one invented here. `apps/island`'s test suite
/// checks every constructor below against the real `InterventionAction`, so
/// these cannot quietly stop matching it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct InterventionRequest(pub serde_json::Value);

impl InterventionRequest {
    /// Any intervention at all, for a caller who has the JSON already.
    pub fn from_json(value: serde_json::Value) -> Self {
        Self(value)
    }

    /// Change one climate parameter. `parameter` is `Temperature`,
    /// `Precipitation`, `WindSpeed` or `SolarRadiation`; `region` is
    /// `None` for the whole island.
    pub fn modify_climate(parameter: &str, value: f32, region: Option<serde_json::Value>) -> Self {
        Self(serde_json::json!({
            "ModifyClimate": {
                "parameter": parameter,
                "value": value,
                "region": region,
            }
        }))
    }

    /// Put water into the island's stores. Water is the only resource the
    /// island accepts: the rest are named materials in the flux ledger.
    pub fn inject_water(amount_kg: f32, at: InterventionTarget) -> Self {
        Self(serde_json::json!({
            "InjectResource": {
                "resource_type": "Water",
                "amount": amount_kg,
                "location": at.as_json(),
            }
        }))
    }

    /// Put producer biomass into a region. Producers only: the island has
    /// no animal species to add biomass to.
    pub fn inject_producers(amount: f32, region: serde_json::Value) -> Self {
        Self(serde_json::json!({
            "InjectBiomass": {
                "biomass_type": "Producers",
                "amount": amount,
                "region": region,
            }
        }))
    }

    /// Build one of the resource economy's recipes at a place.
    pub fn construct(structure: &str, at: InterventionTarget) -> Self {
        Self(serde_json::json!({
            "ConstructStructure": {
                "structure": structure,
                "location": at.as_json(),
            }
        }))
    }

    /// Remove a human from the island: the registry, the estate table, the
    /// body in the material ledger and the two hashed accumulators.
    ///
    /// The field is `human_id`, which is the island's name for it, not
    /// `agent_id`, which is what the rest of the schema calls the same
    /// string. Renaming it here would make the request fail to parse.
    pub fn remove_human(human_id: impl Into<String>) -> Self {
        Self(serde_json::json!({ "RemoveHuman": { "human_id": human_id.into() } }))
    }

    /// An intervention the island refuses by name and reason — a terrain
    /// edit, which is canon and hashed into the digest.
    ///
    /// Here so a client can exercise a refusal deliberately rather than by
    /// mistake, and so the test that walks these constructors covers the
    /// refusal path as well as the applied one.
    pub fn sculpt_terrain(elevation_delta_m: f32, region: serde_json::Value) -> Self {
        Self(serde_json::json!({
            "SculptTerrain": {
                "elevation_delta_m": elevation_delta_m,
                "region": region,
            }
        }))
    }
}

/// What became of a command.
///
/// Moved here from the server so a client reads the same variants the
/// server writes. `Refused` is not an error path: it is the island saying
/// no, by name and reason, which is the behaviour the intervention table
/// promises.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Outcome {
    /// Accepted and waiting for the next step to apply it.
    Queued,
    Created {
        agent_id: String,
        space: Option<String>,
        cell: (usize, usize),
        tick: u64,
        /// A folder that could not be written. The person exists anyway.
        storage_error: Option<String>,
    },
    Refused {
        problems: Vec<String>,
    },
    /// An intervention the island carried out, or a directive it carried
    /// out on the loop.
    Intervened {
        summary: String,
        touched: usize,
        tick: u64,
    },
    /// A snapshot was written, and what it cost. The duration is reported
    /// because it is long enough that an operator deserves to be told
    /// rather than left watching a stopped clock.
    Saved {
        path: String,
        bytes: u64,
        took_ms: u64,
        tick: u64,
    },
    /// A control command: recorded, and nothing in the world moved.
    Noted,
}

/// A write the island accepted and queued: the id to poll, and where.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Accepted {
    pub command: u64,
    pub poll: String,
}

/// Why a request was not carried out.
///
/// `errors` is always sentences meant to be read by a person. `writes` and
/// `writes_mode` are present only on a refusal to write, and say which
/// of the three control modes the server is in, so a frontend can tell the
/// difference between "your token is wrong" and "this server does not take
/// writes from off-host at all".
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Refusal {
    #[serde(default)]
    pub errors: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub writes: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub writes_mode: Option<String>,
}

impl Refusal {
    /// The reasons as one sentence, for a frontend with one line to say it
    /// in. Empty only when the server sent no reason at all, which it is
    /// not supposed to do.
    pub fn message(&self) -> String {
        self.errors.join(" ")
    }
}
