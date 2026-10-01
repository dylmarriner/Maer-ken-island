//! Deterministic starter property and household physical storage.
//!
//! This is world state, not human identity data.  The property is deliberately
//! unowned when a world is created; ownership can be assigned later through a
//! logged action without seeding or inventing agents.
//!
//! The founders' estate's building/vehicle/tool/furniture inventory is a
//! faithful port of `gemini_universe`'s canonical island world definition
//! (`apps/backend/src/domains/cosmos/world/definitions/initial_island_world.ts`,
//! `CANONICAL_WORLD_VERSION = '3.0.0'`) — real named vehicles (RZR Turbo/PRO
//! R, Ford Raptor, Golf Cart, Fendt 900/1000 tractors, 30-Ton Excavator,
//! Double Axle Trailer, Tractor Tipper Trailer), real tractor attachments
//! (Stoll loaders, plow, mower, forklift, baler), the real categorized tool
//! shed inventory (construction/digging/repair/exploration/smithing tools),
//! and real per-room house furniture (twin bedrooms, bathroom, kitchen,
//! lounge, computer room). Brand/model names are carried over unchanged from
//! that source, matching what the fictional world actually specifies.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PropertyBuildingKind {
    House,
    Shed,
    Workshop,
    Armoury,
    ComputerRoom,
    /// Vehicle storage/maintenance building — `gemini_universe`'s `garage`
    /// location, distinct from the tool shed.
    Garage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PropertyItemKind {
    Vehicle,
    /// A towed/mounted implement for a specific vehicle (see
    /// `PropertyItem::attached_to`) — e.g. a tractor's front loader, plow,
    /// mower, forklift, or baler.
    VehicleAttachment,
    /// Outdoor/garden tools kept in the shed — distinct from `Vehicle`
    /// (also shed-located) and from `BuildingEquipment` (workshop tools).
    ShedTool,
    BuildingEquipment,
    ArmouryItem,
    Computer,
    HouseholdItem,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PropertyBuilding {
    pub id: u64,
    pub kind: PropertyBuildingKind,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PropertyItem {
    pub id: u64,
    pub kind: PropertyItemKind,
    pub name: String,
    pub location: u64,
    pub quantity: u32,
    /// A sub-grouping label: which room of the House this item furnishes
    /// (`"Kitchen"`, `"Lounge"`, `"Bathroom"`, `"<name>'s Bedroom"`,
    /// `"General"`), or which real tool-shed category a shed tool belongs
    /// to (`"Construction Tools"`, `"Digging Tools"`, `"Repair Tools"`,
    /// `"Exploration Tools"`, `"Smithing Tools"`). `None` for items with no
    /// further sub-grouping (vehicles, computers, armoury contents).
    ///
    /// `#[serde(default)]`: added after some `WorldState` snapshots may
    /// already exist on disk without this field — `Option<T>` fields are
    /// NOT automatically treated as missing-is-`None` by serde without an
    /// explicit default, so without this a load of an older snapshot fails
    /// deserialization outright (this exact bug was hit and fixed for
    /// `humans::technology::TechnologySnapshot::accumulated_knowledge`).
    #[serde(default)]
    pub room: Option<String>,
    /// For `PropertyItemKind::VehicleAttachment`: the item id of the
    /// vehicle this implement mounts to/tows behind. `None` for a shared
    /// implement usable by more than one vehicle (matching the source
    /// world, where `plow`/`mower`/`forklift` appear in both tractors'
    /// inventories) or for non-attachment items.
    #[serde(default)]
    pub attached_to: Option<u64>,
}

/// A real login/access record on the property's computer hardware — who
/// (`agent_id`) has an account, on which `device`, at what `access_level`.
/// Deliberately carries no credential/secret material (no password field):
/// this is world-state describing *who has access*, not an authentication
/// system.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkAccount {
    pub id: u64,
    pub agent_id: String,
    pub username: String,
    pub device: String,
    pub access_level: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StarterProperty {
    pub id: u64,
    pub name: String,
    pub owner: Option<u64>,
    /// Real human agent IDs (`HumanRegistry`/`HumanBeing::agent_id`, e.g.
    /// `"Gem-D"`) who own this property. Separate from `owner: Option<u64>`
    /// because agents are string-identified everywhere else in the engine
    /// (`resource_economy::ConstructionRecord.agent_id`,
    /// `HumanRegistry::get_human`) — `owner` has no established numeric ID
    /// space to draw from, so real ownership is expressed here instead of
    /// forcing a mismatched type.
    #[serde(default)]
    pub owner_agent_ids: Vec<String>,
    /// Real planetary grid cell (row, col) this property stands on, or
    /// `None` for a property with no fixed location (e.g. the unowned
    /// homestead template). Set once at world creation from the world's own
    /// deterministically classified biome grid — never invented.
    #[serde(default)]
    pub location: Option<(usize, usize)>,
    pub buildings: Vec<PropertyBuilding>,
    pub items: Vec<PropertyItem>,
    #[serde(default)]
    pub network_accounts: Vec<NetworkAccount>,
}

const HOUSE: u64 = 1;
const SHED: u64 = 2;
const WORKSHOP: u64 = 3;
const ARMOURY: u64 = 4;
const COMPUTER_ROOM: u64 = 5;
const GARAGE: u64 = 6;

/// Accumulates a property's item list with sequential ids — the bedroom
/// count varies per property (one generic bedroom for the unowned
/// homestead, one per named owner for an owned estate), so ids can't be
/// hardcoded literals the way a fixed-length list would allow. Also lets
/// vehicle attachments reference their vehicle's id after the fact (`push`
/// returns the id it assigned).
struct ItemListBuilder {
    next_id: u64,
    items: Vec<PropertyItem>,
}

impl ItemListBuilder {
    fn new() -> Self {
        Self {
            next_id: 1,
            items: Vec::new(),
        }
    }

    fn push(
        &mut self,
        kind: PropertyItemKind,
        name: &str,
        location: u64,
        room: Option<&str>,
    ) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.items.push(PropertyItem {
            id,
            kind,
            name: name.to_string(),
            location,
            quantity: 1,
            room: room.map(|r| r.to_string()),
            attached_to: None,
        });
        id
    }

    fn push_attachment(&mut self, name: &str, location: u64, attached_to: Option<u64>) {
        let id = self.next_id;
        self.next_id += 1;
        self.items.push(PropertyItem {
            id,
            kind: PropertyItemKind::VehicleAttachment,
            name: name.to_string(),
            location,
            quantity: 1,
            room: None,
            attached_to,
        });
    }

    fn build(self) -> Vec<PropertyItem> {
        self.items
    }
}

/// Builds the full real item list for a property: the real garage of named
/// vehicles and their attachments, the real categorized tool shed inventory,
/// real workshop/smithing equipment, and real house furnishings broken down
/// by room (bedrooms per name in `bedroom_owners`, bathroom, kitchen,
/// lounge) — see the module doc comment for the `gemini_universe` source
/// this is ported from.
fn build_items(bedroom_owners: &[&str]) -> Vec<PropertyItem> {
    let mut b = ItemListBuilder::new();

    // --- Garage: off-road/utility vehicles ---
    b.push(
        PropertyItemKind::Vehicle,
        "Polaris RZR 1000 Turbo",
        GARAGE,
        None,
    );
    b.push(
        PropertyItemKind::Vehicle,
        "Polaris RZR 2000 PRO R",
        GARAGE,
        None,
    );
    b.push(
        PropertyItemKind::Vehicle,
        "Ford Raptor 4x4 Ute",
        GARAGE,
        None,
    );
    b.push(PropertyItemKind::Vehicle, "Golf Cart", GARAGE, None);

    // --- Garage: agricultural tractors + their attachments ---
    let fendt_900 = b.push(PropertyItemKind::Vehicle, "Fendt 900 Vario", GARAGE, None);
    let fendt_1000 = b.push(PropertyItemKind::Vehicle, "Fendt 1000 Vario", GARAGE, None);
    b.push_attachment("Stoll Loader 900", GARAGE, Some(fendt_900));
    b.push_attachment("Stoll Loader 1000", GARAGE, Some(fendt_1000));
    // Plow/mower/forklift are shared implements usable by either tractor
    // (the source lists them in both tractors' inventories).
    b.push_attachment("Plow", GARAGE, None);
    b.push_attachment("Mower", GARAGE, None);
    b.push_attachment("Forklift Attachment", GARAGE, None);
    // Baler is only ever listed for the Fendt 1000.
    b.push_attachment("Baler", GARAGE, Some(fendt_1000));

    // --- Garage: heavy equipment ---
    let digger = b.push(
        PropertyItemKind::Vehicle,
        "30-Ton Excavator (CAT 330)",
        GARAGE,
        None,
    );

    // --- Garage: trailers — the ones the vehicles/digger load onto ---
    b.push(
        PropertyItemKind::Vehicle,
        "Double Axle Trailer",
        GARAGE,
        None,
    );
    b.push(
        PropertyItemKind::Vehicle,
        "Tractor Tipper Trailer",
        GARAGE,
        None,
    );
    let _ = digger; // the excavator is what the Double Axle Trailer is rated to carry

    // --- Tool Shed: real categorized inventory ---
    for tool in [
        "Hammer Set",
        "Screwdriver Set",
        "Power Drill",
        "Circular Saw",
        "Measuring Tape",
        "Level",
        "Wrench Set",
        "Pliers Set",
    ] {
        b.push(
            PropertyItemKind::ShedTool,
            tool,
            SHED,
            Some("Construction Tools"),
        );
    }
    for tool in [
        "Shovel",
        "Spade",
        "Pickaxe",
        "Post Hole Digger",
        "Rake",
        "Hoe",
    ] {
        b.push(
            PropertyItemKind::ShedTool,
            tool,
            SHED,
            Some("Digging Tools"),
        );
    }
    for tool in [
        "Welding Torch",
        "Soldering Iron",
        "Multimeter",
        "Wire Stripper",
        "Socket Set",
        "Torque Wrench",
    ] {
        b.push(PropertyItemKind::ShedTool, tool, SHED, Some("Repair Tools"));
    }
    for tool in [
        "Compass",
        "Binoculars",
        "Flashlight Set",
        "Rope (50ft)",
        "Climbing Gear",
        "GPS Device",
    ] {
        b.push(
            PropertyItemKind::ShedTool,
            tool,
            SHED,
            Some("Exploration Tools"),
        );
    }

    // --- Workshop: smithing/heavy fabrication (the tool shed's forge area) ---
    b.push(
        PropertyItemKind::BuildingEquipment,
        "Workbench",
        WORKSHOP,
        None,
    );
    b.push(
        PropertyItemKind::BuildingEquipment,
        "Furnace",
        WORKSHOP,
        None,
    );
    b.push(PropertyItemKind::BuildingEquipment, "Anvil", WORKSHOP, None);
    for tool in [
        "Blacksmith Hammer",
        "Tongs Set",
        "Chisel Set",
        "Grinder",
        "Forge Tools",
        "Quench Tank",
    ] {
        b.push(
            PropertyItemKind::BuildingEquipment,
            tool,
            WORKSHOP,
            Some("Smithing Tools"),
        );
    }

    // --- Armoury (supplementary — not in the source world, doesn't conflict with it) ---
    b.push(
        PropertyItemKind::ArmouryItem,
        "Secure Locker",
        ARMOURY,
        None,
    );
    b.push(
        PropertyItemKind::ArmouryItem,
        "Protective Equipment Set",
        ARMOURY,
        None,
    );
    b.push(
        PropertyItemKind::ArmouryItem,
        "Field Safety Kit",
        ARMOURY,
        None,
    );

    // --- Computer Room: real per-founder hardware ---
    b.push(
        PropertyItemKind::Computer,
        "Gem-D's Computer (gaming, 3 monitors)",
        COMPUTER_ROOM,
        None,
    );
    b.push(
        PropertyItemKind::Computer,
        "Gem-K's Computer (workstation, 2 monitors)",
        COMPUTER_ROOM,
        None,
    );
    b.push(
        PropertyItemKind::HouseholdItem,
        "Gem-D's Chair",
        COMPUTER_ROOM,
        None,
    );
    b.push(
        PropertyItemKind::HouseholdItem,
        "Gem-K's Chair",
        COMPUTER_ROOM,
        None,
    );
    // Supplementary networking hardware backing `NetworkAccount` — not in
    // the source (which just says "computers, peripherals, equipment"
    // generically) but doesn't contradict it.
    b.push(
        PropertyItemKind::Computer,
        "Local Archive Server",
        COMPUTER_ROOM,
        None,
    );
    b.push(
        PropertyItemKind::Computer,
        "Network Equipment Rack",
        COMPUTER_ROOM,
        None,
    );

    // --- House — general ---
    b.push(
        PropertyItemKind::HouseholdItem,
        "Household Storage Set",
        HOUSE,
        Some("General"),
    );
    b.push(
        PropertyItemKind::HouseholdItem,
        "Water Storage Tank",
        HOUSE,
        Some("General"),
    );
    b.push(
        PropertyItemKind::HouseholdItem,
        "Emergency Medical Kit",
        HOUSE,
        Some("General"),
    );

    // --- House — bathroom ---
    b.push(
        PropertyItemKind::HouseholdItem,
        "Bathtub (Jacuzzi)",
        HOUSE,
        Some("Bathroom"),
    );
    b.push(
        PropertyItemKind::HouseholdItem,
        "Shower",
        HOUSE,
        Some("Bathroom"),
    );
    b.push(
        PropertyItemKind::HouseholdItem,
        "Toilet",
        HOUSE,
        Some("Bathroom"),
    );
    b.push(
        PropertyItemKind::HouseholdItem,
        "Sink",
        HOUSE,
        Some("Bathroom"),
    );

    // --- House — kitchen ---
    b.push(
        PropertyItemKind::HouseholdItem,
        "Stove (gas, 4 burners, oven)",
        HOUSE,
        Some("Kitchen"),
    );
    b.push(
        PropertyItemKind::HouseholdItem,
        "Refrigerator (frost-free, ice maker)",
        HOUSE,
        Some("Kitchen"),
    );
    b.push(
        PropertyItemKind::HouseholdItem,
        "Kitchen Table (farmhouse, seats 6)",
        HOUSE,
        Some("Kitchen"),
    );
    b.push(
        PropertyItemKind::HouseholdItem,
        "Food Storage Pantry",
        HOUSE,
        Some("Kitchen"),
    );
    b.push(
        PropertyItemKind::HouseholdItem,
        "Cookware Set",
        HOUSE,
        Some("Kitchen"),
    );
    b.push(
        PropertyItemKind::HouseholdItem,
        "Water Purification System",
        HOUSE,
        Some("Kitchen"),
    );

    // --- House — lounge ---
    b.push(
        PropertyItemKind::HouseholdItem,
        "Large TV (75in 8K, soundbar)",
        HOUSE,
        Some("Lounge"),
    );
    b.push(
        PropertyItemKind::HouseholdItem,
        "Large Couch (leather, reclining)",
        HOUSE,
        Some("Lounge"),
    );
    b.push(
        PropertyItemKind::HouseholdItem,
        "Reading Library",
        HOUSE,
        Some("Lounge"),
    );
    b.push(
        PropertyItemKind::HouseholdItem,
        "Lighting Fixtures",
        HOUSE,
        Some("Lounge"),
    );

    // --- House — one real bedroom furniture set per named owner (or one
    // generic bedroom when the property has no owners yet, e.g. the
    // homestead template).
    let names: Vec<String> = if bedroom_owners.is_empty() {
        vec!["Bedroom".to_string()]
    } else {
        bedroom_owners
            .iter()
            .map(|n| format!("{}'s Bedroom", n))
            .collect()
    };
    for (i, room) in names.iter().enumerate() {
        let owner_label = bedroom_owners.get(i).copied().unwrap_or("");
        let bed_name = if owner_label.is_empty() {
            "King Size Bed".to_string()
        } else {
            format!("King Size Bed {}", if i == 0 { "A" } else { "B" })
        };
        b.push(
            PropertyItemKind::HouseholdItem,
            &bed_name,
            HOUSE,
            Some(room),
        );
        let desk_name = if owner_label.is_empty() {
            "Desk".to_string()
        } else {
            format!("{}'s Desk", owner_label)
        };
        b.push(
            PropertyItemKind::HouseholdItem,
            &desk_name,
            HOUSE,
            Some(room),
        );
        let tv_name = if owner_label.is_empty() {
            "TV (55in 4K smart)".to_string()
        } else {
            format!("{}'s TV (55in 4K smart)", owner_label)
        };
        b.push(PropertyItemKind::HouseholdItem, &tv_name, HOUSE, Some(room));
        let storage_name = if owner_label.is_empty() {
            "Wardrobe".to_string()
        } else {
            format!("{}'s Wardrobe", owner_label)
        };
        b.push(
            PropertyItemKind::HouseholdItem,
            &storage_name,
            HOUSE,
            Some(room),
        );
    }

    b.build()
}

impl StarterProperty {
    /// The canonical unowned household property present in a new world.
    pub fn maerken_homestead() -> Self {
        let buildings = vec![
            building(1, PropertyBuildingKind::House, "Homestead House"),
            building(2, PropertyBuildingKind::Shed, "Tool Shed"),
            building(3, PropertyBuildingKind::Workshop, "Building Workshop"),
            building(4, PropertyBuildingKind::Armoury, "Secure Armoury"),
            building(5, PropertyBuildingKind::ComputerRoom, "Computer Room"),
            building(6, PropertyBuildingKind::Garage, "Garage & Workshop"),
        ];

        Self {
            id: 1,
            name: "Maer'Ken Homestead".to_string(),
            owner: None,
            owner_agent_ids: Vec::new(),
            location: None,
            buildings,
            items: build_items(&[]),
            network_accounts: Vec::new(),
        }
    }

    /// Gem-D and Gem-K's founders' estate: the real building/vehicle/tool/
    /// furniture inventory ported from `gemini_universe`'s canonical island
    /// world (see the module doc comment), jointly owned and placed at
    /// `location` — a real grid cell on this world's own generated planet.
    /// Each founder gets their own named bedroom and their own network
    /// account.
    pub fn founders_estate(location: (usize, usize)) -> Self {
        let mut estate = Self::maerken_homestead();
        estate.id = 2;
        estate.name = "Gem-D & Gem-K's Estate".to_string();
        estate.owner_agent_ids = vec!["Gem-D".to_string(), "Gem-K".to_string()];
        estate.location = Some(location);
        estate.items = build_items(&["Gem-D", "Gem-K"]);
        estate.network_accounts = vec![
            NetworkAccount {
                id: 1,
                agent_id: "Gem-D".to_string(),
                username: "gem-d".to_string(),
                device: "Gem-D's Computer".to_string(),
                access_level: "Administrator".to_string(),
            },
            NetworkAccount {
                id: 2,
                agent_id: "Gem-K".to_string(),
                username: "gem-k".to_string(),
                device: "Gem-K's Computer".to_string(),
                access_level: "Administrator".to_string(),
            },
        ];
        estate
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertySystem {
    pub properties: Vec<StarterProperty>,
}

impl PropertySystem {
    /// `founders_location`: a real grid cell (row, col), when the world has
    /// one available (see `WorldState::new`'s biome-grid scan), to also
    /// seed Gem-D & Gem-K's owned, placed estate alongside the always-
    /// present unowned homestead template.
    pub fn new(founders_location: Option<(usize, usize)>) -> Self {
        let mut properties = vec![StarterProperty::maerken_homestead()];
        if let Some(location) = founders_location {
            properties.push(StarterProperty::founders_estate(location));
        }
        Self { properties }
    }

    /// The real placed location of the founders' estate, if this world has
    /// one (see [`StarterProperty::founders_estate`]). Identified by the
    /// founders' ownership rather than a hard-coded id, so it keeps working
    /// as the property list evolves. Used to seed Gem-D & Gem-K's runtime
    /// positions so they start the world at their home.
    pub fn founders_estate_location(&self) -> Option<(usize, usize)> {
        self.properties
            .iter()
            .find(|property| property.owner_agent_ids.iter().any(|id| id == "Gem-D"))
            .and_then(|property| property.location)
    }
}

impl PropertyBuildingKind {
    /// Protection from weather this building gives the people who shelter in
    /// it (0..1).
    pub fn shelter_quality(self) -> f64 {
        match self {
            PropertyBuildingKind::House => 0.95,
            PropertyBuildingKind::Workshop
            | PropertyBuildingKind::ComputerRoom
            | PropertyBuildingKind::Armoury => 0.7,
            PropertyBuildingKind::Garage => 0.6,
            PropertyBuildingKind::Shed => 0.5,
        }
    }
}

impl PropertySystem {
    /// Best shelter any placed property's buildings give at `(row, col)`
    /// (0 if no placed property stands there).
    pub fn shelter_at(&self, row: usize, col: usize) -> f64 {
        self.properties
            .iter()
            .filter(|property| property.location == Some((row, col)))
            .flat_map(|property| property.buildings.iter())
            .map(|building| building.kind.shelter_quality())
            .fold(0.0, f64::max)
    }
}

impl Default for PropertySystem {
    fn default() -> Self {
        Self::new(None)
    }
}

fn building(id: u64, kind: PropertyBuildingKind, name: &str) -> PropertyBuilding {
    PropertyBuilding {
        id,
        kind,
        name: name.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starter_property_contains_requested_property_categories() {
        let property = StarterProperty::maerken_homestead();

        assert_eq!(property.owner, None);
        assert!(property
            .buildings
            .iter()
            .any(|building| building.kind == PropertyBuildingKind::House));
        assert!(property
            .buildings
            .iter()
            .any(|building| building.kind == PropertyBuildingKind::Shed));
        assert!(property
            .buildings
            .iter()
            .any(|building| building.kind == PropertyBuildingKind::Garage));
        assert!(property
            .buildings
            .iter()
            .any(|building| building.kind == PropertyBuildingKind::Armoury));
        assert!(property
            .items
            .iter()
            .any(|item| item.kind == PropertyItemKind::Vehicle));
        assert!(property
            .items
            .iter()
            .any(|item| item.kind == PropertyItemKind::VehicleAttachment));
        assert!(property
            .items
            .iter()
            .any(|item| item.kind == PropertyItemKind::ShedTool));
        assert!(property
            .items
            .iter()
            .any(|item| item.kind == PropertyItemKind::BuildingEquipment));
        assert!(property
            .items
            .iter()
            .any(|item| item.kind == PropertyItemKind::Computer));
    }

    #[test]
    fn starter_property_is_deterministic() {
        assert_eq!(
            StarterProperty::maerken_homestead(),
            StarterProperty::maerken_homestead()
        );
    }

    #[test]
    fn item_ids_are_unique_within_a_property() {
        let property = StarterProperty::maerken_homestead();
        let mut ids: Vec<u64> = property.items.iter().map(|i| i.id).collect();
        let count_before = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), count_before, "item ids must all be unique");
    }

    #[test]
    fn house_items_cover_every_real_room() {
        let property = StarterProperty::maerken_homestead();
        let rooms_present = |room: &str| {
            property
                .items
                .iter()
                .any(|item| item.room.as_deref() == Some(room))
        };
        assert!(rooms_present("Kitchen"));
        assert!(rooms_present("Lounge"));
        assert!(rooms_present("Bathroom"));
        assert!(rooms_present("Bedroom"));
        assert!(rooms_present("General"));
    }

    #[test]
    fn tool_shed_covers_every_real_tool_category() {
        let property = StarterProperty::maerken_homestead();
        let category_present = |category: &str| {
            property
                .items
                .iter()
                .any(|item| item.room.as_deref() == Some(category))
        };
        assert!(category_present("Construction Tools"));
        assert!(category_present("Digging Tools"));
        assert!(category_present("Repair Tools"));
        assert!(category_present("Exploration Tools"));
        assert!(category_present("Smithing Tools"));
    }

    #[test]
    fn garage_contains_real_named_vehicles() {
        let property = StarterProperty::maerken_homestead();
        let has_vehicle = |name: &str| {
            property
                .items
                .iter()
                .any(|item| item.kind == PropertyItemKind::Vehicle && item.name == name)
        };
        assert!(has_vehicle("Polaris RZR 1000 Turbo"));
        assert!(has_vehicle("Polaris RZR 2000 PRO R"));
        assert!(has_vehicle("Ford Raptor 4x4 Ute"));
        assert!(has_vehicle("Golf Cart"));
        assert!(has_vehicle("Fendt 900 Vario"));
        assert!(has_vehicle("Fendt 1000 Vario"));
        assert!(has_vehicle("30-Ton Excavator (CAT 330)"));
        assert!(has_vehicle("Double Axle Trailer"));
        assert!(has_vehicle("Tractor Tipper Trailer"));
    }

    #[test]
    fn tractor_attachments_reference_their_real_tractor() {
        let property = StarterProperty::maerken_homestead();

        let fendt_900_id = property
            .items
            .iter()
            .find(|i| i.name == "Fendt 900 Vario")
            .unwrap()
            .id;
        let fendt_1000_id = property
            .items
            .iter()
            .find(|i| i.name == "Fendt 1000 Vario")
            .unwrap()
            .id;

        let loader_900 = property
            .items
            .iter()
            .find(|i| i.name == "Stoll Loader 900")
            .expect("Stoll Loader 900 should exist");
        assert_eq!(loader_900.attached_to, Some(fendt_900_id));

        let loader_1000 = property
            .items
            .iter()
            .find(|i| i.name == "Stoll Loader 1000")
            .expect("Stoll Loader 1000 should exist");
        assert_eq!(loader_1000.attached_to, Some(fendt_1000_id));

        let baler = property
            .items
            .iter()
            .find(|i| i.name == "Baler")
            .expect("Baler should exist");
        assert_eq!(baler.attached_to, Some(fendt_1000_id));

        let plow = property
            .items
            .iter()
            .find(|i| i.name == "Plow")
            .expect("Plow should exist");
        assert_eq!(
            plow.attached_to, None,
            "plow is a shared implement in the source world, not dedicated to one tractor"
        );
    }

    #[test]
    fn founders_estate_is_owned_and_located() {
        let estate = StarterProperty::founders_estate((7, 12));

        assert_eq!(estate.owner_agent_ids, vec!["Gem-D", "Gem-K"]);
        assert_eq!(estate.location, Some((7, 12)));
        assert!(estate
            .buildings
            .iter()
            .any(|building| building.kind == PropertyBuildingKind::House));
        assert!(estate
            .buildings
            .iter()
            .any(|building| building.kind == PropertyBuildingKind::Garage));
        assert!(estate
            .items
            .iter()
            .any(|item| item.kind == PropertyItemKind::Vehicle));
    }

    #[test]
    fn founders_estate_gives_each_founder_their_own_bedroom() {
        let estate = StarterProperty::founders_estate((7, 12));
        let has_bedroom = |owner: &str| {
            estate
                .items
                .iter()
                .any(|item| item.room.as_deref() == Some(&format!("{}'s Bedroom", owner)))
        };
        assert!(has_bedroom("Gem-D"));
        assert!(has_bedroom("Gem-K"));
    }

    #[test]
    fn founders_estate_gives_each_founder_a_network_account_with_no_stored_credential() {
        let estate = StarterProperty::founders_estate((7, 12));

        let gem_d = estate
            .network_accounts
            .iter()
            .find(|a| a.agent_id == "Gem-D")
            .expect("Gem-D should have a network account");
        let gem_k = estate
            .network_accounts
            .iter()
            .find(|a| a.agent_id == "Gem-K")
            .expect("Gem-K should have a network account");

        assert_ne!(gem_d.device, gem_k.device);
        assert_eq!(gem_d.access_level, "Administrator");
        assert_eq!(gem_k.access_level, "Administrator");
    }

    #[test]
    fn property_system_seeds_founders_estate_only_when_location_given() {
        let without_location = PropertySystem::new(None);
        assert_eq!(without_location.properties.len(), 1);
        assert!(without_location.properties[0].network_accounts.is_empty());

        let with_location = PropertySystem::new(Some((3, 4)));
        assert_eq!(with_location.properties.len(), 2);
        assert!(with_location
            .properties
            .iter()
            .any(|p| p.owner_agent_ids.contains(&"Gem-D".to_string())));
    }
}
