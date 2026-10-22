//! Which model stands for which thing on the estate.
//!
//! The island knows the founders own "Gem-D's Computer (gaming, 3
//! monitors)" in the Computer Room. A renderer has to turn that into a
//! file, and this is the table that does it. It is a table and not a
//! guess, because a renderer that fell back to something plausible would
//! draw a garage as a shed and nobody would notice it was wrong.
//!
//! Everything here is a *presentation* decision. The simulation does not
//! know these files exist and nothing here is hashed.
//!
//! The models are the Maer-Ken `gem_property` set, vendored under
//! `assets/` with their generator script and their provenance; see
//! `assets/CREDITS.md`, which is blunt about what they are: deterministic
//! low-poly reconstructions built by a Blender script from primitives.

/// What is missing, and said rather than substituted.
///
/// Three kinds of thing on this estate have no model upstream either.
/// They are drawn at their real size -- the island serves every building's
/// footprint and height and every item's length, width and height, each
/// with its source -- and carry no model on top. A renderer that silently
/// drew a shed for a garage would be lying in a way nobody could see.
///
/// There used to be a "placeholder" model here, standing for these. It
/// named a file that did not exist and was filtered out before anything
/// loaded it, so it was never drawn: it was a fake value standing in for
/// "nothing", which is what `None` is for.
pub const WITHOUT_MODELS: [&str; 3] = ["Garage", "ShedTool", "VehicleAttachment"];

/// A model, and whether it is really that thing or a stand-in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Model {
    /// Path under the asset root, as Bevy's asset server wants it.
    pub path: &'static str,
    /// False when the model is somebody else's: a person created after
    /// the founders is drawn with a founder's model by sex, and the
    /// inspector says so. Every building and item model is `true`, because
    /// a thing with no model of its own gets `None` rather than a stand-in.
    pub is_really_this: bool,
}

impl Model {
    const fn real(path: &'static str) -> Self {
        Self {
            path,
            is_really_this: true,
        }
    }
}

/// A building's model, by its `PropertyBuildingKind` in words.
///
/// The computer room is a building to the engine and a *room inside the
/// house* on the ground, which the estate layout already reflects: its
/// footprint is inside the House's. The model is a room, not a building,
/// and placing it is `EstateLayout`'s business rather than this table's.
pub fn building(kind: &str) -> Option<Model> {
    match kind {
        "House" => Some(Model::real("property/buildings/homestead_house.glb")),
        "Shed" => Some(Model::real("property/buildings/equipment_shed.glb")),
        "Workshop" => Some(Model::real("property/buildings/building_workshop.glb")),
        "Armoury" => Some(Model::real("property/buildings/secure_armoury.glb")),
        "ComputerRoom" => Some(Model::real("property/buildings/computer_room.glb")),
        // Upstream has no garage model either. Drawing the shed instead
        // would be a lie nobody could see; the garage is drawn at its own
        // footprint and height and the key says it has no model.
        _ => None,
    }
}

/// An item's model, from its kind and its name.
///
/// The name matters as much as the kind: four things on this estate are
/// `Computer` and they are four different machines. Matched on a
/// distinctive fragment rather than the whole string, because the
/// manifest's names carry parenthetical detail ("(gaming, 3 monitors)")
/// that is part of what the thing is and no part of which model it is.
pub fn item(kind: &str, name: &str) -> Option<Model> {
    match kind {
        "Computer" => computer(name),
        "Vehicle" => vehicle(name),
        "BuildingEquipment" => Some(equipment(name)),
        "ArmouryItem" => Some(armoury(name)),
        "HouseholdItem" => Some(household(name)),
        // Garden tools and towed implements have no models upstream, and
        // an unknown kind gets none rather than a guess. Both are drawn at
        // the size the island gives them; `WITHOUT_MODELS` names them.
        _ => None,
    }
}

/// The four machines in the computer room.
///
/// Two of these are exact: the manifest and the asset set agree on "Local
/// Archive Server" and "Network Equipment Rack". The founders' own two
/// are a choice and it is recorded here rather than left implicit:
/// Gem-D's is described as gaming with three monitors and Gem-K's as a
/// workstation with two, and the asset set offers `primary_workstation`
/// and `portable_laptop`. Neither is a laptop. Gem-D's gets the
/// workstation model because a three-monitor desktop is the nearer of the
/// two, and Gem-K's gets it as well -- the same model, not the laptop,
/// because drawing a workstation as a laptop would be wrong in a way
/// somebody looking at the room would see.
///
/// `portable_laptop` is therefore unused by this table. It is kept in
/// `assets/` because it is part of the set and because the first thing on
/// this estate that is actually portable will want it.
fn computer(name: &str) -> Option<Model> {
    if name.contains("Archive Server") {
        Some(Model::real("computers/local_archive_server.glb"))
    } else if name.contains("Network Equipment") {
        Some(Model::real("computers/network_equipment_rack.glb"))
    } else if name.contains("Computer") {
        Some(Model::real("computers/primary_workstation.glb"))
    } else {
        // A machine the manifest names that is none of these four: no
        // model rather than the wrong one.
        None
    }
}

/// A vehicle's model from its name, or none when the asset set has nothing
/// that is that vehicle.
///
/// The set has four vehicles: a utility truck, a trailer, a motorbike and
/// an all-terrain utility vehicle. The estate's utes and side-by-sides are
/// those things; its tractors, golf cart and excavator are not, and
/// drawing a 30-tonne excavator as a pickup truck would be a picture of
/// something the garage does not hold. Those are drawn at the size the
/// island gives them instead, like every other thing without a model.
fn vehicle(name: &str) -> Option<Model> {
    let lower = name.to_ascii_lowercase();
    let has = |words: &[&str]| words.iter().any(|w| lower.contains(w));
    if has(&["trailer"]) {
        Some(Model::real("vehicles/utility_trailer.glb"))
    } else if has(&["excavator", "digger", "tractor", "fendt", "golf cart"]) {
        None
    } else if has(&["bike", "motorcycle"]) {
        Some(Model::real("vehicles/motorbike.glb"))
    } else if has(&["quad", "atv", "terrain", "rzr", "side-by-side", "utv"]) {
        Some(Model::real("vehicles/all_terrain_utility_vehicle.glb"))
    } else if has(&["ute", "truck", "pickup", "raptor"]) {
        Some(Model::real("vehicles/utility_truck.glb"))
    } else {
        // A vehicle this table has never heard of: no model rather than a
        // guess at one.
        None
    }
}

fn equipment(name: &str) -> Model {
    let lower = name.to_ascii_lowercase();
    if lower.contains("workbench") {
        Model::real("tools/heavy_workbench.glb")
    } else if lower.contains("generator") {
        Model::real("tools/portable_generator.glb")
    } else if lower.contains("weld") {
        Model::real("tools/welding_station.glb")
    } else if lower.contains("ladder") {
        Model::real("tools/ladder_set.glb")
    } else if lower.contains("hoist") {
        Model::real("tools/material_hoist.glb")
    } else if lower.contains("power") {
        Model::real("tools/power_tool_set.glb")
    } else {
        Model::real("tools/hand_tool_set.glb")
    }
}

fn armoury(name: &str) -> Model {
    let lower = name.to_ascii_lowercase();
    if lower.contains("locker") {
        Model::real("property/armoury/secure_locker.glb")
    } else if lower.contains("protective") {
        Model::real("property/armoury/protective_equipment_set.glb")
    } else {
        Model::real("property/armoury/field_safety_kit.glb")
    }
}

fn household(name: &str) -> Model {
    let lower = name.to_ascii_lowercase();
    if lower.contains("tank") || lower.contains("water") {
        Model::real("property/household/water_storage_tank.glb")
    } else if lower.contains("medical") {
        Model::real("property/household/emergency_medical_kit.glb")
    } else {
        Model::real("property/household/household_storage_set.glb")
    }
}

/// A human's model.
///
/// The founders have their own; anybody created later has none, and gets
/// Gem-D's or Gem-K's by sex. That is a stand-in and the inspector says
/// so, because a roster where every woman has Gem-K's face would
/// otherwise read as a claim about what these people look like.
pub fn human(agent_id: &str, biological_sex_is_female: bool) -> Model {
    match agent_id {
        "Gem-D" => Model::real("humans/GemD.glb"),
        "Gem-K" => Model::real("humans/GemK.glb"),
        _ if biological_sex_is_female => Model {
            path: "humans/GemK.glb",
            is_really_this: false,
        },
        _ => Model {
            path: "humans/GemD.glb",
            is_really_this: false,
        },
    }
}

/// A human's portrait, under `assets/`.
///
/// Only the founders have one. Unlike the models, nobody else is given a
/// founder's face as a stand-in: a picture of a face in an inspector reads
/// as what that person looks like, and nobody created later looks like
/// anybody yet.
pub fn portrait(agent_id: &str) -> Option<&'static str> {
    match agent_id {
        "Gem-D" => Some("humans/portraits/GemD.png"),
        "Gem-K" => Some("humans/portraits/GemK.png"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every `PropertyBuildingKind` in `mk_engine::organisms::property`.
    const BUILDING_KINDS: [&str; 6] = [
        "House",
        "Shed",
        "Workshop",
        "Armoury",
        "ComputerRoom",
        "Garage",
    ];

    /// Every `PropertyItemKind`.
    const ITEM_KINDS: [&str; 7] = [
        "Vehicle",
        "VehicleAttachment",
        "ShedTool",
        "BuildingEquipment",
        "ArmouryItem",
        "Computer",
        "HouseholdItem",
    ];

    fn asset_root() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets")
    }

    #[test]
    fn every_building_kind_and_item_kind_has_an_answer() {
        // Not "does not panic": an exhaustive list, so a kind added
        // upstream and not added here shows up as a missing model in this
        // test rather than unnoticed on the estate.
        for kind in BUILDING_KINDS {
            assert_eq!(
                building(kind).is_some(),
                !WITHOUT_MODELS.contains(&kind),
                "{kind} disagrees with WITHOUT_MODELS"
            );
        }
        // A name the manifest really uses for each kind. The first
        // version of this passed "Something" to all seven and failed on
        // `Computer`, which was the test being lazy rather than the table
        // being wrong: the four machines in that room are told apart by
        // name, so a name that is none of them correctly has no model.
        for (kind, name) in ITEM_KINDS.iter().zip([
            "Utility Truck",
            "Tractor Tipper Trailer",
            "Digging Tools",
            "Workbench",
            "Secure Locker",
            "Gem-D's Computer (gaming, 3 monitors)",
            "Water Storage Tank",
        ]) {
            assert_eq!(
                item(kind, name).is_some(),
                !WITHOUT_MODELS.contains(kind),
                "{kind} ({name}) disagrees with WITHOUT_MODELS"
            );
        }
    }

    /// The two lists above are written out by hand, so this holds them to
    /// the estate the island really builds: a building or item kind the
    /// engine adds there and nobody adds here fails, rather than the
    /// exhaustive test passing over a list that has fallen behind.
    #[test]
    fn the_kind_lists_cover_every_kind_on_the_founders_estate() {
        use mk_engine::organisms::property::StarterProperty;
        let estate = StarterProperty::founders_estate((0, 0));
        for building in &estate.buildings {
            let kind = format!("{:?}", building.kind);
            assert!(
                BUILDING_KINDS.contains(&kind.as_str()),
                "{} is a {kind}, which BUILDING_KINDS leaves out",
                building.name
            );
        }
        for item in &estate.items {
            let kind = format!("{:?}", item.kind);
            assert!(
                ITEM_KINDS.contains(&kind.as_str()),
                "{} is a {kind}, which ITEM_KINDS leaves out",
                item.name
            );
        }
    }

    #[test]
    fn every_model_this_table_names_is_a_file_that_exists() {
        // The failure this prevents is the worst kind: a renderer that
        // loads nothing, logs nothing a person reads, and draws an empty
        // estate.
        let root = asset_root();
        let mut named = vec![
            human("Gem-D", false).path.to_string(),
            human("Gem-K", true).path.to_string(),
        ];
        for kind in BUILDING_KINDS {
            named.extend(building(kind).map(|m| m.path.to_string()));
        }
        for kind in ITEM_KINDS {
            for name in [
                "Gem-D's Computer (gaming, 3 monitors)",
                "Gem-K's Computer (workstation, 2 monitors)",
                "Local Archive Server",
                "Network Equipment Rack",
                "Utility Truck",
                "Double Axle Trailer",
                "Motorbike",
                "All Terrain Vehicle",
                "Workbench",
                "Portable Generator",
                "Welding Torch",
                "Ladder Set",
                "Material Hoist",
                "Power Tool Set",
                "Secure Locker",
                "Protective Equipment Set",
                "Field Safety Kit",
                "Water Storage Tank",
                "Emergency Medical Kit",
                "Household Storage Set",
            ] {
                named.extend(item(kind, name).map(|m| m.path.to_string()));
            }
        }
        named.sort();
        named.dedup();
        for path in named {
            assert!(
                root.join(&path).is_file(),
                "{path} is named by this table and is not in assets/"
            );
        }
    }

    #[test]
    fn the_four_machines_in_the_computer_room_are_four_different_things() {
        // The founders' two share a model, which is a recorded choice;
        // the server and the rack are their own. Three distinct models
        // for four machines, and every one of them real.
        let machines = [
            "Gem-D's Computer (gaming, 3 monitors)",
            "Gem-K's Computer (workstation, 2 monitors)",
            "Local Archive Server",
            "Network Equipment Rack",
        ];
        let models: Vec<Model> = machines
            .iter()
            .map(|name| item("Computer", name).expect("every machine in that room has a model"))
            .collect();
        let distinct: std::collections::BTreeSet<&str> = models.iter().map(|m| m.path).collect();
        assert_eq!(
            distinct.len(),
            3,
            "the server and the rack must not be drawn as the founders' desktops: {distinct:?}"
        );
        assert!(item("Computer", "Local Archive Server")
            .is_some_and(|m| m.path.contains("local_archive_server")));
        assert!(item("Computer", "Network Equipment Rack")
            .is_some_and(|m| m.path.contains("network_equipment_rack")));
    }

    #[test]
    fn a_name_the_table_has_never_seen_gets_no_model() {
        // The manifest can grow. Something the table has never heard of --
        // a vehicle or a whole kind -- is drawn at its real size with no
        // model, because any model would be a guess at what it is. (An
        // unknown vehicle used to get the truck; the estate's excavator and
        // tractors were drawn as trucks that way.)
        assert!(item("Vehicle", "Hovercraft").is_none());
        assert!(item("Submarine", "Nautilus").is_none());
        assert!(building("Lighthouse").is_none());
    }

    /// Every vehicle on the founders' estate, by the manifest's own name,
    /// gets the model of what it is or none at all -- never another
    /// vehicle's.
    #[test]
    fn every_vehicle_on_the_estate_is_drawn_as_itself_or_as_its_size() {
        use mk_engine::organisms::property::{PropertyItemKind, StarterProperty};
        let estate = StarterProperty::founders_estate((0, 0));
        let vehicles: Vec<&str> = estate
            .items
            .iter()
            .filter(|i| i.kind == PropertyItemKind::Vehicle)
            .map(|i| i.name.as_str())
            .collect();
        let expected = [
            (
                "Polaris RZR 1000 Turbo",
                Some("vehicles/all_terrain_utility_vehicle.glb"),
            ),
            (
                "Polaris RZR 2000 PRO R",
                Some("vehicles/all_terrain_utility_vehicle.glb"),
            ),
            ("Ford Raptor 4x4 Ute", Some("vehicles/utility_truck.glb")),
            ("Golf Cart", None),
            ("Fendt 900 Vario", None),
            ("Fendt 1000 Vario", None),
            ("30-Ton Excavator (CAT 330)", None),
            ("Double Axle Trailer", Some("vehicles/utility_trailer.glb")),
            (
                "Tractor Tipper Trailer",
                Some("vehicles/utility_trailer.glb"),
            ),
        ];
        for name in &vehicles {
            let (_, want) = expected
                .iter()
                .find(|(n, _)| n == name)
                .unwrap_or_else(|| panic!("{name} is new on the estate: say what it is drawn as"));
            assert_eq!(item("Vehicle", name).map(|m| m.path), *want, "{name}");
        }
        assert_eq!(vehicles.len(), expected.len(), "a vehicle left the estate");
    }

    #[test]
    fn a_created_human_is_marked_as_wearing_somebody_elses_face() {
        assert!(human("Gem-D", false).is_really_this);
        assert!(human("Gem-K", true).is_really_this);
        let made_up = human("hine-moana", true);
        assert!(
            !made_up.is_really_this,
            "a roster where every woman has Gem-K's face must say so"
        );
        assert_eq!(made_up.path, "humans/GemK.glb");
    }

    #[test]
    fn only_the_founders_have_portraits_and_both_are_files() {
        for founder in ["Gem-D", "Gem-K"] {
            let path = portrait(founder).expect("a founder has a portrait");
            assert!(asset_root().join(path).is_file(), "{path} is missing");
        }
        assert_eq!(portrait("hine-moana"), None, "no stand-in faces");
    }
}
