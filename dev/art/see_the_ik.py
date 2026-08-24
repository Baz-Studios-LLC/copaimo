"""Poses the real rig's leg to whatever the GAME's IK solver worked out, and renders it.

    dev/art/see_the_ik.sh

The solver lives in `src/ik.rs` and only there. This does not re-implement it - a second copy
in Python would be a different solver, and two implementations of the same arithmetic agreeing
proves nothing about either. Rust computes; this draws, and then MEASURES BACK what it drew.

That measurement is the point. Turning solved POSITIONS into bone ROTATIONS is the part of a
runtime IK that actually goes wrong - local space against world space, and which axis a bone
runs along - and it is invisible in the solver's own tests because they never touch a rig. So
each case is posed, the ankle's real position is read off the posed armature, and the distance
from where the solver put it is printed. Anything but ~0 means the conversion is wrong even
though the arithmetic was right.

Nothing is written to the asset. Read-only, renders to a folder.

# The frame

The JSON is HIP-RELATIVE and its axes are NAMED - up, forward, across - because Blender is Z-up
and the game is Y-up, and a conversion between them is the single most expensive class of
mistake this project has made. Each point is rebuilt here from the rig's own measured axes via
`prepare_rig.body_frame`, so there is no conversion to get backwards.
"""
import json
import math
import os
import sys

import bpy
import mathutils

ART = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, ART)

GLB = os.path.join(os.path.dirname(os.path.dirname(ART)), "assets", "models",
                   "person_ranger.glb").replace("\\", "/")
SOLVED = os.environ.get("IK_SOLVED", "solved_leg.json")
OUT = os.environ.get("IK_OUT", ".")
SIDE = os.environ.get("IK_SIDE", "L")
SCALE = 170.0
# Past this the pose does not match what the solver said, and the fault is in the conversion
# from positions to rotations rather than in the solver.
POSED_WITHIN = 0.02  # cm

bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=GLB)
rig = next(o for o in bpy.data.objects if o.type == "ARMATURE")

import prepare_rig  # noqa: E402
import animate_ranger  # noqa: E402  (turn_further, for composing an armature-space turn)

mesh = prepare_rig.the_body()
prepare_rig.reach_the_ends(rig, mesh)
across, forward, up = prepare_rig.body_frame(rig)

with open(SOLVED, encoding="utf-8") as f:
    solved = json.load(f)
print(f"read {len(solved['cases'])} solved cases; the game's leg is thigh "
      f"{solved['thigh'] * SCALE:.2f} cm, calf {solved['calf'] * SCALE:.2f} cm, bind at "
      f"{solved['bind_extension'] * 100:.1f}% of straight")


def where(named):
    """Rebuilds a hip-relative point from the rig's own axes."""
    return (up * named["up"]) + (forward * named["forward"]) + (across * named["across"])


def joint_at(name):
    return rig.matrix_world @ rig.pose.bones[name].head


# The three joints, by the heads of the bones themselves rather than by any bone's TAIL. An
# interior bone's tail is placed by the importer at whichever child it picked first, and both
# Thigh and Calf have a twist bone as well as the real one - so a tail here is a coin flip.
HIP, KNEE, ANKLE = f"{SIDE}_Thigh", f"{SIDE}_Calf", f"{SIDE}_Foot"


def aim_bone(name, from_joint, to_joint, wanted):
    """Turns one bone so the line from `from_joint` to `to_joint` points along `wanted`.

    By writing the bone's ARMATURE-SPACE matrix, so Blender back-solves the local rotation with
    the parent's current pose accounted for.

    `animate_ranger.turn_further` was the obvious thing to reach for and it is wrong here. It
    converts its axis through the bone's REST basis, which is only the bone's actual basis while
    the parent is unrotated - true for the thigh, false for the calf the moment the thigh moves.
    The error it produced was not random, which is what identified it: 0.195 cm on a flat foot
    where the knee barely bends, rising to 2.282 cm on a 20 cm step up where it bends most.
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


leaning = rig.matrix_world.to_3x3()
if (leaning - mathutils.Matrix.Identity(3)).median_scale > 1e-6:
    raise SystemExit(
        "REFUSED: the armature is rotated or scaled in the world, so an armature-space axis "
        "is not a world-space one and `aim_bone` would turn the bones about the wrong lines"
    )

# --- Light and camera, side on to the leg.
sun = bpy.data.objects.new("sun", bpy.data.lights.new("sun", type="SUN"))
bpy.context.scene.collection.objects.link(sun)
sun.data.energy = 3.2
sun.rotation_euler = (math.radians(55.0), 0.0, math.radians(35.0))
world = bpy.data.worlds.new("w")
world.use_nodes = True
world.node_tree.nodes["Background"].inputs[0].default_value = (0.11, 0.12, 0.14, 1.0)
bpy.context.scene.world = world

camera = bpy.data.objects.new("cam", bpy.data.cameras.new("cam"))
bpy.context.scene.collection.objects.link(camera)
camera.data.type = "ORTHO"
# On the leg, not the whole figure. A 78 cm leg inside a 170 cm frame is not something a
# 2 cm difference shows up in.
camera.data.ortho_scale = 1.05
bpy.context.scene.camera = camera
scene = bpy.context.scene
scene.render.engine = "BLENDER_EEVEE"
scene.render.resolution_x = 480
scene.render.resolution_y = 560
scene.eevee.taa_render_samples = 24

import bmesh  # noqa: E402


def glowing(name, colour):
    paint = bpy.data.materials.new(name)
    paint.use_nodes = True
    shader = paint.node_tree.nodes["Principled BSDF"]
    shader.inputs["Base Color"].default_value = (*colour, 1.0)
    shader.inputs["Emission Color"].default_value = (*colour, 1.0)
    shader.inputs["Emission Strength"].default_value = 1.6
    return paint


TARGET = glowing("target", (0.95, 0.1, 0.1))
BONE = glowing("bone", (0.2, 0.85, 1.0))
# The leg before it was solved, drawn alongside in a dim colour. Without it a single render is
# a leg in some pose, and telling a 20 cm step from a flat foot means holding two pictures in
# your head - which is how "the mesh is not following the bones" got said about a mesh that was
# following them perfectly.
WAS = glowing("was", (0.35, 0.34, 0.33))

# THE SKELETON DOES NOT RENDER. `show_in_front` and a stick display type are viewport settings,
# and an armature draws nothing at all in a render - so the first version of this produced eight
# pictures of a clothed figure with no visible difference between a flat foot and a 20 cm step,
# which is exactly the sort of thing that gets reported as "looks fine" and is not.
#
# So the solved chain is drawn as real geometry: a rod hip-to-knee, a rod knee-to-ankle, and a
# ball on the target the solver was asked for. A miss then has a size on screen.
drawn = []


def clear_drawing():
    for stale in drawn:
        bpy.data.objects.remove(stale, do_unlink=True)
    drawn.clear()


def draw_ball(at, radius, paint):
    mesh_data = bpy.data.meshes.new("ball")
    ball = bmesh.new()
    bmesh.ops.create_icosphere(ball, subdivisions=2, radius=radius)
    ball.to_mesh(mesh_data)
    ball.free()
    thing = bpy.data.objects.new("ball", mesh_data)
    thing.data.materials.append(paint)
    thing.location = at
    bpy.context.scene.collection.objects.link(thing)
    drawn.append(thing)


def draw_rod(a, b, radius, paint):
    along = b - a
    if along.length < 1e-9:
        return
    mesh_data = bpy.data.meshes.new("rod")
    rod = bmesh.new()
    bmesh.ops.create_cone(rod, cap_ends=True, cap_tris=False, segments=12,
                          radius1=radius, radius2=radius, depth=along.length)
    rod.to_mesh(mesh_data)
    rod.free()
    thing = bpy.data.objects.new("rod", mesh_data)
    thing.data.materials.append(paint)
    thing.location = (a + b) * 0.5
    thing.rotation_mode = "QUATERNION"
    thing.rotation_quaternion = mathutils.Vector((0.0, 0.0, 1.0)).rotation_difference(
        along.normalized())
    bpy.context.scene.collection.objects.link(thing)
    drawn.append(thing)

def the_shin_skin():
    """Where the geometry of the shin actually IS, on the evaluated mesh.

    Because bones agreeing with the solver is only half of it. The leg's skin is not weighted to
    `_Calf` at all - it is on `_CalfTwist01/02`, which drive 221 and 373 vertices where `_Calf`
    drives none - so "the ankle bone is in the right place" and "the trouser leg bent" are two
    different claims and the render showed the second one failing while the first passed.
    """
    depsgraph = bpy.context.evaluated_depsgraph_get()
    evaluated = mesh.evaluated_get(depsgraph)
    got = evaluated.to_mesh()
    groups = {g.index: g.name for g in mesh.vertex_groups}
    wanted = f"{SIDE}_CalfTwist"
    middle, count = mathutils.Vector(), 0
    for original, moved in zip(mesh.data.vertices, got.vertices):
        best, who = 0.0, ""
        for group in original.groups:
            if group.weight > best:
                best, who = group.weight, groups.get(group.group, "")
        if who.startswith(wanted):
            middle += mesh.matrix_world @ moved.co
            count += 1
    evaluated.to_mesh_clear()
    if not count:
        raise SystemExit(f"REFUSED: no vertices are driven by {wanted}*, so this check is blind")
    return middle / count


print(f"\n  {'case':<16} {'solver put the ankle':>21} {'the posed rig has it':>21} "
      f"{'apart':>7} {'missed target by':>17} {'the shin skin':>14}")
worst = 0.0
skin_went = []
for case in solved["cases"]:
    animate_ranger.rest(rig)
    bpy.context.view_layer.update()
    hip = joint_at(HIP)
    rested = (joint_at(KNEE), joint_at(ANKLE))
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
    print(f"  {case['called']:<16} {wants_ankle.z * SCALE:20.2f}z {got.z * SCALE:20.2f}z "
          f"{apart:6.3f}cm {case['missed_by'] * SCALE:16.2f}cm {skin.z * SCALE:12.2f}z")

    clear_drawing()
    draw_rod(hip, rested[0], 0.006, WAS)
    draw_rod(rested[0], rested[1], 0.006, WAS)
    draw_rod(hip, joint_at(KNEE), 0.010, BONE)
    draw_rod(joint_at(KNEE), got, 0.009, BONE)
    draw_ball(joint_at(KNEE), 0.014, BONE)
    draw_ball(got, 0.013, BONE)
    draw_ball(target, 0.016, TARGET)

    # SIDE ON, along the body's across axis - a leg swinging fore and aft is edge-on from the
    # front and invisible. The first version aimed down the forward axis and produced eight
    # pictures of the character's back.
    # From the side the POSED leg is on. `across` runs right-to-left, so viewing from -across
    # puts the unposed right leg nearest the camera, occluding the one that moved - eight
    # renders of a straight trouser leg with the solved chain drawn behind it, which read as
    # "the mesh is not following the bones" when the mesh was never in view.
    aim = (hip + got) * 0.5
    eye = aim + across * 4.0 * (1.0 if SIDE == "L" else -1.0)
    camera.location = eye
    camera.rotation_euler = (aim - eye).normalized().to_track_quat("-Z", "Y").to_euler()
    scene.render.filepath = os.path.join(
        OUT, f"ik_{SIDE}_{case['called'].replace(' ', '_')}.png")
    bpy.ops.render.render(write_still=True)

# Did the SKIN move, or only the skeleton? Two different claims, and the render showed the
# second passing while the first failed.
spread = max((a - b).length for a in skin_went for b in skin_went) * SCALE
print(f"\nthe shin skin moved {spread:.2f} cm across the cases")
if spread < 1.0:
    raise SystemExit(
        f"REFUSED: the bones agree with the solver but the shin skin only moved {spread:.2f} cm "
        f"across cases that put the ankle 20 cm apart. The mesh is not following the pose, so a "
        f"render of it shows nothing - check the armature modifier, and whether the twist bones "
        f"really inherit from _Thigh and _Calf."
    )

print(f"the worst the posed rig differed from the solver was {worst:.3f} cm")
if worst > POSED_WITHIN:
    raise SystemExit(
        f"REFUSED: the posed rig is {worst:.3f} cm from what the solver said, past "
        f"{POSED_WITHIN} cm. The arithmetic in src/ik.rs is not the problem - turning its "
        f"positions into bone rotations is."
    )
print("so the solved positions and the posed bones agree: the conversion is sound")
print(f"wrote ik_{SIDE}_<case>.png to {OUT}")
