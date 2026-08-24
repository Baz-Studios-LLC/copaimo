"""Takes the bulk out of the shoes. NOT CALLED - the shoes were not the problem. Read this.

The shoes were slimmed twice by this file and thinned once, and then restored to as-delivered
on request: "do whatever you did for the old animation, those were perfect". Every target below
is a correct number about a real foot and beside the point for a stylised chunky trainer, whose
design IS a blunt toe box on a thick slab sole. See TROUBLESHOOTING.md.

Kept because the measurements are true and because the next person to compare this shoe against
an anthropometric table should find out here.


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

# The third report of "still bulky", and what it turned out to be

The footprint reached its target and the shoe was reported bulky again. That is what a STYLISED
judgement looks like: the plan dimension measures textbook-correct and the silhouette is still a
brick. Measured by height band, the reason:

    height above the floor   0-1   1-2   2-3   3-4   4-5   ...   9-10 cm
    width of the shoe there  10.6   7.8  10.6   8.7   4.7         6.3 cm

The shoe is at its FULL WIDTH up to 3 cm and narrows above it. That bottom 3 cm is the sole, and
3 cm of sole under a 26.4 cm shoe is 11% of its length. A trainer's sole is about 2 cm at the
heel, 7-8%. So the shoe was standing on a PLATFORM, and two passes over the footprint could never
have found that - neither of them measured a vertical.

# The collar rim must not move, and that is measured, not preferred

An earlier note here said scaling vertically would drop the top of the shoe below the ankle bone.
The real constraint is worse than that and worth writing down properly:

    the leg mesh (L_CalfTwist02) starts at 9.7 cm; the shoe's rim tops out at 10.0

The shoe covers the bottom of the leg by THREE MILLIMETRES. Lowering the collar by any useful
amount opens a hole at the ankle - the leg would end in mid-air. So the shoe is thinned FROM
UNDERNEATH: the sole is squashed onto the floor, and the thinning eases back to nothing by the
collar, which does not move at all. `the_leg_stays_covered` checks it afterwards rather than
trusting that description.

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

# How thick the sole should be, as a share of the shoe's own length.
#
# 0.076 is 2.0 cm on this shoe, against the 3.0 it arrived with. That is a trainer's sole rather
# than a platform, and it is the dimension neither earlier pass touched. See the note above.
SOLE_AS_A_SHARE_OF_LENGTH = 0.076

# Height above the floor, in cm, by which the thinning has eased back to nothing.
#
# 9.0 leaves the collar rim - which starts at about 9.3 - exactly where it is. See the note above
# for why that is a hard constraint and not a nicety.
SOLE_THINNING_FADES_BY = 9.0

# How much of the shoe's own height the sole is allowed to be before the detection is disbelieved.
#
# The sole is found as "how high the widest part of the shoe reaches", which is a measurement and
# can therefore be wrong - a wide strap high on the ankle would fool it. This compares the answer
# against what a sole IS rather than against the number that produced it.
A_SOLE_IS_THE_BOTTOM = 0.45


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


def sole_reaches(spots, wide_way, floor):
    """How high above the floor the WIDEST part of the shoe reaches, in cm.

    The sole flares out past the upper, so the vertices sitting on the shoe's widest silhouette
    are the flange, and the top of the flange is the top of the sole. Found from the mesh and not
    from a bone, because a bone does not move when the mesh is squashed - detecting the sole from
    `ToeBase` would have squashed it again on every run.
    """
    lateral = [p.dot(wide_way) for p in spots]
    middle = (max(lateral) + min(lateral)) * 0.5
    half = (max(lateral) - min(lateral)) * 0.5
    edge = [p for p, x in zip(spots, lateral) if abs(x - middle) >= half * 0.93]
    return (max(p.z for p in edge) - floor) * SCALE


def thinner(above, was, wants, fades_by):
    """Where a height above the floor should end up, in cm, with only the sole squashed.

    Piecewise linear through (0, 0), (was, wants) and (fades_by, fades_by), identity above that.
    Written as a remap rather than a per-vertex factor so it is provably monotone: a factor faded
    by height can reorder two vertices that were the right way round to start with.
    """
    if wants >= was:
        return above           # already thin enough; this is what makes a second run a no-op
    if above <= was:
        return above * wants / was
    if above >= fades_by:
        return above
    return wants + (above - was) * (fades_by - wants) / (fades_by - was)


def the_leg_stays_covered(mesh, owner, side, talk=True):
    """Refuses if the shoe's rim no longer reaches the bottom of the leg.

    The leg mesh simply stops where the shoe swallows it, so this is not cosmetic: a rim that
    drops below it leaves the ankle ending in mid-air. Measured as delivered, the margin is 0.3 cm
    on the left - which is why nothing here is allowed to lower the collar.
    """
    at = {i: mesh.matrix_world @ v.co for i, v in enumerate(mesh.data.vertices)}
    shoe = [i for i in at if owner.get(i) in (f"{side}_Foot", f"{side}_ToeBase")]
    leg = [i for i in at if owner.get(i, "").startswith(side)
           and "Foot" not in owner.get(i, "") and "Toe" not in owner.get(i, "")]
    if not shoe or not leg:
        refuse(f"cannot find the {side} shoe or the {side} leg to compare them")
    rim = max(at[i].z for i in shoe)
    ends = min(at[i].z for i in leg)
    covers = (rim - ends) * SCALE
    if talk:
        print(f"    {side} rim reaches {covers:+.1f} cm past the bottom of the leg")
    if covers < 0.0:
        refuse(f"the {side} shoe's rim is {-covers:.1f} cm BELOW the bottom of the leg - the "
               f"ankle would end in mid-air")


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
    owner = {}
    for vertex in mesh.data.vertices:
        best, who = 0.0, ""
        for group in vertex.groups:
            if group.weight > best:
                best, who = group.weight, groups.get(group.group, "")
        owner[vertex.index] = who
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

    # THE SOLE, second, because how high the widest part of the shoe reaches is a measurement and
    # the pass above moves it. Re-measured here rather than carried down from the loop.
    print("  the sole:")
    for side in "LR":
        spots = [mesh.matrix_world @ mesh.data.vertices[i].co for i in mine[side]]
        long_way, wide_way, length, width = measure(spots, forward, up)
        stands = (max(p.z for p in spots) - floor) * SCALE
        was = sole_reaches(spots, wide_way, floor)
        wants = length * SCALE * SOLE_AS_A_SHARE_OF_LENGTH
        share = was / max(stands, 1e-9)
        print(f"    {side} sole is {was:.1f} cm thick under a {length * SCALE:.1f} cm shoe "
              f"({was / (length * SCALE) * 100:.1f}% of its length, {share * 100:.0f}% of the "
              f"shoe's height) -> aiming for {wants:.1f} cm")
        # Compared against what a sole IS, not against the number that produced it. A wide strap
        # high on the ankle would fool the detection, and this is what would catch it.
        if share > A_SOLE_IS_THE_BOTTOM:
            refuse(f"the widest part of the {side} shoe reaches {was:.1f} cm, which is "
                   f"{share * 100:.0f}% of the way up it - that is not a sole, and squashing "
                   f"everything below it would squash the shoe")
        if was <= wants:
            print(f"    {side} sole is already {was:.1f} cm, nothing to thin")
            continue
        if was >= SOLE_THINNING_FADES_BY:
            refuse(f"the {side} sole reaches {was:.1f} cm, at or past the "
                   f"{SOLE_THINNING_FADES_BY} cm the thinning has to have faded out by - there "
                   f"is no room left to ease it back into the collar")
        for index in mine[side]:
            spot = mesh.matrix_world @ mesh.data.vertices[index].co
            above = (spot.z - floor) * SCALE
            spot.z = floor + thinner(above, was, wants, SOLE_THINNING_FADES_BY) / SCALE
            mesh.data.vertices[index].co = into_mesh @ spot

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
        _, wide_way, _, _ = measure(spots, forward, up)
        print(f"      sole {sole_reaches(spots, wide_way, floor):.1f} cm thick, shoe stands "
              f"{(max(p.z for p in spots) - floor) * SCALE:.1f} cm tall")
        the_leg_stays_covered(mesh, owner, side)

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
