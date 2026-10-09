#!/bin/sh
# Does `island-ui` actually draw the island?
#
#   scripts/render-smoke.sh [path/to/island-ui] [frame.png] [island|estate]
#
# Runs the desktop application on a virtual screen against an island it
# starts itself, photographs a frame, and asserts the picture is of
# something. It needs Xvfb, a software Vulkan driver (lavapipe, from
# `mesa-vulkan-drivers`) and ImageMagick; `--check-tools` reports what is
# missing without running anything.
#
# This exists because the first frame this application ever drew was
# nearly black, and nothing in the unit tests could have told me. Two real
# defects were in it:
#
#   * every terrain triangle was wound so its geometric normal pointed
#     down, so all 285 chunks were backface-culled from a camera above
#     them;
#   * `bevy_egui` was taken with `default-features = false`, which drops
#     `default_fonts`, so egui had no font data and every panel drew as an
#     empty box.
#
# The thresholds below are set from those two frames, measured:
#
#   | frame   | colours | land  | sea   | light text |
#   |---------|---------|-------|-------|------------|
#   | broken  |     497 |  0.0% |  0.0% |      0.00% |
#   | drawing |   3,964 |  1.1% | 14.8% |      0.08% |
#
# They are deliberately far below the working figures and far above the
# broken ones: this is here to catch a frame that is blank or textless,
# not to pin a colour scheme.
set -eu

UI=${1:-target/release/island-ui}
FRAME=${2:-/tmp/island-ui-frame.png}
# Which of the two frames to photograph. They are different pictures with
# different failure modes, so they get different assertions below: the
# island is a landmass in a sea, the estate is five buildings and 84
# things on a cleared yard, and a check written for one says nothing
# useful about the other.
VIEW=${3:-island}
case "$VIEW" in
    island|estate) ;;
    *) echo "view must be island or estate, not $VIEW" >&2; exit 2 ;;
esac
DISPLAY_NUM=${ISLAND_SMOKE_DISPLAY:-:88}
# Software rasterisation of 285 chunks is slow, and the island has to
# bootstrap first. Generous on purpose: a false failure here is worse than
# a slow job.
SETTLE=${ISLAND_SMOKE_SETTLE:-90}

missing=""
for tool in Xvfb convert; do
    command -v "$tool" >/dev/null 2>&1 || missing="$missing $tool"
done
[ -e /usr/share/vulkan/icd.d/lvp_icd.x86_64.json ] || [ -e /usr/share/vulkan/icd.d/lvp_icd.json ] \
    || missing="$missing lavapipe(mesa-vulkan-drivers)"

if [ "${1:-}" = "--check-tools" ]; then
    if [ -n "$missing" ]; then
        echo "missing:$missing" >&2
        echo "  apt-get install -y --no-install-recommends xvfb imagemagick \\"
        echo "      mesa-vulkan-drivers libxkbcommon-x11-0"
        exit 1
    fi
    echo "everything this needs is here"
    exit 0
fi

if [ -n "$missing" ]; then
    echo "render smoke test cannot run; missing:$missing" >&2
    exit 1
fi
[ -x "$UI" ] || { echo "no island-ui at $UI -- build it first" >&2; exit 1; }

root=$(cd "$(dirname "$0")/.." && pwd)
data=$(mktemp -d)
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
    --view "$VIEW" \
    --speed max >"$data/app.log" 2>&1 &
APP=$!

sleep "$SETTLE"

if ! kill -0 "$APP" 2>/dev/null; then
    echo "island-ui exited before a frame could be taken:" >&2
    tail -30 "$data/app.log" >&2
    exit 1
fi

DISPLAY="$DISPLAY_NUM" import -window root "$FRAME" 2>/dev/null \
    || DISPLAY="$DISPLAY_NUM" xwd -root -silent | convert xwd:- "$FRAME"

python3 - "$FRAME" "$VIEW" "$data/app.log" <<'PY'
import collections, re, subprocess, sys

frame, view, log = sys.argv[1], sys.argv[2], sys.argv[3]
raw = subprocess.run(["convert", frame, "-depth", "8", "rgb:-"],
                     capture_output=True, check=True).stdout
pixels = [tuple(raw[i:i + 3]) for i in range(0, len(raw), 3)]
counts = collections.Counter(pixels)
total = len(pixels)

def share(predicate):
    return 100.0 * sum(n for colour, n in counts.items() if predicate(*colour)) / total

colours = len(counts)
land = share(lambda r, g, b: g > r > b and g > 120)
sea = share(lambda r, g, b: b > g > r and b > 90)
text = share(lambda r, g, b: r > 150 and g > 150 and b > 150)

problems = []
# Blank and textless are failures of either view, and both are failures
# this script was written for: the first frame ever drawn was both.
if colours < 1500:
    problems.append(f"only {colours} distinct colours: the frame is close to blank")
if text < 0.02:
    problems.append(f"light text is {text:.3f}% of the frame; egui has no fonts")

if view == "island":
    print(f"frame: {colours} colours, land {land:.1f}%, sea {sea:.1f}%, "
          f"light text {text:.2f}%")
    if land < 0.2:
        problems.append(f"land is {land:.2f}% of the frame; the island is not being drawn")
    if sea < 5.0:
        problems.append(f"sea is {sea:.2f}% of the frame; the ground is not being drawn")
else:
    # The estate is a cleared yard with buildings on it, photographed
    # from above: pale and bright, with no sea anywhere in it. Measured
    # on a working frame at 1600x1000: 3,227 colours and 13.6% yard.
    yard = share(lambda r, g, b: r > 110 and g > 120 and b > 90 and g >= r)
    print(f"frame: {colours} colours, yard {yard:.1f}%, light text {text:.2f}%")
    if yard < 3.0:
        problems.append(f"the yard is {yard:.2f}% of the frame; the estate is not being drawn")

    # And the models, which a photograph cannot tell from a box at this
    # size. The application says how many it hung on the estate, and the
    # asset server says loudly when it cannot find one -- so the log
    # answers the question the picture cannot.
    lines = open(log, errors="replace").read()
    drawn = re.search(r"\((\d+) with models of their own\)", lines)
    if not drawn:
        problems.append("the application never said what it drew on the estate")
    elif int(drawn.group(1)) == 0:
        problems.append("the estate was drawn with no models on it at all")
    else:
        print(f"models: {drawn.group(1)} things carry a model of their own")
    for missing in re.findall(r"[Pp]ath not found:? ?(\S*)", lines):
        problems.append(f"a model was asked for and is not there: {missing}")

if problems:
    print(f"the desktop application is not drawing the {view}:", file=sys.stderr)
    for problem in problems:
        print(f"  - {problem}", file=sys.stderr)
    sys.exit(1)
print(f"the desktop application draws the {view}")
PY
