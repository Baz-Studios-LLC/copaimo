"""Render one STEP of a clip from the side, as a row of poses.

The Animator's Survival Kit lays a run out as contact - down - pass - up - contact, seven
drawings for one step, and that row is how a run is actually judged: the shapes read against
each other, and a weak extreme is obvious next to a strong one in a way it never is while
scrubbing a timeline. So this produces the same artefact from our clip.

One STEP, not one cycle. Williams' seven drawings are contact to contact, which is half of a
full cycle - so frames 1 to 13 of a 24-frame clip. Rendering the whole cycle would put two
steps beside one and make every comparison off by a factor.

Orthographic and dead side-on, because a perspective camera makes the near arm read longer
than the far one, and arm swing is exactly what this is for. Transparent film, composited
afterwards, so the strip can be one image.
"""
import math
import os
import sys

import bpy
import mathutils

ART = "C:/Users/jsull/Desktop/copaimo/dev/art"
sys.path.insert(0, ART)

GLB = "C:/Users/jsull/Desktop/copaimo/assets/models/person_ranger.glb"
OUT = os.environ.get("SHEET_OUT", ".")
CLIP = os.environ.get("SHEET_CLIP", "run")
SHOTS = 7  # as many drawings as Williams uses, so the rows line up

bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=GLB)
rig = next(o for o in bpy.data.objects if o.type == "ARMATURE")

import prepare_rig

mesh = prepare_rig.the_body()
prepare_rig.reach_the_ends(rig, mesh)
_, forward, up = prepare_rig.body_frame(rig)

action = bpy.data.actions[CLIP]
if rig.animation_data is None:
    rig.animation_data_create()
rig.animation_data.action = action
if rig.animation_data.action_slot is None and action.slots:
    rig.animation_data.action_slot = action.slots[0]
lo, hi = (int(round(v)) for v in action.frame_range)
half = (hi - lo) // 2  # one step

sun = bpy.data.objects.new("sun", bpy.data.lights.new("sun", type="SUN"))
bpy.context.scene.collection.objects.link(sun)
sun.data.energy = 3.0
sun.rotation_euler = (math.radians(60.0), 0.0, math.radians(200.0))

world = bpy.data.worlds.new("w")
world.use_nodes = True
world.node_tree.nodes["Background"].inputs[1].default_value = 1.1
bpy.context.scene.world = world

camera = bpy.data.objects.new("cam", bpy.data.cameras.new("cam"))
bpy.context.scene.collection.objects.link(camera)
camera.data.type = "ORTHO"
camera.data.ortho_scale = 1.5
bpy.context.scene.camera = camera

scene = bpy.context.scene
scene.render.engine = "BLENDER_EEVEE"
scene.render.resolution_x = 300
scene.render.resolution_y = 460
scene.render.film_transparent = True
scene.eevee.taa_render_samples = 24

# The camera is fixed for the whole row, so the body's rise and fall is visible across it -
# aiming per frame would cancel exactly the vertical the sheet is meant to show.
bpy.context.scene.frame_set(lo)
middle = rig.matrix_world @ rig.pose.bones["Waist"].head
camera.location = (middle.x, middle.y + 4.0, middle.z)
camera.rotation_euler = (math.radians(90.0), 0.0, math.radians(180.0))

shots = []
for n in range(SHOTS):
    frame = lo + round(n * half / (SHOTS - 1))
    bpy.context.scene.frame_set(frame)

    shoulder = rig.matrix_world @ rig.pose.bones["R_Upperarm"].head
    hand = rig.matrix_world @ rig.pose.bones["R_Hand"].head
    line = (hand - shoulder).normalized()
    swing = math.degrees(math.atan2(line.dot(forward), -line.dot(up)))
    print(f"ROW shot {n + 1} = frame {frame:2d}, right arm {swing:+7.1f} deg from straight down")

    scene.render.filepath = os.path.join(OUT, f"sheet_{CLIP}_{n:02d}.png")
    bpy.ops.render.render(write_still=True)
    shots.append(scene.render.filepath)

print("ROW " + " ".join(os.path.basename(s) for s in shots))
