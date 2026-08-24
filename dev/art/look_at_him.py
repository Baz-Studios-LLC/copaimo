"""Renders the character from several angles, so a mesh change can be LOOKED at.

Geometry measurements cannot see what matters here. This file's standing lesson is that a
mesh can pass every numeric guard and still be lit as a different shape, and a hole left by a
cut is the same kind of fault - obvious to an eye and invisible to a vertex count. So after
anything that removes or adds geometry, this.

Front, side, back and a close pass on the armpit and hip, which is where the limb-to-body
fusions were cut and therefore where a hole would be.
"""
import math
import os
import sys

import bpy
import mathutils

ART = "C:/Users/jsull/Desktop/copaimo/dev/art"
sys.path.insert(0, ART)

# Overridable, so the RAW export can be put through the same lens as the prepared one. That
# comparison is the only way to tell "the pipeline broke this" from "it arrived like this",
# and guessing which is a fast way to spend a day fixing the generator's own work.
GLB = os.environ.get("LOOK_GLB", "C:/Users/jsull/Desktop/copaimo/assets/models/person_ranger.glb")
OUT = os.environ.get("LOOK_OUT", ".")

bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=GLB)
rig = next(o for o in bpy.data.objects if o.type == "ARMATURE")

import prepare_rig

mesh = prepare_rig.the_body()
if os.environ.get("LOOK_RAW") != "1":
    prepare_rig.reach_the_ends(rig, mesh)

sun = bpy.data.objects.new("sun", bpy.data.lights.new("sun", type="SUN"))
bpy.context.scene.collection.objects.link(sun)
sun.data.energy = 3.5
sun.rotation_euler = (math.radians(55.0), 0.0, math.radians(35.0))

fill = bpy.data.objects.new("fill", bpy.data.lights.new("fill", type="SUN"))
bpy.context.scene.collection.objects.link(fill)
fill.data.energy = 1.4
fill.rotation_euler = (math.radians(65.0), 0.0, math.radians(-150.0))

world = bpy.data.worlds.new("w")
world.use_nodes = True
world.node_tree.nodes["Background"].inputs[0].default_value = (0.14, 0.15, 0.17, 1.0)
world.node_tree.nodes["Background"].inputs[1].default_value = 1.0
bpy.context.scene.world = world

camera = bpy.data.objects.new("cam", bpy.data.cameras.new("cam"))
bpy.context.scene.collection.objects.link(camera)
camera.data.type = "ORTHO"
bpy.context.scene.camera = camera

scene = bpy.context.scene
scene.render.engine = "BLENDER_EEVEE"
scene.render.resolution_x = 420
scene.render.resolution_y = 700
scene.eevee.taa_render_samples = 32

low = min((mesh.matrix_world @ v.co).z for v in mesh.data.vertices)
high = max((mesh.matrix_world @ v.co).z for v in mesh.data.vertices)
middle = mathutils.Vector((0.0, 0.0, (low + high) * 0.5))
# The ankle joint, for the shoe shots - measured from the rig, because a shoe is a hand's
# breadth off the floor and no share of standing height lands there reliably. Aimed at ONE
# shoe rather than between the pair, or a close shot frames the gap between them.
ankle = rig.matrix_world @ rig.pose.bones["L_Foot"].head

# (name, degrees round, ortho scale, what to centre on)
SHOTS = (
    ("front", 0.0, 1.15, middle),
    ("side", 90.0, 1.15, middle),
    ("back", 180.0, 1.15, middle),
    ("armpit", 35.0, 0.42, mathutils.Vector((0.0, 0.0, high * 0.72))),
    ("hip", 20.0, 0.42, mathutils.Vector((0.0, 0.0, high * 0.55))),
    # The wrists, because that is where the generator hung four loose strap pieces off the
    # forearms - the thing that had to be removed, and the place to check nothing else went.
    ("wrists", 0.0, 0.50, mathutils.Vector((0.0, 0.0, high * 0.62))),
    ("wrists_side", 90.0, 0.50, mathutils.Vector((0.0, 0.0, high * 0.62))),
    # The thighs and knees. The fusion cut took faces owned partly by `Waist` down at knee
    # height, which is a MIS-WEIGHTED vertex rather than a fusion - so this is where a hole in
    # the trouser leg would be if that criterion was too broad.
    ("thighs", 0.0, 0.55, mathutils.Vector((0.0, 0.0, high * 0.42))),
    ("thighs_side", 90.0, 0.55, mathutils.Vector((0.0, 0.0, high * 0.42))),
    # Close on the forearm and the sleeve cuff, which is where four shells were removed as
    # stray straps - so it is where a wrongly removed sleeve would show.
    ("cuff", 25.0, 0.30, mathutils.Vector((0.0, 0.0, high * 0.65))),
    ("cuff_side", 105.0, 0.30, mathutils.Vector((0.0, 0.0, high * 0.65))),
    # The hand itself, close, from two sides - reported as angled backwards.
    ("hand", 20.0, 0.26, mathutils.Vector((0.0, 0.0, high * 0.56))),
    ("hand_side", 100.0, 0.26, mathutils.Vector((0.0, 0.0, high * 0.56))),
    # The three areas reported as needing mesh work: the side of the jacket, under the arms,
    # and the backpack. Close enough to see geometry rather than silhouette.
    ("jacket_side", 90.0, 0.60, mathutils.Vector((0.0, 0.0, high * 0.66))),
    ("jacket_side_far", 70.0, 0.75, mathutils.Vector((0.0, 0.0, high * 0.66))),
    ("underarm", 55.0, 0.40, mathutils.Vector((0.0, 0.0, high * 0.72))),
    ("underarm_other", -55.0, 0.40, mathutils.Vector((0.0, 0.0, high * 0.72))),
    ("pack", 180.0, 0.70, mathutils.Vector((0.0, 0.0, high * 0.72))),
    ("pack_side", 140.0, 0.70, mathutils.Vector((0.0, 0.0, high * 0.72))),
    # The shoes, reported as bulky twice. Height is the dimension the slimming never touched,
    # so the SIDE and the three-quarter are the ones that show it: a 26 cm shoe standing 11 cm
    # tall reads as a brick from the side and measures perfectly from above.
    ("shoe_side", 0.0, 0.30, ankle),
    ("shoe_front", 90.0, 0.30, ankle),
    ("shoe_quarter", 40.0, 0.30, ankle),
    ("shoe_above", 40.0, 0.30, ankle),
)

ONLY = [w for w in os.environ.get("LOOK_ONLY", "").split(",") if w]

made = []
for name, turn, zoom, aim in SHOTS:
    if ONLY and not any(w in name for w in ONLY):
        continue
    # A shoe is wider than it is tall, so it gets a landscape frame. In the portrait one
    # ortho_scale governs the HEIGHT, and a scale wide enough for the toe wasted two thirds
    # of the frame on empty floor - the first shoe render came back with the toe cropped off.
    wide = name.startswith("shoe")
    scene.render.resolution_x, scene.render.resolution_y = (700, 420) if wide else (420, 700)
    camera.data.ortho_scale = zoom
    angle = math.radians(turn)
    tilt = 55.0 if name == "shoe_above" else 90.0
    camera.location = (
        aim.x + 4.0 * math.sin(angle) * math.sin(math.radians(tilt)),
        aim.y - 4.0 * math.cos(angle) * math.sin(math.radians(tilt)),
        aim.z + 4.0 * math.cos(math.radians(tilt)),
    )
    camera.rotation_euler = (math.radians(tilt), 0.0, angle)
    scene.render.filepath = os.path.join(OUT, f"look_{name}.png")
    bpy.ops.render.render(write_still=True)
    made.append(scene.render.filepath)
    print(f"  {name}")

print("wrote " + ", ".join(os.path.basename(m) for m in made))
