# How far is simulated human work from real life? Inventory, sourced gap-closing figures, plan

Date: 2026-10-13. Repo: /run/media/kubuntux/DEVELOPMENT/Maer-ken-island (branch fix/deployable-backend). No repo files edited.
Companion raw research (full tables, quotes, URLs, VERIFIED/UNVERIFIED tags), same folder:
`work_sources_A.md` (Compendium METs, load carriage, learning), `work_sources_B.md` (shelter, timber, drying), `work_sources_C.md` (foraging, gardens, fishing, fire/water), `work_sources_D.md` (tools, rope, pottery, charcoal, bloomery, mining).
Tag meaning: VERIFIED = researcher read the number in fetched page text; UNVERIFIED = snippet or memory (do NOT put in a pack until read); DERIVED/COMPUTED = arithmetic on a verified number.
Web research limits: Firecrawl out of credits, WebSearch budget ran out partway; gaps are listed in section 4.

---------------------------------------------------------------------------
## 1. Inventory: how each ActionKind is executed on the island today

### 1.0 The headline finding
Two separate action systems exist and the island uses almost none of the upstream one.

1. **Upstream economy path** (`ResourceEconomyState::apply_human_action`, `crates/mk_engine/src/resource_economy.rs:290` `apply_action_parts`, called from `crates/mk_engine/src/humans/mod.rs:1059` via `regional/humans.rs:348 step_regional_humans`). It resolves Gather/Mine/Craft/Build in ONE 60 s tick, with count-based items and no mass.
2. **The island's own timed path** (`regional/life.rs:869 work`, `regional/labour.rs`, `regional/timber.rs`). Only felling a tree and hauling a log use it.

On the island the economy is created empty: `life.rs:394` `economy: ResourceEconomyState::new()` (`resource_economy.rs:151`, `nodes: Vec::new()`). `from_terrain` (`resource_economy.rs:181`) is not called for the island. Consequence: a founder's chosen `Gather`, `Mine`, `Craft` or `Build` finds no node and records a `Failed` event "no resource node" (`resource_economy.rs:333`); `Craft` fails "no craftable recipe" (no inputs); `Build` fails "insufficient building materials". Founders are created with no containers and no tools in `CarryingState` (`agents/mod.rs:99-120`; `add_container` is called only in tests, `resource_economy.rs:921-1154`). So these four actions are, on the island, no-ops except for the energy they are charged and for felling/hauling, which `work()` runs when the action is `Gather`.

### 1.1 Per-action table

| ActionKind | Where executed | Takes time? | Costs energy? | Tools / skill / materials | Mass moved | Produces |
|---|---|---|---|---|---|---|
| **Gather** (outdoors, estate, shed axe present) | `life.rs:869-1046 work` (haul branch `:918`, fell branch `:942`) | YES: walk to tree at `walking_time_s` (`labour.rs:439`) + `fell_tree` steel-axe minutes (`labour.rs:290`, pack `felling_time_30cm_tree`), across ticks (`life.rs:996-998`) | YES: `labour_met`/MET 3.55 walking then felling MET, extra kcal booked `life.rs:1003-1013` | Needs estate item "Felling Axe" (`life.rs:163,1219 estate_has_tool`). Tool is the estate's, always `SteelAxe`, never wears. No skill. Saw/pan items exist but unused | Hauling: logs are cross-cut to <= sustainable load, ute 639.6 kg payload, excavator 5,800 kg (`life.rs:1083 plan_haul`, `timber.rs`). Hand-carry obeys `sustainable_load_kg` (`life.rs:1131`) | Green log on ground (`life.rs:1019-1033`, carbon/water in ledger). Logs never dry, never become usable wood (D40). Felled tree is always a "tree nearest you" |
| **Gather** (anything else: food, fibre, clay, stone) | upstream `resource_economy.rs:327 gather_parts` | NO, one tick | MET 3.55 "walking" (`humans/lifecycle.rs:677`) | Axe/pickaxe/hands by node (`resource_economy.rs:733`). Skill none | Item counts only: 5 per tick with the tool, 1 without (`:346`); hands hold 1 item each (`agents/mod.rs:165-185`), 2 hands + mouth, containers cap in item counts, no kg. | On the island: nothing (no nodes). Upstream/other agents: items appear from a node whose stock is `yield*20` (`:665`) and regrows by fixed ticks (`:710`), not from the ecology ledger |
| **Mine** | `resource_economy.rs:327` (same fn, `action == Mine`) | NO, one tick | MET 6.75 (`lifecycle.rs:667`) | Needs a Pickaxe item (`:342`); no skill; no shaft/hardness; `labour.rs:337 mine_hard_rock` (pack 0.2-2 t/day) exists but is not called anywhere outside tests (`tests/island_labour.rs`) | none | nothing on the island; `regional/deposits.rs` generates real deposits but no action extracts them |
| **Build** | `resource_economy.rs:480 build_parts` | NO, one tick | MET 3.5 (`lifecycle.rs:671`) | Inputs consumed from hands/containers (WoodenShelter = 8 Planks + 2 Rope, `:806`; Storage 6 Planks; StoneHouse gated by `accumulated_knowledge >= 30`, `:88`). No labour hours, no tool, no site prep/foundation, no skill. `labour.rs:391 build_timber_frame` (22 person-h/m2 power tool, x2 hand tools) is not wired | none (counts) | A `ConstructedStructure` with `shelter_quality` constant (0.4-0.95, `:643`) appears instantly. Also `place_structure`/`construct_for_operator` (`:542,577`) make structures with no material if an operator calls them |
| **Craft** | `resource_economy.rs:413 craft_parts` | NO, one tick | MET 3.5 | Fixed recipe table (`:759-835`): 3 Wood -> 2 Planks, 2 Fibre -> 1 Rope, 3 IronOre + 1 Fuel -> 1 Ingot (no furnace/kiln, no fuel burn), axe = 1 Plank + 1 Masonry. First recipe whose inputs are held fires. No mass conservation (3 stone -> 2 masonry), no skill | none | Items appear. Tools get durability 100 and `use_tool` decrements per item (`agents/mod.rs:151`) |
| **Transfer** | `humans/mod.rs:1105-1141` + `agents/mod.rs:299 transfer_one_to` | NO | baseline | adjacent humans within 1 cell (2 km!) | one item per tick, no mass | an item changes hands. No carrying of anything to storage or from stores |
| **SeekFood** (eating) | `life.rs:1241 meals` + `lifecycle.rs:622 is_eating` | Eating time: ATUS 64.5 min/day, 10 mL/s drinking (D10). Walking to food: none (see Move) | MET 3.55 only while food not "at hand" (`lifecycle.rs:657`), else baseline | no foraging skill, no food carried, no cooking, no tool | The ecology cell is harvested for exactly the carbon respired since the last meal (`life.rs:1279 gather_biotic`, radius `FORAGE_RADIUS_CELLS = 1`, i.e. up to 2 km away, `:73`). Ledger-conserving, but food never exists as an object, is never carried, stored, cooked, or spoils | Nutrition via `materials.eat` (`materials.rs:868`) |
| **SeekWater** (drinking) | `life.rs:1241 meals` (`:1314-1360`) | drinking time only | as above | no container, no fetching, no boiling | water drawn from nearest wet cell within `WATER_RADIUS_CELLS = 2` (4 km) or the home stream, drunk on the spot, no jerrycan or storage (`gather_water`, `materials.rs:768`). "Dig for water where they stand" fallback (`:1337`) takes no digging time | water consumed, losses booked to soil/air |
| **Move / Explore / SeekShelter** | `humans/mod.rs:985-1017` | NO real travel time: a human moves one 2 km cell with probability `20 km/day / cell width * dt` (`humans/mod.rs:1389`, `WALKING_KM_PER_DAY = 20`, `:1382`); inside the estate, positions are re-set by reconciliation (`regional/humans.rs:392-448`) | MET 3.55 | no load limit, no terrain/slope in cell moves (Naismith unused outside timber hauls) | none | position change |
| Harm / Intimacy / Rest / Code / WebSearch | not work | n/a | Intimacy 2.05 MET | n/a | n/a | n/a |

### 1.2 Where things appear from nothing, or carrying has no limit

- **Buildings appear without being built.** The estate (house, shed, garage, workshop, armoury, cottages) is placed at bootstrap by `regional/property.rs:315 place_regional_estate` and `estate_layout.rs:614 layout_estate`; nothing in the simulation constructed it. Upstream `Build` creates a structure in one tick from 8 abstract Planks (`resource_economy.rs:806`). The island ignores `labour.rs:391 build_timber_frame`.
- **Resources appear from nothing (or from a ledger-free node).** Upstream nodes are a fixed stock `yield*20` with fixed regrowth, not the ecology (`resource_economy.rs:219,665,710`). `Craft` outputs more mass than it takes (3 Wood -> 2 Planks is fine, 2 Stone-masonry/axe recipes have no mass basis). Meanwhile `materials.rs` has ledger-honest `hunt` (`:558`), `burn` (`:593`), `calcine` (`:626`), `make_charcoal` (`:653`), `craft` (`:684`), `build` (`:1017`), `gather_fossil` (`:532`), but the island tick calls none of them (only `tests/island_material_flows.rs`).
- **Food and water have no logistics.** Food is harvested from the ecology at the eater's cell at the moment of the meal (`life.rs:1279`); water is drawn at the moment of the drink (`:1323`). Nothing is carried, stored, cooked or fetched. Distances (2 km, 4 km) are not walked.
- **Carrying has no load limit in the economy path.** `CarryingState` counts items (hand slot = 1 item, `agents/mod.rs:165`); `ItemKind` (`agents/mod.rs:50`) has no mass. The mass-based rules (`labour.rs:470 sustainable_load_kg`, `:476 max_load_kg`, `:482 check_carry`, pace penalty `:461-465`) are applied only inside the timber haul (`life.rs:1131`, `timber.rs:305`).
- **No skill or learning effect on any rate.** `technology.accumulated_knowledge` only unlocks recipes (`resource_economy.rs:81-97`). No "hours practised" exists; `humans/learning.rs` is cognitive, not craft.
- **Tools never wear on the island path** (the estate's axe has no durability; upstream `use_tool` is item-count based).
- **Labour MET inventory** (`humans/lifecycle.rs:601-604,656-684`): walking 3.55, hand mining 6.75, carpentry 3.5, sexual activity 2.05. Gather is costed as walking, Craft/Build as `carpentry_general`; Carrying, social, harm, idle have no MET (D9 text).
- `labour.rs` already has (not wired): `dig` (`:314`), `mine_hard_rock` (`:337`), `pan_gravel` (`:353`), `build_timber_frame` (`:391`), `ActiveTask` (`:549`, used only by `tests/island_labour.rs`). `ActiveTask::advance` is the right multi-tick carrier, but `life.rs` uses its own `FellingJob`/`HaulTask` maps (`life.felling`, `life.hauling`) instead.

Documented as D9 (`docs/island/DEVIATIONS.md:17`), D40 (`:48`), WORK_REMAINING 1, 2b, 2c.

---------------------------------------------------------------------------
## 2. Sourced real-world figures to close the gap

All rows below are copied from the part files with their tags. URLs are the ones the researchers fetched.
Project rule: a figure goes into a pack only if tagged VERIFIED here (or read by the implementer); UNVERIFIED rows are leads only.

### 2.1 Energy cost (Compendium of Physical Activities, 2024 Adult Compendium, Herrmann et al. 2024, J Sport Health Sci 13(1); pages on https://pacompendium.com/). All VERIFIED as parsed from the site. 2011-edition equivalence UNVERIFIED (the repo's pack cites ainsworth2011; re-check each code).

| Task | Code | MET | Exact description |
|---|---|---|---|
| Felling, small-medium | 08058 | 5.3 | Felling trees, small-medium size |
| Felling, large | 08057 | 8.3 | Felling trees, large size |
| Axe chopping slow/fast | 11260 / 11262 | 5.0 / 8.0 | Forestry, ax chopping, slow, 1.25 kg axe, 19 blows/min, moderate effort / fast ... 35 blows/min |
| Splitting logs | 08019 / 08020 | 4.5 / 6.5 | Chopping wood, splitting logs, moderate / vigorous effort |
| Sawing lumber by hand | 11266 | 8.5 | Forestry, vigorous effort (e.g., barking, felling, or trimming trees, carrying or stacking logs, planting seeds, sawing lumber by hand) |
| Carrying/stacking wood | 08009 / 08010 | 4.1 / 5.5 | Carrying, loading or stacking wood, loading/unloading or carrying lumber, light-to-moderate / moderate effort |
| Haul brush | 08030 | 6.3 | Clearing land, haul brush, undergrowth, or ground, vigorous effort |
| Carpentry | 11038 / 11040 / 11042 | 2.5 / 4.3 / 7.0 | Carpentry, general, light / moderate / heavy or vigorous effort |
| Carpentry sawing hardwood | 06070 | 6.0 | Carpentry, sawing hardwood, planing and drilling wood, moderate-to-vigorous effort |
| Roofing | 06180 | 6.0 | Roofing |
| Hammering nails | 06124 | 3.0 | Hammering nails |
| Digging, shovel, pick | 11540 / 11790 / 08052 | 7.3 / 7.8 / 7.3 | Shoveling, digging ditches / Using heavy tools (not power) such as shovel, pick, tunnel bar, spade / Digging, spading, filling garden, composting, vigorous effort |
| Coal mining by hand | 11110 / 11080 | 6.3 / 5.3 | Coal mining, shoveling coal, by hand / Coal mining, drilling coal, rock |
| Masonry | 11480 | 4.3 | Masonry, concrete, moderate effort (no bricklaying or pottery row exists; proxy only) |
| Carry 15-155 lb level | 17018 | 4.5 | Carrying 15 - 155 lb (6.8 - 70.4 kg) load ..., level ground or downstairs, slow pace |
| Carry 50-150 lb level | 17019 | 6.5 | Carrying 50 to 150 pound load (e.g., equine or bovine feed, fence pipes, furniture), level ground, moderate pace |
| Walk carrying 25-49 / 50-74 / 75-99 / 100+ lb | 11820 / 11830 / 11840 / 11850 | 4.0 / 5.5 / 7.0 / 7.3 | Walking or walk downstairs or standing, carrying objects about ... |
| Fetching water | 11191 | 4.3 | Farming, hauling water for animals, fetching water from a well or stream |
| Head hauling water | 17076 | 4.5 | Hauling water, head hauling, walking on flat surface |
| Wheelbarrow | 08255 / 08256 | 4.8 / 6.0 | Wheel barrow, pushing garden cart or wheelbarrow, general / ... up to 150kg |
| Foraging with pack | 04150 / 04152 | 2.8 / 3.0 | Foraging, 0-9.9kg backpack / 10-15kg backpack |
| Weeding / hoeing | 08241 | 5.0 | Weeding, cultivating garden, using hoe moderate-to-vigorous effort |
| Planting | 08145 | 4.3 | Planting crops or garden, stooping, moderate effort |
| Picking fruit/vegetables | 08248 | 4.5 | Harvesting Produce, Picking fruit off trees, gleaning fruits, picking fruits/vegetables ... vigorous effort |
| Farming moderate / vigorous | 11146 / 11145 | 4.8 / 7.8 | Farming, moderate effort (e.g., ... harvesting crops) / vigorous effort |
| Butchering small / large | 05044 / 05045 | 3.0 / 6.0 | Butchering animals, small / large, vigorous effort |
| Cutting/smoking fish, drying meat | 05046 | 2.3 | Cutting and smoking fish, drying fish or meat |
| Fishing: general, bank+walking, net, spear | 04001 / 04020 / 04062 / 04065 | 3.5 / 4.0 / 3.5 / 2.3 | Fishing, general / from river bank and walking / dip net / with a spear, standing |
| Hunting deer / small game / dragging carcass | 04080 / 04120 / 04081 | 6.0 / 5.0 / 9.8 | Hunting, deer, elk, large game / ... small game / Hunting large game, dragging carcass |
| Cooking | 05049 / 05050 / 05051 | 3.5 / 2.0 / 2.3 | Cooking or food preparation, moderate effort / standing ... light effort / walking |
| Building a fire (indoor row; outdoor none) | 05149 | 2.5 | Building a fire inside |
| Crafts (weaving, carving, standing) | 09090 / 11765 | 3.3 / 4.0 | Standing - arts and crafts, ... weaving, moderate effort / Tailoring, weaving, moderate effort |
| Rope (proxy) | 11620 | 3.8 | Standing, moderate effort, intermittent lifting 50 lbs, hitch or twisting ropes |

No Compendium rows exist (VERIFIED absence) for bricklaying, pottery, blacksmithing/forge (nearest 11430 machine tooling 3.0 is not a fit), thatching, basket/rope making, outdoor fire-lighting, yoke carrying. Use nearest proxy and say so in the pack `note` (this is what `labour.rs` already does with "carpentry_general").

### 2.2 Carrying capacity, load-carriage speed, water

| Claim | Value | Source / URL | Tag |
|---|---|---|---|
| Fighting load limit | "30 percent (about 48 pounds) as a tactical load in a combat zone." | Vogel et al., "Soldier Load; When Technology Fails", Army, 1987. https://regimentalrogue.com/soldiers_load/1987_Soldier_Load_When_Technology_Fails.htm | VERIFIED |
| Approach-march limit | "45 percent of a soldier's body weight on approach marches (for the average soldier. about 72 pounds)" | same | VERIFIED |
| Note on the repo's pack | `sustainable_load_fraction` cites knapik2004 (0.30-0.45, typical 0.33). The Knapik 2004 abstract (PMID 14964502) does NOT contain 30/45%; they are verified only from the 1987 Army article above. Add that source to the pack's `sources`. | part_A section 2a | action item |
| Pre-18th-century marching load | "Before the 18th century, foot soldiers seldom carried more than 15 kg while on the march" | Knapik, Reynolds, Harman, Mil Med 2004;169(1):45-56. https://pubmed.ncbi.nlm.nih.gov/14964502/ | VERIFIED |
| Load-position energy | "each kilogram added to the foot increases energy expenditure 7% to 10%; each kilogram added to the thigh increases energy expenditure 4%" | same | VERIFIED |
| Free carrying | head-carrying women "could carry loads of up to 20% of their body weight without increasing their rate of energy consumption ... a 30% load increased energy consumption by 10%, a 40% load by 20%" | Maloiy, Heglund, Prager, Cavagna, Taylor, Nature 1986;319:668. https://doi.org/10.1038/319668a0 | VERIFIED (abstract) |
| Heavy porters | Nepalese porters "carry head-supported loads equal to 100 to 200% of their body weight ... for many days up and down steep mountain footpaths"; typical about 80-90% of body mass; "No mountaineers and only a few porters were able to carry 60 kg loads" | Bastien et al., Science 2005;308:1755 https://doi.org/10.1126/science.1111513 ; Minetti et al. 2006 https://pmc.ncbi.nlm.nih.gov/articles/PMC1635507 | VERIFIED. These are elite trained porters, not a founder |
| Walking energy model accuracy | Pandolf equation "accurately predicted metabolic cost for 40 and 50 kg at 4.8 km/h ... Most equations underpredicted" | Vine et al. 2022, J Strength Cond Res 36(5):1297. https://pubmed.ncbi.nlm.nih.gov/32398631/ | VERIFIED (equation form itself UNVERIFIED) |
| Soldier marching MET | 11882 "Soldiers, military marching, 1.5 to 2.5 mph, 10 to 30 kg load" 4.0; 11886 "loaded marching, varying terrain, 25-40 kg load" 6.3 | Compendium https://pacompendium.com/walking | VERIFIED |
| Walking speed with load | no direct figure retrieved; the repo's `walking_time_s` falls linearly to 70% pace at max load (`labour.rs:461-465`), an own assumption | | gap |
| Water per person | WHO: "a minimum of 7.5 litres per capita per day will meet the requirements of most people under most conditions"; basic access "average quantity unlikely to exceed 20 l/c/d", "Between 100 and 1000m or 5 to 30 minutes total collection time"; no access "quantity collected often below 5 l/c/d", "More than 1000m or 30 minutes"; intermediate about 50 l/c/d; optimal 100+ | Howard & Bartram, WHO/SDE/WSH/03.02, 2003. https://www.arctic.gov/uploads/assets/watersan6-water-quantity.pdf | VERIFIED |
| Sphere 15 L/day | quoted inside the WHO text only; current Sphere handbook not read | same | VERIFIED as cited |
| Water-container weight | "average container weighed 19.5 kg" | Geere et al. 2018 J Glob Health 8:010406 (search summary) | UNVERIFIED |
| Water trips | "women collected water, on average, 23 times each day during the summer months" (Haryana) | Graham, Hirai, Kim 2016 PLoS ONE. https://pmc.ncbi.nlm.nih.gov/articles/PMC4889070 | VERIFIED |
| Yoke/two-bucket | none found | | gap |

DERIVED for the plan: 20 L is about 20 kg of water; a founder at sustainable load 0.33 x body mass carries about 20-25 kg, so a 20 L container is a one-load trip and each family member's 2.7-3.7 L/day total water (IOM, repo pack `total_water_adequate_intake`) plus cooking/washing quickly implies several trips a day if the source is 1-2 km away. Today there are zero trips.

### 2.3 Learning and skill effect on rates

| Claim | Value | Source / URL | Tag |
|---|---|---|---|
| Flint knapping novices | "4 hours of practice did not result in any increase in the overall productivity or reduction intensity of novices"; experts "produced flakes that were not only more numerous, but also larger and more elongated"; expert level = "multiple years" | Stout et al., ch. 12 in Schick & Toth (eds), The Cutting Edge, Stone Age Institute 2005. https://www.stoneageinstitute.org/pdfs/cutting-edge-ch12-stout.pdf | VERIFIED |
| Learning functions | "The exponential function fit better than the power function in all the unaveraged data sets" (40 data sets, 475 subjects) | Heathcote, Brown, Mewhort 2000, Psychon Bull Rev 7:185. https://pubmed.ncbi.nlm.nih.gov/10909131/ | VERIFIED |
| Wright 80% | "Wright found that phi approx 80 % in aircraft manufacturing"; "unit cost decreases by 20% for every doubling" | Wright 1936 via https://en.wikipedia.org/wiki/Learning_curve | VERIFIED via secondary source only; it is an aircraft factory, not manual crafts |
| Blockwork | "the patterns emerged from this study provide little evidence for that" (a learning effect, 52 buildings) | IJPPM 2016 https://www.emerald.com/insight/content/doi/10.1108/IJPPM-08-2015-0119/full/html | VERIFIED (abstract). Warns against a universal construction learning rate |
| Practice explains little in jobs | "less than 1% for professions" | Macnamara, Hambrick, Oswald 2014 Psychol Sci. https://pubmed.ncbi.nlm.nih.gov/24986855/ | VERIFIED |
| English apprenticeship | "seven years was set as the legal minimum in 1563; shorter terms of three to five years seem..." ; apprentices "heavily engaged in productive work from the beginning" | Wallis, LSE WP 2207, 2007. https://www.lse.ac.uk/Economic-History/Assets/Documents/Research/FACTS/WorkingPapers/2007/2207Wallis.pdf | VERIFIED |
| Adze-maker | "may last five years or more" | Stout 2002 Curr Anthropol 43(5) | UNVERIFIED |
| Novice vs expert productivity ratio for felling, thatching, bricklaying | none found | | gap |

Reading: no source gives a novice/expert ratio for these crafts. The defensible, sourced shape is "slow, many years (3-7), apprentices productive from the start, exponential not power law". The ratio itself is a parameter the owner must choose (mark as assumption, not as data) until sourced.

### 2.4 Shelter and timber-frame house (part_B)

| Item | Value | Source / URL | Tag |
|---|---|---|---|
| Repo pack today | `timber_frame_construction_labour` 15-35 (typical 22) person-hours/m2 power tools; hand tools "only roughly double it" (rsmeans, low confidence) | `fixtures/reference/labour/productivity.json` | in repo; the "x2 hand tools" is a note, not a cited number |
| Framing lumber per US house | "in addition to roughly 15,000 board feet of framing lumber" (about 35 m3 DERIVED) | NAHB Eye on Housing, July 2022. https://eyeonhousing.org/2022/07/since-pandemic-onset-lumber-products-have-added-14k-to-house-price-51-to-rent/ | VERIFIED |
| Framing timber per NZ house | "approximately 14 m3 of structural timber per house" | van Houtte & Parker, NZ J Forestry 64(2) 2019. https://nzif.org.nz/nzif-journal/publications/downloadfulltext/23013 | VERIFIED. Floor areas not stated, so m3/m2 cannot be formed |
| Sawmill recovery | mean 32%; "volume recovery is benchmarked at 60%" | Ngobi et al., Afr J Rural Dev. https://afjrdev.org/index.php/jos/article/download/391/328/1979 | VERIFIED (Uganda softwood). Pit-saw recovery not found |
| Pit saw output | "compared to the 12 or so a day by two men in a saw pit" (boards/day) | https://en.wikipedia.org/wiki/Saw_pit | VERIFIED but conflicts with pay-rate inference (about 100 ft/day) and an UNVERIFIED 3,068 sq ft in 60 h for two sawyers (Lost Art Press) |
| Hewing | "In cool weather I hew one side of a 15-ft. log 15 in. in diameter in about half an hour." Green logs "weigh between 40 lb. and 70 lb. per cubic foot" | Fine Woodworking https://finewoodworking.com/?p=443495 | VERIFIED |
| Thatch | "About 150 bundles make a tonne, which covers about 4 squares, or 37 square metres, of long straw thatching" (about 27 kg/m2 applied); finished roof "typically around 34 kg/m2" | https://thatchinginfo.com/long-straw-thatch/ ; https://en.wikipedia.org/wiki/Thatching | VERIFIED (trade site / encyclopedia). Thatcher m2/day not found |
| Iron Age roundhouse (Butser) | 6 m house: "Just over one tonne of straw was used to thatch this house"; daub "3.5 tonnes of clay, 3.5 tonnes of earth, 40 bales of straw, the hair from 40 pigs"; Pimperne 12.8 m: "The rafters were in effect trees, weighing 740- 985 kg"; Moel y Gerddi: "It took some three months to thatch the house." | http://www.butser.org.uk/maidencastle.html ; https://butser.org.uk/pimperne.html ; http://www.butser.org.uk/moelygerddi.html | VERIFIED. No person-day totals exist in readable sources |
| Sod house | "the sod, about one acre for the average house"; "in two or three weeks, with an investment of two to twelve dollars, a sturdy house was built" | Welsch, Nebraska History 48 (1967). https://history.nebraska.gov/wp-content/uploads/2019/09/doc_publications_NH1967NE_Soddy.pdf | VERIFIED (blocks 2x3 ft x 3-4 in; re-creation: bricks "nearly 85 pounds" apiece, https://www.tsln.com/news/special-interest/homeland/building-history-sod-walls-and-appreciation-for-pioneers/) |
| Cob | one builder: "660 cu.ft of cob will be built in 660/5 = 132 hours or 19 7hr-person-days"; "I can build a 400 sq ft, 1 bedroom cabin in a month" | https://cob.lasqueti.ca/blog/qa-about-fast-cob-and-our-natural-building-workshops | VERIFIED but self-reported by an expert |
| Adobe | brick "weighing about 10 kg"; "two can produce about 500 adobes per day"; drying "between 1-3 months" | https://en.wikipedia.org/wiki/Adobe ; https://greenhomebuilding.com/QandA/adobe/ways.htm ; https://www.parks.ca.gov/pages/585/files/FTSHP_SLHP_Adobe_Brick_Making.pdf | VERIFIED (practitioner) |
| Post holes / hand excavation | "approximately 20 to 30 minutes" per hard post hole; ordinary soil 3.0-4.0 m3/person-day (repo pack holds 3-6 soft soil, johannessen2008, medium confidence) | https://www.ramirent.sk/en/ramiguide/... ; https://infralens.in/handbook/excavation-rates | VERIFIED (secondary sites) |
| Lime | "0.5 tonnes or more of coal being used per tonne of finished lime (15 MJ/kg)"; "For every tonne of limestone burnt, around 250kg of wood is needed." | https://en.wikipedia.org/wiki/Lime_kiln ; https://www.lowimpact.org/posts/infoarticle/burning-lime/ | VERIFIED |
| Brick firing | "0.7 cubic yards of fuelwood to fire 1000 bricks" (example only); clamps "2,05MJ/kg of fired clay" | Practical Action brick production | VERIFIED |
| Samoan fale | lashing "30,000 to 50,000 feet (9,100 to 15,200 m) of ʻafa" per house; "can take months of work" | https://en.wikipedia.org/wiki/Architecture_of_Samoa | VERIFIED (encyclopedia). Maori whare build time NOT found |
| Not found | person-days for hut/cabin/lean-to, nails per house, hand-vs-power labour ratio, shingle output, poles per m2 | | gaps |

Reconciling with the repo: NZ 14 m3 per house against the repo's `timber.rs` mass-based logs gives the material side; labour 22 h/m2 (power tools) x floor area is the labour side (a 100 m2 house is 2,200 person-hours power-tool, 275 eight-hour person-days). The doubling for hand tools is unsourced; two founders alone would need about 275 working days even at the power-tool figure.

### 2.5 Wood seasoning (D40), firewood
| Item | Value | Source / URL | Tag |
|---|---|---|---|
| Air drying 1-in lumber | "15 to 30 days for some of the low-density species ... 200 to 300 days for slow-drying species" to 20-25% MC | Wood Handbook FPL-GTR-190 ch.13 p.13-6. https://research.fs.usda.gov/download/treesearch/37419.pdf | VERIFIED |
| Floor of air drying | "the minimum moisture content of thoroughly air-dried lumber is 12% to 15%" | same | VERIFIED |
| 4/4 red oak, Madison WI, days to 20% MC | stacked 1 Jun 62 d; 1 Jul 66; 1 Aug 82; 1 Sep 154; 1 Oct 184; 1 Jan 144; 1 Mar 100 | Simpson & Hart 2000, FPL-GTR-121. https://research.fs.usda.gov/download/treesearch/9684.pdf | VERIFIED; "the time range for air drying northern red oak is 70 to 200 days" |
| Firewood | "Typically, seasoned (dry) wood has 20% or less moisture content."; air seasoning "one to three years" | https://en.wikipedia.org/wiki/Firewood | VERIFIED; blogs say 6-12 months (UNVERIFIED), conflict |
| "1 year per inch" rule | contradicts measured 62-184 days for 1 in oak | Lost Art Press (blocked) | UNVERIFIED; do not use |
| Thickness effect | drying time rises roughly with the square of thickness | not read | UNVERIFIED |
| No NZ outdoor EMC data found | | | gap. US EMC table (Chicago 12.2-14.2%, Juneau 13.6-18.1%) is in the Wood Handbook ch.13 |

### 2.6 Food, water, fire, cooking (part_C)
| Item | Value | Source / URL | Tag |
|---|---|---|---|
| Acorn gathering (gross, excludes leaching/grinding) | "The average caloric return for participants foraging without a load was 6142 kcal/h." ; acorns 3979 kcal/kg | Niclou et al. 2025, Am J Biol Anthropol. https://pmc.ncbi.nlm.nih.gov/articles/PMC11923397 | VERIFIED; do not use as an all-in return |
| Root digging | Hadza children "about 307 g/hr (around 163 kcal)"; Mikea "net acquisition rates of 536 kcal/hr for girls and 504 kcal/hr for boys"; "656 net kilocalories an hour when harvesting ovy tubers" | Mateos et al. 2022 https://pmc.ncbi.nlm.nih.gov/articles/PMC9250475 ; Pretelli et al. 2022 https://pmc.ncbi.nlm.nih.gov/articles/PMC9555775 | VERIFIED (secondary reports); factor-of-10 spread across studies |
| Hadza walking | 5-12 km/day average daily movement | Brill & Dyble 2025 Table 1 https://pmc.ncbi.nlm.nih.gov/articles/PMC12645331 | VERIFIED |
| Kumara (NZ) yield | "23 year mean yield at Whatarangi was found to be 10.15 +/- 0.95 tonne/ha, and over 14 years at Robin Hood Bay as 7.53 +/- 1.24 tonne/ha"; planted mid-October, harvested mid-April to mid-May; 113.8 kcal/100 g; model fallow 14 y, cropping 2 y, storage loss 10%, seed 1.10 t/ha, 2000 kcal/person/day | Leach et al. 2024, J Pacific Archaeology 14(1). https://pacificarchaeology.org/index.php/journal/article/download/... (see part_C for full URL) | VERIFIED |
| Garden area per person | 370 m2 (advocate claim) to 700 m2 (FAO, forum) to 5,000 m2 (Pimentel); 10x spread | part_C | mostly UNVERIFIED; do not pack until sourced |
| Hand-hoe weeding | 140 h/ha (FAO 1990 via a paper snippet) | https://cris.library.msu.ac.zw/handle/11408/4386 | UNVERIFIED |
| Venison yield | "roughly 48.5 percent of the field-dressed weight" boneless | Bangor Daily News 2025 citing ODNR/NDA | VERIFIED (news) |
| Wood fuel energy | "the energy content of wood is 4.5 kWh/kg or 16.2 gigajoules/tonne"; red oak 14.9 MJ/kg | https://en.wikipedia.org/wiki/Fuelwood ; https://en.wikipedia.org/wiki/Wood_fuel | VERIFIED (secondary) |
| Household firewood | Kenya "5.4kgd-1 in the rocket stove group and 6.7kgd-1 in the three-stone stoves group" per household; Uganda "1.56 m3 of firewood per year" for about 7 people (0.22 m3/person/yr COMPUTED) | https://researchportal.ukhsa.gov.uk/en/publications/a-comparison-of-fuel-use-between-a-low-cost-improved-wood-stove-a/ ; https://nru.uncst.go.ug/handle/123456789/6384 | VERIFIED; per-person about 1-1.3 kg/day COMPUTED, below the 3-5 kg/day in the brief (not found) |
| Heating 1 L water 15 to 100 C | about 355 kJ; about 0.15 kg dry wood at 15% efficiency | COMPUTED (constants from memory) | COMPUTED |
| Three-stone fire efficiency | 11.7-22.8% | MPRA/ILNS (snippet) | UNVERIFIED |
| Cooking time (modern US) | "food preparation and cleanup 0.71 h/day average (men 0.51, women 0.91)" | BLS ATUS 2025 https://www.bls.gov/news.release/atus.t01.htm | VERIFIED |
| Boil water | "Heating water to a rolling boil ... is sufficient to inactivate pathogenic bacteria, viruses and protozoa." (no duration on page) | WHO-FWC-WSH-15.02 | VERIFIED for rolling boil; "1 minute" UNVERIFIED |
| Bow drill | beginners 10-45 min, success 60% first attempt | hobby blogs | UNVERIFIED (anecdote) |
| Collection time threshold | "total water collection time should be less than 30 minutes for basic access" | Graham 2016 PMC4889070 | VERIFIED |
| Not found | grinding g/h, fishing CPUE, hunting hours/kill, butchering minutes, tanning, crop days to maturity, seed rates, bed digging min/m2, clay pot volumes, fuelwood/water hours per day (Blackden & Wodon, Sorenson 2011), Kelly foraging table | | gaps |

### 2.7 Tools, rope, pottery, charcoal, smelting, mining (part_D)
| Item | Value | Source / URL | Tag |
|---|---|---|---|
| Stone-axe felling | "thirty minutes to an hour are needed to fell a willow whose trunk is 20 cm in diameter, and between an hour and an hour and a half to cut down an ash tree with the same dimensions" | Chalain/Clairvaux palafittes site, French Ministry of Culture (see part_D for URL) | VERIFIED. The repo pack (saraydar1971) has stone axe 40-120 min for 30 cm DBH |
| Metal vs stone axes | "the process went markedly faster using metal axes than with the stone axes, particularly when cutting larger trees"; bronze "just as good as steel" | Mathieu, Expedition 42(2) 2000 (no numbers; the 1997 J Field Archaeol paper unobtainable) | VERIFIED (qualitative) |
| Flint axe grinding | 4.5-5.5 h for faces and edges (Madsen 1984) | secondary snippet | UNVERIFIED |
| Hand-forged nails | 200-300/day novice, 800-1,000 practised, about 2,300 skilled | snippet | UNVERIFIED |
| Hand flax stripping | "A machine could produce about 250 kilograms per day, while stripping by hand produced about 1 kilogram." | Te Ara, https://www.teara.govt.nz/en/flax-and-flax-working/page-4 | VERIFIED (single sentence, skill unstated) |
| Cordage m/h | 1.5-4 m/h anecdote | forum | UNVERIFIED |
| Pottery firing fuel | "The fuel consumption during experimental firings at CEA was on average between 0.5 to 1 m3."; clamp max "868 C" about 27 h; kiln >900 C in under 9 h, unloaded after "49 hours" | Ther 2004 (see part_D) | VERIFIED. Volume-based; wood kg per kg pot not obtainable |
| Charcoal yield, earth mound | "an average yield of 16% on weight basis, or 128-kg charcoal/mst wood"; "between 9 and 22 days" per kiln; "groups of 4 to 8 men"; Marracuene "12% on weight basis"; improved Kenyan earth kiln "recovery of about 27%" | FAO X6796E https://www.fao.org/4/X6796E/X6796E05.htm ; https://www.fornis.net/sites/default/files/documents/CharcoalProduction.pdf | VERIFIED. Repo pack `charcoal_yield` (fao1987): 0.10-0.20 earth mound, consistent |
| Bloomery smelts | Cleere 1969: 91 kg ore, "217.5 lb (98.6 kg) of charcoal", "10.5 hours", "a bloom weighing 19.5 lb (8.8kg)" (other 3 smelts "little or no bloom"); Wareham Forge Exp 40-42: 18-20.7 kg ore, 28-43+ kg charcoal, 3.5-4.75 h, 2.9-5.6 kg bloom; Wealden 2015: 73 kg charcoal for a 1.85 kg billet from 43 kg ore, with a 2-hour preheat; EconoNorse guide "Expect to consume from 50 - 75 kg of hardwood charcoal per smelt", "5 to 6 hours", "at least THREE individuals" | https://www.wealdeniron.org.uk/?p=2908 ; https://warehamforge.ca/ironsmelting/smelt/smelt09.html ; https://www.wealdeniron.org.uk/smelting-experiments-in-2015/ ; https://www.warehamforge.ca/ironsmelting/iron2005/Econo-Norse.pdf | VERIFIED. Charcoal per kg raw bloom 8-11 kg (repo pack bloomery 1-2 kg charcoal per kg ore is consistent per kg ore; bloom per kg ore 0.1-0.25 matches Cleere 0.097, Wareham 0.16-0.27) |
| Re-smelting 1790 | "Two men working for two days can re-smelt 1 Skippund iron" (about 160 kg) | Evenstad 1790, Univ. Oslo, https://khm.uio.no/english/research/previous-projects/langeid/project/iron-steel/evenstad | VERIFIED (reworking, not ore) |
| NZ ironsand | "beach sand composed of a type of iron oxide called titanomagnetite"; "Between 1869 and 1914 at least four New Zealand companies built blast furnaces to smelt ironsands, but all encountered seemingly insurmountable difficulties" (fine grains, "a thick pasty layer ... blocked up the tap holes") | Te Ara https://teara.govt.nz/en/iron-and-steel/print | VERIFIED. Bloomery-on-ironsand: no source |
| Salt boiling | "To make one ton of salt, you needed 33 tons ... of seawater and as much as 21 tons of wood."; seawater 3.5% evaporites, 77.8% NaCl | https://www.visitgreateroslo.com/en/sarbuvollen ; Greenwood (citcem.org) | VERIFIED (tourism page: upper bound) |
| Gold panning | "Assuming about ten pans per hour, a good panner should be able to pan about a half-yard a day. A very proficient panner working easy material may be able to pan up to a yard a day." | Detector Prospector guide (hobbyist; Wells 1969 not obtained) | VERIFIED (hobbyist). Repo pack: 0.3-1.0 m3/day (wells1969) consistent |
| Hand coal mining | NSW 1948 "10.4 tons per man shift" for a face miner using explosives; "3.4 tons" all employees; US 1891 "2.57 tons per man per day" (all employees) | Elford & McKeown 1948 (see part_D); wisc.edu Minerals Yearbook (snippet) | NSW VERIFIED but not comparable to a pick-only island; US UNVERIFIED |
| Quarrying, fire-setting, lime labour, copper, sun-salt | none | | gaps |

---------------------------------------------------------------------------
## 3. Implementation plan (ordered, founders' daily life first)

### 3.0 Principles
- One mechanism: generalise `life.rs work` to a **task queue per human** built on `labour.rs ActiveTask`. Today there are two ad hoc maps (`life.felling`, `life.hauling`, with `FellingJob`/`HaulTask`). Replace them with `life.tasks: BTreeMap<AgentId, VecDeque<TimedTask>>` where each task = `{ kind, spec: TaskSpec, location, inputs_required, outputs, mass_carried_kg }`. `ActiveTask::advance` already carries a task across ticks and delivers output as the work progresses. The state must be serialised and hashed (snapshot version bump, `ISLAND_SNAPSHOT_VERSION` `life.rs:171`, `state_digest` `life.rs:1624`), exactly as felling is today (`life.rs:1519-1524`).
- One inventory: give the island's founders a **mass-based load** (kg) alongside the item-count `CarryingState`, or better replace the count slots for island humans with a `Load { items: Vec<(Material|ItemKind, kg)> }` enforced by `labour.rs check_carry` (`:482`). Items carry real mass via `materials::composition` and the catalogue densities. The ledger (`materials.rs`) remains the only place stocks change.
- Everything conservative: any new transformation goes through `MaterialStocks` (`materials.rs:684 craft`, `:653 make_charcoal`, `:593 burn`, `:558 hunt`, `:1017 build`) inside `life.audited` so the flux ledger closes.
- Rule: no new number without a VERIFIED source; "estimate" items go in a new pack file `fixtures/reference/labour/assumptions.json` flagged `confidence: "assumption"` and listed for owner sign-off.
- Order: things the founders do every day first (eat, drink, warm, cook), then supply chains (wood, tools), then construction, then industry.

### 3.1 Step 1 (foundation, small): mass inventory and load enforcement
1. Add a `carried_kg` and per-item `kg` to the island human's carrying (new struct beside `agents/mod.rs:99 CarryingState`, island-only to avoid touching upstream determinism), capacity from `LabourTable::max_load_kg` and `sustainable_load_kg` (`labour.rs:470,476`).
2. Containers become real: bucket/jerrycan (20 L = 20 kg water; WHO 20 L/day basic access and the unverified 19.5 kg container), basket/sack, backpack. The estate shed/store lists tools already (`life.rs:1219 estate_has_tool`); extend to containers so "has container" gates trips.
3. Apply load to pace and energy: `walking_time_s` (exists) plus a MET lookup by load using the Compendium walking-with-load rows 11820/11830/11840/11850 (VERIFIED) and 17076/11191 for water. Add the 7-10% per kg-on-foot note only as a comment.
Tests: `labour_rejects_over_max_load` (exists-style), `a_founder_cannot_pick_up_more_than_the_sustainable_load_in_one_trip`, `carry_pace_falls_beyond_sustainable_fraction`, conservation test: moving kg between ground, hands, store leaves `MaterialStocks` total unchanged.

### 3.2 Step 2: water fetching and storage (highest daily value, simplest chain)
- Replace the instantaneous draw in `meals` (`life.rs:1314-1360`) with: if the household cistern/stream is within home range, drink there; else task `FetchWater { from, to, litres }`: walk to source (distance from `EstateLayout`/cell centres, `walking_time_s` with 0 load), fill (hydrology draw `materials.gather_water`, `materials.rs:768`), walk back loaded (<= container size, <= max load), pour into the house store (new `Material::Water` stock at the house). Drinking then draws from the store, not the stream.
- Needed numbers: MET 4.3 (11191) / 4.5 (17076) (VERIFIED); WHO 7.5/20 L/day, 30 min collection (VERIFIED); container weight (gap: read Geere 2018 table). Boiling: fuel from Step 4; "rolling boil" VERIFIED, duration UNVERIFIED.
Tests: `fetched_water_arrives_in_the_store_and_leaves_the_stream_by_the_same_mass`, `a_round_trip_takes_distance_over_loaded_pace_plus_fill_time` (against `walking_time_s`), `no_container_no_carry`, `founders_drink_from_the_store_and_refill_before_it_runs_dry`, extend `island_life_acceptance.rs` and `island_water_supply.rs` drinking bound (0.5-1.5x IOM share) to stay green.

### 3.3 Step 3: foraging, carrying food home, eating from the larder
- `SeekFood` becomes a foraging trip: choose a cell with plant food (`materials.gather_biotic`, `:383`), walk there (distance from cell centre, real pace), gather for `t` minutes at a sourced kcal/h, carry kg back, store in the house larder, eat from the larder (D10 eating time stays: 64.5 min/day ATUS).
- Rates: Compendium foraging with pack 04150/04152 (2.8-3.0 MET, VERIFIED) or gathering 4.5 (08248); returns per hour: use a LOW and a HIGH from VERIFIED rows only (Hadza/Mikea roots 163-656 kcal/h, Hadza child males 1,089 kcal/h; acorns 5,100-6,100 gross but needs the processing cost which is a gap). Ecology supply constraint is already in the ledger (the cell's PlantFood).
- Gap to close before this lands: a cited kcal/h for NZ-island wild food (fernroot, berries, shellfish, eels). Kelly (1995)/Binford tables are the standard source and were not reachable; the implementer should read them or Venkataraman 2017.
Tests: `a_forager_brings_back_no_more_than_their_return_rate_times_time`, `food_in_larder_plus_body_equals_food_taken_from_the_ecology` (carbon closure), `walking_to_food_takes_the_distance_over_pace`, and the existing eating/body-carbon band `0.98-1.02x` must still pass.

### 3.4 Step 4: firewood, fire, cooking (D40 and WORK_REMAINING 1)
- Close 1a/1b: add moisture drying of stacked logs using the Wood Handbook air-drying data (VERIFIED: 15-30 d to 300 d, minimum 12-15% MC; per-month stacking-date table for red oak) with a thickness scaling that must be sourced first (UNVERIFIED now). Seasoned (<= 20% MC) wood joins the stock (`materials.rs` Wood). Splitting: new `SplitWood` task, MET 4.5/6.5 (08019/08020 VERIFIED); a rate in kg/h is a GAP (no source found) so first version can derive it from blows per minute at 11260/11262 (19-35 blows/min, VERIFIED) times an assumed kg/blow flagged as an assumption.
- Fire: `MakeFire` task needs tinder + fuel + an ignition source from the estate (matches/lighter item, owner decision) so no bow-drill timing is needed for founders; 2.5 MET (05149, an indoor row, flagged). Burning goes through `materials.burn` (`:593`), booking CO2/O2.
- Cooking: `Cook` task consumes larder food and wood kg: fuel per meal from VERIFIED data (wood 16 MJ/kg; 1 L water 355 kJ COMPUTED; three-stone efficiency UNVERIFIED so use cooking-stove/estate-stove data from the estate's items first). Time: MET 3.5/2.0/2.3 (05049/05050/05051, VERIFIED), duration from ATUS 0.71 h/day (VERIFIED, a modern average).
- The estate probably has a stove/heater in its items; check `estate_layout.rs` spaces and `energy.rs` loads first so cooking uses the estate's own appliance (electric/gas), burning wood only where the estate has a wood stove.
Tests: `a_log_dries_toward_equilibrium_not_instantly`, `green_wood_burns_but_gives_less_heat` (only once the sourced moisture-heat relation exists), `burning_wood_books_the_same_carbon_as_the_ledger_returns` (carbon, O2 closure through `audited`), `cooking_consumes_what_it_returns`, `fire_needs_fuel_and_a_lighter`.

### 3.5 Step 5: route Gather (stone, clay, fibre, sand) and Mine through timed tasks
- Create real resource sites from the island's own state, not `from_terrain` count nodes: stone and clay from geology/lithology, fibre (harakeke) from vegetation, ore from `regional/deposits.rs`. Each work task: walk (distance), work duration from rate (kg or m3 / person-hour) at the Compendium MET, output into the load (mass-limited), plus tool durability wear.
- Rates available now (repo packs): hand excavation 3-6 m3/day soft soil, 1.5-3 hard clay/gravel, 0.5-1.5 soft rock (johannessen2008); artisanal hard-rock mining 0.2-2.0 t/day typical 0.5 (hentschel2002); gold panning 0.3-1.0 m3/day (wells1969). `labour.rs dig/mine_hard_rock/pan_gravel` already compute `TaskSpec`. Add: fibre stripping about 1 kg/day by hand (Te Ara, VERIFIED, single sentence, flag), cordage (gap). Compendium MET: 11790 (heavy tools 7.8), 11110/11080 (coal 6.3/5.3), 11540 (7.3). The repo uses `HAND_MINING_MET = 6.75` "hand_mining"; keep.
- Tools: Pickaxe/Shovel/Chisel required as items with durability; `ToolKind` expansion is listed in WORK_REMAINING 2b. Depletion and no regeneration for ore (`deposits.rs`).
Tests: `mining_needs_a_pick`, `a_shift_of_digging_moves_the_packs_cubic_metres` (matches `island_labour.rs` style), `ore_removed_equals_deposit_tonnage_lost`, `panned_gold_equals_grade_times_volume_times_recovery`.

### 3.6 Step 6: Build as timed, material-consuming work (staff cottages, farm buildings)
- `Build` becomes `BuildStructure { design, site }` with a bill of materials (timber m3 by design, from VERIFIED NZ 14 m3 per house or per-design numbers once floor-area-based data is read; fasteners gap), a site (foundation: dig post holes 20-30 min each, VERIFIED secondary), and labour = `build_timber_frame(floor_m2, power_tools)` hours at 3.5 MET. Materials must be at the site (hauled via the same load tasks as Step 1, existing excavator/ute logic for beams). Progress is incremental: a partly-built `Structure { progress: 0..1, materials_in }` is visible; shelter quality only rises when complete; unfinished buildings have no shelter value.
- The "hand tools double" labour is unsourced; ask the owner either to accept a flagged factor or let the shed's power tools (estate has them, D13) do the job: power tools consume energy from `EstateEnergy` (D13 already models). Prefer the power-tool path for the founders and keep 22 h/m2 (rsmeans pack, low confidence); two founders alone cannot build more than a small cottage in a year, which is itself a realistic constraint and matches the owner's staff/town decisions (memory: staff in own cottages).
- Interaction with Step 5: sawmilling. Cutting logs to boards: use 32% mean/60% benchmark recovery (VERIFIED) only for pit-saw/small mills until a better source is read; machine mills are outside D9.
Tests: `a_cottage_cannot_be_started_without_its_timber_on_site`, `building_takes_floor_area_times_pack_hours_across_ticks`, `timber_used_equals_log_wood_removed_from_the_store`, `an_unfinished_house_gives_no_shelter`, `two_builders_finish_in_half_the_elapsed_time_only_up_to_the_crew_limit` (needs a sourced crew-scaling or a flagged assumption), snapshot/restore mid-build.

### 3.7 Step 7: tools, rope, pottery, charcoal, smelting (industry chain)
- Order: rope (cordage, gap), clay pottery (water jars, which feed Step 2; sourced fuel 0.5-1 m3 per firing, 868-900+ C), charcoal (16% mound yield with 9-22 day cycle and 4-8 men, VERIFIED; repo pack 10-20%), lime/bricks (VERIFIED fuel numbers), bloomery iron (VERIFIED four experiments: ratios in pack; ironsand is a flagged gap with failures documented), forge work (no sourced times). Each uses `materials.make_charcoal/calcine/craft` (ledger-closed) and a multi-day `ActiveTask` (kiln occupied, tending shifts).
- Keep owner-visible: any task with a source gap is shipped as `estimate` in `assumptions.json`.
Tests: `charcoal_yield_lies_in_the_packs_range`, `a_kiln_burn_takes_days_and_occupies_crew`, `smelt_outputs_stay_within_cleere_wareham_bounds`, `no_iron_without_charcoal_and_ore_and_a_furnace`, `ironsand_bloomery_requires_flagged_assumption` (documents the gap).

### 3.8 Step 8: skill and learning
- Add `skill_hours: BTreeMap<SkillKind, f64>` to the island human; duration multiplier `m(h) = m_expert + (m_novice - m_expert) * exp(-h / tau)` (exponential; Heathcote 2000 VERIFIED), tau chosen so expertise takes 3-7 years of practice (Wallis; Stout; both VERIFIED in part). `m_novice/m_expert` has NO source: ship as an owner-flagged assumption (no source for felling, thatching, bricklaying ratios was found).
- Starting skill for the founders is an owner decision (adults raised in modern NZ: no manual craft skill).
Tests: `skill_only_changes_duration_not_energy_per_hour`, `practice_monotonically_speeds_up`, `learning_saturates_at_expert`, determinism.

### 3.9 New reference-pack items needed (and the sources found above)
Add to `fixtures/reference/labour/productivity.json` / `humans/physiology.json`, each with `source`, `confidence`, quotes as `note`:
1. `met_by_activity` rows (humans/physiology.json): carry-by-load 11820/11830/11840/11850, water 11191/17076, firewood 08009/08010, splitting 08019/08020, felling 08057/08058, foraging 04150/04152, picking 08248, weeding 08241, planting 08145, cooking 05049/05050/05051, butchering 05044/05045, fishing 04001/04020/04062/04065, hunting 04080/04120/04081, roofing 06180, hammering 06124, digging 11790/11540. Source: Compendium 2024 (herrmann2024), URL https://pacompendium.com/. Check 2011 equivalence first (the pack cites ainsworth2011).
2. `load_fraction_limits`: 0.30 fighting / 0.45 approach march, source Vogel 1987 Army; free-carry 0.20 and 40% => +20% cost (Maloiy 1986); porter 0.8-2.0 as an upper bound documented only.
3. `water_access_and_quantity`: 7.5, 20, 50, 100 L/c/d and 30 min / 1 km thresholds (WHO 2003).
4. `air_drying_days`: Wood Handbook ch.13 and GTR-121 table (red oak by stacking month), 12-15% minimum, 20% firewood criterion.
5. `timber_per_house`: 14 m3 NZ (van Houtte & Parker 2019); 35 m3 DERIVED US; `sawmill_recovery`: 0.32 mean / 0.60 benchmark.
6. `thatch_straw_per_m2`: 27 kg/m2 applied (DERIVED from 150 bundles/tonne/37 m2) and 34 kg/m2 (Wikipedia).
7. `pit_saw_output`, `hewing_time` (half an hour per 15-ft face), `green_log_density` (40-70 lb/cu ft) from Fine Woodworking.
8. `charcoal_mound_cycle`: 9-22 days, crews 4-8, yields 12-27%.
9. `bloomery_experiments`: Cleere/Wareham/Wealden rows with charcoal-per-bloom and time.
10. `pottery_firing_fuel`: 0.5-1 m3 per firing, temperatures and hours (Ther 2004).
11. `kumara_yield_and_calendar`: 7.5-10.15 t/ha, plant mid-Oct, harvest mid-Apr to mid-May, 113.8 kcal/100 g, loss 10%, seed 1.10 t/ha, fallow 14 y (Leach 2024).
12. `wood_fuel_energy`: 16.2 MJ/kg (Wikipedia citing WA SEDO; prefer a primary source), household use 0.22 m3/person/yr (Uganda) and 6.7 kg/day household (Kenya).
13. `forage_return_rates`: only VERIFIED: Hadza/Mikea tuber rates and acorn gross (no all-in figure).
14. `learning_curve_shape`: Heathcote 2000 (exponential) and Stout 2005 (novice plateau); parameters flagged assumption.
Sources still to obtain before the matching step ships (all currently gaps): Kelly 1995 / Binford foraging table; grinding rates; fishing/hunting/butchering times; crop days-to-maturity and seed rates; clay pot volumes; thatcher m2/day; person-days for huts/cabins; Maori whare time (Te Ara, NZETC pages did not load); Mathieu & Meyer 1997 numbers; cordage m/h; Blackden & Wodon / Sorenson fuel and water time use; Knapik 2004 full text.

### 3.10 Which tests prove it overall
- `crates/mk_engine/tests/island_labour.rs` (extend): every action's `TaskSpec` equals pack-derived duration; actions take > 1 tick.
- New `island_work_chains.rs`: a founder day-in-the-life across 7 simulated days with the time budget summing to <= 24 h per person, and an assertion per chain (water, food, fire/cook, wood).
- Conservation: existing `island_material_flows.rs` plus the flux-ledger audit inside `life.audited` stays closed after each new task (`audits_closed`).
- Determinism and replay: `island_replay.rs`, `island_snapshot.rs` (mid-task snapshot restore, version bump) and the state digest including the new task queues and skill hours.
- Realism bounds in `island_life_acceptance.rs` (body carbon 0.98-1.02x; eating time within factor 2 of ATUS 64.5 min; drinking within 0.5-1.5x IOM share): they must still hold with trips added, which is the check that walking and carrying costs do not starve the founders. Expect to retune MET baselines (D22 floor 1.5 MET) once carrying is costed; keep D22's reasoning.
- A "no free lunch" test per action: output mass <= input mass + ledger-sourced mass; no structure exists whose `progress < 1` gives shelter; no `Build` succeeds without delivered materials; no `Gather` succeeds without the required tool item; no load exceeds `max_load_kg`.
- Doc updates: rewrite D9 and D40 rows, and `WORK_REMAINING.md` 2c "Done when: every action a founder can choose takes a sourced time and MET" becomes the acceptance list above.

### 3.11 Decisions the owner must make (listed so the work is not blocked silently)
1. Do founders start with matches/lighter, stove, jerrycans, sacks, power tools (estate items) or must they make them? (Items in the estate property list already, check `property.rs`.)
2. Founders' starting skill and the novice/expert ratio (no source exists).
3. Is the existing estate house/cottages "pre-built" (accepted scenario start, like D15) while all later buildings must be built? This should be recorded as a deviation so it is not mistaken for realism.
4. Upstream `resource_economy` path: keep for non-island worlds, but on the island either delete its action handlers for humans or make them call the new task system, to avoid two truths.
5. Hand-tool factor for building (x2 in the pack note is unsourced).

---------------------------------------------------------------------------
## 4. Honest limits of this research
- Firecrawl credits were exhausted; WebSearch budget ended partway in parts B and D. Many classical sources (Kelly, Hawkes/O'Connell, Wright 1994 grinding, Blackden & Wodon, Mathieu & Meyer 1997, Knapik 2004 full text, Reynolds 1978) were not reachable.
- Compendium METs are VERIFIED on the 2024 site; the repo's pack cites the 2011 edition, equivalence unchecked.
- The brief's figures "13,000 board feet per 2,000 sq ft", "3-5 kg firewood per person per day", "FAO 1 m3 per person per year" were NOT found; verified data are lower (about 1-1.3 kg/person/day household firewood in Kenya/Uganda). Do not use those brief figures.
- Several VERIFIED rows are secondary (Wikipedia, trade sites, tourism page, hobbyist guide). Prefer to re-read primary sources when each goes into a pack, and mark `confidence: low` as the existing packs do.
