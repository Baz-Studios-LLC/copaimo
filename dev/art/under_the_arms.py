"""Renders up into the armpits, which is the one place the other views cannot see.

Every render so far has looked at the character from around eye level or straight on, and a
strap hanging UNDER an arm is hidden by the arm itself from all of them. So this puts the
camera below and outside, looking up and in.

The chest straps are NOT what this is for - those were highlighted, shown and confirmed fine.
"""
import math
import os
import sys

import bpy
import mathutils

ART = "C:/Users/jsull/Desktop/copaimo/dev/art"
sys.path.insert(0, ART)

GLB = os.environ.get("UNDER_GLB", "C:/Users/jsull/Desktop/copaimo/assets/models/person_ranger.glb")
OUT = os.environ.get("UNDER_OUT", ".")

bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=GLB)
rig = next(o for o in bpy.data.objects if o.type == "ARMATURE")

import prepare_rig

mesh = prepare_rig.the_body()
if os.environ.get("UNDER_RAW") != "1":
    prepare_rig.reach_the_ends(rig, mesh)

sun = bpy.data.objects.new("sun", bpy.data.lights.new("sun", type="SUN"))
bpy.context.scene.collection.objects.link(sun)
sun.data.energy = 4.0
# Lit from BELOW, so an armpit is not just a dark hole.
sun.rotation_euler = (math.radians(-40.0), 0.0, math.radians(30.0))
fill = bpy.data.objects.new("fill", bpy.data.lights.new("fill", type="SUN"))
bpy.context.scene.collection.objects.link(fill)
fill.data.energy = 2.0
fill.rotation_euler = (math.radians(-60.0), 0.0, math.radians(200.0))

world = bpy.data.worlds.new("w")
world.use_nodes = True
world.node_tree.nodes["Background"].inputs[0].default_value = (0.18, 0.19, 0.21, 1.0)
world.node_tree.nodes["Background"].inputs[1].default_value = 1.2
bpy.context.scene.world = world

camera = bpy.data.objects.new("cam", bpy.data.cameras.new("cam"))
bpy.context.scene.collection.objects.link(camera)
camera.data.type = "ORTHO"
bpy.context.scene.camera = camera
scene = bpy.context.scene
scene.render.engine = "BLENDER_EEVEE"
scene.render.resolution_x = 460
scene.render.resolution_y = 520
scene.eevee.taa_render_samples = 32

# The armpit's real height, from the shoulder joint - not a guessed fraction.
pit = min(
    (rig.matrix_world @ rig.pose.bones[f"{s}_Upperarm"].head).z for s in "LR"
)
print(f"the shoulder joints sit at z {pit * 170.0:.1f} cm")

for name, turn, rise in (
    ("left_up", 55.0, -35.0),
    ("right_up", -55.0, -35.0),
    ("left_out", 80.0, -15.0),
    ("right_out", -80.0, -15.0),
):
    camera.data.ortho_scale = 0.45
    yaw = math.radians(turn)
    pitch = math.radians(rise)
    away = 3.0
    camera.location = (
        away * math.sin(yaw) * math.cos(pitch),
        -away * math.cos(yaw) * math.cos(pitch),
        pit + away * math.sin(-pitch) * -1.0,
    )
    camera.rotation_euler = (math.radians(90.0) + pitch, 0.0, yaw)
    scene.render.filepath = os.path.join(OUT, f"under_{name}.png")
    bpy.ops.render.render(write_still=True)

print("wrote under_left_up, under_right_up, under_left_out, under_right_out")
