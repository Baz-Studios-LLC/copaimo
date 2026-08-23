"""Highlights the long flat ribbons of geometry that run from the body out to the arm.

These are the straps reported as "attached to the arm from the back". They are not a separate
shell and they are not dark in the texture - what makes them findable is their SHAPE: a long
thin blade spanning bones that are far apart in the skeleton.

The test is the one written into the note left on `cut_the_fusions`, which holed the trousers
by being too broad:

* a face carrying an edge longer than 4x the median edge
* whose ends are driven by bones at least 4 joints apart
* and - the restriction that was missing - where those two regions are ARM and TRUNK.

Never leg-and-trunk. That pair is what took the trouser leg last time: there are `Waist`
weights sitting down at knee height, so an ordinary trouser face can look like a bridge.

Nothing is deleted here. Red is what would go.
"""
import math
import os
import sys

import bpy
import mathutils

ART = "C:/Users/jsull/Desktop/copaimo/dev/art"
sys.path.insert(0, ART)

GLB = "C:/Users/jsull/Desktop/copaimo/assets/models/person_ranger.glb"
OUT = os.environ.get("BLADE_OUT", ".")

bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=GLB)
rig = next(o for o in bpy.data.objects if o.type == "ARMATURE")

import prepare_rig

mesh = prepare_rig.the_body()
prepare_rig.reach_the_ends(rig, mesh)
data = mesh.data

groups = {g.index: g.name for g in mesh.vertex_groups}


def owner(vertex):
    best, who = 0.0, ""
    for group in vertex.groups:
        if group.weight > best:
            best, who = group.weight, groups.get(group.group, "")
    return who


def region(name):
    for part in ("Forearm", "Upperarm", "Hand", "Clavicle"):
        if part in name:
            return "arm"
    for part in ("Thigh", "Calf", "Foot", "Toe"):
        if part in name:
            return "leg"
    for part in ("Spine", "Waist", "Hip", "Pelvis", "Neck", "Head"):
        if part in name:
            return "trunk"
    return None


owners = [owner(v) for v in data.vertices]
regions = [region(o) for o in owners]

joined = {}
for bone in rig.data.bones:
    if bone.parent:
        joined.setdefault(bone.name, set()).add(bone.parent.name)
        joined.setdefault(bone.parent.name, set()).add(bone.name)
known = {}


def apart(a, b):
    if a == b:
        return 0
    key = (a, b) if a < b else (b, a)
    if key in known:
        return known[key]
    seen, edge, far = {a}, [a], 0
    while edge and far < 14:
        far += 1
        nxt = []
        for here in edge:
            for there in joined.get(here, ()):
                if there == b:
                    known[key] = far
                    return far
                if there not in seen:
                    seen.add(there)
                    nxt.append(there)
        edge = nxt
    known[key] = 99
    return 99


lengths = sorted(
    (data.vertices[e.vertices[0]].co - data.vertices[e.vertices[1]].co).length
    for e in data.edges
)
median = lengths[len(lengths) // 2]
long_enough = median * 4.0

blades = []
for poly in data.polygons:
    ring = poly.vertices[:]
    hit = False
    for i in range(len(ring)):
        a, b = ring[i], ring[(i + 1) % len(ring)]
        span = (data.vertices[a].co - data.vertices[b].co).length
        if span <= long_enough:
            continue
        if apart(owners[a], owners[b]) < 4:
            continue
        pair = {regions[a], regions[b]}
        if pair == {"arm", "trunk"}:
            hit = True
            break
    if hit:
        blades.append(poly.index)

area = sum(data.polygons[i].area for i in blades) * 170.0 * 170.0
print(f"median edge {median * 170.0:.2f} cm, so long is over {long_enough * 170.0:.2f} cm")
print(f"{len(blades)} arm-to-trunk blade faces, {area:.1f} cm2")

red = bpy.data.materials.new("blades")
red.use_nodes = True
shader = red.node_tree.nodes["Principled BSDF"]
shader.inputs["Base Color"].default_value = (0.95, 0.06, 0.06, 1.0)
shader.inputs["Emission Color"].default_value = (0.95, 0.06, 0.06, 1.0)
shader.inputs["Emission Strength"].default_value = 1.3
data.materials.append(red)
slot = len(data.materials) - 1
for i in blades:
    data.polygons[i].material_index = slot

sun = bpy.data.objects.new("sun", bpy.data.lights.new("sun", type="SUN"))
bpy.context.scene.collection.objects.link(sun)
sun.data.energy = 3.2
sun.rotation_euler = (math.radians(55.0), 0.0, math.radians(35.0))
world = bpy.data.worlds.new("w")
world.use_nodes = True
world.node_tree.nodes["Background"].inputs[0].default_value = (0.17, 0.18, 0.20, 1.0)
bpy.context.scene.world = world

camera = bpy.data.objects.new("cam", bpy.data.cameras.new("cam"))
bpy.context.scene.collection.objects.link(camera)
camera.data.type = "ORTHO"
bpy.context.scene.camera = camera
scene = bpy.context.scene
scene.render.engine = "BLENDER_EEVEE"
scene.render.resolution_x = 440
scene.render.resolution_y = 620
scene.eevee.taa_render_samples = 24

low = min((mesh.matrix_world @ v.co).z for v in data.vertices)
high = max((mesh.matrix_world @ v.co).z for v in data.vertices)
middle = mathutils.Vector((0.0, 0.0, (low + high) * 0.5))
chest = mathutils.Vector((0.0, 0.0, low + (high - low) * 0.62))

for name, turn, zoom, aim in (
    ("whole", 25.0, 1.15, middle),
    ("back", 180.0, 1.15, middle),
    ("arm", 60.0, 0.55, chest),
    ("arm_other", -60.0, 0.55, chest),
):
    camera.data.ortho_scale = zoom
    yaw = math.radians(turn)
    camera.location = (aim.x + 4.0 * math.sin(yaw), aim.y - 4.0 * math.cos(yaw), aim.z)
    camera.rotation_euler = (math.radians(90.0), 0.0, yaw)
    scene.render.filepath = os.path.join(OUT, f"blade_{name}.png")
    bpy.ops.render.render(write_still=True)

print("wrote blade_whole, blade_back, blade_arm, blade_arm_other")
