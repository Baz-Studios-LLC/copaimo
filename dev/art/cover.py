"""Builds the ground cover's own pieces: one blade of grass, and a flower head.

    dev/art/build.sh

# What is authored here is the PIECE, not the tuft

Ground cover is not placed the way trees and rocks are. A tuft is composed: the
world decides how many blades it has, how far round they fan, which way the clump
leans, how deep a green it is, and how tall it has grown — all from the ground it
stands on. That composition is what stops a meadow reading as wallpaper, and it
stays exactly where it is.

So what a file provides is the SHAPE of one blade, and one head of petals. The
game stamps them many times over with the variation it already computes.

# The budget is fragments, not vertices

Worth reading before making these prettier. A generated blade is seven vertices,
and a chunk of open country carries about ninety thousand against a ceiling of a
hundred and forty-five — so a blade can afford ten vertices and not thirty.

But the real cost is WIDTH. Grass overdraws itself many times over, and going from
a four-centimetre wedge to a one-and-a-third-centimetre ribbon once put the vertex
count UP a fifth and the fragment count DOWN by thirty per cent at the same frame
cost. So: narrow. Narrower than looks right in isolation.

# Colour here is a MODULATION

Both pieces are painted in greys, not greens. The game multiplies them by the
colour that tuft should be — deep in a thicket, pale on thin ground, and whatever
the flower's own colour turned out to be. A green authored here would fight that
and a meadow would come out one flat colour, which is the thing the composition
exists to avoid.
"""

import math
import os

import bpy

# A blade is authored at UNIT height and scaled by the game, so its width is
# written as a share of its length. 1.8% of 0.72 m is about 1.3 cm, which is the
# ribbon width the world is already tuned to.
BLADE_WIDE = 0.018

# How far the tip leans out from the foot, as a share of the blade's height. Past
# upright and falling, which is the silhouette that says grass.
BLADE_ARCH = 0.62

# Root shade against tip shade. The foot of a blade sits in the shadow of its own
# tuft, and this is most of what makes a clump read as a clump.
ROOT_SHADE = 0.52


def fresh() -> None:
    bpy.ops.wm.read_factory_settings(use_empty=True)


def sheet(name, rows, shades):
    """One narrow ribbon from a list of (height, half-width, lean) stations.

    Built by hand rather than from a primitive: a blade is eight vertices and a
    primitive would spend thirty on being round in a direction nobody sees.
    """
    mesh = bpy.data.meshes.new(name)
    places = []
    faces = []
    for index, (up, half, out) in enumerate(rows):
        # Arching toward -Y in Blender, which the Y-up conversion turns into +Z —
        # and +Z is the direction the game rotates a blade away from.
        places.append((-half, -out, up))
        places.append((half, -out, up))
        if index:
            base = (index - 1) * 2
            faces.append((base, base + 1, base + 3, base + 2))
    mesh.from_pydata(places, [], faces)
    mesh.update()

    obj = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(obj)

    # Grey, dark at the root: a modulation for the game to tint.
    mesh.color_attributes.new(name="Color", type="FLOAT_COLOR", domain="POINT")
    layer = mesh.color_attributes["Color"]
    for index in range(len(places)):
        shade = shades[min(index // 2, len(shades) - 1)]
        layer.data[index].color = (shade, shade, shade, 1.0)
    return obj


def blade():
    """A blade of grass: a ribbon that leaves the ground steeply and falls over.

    Four stations and eight vertices, one more station than the generated blade
    it replaces — which is the whole of the budget there was to spend.
    """
    wide = BLADE_WIDE
    rows = [
        (0.00, wide, 0.00),
        (0.42, wide * 0.92, BLADE_ARCH * 0.10),
        (0.76, wide * 0.66, BLADE_ARCH * 0.42),
        (1.00, wide * 0.16, BLADE_ARCH),
    ]
    shades = [ROOT_SHADE, ROOT_SHADE + 0.18, 0.88, 1.0]
    return sheet("blade", rows, shades)


def petals():
    """A head of petals: a shallow cup, seen mostly from above and from the side.

    Six vertices for five petals — a fan around a raised middle. The generated
    head was three arching ribbons at twenty-one vertices, so this is both a
    better shape and a third of the cost, which is where the budget for the extra
    blade station came from.
    """
    span = 0.075
    mesh = bpy.data.meshes.new("petals")
    places = [(0.0, 0.0, 0.030)]
    faces = []
    count = 5
    for index in range(count):
        angle = index / count * math.tau
        places.append((math.cos(angle) * span, math.sin(angle) * span, 0.0))
    for index in range(count):
        faces.append((0, 1 + index, 1 + (index + 1) % count))
    mesh.from_pydata(places, [], faces)
    mesh.update()

    obj = bpy.data.objects.new("petals", mesh)
    bpy.context.collection.objects.link(obj)
    mesh.color_attributes.new(name="Color", type="FLOAT_COLOR", domain="POINT")
    layer = mesh.color_attributes["Color"]
    # The middle a little darker than the rim, which is what a flower does.
    for index in range(len(places)):
        shade = 0.72 if index == 0 else 1.0
        layer.data[index].color = (shade, shade, shade, 1.0)
    return obj


def build(name, make) -> None:
    fresh()
    obj = make()
    bpy.context.view_layer.objects.active = obj
    obj.select_set(True)
    # SMOOTH, and it costs nothing to look at. Flat shading makes the exporter
    # split every vertex per face — an eight-vertex blade ships as twelve, and a
    # chunk of meadow went from 93k vertices to 157k against a ceiling of 145k.
    # Smooth shares them, and the shading in the file is irrelevant here because
    # the stamp overwrites every normal with straight up (see below).
    bpy.ops.object.shade_smooth()
    # Nothing is done about NORMALS here, on purpose. A blade wants to be lit from
    # above rather than honestly — its own normal points sideways, and a meadow lit
    # that way flickers dark as the camera turns, which is why the generated blades
    # already face theirs up. Blender will not have it set on a vertex (it is
    # read-only) and doing it with custom split normals would put the behaviour in
    # an export setting. The stamp writes it instead, where it cannot be lost.

    here = os.path.dirname(os.path.abspath(__file__))
    bpy.ops.wm.save_as_mainfile(filepath=os.path.join(here, f"cover_{name}.blend"))
    print(f"BUILT cover_{name} — {len(obj.data.vertices)} vertices")


build("blade", blade)
build("petals", petals)
