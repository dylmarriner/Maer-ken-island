# Part A research: MET codes, load carriage, learning curves

Tag key: VERIFIED = I read the page/abstract text containing the number (raw HTML via curl, Europe PMC abstract API, WHO PDF via curl, or WebFetch page text). UNVERIFIED = search snippet or memory only.
Firecrawl was out of credits, so only curl, WebFetch, WebSearch and Europe PMC were used.

## 1. Compendium of Physical Activities

Source for every row in this section: Compendium of Physical Activities, Adult Compendium (the 2024 update; Herrmann et al. 2024, J Sport Health Sci 13(1)), category pages scraped from https://pacompendium.com/occupation, /home-activities, /home-repair, /lawn-garden, /fishing-hunting, /miscellaneous, /walking. Tag: VERIFIED (raw page text, code + MET + description parsed together).
Not verified: whether each code and MET is identical in the 2011 edition (Ainsworth et al. 2011, Med Sci Sports Exerc 43:1575). The 2011 Google Sites page returned no table. Treat 2011 equivalence as UNVERIFIED. Several 2024 codes are new (the 17xxx carrying and hauling codes, 08057/08058 felling, 04150 foraging).
Descriptions are quoted exactly as on the site. Double spaces in the source are collapsed.

### 1a. Woodwork, forestry, digging, building

| Code | MET | Exact description | Tag |
|---|---|---|---|
| 11038 | 2.5 | Carpentry, general, light effort | VERIFIED |
| 11040 | 4.3 | Carpentry, general, moderate effort | VERIFIED |
| 11042 | 7.0 | Carpentry, general, heavy or vigorous effort | VERIFIED |
| 06040 | 3.0 | Carpentry, general, workshop (Taylor Code 620) | VERIFIED |
| 06050 | 6.0 | Carpentry, outside house, (Taylor Code 640) | VERIFIED |
| 06052 | 3.8 | Carpentry, outside house, building a fence | VERIFIED |
| 06060 | 3.3 | Carpentry, finishing, varnishing, or refinishing cabinets or furniture | VERIFIED |
| 06070 | 6.0 | Carpentry, sawing hardwood, planing and drilling wood, moderate-to-vigorous effort | VERIFIED |
| 06072 | 4.0 | Carpentry, home remodeling tasks, moderate effort | VERIFIED |
| 06074 | 2.3 | Carpentry, home remodeling tasks, light effort | VERIFIED |
| 06124 | 3.0 | Hammering nails | VERIFIED |
| 06180 | 6.0 | Roofing | VERIFIED |
| 11264 | 5.0 | Forestry, moderate effort (e.g., sawing wood with power saw, weeding, hoeing) | VERIFIED |
| 11266 | 8.5 | Forestry, vigorous effort (e.g., barking, felling, or trimming trees, carrying or stacking logs, planting seeds, sawing lumber by hand ) | VERIFIED |
| 11260 | 5.0 | Forestry, ax chopping, slow, 1.25 kg axe, 19 blows/min, moderate effort | VERIFIED |
| 11262 | 8.0 | Forestry, ax chopping, fast, 1.25 kg axe, 35 blows/min, vigorous effort | VERIFIED |
| 11250 | 17.5 | Forestry, ax chopping, very fast, 1.25 kg axe, 51 blows/min, extremely vigorous effort | VERIFIED |
| 08019 | 4.5 | Chopping wood, splitting logs, moderate effort | VERIFIED |
| 08020 | 6.5 | Chopping wood, splitting logs, vigorous effort | VERIFIED |
| 08057 | 8.3 | Felling trees, large size | VERIFIED |
| 08058 | 5.3 | Felling trees, small-medium size | VERIFIED |
| 08009 | 4.1 | Carrying, loading or stacking wood, loading/unloading or carrying lumber, light-to- moderate effort | VERIFIED |
| 08010 | 5.5 | Carrying, loading or stacking wood, loading/unloading or carrying lumber, moderate effort | VERIFIED |
| 08030 | 6.3 | Clearing land, haul brush, undergrowth, or ground, vigorous effort | VERIFIED |
| 08025 | 3.5 | Clearing light brush, thinning garden, moderate effort | VERIFIED |
| 11540 | 7.3 | Shoveling, digging ditches | VERIFIED |
| 11550 | 8.8 | Shoveling, more than 16 lbs/minute, deep digging, vigorous effort | VERIFIED |
| 11560 | 5.0 | Shoveling, less than 10 lbs/minute, moderate effort | VERIFIED |
| 11570 | 6.5 | Shoveling, 10 to 15 lbs/minute, vigorous effort | VERIFIED |
| 11790 | 7.8 | Using heavy tools (not power) such as shovel, pick, tunnel bar, spade | VERIFIED |
| 08050 | 5.0 | Digging, spading, filling garden, composting (Taylor Code 590) | VERIFIED |
| 08052 | 7.3 | Digging, spading, filling garden, composting, vigorous effort | VERIFIED |
| 08045 | 3.5 | Digging, spading, filling garden, composting, light-to-moderate effort | VERIFIED |
| 08192 | 5.5 | Shoveling dirt or mud | VERIFIED |
| 06210 | 5.0 | Spreading dirt with a shovel | VERIFIED |
| 11892 | 6.0 | Military activities, arterial field preparation, digging defensive positions | VERIFIED |
| 11480 | 4.3 | Masonry, concrete, moderate effort | VERIFIED |
| 11482 | 2.5 | Masonry, concrete, light effort | VERIFIED |
| 11630 | 4.5 | Standing, moderate/heavy tasks (e.g., lifting more than 50 lbs, masonry, painting, paper hanging) | VERIFIED |
| 11110 | 6.3 | Coal mining, shoveling coal, by hand | VERIFIED |
| 11080 | 5.3 | Coal mining, drilling coal, rock | VERIFIED |
| 11100 | 5.5 | Coal mining, general | VERIFIED |
| 11090 | 5.0 | Coal mining, erecting supports | VERIFIED |

### 1b. Carrying loads and hauling

| Code | MET | Exact description | Tag |
|---|---|---|---|
| 17016 | 4.0 | Carrying 5 to 14 lb (2.3 to 6.4 kg) load (e.g. suitcase, boxes, groceries), level ground, moderate pace | VERIFIED |
| 17018 | 4.5 | Carrying 15 – 155 lb (6.8 – 70.4 kg) load (e.g. suitcase, boxes, furniture), level ground or downstairs, slow pace | VERIFIED |
| 17019 | 6.5 | Carrying 50 to 150 pound load (e.g., equine or bovine feed, fence pipes, furniture), level ground, moderate pace | VERIFIED |
| 17026 | 5.5 | Carrying load, 1 to 15 lb load, upstairs | VERIFIED |
| 17027 | 6.0 | Carrying load, 16 to 24 lb load, upstairs | VERIFIED |
| 17028 | 8.0 | Carrying load, 25 to 49 lb load, upstairs | VERIFIED |
| 17029 | 10.0 | Carrying load, 50 to 74 lb load, upstairs | VERIFIED |
| 17030 | 12.0 | Carrying load, >74 lb load, upstairs | VERIFIED |
| 17025 | 8.3 | Carrying load upstairs, general | VERIFIED |
| 11050 | 8.0 | Carrying heavy loads (e.g., bricks, tools) | VERIFIED |
| 11060 | 8.0 | Carrying moderate loads up stairs, moving boxes, 25-49 lbs | VERIFIED |
| 11490 | 7.5 | Moving, carrying, or pushing heavy objects, 75 lbs or more, only active time (e.g., desks, moving van work) | VERIFIED |
| 11795 | 3.5 | Walking on job, 2.5 mph, slow speed, carrying light objects less than 25 lbs | VERIFIED |
| 11797 | 3.8 | Walking, 2.5 mph, slow speed, carrying heavy objects more than 25 lbs | VERIFIED |
| 11820 | 4.0 | Walking or walk downstairs or standing, carrying objects about 25 to 49 lbs | VERIFIED |
| 11830 | 5.5 | Walking or walk downstairs or standing, carrying objects about 50 to 74 lbs | VERIFIED |
| 11840 | 7.0 | Walking or walk downstairs or standing, carrying objects about 75 to 99 lbs | VERIFIED |
| 11850 | 7.3 | Walking or walk downstairs or standing, carrying objects about 100 lbs or more | VERIFIED |
| 05120 | 5.8 | Moving furniture, household items, carrying boxes | VERIFIED |
| 05150 | 9.0 | Moving household items upstairs, carrying boxes or furniture | VERIFIED |
| 11766 | 6.5 | Truck driving, loading and unloading truck, tying down load, standing, walking and carrying heavy loads | VERIFIED |
| 08255 | 4.8 | Wheel barrow, pushing garden cart or wheelbarrow, general | VERIFIED |
| 08256 | 6.0 | Wheel barrow, pushing large garden cart or heavy wheelbarrow up to 150kg | VERIFIED |
| 11191 | 4.3 | Farming, hauling water for animals, fetching water from a well or stream | VERIFIED |
| 17076 | 4.5 | Hauling water, head hauling, walking on flat surface | VERIFIED |
| 05110 | 5.0 | Maple syruping/sugar bushing (including carrying buckets, carrying wood) | VERIFIED |
| 11882 | 4.0 | Soldiers, military marching, 1.5 to 2.5 mph, 10 to 30 kg load | VERIFIED |
| 11884 | 5.0 | Airborne Shuffle, 2.5 to 3.5 mph, 20-30 kg load | VERIFIED |
| 11886 | 6.3 | Soldiers, military loaded marching, varying terrain, 25-40 kg load | VERIFIED |
| 11887 | 6.0 | Soldiers, walking, 2.8 mph, 5% grade, up to 21.5 kg load | VERIFIED |
| 11888 | 8.5 | Soldiers, walking, 2.8 mph, 10% grade, up to 21.5 kg load | VERIFIED |
| 11889 | 11.0 | Soldiers, walking, 2.8 mph, 15% grade, up to 21.5 kg load | VERIFIED |
| 17434 | 4.8 | Walking, treadmill, 2.5 mph, 0% grade, 5 to 20 degrees C, 40 lb (18.2 kg) load | VERIFIED |
| 17060 | 10.0 | Climbing hills, 20+ pound load, 5 to 20% grade, moderate to brisk pace | VERIFIED |
| 04150 | 2.8 | Foraging, 0-9.9kg backpack | VERIFIED |
| 04152 | 3.0 | Foraging, 10-15kg backpack | VERIFIED |

### 1c. Farming, garden, fishing, hunting, food, craft

| Code | MET | Exact description | Tag |
|---|---|---|---|
| 08241 | 5.0 | Weeding, cultivating garden, using hoe moderate-to-vigorous effort | VERIFIED |
| 08240 | 4.5 | Weeding, cultivating garden, moderate effort (Taylor Code 580) | VERIFIED |
| 08239 | 3.8 | Weeding, cultivating garden, light-to-moderate effort | VERIFIED |
| 08145 | 4.3 | Planting crops or garden, stooping, moderate effort | VERIFIED |
| 08140 | 4.3 | Planting seedlings, shrubs, stooping, moderate effort | VERIFIED |
| 08150 | 4.5 | Planting trees | VERIFIED |
| 08160 | 4.0 | Raking lawn or leaves, moderate effort | VERIFIED |
| 08246 | 3.5 | Picking fruit or coffee from trees, moderate effort | VERIFIED |
| 08248 | 4.5 | Harvesting Produce, Picking fruit off trees, gleaning fruits, picking fruits/vegetables, climbing ladder to pick fruit, vigorous effort | VERIFIED |
| 11510 | 3.5 | Orange grove work, picking fruit | VERIFIED |
| 11146 | 4.8 | Farming, moderate effort (e.g., feeding animals, chasing cattle by walking and/or horseback, spreading manure, harvesting crops) | VERIFIED |
| 11145 | 7.8 | Farming, vigorous effort (e.g., baling hay, cleaning barn), includes former code 11200 | VERIFIED |
| 11195 | 3.8 | Farming, rice, planting, grain milling activities | VERIFIED |
| 11210 | 3.5 | Farming, milking by hand, cleaning pails, moderate effort | VERIFIED |
| 11192 | 4.5 | Farming, taking care of animals (e.g., grooming, brushing, shearing sheep, assisting with birthing, medical care, branding), general | VERIFIED |
| 05044 | 3.0 | Butchering animals, small | VERIFIED |
| 05045 | 6.0 | Butchering animals, large, vigorous effort | VERIFIED |
| 05046 | 2.3 | Cutting and smoking fish, drying fish or meat | VERIFIED |
| 04001 | 3.5 | Fishing, general | VERIFIED |
| 04030 | 2.0 | Fishing from boat or canoe, sitting | VERIFIED |
| 04020 | 4.0 | Fishing from river bank and walking | VERIFIED |
| 04040 | 3.5 | Fishing from river bank, standing (Taylor Code 660) | VERIFIED |
| 04062 | 3.5 | Fishing, dip net, setting net and retrieving fish, general | VERIFIED |
| 04063 | 3.8 | Fishing, set net, setting net and retrieving fish, general | VERIFIED |
| 04065 | 2.3 | Fishing with a spear, standing | VERIFIED |
| 04007 | 4.0 | Fishing, catching fish with hands | VERIFIED |
| 11247 / 11248 | 3.5 / 5.0 | Fishing, commercial, light effort / moderate effort | VERIFIED |
| 04100 | 5.0 | Hunting, general | VERIFIED |
| 04080 | 6.0 | Hunting, deer, elk, large game (Taylor Code 170) | VERIFIED |
| 04081 | 9.8 | Hunting large game, dragging carcass | VERIFIED |
| 04120 | 5.0 | Hunting, rabbit, squirrel, prairie chick, raccoon, small game (Taylor Code 690) | VERIFIED |
| 04070 | 2.5 | Hunting, bow and arrow or crossbow | VERIFIED |
| 04124 | 2.0 | Trapping game, general | VERIFIED |
| 05049 | 3.5 | Cooking or food preparation, moderate effort | VERIFIED |
| 05050 | 2.0 | Cooking or food preparation – standing or sitting or in general (not broken into stand/walk components), manual appliances, light effort | VERIFIED |
| 05051 | 2.3 | Cooking or food preparation, walking | VERIFIED |
| 05035 | 3.3 | Kitchen activity, general, (e.g., cooking, washing dishes, cleaning up) moderate effort | VERIFIED |
| 05041 | 2.0 | Wash dishes, standing or in general (not broken into stand/walk components) | VERIFIED |
| 05149 | 2.5 | Building a fire inside | VERIFIED |
| 05057 | 3.0 | Cooking Indian bread or other food on an outside stove | VERIFIED |
| 05090 | 2.3 | Laundry, fold or hang clothes, put clothes in washer or dryer, packing suitcase, washing clothes by hand, implied standing, light effort | VERIFIED |
| 11740 | 1.8 | Tailoring, hand sewing | VERIFIED |
| 05080 | 1.3 | Knitting, sewing, light effort, wrapping presents, sitting | VERIFIED |
| 11765 | 4.0 | Tailoring, weaving, moderate effort (e.g., spinning and weaving operations, delivering boxes of yarn to spinners, loading of warp beam, pinwinding, conewinding, warping, cloth clutting) | VERIFIED |
| 09075 | 1.8 | Sitting: arts and crafts, carving wood, weaving, spinning wool, light effort | VERIFIED |
| 09080 | 3.0 | Sitting: arts and crafts, carving wood, weaving, spinning wool, moderate effort | VERIFIED |
| 09090 | 3.3 | Standing – arts and crafts, sand painting, carving, weaving, moderate effort | VERIFIED |
| 09095 | 3.5 | Standing – arts and crafts, sand painting, carving, weaving, vigorous effort | VERIFIED |

### 1d. Gaps (no dedicated Compendium entry found; searched all 8 category pages)

| Wanted | Finding | Nearest proxy (code, MET) | Tag |
|---|---|---|---|
| Bricklaying | No brick-laying entry | 11480 masonry concrete moderate 4.3; 11630 standing moderate/heavy tasks incl. masonry 4.5; 11050 carrying heavy loads (bricks) 8.0 | VERIFIED (absence on parsed pages) |
| Pottery / clay | none | 09090 standing arts and crafts moderate 3.3 | VERIFIED (absence) |
| Blacksmith / forge | none | 11430 Machine tooling (...welding) light-to-moderate effort 3.0; 11493 mining support/welding 8.5 | VERIFIED (absence) |
| Thatching | none | 06180 Roofing 6.0 | VERIFIED (absence) |
| Rope / basket making | none | 11620 Standing, moderate effort, intermittent lifting 50 lbs, hitch or twisting ropes 3.8 (rope only); 09075-09095 crafts | VERIFIED (absence) |
| Collecting firewood | no exact entry | 08010 carrying, loading or stacking wood 5.5; 08030 haul brush 6.3 | VERIFIED |
| Hand sawing softwood | no separate entry | 06070 sawing hardwood 6.0; 11266 sawing lumber by hand 8.5 | VERIFIED |
| Building outdoor fire | none | 05149 building a fire inside 2.5 | VERIFIED |
| Yoke / two-bucket carrying | none | 17076 head hauling 4.5; 11191 hauling water 4.3 | VERIFIED |

## 2. Human load carriage and water

### 2a. Load fractions and energy cost

| Claim | Exact quote (short) | Value | Source | URL | Tag |
|---|---|---|---|---|---|
| Fighting load limit | "30 percent (about 48 pounds) as a tactical load in a combat zone." | 30% body weight, about 48 lb | Vogel et al., "Soldier Load; When Technology Fails", Army, 1987 (via WebFetch of regimentalrogue.com reprint) | https://regimentalrogue.com/soldiers_load/1987_Soldier_Load_When_Technology_Fails.htm | VERIFIED |
| Approach-march load limit | "45 percent of a soldier's body weight on approach marches (for the average soldier. about 72 pounds)" | 45% body weight, about 72 lb | same | same | VERIFIED |
| Emergency load | Article says emergency loads "may, on occasion, exceed by a large margin the desirable goal of 72 pounds"; cites 154 lb carried 20 km in an Infantry Board test | no percentage given; 154 lb (about 70 kg) | same | same | VERIFIED |
| Knapik 2004 recommends 30%/45%? | The abstract does NOT contain the 30/45% fractions. They come from US Army FM 21-18 / MIL-STD-1472F, which a WebSearch summary attributes to the Knapik-era literature (22 kg = 30% for fighting load, 33 kg = 45% for approach march, "75% of body weight" for emergency in a USMC report, authors' own calculation) | 30% / 45% / about 75% (emergency) | WebSearch summary of DTIC ADA212050 and a 2003 USMC combat load report; not read directly | https://apps.dtic.mil/sti/pdfs/ADA212050.pdf | UNVERIFIED (DTIC returned 403 to me) |
| Historical march loads | "Before the 18th century, foot soldiers seldom carried more than 15 kg while on the march" | 15 kg | Knapik, Reynolds, Harman, Mil Med 2004;169(1):45-56 (abstract via Europe PMC) | https://pubmed.ncbi.nlm.nih.gov/14964502/ | VERIFIED |
| Load position cost | "each kilogram added to the foot increases energy expenditure 7% to 10%; each kilogram added to the thigh increases energy expenditure 4%" | +7-10% per kg on foot; +4% per kg on thigh | same | same | VERIFIED |
| Load cost model status | "The energy cost of walking with backpack loads increases progressively with increases in weight carried, body mass, walking speed, or grade... Predictive equations have been developed, but these may not be accurate for prolonged load carriage." | qualitative | same | same | VERIFIED |
| Pandolf equation | Pandolf, Givoni, Goldman 1977, J Appl Physiol 43(4):577-581 exists (title: "Predicting energy expenditure with loads while standing or walking very slowly"). The equation form M = 1.5W + 2.0(W+L)(L/W)^2 + n(W+L)(1.5V^2 + 0.35VG) is from my memory, not read | n/a | Europe PMC record only (no abstract text) | https://pubmed.ncbi.nlm.nih.gov/908672/ | citation VERIFIED, equation UNVERIFIED |
| Equation accuracy | "The PAN equation accurately predicted metabolic cost for 40 and 50 kg at 4.8 km/h... Most equations (GG, SAN, ACSM, MMM) underpredicted" | 25-70 kg loads, 2.5 and 4.8 km/h tested on 39 infantry soldiers; Pandolf best at 40-50 kg at 4.8 km/h | Vine et al. 2022, J Strength Cond Res 36(5):1297-1303 | https://pubmed.ncbi.nlm.nih.gov/32398631/ | VERIFIED |
| Soldier marching MET | "Soldiers, military marching, 1.5 to 2.5 mph, 10 to 30 kg load" | 4.0 MET; 25-40 kg varied terrain 6.3 MET; 2.8 mph 10% grade up to 21.5 kg 8.5 MET | Compendium (see 1b) | https://pacompendium.com/walking | VERIFIED |
| Walking-with-backpack speed | no source read giving speed vs load directly | n/a | n/a | n/a | gap |

### 2b. Porters, head carrying, yokes

| Claim | Exact quote (short) | Value | Source | URL | Tag |
|---|---|---|---|---|---|
| Nepalese porter loads | "carry head-supported loads equal to 100 to 200% of their body weight (Mb) for many days up and down steep mountain footpaths at high altitudes" | 100-200% body mass | Bastien, Schepens, Willems, Heglund, "Energetics of load carrying in Nepalese porters", Science 2005;308:1755 (abstract via Europe PMC) | https://doi.org/10.1126/science.1111513 | VERIFIED |
| Porters vs African women | "Female Nepalese porters... carry on average loads that are 10% of their Mb heavier than the maximum loads carried by the African women, yet do so at a 25% smaller metabolic cost." | +10% Mb load, -25% metabolic cost | same | same | VERIFIED |
| African women vs recruits | "African women carry head-supported loads of up to 60% of their Mb far more economically than army recruits carrying equivalent loads in backpacks" | 60% Mb | same | same | VERIFIED |
| Porter loads on trail | porters "about 80-90% of their body mass, up to 200%"; trekking test "about 45 kg (83% of body mass)"; "No mountaineers and only a few porters were able to carry 60 kg loads" | 80-90% typical; 45 kg trek; lab loads up to 90% BM | Minetti, Formenti, Ardigo 2006, Proc R Soc B (PMC1635507; fetched text) | https://pmc.ncbi.nlm.nih.gov/articles/PMC1635507 | VERIFIED |
| Porter vertical cost | "C_vert being 20.7% lower"; loaded walking cost "-20%" | -20 to -21% vs mountaineers | same | same | VERIFIED |
| Luo women load | "women of the Luo tribe carrying loads equivalent to 70% of their body mass balanced on the top of their heads" | 70% body mass | Maloiy, Heglund, Prager, Cavagna, Taylor, Nature 1986;319:668-669 (abstract via Europe PMC) | https://doi.org/10.1038/319668a0 | VERIFIED |
| Free carrying threshold | "could carry loads of up to 20% of their body weight without increasing their rate of energy consumption. For heavier loads there was a proportional increase... a 30% load increased energy consumption by 10%, a 40% load by 20%" | 0% extra cost up to 20% BM; +10% at 30%; +20% at 40% | same | same | VERIFIED |
| Baseline cost of load | "carrying a load equal to 20% of body weight increased the rate of energy consumption by 20%" (for horses, humans, dogs, rats) | cost proportional to load | same | same | VERIFIED |
| Head-carry mechanism | "weight-specific mechanical work... decreases with load in the African women, whereas it increases in control subjects" via better pendulum-like energy transfer | qualitative | Heglund, Willems, Penta, Cavagna, Nature 1995;375:52-54 | https://doi.org/10.1038/375052a0 | VERIFIED |
| Head hauling MET | "Hauling water, head hauling, walking on flat surface" | 4.5 MET | Compendium 17076 | https://pacompendium.com/walking | VERIFIED |
| Yoke / two-bucket | no figure retrieved | n/a | n/a | n/a | gap |

### 2c. Water volumes, distances, carrying burden

| Claim | Exact quote (short) | Value | Source | URL | Tag |
|---|---|---|---|---|---|
| WHO minimum for consumption + hygiene | "a minimum of 7.5 litres per capita per day will meet the requirements of most people under most conditions" | 7.5 L/person/day | Howard & Bartram, "Domestic Water Quantity, Service Level and Health", WHO/SDE/WSH/03.02, 2003 (read from PDF copy) | https://www.arctic.gov/uploads/assets/watersan6-water-quantity.pdf | VERIFIED |
| Basic access | "Basic access (average quantity unlikely to exceed 20 l/c/d)": "Between 100 and 1000m or 5 to 30 minutes total collection time" | up to 20 L/person/day at 100-1000 m | same, Table S1 | same | VERIFIED |
| No access | "No access (quantity collected often below 5 l/c/d)": "More than 1000m or 30 minutes total collection time" | below 5 L/person/day, over 1 km | same | same | VERIFIED |
| Intermediate access | "average quantity about 50 l/c/d"; tap on-plot or within 100 m or 5 minutes | 50 L/person/day | same | same | VERIFIED |
| Optimal access | "average quantity 100 l/c/d and above" | 100+ L/person/day | same | same | VERIFIED |
| JMP definition | "reasonable access... the availability of at least 20 litres per person per day from a source within one kilometre of the users dwelling" (WHO and UNICEF, 2000) | 20 L within 1 km | same, citing JMP 2000 | same | VERIFIED |
| Sphere | "the SPHERE project sets out 15 litres of water used per capita per day as being a key indicator in meeting minimum standards for disaster relief (SPHERE, 1998)" | 15 L/person/day (1998 edition) | same | same | VERIFIED (via WHO text citing Sphere); current Sphere handbook not read (403) |
| Drinking and food hydration | "a minimum quantity... for hydration (via both direct consumption and food) can be estimated as 5.5 litres per capita per day" | 5.5 L; up to about 4.5 to 6 L/day adults in heat | same | same | VERIFIED |
| Container weight and pain burden | Geere et al.: water carriers have "upper back (RR 2.16...)" and "hand pain (RR 3.11...)" risks | RR 2.16 upper back, 3.11 hand (current carriers) | Geere et al. 2018, J Glob Health 8:010406 (abstract via Europe PMC) | https://doi.org/10.7189/jogh.08.010406 | VERIFIED |
| Average container weight | a search-result summary said "average container weighed 19.5 kg" and that most carrying was women/children on the head. I could not find it in the fetched text. Table 5 gives 260 of 474 current carriers as head loaders (about 55%, WebFetch summary) | 19.5 kg; 55% head loaders | Geere et al. 2018 | https://jogha.org/documents/issue201801/jogh-08-010406.htm | 19.5 kg UNVERIFIED; 260/474 VERIFIED (WebFetch) |
| Jerrycan 20 L / women carry 15-20 kg; women fetch share | not retrieved | n/a | n/a | n/a | gap |

Worked implication (my arithmetic, not sourced): 20 L is about 20 kg of water. At 20 L per person per day and 1 km one way, a family of 5 needs 100 L = 5 jerrycan trips of 2 km round trip.

## 3. Learning, practice and skill

| Claim | Exact quote (short) | Value | Source | URL | Tag |
|---|---|---|---|---|---|
| Wright 1936 learning rate | "Wright found that phi approx 80 % in aircraft manufacturing"; "the unit cost decreases by 20% for every doubling of total units made" | 80% progress ratio (aircraft) | Wikipedia "Learning curve" (WebFetch), which cites Wright 1936, J Aeronautical Sciences 3(4):122-128. The page frames this as unit cost; other secondary sources frame it as cumulative average labour, so the framing is uncertain | https://en.wikipedia.org/wiki/Learning_curve | VERIFIED (secondary source) |
| Wright model form | "y = Kx^n... n = log(phi)/log(2)" | exponent, 80% gives n about -0.322 | same | same | VERIFIED (form); -0.322 from a Utrecht chapter snippet, UNVERIFIED |
| Experience-curve range | BCG 1970s experience curve effects "ranged from 10 to 25 percent" per doubling | 75-90% progress ratio (conversion is inference) | same | same | VERIFIED (range), conversion is my inference |
| Typical "80-90%" for construction | no source found stating this for manual construction | n/a | n/a | n/a | UNVERIFIED |
| Construction learning curves are unreliable | Thomas, Mathews, Ward 1986 J Constr Eng Manage 112(2):245-258: tested five models on 65 data sets, "best predictor is a cubic model"; straight-line "is not a constant value" | learning rate varies | WebSearch summary of TRID record | https://trid.trb.org/View/276645 | UNVERIFIED (snippet) |
| Blockwork: little learning effect | "the patterns emerged from this study provide little evidence for that" (52 multi-storey residential buildings, concrete blockwork) | no fixed % per floor cycle | Emerald, IJPPM 2016, "Learning effect on labour productivity of repetitive concrete masonry blockwork: Fact or fable?" | https://www.emerald.com/insight/content/doi/10.1108/IJPPM-08-2015-0119/full/html | VERIFIED (abstract) |
| Power law challenged | "The exponential function fit better than the power function in all the unaveraged data sets" (40 data sets, 7,910 learning series, 475 subjects, 24 experiments) | exponential beats power law for individuals | Heathcote, Brown, Mewhort 2000, Psychon Bull Rev 7:185-207 | https://pubmed.ncbi.nlm.nih.gov/10909131/ | VERIFIED |
| Newell & Rosenbloom 1981 power law | book chapter in Anderson (ed.), Cognitive Skills and Their Acquisition; no text read | n/a | n/a | n/a | UNVERIFIED (not read) |
| Crossman 1959 | cigar-making speed-skill power law; searches returned nothing on it | n/a | Ergonomics 2(2):153-166 | n/a | UNVERIFIED (memory) |
| Deliberate practice limit | "deliberate practice explained 26% of the variance in performance for games, 21% for music, 18% for sports, 4% for education, and less than 1% for professions" | caveat to "10,000 hours" | Macnamara, Hambrick, Oswald 2014, Psychol Sci 25:1608-1618 | https://pubmed.ncbi.nlm.nih.gov/24986855/ | VERIFIED |
| Flint knapping, novice | "4 hours of practice did not result in any increase in the overall productivity or reduction intensity of novices and was actually associated with a decrease in the size of flakes"; experts "produced flakes that were not only more numerous, but also larger and more elongated" | 4 h practice: no gain; 40 min sessions | Stout et al., ch. 12 "Knapping skill: an experimental..." in Schick & Toth (eds), The Cutting Edge, Stone Age Institute 2005 (read PDF text) | https://www.stoneageinstitute.org/pdfs/cutting-edge-ch12-stout.pdf | VERIFIED |
| Knapping stages | Table: pre-practice "0 - 40 minutes", post-practice novice "280 - 320 minutes", expert "multiple years" | 3 stages; expert = multiple years | same | same | VERIFIED |
| Novice flake efficiency | "novice knapping only produced 0.38 usable flakes per strike"; expert number not seen | 0.38 usable flakes/strike (novice) | Putt 2015 conference abstract (via WebSearch summary) | https://paleoanthro.net/media/meetings/files/Putt_2015.pdf | UNVERIFIED (snippet) |
| Machine felling learning | beginner harvester operators went "from about 27.6 to 43.8 m3 per productive machine hour" over the first year; learning ended after "9 and 6.5 months"; productivity rose "218% and 104%" | about 1.6x in a year (43.8/27.6); learning curve ends 6.5-9 months | South African harvester study (WebSearch summary; paper not identified) | n/a | UNVERIFIED (snippet); machine, not manual |
| Chainsaw crews | "newer crews spent 32% higher time on preparation before tree felling"; forgetting factor "9.4%" experienced vs "19%" start-up crew | 32%; 9.4% vs 19% | Tanzania chainsaw studies (WebSearch summary) | https://repository.costech.or.tz/items/4a9df25b-2e28-4706-9d4e-27e82e5c8724 | UNVERIFIED (snippet) |
| Thatcher apprenticeship | "can take around 2 years to complete" (Level 2); "five to seven years to master the craft" (Countryside Agency, 2000) | 2 years formal; 5-7 years mastery | National Careers Service / Farmers Weekly (WebSearch summary) | https://fwi.co.uk/news/thatching-now-far-from-dying-craft | UNVERIFIED (snippet) |
| Novice-to-expert output ratio in manual crafts | no source found giving a direct productivity ratio for thatching, felling by axe, bricklaying | n/a | n/a | n/a | gap |

## Caveats
- WebFetch answers come from a small model summarising the page, so quotes from it are as read but not raw. Compendium data, WHO text, Europe PMC abstracts and the Stout PDF were read raw.
- Knapik 2004 full text was not accessible (PubMed cookie wall, DTIC 403, OUP blocked). The abstract is verified. The 30%/45% fractions are verified only from the 1987 Army article, not from Knapik 2004.
- The Compendium has no entries for bricklaying, pottery, blacksmithing, thatching, basket or rope making, so use the proxies in 1d and flag them as estimates.
