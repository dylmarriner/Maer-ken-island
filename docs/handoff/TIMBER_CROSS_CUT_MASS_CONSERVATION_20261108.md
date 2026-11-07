# Timber cross-cut mass conservation

Date: 2026-11-08  
Repository: `dylmarriner/Maer-ken-island`  
Branch: `fix/timber-cross-cut-exact-conservation-20261108`  
Base: `main`

## Finding

`Log::cross_cut` divided dry wood, carbon, water and length into `n` identical
binary64 shares. Repeated rounded shares do not always sum to the original value.
For example, three shares of `914.6308346979964` kg sum one ULP above the
source; a length of `7.123456789012345` m sums one ULP below it. The cross-cut
operation therefore could create or lose a tiny amount of material in the
ledger even though it is only meant to partition an existing log.

## Change

- Keep the first `n - 1` pieces at the equal share.
- Assign the final piece the remaining dry mass, carbon, water and length.
- Derive each piece's centre from the accumulated preceding length, so the
  final geometry uses the same residual rather than assuming every rounded
  piece has identical length.
- No source parameters, species data, snapshot fields, replay format, RNG,
  single-writer ownership or LLM behaviour changed.

## Regression

`cross_cutting_assigns_floating_point_remainders_without_losing_material`
uses non-exactly-divisible dry mass and length values. It compares the summed
piece totals to the original values by `to_bits()`, not a loose tolerance, for
dry mass, carbon, water and length.

## Commits

- Production: `fa81e5ba92a69b3ef414d8a75faab1f896798881`
- Regression: `3aa0c6bdf56842debc62671be0c501aad8ddf344`

## Verification

No local Cargo result is claimed because custom OpenCommander device discovery
is currently returning HTTP 429. Exact-head CI must pass formatting, Clippy,
fast Rust tests, desktop/backend acceptance and week/year reference-pack tests.
The week/year digest must not be changed merely to accommodate this precision
correction; any digest difference must be audited against the simulation path
before a deliberate re-pin.

No live island state, production infrastructure, deployment or irreversible
data operation was changed.
