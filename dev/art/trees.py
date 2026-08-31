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
# fills a good part of the screen when you walk under it; the three little balls
# that make a desert bush never read as anything but a bush, and paying four times
# the triangles for them buys nothing. The threshold is in metres of radius, so it
# keeps deciding correctly as species are added.
ROUND_ABOVE = 1.3

# How far a leaf mass is pushed in and out of round, as a share of its radius.
#
# # A perfectly round crown is a lollipop
#
# The broadleaf species were built from smooth ico-spheres, and a few of those
# overlapping make ONE convex mass with a smooth outline - which is precisely the
# thing everybody draws when they draw a tree badly, and it is what the game's
# trees were reported as. The outline pass made it plainer: ink traces whatever
# curve is there, and what was there was a circle.
#
# What a real crown has, and what every stylised tree that reads well has, is a
# BROKEN outline: lobes that stick out, bites that cut in, and gaps you can see
# sky through. So every vertex is pushed along its own direction by a repeatable
# amount, which turns a ball into a lump. A third is enough to break the curve
# and little enough that a lump is still a lump.
LEAVES_WANDER = 0.34

# How coarse a leaf mass is.
#
# Two subdivisions is eighty faces, which at the size these are drawn is a facet
# per two or three pixels: enough to catch the light in planes rather than as a
# gradient, which is what makes foliage read as leaves in this style rather than
# as painted plastic. It is also a QUARTER of the triangles the round version
# cost, and a wood is hundreds of instances.
LEAVES_FACETS = 2


def wobble(seed, salt):
    """A repeatable number in 0..1, so a tree built twice is the same tree."""
    value = math.sin(seed * 12.9898 + salt * 78.233) * 43758.5453
    return value - math.floor(value)


def clump(radius, at, squash=0.82, seed=0, rough=LEAVES_WANDER):
    """One mass of foliage: a lump, flattened a little, out of round on purpose.

    See `LEAVES_WANDER`. `seed` makes each mass its own shape while keeping the
    whole tree repeatable - two builds of the same species are the same file.
    """
    bpy.ops.mesh.primitive_ico_sphere_add(
        subdivisions=LEAVES_FACETS, radius=radius, location=at
    )
    ball = bpy.context.object
    # Pushed along its own direction from the middle, so the mass keeps its centre
    # and only its OUTLINE changes. Scaling would move the whole thing.
    for at_vertex, vertex in enumerate(ball.data.vertices):
        out = vertex.co.normalized()
        much = 1.0 - rough * 0.5 + rough * wobble(seed + 1, at_vertex + 1)
        vertex.co = out * (radius * much)
    ball.scale = (1.0, 1.0, squash)
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    return ball


def oak():
    """Broad and heavy, and NOT round.

    # What was wrong with it

    Four big overlapping balls on a bare stem, which merge into one smooth convex
    mass: a lollipop, and reported as one. The masses are smaller and there are
    more of them now, spread wider and set at heights that differ by more than
    their own radii - so the outline has lobes and bites in it instead of a curve,
    and there is sky between them. Two of them hang BELOW the fork, which is what
    stops the crown reading as a cap balanced on a pole.
    """
    # Where the trunk gives out and the crown starts.
    fork = 3.6
    # Each mass of foliage: middle and radius. The crown is built from these and
    # so are the branches, so the two cannot disagree about where the leaves are.
    crown = [
        ((0.10, -0.10, 6.35), 2.05),
        ((1.95, 0.55, 5.65), 1.55),
        ((-1.80, -0.60, 5.95), 1.45),
        ((0.35, 1.85, 6.60), 1.35),
        ((-0.55, -1.90, 6.25), 1.25),
        ((1.15, -1.05, 7.45), 1.20),
        ((-1.25, 1.10, 7.30), 1.10),
        # The two that come down the SIDES of the stem, not below it.
        #
        # Set out at 1.55 m and down at 4.35 they hung clear of everything else and
        # read as fruit - a chain of separate balls under the crown. Tucked in and
        # raised until they overlap the masses above, they do the job they are for:
        # a lower edge with lobes in it instead of a line where the crown stops.
        ((1.20, 0.70, 5.05), 1.15),
        ((-1.05, -0.80, 5.20), 1.05),
    ]
    wood = [trunk(0.44, 0.0, fork + 0.6, sides=12)]
    leaves = [clump(radius, at, seed=at_leaf) for at_leaf, (at, radius) in enumerate(crown)]
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
    # # It was still a lollipop, and the silhouette sheet said so
    #
    # Measured on `dev/art/shots/trees_silhouette.png`: the crown was a compact
    # ball three metres across sitting on four and a half metres of clean stem, so
    # even at half the tree's height it read as a ball on a pole. What a birch
    # actually is, and what fixes the read, is AIRY and WIDE for its weight -
    # foliage hung in loose sprays that reach down beside the stem, not a cap on
    # the end of it. So the fork comes down again, the masses reach half a metre
    # further out, and four of the nine now sit below where the old crown started.
    fork = 3.8
    crown = [
        ((0.10, 0.00, 7.00), 1.40),
        ((1.75, 0.45, 6.20), 1.15),
        ((-1.70, -0.55, 6.45), 1.10),
        ((0.30, 1.60, 7.70), 1.00),
        ((1.30, -1.35, 7.30), 0.95),
        ((-1.45, 1.30, 7.00), 0.90),
        # The sprays down the stem, which is what a birch has and a lollipop does not.
        ((-0.80, 0.70, 5.20), 0.95),
        ((1.15, -0.60, 4.85), 0.90),
        ((-1.10, -0.90, 5.60), 0.85),
    ]
    wood = [trunk(0.24, 0.0, fork + 0.7, sides=10, lean=math.radians(2.5))]
    # Squashed less than an oak's and roughened more: a birch's leaves hang in
    # loose sprays rather than in the solid masses an oak carries.
    leaves = [
        clump(radius, at, squash=0.94, seed=20 + at_leaf, rough=LEAVES_WANDER * 1.25)
        for at_leaf, (at, radius) in enumerate(crown)
    ]
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
        ((0.00, 0.00, 5.05), 1.85),
        ((2.45, 0.40, 4.70), 1.45),
        ((-2.30, -0.45, 4.80), 1.35),
        ((0.35, 2.15, 4.75), 1.25),
        ((-0.45, -2.10, 4.65), 1.20),
        ((1.55, -1.60, 4.95), 1.10),
        ((-1.60, 1.50, 4.85), 1.05),
    ]
    wood = [trunk(0.38, 0.0, fork + 0.5, sides=10)]
    # Flattened hard, which is what makes an umbrella an umbrella, and roughened
    # least: a flat crown seen edge-on is mostly outline, and too much wander
    # there reads as a torn edge rather than as a canopy.
    leaves = [
        clump(radius, at, squash=0.42, seed=40 + at_leaf, rough=LEAVES_WANDER * 0.7)
        for at_leaf, (at, radius) in enumerate(crown)
    ]
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
