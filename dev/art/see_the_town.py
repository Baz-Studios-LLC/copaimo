"""Lays every town building out in one scene and opens it, to be LOOKED at.

    dev/art/see_the_town.sh

# Why this exists

The same reason `see_the_character.py` does. A building that measures correctly can
still be wrong in every way that matters - a door in the wrong wall, a room the
camera cannot follow anybody into, a roof that reads as a tent - and none of that
shows in a number. This puts the four of them on a floor, side by side, each with a
1.7 m post standing at its door for scale, and opens Blender on it.

Nothing here is part of the game build. It reads the same `.blend` files
`dev/art/town.py` writes, so what is on screen is what ships.
"""

import os
import sys

import bpy
import mathutils

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

BUILDINGS = (
    # The old world, then its landmarks; then the city and its own.
    "cottage", "townhouse", "shop", "guild_hall", "market_cross", "well",
    "city_block", "city_tower", "city_spire", "monument",
)

# How much room each gets on the shelf, in metres. Wider than the widest building,
# so nothing overlaps its neighbour and the gaps read as gaps.
BAY = 24.0

# A warden is this tall. The post is not decoration: a building sized by eye against
# nothing is how a cottage ends up a barn.
WARDEN = 1.7


def clear():
    bpy.ops.wm.read_factory_settings(use_empty=True)
    for obj in list(bpy.data.objects):
        bpy.data.objects.remove(obj, do_unlink=True)


def paint(obj, rgb):
    material = bpy.data.materials.new("plain")
    material.use_nodes = True
    material.node_tree.nodes["Principled BSDF"].inputs["Base Color"].default_value = rgb
    obj.data.materials.append(material)


def bring_in(name, at_x):
    """Appends one building's objects and shifts them onto their bay."""
    path = os.path.join(HERE, f"town_{name}.blend")
    if not os.path.exists(path):
        print(f"  {name}: no {os.path.basename(path)} - run dev/art/town.py first")
        return None
    before = set(bpy.data.objects)
    with bpy.data.libraries.load(path) as (source, into):
        into.objects = list(source.objects)
    brought = []
    for obj in into.objects:
        if obj is None or obj.type != "MESH":
            continue
        bpy.context.collection.objects.link(obj)
        obj.location.x += at_x
        brought.append(obj)
    for obj in set(bpy.data.objects) - before - set(brought):
        if obj.type != "MESH":
            bpy.data.objects.remove(obj, do_unlink=True)
    return brought


def measure(objs):
    xs, ys, zs = [], [], []
    for obj in objs:
        for corner in obj.bound_box:
            at = obj.matrix_world @ mathutils.Vector(corner)
            xs.append(at.x)
            ys.append(at.y)
            zs.append(at.z)
    return (max(xs) - min(xs), max(ys) - min(ys), max(zs) - min(zs), min(ys), min(zs))


def main():
    clear()
    scene = bpy.context.scene

    bpy.ops.mesh.primitive_plane_add(size=460.0, location=(BAY * 4.5, 0.0, -0.02))
    paint(bpy.context.object, (0.16, 0.19, 0.14, 1.0))

    print("THE TOWN'S BUILDINGS, as they will stand in the world:")
    for index, name in enumerate(BUILDINGS):
        at_x = index * BAY
        brought = bring_in(name, at_x)
        if not brought:
            continue
        wide, deep, tall, front, floor = measure(brought)

        # The scale post, at the door - which every building here puts in its -Y
        # wall, so that is where somebody walks in.
        bpy.ops.mesh.primitive_cylinder_add(
            radius=0.24, depth=WARDEN, location=(at_x, front - 1.6, floor + WARDEN * 0.5)
        )
        bpy.context.object.name = f"warden_at_{name}"
        paint(bpy.context.object, (0.85, 0.22, 0.18, 1.0))

        print(
            f"  {name:<11} {wide:5.1f} x {deep:5.1f} m and {tall:5.1f} m tall"
            f"   ({tall / WARDEN:.1f} wardens high)"
        )

    bpy.ops.object.light_add(type="SUN", location=(20.0, -30.0, 40.0))
    bpy.context.object.data.energy = 4.0
    bpy.context.object.rotation_euler = (0.6, 0.1, 0.7)
    scene.world = bpy.data.worlds.new("sky")
    scene.world.use_nodes = True
    scene.world.node_tree.nodes["Background"].inputs[0].default_value = (0.10, 0.13, 0.16, 1.0)

    # A camera looking down the row, so the file opens on something worth seeing.
    camera = bpy.data.objects.new("look", bpy.data.cameras.new("look"))
    bpy.context.collection.objects.link(camera)
    scene.camera = camera
    camera.data.lens = 50.0
    aim = mathutils.Vector((BAY * 4.5, 0.0, 14.0))
    eye = aim + mathutils.Vector((-10.0, -150.0, 46.0))
    camera.location = eye
    camera.rotation_euler = (aim - eye).normalized().to_track_quat("-Z", "Y").to_euler()

    # Written OUTSIDE dev/art on purpose. Everything in that folder is an asset the
    # exporter will try to ship, and a 260 m viewing shelf is not an asset - it got
    # as far as being REFUSED for being over 60 m across, which is the export gate
    # doing exactly its job.
    out = os.path.join(HERE, "aside", "town_view.blend")
    bpy.ops.wm.save_as_mainfile(filepath=out)
    print(f"opening {out}")


main()
