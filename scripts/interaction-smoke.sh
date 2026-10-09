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
set -eu

UI=${1:-target/release/island-ui}
OUT=${2:-/tmp/island-ui-interaction}
DISPLAY_NUM=${ISLAND_INPUT_DISPLAY:-:89}
SETTLE=${ISLAND_INPUT_SETTLE:-90}
# Software rasterising 285 chunks takes a moment per frame, so a gesture
# needs time to be drawn before it is photographed.
REDRAW=${ISLAND_INPUT_REDRAW:-12}

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
# this is the orbit. 480 px is a big gesture on purpose -- a small one
# could be lost in rounding and prove nothing.
DISPLAY="$DISPLAY_NUM" xdotool mousemove 800 500
DISPLAY="$DISPLAY_NUM" xdotool mousedown 1
DISPLAY="$DISPLAY_NUM" xdotool mousemove_relative -- 480 0
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

# The panel. "the estate" sits under "Looking at" in the left panel; the
# coordinates come from the rendered frame rather than from the layout
# code, which is the point -- if the panel moves, this stops finding it
# and the assertion below fails rather than passing silently.
DISPLAY="$DISPLAY_NUM" xdotool mousemove 182 224 click 1
sleep "$REDRAW"
alive "a click on the estate button"
shot 03-estate

python3 - "$OUT" "$(changed 00-opened 01-dragged)" "$(changed 01-dragged 02-zoomed)" \
        "$(changed 02-zoomed 03-estate)" <<'PY'
import sys

out, dragged, zoomed, estate = sys.argv[1], *(float(x) for x in sys.argv[2:5])
print(f"a drag moved {dragged:.2f}% of the pixels")
print(f"a scroll moved {zoomed:.2f}% of the pixels")
print(f"clicking 'the estate' moved {estate:.2f}% of the pixels")

# Thresholds well below what each gesture actually does and well above
# nothing. The failure being caught is "input arrives and the picture
# does not change at all", not a particular sensitivity.
problems = []
if dragged < 1.0:
    problems.append(f"a 480 px drag changed {dragged:.2f}% of the frame; the camera is not orbiting")
if zoomed < 1.0:
    problems.append(f"five scroll notches changed {zoomed:.2f}% of the frame; the camera is not zooming")
if estate < 5.0:
    problems.append(f"clicking 'the estate' changed {estate:.2f}% of the frame; the panel is not answering")

if problems:
    print("the desktop application is not answering a mouse:", file=sys.stderr)
    for problem in problems:
        print(f"  - {problem}", file=sys.stderr)
    print(f"frames are in {out}", file=sys.stderr)
    sys.exit(1)
print("the desktop application answers a mouse")
PY
