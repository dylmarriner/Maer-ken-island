//! Deterministic material economy for agent/world interaction.
//!
//! This module is the stateful bridge between world resources and human-made
//! artefacts.  It deliberately uses integer quantities and stable `Vec`
//! ordering so snapshots and replays produce the same result on every host.

use crate::agents::{ActionKind, Agent, CarryingState, GridPosition, ItemKind};
use crate::humans::HumanBeing;
use mk_core::time::Tick;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceNodeKind {
    Tree,
    StoneOutcrop,
    IronVein,
    CopperVein,
    CoalSeam,
    ClayDeposit,
    SandBank,
    FibrePatch,
    FoodPatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolKind {
    Hands,
    Axe,
    Pickaxe,
    Shovel,
    Hammer,
    Furnace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolInstance {
    pub kind: ToolKind,
    pub durability: u32,
}

impl ToolInstance {
    pub const fn hands() -> Self {
        Self {
            kind: ToolKind::Hands,
            durability: u32::MAX,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceNode {
    pub id: u64,
    pub position: GridPosition,
    pub kind: ResourceNodeKind,
    pub available: u32,
    pub capacity: u32,
    pub regeneration_per_tick: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecipeId {
    Planks,
    Masonry,
    Rope,
    Fuel,
    CopperIngot,
    IronIngot,
    StoneAxe,
    StonePickaxe,
    WoodenShelter,
    Workshop,
    Storage,
    /// Invented tier — gated by `recipe_unlocked`, unlike everything above
    /// (which is always available).
    IronAxe,
    IronPickaxe,
    Cart,
    StoneHouse,
}

/// The `accumulated_knowledge` (`humans::technology::TechnologySnapshot`)
/// a human needs before this recipe becomes craftable/buildable. Recipes
/// with no entry here (the baseline survival tier) are always unlocked.
/// This is the "humans invent things as they get smarter" gate.
pub fn recipe_unlock_threshold(id: RecipeId) -> f64 {
    match id {
        RecipeId::IronAxe | RecipeId::IronPickaxe => 5.0,
        RecipeId::Cart => 15.0,
        RecipeId::StoneHouse => 30.0,
        _ => 0.0,
    }
}

/// Whether a human/agent with `knowledge` accumulated invention progress
/// has unlocked `id`.
pub fn recipe_unlocked(id: RecipeId, knowledge: f64) -> bool {
    knowledge >= recipe_unlock_threshold(id)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Recipe {
    pub id: RecipeId,
    pub input: &'static [(ItemKind, u32)],
    pub output: (ItemKind, u32),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConstructedStructure {
    pub id: u64,
    pub position: GridPosition,
    pub recipe: RecipeId,
    pub material_cost: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EconomyEventKind {
    Gathered,
    Processed,
    Crafted,
    Constructed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EconomyEvent {
    pub tick: Tick,
    pub agent_id: String,
    pub kind: EconomyEventKind,
    pub subject: String,
    pub quantity: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceEconomyState {
    pub nodes: Vec<ResourceNode>,
    pub structures: Vec<ConstructedStructure>,
    pub events: Vec<EconomyEvent>,
    pub next_structure_id: u64,
}

impl Default for ResourceEconomyState {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceEconomyState {
    /// An economy with no resource nodes. A live world seeds its nodes from
    /// the biome map with [`Self::from_biomes`]; interventions and scenarios
    /// add nodes on top of that.
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            structures: Vec::new(),
            events: Vec::new(),
            next_structure_id: 1,
        }
    }

    /// Seed one node per material kind on every land cell, from what that
    /// cell's biome actually yields (`mk_core::biomes::biome_resources`).
    /// Nodes are ordered by cell, then by the biome's resource order.
    ///
    /// A node's capacity is the biome's base yield summed over every
    /// resource that maps onto the node kind, times
    /// [`STANDING_STOCK_PER_YIELD`]. Living stocks (trees, fibre plants,
    /// food) regrow; clay and sand renew only where water deposits them
    /// (rivers, wetlands); rock, ore and coal never regrow. Aquatic cells get
    /// no nodes: the material economy is worked on foot.
    pub fn from_biomes(biomes: &mk_core::grid::Grid2<mk_core::biomes::BiomeType>) -> Self {
        let spec = mk_core::grid::GridSpec::new(biomes.nlat(), biomes.nlon());
        Self::from_terrain(biomes, &mk_core::grid::Grid2::new(&spec, 0.0))
    }

    /// [`Self::from_biomes`], plus the volcanic ground that covers only part
    /// of a cell. `volcanic_area_fraction` is each cell's share of arc, rift
    /// or hotspot terrain (`volcanism::VolcanicCell::volcanic_area_fraction`).
    /// A land cell that is not itself `Volcanic` also yields the volcanic
    /// biome's rock and ore, scaled by that share, so a continental cell
    /// crossed by a volcanic arc still carries the arc's ore.
    pub fn from_terrain(
        biomes: &mk_core::grid::Grid2<mk_core::biomes::BiomeType>,
        volcanic_area_fraction: &mk_core::grid::Grid2<f64>,
    ) -> Self {
        let mut economy = Self::new();
        let mut next_id = 1u64;
        for (row, col, biome) in biomes.indexed_iter() {
            if biome.is_aquatic() {
                continue;
            }
            let mut yields: Vec<(ResourceNodeKind, f64)> = Vec::new();
            for resource in mk_core::biomes::biome_resources(*biome) {
                // Coal forms from buried wetland peat: a wetland's plant
                // fibre also stands as a peat seam.
                if *biome == mk_core::biomes::BiomeType::Wetland
                    && resource.kind == mk_core::biomes::ResourceKind::Fiber
                {
                    add_yield(&mut yields, ResourceNodeKind::CoalSeam, resource.base_yield);
                }
                if let Some(kind) = node_kind_for_resource(resource.kind) {
                    add_yield(&mut yields, kind, resource.base_yield);
                }
            }
            let volcanic_share = volcanic_area_fraction
                .get_safe(row, col)
                .copied()
                .unwrap_or(0.0)
                .clamp(0.0, 1.0);
            if *biome != mk_core::biomes::BiomeType::Volcanic && volcanic_share > 0.0 {
                for resource in
                    mk_core::biomes::biome_resources(mk_core::biomes::BiomeType::Volcanic)
                {
                    if let Some(kind) = node_kind_for_resource(resource.kind) {
                        add_yield(&mut yields, kind, resource.base_yield * volcanic_share);
                    }
                }
            }
            for (kind, total_yield) in yields {
                let capacity = (total_yield * STANDING_STOCK_PER_YIELD).round() as u32;
                if capacity == 0 {
                    continue;
                }
                economy.nodes.push(ResourceNode {
                    id: next_id,
                    position: GridPosition::new(row as i32, col as i32),
                    kind,
                    available: capacity,
                    capacity,
                    regeneration_per_tick: regeneration_for(kind, *biome, total_yield),
                });
                next_id += 1;
            }
        }
        economy
    }

    /// Regenerate renewable resources once per world tick.
    pub fn regenerate(&mut self) {
        for node in &mut self.nodes {
            node.available = node
                .available
                .saturating_add(node.regeneration_per_tick)
                .min(node.capacity);
        }
    }

    /// Resolve the agent's selected action against the node at its position.
    /// This is called after decision-making and before the world hash commit.
    pub fn apply_agent_action(
        &mut self,
        agent: &mut Agent,
        tick: Tick,
        elevation_grid: &mk_core::grid::Grid2<f64>,
    ) {
        // Non-human agents carry no `technology` state, so they work with the
        // baseline (always-unlocked) recipes; invention is a human capacity
        // (`apply_human_action` passes the human's accumulated knowledge).
        self.apply_action_parts(
            &agent.id.to_string(),
            agent.position,
            agent.last_action.kind,
            &mut agent.carrying,
            tick,
            elevation_grid,
            0.0,
        );
    }

    /// Resolve a schema-backed human through the same material rules as an agent.
    pub fn apply_human_action(
        &mut self,
        human: &mut HumanBeing,
        tick: Tick,
        elevation_grid: &mk_core::grid::Grid2<f64>,
    ) -> bool {
        let agent_id = human.agent_id().to_string();
        let knowledge = human.technology.accumulated_knowledge;
        self.apply_action_parts(
            &agent_id,
            human.position,
            human.economy_action.kind,
            &mut human.carrying,
            tick,
            elevation_grid,
            knowledge,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn apply_action_parts(
        &mut self,
        agent_id: &str,
        position: GridPosition,
        action: ActionKind,
        carrying: &mut CarryingState,
        tick: Tick,
        elevation_grid: &mk_core::grid::Grid2<f64>,
        knowledge: f64,
    ) -> bool {
        match action {
            ActionKind::Gather | ActionKind::Mine => {
                self.gather_parts(agent_id, position, action, carrying, tick)
            }
            ActionKind::Craft => self.craft_parts(agent_id, carrying, tick, knowledge),
            ActionKind::Build => self.build_parts(
                agent_id,
                position,
                carrying,
                tick,
                elevation_grid,
                knowledge,
            ),
            _ => true,
        }
    }

    pub fn gather(&mut self, agent: &mut Agent, tick: Tick) {
        self.gather_parts(
            &agent.id.to_string(),
            agent.position,
            agent.last_action.kind,
            &mut agent.carrying,
            tick,
        );
    }

    fn gather_parts(
        &mut self,
        agent_id: &str,
        position: GridPosition,
        action: ActionKind,
        carrying: &mut CarryingState,
        tick: Tick,
    ) -> bool {
        let Some(index) = self.node_for_action(position, action) else {
            self.fail(agent_id, tick, "no resource node");
            return false;
        };
        let node_kind = self.nodes[index].kind;
        let required = required_tool(node_kind);
        let has_required_tool = carrying.has_tool(required);
        if action == ActionKind::Mine && !has_required_tool {
            self.fail(agent_id, tick, "mining requires a pickaxe");
            return false;
        }
        let requested = if has_required_tool { 5 } else { 1 };
        let quantity = self.nodes[index].available.min(requested);
        if quantity == 0 {
            self.fail(agent_id, tick, "resource depleted");
            return false;
        }
        let item = item_for_node(node_kind);
        let accepted = carrying.add(item, quantity);
        self.nodes[index].available -= accepted;
        if accepted > 0 {
            carrying.use_tool(required, accepted);
            self.record(
                tick,
                agent_id,
                EconomyEventKind::Gathered,
                format!("{:?}", node_kind),
                accepted,
            );
            true
        } else {
            self.fail(agent_id, tick, "no physical carrying capacity");
            false
        }
    }

    /// The node at `position` this action works. Mining works rock and ore
    /// (the first of those in the cell with stock left); gathering prefers
    /// food, then fibre, wood, clay and sand. A depleted node is chosen only
    /// when nothing else of the right kind is left, so the failure names it.
    fn node_for_action(&self, position: GridPosition, action: ActionKind) -> Option<usize> {
        let preference: &[ResourceNodeKind] = if action == ActionKind::Mine {
            &[
                ResourceNodeKind::IronVein,
                ResourceNodeKind::CopperVein,
                ResourceNodeKind::CoalSeam,
                ResourceNodeKind::StoneOutcrop,
            ]
        } else {
            &[
                ResourceNodeKind::FoodPatch,
                ResourceNodeKind::FibrePatch,
                ResourceNodeKind::Tree,
                ResourceNodeKind::ClayDeposit,
                ResourceNodeKind::SandBank,
                ResourceNodeKind::StoneOutcrop,
                ResourceNodeKind::CoalSeam,
            ]
        };
        let rank = |kind: ResourceNodeKind| preference.iter().position(|k| *k == kind);
        let here = || {
            self.nodes
                .iter()
                .enumerate()
                .filter(move |(_, n)| n.position == position)
                .filter_map(move |(i, n)| rank(n.kind).map(|r| (r, i, n.available)))
        };
        here()
            .filter(|(_, _, available)| *available > 0)
            .min_by_key(|(r, _, _)| *r)
            .or_else(|| here().min_by_key(|(r, _, _)| *r))
            .map(|(_, i, _)| i)
    }

    pub fn craft(&mut self, agent: &mut Agent, tick: Tick) {
        self.craft_parts(&agent.id.to_string(), &mut agent.carrying, tick, 0.0);
    }

    fn craft_parts(
        &mut self,
        agent_id: &str,
        carrying: &mut CarryingState,
        tick: Tick,
        knowledge: f64,
    ) -> bool {
        // Processing recipes are tried in stable order before tool recipes;
        // invented (knowledge-gated) recipes are appended last, after the
        // baseline tier, and filtered to only those this crafter has
        // actually unlocked.
        // Invented tool recipes are tried before their stone-age baseline
        // equivalents, so a crafter who has unlocked and can afford the
        // better iron tool prefers it over falling back to the stone one.
        let recipes = [
            recipe(RecipeId::Planks),
            recipe(RecipeId::Masonry),
            recipe(RecipeId::Rope),
            recipe(RecipeId::Fuel),
            recipe(RecipeId::IronIngot),
            recipe(RecipeId::CopperIngot),
            recipe(RecipeId::IronAxe),
            recipe(RecipeId::IronPickaxe),
            recipe(RecipeId::StoneAxe),
            recipe(RecipeId::StonePickaxe),
        ];
        for current in recipes {
            if !recipe_unlocked(current.id, knowledge) {
                continue;
            }
            if has_inputs(carrying, current.input) {
                consume_inputs(carrying, current.input);
                carrying.add(current.output.0, current.output.1);
                if let Some(tool) = tool_from_recipe(current.id) {
                    carrying.add_tool(tool);
                }
                self.record(
                    tick,
                    agent_id,
                    EconomyEventKind::Crafted,
                    format!("{:?}", current.id),
                    current.output.1,
                );
                return true;
            }
        }
        self.fail(agent_id, tick, "no craftable recipe");
        false
    }

    pub fn build(
        &mut self,
        agent: &mut Agent,
        tick: Tick,
        elevation_grid: &mk_core::grid::Grid2<f64>,
    ) {
        self.build_parts(
            &agent.id.to_string(),
            agent.position,
            &mut agent.carrying,
            tick,
            elevation_grid,
            0.0,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn build_parts(
        &mut self,
        agent_id: &str,
        position: GridPosition,
        carrying: &mut CarryingState,
        tick: Tick,
        elevation_grid: &mk_core::grid::Grid2<f64>,
        knowledge: f64,
    ) -> bool {
        if !crate::physics::is_buildable(elevation_grid, position.row, position.col) {
            self.fail(agent_id, tick, "terrain too steep to build on");
            return false;
        }
        // Invented (knowledge-gated) recipes are tried before the baseline
        // tier: `Storage`'s requirement (Planks:6) is a strict subset of
        // `StoneHouse`'s (Masonry:16, Planks:6, IronIngot:2), so checking
        // `Storage` first would always shadow `StoneHouse` for a builder who
        // could afford either — nobody would ever actually build the
        // invented structure.
        let options = [
            RecipeId::StoneHouse,
            RecipeId::Cart,
            RecipeId::WoodenShelter,
            RecipeId::Workshop,
            RecipeId::Storage,
        ];
        for recipe_id in options {
            if !recipe_unlocked(recipe_id, knowledge) {
                continue;
            }
            let current = recipe(recipe_id);
            if has_inputs(carrying, current.input) {
                consume_inputs(carrying, current.input);
                self.structures.push(ConstructedStructure {
                    id: self.next_structure_id,
                    position,
                    recipe: recipe_id,
                    material_cost: current.output.1,
                });
                self.next_structure_id = self.next_structure_id.saturating_add(1);
                self.record(
                    tick,
                    agent_id,
                    EconomyEventKind::Constructed,
                    format!("{:?}", recipe_id),
                    current.output.1,
                );
                return true;
            }
        }
        self.fail(agent_id, tick, "insufficient building materials");
        false
    }

    /// Construct `recipe_id` at `position` on an operator's behalf.
    ///
    /// Unlike [`Self::build`] no agent inventory is consumed — the operator is
    /// outside the simulated economy — but the same physical rule applies:
    /// the cell must be buildable terrain. Only structure recipes are
    /// accepted. The construction is recorded in the event log under
    /// `actor_id`, so it is as visible as any agent-built structure. Returns
    /// the new structure's id.
    pub fn construct_for_operator(
        &mut self,
        actor_id: &str,
        recipe_id: RecipeId,
        position: GridPosition,
        tick: Tick,
        elevation_grid: &mk_core::grid::Grid2<f64>,
    ) -> Result<u64, &'static str> {
        let current = recipe(recipe_id);
        if current.output.0 != ItemKind::Structure {
            return Err("recipe does not produce a structure");
        }
        if !crate::physics::is_buildable(elevation_grid, position.row, position.col) {
            self.fail(actor_id, tick, "terrain too steep to build on");
            return Err("terrain too steep to build on");
        }
        let id = self.next_structure_id;
        self.structures.push(ConstructedStructure {
            id,
            position,
            recipe: recipe_id,
            material_cost: current.input.iter().map(|(_, qty)| qty).sum(),
        });
        self.next_structure_id = self.next_structure_id.saturating_add(1);
        self.record(
            tick,
            actor_id,
            EconomyEventKind::Constructed,
            format!("{:?}", recipe_id),
            current.output.1,
        );
        Ok(id)
    }

    fn record(
        &mut self,
        tick: Tick,
        agent_id: &str,
        kind: EconomyEventKind,
        subject: String,
        quantity: u32,
    ) {
        self.events.push(EconomyEvent {
            tick,
            agent_id: agent_id.to_string(),
            kind,
            subject,
            quantity,
        });
    }

    fn fail(&mut self, agent_id: &str, tick: Tick, reason: &str) {
        self.record(
            tick,
            agent_id,
            EconomyEventKind::Failed,
            reason.to_string(),
            0,
        );
    }
}

fn add_yield(yields: &mut Vec<(ResourceNodeKind, f64)>, kind: ResourceNodeKind, amount: f64) {
    match yields.iter_mut().find(|(k, _)| *k == kind) {
        Some((_, total)) => *total += amount,
        None => yields.push((kind, amount)),
    }
}

/// Protection from weather a constructed structure gives the people in its
/// cell (0..1), or `None` for structures nobody shelters in.
pub fn structure_shelter_quality(recipe: RecipeId) -> Option<f64> {
    match recipe {
        RecipeId::StoneHouse => Some(0.95),
        RecipeId::WoodenShelter => Some(0.8),
        RecipeId::Workshop => Some(0.6),
        RecipeId::Storage => Some(0.4),
        _ => None,
    }
}

impl ResourceEconomyState {
    /// Best shelter any constructed structure at `position` gives (0 if none).
    pub fn built_shelter_at(&self, position: GridPosition) -> f64 {
        self.structures
            .iter()
            .filter(|structure| structure.position == position)
            .filter_map(|structure| structure_shelter_quality(structure.recipe))
            .fold(0.0, f64::max)
    }
}

/// Standing stock of a node per unit of biome base yield.
pub const STANDING_STOCK_PER_YIELD: f64 = 20.0;

/// Which economy node a biome resource feeds. Resources with no material
/// item in this economy (herbs, hide, gems, salt, fish, …) return `None`.
fn node_kind_for_resource(resource: mk_core::biomes::ResourceKind) -> Option<ResourceNodeKind> {
    use mk_core::biomes::ResourceKind as R;
    match resource {
        R::Wood | R::Resin => Some(ResourceNodeKind::Tree),
        R::Stone => Some(ResourceNodeKind::StoneOutcrop),
        R::Minerals => Some(ResourceNodeKind::IronVein),
        R::MetalOre => Some(ResourceNodeKind::CopperVein),
        R::Clay => Some(ResourceNodeKind::ClayDeposit),
        R::Sand => Some(ResourceNodeKind::SandBank),
        R::Fiber => Some(ResourceNodeKind::FibrePatch),
        R::Food | R::Fruit | R::Nuts | R::Fungi | R::Meat | R::Honey => {
            Some(ResourceNodeKind::FoodPatch)
        }
        _ => None,
    }
}

/// Per-tick regrowth: a tenth of the base yield (at least 1) for living
/// stocks and for clay/sand where water lays it down; zero for rock, ore
/// and coal.
fn regeneration_for(
    kind: ResourceNodeKind,
    biome: mk_core::biomes::BiomeType,
    total_yield: f64,
) -> u32 {
    use mk_core::biomes::BiomeType;
    let renews = match kind {
        ResourceNodeKind::Tree | ResourceNodeKind::FibrePatch | ResourceNodeKind::FoodPatch => true,
        ResourceNodeKind::ClayDeposit | ResourceNodeKind::SandBank => {
            matches!(biome, BiomeType::River | BiomeType::Wetland)
        }
        ResourceNodeKind::StoneOutcrop
        | ResourceNodeKind::IronVein
        | ResourceNodeKind::CopperVein
        | ResourceNodeKind::CoalSeam => false,
    };
    if renews {
        ((total_yield / 10.0).round() as u32).max(1)
    } else {
        0
    }
}

fn required_tool(kind: ResourceNodeKind) -> ToolKind {
    match kind {
        ResourceNodeKind::Tree => ToolKind::Axe,
        ResourceNodeKind::IronVein
        | ResourceNodeKind::CopperVein
        | ResourceNodeKind::CoalSeam
        | ResourceNodeKind::StoneOutcrop => ToolKind::Pickaxe,
        _ => ToolKind::Hands,
    }
}

fn item_for_node(kind: ResourceNodeKind) -> ItemKind {
    match kind {
        ResourceNodeKind::Tree => ItemKind::Wood,
        ResourceNodeKind::StoneOutcrop => ItemKind::Stone,
        ResourceNodeKind::IronVein => ItemKind::IronOre,
        ResourceNodeKind::CopperVein => ItemKind::CopperOre,
        ResourceNodeKind::CoalSeam => ItemKind::Coal,
        ResourceNodeKind::ClayDeposit => ItemKind::Clay,
        ResourceNodeKind::SandBank => ItemKind::Sand,
        ResourceNodeKind::FibrePatch => ItemKind::Fiber,
        ResourceNodeKind::FoodPatch => ItemKind::Food,
    }
}

fn recipe(id: RecipeId) -> Recipe {
    const WOOD: &[(ItemKind, u32)] = &[(ItemKind::Wood, 3)];
    const STONE: &[(ItemKind, u32)] = &[(ItemKind::Stone, 3)];
    const FIBER: &[(ItemKind, u32)] = &[(ItemKind::Fiber, 2)];
    const COAL: &[(ItemKind, u32)] = &[(ItemKind::Coal, 2)];
    const IRON: &[(ItemKind, u32)] = &[(ItemKind::IronOre, 3), (ItemKind::Fuel, 1)];
    const COPPER: &[(ItemKind, u32)] = &[(ItemKind::CopperOre, 2), (ItemKind::Fuel, 1)];
    match id {
        RecipeId::Planks => Recipe {
            id,
            input: WOOD,
            output: (ItemKind::Planks, 2),
        },
        RecipeId::Masonry => Recipe {
            id,
            input: STONE,
            output: (ItemKind::Masonry, 2),
        },
        RecipeId::Rope => Recipe {
            id,
            input: FIBER,
            output: (ItemKind::Rope, 1),
        },
        RecipeId::Fuel => Recipe {
            id,
            input: COAL,
            output: (ItemKind::Fuel, 2),
        },
        RecipeId::IronIngot => Recipe {
            id,
            input: IRON,
            output: (ItemKind::IronIngot, 1),
        },
        RecipeId::CopperIngot => Recipe {
            id,
            input: COPPER,
            output: (ItemKind::CopperIngot, 1),
        },
        RecipeId::StoneAxe => Recipe {
            id,
            input: &[(ItemKind::Planks, 1), (ItemKind::Masonry, 1)],
            output: (ItemKind::Tool, 1),
        },
        RecipeId::StonePickaxe => Recipe {
            id,
            input: &[(ItemKind::Planks, 1), (ItemKind::Masonry, 2)],
            output: (ItemKind::Tool, 1),
        },
        RecipeId::WoodenShelter => Recipe {
            id,
            input: &[(ItemKind::Planks, 8), (ItemKind::Rope, 2)],
            output: (ItemKind::Structure, 1),
        },
        RecipeId::Workshop => Recipe {
            id,
            input: &[
                (ItemKind::Planks, 12),
                (ItemKind::Masonry, 8),
                (ItemKind::IronIngot, 4),
            ],
            output: (ItemKind::Structure, 1),
        },
        RecipeId::Storage => Recipe {
            id,
            input: &[(ItemKind::Planks, 6)],
            output: (ItemKind::Structure, 1),
        },
        RecipeId::IronAxe => Recipe {
            id,
            input: &[(ItemKind::Planks, 1), (ItemKind::IronIngot, 1)],
            output: (ItemKind::Tool, 1),
        },
        RecipeId::IronPickaxe => Recipe {
            id,
            input: &[(ItemKind::Planks, 1), (ItemKind::IronIngot, 2)],
            output: (ItemKind::Tool, 1),
        },
        RecipeId::Cart => Recipe {
            id,
            input: &[(ItemKind::Planks, 10), (ItemKind::IronIngot, 2)],
            output: (ItemKind::Structure, 1),
        },
        RecipeId::StoneHouse => Recipe {
            id,
            input: &[
                (ItemKind::Masonry, 16),
                (ItemKind::Planks, 6),
                (ItemKind::IronIngot, 2),
            ],
            output: (ItemKind::Structure, 1),
        },
    }
}

fn tool_from_recipe(id: RecipeId) -> Option<ToolKind> {
    match id {
        RecipeId::StoneAxe | RecipeId::IronAxe => Some(ToolKind::Axe),
        RecipeId::StonePickaxe | RecipeId::IronPickaxe => Some(ToolKind::Pickaxe),
        _ => None,
    }
}

fn has_inputs(carrying: &CarryingState, inputs: &[(ItemKind, u32)]) -> bool {
    inputs
        .iter()
        .all(|(kind, quantity)| carrying.count(*kind) >= *quantity)
}

fn consume_inputs(carrying: &mut CarryingState, inputs: &[(ItemKind, u32)]) {
    for (kind, quantity) in inputs {
        carrying.remove(*kind, *quantity);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::{ActionKind, Agent, AgentId};

    /// A test field with one node of each kind along the diagonal, at
    /// fixed positions the tests below walk agents onto.
    fn test_field() -> ResourceEconomyState {
        let specs = [
            (1, 2, 2, ResourceNodeKind::Tree, 100, 1),
            (2, 4, 4, ResourceNodeKind::StoneOutcrop, 160, 1),
            (3, 6, 6, ResourceNodeKind::IronVein, 120, 0),
            (4, 8, 8, ResourceNodeKind::CopperVein, 120, 0),
            (5, 10, 10, ResourceNodeKind::CoalSeam, 100, 0),
            (6, 12, 12, ResourceNodeKind::ClayDeposit, 180, 2),
            (7, 14, 14, ResourceNodeKind::SandBank, 180, 2),
            (8, 16, 16, ResourceNodeKind::FibrePatch, 100, 2),
            (9, 18, 18, ResourceNodeKind::FoodPatch, 100, 3),
        ];
        let mut economy = ResourceEconomyState::new();
        economy.nodes = specs
            .into_iter()
            .map(|(id, row, col, kind, capacity, regen)| ResourceNode {
                id,
                position: GridPosition::new(row, col),
                kind,
                available: capacity,
                capacity,
                regeneration_per_tick: regen,
            })
            .collect();
        economy
    }

    fn agent_at(row: i32, col: i32) -> Agent {
        Agent::spawn_at(AgentId::new("builder"), row, col)
    }

    /// Flat (zero-slope) elevation grid, large enough for these tests'
    /// agent positions, so terrain gating never blocks them.
    fn flat_elevation_grid() -> mk_core::grid::Grid2<f64> {
        mk_core::grid::Grid2::new(&mk_core::grid::GridSpec::new(16, 16), 0.0)
    }

    #[test]
    fn gather_process_craft_and_build_is_end_to_end() {
        let mut economy = test_field();
        let elevation = flat_elevation_grid();
        let mut agent = agent_at(2, 2);
        agent.carrying.add_container("work basket", 64);
        agent.last_action = crate::agents::AgentAction {
            kind: ActionKind::Gather,
            intensity: 1.0,
        };
        for tick in 0..12 {
            economy.apply_agent_action(&mut agent, tick, &elevation);
        }
        assert!(agent.carrying.count(ItemKind::Wood) > 0);
        agent.last_action.kind = ActionKind::Craft;
        for tick in 12..16 {
            economy.apply_agent_action(&mut agent, tick, &elevation);
        }
        assert!(agent.carrying.count(ItemKind::Planks) >= 8);
        agent.carrying.add(ItemKind::Rope, 2);
        agent.last_action.kind = ActionKind::Build;
        economy.apply_agent_action(&mut agent, 13, &elevation);
        assert_eq!(economy.structures.len(), 1);
        assert!(economy
            .events
            .iter()
            .any(|event| event.kind == EconomyEventKind::Constructed));
    }

    #[test]
    fn biome_seeding_follows_what_each_biome_yields() {
        use mk_core::biomes::BiomeType;
        let spec = mk_core::grid::GridSpec::new(2, 2);
        let mut biomes = mk_core::grid::Grid2::new(&spec, BiomeType::DeepOcean);
        *biomes.get_mut(0, 0) = BiomeType::TemperateForest;
        *biomes.get_mut(0, 1) = BiomeType::Alpine;
        *biomes.get_mut(1, 0) = BiomeType::Wetland;
        let economy = ResourceEconomyState::from_biomes(&biomes);

        let at = |row, col| {
            economy
                .nodes
                .iter()
                .filter(move |n| n.position == GridPosition::new(row, col))
        };
        // Ocean cells carry no nodes.
        assert_eq!(at(1, 1).count(), 0);
        // Forest: wood (7) + resin (3) = 10 yield → 200 standing stock.
        let tree = at(0, 0)
            .find(|n| n.kind == ResourceNodeKind::Tree)
            .expect("forest has trees");
        assert_eq!(tree.capacity, 200);
        assert_eq!(tree.regeneration_per_tick, 1);
        assert!(at(0, 0).any(|n| n.kind == ResourceNodeKind::FoodPatch));
        assert!(!at(0, 0).any(|n| n.kind == ResourceNodeKind::IronVein));
        // Alpine rock and ore never regrow.
        let ore: Vec<_> = at(0, 1)
            .filter(|n| {
                matches!(
                    n.kind,
                    ResourceNodeKind::StoneOutcrop
                        | ResourceNodeKind::IronVein
                        | ResourceNodeKind::CopperVein
                )
            })
            .collect();
        assert_eq!(ore.len(), 3);
        assert!(ore.iter().all(|n| n.regeneration_per_tick == 0));
        // Wetland: renewing clay, fibre, and a finite peat seam.
        let clay = at(1, 0)
            .find(|n| n.kind == ResourceNodeKind::ClayDeposit)
            .unwrap();
        assert!(clay.regeneration_per_tick > 0);
        let peat = at(1, 0)
            .find(|n| n.kind == ResourceNodeKind::CoalSeam)
            .unwrap();
        assert_eq!(peat.regeneration_per_tick, 0);
        assert!(at(1, 0).any(|n| n.kind == ResourceNodeKind::FibrePatch));
        // Ids are unique.
        let mut ids: Vec<u64> = economy.nodes.iter().map(|n| n.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), economy.nodes.len());
    }

    #[test]
    fn a_volcanic_arc_through_a_cell_adds_its_ore_in_proportion() {
        use mk_core::biomes::BiomeType;
        let spec = mk_core::grid::GridSpec::new(1, 2);
        let biomes = mk_core::grid::Grid2::new(&spec, BiomeType::Grassland);
        let mut share = mk_core::grid::Grid2::new(&spec, 0.0);
        *share.get_mut(0, 1) = 0.1;
        let economy = ResourceEconomyState::from_terrain(&biomes, &share);
        let ore_at = |col| {
            economy
                .nodes
                .iter()
                .filter(|n| {
                    n.position == GridPosition::new(0, col)
                        && n.kind == ResourceNodeKind::CopperVein
                })
                .map(|n| n.capacity)
                .sum::<u32>()
        };
        // Grassland alone has no metal ore; 10% arc terrain brings a tenth of
        // the volcanic biome's (6 × 20 × 0.1 = 12).
        assert_eq!(ore_at(0), 0);
        assert_eq!(ore_at(1), 12);
        let stone_at_arc = economy
            .nodes
            .iter()
            .find(|n| {
                n.position == GridPosition::new(0, 1) && n.kind == ResourceNodeKind::StoneOutcrop
            })
            .expect("arc rock");
        assert_eq!(stone_at_arc.regeneration_per_tick, 0);
    }

    #[test]
    fn gathering_and_mining_work_different_nodes_in_one_cell() {
        use mk_core::biomes::BiomeType;
        let spec = mk_core::grid::GridSpec::new(1, 1);
        let biomes = mk_core::grid::Grid2::new(&spec, BiomeType::MontaneForest);
        let mut economy = ResourceEconomyState::from_biomes(&biomes);
        let elevation = mk_core::grid::Grid2::new(&spec, 0.0);
        let mut agent = agent_at(0, 0);
        agent.carrying.add_container("sack", 64);
        agent.carrying.add_tool(ToolKind::Pickaxe);

        agent.last_action = crate::agents::AgentAction {
            kind: ActionKind::Gather,
            intensity: 1.0,
        };
        economy.apply_agent_action(&mut agent, 0, &elevation);
        assert!(agent.carrying.count(ItemKind::Food) > 0);

        agent.last_action.kind = ActionKind::Mine;
        economy.apply_agent_action(&mut agent, 1, &elevation);
        assert!(agent.carrying.count(ItemKind::IronOre) > 0);
    }

    #[test]
    fn depleted_nodes_regenerate_only_when_configured() {
        let mut economy = test_field();
        economy.nodes[0].available = 0;
        economy.nodes[2].available = 0;
        economy.regenerate();
        assert_eq!(economy.nodes[0].available, 1);
        assert_eq!(economy.nodes[2].available, 0);
    }

    #[test]
    fn mining_requires_a_pickaxe_and_preserves_ore_identity() {
        let mut economy = test_field();
        let elevation = flat_elevation_grid();
        let mut agent = agent_at(6, 6);
        agent.carrying.add_container("ore sack", 32);
        agent.last_action = crate::agents::AgentAction {
            kind: ActionKind::Mine,
            intensity: 1.0,
        };
        economy.apply_agent_action(&mut agent, 0, &elevation);
        assert_eq!(agent.carrying.count(ItemKind::IronOre), 0);
        agent.carrying.add_tool(ToolKind::Pickaxe);
        economy.apply_agent_action(&mut agent, 1, &elevation);
        assert!(agent.carrying.count(ItemKind::IronOre) > 0);
        assert_eq!(agent.carrying.count(ItemKind::CopperOre), 0);
    }

    #[test]
    fn build_fails_on_terrain_too_steep() {
        let mut economy = test_field();
        let mut elevation = flat_elevation_grid();
        *elevation.get_mut(2, 2) = crate::physics::MAX_CLIMB_HEIGHT_M * 10.0;

        let mut agent = agent_at(2, 2);
        agent.carrying.add_container("work basket", 64);
        agent.carrying.add(ItemKind::Planks, 8);
        agent.carrying.add(ItemKind::Rope, 2);
        agent.last_action = crate::agents::AgentAction {
            kind: ActionKind::Build,
            intensity: 1.0,
        };
        economy.apply_agent_action(&mut agent, 0, &elevation);

        assert_eq!(economy.structures.len(), 0);
        assert!(economy.events.iter().any(|event| {
            event.kind == EconomyEventKind::Failed && event.subject.contains("steep")
        }));
    }

    #[test]
    fn baseline_recipes_are_always_unlocked_but_invented_ones_are_not() {
        assert!(recipe_unlocked(RecipeId::Planks, 0.0));
        assert!(recipe_unlocked(RecipeId::WoodenShelter, 0.0));
        assert!(!recipe_unlocked(RecipeId::IronAxe, 0.0));
        assert!(!recipe_unlocked(RecipeId::Cart, 10.0));
        assert!(!recipe_unlocked(RecipeId::StoneHouse, 25.0));
        assert!(recipe_unlocked(RecipeId::IronAxe, 5.0));
        assert!(recipe_unlocked(RecipeId::Cart, 15.0));
        assert!(recipe_unlocked(RecipeId::StoneHouse, 30.0));
    }

    #[test]
    fn insufficient_knowledge_blocks_crafting_an_invented_tool_even_with_materials() {
        let mut economy = test_field();
        let mut carrying = CarryingState::default();
        carrying.add_container("bag", 32);
        carrying.add(ItemKind::Planks, 5);
        carrying.add(ItemKind::IronIngot, 5);

        economy.craft_parts("inventor", &mut carrying, 0, 0.0);
        // Nothing invented is unlocked yet, but Planks/IronIngot themselves
        // aren't consumed by a baseline recipe here (no Masonry present for
        // StoneAxe, no Coal/CopperOre for Fuel/CopperIngot with what's
        // carried) — the point is specifically that IronAxe/IronPickaxe are
        // never attempted.
        assert!(!carrying.has_tool(ToolKind::Axe));
        assert!(!carrying.has_tool(ToolKind::Pickaxe));
    }

    #[test]
    fn sufficient_knowledge_unlocks_crafting_the_iron_axe() {
        let mut economy = test_field();
        let mut carrying = CarryingState::default();
        carrying.add_container("bag", 32);
        carrying.add(ItemKind::Planks, 5);
        carrying.add(ItemKind::IronIngot, 5);

        economy.craft_parts("inventor", &mut carrying, 0, 5.0);
        assert!(carrying.has_tool(ToolKind::Axe));
    }

    #[test]
    fn sufficient_knowledge_unlocks_building_the_stone_house() {
        let mut economy = test_field();
        let elevation = flat_elevation_grid();
        let mut carrying = CarryingState::default();
        carrying.add_container("work basket", 64);
        carrying.add(ItemKind::Masonry, 16);
        carrying.add(ItemKind::Planks, 6);
        carrying.add(ItemKind::IronIngot, 2);

        economy.build_parts(
            "inventor",
            GridPosition::new(2, 2),
            &mut carrying,
            0,
            &elevation,
            30.0,
        );

        assert!(economy
            .structures
            .iter()
            .any(|s| s.recipe == RecipeId::StoneHouse));
    }
}
