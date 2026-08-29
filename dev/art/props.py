"""Builds the world's litter — rocks, logs, bushes and the rest — one `.blend` each.

    dev/art/build.sh

# These carry their colour in their vertices

The opposite rule from trees, and it matters. A tree is planted as an object and
tinted by the material its variety wears. Litter is not: a chunk's worth of
boulders, bushes and fallen logs is WELDED into one mesh, because fifty separate
little objects per chunk would be fifty draw calls paid for again in every shadow
cascade. One mesh wears one material, so every colour a rock has must live in its
vertices.

So each prop is **one object named `prop`**, with a colour attribute. The game's
prop material is plain white and multiplies by it.

# One shape per kind

The pool holds three variants of each kind, and litter is placed with a random turn
through a full circle and a random scale, so one authored shape reads as many. The
same bargain the trees make.

# The colour has the light baked into it

Every prop is darker at its base and lighter on top. Nothing here is textured and
the world is lit by one sun, so the shading that makes a rock look like a rock —
sitting IN the ground rather than on it, with light falling from above — has to be
painted in. It is the same trick the ground cover uses.
"""

import math
import os
import sys

import bpy

# The shared box/paint/outline helpers, found the same way `town.py` finds them:
# Blender runs a script with its own folder off sys.path.
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import masonry
import mathutils

KINDS = ("boulder", "scree", "bush", "stump", "log", "snag", "cactus", "brush")

# Every colour, in sRGB as anybody would write it down. Converted on the way in,
# because a colour attribute is LINEAR and the game reads it straight through.
PAINT = {
    "stone": (0.45, 0.44, 0.41),
    "stone_dark": (0.27, 0.26, 0.24),
    "bark": (0.31, 0.23, 0.16),
    "deadwood": (0.56, 0.51, 0.43),
    "leaf": (0.24, 0.36, 0.19),
    "cactus": (0.26, 0.42, 0.26),
    "straw": (0.45, 0.36, 0.23),
}

# How much darker the bottom of a thing is than the top.
FOOT_SHADE = 0.55


def to_linear(part: float) -> float:
    if part <= 0.04045:
        return part / 12.92
    return ((part + 0.055) / 1.055) ** 2.4


def fresh() -> None:
    bpy.ops.wm.read_factory_settings(use_empty=True)


def paint(obj, colour: str, tall: float) -> None:
    """Writes the colour into the mesh, darkening toward the foot.

    `tall` is how far up the shading ramp runs, in metres, and it is the height of
    the NEIGHBOURHOOD rather than of the piece. Scaled to the piece, a spill of
    scree came out near white while a boulder of the same rock read mid grey: every
    little chunk reached the top of its own ramp. Low things are darker because
    they are down among the grass, so the ramp has to be measured in the world and
    not in the object.
    """
    mesh = obj.data
    rgb = [to_linear(part) for part in PAINT[colour]]
    if not mesh.color_attributes:
        mesh.color_attributes.new(name="Color", type="FLOAT_COLOR", domain="POINT")
    layer = mesh.color_attributes["Color"]
    lowest = min(point.co.z for point in mesh.vertices)
    for point in mesh.vertices:
        # 0 at the foot, 1 a `tall` above it.
        up = min(1.0, max(0.0, (point.co.z - lowest) / max(tall, 1.0e-4)))
        shade = FOOT_SHADE + (1.0 - FOOT_SHADE) * up
        layer.data[point.index].color = (rgb[0] * shade, rgb[1] * shade, rgb[2] * shade, 1.0)


def rough(obj, amount: float, seed: int) -> None:
    """Pushes every vertex about a bit, so a shape is not a primitive.

    A boulder is an icosphere until this runs. Deterministic from `seed`, so the
    same rock is built every time — an asset that changed on every export would
    show up as a diff nobody asked for.
    """
    spin = mathutils.Vector((12.9898, 78.233, 37.719))
    for point in obj.data.vertices:
        for axis in range(3):
            noise = math.sin(point.co.dot(spin) * (axis + 1) + seed * 7.13)
            point.co[axis] += noise * amount


def ball(radius, at, squash=1.0, seed=0, jitter=0.0, subdiv=2):
    bpy.ops.mesh.primitive_ico_sphere_add(subdivisions=subdiv, radius=radius, location=at)
    obj = bpy.context.object
    obj.scale = (1.0, 1.0, squash)
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    if jitter:
        rough(obj, jitter, seed)
    return obj


def stick(radius, length, at, tilt=(0.0, 0.0, 0.0), sides=7):
    bpy.ops.mesh.primitive_cylinder_add(vertices=sides, radius=radius, depth=length, location=at)
    obj = bpy.context.object
    obj.rotation_euler = tilt
    bpy.ops.object.transform_apply(location=False, rotation=True, scale=False)
    return obj


def cone(radius, top, length, at, sides=7):
    bpy.ops.mesh.primitive_cone_add(
        vertices=sides, radius1=radius, radius2=top, depth=length, location=at
    )
    return bpy.context.object


# ------------------------------------------------------------------- the kinds


def boulder():
    """A stone too big to move. Angular, wider than it is tall."""
    parts = [(ball(1.15, (0.0, 0.0, 0.85), squash=0.78, seed=3, jitter=0.16), "stone")]
    parts.append((ball(0.55, (0.85, 0.30, 0.40), squash=0.8, seed=8, jitter=0.10), "stone_dark"))
    return parts, 1.7


def scree():
    """A spill of small broken stone: what a slope sheds."""
    parts = []
    spots = [
        (0.42, (0.00, 0.00, 0.26), 1),
        (0.30, (0.72, 0.24, 0.18), 2),
        (0.26, (-0.58, 0.44, 0.16), 3),
        (0.22, (0.24, -0.66, 0.14), 4),
        (0.19, (-0.40, -0.52, 0.12), 5),
    ]
    # The DARK stone, not the boulder's. A scree chunk is small and angular, so
    # most of its facets point somewhere near the sun and take the full of it —
    # painted in the same grey as a boulder it came out looking like chalk while
    # the boulder beside it read as rock. Fixing the ramp helped and was not
    # enough; the rest is lighting, so the paint has to give way.
    for index, (radius, at, seed) in enumerate(spots):
        stone = "stone" if index == 0 else "stone_dark"
        parts.append((ball(radius, at, squash=0.62, seed=seed, jitter=0.10, subdiv=1), stone))
    # A pile of stone lies IN the grass, so its ramp is a boulder's, not its own.
    return parts, 1.5


def bush():
    """Low woody growth: the layer between the grass and the trees."""
    parts = [(stick(0.07, 0.5, (0.0, 0.0, 0.25), sides=6), "bark")]
    for radius, at, seed in [
        (0.62, (0.0, 0.0, 0.72), 1),
        (0.44, (0.48, 0.20, 0.55), 2),
        (0.40, (-0.42, -0.26, 0.58), 3),
    ]:
        parts.append((ball(radius, at, squash=0.72, seed=seed, jitter=0.07), "leaf"))
    return parts, 1.1


def stump():
    """What is left where a tree came down: broken off, not cut."""
    parts = [(cone(0.44, 0.36, 0.9, (0.0, 0.0, 0.45), sides=9), "bark")]
    # The splintered top, sitting proud on one side.
    parts.append((ball(0.34, (0.10, 0.06, 0.94), squash=0.35, seed=6, jitter=0.09), "deadwood"))
    # A root breaking the ground.
    parts.append((stick(0.11, 0.7, (0.46, 0.10, 0.10), tilt=(0.0, math.radians(78), 0.0)), "bark"))
    return parts, 0.95


def log():
    """And the tree that came down, lying beside it."""
    parts = [
        (
            stick(0.34, 3.4, (0.0, 0.0, 0.34), tilt=(0.0, math.radians(90), 0.0), sides=9),
            "bark",
        )
    ]
    # A stub of a branch, and the pale broken end.
    parts.append((stick(0.10, 0.6, (0.55, 0.26, 0.52), tilt=(math.radians(64), 0.0, 0.0)), "bark"))
    parts.append((ball(0.30, (1.68, 0.0, 0.34), squash=0.9, seed=4, jitter=0.05, subdiv=1), "deadwood"))
    return parts, 0.9


def snag():
    """A dead tree still standing: bare, broken-limbed, pale."""
    parts = [(cone(0.30, 0.13, 4.4, (0.0, 0.0, 2.2), sides=8), "deadwood")]
    for radius, length, at, tilt in [
        (0.10, 1.2, (0.42, 0.0, 2.9), (0.0, math.radians(58), 0.0)),
        (0.08, 0.9, (-0.34, 0.18, 3.5), (0.0, math.radians(-64), 0.0)),
        (0.07, 0.7, (0.10, -0.36, 2.1), (math.radians(66), 0.0, 0.0)),
    ]:
        parts.append((stick(radius, length, at, tilt=tilt, sides=6), "deadwood"))
    return parts, 4.6


def cactus():
    """Dry country's answer to a tree: a column and two arms."""
    parts = [(cone(0.26, 0.22, 2.3, (0.0, 0.0, 1.15), sides=10), "cactus")]
    parts.append((ball(0.24, (0.0, 0.0, 2.3), squash=0.8, seed=1), "cactus"))
    for hand, at, height in [(1.0, 0.55, 1.35), (-1.0, -0.5, 1.05)]:
        parts.append((stick(0.14, 0.7, (at, 0.0, height), tilt=(0.0, math.radians(90), 0.0), sides=8), "cactus"))
        parts.append((cone(0.14, 0.11, 0.8, (at * 1.45, 0.0, height + 0.4), sides=8), "cactus"))
    return parts, 2.5


def brush():
    """A tangle of dead sticks, which is most of what dry ground grows."""
    parts = []
    for index in range(7):
        angle = index * 2.399
        lean = math.radians(52 + (index % 3) * 11)
        parts.append(
            (
                stick(
                    0.045,
                    0.9 + 0.1 * (index % 3),
                    (math.cos(angle) * 0.16, math.sin(angle) * 0.16, 0.36),
                    tilt=(lean * math.sin(angle), lean * math.cos(angle), 0.0),
                    sides=5,
                ),
                "straw",
            )
        )
    return parts, 1.3


BUILDERS = {
    "boulder": boulder,
    "scree": scree,
    "bush": bush,
    "stump": stump,
    "log": log,
    "snag": snag,
    "cactus": cactus,
    "brush": brush,
}

# Above this angle between two faces, the edge stays sharp. Lower than the trees
# use: a rock wants its facets, and only its round parts smoothed.
SHARP_ABOVE = math.radians(38.0)


def build(kind: str) -> None:
    fresh()
    parts, ramp = BUILDERS[kind]()

    # Painted before joining, while each part still knows what it is made of.
    for obj, colour in parts:
        bpy.ops.object.select_all(action="DESELECT")
        obj.select_set(True)
        bpy.context.view_layer.objects.active = obj
        bpy.ops.object.transform_apply(location=False, rotation=True, scale=False)
        paint(obj, colour, ramp)

    bpy.ops.object.select_all(action="SELECT")
    whole = parts[0][0]
    bpy.context.view_layer.objects.active = whole
    if len(parts) > 1:
        bpy.ops.object.join()
    whole = bpy.context.object
    whole.name = "prop"
    whole.data.name = "prop"
    bpy.ops.object.shade_auto_smooth(angle=SHARP_ABOVE)

    # An edge on it, like everything else in the world wears. See
    # `masonry.outline` - the art direction is "almost but not quite cel shaded",
    # and an outline is half of that; the banded light in `cloud_shade.wgsl` is the
    # other half and already applies to everything this material touches.
    whole = masonry.outline(whole)

    # On the floor, which the export gate insists on.
    low = min(
        (whole.matrix_world @ mathutils.Vector(corner)).z for corner in whole.bound_box
    )
    whole.location.z -= low
    bpy.ops.object.transform_apply(location=True, rotation=False, scale=False)

    here = os.path.dirname(os.path.abspath(__file__))
    bpy.ops.wm.save_as_mainfile(filepath=os.path.join(here, f"prop_{kind}.blend"))
    print(f"BUILT prop_{kind}")


for kind in KINDS:
    build(kind)
