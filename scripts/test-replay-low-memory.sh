#!/usr/bin/env bash
# Low-memory, offline replay regression runner. Never clears caches or swap.
set -euo pipefail
cd "$(dirname "$0")/.."
export CARGO_BUILD_JOBS=1
export CARGO_INCREMENTAL=0
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
timeout --signal=TERM --kill-after=15s "$test_timeout" cargo test --offline --locked -p mk_engine --lib replay -- --test-threads=1
