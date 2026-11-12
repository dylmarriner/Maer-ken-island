#!/usr/bin/env bash
# Low-memory, offline replay regression runner. Never clears caches or swap.
set -euo pipefail
cd "$(dirname "$0")/.."
# Force bounded compilation even when inherited environment requests high parallelism.
export CARGO_BUILD_JOBS=1
export CARGO_INCREMENTAL=0
# Keep compiler flags untouched to preserve cached fingerprints.
# Preserve existing Cargo profile fingerprints by default to reuse compiled artifacts.
# Set REPLAY_REDUCED_DEBUG=1 only for a fresh isolated build.
if [[ "${REPLAY_REDUCED_DEBUG:-0}" == 1 ]]; then
  export CARGO_PROFILE_DEV_DEBUG=0
fi
# Avoid changing RUSTFLAGS here: doing so invalidates Cargo's existing dependency
# artifacts and can trigger a complete rebuild on memory-constrained hosts.
# Explicit reduced-debug mode is opt-in because profile changes also force rebuilds.
# Exit 124 is a timeout, not a passing test.
test_timeout="${REPLAY_TEST_TIMEOUT:-900}"
if ! [[ "$test_timeout" =~ ^[1-9][0-9]*$ ]] || (( ${#test_timeout} > 8 )) || (( 10#$test_timeout > 86400 )); then
  printf 'REPLAY_TEST_TIMEOUT must be an integer between 1 and 86400 seconds\n' >&2
  exit 2
fi
# Explicitly validate the opt-in mode: a typo must not silently trigger a
# different compilation profile or produce misleading performance evidence.
if [[ "${REPLAY_REDUCED_DEBUG:-0}" != 0 && "${REPLAY_REDUCED_DEBUG:-0}" != 1 ]]; then
  printf 'REPLAY_REDUCED_DEBUG must be 0 or 1\n' >&2
  exit 2
fi
if ! command -v timeout >/dev/null 2>&1 || ! command -v cargo >/dev/null 2>&1; then
  printf 'Replay runner requires timeout and cargo on PATH\n' >&2
  exit 127
fi
# Capture the actual exit status while preserving set -e semantics.
# A timeout is an infrastructure failure, never replay verification.
status=0
timeout --signal=TERM --kill-after=15s "$test_timeout" cargo test --offline --locked -p mk_engine --lib replay -- --test-threads=1 || status=$?
if (( status == 124 )); then
  printf 'Replay verification INCOMPLETE: Cargo exceeded %s seconds (exit 124). No replay/hash claim is valid.\n' "$test_timeout" >&2
fi
exit "$status"
