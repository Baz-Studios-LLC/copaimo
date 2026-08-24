"""Measures the four things reported wrong: hands, upper arms, feet, and the toe bend.

    blender --background --python anatomy_audit.py -- [--model PATH]

Read-only, and it measures the MESH wherever the mesh is what a person actually sees. That
distinction is the point of this file: `verify_gait` reports `toe_out_deg: -0.0` on all three
clips and it is not lying - it measures the ToeBase BONE against the line of travel, and that
bone really is straight. A shoe can be splayed around a straight bone, and a shoe is what gets
looked at.

What is measured, and against what:

  FEET       the shoe's own long axis, from its geometry, against the line of travel. Bones too,
             so the two can be compared and the difference located.
  HANDS      which way the palm faces, from the hand's flat - the direction the hand's vertex
             cloud varies least in is the palm's normal.
  UPPER ARMS how far the elbow's hinge has been rolled about the arm's own axis. An arm twisted
             at rest hinges sideways instead of forwards, which is the thing that reads as wrong
             long before anybody can say why.
  TOES       how far the toe bends through each clip, and when. A foot that never bends rolls
             off flat, which is the difference between walking and being slid along the ground.
"""
import math
import os
import sys

import bpy
import mathutils

ART = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, ART)

SCALE = 170.0


def argv():
    return sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []


args = argv()
MODEL = args[args.index("--model") + 1] if "--model" in args else os.path.join(
    os.path.dirname(os.path.dirname(ART)), "assets", "models", "person_ranger.glb")

bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=MODEL.replace("\\", "/"))
rig = next(o for o in bpy.data.objects if o.type == "ARMATURE")

import prepare_rig  # noqa: E402

mesh = prepare_rig.the_body()
prepare_rig.reach_the_ends(rig, mesh)
prepare_rig.drop_the_widgets(rig)
across, forward, up = prepare_rig.body_frame(rig)
print(f"{os.path.basename(MODEL)}: {len(rig.data.bones)} bones")
print(f"body frame: forward {tuple(round(v, 3) for v in forward)}, "
      f"across {tuple(round(v, 3) for v in across)}\n")

groups = {g.index: g.name for g in mesh.vertex_groups}
owned = {}
for vertex in mesh.data.vertices:
    best, who = 0.0, ""
    for group in vertex.groups:
        if group.weight > best:
            best, who = group.weight, groups.get(group.group, "")
    owned.setdefault(who, []).append(mesh.matrix_world @ vertex.co)


def principal(points, ignoring=None):
    """The direction a cloud spreads most in, and the direction it spreads least in."""
    import numpy

    cloud = [p - (ignoring * p.dot(ignoring)) if ignoring else p for p in points]
    array = numpy.array([[p.x, p.y, p.z] for p in cloud])
    array = array - array.mean(axis=0)
    _u, spread, axes = numpy.linalg.svd(array, full_matrices=False)
    return (mathutils.Vector(axes[0]).normalized(),
            mathutils.Vector(axes[2]).normalized(),
            spread)


def rest_head(name):
    return rig.matrix_world @ rig.data.bones[name].matrix_local.translation


def rest_tail(name):
    bone = rig.data.bones[name]
    return rig.matrix_world @ (bone.matrix_local @ mathutils.Vector((0.0, bone.length, 0.0)))


# --- The feet, bone against shoe.
print("FEET - the bone's line, then the shoe's own")
for side in "LR":
    span = rest_tail(f"{side}_ToeBase") - rest_head(f"{side}_Foot")
    flat = mathutils.Vector((span.x, span.y, 0.0))
    bone_yaw = math.degrees(math.atan2(flat.dot(across), flat.dot(forward)))
    shoe = owned.get(f"{side}_ToeBase", []) + owned.get(f"{side}_Foot", [])
    if shoe:
        long_way, _, _ = principal(shoe, ignoring=up)
        if long_way.dot(forward) < 0:
            long_way = -long_way
        shoe_yaw = math.degrees(math.atan2(long_way.dot(across), long_way.dot(forward)))
        print(f"  {side}: bone points {bone_yaw:+6.1f} deg off travel, "
              f"shoe points {shoe_yaw:+6.1f} deg  ({len(shoe)} verts)")
    else:
        print(f"  {side}: bone points {bone_yaw:+6.1f} deg off travel; no shoe vertices found")

# --- The shoes, against what a foot is a proportion of.
#
# Reported as "very bulky", which is a judgement - but a judgement about a proportion, and a
# proportion is measurable. An adult foot is about 15% of standing height long and roughly 40% of
# its own length across. Those are the numbers a bulky shoe is bulky AGAINST, so they are stated
# here rather than left as an impression.
print("SHOES - size against the proportions a foot usually has")
tall = max(p.z for group in owned.values() for p in group) - min(
    p.z for group in owned.values() for p in group)
for side in "LR":
    shoe = owned.get(f"{side}_ToeBase", []) + owned.get(f"{side}_Foot", [])
    if not shoe:
        continue
    long_way, _, _ = principal(shoe, ignoring=up)
    if long_way.dot(forward) < 0:
        long_way = -long_way
    wide_way = up.cross(long_way).normalized()
    length = max(p.dot(long_way) for p in shoe) - min(p.dot(long_way) for p in shoe)
    width = max(p.dot(wide_way) for p in shoe) - min(p.dot(wide_way) for p in shoe)
    deep = max(p.z for p in shoe) - min(p.z for p in shoe)
    print(f"  {side}: {length * SCALE:5.1f} cm long, {width * SCALE:5.1f} wide, "
          f"{deep * SCALE:5.1f} tall")
    print(f"      length is {length / tall * 100:4.1f}% of height (a foot is about 15), "
          f"width is {width / length * 100:4.1f}% of its own length (about 40)")

# --- The hands.
print("\nHANDS - which way the palm faces (toward the thigh is what was asked for)")
for side, inward in (("L", -1.0), ("R", 1.0)):
    hand = owned.get(f"{side}_Hand", [])
    if not hand:
        print(f"  {side}: nothing weighted to {side}_Hand")
        continue
    _, flat_way, spread = principal(hand)
    # Point it inboard, so both hands are described the same way round.
    if flat_way.dot(across) * inward < 0:
        flat_way = -flat_way
    towards = flat_way.dot(across) * inward
    print(f"  {side}: palm normal fwd {flat_way.dot(forward):+.2f} "
          f"across {flat_way.dot(across):+.2f} up {flat_way.dot(up):+.2f}"
          f"   -> {towards * 100:+.0f}% toward the thigh"
          f"   (flatness {spread[2] / spread[0]:.2f})")

# --- The upper arms: how far the elbow hinge is rolled about the arm's own axis.
print("\nUPPER ARMS - which way the elbow hinges (a forward hinge is 0)")
for side in "LR":
    shoulder = rest_head(f"{side}_Upperarm")
    elbow = rest_head(f"{side}_Forearm")
    wrist = rest_head(f"{side}_Hand")
    arm = (elbow - shoulder).normalized()
    # The forearm's offset from the upper arm's line IS the hinge direction: a hinge that folds
    # forward puts the wrist forward of the shoulder-elbow line when the elbow bends.
    lower = wrist - elbow
    off = lower - arm * lower.dot(arm)
    if off.length < 1e-6:
        print(f"  {side}: the arm is dead straight, so the hinge has no direction to read")
        continue
    off.normalize()
    rolled = math.degrees(math.atan2(off.dot(across), -off.dot(forward)))
    print(f"  {side}: hinge points fwd {-off.dot(forward):+.2f} across {off.dot(across):+.2f} "
          f"up {off.dot(up):+.2f}   -> rolled {rolled:+.1f} deg from a forward hinge")

# --- The toe bend, through every clip.
print("\nTOES - how far the toe bends through each clip")
if rig.animation_data is None:
    rig.animation_data_create()
for clip in sorted(bpy.data.actions, key=lambda a: a.name):
    rig.animation_data.action = clip
    first, last = (int(round(v)) for v in clip.frame_range)
    # THE JOINT ANGLE, toe against foot - not the toe's rotation in the world.
    #
    # The first version of this measured the toe bone against its own REST direction in world
    # space, and reported the walk bending 8 degrees and the run 66. Both numbers were real and
    # neither was the toe bend: most of the run's 66 is the whole foot pointing during flight,
    # carrying a perfectly rigid toe with it. What a person means by "the toe bends" is
    # dorsiflexion AT THE BALL, which is the angle between the two bones, and that is a thing
    # neither of them can fake by moving together.
    swing = {}
    for side in "LR":
        def between(matrix_of):
            foot = (matrix_of(f"{side}_Foot") @ mathutils.Vector((0.0, 1.0, 0.0))).normalized()
            toe = (matrix_of(f"{side}_ToeBase") @ mathutils.Vector((0.0, 1.0, 0.0))).normalized()
            return math.degrees(foot.angle(toe))

        rested = between(lambda n: rig.data.bones[n].matrix_local.to_3x3())
        angles = []
        for frame in range(first, last + 1):
            bpy.context.scene.frame_set(frame)
            bpy.context.view_layer.update()
            angles.append(between(lambda n: rig.pose.bones[n].matrix.to_3x3()) - rested)
        swing[side] = (min(angles), max(angles))
    print(f"  {clip.name:<8} L {swing['L'][0]:5.1f} to {swing['L'][1]:5.1f} deg "
          f"(swing {swing['L'][1] - swing['L'][0]:5.1f}),  "
          f"R {swing['R'][0]:5.1f} to {swing['R'][1]:5.1f} deg "
          f"(swing {swing['R'][1] - swing['R'][0]:5.1f})")
