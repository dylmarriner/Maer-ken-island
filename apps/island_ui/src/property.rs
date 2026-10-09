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
/// Three kinds of thing on this estate have no model upstream either, and
/// the honest answer is a placeholder plus this list. A renderer that
/// silently drew a shed for a garage would be lying in a way nobody could
/// see.
pub const WITHOUT_MODELS: [&str; 3] = ["Garage", "ShedTool", "VehicleAttachment"];

/// A model, and whether it is really that thing or a stand-in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Model {
    /// Path under the asset root, as Bevy's asset server wants it.
    pub path: &'static str,
    /// False when nothing models this and the renderer is drawing a box.
    /// The key on screen says so; it is not left to be inferred from a
    /// shape that happens to look wrong.
    pub is_really_this: bool,
}

impl Model {
    const fn real(path: &'static str) -> Self {
        Self {
            path,
            is_really_this: true,
        }
    }

    const fn placeholder() -> Self {
        Self {
            path: PLACEHOLDER,
            is_really_this: false,
        }
    }
}

/// Drawn for anything with no model of its own: an untextured box at the
/// right place, which the key names as a placeholder.
pub const PLACEHOLDER: &str = "property/placeholder.glb";

/// A building's model, by its `PropertyBuildingKind` in words.
///
/// The computer room is a building to the engine and a *room inside the
/// house* on the ground, which the estate layout already reflects: its
/// footprint is inside the House's. The model is a room, not a building,
/// and placing it is `EstateLayout`'s business rather than this table's.
pub fn building(kind: &str) -> Model {
    match kind {
        "House" => Model::real("property/buildings/homestead_house.glb"),
        "Shed" => Model::real("property/buildings/equipment_shed.glb"),
        "Workshop" => Model::real("property/buildings/building_workshop.glb"),
        "Armoury" => Model::real("property/buildings/secure_armoury.glb"),
        "ComputerRoom" => Model::real("property/buildings/computer_room.glb"),
        // Upstream has no garage model either. Drawing the shed instead
        // would be a lie nobody could see, so this is a box and the key
        // says it is a box.
        "Garage" => Model::placeholder(),
        _ => Model::placeholder(),
    }
}

/// An item's model, from its kind and its name.
///
/// The name matters as much as the kind: four things on this estate are
/// `Computer` and they are four different machines. Matched on a
/// distinctive fragment rather than the whole string, because the
/// manifest's names carry parenthetical detail ("(gaming, 3 monitors)")
/// that is part of what the thing is and no part of which model it is.
pub fn item(kind: &str, name: &str) -> Model {
    match kind {
        "Computer" => computer(name),
        "Vehicle" => vehicle(name),
        "BuildingEquipment" => equipment(name),
        "ArmouryItem" => armoury(name),
        "HouseholdItem" => household(name),
        // Garden tools and towed implements: no models upstream, so
        // boxes, and `WITHOUT_MODELS` names them.
        "ShedTool" | "VehicleAttachment" => Model::placeholder(),
        _ => Model::placeholder(),
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
fn computer(name: &str) -> Model {
    if name.contains("Archive Server") {
        Model::real("computers/local_archive_server.glb")
    } else if name.contains("Network Equipment") {
        Model::real("computers/network_equipment_rack.glb")
    } else if name.contains("Computer") {
        Model::real("computers/primary_workstation.glb")
    } else {
        Model::placeholder()
    }
}

fn vehicle(name: &str) -> Model {
    let lower = name.to_ascii_lowercase();
    if lower.contains("trailer") {
        Model::real("vehicles/utility_trailer.glb")
    } else if lower.contains("bike") || lower.contains("motorcycle") {
        Model::real("vehicles/motorbike.glb")
    } else if lower.contains("quad") || lower.contains("atv") || lower.contains("terrain") {
        Model::real("vehicles/all_terrain_utility_vehicle.glb")
    } else {
        // Truck, ute, tractor: the utility truck is the general vehicle
        // of this set, and it is a real model rather than a placeholder.
        Model::real("vehicles/utility_truck.glb")
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
        // upstream and not added here shows up as a placeholder in this
        // test rather than as a box on the estate.
        for kind in BUILDING_KINDS {
            let model = building(kind);
            assert_eq!(
                model.is_really_this,
                !WITHOUT_MODELS.contains(&kind),
                "{kind} disagrees with WITHOUT_MODELS"
            );
        }
        // A name the manifest really uses for each kind. The first
        // version of this passed "Something" to all seven and failed on
        // `Computer`, which was the test being lazy rather than the table
        // being wrong: the four machines in that room are told apart by
        // name, so a name that is none of them is correctly a
        // placeholder.
        for (kind, name) in ITEM_KINDS.iter().zip([
            "Utility Truck",
            "Tractor Tipper Trailer",
            "Digging Tools",
            "Workbench",
            "Secure Locker",
            "Gem-D's Computer (gaming, 3 monitors)",
            "Water Storage Tank",
        ]) {
            let model = item(kind, name);
            assert_eq!(
                model.is_really_this,
                !WITHOUT_MODELS.contains(kind),
                "{kind} ({name}) disagrees with WITHOUT_MODELS"
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
            PLACEHOLDER.to_string(),
            human("Gem-D", false).path.to_string(),
            human("Gem-K", true).path.to_string(),
        ];
        for kind in BUILDING_KINDS {
            named.push(building(kind).path.to_string());
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
                named.push(item(kind, name).path.to_string());
            }
        }
        named.sort();
        named.dedup();
        for path in named {
            if path == PLACEHOLDER {
                // Generated by the renderer rather than loaded, so there
                // is no file to check -- and that is deliberate: a
                // missing-asset placeholder that is itself a missing
                // asset would be useless.
                continue;
            }
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
        // for four machines, and none of them a placeholder.
        let machines = [
            "Gem-D's Computer (gaming, 3 monitors)",
            "Gem-K's Computer (workstation, 2 monitors)",
            "Local Archive Server",
            "Network Equipment Rack",
        ];
        let models: Vec<Model> = machines.iter().map(|name| item("Computer", name)).collect();
        assert!(
            models.iter().all(|m| m.is_really_this),
            "every machine in that room has a model: {models:?}"
        );
        let distinct: std::collections::BTreeSet<&str> = models.iter().map(|m| m.path).collect();
        assert_eq!(
            distinct.len(),
            3,
            "the server and the rack must not be drawn as the founders' desktops: {distinct:?}"
        );
        assert!(item("Computer", "Local Archive Server")
            .path
            .contains("local_archive_server"));
        assert!(item("Computer", "Network Equipment Rack")
            .path
            .contains("network_equipment_rack"));
    }

    #[test]
    fn a_name_the_table_has_never_seen_still_gets_something() {
        // The manifest can grow. An unknown vehicle is a vehicle, and
        // drawing the truck is better than drawing a box; an unknown
        // *kind* is a box, because guessing there would be inventing.
        assert!(item("Vehicle", "Hovercraft").is_really_this);
        assert!(!item("Submarine", "Nautilus").is_really_this);
        assert!(!building("Lighthouse").is_really_this);
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
}
