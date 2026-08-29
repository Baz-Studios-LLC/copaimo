"""Builds the lamps that light a settlement after dark.

    dev/art/build.sh

Two, because there are two ages of the world. A village hangs a lantern on a timber
post at head height and the light falls in a pool round its foot; a city stands a
steel column at the kerb with an arm out over the carriageway. Same job, and you can
tell which town you are in from the silhouette alone before either is lit.

# The head is the light

The game puts a point light at the head of each lamp near the player and brings it up
as the sun goes down - see `world::lamp`. What is built here is the FITTING, and the
one thing it has to get right is that the head reads as a lamp when it is dark and as
a lamp when it is not. So the glass is a real box with a frame round it rather than a
painted face, and it sits where the light will be.
"""

import math
import os
import sys

import bpy

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import masonry
from masonry import box, tube, wedge

# How high each kind's light hangs, in metres. The game reads these back so the point
# light sits in the head rather than somewhere near it.
STREET_HEAD = 5.6
POST_HEAD = 3.1


def street():
    """A city's street lamp: a steel column with an arm out over the road."""
    parts = []
    # A base that reads as bolted down, which is most of what says "municipal".
    parts.append(box((0.52, 0.52, 0.18), (0.0, 0.0, 0.09), "parapet"))
    parts.append(tube(0.16, 0.5, (0.0, 0.0, 0.42), "steel", sides=10))
    parts.append(tube(0.11, STREET_HEAD - 0.7, (0.0, 0.0, 0.65 + (STREET_HEAD - 0.7) * 0.5), "steel", sides=10))

    # The arm, out over the carriageway, and the head on the end of it.
    reach = 1.5
    parts.append(
        box((reach, 0.13, 0.13), (reach * 0.5, 0.0, STREET_HEAD + 0.05), "steel")
    )
    parts.append(
        box((0.5, 0.13, 0.5), (0.26, 0.0, STREET_HEAD - 0.22), "steel",
            tilt=(0.0, math.radians(-38.0), 0.0))
    )
    at = (reach, 0.0, STREET_HEAD)
    parts.append(box((0.86, 0.42, 0.14), (at[0], at[1], at[2] + 0.07), "parapet"))
    # The glass, hung under the housing where the light will be.
    parts.append(box((0.74, 0.34, 0.2), (at[0], at[1], at[2] - 0.1), "glass"))
    return parts, STREET_HEAD + 0.2


def post():
    """A village's lamp: a lantern on a timber post, with an iron bracket."""
    parts = []
    parts.append(box((0.44, 0.44, 0.16), (0.0, 0.0, 0.08), "stone"))
    parts.append(box((0.19, 0.19, POST_HEAD - 0.5), (0.0, 0.0, 0.16 + (POST_HEAD - 0.5) * 0.5), "timber"))
    # A bracket, which is the piece that stops a post reading as a post.
    parts.append(
        box((0.5, 0.09, 0.09), (0.0, 0.0, POST_HEAD - 0.28), "steel", tilt=(0.0, 0.0, 0.0))
    )
    parts.append(
        box((0.34, 0.08, 0.34), (0.0, 0.0, POST_HEAD - 0.5), "steel",
            tilt=(0.0, math.radians(42.0), 0.0))
    )

    # THE LANTERN: a glazed box with a frame at its corners and a little roof, which
    # is the whole silhouette. Four uprights rather than a solid case, so it reads as
    # a lantern with something burning in it.
    span = 0.42
    parts.append(box((span, span, 0.06), (0.0, 0.0, POST_HEAD - 0.24), "steel"))
    parts.append(box((span * 0.86, span * 0.86, 0.46), (0.0, 0.0, POST_HEAD), "glass"))
    for sx in (-1.0, 1.0):
        for sy in (-1.0, 1.0):
            parts.append(
                box((0.05, 0.05, 0.5), (sx * span * 0.44, sy * span * 0.44, POST_HEAD), "steel")
            )
    parts.append(wedge(span + 0.16, span + 0.16, 0.22, (0.0, 0.0, POST_HEAD + 0.25), "steel"))
    parts.append(tube(0.04, 0.14, (0.0, 0.0, POST_HEAD + 0.54), "brass", sides=6))
    return parts, POST_HEAD + 0.62


FIGURES = {"street": street, "post": post}


def build(name: str) -> None:
    masonry.fresh()
    parts, tall = FIGURES[name]()
    whole = masonry.weld(parts, masonry.PALETTE, tall, name="prop")
    masonry.outline(whole)
    masonry.save_beside(f"lamp_{name}.blend")
    print(f"BUILT lamp_{name}  ({len(parts)} pieces, {tall:.1f} m tall)")


for figure in FIGURES:
    build(figure)

# Where the light hangs on each, written where the game can read it.
#
# The point light has to sit IN the head. Guessing at it puts the glow beside the
# fitting, which reads as a lamp with a bug in it - and the two heights are different,
# so one guess cannot serve both.
HERE = os.path.dirname(os.path.abspath(__file__))
NOTE = os.path.join(os.path.dirname(os.path.dirname(HERE)), "assets", "models", "lamp.txt")
os.makedirs(os.path.dirname(NOTE), exist_ok=True)
with open(NOTE, "w", encoding="utf-8") as note:
    note.write("# Written by dev/art/lamp.py. Read by world::lamp's tests.\n")
    note.write(f"STREET_HEAD {STREET_HEAD}\n")
    note.write(f"POST_HEAD {POST_HEAD}\n")
    note.write("STREET_ARM 1.5\n")
print(f"WROTE {NOTE}")
