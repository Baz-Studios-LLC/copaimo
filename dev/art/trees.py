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
# These are terrain-core's own species names: a file is matched to the tree pool
# by species, so the name is the contract and not a label.
SPECIES = ("oak", "birch", "spruce", "pine", "acacia")


def fresh() -> None:
    bpy.ops.wm.read_factory_settings(use_empty=True)


def trunk(radius, low, high, sides=12, lean=0.0, at=(0.0, 0.0)):
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


def branch_to(start, end, radius):
    """A limb from one point to another, thick end first.

    # Aimed, not angled

    Branches used to be placed by an angle and a length, and they did not reach
    the foliage: from the game camera an oak wore a pair of bare crossed sticks
    under a floating ball of leaves. An angle and a length are two numbers that
    have to be right together, and eyeballing them in a script is guesswork.

    So a branch is given the point it must ARRIVE at — the middle of the clump it
    holds up — and its length and orientation are derived. It ends inside the
    foliage by construction, and moving a clump moves its branch with it.
    """
    span = end - start
    reach = span.length
    if reach < 1.0e-4:
        raise ValueError("a branch has to go somewhere")
    bpy.ops.mesh.primitive_cone_add(
        vertices=6,  # coarse on purpose: this ends up inside the leaves
        radius1=radius,
        radius2=radius * 0.45,
        depth=reach,
        location=start + span * 0.5,
    )
    limb = bpy.context.object
    # A cone is built along +Z; turn that axis onto the span.
    limb.rotation_euler = (
        mathutils.Vector((0.0, 0.0, 1.0))
        .rotation_difference(span.normalized())
        .to_euler()
    )
    return limb


def skirt(radius, deep, z, sides=14):
    """One layer of a conifer — a wide shallow cone."""
    bpy.ops.mesh.primitive_cone_add(
        vertices=sides, radius1=radius, radius2=radius * 0.18, depth=deep,
        location=(0.0, 0.0, z),
    )
    return bpy.context.object


# A clump this big or bigger is worth the extra subdivision.
#
# Detail follows SIZE rather than being one number for everything. An oak's crown
# fills a good part of the screen when you walk under it and its outline wants to
# be round; the three little balls that make a desert bush never read as anything
# but a bush, and paying four times the triangles for them buys nothing. The
# threshold is in metres of radius, so it keeps deciding correctly as species are
# added.
ROUND_ABOVE = 1.3


def clump(radius, at, squash=0.82):
    """One mass of foliage: a ball, flattened a little, round in proportion."""
    bpy.ops.mesh.primitive_ico_sphere_add(
        subdivisions=3 if radius >= ROUND_ABOVE else 2, radius=radius, location=at
    )
    ball = bpy.context.object
    ball.scale = (1.0, 1.0, squash)
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    return ball


def oak():
    """Broad and round: the shape most people draw when they draw a tree."""
    # Where the trunk gives out and the crown starts.
    fork = 4.1
    # Each mass of foliage: middle and radius. The crown is built from these and
    # so are the branches, so the two cannot disagree about where the leaves are.
    crown = [
        ((0.0, 0.0, 6.4), 2.55),
        ((1.75, 0.45, 5.5), 1.75),
        ((-1.55, -0.70, 5.75), 1.62),
        ((0.30, 1.45, 7.35), 1.40),
    ]
    wood = [trunk(0.44, 0.0, fork + 0.5, sides=12)]
    leaves = [clump(radius, at) for at, radius in crown]
    # A limb from the fork into the middle of every outlying mass. Into the
    # MIDDLE, so the end of the branch is swallowed by the foliage rather than
    # stopping at its edge where a gap would show.
    start = mathutils.Vector((0.0, 0.0, fork))
    for at, _ in crown[1:]:
        wood.append(branch_to(start, mathutils.Vector(at), 0.15))
    return wood, leaves


def pine():
    """Tall, straight, layered — bare stem for the first third of its height."""
    # Stopping BELOW the apex of the top layer. Run the stem to the tree's full
    # height and it stands proud of the crown as a bare spike, which is the one
    # thing that made these read as geometry rather than as trees.
    wood = [trunk(0.34, 0.0, 12.2, sides=12)]
    leaves = [
        skirt(3.30, 3.1, 5.4),
        skirt(2.70, 2.9, 7.7),
        skirt(2.00, 2.7, 9.9),
        skirt(1.20, 2.4, 11.9),
    ]
    return wood, leaves


def birch():
    """Slim and pale, leaning a little, with a deep crown.

    # A tree is a proportion, not a height

    This forked at 7.4 m on a 10.3 m tree, so nearly three quarters of it was bare
    stem under one small ball of leaves. In the game — where the trunk is chalk
    pale, because that is what makes a birch a birch — it read as a lamp post with
    a shrub balanced on top.

    The height was never the problem. What was wrong is the SHARE of the tree that
    is crown: a quarter reads as a pole, and about half reads as a tree. So the
    fork came down to 4.9 m and the crown grew into five masses that reach below
    the fork, which is also how a birch actually looks — foliage well down the
    stem rather than a cap on the end of it.
    """
    fork = 4.9
    crown = [
        ((0.10, 0.00, 7.00), 2.00),
        ((1.30, 0.35, 6.15), 1.40),
        ((-1.20, -0.45, 6.35), 1.30),
        ((0.20, 1.05, 8.00), 1.15),
        ((-0.35, 0.55, 5.50), 1.05),
    ]
    wood = [trunk(0.24, 0.0, fork + 0.7, sides=10, lean=math.radians(2.5))]
    leaves = [clump(radius, at, squash=0.9) for at, radius in crown]
    start = mathutils.Vector((0.0, 0.0, fork))
    for at, _ in crown[1:]:
        wood.append(branch_to(start, mathutils.Vector(at), 0.085))
    return wood, leaves


def spruce():
    """The tallest thing in the wood, and the narrowest for its height."""
    # Again: under the apex of the topmost layer, never through it.
    wood = [trunk(0.30, 0.0, 13.6, sides=12)]
    leaves = [skirt(2.5 - i * 0.32, 2.5, 3.0 + i * 2.1, sides=14) for i in range(6)]
    return wood, leaves


def acacia():
    """Flat-topped and open: all shade and no height, for dry country.

    An umbrella, because shade is the scarce thing where an acacia grows — a bare
    trunk that forks low and wide, and a crown that is broad and SHALLOW. It is
    the one silhouette here that is wider than it is tall, which is what makes it
    read as dry country from a long way off.
    """
    fork = 2.9
    crown = [
        ((0.00, 0.00, 5.05), 2.30),
        ((2.35, 0.35, 4.65), 1.70),
        ((-2.20, -0.40, 4.75), 1.60),
        ((0.30, 2.05, 4.70), 1.45),
        ((-0.40, -2.00, 4.60), 1.35),
    ]
    wood = [trunk(0.38, 0.0, fork + 0.5, sides=10)]
    leaves = [clump(radius, at, squash=0.42) for at, radius in crown]
    start = mathutils.Vector((0.0, 0.0, fork))
    for at, _ in crown[1:]:
        wood.append(branch_to(start, mathutils.Vector(at), 0.13))
    return wood, leaves


BUILDERS = {
    "oak": oak,
    "birch": birch,
    "spruce": spruce,
    "pine": pine,
    "acacia": acacia,
}


# Above this angle between two faces, the edge between them stays SHARP.
#
# Sixty degrees smooths everything round — an eight-sided trunk turns 45 degrees a
# face and a coarse ball far less — while leaving the corners that should read as
# corners: the rim of a conifer layer turns a right angle or more.
#
# Flat shading everywhere was the first cut, and in the game every facet read as
# its own panel: a canopy came out as a heap of triangles rather than a mass of
# leaves. Smoothing the whole object instead would have rounded the layer rims off
# a spruce, which is the one thing that makes a spruce look like a spruce.
SHARP_ABOVE = math.radians(60.0)


def weld(parts, name):
    """Joins parts into one object under a known name, smoothly shaded.

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
    bpy.ops.object.shade_auto_smooth(angle=SHARP_ABOVE)
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
