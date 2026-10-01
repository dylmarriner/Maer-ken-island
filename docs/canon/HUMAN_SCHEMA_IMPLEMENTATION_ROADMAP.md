## Human Schema Implementation Roadmap

This roadmap defines how to move from the categorized schema definitions in `docs/canon/human-schemas/` to a fully implemented, validated, and usable human replication system.

---

## 1. Goal

Implement every schema category as a real, testable part of the human simulation stack so that:

- each schema has authoritative example data
- each schema has runtime validation coverage
- each schema maps to an actual engine/module responsibility
- schema evolution can happen safely without breaking consumers

---

## 2. Current Categorized Schema Modules

The schema surface is now split into these modules:

1. `identitySchemas.js`
2. `psychologySchemas.js`
3. `bodySchemas.js`
4. `runtimeSchemas.js`
5. `coreSystemsSchemas.js`
6. `cognitionSchemas.js`
7. `emotionSchemas.js`
8. `biologySchemas.js`
9. `sensorySchemas.js`
10. `reproductiveSchemas.js`
11. `brainSchemas.js`

The public aggregation entrypoint remains:

- `docs/canon/HumanReplicationSchema.js`

### Concrete Schemas Currently Used by `HumanReplicationSchema.js`

The roadmap must track not only category files, but also the concrete schemas imported by the public aggregation layer.

#### `identitySchemas.js`

- `CoreIdentitySchema`
- `DnaStrandSchema`
- `SexChromosomesSchema`
- `GeneticExpressionSchema`
- `GenomeSchema`
- `BirthChartSchema`
- `NarrativeSelfSchema`
- `IdentitySchema`
- `FullPhenotypeSchema`
- `HumanIdentitySchema`

#### `psychologySchemas.js`

- `TemperamentMatrixSchema`
- `NeurocognitiveProfileSchema`
- `PersonalityTraitSchema`
- `PersonalityTraitsSchema`
- `DriveWeightsSchema`
- `HormonalBaselineBiasSchema`
- `StressResponseProfileSchema`
- `AttachmentStyleSchema`
- `RelationalDefaultsSchema`
- `IdentityAxiomsSchema`

#### `bodySchemas.js`

- `BodyVitalsSchema`
- `BodyPhysiologySchema`
- `BodyAppearanceSchema`
- `DnaSchema`
- `BodySchema`

#### `runtimeSchemas.js`

- `CurrentCognitionSchema`
- `CurrentEmotionSchema`
- `RuntimeSchema`

#### `coreSystemsSchemas.js`

- `BioSysConfigSchema`
- `PsycheSysConfigSchema`
- `ChaosSysConfigSchema`
- `WillSysConfigSchema`
- `CoreSystemsSchema`
- `ArchitecturalLawsSchema`
- `DeterministicArchitectureSchema`
- `ImmutableStateSchema`
- `SystemDependenciesSchema`

#### `cognitionSchemas.js`

- `LegacyMemorySystemsSchema`
- `MemorySystemsSchema`
- `AttentionSystemSchema`
- `ConsciousnessSchema`
- `LearningAdaptationSchema`
- `SocialCognitionSchema`
- `CreativeSystemsSchema`
- `DecisionMakingSchema`

#### `emotionSchemas.js`

- `ComprehensiveEmotionTaxonomySchema`
- `GranularEmotionsSchema`
- `DarkTriadSchema`

#### `biologySchemas.js`

- `ImmuneSystemSchema`
- `SkinSystemSchema`

#### `sensorySchemas.js`

- `SensorySystemsSchema`

#### `reproductiveSchemas.js`

- `ReproductiveSystemsSchema`

#### `brainSchemas.js`

- `FunctionalBrainRegionsSchema`
- `NeuralPopulationDynamicsSchema`
- `NeurotransmitterModulationSchema`
- `AdvancedMemorySystemsSchema`
- `PredictiveProcessingSchema`
- `MesoscaleBrainEngineSchema`
- `FormalPredictiveProcessingSchema`
- `NeurochemicalAttractorControlSchema`
- `ExtremeBrainDetailSchema`

These concrete schemas already exist in `docs/canon/human-schemas/` and are re-exported via `docs/canon/human-schemas/index.js`. Any future roadmap audit should validate both:

- category/module coverage
- per-schema coverage against `HumanReplicationSchema.js`

---

## 3. Implementation Strategy

Implementation should happen in layers, from lowest-risk foundational schemas to highest-complexity emergent systems.

### Phase 0 — Stabilize Schema Authority

**Objective:** Make the schema split itself authoritative and safe to build on.

Tasks:

- ensure every categorized schema file exports only valid schemas
- align `HumanReplicationSchema.d.ts` with the split JS module structure
- remove any placeholder typing or `z.any()` that should be concrete
- define naming rules for all schema files and exported symbols
- add a schema versioning policy

Deliverables:

- stable categorized JS schema modules
- matching `.d.ts` roadmap/update plan
- schema ownership conventions

---

### Phase 1 — Example Data Packs

**Objective:** Make each schema implementable by pairing it with valid example payloads.

Tasks:

- create one minimal valid fixture per category
- create one full canonical fixture per category
- create one integrated `HumanSchema` fixture
- validate current templates against the split modules
- separate “example”, “canonical”, and “test” fixtures

Recommended output structure:

- `docs/canon/examples/identity/`
- `docs/canon/examples/psychology/`
- `docs/canon/examples/body/`
- etc.

Deliverables:

- parseable fixtures for each schema category
- one complete golden human example

---

### Phase 2 — Validation and Contract Testing

**Objective:** Guarantee every schema works independently and as part of the full human document.

Tasks:

- add per-file schema parse tests
- add integrated `HumanSchema.parse(...)` tests
- add invalid fixture tests for rejection behavior
- add compatibility tests for aggregator exports
- add snapshot tests for canonical templates

Recommended test classes:

- valid minimal object
- valid maximal object
- missing required field
- wrong enum value
- wrong nested structure
- regression fixture from previous schema versions

Deliverables:

- schema contract test suite
- regression coverage for future refactors

---

### Phase 3 — Engine Mapping

**Objective:** Map every schema category to the runtime subsystem that consumes it.

Tasks:

- map `identitySchemas` to human registry/bootstrap logic
- map `psychologySchemas` to personality and trait systems
- map `bodySchemas` to embodiment/vitals state
- map `runtimeSchemas` to sim tick/runtime state containers
- map `coreSystemsSchemas` to foundational simulation loops
- map `cognitionSchemas` to memory/attention/decision systems
- map `emotionSchemas` to emotion state machines and affect blending
- map `biologySchemas` to immune/skin/body integrity systems
- map `sensorySchemas` to perception and sensory integration layers
- map `reproductiveSchemas` to reproductive lifecycle logic
- map `brainSchemas` to advanced cognition and brain dynamics engines

Deliverables:

- schema-to-engine responsibility map
- list of consuming modules in Rust/JS

---

### Phase 4 — Incremental Runtime Implementation

**Objective:** Implement the runtime in dependency order.

Recommended order:

1. `identitySchemas`
2. `bodySchemas`
3. `runtimeSchemas`
4. `coreSystemsSchemas`
5. `psychologySchemas`
6. `cognitionSchemas`
7. `emotionSchemas`
8. `sensorySchemas`
9. `biologySchemas`
10. `reproductiveSchemas`
11. `brainSchemas`

Why this order:

- identity/body/runtime define the base object model
- core systems define the simulation backbone
- psychology/cognition/emotion define human behavior
- sensory/biology/reproduction add embodied complexity
- brain schemas are the most complex and should come after lower-level systems are stable

Deliverables:

- implemented readers/loaders for each category
- subsystem integration checkpoints after each module family

---

### Phase 5 — Canonical Templates Completion

**Objective:** Upgrade templates from partial examples to fully valid canonical humans.

Tasks:

- complete `analyticalSensitive`
- complete `intuitiveAutonomous`
- ensure both parse against full `HumanSchema`
- create template inheritance/composition rules if needed
- mark which fields are canonical, optional, generated, or runtime-only

Deliverables:

- two full validated canonical templates
- optional template generation utilities

---

### Phase 6 — Documentation and Consumer Guidance

**Objective:** Make the schema system usable by future maintainers and downstream projects.

Tasks:

- create README per schema category
- document required vs optional fields
- document runtime-only vs persisted fields
- document generation pipeline for full human objects
- document compatibility expectations for downstream consumers

Deliverables:

- schema docs by category
- implementation notes for engine developers

---

### Phase 7 — Versioning and Migration Support

**Objective:** Support long-term evolution of the schema without chaos.

Tasks:

- introduce migration rules per schema version
- define deprecation markers for fields
- add compatibility tests across versions
- create upgrade scripts for old human documents

Deliverables:

- migration policy
- version upgrade checklist

---

## 4. Category-by-Category Implementation Checklist

### `identitySchemas.js`

Concrete schemas:

- `CoreIdentitySchema`
- `DnaStrandSchema`
- `SexChromosomesSchema`
- `GeneticExpressionSchema`
- `GenomeSchema`
- `BirthChartSchema`
- `NarrativeSelfSchema`
- `IdentitySchema`
- `FullPhenotypeSchema`
- `HumanIdentitySchema`

Implement:

- canonical person identity records
- birth chart data authority
- genome encoding/decoding rules
- phenotype derivation rules
- narrative identity storage format

Needs:

- fixture humans
- genotype/phenotype consistency rules
- import/export compatibility

---

### `psychologySchemas.js`

Concrete schemas:

- `TemperamentMatrixSchema`
- `NeurocognitiveProfileSchema`
- `PersonalityTraitSchema`
- `PersonalityTraitsSchema`
- `DriveWeightsSchema`
- `HormonalBaselineBiasSchema`
- `StressResponseProfileSchema`
- `AttachmentStyleSchema`
- `RelationalDefaultsSchema`
- `IdentityAxiomsSchema`

Implement:

- temperament initialization
- trait-domain loading
- drive weighting behavior
- stress/attachment defaults
- hormonal-bias interpretation

Needs:

- trait balancing rules
- domain-specific validation fixtures
- clear behavior mapping documentation

---

### `bodySchemas.js`

Concrete schemas:

- `BodyVitalsSchema`
- `BodyPhysiologySchema`
- `BodyAppearanceSchema`
- `DnaSchema`
- `BodySchema`

Implement:

- vitals state container
- physiology state updates
- appearance storage
- DNA legacy compatibility layer

Needs:

- baseline ranges
- body initialization helpers
- integration with simulation state

---

### `runtimeSchemas.js`

Concrete schemas:

- `CurrentCognitionSchema`
- `CurrentEmotionSchema`
- `RuntimeSchema`

Implement:

- current cognition runtime state
- current emotion runtime state
- tick/runtime metadata

Needs:

- clear separation between persisted and transient fields
- tick update rules

---

### `coreSystemsSchemas.js`

Concrete schemas:

- `BioSysConfigSchema`
- `PsycheSysConfigSchema`
- `ChaosSysConfigSchema`
- `WillSysConfigSchema`
- `CoreSystemsSchema`
- `ArchitecturalLawsSchema`
- `DeterministicArchitectureSchema`
- `ImmutableStateSchema`
- `SystemDependenciesSchema`

Implement:

- biosys
- psychesys
- chaossys
- willsys
- architectural laws
- deterministic architecture
- immutable state rules
- system dependency definitions

Needs:

- law-to-engine mapping
- deterministic update tests
- fail-closed behavior tests

---

### `cognitionSchemas.js`

Concrete schemas:

- `LegacyMemorySystemsSchema`
- `MemorySystemsSchema`
- `AttentionSystemSchema`
- `ConsciousnessSchema`
- `LearningAdaptationSchema`
- `SocialCognitionSchema`
- `CreativeSystemsSchema`
- `DecisionMakingSchema`

Implement:

- legacy and modern memory systems
- attention system
- consciousness model
- learning/adaptation model
- social cognition
- creativity
- decision making

Needs:

- staged implementation to avoid overbuilding
- runtime metrics for each cognitive subsystem

---

### `emotionSchemas.js`

Concrete schemas:

- `ComprehensiveEmotionTaxonomySchema`
- `GranularEmotionsSchema`
- `DarkTriadSchema`

Implement:

- comprehensive taxonomy ingestion
- granular emotion state usage
- dark triad behavioral hooks

Needs:

- emotion blending rules
- emotion decay rules
- links to physiology/hormones/stress

---

### `biologySchemas.js`

Concrete schemas:

- `ImmuneSystemSchema`
- `SkinSystemSchema`

Implement:

- immune simulation state
- skin integrity/healing state

Needs:

- health event triggers
- healing/infection progression rules

---

### `sensorySchemas.js`

Concrete schemas:

- `SensorySystemsSchema`

Implement:

- tactile
- proprioception
- visual
- auditory
- vestibular
- interoception
- sensory integration

Needs:

- coordinate-space conventions
- event ingestion interfaces
- performance boundaries for high-detail inputs

---

### `reproductiveSchemas.js`

Concrete schemas:

- `ReproductiveSystemsSchema`

Implement:

- sexual state
- reproductive cycle state
- genetics/reproduction tracking
- mate selection logic

Needs:

- ethical/data handling boundaries
- lifecycle transitions
- interaction rules with hormones/body systems

---

### `brainSchemas.js`

Concrete schemas:

- `FunctionalBrainRegionsSchema`
- `NeuralPopulationDynamicsSchema`
- `NeurotransmitterModulationSchema`
- `AdvancedMemorySystemsSchema`
- `PredictiveProcessingSchema`
- `MesoscaleBrainEngineSchema`
- `FormalPredictiveProcessingSchema`
- `NeurochemicalAttractorControlSchema`
- `ExtremeBrainDetailSchema`

Implement:

- functional brain region state
- neural population dynamics
- neurotransmitter modulation
- advanced memory layers
- predictive processing
- mesoscale brain engine
- attractor control
- integration/emergent properties

Needs:

- heavy phasing and feature flags
- fallback simplified models
- strict performance budgeting
- deterministic math tests

---

## 5. Recommended Milestones

### Milestone A — Schema Contracts Complete

- categorized modules stable
- tests passing
- example fixtures available

### Milestone B — Foundational Runtime Online

- identity/body/runtime/core systems implemented
- full human object can load and validate

### Milestone C — Behavioral Runtime Online

- psychology/cognition/emotion systems implemented
- human behavior begins responding to state

### Milestone D — Embodied Runtime Online

- biology/sensory/reproduction integrated
- body and perception influence cognition

### Milestone E — Advanced Brain Runtime Online

- brain schemas implemented behind controlled rollout
- advanced cognition models stable enough for experimentation

---

## 6. Risks

Key risks to manage:

- schema detail exceeds realistic runtime capacity
- templates remain partial and block validation
- `.d.ts` and `.js` drift apart
- runtime fields and persisted fields become mixed
- advanced brain/emotion models are implemented before core system stability

Mitigations:

- implement by layer, not all at once
- require fixtures and tests before engine hookup
- keep `HumanReplicationSchema.js` as public compatibility layer
- flag experimental subsystems

---

## 7. Recommended Next Actions

Immediate next steps:

1. split and align `HumanReplicationSchema.d.ts` to match the categorized JS modules
2. create minimal valid fixtures for every schema category
3. create one full golden `HumanSchema` fixture
4. add contract tests for every categorized module
5. document which engine subsystem owns each schema file

---

## 8. Current Implementation Status Snapshot

This section reflects the current repository state after the initial schema-alignment fixes that landed alongside this roadmap.

### Completed

- categorized JS schema modules exist under `docs/canon/human-schemas/`
- `docs/canon/HumanReplicationSchema.js` remains the public aggregation entrypoint
- Rust canonical schema representation exists in `crates/mk_core/src/human/schema.rs`
- Rust profile/runtime mapping exists in `crates/mk_core/src/human/profile.rs`
- initial integrated example fixtures now exist under `docs/canon/examples/humans/`
- initial Rust contract tests now exist in:
  - `crates/mk_core/tests/human_schema_contracts.rs`
  - `crates/mk_core/tests/human_schema_templates.rs`
- schema-to-engine ownership documentation now exists in:
  - `docs/canon/SCHEMA_TO_ENGINE_OWNERSHIP_MAP.md`

### Partial

- `docs/canon/HumanReplicationSchema.d.ts` exists but still needs a full cleanup/alignment pass against the split JS module surface
- `docs/canon/HumanReplicationSchema.js` has corrected top-level accessors, but the bundled example templates remain partial examples rather than complete end-to-end canonical templates
- integrated example coverage currently exists at the full-human level, but category-by-category minimal/canonical/invalid fixtures are still incomplete
- runtime use of many schema families remains partial even though canonical data structures exist

### Missing or Still Needed

- one minimal fixture per schema category
- one canonical/full fixture per schema category
- invalid fixture coverage per category
- end-to-end validation of the JS templates against the full JS schema surface
- stronger consumer guidance around persisted vs runtime-only fields
- versioning/migration support artifacts

### Deferred Runtime Work

The following remain primarily data-contract level and are not yet fully implemented as runtime systems:

- biology schema runtime behavior
- sensory schema runtime behavior
- reproductive schema runtime behavior
- advanced brain-detail runtime behavior

These are not schema-authority failures; they are downstream implementation backlog.

---

## 9. Suggested Execution Order for This Repository

If implementation starts now, use this order:

1. `identitySchemas.js`
2. `bodySchemas.js`
3. `runtimeSchemas.js`
4. `coreSystemsSchemas.js`
5. `psychologySchemas.js`
6. `cognitionSchemas.js`
7. `emotionSchemas.js`
8. `sensorySchemas.js`
9. `biologySchemas.js`
10. `reproductiveSchemas.js`
11. `brainSchemas.js`
12. complete template fixtures
13. align `.d.ts`
14. add integration tests

---

## 10. Definition of Done

The schema implementation effort is complete when:

- all categorized schemas parse valid fixtures
- all schemas reject invalid fixtures predictably
- full `HumanSchema` templates validate end-to-end
- every schema category has a documented engine owner
- the `.d.ts` surface matches the JS modules
- downstream consumers can import either the categorized modules or the aggregator safely
