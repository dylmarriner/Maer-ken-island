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
# Boundary timeout values must remain accepted; the fake Cargo exits immediately.
cat > "$tmp/cargo" <<'BOUNDARY_CARGO'
#!/usr/bin/env bash
set -euo pipefail
[[ "$*" == 'test --offline --locked -p mk_engine --lib replay -- --test-threads=1' ]]
printf 'boundary accepted\n'
BOUNDARY_CARGO
chmod +x "$tmp/cargo"
for valid in 1 86400; do
  PATH="$tmp:$PATH" REPLAY_TEST_TIMEOUT="$valid" REPLAY_REDUCED_DEBUG=0 bash scripts/test-replay-low-memory.sh >"$tmp/out" 2>&1
  grep -q 'boundary accepted' "$tmp/out"
done
# Cargo exit code 127 must remain distinguishable from timeout exit 124.
cat > "$tmp/cargo" <<'MISSING_CARGO'
#!/usr/bin/env bash
exit 127
MISSING_CARGO
chmod +x "$tmp/cargo"
set +e
PATH="$tmp:$PATH" REPLAY_TEST_TIMEOUT=5 REPLAY_REDUCED_DEBUG=0 bash scripts/test-replay-low-memory.sh >"$tmp/out" 2>&1
missing_status=$?
set -e
if [[ "$missing_status" != 127 ]]; then
  echo "Expected Cargo failure 127, got $missing_status" >&2
  exit 1
fi
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
# A failing Rust test must propagate its status, not be mistaken for success.
cat > "$tmp/cargo" <<'FAIL_CARGO'
#!/usr/bin/env bash
exit 42
FAIL_CARGO
chmod +x "$tmp/cargo"
set +e
PATH="$tmp:$PATH" REPLAY_TEST_TIMEOUT=5 REPLAY_REDUCED_DEBUG=0 bash scripts/test-replay-low-memory.sh >"$tmp/out" 2>&1
failure_status=$?
set -e
if [[ "$failure_status" != 42 ]]; then
  echo "Expected Cargo failure exit 42, got $failure_status" >&2
  exit 1
fi
# Verify the timeout runner passes a literal numeric deadline and termination policy.
# A fake timeout intercepts argv without launching Cargo or waiting 86400 seconds.
cat > "$tmp/timeout" <<'FAKE_TIMEOUT'
#!/usr/bin/env bash
set -euo pipefail
[[ "$1" == '--signal=TERM' && "$2" == '--kill-after=15s' && "$3" == 86400 ]]
shift 3
[[ "$1" == cargo && "$2" == test ]]
printf 'timeout invocation verified\n'
FAKE_TIMEOUT
chmod +x "$tmp/timeout"
PATH="$tmp:$PATH" REPLAY_TEST_TIMEOUT=86400 REPLAY_REDUCED_DEBUG=0 bash scripts/test-replay-low-memory.sh >"$tmp/out" 2>&1
grep -q 'timeout invocation verified' "$tmp/out"
# Inherited settings must not override the low-memory safety contract.
cat > "$tmp/cargo" <<'INHERITED_CARGO'
#!/usr/bin/env bash
set -euo pipefail
[[ "$CARGO_BUILD_JOBS" == 1 && "$CARGO_INCREMENTAL" == 0 ]]
printf 'inherited limits overridden\n'
INHERITED_CARGO
chmod +x "$tmp/cargo"
rm -f "$tmp/timeout"
PATH="$tmp:$PATH" CARGO_BUILD_JOBS=64 CARGO_INCREMENTAL=1 REPLAY_TEST_TIMEOUT=5 REPLAY_REDUCED_DEBUG=0 bash scripts/test-replay-low-memory.sh >"$tmp/out" 2>&1
grep -q 'inherited limits overridden' "$tmp/out"
echo 'runner contract tests passed'
