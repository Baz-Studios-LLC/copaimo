#!/usr/bin/env bash
# Dumps the kit's look scene and draws it, in one go.
#
#   dev/peek.sh out.png [--scale 330 --pitch 26 ...]
#
# The dump is a test rather than a binary because a Bench is easiest to build where
# the kit's own tests already live — see `look::dump_the_new_parts`.
set -euo pipefail

out=$1
shift

scene=$(mktemp -t copaimo-scene-XXXXXX.json)
trap 'rm -f "$scene"' EXIT

cargo test dump_the_new_parts -- --ignored --nocapture 2>/dev/null |
    awk '/^SCENE /{f=1; sub(/^SCENE /,""); print; next} f{print; if ($0=="}") exit}' >"$scene"

if [ ! -s "$scene" ]; then
    echo "the dump printed nothing — did the test compile?" >&2
    exit 1
fi

python dev/look.py "$scene" "$out" "$@"
