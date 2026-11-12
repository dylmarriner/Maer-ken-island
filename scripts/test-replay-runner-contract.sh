#!/usr/bin/env bash
# Validate runner argument/environment contract without compiling the Rust workspace.
set -euo pipefail
cd "$(dirname "$0")/.."
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
cat > "$tmp/cargo" <<'CARGO'
#!/usr/bin/env bash
set -euo pipefail
[[ "$*" == 'test --offline --locked -p mk_engine --lib replay -- --test-threads=1' ]]
[[ "$CARGO_BUILD_JOBS" == 1 && "$CARGO_INCREMENTAL" == 0 ]]
[[ "${CARGO_PROFILE_DEV_DEBUG:-}" == "" ]]
printf 'cargo contract passed\n'
CARGO
chmod +x "$tmp/cargo"
PATH="$tmp:$PATH" REPLAY_TEST_TIMEOUT=5 REPLAY_REDUCED_DEBUG=0 bash scripts/test-replay-low-memory.sh | grep -q 'cargo contract passed'
if PATH="$tmp:$PATH" REPLAY_REDUCED_DEBUG=invalid bash scripts/test-replay-low-memory.sh >"$tmp/out" 2>&1; then
  echo 'Invalid debug mode unexpectedly accepted' >&2
  exit 1
fi
grep -q 'REPLAY_REDUCED_DEBUG must be 0 or 1' "$tmp/out"
# Reduced-debug mode is explicitly opt-in and must reach Cargo.
sed 's/\[\[ "${CARGO_PROFILE_DEV_DEBUG:-}" == "" \]\]/[[ "${CARGO_PROFILE_DEV_DEBUG:-}" == 0 ]]/' "$tmp/cargo" > "$tmp/cargo.debug"
cp "$tmp/cargo.debug" "$tmp/cargo"
chmod +x "$tmp/cargo"
PATH="$tmp:$PATH" REPLAY_TEST_TIMEOUT=5 REPLAY_REDUCED_DEBUG=1 bash scripts/test-replay-low-memory.sh | grep -q 'cargo contract passed'
# Reject timeout values beyond the documented bound before running Cargo.
if PATH="$tmp:$PATH" REPLAY_TEST_TIMEOUT=86401 bash scripts/test-replay-low-memory.sh >"$tmp/out" 2>&1; then
  echo 'Out-of-range timeout unexpectedly accepted' >&2
  exit 1
fi
grep -q 'REPLAY_TEST_TIMEOUT must be an integer' "$tmp/out"
# Zero, signed, and padded timeouts must fail before invoking Cargo.
for invalid in 0 -1 +1 01 1s; do
  if PATH="$tmp:$PATH" REPLAY_TEST_TIMEOUT="$invalid" bash scripts/test-replay-low-memory.sh >"$tmp/out" 2>&1; then
    echo "Invalid timeout unexpectedly accepted: $invalid" >&2
    exit 1
  fi
  grep -q 'REPLAY_TEST_TIMEOUT must be an integer' "$tmp/out"
done
# Confirm timeout status is preserved rather than misreported as a successful replay.
cat > "$tmp/cargo" <<'SLOW_CARGO'
#!/usr/bin/env bash
sleep 3
SLOW_CARGO
chmod +x "$tmp/cargo"
set +e
PATH="$tmp:$PATH" REPLAY_TEST_TIMEOUT=1 REPLAY_REDUCED_DEBUG=0 bash scripts/test-replay-low-memory.sh >"$tmp/out" 2>&1
timeout_status=$?
set -e
if [[ "$timeout_status" != 124 ]]; then
  echo "Expected replay timeout exit 124, got $timeout_status" >&2
  exit 1
fi
echo 'runner contract tests passed'
