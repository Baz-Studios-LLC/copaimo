"""Builds the warden's own body, and the hairstyles that go on it.

    dev/art/build.sh

# What a character creator needs from a model

A player picks a build, a skin, an eye colour, a hairstyle and a hair colour. Every
one of those is a COLOUR on a part or a swap of a part — so the body cannot be one
welded object the way a rock is. It comes out as four meshes the game tints
separately:

| Mesh | Tinted by |
| --- | --- |
| `skin` | skin colour — head, neck, ears, hands |
| `clothes` | fixed for now: tunic, sleeves, trousers, boots |
| `sclera` | fixed white, so an eye reads as an eye |
| `eyes` | eye colour — the irises |

A hairstyle is a file of its own, one mesh named `hair`, so styles can be added
without touching a body. They are named `part_*` because they do not stand on the
ground: the export gate's footing rule is for things placed in the world, and a
hairstyle sits on a head a metre and a half up.

# Cartoon, not a person

The whole world is stylised, so this is too, and the proportions are the decision
that makes it read that way. A real adult is about seven and a half heads tall.
This one is **five and a half** — a big head on a small body, which is what says
"character" rather than "mannequin" and is also what makes a face readable from the
follow camera at all. Everything is a rounded box or a ball; there is no anatomy in
here and there should not be.

# The two builds

Not a slider and not a skeleton — two presets, as asked. They differ in shoulder
and hip width, in how long the torso is, and by three centimetres of height.
Everything else, the head above all, is shared, which is what lets one hairstyle
fit both.
"""

import math
import os

import bpy
import mathutils

TALL = 1.80

# The head: big, because that is the whole of the cartoon read. Its middle sits
# here in both builds, so a hairstyle authored once fits either.
HEAD_AT = 1.52
HEAD_HIGH = 0.385
HEAD_WIDE = 0.325
HEAD_DEEP = 0.305

# Greys, all of them: every part is tinted by the game. A colour authored here
# would fight the player's choice. The number is the SHADE — darker at the foot of
# a part, so a figure has some depth to it under one flat sun.
FOOT_SHADE = 0.80

BUILDS = ("male", "female")
STYLES = ("crop", "bob", "tail", "braids", "curls")


def fresh() -> None:
    bpy.ops.wm.read_factory_settings(use_empty=True)


def shade_in(obj, low: float, high: float, ramp_from: float, ramp_to: float) -> None:
    """Writes a grey ramp into the mesh, `low` at the bottom to `high` at the top.

    # In WORLD height, not the mesh's own

    `ramp_from` and `ramp_to` are heights in the world — a foot and a crown — so the
    vertex has to be measured there too. This read `point.co.z`, which is measured
    from the object's ORIGIN, and after a join that origin is wherever the first part
    happened to sit. A wig joined from a cap at 1.58 m came out with every vertex
    reading as below the ramp, so the whole thing was painted one flat tone; the
    bodies were being shaded off a ramp anchored at the head. Caught by the test that
    asks whether a model still carries its gradient.
    """
    mesh = obj.data
    if not mesh.color_attributes:
        mesh.color_attributes.new(name="Color", type="FLOAT_COLOR", domain="POINT")
    layer = mesh.color_attributes["Color"]
    span = max(ramp_to - ramp_from, 1.0e-4)
    place = obj.matrix_world
    for point in mesh.vertices:
        up = min(1.0, max(0.0, ((place @ point.co).z - ramp_from) / span))
        shade = low + (high - low) * up
        layer.data[point.index].color = (shade, shade, shade, 1.0)


def blob(size, at, subdiv=2):
    """A rounded lump. Still here for the hair, which is made of them."""
    bpy.ops.mesh.primitive_ico_sphere_add(subdivisions=subdiv, radius=0.5, location=at)
    obj = bpy.context.object
    obj.scale = size
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    return obj


def box(size, at, tilt=None):
    bpy.ops.mesh.primitive_cube_add(size=1.0, location=at)
    obj = bpy.context.object
    obj.scale = size
    if tilt:
        obj.rotation_euler = tilt
    bpy.ops.object.transform_apply(location=False, rotation=True, scale=True)
    return obj


def rod(radius, length, at, tilt=None, sides=8):
    bpy.ops.mesh.primitive_cylinder_add(
        vertices=sides, radius=radius, depth=length, location=at
    )
    obj = bpy.context.object
    if tilt:
        obj.rotation_euler = tilt
        bpy.ops.object.transform_apply(location=False, rotation=True, scale=False)
    return obj


# How many sides a lofted ring has. Eight: enough to round off under subdivision,
# few enough that the cage is a shape somebody could have modelled by hand.
RING = 8


def loft(rings, name="part", close_bottom=True, close_top=True):
    """One continuous surface through a stack of rings.

    # This is the whole difference between a person and a doll

    The first body was separate primitives — a ball for a head, cylinders for arms,
    two lumps for a torso — and it read as a jointed doll however the numbers were
    tuned. It was not the proportions. It was that a body made of separate closed
    shapes has SEAMS everywhere a real one has a continuous surface, and the eye
    finds every one of them.

    So a limb or a torso is a single skin lofted through rings: give it the width,
    depth and height of each station and it bridges them into one hull. Subdivided
    once afterwards it comes out smooth and rounded, which is exactly how a
    stylised character is actually built — a low cage, then subdivision.

    Each ring is `(height, half_wide, half_deep)`.
    """
    places = []
    faces = []
    for up, half_wide, half_deep in rings:
        for step in range(RING):
            angle = step / RING * math.tau
            places.append(
                (math.cos(angle) * half_wide, math.sin(angle) * half_deep, up)
            )
    for level in range(len(rings) - 1):
        low = level * RING
        high = low + RING
        for step in range(RING):
            nxt = (step + 1) % RING
            faces.append((low + step, low + nxt, high + nxt, high + step))
    if close_bottom:
        faces.append(tuple(range(RING - 1, -1, -1)))
    if close_top:
        base = (len(rings) - 1) * RING
        faces.append(tuple(base + step for step in range(RING)))

    mesh = bpy.data.meshes.new(name)
    mesh.from_pydata(places, [], faces)
    mesh.update()
    obj = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(obj)
    return obj


def smooth_out(obj, levels=1):
    """Subdivides a cage into the rounded thing it was a cage for."""
    bpy.ops.object.select_all(action="DESELECT")
    obj.select_set(True)
    bpy.context.view_layer.objects.active = obj
    modifier = obj.modifiers.new(name="round", type="SUBSURF")
    modifier.levels = levels
    modifier.render_levels = levels
    bpy.ops.object.modifier_apply(modifier="round")
    return obj


def weld(parts, name):
    """Joins parts into one mesh under a known name. The name is the contract."""
    bpy.ops.object.select_all(action="DESELECT")
    for part in parts:
        part.select_set(True)
    bpy.context.view_layer.objects.active = parts[0]
    if len(parts) > 1:
        bpy.ops.object.join()
    whole = bpy.context.object
    whole.name = name
    whole.data.name = name
    return whole


# ------------------------------------------------------------------- the body


def person(build: str):
    """One body, as four meshes the game tints apart.

    Built from lofted hulls rather than stacked primitives — see [`loft`] for why
    that is the difference between a person and a doll. Limbs run INTO the torso
    rather than up to it, so there is no seam at a shoulder or a hip.
    """
    boot_top = 0.125
    hip = 0.87 if build == "male" else 0.855
    chest = 1.24 if build == "male" else 1.22
    neck_at = HEAD_AT - HEAD_HIGH * 0.5

    # Two presets, and this is all of the difference: a male tapers from a wide
    # shoulder to a narrow hip, a female the other way about and a little smaller.
    shoulder = 0.205 if build == "male" else 0.170
    waist = 0.145 if build == "male" else 0.135
    seat = 0.155 if build == "male" else 0.175
    deep = 0.105 if build == "male" else 0.098
    arm_out = shoulder + 0.035
    leg_out = seat * 0.52

    # --- the head, and a neck under it
    #
    # The head is its own ROUNDED VOLUME, not part of the neck's loft. Lofting
    # chest-neck-jaw-crown in one hull gave a cone: the jaw ring and the head ring
    # were near enough the same width, so there was no head in it at all — the face
    # came out as a long pale wedge with the hair sitting on top like a cap. A
    # cartoon head is a ball on a short neck, and the ball has to be a ball.
    #
    # A subdivided CUBE rather than a sphere: it rounds off to something with a
    # flatter face and a squarer crown, which is what a stylised head looks like,
    # and it keeps its poles out of the face where a sphere puts one.
    head = box((HEAD_WIDE, HEAD_DEEP, HEAD_HIGH), (0.0, 0.0, HEAD_AT))
    smooth_out(head, 2)
    neck = loft(
        [
            (chest - 0.04, 0.084, 0.074),
            (chest + 0.05, 0.064, 0.058),
            (neck_at + 0.055, 0.060, 0.055),
        ],
        "neck",
    )
    smooth_out(neck, 2)
    skin = [head, neck]
    for hand in (-1, 1):
        ear = blob((0.040, 0.062, 0.088), (hand * (HEAD_WIDE * 0.5 - 0.005), 0.018, HEAD_AT + 0.005), subdiv=1)
        skin.append(smooth_out(ear, 1))
        fist = blob((0.098, 0.092, 0.112), (hand * arm_out, 0.0, hip - 0.085))
        skin.append(fist)

    # --- the tunic: one hull from the hem to the shoulder
    torso = loft(
        [
            (hip - 0.13, seat * 0.96, deep * 0.98),
            (hip + 0.02, seat, deep),
            ((hip + chest) * 0.5, waist, deep * 0.88),
            (chest - 0.06, shoulder, deep * 1.02),
            (chest + 0.02, shoulder * 0.86, deep * 0.86),
        ],
        "torso",
    )
    smooth_out(torso, 2)
    clothes = [torso]
    for hand in (-1, 1):
        # A sleeve that starts INSIDE the shoulder and tapers to the wrist.
        arm = loft(
            [
                (chest - 0.02, 0.062, 0.058),
                (chest - 0.16, 0.055, 0.052),
                ((chest + hip) * 0.5 - 0.06, 0.046, 0.044),
                (hip - 0.06, 0.040, 0.039),
            ],
            "arm",
        )
        for point in arm.data.vertices:
            point.co.x += hand * arm_out
        smooth_out(arm, 1)
        clothes.append(arm)
        # A leg from inside the hem down to the boot.
        leg = loft(
            [
                (hip + 0.04, 0.072, 0.070),
                (hip - 0.22, 0.062, 0.060),
                ((hip + boot_top) * 0.5, 0.052, 0.051),
                (boot_top - 0.02, 0.048, 0.047),
            ],
            "leg",
        )
        for point in leg.data.vertices:
            point.co.x += hand * leg_out
        smooth_out(leg, 1)
        clothes.append(leg)
        # And a boot. NOT subdivided: a cube put through subdivision comes out a
        # ball, and the figures walked about on two spheres. A boot is the one stiff
        # thing on a soft body, so it keeps its corners — and it gets a toe, because
        # a foot that is as deep at the heel as at the toe reads as a brick.
        clothes.append(box((0.125, 0.20, boot_top), (hand * leg_out, -0.015, boot_top * 0.55)))
        clothes.append(box((0.115, 0.085, boot_top * 0.72), (hand * leg_out, -0.135, boot_top * 0.42)))

    # --- the eyes
    #
    # Big, simple and set well apart: a cartoon face is two eyes and nothing else,
    # and they have to carry the read from behind the follow camera.
    front = -HEAD_DEEP * 0.5
    sclera, eyes = [], []
    for hand in (-1, 1):
        # Set LOW on the head, not high: eyes sit about halfway down a face, and
        # placed above the middle they read as a forehead with dots on it.
        at = (hand * 0.070, front + 0.040, HEAD_AT - 0.010)
        sclera.append(blob((0.098, 0.062, 0.082), at))
        eyes.append(blob((0.055, 0.055, 0.058), (at[0], at[1] - 0.024, at[2] - 0.004)))

    return {
        "skin": (skin, 0.88, 1.0),
        "clothes": (clothes, 0.74, 1.0),
        "sclera": (sclera, 1.0, 1.0),
        "eyes": (eyes, 0.92, 1.0),
    }


# ------------------------------------------------------------------- the hair
#
# # No booleans
#
# Each style used to be a full cap with the face CUT out of it by a boolean. Two
# things went wrong with that, in order. The cutting boxes were positioned with
# fixed numbers, so making the head bigger left them in the wrong place and one
# style lost its whole cap — a bald figure. And when they were tied to the head
# properly, the boolean started returning nothing at all: the wig is several
# overlapping shells joined together, not one solid, and a difference against that
# is unreliable by nature.
#
# So nothing is cut. A wig is built to SIT BEHIND THE FACE in the first place —
# every piece is placed back and up from the head's middle, and the front of the
# cap stops short of the front of the head. That has no failure mode: it is the
# same arithmetic that places an ear.


def cap(grow=0.035, back=0.14, lift=0.16, tall=0.94):
    """The shell of hair over the crown, stopping short of the face.

    `back` and `lift` are shares of the head's depth and height, so this follows
    the head wherever the head goes.
    """
    return blob(
        (HEAD_WIDE + grow, HEAD_DEEP + grow * 0.6, HEAD_HIGH * tall),
        (0.0, HEAD_DEEP * back, HEAD_AT + HEAD_HIGH * lift),
    )


def crop():
    """Short, close to the head."""
    return [cap()], []


def bob():
    """A rounded bob to the jaw, with a fall either side of the face."""
    parts = [cap(grow=0.05, lift=0.14)]
    for hand in (-1, 1):
        parts.append(
            blob(
                (0.105, HEAD_DEEP * 0.92, HEAD_HIGH * 0.86),
                (hand * (HEAD_WIDE * 0.5 + 0.005), HEAD_DEEP * 0.06, HEAD_AT - HEAD_HIGH * 0.20),
            )
        )
    return parts, []


def tail():
    """Pulled back, with a tail behind."""
    parts = [
        cap(grow=0.03, lift=0.17),
        blob((0.115, 0.135, 0.135), (0.0, HEAD_DEEP * 0.5 + 0.02, HEAD_AT + HEAD_HIGH * 0.05)),
        rod(0.046, 0.36, (0.0, HEAD_DEEP * 0.5 + 0.055, HEAD_AT - HEAD_HIGH * 0.42), sides=7),
    ]
    return parts, []


def braids():
    """Two braids down either side, in beads so they read as plaited."""
    parts = [cap(grow=0.04, lift=0.15)]
    for hand in (-1, 1):
        for step, radius in ((0.0, 0.056), (0.135, 0.051), (0.26, 0.042)):
            parts.append(
                blob(
                    (radius * 2.0, radius * 2.0, 0.135),
                    (
                        hand * (HEAD_WIDE * 0.5 + 0.015),
                        HEAD_DEEP * 0.10,
                        HEAD_AT - HEAD_HIGH * 0.38 - step,
                    ),
                    subdiv=1,
                )
            )
    return parts, []


def curls():
    """A mass of curls, which is a mass of little balls — kept off the face."""
    parts = []
    for index in range(12):
        angle = index * 2.399
        ring = 0.55 + 0.45 * math.cos(index * 1.1)
        # Pushed back, so the ones that would land on the brow land on the crown.
        parts.append(
            blob(
                (0.155, 0.155, 0.155),
                (
                    math.cos(angle) * HEAD_WIDE * 0.44 * ring,
                    math.sin(angle) * HEAD_DEEP * 0.40 * ring + HEAD_DEEP * 0.16,
                    HEAD_AT + HEAD_HIGH * 0.30 + 0.045 * math.sin(index * 0.9),
                ),
                subdiv=1,
            )
        )
    return parts, []


HAIR = {"crop": crop, "bob": bob, "tail": tail, "braids": braids, "curls": curls}

SHARP_ABOVE = math.radians(62.0)


def build_body(build: str) -> None:
    fresh()
    made = person(build)
    welded = []
    for name, (parts, low, high) in made.items():
        for part in parts:
            bpy.ops.object.select_all(action="DESELECT")
            part.select_set(True)
            bpy.context.view_layer.objects.active = part
            bpy.ops.object.transform_apply(location=False, rotation=True, scale=False)
        whole = weld(parts, name)
        shade_in(whole, low, high, 0.0, TALL)
        welded.append(whole)

    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.shade_auto_smooth(angle=SHARP_ABOVE)

    # Standing on the floor, and standing exactly TALL.
    lowest = min(
        (obj.matrix_world @ mathutils.Vector(c)).z for obj in welded for c in obj.bound_box
    )
    for obj in welded:
        obj.location.z -= lowest
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.transform_apply(location=True, rotation=False, scale=False)

    here = os.path.dirname(os.path.abspath(__file__))
    bpy.ops.wm.save_as_mainfile(filepath=os.path.join(here, f"person_{build}.blend"))
    highest = max(
        (obj.matrix_world @ mathutils.Vector(c)).z for obj in welded for c in obj.bound_box
    )
    print(f"BUILT person_{build} — {highest:.2f} m, {len(welded)} meshes")


def build_hair(style: str) -> None:
    fresh()
    parts, _ = HAIR[style]()
    for part in parts:
        bpy.ops.object.select_all(action="DESELECT")
        part.select_set(True)
        bpy.context.view_layer.objects.active = part
        bpy.ops.object.transform_apply(location=False, rotation=True, scale=False)
    whole = weld(parts, "hair")
    # Ramped over the head rather than over the body: hair occupies thirty
    # centimetres, and a ramp measured over 1.8 m would leave it one flat tone.
    shade_in(whole, 0.78, 1.0, HEAD_AT - 0.30, HEAD_AT + HEAD_HIGH * 0.5)
    bpy.ops.object.select_all(action="DESELECT")
    whole.select_set(True)
    bpy.context.view_layer.objects.active = whole
    bpy.ops.object.shade_auto_smooth(angle=SHARP_ABOVE)

    here = os.path.dirname(os.path.abspath(__file__))
    bpy.ops.wm.save_as_mainfile(filepath=os.path.join(here, f"part_hair_{style}.blend"))
    print(f"BUILT part_hair_{style} — {len(whole.data.vertices)} vertices")


for one in BUILDS:
    build_body(one)
for one in STYLES:
    build_hair(one)
