#!/bin/sh
# Does `island-ui` answer a mouse?
#
#   scripts/interaction-smoke.sh [path/to/island-ui] [out-dir]
#
# `render-smoke.sh` photographs one settled frame and proves the
# application draws. It says nothing about whether anything *responds*:
# a window that renders a correct first frame and then ignores every
# click looks identical to a working one in a screenshot.
#
# So this drives the thing with synthetic X11 input -- a drag, a scroll,
# a click on a panel button -- and asserts the picture changed in the way
# that gesture should change it, and that the application was still alive
# afterwards. It needs Xvfb, xdotool and ImageMagick.
#
# What it cannot answer is whether the sensitivities feel right under a
# real hand. That is a judgement, not a measurement, and no script
# settles it. What it does answer is the failure one step below that:
# input arriving and nothing happening.
#
# Two things the first version of this script got wrong, both found by
# running it, and both worth keeping written down:
#
#   * **A gesture must span frames.** `drive_the_camera` samples
#     `buttons.pressed(MouseButton::Left)` once per frame. An xdotool
#     mousedown-move-up completes in milliseconds, so on a software
#     rasteriser drawing the island at 0.6 FPS the whole drag landed
#     inside one 1.7-second frame: Bevy saw press and release in the
#     same update, `pressed()` was false, and the motion was discarded.
#     The drag below is therefore held down across several sleeps.
#   * **Panel coordinates depend on the panel's contents.** The button
#     was clicked at a y measured from a screenshot of a *remote* run.
#     An embedded one adds a line -- "Running in this process, on
#     loopback." -- under the address, which pushes everything below it
#     down and put "the island" where "the estate" had been. The click
#     landed on the already-selected button and changed 0.32% of the
#     frame, which is what a hover highlight costs.
#
# The second is why the view switch is now driven by the `e` key as well
# as by the button: the key cannot drift with the layout, and the button
# click still has to work for the assertion below to pass.
set -eu

UI=${1:-target/release/island-ui}
OUT=${2:-/tmp/island-ui-interaction}
DISPLAY_NUM=${ISLAND_INPUT_DISPLAY:-:89}
SETTLE=${ISLAND_INPUT_SETTLE:-90}
# Software rasterising 285 chunks takes a moment per frame, so a gesture
# needs time to be drawn before it is photographed. Measured, the island
# view runs at 0.6 FPS under lavapipe -- 1.7 s a frame -- so twelve
# seconds is about seven frames, and a gesture held for less than one of
# them is not sampled at all.
REDRAW=${ISLAND_INPUT_REDRAW:-12}
# How long each step of a held drag waits. It has to exceed one frame or
# the button is down and up again between samples; see the note above.
HOLD=${ISLAND_INPUT_HOLD:-3}

missing=""
for tool in Xvfb xdotool convert compare; do
    command -v "$tool" >/dev/null 2>&1 || missing="$missing $tool"
done
[ -e /usr/share/vulkan/icd.d/lvp_icd.x86_64.json ] || [ -e /usr/share/vulkan/icd.d/lvp_icd.json ] \
    || missing="$missing lavapipe(mesa-vulkan-drivers)"

if [ "${1:-}" = "--check-tools" ]; then
    if [ -n "$missing" ]; then
        echo "missing:$missing" >&2
        echo "  apt-get install -y --no-install-recommends xvfb xdotool imagemagick \\"
        echo "      mesa-vulkan-drivers libxkbcommon-x11-0"
        exit 1
    fi
    echo "everything this needs is here"
    exit 0
fi
if [ -n "$missing" ]; then
    echo "interaction smoke test cannot run; missing:$missing" >&2
    exit 1
fi
[ -x "$UI" ] || { echo "no island-ui at $UI -- build it first" >&2; exit 1; }

root=$(cd "$(dirname "$0")/.." && pwd)
data=$(mktemp -d)
mkdir -p "$OUT"
cleanup() {
    [ -n "${APP:-}" ] && kill "$APP" 2>/dev/null || true
    [ -n "${XVFB:-}" ] && kill "$XVFB" 2>/dev/null || true
    rm -rf "$data"
}
trap cleanup EXIT INT TERM

Xvfb "$DISPLAY_NUM" -screen 0 1600x1000x24 >"$data/xvfb.log" 2>&1 &
XVFB=$!
sleep 2

DISPLAY="$DISPLAY_NUM" "$UI" \
    --scenario "$root/fixtures/island/default_scenario.json" \
    --data-dir "$data/people" \
    --speed max >"$data/app.log" 2>&1 &
APP=$!
sleep "$SETTLE"

alive() {
    kill -0 "$APP" 2>/dev/null && return 0
    echo "island-ui exited during: $1" >&2
    tail -30 "$data/app.log" >&2
    exit 1
}
shot() { DISPLAY="$DISPLAY_NUM" import -window root "$OUT/$1.png"; }

# How different two frames are, as a percentage of pixels. `compare`
# writes the count to stderr and exits non-zero when they differ, which
# is the ordinary case here, so its status is deliberately ignored.
changed() {
    differing=$(compare -metric AE "$OUT/$1.png" "$OUT/$2.png" null: 2>&1 >/dev/null || true)
    differing=${differing%%.*}
    echo "$differing" | awk '{ printf "%.2f", 100 * $1 / (1600 * 1000) }'
}

alive "settling"
shot 00-opened

# A left-drag across the middle of the window, away from either panel:
# this is the orbit. 480 px is a big gesture on purpose, and it is made
# in steps with the button held down between them, because the camera
# only sees a drag that is still held when a frame is sampled.
DISPLAY="$DISPLAY_NUM" xdotool mousemove 800 500
DISPLAY="$DISPLAY_NUM" xdotool mousedown 1
sleep "$HOLD"
for _ in 1 2 3 4; do
    DISPLAY="$DISPLAY_NUM" xdotool mousemove_relative -- 120 0
    sleep "$HOLD"
done
DISPLAY="$DISPLAY_NUM" xdotool mouseup 1
sleep "$REDRAW"
alive "a drag"
shot 01-dragged

# Five notches of scroll: the zoom, which is exponential, so five is a
# visible step rather than a nudge.
DISPLAY="$DISPLAY_NUM" xdotool click --repeat 5 4
sleep "$REDRAW"
alive "a scroll"
shot 02-zoomed

# The keyboard route to the estate, which cannot drift with the panel's
# layout: `e` picks the estate frame, `i` the island.
DISPLAY="$DISPLAY_NUM" xdotool key e
sleep "$REDRAW"
alive "the e key"
shot 03-estate

# And back to the island with `i`, so the switch is shown to work both
# ways rather than once.
DISPLAY="$DISPLAY_NUM" xdotool key i
sleep "$REDRAW"
alive "the i key"
shot 04-island

# Now the panel's own button, which is the thing a person actually
# clicks. "the estate" sits under "Looking at" in the left panel, and
# its y depends on what is above it -- an embedded island prints a line
# a remote one does not. So the button is found in the frame rather
# than assumed: it is the one highlighted word on that line, and the
# highlight is the panel's selection blue.
button_y=$(python3 - "$OUT/04-island.png" <<'FIND'
import subprocess, sys

# The selected-tab highlight egui draws is a saturated blue; the rest of
# the panel is grey. Scan the left panel for the row holding the most of
# it: that is the "Looking at" line, and the unselected button sits on
# the same row.
raw = subprocess.run(["convert", sys.argv[1], "-crop", "320x400+0+150", "-depth", "8", "rgb:-"],
                     capture_output=True, check=True).stdout
width = 320
best, best_row = 0, None
for row in range(len(raw) // (3 * width)):
    start = row * width * 3
    blue = sum(
        1
        for x in range(width)
        if raw[start + 3 * x + 2] > 120
        and raw[start + 3 * x + 2] > raw[start + 3 * x] + 40
        and raw[start + 3 * x + 2] > raw[start + 3 * x + 1] + 20
    )
    if blue > best:
        best, best_row = blue, row
print(150 + best_row if best_row is not None else 242)
FIND
)
echo "the 'Looking at' row is at y=$button_y"
# "the estate" is the second word on that row; "the island" is the first
# and is the highlighted one, so the estate's button is to its right.
DISPLAY="$DISPLAY_NUM" xdotool mousemove 182 "$button_y" click 1
sleep "$REDRAW"
alive "a click on the estate button"
shot 05-clicked

python3 - "$OUT" "$(changed 00-opened 01-dragged)" "$(changed 01-dragged 02-zoomed)" \
        "$(changed 02-zoomed 03-estate)" "$(changed 03-estate 04-island)" \
        "$(changed 04-island 05-clicked)" <<'PY'
import sys

out = sys.argv[1]
dragged, zoomed, to_estate, back, clicked = (float(x) for x in sys.argv[2:7])
print(f"a held drag moved {dragged:.2f}% of the pixels")
print(f"a scroll moved {zoomed:.2f}% of the pixels")
print(f"the e key moved {to_estate:.2f}% of the pixels")
print(f"the i key moved {back:.2f}% of the pixels")
print(f"clicking 'the estate' moved {clicked:.2f}% of the pixels")

# Thresholds well below what each gesture actually does and well above
# nothing. The failure being caught is "input arrives and the picture
# does not change at all", not a particular sensitivity.
problems = []
if dragged < 1.0:
    problems.append(f"a held 480 px drag changed {dragged:.2f}% of the frame; the camera is not orbiting")
if zoomed < 1.0:
    problems.append(f"five scroll notches changed {zoomed:.2f}% of the frame; the camera is not zooming")
if to_estate < 5.0:
    problems.append(f"the e key changed {to_estate:.2f}% of the frame; the view is not switching")
if back < 5.0:
    problems.append(f"the i key changed {back:.2f}% of the frame; the view does not switch back")
if clicked < 5.0:
    problems.append(f"clicking 'the estate' changed {clicked:.2f}% of the frame; the panel is not answering")

if problems:
    print("the desktop application is not answering a mouse:", file=sys.stderr)
    for problem in problems:
        print(f"  - {problem}", file=sys.stderr)
    print(f"frames are in {out}", file=sys.stderr)
    sys.exit(1)
print("the desktop application answers a mouse and a keyboard")
PY
