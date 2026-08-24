"""Gives the torso the joint it is missing, and names the one above it for what it is.

    blender --background --python add_spine.py -- [--dry-run] [<in.glb> <out.glb>]

`docs/rigging.md` puts a standard game spine at five joints - a pelvis, three spine, and a chest.
This rig had four: Pelvis, Waist, Spine01, Spine02. Which one was missing is not a matter of
counting, though, and `look_at_the_spine.py` is what settled it:

    bone       length   drives   its skin runs
    Waist       10.0 cm      27   79 to  95 cm
    Spine01     19.9 cm     403   92 to 122 cm      <- one joint over 30 cm of back
    Spine02     17.5 cm     140  118 to 139 cm

Both clavicles AND the neck leave at Spine02's tail, so SPINE02 IS ALREADY THE CHEST. The gap is
lower down: Spine01 alone carries 403 vertices across thirty centimetres, which is two joints'
work, and a torso that bends there bends all in one place.

So: split Spine01, and rename Spine02 to `Chest`.

# Why the rename, when renames are usually not worth it

Because without it the new bone would have to be called something other than `Spine02`, and the
existing `Spine02` would go on being posed by name in three places that mean THE UPPER TORSO by
it. Insert a bone below that and those three lines silently start driving the middle of the back
instead. A rename is loud; a name that quietly means something else is not.

`Chest` is also simply more accurate: it is the bone the arms and the head hang from.

`prepare_rig` is untouched. It works on the ORIGINAL delivery, where `Spine02` still means what
it always did, and keeping the two apart is what stops there being two naming worlds.
"""
import math
import os
import sys

import bpy
import mathutils

ART = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, ART)

import prepare_rig  # noqa: E402

SCALE = 170.0

WAS_THE_CHEST = "Spine02"
CHEST = "Chest"
LOWER = "Spine01"
ADDED = "Spine02"

# Where along Spine01 the new joint goes, as a share of its length from the hips up.
#
# Halfway. The bone is 19.9 cm long and drives an even spread of skin, so there is no measured
# reason to favour either end - and a lumbar spine does bend about its middle. Stated as a share
# rather than a height so it survives the model being rescaled.
SPLITS_AT = 0.5

# How much of the old bone's span the handover is spread over, as a share of its length.
#
# A hard cut at one ring of vertices creases the back exactly there. This blends across a third of
# the bone, which is the same reasoning as `JOINT_BLENDS` in src/ik.rs and for the same reason.
BLENDS_OVER = 0.34

REST_MUST_NOT_MOVE = 1e-6
CARRIES_INFLUENCES = 4


def refuse(why):
    raise SystemExit(f"REFUSED: {why}")


def argv():
    return sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []


def deformed_now(mesh):
    depsgraph = bpy.context.evaluated_depsgraph_get()
    evaluated = mesh.evaluated_get(depsgraph)
    got = evaluated.to_mesh()
    spots = [mesh.matrix_world @ v.co.copy() for v in got.vertices]
    evaluated.to_mesh_clear()
    return spots


def smoothly(at):
    """Smoothstep, so the handover has no corner in it."""
    at = min(1.0, max(0.0, at))
    return at * at * (3.0 - 2.0 * at)


def main():
    args = [a for a in argv() if not a.startswith("--")]
    dry = "--dry-run" in argv()
    source = args[0] if args else os.path.join(ART, "ranger_apose.glb").replace("\\", "/")
    out_path = args[1] if len(args) > 1 else source

    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=source)
    rig = next(o for o in bpy.data.objects if o.type == "ARMATURE")
    mesh = prepare_rig.the_body()
    prepare_rig.reach_the_ends(rig, mesh)
    prepare_rig.drop_the_widgets(rig)

    print(f"reading {source}")
    print(f"  {len(rig.data.bones)} bones before")
    if ADDED in rig.data.bones and CHEST in rig.data.bones:
        refuse("this rig already has both a Chest and a new Spine02 - it has been run before")
    if WAS_THE_CHEST not in rig.data.bones:
        refuse(f"no {WAS_THE_CHEST} to rename; this is not the rig this was written for")

    was = deformed_now(mesh)

    # Where the old lower bone runs, before anything is touched.
    lower = rig.data.bones[LOWER]
    head = rig.matrix_world @ lower.matrix_local.translation
    tail = rig.matrix_world @ (
        lower.matrix_local @ mathutils.Vector((0.0, lower.length, 0.0))
    )
    along = tail - head
    print(f"  {LOWER} runs {head.z * SCALE:.1f} to {tail.z * SCALE:.1f} cm, "
          f"{along.length * SCALE:.1f} cm long")

    # 1. The rename, and the vertex group with it or the skin stops following.
    rig.data.bones[WAS_THE_CHEST].name = CHEST
    group = mesh.vertex_groups.get(WAS_THE_CHEST)
    if group is not None:
        group.name = CHEST
    print(f"  {WAS_THE_CHEST} -> {CHEST}, and its vertex group with it")

    # 2. The new bone, taking the upper part of the old one's span.
    with prepare_rig.in_edit_mode(rig) as edit:
        old = edit[LOWER]
        chest = edit[CHEST]
        middle = old.head.lerp(old.tail, SPLITS_AT)
        fresh = edit.new(ADDED)
        fresh.head = middle
        fresh.tail = old.tail.copy()
        fresh.roll = old.roll
        fresh.parent = old
        fresh.use_connect = True
        fresh.use_deform = True
        old.tail = middle
        # The chest hangs off the NEW bone now, not the old one - that is what putting a joint
        # into a chain means, and forgetting it would leave the new bone driving nothing that
        # anything above it can feel.
        chest.parent = fresh
        chest.use_connect = True
    print(f"  {ADDED} inserted, {LOWER} now {rig.data.bones[LOWER].length * SCALE:.1f} cm and "
          f"{ADDED} {rig.data.bones[ADDED].length * SCALE:.1f} cm")

    # 3. The weights. Every vertex the old bone drives has its share SPLIT between the two by
    #    where it sits along the original span - redistributed, never added to, so the total a
    #    vertex carries is exactly what it carried before.
    lower_group = mesh.vertex_groups.get(LOWER)
    if lower_group is None:
        refuse(f"nothing is weighted to {LOWER}, so there is no skin to divide")
    upper_group = mesh.vertex_groups.get(ADDED) or mesh.vertex_groups.new(name=ADDED)

    moved, shared = 0, 0
    for vertex in mesh.data.vertices:
        held = next((g.weight for g in vertex.groups if g.group == lower_group.index), 0.0)
        if held <= 0.0:
            continue
        spot = mesh.matrix_world @ vertex.co
        at = (spot - head).dot(along) / along.length_squared
        # 0 below the handover, 1 above it, smooth across it.
        share = smoothly((at - (SPLITS_AT - BLENDS_OVER / 2.0)) / BLENDS_OVER)
        if share <= 0.0:
            continue
        upper_group.add([vertex.index], held * share, "REPLACE")
        if share >= 1.0:
            lower_group.remove([vertex.index])
            moved += 1
        else:
            lower_group.add([vertex.index], held * (1.0 - share), "REPLACE")
            shared += 1
    print(f"  {moved} vertices handed to {ADDED}, {shared} shared across the join")

    bpy.context.view_layer.objects.active = mesh
    bpy.ops.object.vertex_group_limit_total(limit=CARRIES_INFLUENCES)
    bpy.ops.object.vertex_group_normalize_all(lock_active=False)

    print(f"  {len(rig.data.bones)} bones after")
    prepare_rig.check_the_skin(mesh)

    # The rest pose must not move. Splitting a bone in two along its own line, and dividing the
    # skin between the halves, is a change of ARTICULATION and not of shape - the two halves rest
    # exactly where the whole did. Anything else means the new bone is not collinear with the old.
    now = deformed_now(mesh)
    if len(now) != len(was):
        refuse(f"the vertex count changed, {len(was)} to {len(now)}")
    stirred = max((a - b).length for a, b in zip(was, now))
    print(f"  the rest pose moved {stirred * SCALE * 10000:.4f} microns at most")
    if stirred > REST_MUST_NOT_MOVE:
        refuse(f"the rest pose moved {stirred * SCALE:.4f} cm; splitting a bone must not "
               f"change the shape")

    # And the chain must be what was asked for, in order, or something above is parented wrong.
    wanted = ["Waist", LOWER, ADDED, CHEST]
    at = rig.data.bones[CHEST]
    chain = []
    while at is not None and len(chain) < 8:
        chain.append(at.name)
        at = at.parent
    chain.reverse()
    if chain[-len(wanted):] != wanted:
        refuse(f"the spine runs {chain}, which does not end in {wanted}")
    print(f"  the spine now runs {' -> '.join(chain)}")
    for who in ("L_Clavicle", "R_Clavicle", "NeckTwist01"):
        if rig.data.bones[who].parent.name != CHEST:
            refuse(f"{who} hangs off {rig.data.bones[who].parent.name}, not the {CHEST}")

    if dry:
        print("\ndry run, nothing written")
        return

    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.export_scene.gltf(
        filepath=out_path,
        export_format="GLB",
        use_selection=True,
        export_yup=True,
        export_apply=False,
        export_animations=False,
    )
    print(f"\nwrote {out_path}")


if __name__ == "__main__":
    main()
