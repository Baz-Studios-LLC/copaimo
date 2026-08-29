"""Builds the stone bridges that carry a road over water.

    dev/art/build.sh

# A bridge, not a causeway

The road network is walked over dry ground, so it can never enter water - but two
landmasses that need joining are joined by something, and the something is not a
filling-in of the channel. Raising the seabed until a road can drive across would
redraw the coastline, move the biome that follows the coastline, and put solid
ground where the map says sea. So the water stays exactly as it is and a structure
is carried over it.

# Modular, because the spans are long

The world's crossings come out at several hundred metres and more, and a fantasy
world is welcome to a long bridge. Nothing here models one: this builds ONE arch and
the game repeats it along the crossing, which is how a real viaduct is built and the
only way a 1.2 km bridge is affordable. Two figures come out of it:

  bridge_span  one arch and the pier under it, 18 m of bridge
  bridge_end   the abutment where the deck meets the shore

`SPAN_LONG` and `DECK_ABOVE_FOOT` are the contract with the game. The game places a
span every `SPAN_LONG` metres and drops each one so its road surface lands on the
deck height, which is `DECK_ABOVE_FOOT` above the model's own foot. Both numbers are
checked against the exported models by a test, the same way a building's footprint is.

# The pier goes a long way down

Further than any water here is deep, on purpose. A pier that stops at the seabed
needs to know where the seabed is, per crossing, per pier; a pier that runs well
past it is buried in the bed and nobody can tell. The part anybody sees is the arch
springing clear of the water.
"""

import math
import os
import sys

import bpy

# Blender runs a script with the CWD wherever it was launched from, not beside the
# file, so the folder these scripts share has to be put on the path by hand.
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import masonry
from masonry import box

# ------------------------------------------------------- the contract with the game
#
# One arch and the pier beside it. The game lays these end to end, so this is the
# repeat distance as well as the model's length.
SPAN_LONG = 18.0

# How far above the model's own foot the road surface sits.
#
# `masonry.weld` stands every figure on z = 0, so the model's foot is the bottom of
# its pier and this is the whole height of the thing up to the deck. The game
# subtracts it from the crossing's deck height to know where to put the model.
DECK_ABOVE_FOOT = 24.0

# Kerb to kerb, and the parapets outside that.
ROADWAY_WIDE = 6.4
PARAPET_THICK = 0.5
PARAPET_TALL = 0.95
DECK_WIDE = ROADWAY_WIDE + PARAPET_THICK * 2.0

# The deck slab, and the spandrel course under it.
SLAB_THICK = 0.8
SPANDREL_UNDER = 1.2

# The arch. Semicircular, so the opening is twice the rise - the shape every stone
# bridge in the reference folder is built on, and the one that reads as "bridge" at
# any distance.
ARCH_RISE = 5.5
ARCH_RING = 0.9
VOUSSOIRS = 15

# Where the arch springs from, measured down from the road surface.
SPRING_UNDER = SLAB_THICK + SPANDREL_UNDER + ARCH_RISE

# What is left below the springing is pier, and it runs to the model's foot.
PIER_DEEP = DECK_ABOVE_FOOT - SPRING_UNDER

PAINT = {
    "stone": (0.50, 0.49, 0.46),
    "stone2": (0.44, 0.43, 0.40),
    "coping": (0.58, 0.57, 0.53),
    "road": (0.46, 0.36, 0.24),
}


def _arch_top(along: float) -> float:
    """How high the arch's outer face stands at a point along the span.

    Measured up from the springing. Off the ends of the opening there is no arch,
    only pier, so the answer is the springing itself.
    """
    half = ARCH_RISE
    if abs(along) >= half:
        return 0.0
    return math.sqrt(half * half - along * along) + ARCH_RING


def _deck(parts, long: float, at_x: float = 0.0) -> None:
    """The road surface, its slab and both parapets, for `long` metres of bridge."""
    surface = DECK_ABOVE_FOOT
    parts.append(
        box((long, DECK_WIDE, SLAB_THICK), (at_x, 0.0, surface - SLAB_THICK * 0.5), "stone")
    )
    # The roadway itself, laid ON the slab so the deck is not the same colour as the
    # masonry holding it up - a bridge you cannot see the road on reads as a wall.
    parts.append(box((long, ROADWAY_WIDE, 0.12), (at_x, 0.0, surface + 0.06), "road"))
    for side in (-1.0, 1.0):
        out = side * (DECK_WIDE - PARAPET_THICK) * 0.5
        parts.append(
            box(
                (long, PARAPET_THICK, PARAPET_TALL),
                (at_x, out, surface + PARAPET_TALL * 0.5),
                "stone2",
            )
        )
        # A coping along the top, which is what stops a parapet reading as a kerb.
        parts.append(
            box(
                (long, PARAPET_THICK + 0.16, 0.14),
                (at_x, out, surface + PARAPET_TALL + 0.07),
                "coping",
            )
        )


def span():
    """One arch, the pier under it and the deck over it."""
    parts = []
    spring = DECK_ABOVE_FOOT - SPRING_UNDER

    # THE PIER, in two halves at the ends of the module, so consecutive spans butt
    # together into one pier. Tapered, because a pier that does not batter looks like
    # a column somebody left in a river.
    pier_wide = (SPAN_LONG - ARCH_RISE * 2.0) * 0.5
    for side in (-1.0, 1.0):
        at = side * (SPAN_LONG - pier_wide) * 0.5
        courses = 6
        for course in range(courses):
            part = course / courses
            # Battered: wider at the foot, and each course a shade narrower.
            grow = 1.0 + (1.0 - part) * 0.22
            parts.append(
                box(
                    (pier_wide * grow, (ROADWAY_WIDE + 0.6) * (1.0 + (1.0 - part) * 0.1),
                     PIER_DEEP / courses),
                    (at, 0.0, PIER_DEEP * (part + 0.5 / courses)),
                    "stone" if course % 2 else "stone2",
                )
            )
        # A cutwater on the upstream side of each pier: the wedge that splits the
        # current. It is the detail that says "bridge" more than the arch does.
        for face in (-1.0, 1.0):
            parts.append(
                box(
                    (pier_wide * 0.5, pier_wide * 0.5, PIER_DEEP * 0.72),
                    (at, face * (ROADWAY_WIDE + 0.6) * 0.5, PIER_DEEP * 0.36),
                    "stone2",
                    tilt=(0.0, 0.0, math.radians(45.0)),
                )
            )

    # THE ARCH RING, as voussoirs turned about the springing line.
    for stone in range(VOUSSOIRS):
        turn = math.pi * (stone + 0.5) / VOUSSOIRS
        radius = ARCH_RISE + ARCH_RING * 0.5
        arc = math.pi * ARCH_RISE / VOUSSOIRS * 1.06
        parts.append(
            box(
                (arc, ROADWAY_WIDE + 0.4, ARCH_RING),
                (
                    math.cos(turn) * radius,
                    0.0,
                    spring + math.sin(turn) * radius,
                ),
                "stone" if stone % 2 else "stone2",
                # Local X along the ring, local Z out through its thickness.
                tilt=(0.0, -(turn + math.pi * 0.5), 0.0),
            )
        )

    # THE SPANDREL: what fills the triangle between the arch's back and the deck.
    # Built as a course of blocks rather than one slab, so the masonry reads.
    slices = 18
    for slice_ in range(slices):
        along = (slice_ + 0.5) / slices * SPAN_LONG - SPAN_LONG * 0.5
        foot = spring + _arch_top(along)
        head = DECK_ABOVE_FOOT - SLAB_THICK
        if head - foot < 0.15:
            continue
        parts.append(
            box(
                (SPAN_LONG / slices * 1.02, ROADWAY_WIDE + 0.4, head - foot),
                (along, 0.0, (foot + head) * 0.5),
                "stone2" if slice_ % 2 else "stone",
            )
        )

    _deck(parts, SPAN_LONG)
    return parts, DECK_ABOVE_FOOT + PARAPET_TALL + 0.14


def end():
    """The abutment: where the deck meets the shore.

    Solid, and deliberately longer under the road than over it. This is the piece
    that takes the deck's height down to whatever the shore is doing, so it is a
    block of masonry with a road over the top rather than an arch.
    """
    parts = []
    long = SPAN_LONG * 0.5
    courses = 8
    for course in range(courses):
        part = course / courses
        grow = 1.0 + (1.0 - part) * 0.18
        parts.append(
            box(
                (long * grow, DECK_WIDE * (1.0 + (1.0 - part) * 0.08),
                 (DECK_ABOVE_FOOT - SLAB_THICK) / courses),
                (0.0, 0.0, (DECK_ABOVE_FOOT - SLAB_THICK) * (part + 0.5 / courses)),
                "stone" if course % 2 else "stone2",
            )
        )
    _deck(parts, long)
    return parts, DECK_ABOVE_FOOT + PARAPET_TALL + 0.14


FIGURES = {
    "span": span,
    "end": end,
}


def build(name: str) -> None:
    masonry.fresh()
    parts, tall = FIGURES[name]()
    whole = masonry.weld(parts, PAINT, tall, name="prop")
    # The same dark edge everything else in the world wears.
    masonry.outline(whole)
    masonry.save_beside(f"bridge_{name}.blend")
    print(f"BUILT bridge_{name}  ({len(parts)} pieces, {tall:.1f} m tall)")


for figure in FIGURES:
    build(figure)

# What was built, written where the game can read it.
#
# `SPAN_LONG` and `DECK_ABOVE_FOOT` are a contract between this file and
# `world::bridge`, and a contract nobody checks is a comment. If an arch's span
# changes here and not there, the game lays the arches at the wrong spacing - which
# shows as gaps between them, or as every second pier buried in its neighbour, and
# never as an error. `the_bridge_models_are_the_size_the_game_thinks_they_are` reads
# this and fails instead.
HERE = os.path.dirname(os.path.abspath(__file__))
NOTE = os.path.join(os.path.dirname(os.path.dirname(HERE)), "assets", "models", "bridge.txt")
os.makedirs(os.path.dirname(NOTE), exist_ok=True)
with open(NOTE, "w", encoding="utf-8") as note:
    note.write("# Written by dev/art/bridge.py. Read by world::bridge's tests.\n")
    note.write(f"SPAN_LONG {SPAN_LONG}\n")
    note.write(f"DECK_ABOVE_FOOT {DECK_ABOVE_FOOT}\n")
    note.write(f"ROADWAY_WIDE {ROADWAY_WIDE}\n")
print(f"WROTE {NOTE}")
