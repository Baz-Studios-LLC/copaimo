"""The pieces every scripted figure in this folder is built out of.

# Why this exists

`ranch.py` and `props.py` had each grown their own `to_linear` and `paint`, letter
for letter, and `people.py` its own `box`. Three copies of a colour conversion is
three places for a colour conversion to be wrong in, and the one that is wrong is
always the one nobody edited.

So the primitives live here and the figures import them. What is NOT here is
anything a figure decides for itself — its palette, its proportions, its shapes.
This is the mortar, not the building.

# A colour attribute is LINEAR

Every palette in this folder is written in sRGB because that is what a person can
read off a colour picker, and every one of them has to be converted on the way into
a vertex colour. That conversion is `to_linear`, and getting it wrong does not
error: it just makes everything slightly too bright, everywhere, forever.
"""

import os

import bpy
import mathutils

# How much darker the foot of a thing is than its top.
#
# Gentler than the ground litter's, because a building is tall and a strong ramp
# reads as a spotlight on the roof rather than as shade at the bottom of a wall.
FOOT_SHADE = 0.74

# Above this angle a join is drawn as an edge rather than smoothed over.
SHARP_ABOVE = 0.5236  # 30 degrees


def to_linear(part: float) -> float:
    """One sRGB channel into linear, which is what a colour attribute holds."""
    if part <= 0.04045:
        return part / 12.92
    return ((part + 0.055) / 1.055) ** 2.4


def fresh() -> None:
    """An empty file. Every figure starts from one."""
    bpy.ops.wm.read_factory_settings(use_empty=True)


def paint(obj, rgb, tall: float, floor: float = 0.0) -> None:
    """Vertex-colours one object, darker at its foot.

    `rgb` is sRGB. `tall` is what counts as the top of the thing, so the ramp is
    the figure's own rather than each piece's — a doorstep and a chimney belong to
    one building and have to shade as one.

    `floor` is where the bottom of the ramp sits, which matters indoors: a room's
    floor is not at z=0 of the building it is in, and shading it from the building's
    foot would leave an upstairs room lit as though it were a roof.
    """
    mesh = obj.data
    linear = [to_linear(part) for part in rgb]
    if not mesh.color_attributes:
        mesh.color_attributes.new(name="Color", type="FLOAT_COLOR", domain="POINT")
    layer = mesh.color_attributes["Color"]
    span = max(tall - floor, 1.0e-4)
    for point in mesh.vertices:
        up = min(1.0, max(0.0, (point.co.z - floor) / span))
        shade = FOOT_SHADE + (1.0 - FOOT_SHADE) * up
        layer.data[point.index].color = (
            linear[0] * shade,
            linear[1] * shade,
            linear[2] * shade,
            1.0,
        )


def box(size, at, colour, tilt=None):
    """One box, given as full extents in metres and the middle it sits at."""
    bpy.ops.mesh.primitive_cube_add(size=1.0, location=at)
    obj = bpy.context.object
    obj.scale = size
    if tilt:
        obj.rotation_euler = tilt
    bpy.ops.object.transform_apply(location=False, rotation=True, scale=True)
    return (obj, colour)


def wedge(span, deep, high, at, colour, ridge="y"):
    """A pitched roof: a prism with its ridge down the middle.

    `ridge` says which way the ridge runs, and it is the single most important
    thing about a roof's silhouette: a ridge along Y shows its GABLE to anyone
    standing to the south, and a ridge along X shows its EAVES. A street of
    buildings that all face the same way is a terrace; one that alternates is a
    village.
    """
    half, back = span * 0.5, deep * 0.5
    if ridge == "y":
        places = [
            (-half, -back, 0.0), (half, -back, 0.0), (half, back, 0.0), (-half, back, 0.0),
            (0.0, -back, high), (0.0, back, high),
        ]
    else:
        places = [
            (-half, -back, 0.0), (half, -back, 0.0), (half, back, 0.0), (-half, back, 0.0),
            (-half, 0.0, high), (half, 0.0, high),
        ]
    faces = [(0, 1, 2, 3), (0, 4, 5, 3), (1, 2, 5, 4), (0, 1, 4), (3, 2, 5)]
    if ridge != "y":
        faces = [(0, 1, 2, 3), (0, 1, 5, 4), (3, 2, 5, 4), (0, 3, 4), (1, 2, 5)]
    return _from_points("roof", places, faces, at, colour)


def gable_wall(span, high, at, thick, colour, facing="y"):
    """The triangle of wall that fills the end of a pitched roof.

    Without it a gable roof is a lid resting on a box with daylight under both
    slopes, which is the commonest way a first attempt at a roof goes wrong.
    """
    half = span * 0.5
    t = thick * 0.5
    if facing == "y":
        places = [
            (-half, -t, 0.0), (half, -t, 0.0), (0.0, -t, high),
            (-half, t, 0.0), (half, t, 0.0), (0.0, t, high),
        ]
    else:
        places = [
            (-t, -half, 0.0), (-t, half, 0.0), (-t, 0.0, high),
            (t, -half, 0.0), (t, half, 0.0), (t, 0.0, high),
        ]
    faces = [(0, 1, 2), (3, 4, 5), (0, 1, 4, 3), (1, 2, 5, 4), (0, 2, 5, 3)]
    return _from_points("gable", places, faces, at, colour)


def lean(span, deep, high, at, colour, drops_to=0.0):
    """A single-pitch roof, high along one edge and low along the other.

    What a lean-to and a shopfront awning both are, and what a row of houses wants
    when its roofs are not all facing the same way.
    """
    half, back = span * 0.5, deep * 0.5
    places = [
        (-half, -back, 0.0), (half, -back, 0.0), (half, back, 0.0), (-half, back, 0.0),
        (-half, -back, high), (half, -back, high),
        (half, back, drops_to), (-half, back, drops_to),
    ]
    faces = [
        (0, 1, 2, 3), (4, 5, 6, 7), (0, 1, 5, 4),
        (3, 2, 6, 7), (0, 3, 7, 4), (1, 2, 6, 5),
    ]
    return _from_points("lean", places, faces, at, colour)


def tube(radius, deep, at, colour, sides=12, tilt=None):
    bpy.ops.mesh.primitive_cylinder_add(vertices=sides, radius=radius, depth=deep, location=at)
    obj = bpy.context.object
    if tilt:
        obj.rotation_euler = tilt
        bpy.ops.object.transform_apply(location=False, rotation=True, scale=False)
    return (obj, colour)


def _from_points(name, places, faces, at, colour):
    mesh = bpy.data.meshes.new(name)
    mesh.from_pydata([(x + at[0], y + at[1], z + at[2]) for x, y, z in places], [], faces)
    mesh.update()
    obj = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(obj)
    return (obj, colour)


def weld(parts, palette, tall, name="prop", floor_of=None):
    """Paints every piece, joins them into one object, and stands it on the floor.

    One object because a placed thing is carried whole and spawned as a scene, and
    one material for the lot — the colour is in the vertices, so a figure can be
    recoloured by editing a table rather than by opening a material editor.

    `floor_of` is asked for each piece's colour name and may return the height its
    shading ramp should start from. Indoors that is the storey the piece stands on;
    everywhere else it is the ground.
    """
    for obj, colour in parts:
        bpy.ops.object.select_all(action="DESELECT")
        obj.select_set(True)
        bpy.context.view_layer.objects.active = obj
        bpy.ops.object.transform_apply(location=False, rotation=True, scale=False)
        floor = floor_of(colour) if floor_of else 0.0
        paint(obj, palette[colour], tall, floor)

    bpy.ops.object.select_all(action="SELECT")
    bpy.context.view_layer.objects.active = parts[0][0]
    if len(parts) > 1:
        bpy.ops.object.join()
    whole = bpy.context.object
    whole.name = name
    whole.data.name = name
    bpy.ops.object.shade_auto_smooth(angle=SHARP_ABOVE)

    low = min((whole.matrix_world @ mathutils.Vector(c)).z for c in whole.bound_box)
    whole.location.z -= low
    bpy.ops.object.transform_apply(location=True, rotation=False, scale=False)
    return whole


def save_beside(filename: str) -> str:
    """Saves the current file next to these scripts, and says where it went."""
    here = os.path.dirname(os.path.abspath(__file__))
    path = os.path.join(here, filename)
    bpy.ops.wm.save_as_mainfile(filepath=path)
    return path
