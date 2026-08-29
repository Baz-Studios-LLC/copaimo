"""Assembles a short bridge from the exported pieces and renders it, to be LOOKED at.

    dev/art/see_the_bridge.sh

# Why this exists

The same reason `see_the_town.py` does, and one more besides. A bridge is built as
ONE arch that the game repeats, so the thing that can be wrong is not the arch - it
is the join. A span whose pier is a hair too wide leaves a gap between arches; one a
hair too narrow buries every second pier inside its neighbour. Neither shows in a
measurement of one span, because one span is correct in both cases.

So this stands three spans end to end with an abutment at each end, exactly the way
`world::bridge` lays them, and renders it against water at the height a real
crossing puts it. What comes out is a picture of the join.

Nothing here is part of the game build.
"""

import math
import os
import sys

import bpy
import mathutils

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import bridge as figures

SPANS = 3
OUT = os.path.join(HERE, "shots", "bridge.png")


def load(name: str):
    """Brings one built figure in from its .blend."""
    path = os.path.join(HERE, f"bridge_{name}.blend")
    before = set(bpy.data.objects)
    with bpy.data.libraries.load(path) as (src, dst):
        dst.objects = [o for o in src.objects]
    made = None
    for obj in bpy.data.objects:
        if obj not in before and obj.type == "MESH":
            made = obj
            break
    return made


def place(source, at, turn=0.0):
    copy = source.copy()
    copy.data = source.data.copy()
    copy.location = at
    copy.rotation_euler = (0.0, 0.0, turn)
    bpy.context.collection.objects.link(copy)
    return copy


bpy.ops.wm.read_factory_settings(use_empty=True)

span = load("span")
end = load("end")

# The deck height a real crossing came out at, so the water sits where it will in
# the game rather than wherever is flattering.
DECK = 14.7
FOOT = DECK - figures.DECK_ABOVE_FOOT

run = figures.SPAN_LONG * SPANS
for i in range(SPANS):
    place(span, ((i + 0.5) * figures.SPAN_LONG - run * 0.5, 0.0, FOOT))
for side in (-1.0, 1.0):
    place(end, (side * (run + figures.SPAN_LONG * 0.5) * 0.5, 0.0, FOOT))

# The shores, as two blocks of land with a channel between them.
for side in (-1.0, 1.0):
    bpy.ops.mesh.primitive_cube_add(
        size=1.0,
        location=(side * (run * 0.5 + 40.0), 0.0, DECK - 20.0),
    )
    shore = bpy.context.object
    shore.scale = (80.0, 160.0, 40.0)
    bpy.ops.object.transform_apply(scale=True)

# The water.
bpy.ops.mesh.primitive_plane_add(size=600.0, location=(0.0, 0.0, 0.0))

# A 1.7 m post on the deck, for scale - the same check every other figure gets.
bpy.ops.mesh.primitive_cube_add(size=1.0, location=(0.0, 0.0, DECK + 0.85))
post = bpy.context.object
post.scale = (0.4, 0.4, 1.7)

# A material that shows the vertex colour the figures carry, because that IS their
# colour - painted in `masonry.weld`, no material involved. Rendered without one,
# every figure is white and the only thing a picture can show is silhouette.
paint = bpy.data.materials.new("paint")
paint.use_nodes = True
# The game culls back faces, which is what makes the inverted-hull outline read as a
# LINE rather than as a black shell over everything. A render without culling shows
# the shell instead of the figure - the whole bridge came out solid black.
paint.use_backface_culling = True
tree = paint.node_tree
attr = tree.nodes.new("ShaderNodeVertexColor")
attr.layer_name = "Color"
shader = tree.nodes["Principled BSDF"]
shader.inputs["Roughness"].default_value = 0.9
tree.links.new(attr.outputs["Color"], shader.inputs["Base Color"])
for obj in bpy.data.objects:
    if obj.type == "MESH" and obj.data.color_attributes:
        obj.data.materials.clear()
        obj.data.materials.append(paint)

sun = bpy.data.objects.new("sun", bpy.data.lights.new("sun", type="SUN"))
sun.data.energy = 4.0
sun.rotation_euler = (math.radians(52.0), 0.0, math.radians(38.0))
bpy.context.collection.objects.link(sun)

camera = bpy.data.objects.new("camera", bpy.data.cameras.new("camera"))
camera.location = (run * 0.9, -run * 2.4, DECK + 34.0)
aim = mathutils.Vector((0.0, 0.0, DECK - 9.0)) - mathutils.Vector(camera.location)
camera.rotation_euler = aim.to_track_quat("-Z", "Y").to_euler()
bpy.context.collection.objects.link(camera)
bpy.context.scene.camera = camera

scene = bpy.context.scene
scene.render.engine = "BLENDER_EEVEE"
scene.render.resolution_x = 1600
scene.render.resolution_y = 900
scene.render.filepath = OUT
os.makedirs(os.path.dirname(OUT), exist_ok=True)
bpy.ops.render.render(write_still=True)
print(f"RENDERED {OUT}")
