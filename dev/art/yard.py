"""Builds what stands on a lot that has no building on it.

    dev/art/build.sh

# The empty tan parcels

A settlement is thinned to what a place of its kind HAS - sixteen houses in a
village, thirty-four in a city - and the lots left over were simply dropped. Once
the ground under a town stopped being meadow and started being packed earth, that
showed: photographed from the middle of a village, half the frontage was bare dirt
with nothing on it, and a city could hold thirty-four buildings and still read as
empty because each one stood alone in a tan field.

The answer is not more houses. A place is dense when its street EDGE is occupied,
and a fence, a row of beans, a lean-to and a stack of timber occupy it as surely as
a wall does - at a fraction of the geometry, and they say something a wall does not,
which is that somebody lives here and does something all day.

# A programme, not a scatter

Each figure here is one PURPOSE with its parts arranged to imply a relationship: a
garden has beds and a path from the gate to where the door would be; a work yard has
a bench under a lean-to with its material stacked beside it. That relationship is
what reads as authored. A hundred props scattered by a random number read as litter
however many there are of them.

# Two families, because there are two ages of the world

The world's old half is timber, thatch and packed earth; its cities are concrete,
steel and glass. One kit for both put a post-and-rail fence and a stack of crates in
the middle of a modern city, which reads as a farmyard somebody left between two
office towers - the research calls this architectural families, and mixing them is
the fastest way to make a generated place look assembled rather than built.

So every programme exists twice: the same PURPOSE in the vocabulary of its own age.
A crafts quarter has a work yard either way - it is a lean-to and stacked timber in a
village and a service bay with a skip and pallets in a city. Nine figures, each a
single welded object with the same ink outline every building wears, painted out of
`masonry.PALETTE` so nothing can drift from the buildings around it.
"""

import math
import os
import sys

import bpy

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import masonry
from masonry import box, lean, tube, wedge

# A lot's worth of ground, matching `Building::Cottage`'s footprint so a yard drops
# onto exactly the lots a house would have taken.
WIDE, DEEP = 9.0, 7.5

# How high a yard is allowed to build. Nothing here competes with a roofline: a yard
# that stands as tall as a house is a house.
LOW = 2.6


def _rail_fence(parts, wide, deep, colour="timber", rails=2, tall=1.05, gap=None):
    """A post-and-rail fence around three sides, with the front left open.

    Open at the FRONT on purpose - that is the side facing the street, and a yard you
    cannot see into is a box. The gap is where a gate would hang.
    """
    posts = 5
    for side in (-1.0, 1.0):
        for i in range(posts):
            along = (i / (posts - 1) - 0.5) * deep
            parts.append(box((0.14, 0.14, tall), (side * wide * 0.5, along, tall * 0.5), colour))
        for rail in range(rails):
            z = tall * (0.45 + rail * 0.42)
            parts.append(box((0.09, deep, 0.11), (side * wide * 0.5, 0.0, z), colour))
    # The back run.
    for i in range(posts + 1):
        across = (i / posts - 0.5) * wide
        parts.append(box((0.14, 0.14, tall), (across, deep * 0.5, tall * 0.5), colour))
    for rail in range(rails):
        z = tall * (0.45 + rail * 0.42)
        parts.append(box((wide, 0.09, 0.11), (0.0, deep * 0.5, z), colour))
    # And the two stubs of the front run, either side of the way in.
    opening = gap if gap else wide * 0.34
    run = (wide - opening) * 0.5
    for side in (-1.0, 1.0):
        at = side * (opening * 0.5 + run * 0.5)
        parts.append(box((run, 0.09, 0.11), (at, -deep * 0.5, tall * 0.62), colour))
        parts.append(box((0.14, 0.14, tall), (side * opening * 0.5, -deep * 0.5, tall * 0.5), colour))


def _path(parts, wide, deep, opening=3.0):
    """Beaten earth from the gate to the back of the yard.

    The one part that is not an object: it is the evidence that the objects get used.
    """
    parts.append(box((opening * 0.72, deep * 0.86, 0.06), (0.0, 0.04, 0.03), "infloor"))


def garden():
    """A kitchen garden: beds in rows, a fruit tree, a water butt."""
    parts = []
    _rail_fence(parts, WIDE, DEEP, "timber", rails=1, tall=0.72)
    _path(parts, WIDE, DEEP)

    # Four beds, two either side of the path, each with its rows showing.
    for side in (-1.0, 1.0):
        for row in range(2):
            at = (side * WIDE * 0.27, (row - 0.5) * DEEP * 0.42)
            parts.append(box((WIDE * 0.34, DEEP * 0.3, 0.26), (at[0], at[1], 0.13), "infloor"))
            for drill in range(3):
                along = (drill / 2.0 - 0.5) * DEEP * 0.22
                parts.append(
                    box((WIDE * 0.3, 0.22, 0.3), (at[0], at[1] + along, 0.34), "leafy")
                )

    # A fruit tree in the back corner, and a butt to water from.
    parts.append(tube(0.13, 1.5, (WIDE * 0.32, DEEP * 0.3, 0.75), "timber", sides=7))
    parts.append(box((1.7, 1.7, 1.1), (WIDE * 0.32, DEEP * 0.3, 2.05), "leafy"))
    parts.append(tube(0.42, 0.9, (-WIDE * 0.36, DEEP * 0.32, 0.45), "timber", sides=10))
    return parts, 2.6


def work_yard():
    """A workshop's outdoor half: a lean-to, a bench, stacked timber, a barrel."""
    parts = []
    _rail_fence(parts, WIDE, DEEP, "timber", rails=2, tall=1.05)
    _path(parts, WIDE, DEEP)

    # THE LEAN-TO, against the back fence. Posts, a sloping roof, and a bench under
    # it - the three pieces that say "work happens here and it happens outdoors".
    span, deep = WIDE * 0.62, DEEP * 0.36
    at_y = DEEP * 0.28
    for side in (-1.0, 1.0):
        parts.append(box((0.16, 0.16, 2.3), (side * span * 0.5, at_y - deep * 0.5, 1.15), "timber"))
        parts.append(box((0.16, 0.16, 2.75), (side * span * 0.5, at_y + deep * 0.5, 1.38), "timber"))
    parts.append(lean(span + 0.4, deep + 0.3, 2.75, (0.0, at_y, 0.0), "slate", drops_to=2.3))
    parts.append(box((span * 0.8, 0.5, 0.14), (0.0, at_y + deep * 0.2, 0.9), "counter"))
    for side in (-1.0, 1.0):
        parts.append(box((0.14, 0.4, 0.9), (side * span * 0.3, at_y + deep * 0.2, 0.45), "timber"))

    # Timber stacked in courses, each course turned across the one below it.
    for course in range(4):
        turn = course % 2
        size = (1.9, 0.5, 0.22) if turn else (0.5, 1.9, 0.22)
        parts.append(box(size, (-WIDE * 0.3, -DEEP * 0.16, 0.11 + course * 0.23), "timber"))
    parts.append(tube(0.4, 0.95, (WIDE * 0.32, -DEEP * 0.18, 0.48), "timber", sides=10))
    # A sawhorse, which is the smallest thing that reads as a job half done.
    for side in (-1.0, 1.0):
        parts.append(
            box((0.1, 0.1, 0.8), (WIDE * 0.06 + side * 0.35, -DEEP * 0.3, 0.4), "timber",
                tilt=(math.radians(side * 12.0), 0.0, 0.0))
        )
    parts.append(box((0.16, 1.3, 0.12), (WIDE * 0.06, -DEEP * 0.3, 0.82), "timber"))
    return parts, 2.9


def pen():
    """An animal pen: heavier fence, a trough, a heap of hay, a shelter."""
    parts = []
    _rail_fence(parts, WIDE, DEEP, "timber", rails=2, tall=1.25, gap=2.2)
    parts.append(box((WIDE * 0.9, DEEP * 0.9, 0.05), (0.0, 0.0, 0.025), "infloor"))

    # A three-sided shelter in the back corner.
    span, deep = WIDE * 0.42, DEEP * 0.3
    at = (-WIDE * 0.24, DEEP * 0.3)
    parts.append(box((span, 0.16, 1.9), (at[0], at[1] + deep * 0.5, 0.95), "timber"))
    for side in (-1.0, 1.0):
        parts.append(box((0.16, deep, 1.9), (at[0] + side * span * 0.5, at[1], 0.95), "timber"))
    parts.append(lean(span + 0.3, deep + 0.3, 2.2, (at[0], at[1], 0.0), "thatch", drops_to=1.8))

    # The trough, and the hay that is the reason anything comes to it.
    parts.append(box((2.2, 0.7, 0.5), (WIDE * 0.2, DEEP * 0.16, 0.25), "timber"))
    parts.append(box((2.0, 0.5, 0.1), (WIDE * 0.2, DEEP * 0.16, 0.52), "counter"))
    parts.append(wedge(1.8, 1.8, 1.0, (WIDE * 0.24, -DEEP * 0.24, 0.0), "thatch"))
    return parts, 2.4


def store():
    """A service yard: crates, barrels, and planks under a sheet."""
    parts = []
    _rail_fence(parts, WIDE, DEEP, "timber", rails=2, tall=1.05)
    parts.append(box((WIDE * 0.86, DEEP * 0.86, 0.05), (0.0, 0.0, 0.025), "infloor"))

    # Crates, stacked the way crates actually stack: a big course, a smaller one on
    # top, and one set down beside the pile because somebody was carrying it.
    for i, (x, y, s, h) in enumerate(
        [(-0.3, 0.3, 1.15, 1.0), (0.72, 0.34, 1.0, 0.86), (-0.34, -0.28, 1.05, 0.9)]
    ):
        parts.append(box((s, s, h), (x * WIDE * 0.34, y * DEEP * 0.5, h * 0.5), "shelf"))
        if i < 2:
            parts.append(
                box((s * 0.8, s * 0.8, h * 0.7), (x * WIDE * 0.34, y * DEEP * 0.5, h + h * 0.35), "board")
            )
    for side in (-1.0, 1.0):
        parts.append(tube(0.38, 0.9, (side * WIDE * 0.34, -DEEP * 0.3, 0.45), "timber", sides=10))
    # Planks under a sheet, leaning on the back fence.
    parts.append(
        box((WIDE * 0.5, 0.6, 1.5), (WIDE * 0.16, DEEP * 0.36, 0.75), "timber",
            tilt=(math.radians(-16.0), 0.0, 0.0))
    )
    parts.append(
        box((WIDE * 0.54, 0.7, 0.1), (WIDE * 0.16, DEEP * 0.32, 1.5), "cloth",
            tilt=(math.radians(-16.0), 0.0, 0.0))
    )
    return parts, 2.4


def stall():
    """A market stall: an awning, a counter, and what is for sale under it.

    The market district's programme. It has no fence - a stall belongs to the street
    rather than to a plot, and fencing one would put a wall across the square.
    """
    parts = []
    span, deep = WIDE * 0.72, DEEP * 0.46
    parts.append(box((span + 0.6, deep + 0.6, 0.06), (0.0, 0.0, 0.03), "infloor"))
    for sx in (-1.0, 1.0):
        for sy in (-1.0, 1.0):
            parts.append(
                box((0.13, 0.13, 2.3), (sx * span * 0.5, sy * deep * 0.5, 1.15), "timber")
            )
    # A pitched awning, which is the silhouette that says market from across a square.
    parts.append(wedge(span + 0.7, deep + 0.7, 0.85, (0.0, 0.0, 2.3), "cloth", ridge="x"))
    parts.append(box((span, 0.6, 0.12), (0.0, -deep * 0.4, 1.0), "counter"))
    for side in (-1.0, 1.0):
        parts.append(box((0.12, 0.5, 1.0), (side * span * 0.42, -deep * 0.4, 0.5), "timber"))
    # Goods: a crate open on the counter, two sacks, a hanging bunch.
    parts.append(box((0.9, 0.5, 0.34), (-span * 0.24, -deep * 0.4, 1.23), "shelf"))
    parts.append(box((0.8, 0.42, 0.2), (-span * 0.24, -deep * 0.4, 1.5), "flower"))
    for side in (-1.0, 1.0):
        parts.append(
            tube(0.34, 0.7, (side * span * 0.3, deep * 0.22, 0.35), "thatch", sides=9)
        )
    parts.append(box((0.5, 0.3, 0.6), (span * 0.26, -deep * 0.36, 1.85), "leafy"))
    return parts, 3.2


# ============================================================ THE MODERN CITY
#
# The same purposes in the other age's vocabulary: concrete kerbs instead of timber
# rails, mesh instead of post-and-rail, a skip instead of a woodpile, a steel-framed
# kiosk instead of a canvas stall. Nothing here is thatched, and nothing is a crate.


def _kerb(parts, wide, deep, colour="concrete2", tall=0.34):
    """A raised kerb round the plot, which is how a city says where ground ends.

    A city does not fence a planted square; it kerbs it. The kerb is low enough to
    step over and high enough to read as a made edge from across the street.
    """
    for side in (-1.0, 1.0):
        parts.append(box((0.34, deep, tall), (side * wide * 0.5, 0.0, tall * 0.5), colour))
        parts.append(box((wide, 0.34, tall), (0.0, side * deep * 0.5, tall * 0.5), colour))


def _bollard(parts, at, colour="steel", tall=0.95):
    parts.append(tube(0.11, tall, (at[0], at[1], tall * 0.5), colour, sides=8))
    parts.append(tube(0.14, 0.09, (at[0], at[1], tall), "parapet", sides=8))


def _bench(parts, at, span=1.9, turn=0.0):
    """A bench: a slab on two steel legs, which is every civic bench ever made."""
    sin, cos = math.sin(turn), math.cos(turn)
    out = lambda x, y: (at[0] + x * cos - y * sin, at[1] + x * sin + y * cos)
    seat = out(0.0, 0.0)
    parts.append(box((span, 0.5, 0.11), (seat[0], seat[1], 0.46), "timber", tilt=(0.0, 0.0, turn)))
    parts.append(box((span, 0.12, 0.42), (seat[0], seat[1], 0.75), "timber", tilt=(0.0, 0.0, turn)))
    for side in (-1.0, 1.0):
        leg = out(side * span * 0.38, 0.0)
        parts.append(box((0.09, 0.46, 0.46), (leg[0], leg[1], 0.23), "steel", tilt=(0.0, 0.0, turn)))


def city_green():
    """A planted square: kerbed beds, clipped hedge, a tree in a grate, benches."""
    parts = []
    parts.append(box((WIDE, DEEP, 0.06), (0.0, 0.0, 0.03), "concrete"))
    _kerb(parts, WIDE, DEEP)

    # Two raised beds with hedge clipped flat on top. Flat because a city clips.
    for side in (-1.0, 1.0):
        at = (side * WIDE * 0.28, DEEP * 0.14)
        parts.append(box((WIDE * 0.36, DEEP * 0.4, 0.5), (at[0], at[1], 0.25), "concrete2"))
        parts.append(box((WIDE * 0.32, DEEP * 0.36, 0.62), (at[0], at[1], 0.81), "leafy"))

    # A tree in a grate, which is the one piece of nature a city admits to planting.
    parts.append(box((1.5, 1.5, 0.07), (0.0, -DEEP * 0.18, 0.06), "steel"))
    parts.append(tube(0.15, 2.4, (0.0, -DEEP * 0.18, 1.2), "timber", sides=8))
    parts.append(box((2.3, 2.3, 1.4), (0.0, -DEEP * 0.18, 3.0), "leafy"))

    _bench(parts, (-WIDE * 0.3, -DEEP * 0.34))
    _bench(parts, (WIDE * 0.3, -DEEP * 0.34))
    for side in (-1.0, 1.0):
        _bollard(parts, (side * WIDE * 0.46, -DEEP * 0.46))
    return parts, 3.8


def city_service():
    """A service bay: mesh fence, a skip, stacked pallets, plant, bollards."""
    parts = []
    parts.append(box((WIDE, DEEP, 0.06), (0.0, 0.0, 0.03), "concrete2"))

    # MESH FENCE round three sides. Posts, a top rail, and a panel thin enough that
    # it reads as mesh rather than as a wall.
    tall = 1.9
    for side in (-1.0, 1.0):
        for i in range(4):
            along = (i / 3.0 - 0.5) * DEEP
            parts.append(box((0.11, 0.11, tall), (side * WIDE * 0.5, along, tall * 0.5), "steel"))
        parts.append(box((0.05, DEEP, tall * 0.86), (side * WIDE * 0.5, 0.0, tall * 0.5), "mullion"))
        parts.append(box((0.14, DEEP, 0.1), (side * WIDE * 0.5, 0.0, tall), "steel"))
    for i in range(5):
        across = (i / 4.0 - 0.5) * WIDE
        parts.append(box((0.11, 0.11, tall), (across, DEEP * 0.5, tall * 0.5), "steel"))
    parts.append(box((WIDE, 0.05, tall * 0.86), (0.0, DEEP * 0.5, tall * 0.5), "mullion"))
    parts.append(box((WIDE, 0.14, 0.1), (0.0, DEEP * 0.5, tall), "steel"))

    # A skip, tapered the way a skip is, with its lip proud of the body.
    at = (-WIDE * 0.22, DEEP * 0.2)
    parts.append(box((3.0, 1.7, 1.15), (at[0], at[1], 0.58), "canopy"))
    parts.append(box((3.2, 1.9, 0.12), (at[0], at[1], 1.2), "steel"))
    parts.append(box((2.6, 1.3, 0.4), (at[0], at[1], 1.35), "board"))

    # Pallets, stacked flat, and a plant unit humming against the fence.
    for course in range(4):
        parts.append(
            box((1.5, 1.1, 0.16), (WIDE * 0.26, DEEP * 0.22, 0.08 + course * 0.17), "timber")
        )
    parts.append(box((1.6, 1.2, 1.5), (WIDE * 0.24, -DEEP * 0.2, 0.75), "parapet"))
    parts.append(box((1.3, 0.1, 1.1), (WIDE * 0.24, -DEEP * 0.2 - 0.6, 0.9), "mullion"))
    for side in (-1.0, 1.0):
        _bollard(parts, (side * WIDE * 0.3, -DEEP * 0.46))
    return parts, 2.4


def city_kiosk():
    """A kiosk: a steel frame with a glass front and a flat canopy over it.

    The market stall's opposite number. Same job - somebody selling something to the
    street - in a vocabulary with no canvas in it.
    """
    parts = []
    span, deep = WIDE * 0.66, DEEP * 0.44
    parts.append(box((span + 0.8, deep + 0.8, 0.07), (0.0, 0.0, 0.035), "concrete"))

    # The box itself: a low plinth, glazing above it, mullions at the corners.
    parts.append(box((span, deep, 0.9), (0.0, 0.0, 0.45), "parapet"))
    parts.append(box((span * 0.94, deep * 0.94, 1.5), (0.0, 0.0, 1.65), "curtain"))
    for sx in (-1.0, 1.0):
        for sy in (-1.0, 1.0):
            parts.append(
                box((0.13, 0.13, 2.5), (sx * span * 0.5, sy * deep * 0.5, 1.25), "mullion")
            )
    # A flat canopy, cantilevered over the front - flat, because a pitched roof would
    # be the village stall again.
    parts.append(box((span + 1.0, deep + 1.2, 0.16), (0.0, -0.25, 2.62), "steel"))
    parts.append(box((span * 0.5, 0.1, 0.42), (0.0, -deep * 0.5 - 0.55, 2.3), "neon"))
    # A counter shelf out of the front, and the goods behind the glass.
    parts.append(box((span * 0.8, 0.4, 0.1), (0.0, -deep * 0.5 - 0.2, 1.0), "steel"))
    parts.append(box((span * 0.6, deep * 0.3, 0.5), (0.0, 0.1, 1.25), "shelf"))
    for side in (-1.0, 1.0):
        _bollard(parts, (side * (span * 0.5 + 0.7), -deep * 0.5 - 0.7), tall=0.8)
    return parts, 2.8


def city_forecourt():
    """Paved breathing space: seating, planters and lamps, and nothing else.

    A city's version of the village's open ground - and it is not empty, it is PAVED,
    which is the difference between somewhere nobody built and somewhere finished.
    """
    parts = []
    parts.append(box((WIDE, DEEP, 0.06), (0.0, 0.0, 0.03), "concrete"))
    _kerb(parts, WIDE, DEEP, tall=0.26)
    # Banding in the paving, which is what stops a slab reading as a slab.
    for i in range(3):
        parts.append(
            box((WIDE * 0.94, 0.22, 0.02), (0.0, (i - 1.0) * DEEP * 0.28, 0.075), "concrete2")
        )
    for side in (-1.0, 1.0):
        _bench(parts, (side * WIDE * 0.26, DEEP * 0.24), turn=math.pi)
        parts.append(
            box((1.2, 1.2, 0.62), (side * WIDE * 0.3, -DEEP * 0.26, 0.31), "concrete2")
        )
        parts.append(
            box((1.05, 1.05, 0.5), (side * WIDE * 0.3, -DEEP * 0.26, 0.87), "leafy")
        )
        # A lamp: a column with an arm and a head, which reads at any distance.
        at = (side * WIDE * 0.44, 0.0)
        parts.append(tube(0.09, 4.2, (at[0], at[1], 2.1), "steel", sides=8))
        parts.append(box((0.7, 0.16, 0.12), (at[0] - side * 0.3, at[1], 4.2), "steel"))
        parts.append(box((0.5, 0.3, 0.14), (at[0] - side * 0.55, at[1], 4.12), "parapet"))
    return parts, 4.3

FIGURES = {
    "garden": garden,
    "work": work_yard,
    "pen": pen,
    "store": store,
    "stall": stall,
    # The same purposes, in the city's own vocabulary.
    "city_green": city_green,
    "city_service": city_service,
    "city_kiosk": city_kiosk,
    "city_forecourt": city_forecourt,
}


def build(name: str) -> None:
    masonry.fresh()
    parts, tall = FIGURES[name]()
    whole = masonry.weld(parts, masonry.PALETTE, tall, name="prop")
    # The same ink every building wears, so a yard belongs to the same drawing.
    masonry.outline(whole)
    masonry.save_beside(f"yard_{name}.blend")
    print(f"BUILT yard_{name}  ({len(parts)} pieces, {tall:.1f} m tall)")


for figure in FIGURES:
    build(figure)
