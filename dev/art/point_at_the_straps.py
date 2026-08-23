"""Renders the character with the loose chest straps picked out in red.

The straps that end in mid-air on the chest are NOT a separate shell - `point_at_it.py`
established that the only shell at armpit height is the necklace pendant, which is why
nothing has been deleted yet. They are welded into the body, so they have to be found as
FACES.

Three tests together, and each one is needed:

* near-black in the base colour map, because the webbing is the darkest thing on the chest
* between z 108 and 134, which is shoulder to mid-chest
* on the FRONT of the body, so the pack's own straps at the back are left alone

That finds 10 faces, 22.9 cm2, roughly symmetric - four right, five left, one central - and
protruding 10 to 14.8 cm forward of the body centre, which is a strap standing off a chest.

Nothing is deleted here. This only points.
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
DARK = 0.09
LOW, HIGH = 108.0, 134.0

bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=GLB)
rig = next(o for o in bpy.data.objects if o.type == "ARMATURE")

import prepare_rig

mesh = prepare_rig.the_body()
prepare_rig.reach_the_ends(rig, mesh)
across, forward, up = prepare_rig.body_frame(rig)
data = mesh.data

image = [
    node.image for mat in data.materials if mat and mat.use_nodes
    for node in mat.node_tree.nodes
    if node.type == "TEX_IMAGE" and node.image and "basecolor" in node.image.name
][0]
wide, tall = image.size
pixels = list(image.pixels)
uvs = data.uv_layers[0].data


def colour_of(poly):
    spots = [uvs[l].uv for l in poly.loop_indices]
    u = sum(a[0] for a in spots) / len(spots)
    v = sum(a[1] for a in spots) / len(spots)
    x = min(wide - 1, max(0, int(u * wide)))
    y = min(tall - 1, max(0, int(v * tall)))
    at = (y * wide + x) * 4
    return max(pixels[at], pixels[at + 1], pixels[at + 2])


straps = []
for poly in data.polygons:
    centre = mesh.matrix_world @ poly.center
    if not (LOW <= centre.z * 170.0 <= HIGH):
        continue
    if centre.dot(forward) < 0.0:
        continue
    if colour_of(poly) > DARK:
        continue
    straps.append(poly.index)

print(f"{len(straps)} strap faces, "
      f"{sum(data.polygons[i].area for i in straps) * 170.0 * 170.0:.1f} cm2")

red = bpy.data.materials.new("point_at_the_straps")
red.use_nodes = True
shader = red.node_tree.nodes["Principled BSDF"]
shader.inputs["Base Color"].default_value = (0.95, 0.05, 0.05, 1.0)
shader.inputs["Emission Color"].default_value = (0.95, 0.05, 0.05, 1.0)
shader.inputs["Emission Strength"].default_value = 1.2
data.materials.append(red)
slot = len(data.materials) - 1
for i in straps:
    data.polygons[i].material_index = slot

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
scene.render.resolution_y = 560
scene.eevee.taa_render_samples = 24

aim = mathutils.Vector((0.0, 0.0, 1.21 * 0.98))
for name, turn, zoom in (("front", 0.0, 0.55), ("quarter", 40.0, 0.55),
                         ("other", -40.0, 0.55), ("wide", 20.0, 1.1)):
    camera.data.ortho_scale = zoom
    angle = math.radians(turn)
    camera.location = (
        aim.x + 4.0 * math.sin(angle), aim.y - 4.0 * math.cos(angle), aim.z
    )
    camera.rotation_euler = (math.radians(90.0), 0.0, angle)
    scene.render.filepath = os.path.join(OUT, f"straps_{name}.png")
    bpy.ops.render.render(write_still=True)

print("wrote straps_front, straps_quarter, straps_other, straps_wide")
