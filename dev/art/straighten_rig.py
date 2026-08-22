"""Writes a copy of the ranger whose REST POSE is straight.

    blender --background --python dev/art/straighten_rig.py -- <in.glb> <out.glb>

Run by `dev/art/animate_ranger.sh` before the gaits are authored. Reads
`Ranger_Rig_Idle.glb` - the file as it arrived, kept untouched - and writes
`dev/art/ranger_straight.glb` with the same character standing properly.

# What is wrong with the rest pose, and why it cannot be left alone

The generator gave this rig a right thigh resting 7.4 degrees abducted against the
left's 1.9, and both feet toed out 18.5 degrees where a person is 7 to 10. Those are
constants, and for a long time they were corrected per pose by rotating each bone about
a world axis after the gait had posed it. That is the bug this file exists to end.

A pose bone's rotation lives in a basis that is the PARENT's posed frame times the
bone's own rest matrix. So conjugating a world axis through the bone's fully-posed frame
gives a correction that is constant in world space and therefore VARIABLE in the bone's
own frame, changing at whatever rate the parent moves. Measured on this rig:

    bone      parent motion          twist injected into the bone
    Thigh     pelvis, +/- 8 deg      0.19 deg peak-to-peak
    Foot      hip + knee, ~90 deg    10.2 deg peak-to-peak, once per step

with the toe not moving at all. That is exactly "the feet twist weirdly when jogging",
and it is worse in the sprint because the hip and knee range is bigger. The measured
toe-out angle was right the whole time - rotating about world +X changes the
frontal-plane projection by exactly the angle asked for at any pose - so the number
looked correct while the operator dumped the remainder into roll. **The metric was
right and the operator was wrong**, which is the most expensive shape a bug can take.

# Why bake it rather than pre-multiply a constant

A constant correction in each bone's REST basis is provably identical to having edited
the rest pose - verified elsewhere to within 4.4e-6 degrees - and would have been the
ten-line version of this. Baking is worth the extra work because it also cleans the
things a per-pose correction cannot reach: the glTF bind pose and its
inverseBindMatrices, every bone the gait does not key, and anything that blends these
clips later in the game. After this, the authoring script has no correction step at all.

# The two traps

**`pose.armature_apply()` moves only bones.** Run alone, it leaves the mesh where it was
and the skin snaps straight back to the splayed shape - measured, mesh vertex data
changes by exactly 0.000000. The deformation has to be written into the MESH first.

**The corrections interact, so they are solved rather than applied.** Straightening a
thigh swings the foot below it, which changes the foot's measured yaw; and the foot's
own correction axis then rides a corrected parent. Repairing proximal to distal in one
pass left the toe-out at 5.89 degrees instead of 8.00. So this is a fixed-point loop:
build the whole correction, apply it to a scratch copy, measure what is left, add it in,
and repeat until nothing moves.
"""

import math
import sys

import bpy
import mathutils

# What the legs and feet should measure once this is done, in degrees.
#
# The hips are 30 cm apart at this rig's scale and a person stands with the heels 10 to
# 15 apart, so a real leg converges slightly on the way down - which is what the
# negative number is. Feet at 8 degrees sits inside the human 7 to 10.
LEGS_SIT_AT = -3.5
TOES_SIT_AT = 8.0

# And how far the knees are bent FORWARD in the rest pose, in degrees.
#
# # An IK chain needs to know which way its knee goes
#
# A two-bone chain reaching a point has a whole circle of solutions - the knee can sit
# anywhere on it - and a dead straight rest pose says nothing about which. This rig's
# thigh and shin rest 3.4 degrees apart, which is nothing, and Blender's solver duly
# picked a BACKWARD knee on the sprint: "the R knee sits 0.011 behind the hip-to-ankle
# line, so the leg folds like a bird's".
#
# A pole target is the other answer and it was tried first. It works on the knee and
# wrecks the foot: pole angle rotates the whole chain about the hip-to-ankle axis, so
# the search that put the knees forward also turned both feet 168 degrees away from the
# line of travel. The knee was right and the foot was backwards.
#
# A bent bind pose has none of that coupling, and it is what game rigs ship with for
# exactly this reason. Eight degrees is enough to be unambiguous and little enough that
# the character still reads as standing.
KNEES_BENT_BY = 8.0

# How many times to go round the fixed-point loop, and when to call it done.
PASSES = 30
CLOSE_ENOUGH = 0.01


def argv():
    return sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []


def flush():
    bpy.context.view_layer.update()


def rest_direction(rig, bone: str):
    """Which way a bone points in its REST pose, in armature space.

    Off `matrix_local`, so it needs no depsgraph evaluation and cannot read a stale
    pose. Bones run along their own local +Y in Blender, which is why it is column 1.
    """
    return rig.data.bones[bone].matrix_local.to_3x3().col[1].normalized()


def which_way_is_forward(rig):
    """The direction the body travels, and the one across it. From the rest pose.

    Averaged over BOTH feet. Either one alone rests toed out, so using it would tilt
    the reference by half the angle between the feet - which silently cost about 5% on
    every fore-aft measurement while the left foot read as perfectly straight, being
    the thing that defined the axis.
    """
    toward = mathutils.Vector((0.0, 0.0, 0.0))
    for side in "LR":
        heel = rig.data.bones[f"{side}_Foot"].matrix_local.translation
        toe = rig.data.bones[f"{side}_ToeBase"].matrix_local.translation
        span = toe - heel
        toward += mathutils.Vector((span.x, span.y, 0.0)).normalized()
    forward = toward.normalized()
    return forward, mathutils.Vector((-forward.y, forward.x, 0.0))


def leg_splay(rig, side: str, forward, across) -> float:
    """How far a whole leg leans OUT across the body, hip to ankle, in degrees.

    Hip to ankle rather than segment by segment, because that is what reads: a thigh
    angled out and a shin angled back look straight, and the eye follows the whole limb.
    """
    hip = rig.data.bones[f"{side}_Thigh"].matrix_local.translation
    ankle = rig.data.bones[f"{side}_Foot"].matrix_local.translation
    along = ankle - hip
    sideways = along.dot(across) * (1.0 if side == "L" else -1.0)
    return math.degrees(math.atan2(sideways, max(1e-6, -along.z)))


def toe_out(rig, side: str, forward, across) -> float:
    """How far a foot points away from the line of travel, in degrees."""
    heel = rig.data.bones[f"{side}_Foot"].matrix_local.translation
    toe = rig.data.bones[f"{side}_ToeBase"].matrix_local.translation
    flat = mathutils.Vector(((toe - heel).x, (toe - heel).y, 0.0))
    if flat.length < 1e-9:
        return 0.0
    flat.normalize()
    yaw = math.degrees(math.atan2(flat.dot(across), flat.dot(forward)))
    return yaw * (1.0 if side == "L" else -1.0)


def in_rest_basis(rig, bone: str, axis, degrees: float):
    """A rotation about an ARMATURE axis, expressed in the bone's own rest basis.

    Which is the space `rotation_quaternion` actually lives in. Constant by
    construction: the same quaternion on every frame of every clip, because it does
    not mention the pose.

    The angle is negated on the right, and that is not cosmetic. Both measurements -
    splay and toe-out - are reported as "how far OUT", which is a mirrored quantity,
    while a rotation about the shared armature axis moves the left leg out and the
    right leg IN. Feeding a mirrored measurement to an unmirrored rotation made the
    right side's correction run the wrong way, and since this is a feedback loop the
    residual then DOUBLED every pass: -8.3, -16.4, -31.5, -61.8, -93.5, and the right
    leg ended up at 90 degrees of splay. A sign error in a closed loop does not show
    up as a small error.
    """
    basis = rig.data.bones[bone].matrix_local.to_3x3()
    # The knee's flexion is NOT a mirrored quantity - both knees bend the same way,
    # forward - so it is the one correction that must not be handed.
    hand = 1.0 if (bone.startswith("L_") or bone.endswith("_Calf")) else -1.0
    return mathutils.Quaternion(
        (basis.inverted() @ mathutils.Vector(axis)).normalized(),
        math.radians(degrees * hand),
    )


def knee_bend(rig, side: str, forward) -> float:
    """How far the knee sits FORWARD of the hip-to-ankle line, in degrees."""
    hip = rig.data.bones[f"{side}_Thigh"].matrix_local.translation
    knee = rig.data.bones[f"{side}_Calf"].matrix_local.translation
    ankle = rig.data.bones[f"{side}_Foot"].matrix_local.translation
    span = ankle - hip
    if span.length < 1e-9:
        return 0.0
    along = span.normalized()
    out = (knee - hip) - along * (knee - hip).dot(along)
    return math.degrees(math.atan2(out.dot(forward), max(1e-6, (knee - hip).length)))


def how_far_off(rig, forward, across):
    """How far each measurement is from where it should be, in degrees."""
    off = {}
    for side in "LR":
        off[f"{side}_Thigh"] = LEGS_SIT_AT - leg_splay(rig, side, forward, across)
        off[f"{side}_Calf"] = KNEES_BENT_BY - knee_bend(rig, side, forward)
        off[f"{side}_Foot"] = TOES_SIT_AT - toe_out(rig, side, forward, across)
    return off


def pose_it(rig, turns) -> None:
    for posed in rig.pose.bones:
        posed.rotation_mode = "QUATERNION"
        posed.rotation_quaternion = turns.get(posed.name, mathutils.Quaternion()).copy()
    flush()


def make_it_the_rest_pose(rig) -> None:
    was = bpy.context.view_layer.objects.active
    bpy.context.view_layer.objects.active = rig
    bpy.ops.object.mode_set(mode="POSE")
    bpy.ops.pose.armature_apply()
    bpy.ops.object.mode_set(mode="OBJECT")
    bpy.context.view_layer.objects.active = was


def solve_the_corrections(rig, forward, across):
    """The corrections that land every measurement on target, as a fixed point.

    Applying a measured correction once does not work: straightening a thigh swings the
    foot below it, so the foot's measured yaw changes, and the foot's own correction
    axis then rides a corrected parent. Measured, a single proximal-to-distal pass left
    the toe-out at 5.89 degrees against a target of 8.00.

    So the whole correction is applied to a SCRATCH COPY, the residual is measured off
    that, and it is added in. Two or three passes and there is nothing left. The real
    rig is not touched here at all.
    """
    axes = {
        "Thigh": tuple(forward),  # abduction: about the line of travel
        "Calf": tuple(across),  # knee flexion: about the axis across the body
        "Foot": (0.0, 0.0, 1.0),  # toe-out: about the vertical
    }
    total = {}
    for side in "LR":
        for part in ("Thigh", "Calf", "Foot"):
            total[f"{side}_{part}"] = 0.0

    for attempt in range(PASSES):
        scratch = rig.copy()
        scratch.data = rig.data.copy()
        bpy.context.collection.objects.link(scratch)
        try:
            pose_it(
                scratch,
                {
                    bone: in_rest_basis(scratch, bone, axes[bone.split("_")[1]], amount)
                    for bone, amount in total.items()
                },
            )
            make_it_the_rest_pose(scratch)
            left = how_far_off(scratch, *which_way_is_forward(scratch))
        finally:
            bpy.data.objects.remove(scratch, do_unlink=True)

        worst = max(abs(v) for v in left.values())
        print(
            f"  pass {attempt + 1}: worst residual {worst:+.4f} deg  "
            + ", ".join(f"{k} {v:+.2f}" for k, v in sorted(left.items()))
        )
        for bone, amount in left.items():
            total[bone] += amount
        if worst < CLOSE_ENOUGH:
            break

    return {
        bone: in_rest_basis(rig, bone, axes[bone.split("_")[1]], amount)
        for bone, amount in total.items()
        if abs(amount) > 1e-9
    }


def deform_matrices(rig, mesh):
    """What the armature modifier does to each vertex: sum of w * pose * rest^-1."""
    per_bone = {
        posed.name: posed.matrix @ posed.bone.matrix_local.inverted()
        for posed in rig.pose.bones
    }
    groups = {g.index: g.name for g in mesh.vertex_groups}
    out = []
    for vertex in mesh.data.vertices:
        piled = mathutils.Matrix.Diagonal((0.0, 0.0, 0.0, 0.0))
        weight = 0.0
        for item in vertex.groups:
            matrix = per_bone.get(groups.get(item.group))
            if matrix is None or item.weight == 0.0:
                continue
            weight += item.weight
            for row in range(4):
                for col in range(4):
                    piled[row][col] += matrix[row][col] * item.weight
        if weight <= 1e-9:
            out.append(mathutils.Matrix.Identity(4))
            continue
        for row in range(4):
            for col in range(4):
                piled[row][col] /= weight
        out.append(piled)
    return out


def bake_the_skin(rig, mesh) -> None:
    """Writes the current deformation into the mesh data.

    Without this, `armature_apply` moves the bones and the skin snaps straight back -
    measured, mesh vertex data changes by exactly 0.000000, because the bones moved and
    nothing told the vertices.

    Shape keys, if there are any, are the thing that actually drives the shape while
    they exist, so writes to `vertices[].co` are inert and the key blocks have to be
    transformed instead. Doing both would apply the deformation twice.
    """
    matrices = deform_matrices(rig, mesh)
    keys = mesh.data.shape_keys
    if keys:
        for block in keys.key_blocks:
            for i, point in enumerate(block.data):
                point.co = matrices[i] @ point.co
        print(f"  baked into {len(keys.key_blocks)} shape key block(s)")
    else:
        for i, vertex in enumerate(mesh.data.vertices):
            vertex.co = matrices[i] @ vertex.co
        print(f"  baked into {len(mesh.data.vertices)} vertices")
    mesh.data.update()


def main() -> None:
    args = argv()
    if len(args) < 2:
        raise SystemExit("need <in.glb> <out.glb>")
    source, out = args[0], args[1]

    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=source)
    rig = next(o for o in bpy.data.objects if o.type == "ARMATURE")
    body = max(
        (o for o in bpy.data.objects if o.type == "MESH"),
        key=lambda o: len(o.data.vertices),
    )
    for posed in rig.pose.bones:
        posed.rotation_mode = "QUATERNION"
    flush()

    forward, across = which_way_is_forward(rig)
    print(f"forward is ({forward.x:+.3f}, {forward.y:+.3f}) at rest")
    before = {
        "L splay": leg_splay(rig, "L", forward, across),
        "R splay": leg_splay(rig, "R", forward, across),
        "L knee": knee_bend(rig, "L", forward),
        "R knee": knee_bend(rig, "R", forward),
        "L toe-out": toe_out(rig, "L", forward, across),
        "R toe-out": toe_out(rig, "R", forward, across),
    }
    print("as it arrived: " + ", ".join(f"{k} {v:+.2f}" for k, v in before.items()))

    print(f"solving for legs at {LEGS_SIT_AT:+.1f} and toes at {TOES_SIT_AT:+.1f}:")
    turns = solve_the_corrections(rig, forward, across)
    for bone, turn in sorted(turns.items()):
        print(f"  {bone}: {math.degrees(turn.angle):+.2f} deg in its own rest basis")

    # The order is load-bearing: pose it, write the deformation into the MESH, and only
    # then make the pose the rest. The other way round leaves the skin behind.
    pose_it(rig, turns)
    bake_the_skin(rig, body)
    make_it_the_rest_pose(rig)
    flush()

    forward, across = which_way_is_forward(rig)
    after = {
        "L splay": leg_splay(rig, "L", forward, across),
        "R splay": leg_splay(rig, "R", forward, across),
        "L knee": knee_bend(rig, "L", forward),
        "R knee": knee_bend(rig, "R", forward),
        "L toe-out": toe_out(rig, "L", forward, across),
        "R toe-out": toe_out(rig, "R", forward, across),
    }
    print("now standing: " + ", ".join(f"{k} {v:+.2f}" for k, v in after.items()))

    worst = max(
        abs(after["L splay"] - LEGS_SIT_AT),
        abs(after["R splay"] - LEGS_SIT_AT),
        abs(after["L toe-out"] - TOES_SIT_AT),
        abs(after["R toe-out"] - TOES_SIT_AT),
    )
    if worst > 0.5:
        raise SystemExit(
            f"the rest pose is still {worst:.2f} deg off target; refusing to write it"
        )

    bpy.ops.export_scene.gltf(
        filepath=out,
        export_format="GLB",
        export_yup=True,
        export_animation_mode="ACTIONS",
        export_animations=True,
        export_skins=True,
        export_morph=False,
        export_apply=False,
    )
    print(f"WROTE {out}")
    print(f"clips carried through: {[a.name for a in bpy.data.actions]}")


main()
