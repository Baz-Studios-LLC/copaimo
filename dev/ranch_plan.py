"""Lays out the ranch and writes `assets/world/placed.json`.

    python dev/ranch_plan.py            # write the plan
    python dev/ranch_plan.py --show     # print it without writing

# Why the layout is a script and the buildings are not

`dev/art/ranch.py` builds the pieces; this decides where they stand. Keeping them
apart is the whole point of the placement sheet: the yard can be rearranged, a
fence moved, a second trough added, without rebuilding a single model.

It is a script rather than hand-written JSON because a paddock is thirty fence
sections and nobody should place those by hand — and because the numbers below say
what they mean. `HOUSE_AT` is metres from the middle of the ranch, not a world
coordinate to be recomputed if the ranch ever moves.

# Which way a thing faces

A model is authored facing +Y in Blender, and the glTF Y-up conversion turns that
into -Z, which is what the game means by forward. So at `turn = 0` a door faces
-Z, which is north. The yard is entered from the south, so the buildings that
front onto it are turned by half a circle.

# The plan

    N
              silo  barn
      paddock        \\
              house   yard
                       trough
                gate
    S            (you arrive here)
"""

import json
import math
import os
import sys

# The middle of the ranch, from `config.rs`. Everything below is metres from here.
RANCH_AT = (-3064.0, 659.0)

# Half a turn: what a thing needs to face the yard rather than away from it.
ABOUT = math.pi

# Where each building stands, as (east, south) metres from the middle, and which
# way it is turned. South is +z, so a larger second number is nearer the gate.
BUILDINGS = [
    # The gate first, because it is what you walk through. Its span is along X, so
    # it needs no turning: you pass under it heading north.
    ("ranch_gate", (0.0, 26.0), 0.0),
    # The house on the west side, fronting the yard.
    ("ranch_house", (-12.0, 7.0), ABOUT),
    # The barn on the east, set a little further back so the yard is a yard and
    # not a corridor. Bigger than the house and it should read that way.
    ("ranch_barn", (13.0, 1.0), ABOUT),
    # The silo tucked behind the barn, where a tall thing does not block the view
    # of the house from the gate.
    ("ranch_silo", (23.0, -8.0), 0.0),
    # Two troughs: one in the yard, one at the paddock rail.
    ("ranch_trough", (1.0, 14.0), 0.0),
    ("ranch_trough", (-20.0, -5.0), math.pi * 0.5),
]

# The paddock, west and north of the house: where a monster is turned out.
#
# Given as its own rectangle in the same frame. The gap is where the rail opens
# onto the yard — a paddock with no way in is a pen.
PADDOCK = {
    "west": -48.0,
    "east": -12.0,
    "north": -30.0,
    "south": 0.0,
    # One four-metre section, matching `ranch_fence`.
    "section": 4.0,
    # The opening, measured along the south rail from its east end.
    "gap_from": 4.0,
    "gap_to": 12.0,
}


def fence_runs():
    """Every fence section round the paddock, as (at, turn).

    Sections are authored along X. A run along Z is the same section turned a
    quarter, and the loop steps by the section length so they meet end to end
    rather than overlapping — an overlap is two posts in one hole, and it shows.
    """
    step = PADDOCK["section"]
    out = []

    # The two rails that run east-west.
    for side in ("north", "south"):
        z = PADDOCK[side]
        x = PADDOCK["west"]
        while x + step <= PADDOCK["east"] + 0.01:
            middle = x + step * 0.5
            # The opening in the south rail.
            if side == "south":
                from_east = PADDOCK["east"] - middle
                if PADDOCK["gap_from"] <= from_east <= PADDOCK["gap_to"]:
                    x += step
                    continue
            out.append(((middle, z), 0.0))
            x += step

    # And the two that run north-south.
    for side in ("west", "east"):
        x = PADDOCK[side]
        z = PADDOCK["north"]
        while z + step <= PADDOCK["south"] + 0.01:
            out.append(((x, z + step * 0.5), math.pi * 0.5))
            z += step
    return out


def plan():
    things = []
    at_id = 1

    def add(kind, offset, turn):
        nonlocal at_id
        things.append(
            {
                "id": at_id,
                "kind": kind,
                "at": [RANCH_AT[0] + offset[0], RANCH_AT[1] + offset[1]],
                "lift": 0.0,
                "turn": round(turn % (math.pi * 2.0), 6),
                "scale": 1.0,
            }
        )
        at_id += 1

    for kind, offset, turn in BUILDINGS:
        add(kind, offset, turn)
    for offset, turn in fence_runs():
        add("ranch_fence", offset, turn)
    return {"format": 1, "placed": things}


def main() -> None:
    sheet = plan()
    text = json.dumps(sheet, indent=2) + "\n"
    if "--show" in sys.argv:
        print(text)
        return
    here = os.path.dirname(os.path.abspath(__file__))
    out = os.path.join(os.path.dirname(here), "assets", "world", "placed.json")
    with open(out, "w", encoding="utf-8") as file:
        file.write(text)
    kinds = {}
    for thing in sheet["placed"]:
        kinds[thing["kind"]] = kinds.get(thing["kind"], 0) + 1
    print(f"wrote {out} — {len(sheet['placed'])} things")
    for kind, count in sorted(kinds.items()):
        print(f"  {kind}: {count}")


main()
