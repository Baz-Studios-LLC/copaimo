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


def flower():
    """A flower: a stem, and a cup of petals on the end of it.

    # It was confetti

    The first version was a flat five-sided fan, sized by the tuft's own scale and
    placed at 92% of its height with nothing joining it to the ground. In the game
    it read exactly as what it was: coloured pentagons hovering over the grass. Two
    faults and a third.

    * **No stem.** A flower is a stem with something on the end. Without one there
      is nothing to say the colour belongs to a plant.
    * **Flat.** A horizontal disc is a plate from above and invisible from the
      side, and the camera here is near the ground. Petals rise from the middle now,
      so there is a cup to see edge-on.
    * **Too big.** Sized by `scale` rather than by the tuft's HEIGHT, the head came
      out about fifteen centimetres across. A wildflower is two or three.

    Authored at unit height and scaled by the tuft's height, as two meshes — `stem`
    and `petals` — because they are tinted differently: the stem is the leaves'
    green, the head is whatever colour that flower drew.
    """
    # Standing ABOVE the grass around it, not among it. A blade runs to between
    # 0.72 and 1.14 of the tuft's height, so a head at 0.90 was hidden by half the
    # blades in its own tuft — which is most of why the first one had to be huge to
    # be seen at all.
    head = 1.06
    # About eight and a half centimetres across on a 0.72 m tuft. Fifteen was a
    # dinner plate and two was a speck; this is the size a stylised wildflower
    # reads at from the walking camera without becoming the loudest thing in the
    # field. It is the number to move if they want to be more or less noticeable —
    # everything else about the shape is settled.
    span = 0.058
    lift = 0.075

    # The stem: a narrow strip that leans a little, three rows so it is not a
    # ruler. Deliberately not the blade shape — a blade arches over and falls, and
    # a stem holding a flower up does not.
    stem_rows = [
        (0.00, 0.008, 0.000),
        (head * 0.55, 0.007, 0.010),
        (head, 0.005, 0.022),
    ]
    stem = sheet("stem", stem_rows, [0.62, 0.85, 1.0])

    # The head: five petals radiating from a small middle.
    #
    # Two wrong shapes came before this one, and both were the same mistake in
    # different clothes — a petal treated as a triangle.
    #
    # * Pointed outward, a ring of them is an AGAVE. `terrain_core::cover` already
    #   records this trap for grass tufts ("a ring of those is an agave"), and it
    #   applies just as well here.
    # * Pointed inward — wide at the rim, meeting at one shared middle vertex — a
    #   ring of them is a folded PAPER FAN, because every petal converges on a
    #   single point and the head becomes a cone.
    #
    # A flower is petals radiating from a small disc. So each petal is a quad with
    # an inner EDGE rather than an inner point, and a darker boss closes the middle
    # between them. The rim sits higher than the inner edge, so the head is a
    # shallow cup with something to see from the side.
    mesh = bpy.data.meshes.new("petals")
    count = 5
    # Half the angular width of a petal at its rim. Comfortably under half the gap
    # between petals, so five petals read as five and not as a disc.
    fan = 0.46
    boss = span * 0.24
    # NEARLY FLAT, and that is the last correction. The rim was a full `lift` above
    # the inner edge, which is a forty-five degree funnel — and a funnel seen from
    # anywhere but straight above is a paper fan, which is what three rounds of this
    # kept coming back as. A daisy's petals lie almost level; what gives a small
    # flower its presence is the STEM holding it up, not depth in the head.
    places = [(0.0, 0.0, head + lift * 0.34)]
    for index in range(count):
        axis = index / count * math.tau
        for radius, side, up in (
            (boss, -fan * 0.6, lift * 0.30),
            (boss, fan * 0.6, lift * 0.30),
            (span, fan, lift * 0.52),
            (span, -fan, lift * 0.52),
        ):
            places.append(
                (math.cos(axis + side) * radius, math.sin(axis + side) * radius, head + up)
            )
    faces = []
    for index in range(count):
        inner_a, inner_b, rim_b, rim_a = (1 + index * 4 + step for step in range(4))
        faces.append((inner_a, inner_b, rim_b, rim_a))
        # And close the middle toward the boss, or the flower has a hole in it.
        faces.append((0, inner_a, inner_b))
    mesh.from_pydata(places, [], faces)
    mesh.update()
    petals = bpy.data.objects.new("petals", mesh)
    bpy.context.collection.objects.link(petals)
    mesh.color_attributes.new(name="Color", type="FLOAT_COLOR", domain="POINT")
    layer = mesh.color_attributes["Color"]
    for index in range(len(places)):
        # A dark middle and bright petals: the throat is what says flower rather
        # than shape, and it costs nothing.
        shade = 0.42 if index == 0 else (0.66 if (index - 1) % 4 < 2 else 1.0)
        layer.data[index].color = (shade, shade, shade, 1.0)

    return [stem, petals]


def build(name, make) -> None:
    fresh()
    made = make()
    parts = made if isinstance(made, list) else [made]
    for obj in parts:
        obj.select_set(True)
    bpy.context.view_layer.objects.active = parts[0]
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
    counted = sum(len(obj.data.vertices) for obj in parts)
    print(f"BUILT cover_{name} — {counted} vertices in {len(parts)} mesh(es)")


build("blade", blade)
build("flower", flower)
