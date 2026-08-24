"""Takes the bulk out of the shoes, without opening a seam where the trouser meets them.

    blender --background --python slim_the_shoes.py -- [--dry-run] [--to 0.86]

Reported as "very bulky", and measured by `anatomy_audit.py`:

    L  31.7 cm long, 13.8 wide, 10.0 tall   18.6% of standing height
    R  31.5 cm long, 13.5 wide, 12.0 tall   18.5%

An adult foot is about 15% of height. These are a quarter longer than that, which is what bulk
looks like as a number.

# Horizontally only, and tapered to nothing at the ankle

Two constraints decide the shape of this.

The shoe is skinned to `_Foot` and `_ToeBase` and the trouser cuff above it is skinned to the
calf, so shrinking the shoe uniformly pulls it away from a cuff that has not moved and opens a
ring-shaped hole at the join. So the scale FADES to 1.0 near the ankle: the shoe keeps its
opening exactly and loses its bulk at the toe, which is where the bulk is.

And horizontally only. The shoe stands from the floor to about 11 cm with the ankle joint at 7.1,
so scaling it vertically about the sole would drop the top of the shoe below the bone that drives
it. Length and width are what "bulky" means here anyway.

# Nothing needs re-authoring afterwards

`foot_roll.foot_landmarks` measures the ball, the sole and the tip FROM THE MESH rather than from
constants, so the clips re-solve against the new shoe on the next build. That is the payoff for
having measured them in the first place.
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

# What to multiply the shoe's length and width by, out at the toe.
#
# 0.86 takes 31.7 cm to 27.3, which is 16.0% of standing height against the 18.6 it was and the
# ~15 a real foot is. Deliberately NOT all the way to 15: this character is stylised and reads
# as a teenager in chunky trainers, and a shoe scaled to an anthropometric table would be a
# different design rather than a less bulky version of this one.
SLIMS_TO = 0.86

# How far from the ankle the full slimming has come in, as a share of the shoe's own length.
#
# Below this the scale eases back toward 1.0 so the shoe's opening is untouched and the cuff
# above it still meets it. A third of the shoe is roughly the ankle and its collar.
FADES_IN_OVER = 0.34

REST_MUST_NOT_MOVE_AT_THE_ANKLE = 0.002   # model units, about 3 mm at this scale


def refuse(why):
    raise SystemExit(f"REFUSED: {why}")


def argv():
    return sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []


def smoothly(at):
    at = min(1.0, max(0.0, at))
    return at * at * (3.0 - 2.0 * at)


def measure(points, forward, up):
    long_way = None
    import numpy

    cloud = numpy.array([[p.x, p.y, p.z] for p in
                         [q - up * q.dot(up) for q in points]])
    cloud = cloud - cloud.mean(axis=0)
    _u, _s, axes = numpy.linalg.svd(cloud, full_matrices=False)
    long_way = mathutils.Vector(axes[0]).normalized()
    if long_way.dot(forward) < 0:
        long_way = -long_way
    wide_way = up.cross(long_way).normalized()
    return (
        long_way,
        wide_way,
        max(p.dot(long_way) for p in points) - min(p.dot(long_way) for p in points),
        max(p.dot(wide_way) for p in points) - min(p.dot(wide_way) for p in points),
    )


def main():
    args = [a for a in argv() if not a.startswith("--")]
    dry = "--dry-run" in argv()
    every = argv()
    slims = float(every[every.index("--to") + 1]) if "--to" in every else SLIMS_TO
    source = args[0] if args else os.path.join(ART, "ranger_apose.glb").replace("\\", "/")
    out_path = args[1] if len(args) > 1 else source

    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=source)
    rig = next(o for o in bpy.data.objects if o.type == "ARMATURE")
    mesh = prepare_rig.the_body()
    prepare_rig.reach_the_ends(rig, mesh)
    prepare_rig.drop_the_widgets(rig)
    across, forward, up = prepare_rig.body_frame(rig)
    print(f"reading {source}, slimming to {slims}")

    groups = {g.index: g.name for g in mesh.vertex_groups}
    mine = {"L": [], "R": []}
    for vertex in mesh.data.vertices:
        best, who = 0.0, ""
        for group in vertex.groups:
            if group.weight > best:
                best, who = group.weight, groups.get(group.group, "")
        if who.endswith("_Foot") or who.endswith("_ToeBase"):
            mine[who[0]].append(vertex.index)

    tall = (max((mesh.matrix_world @ v.co).z for v in mesh.data.vertices)
            - min((mesh.matrix_world @ v.co).z for v in mesh.data.vertices))
    before = {}
    for side in "LR":
        if len(mine[side]) < 20:
            refuse(f"only {len(mine[side])} vertices belong to the {side} shoe")
        spots = [mesh.matrix_world @ mesh.data.vertices[i].co for i in mine[side]]
        _, _, length, width = measure(spots, forward, up)
        before[side] = (length, width)
        print(f"  {side} before: {length * SCALE:5.1f} cm long, {width * SCALE:5.1f} wide "
              f"({length / tall * 100:.1f}% of height)")

    into_mesh = mesh.matrix_world.inverted()
    for side in "LR":
        ankle = rig.matrix_world @ rig.pose.bones[f"{side}_Foot"].head
        spots = [mesh.matrix_world @ mesh.data.vertices[i].co for i in mine[side]]
        long_way, wide_way, length, _ = measure(spots, forward, up)
        for index in mine[side]:
            spot = mesh.matrix_world @ mesh.data.vertices[index].co
            from_ankle = spot - ankle
            # Fade in with distance from the ankle, measured along the shoe rather than through
            # the air, so the collar is untouched and the toe gets the whole of it.
            reach = abs(from_ankle.dot(long_way)) / max(length, 1e-9)
            eased = 1.0 + (slims - 1.0) * smoothly(reach / FADES_IN_OVER)
            flat = (from_ankle.dot(long_way) * long_way) + (from_ankle.dot(wide_way) * wide_way)
            moved = spot - flat + flat * eased
            mesh.data.vertices[index].co = into_mesh @ moved

    mesh.data.update()
    for side in "LR":
        spots = [mesh.matrix_world @ mesh.data.vertices[i].co for i in mine[side]]
        _, _, length, width = measure(spots, forward, up)
        was_long, was_wide = before[side]
        print(f"  {side} after:  {length * SCALE:5.1f} cm long, {width * SCALE:5.1f} wide "
              f"({length / tall * 100:.1f}% of height)  "
              f"-{(1 - length / was_long) * 100:.1f}% / -{(1 - width / was_wide) * 100:.1f}%")

    # The ankle's own collar must not have moved, or the trouser cuff no longer meets the shoe.
    for side in "LR":
        ankle = rig.matrix_world @ rig.pose.bones[f"{side}_Foot"].head
        nearest = min(
            ((mesh.matrix_world @ mesh.data.vertices[i].co) - ankle).length for i in mine[side]
        )
        if nearest > REST_MUST_NOT_MOVE_AT_THE_ANKLE + 0.05:
            refuse(f"the {side} shoe's nearest vertex is now {nearest * SCALE:.1f} cm from the "
                   f"ankle, so the collar has pulled away from the leg")

    if not mesh.data.has_custom_normals:
        refuse("slimming lost the custom split normals, which lights the character as a "
               "different shape - see the note on welding in prepare_rig")
    print(f"  {len(mesh.data.vertices)} vertices, custom split normals: "
          f"{mesh.data.has_custom_normals}")
    prepare_rig.check_the_skin(mesh)

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
