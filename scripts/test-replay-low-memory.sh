#!/usr/bin/env bash
# Low-memory, offline replay regression runner. Never clears caches or swap.
set -euo pipefail
cd "$(dirname "$0")/.."
export CARGO_BUILD_JOBS=1
export CARGO_INCREMENTAL=0
export CARGO_PROFILE_DEV_DEBUG=0
export CARGO_PROFILE_DEV_PACKAGE_MK_ENGINE_OPT_LEVEL=0
export CARGO_PROFILE_DEV_PACKAGE_MK_CORE_OPT_LEVEL=0
export CARGO_PROFILE_DEV_PACKAGE_MK_ISLAND_OPT_LEVEL=0
# Avoid changing RUSTFLAGS here: doing so invalidates Cargo's existing dependency
# artifacts and can trigger a complete rebuild on memory-constrained hosts.
# The profile debug setting above controls debug info without that override.
# Exit 124 is a timeout, not a passing test.
timeout "${REPLAY_TEST_TIMEOUT:-900}" cargo test --offline --locked -p mk_engine --lib replay -- --test-threads=1
