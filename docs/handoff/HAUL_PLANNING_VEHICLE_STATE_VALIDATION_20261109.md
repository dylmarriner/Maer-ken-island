# Haul-planning vehicle-state validation — 9 November 2026

Repository: `dylmarriner/Maer-ken-island`
Branch: `feat/d40-two-person-saw-ute-crosscut-20261109`
PR: [#136](https://github.com/dylmarriner/Maer-ken-island/pull/136)

## Finding

`haul_plans_over` is a public planning boundary and accepts public `RoadVehicle` and
`TrackedMachine` values. It previously validated the timber log but trusted vehicle IDs,
fuel quantities and consumption rates. A malformed negative burn rate could make a vehicle
appear to gain fuel during a haul; non-finite fuel or rates could also flow into eligibility
comparisons without a typed failure.

The estate's current vehicle factory supplies sourced positive rates, but the public planner
must not rely on every caller or restored state having passed through that factory.

## Production correction

- Added `LabourError::InvalidVehicleState` with a specific diagnostic.
- Before planning, validate every supplied road vehicle and tracked machine:
  - item ID is non-zero;
  - fuel litres are finite and non-negative (zero remains a valid empty tank);
  - burn rate is finite and strictly positive.
- Invalid supplied fleet state fails before task construction or material-ID reservation.
- Absent vehicles remain valid; a vehicle with zero fuel is valid but cannot produce a powered haul plan.

## Regression coverage

`haul_planning_rejects_malformed_road_and_tracked_vehicle_state` covers zero IDs, negative,
NaN and infinite fuel, plus zero, negative, NaN and infinite consumption rates for both
vehicle classes. Existing empty-tank coverage remains in place.

## Commits

- Typed error and production guard: `8947bfe8bd5c686670a161b5fba86120a32ee215`.
- Regression suite: `b50d4d4278e56edebf08bcf2b2d630efd65a7d12`.

## Verification

- Exact-head CI must run Rust formatting, Clippy, fast tests, desktop/backend acceptance and
  week/year reference packs after these commits.
- The preceding source-equivalent D40 workflow [#42569859195](https://github.com/dylmarriner/Maer-ken-island/actions/runs/42569859195)
  passed formatting, Clippy, fast tests, desktop build/tests/render/input/frame-cost checks,
  installed backend, web dashboard, computer service, repository scripts, dependency audit
  and both reference packs. It does not verify this new vehicle-state guard.
- No local Cargo result is claimed; custom OpenCommander device discovery is still returning
  HTTP 429 in this run.

## Invariants and remaining work

No route speed, fuel-consumption coefficient, snapshot schema, append-only log, replay hash,
RNG, simulation cadence, material ledger or reference pack changed. No deployment or live
simulation mutation occurred.

D40 terrain-aware Ute routing and persistent vehicle positions remain open. This validation
is a separate fail-fast boundary, not a claim that route modelling is complete.
