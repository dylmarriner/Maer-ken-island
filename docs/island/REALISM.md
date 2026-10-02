# Realism standard

Maer-Ken Island is a realistic simulation of a planet and its people. This standard applies to
every phase from 0c onward; reviewers reject changes that do not meet it.

## 1. Every number has a source

Every physical, biological, economic or behavioural parameter added to the code cites where it
comes from, next to the constant:

```rust
/// Kleiber's law exponent for basal metabolic rate vs body mass.
/// Source: Kleiber 1932, Hilgardia 6:315; West et al. 1997, Science 276:122.
pub const KLEIBER_EXPONENT: f64 = 0.75;
```

A value derived from others shows the derivation instead (`g·R²/G`). A value with no published
basis is a deviation (section 4), not a silent guess.

## 2. Every system is validated against reality

Each subsystem has a validation test that compares its output with real-world reference data
from `fixtures/reference/` (Phase 0c Task 3), states its tolerance, and names its source. Slow
validations run in the slow tier.

The planet is not Earth, so reference data is **translated by physics, not copied**:

- Time constants come from the island canon (`fixtures/island/canon.json`): the local day is
  36 hours and the year 288.19 local days (432.3 Earth days). Seasonal statistics are compared per
  season, not per calendar month.
- Biology runs in SI seconds. Human and animal physiology, ageing, gestation and growth use real
  durations whatever the planet's day or year.
- Atmospheric and ocean dynamics are compared through dimensionless or rotation-scaled quantities
  (Rossby number, deformation radius, Eady growth rate), because the planet rotates once every
  36 hours rather than 24.
- Quantities that do not depend on the planet (lapse rate, allometry, metabolic cost of work,
  turbine efficiency, ore grades) are compared directly.

## 3. No game shortcuts

The simulation never:

- regenerates anything without a physical cause (resources follow biomass, deposits deplete);
- completes an action instantly (work takes time and energy);
- runs a machine without power or fuel;
- creates or destroys carbon, oxygen, water, methane or money (ledgers close every step);
- moves anything across the domain edge except through the documented boundary forcing;
- hides a limit (every known limit is in the deviation register).

## 4. The deviation register

`docs/island/DEVIATIONS.md` lists every known departure from reality: what reality does, what the
model does, the expected error, why, the phase that owns it, and its status:

- **Open**: a phase plans to remove it.
- **Accepted**: the owner has chosen it (e.g. the density exception) or it is a permanent limit
  of the resolution or scope; it stays documented.
- **Resolved**: removed; the row stays as history with the commit that resolved it.

A test (`crates/mk_core/tests/deviation_register.rs`) fails if any row lacks an owning phase or a
valid status, so the register cannot silently rot.

## 5. Where human behaviour is wrong, fix it upstream

When the human runtime disagrees with real physiology or behaviour, the fix goes to
`dylmarriner/Maer-Ken` first and is synced, so the island never runs a second human model.
