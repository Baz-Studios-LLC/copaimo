"""Draws the dumped patch of ground cover. See dev/cover_look.sh."""

import math
import os
import sys

import bpy
import mathutils

out, patch = sys.argv[sys.argv.index("--") + 1], sys.argv[sys.argv.index("--") + 2]
bpy.ops.wm.read_factory_settings(use_empty=True)
scene = bpy.context.scene
scene.render.engine = "BLENDER_EEVEE"
scene.render.resolution_x, scene.render.resolution_y = 1200, 700

world = bpy.data.worlds.new("w")
scene.world = world
world.node_tree.nodes["Background"].inputs[0].default_value = (0.45, 0.60, 0.76, 1)
world.node_tree.nodes["Background"].inputs[1].default_value = 0.9

bpy.ops.wm.ply_import(filepath=patch)
cover = bpy.context.object

# UPRIGHT. The dump is in the game's axes — Y is up — and Blender's PLY import
# takes the numbers literally, so the patch arrives lying on its side. It still
# looks like a field of grass from a careless angle, which is worse than looking
# broken: two renders were judged that way before a stray coordinate gave it away
# (a flower head reported 3 m up, which is the width of the patch, not a height).
cover.rotation_euler = (math.radians(90), 0.0, 0.0)
bpy.ops.object.transform_apply(location=False, rotation=True, scale=False)

# The game's cover material: white, multiplied by the colour in the vertices, and
# lit from both sides because a blade is a single sheet.
paint = bpy.data.materials.new("cover")
tree = paint.node_tree
shader = tree.nodes["Principled BSDF"]
shader.inputs["Roughness"].default_value = 0.9
attr = tree.nodes.new("ShaderNodeVertexColor")
attr.layer_name = "Col"
tree.links.new(attr.outputs["Color"], shader.inputs["Base Color"])
paint.use_backface_culling = False
cover.data.materials.append(paint)

# The ground under it, in the grass green the world paints.
bpy.ops.mesh.primitive_plane_add(size=40, location=(1.6, 1.6, 0.0))
ground = bpy.data.materials.new("ground")
ground.node_tree.nodes["Principled BSDF"].inputs["Base Color"].default_value = (
    0.055, 0.16, 0.035, 1,
)
bpy.context.object.data.materials.append(ground)

sun = bpy.data.objects.new("sun", bpy.data.lights.new("sun", "SUN"))
scene.collection.objects.link(sun)
sun.data.energy = 3.6
sun.rotation_euler = (math.radians(50), 0, math.radians(35))

cam = bpy.data.objects.new("cam", bpy.data.cameras.new("cam"))
scene.collection.objects.link(cam)
scene.camera = cam
# CLOSE=1 puts the camera on a flower instead of over the patch, which is the only
# way to judge a shape a few centimetres across.
#
# The flower is FOUND rather than guessed at. Aiming at coordinates worked out by
# hand from the dump's loop landed in a thicket of grass with no flower in frame,
# and there is no need to guess: a petal is the only thing in a field of grass that
# is not green, so the brightest non-green vertex is a flower head.
if os.environ.get("CLOSE"):
    colours = cover.data.color_attributes[0].data
    found = None
    for point in cover.data.vertices:
        colour = colours[point.index].color
        # Not green: a petal. Grass runs green well clear of red and blue.
        if colour[1] > colour[0] and colour[1] > colour[2]:
            continue
        world = cover.matrix_world @ point.co
        if found is None or world.z > found.z:
            found = world
    target = found if found else mathutils.Vector((1.6, 1.6, 0.4))
    print(f"CLOSE on a flower at {tuple(round(part, 2) for part in target)}")
    # A third of a metre back and a little above, looking down at it. Closer than
    # this and a 10 cm flower fills the frame as an abstract fan, which tells you
    # nothing about whether it reads as a flower.
    # Looking DOWN at about forty degrees, which is where the game's follow camera
    # actually is — behind the warden and above them. Judged from below the rim
    # first, and an upward-opening cup seen from underneath is a paper fan whatever
    # its shape.
    cam.data.lens = 50
    cam.location = target + mathutils.Vector((0.14, -0.20, 0.30))
else:
    cam.data.lens = 48
    # Down at the grass, which is where the game's camera looks from.
    cam.location = mathutils.Vector((1.6, -3.0, 1.7))
    target = mathutils.Vector((1.6, 1.6, 0.45))
cam.rotation_euler = (target - cam.location).to_track_quat("-Z", "Y").to_euler()

scene.render.filepath = out
bpy.ops.render.render(write_still=True)
print(f"RENDERED {out}")
