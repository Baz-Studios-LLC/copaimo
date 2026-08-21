"""Builds the world's tree species as low-poly figures, one `.blend` each.

    dev/art/build.sh

# Why these fit the game without rearranging it

A tree in this world is already a POOL entry: `stream.rs` grows a handful of
varieties once, and every tree in every chunk is an instance of one of them with
its own position, turn and scale. A variety is two meshes — `wood` and `leaves` —
because bark and foliage wear different materials and one mesh can only wear one.

So an authored tree is exactly two objects named `wood` and `leaves`, and it drops
into the slot the generated one occupied. Placement, streaming, the shadow ring
and the per-tree scale all stay as they are.

**No vertex colours, on purpose.** Trees are tinted by the material their variety
wears — that is how a wood comes out in twenty different greens rather than one
flat one — so colour in the mesh would fight it. Ground cover is the opposite case
and carries its colour in its vertices. See `as_coloured_mesh`.

# The species

Each is built for its SILHOUETTE, because that is all you see of a tree at the
distance the game draws it: broad and round, tall and layered, slim and sparse.
Heights are real metres against a 1.8 m warden.
"""

import math
import os

import bpy
import mathutils

# Each species: how tall it stands, in metres, against a 1.8 m person.
SPECIES = ("oak", "pine", "birch", "spruce", "scrub")


def fresh() -> None:
    bpy.ops.wm.read_factory_settings(use_empty=True)


def trunk(radius, low, high, sides=8, lean=0.0, at=(0.0, 0.0)):
    """A tapered stem. Real trees are thicker at the foot, and it reads."""
    deep = high - low
    bpy.ops.mesh.primitive_cone_add(
        vertices=sides,
        radius1=radius,
        radius2=radius * 0.62,
        depth=deep,
        location=(at[0], at[1], low + deep / 2),
    )
    stem = bpy.context.object
    if lean:
        stem.rotation_euler = (lean, 0.0, 0.0)
    return stem


def limb(radius, length, at, pitch, spin):
    """One branch, angled out and up from a point on the stem."""
    bpy.ops.mesh.primitive_cone_add(
        vertices=6, radius1=radius, radius2=radius * 0.5, depth=length, location=at
    )
    branch = bpy.context.object
    branch.rotation_euler = (pitch, 0.0, spin)
    return branch


def skirt(radius, deep, z, sides=9):
    """One layer of a conifer — a wide shallow cone."""
    bpy.ops.mesh.primitive_cone_add(
        vertices=sides, radius1=radius, radius2=radius * 0.18, depth=deep,
        location=(0.0, 0.0, z),
    )
    return bpy.context.object


def clump(radius, at, squash=0.82):
    """One mass of foliage: a coarse ball, flattened a little."""
    bpy.ops.mesh.primitive_ico_sphere_add(subdivisions=2, radius=radius, location=at)
    ball = bpy.context.object
    ball.scale = (1.0, 1.0, squash)
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    return ball


def oak():
    """Broad and round: the shape most people draw when they draw a tree."""
    wood = [trunk(0.42, 0.0, 3.6, sides=8)]
    wood.append(limb(0.16, 2.2, (0.6, 0.0, 3.2), math.radians(58), 0.0))
    wood.append(limb(0.14, 2.0, (-0.5, 0.4, 3.4), math.radians(-52), math.radians(30)))
    leaves = [
        clump(2.75, (0.0, 0.0, 6.1)),
        clump(1.85, (1.7, 0.5, 5.0)),
        clump(1.70, (-1.5, -0.7, 5.3)),
        clump(1.55, (0.3, 1.4, 7.3)),
    ]
    return wood, leaves


def pine():
    """Tall, straight, layered — bare stem for the first third of its height."""
    # Stopping BELOW the apex of the top layer. Run the stem to the tree's full
    # height and it stands proud of the crown as a bare spike, which is the one
    # thing that made these read as geometry rather than as trees.
    wood = [trunk(0.34, 0.0, 12.2, sides=8)]
    leaves = [
        skirt(3.30, 3.1, 5.4),
        skirt(2.70, 2.9, 7.7),
        skirt(2.00, 2.7, 9.9),
        skirt(1.20, 2.4, 11.9),
    ]
    return wood, leaves


def birch():
    """Slim and sparse, leaning a little — the pale one in a wood."""
    wood = [trunk(0.20, 0.0, 9.6, sides=7, lean=math.radians(2.5))]
    wood.append(limb(0.08, 1.1, (0.22, 0.0, 7.6), math.radians(58), 0.0))
    leaves = [
        clump(1.45, (0.25, 0.1, 9.0), squash=0.9),
        clump(1.15, (-0.7, 0.4, 7.8), squash=0.9),
        clump(1.00, (0.8, -0.5, 8.2), squash=0.9),
    ]
    return wood, leaves


def spruce():
    """The tallest thing in the wood, and the narrowest for its height."""
    # Again: under the apex of the topmost layer, never through it.
    wood = [trunk(0.30, 0.0, 13.6, sides=8)]
    leaves = [skirt(2.5 - i * 0.32, 2.5, 3.0 + i * 2.1, sides=8) for i in range(6)]
    return wood, leaves


def scrub():
    """Barely a tree: a low wide mass for dry ground, with a stub of a stem."""
    wood = [trunk(0.16, 0.0, 0.7, sides=6)]
    leaves = [
        clump(1.05, (0.0, 0.0, 1.15), squash=0.62),
        clump(0.80, (0.75, 0.35, 0.95), squash=0.6),
        clump(0.72, (-0.65, -0.45, 1.0), squash=0.6),
    ]
    return wood, leaves


BUILDERS = {
    "oak": oak,
    "pine": pine,
    "birch": birch,
    "spruce": spruce,
    "scrub": scrub,
}


def weld(parts, name):
    """Joins parts into one flat-shaded object under a known name.

    The NAME is the contract: the game looks for `wood` and `leaves` by name when
    it reads the file, because a tree wears two materials and has to know which
    half is which. Anything else in the file would be silently dropped.
    """
    bpy.ops.object.select_all(action="DESELECT")
    for part in parts:
        part.select_set(True)
    bpy.context.view_layer.objects.active = parts[0]
    if len(parts) > 1:
        bpy.ops.object.join()
    whole = bpy.context.object
    whole.name = name
    whole.data.name = name
    bpy.ops.object.shade_flat()
    return whole


def sit_on_the_floor(objects) -> None:
    """Drops the whole tree so its lowest point is Z=0, which the gate insists on."""
    low = min(
        (obj.matrix_world @ mathutils.Vector(corner)).z
        for obj in objects
        for corner in obj.bound_box
    )
    for obj in objects:
        obj.location.z -= low
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.transform_apply(location=True, rotation=False, scale=False)


def build(name: str) -> None:
    fresh()
    wood_parts, leaf_parts = BUILDERS[name]()
    # Applied before welding: a cone carries its angle as a rotation, and joining
    # objects with live rotations bakes them in a way that moves the geometry.
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.transform_apply(location=False, rotation=True, scale=False)

    wood = weld(wood_parts, "wood")
    leaves = weld(leaf_parts, "leaves")
    sit_on_the_floor([wood, leaves])

    here = os.path.dirname(os.path.abspath(__file__))
    out = os.path.join(here, f"tree_{name}.blend")
    bpy.ops.wm.save_as_mainfile(filepath=out)
    tall = max(
        (obj.matrix_world @ mathutils.Vector(corner)).z
        for obj in (wood, leaves)
        for corner in obj.bound_box
    )
    print(f"BUILT tree_{name} — {tall:.1f} m")


for species in SPECIES:
    build(species)
