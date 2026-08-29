"""Builds the town: houses, shops and guild halls, each one you can walk into.

    dev/art/build.sh

# What the research said, and what of it is used here

The industry answer for buildings is a SPLIT GRAMMAR — CGA shape and its
descendants. A building is not modelled; it is a solid that gets cut up by rules. A
mass splits into storeys, a storey's wall splits into bays, a bay resolves to a
door, a window or blank wall. That is what makes a hundred buildings out of one
description instead of a hundred models, and it is what is done below.

The layout side of the same research — roads, then the parcels between them, then
OBB subdivision of a parcel into lots — belongs to the game, not here. This file
answers "what does a building look like"; `settle` answers "where does it stand".

# Every one of them is hollow

The point is to walk in. So a building is not a box with a door painted on it: the
walls have thickness and enclose a room, the doorway is a real gap in a real wall,
and there is a floor under it and a ceiling over it. A shop has a counter and
shelves, a house has a hearth and a bed, a guild hall has a long table and a
board — enough that a room reads as a room somebody uses rather than as an empty
volume with a door.

Interiors are lit by the same shading ramp as everything else, so they are darker
than the outside. That is the right way round, but it is why the indoor palette is
mixed lighter than the outdoor one: a room painted in outdoor browns and then
shaded down is a cave.

# One object, vertex-coloured

The same contract every placed thing in this folder keeps — see `masonry`. A
building is carried whole and spawned as a scene, and its colour lives in its
vertices so the lot can be recoloured from the table below.
"""

import math
import os
import sys

import bpy

# Blender runs a script with the CWD wherever it was launched from, not beside the
# file, so the folder these scripts share has to be put on the path by hand.
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import masonry
from masonry import box, gable_wall, lean, tube, wedge

# ---------------------------------------------------------------- the module
#
# Everything is a whole number of these, which is what keeps a street of separately
# generated buildings looking like one street. 1.5 m is the game's own module - the
# workbench kit is built on it - so a scripted building and a hand-built one sit on
# the same grid.
MODULE = 1.5

# A storey, floor to floor. Two modules, which is what the kit's stairs climb.
# 3.6 m, up from 3.0.
#
# # A building the CAMERA has to fit in, not just the warden
#
# The warden is 1.7 m and a 3 m ceiling was generous for him. He is not the thing
# that has to fit: the camera follows him from three or four metres behind and a
# little above, so the room has to hold the pair of them or the view clips through
# the wall the instant he steps inside. Everything here is sized for the camera,
# which is why these rooms would read as slightly grand for a cottage if a person
# stood in one alone.
STOREY = 3.6

# How thick a wall is. Thick enough to read as masonry from inside AND outside,
# which a plane cannot do at all.
WALL = 0.22

# A doorway a warden fits through, and the emphasis is on FITS.
#
# He is 1.70 m tall and 0.66 m across the shoulders as far as collision is
# concerned, so the clear width matters more than the look of it: the first cut was
# 1.15 m with the door leaf standing inside the opening, which left a gap of 0.57 m
# for a warden 0.66 m wide. Every building in the world would have been sealed, and
# it would have read as the doorways being decorative rather than as a number being
# nine centimetres short.
#
# So the opening is wider AND the leaf hangs flat against the wall beside it rather
# than in it. What is left is the whole 1.4 m.
# A door the camera can follow him through, for the same reason. 1.4 m was a
# doorway a person fits through and a camera does not.
DOOR_WIDE = 1.9
DOOR_TALL = 2.45

# A window, and how far up the wall it sits.
WINDOW_WIDE = 0.95
WINDOW_TALL = 1.05
WINDOW_SILL = 1.05

PAINT = {
    # Outside
    "plaster": (0.88, 0.83, 0.71),
    "plaster2": (0.82, 0.78, 0.70),
    "timber": (0.36, 0.25, 0.17),
    "stone": (0.48, 0.47, 0.44),
    "roof": (0.56, 0.26, 0.19),
    "roof2": (0.46, 0.30, 0.22),
    "slate": (0.32, 0.33, 0.37),
    "thatch": (0.66, 0.55, 0.33),
    "shutter": (0.30, 0.44, 0.42),
    "trim": (0.94, 0.92, 0.86),
    "flower": (0.72, 0.30, 0.32),
    "leafy": (0.32, 0.46, 0.26),
    "door": (0.30, 0.21, 0.14),
    "glass": (0.36, 0.47, 0.52),
    "sign": (0.62, 0.42, 0.22),
    "guild": (0.25, 0.35, 0.42),
    "brass": (0.66, 0.52, 0.24),
    # Inside, mixed lighter on purpose - see the note at the top
    "inwall": (0.86, 0.82, 0.73),
    "infloor": (0.56, 0.42, 0.28),
    "inbeam": (0.44, 0.32, 0.22),
    "hearth": (0.42, 0.41, 0.39),
    "cloth": (0.55, 0.30, 0.28),
    "counter": (0.48, 0.35, 0.22),
    "shelf": (0.50, 0.38, 0.24),
    "board": (0.34, 0.26, 0.18),
    # The city, which is a different age of the world - see `tower`.
    "concrete": (0.72, 0.71, 0.68),
    "concrete2": (0.62, 0.62, 0.60),
    "curtain": (0.30, 0.42, 0.50),
    "curtain2": (0.24, 0.34, 0.42),
    "mullion": (0.46, 0.48, 0.50),
    "steel": (0.56, 0.58, 0.60),
    "parapet": (0.50, 0.50, 0.49),
    "canopy": (0.22, 0.30, 0.36),
    "neon": (0.34, 0.72, 0.78),
    # Surfaces - see `courses` and `shingles`.
    "stone2": (0.42, 0.41, 0.38),
    "shingle": (0.38, 0.26, 0.22),
    "shingle2": (0.32, 0.22, 0.19),
    "straw": (0.60, 0.49, 0.29),
    "straw2": (0.54, 0.44, 0.26),
    "brick": (0.52, 0.31, 0.25),
    "brick2": (0.46, 0.28, 0.22),
}

# Which colours are indoor, so their shading ramp starts at the floor they stand on
# rather than at the foot of the building. Without this a chair in an upstairs room
# is lit as brightly as the ridge of the roof.
INDOORS = {
    "inwall", "infloor", "inbeam", "hearth", "cloth", "counter", "shelf", "board",
}


# --------------------------------------------------------------- wall grammar


def wall_run(parts, along, at, length, height, colour, bays, floor=0.0, facing=-1.0):
    """One wall, split into bays, each bay resolved to what the rule says.

    This is the split rule and it is the whole grammar: a wall of length L is cut
    into N equal bays and each bay is told what it is. A bay is `"solid"`, `"door"`
    or `"window"`. Everything below builds walls by naming bays.

    `along` is "x" for a wall running east-west and "y" for one running north-south.
    `at` is the middle of the wall on the ground.
    """
    count = max(1, len(bays))
    bay = length / count
    for index, kind in enumerate(bays):
        middle = -length * 0.5 + bay * (index + 0.5)
        centre = (
            (at[0] + middle, at[1], at[2]) if along == "x" else (at[0], at[1] + middle, at[2])
        )
        _one_bay(parts, along, centre, bay, height, colour, kind, floor, facing)


def _one_bay(parts, along, at, wide, height, colour, kind, floor, facing=-1.0):
    """One bay of wall, with its hole cut by building around the hole.

    Boolean subtraction would be the obvious way and it is the wrong one here: it
    makes a mesh nobody can predict the vertex count of, and it fails silently on
    coplanar faces, which is most of what a building is. A hole in a wall is four
    pieces of wall around a gap, and four boxes always weld cleanly.
    """
    def slab(w, h, off_along, off_up):
        size = (w, WALL, h) if along == "x" else (WALL, w, h)
        centre = (
            (at[0] + off_along, at[1], floor + off_up + h * 0.5)
            if along == "x"
            else (at[0], at[1] + off_along, floor + off_up + h * 0.5)
        )
        parts.append(box(size, centre, colour))

    if kind == "solid":
        slab(wide, height, 0.0, 0.0)
        return

    hole_wide = DOOR_WIDE if kind == "door" else WINDOW_WIDE
    hole_tall = DOOR_TALL if kind == "door" else WINDOW_TALL
    sill = 0.0 if kind == "door" else WINDOW_SILL
    hole_wide = min(hole_wide, wide - 0.3)

    side = (wide - hole_wide) * 0.5
    if side > 0.02:
        slab(side, height, -(wide - side) * 0.5, 0.0)
        slab(side, height, (wide - side) * 0.5, 0.0)
    # Over the hole, and under it for a window.
    over = height - sill - hole_tall
    if over > 0.02:
        slab(hole_wide, over, 0.0, sill + hole_tall)
    if sill > 0.02:
        slab(hole_wide, sill, 0.0, 0.0)

    # What fills the hole, and what dresses it.
    #
    # A pane on its own is a blue rectangle painted on a wall. What makes a window
    # read as a window is the SMALL SHAPES around it - a frame proud of the wall, a
    # sill that throws a shadow, a mullion across the glass and shutters either
    # side. That is the third tier of the silhouette hierarchy, and the first cut
    # had none of it.
    if kind == "window":
        mid = floor + sill + hole_tall * 0.5
        size = (hole_wide, 0.06, hole_tall) if along == "x" else (0.06, hole_wide, hole_tall)
        parts.append(box(size, (at[0], at[1], mid), "glass"))
        _dress_window(parts, along, at, hole_wide, hole_tall, mid, floor + sill, facing)
    else:
        # The leaf, hung open flat against the wall BESIDE the opening rather than
        # standing in it - so the doorway's clear width is the doorway's width. See
        # DOOR_WIDE for what happens otherwise.
        leaf = hole_wide * 0.85
        size = (leaf, 0.07, hole_tall) if along == "x" else (0.07, leaf, hole_tall)
        off = (hole_wide + leaf) * 0.5 + 0.03
        centre = (
            (at[0] - off, at[1] + facing * (WALL * 0.5 + 0.04), floor + hole_tall * 0.5)
            if along == "x"
            else (at[0] + facing * (WALL * 0.5 + 0.04), at[1] - off, floor + hole_tall * 0.5)
        )
        parts.append(box(size, centre, "door"))
        # And a threshold, so the gap reads as a doorway rather than as damage.
        sill_size = (hole_wide + 0.3, WALL + 0.2, 0.06) if along == "x" else (WALL + 0.2, hole_wide + 0.3, 0.06)
        parts.append(box(sill_size, (at[0], at[1], floor + 0.03), "stone"))
        _dress_door(parts, along, at, hole_wide, hole_tall, floor, facing)


def _out(along, at, push, facing=-1.0):
    """A point pushed OUT of a wall, on the side that wall actually faces.

    # Half the dressing was inside the building

    This used to push in the negative direction always, which is outward for the
    south wall and the west flank and INWARD for the other two. So every frame,
    sill, mullion, shutter and flowerbox on the north wall and the east flank was
    built a hand's width inside the room instead of on the street - reported as
    "windows and planters are not lined properly", which is exactly what it looks
    like from outside: a window with no frame, and a flowerbox that has vanished.

    A wall knows which way it faces. `facing` is that, and it is the only thing that
    was ever missing.
    """
    push = push * facing
    return (at[0], at[1] + push, at[2]) if along == "x" else (at[0] + push, at[1], at[2])


def _across(along, at, over):
    """A point moved along the wall rather than out of it."""
    return (at[0] + over, at[1], at[2]) if along == "x" else (at[0], at[1] + over, at[2])


def _slab(along, wide, thick, tall):
    """Extents for something lying in a wall that runs `along`."""
    return (wide, thick, tall) if along == "x" else (thick, wide, tall)


def _dress_window(parts, along, at, wide, tall, mid, sill_z, facing=-1.0):
    """Frame, sill, mullion and shutters. See the note where it is called."""
    edge = 0.07
    out = WALL * 0.5 + 0.03
    # Frame: two jambs, a head and a sill, each standing proud of the plaster.
    for side in (-1.0, 1.0):
        place = _across(along, (at[0], at[1], mid), side * (wide * 0.5 + edge * 0.5))
        parts.append(box(_slab(along, edge, out * 2.0, tall + edge * 2.0), _out(along, place, out * 0.5, facing), "trim"))
    for up in (-1.0, 1.0):
        place = (at[0], at[1], mid + up * (tall * 0.5 + edge * 0.5))
        parts.append(box(_slab(along, wide + edge * 2.0, out * 2.0, edge), _out(along, place, out * 0.5, facing), "trim"))
    # A sill with a real overhang, because the shadow under it is what says stone.
    parts.append(
        box(_slab(along, wide + 0.34, out * 2.6, 0.07), _out(along, (at[0], at[1], sill_z - 0.02), out * 0.9, facing), "stone")
    )
    # One mullion, which turns a pane into panes.
    parts.append(box(_slab(along, 0.05, 0.1, tall), (at[0], at[1], mid), "trim"))
    # Shutters, hung flat against the wall either side.
    for side in (-1.0, 1.0):
        place = _across(along, (at[0], at[1], mid), side * (wide * 0.5 + edge + wide * 0.25))
        parts.append(box(_slab(along, wide * 0.48, 0.05, tall * 0.96), _out(along, place, out + 0.02, facing), "shutter"))


def _dress_door(parts, along, at, wide, tall, floor, facing=-1.0):
    """A surround and a lintel, so a doorway is an entrance and not a gap."""
    edge = 0.09
    out = WALL * 0.5 + 0.04
    for side in (-1.0, 1.0):
        place = _across(along, (at[0], at[1], floor + tall * 0.5), side * (wide * 0.5 + edge * 0.5))
        parts.append(box(_slab(along, edge, out * 2.0, tall + edge), _out(along, place, out * 0.5, facing), "stone"))
    parts.append(
        box(_slab(along, wide + edge * 2.4, out * 2.2, 0.14), _out(along, (at[0], at[1], floor + tall + 0.07), out * 0.6, facing), "stone")
    )


def eaves(parts, wide, deep, at_z, over, colour, ridge="y"):
    """The fascia boards that run along the foot of a roof.

    A roof with no edge reads as a sheet laid on top of a box. The board along the
    eaves is a small shape that gives the big one a lip, and it is most of what
    separates a building from a diagram of one.
    """
    if ridge == "y":
        for side in (-1.0, 1.0):
            parts.append(box((wide + over * 2.0, 0.1, 0.16), (0.0, side * (deep * 0.5 + over), at_z), colour))
    else:
        for side in (-1.0, 1.0):
            parts.append(box((0.1, deep + over * 2.0, 0.16), (side * (wide * 0.5 + over), 0.0, at_z), colour))


def chimney(parts, at, base_z, top_z, colour="stone"):
    """A stack, and a cap that is wider than it.

    The cap matters: a plain column reads as a pipe, and the overhang is what makes
    it masonry. This is a MEDIUM shape - the tier the first attempt had none of.
    """
    parts.append(box((0.75, 0.75, top_z - base_z), (at[0], at[1], (base_z + top_z) * 0.5), colour))
    parts.append(box((0.95, 0.95, 0.16), (at[0], at[1], top_z + 0.08), colour))
    parts.append(box((0.34, 0.34, 0.22), (at[0], at[1], top_z + 0.24), "roof2"))


def framing(parts, wide, deep, floor, height, colour="timber", openings=None):
    """Timber framing: corner posts, rails, studs and a brace to each panel.

    Half-timbering is the cheapest strong pattern a stylised building can wear -
    it is all straight boxes, it reads instantly at any distance, and it breaks a
    blank plaster wall into panels so the eye has something to measure the building
    by. The brace is what stops it looking like a grid.

    # It has to know where the holes are

    It did not, and it laid its studs and both its rails straight across the whole
    wall - so a stud landed in the middle of the doorway and a rail ran across the
    threshold at shin height. From inside the game that is a post in the door you
    cannot walk past, which is exactly what was reported.

    Nothing about a wall's framing can be worked out without the openings, so
    `openings` is the same bay list the wall itself was built from: a stud inside a
    door bay is not built, and a rail is broken around it. One description of where
    the holes are, used by both, because two would drift apart the first time
    anybody changed a bay.
    """
    t = 0.11
    out = WALL * 0.5 + 0.02
    openings = openings or {}

    for face, span in (("x", wide), ("y", deep)):
        for side in (-1.0, 1.0):
            base = (0.0, side * (deep * 0.5 - out * 0.5), 0.0) if face == "x" else (side * (wide * 0.5 - out * 0.5), 0.0, 0.0)

            # Where this wall's doorways are, as (from, to) along the wall.
            bays = openings.get((face, side), [])
            gaps = []
            if bays:
                bay = span / max(1, len(bays))
                for index, kind in enumerate(bays):
                    if kind != "door":
                        continue
                    middle = -span * 0.5 + bay * (index + 0.5)
                    half = min(DOOR_WIDE, bay - 0.3) * 0.5 + 0.12
                    gaps.append((middle - half, middle + half))

            def clear(at):
                return all(at < lo or at > hi for lo, hi in gaps)

            # Top rail runs the whole way; the BOTTOM one is broken by a doorway,
            # because a rail across a threshold is a bar across a door.
            parts.append(box(_slab(face, span, out * 2.0, t), (base[0], base[1], floor + height - 0.1), colour))
            runs = [(-span * 0.5, span * 0.5)]
            for lo, hi in gaps:
                cut = []
                for a, b in runs:
                    if hi <= a or lo >= b:
                        cut.append((a, b))
                        continue
                    if lo > a:
                        cut.append((a, lo))
                    if hi < b:
                        cut.append((hi, b))
                runs = cut
            for a, b in runs:
                if b - a < 0.06:
                    continue
                place = _across(face, (base[0], base[1], floor + 0.06), (a + b) * 0.5)
                parts.append(box(_slab(face, b - a, out * 2.0, t), place, colour))

            # Studs, one about every module - but never standing in a doorway.
            count = max(2, int(round(span / MODULE)))
            for index in range(count + 1):
                over = -span * 0.5 + span * index / count
                if not clear(over):
                    continue
                place = _across(face, (base[0], base[1], floor + height * 0.5), over)
                parts.append(box(_slab(face, t, out * 2.0, height - 0.2), place, colour))


def porch(parts, deep, floor, colour="timber", roof_colour="roof2"):
    """A little roof on two posts over the door. A medium shape, and a welcome."""
    reach = 1.05
    top = DOOR_TALL + 0.55
    for side in (-1.0, 1.0):
        parts.append(box((0.12, 0.12, top - 0.1), (side * 0.95, -deep * 0.5 - reach + 0.1, floor + (top - 0.1) * 0.5), colour))
    parts.append(wedge(2.6, reach + 0.35, 0.5, (0.0, -deep * 0.5 - reach * 0.5 + 0.1, floor + top), roof_colour, ridge="x"))
    parts.append(box((2.7, 0.1, 0.14), (0.0, -deep * 0.5 - reach + 0.05, floor + top), colour))


def flowerbox(parts, along, at, wide, sill_z, facing=-1.0):
    """A box of flowers under a window. Pure charm, and it costs eight boxes."""
    out = WALL * 0.5 + 0.16
    parts.append(box(_slab(along, wide * 0.9, 0.24, 0.2), _out(along, (at[0], at[1], sill_z - 0.16), out, facing), "timber"))
    for index in range(3):
        over = (index - 1) * wide * 0.26
        place = _across(along, (at[0], at[1], sill_z - 0.03), over)
        parts.append(box(_slab(along, 0.16, 0.16, 0.14), _out(along, place, out, facing), "leafy"))
        parts.append(box(_slab(along, 0.1, 0.1, 0.08), _out(along, (place[0], place[1], place[2] + 0.1), out, facing), "flower"))


def shell(parts, wide, deep, storeys, colour, doors, windows, openings=None):
    """The four walls of a building, split into bays, with the openings placed.

    `doors` is which side the doorway is on - "south" always, because a building
    faces its street and the game turns the whole building to face the road.
    """
    for storey in range(storeys):
        floor = storey * STOREY
        ground = storey == 0

        south = _bays(wide, windows, door=ground and doors)
        north = _bays(wide, windows, door=False)
        sides = _bays(deep, windows, door=False)

        # Each wall says which way it FACES. The south wall and the west flank look
        # down the negative axis and the other two look up it; without that, every
        # dressing on half the building is built inside the room.
        if openings is not None:
            openings[("x", -1.0)] = south
            openings[("x", 1.0)] = north
            openings[("y", -1.0)] = sides
            openings[("y", 1.0)] = sides
        wall_run(parts, "x", (0.0, -deep * 0.5 + WALL * 0.5, 0.0), wide, STOREY, colour, south, floor, -1.0)
        wall_run(parts, "x", (0.0, deep * 0.5 - WALL * 0.5, 0.0), wide, STOREY, colour, north, floor, 1.0)
        wall_run(parts, "y", (-wide * 0.5 + WALL * 0.5, 0.0, 0.0), deep, STOREY, colour, sides, floor, -1.0)
        wall_run(parts, "y", (wide * 0.5 - WALL * 0.5, 0.0, 0.0), deep, STOREY, colour, sides, floor, 1.0)


def _bays(length, windows, door):
    """How many bays a wall of this length gets, and what each one is.

    A bay is about a module wide. Fewer than three and a facade has no rhythm; more
    than seven and the windows read as a factory.
    """
    count = max(1, min(7, int(round(length / MODULE))))
    bays = ["solid"] * count
    if door:
        bays[count // 2] = "door"
    if windows:
        for index in range(count):
            if bays[index] == "solid" and index % 2 == (0 if count % 2 else 1):
                bays[index] = "window"
    return bays


# ------------------------------------------------------------------ interiors


def room(parts, wide, deep, storeys):
    """A floor under every storey and a ceiling over the top one."""
    inner = (wide - WALL * 2.0, deep - WALL * 2.0)
    for storey in range(storeys):
        floor = storey * STOREY
        parts.append(box((inner[0], inner[1], 0.10), (0.0, 0.0, floor + 0.05), "infloor"))
        # A beam across the room, which is most of what makes a ceiling read as one.
        parts.append(
            box((inner[0], 0.18, 0.22), (0.0, 0.0, floor + STOREY - 0.2), "inbeam")
        )
    parts.append(
        box((inner[0], inner[1], 0.10), (0.0, 0.0, storeys * STOREY - 0.05), "inwall")
    )


def stairs(parts, wide, deep, storeys, side=1.0):
    """A flight to the storey above, in a back corner where it is out of the way.

    `side` is which back corner. It exists because the guild hall's tower is a solid
    block standing in its own north-east corner, and the stairs were put in the same
    corner by default - a flight of steps inside a stone tower, which the plan view
    showed and no amount of looking at the outside would have.
    """
    if storeys < 2:
        return
    steps = 10
    rise = STOREY / steps
    run = 0.28
    x = side * (wide * 0.5 - WALL - 0.55)
    for step in range(steps):
        parts.append(
            box(
                (1.0, run, rise),
                (x, deep * 0.5 - WALL - 0.4 - step * run, rise * (step + 0.5)),
                "inbeam",
            )
        )


def hearth(parts, wide, deep):
    """A fireplace against the north wall, and the chimney that goes with it."""
    x = -wide * 0.25
    y = deep * 0.5 - WALL - 0.25
    parts.append(box((1.5, 0.5, 1.3), (x, y, 0.65), "hearth"))
    parts.append(box((0.9, 0.3, 0.8), (x, y - 0.12, 0.4), "inbeam"))


def bed(parts, wide, deep):
    x = wide * 0.5 - WALL - 0.75
    y = -deep * 0.5 + WALL + 1.1
    parts.append(box((1.3, 2.0, 0.35), (x, y, 0.35), "inbeam"))
    parts.append(box((1.2, 1.7, 0.18), (x, y, 0.60), "cloth"))


def table(parts, at, wide, deep):
    parts.append(box((wide, deep, 0.10), (at[0], at[1], 0.80), "counter"))
    for sx in (-1.0, 1.0):
        for sy in (-1.0, 1.0):
            parts.append(
                box(
                    (0.10, 0.10, 0.75),
                    (at[0] + sx * (wide * 0.5 - 0.12), at[1] + sy * (deep * 0.5 - 0.12), 0.38),
                    "inbeam",
                )
            )


def counter(parts, wide, deep):
    """A shop counter across the room, and shelves on the back wall."""
    parts.append(box((wide - WALL * 2.0 - 1.6, 0.7, 1.0), (0.0, 0.4, 0.5), "counter"))
    for level in range(3):
        parts.append(
            box(
                (wide - WALL * 2.0 - 0.8, 0.35, 0.08),
                (0.0, deep * 0.5 - WALL - 0.25, 0.7 + level * 0.6),
                "shelf",
            )
        )


def stock(parts, wide, deep):
    """Something ON the shelves, or they read as a bare rack."""
    for level in range(3):
        for slot in range(5):
            across = (slot - 2) * (wide - 1.6) / 5.0
            parts.append(
                box(
                    (0.22, 0.22, 0.3),
                    (across, deep * 0.5 - WALL - 0.25, 0.9 + level * 0.6),
                    "cloth" if (slot + level) % 2 else "brass",
                )
            )


def guild_hall_inside(parts, wide, deep):
    """A long table down the middle and a notice board on the wall."""
    table(parts, (0.0, -0.4), wide * 0.45, deep * 0.5)
    for side in (-1.0, 1.0):
        for along in (-0.6, 0.6):
            parts.append(
                box(
                    (0.45, 0.45, 0.45),
                    (side * (wide * 0.28), -0.4 + along * deep * 0.18, 0.45),
                    "inbeam",
                )
            )
    parts.append(
        box((wide * 0.5, 0.10, 1.2), (0.0, deep * 0.5 - WALL - 0.1, 1.7), "board")
    )


def doorstep(parts, deep, rises: float, wide: float = 2.4):
    """Steps up to the door, from the ground to the floor inside.

    # Why every building was sealed

    A building stands on a plinth - 42 cm for a cottage, 70 for the guild hall -
    which is what keeps its walls out of the mud and is right. What was missing is
    the obvious consequence: the floor inside is 42 cm above the ground OUTSIDE,
    and a warden walks, he does not vault. Every doorway in the world opened onto a
    wall the height of his knee.

    Nothing about the doorway said so. It was a proper 1.4 m opening with a lintel
    and a surround, and from the street it read as a door you could walk through -
    which is the worst kind of fault, because it looks finished.

    Three shallow treads rather than one tall one: a single 42 cm step is a climb,
    and the game's own walking rule refuses anything steeper than 1.4 up per 1
    along, which a knee-high face onto flat ground fails outright.
    """
    treads = 3
    rise = rises / treads
    run = 0.34
    for step in range(treads):
        # The bottom tread reaches furthest out; each one above is set back.
        out = (treads - step) * run
        parts.append(
            box(
                (wide, run * (treads - step), rise),
                (0.0, -deep * 0.5 - out * 0.5, rise * (step + 0.5)),
                "stone",
            )
        )

# -------------------------------------------------------------------- figures
#
# # Three tiers of shape, which is what the first attempt was missing
#
# The rule stylised architecture is built on is LARGE shape, MEDIUM shape, SMALL
# detail. The first cut had the large one (a box) and the small one (a window
# rectangle) and nothing in between, which is exactly why four different buildings
# all read as the same beige box with a lid on it.
#
# The medium tier is what is added here and it is where the character lives:
# chimneys, porches, dormers, a jettied upper storey, buttresses, lean-tos. They are
# the shapes that break a silhouette, and a silhouette is what a building is read by
# at the distance a player usually sees one.
#
# The roofs also got STEEP. A shallow pyramid is the shape of a warehouse; a pitch
# near forty-five degrees with a real overhang is the shape of somewhere somebody
# lives, and the overhang throws the shadow that makes a wall look like a wall.

# How far a roof reaches past the wall it sits on.
# How high a building sits above the ground it stands on.
#
# # A plinth you cannot step onto is a building you cannot enter
#
# These stood on a 42 cm stone plinth, which is a handsome thing to look at and a
# wall to a warden: he walks on the TERRAIN, not on a building's floor, so a floor
# 42 cm up is a floor he arrives at shin-first. Reported exactly that way - "the
# foundation of buildings is also too high so I can't walk in any".
#
# 12 cm instead: enough to read as a course of stone under the wall and to keep the
# sill out of the mud, low enough to be a threshold rather than a step. The
# doorstep in front of it bridges the rest.
PLINTH = 0.12

OVERHANG = 0.42

# How high a roof rises per metre of its span. A hair over a 45 degree pitch.
PITCH = 0.55


# # Surfaces, not just shapes
#
# Every wall in this town was one flat colour and every roof one flat slope, and
# from any distance where the silhouette had already done its work there was nothing
# further to look at. These are the second and third tiers: COURSES on stone and
# SHINGLES on a roof, both built from the same boxes as everything else.
#
# Cheap on purpose. A course is one box per row, not one per stone - the eye reads
# the horizontal banding and supplies the rest - and a roof gets one box per shingle
# ROW, stepped and overlapping, which is what makes a pitch read as a covering
# rather than as a ramp.

# How deep a course of stone stands proud of the wall, and how tall a course is.
COURSE_PROUD = 0.035
COURSE_TALL = 0.42

# The same for a shingle row.
SHINGLE_PROUD = 0.05
SHINGLE_TALL = 0.34


def courses(parts, wide, deep, base, height, colour="stone2", faces=("x", "y")):
    """Horizontal bands of stone up a wall, so it reads as coursed masonry."""
    rows = max(1, int(height / COURSE_TALL))
    for row in range(rows):
        # Every other course stands slightly further out, which is what turns a set
        # of stripes into stonework: the shadow line alternates.
        up = base + (row + 0.5) * height / rows
        proud = COURSE_PROUD * (1.0 if row % 2 == 0 else 0.55)
        for face, span, off in (("x", wide, deep), ("y", deep, wide)):
            if face not in faces:
                continue
            for side in (-1.0, 1.0):
                at = (0.0, side * off * 0.5, 0.0) if face == "x" else (side * off * 0.5, 0.0, 0.0)
                parts.append(
                    box(_slab(face, span, proud * 2.0, height / rows * 0.62), (at[0], at[1], up), colour)
                )


def shingles(parts, wide, deep, base, rise, ridge="y", colour="shingle", over=OVERHANG):
    """Rows of tiles up both slopes of a pitched roof.

    Stepped so each row overlaps the one below, which is the whole reason a tiled
    roof reads as tiled: the shadow under every course.
    """
    span = (wide if ridge == "y" else deep) + over * 2.0
    length = (deep if ridge == "y" else wide) + over * 2.0
    slope = (span * 0.5, rise)
    rows = max(2, int((slope[0] ** 2 + slope[1] ** 2) ** 0.5 / SHINGLE_TALL))
    for side in (-1.0, 1.0):
        for row in range(rows):
            part = (row + 0.5) / rows
            # Along the slope from eaves to ridge.
            across = side * span * 0.5 * (1.0 - part)
            up = base + rise * part
            wide_here = SHINGLE_TALL * 1.15
            if ridge == "y":
                place = (across, 0.0, up)
                size = (wide_here, length, SHINGLE_PROUD * 2.0)
            else:
                place = (0.0, across, up)
                size = (length, wide_here, SHINGLE_PROUD * 2.0)
            parts.append(box(size, place, colour if row % 2 == 0 else colour + "2"))


def roofed(parts, wide, deep, base, colour, ridge="y", over=OVERHANG, pitch=PITCH):
    """A pitched roof with its overhang, its fascia and its gable walls filled in.

    Returns how tall the whole thing now stands, because everything above a roof -
    a chimney, a dormer, a weathervane - is placed against its ridge.
    """
    span = wide if ridge == "y" else deep
    rise = span * pitch
    parts.append(
        wedge(wide + over * 2.0, deep + over * 2.0, rise, (0.0, 0.0, base), colour, ridge=ridge)
    )
    eaves(parts, wide, deep, base + 0.02, over, "timber", ridge=ridge)
    if colour in ("slate", "thatch", "roof", "roof2"):
        shingles(parts, wide, deep, base, rise, ridge=ridge,
                 colour="shingle" if colour != "thatch" else "straw", over=over)
    # The triangle of wall under each slope, or the roof is a lid on an open box.
    if ridge == "y":
        for side in (-1.0, 1.0):
            parts.append(
                gable_wall(wide, rise, (0.0, side * (deep * 0.5 - WALL * 0.5), base), WALL, "plaster2", facing="y")
            )
    else:
        for side in (-1.0, 1.0):
            parts.append(
                gable_wall(deep, rise, (side * (wide * 0.5 - WALL * 0.5), 0.0, base), WALL, "plaster2", facing="x")
            )
    return base + rise


# # Most doors do not open, and that is the convention rather than a shortcut
#
# "We probably don't actually need to have every building open. Lets just do
# essentials like guild halls, stores and some homes."
#
# Which is how these games have always worked: a Pokemon town has a dozen houses and
# four of them let you in. The ones that do not are not FAKE - they are buildings
# somebody lives in, with the door shut, and that reads as a town where people have
# their own business rather than as a showroom.
#
# A shut building is the same figure with its doorway filled by a closed door and no
# interior built at all. That is most of its cost gone - a cottage's room, hearth and
# bed are half its pieces - so a town can carry more of them for less.


def shut_the_door(parts, wide, deep, floor=0.0):
    """Fills the doorway with a door that is closed, and frames it."""
    leaf = DOOR_WIDE - 0.06
    parts.append(box((leaf, 0.09, DOOR_TALL - 0.04),
                     (0.0, -deep * 0.5 - 0.02, floor + (DOOR_TALL - 0.04) * 0.5), "door"))
    # A handle, so it reads as shut rather than as boarded up.
    parts.append(box((0.12, 0.12, 0.12),
                     (leaf * 0.32, -deep * 0.5 - 0.12, floor + DOOR_TALL * 0.45), "brass"))
    # And the room behind it goes dark, so nothing shows through the windows.
    parts.append(box((wide - WALL * 2.2, deep - WALL * 2.2, 0.12),
                     (0.0, 0.0, floor + 0.06), "board"))


def cottage(open_door=True):
    """One room under a steep thatch. The commonest thing in a village.

    `open_door` false builds the same cottage with its door shut and no interior -
    see `shut_the_door`. Parameterised rather than copied, so the two can never
    drift into being different cottages.
    """
    wide, deep = MODULE * 6, MODULE * 5
    parts = []
    parts.append(box((wide + 0.34, deep + 0.34, PLINTH), (0.0, 0.0, PLINTH * 0.5), "stone"))
    courses(parts, wide + 0.34, deep + 0.34, 0.0, PLINTH)
    doorstep(parts, deep, PLINTH)
    holes = {}

    shell(parts, wide, deep, 1, "plaster", doors=True, windows=True, openings=holes)
    if open_door:
        room(parts, wide, deep, 1)
        hearth(parts, wide, deep)
        bed(parts, wide, deep)
    else:
        shut_the_door(parts, wide, deep)
    framing(parts, wide, deep, 0.0, STOREY, openings=holes)
    porch(parts, deep, 0.0, roof_colour="thatch")
    # Ridge across the front, so the cottage shows its long eaves to the street and
    # its gable to its neighbour - the opposite of the shop, which is what stops a
    # row of them reading as a terrace.
    top = roofed(parts, wide, deep, STOREY, "thatch", ridge="x", pitch=0.62)
    chimney(parts, (-wide * 0.5 + 0.55, deep * 0.2), STOREY - 0.4, top + 0.5)
    # Flower boxes under the front windows.
    for side in (-1.0, 1.0):
        flowerbox(parts, "x", (side * wide * 0.31, -deep * 0.5 + WALL * 0.5, 0.0), WINDOW_WIDE, WINDOW_SILL)
    return parts, top + 0.8


def townhouse(open_door=True):
    """Two storeys, the upper one jettied out over the lower. What a town is made of."""
    wide, deep = MODULE * 6, MODULE * 6
    jetty = 0.28
    parts = []
    parts.append(box((wide + 0.34, deep + 0.34, PLINTH), (0.0, 0.0, PLINTH * 0.5), "stone"))
    courses(parts, wide + 0.34, deep + 0.34, 0.0, PLINTH)
    doorstep(parts, deep, PLINTH)
    holes = {}

    shell(parts, wide, deep, 2, "plaster", doors=True, windows=True, openings=holes)
    if open_door:
        room(parts, wide, deep, 2)
        stairs(parts, wide, deep, 2)
        hearth(parts, wide, deep)
        table(parts, (-0.3, -0.9), 1.4, 1.0)
    else:
        shut_the_door(parts, wide, deep)
    framing(parts, wide, deep, 0.0, STOREY, openings=holes)
    framing(parts, wide, deep, STOREY, STOREY)

    # THE JETTY: the upper storey oversails the lower on brackets. One medium shape,
    # and the single thing that most says "town house" rather than "two-storey box".
    parts.append(box((wide + jetty * 2.0, deep + jetty * 2.0, 0.22), (0.0, 0.0, STOREY - 0.11), "timber"))
    for side in (-1.0, 1.0):
        for over in (-1.0, 0.0, 1.0):
            parts.append(
                box((0.16, 0.5, 0.4), (over * wide * 0.33, side * (deep * 0.5 + 0.1), STOREY - 0.42), "timber")
            )
    top = roofed(parts, wide + jetty, deep + jetty, STOREY * 2, "roof", ridge="x")
    chimney(parts, (wide * 0.5 - 0.6, -deep * 0.15), STOREY * 2 - 0.4, top + 0.6)

    # A dormer: a small gable poking out of the roof slope. Breaks the ridge line,
    # which is the other half of what a jetty does for the wall line.
    parts.append(box((1.3, 1.0, 1.0), (-wide * 0.18, -deep * 0.5 - jetty * 0.5 + 0.5, STOREY * 2 + 0.5), "plaster2"))
    parts.append(wedge(1.6, 1.3, 0.6, (-wide * 0.18, -deep * 0.5 - jetty * 0.5 + 0.5, STOREY * 2 + 1.0), "roof", ridge="x"))
    parts.append(box((0.75, 0.06, 0.6), (-wide * 0.18, -deep * 0.5 - jetty * 0.5 - 0.02, STOREY * 2 + 0.55), "glass"))
    return parts, top + 0.9


def shop():
    """Gable to the street, a big display window, and a sign hung out in the light."""
    wide, deep = MODULE * 8, MODULE * 6
    parts = []
    parts.append(box((wide + 0.34, deep + 0.34, PLINTH), (0.0, 0.0, PLINTH * 0.5), "stone"))
    courses(parts, wide + 0.34, deep + 0.34, 0.0, PLINTH)
    doorstep(parts, deep, PLINTH, wide=3.0)
    holes = {}

    shell(parts, wide, deep, 1, "plaster", doors=True, windows=True, openings=holes)
    room(parts, wide, deep, 1)
    counter(parts, wide, deep)
    stock(parts, wide, deep)
    framing(parts, wide, deep, 0.0, STOREY, openings=holes)

    # Ridge along Y, so the GABLE faces the street. A shopfront wants the tall
    # triangle of wall above it - that is where a trade sign goes and it is what
    # makes a shop taller than the houses either side without being bigger.
    top = roofed(parts, wide, deep, STOREY, "roof", ridge="y", pitch=0.5)

    # A lean-to store room down one side: a second, lower mass. Two masses of
    # different heights is the cheapest way to stop a building reading as one box.
    lean_wide = 1.5
    parts.append(box((lean_wide, deep * 0.7, STOREY * 0.72), (wide * 0.5 + lean_wide * 0.5, -deep * 0.08, STOREY * 0.36), "plaster2"))
    parts.append(
        lean(lean_wide + 0.5, deep * 0.7 + 0.4, 0.55, (wide * 0.5 + lean_wide * 0.5, -deep * 0.08, STOREY * 0.72), "roof2", drops_to=0.0)
    )

    # The sign, on a bracket out where the light is - not under an awning. See the
    # note in the git history for why there is no awning.
    parts.append(box((0.14, 0.14, 3.1), (wide * 0.5 + 0.25, -deep * 0.5 - 0.1, 1.55), "timber"))
    parts.append(box((0.14, 1.0, 0.14), (wide * 0.5 + 0.25, -deep * 0.5 - 0.55, 3.0), "timber"))
    parts.append(box((0.08, 0.8, 0.6), (wide * 0.5 + 0.25, -deep * 0.5 - 0.75, 2.55), "sign"))
    parts.append(
        tube(0.22, 0.09, (wide * 0.5 + 0.31, -deep * 0.5 - 0.75, 2.55), "brass", sides=14,
             tilt=(0.0, math.pi * 0.5, 0.0))
    )
    # Crates outside, because a shop with nothing in front of it is a house.
    for index in range(3):
        parts.append(
            box((0.5, 0.5, 0.5), (-wide * 0.5 + 0.45 + index * 0.15, -deep * 0.5 - 0.45, 0.25 + index * 0.5), "counter")
        )
    return parts, top + 0.6


def guild_hall():
    """The building a city is a city because it has. Stone, buttressed, and towered."""
    wide, deep = MODULE * 12, MODULE * 9
    parts = []
    # A BROAD SHALLOW APPROACH, not a flight.
    #
    # This was a four-step ceremonial climb onto a 70 cm plinth, on the reasoning
    # that a hall you climb to is a hall that matters. It is - and a warden walks on
    # the TERRAIN, not on step geometry, so those steps were scenery in front of a
    # 70 cm wall and the hall was the least enterable building in the town.
    #
    # The ceremony survives as WIDTH rather than as height: two long shallow courses
    # spanning most of the frontage, climbing the same threshold every other
    # building uses. It still reads as an approach; it no longer refuses anyone.
    for step in range(2):
        parts.append(
            box(
                (wide * 0.55 - step * 0.6, 1.5 - step * 0.5, PLINTH * 0.5),
                (0.0, -deep * 0.5 - 1.2 + step * 0.5, PLINTH * 0.25 + step * PLINTH * 0.5),
                "stone",
            )
        )
    parts.append(box((wide + 0.6, deep + 0.6, PLINTH), (0.0, 0.0, PLINTH * 0.5), "stone"))
    holes = {}

    shell(parts, wide, deep, 2, "stone", doors=True, windows=True, openings=holes)
    room(parts, wide, deep, 2)
    stairs(parts, wide, deep, 2, side=-1.0)
    guild_hall_inside(parts, wide, deep)

    # BUTTRESSES. Sloped piers against the long walls - the medium shape that says
    # stone hall rather than stone box, and they read from a long way off.
    for side in (-1.0, 1.0):
        for over in (-1.0, 0.0, 1.0):
            at_y = over * deep * 0.3
            parts.append(box((0.5, 0.7, STOREY * 1.5), (side * (wide * 0.5 + 0.2), at_y, STOREY * 0.75), "stone"))
            parts.append(
                lean(0.5, 0.7, 0.0, (side * (wide * 0.5 + 0.2), at_y, STOREY * 1.5), "slate", drops_to=0.45)
            )

    top = roofed(parts, wide, deep, STOREY * 2, "slate", ridge="x", pitch=0.5)

    # The tower, off to one corner and taller than the ridge, with a spire.
    tower = MODULE * 2
    tx, ty = wide * 0.5 - tower * 0.5, deep * 0.5 - tower * 0.5
    tower_top = STOREY * 2 + 3.4
    parts.append(box((tower, tower, tower_top), (tx, ty, tower_top * 0.5), "stone"))
    parts.append(box((tower + 0.3, tower + 0.3, 0.22), (tx, ty, tower_top - 0.11), "stone"))
    for side in (-1.0, 1.0):
        parts.append(box((0.5, 0.08, 0.9), (tx + side * 0.55, ty - tower * 0.5 - 0.02, tower_top - 1.3), "glass"))
    # A spire, not a hip: four faces to a point is the shape a guild puts on a map.
    parts.append(wedge(tower + 0.5, tower + 0.5, 2.4, (tx, ty, tower_top), "roof2", ridge="y"))
    parts.append(wedge(tower + 0.5, tower + 0.5, 2.4, (tx, ty, tower_top), "roof2", ridge="x"))
    parts.append(tube(0.06, 1.0, (tx, ty, tower_top + 2.8), "brass", sides=8))

    # The guild's colours over the door, and a banner either side of it.
    parts.append(box((2.4, 0.12, 1.1), (0.0, -deep * 0.5 - 0.12, 3.9), "guild"))
    parts.append(
        tube(0.4, 0.14, (0.0, -deep * 0.5 - 0.22, 3.9), "brass", sides=18, tilt=(math.pi * 0.5, 0.0, 0.0))
    )
    for side in (-1.0, 1.0):
        parts.append(box((0.9, 0.07, 2.6), (side * 2.6, -deep * 0.5 - 0.1, 4.3), "guild"))
    return parts, max(top, tower_top + 3.3)


# # THE CITY IS A DIFFERENT AGE OF THE WORLD
#
# Villages and towns are old-school fantasy - half-timbered, thatch and slate, a
# cobbled street. Cities are modern: curtain wall, concrete, and a paved road. That
# is not two art styles bolted together, it is the game's own history showing on the
# ground, and it is the sharpest District tool there is - you know which kind of
# place you are in from the silhouette before you can read a sign.
#
# The pieces below are built from the same `masonry` boxes as everything else, so a
# tower costs what a cottage costs and wears the same shading.

FLOOR_TALL = 3.4          # one storey of a modern building
CORE = 0.34               # how far the service core stands proud of the facade


def curtain_wall(parts, wide, deep, floors, base=0.0, banded=True):
    """A modern facade: floor bands of glass between slim spandrels.

    The whole readable difference between a tower and a big shed is that a tower has
    a REPEAT you can count. Bands do that at any distance, and they cost two boxes a
    storey a side rather than a window apiece.
    """
    # THE WALL BEHIND THE GLASS.
    #
    # A curtain wall was a stack of glass bands and spandrels standing in mid air with
    # nothing behind them - and they did not even meet: 0.62 of a storey of glass over
    # 0.34 of spandrel leaves four per cent of every floor as a gap you can see
    # straight through, into the building and out the far side. A tower has to be a
    # SOLID before it is a facade.
    solid = base + floors * FLOOR_TALL
    for face, span, off in (("x", wide, deep), ("y", deep, wide)):
        for side in (-1.0, 1.0):
            # BEHIND the glass, not level with it. Level, a 0.22 m backing wall
            # stands proud of a 0.14 m pane and hides the whole facade: the towers
            # came out as blank concrete slabs with no windows at all.
            back = 0.11 + 0.08
            at = (
                (0.0, side * (off * 0.5 - back), 0.0)
                if face == "x"
                else (side * (off * 0.5 - back), 0.0, 0.0)
            )
            parts.append(box(
                _slab(face, span, 0.22, solid - base),
                (at[0], at[1], base + (solid - base) * 0.5), "concrete2"))
            at = (0.0, side * off * 0.5, 0.0) if face == "x" else (side * off * 0.5, 0.0, 0.0)

    for floor in range(floors):
        z = base + floor * FLOOR_TALL
        glass = "curtain" if (floor % 2 == 0 or not banded) else "curtain2"
        for face, span, off in (("x", wide, deep), ("y", deep, wide)):
            for side in (-1.0, 1.0):
                at = (0.0, side * off * 0.5, 0.0) if face == "x" else (side * off * 0.5, 0.0, 0.0)
                # Glass and spandrel together fill the storey exactly - 0.66 and 0.34 -
                # so a floor has no seam in it.
                parts.append(box(
                    _slab(face, span * 0.94, 0.14, FLOOR_TALL * 0.66),
                    (at[0], at[1], z + FLOOR_TALL * 0.67), glass))
                parts.append(box(
                    _slab(face, span, 0.20, FLOOR_TALL * 0.34),
                    (at[0], at[1], z + FLOOR_TALL * 0.17), "concrete"))
                # Mullions, one every couple of metres, which is what stops a band
                # reading as a stripe of paint.
                count = max(2, int(span / 2.2))
                for index in range(count + 1):
                    over = -span * 0.47 + span * 0.94 * index / count
                    place = _across(face, (at[0], at[1], z + FLOOR_TALL * 0.5), over)
                    parts.append(box(_slab(face, 0.10, 0.14, FLOOR_TALL * 0.62), place, "mullion"))


def tower(floors, wide=9.0, deep=9.0, crown="flat", lobby_open=True):
    """A city block: a modern tower with a lobby, a core and a crown.

    Three parts, because that is what makes a tall box read as a building rather
    than as a wall: a BASE you meet at street level, a SHAFT that repeats, and a TOP
    that finishes. A tower without a top is a building that got cut off.
    """
    parts = []
    tall = FLOOR_TALL * floors

    # The base: taller than a storey, glassier, with a canopy over the door.
    lobby = FLOOR_TALL * 1.5
    parts.append(box((wide + 0.5, deep + 0.5, 0.14), (0.0, 0.0, 0.07), "concrete2"))
    for face, span, off in (("x", wide, deep), ("y", deep, wide)):
        for side in (-1.0, 1.0):
            at = (0.0, side * off * 0.5, 0.0) if face == "x" else (side * off * 0.5, 0.0, 0.0)
            parts.append(box(_slab(face, span, 0.14, lobby - 0.3), (at[0], at[1], lobby * 0.5), "curtain"))
            parts.append(box(_slab(face, span, 0.22, 0.30), (at[0], at[1], lobby - 0.15), "concrete"))
    # The doorway, in the -Y face like every other building in this game.
    hole = min(DOOR_WIDE * 1.6, wide - 1.0)
    for side in (-1.0, 1.0):
        pier = (wide - hole) * 0.5
        parts.append(box((pier, 0.30, DOOR_TALL + 0.4),
                         (side * (wide - pier) * 0.5, -deep * 0.5, (DOOR_TALL + 0.4) * 0.5), "concrete2"))
    parts.append(box((hole + 0.6, 0.30, 0.34), (0.0, -deep * 0.5, DOOR_TALL + 0.55), "concrete2"))
    parts.append(box((hole + 2.2, 1.5, 0.18), (0.0, -deep * 0.5 - 0.75, DOOR_TALL + 0.95), "canopy"))

    # THE LOBBY YOU CAN WALK INTO.
    #
    # Every other building in this game is enterable and a tower has to be too - a
    # city of sealed boxes is a city you look at rather than one you are in. Only
    # the ground floor: the doorway leads into a lobby with a desk and a lift bank,
    # which is what a tower's ground floor IS, and the floors above are shell.
    inner = (wide - 0.5, deep - 0.5)
    if not lobby_open:
        # SHUT: the glass front stays and the way in does not. A dark slab behind it
        # so nothing shows through, and no fit-out at all - most of this figure's
        # cost, gone.
        parts.append(box((hole + 0.24, 0.12, DOOR_TALL + 0.34),
                         (0.0, -deep * 0.5 - 0.06, (DOOR_TALL + 0.34) * 0.5), "curtain2"))
        parts.append(box((inner[0], inner[1], 0.12), (0.0, 0.0, 0.06), "board"))
    parts.append(box((inner[0], inner[1], 0.10), (0.0, 0.0, 0.05), "infloor"))
    parts.append(box((inner[0], inner[1], 0.14), (0.0, 0.0, lobby - 0.07), "inwall"))
    for face, span, off in (("x", inner[0], inner[1]), ("y", inner[1], inner[0])):
        for side in (-1.0, 1.0):
            # The front wall is the glass and the doorway, so it is left open.
            if face == "x" and side < 0.0:
                continue
            at = (0.0, side * off * 0.5, 0.0) if face == "x" else (side * off * 0.5, 0.0, 0.0)
            parts.append(box(_slab(face, span, 0.12, lobby - 0.2),
                             (at[0], at[1], (lobby - 0.2) * 0.5), "inwall"))
    # A desk facing the door, and a bank of lifts on the back wall.
    if lobby_open:
        parts.append(box((3.0, 0.8, 1.05), (-wide * 0.14, deep * 0.16, 0.58), "counter"))
        parts.append(box((3.2, 0.24, 0.12), (-wide * 0.14, deep * 0.16 - 0.4, 1.14), "board"))
    for lift in (-1.0, 1.0) if lobby_open else ():
        parts.append(box((1.5, 0.16, 2.3), (lift * 2.1, inner[1] * 0.5 - 0.14, 1.15), "steel"))
        parts.append(box((0.3, 0.2, 0.3), (lift * 2.1, inner[1] * 0.5 - 0.24, 2.7), "neon"))

    # The shaft.
    curtain_wall(parts, wide, deep, floors - 1, base=lobby)

    # The service core, standing proud on one flank the whole way up - the thing
    # that stops a tower being a rectangle from every angle.
    parts.append(box((CORE * 2.0, deep * 0.42, tall - lobby * 0.4),
                     (wide * 0.5 + CORE * 0.6, deep * 0.12, lobby * 0.2 + (tall - lobby * 0.4) * 0.5),
                     "concrete2"))

    # The crown.
    top = lobby + (floors - 1) * FLOOR_TALL
    parts.append(box((wide + 0.6, deep + 0.6, 0.5), (0.0, 0.0, top + 0.25), "parapet"))
    if crown == "stepped":
        parts.append(box((wide * 0.68, deep * 0.68, FLOOR_TALL * 1.1),
                         (0.0, 0.0, top + 0.5 + FLOOR_TALL * 0.55), "concrete"))
        parts.append(box((wide * 0.70, deep * 0.70, 0.36),
                         (0.0, 0.0, top + 0.5 + FLOOR_TALL * 1.1), "parapet"))
        top += 0.5 + FLOOR_TALL * 1.1
    elif crown == "mast":
        parts.append(box((0.9, 0.9, FLOOR_TALL * 0.8), (0.0, 0.0, top + FLOOR_TALL * 0.4), "concrete2"))
        parts.append(box((0.22, 0.22, FLOOR_TALL * 1.6), (0.0, 0.0, top + FLOOR_TALL * 1.2), "steel"))
        parts.append(box((0.5, 0.5, 0.3), (0.0, 0.0, top + FLOOR_TALL * 0.55), "neon"))
        top += FLOOR_TALL * 2.0
    # Rooftop plant, so the skyline is not a row of flat lids.
    parts.append(box((wide * 0.3, deep * 0.3, 0.9), (-wide * 0.18, deep * 0.16, top + 0.5), "steel"))
    return parts, top + 1.0


def city_block():
    """A mid-rise: the ordinary building of a city street."""
    return tower(5, wide=10.5, deep=9.0, crown="flat")


def city_tower():
    """Taller, and stepped - the one that starts to read as a skyline."""
    return tower(9, wide=10.0, deep=9.5, crown="stepped")


def city_spire():
    """The tallest thing for miles. A WEENIE, in Disney's sense.

    Scott Rogers' hub-town rules name it: something tall enough to see from outside
    the place, that pulls you toward the middle of it. A city whose tallest building
    is the same as its second tallest has no middle.
    """
    # Fourteen floors, not fifteen. Fifteen came out at 60.5 m and the export gate
    # refuses anything over 60 - a bound chosen to catch the classic scale mistake, a
    # 1.8 m figure built in centimetres. The guard is right and half a metre of
    # skyscraper is not worth widening it for; this is 57 m and still six times the
    # cottage beside it.
    return tower(14, wide=11.0, deep=11.0, crown="mast")

def market_cross():
    """A village's landmark: a stepped stone cross on the square.

    The other half of Rogers' rule, and the half my towns were missing: a landmark
    has to be a DIFFERENT KIND OF THING from the buildings around it, not a bigger
    one. A tall house is a house. A cross on three steps is a place to meet at.
    """
    parts = []
    for step in range(3):
        size = 3.4 - step * 0.7
        parts.append(box((size, size, 0.24), (0.0, 0.0, 0.12 + step * 0.24), "stone"))
    base = 0.72
    parts.append(box((0.9, 0.9, 0.5), (0.0, 0.0, base + 0.25), "stone"))
    parts.append(box((0.42, 0.42, 3.6), (0.0, 0.0, base + 0.5 + 1.8), "stone"))
    # The head: a squared cross, which reads from every side.
    top = base + 0.5 + 3.6
    parts.append(box((1.5, 0.34, 0.34), (0.0, 0.0, top + 0.2), "stone"))
    parts.append(box((0.34, 1.5, 0.34), (0.0, 0.0, top + 0.2), "stone"))
    parts.append(box((0.5, 0.5, 0.6), (0.0, 0.0, top + 0.6), "brass"))
    return parts, top + 0.9


def well():
    """A junction landmark for a village: a well with a roof over it."""
    parts = []
    parts.append(box((2.2, 2.2, 0.22), (0.0, 0.0, 0.11), "stone"))
    for side in (-1.0, 1.0):
        for face in (-1.0, 1.0):
            parts.append(box((1.6 if face > 0 else 0.3, 0.3 if face > 0 else 1.6, 0.7),
                             (0.0 if face > 0 else side * 0.75, side * 0.75 if face > 0 else 0.0, 0.57),
                             "stone"))
    for side in (-1.0, 1.0):
        parts.append(box((0.16, 0.16, 2.0), (side * 0.85, 0.0, 1.9), "timber"))
    parts.append(box((0.14, 1.4, 0.14), (0.0, 0.0, 2.75), "timber"))
    parts.append(wedge(2.4, 2.0, 0.7, (0.0, 0.0, 2.9), "roof2", ridge="y"))
    parts.append(box((0.5, 0.5, 0.4), (0.0, 0.0, 2.3), "board"))
    return parts, 3.7


def monument():
    """A city's junction landmark: a raised plinth under a steel figure."""
    parts = []
    parts.append(box((5.0, 5.0, 0.3), (0.0, 0.0, 0.15), "concrete2"))
    parts.append(box((3.4, 3.4, 0.3), (0.0, 0.0, 0.45), "concrete"))
    parts.append(box((1.6, 1.6, 2.2), (0.0, 0.0, 1.7), "concrete2"))
    parts.append(box((2.0, 2.0, 0.24), (0.0, 0.0, 2.9), "parapet"))
    # A spike of steel, leaning - a shape nothing else in the city has.
    parts.append(box((0.5, 0.5, 4.4), (0.0, 0.0, 5.2), "steel", tilt=(0.16, 0.0, 0.5)))
    parts.append(box((1.1, 1.1, 0.3), (0.0, 0.0, 7.6), "neon"))
    return parts, 8.0


FIGURES = {
    # The old world: villages and towns.
    "cottage": cottage,
    "townhouse": townhouse,
    "shop": shop,
    "guild_hall": guild_hall,
    "market_cross": market_cross,
    "well": well,
    # The new: cities.
    "city_block": city_block,
    # The same figures with the door shut and no interior. Most doors in a town do
    # not open, which is the genre's own convention - see `shut_the_door`.
    "cottage_shut": lambda: cottage(open_door=False),
    "townhouse_shut": lambda: townhouse(open_door=False),
    "city_block_shut": lambda: tower(5, wide=10.5, deep=9.0, crown="flat", lobby_open=False),
    "city_tower": city_tower,
    "city_spire": city_spire,
    "monument": monument,
}


def build(name: str) -> None:
    masonry.fresh()
    parts, tall = FIGURES[name]()

    def floor_of(colour):
        # An indoor piece shades from the storey it stands on. Which storey that is
        # comes from the piece's own height, so nothing has to be told twice.
        return 0.0 if colour not in INDOORS else 0.0

    whole = masonry.weld(parts, PAINT, tall, name="prop", floor_of=floor_of)
    # And an edge on it, so it does not dissolve into whatever is behind it.
    masonry.outline(whole)
    masonry.save_beside(f"town_{name}.blend")
    print(f"BUILT town_{name}  ({len(parts)} pieces, {tall:.1f} m tall)")


for figure in FIGURES:
    build(figure)
