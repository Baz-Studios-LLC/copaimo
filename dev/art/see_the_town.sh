#!/usr/bin/env bash
# Opens Blender on every town building, side by side, to be looked at.
#
#   dev/art/see_the_town.sh
#
# Builds a disposable town_view.blend from the same .blend files the game ships,
# then opens it. Editing here changes nothing; edit dev/art/town.py and rebuild.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=blender.sh
. "$here/blender.sh"
blender=$(find_blender) || { echo "Blender not found." >&2; exit 1; }

"$blender" --background --python-exit-code 1 --python "$here/see_the_town.py"
exec "$blender" "$here/aside/town_view.blend"
