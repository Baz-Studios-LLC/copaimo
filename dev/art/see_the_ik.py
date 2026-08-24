"""Poses the real rig's leg to whatever the GAME's IK solver worked out, in a Blender window.

    dev/art/see_the_ik.sh              # open it in Blender, one frame per case
    dev/art/see_the_ik.sh --stills     # render PNGs instead

The solver lives in `src/ik.rs` and only there. This does not re-implement it - a second copy in
Python would be a different solver, and two implementations of the same arithmetic agreeing
proves nothing about either. Rust computes; this poses and MEASURES BACK.

That measurement is the point. Turning solved POSITIONS into bone ROTATIONS is the part of a
runtime IK that actually goes wrong - local space against world space, and which axis a bone
runs along - and it is invisible in the solver's own tests because they never touch a rig. So
each case is posed, the ankle's real position read off the posed armature, and the distance from
where the solver put it printed. Anything but ~0 means the conversion is wrong even though the
arithmetic was right. It was 2.282 cm the first time.

# One frame per case, so it can be scrubbed

The cases are keyed as frames on the rig rather than rendered to separate images, because
separate images mean holding two pictures in your head to compare them - and doing that wrong is
how "the mesh is not following the bones" got said about a mesh that was following perfectly.
Interpolation is CONSTANT: each frame is a discrete case, not a stage in a motion.

Built headless and saved, then opened, for the reason `gait_watch.py` documents at length: the
glTF importer dies on the context a GUI gets during startup, so a window built by `--python` at
launch comes up empty with the failure buried in the console.

# The frame of reference

The JSON is HIP-RELATIVE and its axes are NAMED - up, forward, across - because Blender is Z-up
and the game is Y-up, and a conversion between them is the most expensive class of mistake this
project has made. Each point is rebuilt here from the rig's own measured axes via
`prepare_rig.body_frame`, so there is no conversion to get backwards.

Nothing is written to the asset.
"""
import json
import math
import os
import sys

import bmesh
import bpy
import mathutils

ART = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, ART)

GLB = os.path.join(os.path.dirname(os.path.dirname(ART)), "assets", "models",
                   "person_ranger.glb").replace("\\", "/")
SCALE = 170.0
# Past this the pose does not match what the solver said, and the fault is in the conversion
# from positions to rotations rather than in the solver.
POSED_WITHIN = 0.02  # cm
# The skin has to move too, and by a real amount - see `the_shin_skin`.
SKIN_MOVES_AT_LEAST = 1.0  # cm


def argv():
    return sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []


args = argv()


def flag(name, fallback=None):
    return args[args.index(name) + 1] if name in args else fallback


SOLVED = flag("--solved", "solved_leg.json")
OUT = flag("--out", ".")
SIDE = flag("--side", "L")
SAVE_TO = flag("--save")
STILLS = "--stills" in args

# NOT read_homefile(use_empty=True) - see gait_watch.py. Clearing the startup objects by hand is
# enough and does not need a context the importer cannot work in.
for stale in list(bpy.data.objects):
    bpy.data.objects.remove(stale, do_unlink=True)
bpy.ops.import_scene.gltf(filepath=GLB)
rig = next(o for o in bpy.data.objects if o.type == "ARMATURE")

import animate_ranger  # noqa: E402  (rest(); main() is guarded, so importing runs nothing)
import prepare_rig  # noqa: E402

mesh = prepare_rig.the_body()
prepare_rig.reach_the_ends(rig, mesh)
across, forward, up = prepare_rig.body_frame(rig)

# THE SKELETON IS NOT VISIBLE UNTIL THE WIDGETS GO.
#
# Blender's glTF importer builds an Icosphere and assigns it as a custom shape to EVERY bone, so
# that joints show up at all - glTF carries no bone lengths, so there is nothing to draw
# otherwise. With 71 bones that is 71 spheres, a cluster of them per hand, and the bones
# themselves completely hidden behind them.
#
# `prepare_rig.drop_the_widgets` exists for exactly this and its own note says why hiding the
# Icosphere object is not enough. Worth being clear that this is an IMPORT artifact and not
# something in the asset: the shipped GLB holds two meshes, `Backpack` and the body, and no
# widget at all.
prepare_rig.drop_the_widgets(rig)
for stale in [o for o in bpy.data.objects if o.name.startswith("Icosphere")]:
    bpy.data.objects.remove(stale, do_unlink=True)

rig.show_in_front = True
rig.data.display_type = "OCTAHEDRAL"
rig.data.show_names = False
# The twist bones are skinning helpers - they carry the arm and leg skin and are never posed by
# hand - and there are sixteen of them sitting on top of the joints that matter. Hidden so the
# hip-knee-ankle chain can actually be read.
for bone in rig.data.bones:
    if "Twist" in bone.name:
        bone.hide = True

with open(SOLVED, encoding="utf-8") as handle:
    solved = json.load(handle)
cases = solved["cases"]
print(f"read {len(cases)} solved cases; the game's leg is thigh "
      f"{solved['thigh'] * SCALE:.2f} cm, calf {solved['calf'] * SCALE:.2f} cm, bind at "
      f"{solved['bind_extension'] * 100:.1f}% of straight")

if (rig.matrix_world.to_3x3() - mathutils.Matrix.Identity(3)).median_scale > 1e-6:
    raise SystemExit(
        "REFUSED: the armature is rotated or scaled in the world, so an armature-space axis is "
        "not a world-space one and the aiming below would turn bones about the wrong lines"
    )


def where(named):
    """Rebuilds a hip-relative point from the rig's own axes."""
    return (up * named["up"]) + (forward * named["forward"]) + (across * named["across"])


def joint_at(name):
    return rig.matrix_world @ rig.pose.bones[name].head


# The three joints, by the HEADS of the bones rather than any bone's tail. An interior bone's
# tail is placed by the importer at whichever child it picked first, and Thigh and Calf each have
# a twist bone as well as the real one - so a tail here is a coin flip.
HIP, KNEE, ANKLE = f"{SIDE}_Thigh", f"{SIDE}_Calf", f"{SIDE}_Foot"
POSED = (HIP, KNEE)


def aim_bone(name, from_joint, to_joint, wanted):
    """Turns one bone so the line from `from_joint` to `to_joint` points along `wanted`.

    By writing the bone's ARMATURE-SPACE matrix, so Blender back-solves the local rotation with
    the parent's current pose accounted for.

    `animate_ranger.turn_further` was the obvious thing to reach for and is wrong here. It
    converts its axis through the bone's REST basis, which is the bone's actual basis only while
    the parent is unrotated - true for the thigh, false for the calf the moment the thigh moves.
    The error was not random, which is what identified it: 0.195 cm on a flat foot where the knee
    barely bends, rising to 2.282 cm on a 20 cm step up where it bends most.
    """
    was = joint_at(to_joint) - joint_at(from_joint)
    if was.length < 1e-9 or wanted.length < 1e-9:
        return
    turn = was.normalized().rotation_difference(wanted.normalized())
    posed = rig.pose.bones[name]
    held = posed.matrix.copy()
    posed.matrix = (
        mathutils.Matrix.Translation(held.translation)
        @ (turn.to_matrix() @ held.to_3x3()).to_4x4()
    )
    bpy.context.view_layer.update()


def the_shin_skin():
    """Where the shin's GEOMETRY actually is, on the evaluated mesh.

    Because bones agreeing with the solver is only half of it. The leg's skin is not weighted to
    `_Calf` at all - that drives zero vertices - it is on `_CalfTwist01/02`, which drive 221 and
    373. So "the ankle bone is in the right place" and "the trouser leg bent" are two separate
    claims, and reading the render told me the second was failing when it was not.
    """
    depsgraph = bpy.context.evaluated_depsgraph_get()
    evaluated = mesh.evaluated_get(depsgraph)
    got = evaluated.to_mesh()
    groups = {g.index: g.name for g in mesh.vertex_groups}
    middle, count = mathutils.Vector(), 0
    for original, moved in zip(mesh.data.vertices, got.vertices):
        best, who = 0.0, ""
        for group in original.groups:
            if group.weight > best:
                best, who = group.weight, groups.get(group.group, "")
        if who.startswith(f"{SIDE}_CalfTwist"):
            middle += mesh.matrix_world @ moved.co
            count += 1
    evaluated.to_mesh_clear()
    if not count:
        raise SystemExit("REFUSED: nothing is driven by _CalfTwist*, so this check is blind")
    return middle / count


# --- The markers.
#
# THE SKELETON DOES NOT RENDER, and does not show in a viewport screenshot of a solid-shaded
# view either unless it is asked to. `show_in_front` and a stick display are viewport settings
# and an armature draws nothing at all in a render - so the first version of this produced eight
# pictures of a clothed figure with no visible difference between a flat foot and a 20 cm step.
#
# So the solved chain is real geometry: two rods and three balls, made once at unit length and
# KEYED per frame, which is what lets the cases be scrubbed instead of compared from memory.


def glowing(name, colour, strength=1.6):
    paint = bpy.data.materials.new(name)
    paint.use_nodes = True
    shader = paint.node_tree.nodes["Principled BSDF"]
    shader.inputs["Base Color"].default_value = (*colour, 1.0)
    shader.inputs["Emission Color"].default_value = (*colour, 1.0)
    shader.inputs["Emission Strength"].default_value = strength
    return paint


def a_rod(name, paint, thickness):
    """A cylinder of unit length along +Z, so a segment is one scale and one rotation."""
    data = bpy.data.meshes.new(name)
    rod = bmesh.new()
    bmesh.ops.create_cone(rod, cap_ends=True, cap_tris=False, segments=12,
                          radius1=thickness, radius2=thickness, depth=1.0)
    rod.to_mesh(data)
    rod.free()
    thing = bpy.data.objects.new(name, data)
    thing.data.materials.append(paint)
    thing.rotation_mode = "QUATERNION"
    bpy.context.scene.collection.objects.link(thing)
    return thing


def a_ball(name, paint, radius):
    data = bpy.data.meshes.new(name)
    ball = bmesh.new()
    bmesh.ops.create_icosphere(ball, subdivisions=2, radius=radius)
    ball.to_mesh(data)
    ball.free()
    thing = bpy.data.objects.new(name, data)
    thing.data.materials.append(paint)
    bpy.context.scene.collection.objects.link(thing)
    return thing


BONE = glowing("solved", (0.2, 0.85, 1.0))
WAS = glowing("at rest", (0.42, 0.40, 0.38), strength=0.5)
TARGET = glowing("asked for", (0.95, 0.1, 0.1))

thigh_rod = a_rod("thigh solved", BONE, 0.010)
calf_rod = a_rod("calf solved", BONE, 0.009)
rest_thigh = a_rod("thigh at rest", WAS, 0.005)
rest_calf = a_rod("calf at rest", WAS, 0.005)
knee_ball = a_ball("knee", BONE, 0.014)
ankle_ball = a_ball("ankle solved", BONE, 0.013)
target_ball = a_ball("asked for", TARGET, 0.016)


def lay_rod(rod, a, b, frame):
    along = b - a
    rod.location = (a + b) * 0.5
    rod.scale = (1.0, 1.0, max(along.length, 1e-6))
    if along.length > 1e-9:
        rod.rotation_quaternion = mathutils.Vector((0.0, 0.0, 1.0)).rotation_difference(
            along.normalized())
    for channel in ("location", "rotation_quaternion", "scale"):
        rod.keyframe_insert(channel, frame=frame)


def lay_ball(ball, at, frame):
    ball.location = at
    ball.keyframe_insert("location", frame=frame)


# --- Pose every case, one per frame.
print(f"\n  {'frame':>5} {'case':<16} {'solver ankle':>13} {'posed ankle':>12} {'apart':>8} "
      f"{'missed by':>10} {'shin skin':>10}")
worst = 0.0
skin_went = []
for number, case in enumerate(cases, start=1):
    animate_ranger.rest(rig)
    bpy.context.view_layer.update()
    hip = joint_at(HIP)
    rest_knee, rest_ankle = joint_at(KNEE), joint_at(ANKLE)
    wants_knee = hip + where(case["joint"])
    wants_ankle = hip + where(case["end"])
    target = hip + where(case["target"])

    aim_bone(HIP, HIP, KNEE, wants_knee - hip)
    aim_bone(KNEE, KNEE, ANKLE, wants_ankle - joint_at(KNEE))

    got = joint_at(ANKLE)
    apart = (got - wants_ankle).length * SCALE
    worst = max(worst, apart)
    skin = the_shin_skin()
    skin_went.append(skin)
    print(f"  {number:5} {case['called']:<16} {wants_ankle.z * SCALE:12.2f}z "
          f"{got.z * SCALE:11.2f}z {apart:7.3f}cm {case['missed_by'] * SCALE:9.2f}cm "
          f"{skin.z * SCALE:9.2f}z")

    for name in POSED:
        rig.pose.bones[name].keyframe_insert("rotation_quaternion", frame=number)
    lay_rod(rest_thigh, hip, rest_knee, number)
    lay_rod(rest_calf, rest_knee, rest_ankle, number)
    lay_rod(thigh_rod, hip, joint_at(KNEE), number)
    lay_rod(calf_rod, joint_at(KNEE), got, number)
    lay_ball(knee_ball, joint_at(KNEE), number)
    lay_ball(ankle_ball, got, number)
    lay_ball(target_ball, target, number)

# Each frame is a separate case, not a stage in a motion, so nothing should be interpolated
# between them - scrubbing halfway would otherwise show a pose the solver never produced.
for holder in [rig, thigh_rod, calf_rod, rest_thigh, rest_calf, knee_ball, ankle_ball,
               target_ball]:
    if not (holder.animation_data and holder.animation_data.action):
        continue
    # Via animate_ranger, because Blender 5 has no `action.fcurves` - actions are slots,
    # layers, strips and channelbags now, and the old attribute raises AttributeError, which
    # reads as a broken script rather than a moved API. That helper exists for this exact
    # reason and reaching past it would have been a fourth copy of the same four lines.
    bag = animate_ranger.where_the_curves_live(holder.animation_data.action)
    if bag is None:
        continue
    for curve in bag.fcurves:
        for key in curve.keyframe_points:
            key.interpolation = "CONSTANT"

scene = bpy.context.scene
scene.frame_start = 1
scene.frame_end = len(cases)
scene.frame_set(1)
scene.render.fps = 2  # slow, so playing it steps through the cases rather than flickering

# --- Checks. Both of these have failed, and each named its own cause.
spread = max((a - b).length for a in skin_went for b in skin_went) * SCALE
print(f"\nthe shin skin moved {spread:.2f} cm across the cases")
if spread < SKIN_MOVES_AT_LEAST:
    raise SystemExit(
        f"REFUSED: the bones agree with the solver but the shin skin only moved {spread:.2f} cm "
        f"across cases that put the ankle 20 cm apart. The mesh is not following the pose, so "
        f"looking at it shows nothing - check the armature modifier, and whether the twist "
        f"bones really inherit from _Thigh and _Calf."
    )
print(f"the worst the posed rig differed from the solver was {worst:.3f} cm")
if worst > POSED_WITHIN:
    raise SystemExit(
        f"REFUSED: the posed rig is {worst:.3f} cm from what the solver said, past "
        f"{POSED_WITHIN} cm. The arithmetic in src/ik.rs is not the problem - turning its "
        f"positions into bone rotations is."
    )
print("so the solved positions and the posed bones agree: the conversion is sound")

# --- Light, and a view aimed at the leg.
sun = bpy.data.objects.new("sun", bpy.data.lights.new("sun", type="SUN"))
bpy.context.scene.collection.objects.link(sun)
sun.data.energy = 3.2
sun.rotation_euler = (math.radians(55.0), 0.0, math.radians(35.0))
world = bpy.data.worlds.new("w")
world.use_nodes = True
world.node_tree.nodes["Background"].inputs[0].default_value = (0.11, 0.12, 0.14, 1.0)
bpy.context.scene.world = world

# On the leg, not the whole figure: a 78 cm leg inside a 170 cm frame is not something a 2 cm
# difference shows up in. And from the side the POSED leg is on - `across` runs right to left, so
# viewing from -across puts the unposed right leg nearest the camera and it occludes the one that
# moved, which produced eight renders of a straight trouser leg.
aim = mathutils.Vector((0.0, 0.0, joint_at(HIP).z * 0.55))
side = across * (1.0 if SIDE == "L" else -1.0)

aimed = 0
for screen in bpy.data.screens:
    for area in screen.areas:
        if area.type != "VIEW_3D":
            continue
        space = area.spaces.active
        space.shading.type = "SOLID"
        space.overlay.show_floor = False
        space.region_3d.view_perspective = "ORTHO"
        space.region_3d.view_rotation = mathutils.Vector(side).to_track_quat("Z", "Y")
        space.region_3d.view_location = aim
        space.region_3d.view_distance = 1.1
        aimed += 1
print(f"aimed {aimed} saved 3D view(s) at the {SIDE} leg")

if STILLS:
    camera = bpy.data.objects.new("cam", bpy.data.cameras.new("cam"))
    bpy.context.scene.collection.objects.link(camera)
    camera.data.type = "ORTHO"
    camera.data.ortho_scale = 1.05
    scene.camera = camera
    scene.render.engine = "BLENDER_EEVEE"
    scene.render.resolution_x = 480
    scene.render.resolution_y = 560
    scene.eevee.taa_render_samples = 24
    eye = aim + side * 4.0
    camera.location = eye
    camera.rotation_euler = (aim - eye).normalized().to_track_quat("-Z", "Y").to_euler()
    for number, case in enumerate(cases, start=1):
        scene.frame_set(number)
        scene.render.filepath = os.path.join(
            OUT, f"ik_{SIDE}_{case['called'].replace(' ', '_')}.png")
        bpy.ops.render.render(write_still=True)
    print(f"wrote ik_{SIDE}_<case>.png to {OUT}")

if SAVE_TO:
    # The reload watcher from gait_watch, so an open window is never showing stale work.
    #
    # This is the fourth time on this project that a report came back about something already
    # fixed, because the window was built before the fix - the sphere widgets being the most
    # recent, reported twice. A viewer that cannot tell you it is out of date is worse than no
    # viewer, because it makes everything you say about it unreliable and neither side can see
    # that. gait_watch has carried the watcher for exactly this; it only needed importing, which
    # in turn needed its own `main()` guarding.
    import gait_watch  # noqa: E402

    gait_watch.stamp_the_scene(GLB, f"ik cases, {SIDE} leg")
    gait_watch.install_the_watcher()
    bpy.ops.wm.save_as_mainfile(filepath=SAVE_TO)
    print(f"saved {SAVE_TO}")
