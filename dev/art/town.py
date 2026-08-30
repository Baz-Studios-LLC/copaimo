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

# The palette lives in `masonry` now, because it is shared.
#
# `yard.py` builds the gardens, pens and work yards that stand on the lots this file
# does not put a building on, and they have to be painted out of the same pot - a
# garden fence in a slightly different brown from the cottage behind it is the one
# thing that would say "two generators" out loud.
PAINT = masonry.PALETTE

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
    for (middle, bay), kind in zip(bay_places(length, bays), bays):
        centre = (
            (at[0] + middle, at[1], at[2]) if along == "x" else (at[0], at[1] + middle, at[2])
        )
        # How much wall is left either side of this bay, so nothing dressed onto it
        # can hang off the end of the building.
        room = (length * 0.5 + middle, length * 0.5 - middle)
        _one_bay(parts, along, centre, bay, height, colour, kind, floor, facing, room)


def _one_bay(parts, along, at, wide, height, colour, kind, floor, facing=-1.0, room=None):
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

    hole_wide = hole_in(kind, wide)
    hole_tall = DOOR_TALL if kind == "door" else WINDOW_TALL
    sill = 0.0 if kind == "door" else WINDOW_SILL

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
        _dress_window(parts, along, at, hole_wide, hole_tall, mid, floor + sill, facing, room)
    else:
        # The leaf, hung open flat against the wall BESIDE the opening rather than
        # standing in it - so the doorway's clear width is the doorway's width. See
        # DOOR_WIDE for what happens otherwise.
        # THE DOOR SWINGS IN.
        #
        # It stood flat against the wall OUTSIDE, beside its own opening - and a
        # facade has windows beside its door. On the shop the leaf spans 0.98 to 2.60
        # left of the doorway and the next window spans 1.47 to 2.42, so the open door
        # was parked squarely on the glass: reported as "a window behind the door".
        #
        # A door that swings outward is also the wrong door. Nothing in this town has
        # a porch deep enough to swing into, and a leaf on the street side is a leaf
        # somebody walks into. Inward it goes, which is where a real one goes, and
        # then it cannot cover anything on the facade whatever the bays either side
        # turn out to be.
        leaf = hole_wide * 0.85
        size = (leaf, 0.07, hole_tall) if along == "x" else (0.07, leaf, hole_tall)
        off = (hole_wide + leaf) * 0.5 + 0.03
        inward = -facing * (WALL * 0.5 + 0.06)
        centre = (
            (at[0] - off, at[1] + inward, floor + hole_tall * 0.5)
            if along == "x"
            else (at[0] + inward, at[1] - off, floor + hole_tall * 0.5)
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


def _dress_window(parts, along, at, wide, tall, mid, sill_z, facing=-1.0, room=None):
    """Frame, sill, mullion and shutters. See the note where it is called.

    `room` is how much wall there is either side of this bay. A shutter is hung
    outside the frame, and on the last bay of a wall that put it PAST THE CORNER -
    the cottage had one standing 26 cm off the end of the house, in mid air under
    the eave. Where there is not room for a full leaf it is narrowed to fit, which
    reads as a shutter against a corner rather than as a mistake.
    """
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
    #
    # A PAIR, and the same size. Narrowing only the one that would not fit gives a
    # window with a wide shutter on one side and a thin one on the other, which reads
    # as a mistake rather than as a shutter against a corner - and on the cottage
    # that is both its front windows, so the whole village wears it.
    clear = wide * 0.5 + edge
    leaf = wide * 0.48
    if room is not None:
        leaf = min(leaf, min(room) - clear)
    if leaf >= 0.12:
        for side in (-1.0, 1.0):
            place = _across(along, (at[0], at[1], mid), side * (clear + leaf * 0.5))
            parts.append(box(_slab(along, leaf, 0.05, tall * 0.96), _out(along, place, out + 0.02, facing), "shutter"))


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
    storey = int(round(floor / STOREY))

    for face, span in (("x", wide), ("y", deep)):
        for side in (-1.0, 1.0):
            base = (0.0, side * (deep * 0.5 - out * 0.5), 0.0) if face == "x" else (side * (wide * 0.5 - out * 0.5), 0.0, 0.0)

            # Where this wall's doorways are, as (from, to) along the wall.
            bays = openings.get((face, side, storey), [])
            gaps = []
            for (middle, width), kind in zip(bay_places(span, bays), bays):
                if kind != "door":
                    continue
                half = hole_in(kind, width) * 0.5 + 0.12
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


def porch(parts, deep, floor, colour="timber", roof_colour="roof2", at=0.0):
    """A little roof on two posts over the door. A medium shape, and a welcome.

    `at` is where the doorway is, and every measurement here is taken off `DOOR_WIDE`
    rather than written out. Both used to be assumed: the posts stood at 0.95 either
    side of nought while the doorway they framed was at +0.75, so the porch covered
    plaster and the door stood out in the rain beside it.
    """
    reach = 1.05
    top = DOOR_TALL + 0.55
    stand = DOOR_WIDE * 0.5 + 0.2
    for side in (-1.0, 1.0):
        parts.append(box((0.12, 0.12, top - 0.1), (at + side * stand, -deep * 0.5 - reach + 0.1, floor + (top - 0.1) * 0.5), colour))
    parts.append(wedge(stand * 2.0 + 0.7, reach + 0.35, 0.5, (at, -deep * 0.5 - reach * 0.5 + 0.1, floor + top), roof_colour, ridge="x"))
    parts.append(box((stand * 2.0 + 0.8, 0.1, 0.14), (at, -deep * 0.5 - reach + 0.05, floor + top), colour))


def flowerbox(parts, along, at, wide, sill_z, facing=-1.0):
    """A box of flowers under a window. Pure charm, and it costs eight boxes."""
    out = WALL * 0.5 + 0.16
    parts.append(box(_slab(along, wide * 0.9, 0.24, 0.2), _out(along, (at[0], at[1], sill_z - 0.16), out, facing), "timber"))
    for index in range(3):
        over = (index - 1) * wide * 0.26
        place = _across(along, (at[0], at[1], sill_z - 0.03), over)
        parts.append(box(_slab(along, 0.16, 0.16, 0.14), _out(along, place, out, facing), "leafy"))
        parts.append(box(_slab(along, 0.1, 0.1, 0.08), _out(along, (place[0], place[1], place[2] + 0.1), out, facing), "flower"))


def shell(parts, wide, deep, storeys, colour, doors, windows, openings=None, back=None):
    """The four walls of a building, split into bays, with the openings placed.

    `doors` is which side the doorway is on - "south" always, because a building
    faces its street and the game turns the whole building to face the road.

    `back` lets a figure say what its ground-floor NORTH wall is made of, bay by bay.
    A wall's openings are supposed to come from what is behind them, and the default
    rule cannot know: it put a window straight behind the cottage's fireplace, which
    from inside is a hole in a chimney breast and from outside is a window with a
    wall of stone in it. See `cottage_plan`.
    """
    for storey in range(storeys):
        floor = storey * STOREY
        ground = storey == 0

        south = _bays(wide, windows, door=ground and doors)
        north = back if (back and ground) else _bays(wide, windows, door=False)
        sides = _bays(deep, windows, door=False)

        # Each wall says which way it FACES. The south wall and the west flank look
        # down the negative axis and the other two look up it; without that, every
        # dressing on half the building is built inside the room.
        # PER STOREY, because a building has more than one.
        #
        # These were keyed on the wall alone, so on a two-storey house the loop
        # wrote the ground floor's bays and then overwrote them with the first
        # floor's - and the first floor has no door in it. `framing` then framed the
        # ground floor believing there was no doorway and stood a stud in the middle
        # of it: a timber post through the townhouse's front door, at the exact
        # height a warden walks, for as long as the townhouse has had two storeys.
        #
        # `framing`'s own docstring describes fixing this fault. It was fixed for
        # buildings with one storey.
        if openings is not None:
            openings[("x", -1.0, storey)] = south
            openings[("x", 1.0, storey)] = north
            openings[("y", -1.0, storey)] = sides
            openings[("y", 1.0, storey)] = sides
        wall_run(parts, "x", (0.0, -deep * 0.5 + WALL * 0.5, 0.0), wide, STOREY, colour, south, floor, -1.0)
        wall_run(parts, "x", (0.0, deep * 0.5 - WALL * 0.5, 0.0), wide, STOREY, colour, north, floor, 1.0)
        wall_run(parts, "y", (-wide * 0.5 + WALL * 0.5, 0.0, 0.0), deep, STOREY, colour, sides, floor, -1.0)
        wall_run(parts, "y", (wide * 0.5 - WALL * 0.5, 0.0, 0.0), deep, STOREY, colour, sides, floor, 1.0)


# ------------------------------------------------------ where the bays actually are

# How much wall is left either side of an opening, and how narrow a bay may be
# squeezed to. A bay has to stay wide enough to carry a window with its reveals,
# which is what stops a doorway borrowing until its neighbours cannot hold glass.
JAMB = 0.18
REVEAL = 0.15
LEAST_BAY = WINDOW_WIDE + REVEAL * 2.0
DOOR_BAY = DOOR_WIDE + JAMB * 2.0


def bay_places(length, bays):
    """Where each bay sits along its wall and how wide it is, as (middle, width).

    # One description of a wall's grid, because there were three

    `wall_run` builds the bays, `framing` has to know where the holes in them are so
    it does not put a stud through a doorway, and the figures have to know where the
    door is so the porch and the steps land on it. All three worked the grid out for
    themselves from the same two lines of arithmetic, and three copies of a formula
    is one formula and two bugs waiting for somebody to edit the first.

    # A doorway is a fixed size and a bay is not

    The clear opening was `min(DOOR_WIDE, bay - 0.3)`, so on the 9 m front of a
    cottage - six bays of 1.5 m - the 1.9 m doorway this project believes it has was
    quietly built at 1.195 m. Every door in the game was. Nothing said so: the
    constant still read 1.9, and the research that reviewed the metrics read the
    constant rather than the mesh and passed it.

    So a door bay takes the width a door needs and the rest of the wall gives it up
    between them. Measured off the built mesh by `measure_the_cottage`, not argued.
    """
    count = max(1, len(bays))
    widths = [length / count] * count
    for index, kind in enumerate(bays):
        if kind != "door" or widths[index] >= DOOR_BAY:
            continue
        others = [n for n in range(count) if n != index]
        spare = sum(max(0.0, widths[n] - LEAST_BAY) for n in others)
        borrow = min(DOOR_BAY - widths[index], spare)
        if borrow <= 0.0:
            continue
        # Equally from every other bay, so the rhythm either side stays even. They
        # are all the same width when this runs, so an equal share can never push
        # one below `LEAST_BAY` once the total has been capped at what is spare.
        for n in others:
            widths[n] -= borrow / len(others)
        widths[index] += borrow

    places = []
    run = -length * 0.5
    for width in widths:
        places.append((run + width * 0.5, width))
        run += width
    return places


def hole_in(kind, width):
    """The clear opening a bay of this width gets, reveals taken off."""
    want = DOOR_WIDE if kind == "door" else WINDOW_WIDE
    return min(want, width - REVEAL * 2.0)


def _bays(length, windows, door):
    """How many bays a wall of this length gets, and what each one is.

    A bay is about a module wide. Fewer than three and a facade has no rhythm; more
    than seven and the windows read as a factory.

    # A wall with a door in it gets an ODD number of bays

    The door goes in the middle bay, and the middle bay of six is not the middle of
    the wall - it is three quarters of a metre off it. So the doorway stood at +0.75
    while the porch over it, the steps up to it and the gap the GAME leaves in the
    wall for the player to walk through were all centred on nought. You could walk
    through the plaster beside the door, and a quarter of the real doorway was solid
    to the player.

    An odd rhythm is what a facade with a central entrance has anyway - three, five,
    seven bays - so this is the split grammar agreeing with the architecture rather
    than a correction bolted onto it.
    """
    count = max(1, min(7, int(round(length / MODULE))))
    if door and count % 2 == 0:
        count = max(1, count - 1)
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


# How wide and deep a fireplace is. The chimney is sized off the same numbers, so
# a stack can never again stand somewhere its own fire is not.
FIRE_WIDE = 1.5
FIRE_DEEP = 0.5


def fireside(wide, deep, left=True):
    """Where a fire stands against the back wall, and therefore where its stack does.

    # Two expressions for one fact, in two buildings

    The cottage worked its fireplace out as `-wide * 0.25` and its chimney as
    `-wide * 0.5 + 0.55`, and the stack came down 2.5 m from the fire. The townhouse
    was worse: fire at `-wide * 0.25`, stack at `(wide * 0.5 - 0.6, -deep * 0.15)`,
    which put the flue at the OPPOSITE CORNER of the house - six metres across and
    five back, over the front room, venting a fireplace that was not there.

    Neither is a hard bug to see once the two lines are next to each other, and that
    is the point: they never were. So there is one expression now and both are told
    it, which is the same fix as `bay_places` and for the same reason.

    A fire is set in from the corner by rather less than its own width, so the
    chimney breast has wall either side of it to be a breast against.
    """
    side = -1.0 if left else 1.0
    return (side * (wide * 0.5 - WALL - FIRE_WIDE * 0.8),
            deep * 0.5 - WALL - FIRE_DEEP * 0.5)


def blind_behind(wide, bays, fire_x):
    """Makes solid any bay a fireplace stands against.

    The bay rule alternates windows along a wall and cannot know what is behind them.
    It cut one straight through the cottage's chimney breast: from the street a window
    with a wall of stone in it, from inside a hole in the fireplace.
    """
    for index, (middle, width) in enumerate(bay_places(wide, bays)):
        against = (middle - width * 0.5 < fire_x + FIRE_WIDE * 0.5
                   and middle + width * 0.5 > fire_x - FIRE_WIDE * 0.5)
        if bays[index] == "window" and against:
            bays[index] = "solid"
    return bays


def hearth(parts, at):
    """A fireplace where the plan puts it.

    # The stack that stood two and a half metres from its own fire

    This used to work its position out from the building's size - `-wide * 0.25` -
    and `chimney` was called with a different expression, `-wide * 0.5 + 0.55`. Both
    were plausible, neither was wrong on its own, and nothing compared them. So every
    cottage in the world had a chimney 1.7 m along and 1.8 m back from the fireplace
    it was supposed to carry: a flue through the middle of the roof over an empty
    corner of the room, and a fire venting into the ceiling.

    Two derivations of one fact is the same fault as two copies of a formula. There
    is one now, in `cottage_plan`, and both are told it.
    """
    x, y = at
    parts.append(box((FIRE_WIDE, FIRE_DEEP, 1.3), (x, y, 0.65), "hearth"))
    parts.append(box((0.9, 0.3, 0.8), (x, y - 0.12, 0.4), "inbeam"))


# What a bed takes up on the floor. The plan keeps it clear of the way in.
BED_WIDE = 1.3
BED_DEEP = 2.0


def bed(parts, at):
    """A bed where the plan puts it, which is in the alcove and off the route in."""
    x, y = at
    parts.append(box((BED_WIDE, BED_DEEP, 0.35), (x, y, 0.35), "inbeam"))
    parts.append(box((BED_WIDE - 0.1, BED_DEEP - 0.3, 0.18), (x, y, 0.60), "cloth"))


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


def doorstep(parts, deep, rises: float, wide: float = 2.4, at: float = 0.0):
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
                (at, -deep * 0.5 - out * 0.5, rise * (step + 0.5)),
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


def shut_the_door(parts, wide, deep, floor=0.0, at=0.0):
    """Fills the doorway with a door that is closed, and frames it.

    The leaf is `DOOR_WIDE` less a gap, and until the bay grid was fixed the opening
    it filled was 1.195 m - so every shut door in the game was a leaf 64 cm wider
    than its own hole, sunk into the plaster either side of it.
    """
    leaf = DOOR_WIDE - 0.06
    parts.append(box((leaf, 0.09, DOOR_TALL - 0.04),
                     (at, -deep * 0.5 - 0.02, floor + (DOOR_TALL - 0.04) * 0.5), "door"))
    # A handle, so it reads as shut rather than as boarded up.
    parts.append(box((0.12, 0.12, 0.12),
                     (at + leaf * 0.32, -deep * 0.5 - 0.12, floor + DOOR_TALL * 0.45), "brass"))
    # And the room behind it goes dark, so nothing shows through the windows.
    parts.append(box((wide - WALL * 2.2, deep - WALL * 2.2, 0.12),
                     (0.0, 0.0, floor + 0.06), "board"))


# =========================================================== THE COTTAGE, PLANNED
#
# # A room with props in it is not a floor plan
#
# Every interior in this file is one open volume per storey with furniture standing
# in it - `room` says so in its own docstring - and that was honest as far as it
# went. What it cannot do is make a building read as somewhere people LIVE, because
# what does that is the relationships between the parts: the fire you can see from
# the door, the bed that is not in the way, the window that lights the chair.
#
# Nothing held those relationships, so nothing could be wrong about them and nothing
# could be checked. The chimney missed the fire by two and a half metres for the life
# of the project. A window was cut into the wall behind the fireplace. The porch
# covered plaster while the door stood beside it in the rain. Each of those is one
# expression disagreeing with another expression, and no amount of looking at the
# outside of the house finds any of them.
#
# So the cottage gets a PLAN, and the plan is the only thing that says where anything
# goes. Everything below is told; nothing works it out again.
#
# This is deliberately the only building that has one yet. The research is explicit
# that exterior variety can be broad while interior grammar should be narrow and
# learnable - Embark shipped thirty individually believable buildings on THE FINALS
# and found them collectively confusing - so one family is proven, with checks, before
# a second is written. It is equally explicit about what NOT to do: "create three tiny
# rooms just to claim a floor plan". A cottage is a common room with a place to sleep
# off it. That is the whole plan and it is enough.

# The metrics, in one place.
#
# Scattered as magic numbers they drift and contradict each other, which is what this
# file has just spent a day paying for. Real accessibility minima are a reality anchor
# and not a game metric: a third-person camera wants roughly one and a half times life
# size, which is why the doorway is `DOOR_WIDE` and not a realistic 0.9 m.
COTTAGE = {
    "wide": MODULE * 6,             # 9.0 m
    "deep": MODULE * 5,             # 7.5 m
    "route": DOOR_WIDE,             # the way in, kept clear, as wide as the door
    "apron": 1.2,                   # standing room in front of the fire
    "alcove_deep": 2.6,             # how far the sleeping alcove reaches in
    "alcove_front": 3.4,            # how far back it starts, which is its own opening
}


def _rect(at, size):
    """A footprint as (x0, y0, x1, y1), which is what every check below wants."""
    return (at[0] - size[0] * 0.5, at[1] - size[1] * 0.5,
            at[0] + size[0] * 0.5, at[1] + size[1] * 0.5)


def cottage_plan(hearth_left=True):
    """Where everything in a cottage goes, worked out once, before any of it is built.

    # Circulation first, and the rooms put against it

    The order is the one the research gives and it is not the obvious one: the way in
    is reserved BEFORE any room is assigned, and everything else is placed against it.
    Generating rooms first and threading a route through what is left over is how a
    building ends up with a bed in its hallway.

    `hearth_left` is the variation axis, and it is a CAUSE rather than a decoration:
    it moves the fire, the stack, the solid bay behind the fire, the partition, the
    alcove, its window, the bed and the table together, so the two cottages differ in
    a way that stays coherent instead of in nine independent rolls that can contradict
    each other.
    """
    wide, deep = COTTAGE["wide"], COTTAGE["deep"]
    hx, hy = wide * 0.5 - WALL, deep * 0.5 - WALL
    side = -1.0 if hearth_left else 1.0
    TABLE = (1.6, 1.0)

    # 1. THE WAY IN. Which bay the front door lands in decides everything else, so it
    #    is asked for rather than assumed - see `bay_places`.
    front = _bays(wide, True, door=True)
    places = bay_places(wide, front)
    door = next(m for (m, _), kind in zip(places, front) if kind == "door")
    route = (door - COTTAGE["route"] * 0.5, -hy, door + COTTAGE["route"] * 0.5, hy)

    # 2. THE FIRE, against the back wall on its own side, and the stack ON it.
    fire = fireside(wide, deep, hearth_left)
    # Standing room in front of it, which is a room's second anchor after the door
    # and the reason nothing is allowed to be put there.
    apron = (fire[0] - FIRE_WIDE * 0.5, fire[1] - FIRE_DEEP * 0.5 - COTTAGE["apron"],
             fire[0] + FIRE_WIDE * 0.5, fire[1] - FIRE_DEEP * 0.5)

    # 3. THE BACK WALL, which is asked what is behind it.
    #
    #    The default rule alternates windows along a wall and cannot know. It cut one
    #    straight behind the fireplace: from the street a window with a wall of stone
    #    in it, from inside a hole in the chimney breast. A bay with a fire against it
    #    is solid.
    back = blind_behind(wide, _bays(wide, True, door=False), fire[0])

    # 4. THE ALCOVE, in the far back corner from the fire, behind one wall.
    #
    #    One wall, not two, and no door in it. An alcove is a place you can see into
    #    from the room it belongs to; give it a doorway and it becomes a second room,
    #    and a cottage with two rooms in it has neither.
    inner = (-hx, -hy, hx, hy)
    across = -side * (hx - COTTAGE["alcove_deep"])
    back_of = -hy + COTTAGE["alcove_front"]
    alcove = (across, back_of, hx, hy) if side < 0 else (-hx, back_of, across, hy)

    # 5. THE BED, at the back of the alcove - clear of the route by where it is, not
    #    by luck.
    lying = ((across + (hx if side < 0 else -hx)) * 0.5, hy - BED_DEEP * 0.65)
    sitting = (fire[0] - side * (FIRE_WIDE * 0.5 - 0.25), -0.5)

    # 6. WHAT THE WINDOWS LIGHT. The front wall is all common room - that is what
    #    putting the alcove at the BACK buys - and the alcove gets its own from the
    #    back wall, so nobody sleeps in a cupboard.
    front_windows = [(m, -deep * 0.5) for (m, _), kind in zip(places, front) if kind == "window"]
    lit_alcove = [
        (m, deep * 0.5)
        for (m, _), kind in zip(bay_places(wide, back), back)
        if kind == "window" and alcove[0] < m < alcove[2]
    ]

    return {
        "wide": wide, "deep": deep, "inner": inner, "table_size": TABLE,
        "door": door, "route": route,
        "front": front, "back": back,
        "hearth": fire, "chimney": fire, "apron": apron,
        "partition": (across, back_of, hy), "alcove": alcove,
        "bed": lying, "bed_rect": _rect(lying, (BED_WIDE, BED_DEEP)),
        # The table is furniture rather than structure, so it goes last and it goes
        # where the protected things are not: beside the fire, out of the route.
        "table": sitting, "table_rect": _rect(sitting, TABLE),
        "windows": front_windows,
        "alcove_windows": lit_alcove,
        # No rear door in this slice. One would need a matching gap in the game's own
        # wall - see `Plot::walls` - and a yard proven reachable behind it, which is
        # its own piece of work. The back is made to READ as a working back instead:
        # no porch, no flower boxes, a blind bay where the chimney breast is.
        "rear": None,
    }


def partition(parts, plan):
    """The one wall inside a cottage, and the way round it."""
    across, back_of, hy = plan["partition"]
    run = hy - back_of
    tall = STOREY - 0.1
    parts.append(box((WALL, run, tall), (across, back_of + run * 0.5, tall * 0.5), "inwall"))
    # A post at its open end, so the wall stops deliberately rather than just ending.
    parts.append(box((0.22, 0.22, tall), (across, back_of, tall * 0.5), "inbeam"))


def cottage(open_door=True, hearth_left=True):
    """A common room with a fire, and a place to sleep off it, under a steep thatch.

    Built from `cottage_plan`. Nothing here decides where anything goes; it asks.

    `open_door` false builds the same cottage with its door shut and no interior -
    see `shut_the_door`. Parameterised rather than copied, so the two can never
    drift into being different cottages.
    """
    plan = cottage_plan(hearth_left)
    wide, deep = plan["wide"], plan["deep"]
    parts = []
    parts.append(box((wide + 0.34, deep + 0.34, PLINTH), (0.0, 0.0, PLINTH * 0.5), "stone"))
    courses(parts, wide + 0.34, deep + 0.34, 0.0, PLINTH)
    doorstep(parts, deep, PLINTH, at=plan["door"])
    holes = {}

    shell(parts, wide, deep, 1, "plaster", doors=True, windows=True, openings=holes,
          back=plan["back"])
    if open_door:
        room(parts, wide, deep, 1)
        partition(parts, plan)
        hearth(parts, plan["hearth"])
        bed(parts, plan["bed"])
        table(parts, plan["table"], *plan["table_size"])
    else:
        shut_the_door(parts, wide, deep, at=plan["door"])
    framing(parts, wide, deep, 0.0, STOREY, openings=holes)
    porch(parts, deep, 0.0, roof_colour="thatch", at=plan["door"])
    # Ridge across the front, so the cottage shows its long eaves to the street and
    # its gable to its neighbour - the opposite of the shop, which is what stops a
    # row of them reading as a terrace.
    top = roofed(parts, wide, deep, STOREY, "thatch", ridge="x", pitch=0.62)
    chimney(parts, plan["chimney"], STOREY - 0.4, top + 0.5)
    # Flower boxes under the front windows - under the ones there ARE, rather than
    # under where a second expression guessed they would be. They were at 31% of the
    # width and the windows are at 41%, so every planter in the village hung on blank
    # plaster - half a metre from one window and a metre from the other.
    for at in plan["windows"]:
        flowerbox(parts, "x", (at[0], -deep * 0.5 + WALL * 0.5, 0.0), WINDOW_WIDE, WINDOW_SILL)
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

    # Where the fire is, and therefore where the stack is and which bay behind it
    # goes blind. Worked out before the door is opened, because a house with its
    # door shut still has a chimney and it still has to stand over something.
    fire = fireside(wide, deep)
    shell(parts, wide, deep, 2, "plaster", doors=True, windows=True, openings=holes,
          back=blind_behind(wide, _bays(wide, True, door=False), fire[0]))
    if open_door:
        room(parts, wide, deep, 2)
        stairs(parts, wide, deep, 2)
        hearth(parts, fire)
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
    chimney(parts, fire, STOREY * 2 - 0.4, top + 0.6)

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

    # ------------------------------------------------------------- THE CAMPANILE
    #
    # # A landmark has to WIN the skyline, not merely be on it
    #
    # This tower used to top out at 13 m. Measured against what stands round it in a
    # city - blocks at 19.7 m, towers at 37.6 and a spire at 57.1 - the guild's own
    # hall was the shortest thing on the street, and a photograph from the city
    # entrance showed a row of near-identical slabs with nothing saying which way to
    # walk. A landmark that loses to the background is not a landmark.
    #
    # So it goes to 74 m, clear of the tallest tower by a sixth of its height. Height
    # alone is not enough either - a taller slab is still a slab - so the shape is
    # deliberately the one thing in a city of extruded rectangles that TAPERS: a
    # square shaft in setback stages, an open belfry, an octagonal lantern, and a
    # spire to a point. Read as a silhouette against the sky, nothing else here is
    # remotely that shape.
    # PROPORTION, not just height. At MODULE * 3 square and 80 m this was 1:18 -
    # photographed from the city entrance it read as a radio mast with a city under
    # it, not as the guild's hall. A campanile runs about 1:10, so the shaft gets
    # wider rather than shorter and the building at its foot has something to belong
    # to.
    tower = MODULE * 5
    tx, ty = wide * 0.5 - tower * 0.5, deep * 0.5 - tower * 0.5

    # The shaft, in stages. Each steps in a little and wears a string course, which
    # is what stops sixty metres of masonry reading as an extruded box.
    stages = 5
    stage_tall = 10.0
    at = 0.0
    for stage in range(stages):
        span = tower * (1.0 - stage * 0.045)
        parts.append(box((span, span, stage_tall), (tx, ty, at + stage_tall * 0.5), "stone"))
        # A string course on top of every stage but the last, which the belfry caps.
        parts.append(
            box((span + 0.34, span + 0.34, 0.3), (tx, ty, at + stage_tall - 0.15), "stone2")
        )
        # A tall slit down each face, so the stage has a scale you can read from the
        # ground - without them the shaft is a plain column and reads as nearer and
        # shorter than it is.
        if stage > 0:
            for face, (ox, oy) in enumerate(
                ((0.0, -1.0), (0.0, 1.0), (-1.0, 0.0), (1.0, 0.0))
            ):
                parts.append(
                    box(
                        (0.7 if face < 2 else 0.1, 0.1 if face < 2 else 0.7, 4.2),
                        (
                            tx + ox * (span * 0.5 + 0.02),
                            ty + oy * (span * 0.5 + 0.02),
                            at + stage_tall * 0.52,
                        ),
                        "glass",
                    )
                )
        at += stage_tall

    # THE BELFRY: an open stage, which is the moment the tower stops being solid.
    belfry = tower * 0.94
    belfry_tall = 8.4
    for corner_x in (-1.0, 1.0):
        for corner_y in (-1.0, 1.0):
            parts.append(
                box(
                    (belfry * 0.24, belfry * 0.24, belfry_tall),
                    (
                        tx + corner_x * (belfry * 0.38),
                        ty + corner_y * (belfry * 0.38),
                        at + belfry_tall * 0.5,
                    ),
                    "stone",
                )
            )
    # The guild's own colour, deep inside the belfry where it reads as a lit window
    # rather than as paint on a wall.
    parts.append(
        box((belfry * 0.52, belfry * 0.52, belfry_tall * 0.8), (tx, ty, at + belfry_tall * 0.5), "guild")
    )
    parts.append(box((belfry + 0.5, belfry + 0.5, 0.42), (tx, ty, at + belfry_tall + 0.21), "stone2"))
    at += belfry_tall + 0.42

    # THE LANTERN: two squares at forty-five degrees, which reads as an octagon from
    # every side and costs eight boxes rather than a lathe.
    lantern = tower * 0.66
    for turn in (0.0, math.pi * 0.25):
        parts.append(
            box(
                (lantern, lantern, 6.2),
                (tx, ty, at + 3.1),
                "stone",
                tilt=(0.0, 0.0, turn),
            )
        )
    for turn in (0.0, math.pi * 0.25):
        parts.append(
            box(
                (lantern + 0.4, lantern + 0.4, 0.3),
                (tx, ty, at + 6.35),
                "stone2",
                tilt=(0.0, 0.0, turn),
            )
        )
    at += 6.5

    # THE SPIRE. Four faces to a point, which is the shape a guild puts on a map.
    spire = 9.4
    for ridge in ("x", "y"):
        parts.append(wedge(lantern + 0.5, lantern + 0.5, spire, (tx, ty, at), "guild", ridge=ridge))
    at += spire
    # And a beacon on the very top, so it is still a landmark at dusk.
    parts.append(tube(0.16, 1.6, (tx, ty, at + 0.8), "brass", sides=8))
    parts.append(box((0.75, 0.75, 0.75), (tx, ty, at + 1.9), "brass"))
    tower_top = at + 2.3

    # The guild's colours over the door, and a banner either side of it.
    parts.append(box((2.4, 0.12, 1.1), (0.0, -deep * 0.5 - 0.12, 3.9), "guild"))
    parts.append(
        tube(0.4, 0.14, (0.0, -deep * 0.5 - 0.22, 3.9), "brass", sides=18, tilt=(math.pi * 0.5, 0.0, 0.0))
    )
    for side in (-1.0, 1.0):
        parts.append(box((0.9, 0.07, 2.6), (side * 2.6, -deep * 0.5 - 0.1, 4.3), "guild"))
    return parts, max(top, tower_top)


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
                #
                # RECESSED, and that is not decoration. The research's rule for where
                # ink belongs puts a window frame in the secondary weight only "when
                # its geometry creates a true depth or normal break" - a coloured
                # rectangle flush with the wall is a coplanar palette change and gets
                # none. So the glass sits back behind the frame that surrounds it.
                band = FLOOR_TALL * 0.66
                sunk = _out(face, (at[0], at[1], z + FLOOR_TALL * 0.67), 0.12, -side)
                parts.append(box(_slab(face, span * 0.94, 0.14, band), sunk, glass))
                parts.append(box(
                    _slab(face, span, 0.20, FLOOR_TALL * 0.34),
                    (at[0], at[1], z + FLOOR_TALL * 0.17), "concrete"))

                # THE FRAME, IN INK, AND THE MULLIONS THAT MAKE IT SQUARES.
                #
                # An inverted hull draws silhouettes, so it can never draw a line
                # round a window - which is why a tower's glazing had no edge on it
                # while every other thing in the world did. The research's answer is
                # an authored interior line: geometry, in the ink colour, put where
                # the line belongs.
                #
                # Spaced by the band's own HEIGHT rather than "every couple of
                # metres", so what a division makes is a SQUARE. A row of long thin
                # rectangles reads as a stripe with bars over it; squares read as
                # windows, which is what they are.
                rail = 0.13
                proud = 0.03
                for up in (-1.0, 1.0):
                    parts.append(box(
                        _slab(face, span * 0.96, 0.16, rail),
                        _out(face, (at[0], at[1], z + FLOOR_TALL * 0.67 + up * band * 0.5), proud, side),
                        "ink"))
                lights = max(2, round(span * 0.94 / band))
                for index in range(lights + 1):
                    over = -span * 0.47 + span * 0.94 * index / lights
                    place = _across(face, (at[0], at[1], z + FLOOR_TALL * 0.67), over)
                    parts.append(box(
                        _slab(face, rail, 0.16, band + rail),
                        _out(face, place, proud, side),
                        "ink"))


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


# ------------------------------------------------------- the plan, against the mesh


def doorways_in(solids):
    """Every doorway in a building's front wall, as (middle, clear width).

    # The wall is found, not assumed

    The first version of this was told the building's width, and worked it out from
    the widest thing standing on the ground - which for the shop is an awning post
    outside the wall. So it walked two metres past the corner of the building, found
    open air there, and reported a second doorway. A measurement that guesses its own
    ruler is worse than no measurement: it fails in a way that looks like a finding.

    So the wall says where it is. Pieces standing on the ground and reaching above
    head height are grouped by the plane they share, and the front is the group
    nearest the street, which also gives its two ends. Not FULL height: a city
    block's lobby piers stop at 2.85 m under a five-metre storey, and asking for
    full-height slabs found the building's own core instead and called the whole
    ground floor a ten-metre doorway.

    # What tells a doorway from a window

    Both are gaps. A doorway is open from the FLOOR to above head height and a window
    is not - it has a sill under it and a lintel over it - so the wall is sampled at
    three heights and only a gap open at all three counts. Sampling at one height
    finds shop windows and calls them doors.

    Only pieces as thick as the wall count as wall. An open door LEAF stands against
    the plaster beside its own opening and is 7 cm thick, and counting it would have
    the door narrowing the doorway.
    """
    slabs = [b for b in solids if b[4] < 0.15 and b[5] > 2.0
             and b[3] - b[2] > WALL * 0.9]
    assert slabs, "nothing in this figure looks like a wall standing on the ground"
    planes = {}
    for slab in slabs:
        planes.setdefault(round((slab[2] + slab[3]) * 0.5, 1), []).append(slab)
    front = min(planes)
    doorways_in.front = front
    low = min(b[0] for b in planes[front])
    high = max(b[1] for b in planes[front])

    wall = [b for b in solids
            if b[3] - b[2] > WALL * 0.9 and b[5] > 0.1
            and b[2] < front + 0.14 and b[3] > front - 0.14]
    ways, opened, along = [], None, low
    while along <= high + 1e-9:
        open_here = all(
            not any(x0 <= along <= x1 and z0 <= up <= z1 for x0, x1, _, _, z0, z1 in wall)
            for up in (0.3, 1.6, 2.2)
        )
        if open_here and opened is None:
            opened = along
        if not open_here and opened is not None:
            ways.append(((opened + along) * 0.5, along - opened))
            opened = None
        along += 0.005
    if opened is not None:
        ways.append(((opened + high) * 0.5, high - opened))
    return [(middle, clear) for middle, clear in ways if clear > 0.6]


# How far proud of the wall's own face a lit pane sits, in metres.
#
# The glass is built in the MIDDLE of the wall, so a pane at the glass's own place
# would be buried in plaster. Half the wall clears its face and the rest is the
# margin that stops the two z-fighting.
PANE_PROUD = 0.03


def windows_in(parts):
    """Every window in a figure, measured off the glass it actually built.

    # Why the game cannot work these out for itself

    It was doing exactly that. `light_the_windows` placed its lit panes from the
    building's LOT FOOTPRINT - two on the front at 24 % of the width, one halfway
    down each flank - and the lot is not the building: it is what the building keeps
    clear on the ground, and it is bigger. So the panes stood beside the windows
    rather than in them, and the flank ones floated out in the air where the wall
    is not.

    That is the same fault as the flower boxes that hung at 31 % of the width while
    the windows were at 41 %, and the same fault as the chimney that stood two and a
    half metres from its fire: one fact, worked out twice, in two places that never
    met. It was invisible until somebody looked at a village after dark.

    So the windows are MEASURED here, off the glass, and the game is told. A window
    is a `glass` box; its thin axis is the wall it sits in and the way it faces.

    Returned in the GAME's frame, not Blender's, because that is what the game will
    place children in and one conversion in one place is the whole point. Blender
    (x, y, z) arrives as (x, z, -y) - the same turn `DOOR_ON_BLENDER_Y` describes.
    """
    found = []
    for obj, colour in parts:
        if colour != "glass":
            continue
        points = [obj.matrix_world @ v.co for v in obj.data.vertices]
        low = (min(p.x for p in points), min(p.y for p in points), min(p.z for p in points))
        high = (max(p.x for p in points), max(p.y for p in points), max(p.z for p in points))
        size = [high[axis] - low[axis] for axis in range(3)]
        at = [(low[axis] + high[axis]) * 0.5 for axis in range(3)]

        # The thin axis is the one through the wall, so it says which wall this is.
        across = 0 if size[0] < size[1] else 1
        # And out of it, away from the middle of the building.
        way = 1.0 if at[across] > 0.0 else -1.0
        at[across] += way * (WALL * 0.5 + PANE_PROUD)

        # Which storey it lights, from the height of its sill.
        storey = int(max(0.0, low[2]) // STOREY)

        found.append(
            (
                storey,
                (at[0], at[2], -at[1]),
                (size[0], size[2], size[1]),
            )
        )
    found.sort()
    return found


def floor_of(parts, front):
    """How high the floor inside is, and how far the step to it reaches out.

    # The game had no idea it was above the ground

    A building's floor is laid on top of its plinth, and the ground it stands on is
    the HIGHEST of its four corners - so on any slope the floor is well clear of the
    earth beside it. The warden stood at terrain height regardless and sank into the
    boards, which on a hillside is most of a shin.

    The step out front is what makes that walkable: three shallow treads from the
    ground to the threshold. So it is measured too - the game needs the ramp as much
    as the height, or entering a house becomes a hop.

    `front` is where the front wall's own plane is, which `doorways_in` has already
    had to find.
    """
    # The floor STANDING ON THE GROUND, not the one over it. `room` lays one per
    # storey, and taking the highest reported a townhouse's first floor at 3.7 m -
    # which as a walking surface would have put the warden on the roof.
    inside = [
        b
        for obj, colour in parts
        if colour == "infloor"
        for b in [_extent(obj)]
        if b[4] < 0.2
    ]
    top = max((b[5] for b in inside), default=0.0)

    # The treads: stone, below the threshold, and standing out past the front wall.
    face = front - WALL * 0.5
    treads = [
        b
        for obj, colour in parts
        if colour == "stone"
        for b in [_extent(obj)]
        if b[2] < face - 0.02 and b[5] < top + PLINTH + 0.05
    ]
    reach = max((face - b[2] for b in treads), default=0.0)
    wide = max((b[1] - b[0] for b in treads), default=0.0)
    return top, reach, wide


def _extent(obj):
    """One object's box, in the frame it was built in."""
    points = [obj.matrix_world @ v.co for v in obj.data.vertices]
    return (
        min(p.x for p in points), max(p.x for p in points),
        min(p.y for p in points), max(p.y for p in points),
        min(p.z for p in points), max(p.z for p in points),
    )


def every_doorway():
    """Measures the front door of every figure that has one.

    # The fix was to the grammar, so the check has to be too

    The 1.195 m off-centre doorway was not a cottage bug. It was `_bays` putting the
    door in the middle bay of an even number of bays, so EVERY building in the game
    had it, and a check that only looked at the cottage would have gone green while a
    guild hall still refused the player at its own front door.
    """
    found = []
    for name in ("cottage", "townhouse", "shop", "guild_hall",
                 "city_block", "city_tower", "city_spire"):
        masonry.fresh()
        parts, _ = FIGURES[name]()
        solids = []
        for obj, _ in parts:
            points = [obj.matrix_world @ v.co for v in obj.data.vertices]
            solids.append((min(p.x for p in points), max(p.x for p in points),
                           min(p.y for p in points), max(p.y for p in points),
                           min(p.z for p in points), max(p.z for p in points)))
        ways = doorways_in(solids)
        assert len(ways) == 1, f"{name} has {len(ways)} doorways in its front: {ways}"
        middle, clear = ways[0]
        assert abs(middle) < 0.05, \
            f"{name}'s doorway is {middle:+.3f} m off the middle of its front wall, and the \
game leaves its collision gap centred - see DOOR_CLEAR in src/world/town.rs"
        assert clear > DOOR_WIDE - 0.4, \
            f"{name}'s doorway is only {clear:.3f} m clear"
        found.append(
            (name, middle, clear, windows_in(parts), floor_of(parts, doorways_in.front))
        )
    return found


def measure_the_cottage(hearth_left=True):
    """Builds a cottage and MEASURES it, then checks the plan against what was built.

    # Validate the ruler before the thing it measures

    A plan the game trusts is worth nothing if the geometry quietly disagrees with
    it, and this file has just produced four faults of exactly that shape - a chimney
    that missed its fire, a window behind a fireplace, a porch beside its door, a
    planter under blank wall. Every one of them was two expressions that were each
    fine and never compared.

    So the numbers written into `town.txt` are not the plan's opinion of the cottage.
    They are taken off the built mesh: the doorway is found by walking across the
    front wall and looking for the gap, the stack and the fire are found by their
    sizes and their footprints are intersected. If the plan and the mesh ever part
    company the build stops here rather than shipping a house that lies.

    `world::town`'s tests then check those measured numbers against the GAME's
    contracts - chiefly that the opening the player can see is inside the gap the
    game leaves in the wall for them to walk through, which for the life of the
    project it was not.
    """
    masonry.fresh()
    plan = cottage_plan(hearth_left)
    parts, _ = cottage(True, hearth_left)
    wide, deep = plan["wide"], plan["deep"]

    def extent(obj):
        points = [obj.matrix_world @ v.co for v in obj.data.vertices]
        return (min(p.x for p in points), max(p.x for p in points),
                min(p.y for p in points), max(p.y for p in points),
                min(p.z for p in points), max(p.z for p in points))

    solids = [extent(obj) for obj, _ in parts]
    ways = doorways_in(solids)
    assert len(ways) == 1, f"the cottage's front wall has {len(ways)} doorways in it: {ways}"
    middle, clear = ways[0]
    assert abs(middle - plan["door"]) < 0.02, \
        f"the doorway was built at {middle:+.3f} and the plan puts it at {plan['door']:+.3f}"
    assert abs(clear - DOOR_WIDE) < 0.05, \
        f"the doorway was built {clear:.3f} m clear and DOOR_WIDE says {DOOR_WIDE}"

    # THE STACK OVER THE FIRE. Both found by their own size, and intersected.
    def like(want, got, slack=0.06):
        return abs(want - got) < slack

    fires = [b for b in solids
             if like(FIRE_WIDE, b[1] - b[0]) and like(FIRE_DEEP, b[3] - b[2]) and b[4] < 0.05]
    stacks = [b for b in solids
              if like(0.75, b[1] - b[0]) and like(0.75, b[3] - b[2]) and b[5] > STOREY]
    assert len(fires) == 1 and len(stacks) == 1, \
        f"found {len(fires)} fireplaces and {len(stacks)} chimney stacks"
    fire, stack = fires[0], stacks[0]
    over = min(fire[1], stack[1]) - max(fire[0], stack[0])
    through = min(fire[3], stack[3]) - max(fire[2], stack[2])
    assert over > 0.3 and through > 0.15, (
        f"the chimney stands at x {stack[0]:+.2f}..{stack[1]:+.2f} y {stack[2]:+.2f}..{stack[3]:+.2f} "
        f"and the fire at x {fire[0]:+.2f}..{fire[1]:+.2f} y {fire[2]:+.2f}..{fire[3]:+.2f} - "
        "a flue has to come down onto its own fire"
    )

    # NOTHING DRESSED ONTO A WINDOW HANGS OFF THE END OF THE HOUSE.
    band = (WINDOW_SILL + 0.3, WINDOW_SILL + WINDOW_TALL - 0.1)
    for x0, x1, y0, y1, z0, z1 in solids:
        if z0 < band[0] or z1 > band[1]:
            continue
        assert x1 <= wide * 0.5 + 0.01 and x0 >= -wide * 0.5 - 0.01, \
            f"window dressing at x {x0:+.3f}..{x1:+.3f} hangs past the corner at {wide * 0.5:+.3f}"
        assert y1 <= deep * 0.5 + 0.01 and y0 >= -deep * 0.5 - 0.01, \
            f"window dressing at y {y0:+.3f}..{y1:+.3f} hangs past the corner at {deep * 0.5:+.3f}"

    return plan, middle, clear


def write_the_plan(note, plan, door, clear):
    """The cottage's plan, in the units the game measures its lots in."""
    note.write(f"DOORWAY cottage {door:.4f} {clear:.4f}\n")
    for name, middle, wide, _, _ in DOORWAYS:
        if name != "cottage":
            note.write(f"DOORWAY {name} {middle:.4f} {wide:.4f}\n")
    # THE FLOOR INSIDE, and the step up to it.
    for name, _, _, _, (top, reach, step_wide) in DOORWAYS:
        note.write(f"FLOOR town_{name} {top:.4f} {reach:.4f} {step_wide:.4f}\n")
    # AND EVERY WINDOW, so the game can light the glass rather than the plaster.
    for name, _, _, windows, _ in DOORWAYS:
        for storey, at, size in windows:
            note.write(
                f"WINDOW town_{name} {storey} "
                f"{at[0]:.4f} {at[1]:.4f} {at[2]:.4f} "
                f"{size[0]:.4f} {size[1]:.4f} {size[2]:.4f}\n"
            )
    for name in ("inner", "route", "alcove", "apron", "bed_rect", "table_rect"):
        x0, y0, x1, y1 = plan[name]
        note.write(f"COTTAGE {name.upper()} {x0:.4f} {y0:.4f} {x1:.4f} {y1:.4f}\n")
    for name in ("hearth", "chimney"):
        note.write(f"COTTAGE {name.upper()} {plan[name][0]:.4f} {plan[name][1]:.4f}\n")
    for at in plan["windows"]:
        note.write(f"COTTAGE FRONT_WINDOW {at[0]:.4f} {at[1]:.4f}\n")
    for at in plan["alcove_windows"]:
        note.write(f"COTTAGE ALCOVE_WINDOW {at[0]:.4f} {at[1]:.4f}\n")
    rear = plan["rear"]
    note.write("COTTAGE REAR none\n" if rear is None else f"COTTAGE REAR {rear[0]:.4f} {rear[1]:.4f}\n")


# Both variations are built and measured, so the axis is proven rather than claimed.
# Only the default is exported: wiring the mirrored one in as a second kind is a
# change to the settlement, not to the figure.
COTTAGE_PLAN, COTTAGE_DOOR, COTTAGE_CLEAR = measure_the_cottage(True)
measure_the_cottage(False)
DOORWAYS = every_doorway()
for _name, _middle, _clear, _windows, _floor in DOORWAYS:
    print(
        f"MEASURED {_name:11} doorway {_clear:.3f} m clear at {_middle:+.3f}, "
        f"{len(_windows)} windows on {len({w[0] for w in _windows})} storeys, "
        f"floor {_floor[0]:.2f} m up behind a {_floor[1]:.2f} m step"
    )


# Which way the doorway faces, written where the game can read it.
#
# `shell` puts the door on the SOUTH wall - Blender -Y - and the glTF export turns
# Blender's Z-up into Y-up, so it arrives in the game facing +Z. The game turns each
# model by `model_turn` to land that on the street. The two were opposite for the
# life of the project and every measurement missed it, because every measurement
# asked the LOT where the door should be rather than asking the MODEL where it is.
HERE = os.path.dirname(os.path.abspath(__file__))
NOTE = os.path.join(os.path.dirname(os.path.dirname(HERE)), "assets", "models", "town.txt")
os.makedirs(os.path.dirname(NOTE), exist_ok=True)
with open(NOTE, "w", encoding="utf-8") as note:
    note.write("# Written by dev/art/town.py. Read by world::town's tests.\n")
    note.write("DOOR_ON_BLENDER_Y -1\n")
    # The FACADE of each city figure: the wall the game hangs lit windows on.
    #
    # Not the footprint. The footprint is what a building keeps clear on the ground
    # and it is bigger than the building - so panes placed against it floated a
    # metre off the glass, past the corners, lining up with nothing. This is the
    # `wide` and `deep` the figure was actually built with, the storeys that are
    # GLAZED (a tower spends its ground floor on a lobby), and how far up the
    # glazing starts.
    note.write(f"FLOOR_TALL {FLOOR_TALL}\n")
    note.write(f"LOBBY {FLOOR_TALL * 1.5}\n")
    note.write("FACADE city_block 10.5 9.0 4\n")
    note.write("FACADE city_tower 10.0 9.5 8\n")
    note.write("FACADE city_spire 11.0 11.0 13\n")
    write_the_plan(note, COTTAGE_PLAN, COTTAGE_DOOR, COTTAGE_CLEAR)
print(f"WROTE {NOTE}")
