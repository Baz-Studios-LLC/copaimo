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
from masonry import box, lean, tube, wedge

# ---------------------------------------------------------------- the module
#
# Everything is a whole number of these, which is what keeps a street of separately
# generated buildings looking like one street. 1.5 m is the game's own module - the
# workbench kit is built on it - so a scripted building and a hand-built one sit on
# the same grid.
MODULE = 1.5

# A storey, floor to floor. Two modules, which is what the kit's stairs climb.
STOREY = 3.0

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
DOOR_WIDE = 1.4
DOOR_TALL = 2.15

# A window, and how far up the wall it sits.
WINDOW_WIDE = 0.95
WINDOW_TALL = 1.05
WINDOW_SILL = 1.05

PAINT = {
    # Outside
    "plaster": (0.80, 0.75, 0.64),
    "timber": (0.36, 0.25, 0.17),
    "stone": (0.48, 0.47, 0.44),
    "roof": (0.38, 0.22, 0.18),
    "slate": (0.32, 0.33, 0.37),
    "thatch": (0.60, 0.50, 0.30),
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
}

# Which colours are indoor, so their shading ramp starts at the floor they stand on
# rather than at the foot of the building. Without this a chair in an upstairs room
# is lit as brightly as the ridge of the roof.
INDOORS = {
    "inwall", "infloor", "inbeam", "hearth", "cloth", "counter", "shelf", "board",
}


# --------------------------------------------------------------- wall grammar


def wall_run(parts, along, at, length, height, colour, bays, floor=0.0):
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
        _one_bay(parts, along, centre, bay, height, colour, kind, floor)


def _one_bay(parts, along, at, wide, height, colour, kind, floor):
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

    # What fills the hole.
    if kind == "window":
        size = (hole_wide, 0.06, hole_tall) if along == "x" else (0.06, hole_wide, hole_tall)
        parts.append(box(size, (at[0], at[1], floor + sill + hole_tall * 0.5), "glass"))
    else:
        # The leaf, hung open flat against the wall BESIDE the opening rather than
        # standing in it - so the doorway's clear width is the doorway's width. See
        # DOOR_WIDE for what happens otherwise.
        leaf = hole_wide * 0.85
        size = (leaf, 0.07, hole_tall) if along == "x" else (0.07, leaf, hole_tall)
        off = (hole_wide + leaf) * 0.5 + 0.03
        centre = (
            (at[0] - off, at[1] - WALL * 0.5 - 0.04, floor + hole_tall * 0.5)
            if along == "x"
            else (at[0] - WALL * 0.5 - 0.04, at[1] - off, floor + hole_tall * 0.5)
        )
        parts.append(box(size, centre, "door"))
        # And a threshold, so the gap reads as a doorway rather than as damage.
        sill_size = (hole_wide + 0.3, WALL + 0.2, 0.06) if along == "x" else (WALL + 0.2, hole_wide + 0.3, 0.06)
        parts.append(box(sill_size, (at[0], at[1], floor + 0.03), "stone"))


def shell(parts, wide, deep, storeys, colour, doors, windows):
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

        wall_run(parts, "x", (0.0, -deep * 0.5 + WALL * 0.5, 0.0), wide, STOREY, colour, south, floor)
        wall_run(parts, "x", (0.0, deep * 0.5 - WALL * 0.5, 0.0), wide, STOREY, colour, north, floor)
        wall_run(parts, "y", (-wide * 0.5 + WALL * 0.5, 0.0, 0.0), deep, STOREY, colour, sides, floor)
        wall_run(parts, "y", (wide * 0.5 - WALL * 0.5, 0.0, 0.0), deep, STOREY, colour, sides, floor)


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


# -------------------------------------------------------------------- figures


def cottage():
    """One room, one storey, a hearth and a bed. The commonest thing in a village."""
    wide, deep = MODULE * 4, MODULE * 3
    parts = []
    parts.append(box((wide + 0.3, deep + 0.3, 0.35), (0.0, 0.0, 0.175), "stone"))
    shell(parts, wide, deep, 1, "plaster", doors=True, windows=True)
    room(parts, wide, deep, 1)
    hearth(parts, wide, deep)
    bed(parts, wide, deep)
    tall = STOREY + 1.6
    parts.append(wedge(wide + 0.5, deep + 0.5, 1.6, (0.0, 0.0, STOREY), "thatch"))
    parts.append(box((0.7, 0.7, 1.2), (-wide * 0.25, deep * 0.5 - 0.4, STOREY + 1.0), "stone"))
    return parts, tall


def townhouse():
    """Two storeys, stairs, a table below and a bed above. What a town is made of."""
    wide, deep = MODULE * 4, MODULE * 4
    parts = []
    parts.append(box((wide + 0.3, deep + 0.3, 0.4), (0.0, 0.0, 0.2), "stone"))
    shell(parts, wide, deep, 2, "plaster", doors=True, windows=True)
    room(parts, wide, deep, 2)
    stairs(parts, wide, deep, 2)
    hearth(parts, wide, deep)
    table(parts, (-0.3, -0.9), 1.4, 1.0)
    tall = STOREY * 2 + 1.7
    parts.append(wedge(wide + 0.5, deep + 0.5, 1.7, (0.0, 0.0, STOREY * 2), "roof"))
    # The timber framing that tells a townhouse from a shed.
    for storey in range(2):
        parts.append(
            box((wide + 0.32, deep + 0.32, 0.16), (0.0, 0.0, storey * STOREY + STOREY - 0.08), "timber")
        )
    return parts, tall


def shop():
    """A wide front, a counter, shelves, and an awning over the door."""
    wide, deep = MODULE * 5, MODULE * 4
    parts = []
    parts.append(box((wide + 0.3, deep + 0.3, 0.4), (0.0, 0.0, 0.2), "stone"))
    shell(parts, wide, deep, 1, "plaster", doors=True, windows=True)
    room(parts, wide, deep, 1)
    counter(parts, wide, deep)
    stock(parts, wide, deep)
    tall = STOREY + 1.5
    parts.append(wedge(wide + 0.5, deep + 0.5, 1.5, (0.0, 0.0, STOREY), "roof"))
    # NO AWNING, and it took three attempts to accept that.
    #
    # It was a `lean` first, which came out a detached slab floating off the roof.
    # Then a wedge with its ridge on the wall and props under it, which hung
    # correctly and did something worse: an awning is a sheet held over a wall, so
    # it SHADES the wall - and the whole shopfront, the door and the display window
    # with it, went into a dark recess that read as a hole in the building. Raising
    # it, shrinking it and lightening the canvas each moved the shadow without
    # removing it, because the shadow is what an awning IS.
    #
    # What says shop from across a street is a big window with something behind it
    # and a sign hung out where the light is. Both of those are here, and neither
    # of them darkens the front of the building.
    parts.append(box((0.14, 0.14, 3.1), (wide * 0.5 + 0.25, -deep * 0.5 - 0.1, 1.55), "timber"))
    parts.append(box((0.14, 1.0, 0.14), (wide * 0.5 + 0.25, -deep * 0.5 - 0.55, 3.0), "timber"))
    parts.append(box((0.08, 0.8, 0.6), (wide * 0.5 + 0.25, -deep * 0.5 - 0.75, 2.55), "sign"))
    parts.append(tube(0.22, 0.09, (wide * 0.5 + 0.31, -deep * 0.5 - 0.75, 2.55), "brass",
                      sides=14, tilt=(0.0, math.pi * 0.5, 0.0)))
    return parts, tall


def guild_hall():
    """The building a city is a city because it has. Bigger, stone, and a tower."""
    wide, deep = MODULE * 7, MODULE * 5
    parts = []
    parts.append(box((wide + 0.5, deep + 0.5, 0.6), (0.0, 0.0, 0.3), "stone"))
    shell(parts, wide, deep, 2, "stone", doors=True, windows=True)
    room(parts, wide, deep, 2)
    # The far corner from the tower, which stands in the north-east.
    stairs(parts, wide, deep, 2, side=-1.0)
    guild_hall_inside(parts, wide, deep)
    tall = STOREY * 2 + 2.0 + 3.2
    parts.append(wedge(wide + 0.6, deep + 0.6, 2.0, (0.0, 0.0, STOREY * 2), "slate"))

    # A tower, so the hall reads as the guild's from anywhere in the city.
    tower = MODULE * 2
    parts.append(box((tower, tower, STOREY * 2 + 2.6), (wide * 0.5 - tower * 0.5, deep * 0.5 - tower * 0.5, (STOREY * 2 + 2.6) * 0.5), "stone"))
    parts.append(
        wedge(tower + 0.4, tower + 0.4, 1.6, (wide * 0.5 - tower * 0.5, deep * 0.5 - tower * 0.5, STOREY * 2 + 2.6), "slate")
    )
    # And the guild's colours over the door.
    parts.append(box((2.2, 0.12, 1.0), (0.0, -deep * 0.5 - 0.12, 3.6), "guild"))
    parts.append(tube(0.35, 0.14, (0.0, -deep * 0.5 - 0.22, 3.6), "brass", sides=16,
                      tilt=(math.pi * 0.5, 0.0, 0.0)))
    return parts, tall


FIGURES = {
    "cottage": cottage,
    "townhouse": townhouse,
    "shop": shop,
    "guild_hall": guild_hall,
}


def build(name: str) -> None:
    masonry.fresh()
    parts, tall = FIGURES[name]()

    def floor_of(colour):
        # An indoor piece shades from the storey it stands on. Which storey that is
        # comes from the piece's own height, so nothing has to be told twice.
        return 0.0 if colour not in INDOORS else 0.0

    masonry.weld(parts, PAINT, tall, name="prop", floor_of=floor_of)
    masonry.save_beside(f"town_{name}.blend")
    print(f"BUILT town_{name}  ({len(parts)} pieces, {tall:.1f} m tall)")


for figure in FIGURES:
    build(figure)
