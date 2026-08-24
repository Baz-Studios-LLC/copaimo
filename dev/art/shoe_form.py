"""Gives the shoes the shape of shoes. NOT CALLED - it made them worse. Read this first.

# Why this is not in the build

Rendered textured, in order, against every earlier state of the shoe, this pass is plainly the
worst of them: the toe curls up, the slab sole flattens out, and a chunky trainer reads as
something soft. Reported as "both look like UGGs".

The mistake is not in the code below, which does what it says. It is in what the code was
measured AGAINST. A toe spring of 1-2 cm and a tapering toe box are real shoe-last proportions
and they are correct for a real shoe. This is a STYLISED chunky trainer, and its whole design
is a blunt full toe box on a thick slab sole - the two things a toe spring and a taper sand
off. Judged against an anthropometric table it improved; judged by how it READS it got worse,
and how it reads is the only thing that matters here.

That is the same fault as the three passes that slimmed the footprint before it. Each time the
shoe was compared against the wrong thing.

The measurements are kept because they are true and were expensive: 64 welded vertices, the
split count that hides it, the two mechanical bugs below. Anyone reaching for a toe spring on
this character should find out here rather than after four rounds.


    blender --background --python shoe_form.py -- [--dry-run] [--cuts 2]

# What was wrong, and why three passes of slimming never touched it

Reported bulky, slimmed, reported bulky, slimmed again, and then: "These are not shoes."

They were not. Rendered in CLAY - every material replaced with plain grey, which is the only
way to see form on a character whose texture is doing the work - each shoe is a rounded wedge
with a flat vertical cliff where the toe should be. The laces, the midsole stripe and the heel
tab are all PAINTED. There is no sole, no toe spring, no toe taper, no heel counter.

The number behind it: welded, each shoe is SIXTY-FOUR VERTICES. Not 190 - that is the split
count, and glTF splits a vertex at every UV seam and every hard edge. Sixty-four vertices
cannot describe a shoe. No scale factor applied to a 64-vertex blob makes a shoe, which is why
three passes of tuning its length, width and sole thickness could never have worked.

So: give it geometry, then give it the features a shoe has.

# The correction this file exists to record

`slim_the_shoes` said the shoe could not be made shorter because "the leg mesh starts at 9.7 cm
and the rim tops out at 10.0, so the shoe swallows the leg by three millimetres" - and that
lowering the collar would leave the ankle ending in mid-air. That was WRONG, and wrong in an
instructive way.

9.7 cm is where the leg's OWNERSHIP changes hands, not where its surface stops. Welded, the
shoe and the bottom of the leg are one closed shell of 102 vertices with 25 edges joining them
and ZERO open edges. There is no hole to expose. The claim came from `is_boundary` on a mesh
where every vertex is split, which reports every edge as a boundary and is therefore an answer
to no question at all.

*Weld before asking a topology question.* Positions survive the export; connectivity does not.

# The three features, and why these three

Each is measured against what a shoe IS, and each was measured as ABSENT here first:

  TOE SPRING   the sole lifts clear of the ground toward the tip, so the shoe rolls off it
               rather than catching. Measured: the sole was flat to the tip. This is the single
               most recognisable thing a shoe has and the thing the clay render screamed about.
  TOE TAPER    the toe box narrows and drops toward the tip. Measured: full width and full
               height right up to a vertical cliff at the front.
  SOLE SHELF   a step where the midsole meets the upper, so the sole reads as a separate part.
               Measured: one continuous surface, with the texture drawing a midsole across it.

Nothing here touches the collar or the ankle join. The shoe is reshaped from the ball forward
and from the sole up, which is where all three faults are.
"""
import math
import os
import sys

import bpy
import mathutils

ART = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, ART)

import prepare_rig  # noqa: E402
import slim_the_shoes  # noqa: E402

SCALE = 170.0
UP = mathutils.Vector((0.0, 0.0, 1.0))

# How many times to cut the shoe's faces. 2 takes a 64-vertex shoe to roughly 900, which is
# where a game shoe usually sits and is enough to hold a toe spring and a sole shelf without
# either reading as a single fold.
CUTS = 2

# Where along the shoe the toe spring starts lifting, as a share of its length from the heel.
#
# 0.68 is the ball of the foot, which is where a shoe actually breaks. Starting it further back
# lifts the whole forefoot off the ground and the character walks on his heels.
SPRING_STARTS = 0.68

# How far the tip of the sole ends up off the ground, as a share of the shoe's length.
#
# 0.055 is 1.45 cm on this shoe. Trainers run 1-2 cm; a shoe with none catches its toe, which
# is exactly the flat cliff this replaces.
SPRING_LIFTS = 0.055

# How much of its width the toe keeps at the very tip, and how much of its height.
TOE_NARROWS_TO = 0.62
TOE_LOWERS_TO = 0.72

# Where the toe taper begins, as a share of length. Later than the spring: a shoe's widest
# point is the ball, and narrowing from there would take the width out of the part of the foot
# that actually needs it.
TAPER_STARTS = 0.72

# The sole shelf: how far the sole stands proud of the upper, in cm, and how far above it the
# step has blended away.
#
# 0.25 cm is a visible edge at arm's length without becoming a flange. The height it sits at
# comes from `slim_the_shoes.sole_reaches`, so the shelf lands wherever the sole actually is
# rather than at a number typed here that would drift the moment the sole is re-tuned.
SHELF_STANDS_PROUD = 0.25
SHELF_ROUNDS_OVER = 0.9

# What the result has to satisfy afterwards. These compare against what a shoe IS, not against
# the constants that produced them - a factor can be applied in full and still not have done
# anything, and only the second kind of check notices.
TIP_MUST_CLEAR = 0.8            # cm of daylight under the toe
SOLE_MUST_STILL_TOUCH = 0.05    # cm: the ball stays on the floor
NOTHING_MOVES_MORE_THAN = 4.0   # cm


def refuse(why):
    raise SystemExit(f"REFUSED: {why}")


def argv():
    return sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []


def smoothly(at):
    at = min(1.0, max(0.0, at))
    return at * at * (3.0 - 2.0 * at)


def shoe_vertices(mesh):
    """Which vertices belong to which shoe, by the bone that drives them most."""
    groups = {g.index: g.name for g in mesh.vertex_groups}
    mine = {"L": [], "R": []}
    for vertex in mesh.data.vertices:
        best, who = 0.0, ""
        for group in vertex.groups:
            if group.weight > best:
                best, who = group.weight, groups.get(group.group, "")
        if who.endswith("_Foot") or who.endswith("_ToeBase"):
            mine[who[0]].append(vertex.index)
    return mine


def owners(mesh):
    """Which bone drives each vertex most, by name - what `the_leg_stays_covered` wants."""
    groups = {g.index: g.name for g in mesh.vertex_groups}
    who = {}
    for vertex in mesh.data.vertices:
        best, name = 0.0, ""
        for group in vertex.groups:
            if group.weight > best:
                best, name = group.weight, groups.get(group.group, "")
        who[vertex.index] = name
    return who


def frame_of(spots, forward):
    """The shoe's own axes and where its heel is, so everything below reads the same way."""
    long_way, wide_way, length, width = slim_the_shoes.measure(spots, forward, UP)
    heel = min(p.dot(long_way) for p in spots)
    middle = (max(p.dot(wide_way) for p in spots) + min(p.dot(wide_way) for p in spots)) * 0.5
    return long_way, wide_way, length, heel, middle


def springs_by(along, lift):
    """How far the sole has lifted at this point along the shoe.

    Squared, so the ball stays flat and the sole curves up from there the way a sole does. A
    linear ramp puts a visible kink at the hinge, which reads as a crease rather than a curve.
    """
    if along <= SPRING_STARTS:
        return 0.0
    at = (along - SPRING_STARTS) / max(1.0 - SPRING_STARTS, 1e-9)
    return lift * at * at


def shape_one(mesh, indices, forward, floor, shelf_at, talk=True):
    """Puts a toe spring, a toe taper and a sole shelf on one shoe. Returns how far it moved."""
    into_mesh = mesh.matrix_world.inverted()
    spots = [mesh.matrix_world @ mesh.data.vertices[i].co for i in indices]
    long_way, wide_way, length, heel, middle = frame_of(spots, forward)
    half = max((max(p.dot(wide_way) for p in spots)
                - min(p.dot(wide_way) for p in spots)) * 0.5, 1e-9)
    lift = length * SPRING_LIFTS
    if talk:
        print(f"    a {length * SCALE:.1f} cm shoe: the tip lifts {lift * SCALE:.2f} cm from "
              f"{SPRING_STARTS * 100:.0f}% along, the taper starts at "
              f"{TAPER_STARTS * 100:.0f}%, the shelf sits at {shelf_at:.1f} cm")

    worst = 0.0
    for index in indices:
        spot = mesh.matrix_world @ mesh.data.vertices[index].co
        was = spot.copy()
        along = (spot.dot(long_way) - heel) / max(length, 1e-9)
        above = (spot.z - floor) * SCALE

        # TOE SPRING. The whole toe box rises together, so this is a hinge at the ball rather
        # than a crease in the sole.
        rose = springs_by(along, lift)
        spot.z += rose

        # TOE TAPER. Width toward the shoe's own centreline and height toward the sole, both
        # eased in, so the forefoot is untouched and only the tip is drawn in. Height is scaled
        # about the SPRUNG sole under this vertex, not about the floor, or the taper would
        # quietly undo the lift it just got.
        if along > TAPER_STARTS:
            at = smoothly((along - TAPER_STARTS) / max(1.0 - TAPER_STARTS, 1e-9))
            across = spot.dot(wide_way) - middle
            spot += wide_way * across * (TOE_NARROWS_TO - 1.0) * at
            under = floor + rose
            spot.z = under + (spot.z - under) * (1.0 + (TOE_LOWERS_TO - 1.0) * at)

        # SOLE SHELF. The sole stands proud of the upper and blends away above it, so the
        # midsole the texture already draws has an edge to sit on.
        #
        # Scaled outward from the shoe's own centreline, NOT pushed by a fixed amount. The
        # first version used copysign, which moved every vertex the full 0.25 cm whether it
        # sat at the edge of the sole or on the flat underside - so the underside split down
        # its middle and the whole sole came out scalloped and melted. Scaling leaves the
        # centreline where it is and gives the outermost vertex the full step.
        if above < shelf_at + SHELF_ROUNDS_OVER:
            fades = 1.0 - smoothly(max(0.0, above - shelf_at) / SHELF_ROUNDS_OVER)
            across = spot.dot(wide_way) - middle
            spot += wide_way * across * (SHELF_STANDS_PROUD / SCALE / half) * fades

        mesh.data.vertices[index].co = into_mesh @ spot
        worst = max(worst, (spot - was).length * SCALE)
    return worst


def looks_like_a_shoe(mesh, indices, forward, floor, side):
    """Refuses unless the three features are actually there to be seen."""
    spots = [mesh.matrix_world @ mesh.data.vertices[i].co for i in indices]
    long_way, wide_way, length, heel, _ = frame_of(spots, forward)

    def band(lo, hi):
        return [p for p in spots if lo <= (p.dot(long_way) - heel) / max(length, 1e-9) <= hi]

    def wide(points):
        if not points:
            return 0.0
        return (max(p.dot(wide_way) for p in points)
                - min(p.dot(wide_way) for p in points)) * SCALE

    tip, ball = band(0.95, 1.01), band(0.40, 0.60)
    if not tip or not ball:
        refuse(f"cannot find the {side} shoe's tip or its ball to measure them")
    clears = (min(p.z for p in tip) - floor) * SCALE
    touches = (min(p.z for p in ball) - floor) * SCALE
    print(f"    {side} tip clears the ground by {clears:.2f} cm, the ball sits {touches:.2f} "
          f"off it, {wide(band(0.90, 1.01)):.1f} cm wide at the toe against "
          f"{wide(band(0.55, 0.75)):.1f} at the ball")
    if clears < TIP_MUST_CLEAR:
        refuse(f"the {side} toe clears the ground by {clears:.2f} cm, under {TIP_MUST_CLEAR} - "
               f"there is no toe spring to see")
    if touches > SOLE_MUST_STILL_TOUCH:
        refuse(f"the {side} ball of the foot is {touches:.2f} cm off the ground - the shoe is "
               f"balanced on its heel")
    if wide(band(0.90, 1.01)) >= wide(band(0.55, 0.75)):
        refuse(f"the {side} toe is no narrower than the ball, so it does not taper - it is "
               f"still a wall")


def main():
    every = argv()
    args = [a for a in every if not a.startswith("--")]
    dry = "--dry-run" in every
    cuts = int(every[every.index("--cuts") + 1]) if "--cuts" in every else CUTS
    source = args[0] if args else os.path.join(ART, "ranger_apose.glb").replace("\\", "/")
    out_path = args[1] if len(args) > 1 else source

    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=source)
    rig = next(o for o in bpy.data.objects if o.type == "ARMATURE")
    mesh = prepare_rig.the_body()
    prepare_rig.reach_the_ends(rig, mesh)
    prepare_rig.drop_the_widgets(rig)
    _, forward, _ = prepare_rig.body_frame(rig)
    print(f"reading {source}")

    # GEOMETRY FIRST. Only faces wholly inside a shoe are cut, so the join to the leg keeps the
    # topology it has and nothing has to be re-stitched afterwards.
    mine = shoe_vertices(mesh)

    # THIS DOES NOT CONVERGE, so it refuses rather than running twice.
    #
    # `slim_the_shoes` is written as targets and a second run is a no-op. This one cannot be:
    # subdividing again would double the geometry and the toe would spring twice. So the
    # already-shaped case is REFUSED, and the way back is git rather than a second pass.
    floor_first = min((mesh.matrix_world @ v.co).z for v in mesh.data.vertices)
    for side in "LR":
        spots = [mesh.matrix_world @ mesh.data.vertices[i].co for i in mine[side]]
        long_way, _, length, heel, _ = frame_of(spots, forward)
        tip = [p for p in spots if (p.dot(long_way) - heel) / max(length, 1e-9) > 0.95]
        if tip and (min(p.z for p in tip) - floor_first) * SCALE >= TIP_MUST_CLEAR:
            refuse(f"the {side} toe already clears the ground by "
                   f"{(min(p.z for p in tip) - floor_first) * SCALE:.2f} cm, so this has been "
                   f"run before - restore the asset from git rather than shaping it twice")

    # WHERE THE SOLE IS, measured before the mesh gets any denser.
    #
    # `sole_reaches` finds the highest vertex sitting on the shoe's widest silhouette, which is
    # the top of the sole on a coarse mesh and NONSENSE on a fine one: subdividing puts new
    # vertices along the shoe's side at nearly full width and much higher up, and the answer
    # jumped from 3.0 cm to 10.0 - the top of the shoe. The shelf would then have been applied
    # to the whole shoe. Measured here, on the mesh the detector was written for.
    floor_now = min((mesh.matrix_world @ v.co).z for v in mesh.data.vertices)
    shelf = {}
    for side in "LR":
        spots = [mesh.matrix_world @ mesh.data.vertices[i].co for i in mine[side]]
        _, wide_way, _, _, _ = frame_of(spots, forward)
        shelf[side] = slim_the_shoes.sole_reaches(spots, wide_way, floor_now)
    print(f"  the sole tops out at L {shelf['L']:.1f} cm, R {shelf['R']:.1f} cm")

    belongs = set(mine["L"]) | set(mine["R"])
    inside = [p.index for p in mesh.data.polygons if all(v in belongs for v in p.vertices)]
    before = len(mesh.data.vertices)
    print(f"  {len(belongs)} shoe vertices in {len(inside)} faces, cutting them {cuts}x")
    prepare_rig.subdivide_these(mesh, inside, cuts)
    mine = shoe_vertices(mesh)
    print(f"  the body went from {before} to {len(mesh.data.vertices)} vertices; the shoes "
          f"now hold {len(mine['L'])} + {len(mine['R'])}")
    if len(mesh.data.vertices) <= before:
        refuse("subdividing added no vertices, so there is nothing new to shape")

    floor = min((mesh.matrix_world @ v.co).z for v in mesh.data.vertices)
    print("  shaping:")
    for side in "LR":
        worst = shape_one(mesh, mine[side], forward, floor, shelf[side])
        print(f"    {side} moved at most {worst:.2f} cm")
        if worst > NOTHING_MOVES_MORE_THAN:
            refuse(f"a {side} shoe vertex moved {worst:.2f} cm, past "
                   f"{NOTHING_MOVES_MORE_THAN} - that is a fold, not a shape")
    mesh.data.update()

    # And rebuild the shading over what was just changed. Subdivision interpolates the custom
    # split normals onto the new loops, and on a mesh with no connectivity to smooth across
    # that produces the lobed, melted read this shoe came back with twice. See `reshade`.
    prepare_rig.reshade(mesh, set(mine["L"]) | set(mine["R"]))

    print("  after:")
    who = owners(mesh)
    for side in "LR":
        looks_like_a_shoe(mesh, mine[side], forward, floor, side)
        slim_the_shoes.the_leg_stays_covered(mesh, who, side)

    if not mesh.data.has_custom_normals:
        refuse("shaping lost the custom split normals, which lights the character as a "
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
