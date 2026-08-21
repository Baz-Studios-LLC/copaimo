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

# The head: big, round, and the whole of the read. Its middle sits here in both
# builds, so a hairstyle authored once fits either.
#
# # The proportions are a genre, and they are specific
#
# A chibi character — the JRPG convention this world belongs to — is about four and
# a half heads tall, against seven and a half for a real adult. The eyes are the
# other half of it: roughly a THIRD of the face's width, taller than they are wide,
# set low, with a big iris and a dark pupil. There is no nose and no mouth.
#
# Everything before this was drifting toward a small-headed, small-eyed mannequin —
# five and a bit heads with eyes a fifth of the face. Those are realistic
# proportions worn by a stylised model, which is exactly what made it read as a
# doll.
HEAD_AT = 1.50
HEAD_HIGH = 0.40
HEAD_WIDE = 0.36
HEAD_DEEP = 0.34

# An eye, in metres. Found by building them in a live Blender and looking, which is
# the only way to settle a number whose whole job is how it reads.
EYE_WIDE = 0.100
EYE_TALL = 0.132

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


# What subdivision leaves of a cage, as a fraction of its size.
#
# # Measured, not guessed, and it explains three separate faults
#
# Subdivision pulls a cage IN toward its limit surface, and by a lot: a cube at
# level 2 comes out at 0.840 of its cage, an eight-sided loft at 0.821 to 0.837.
# Measured in a live Blender rather than reasoned about, because the number is what
# matters and it is not obvious.
#
# Everything in this file was written against the CAGE. So the head was 0.325 wide
# in the numbers and 0.273 in the world, and the eyes — placed against the numbers —
# sat about fifteen millimetres in FRONT of the real face. That is the "goggles"; the
# long neck and the gap at the shoulder are the same arithmetic in two other places.
#
# So a cage is built DIVIDED by this, and what comes out matches what is written.
# One constant, and every other number in the file becomes true.
SUBSURF_KEEPS = 0.835

# Vertically, a loft barely shrinks at all — 0.975 — because its end caps pin the
# top and bottom rings. So only the radius is compensated.
LOFT_KEEPS_TALL = 0.975


def grow(size):
    """A cage big enough that subdivision leaves the size actually wanted."""
    return tuple(part / SUBSURF_KEEPS for part in size)


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
    # Radially compensated, so a ring written as 0.205 comes out 0.205.
    # SORTED BY HEIGHT, and that is load-bearing.
    #
    # The side quads are wound on the assumption that each ring is above the last.
    # Hand the rings over top-to-bottom — which is the natural way to describe an
    # arm, from the shoulder down — and every quad is wound the other way, so the
    # whole hull is inside out. Backface culling then hides the near wall and shows
    # the lit interior of the far one, and the limb reads as TRANSLUCENT.
    #
    # It took a plain-white render with culling forced on to see it: the arms and
    # legs came out dark against a bright torso, which is the interior of a shell.
    # A test on one loft in isolation had passed, because the list I wrote for the
    # test happened to ascend.
    #
    # So the order stops mattering. Describing an arm downward is the natural way to
    # describe an arm.
    rings = sorted(rings, key=lambda ring: ring[0])
    # A ring is `(height, half_wide, half_deep)`, or the same with a fourth number:
    # how far FORWARD it sits. A stack of concentric rings can only ever be a tube,
    # and a foot is the one part of a body that is obviously longer than it is wide.
    rings = [
        (
            ring[0],
            ring[1] / SUBSURF_KEEPS,
            ring[2] / SUBSURF_KEEPS,
            ring[3] if len(ring) > 3 else 0.0,
        )
        for ring in rings
    ]
    for up, half_wide, half_deep, ahead in rings:
        for step in range(RING):
            angle = step / RING * math.tau
            places.append(
                (
                    math.cos(angle) * half_wide,
                    math.sin(angle) * half_deep + ahead,
                    up,
                )
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


def boot(at_x: float, tall: float):
    """A boot: a sole, an upper with a toe, and an ankle.

    # A block is not a foot

    These were two boxes — a slab and a smaller slab for a toe — and from the game
    camera the figure walked about on bricks. A foot is obviously LONGER than it is
    wide, fatter at the toe than at the heel, and has a sole under it. None of those
    survive being a cube.

    Lofted up its height with each ring shifted forward, so the sole reaches out past
    the ankle into a toe and the upper narrows back over it.
    """
    stack = loft(
        [
            # (height, half-width, half-length, how far forward)
            (0.000, 0.058, 0.100, -0.020),
            (0.030, 0.063, 0.110, -0.022),
            (tall * 0.55, 0.057, 0.090, -0.006),
            (tall * 1.00, 0.058, 0.064, 0.012),
            # A CUFF, wider than the leg above it and reaching up past the ankle.
            # Narrower than the leg, the boot let the shin taper into a spike and
            # there was a visible pinch at every ankle.
            (tall * 1.42, 0.064, 0.062, 0.014),
        ],
        "boot",
    )
    for point in stack.data.vertices:
        point.co.x += at_x
    return smooth_out(stack, 1)


def mitt(hand: int, at_x: float, at_z: float):
    """A hand: a flattened mitten with a thumb, not a ball.

    A sphere on the end of a sleeve reads as a ball, because that is what it is. A
    hand is FLAT — much wider than deep — it widens at the knuckles and comes back in
    at the fingertips, and it has a thumb on the inside. Four rings and one lump
    carries all of that at the size it is drawn.
    """
    stack = loft(
        [
            (at_z - 0.120, 0.036, 0.019),
            (at_z - 0.086, 0.052, 0.024),
            (at_z - 0.040, 0.055, 0.026),
            (at_z + 0.010, 0.044, 0.024),
        ],
        "hand",
    )
    # Forward of the palm as well as inside of it, and big enough to see: a thumb
    # tucked against the side is a bump nobody reads.
    thumb = blob(
        (0.032, 0.030, 0.058),
        (-hand * 0.046, -0.020, at_z - 0.048),
        subdiv=1,
    )
    for part in (stack, thumb):
        for point in part.data.vertices:
            point.co.x += at_x
    smooth_out(stack, 1)
    return weld([stack, thumb], "hand")


# --------------------------------------------------------------------- the rig
#
# # Why a skeleton, and why now
#
# Nothing in the world moves yet, and a static figure needs no bones. But every
# part of the body was just rebuilt, and the ONE thing that decides whether a rig
# is any good is where the joints sit relative to the shapes — an elbow bone in the
# middle of a forearm creases the sleeve wherever it bends. Rigging the figure while
# the shapes are fresh means the joints are placed against the geometry that exists,
# not against remembered numbers.
#
# Bevy reads glTF skins on its own: a rigged mesh arrives as a `SkinnedMesh` with no
# game code at all. What it does NOT bring is animation — that is clips and an
# `AnimationPlayer`, and it comes next.

# Which bones a part belongs to, and at what height each takes over.
#
# Keyed by object name, filled as `person` builds each piece.
CHAINS: dict = {}

# How many bones may claim one vertex. Two: a vertex lies between the bone above it
# and the bone below it, and nothing else has any business moving it.
MOST_BONES = 2


def chain_of(part, links) -> None:
    """Records which bones own a part, as `(bone, height)` from the bottom up."""
    CHAINS[part.name] = links


def weigh(part) -> None:
    """Weights one part to its own bones, blending along its length.

    # Distance to a bone is the wrong instrument

    The first attempt weighted every vertex by inverse distance to the nearest few
    bones, over the whole skeleton. Posed, the torso TORE: a sheet of it stretched
    from the chest down past the hip, because a vertex on the front of the belly is
    genuinely nearer to a thigh bone than to the spine, and nothing in the rule said
    otherwise. A shoulder dragged the chest for the same reason.

    But the part a vertex belongs to is not a guess here — every piece of this body
    is built by name. A vertex in the left sleeve is owned by the left arm and by
    nothing else, whatever it happens to be near. So the chain says which bones may
    claim a part at all, and the vertex's HEIGHT along that chain says how the claim
    is shared between the two it lies between.

    Assigned before the parts are welded, because after the weld there is no way to
    tell which vertex came from which piece. Blender merges vertex groups by name on
    join, so the weights survive it.
    """
    links = CHAINS.get(part.name)
    if not links:
        return
    groups = {}
    for bone, _ in links:
        if bone not in groups:
            groups[bone] = part.vertex_groups.new(name=bone)
    place = part.matrix_world
    for point in part.data.vertices:
        up = (place @ point.co).z
        # Below the first link or above the last: all of it to that end.
        if up <= links[0][1]:
            groups[links[0][0]].add([point.index], 1.0, "REPLACE")
            continue
        if up >= links[-1][1]:
            groups[links[-1][0]].add([point.index], 1.0, "REPLACE")
            continue
        for (lower, at_low), (upper, at_high) in zip(links, links[1:]):
            if at_low <= up <= at_high:
                span = max(at_high - at_low, 1.0e-5)
                share = (up - at_low) / span
                groups[lower].add([point.index], 1.0 - share, "REPLACE")
                groups[upper].add([point.index], share, "REPLACE")
                break


def bone_plan(build: str):
    """Every bone, as `(name, parent, head, tail)` in world metres.

    Placed against the body's own numbers — the same `hip`, `chest` and `HEAD_AT` the
    shapes are built from — so a joint cannot drift away from the shape it bends.
    """
    marks = body_marks(build)
    hip, chest = marks["hip"], marks["chest"]
    arm_out, leg_out = marks["arm_out"], marks["leg_out"]
    boot_top = marks["boot_top"]

    bones = [
        ("hips", None, (0.0, 0.0, hip), (0.0, 0.0, hip + 0.14)),
        ("spine", "hips", (0.0, 0.0, hip + 0.14), (0.0, 0.0, chest - 0.06)),
        ("chest", "spine", (0.0, 0.0, chest - 0.06), (0.0, 0.0, chest + 0.04)),
        ("neck", "chest", (0.0, 0.0, chest + 0.04), (0.0, 0.0, HEAD_AT - 0.14)),
        ("head", "neck", (0.0, 0.0, HEAD_AT - 0.14), (0.0, 0.0, HEAD_AT + 0.20)),
    ]
    for side, hand in (("l", 1), ("r", -1)):
        elbow, wrist = marks["elbow"], marks["wrist"]
        knee, ankle = marks["knee"], marks["ankle"]
        bones += [
            (
                f"arm.{side}",
                "chest",
                (hand * arm_out, 0.0, chest - 0.03),
                (hand * arm_out, 0.0, elbow),
            ),
            (
                f"forearm.{side}",
                f"arm.{side}",
                (hand * arm_out, 0.0, elbow),
                (hand * arm_out, 0.0, wrist),
            ),
            (
                f"hand.{side}",
                f"forearm.{side}",
                (hand * arm_out, 0.0, wrist),
                (hand * arm_out, 0.0, wrist - 0.10),
            ),
            (
                f"thigh.{side}",
                "hips",
                (hand * leg_out, 0.0, hip),
                (hand * leg_out, 0.0, knee),
            ),
            (
                f"shin.{side}",
                f"thigh.{side}",
                (hand * leg_out, 0.0, knee),
                (hand * leg_out, 0.0, ankle),
            ),
            (
                f"foot.{side}",
                f"shin.{side}",
                (hand * leg_out, 0.0, ankle),
                (hand * leg_out, -0.14, 0.02),
            ),
        ]
    return bones


def rig(parts, build: str, lift: float):
    """Builds the skeleton and binds the already-weighted parts to it.

    `lift` is how far the figure was moved to seat it on the floor, applied to every
    bone: the bones are written in world metres against the design's own numbers, and
    a body seated after the plan was made would leave its skeleton behind.
    """
    frame = bpy.data.armatures.new("skeleton")
    rigged = bpy.data.objects.new("skeleton", frame)
    bpy.context.collection.objects.link(rigged)
    bpy.context.view_layer.objects.active = rigged
    bpy.ops.object.mode_set(mode="EDIT")
    made = {}
    for name, parent, head, tail in bone_plan(build):
        bone = frame.edit_bones.new(name)
        # The same half circle `face_forward` gives the meshes: about the up axis,
        # so x and y both negate. Done here rather than by transforming the armature
        # afterwards, because a bone's rest pose is what the weights were computed
        # against and moving it later would shear the whole figure.
        bone.head = (-head[0], -head[1], head[2] + lift)
        bone.tail = (-tail[0], -tail[1], tail[2] + lift)
        if parent:
            bone.parent = made[parent]
        made[name] = bone
    bpy.ops.object.mode_set(mode="OBJECT")

    for obj in parts:
        obj.parent = rigged
        skin = obj.modifiers.new(name="skin", type="ARMATURE")
        skin.object = rigged
    return rigged


# ------------------------------------------------------------------ the motions
#
# # A rig with no clip is a figure that slides
#
# The skeleton was built and nothing moved it, so the warden slid about the world
# like a chess piece. These are the clips that fix that, authored here for the same
# reason the shapes are: a walk is a set of numbers, and numbers belong in a file
# that can be read and diffed.
#
# Twenty-four frames to a cycle at twenty-four a second, so one stride is one
# second and the maths stays legible. Four keys: contact, passing, contact,
# passing — the shape of every walk ever animated. Bevy plays the clip and scales
# its speed by how fast the warden is actually going, so the feet keep up.

# How far a limb swings, in degrees.
STRIDE = 26.0
KNEE_BEND = 34.0
ARM_SWING = 20.0
ELBOW_BEND = 22.0


def curves_of(action):
    """Every F-curve in an action, whichever Action system Blender is using.

    Blender 4.4 replaced `action.fcurves` with layers, strips and channelbags, and
    5.x has only the new one — so reaching for `fcurves` is an AttributeError rather
    than an empty list, which reads as a broken script rather than a moved API.
    """
    if hasattr(action, "fcurves"):
        return list(action.fcurves)
    found = []
    for layer in action.layers:
        for strip in layer.strips:
            for bag in getattr(strip, "channelbags", []):
                found.extend(bag.fcurves)
    return found


def ease(action) -> None:
    """Smooths every key in an action, so a walk is not a set of lurches."""
    for curve in curves_of(action):
        for point in curve.keyframe_points:
            point.interpolation = "BEZIER"


def keyed(rig, bone: str, frame: int, pitch=0.0, roll=0.0, yaw=0.0) -> None:
    """Sets one bone's rotation on one frame."""
    posed = rig.pose.bones[bone]
    posed.rotation_mode = "XYZ"
    posed.rotation_euler = (
        math.radians(pitch),
        math.radians(roll),
        math.radians(yaw),
    )
    posed.keyframe_insert(data_path="rotation_euler", frame=frame)


def keyed_at(rig, bone: str, frame: int, up: float) -> None:
    """Sets one bone's position on one frame, for the body's own bob."""
    posed = rig.pose.bones[bone]
    posed.location = (0.0, 0.0, up)
    posed.keyframe_insert(data_path="location", frame=frame)


def walk_cycle(rig) -> None:
    """A walk: two strides, opposite limbs, and a bob at each footfall.

    The bob is what sells it. Without a body that rises and falls, a walk reads as
    a figure whose legs move while it glides — which is most of the way back to
    sliding. It falls TWICE per cycle, once on each foot, so it is at twice the
    frequency of the legs.
    """
    action = bpy.data.actions.new("walk")
    rig.animation_data_create()
    rig.animation_data.action = action

    # (frame, which leg is forward)
    for frame, lead in ((1, 1), (13, -1), (25, 1)):
        for side_of, hand in (("l", 1), ("r", -1)):
            forward = lead * hand
            # Contact: one leg reaching out, the other trailing behind.
            keyed(rig, f"thigh.{side_of}", frame, pitch=-STRIDE * forward)
            keyed(rig, f"shin.{side_of}", frame, pitch=KNEE_BEND * 0.30 * (1 - forward) * 0.5)
            # Arms go with the OPPOSITE leg, which is what stops a walk looking
            # like a wind-up toy.
            keyed(rig, f"arm.{side_of}", frame, pitch=ARM_SWING * forward)
            keyed(rig, f"forearm.{side_of}", frame, pitch=-ELBOW_BEND * 0.6)
        keyed(rig, "spine", frame, pitch=2.0)
        keyed_at(rig, "hips", frame, 0.0)

    # (frame, which leg is passing under the body)
    for frame, lead in ((7, 1), (19, -1)):
        for side_of, hand in (("l", 1), ("r", -1)):
            passing = lead * hand
            # Passing: the swinging leg is under the hips with a bent knee, the
            # standing one straight and taking the weight.
            keyed(rig, f"thigh.{side_of}", frame, pitch=STRIDE * 0.35 * passing)
            keyed(rig, f"shin.{side_of}", frame, pitch=KNEE_BEND * max(0.0, passing))
            keyed(rig, f"arm.{side_of}", frame, pitch=-ARM_SWING * 0.30 * passing)
            keyed(rig, f"forearm.{side_of}", frame, pitch=-ELBOW_BEND)
        keyed(rig, "spine", frame, pitch=3.5)
        # Highest as the body passes over the standing leg.
        keyed_at(rig, "hips", frame, 0.022)

    ease(action)
    return action


def idle_cycle(rig) -> None:
    """Standing: a slow breath, so a stopped warden is not a statue."""
    action = bpy.data.actions.new("idle")
    rig.animation_data.action = action
    for frame, rise in ((1, 0.0), (36, 0.006), (72, 0.0)):
        keyed_at(rig, "hips", frame, rise)
        keyed(rig, "spine", frame, pitch=1.0 + rise * 90.0)
        keyed(rig, "head", frame, pitch=-rise * 60.0)
    ease(action)
    return action


def animate(rig) -> None:
    """Puts every clip on the rig, and leaves it holding the walk.

    Both actions are stashed in an NLA track apiece: the glTF exporter writes ONE
    clip per track, and an action that is merely present in the file — not on a
    track and not assigned — is not exported at all. That is the whole trick, and
    it is the sort of thing that looks like a broken exporter.
    """
    for make in (walk_cycle, idle_cycle):
        action = make(rig)
        track = rig.animation_data.nla_tracks.new()
        track.name = action.name
        track.strips.new(action.name, 1, action)
        rig.animation_data.action = None

    # BACK TO REST, and this is not tidiness.
    #
    # `keyframe_insert` sets the value as well as recording it, so authoring a clip
    # leaves the rig standing in whatever its last keyframe said — hips lifted, spine
    # pitched. The .blend is saved in that pose and the mesh evaluates deformed, so
    # the export gate refused both bodies for floating four centimetres off the
    # floor. It was right to: a figure that hovers in the file hovers in the game.
    #
    # And the refusal is why the GLBs were stale rather than animated — the clips
    # were being written correctly the whole time and never reaching the game.
    for posed in rig.pose.bones:
        posed.location = (0.0, 0.0, 0.0)
        posed.rotation_euler = (0.0, 0.0, 0.0)
        posed.rotation_quaternion = (1.0, 0.0, 0.0, 0.0)
        posed.scale = (1.0, 1.0, 1.0)


# ------------------------------------------------------------------- the body


def body_marks(build: str) -> dict:
    """Every landmark of a body, in world metres.

    # One set of numbers for the shapes AND the bones

    The shapes are built from these and so is the skeleton. Written twice they would
    drift — an elbow bone a couple of centimetres off the sleeve's own middle ring
    creases the sleeve wherever it bends, and nothing about the model would look
    wrong until it was posed. This project has met that shape of bug enough times to
    know not to invite it.

    Higher hips and a higher chest than a realistic figure: the body is SHORT because
    the head is big, and a chibi has almost no neck to speak of.
    """
    hip = 0.80 if build == "male" else 0.79
    chest = 1.22 if build == "male" else 1.20
    # Two presets, and this is all of the difference: a male tapers from a wide
    # shoulder to a narrow hip, a female the other way about and a little smaller.
    shoulder = 0.190 if build == "male" else 0.162
    seat = 0.158 if build == "male" else 0.172
    boot_top = 0.135
    return {
        "boot_top": boot_top,
        "hip": hip,
        "chest": chest,
        "neck_at": HEAD_AT - HEAD_HIGH * 0.5,
        "shoulder": shoulder,
        "waist": 0.150 if build == "male" else 0.142,
        "seat": seat,
        "deep": 0.112 if build == "male" else 0.104,
        # Just OUTSIDE the real shoulder. Two mistakes were made here in turn: at
        # shoulder + 0.035 the arms hung clear of the body as separate tubes, and at
        # shoulder - 0.012 — with thicker chibi sleeves — they merged into it and the
        # figure came out as one wide mass with no arms in it. A sleeve wants to
        # touch the torso and still be a sleeve.
        "arm_out": shoulder + 0.022,
        "leg_out": seat * 0.52,
        # The joints, each at the middle ring of the shape it bends.
        "elbow": (chest + hip) * 0.5 - 0.04,
        "wrist": hip - 0.045,
        "knee": (hip + boot_top) * 0.5,
        "ankle": boot_top * 0.9,
    }


def side(hand: int) -> str:
    """Which side of the body a `hand` of 1 or -1 belongs to.

    # Why +X is the RIGHT side here

    Everything in this file builds its front on -Y, because that is the direction
    Blender's own front view looks from and it is what feels natural to model
    toward. The glTF Y-up conversion turns Blender -Y into +Z, and the game's
    forward is -Z — so every figure came out walking backwards.

    So the finished figure is turned half a circle about its up axis (see
    `face_forward`), which also carries +X round to -X. A part built at +X therefore
    ends up on the model's own LEFT once it is turned, and the name has to say so
    here rather than lie about it downstream.
    """
    return "r" if hand > 0 else "l"


def person(build: str):
    """One body, as five meshes the game tints apart.

    Built from lofted hulls rather than stacked primitives — see [`loft`] for why
    that is the difference between a person and a doll. Limbs run INTO the torso
    rather than up to it, so there is no seam at a shoulder or a hip.

    Each part is also tagged with the bones that own it — see [`weigh`] — because
    after the parts are welded there is no telling which vertex came from which.
    """
    marks = body_marks(build)
    boot_top = marks["boot_top"]
    hip = marks["hip"]
    chest = marks["chest"]
    neck_at = marks["neck_at"]
    shoulder = marks["shoulder"]
    waist = marks["waist"]
    seat = marks["seat"]
    deep = marks["deep"]
    arm_out = marks["arm_out"]
    leg_out = marks["leg_out"]

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
    head = box(grow((HEAD_WIDE, HEAD_DEEP, HEAD_HIGH)), (0.0, 0.0, HEAD_AT))
    smooth_out(head, 2)
    neck = loft(
        [
            (chest - 0.04, 0.084, 0.074),
            (chest + 0.05, 0.064, 0.058),
            (neck_at + 0.055, 0.060, 0.055),
        ],
        "neck",
    )
    smooth_out(neck, 1)
    chain_of(head, [("head", 0.0)])
    chain_of(neck, [("chest", chest - 0.04), ("neck", chest + 0.07), ("head", neck_at)])
    skin = [head, neck]
    for hand in (-1, 1):
        ear = blob((0.040, 0.062, 0.088), (hand * (HEAD_WIDE * 0.5 - 0.005), 0.018, HEAD_AT + 0.005), subdiv=1)
        chain_of(ear, [("head", 0.0)])
        skin.append(smooth_out(ear, 1))
        fist = mitt(hand, hand * arm_out, hip - 0.045)
        chain_of(fist, [(f"hand.{side(hand)}", 0.0)])
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
    smooth_out(torso, 1)
    # The trunk blends up through the spine, and NOTHING below the hip may claim
    # it: weighting by distance let a thigh drag the belly and tore the torso open.
    chain_of(
        torso,
        [("hips", hip - 0.10), ("spine", hip + 0.16), ("chest", chest - 0.04)],
    )
    clothes = [torso]
    for hand in (-1, 1):
        # A sleeve that starts INSIDE the shoulder and tapers to the wrist.
        arm = loft(
            [
                (chest - 0.02, 0.076, 0.072),
                (chest - 0.14, 0.068, 0.065),
                ((chest + hip) * 0.5 - 0.04, 0.058, 0.056),
                # Into the HAND, not down to it: a closed cap pulls inward
                # under subdivision and showed as a spike above every wrist.
                (hip - 0.105, 0.048, 0.047),
            ],
            "arm",
        )
        for point in arm.data.vertices:
            point.co.x += hand * arm_out
        smooth_out(arm, 1)
        chain_of(
            arm,
            [
                (f"hand.{side(hand)}", marks["wrist"] - 0.07),
                (f"forearm.{side(hand)}", marks["wrist"]),
                (f"arm.{side(hand)}", marks["elbow"]),
                ("chest", chest - 0.01),
            ],
        )
        clothes.append(arm)
        # A leg from inside the hem down to the boot.
        leg = loft(
            [
                (hip + 0.04, 0.086, 0.084),
                (hip - 0.20, 0.076, 0.074),
                ((hip + boot_top) * 0.5, 0.064, 0.063),
                # DEEP inside the boot, not at its rim. Subdivision pulls a closed
            # cap inward along the loft's axis as well as radially, so a leg whose
            # last ring sat level with the boot's top lifted clear of it and the
            # tapered cap showed as a spike above the ankle. Same arithmetic as the
            # radial shrink, in the direction nobody thinks about.
            (boot_top * 0.35, 0.052, 0.051),
            ],
            "leg",
        )
        for point in leg.data.vertices:
            point.co.x += hand * leg_out
        smooth_out(leg, 1)
        chain_of(
            leg,
            [
                (f"foot.{side(hand)}", marks["ankle"] - 0.04),
                (f"shin.{side(hand)}", marks["ankle"] + 0.03),
                (f"thigh.{side(hand)}", marks["knee"]),
                ("hips", hip + 0.02),
            ],
        )
        clothes.append(leg)
        # And a boot. NOT subdivided: a cube put through subdivision comes out a
        # ball, and the figures walked about on two spheres. A boot is the one stiff
        # thing on a soft body, so it keeps its corners — and it gets a toe, because
        # a foot that is as deep at the heel as at the toe reads as a brick.
        shoe = boot(hand * leg_out, boot_top)
        chain_of(shoe, [(f"foot.{side(hand)}", 0.0)])
        clothes.append(shoe)

    # --- the eyes
    #
    # Three parts, because an eye needs three colours: a white, an iris the player
    # chooses, and a pupil that is always dark. Two spheres was the earlier attempt
    # and it read as goggles — a ball stuck on a face rather than an eye in it.
    #
    # Proud of the face by a few millimetres rather than sunk into it: sunk, they
    # came out as pinholes. Flattened hard in depth so each is a DISC with a slight
    # dome, which is what a stylised eye is.
    front = -HEAD_DEEP * 0.5
    sclera, eyes, pupils = [], [], []
    for hand in (-1, 1):
        x = hand * 0.070
        z = HEAD_AT - 0.030
        white = blob((EYE_WIDE, 0.052, EYE_TALL), (x, front + 0.028, z))
        iris = blob(
            (EYE_WIDE * 0.66, 0.044, EYE_TALL * 0.60),
            (x, front + 0.014, z - EYE_TALL * 0.14),
        )
        dot = blob(
            (EYE_WIDE * 0.30, 0.038, EYE_TALL * 0.28),
            (x, front + 0.004, z - EYE_TALL * 0.16),
        )
        # All three ride the head and nothing else, or a shoulder pulls an eye
        # out of its socket.
        for part in (white, iris, dot):
            chain_of(part, [("head", 0.0)])
        sclera.append(white)
        eyes.append(iris)
        pupils.append(dot)

    return {
        "skin": (skin, 0.88, 1.0),
        "clothes": (clothes, 0.74, 1.0),
        # The white, the iris and the pupil are each flat: they are small enough
        # that a gradient across one is shading a dot.
        "sclera": (sclera, 1.0, 1.0),
        "eyes": (eyes, 1.0, 1.0),
        "pupil": (pupils, 1.0, 1.0),
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


# --------------------------------------------------------------------- the hats
#
# A hat is its OWN slot, not a hairstyle. Somebody wearing a cap still has hair
# under it, so the two have to be separate things the game can pick independently —
# and a hat is authored to sit over a wig rather than instead of one.


def baseball_cap():
    """A baseball cap: a crown over the head and a peak out the front.

    Named in full because `cap` is already taken in this file — it is the wig shell
    every hairstyle is built on, and defining a second `cap` here quietly replaced
    it, so every hairstyle started returning a list of lists.

    Sized a little larger than the widest wig so it sits over hair rather than
    inside it. The peak is the whole of the read from any distance — a crown alone
    is a swimming cap — so it is wide, and it tips DOWN, because a peak parallel to
    the ground reads as a plate.
    """
    parts = [
        # The crown: over the top and the back, stopping short of the brow.
        blob(
            (HEAD_WIDE + 0.055, HEAD_DEEP + 0.050, HEAD_HIGH * 0.78),
            (0.0, HEAD_DEEP * 0.05, HEAD_AT + HEAD_HIGH * 0.20),
        ),
        # The button on the crown, which is a cap's one piece of detail.
        blob((0.032, 0.032, 0.026), (0.0, HEAD_DEEP * 0.05, HEAD_AT + HEAD_HIGH * 0.50), subdiv=1),
    ]
    # The peak.
    #
    # A flattened ELLIPSOID, not a loft. The loft stacks its rings along Z, so what
    # came out was a thin strip across the head — 33 cm wide and 4 cm deep — where a
    # peak wants the opposite: wide across the brow and reaching FORWARD, thin
    # vertically. It read as a beak.
    #
    # Tipped down about fifteen degrees, because a peak parallel to the ground is a
    # plate and the shadow it casts is the whole point of one.
    peak = blob((0.290, 0.185, 0.034), (0.0, 0.0, 0.0), subdiv=2)
    lean = math.radians(-15.0)
    for point in peak.data.vertices:
        y, z = point.co.y, point.co.z
        point.co.y = y * math.cos(lean) - z * math.sin(lean) - HEAD_DEEP * 0.46
        point.co.z = y * math.sin(lean) + z * math.cos(lean) + HEAD_AT + HEAD_HIGH * 0.085
    parts.append(peak)
    return parts


HATS = {"cap": baseball_cap}

SHARP_ABOVE = math.radians(62.0)


def seat_on_floor(objects) -> float:
    """Drops everything so the lowest EVALUATED vertex is at Z=0.

    # bound_box does not know about modifiers

    Seating used `object.bound_box`, which is the mesh as authored — before the
    subdivision that pulls a closed cap inward, and before the smooth-by-angle
    modifier Blender 5 implements as geometry nodes. So the figure was seated against
    a surface that is not the one that gets drawn, and the export gate refused both
    bodies for floating four centimetres. The gate was right, and its refusal is why
    the animated models never reached the game: the export had been failing all along
    while the clips were being written correctly.

    Measured through the depsgraph instead, which is the geometry as it will actually
    be exported — and done LAST, with the armature moved by the same amount, because
    a pure translation of bones and mesh together is safe for a rig.
    """
    # The graph is flushed FIRST. Authoring the clips left the rig posed and the
    # pose was then cleared, but the depsgraph still held the deformed figure — so
    # this measured a lowered foot and over-corrected, burying the warden four
    # centimetres. Clearing a pose does not re-evaluate anything on its own.
    bpy.context.view_layer.update()
    depsgraph = bpy.context.evaluated_depsgraph_get()
    lowest = None
    for obj in objects:
        if obj.type != "MESH":
            continue
        evaluated = obj.evaluated_get(depsgraph)
        mesh = evaluated.to_mesh()
        for point in mesh.vertices:
            up = (obj.matrix_world @ point.co).z
            lowest = up if lowest is None else min(lowest, up)
        evaluated.to_mesh_clear()
    if lowest is None or abs(lowest) < 1.0e-5:
        return 0.0
    # Into the GEOMETRY, and the caller puts the same shift into the bones.
    #
    # Moving the armature OBJECT instead looks like it works and does not: the glTF
    # spec says the node transform of a skinned mesh is IGNORED, because such a mesh
    # is placed entirely by its joints and their inverse bind matrices. So the
    # skeleton node carried a -0.038 translation, Blender honoured it, the exporter
    # wrote it out faithfully, and the game ignored it exactly as it should — leaving
    # the warden buried to the ankles.
    for obj in objects:
        if obj.type != "MESH" or obj.parent is not None:
            continue
        for point in obj.data.vertices:
            point.co.z -= lowest
    return lowest


def face_forward(objects) -> None:
    """Turns a finished figure half a circle, so its front is the game's forward.

    Blender -Y becomes glTF +Z and the game's forward is -Z, so a figure modelled
    toward the front view faces backwards in the world. Turning it here rather than
    at spawn keeps the rule in one place: a model faces -Z, full stop, and nothing
    downstream needs to know which way it was authored.
    """
    turn = mathutils.Matrix.Rotation(math.pi, 4, "Z")
    for obj in objects:
        obj.matrix_world = turn @ obj.matrix_world
    bpy.ops.object.select_all(action="DESELECT")
    for obj in objects:
        obj.select_set(True)
    if objects:
        bpy.context.view_layer.objects.active = objects[0]
        bpy.ops.object.transform_apply(location=True, rotation=True, scale=False)


def build_body(build: str) -> None:
    fresh()
    made = person(build)
    welded = []
    for name, (parts, low, high) in made.items():
        for part in parts:
            # BEFORE the weld: afterwards there is no telling which vertex came
            # from which piece. Blender merges vertex groups by name on join.
            weigh(part)
            bpy.ops.object.select_all(action="DESELECT")
            part.select_set(True)
            bpy.context.view_layer.objects.active = part
            bpy.ops.object.transform_apply(location=False, rotation=True, scale=False)
        whole = weld(parts, name)
        shade_in(whole, low, high, 0.0, TALL)
        welded.append(whole)

    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.shade_auto_smooth(angle=SHARP_ABOVE)

    # Nothing is seated yet: that happens once the rig exists, so the bones can be
    # moved with the body. See `seat_on_floor`.
    lowest = 0.0

    # The skeleton LAST — after the figure has been dropped onto the floor. Bones
    # are written in world metres, and a body moved after rigging leaves its
    # skeleton standing where it used to be.
    # Turned BEFORE the rig, while the meshes are still unparented: once they are
    # children of an armature, applying a transform to one does nothing. The bones
    # are turned by the same half circle inside `rig`.
    # Seated while the meshes are still unparented, so the offset can go into their
    # vertices — and the same offset goes into the bones.
    lowest = seat_on_floor(welded)
    face_forward(welded)
    skeleton = rig(welded, build, -lowest)
    # And the clips, so the figure walks rather than slides.
    # SEATED BEFORE THE CLIPS EXIST, and that ordering is the fix.
    #
    # An NLA track plays by default, so once the walk was on one the evaluated mesh
    # was posed mid-stride — a leg out front, which read as a figure 0.93 m deep and
    # a foot 0.037 m below the floor. Muting tracks to measure would work; not having
    # any yet is simpler and cannot be forgotten.
    animate(skeleton)

    here = os.path.dirname(os.path.abspath(__file__))
    bpy.ops.wm.save_as_mainfile(filepath=os.path.join(here, f"person_{build}.blend"))
    highest = max(
        (obj.matrix_world @ mathutils.Vector(c)).z for obj in welded for c in obj.bound_box
    )
    print(f"BUILT person_{build} — {highest:.2f} m, {len(welded)} meshes")


def build_hat(style: str) -> None:
    """Builds one hat. Same shading and naming rules as hair — see `build_hair`."""
    fresh()
    parts = HATS[style]()
    for part in parts:
        bpy.ops.object.select_all(action="DESELECT")
        part.select_set(True)
        bpy.context.view_layer.objects.active = part
        bpy.ops.object.transform_apply(location=False, rotation=True, scale=False)
    whole = weld(parts, "hat")
    shade_in(whole, 0.74, 1.0, HEAD_AT - 0.24, HEAD_AT + HEAD_HIGH * 0.55)
    bpy.ops.object.select_all(action="DESELECT")
    whole.select_set(True)
    bpy.context.view_layer.objects.active = whole
    bpy.ops.object.shade_auto_smooth(angle=SHARP_ABOVE)
    face_forward([whole])

    here = os.path.dirname(os.path.abspath(__file__))
    bpy.ops.wm.save_as_mainfile(filepath=os.path.join(here, f"part_hat_{style}.blend"))
    print(f"BUILT part_hat_{style} — {len(whole.data.vertices)} vertices")


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
    face_forward([whole])

    here = os.path.dirname(os.path.abspath(__file__))
    bpy.ops.wm.save_as_mainfile(filepath=os.path.join(here, f"part_hair_{style}.blend"))
    print(f"BUILT part_hair_{style} — {len(whole.data.vertices)} vertices")


for one in BUILDS:
    build_body(one)
for one in STYLES:
    build_hair(one)
for one in HATS:
    build_hat(one)
