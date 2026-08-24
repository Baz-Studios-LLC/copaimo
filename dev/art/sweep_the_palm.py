"""Previews several palm rolls in one pass, without rebuilding the clips for each.

The palm angle has now defeated six attempts to measure it - a plane fit through the hand
vertices, a knuckle-line cross product, and four variations between - because the glove cuff
and the splayed fingers dominate every point cloud the palm might be derived from, and the
left and right hands disagree by 60 degrees on the same frame.

So it gets looked at instead. The hand bones are rotated about the FOREARM's own axis, which
is what pronation is, by a range of deltas from whatever the built asset already has. Each is
rendered. No rebuild per value: PALM_IN would need the whole pipeline run each time.

Research, for what the target is: "when sprinting, open your hands so that they are flat and
outstretched, with your palms facing inward towards your body".
"""
import math
import os
import sys

import bpy
import mathutils

ART = "C:/Users/jsull/Desktop/copaimo/dev/art"
sys.path.insert(0, ART)

GLB = "C:/Users/jsull/Desktop/copaimo/assets/models/person_ranger.glb"
OUT = os.environ.get("SWEEP_OUT", ".")
CLIP = os.environ.get("SWEEP_CLIP", "run")
FRAME = int(os.environ.get("SWEEP_FRAME", "9"))
DELTAS = (-90.0, -60.0, -30.0, 0.0, 30.0, 60.0)

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
fill.data.energy = 1.6
fill.rotation_euler = (math.radians(70.0), 0.0, math.radians(-140.0))
world = bpy.data.worlds.new("w")
world.use_nodes = True
world.node_tree.nodes["Background"].inputs[0].default_value = (0.17, 0.18, 0.20, 1.0)
bpy.context.scene.world = world

camera = bpy.data.objects.new("cam", bpy.data.cameras.new("cam"))
bpy.context.scene.collection.objects.link(camera)
camera.data.type = "ORTHO"
camera.data.ortho_scale = 0.30
bpy.context.scene.camera = camera
scene = bpy.context.scene
scene.render.engine = "BLENDER_EEVEE"
scene.render.resolution_x = 320
scene.render.resolution_y = 400
scene.eevee.taa_render_samples = 24

action = bpy.data.actions[CLIP]
if rig.animation_data is None:
    rig.animation_data_create()
rig.animation_data.action = action
if action.slots:
    rig.animation_data.action_slot = action.slots[0]
scene.frame_set(FRAME)
# Drop the action once the frame is posed. With it still assigned, every depsgraph update
# re-drives the hand from the clip and silently discards the rotation set below - which is
# why the first sweep produced six identical images.
rig.animation_data.action = None

held = {
    side: rig.pose.bones[f"{side}_Hand"].rotation_quaternion.copy() for side in "LR"
}


def roll(side, degrees):
    """Turns the hand about the FOREARM's long axis - which is what pronation is."""
    posed = rig.pose.bones[f"{side}_Hand"]
    along = (
        rig.pose.bones[f"{side}_Forearm"].matrix.to_3x3()
        @ mathutils.Vector((0.0, 1.0, 0.0))
    ).normalized()
    rest = posed.bone.matrix_local.to_3x3()
    local = (rest.inverted() @ along).normalized()
    posed.rotation_mode = "QUATERNION"
    posed.rotation_quaternion = (
        mathutils.Quaternion(local, math.radians(degrees)) @ held[side]
    )


for delta in DELTAS:
    for side in "LR":
        roll(side, delta)
    bpy.context.view_layer.update()
    wrist = rig.matrix_world @ rig.pose.bones["R_Hand"].head
    # Looked at from the FRONT, because "facing inward" is a thing you judge head on.
    camera.location = (wrist.x, wrist.y - 4.0, wrist.z)
    camera.rotation_euler = (math.radians(90.0), 0.0, 0.0)
    scene.render.filepath = os.path.join(OUT, f"sweep_{int(delta):+04d}.png")
    bpy.ops.render.render(write_still=True)
    print(f"  delta {delta:+.0f}")

print("wrote " + ", ".join(f"sweep_{int(d):+04d}" for d in DELTAS))
