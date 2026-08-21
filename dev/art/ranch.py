"""Builds the ranch: the house, the barn, and the pieces that make a yard.

    dev/art/build.sh

# The ranch is where the game starts, so it is what the game looks like

A new warden opens their eyes here, and until there are monsters to raise this is
the only PLACE in the world rather than scenery. It wants to read as somewhere
somebody lives: a house with a chimney, a barn big enough to stable something, a
fenced paddock, and the ordinary clutter of a working yard.

# One object, vertex-coloured, like the litter

A placed thing is carried whole and spawned as a scene, so a building could wear
proper materials. It carries its colour in its vertices instead, for the same
reason the litter does — one material for the lot, and the game's shading applies
to it evenly. It also means a building can be recoloured by editing one table
here rather than by opening a material editor.

# Placed by a sheet, not by this script

What this builds is `assets/models/ranch_*.glb`. WHERE each one stands is
`assets/world/placed.json`, written by `dev/ranch_plan.py` — a short list of *this
thing, here, turned this way*. Keeping them apart is what lets the yard be
rearranged without rebuilding a single model.

# Sizes are against a 1.8 m warden

A door is 2.1 m. The house eaves are 3.2 m and its ridge 5.4 m. The barn is
deliberately bigger than the house — 11 m across and 7.5 m to the ridge — because
a barn that does not dwarf a house is a shed.
"""

import math
import os

import bpy
import mathutils

PIECES = ("house", "barn", "fence", "gate", "trough", "silo")

# Every colour, in sRGB. Converted on the way in: a colour attribute is LINEAR.
PAINT = {
    "wall": (0.74, 0.68, 0.56),
    "beam": (0.34, 0.24, 0.16),
    "roof": (0.36, 0.21, 0.17),
    "barnwall": (0.52, 0.24, 0.20),
    "barnroof": (0.30, 0.30, 0.32),
    "stone": (0.45, 0.44, 0.41),
    "door": (0.28, 0.20, 0.13),
    "glass": (0.34, 0.45, 0.50),
    "metal": (0.42, 0.43, 0.45),
    "water": (0.16, 0.30, 0.38),
}

# How much darker the foot of a thing is than its top. Gentler than the litter's,
# because a building is tall and a strong ramp reads as a spotlight on the roof.
FOOT_SHADE = 0.74


def to_linear(part: float) -> float:
    if part <= 0.04045:
        return part / 12.92
    return ((part + 0.055) / 1.055) ** 2.4


def fresh() -> None:
    bpy.ops.wm.read_factory_settings(use_empty=True)


def paint(obj, colour: str, tall: float) -> None:
    mesh = obj.data
    rgb = [to_linear(part) for part in PAINT[colour]]
    if not mesh.color_attributes:
        mesh.color_attributes.new(name="Color", type="FLOAT_COLOR", domain="POINT")
    layer = mesh.color_attributes["Color"]
    for point in mesh.vertices:
        up = min(1.0, max(0.0, point.co.z / max(tall, 1.0e-4)))
        shade = FOOT_SHADE + (1.0 - FOOT_SHADE) * up
        layer.data[point.index].color = (rgb[0] * shade, rgb[1] * shade, rgb[2] * shade, 1.0)


def box(size, at, colour, tilt=None):
    """One box, given as full extents in metres and the middle it sits at."""
    bpy.ops.mesh.primitive_cube_add(size=1.0, location=at)
    obj = bpy.context.object
    obj.scale = size
    if tilt:
        obj.rotation_euler = tilt
    bpy.ops.object.transform_apply(location=False, rotation=True, scale=True)
    return (obj, colour)


def wedge(span, deep, high, at, colour):
    """A pitched roof: a prism lying along Y, ridge down the middle."""
    half, back = span * 0.5, deep * 0.5
    places = [
        (-half, -back, 0.0), (half, -back, 0.0), (half, back, 0.0), (-half, back, 0.0),
        (0.0, -back, high), (0.0, back, high),
    ]
    faces = [(0, 1, 2, 3), (0, 4, 5, 3), (1, 2, 5, 4), (0, 1, 4), (3, 2, 5)]
    mesh = bpy.data.meshes.new("roof")
    mesh.from_pydata([(x + at[0], y + at[1], z + at[2]) for x, y, z in places], [], faces)
    mesh.update()
    obj = bpy.data.objects.new("roof", mesh)
    bpy.context.collection.objects.link(obj)
    return (obj, colour)


def tube(radius, deep, at, colour, sides=12, tilt=None):
    bpy.ops.mesh.primitive_cylinder_add(vertices=sides, radius=radius, depth=deep, location=at)
    obj = bpy.context.object
    if tilt:
        obj.rotation_euler = tilt
        bpy.ops.object.transform_apply(location=False, rotation=True, scale=False)
    return (obj, colour)


# --------------------------------------------------------------------- the pieces


def house():
    """The warden's own house: stone footing, timber walls, a pitched roof."""
    eaves, ridge = 3.2, 2.2
    wide, deep = 7.4, 6.0
    parts = [
        # A footing, so it sits IN the ground rather than on it.
        box((wide + 0.4, deep + 0.4, 0.5), (0.0, 0.0, 0.25), "stone"),
        box((wide, deep, eaves), (0.0, 0.0, 0.4 + eaves * 0.5), "wall"),
        wedge(wide + 0.7, deep + 0.7, ridge, (0.0, 0.0, eaves + 0.4), "roof"),
        # Corner posts, which is most of what makes it read as timber-framed.
    ]
    for sx in (-1, 1):
        for sy in (-1, 1):
            parts.append(
                box((0.30, 0.30, eaves), (sx * wide * 0.5, sy * deep * 0.5, 0.4 + eaves * 0.5), "beam")
            )
    parts += [
        # Door on the +Y face, which is the front.
        box((1.1, 0.16, 2.1), (0.0, deep * 0.5 + 0.02, 0.4 + 1.05), "door"),
        box((1.4, 0.10, 0.18), (0.0, deep * 0.5 + 0.06, 0.4 + 2.22), "beam"),
        # A window either side of it, and one on each flank.
        box((1.0, 0.12, 0.9), (-2.3, deep * 0.5 + 0.02, 0.4 + 1.9), "glass"),
        box((1.0, 0.12, 0.9), (2.3, deep * 0.5 + 0.02, 0.4 + 1.9), "glass"),
        box((0.12, 1.0, 0.9), (wide * 0.5 + 0.02, -1.2, 0.4 + 1.9), "glass"),
        # The chimney, off to one side and taller than the ridge.
        box((0.8, 0.8, 2.6), (wide * 0.5 - 1.2, -deep * 0.25, eaves + 1.6), "stone"),
    ]
    return parts, eaves + ridge + 0.4


def barn():
    """Where a monster is stabled: bigger than the house, and plainly a barn."""
    eaves, ridge = 5.0, 2.5
    wide, deep = 11.0, 8.5
    parts = [
        box((wide + 0.5, deep + 0.5, 0.5), (0.0, 0.0, 0.25), "stone"),
        box((wide, deep, eaves), (0.0, 0.0, 0.4 + eaves * 0.5), "barnwall"),
        wedge(wide + 0.9, deep + 0.9, ridge, (0.0, 0.0, eaves + 0.4), "barnroof"),
        # The big doors: two leaves, and a rail over them.
        box((2.4, 0.18, 4.0), (-1.3, deep * 0.5 + 0.02, 0.4 + 2.0), "beam"),
        box((2.4, 0.18, 4.0), (1.3, deep * 0.5 + 0.02, 0.4 + 2.0), "beam"),
        box((5.6, 0.24, 0.24), (0.0, deep * 0.5 + 0.05, 0.4 + 4.2), "metal"),
        # A hay door up in the gable, which is what says barn from a distance.
        box((1.6, 0.16, 1.4), (0.0, deep * 0.5 + 0.05, eaves + 0.9), "beam"),
    ]
    # Cross-braced planking down the long flanks.
    for sx in (-1, 1):
        for step in (-2.6, 0.0, 2.6):
            parts.append(
                box((0.26, 0.26, eaves), (sx * wide * 0.5, step, 0.4 + eaves * 0.5), "beam")
            )
    return parts, eaves + ridge + 0.4


def fence():
    """One four-metre run of post-and-rail, to be repeated round a paddock.

    Authored along X so a row of them lies end to end, and short enough that a
    paddock can turn a corner without a gap showing.
    """
    span, high = 4.0, 1.25
    parts = []
    for x in (-span * 0.5, span * 0.5):
        parts.append(box((0.16, 0.16, high), (x, 0.0, high * 0.5), "beam"))
    for z in (0.45, 0.95):
        parts.append(box((span, 0.10, 0.14), (0.0, 0.0, z), "beam"))
    return parts, high


def gate():
    """The way in: two posts, a crossbeam over them, and a hanging sign board."""
    span, high = 4.6, 3.0
    parts = [
        box((0.34, 0.34, high), (-span * 0.5, 0.0, high * 0.5), "beam"),
        box((0.34, 0.34, high), (span * 0.5, 0.0, high * 0.5), "beam"),
        box((span + 0.5, 0.30, 0.34), (0.0, 0.0, high - 0.17), "beam"),
        box((1.9, 0.12, 0.7), (0.0, 0.0, high - 0.75), "wall"),
    ]
    return parts, high


def trough():
    """A water trough: something for the yard that is not a building."""
    long, wide, high = 2.6, 0.9, 0.62
    parts = [
        box((long, wide, 0.14), (0.0, 0.0, 0.07), "beam"),
        box((long, 0.14, high), (0.0, -wide * 0.5, high * 0.5), "beam"),
        box((long, 0.14, high), (0.0, wide * 0.5, high * 0.5), "beam"),
        box((0.14, wide, high), (-long * 0.5, 0.0, high * 0.5), "beam"),
        box((0.14, wide, high), (long * 0.5, 0.0, high * 0.5), "beam"),
        # The water, just under the rim.
        box((long - 0.3, wide - 0.3, 0.06), (0.0, 0.0, high - 0.12), "water"),
    ]
    return parts, high


def silo():
    """A feed silo, because a yard wants one tall thing that is not a roof."""
    high, radius = 6.4, 1.5
    parts = [
        tube(radius + 0.15, 0.4, (0.0, 0.0, 0.2), "stone"),
        tube(radius, high, (0.0, 0.0, 0.4 + high * 0.5), "metal", sides=14),
    ]
    # A shallow conical cap.
    bpy.ops.mesh.primitive_cone_add(
        vertices=14, radius1=radius + 0.2, radius2=0.1, depth=1.2,
        location=(0.0, 0.0, 0.4 + high + 0.6),
    )
    parts.append((bpy.context.object, "barnroof"))
    # Hoops, so it is not a plain pipe.
    for at in (2.0, 3.6, 5.2):
        parts.append((tube(radius + 0.08, 0.16, (0.0, 0.0, at), "beam", sides=14)[0], "beam"))
    return parts, high + 1.6


BUILDERS = {
    "house": house,
    "barn": barn,
    "fence": fence,
    "gate": gate,
    "trough": trough,
    "silo": silo,
}

# Buildings are flat-faced; only the round things want smoothing.
SHARP_ABOVE = math.radians(50.0)


def build(name: str) -> None:
    fresh()
    parts, tall = BUILDERS[name]()
    for obj, colour in parts:
        bpy.ops.object.select_all(action="DESELECT")
        obj.select_set(True)
        bpy.context.view_layer.objects.active = obj
        bpy.ops.object.transform_apply(location=False, rotation=True, scale=False)
        paint(obj, colour, tall)

    bpy.ops.object.select_all(action="SELECT")
    bpy.context.view_layer.objects.active = parts[0][0]
    if len(parts) > 1:
        bpy.ops.object.join()
    whole = bpy.context.object
    whole.name = "prop"
    whole.data.name = "prop"
    bpy.ops.object.shade_auto_smooth(angle=SHARP_ABOVE)

    low = min((whole.matrix_world @ mathutils.Vector(c)).z for c in whole.bound_box)
    whole.location.z -= low
    bpy.ops.object.transform_apply(location=True, rotation=False, scale=False)

    here = os.path.dirname(os.path.abspath(__file__))
    bpy.ops.wm.save_as_mainfile(filepath=os.path.join(here, f"ranch_{name}.blend"))
    print(f"BUILT ranch_{name}")


for piece in PIECES:
    build(piece)
