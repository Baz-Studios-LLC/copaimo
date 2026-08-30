#!/usr/bin/env bash
# Builds every scripted figure in this folder and exports it to assets/models/.
#
#   dev/art/build.sh
#
# Two steps on purpose: the script builds a .blend, and the .blend goes through
# the same export gate a hand-made one does. Nothing gets a shortcut into the
# game just because a script made it.
#
# # This folder holds two different kinds of script
#
# It used to run every .py in here, and that was right while every .py was a
# figure. It has not been for a long time: half of them are TOOLS that take
# arguments and expect a scene already loaded. `audit_character.py` is the first
# one alphabetically, it wants a rig in the scene, and under `set -e` it took the
# whole build down with it - so `dev/art/build.sh` built NOTHING, quietly, for as
# long as that file has existed. It went unnoticed because the figures were being
# built one at a time by hand.
#
# So both kinds are listed below, and anything in the folder that is in neither
# list stops the build and says so. A new script cannot be forgotten into the
# wrong behaviour: it has to be declared, and the declaration is the note saying
# what it is.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../.." && pwd)"
. "$here/blender.sh"

# THE FIGURES. Each builds meshes and saves a .blend beside itself, and each is
# run with no arguments and an empty scene.
figures=(
  town      # houses, shops, the guild hall, the city
  props     # what stands in a yard or on a street
  yard      # the yards themselves, by programme
  lamp      # street lamps and lanterns
  bridge    # spans and their ends
  ranch     # the player's own buildings
  cover     # ground cover, welded per chunk
  trees     # the tree species
  people    # villagers
  warden    # the warden's own kit
)

# EVERYTHING ELSE, and why it is not a figure. None of these is ever run here.
tools=(
  masonry           # the shared kit every figure is built from. A library, not a script.
  audit_character   # measures a delivered character. Wants a rig already in the scene.
  author_gait       # authors a walk cycle. Takes a clip name.
  build_character   # builds person_ranger.glb from the source clips.
  compare_skin      # holds two skinnings side by side.
  foot_roll         # inspects one foot through a cycle.
  golden            # renders the character sheet and compares it to the kept copies.
  ik_gait           # solves a gait for inspection.
  inspect_glb       # prints what is inside a .glb. Takes a path.
  map_pdf           # draws the world map as a PDF.
  render_clay       # untextured renders, for looking at form.
  ribbon_measure    # measures a ribbon's geometry.
  see_the_bridge    # assembles a bridge in a scene to look at.
  see_the_character # assembles the character in a scene to look at.
  see_the_town      # assembles a town in a scene to look at.
  sheet             # renders one figure from four sides. Takes --figure.
)

# NOTHING IN THIS FOLDER IS UNACCOUNTED FOR.
#
# The check that keeps the two lists honest as the folder grows. Without it the
# lists are documentation, and documentation is what just cost this script every
# build it was asked for.
for script in "$here"/*.py; do
  name="$(basename "$script" .py)"
  if ! printf '%s\n' "${figures[@]}" "${tools[@]}" | grep -qx "$name"; then
    echo "dev/art/build.sh does not know what ${name}.py is." >&2
    echo "Add it to \`figures\` if it builds a model, or to \`tools\` with a note" >&2
    echo "saying what it is instead." >&2
    exit 1
  fi
done

blender=$(find_blender) || { echo "Blender not found." >&2; exit 1; }

# A MARK IN TIME, so the export can be told what this build actually wrote.
#
# `model_export.sh <folder>` sweeps every .blend in it, and this folder holds local
# authoring artefacts as well as figures - `ranger.blend` among them, which is
# gitignored and which nothing loads, and which the sweep therefore turned into 18 MB
# of `assets/models/ranger.glb` on every single build. Gitignoring the OUTPUT stopped
# it being committed again and did nothing about it being made; Codex was right that
# the ignore rule was carrying a correctness burden it should not.
stamp="$(mktemp)"

for name in "${figures[@]}"; do
  script="$here/$name.py"
  [ -f "$script" ] || { echo "dev/art/build.sh lists ${name}.py, which is not here." >&2; exit 1; }
  echo "building ${name}.py"
  # `--python-exit-code` matters: Blender exits 0 even when the script it ran
  # died on a traceback, so without this a broken generator produced nothing and
  # the build cheerfully carried on to report "no .blend files found".
  "$blender" --background --python-exit-code 1 --python "$script" >/dev/null
done

# ONLY WHAT THIS BUILD WROTE. Every figure re-saves its .blend, so everything the
# loop above produced is newer than the mark and every stale local blend is older.
# No list to keep in step: the build's own output decides.
mapfile -t made < <(find "$here" -maxdepth 1 -name '*.blend' -newer "$stamp" | sort)
rm -f "$stamp"
[ ${#made[@]} -gt 0 ] || { echo "the build produced no .blend files" >&2; exit 1; }
"$root/dev/model_export.sh" "${made[@]}"
