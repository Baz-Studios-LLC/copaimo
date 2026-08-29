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


# The one pot every scripted figure in this folder is painted from.
#
# Shared rather than per-file: a town's buildings and the yards between them are the
# same place, and two palettes drift the moment one of them is edited.
PALETTE = {
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
    """Builds one piece from points and faces, with its normals pointing OUT.

    # Every hand-wound face in this file was wrong

    `from_pydata` takes the winding it is given, and the winding decides which way a
    face looks. A face wound the wrong way is not merely lit oddly - it is CULLED,
    so you see straight through it, and a roof made of them is a building with no
    roof on it. That is exactly what shipped: audited afterwards, `wedge` had two of
    its five faces inside out on one ridge axis and three on the other, `gable_wall`
    three and two, and `lean` three of six.

    Getting each list right by hand is possible and it is not worth doing: it has to
    be got right again every time a shape is added, the mistake is invisible in a
    wireframe, and nothing about the code says which order is correct. Blender can
    work it out from the geometry, so it does - and a piece cannot be built wrong.
    """
    mesh = bpy.data.meshes.new(name)
    mesh.from_pydata([(x + at[0], y + at[1], z + at[2]) for x, y, z in places], [], faces)
    mesh.update()
    obj = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(obj)

    bpy.context.view_layer.objects.active = obj
    obj.select_set(True)
    bpy.ops.object.mode_set(mode="EDIT")
    bpy.ops.mesh.select_all(action="SELECT")
    bpy.ops.mesh.normals_make_consistent(inside=False)
    bpy.ops.object.mode_set(mode="OBJECT")
    obj.select_set(False)
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


# How thick an outline is, in metres.
#
# 5 cm reads at street level and disappears by the time a building is a shape on the
# skyline, which is what you want - an outline is there to stop a thing dissolving
# into what is behind it, not to draw a cartoon.
# 3 cm, and CLAMPED against the thing it wraps.
#
# # An outline has to be thin next to the thinnest thing it wraps
#
# 9 cm read well on a 10 m building and was a disaster on everything else: a fence
# rail is 12 cm through, so a 9 cm shell is three quarters as thick again as the rail
# and stops being a line around it - it becomes a black slab beside it. Photographed,
# the ranch's fences and the silo's hoops were fringed with black blocks.
#
# The shell is pushed out by the SAME distance everywhere on a mesh, so that distance
# has to suit the mesh's smallest feature rather than its largest. Three centimetres
# suits a fence rail and still reads on a cottage, and the clamp below keeps anything
# smaller than that in proportion too.
OUTLINE_THICK = 0.07

# Never more than this share of the smallest thing in the figure.
OUTLINE_AT_MOST = 0.16

# What colour it is. Near-black with a little of the world's blue in it, because a
# pure black line under a blue sky reads as a hole rather than as a shadow.
OUTLINE_INK = (0.05, 0.055, 0.07)


def outline(whole, thick=OUTLINE_THICK, ink=OUTLINE_INK):
    """Wraps a finished figure in an inverted hull, so it has an edge.

    # Why a hull and not a shader

    "Interiors and exteriors need edges so they dont blend into the background."

    The classic stylised answer, and the one that needs nothing from the engine: copy
    the mesh, push every vertex out along its own normal, turn the faces inside out,
    and paint it near-black. Back-face culling then hides the near side of that shell
    completely and leaves only the far side visible - which, from any angle, is
    exactly the silhouette. Guilty Gear and Zelda both use a version of it.

    Pushed along the NORMAL rather than scaled about the middle: scaling a tall
    building would leave a thick line at its eaves and none at its foot, because the
    two are different distances from the centre.

    It rides along with the model - same mesh, same vertex colours, same material -
    so nothing in the game has to know it exists. The one thing it does need is for
    the model's material to CULL back faces, or the shell is drawn over the building
    and the building is a black box.
    """
    import bmesh

    # Held down to a share of the figure's own smallest dimension, so a fence panel
    # gets a fence panel's outline rather than a building's.
    span = whole.dimensions
    smallest = min(v for v in (span.x, span.y, span.z) if v > 1.0e-4)
    thick = min(thick, smallest * OUTLINE_AT_MOST)

    bpy.ops.object.select_all(action="DESELECT")
    whole.select_set(True)
    bpy.context.view_layer.objects.active = whole
    bpy.ops.object.duplicate()
    shell = bpy.context.object
    shell.name = whole.name + "_outline"

    mesh = bmesh.new()
    mesh.from_mesh(shell.data)
    mesh.verts.ensure_lookup_table()

    # # Along the AVERAGED normal, and the edges are NOT split first
    #
    # Splitting them was the first attempt and it is what put black slabs on every
    # building and every fence: split, each face moves out along its own normal and
    # the six faces of a box stop meeting at the corners - so the shell is not a
    # shell at all, it is a loose pile of panels with the gaps between them showing.
    #
    # A closed hull needs the shared vertex, pushed along the average of the faces
    # that meet there. At a box corner that average points diagonally outward, which
    # inflates the box evenly - exactly what is wanted. `normal_update` computes that
    # average from the geometry, whatever split normals the shading has put on top.
    mesh.normal_update()
    for vertex in mesh.verts:
        vertex.co += vertex.normal * thick
    bmesh.ops.reverse_faces(mesh, faces=mesh.faces)
    mesh.to_mesh(shell.data)
    mesh.free()
    shell.data.update()

    layer = shell.data.color_attributes[0] if shell.data.color_attributes else None
    if layer is None:
        layer = shell.data.color_attributes.new(
            name="Color", type="FLOAT_COLOR", domain="POINT"
        )
    rgb = [to_linear(part) for part in ink]
    for point in shell.data.vertices:
        layer.data[point.index].color = (rgb[0], rgb[1], rgb[2], 1.0)

    bpy.ops.object.select_all(action="DESELECT")
    shell.select_set(True)
    whole.select_set(True)
    bpy.context.view_layer.objects.active = whole
    bpy.ops.object.join()
    joined = bpy.context.object

    # And back onto the floor. The shell is pushed out along every normal, including
    # the ones pointing down, so a figure that was standing exactly on Z=0 now has
    # five centimetres of outline below it - which the export gate refuses, rightly,
    # as a model that would import half-buried.
    low = min((joined.matrix_world @ mathutils.Vector(c)).z for c in joined.bound_box)
    joined.location.z -= low
    bpy.ops.object.transform_apply(location=True, rotation=False, scale=False)
    return joined


def save_beside(filename: str) -> str:
    """Saves the current file next to these scripts, and says where it went."""
    here = os.path.dirname(os.path.abspath(__file__))
    path = os.path.join(here, filename)
    bpy.ops.wm.save_as_mainfile(filepath=path)
    return path
