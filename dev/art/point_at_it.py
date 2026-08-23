"""Renders the character with one shell picked out in red, so a removal can be agreed first.

Two mesh removals in this pipeline have taken the wrong thing - the sleeve cuffs once, and
faces out of the trouser leg once - and both times the render that would have shown it was
either not taken or misread. So anything that is about to be deleted gets pointed at first.

Set POINT_AT to the vertex count of the shell to highlight (they are distinct enough to
identify that way), and POINT_Z to the height in cm to centre the close-ups on.
"""
import math
import os
import sys

import bpy
import mathutils

ART = "C:/Users/jsull/Desktop/copaimo/dev/art"
sys.path.insert(0, ART)

GLB = "C:/Users/jsull/Desktop/copaimo/assets/models/person_ranger.glb"
OUT = os.environ.get("POINT_OUT", ".")
WANT = int(os.environ.get("POINT_AT", "62"))
HEIGHT = float(os.environ.get("POINT_Z", "124"))

bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=GLB)
rig = next(o for o in bpy.data.objects if o.type == "ARMATURE")

import prepare_rig

mesh = prepare_rig.the_body()
prepare_rig.reach_the_ends(rig, mesh)

picked = [s for s in prepare_rig.whole_shells(mesh) if len(s) == WANT]
if not picked:
    print(f"REFUSED: no shell with exactly {WANT} vertices")
    raise SystemExit(1)
shell = set(picked[0])
print(f"pointing at a shell of {len(shell)} vertices")

# A red material on just that shell's faces.
red = bpy.data.materials.new("point_at_it")
red.use_nodes = True
red.node_tree.nodes["Principled BSDF"].inputs["Base Color"].default_value = (
    0.9, 0.05, 0.05, 1.0
)
red.node_tree.nodes["Principled BSDF"].inputs["Emission Color"].default_value = (
    0.9, 0.05, 0.05, 1.0
)
red.node_tree.nodes["Principled BSDF"].inputs["Emission Strength"].default_value = 0.6
mesh.data.materials.append(red)
slot = len(mesh.data.materials) - 1
marked = 0
for poly in mesh.data.polygons:
    if all(i in shell for i in poly.vertices):
        poly.material_index = slot
        marked += 1
print(f"{marked} faces marked")

sun = bpy.data.objects.new("sun", bpy.data.lights.new("sun", type="SUN"))
bpy.context.scene.collection.objects.link(sun)
sun.data.energy = 3.5
sun.rotation_euler = (math.radians(55.0), 0.0, math.radians(35.0))
world = bpy.data.worlds.new("w")
world.use_nodes = True
world.node_tree.nodes["Background"].inputs[0].default_value = (0.16, 0.17, 0.19, 1.0)
bpy.context.scene.world = world

camera = bpy.data.objects.new("cam", bpy.data.cameras.new("cam"))
bpy.context.scene.collection.objects.link(camera)
camera.data.type = "ORTHO"
bpy.context.scene.camera = camera
scene = bpy.context.scene
scene.render.engine = "BLENDER_EEVEE"
scene.render.resolution_x = 420
scene.render.resolution_y = 520
scene.eevee.taa_render_samples = 24

aim = mathutils.Vector((0.0, 0.0, HEIGHT / 170.0))
for name, turn, zoom in (("front", 0.0, 0.55), ("side", 90.0, 0.55),
                         ("back", 180.0, 0.55), ("wide", 35.0, 1.15)):
    camera.data.ortho_scale = zoom
    angle = math.radians(turn)
    camera.location = (
        aim.x + 4.0 * math.sin(angle), aim.y - 4.0 * math.cos(angle), aim.z
    )
    camera.rotation_euler = (math.radians(90.0), 0.0, angle)
    scene.render.filepath = os.path.join(OUT, f"point_{name}.png")
    bpy.ops.render.render(write_still=True)

print("wrote point_front, point_side, point_back, point_wide")
