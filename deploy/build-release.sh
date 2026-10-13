#!/bin/sh
# Build what gets shipped, and nothing else.
#
#   deploy/build-release.sh [--with-ui]
#
# Writes to dist/: the backend binary, the dashboard as static files, and
# -- with --with-ui -- the desktop application. The desktop build is
# opt-in because it pulls in Bevy and takes about nine minutes from cold
# on four cores, which is a long time to wait for something a server does
# not run.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"
dist="$root/dist"
with_ui=no
backend_url=""

while [ $# -gt 0 ]; do
    case "$1" in
        --with-ui) with_ui=yes ;;
        --backend) shift; backend_url="${1:?--backend needs a URL}" ;;
        -h|--help)
            sed -n '2,12p' "$0" | sed 's/^# \{0,1\}//'
            exit 0
            ;;
        *) echo "unknown argument: $1" >&2; exit 2 ;;
    esac
    shift
done

rm -rf "$dist"
mkdir -p "$dist/bin"

echo "==> the backend"
cargo build --release --locked -p island
cp target/release/island "$dist/bin/"

echo "==> the dashboard, as files"
# The island's address goes in a generated config.js rather than in the
# pages, so the same export serves any backend when one is not named here.
if [ -n "$backend_url" ]; then
    "$dist/bin/island" export --to "$dist/web" --backend "$backend_url"
else
    "$dist/bin/island" export --to "$dist/web"
fi

if [ "$with_ui" = yes ]; then
    echo "==> the desktop application (this takes a while)"
    cargo build --release --locked -p island_ui
    cp target/release/island-ui "$dist/bin/"
    # The models and textures it draws from. Without these it starts and
    # draws an empty estate, which is the failure mode worth preventing by
    # shipping them rather than by logging about them.
    mkdir -p "$dist/assets"
    cp -r assets/. "$dist/assets/"
fi

echo
echo "dist/ holds:"
find "$dist" -maxdepth 2 -mindepth 1 | sed "s|$dist|  dist|" | sort
echo
echo "The backend needs fixtures/island/ beside it, or --scenario pointing"
echo "at a scenario and its canon. See docs/island/DEPLOYMENT.md."
