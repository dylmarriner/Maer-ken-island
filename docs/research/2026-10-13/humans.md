# How far are the island's simulated humans from real humans?

Repo: /run/media/kubuntux/DEVELOPMENT/Maer-ken-island (branch fix/deployable-backend). Research date 2026-10-13. No repository files were edited.

All paths below are under `crates/mk_engine/src/` unless stated. "Dead" means the value is computed each tick but nothing that chooses an action, moves a body, or kills a person ever reads it (it only feeds another dead field, a text template, or the dashboard JSON in `io/human_storage.rs:711-714`).

---------------------------------------------------------------------------------------------------

## 0. Verdict in one paragraph

The body is the strong half: metabolism, hydration, sleep timing, growth, gestation, mortality and labour cost are sourced and tested against `fixtures/reference/humans/*.json` (`tests/human_realism.rs`). The mind is the weak half. About 30 "cognitive" modules (senses, attention, memory, learning, decision, creativity, social cognition, 31 emotions, culture, language, consciousness, brain regions...) are 0..1 scalars blended toward schema values with unsourced constants, and almost none of them can change what a person does. The actual action chooser (`humans/autonomy.rs:156 choose_action`) is an argmax over 18 hand-weighted scores that read only: needs (hunger/thirst/fatigue/glucose), six cell-level aggregate numbers (`AgentWorldObservation`), two genetic traits (extraversion, novelty_seek), dark-triad malice, a "willed urge" gate, libido, and the sleep state. There are no objects, places, people-as-individuals, skills, beliefs, plans, or remembered locations in that loop. Nothing about the mind carries a deviation row: DEVIATIONS.md D1-D41 contains no row for senses, emotion, memory, learning, skill, creativity or culture, so the register currently understates the gap (project rule section 4 says every known limit must be listed).

---------------------------------------------------------------------------------------------------

## 1. Inventory: what the code models today

### 1.1 What actually drives behaviour (the whole causal path)

Per tick: `humans/mod.rs:923 step_on` -> `HumanBeing::decide` (mod.rs:1734) -> `AutonomousMind::choose_action` (autonomy.rs:156) -> movement (mod.rs ~1000-1060, `directed_seek_step` autonomy.rs:330) -> `lifecycle::step_lifecycle` (lifecycle.rs:281) -> `resource_economy.apply_human_action`.

Inputs read by `choose_action` / `drive_for` (autonomy.rs:156-238, 467-583):
- `needs` (hunger, thirst, fatigue, glucose, hydration) and `observation.{caloric_access, hydration_access, shelter_quality, social_density, hazard_index, resource_abundance, computer_access}`.
- `genetics.extraversion`, `genetics.novelty_seek`.
- `core_systems.willpower_available` and `core_systems.chaotic_urges.{social, exploration, achievement, reproduction}` (core_systems.rs:344-413, a seeded noisy "urge" pipeline).
- `dark_triad.active_malice / vengeance_drive` (only for `Harm`).
- `reproduction.{libido, arousal, attraction_average, satisfaction}` (only for `Intimacy`).
- `neurochemistry.modulation_effects.current_brain_state == Fatigued` hard-forces `Rest` (autonomy.rs:168-184); that state is set by the circadian clock's two-process sleep gate (neurochemistry.rs:248-268, circadian.rs:268).
- A learned per-action preference (`learn_from_outcome`, autonomy.rs:239-290): a Rescorla-Wagner style prediction-error update on a fitness scalar `0.45 glucose + 0.35 hydration + 0.20 (1-fatigue)` (autonomy.rs:462). This is the only learning that changes behaviour.

`ActionKind` (agents/behavior/mod.rs:13-69) is 17 verbs plus `SendEmail`; actions have no target, no object and no payload. Movement is a one-cell step toward the best of 8 neighbouring cells using omniscient reads of the neighbours' aggregate access (`directed_seek_step`, autonomy.rs:330-378), or toward the nearest person (`step_toward_nearest`, mod.rs:1407). There is no memory of where water was.

### 1.2 Physiology

| Area | Where | Drives behaviour? | Realism status |
|---|---|---|---|
| Energy/metabolism, glycogen + fat stores, starvation | humans/needs.rs (consts lines 11-80), labour.rs, lifecycle.rs:646 `activity_met_for` | Yes (hunger -> SeekFood, death at glucose 0) | Sourced (Adolph 1947, Cahill 2006, Keys 1950, Mifflin-St Jeor, Compendium) and tested; D9, D22. Best-modelled area. Single "glucose" + "energy_reserve"; no protein/micronutrient/vitamin state, so scurvy-type deficiency disease cannot occur (see 3.7). |
| Hydration | needs.rs:11-52 | Yes (thirst -> SeekWater, death at 0) | Sourced (Adolph, IOM 2005, Hew-Butler 2015). Thirst drive is `1 - hydration`; real thirst is osmotic-threshold triggered (see 3.x). |
| Thermoregulation | body.rs:104-122, 222-237 | Partly: `body_temperature_c` only matters at the lethal ends (28 C / 42 C, body.rs:118-119, lifecycle.rs:~575). Nothing makes a cold person seek shelter earlier, shiver (energy cost), sweat (water cost beyond needs.rs sweat term), dress, or light a fire. | UNSOURCED constants: set point 37, thermoneutral 10-32 C ambient, 22 C sheltered, 6 h time constant, 0.15/0.12 C per C. Violates REALISM.md rule 1. Real thermoneutral zone for a naked adult is ~28-30 C, not 10-32; clothing absent. |
| Sleep | humans/circadian.rs (FJK 1999 pacemaker + Borbely two-process), needs.fatigue = process S | Yes (hard sleep gate, Rest) | Sourced, tested (human_realism.rs:199-381), D8/D23. No sleep stages, no arousal threshold, no dreaming, no nap/sleep-debt cost beyond fatigue. |
| Growth/ageing | growth.rs (WHO tables), development.rs, Siler mortality lifecycle.rs:714-768 | Age gates actions (`age_substitute` autonomy.rs:48); mortality kills | Sourced, tested (D21). Ageing of capacities is thin: `physical_capacity.rs`, `proprioception.rs` joint stiffness, `learning.current_plasticity`. |
| Illness | immune.rs (per-pathogen entities, spawn RNG), pathology.rs (psychiatric "attractor" risks) | `immune.system_stress` lowers `physical_capacity` and feeds decision/emotion; inflammation raises fever (body.rs:231) | Mechanism is generic; spawn rates, virulence, durations are unsourced; no transmission between people (pathogens spawn per human by RNG, immune.rs:186), no named diseases, no wounds-to-infection chain. |
| Injury | body.injury, set only by `apply_harm` (mod.rs:1353) | Caps `vital_energy` | There are no accidents, falls, burns, bites or fractures; the only way to be hurt is another human's `Harm`. Injury does not produce pain (see 1.3). Abstract "extrinsic mortality" (Siler term) stands in for all of it. |
| Reproduction | reproduction.rs (28-day cycle, 268 d gestation, Jukic 2013), lifecycle.rs:1112 conception, 1175 birth | Intimacy is a chosen action; births happen | Sourced, tested (gestation, menopause window). No pregnancy burden on energy/mobility is evident; no lactation (grep for breast/wean/lactat: none); no childbirth risk. |
| Infant/child care | regional/life.rs:71 `DEPENDENT_AGE_YEARS = 5.0`, `keep_children_with_mothers` (life.rs:1450) | Children <5 follow the mother | No feeding of dependants modelled in the human layer (no caregiver action, no breastfeeding); children <5 are inside household food books only. Not verified end-to-end through life.rs. |

### 1.3 Senses

All in `humans/sensory.rs` (310 lines). Every sense is a single 0..1 scalar seeded from the schema, then lerped toward `baseline * f(one world scalar)` (sensory.rs:155-215).

| Sense | What exists | Drives behaviour? | Departure from reality |
|---|---|---|---|
| Vision | `visual_acuity`, `color_perception`, `depth_perception`, `motion_detection`; step scales by `daylight_fraction` (0.5+0.5d, 0.2+0.8d, 0.7+0.3d) | NO. `visual_acuity` is read only by the JSON dump (io/human_storage.rs:712); colour/depth/motion read by nothing | No field of view, range, occlusion, dark adaptation (rods vs cones, 5-8 min break, ~40 min full), no line of sight to food/water/people. Nothing is ever "seen". The multipliers are invented. |
| Hearing | `hearing_sensitivity`, `sound_localization`, `speech_recognition`; masked by `social_density*0.3` | NO (only dumped) | No sound sources, thresholds, distance falloff, startle, or waking by noise. Dialogue requires adjacency (mod.rs step_dialogue, D34) rather than audibility. |
| Smell, taste | NONE in the engine. `olfactory` / `gustatory` exist only in schema sensitivity/hallucination structs (mk_core/src/human/schema.rs:182-183, 307) and are never read | n/a | Entirely absent: no food-quality sensing, spoilage, toxins, bitter/sour avoidance, smoke, mate/kin odour. |
| Touch / pain | `overall_pain_level` = 0.2(1-hygiene) + 0.3 fatigue + 0.2(1-hydration) (sensory.rs:158-161); `tactile_event_count` stuck at schema value | Weakly: pain feeds decision.rs:100, emotion.rs:323, attractor_control.rs:203, pathology.rs:350 (all dead ends except via emotion->dark_triad) | Pain is not caused by injury, burns, cold or pressure. `body.injury` never enters it. No nociceptor threshold, no pain-driven withdrawal. |
| Temperature sense | none; `body.body_temperature_c` and `skin` snapshot only | No behavioural effect before lethality | No warm/cold discomfort driving shelter/clothing/fire. |
| Proprioception / balance | proprioception.rs (12 joints flexibility/stress, no pose), `sensory.balance/coordination/balance_sensitivity/spatial_orientation/motion_sickness` | `physical_capacity` multiplies resource gain from needs (physical_capacity.rs) | Joint stiffness is a real function of age/fatigue/inflammation but there is no gait, no falls, no stumbling, no slip on slopes beyond Naismith cost in labour.rs. Vestibular fields are dead. |
| Interoception (hunger/thirst/fatigue) | needs vector IS the interoceptive signal and is read directly and exactly by the chooser; `body_awareness`, `internal_state_monitoring` are dead | Yes, but perfectly and noiselessly | Real interoception is noisy, delayed and individually variable (Garfinkel 2015), and hunger/thirst signals are hormonal and learned. |

### 1.4 Perception and attention

- Perception interface: `agents/mod.rs:353 AgentWorldObservation` has 6 aggregate floats per cell (hydration_access, caloric_access, shelter_quality, social_density, hazard_index, resource_abundance) plus temperature, daylight, lamp lux, indoors flag, computer flags. Built from a cell's facts in `humans/observation.rs` and `regional/humans.rs`. A person perceives the average of a 2 km (or patch) cell, not things. The doc `HUMAN_SCOPE.md` admits "summary-level tactile/multimodal perception".
- Attention: attention.rs:55 `step` blends `attention_fatigue` toward `needs.fatigue` and derives `available`, `focus_level`, `cognitive_load`. It does not select anything: no object of attention, no capacity limit on what is processed, no salience, no orienting, no distraction by events. Consumers: memory encoding, social_cognition, creativity, core_systems' `has_attention` gate for willpower (core_systems.rs:400), emotion curiosity/boredom. The `flow` field is never stepped (it keeps its schema value).
- `cognition.rs` (`HumanCognitionSnapshot`): `cognitive_load` and `fatigue` are labelled "runtime state" but `refresh_phase11_layers` (mod.rs:2160-2190) rebuilds cognition, culture, language, technology (except accumulated_knowledge) and consciousness from the profile every tick, so these are constants. `decision.step` reads `cognition.cognitive_load` (decision.rs:~95), so its "load squeeze" is a fixed per-person number.

### 1.5 Memory

- `memory.rs:74` (episodic/semantic/working "traits" + encoding_strength, consolidation_level, retrieval_strength). Encoding follows `attention.available`; consolidation gets a x1.0-1.5 bonus from melatonin (a crude sleep-dependent consolidation). It stores no content.
- `advanced_memory.rs:158` projects the last `AutonomousMind::experiences` (cap 2,000, autonomy.rs:67) into "episodes" and reward-salient "autobiographical timeline". The working-memory item list, semantic concepts, procedural skills and habits stay empty by design (advanced_memory.rs:12-26; test asserts `procedural_memory.skills.is_empty()`).
- `conversation_history` (mod.rs, dialogue.rs) is the only memory with content about other people, used to compose a greeting that recalls the last thing the partner said.
- Dead: neither `memory` nor `advanced_memory` is read by `choose_action`. Nothing is recalled to decide anything. Working-memory capacity is a 0..1 trait, not "about 4 chunks". No forgetting of content, no spatial/cognitive map, no landmark or route memory, no recognition of individuals.

### 1.6 Cognition and decision-making

- Real chooser: argmax of `learned_preference * incentive + drive + noise` (autonomy.rs:156-238). One step lookahead of zero; no goals, no plans, no multi-step tasks except labour's `ActiveTask` carried across ticks (labour.rs), no deliberation about tools, no weighing of risk.
- `decision.rs:76` computes `effective_rationality/intuition/commitment`; read by `brain_regions.rs` only (dead for behaviour).
- `consciousness.rs`, `brain_regions.rs`, `mesoscale_brain.rs`, `population_dynamics.rs`, `formal_predictive_processing.rs` (1,656 lines; stepped each tick under `construction-counters` timing), `attractor_control.rs`, `core_systems.rs`: elaborate dynamical summaries; `core_systems` is the only one that reaches the chooser (through `willpower_available` and `chaotic_urges`). `formal_predictive_processing.selected_action` is explicitly "not executed" (thought.rs:15-19). Cost without behaviour.
- Reaction time, speed-accuracy trade-off, biases (loss aversion, temporal discounting), habits: absent. Every action takes effect in one tick regardless of the person's state except via `physical_capacity`.

### 1.7 Emotion, mood, motivation

- `emotion.rs:306`: 31 independent 0..1 levels each pulled toward a baseline plus linear contributions from needs, social_cognition, creativity, attention, pain, sickness. No appraisal of events relative to goals, no emotion-specific action tendencies, no mood (slow average) distinct from emotion, no duration model. Event inputs: grief shock (mod.rs:585, `propagate_grief` mod.rs:530), `apply_harm` fear/sadness (mod.rs:1361), social shocks.
- Consumers: `dark_triad.step` (dark_triad.rs:93) -> `active_malice` -> only `Harm` (rare; autonomy.rs comment says most emotions "have no real driver of their own", "23 undriven emotions"); `neurochemistry.step` -> hormones -> brain state (Stressed/Excited/Focused/Relaxed have no behavioural consequence, only Fatigued does) and `learning_rate_multiplier`; `core_systems.step` -> willed urges; `thought.rs:164` and `dialogue.rs:147` choose template text from the dominant emotion.
- `comprehensive_emotion.rs` (376 lines) is dead (one external reference).
- Motivation: six `NeedsVector` drives in the old agent path plus `core_systems` five urges (survival/social/achievement/exploration/reproduction). Real motives with no model: safety/fear-avoidance of hazards (only `hazard_index` biases Explore), status, belonging beyond `social_density`, curiosity as information gain, play, boredom -> behaviour change, pride/mastery from skill, meaning/ritual, parental motivation.

### 1.8 Personality and individual differences

- Genome -> `GeneticsSnapshot` (genetics.rs) with extraversion, novelty_seek, rumination, threat_bias, exec_control, cortisol_sensitivity; `temperament_matrix` (openness, conscientiousness, empathy...) in mk_core. Only `extraversion` and `novelty_seek` reach the chooser; the rest feed derived 0..1 snapshots (technology, culture, language are linear combinations, e.g. `technology.rs:53-75`, `culture.rs:33`, `language.rs:49`). Personality does not develop or change with life events (it is rebuilt from the profile each tick).
- Neurotypes (ADHD, autism) exist as profile switches (`HumanFactory`, mod.rs:2219-2245), affecting `cognition` and `language.mode`.

### 1.9 Social behaviour, relationships, language

- Relationships: `pair_bond: Option<String>` (set by successful intimacy, mod.rs ~1300), parents/children/siblings via lineage, `social_systems.rs` scalars (`social_trust`, `experiential_trust`; `apply_social_shock`), `ConversationRelationship` enum (parent/child/sibling/neighbour...) used to pick conversation partners. There is no per-pair state (familiarity, trust, debt, history) except the conversation log; trust is a single scalar per person.
- Social actions that exist: `SocialApproach` (walk to nearest person), `Transfer` (give one item to the nearest adjacent person, mod.rs ~1120), `Harm`, `Intimacy`. No cooperation on a task, no joint work, no sharing norms, reciprocity, reputation, conflict resolution, teaching, play, care, or leadership. `social_cognition` (theory of mind etc.) is dead for behaviour.
- Language: `language.rs` is eight 0..1 traits (dead). Speech is template composition over the speaker's own state (thought.rs:180, dialogue.rs:385, D35/D36). Conversations carry no information (no "the berries are by the river") and change no behaviour. No language acquisition by children.
- Grief propagation to relatives on death (mod.rs:530) is the one real relationship-driven behaviour.

### 1.10 Learning, skill, tools, invention, culture

- Skill acquisition: NONE in the engine. `mk_core::human::SkillMatrix` (mk_core/src/human/skills.rs; 10 competencies) exists and is never read (`grep` finds only the re-export). `labour.rs` tasks use tool and body (BMR, load, age, `labour.rs:420`) but no per-person proficiency: a person who has felled 1,000 trees is as fast as one who never has. `advanced_memory.procedural_memory.skills` is empty by design.
- `learning.rs:76`: plasticity declines with maturity, `pattern_recognition`, `wisdom_accumulation` accrue from `memory.consolidation_level * acquisition_rate`. Dead for behaviour except through `decision` (dead) and creativity (dead).
- Tool use: `ToolInstance`/`ToolKind` in resource_economy.rs (durability, hands), recipes. Using a tool is an inventory fact, not a skill.
- Invention: `technology.rs:102 step_invention`: `accumulated_knowledge += learning_speed * dt_years` for every living person every tick (mod.rs:1057), unlock thresholds IronAxe/IronPickaxe 5, Cart 15, StoneHouse 30 (resource_economy.rs:85-94). This is a wall-clock timer scaled by a trait. It does not depend on what the person does, observes, tries, fails at, or is taught. `Code` at a computer room adds a bonus (mod.rs:1070). The comment says newborns start at zero (technology.rs doc), so there is no transmission to children; each person independently "invents" the same things on a schedule. 15 recipe ids, 4 gated. `creativity.rs` (13 traits, 2 stepped values) is dead.
- Culture: `culture.rs` is seven 0..1 traits (dead). No shared norms, no taboos, rituals, stories, naming conventions, division of labour, or inheritance of practices.
- Real-internet bridge: `WebSearch`/`SendEmail` (computer_bridge.rs, D17) is the only route by which a person acquires information from outside, and that information is stored as `last_web_search_result` with no effect on skills or knowledge.

### 1.11 Validation and register coverage

- Tested against cited packs: survival without water/food, step-length invariance, circadian free-run/entrainment/phase-shift limits, sleep duration, gestation, mortality curves, child growth, labour MET, menopause, nobody outlives 122 years (`tests/human_realism.rs` lines 65-806).
- No tests and no packs for: any sense, attention, memory, learning, emotion, social behaviour, personality, creativity, culture. `fixtures/reference/humans/` has only `life_history.json` and `physiology.json`.
- DEVIATIONS.md lists nothing for these; D12, D34, D35, D36 (dialogue) are the only cognition-adjacent rows. Suggested new rows are in section 5.

---------------------------------------------------------------------------------------------------

## 2. Authoritative sources and key numbers

Verification key: [V] = I read the figure in a primary page, abstract or PDF during this session; [S] = the figure appeared in a search-result summary of a primary or reputable source (marked with URL) and I did not open the full text; [U] = I could not verify it; do not cite it without checking. NCBI/PMC/PubMed fetches were blocked by a bot check and Firecrawl was out of credits, so many primary pages could only be reached through search summaries. Everything marked [S] must be re-read before it goes into a pack file under the project's "every number is cited" rule.

### 2.1 Reaction time and speed of processing
- Woods et al. 2015, Front Hum Neurosci 9:131, https://www.ncbi.nlm.nih.gov/pmc/articles/PMC4374455/ [V]. N = 1,469 adults 18-65: mean simple reaction time 231 ms (213 ms after hardware correction), SD about 27 ms, increase 0.55 ms/year of age; stimulus detection time 131 ms (SD 30.2) with no age effect (so slowing is motor); hit rate 97.1%; a preceding SOA added about 28 ms. Replication sample N = 189 (18-82): 237.8 ms, 0.45 ms/year.
- Hick-Hyman law (choice RT grows with log2 of alternatives): not fetched; slope value [U]. Use for "how long a decision takes".

### 2.2 Vision
- Webvision, Kalloniatis & Luu, "Light and Dark Adaptation", https://www.webvision.pitt.edu/book/part-viii-psychophysics-of-vision/light-and-dark-adaptation/ [V]: rod-cone break at about 5-8 minutes in the dark; the curve levels off at roughly 1e-5 cd/m2 after about 40 minutes; photopic above, scotopic below about 0.03 cd/m2 (per a search summary of the same chapter [S], https://www.ncbi.nlm.nih.gov/books/NBK11525/). The AMS glossary gives a different "20-30 minutes" figure for peak rod sensitivity [S] (https://glossary.ametsoc.org/wiki/dark-adaptation/), so sources disagree; pick one and cite it.
- Peripheral vision/visual field extent, foveal 1 arcmin acuity: Strasburger, Rentschler & Juttner 2011, J Vision 11(5):13, PMID 22207654 [U for numbers: only the erratum record was retrievable]. Read it before coding field-of-view.
- Spatial acuity of the retina and 20/20 = 1 arcmin: not verified [U].

### 2.3 Hearing
- Arxiv review "The Human Auditory System and Audio", https://arxiv.org/pdf/2307.00084 [S]: young healthy ears hear about 16 Hz to 18 kHz (20 Hz-20 kHz as the conventional round range); 1 kHz is the reference frequency at which phon = dB SPL; equal-loudness contours are ISO 226:2003. Reference threshold 20 micropascals at 1 kHz [S] (two reference pages). 85 dB damage onset: [U] (NIDCD page not retrieved). Use ISO 226:2003 directly for the audibility-vs-frequency curve.

### 2.4 Smell and taste
- Human has about 400 intact olfactory receptor genes (Mainland et al. 2014, Nat Neurosci; 63% had function-altering polymorphisms) [S]; 347 in an older cloning study; about 900 genes with 54% pseudogenes (PLoS ONE 2007, https://www.ncbi.nlm.nih.gov/pmc/articles/PMC1925143/) [S]. So smell varies strongly between individuals.
- Bushdid 2014 "1 trillion odours" and its rebuttal (Gerkin & Castro 2015): not found [U]. Do not use "1 trillion" without reading the rebuttal.
- Taste: five basic tastes, each with its own receptor cell type: Chandrashekar et al. 2006, Nature 444:288-294, https://www.nature.com/articles/nature05401 [S]. Roles: sweet = energy, umami = amino acids, salt = electrolytes, sour and bitter = potentially harmful. The count of about 25 human bitter receptors [U].
- Taste/odour detection thresholds in concentration units: [U]. This is a real gap in my research; a pack needs a cited table (e.g. Devos et al. 1990 standardized olfactory thresholds; I did not fetch it).

### 2.5 Touch, pain, temperature
- Johnson 2001, Curr Opin Neurobiol 11:455-461 (PDF read, https://www.cns.nyu.edu/~david/courses/sm12/Readings/Johnson%20Curr%20Biol%202001.pdf) [V]: four afferent classes: SA1/Merkel (form and texture; receptive field 2-3 mm), RA/Meissner (flutter, slip; fields 3-5 mm), PC/Pacinian (high-frequency vibration, 20-150 Hz threshold region described), SA2/Ruffini (skin stretch, fields about 5x larger); RA density at fingertip 150 per cm2.
- Rolke et al. 2006, Pain 123:231-243 (PDF read) [V]: DFNS standard QST battery (CDT, WDT, CPT, HPT, MDT, MPT, PPT...), 180 healthy subjects; thresholds depend significantly on body region (face vs foot effect sizes up to 2.3-2.8 for thermal detection and pressure pain), age (effect sizes 0.19-0.52) and gender (pain thresholds lower in women, p<0.01). The log-SD of absolute data: CPT 8.66 C, HPT 3.19 C. I could not extract absolute mean thresholds in degrees C; Table 2 there is a detectability table, not the means [U].
- Rolke/DFNS trunk data, Pain 2014 155:1002-1015, PMID 24525274 [V abstract]: abnormal thermal detection needed changes of >3.3-8.2 C; females have lower pressure pain thresholds.
- Two-point discrimination fingertip 2-8 mm, back 30-40 mm [S] (https://en.wikipedia.org/wiki/Two-point_discrimination, secondary; the test itself is criticised as an invalid acuity measure). Weinstein 1968 original [U].

### 2.6 Vestibular / balance and interoception
- Grabherr et al. 2008, Exp Brain Res 186:677-681, https://boris.unibe.ch/33915 [S]: mean yaw rotation direction-detection thresholds 2.8 deg/s at 0.05 Hz, 2.5 at 0.1, 1.7 at 0.2, 0.7 at 0.5, 0.6 at 1, 0.4 at 2, 0.6 at 5 Hz; plateau 0.71 deg/s, corner 0.23 Hz. Translation thresholds 0.58 cm/s (y), 1.24 cm/s (z) from a different study [S].
- Interoception: Garfinkel et al. 2015, Biol Psychol, accuracy (objective, heartbeat tasks), sensibility (self-report) and awareness (metacognitive link) are distinct, and heartbeat-counting scores are disputed [S]. Craig 2002 Nat Rev Neurosci [U: not retrieved].

### 2.7 Thermoregulation and cold/heat
- Kingma, Frijns, van Marken Lichtenbelt 2012, Front Biosci, PMID 22202013 [S]: thermoneutral zone defined as ambient range with no regulatory change in metabolic heat or evaporative loss; shifts with body composition, clothing, age, sex. The 28 C figure and 37 C core [U in this session]. The engine's 10-32 C is not supported by any source I found.
- Brown et al. 2012, NEJM 367:1930-1938: accidental hypothermia is an involuntary fall in core temperature below 35 C; clinical staging by consciousness (conscious; decreased consciousness; unconscious; arrest). Swiss staging temperature bands (32-35 / 28-32 / <28) [U].

### 2.8 Thirst, hunger and nutrition
- Thirst threshold: systematic review of 12 trials, https://www.ncbi.nlm.nih.gov/pmc/articles/PMC5763530/ [S]: plasma osmolality regulated 275-295 mOsm/kg; thirst threshold 285.23 +/- 1.29 mOsm/kg (95% CI), vasopressin release 284.3 +/- 0.71; a 1-2% rise triggers thirst (clinical review); one older study measured 281 mOsm/kg. Use this to replace the linear "1 - hydration" thirst with a threshold-triggered one.
- Scurvy: 1-3 months of vitamin C deprivation; symptoms when body pool falls to about 300 mg (RDA maintains a pool of 1500 mg) [S] (https://pmc.ncbi.nlm.nih.gov/articles/PMC8958866). NIH ODS fact sheet itself [U].

### 2.9 Sleep architecture
- Ohayon, Carskadon, Guilleminault, Vitiello 2004, Sleep 27(7):1255-1273 (PDF read) [V]: meta-analysis, 65 studies, 3,577 subjects aged 5-102. In adults total sleep time falls about 10 minutes per decade; slow-wave sleep percentage falls about 2% per decade; stage 1 and stage 2 percentages, sleep latency and wake after sleep onset rise with age; REM percentage decreases (small effect, about 4% difference in people 60+); only sleep efficiency keeps falling after 60. In children SWS and stage 2 changed about 7% per 5-year period between ages 5 and 15. The paper's per-stage mean percentages for young adults were not extracted [U].
- NREM-REM cycle length 90-110 min (2004 review, PMID 15046198) and 90-120 min later cycles [S]; four to six cycles per 8 h [S]. N2 40-50%, N3 up to 25% quoted by consumer sources [U]. Get the AASM scoring manual values or StatPearls NBK537023 (blocked) before using.
- Sleep deprivation dose-response (Van Dongen et al. 2003) [U: not retrieved].
- Diekelmann & Born 2010, Nat Rev Neurosci 11:114-126: sleep consolidates memory; slow-wave sleep most beneficial for declarative memories [S].

### 2.10 Attention
- Simons & Chabris 1999, Perception 28:1059-1074 [S for design; the "46%" and "50%" failure rates are [U]]: people counting passes often fail to notice a gorilla (inattentional blindness). A radiologist replication cites 83% [U, blog].

### 2.11 Memory
- Cowan 2001, BBS 24:87-114, https://pubmed.ncbi.nlm.nih.gov/11515286/ [V abstract via Europe PMC]: "a single, central capacity limit averaging about four chunks"; Miller's 7 was a rough estimate; the review finds a limit of three to five chunks. This is the number for working-memory capacity (not 7+/-2).
- Murre & Dros 2015, PLoS ONE 10(7):e0120644, https://journals.plos.org/plosone/article?id=10.1371/journal.pone.0120644 [V]: replication of Ebbinghaus's savings curve at 20 min to 31 days; the curve is not smooth and shows an upward jump from the 24 h point; savings at 31 days 0.041 raw or 0.137 corrected (Ebbinghaus-era studies about 0.20); best-AIC fit is the Memory Chain Model / summed exponentials; Ebbinghaus (1880) power fit x = [1 - (2/t)^0.099]^0.51 with t in minutes. Use for forgetting of episodic content.
- Squire's declarative (episodic, semantic) / nondeclarative (procedural) taxonomy [S, secondary description].

### 2.11b Learning curves and skill
- Power vs exponential law of practice: Heathcote, Brown & Mewhort 2000, Psychon Bull Rev 7(2):185-207 [S]: across 40 datasets (7,910 learning series, 475 subjects, 24 experiments) an exponential function fit better than a power function in every unaveraged dataset; averaging across people biases toward a power law; APEX function proposed. Newell & Rosenbloom 1981 (power law T = B*N^-alpha; formula from memory, [U]). Evans, Brown, Mewhort & Heathcote 2018 "Refining the Law of Practice" is the Bayesian follow-up [S]. Implication: use a per-person exponential/APEX curve with a floor, not an ever-faster power law.
- Habit formation: Lally et al. 2010, Eur J Soc Psychol, 96 volunteers, automaticity follows an asymptotic curve; median 66 days to reach 95% of asymptote, range 18-254 days (extrapolated, not observed) [S] (https://openresearch.surrey.ac.uk/esploro/outputs/99783513802346).
- Macnamara, Hambrick & Oswald 2014, Psychol Sci: deliberate practice explained 26% of variance in games, 21% music, 18% sports, 4% education, less than 1% professions (abstract as quoted on a mirror) [S]; about 12% overall (press release, 88 studies, N = 11,135). Implication: practice is necessary but individual differences remain large.

### 2.12 Emotion
- Moors, Ellsworth, Scherer, Frijda 2013, Emotion Review 5:119-124, doi 10.1177/1754073912468165 [S]: appraisal theories treat emotions as processes elicited by evaluating events against what matters to the person (goal relevance, novelty/valence, coping potential in Scherer's component process model; the specific criteria text was not retrieved, [U]).
- OCC model, Ortony, Clore & Collins 1988, 22 emotion types organised by focus on events (well-being, prospect-based, fortunes-of-others), agents' actions, or objects [S, structure only; the full list of 22 [U] from this session]. It is the standard implementable appraisal tree for games.
- Russell 1980 circumplex: valence x arousal, 28 emotion words sorted [S]. Use for mood as a two-dimensional slow state.
- Duration: Verduyn & Lavrijsen 2015, Motiv Emotion 39: of 27 emotions sadness lasted longest (about 120 h in press reports of recalled episodes), shame/surprise/fear/disgust/boredom/irritation/relief shortest (about 30 min for shame); rumination and event importance explain much of it [S] (https://ppw.kuleuven.be/okp/_pdf/Verduyn2015WELLA.pdf). Press figures are averages, check the paper's medians [U].

### 2.13 Decision-making
- Tversky & Kahneman 1992, J Risk Uncertainty 5:297-323 [S, secondary calculators]: median value-function exponent 0.88 and loss aversion lambda 2.25 (25 graduate students); probability-weighting 0.61 (gains) and 0.69 (losses). A meta-analysis mean of about 1.96 is quoted by a blog [U]. Treat the 2.25 as a single small-sample estimate.

### 2.14 Personality
- Polderman et al. 2015, Nat Genet 47:702-709 [S]: 2,748 publications, 17,804 traits, about 14.6 million twin pairs; average heritability 49%; 69% of traits fit a purely additive genetic model.
- Roberts & DelVecchio 2000, Psychol Bull [S] (https://stafforini.com/works/roberts-2000-rankorder-consistency-personality/): 152 longitudinal studies, 3,217 coefficients; rank-order consistency about 0.31 in childhood, 0.54 in college, 0.64 at age 30, plateau about 0.74 at 50-70. A 2022 preregistered meta-analysis (Bleidorn et al.) found little evidence of increasing stability after 25 [S]. Implication: traits are heritable and fairly stable but change slowly with age; fixed forever is wrong.

### 2.15 Social structure and relationships
- Zhou, Sornette, Hill & Dunbar 2005, Proc R Soc B 272:439-444 [S] (https://arxiv.org/abs/cond-mat/0403299): discrete hierarchy of group sizes with scaling ratio about 3 (3-5, 9-15, 30-45...); extrapolation to about 150 for humans. The popular 1.5/5/15/50/150 layers were not confirmed [U].
- Hall 2018/2019, J Soc Pers Relat, 112 students plus 355 adults who had moved [S] (https://news.ku.edu/2018/03/06/study-reveals-number-hours-it-takes-make-friend): about 50 hours together to move from acquaintance to casual friend, about 90 hours to "friend", more than 200 hours to close friend; a modelled estimate gave 94, 164 and 219 hours respectively [S, secondary]. Direct, quantitative, and easy to implement as accumulated shared time per pair.

### 2.16 Theory of mind and language
- Wellman, Cross & Watson 2001, Child Dev 72:655-684, doi 10.1111/1467-8624.00304 [S]: 178 studies; preschoolers go from below-chance to above-chance on false-belief tasks; age, country and four task factors explain 55% of variance; consistent with genuine conceptual change in the preschool years. The age at which above-chance begins (about 3.5-4 years) comes from a secondary study guide [U].
- Brysbaert et al. 2016, Front Psychol 7:1116, https://pmc.ncbi.nlm.nih.gov/articles/PMC4965448 [S]: a 20-year-old native English speaker knows about 42,000 lemmas (range 27,000 for the lowest 5% to 52,000 for the highest 5%), about 4,200 multiword expressions and 11,100 word families; adults add about 6,000 lemmas between 20 and 60 (about one every two days).
- Early vocabulary growth: Wordbank / MacArthur-Bates CDI norms (Frank et al. 2021, MIT Press, 75,000+ children, 29 languages) [S for existence; the medians at 12/18/24 months [U]]. Use https://wordbank.stanford.edu once fetched.
- Motor milestones: Martorell et al. 2006, Acta Paediatr 95 (suppl 450):86-95 [S] (https://www.who.int/tools/child-growth-standards/standards/motor-development-milestones): 816 children 4-24 months; the windows (1st-99th percentile): sitting without support 3.8-9.2 months (window 5.4 months, narrowest), walking with assistance 5.9-13.7 months, standing alone widest at 10.0 months, walking alone 9.4 months. Walking-alone window bounds [U].
- Infant feeding: WHO recommends exclusive breastfeeding for the first 6 months, then safe complementary foods with continued breastfeeding to 2 years or beyond [S] (https://www.who.int/health-topics/complementary-feeding); WHO 2023 guideline for 6-23 months adds daily animal-source foods and fruit/vegetables.

### 2.17 Cultural transmission and technology
- Henrich 2004, Am Antiq 69(2):197-214 [S abstract]: Tasmanian societies lost skills (bone tools, cold-weather clothing, hafted tools, nets, fishing spears, barbed spears, spear-throwers, boomerangs) over about 8,000 years of isolation after the land bridge closed; model: a small pool of learners lets complex skills deteriorate while simple ones persist. Disputed by Read 2006 and later critiques [S].
- Kline & Boyd 2010, Proc R Soc B 277:2559-2564, https://pmc.ncbi.nlm.nih.gov/articles/PMC2894932 [S]: 10 Oceanic societies; larger, better-connected populations had more complex marine-foraging toolkits (complexity scored by component counts).
- Derex et al. 2013, Nature 503:389-391 [S]: groups of 2, 4, 8, 16 in a computer task, larger groups kept more cultural complexity; Andersson & Read 2014 (Nature 511) reanalysis argues the opposite; contested.
- Implication for a tiny island: with two founders and a few children, loss of skills is the expected default, and the code does the reverse (each person's knowledge counter rises on a timer regardless of the group size).
- Hunter-gatherer skill and dependency: Gurven & Kaplan 2007, Popul Dev Rev 33:321-365 [S]: modal adult lifespan 68-78 years in 21 forager/horticulturalist groups; children are net consumers into their teens. Kaplan et al. 2000 (Evol Anthropol 9:156-185) is the standard source on skill peaking late (hunting returns peak in the 30s); I could not retrieve its numbers [U].

---------------------------------------------------------------------------------------------------

## 3. Ranked gaps (impact on believable behaviour on a small island)

Ranking criterion: how visibly a gap breaks believability in a 2-founders-plus-children island within weeks of simulated life, weighted by how much of the existing machinery it would let me reuse.

### Rank 1. Perception is cell averages, not things (senses are dead)
Why it dominates: every other cognitive gap depends on something to perceive. People cannot see the stream, hear the child, smell smoke or spoilage, or recognise a face; they read "hydration_access" for their own cell and the eight neighbours.
Real mechanism and values: sections 2.2-2.6. Minimum viable: field of view and range limited by light (photopic/mesopic/scotopic with dark adaptation 5-8 min to rod-cone break, about 40 min full), hearing by ISO 226 threshold and distance, smell and taste as gating of food quality and hazards, pain/temperature thresholds producing withdrawal.
Implementation sketch:
- New `perception` step producing a `Percept` list (entity kind, bearing, distance, salience, confidence) per person from the world: water cells, food nodes (`resource_economy::ResourceNode`), people, buildings, fire, animals (`regional/fauna.rs`), hazards. Sensory capacities (the existing 0..1 scalars, rescaled to cited thresholds) filter it: visibility range from luminance and dark-adaptation state, audibility from distance falloff and ambient noise, olfactory plume from wind (`regional/weather.rs`, `climate.rs`) for smoke, carrion, ripe fruit.
- `AgentWorldObservation` gets an optional `percepts: Vec<Percept>` (keep the old scalars as fallback so upstream sync does not break). `choose_action` and `directed_seek_step` use the percept list for targets.
- Pain from injury, cold and heat: make `sensory.overall_pain_level` include `body.injury` and thermal strain; add withdrawal reflex (action substitution like `age_substitute`).
Files: humans/sensory.rs, humans/attention.rs, humans/observation.rs, agents/mod.rs (observation struct), regional/humans.rs (build percepts from island data), humans/autonomy.rs (consume), regional/fauna.rs and weather.rs (sources), new reference pack `fixtures/reference/humans/senses.json` and tests. Effort: large; split into vision+hearing+pain first, smell/taste second.

### Rank 2. No skills; invention is a timer; no teaching (learning, tool use, invention, culture)
Why: the island premise is a founding couple building a farm and a town; progress should come from practice, failure, teaching and group size. Today `accumulated_knowledge += learning_speed*dt` for everyone, the same recipes unlock at the same ages, children inherit nothing, and labour speed ignores experience.
Real mechanism: practice curves are exponential/APEX with a floor (Heathcote 2000, 2.11b), habit asymptotes take 18-254 days (Lally 2010), practice explains only about 12% of performance variance overall (Macnamara 2014), skills are lost in small groups (Henrich 2004; Kline & Boyd 2010), forager skill peaks in the 30s and juveniles are net consumers (Kaplan/Gurven, 2.17).
Implementation sketch:
- Per-person `SkillLedger`: map task id (the ids already in `regional/labour.rs`: felling, digging, mining, building, craft recipes, foraging, cooking, fire-making, farming) -> `{repetitions, effective_hours, proficiency}` with `proficiency = floor + (1-floor)*(1 - exp(-k*hours))` and per-person k from `learning` traits and age plasticity. Multiply `labour.rs` task duration/yield by proficiency (the packs already give skilled vs unskilled hours, see labour.rs:390).
- Recipe knowledge becomes a set per person (and per household) acquired by (a) doing the prerequisite tasks, (b) observing a skilled person nearby doing it (observational learning gated by attention and distance), (c) being taught (a `Teach` action: skilled -> novice, adjacency, costs time), (d) random insight proportional to `creativity.effective_insight_generation` when attempting a recipe with partial materials. Replace `recipe_unlocked(id, accumulated_knowledge)` (resource_economy.rs:96) with `person.knows(id)`; keep thresholds only as a fallback for old saves.
- Decay: knowledge decays without use (Ebbinghaus-type summed exponential, Murre 2015) and is lost when the last skilled person dies.
- Children: learn by following parents (they already follow the mother under 5, life.rs:1450) with observational-learning rate and an age-dependent contribution.
Files: new humans/skills.rs (reuse unused `mk_core::human::SkillMatrix` for aptitude), humans/learning.rs, humans/technology.rs, resource_economy.rs (recipe gate and RecipeId knowledge), regional/labour.rs, regional/life.rs, humans/mod.rs (Teach action; ActionKind gains `Teach`/`Practice` in agents/behavior/mod.rs). DEVIATIONS rows first (see 5). Effort: large but high payoff.

### Rank 3. The chooser ignores memory, emotion, goals and personality (cognition and decision-making)
Why: behaviour is a noisy argmax of six needs; people have no routines, no plans, no memory of where food was, no reaction to mood. Two founders will look identical and robotic.
Real mechanism: working memory about 4 chunks (Cowan 2001), simple RT 231 ms and choice RT rising with options (Woods 2015; Hick), loss aversion lambda about 2.25 and alpha 0.88 (T&K 1992, single-sample), habits as automatised context-action links (Lally), memory content decays (Murre & Dros 2015) and is sleep-consolidated (Diekelmann & Born 2010).
Implementation sketch:
- Spatial/episodic memory store (replaces the empty `advanced_memory` lists): remembered places with kind, last-seen tick, yield/danger estimate and confidence decaying exponentially; recall success from `memory.retrieval_strength`, attention at encoding, emotion tag. Chooser adds a "known destinations" option set (go to remembered spring/orchard) before random exploration.
- Goal layer above `ActionKind`: a small stack (e.g. "have water for tomorrow", "finish the roof") with commitment from `decision.effective_commitment`; scores for sub-actions get a goal bonus. Use `decision.rs` and `learning.rs` outputs rather than fabricating new state: `effective_rationality/intuition` select between option-by-option expected value and habit.
- Risk attitude: value function with cited exponent and loss aversion, personal variation from the genome.
Files: humans/memory.rs, advanced_memory.rs, decision.rs, autonomy.rs (`drive_for`, option generation), humans/mod.rs (decide), agents/behavior/mod.rs. Effort: large; do after Rank 1 provides percepts to remember.

### Rank 4. Emotion is 31 independent bars with no appraisal, mood, or behavioural role
Why: believable people react to events (a death, a gift, a storm, a failed harvest) with emotion that decays on a plausible time scale and biases behaviour (withdraw, cling, hurry, bond).
Real mechanism: appraisal against goals (Moors 2013; OCC 22 emotion types; Scherer's criteria), valence-arousal core affect (Russell 1980), emotion durations from minutes (shame about 30 min) to days (sadness about 120 h) driven by rumination and event importance (Verduyn 2015).
Implementation sketch:
- Event bus: every world event that touches a person (harvest success/failure, being given food, a conversation outcome, a near-miss, a death, a birth, being hurt, loss of an item) becomes an `AppraisedEvent` with goal-relevance, valence, controllability, agency; appraisal -> emotion deltas with per-emotion half-lives taken from the duration source (replace the single decay toward baseline in `emotion.rs:306`).
- Mood = slow integral of valence/arousal (Russell); mood biases `drive_for` (low mood lowers Explore/Social, raises Rest; anxiety raises hazard sensitivity; anger raises Harm only with target and low inhibition).
- Delete or park `comprehensive_emotion.rs` (dead) until appraisal exists.
Files: humans/emotion.rs, dark_triad.rs, new humans/appraisal.rs, humans/mod.rs (grief and harm hooks become events), autonomy.rs (mood term). Effort: medium.

### Rank 5. Relationships and cooperation: one trust scalar, no per-person bonds, no joint work, no information exchange
Why: a family on an island is mostly relationships: who helps, who cares for the baby, who to trust, quarrels. Today conversation is decorative text (D35/D36), `Transfer` gives one item to whoever is adjacent, and nothing is learned from other people.
Real mechanism: friendship requires accumulated shared time (about 50/90/200 h for casual/friend/close, Hall 2018); social network layers scale by about 3 (Zhou 2005); theory of mind arrives around the preschool years (Wellman 2001); attachment and kinship drive investment.
Implementation sketch:
- Per-pair `Bond { familiarity_hours, trust, affect, debt, last_interaction }` in a sparse map keyed by agent id, updated by co-presence (hours together), shared tasks, help given and received, conflicts. Thresholds from Hall (50/90/200 h). Replace `social_systems.social_trust` scalar reads with pair lookups.
- Actions: `Help` (join someone's task, share labour), `Share` (food to a child/partner prioritised by bond and need), `Teach` (Rank 2), `Console`. Conversation carries typed content (location of resource, a recipe, a request, an emotion), with real effect: updates the listener's memory/bond.
- Theory of mind age gating from Wellman: before about 4 years no false-belief reasoning; use `social_cognition` traits for adult differences.
Files: humans/social_systems.rs, social_cognition.rs, dialogue.rs, mod.rs (`step_dialogue`, Transfer), agents/behavior/mod.rs. Effort: medium-large.

### Rank 6. Children are not dependent, taught, or developing cognitively; no language acquisition
Why: the island's first decade is mostly children growing up. Today a child is an adult with age-substituted action kinds; no feeding, carrying, milestones, speech.
Real mechanism: exclusive breastfeeding 6 months, then complementary foods to 2 years or more (WHO); motor milestone windows (sitting 3.8-9.2 months, walking with assistance 5.9-13.7 months, standing alone window 10 months; Martorell 2006); first words and vocabulary growth (CDI norms, [fetch Wordbank]); adults ending near 42,000 lemmas (Brysbaert 2016); theory-of-mind transition (Wellman 2001); juveniles are net consumers into their teens (Gurven & Kaplan 2007).
Implementation sketch: infant feeding state (mother's lactation costing her energy per WHO schedule; no mother -> death risk unless another feeds), caregiver actions `Feed/Carry/Watch` with duty cycle, developmental milestones gating mobility/speech/action kinds (replace the flat 5-year `DEPENDENT_AGE_YEARS`), vocabulary size as a per-person integer growing by exposure to speech (conversations, Rank 5).
Files: regional/life.rs (dependency, household feeding), humans/development.rs, growth.rs, language.rs, needs.rs (lactation and infant energy), lifecycle.rs (birth risk, postpartum). Effort: medium; needs lactation and weaning parameters from cited packs.

### Rank 7. Injury, accidents and illness are abstract (no mechanism for the commonest ways to die or suffer on an island)
Why: with tools, fire, water and tides, believable risk comes from cuts, burns, drowning, falls, infection, tooth and gut problems, childbirth; at present only `Harm` injures and mortality is a Siler abstraction.
Real mechanism: injury events drawn from activity-specific hazard rates, wound healing and infection, named infectious diseases with incubation and course (common colds: adults about 2-4 per year, children 6-10 [S, WebMD/university pages, weak sources]; check CDC), nutrient deficiency diseases (scurvy 1-3 months, body pool 1500 mg to about 300 mg).
Implementation sketch: `injury` events from labour tasks (rate per task-hour by tool, keyed RNG), body regions or at least severity classes, wound -> infection risk using `skin.infection_risk` and `immune`; named-disease table (cited incubation/duration/mortality) with person-to-person transmission through adjacency (reuse `living_positions` in mod.rs:~950); micronutrient stores (vitamin C, iron, vitamin D from daylight) in `needs.rs` keyed to foods in `regional/ecology.rs` / materials ledger.
Files: humans/body.rs, immune.rs, skin.rs, needs.rs, lifecycle.rs (`dies_of_hazard` must stop double-counting extrinsic mortality once explicit hazards exist: D21), regional/labour.rs, regional/materials.rs. Effort: large. Caveat: replacing Siler extrinsic mortality must not change calibrated adult mortality (`human_realism.rs:449-506`); keep the Siler term for what is not modelled explicitly.

### Rank 8. Thermoregulation has uncited constants and no behavioural loop (cold/heat comfort, clothing, fire)
Why: an island with a 10 K annual range (D37) and mid-latitude winters needs people who feel cold, add clothing, light fires and seek shelter well before lethality. Currently the only effect is death at 28 C or 42 C.
Real mechanism: thermoneutral zone depends on clothing and body composition (Kingma 2012, [S]); hypothermia defined below 35 C core (Brown 2012, [S]); shivering and sweating cost energy/water.
Implementation sketch: replace the unsourced constants in body.rs:104-122 with cited values, add a thermal-comfort signal from skin and core temperature that feeds the chooser (new drive "thermal" -> SeekShelter, `Craft` clothing, `Build` fire) and adds shivering energy to `EffortFocus.activity`. Clothing as an item with an insulation value (clo) from a cited source, which [U] in this session.
Files: humans/body.rs, needs.rs, autonomy.rs, resource_economy.rs (clothing recipes), regional/energy.rs (fire/heat). Effort: medium.

### Rank 9. Sleep has no stages and no arousal threshold
Why: moderate. Timing is good. Missing: sleeping people cannot be woken by noise or a crying baby (matters with infants), no night waking, no sleep-debt dose response, no memory consolidation specificity.
Real mechanism: 90-110 min NREM-REM cycles, SWS concentrated early, REM later, TST drops about 10 min per decade and SWS about 2% per decade (Ohayon 2004).
Implementation sketch: stage state machine on top of `circadian.rs` asleep gate; arousal threshold per stage vs sound level from Rank 1; sleep debt accumulation; consolidation tied to SWS (memory.rs `sleep_consolidation_bonus`).
Files: humans/circadian.rs, memory.rs, neurochemistry.rs. Effort: small-medium once audibility exists.

### Rank 10. Personality is static and only two traits matter
Why: individuality is cheap to add and visible; real traits are about half heritable (Polderman 2015) and drift with age (Roberts 2000: 0.31 -> 0.74 rank-order stability).
Implementation sketch: route temperament (openness, conscientiousness, agreeableness/empathy, neuroticism/threat_bias, rumination) into `drive_for`, emotion half-lives (rumination lengthens sadness, Verduyn) and bonding; slow trait change from life events (`trait_evolution.rs` exists in mk_core but is not used by the engine; check before adding).
Files: humans/genetics.rs, autonomy.rs, emotion.rs, mk_core/src/human/{temperament,trait_evolution}.rs. Effort: small-medium.

Runners-up (not ranked): language as a communication channel and per-person vocabulary (folded into 5 and 6), proprioception/falls and gait, taste/smell as food-safety gating (inside Rank 1), interoceptive noise, circadian light behaviour (already good), consciousness/brain-region modules (cost without behaviour; see 4).

---------------------------------------------------------------------------------------------------

## 4. Cross-cutting findings the owner should know

1. Cost without behaviour. `formal_predictive_processing.rs` (1,656 lines), `attractor_control.rs` (948), `consciousness`, `brain_regions`, `mesoscale_brain`, `population_dynamics`, `comprehensive_emotion` and most cognitive snapshots run every tick for every person but do not alter a single action. `tests/island_phase4_cost.rs` and `human_scaling.rs` suggest per-person cost matters for the town phase. Either wire them in (Ranks 1-5 do) or gate them to the dashboard only.
2. The register is incomplete. Project rule: every known departure is a row. None exists for: smell/taste absent, senses unused, no skill system, invention on a timer, no cultural transmission, no mood, uncited thermoregulation constants, injury only via Harm, no infant feeding. Proposed rows (D42+): "People perceive their surroundings by sight, hearing, smell, taste and touch" / "People get better at what they practise and teach it to others" / "Knowledge is lost when carriers die" / "People feel the cold and dress or burn fires" / "Injury and disease have causes".
3. Rule 5 in REALISM.md: human-behaviour fixes go to upstream `dylmarriner/Maer-Ken` first (UPSTREAM.md). Every rank above touches `mk_engine::humans`, which is imported upstream code. Phase 0 and 0c already have unsent upstream PRs (UPSTREAM.md). New mind work should be specified as upstream PRs; the island-only seams are `regional/humans.rs`, `regional/labour.rs`, `regional/life.rs` and `humans/observation.rs`.
4. Per-tick rebuilding hides state. `refresh_phase11_layers` (mod.rs:2160) resets cognition/culture/language/technology/consciousness each tick from the profile. Any new mutable knowledge, skill or personality drift must live outside those snapshots or the function must be changed to preserve it (as it already does for `accumulated_knowledge` and `experiential_trust`).
5. A big drift risk: the island's own cited realism tests only cover the body. When Ranks 1-5 land they need reference packs and tests of the same standard; the quantities in section 2 are the starting packs, but every [S] and [U] figure must be re-read first.

---------------------------------------------------------------------------------------------------

## 5. Suggested order and first deliverables

1. Add deviation rows for the mind gaps (cheap, makes the gap explicit; `crates/mk_core/tests/deviation_register.rs` needs each row to have an owning phase and status).
2. Fix the two quick, low-risk realism faults: cite or replace the thermoregulation constants (body.rs:104-122), and make `overall_pain_level` depend on `body.injury` and thermal strain (sensory.rs:158).
3. Skill ledger + knowledge set replacing the invention timer (Rank 2) - biggest visible gain on the farm and town story, needs only labour.rs, resource_economy.rs and a small state struct.
4. Percepts and memory of places (Ranks 1 and 3) - turns the grid-cell reader into someone who knows where the spring is.
5. Appraisal/mood (Rank 4) and per-pair bonds (Rank 5), then child dependence and language (Rank 6).
6. Injury/disease/nutrition (Rank 7), thermal behaviour (Rank 8), sleep stages (Rank 9), personality drift (Rank 10).

## 6. What I could not do

- Could not open NCBI/PMC/PubMed pages (bot check) or use Firecrawl (no credits); reached primary papers only through search summaries, two PDFs and Europe PMC.
- Not found or not verified: Newell & Rosenbloom 1981 formula, Hick's law slope, Miller 1956 limit, Dunbar's 1.5/5/15/50/150 layers, Bushdid/Gerkin olfaction, bitter-receptor count, absolute QST thresholds in C, olfactory and taste concentration thresholds, clothing insulation values, thermoneutral 28 C, Swiss hypothermia stages, AASM per-stage percentages, Van Dongen sleep restriction, Kaplan 2000 skill-by-age returns, WHO walking-alone window, Wordbank medians, the OCC list of 22 emotions, Scherer's appraisal checks.
- I did not trace the full household food path for children in `regional/life.rs` or run any tests; behaviour claims are from reading source and grep.
