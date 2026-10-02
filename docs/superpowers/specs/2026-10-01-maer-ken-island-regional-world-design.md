# Maer-Ken Island Regional World Design

Date: 2026-10-01
Status: Approved architecture, implementation pending plan
Amended: 2026-10-02 — zonal background forcing (§6), vegetation and estate representation levels (§4.5, §4.7), Phase-0 baseline (§17), per-phase previews (§13); owner decisions on island shape (§2), Bevy 0.19 (§13), human tooling (§4.6)
Source project: `dylmarriner/Maer-Ken`
Target project: `dylmarriner/Maer-ken-island`

## 1. Purpose

Maer-Ken Island is a minimized spatial edition of Maer-Ken, not a reduced-fidelity edition.
It retains the deep simulation systems that make Maer-Ken a living world while replacing the
whole-planet spatial domain with one New-Zealand-scale island surrounded by ocean.

The target is a persistent, deterministic regional simulation containing the existing Maer-Ken
human runtime, property and equipment, ecology, geology, tectonics, hydrology, ocean, climate,
weather, resources, buildings, vehicles, tools, computers and local world interactions.

The project must not simulate unrelated continents, settlements or detailed cells on the far side
of a planet merely to support local island behaviour.

## 2. Canonical scale

The default island profile targets approximately New Zealand's total land area:

- target land area: ~268,000 km^2
- one contiguous primary island domain, with shape generated from deterministic terrain/tectonic state
- ocean on every side
- default ocean buffer: configurable, initially 300 km minimum from the generated coastline to each domain edge
- horizontal simulation coordinates are regional Cartesian/metre-based coordinates rather than a required global longitude/latitude raster
- Maer-Ken astronomical and physical constants remain canonical unless an island-specific canon file explicitly overrides a value

The land-area target is a generation constraint, not a fixed rectangle. Coastline, mountains,
valleys, river systems, wetlands, beaches and offshore bathymetry are generated within the regional
domain.

The island is New Zealand's *size*, not its shape. Its outline is unique and procedurally generated
from its own tectonic state — irregular, with headlands, bays and inlets — and must not be a blob,
ellipse or a copy of any real coastline. Shape quality is checked by measurable metrics, and the
owner chooses the default seed from a gallery of generated candidates.

## 3. Architectural principle

The core abstraction is `IslandDomain`: a bounded regional simulation window.

Whole-world systems must be adapted to consume the regional domain and edge/boundary conditions
rather than allocate or iterate a full planetary grid. Systems that need external forcing receive
compact deterministic boundary inputs instead of a simulated rest-of-planet.

Conceptually:

```text
Maer-Ken canonical constants + simulation clock
                  |
                  v
        +-----------------------+
        | IslandDomain          |
        |                       |
        | surrounding ocean     |
        |   +---------------+   |
        |   | NZ-scale land |   |
        |   |               |   |
        |   | property      |   |
        |   | humans        |   |
        |   | ecosystems    |   |
        |   +---------------+   |
        |                       |
        +-----------------------+
                  |
                  v
      persistence / replay / UI
```

## 4. Systems retained

The island build retains real Maer-Ken runtime systems where they are relevant locally.

### 4.1 Geology and tectonics

Retain:

- tectonic plates and boundaries
- crustal deformation
- faults
- uplift/subsidence
- earthquakes
- volcanism
- erosion-coupled landform change
- mineral/resource consequences

The island domain contains enough plate context to produce meaningful local tectonics. Plate state
may extend mathematically beyond the simulation window, but only local cells/entities are materialized.
Boundary forcing must be deterministic and recorded in simulation state/configuration.

### 4.2 Terrain and hydrology

Retain:

- elevation and bathymetry
- mountains, hills, valleys and coastal plains
- river basins and drainage
- rivers, streams, lakes and wetlands
- groundwater where supported by the existing runtime
- erosion, sediment and coastline evolution where supported
- soil/geology-derived terrain properties

Hydrology terminates naturally into the surrounding ocean and must not depend on a global planet grid.

### 4.3 Ocean

Retain:

- surrounding ocean cells
- bathymetry
- tides
- currents
- wave/coastal state where represented by Maer-Ken
- temperature/salinity and marine state where represented
- coastal exchange with rivers/weather/ecology

The regional ocean uses deterministic boundary conditions at the edge of the domain. This preserves
local ocean dynamics without simulating an entire planetary ocean.

### 4.4 Atmosphere, climate and weather

Retain:

- canonical Maer-Ken day/night and seasonal forcing
- temperature
- pressure/wind where represented
- humidity/cloud/rainfall
- storms and local weather
- climate coupling to elevation, coast, ocean and vegetation

Global astronomical calculations may remain analytic. Detailed atmospheric state is local to the
regional domain. Edge forcing represents large-scale circulation entering/leaving the region.

### 4.5 Biosphere and ecology

Retain:

- terrestrial and marine habitat
- plants and trees
- vegetation growth and succession
- fungi/decomposers where represented
- animals and marine life
- genetics/reproduction/evolution where represented by Maer-Ken
- food webs, metabolism and habitat pressures
- extinction/population dynamics where applicable

Deep-time systems must operate on the island population/ecosystem rather than requiring unrelated
global populations.

Vegetation has two representation levels. Across the island, biomass and habitat are aggregate
per-cell quantities and are the authoritative ledger values. Inside high-detail patches (by default
one patch around the founders' estate), individual trees are simulated with positions, size and
growth; their biomass is a disaggregation of the containing cell and must sum back to it. Outside
patches, visible trees are presentation-only instances whose density follows the aggregate fields.

### 4.6 Humans

Retain the current canonical Maer-Ken human runtime, including the existing schema/runtime behaviour
for physiology, needs, cognition, attention, learning, memory, predictive processing, emotion,
temperament, drives, stress, attachment, social cognition, reproduction, development, aging,
death, relationships, dialogue, culture, language, technology and skills to the extent present in
the source runtime.

Gem-D and Gem-K remain canonical founder humans for the default island scenario. They are the only
humans upstream Maer-Ken defines, and they run the complete human runtime — not a reduced version.
The island also carries over the upstream human tooling: human views and detail inspector, the human
foundry for creating new people through the deterministic `new_born_at` path, founder models and
portraits, and the opt-in computer service behind `WebSearch`/`SendEmail`.

The island project must not fork a second independently evolving human model. Human code imported
from Maer-Ken must retain explicit upstream provenance and a deliberate sync mechanism or extraction
boundary.

### 4.7 Property, buildings and interiors

Retain and use the existing Maer-Ken property concepts/assets, including at minimum:

- house
- shed/workshop
- bedrooms
- bathroom
- kitchen
- lounge
- computer room
- garage/workshop spaces
- furniture and household fixtures represented in current data/assets
- storage and inventory relationships

Rooms are navigable/semantic locations in the simulation, not merely decorative labels. The
estate is laid out in metres inside its high-detail patch: building footprints, rooms, doors and
item positions, with a door graph humans route through. The layout derives from the existing
property inventory and never defines items of its own.

### 4.8 Vehicles, tools, equipment and computers

Retain current Maer-Ken property/inventory support for:

- vehicles and vehicle attachments
- workshop/building equipment
- construction, digging, repair, exploration and smithing tools where currently defined
- generators and utility equipment where currently defined
- computers, servers/network equipment and network-account access gates
- human interaction with owned/shared equipment

Computer access remains tied to physical/property state rather than becoming an ungrounded global flag.

### 4.9 Resources and economy

Retain local resource economy behaviour needed for:

- gathering
- mining/extraction
- carrying/inventory
- construction and repair
- crafting/technology actions where supported
- resource depletion/regeneration when represented

Only resources present in the island/ocean domain participate.

## 5. Systems removed or bounded

The following are excluded from detailed runtime state unless needed as compact analytic forcing:

- full spherical terrain raster
- unrelated continents
- unrelated settlements/properties
- whole-planet detailed biosphere populations
- full planetary ocean grid
- full planetary atmospheric grid
- far-side geology/tectonic cells
- global resource inventories unrelated to the regional domain

This does **not** remove astronomy, tides, seasons, tectonic forcing or climate forcing. It changes
how they are supplied to the local simulation.

## 6. Regional boundary conditions

`IslandDomain` owns explicit boundary configuration for systems that previously assumed a complete
planet.

Required boundary categories:

- ocean: inflow/outflow, temperature/salinity/current forcing
- atmosphere: pressure/wind/humidity/temperature forcing
- tectonics: plate velocity/stress continuation across domain edges
- astronomy: analytic sun/moon/season/tide phase derived from canonical clock

Boundary values must be deterministic functions of seed, canon and simulation time, or persisted
state. No wall-clock time or hidden network dependency may feed deterministic simulation state.

The upstream climate relaxes each cell toward the planet's area-weighted mean temperature, which a
regional window cannot compute from its own cells. Global and zonal context therefore comes from a
compact 1-D zonal background model (latitude bands only) that reuses the upstream energy-balance
equations, is calibrated against the upstream global model, and supplies atmosphere and ocean edge
forcing. It is compact analytic forcing under §5, not a planetary grid.

## 7. Spatial representation

The regional domain should support multiple simulation resolutions so expensive systems can remain
tractable:

- coarse geophysical mesh for tectonics/climate/ocean
- medium terrain/hydrology/ecology cells
- high-detail local property/interior/navigation zones near humans and structures

The exact cell sizes are implementation choices and must be benchmark-driven. The architecture must
not require all systems to run at the finest resolution.

Coordinates must share a common regional frame with conversions defined centrally.

## 8. World composition

Default bootstrap order:

1. load canon, seed and regional profile
2. create `IslandDomain` and ocean extent
3. establish tectonic/volcanic/geological state
4. generate elevation and bathymetry toward the configured land-area target
5. initialize ocean boundary/state
6. initialize atmosphere/climate/weather boundary/state
7. derive hydrology, soils and habitats
8. seed vegetation and ecosystems
9. instantiate canonical property/buildings/interiors
10. instantiate equipment, vehicles, tools and computers
11. instantiate Gem-D and Gem-K and bind them to property/location state
12. initialize resources/economy
13. enter deterministic tick loop

Bootstrap must be deterministic for the same canon + seed + scenario configuration.

## 9. Runtime scheduling

Not every subsystem needs the same cadence.

The scheduler should preserve source Maer-Ken rates where valid, while allowing regional overrides:

- human/body/interaction ticks: high frequency
- weather/ocean/local ecology: medium frequency
- hydrology/vegetation/resources: medium/low frequency
- tectonics/geology/deep-time evolution: low frequency or accumulated-step updates

Large elapsed-time advances must use deterministic stepping/aggregation rules rather than skipping
state transitions unpredictably.

## 10. Persistence and replay

The island build must retain deterministic save/load and replay principles.

Persist at minimum:

- canon/profile version
- seed
- simulation clock
- IslandDomain geometry/config
- boundary-condition state/config
- geophysical state
- ocean/climate/weather/hydrology state
- ecosystems
- humans
- property/interiors
- vehicles/tools/computers/inventories
- resources/economy
- lineage/history needed by retained systems

A save restored under the same compatible build must continue deterministically.

## 11. Repository strategy

The current first commit copied broad engine code to make the human runtime self-contained. That is
not the final architecture.

The island repository will be reshaped around the dependency closure required by the regional world.
The implementation should prefer extracting/reusing proven Maer-Ken modules over rewriting them.
Where source modules are too coupled to the planetary `WorldState`, introduce narrow regional
interfaces/adapters and then remove unused planetary-only code from the island project.

Expected target structure:

```text
crates/
  mk_core/          canonical shared types, clock, deterministic utilities, human schema
  mk_island/        IslandDomain, regional world composition and boundary forcing
  mk_engine/        retained/adapted runtime systems needed by the island
  mk_interventions/ retained intervention surface where useful
apps/
  island/           executable/UI entry point
assets/
  humans/
  property/
  vehicles/
  tools/
  computers/
  terrain/
  vegetation/
fixtures/
  human/
  island/
docs/
```

Names may be refined during implementation when preserving source module boundaries is safer than
moving code solely for aesthetics.

## 12. Upstream provenance

The extraction must keep an `UPSTREAM.md` record containing:

- Maer-Ken source repository
- source commit(s)
- imported modules/assets
- island-specific divergences
- synchronization procedure

Intentional island modifications must be reviewable separately from copied upstream code where
practical.

## 13. UI and observability

The initial island UI must be scoped to this regional world and should expose:

- island terrain and surrounding ocean
- vegetation/ecology
- buildings/property
- humans and vehicles
- simulation clock
- inspectable human/property/world state

Planetary globe/orbit views are not required. Regional map/free camera views are appropriate.
Existing Maer-Ken render/property assets should be reused where compatible. The UI targets the
current Bevy release (0.19 at amendment); upstream's Bevy 0.13 UI code is ported, not copied.

Before the UI exists, every implementation phase ends with a deterministic headless preview
(PNG/JSON) of its new state, reviewed by the owner before the next phase builds on it.

## 14. Performance goal

The purpose of the regional architecture is to make Maer-Ken materially cheaper to run while
preserving local simulation depth.

Implementation must establish repeatable benchmarks for:

- bootstrap time
- memory at idle/default world
- simulation ticks per real second
- save/load time and size
- cost by major subsystem

Optimizations must target measured hotspots. Removing whole-planet allocation/iteration is the first
structural optimization.

## 15. Testing and acceptance criteria

The build is accepted when all of the following are true:

1. The repository boots a deterministic single-island world from a fixed seed.
2. Generated land area is within an explicitly documented tolerance of the ~268,000 km^2 target.
3. Ocean surrounds the island on every domain edge-facing direction.
4. Tectonics and volcanism run without requiring a full planet grid.
5. Ocean, climate, weather and hydrology step and exchange state locally.
6. Vegetation/ecology can initialize and advance on land and marine habitats.
7. Gem-D and Gem-K load through the canonical human runtime.
8. The existing house/shed/room/property inventory is present in the island scenario.
9. Current vehicles, tools/equipment and computer/network property items are preserved where defined upstream.
10. Humans can perceive/interact with the local property/resources through real runtime state.
11. Save/load round-trips the complete retained island state.
12. Same seed + same actions + same build produce the same deterministic state hash/replay result.
13. Focused tests for retained Maer-Ken systems pass.
14. No detailed unrelated whole-planet world state is allocated during normal island execution.
15. Performance benchmarks are recorded so the regional build can be compared over time.

## 16. Non-goals

This project is not intended to:

- simplify humans into game NPCs
- replace physical systems with decorative random events
- remove tectonics/ocean/climate/weather for performance
- simulate a complete planet just to preserve old APIs
- maintain a second independent canon that silently diverges from Maer-Ken
- promise real-world biological/geophysical fidelity beyond what the existing Maer-Ken models support

## 17. Migration from current island commit

The current `0c34384` bootstrap remains useful as a tested human-runtime extraction, but it is only a
starting point.

Migration will:

1. make the imported baseline green and CI-gated (it was not fully passing at audit: three
   inherited test failures, no CI, multi-minute debug tests), then preserve it
2. restore the required regional geophysical/ecological/property systems from the pinned Maer-Ken source
3. introduce `IslandDomain` and explicit boundary interfaces
4. replace whole-planet bootstrap/world assumptions with regional composition
5. copy/reuse the existing property, terrain, vegetation and equipment assets needed by the island
6. add deterministic island fixtures/configuration
7. remove planetary-only code once dependency closure and tests prove it is unnecessary
8. add regional benchmarks and replay/save-load gates

The implementation plan following this design must stage those changes so the repository remains
buildable and testable throughout the migration.
