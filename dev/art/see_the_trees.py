"""Renders every tree species side by side, lit and as a flat silhouette.

    dev/art/see_the_trees.sh

# Why a silhouette sheet and not a screenshot

A tree in this game is seen from twenty metres and read in about a tenth of a
second, so what it IS to a player is its outline. That is also the one thing a
screenshot of a wood cannot show you: the trees overlap, the grass crosses them,
the ink pass draws round whatever is in front, and picking out which species you
are objecting to is guesswork - which is exactly how four passes were once spent
on a pair of shoes that were already right.

Flat black on white is the oldest test in character design and the cheapest. If
the shape does not read at thumbnail size in solid black, nothing else will save
it. `render_clay.py` says the same thing about the warden; this is that
instrument pointed at the wood.

Both are drawn because they answer different questions. The silhouette says
whether the outline is broken or a lollipop; the lit render says whether the
facets catch light as leaves or as plastic.
"""

import os
import sys

import bpy
import mathutils

ART = os.path.dirname(os.path.abspath(__file__))
SHOTS = os.path.join(ART, "shots")

SPECIES = ("oak", "birch", "spruce", "pine", "acacia")

# How far apart the species stand, in metres.
#
# Wider than the widest crown by half again, so no two of them touch and every
# outline is its own. An acacia is 7.6 m across, which is the one that sets this.
APART = 11.0

# Where the camera stands, in metres: back, and at about a warden's eye height
# looking slightly up, which is how the player sees a tree.
BACK = 46.0
EYE = 4.0


def fresh() -> None:
    bpy.ops.wm.read_factory_settings(use_empty=True)


def bring_in(species: str, at_x: float):
    """Appends one species' `wood` and `leaves` and stands them at `at_x`."""
    blend = os.path.join(ART, f"tree_{species}.blend")
    with bpy.data.libraries.load(blend, link=False) as (source, into):
        into.objects = [name for name in source.objects if name in ("wood", "leaves")]
    brought = []
    for obj in into.objects:
        if obj is None:
            continue
        bpy.context.collection.objects.link(obj)
        obj.location.x += at_x
        brought.append(obj)
    return brought


def flat(colour, name, takes_light=False):
    """A plain colour, emissive for a silhouette and DIFFUSE for a lit render.

    The first version of this made both of them emissive, so the lit sheet came
    out as flat as the silhouette one and answered the same question twice - and
    the question it was built to answer is whether the facets catch light as
    leaves or as plastic, which needs a surface that takes light.
    """
    material = bpy.data.materials.new(name)
    material.use_nodes = True
    nodes = material.node_tree.nodes
    nodes.clear()
    out = nodes.new("ShaderNodeOutputMaterial")
    shade = nodes.new("ShaderNodeBsdfDiffuse" if takes_light else "ShaderNodeEmission")
    shade.inputs["Color"].default_value = (*colour, 1.0)
    material.node_tree.links.new(shade.outputs[0], out.inputs["Surface"])
    return material


def paint(objects, material):
    for obj in objects:
        obj.data.materials.clear()
        obj.data.materials.append(material)


def render(into: str, silhouette: bool, standing):
    scene = bpy.context.scene
    scene.render.engine = "BLENDER_EEVEE"
    scene.render.resolution_x = 2200
    scene.render.resolution_y = 900
    scene.render.film_transparent = False
    scene.world = bpy.data.worlds.new("sky") if scene.world is None else scene.world
    scene.world.use_nodes = True
    ground = scene.world.node_tree.nodes.get("Background")
    if ground is not None:
        ground.inputs[0].default_value = (1.0, 1.0, 1.0, 1.0) if silhouette else (0.62, 0.74, 0.86, 1.0)
        ground.inputs[1].default_value = 1.0 if silhouette else 0.9

    if silhouette:
        ink = flat((0.0, 0.0, 0.0), "ink")
        for _, wood, leaves in standing:
            paint(wood, ink)
            paint(leaves, ink)
    else:
        bark = flat((0.29, 0.22, 0.17), "bark", takes_light=True)
        leaf = flat((0.30, 0.52, 0.24), "leaf", takes_light=True)
        # Lit rather than emissive, so the FACETS show. A flat colour would answer
        # the silhouette question twice and the lighting question not at all.
        for _, wood, leaves in standing:
            paint(wood, bark)
            paint(leaves, leaf)

    scene.render.filepath = os.path.join(SHOTS, into)
    bpy.ops.render.render(write_still=True)
    print(f"DREW {into}")


def main():
    fresh()
    standing = []
    middle = (len(SPECIES) - 1) * APART * 0.5
    for at, species in enumerate(SPECIES):
        brought = bring_in(species, at * APART - middle)
        wood = [obj for obj in brought if obj.name.startswith("wood")]
        leaves = [obj for obj in brought if obj.name.startswith("leaves")]
        standing.append((species, wood, leaves))

    camera = bpy.data.cameras.new("camera")
    camera.type = "ORTHO"
    # Orthographic, so the tree at the end of the row is the same size as the one
    # in the middle. A perspective row makes the outer species look smaller and
    # invites a judgement about proportion that is the lens talking.
    camera.ortho_scale = len(SPECIES) * APART
    eye = bpy.data.objects.new("camera", camera)
    eye.location = (0.0, -BACK, EYE + 3.0)
    eye.rotation_euler = (mathutils.Vector((1.0, 0.0, 0.0)).angle(mathutils.Vector((0.0, 0.0, 1.0))), 0.0, 0.0)
    bpy.context.collection.objects.link(eye)
    bpy.context.scene.camera = eye

    sun = bpy.data.lights.new("sun", type="SUN")
    sun.energy = 3.0
    lamp = bpy.data.objects.new("sun", sun)
    lamp.rotation_euler = (0.9, 0.2, 0.7)
    bpy.context.collection.objects.link(lamp)

    os.makedirs(SHOTS, exist_ok=True)
    render("trees_lit.png", False, standing)
    render("trees_silhouette.png", True, standing)
    print("SPECIES " + ", ".join(SPECIES))


main()
