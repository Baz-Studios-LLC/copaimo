"""Renders the hand across a clip, so an orientation complaint can be seen rather than argued.

The wrist's bend measures fine - plus or minus 14 degrees from the bind, symmetric between the
sides, and 1.4 degrees at idle - so whatever "angled backwards" is, it is not the flexion. A
strip of frames from the side is the cheapest way to find out what it actually is.

Set HAND_CLIP and HAND_SIDE. Frames are spread across one whole cycle.
"""
import math
import os
import sys

import bpy
import mathutils

ART = "C:/Users/jsull/Desktop/copaimo/dev/art"
sys.path.insert(0, ART)

GLB = "C:/Users/jsull/Desktop/copaimo/assets/models/person_ranger.glb"
OUT = os.environ.get("HAND_OUT", ".")
CLIP = os.environ.get("HAND_CLIP", "run")
SIDE = os.environ.get("HAND_SIDE", "R")
SHOTS = 6

bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=GLB)
rig = next(o for o in bpy.data.objects if o.type == "ARMATURE")

import prepare_rig

mesh = prepare_rig.the_body()
prepare_rig.reach_the_ends(rig, mesh)

sun = bpy.data.objects.new("sun", bpy.data.lights.new("sun", type="SUN"))
bpy.context.scene.collection.objects.link(sun)
sun.data.energy = 3.5
sun.rotation_euler = (math.radians(55.0), 0.0, math.radians(35.0))
fill = bpy.data.objects.new("fill", bpy.data.lights.new("fill", type="SUN"))
bpy.context.scene.collection.objects.link(fill)
fill.data.energy = 1.5
fill.rotation_euler = (math.radians(70.0), 0.0, math.radians(-140.0))

world = bpy.data.worlds.new("w")
world.use_nodes = True
world.node_tree.nodes["Background"].inputs[0].default_value = (0.16, 0.17, 0.19, 1.0)
bpy.context.scene.world = world

camera = bpy.data.objects.new("cam", bpy.data.cameras.new("cam"))
bpy.context.scene.collection.objects.link(camera)
camera.data.type = "ORTHO"
camera.data.ortho_scale = 0.34
bpy.context.scene.camera = camera

scene = bpy.context.scene
scene.render.engine = "BLENDER_EEVEE"
scene.render.resolution_x = 340
scene.render.resolution_y = 420
scene.eevee.taa_render_samples = 24

action = bpy.data.actions[CLIP]
if rig.animation_data is None:
    rig.animation_data_create()
rig.animation_data.action = action
if action.slots:
    rig.animation_data.action_slot = action.slots[0]
lo, hi = (int(round(v)) for v in action.frame_range)
span = hi - lo

made = []
for n in range(SHOTS):
    frame = lo + round(n * span / SHOTS)
    scene.frame_set(frame)
    # The camera follows the hand, side on, so the wrist angle is what fills the frame.
    wrist = rig.matrix_world @ rig.pose.bones[f"{SIDE}_Hand"].head
    camera.location = (wrist.x, wrist.y + 4.0, wrist.z)
    camera.rotation_euler = (math.radians(90.0), 0.0, math.radians(180.0))
    scene.render.filepath = os.path.join(OUT, f"hand_{CLIP}_{n:02d}.png")
    bpy.ops.render.render(write_still=True)
    made.append(frame)

print(f"wrote {SHOTS} shots of the {SIDE} hand across {CLIP}: frames {made}")
