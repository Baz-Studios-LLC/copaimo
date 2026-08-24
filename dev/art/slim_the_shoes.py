"""Takes the bulk out of the shoes, without opening a seam where the trouser meets them.

    blender --background --python slim_the_shoes.py -- [--dry-run] [--long 0.155] [--wide 0.40]

As delivered, measured by `anatomy_audit.py`:

    L  31.7 cm long, 13.8 wide, 10.0 tall   18.6% of standing height
    R  31.5 cm long, 13.5 wide, 12.0 tall   18.5%

An adult foot is about 15% of height and about 40% of its own length across. These were a quarter
longer than that, which is what "bulky" looks like as a number.

# A TARGET, not a scale factor - and this is the second attempt

The first version multiplied by 0.86 and got it wrong in two ways that are worth keeping written
down, because both were invisible in the numbers it printed:

It was NOT IDEMPOTENT. Run it twice and the shoe shrinks twice, so the committed asset's shoe size
depended on how many times the script had been run over it. A target converges instead: measure,
work out the factor needed to reach the target, apply that. Run it again and nothing moves.

And it faded the scale BY DISTANCE ALONG THE SHOE from the ankle, so the toe got the full slimming
and the width beside the ankle got almost none. Measured after that pass, length fell 13.9% and
width only 6.2% - so width went from 43.5% of length to 47%. The shoe came out shorter and
PROPORTIONALLY CHUBBIER, which is exactly why it was reported as still bulky.

The seam that can actually open is at the TOP, where the trouser cuff comes down over the collar.
That is a vertical overlap, so the exemption belongs on a vertical measure - the slimming now
fades out with HEIGHT and leaves the last couple of centimetres of collar alone.

# Horizontally only

The shoe stands from the floor to about 11 cm with the ankle joint at 7.1, so scaling it
vertically about the sole would drop the top of the shoe below the bone that drives it. Length and
width are what bulk means here anyway.

# Nothing needs re-authoring afterwards

`foot_roll.foot_landmarks` measures the ball, the sole and the tip FROM THE MESH rather than from
constants, so the clips re-solve against the new shoe on the next build. Measured across the first
slimming, foot slide did not move at all: walk 0.046, run 0.263, sprint 0.198.
"""
import os
import sys

import bpy
import mathutils

ART = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, ART)

import prepare_rig  # noqa: E402

SCALE = 170.0

# How long the shoe should be, as a share of standing height.
#
# 0.155 is 26.4 cm on this figure, against the 15% a real adult foot is. A little generous on
# purpose: he reads as a teenager in chunky trainers, and a shoe scaled onto an anthropometric
# table would be a different design rather than a less bulky version of this one.
LONG_AS_A_SHARE_OF_HEIGHT = 0.155

# How wide it should be, as a share of its own length. 0.40 is what a foot usually is.
WIDE_AS_A_SHARE_OF_LENGTH = 0.40

# Height above the sole, in centimetres, at which the slimming has eased off to nothing.
#
# The ankle joint sits at 7.1 cm and the shoe stands to about 11, so this leaves the top couple of
# centimetres of collar at full size for the trouser cuff to meet, and slims everything below.
COLLAR_KEEPS_ITS_SIZE_ABOVE = 9.0

# How far the shoe's nearest vertex may end up from the ankle joint before the collar is judged to
# have pulled away from the leg.
COLLAR_STAYS_WITHIN = 7.0  # cm


def refuse(why):
    raise SystemExit(f"REFUSED: {why}")


def argv():
    return sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []


def smoothly(at):
    at = min(1.0, max(0.0, at))
    return at * at * (3.0 - 2.0 * at)


def measure(points, forward, up):
    """The shoe's own long and wide axes, and its size along each."""
    import numpy

    flat = [q - up * q.dot(up) for q in points]
    cloud = numpy.array([[p.x, p.y, p.z] for p in flat])
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
    every = argv()
    dry = "--dry-run" in every
    wants_long = float(every[every.index("--long") + 1]) if "--long" in every \
        else LONG_AS_A_SHARE_OF_HEIGHT
    wants_wide = float(every[every.index("--wide") + 1]) if "--wide" in every \
        else WIDE_AS_A_SHARE_OF_LENGTH
    source = args[0] if args else os.path.join(ART, "ranger_apose.glb").replace("\\", "/")
    out_path = args[1] if len(args) > 1 else source

    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=source)
    rig = next(o for o in bpy.data.objects if o.type == "ARMATURE")
    mesh = prepare_rig.the_body()
    prepare_rig.reach_the_ends(rig, mesh)
    prepare_rig.drop_the_widgets(rig)
    across, forward, up = prepare_rig.body_frame(rig)

    groups = {g.index: g.name for g in mesh.vertex_groups}
    mine = {"L": [], "R": []}
    for vertex in mesh.data.vertices:
        best, who = 0.0, ""
        for group in vertex.groups:
            if group.weight > best:
                best, who = group.weight, groups.get(group.group, "")
        if who.endswith("_Foot") or who.endswith("_ToeBase"):
            mine[who[0]].append(vertex.index)

    everything = [mesh.matrix_world @ v.co for v in mesh.data.vertices]
    tall = max(p.z for p in everything) - min(p.z for p in everything)
    floor = min(p.z for p in everything)
    target_long = tall * wants_long
    print(f"reading {source}")
    print(f"  a {tall * SCALE:.1f} cm figure, so aiming for shoes {target_long * SCALE:.1f} cm "
          f"long and {target_long * wants_wide * SCALE:.1f} wide")

    into_mesh = mesh.matrix_world.inverted()
    for side in "LR":
        if len(mine[side]) < 20:
            refuse(f"only {len(mine[side])} vertices belong to the {side} shoe")
        spots = [mesh.matrix_world @ mesh.data.vertices[i].co for i in mine[side]]
        long_way, wide_way, length, width = measure(spots, forward, up)
        # The factor needed to REACH the target, which is what makes this converge.
        shrink_long = min(1.0, target_long / max(length, 1e-9))
        shrink_wide = min(1.0, (target_long * wants_wide) / max(width, 1e-9))
        print(f"  {side} is {length * SCALE:5.1f} x {width * SCALE:4.1f} cm "
              f"({length / tall * 100:4.1f}% of height, {width / length * 100:4.1f}% as wide as "
              f"long) -> x{shrink_long:.3f} long, x{shrink_wide:.3f} wide")

        ankle = rig.matrix_world @ rig.pose.bones[f"{side}_Foot"].head
        middle = sum(spots, mathutils.Vector()) / len(spots)
        for index in mine[side]:
            spot = mesh.matrix_world @ mesh.data.vertices[index].co
            # Faded by HEIGHT: full slimming at the sole, nothing at the collar. See the note.
            above = (spot.z - floor) * SCALE
            keeps = smoothly(above / max(COLLAR_KEEPS_ITS_SIZE_ABOVE, 1e-9))
            long_by = 1.0 + (shrink_long - 1.0) * (1.0 - keeps)
            wide_by = 1.0 + (shrink_wide - 1.0) * (1.0 - keeps)
            # About the shoe's own middle for length and the ANKLE for width, so it narrows onto
            # the leg rather than sliding sideways off it.
            from_middle = spot - middle
            from_ankle = spot - ankle
            moved = (
                spot
                - long_way * from_middle.dot(long_way)
                + long_way * from_middle.dot(long_way) * long_by
                - wide_way * from_ankle.dot(wide_way)
                + wide_way * from_ankle.dot(wide_way) * wide_by
            )
            mesh.data.vertices[index].co = into_mesh @ moved

    mesh.data.update()
    print("  after:")
    for side in "LR":
        spots = [mesh.matrix_world @ mesh.data.vertices[i].co for i in mine[side]]
        _, _, length, width = measure(spots, forward, up)
        print(f"    {side} {length * SCALE:5.1f} x {width * SCALE:4.1f} cm "
              f"({length / tall * 100:4.1f}% of height, {width / length * 100:4.1f}% as wide as "
              f"long)")
        ankle = rig.matrix_world @ rig.pose.bones[f"{side}_Foot"].head
        nearest = min((p - ankle).length for p in spots) * SCALE
        if nearest > COLLAR_STAYS_WITHIN:
            refuse(f"the {side} shoe's nearest vertex is {nearest:.1f} cm from the ankle, past "
                   f"{COLLAR_STAYS_WITHIN} - the collar has pulled away from the leg")

    if not mesh.data.has_custom_normals:
        refuse("slimming lost the custom split normals, which lights the character as a "
               "different shape - see the note on welding in prepare_rig")
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
