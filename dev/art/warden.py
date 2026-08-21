"""Builds the warden as a stylised low-poly figure and saves `dev/art/warden.blend`.

    dev/art/build.sh              # builds the .blend and exports the .glb

# Scripted, and honest about why

This is not sculpted art. It is geometry described in code, which is a real step
up from the primitives it replaces — one mesh, proper proportions, a silhouette
that reads which way it is facing — and a long way short of what a modeller would
make. It exists so the whole path from Blender to the game is carrying something,
and so the thing you see walking around is the size and shape a person is.

**To take it over by hand:** open `dev/art/warden.blend`, work on it, commit the
`.blend`, and delete this script. A generator and a hand-edited file cannot both
be the source of truth — keeping both is how the two quietly disagree.

# The conventions this obeys

Metres, real scale, 1.8 m tall. Base on Z=0, so the game can place it by its feet.
**Facing +Y**, which the glTF Y-up conversion turns into -Z, which is what Bevy
means by forward. See `dev/model_export.py` for the whole story.

# Silhouette

Stylised: flat faces, no smoothing, big simple shapes that read at the distance
the follow camera actually sits at. The hat brim is longer at the front and there
is a satchel on the back, because a figure has to be readable from behind — the
placeholder was symmetric and you could not tell which way it was pointing, which
is also how a bug in `facing_quat` went unnoticed for months.
"""

import math
import os

import bpy
import mathutils

TALL = 1.8

# Every colour the figure is painted in, as linear RGB. Warden green for the coat
# and hat, matching the palette the placeholder used, so the swap does not also
# change what the warden looks like from a distance.
PAINT = {
    "coat": (0.22, 0.34, 0.24),
    "skin": (0.80, 0.62, 0.48),
    "hat": (0.18, 0.42, 0.22),
    "boot": (0.16, 0.12, 0.09),
    "bag": (0.44, 0.33, 0.20),
    "belt": (0.24, 0.18, 0.12),
}


def paint(name: str):
    """A flat matte material, which is what a stylised figure wants."""
    if name in bpy.data.materials:
        return bpy.data.materials[name]
    material = bpy.data.materials.new(name)
    shader = material.node_tree.nodes["Principled BSDF"]
    shader.inputs["Base Color"].default_value = (*PAINT[name], 1.0)
    shader.inputs["Roughness"].default_value = 0.85
    shader.inputs["Metallic"].default_value = 0.0
    return material


def box(name, at, size, colour, tilt=None):
    """One flat-shaded box, sized and placed in metres."""
    bpy.ops.mesh.primitive_cube_add(size=1.0, location=at)
    part = bpy.context.object
    part.name = name
    part.scale = size
    if tilt:
        part.rotation_euler = tilt
    bpy.ops.object.transform_apply(location=False, rotation=True, scale=True)
    part.data.materials.append(paint(colour))
    return part


def tube(name, at, radius, deep, colour, sides=12, tilt=None):
    """A low-sided cylinder — round enough to read, coarse enough to stay stylised."""
    bpy.ops.mesh.primitive_cylinder_add(
        vertices=sides, radius=radius, depth=deep, location=at
    )
    part = bpy.context.object
    part.name = name
    if tilt:
        part.rotation_euler = tilt
        bpy.ops.object.transform_apply(location=False, rotation=True, scale=False)
    part.data.materials.append(paint(colour))
    return part


def build() -> None:
    bpy.ops.wm.read_factory_settings(use_empty=True)

    # Legs and boots. Feet flat on Z=0 — the game places this figure by its base.
    for hand in (-1, 1):
        x = hand * 0.11
        box(f"boot.{hand}", (x, 0.02, 0.06), (0.15, 0.30, 0.12), "boot")
        box(f"leg.{hand}", (x, 0.0, 0.50), (0.14, 0.16, 0.76), "coat")

    # The coat: a torso that flares at the hem, so the figure has a shape rather
    # than being a column. Two stacked boxes, the lower one wider.
    box("coat.skirt", (0.0, 0.0, 0.98), (0.40, 0.30, 0.34), "coat")
    box("coat.chest", (0.0, 0.0, 1.30), (0.44, 0.26, 0.36), "coat")
    box("belt", (0.0, 0.0, 1.10), (0.46, 0.30, 0.07), "belt")

    # Arms, hanging with a slight outward lean so they read as arms and not as
    # part of the torso.
    for hand in (-1, 1):
        box(
            f"arm.{hand}",
            (hand * 0.27, 0.0, 1.18),
            (0.12, 0.14, 0.56),
            "coat",
            tilt=(0.0, hand * math.radians(-6), 0.0),
        )
        box(f"hand.{hand}", (hand * 0.30, 0.0, 0.90), (0.10, 0.11, 0.12), "skin")

    # Head and neck.
    box("neck", (0.0, 0.0, 1.51), (0.13, 0.13, 0.08), "skin")
    box("head", (0.0, 0.0, 1.64), (0.21, 0.22, 0.22), "skin")

    # The hat: a crown, and a brim pushed FORWARD so the front of the figure is
    # obvious from above and from behind.
    tube("hat.crown", (0.0, 0.0, 1.82), 0.155, 0.16, "hat")
    brim = tube("hat.brim", (0.0, 0.04, 1.745), 0.30, 0.035, "hat")
    brim.scale = (1.0, 1.25, 1.0)
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)

    # And a satchel on the BACK, which is the other half of reading the facing.
    box("satchel", (0.0, -0.22, 1.15), (0.26, 0.12, 0.24), "bag")
    box("strap", (0.0, -0.02, 1.30), (0.30, 0.32, 0.05), "bag")

    # One object, flat shaded. Joined because the game carries a model whole and
    # a figure in twenty pieces is twenty draw calls for no gain.
    bpy.ops.object.select_all(action="SELECT")
    body = bpy.data.objects["coat.chest"]
    bpy.context.view_layer.objects.active = body
    bpy.ops.object.join()
    body.name = "warden"
    bpy.ops.object.shade_flat()

    # Standing exactly TALL, however the parts above happened to land: measured
    # and corrected rather than trusted, because a figure that is 1.83 m fails the
    # export gate and a figure that is 1.7 m is quietly the wrong size.
    corners = [body.matrix_world @ mathutils.Vector(corner) for corner in body.bound_box]
    low = min(corner.z for corner in corners)
    high = max(corner.z for corner in corners)
    body.location.z -= low
    bpy.ops.object.transform_apply(location=True, rotation=False, scale=False)
    grew = TALL / (high - low)
    body.scale = (grew, grew, grew)
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)

    here = os.path.dirname(os.path.abspath(__file__))
    out = os.path.join(here, "warden.blend")
    bpy.ops.wm.save_as_mainfile(filepath=out)
    print(f"BUILT {out}")


build()
