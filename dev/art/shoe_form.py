"""Reshapes the shoes into sneakers. NOT CALLED - there is not enough mesh. Read this first.

Counted properly, each shoe is 64 welded vertices and 113 triangles. There is no collar rim, no
tongue, no throat and no sole unit anywhere in that, and measured through the ankle the shoe and
the leg are ONE SMOOTH TUBE - the shoe tops out around 9-10 cm and the calf continues, tapering,
with no edge at any height. A shoe with no rim reads as a sock, and it cannot be reshaped into a
sneaker because the things that make a sneaker are not there to reshape. They are painted.

So this file cannot finish the job, and everything below is kept for the rebuild that can:
`TOP_OF_THE_SHOE` is a sneaker's side profile station by station, `WIDTH_OF_THE_SHOE` its plan
outline, and `cut_the_collar` puts an edge where the shoe ends - all correct, all starved of
vertices.

Three things it cost, worth reading before touching this character's mesh again:

  THE TEXTURE LOCKS THE SHAPE. The midsole stripe and outsole sit at fixed places in UV space
  and only line up while the geometry keeps its proportions. Any non-uniform reshape slides the
  paint off the form.
  A LAST IS A CEILING, NOT A TARGET. Fitting the outline both ways pushed vertices OUT wherever
  the shoe was narrower than the last wanted, and it came out on a wide flat plate. Narrowing
  what is too wide is slimming; widening what is too narrow invents a shoe that is not there.
  A GUARD MUST KNOW ITS BASELINE. `the_ankle_stays_joined` refused at an absolute 2.2 cm, on a
  junction that already carried 7.96 cm edges before anything was done to it.


    blender --background --python shoe_form.py -- [--dry-run] [--cuts 2]

# What this is, and what the last attempt got wrong

The shoe arrives as a rounded wedge with a flat cliff for a toe and a tall soft tube for a
collar. Welded, it is SIXTY-FOUR VERTICES - not the 190 an audit prints, which is the split
count, since glTF splits a vertex at every UV seam and hard edge. The laces, the midsole stripe
and the heel tab are all painted onto it. It is a boot-shaped blob with a sneaker drawn on.

The previous version of this file added a toe spring and a toe taper - correct shoe-last
features, both wrong here. They sand off the blunt toe box and the slab sole, and the result
was a moccasin. See TROUBLESHOOTING.md.

So this one does not add FEATURES to the blob. It fits the blob to a LAST: a table of what a
sneaker's silhouette is, station by station from heel to toe, in height and in width. Every
shoe vertex is remapped onto that envelope, which keeps the panels and straps where they are
and forces the outline to be a sneaker's.

# The four things that make a sneaker read as one, in the order they matter

  LOW COLLAR    a sneaker's opening sits at or below the ankle bone and DIPS at the front.
                Measured as delivered: 10 cm tall on a 31.7 cm shoe - and a tall soft tube is
                the boot read. A low-top is under a third of its own length. This is the
                biggest single change and the one every earlier pass missed, because they all
                worked on the footprint.
  A SOLE UNIT   a midsole slab of even thickness with a HARD top edge, standing slightly proud.
                Held at a constant height while the upper above it is compressed, so lowering
                the collar does not quietly thin the sole by the same fraction.
  INSTEP CURVE  the top drops from the collar over the instep to a low toe box. The blob is
                nearly flat along the top, which is what makes it read as a clog.
  TOE BOX       rounded and narrowing in plan, but still BLUNT in profile. Not a spring, not a
                point. The tip stays wide enough to look like a shoe rather than a slipper, and
                `looks_like_a_sneaker` refuses if it does not.

# The collar can come down, and this is why that is safe

An earlier note claimed the collar could not be lowered without leaving the ankle in mid-air,
because the leg mesh starts at 9.7 cm and the rim tops out at 10.0. That was wrong. 9.7 cm is
where the leg's OWNERSHIP changes hands, not where its surface stops - welded, the shoe and the
bottom of the leg are one closed shell with ZERO open edges. Lowering the rim stretches the
faces into the ankle rather than opening anything.

It does stretch them, so `carry_the_leg_down` brings the bottom of the leg with the collar and
eases back to nothing up the shin. Without that the ankle becomes a funnel, and
`the_ankle_stays_joined` measures the longest bridging edge afterwards rather than trusting it.

# Not idempotent, on purpose

Subdividing again would double the geometry, and the fit would run against its own output. It
refuses if the shoe is already at sneaker height, and the way back is git.
"""
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

# How many times to cut the shoe's faces. 2 takes a 64-vertex shoe to roughly 250 welded, which
# is where a game shoe sits and is enough to hold a collar dip and a sole edge.
CUTS = 2

# The footprint, as shares of standing height and of the shoe's own length.
LONG_AS_A_SHARE_OF_HEIGHT = 0.155     # 26.4 cm here, against the 31.7 it arrives at
WIDE_AS_A_SHARE_OF_LENGTH = 0.395

# The sole slab: how thick, as a share of the shoe's length, and how far it stands proud of the
# upper in cm.
SOLE_AS_A_SHARE_OF_LENGTH = 0.076     # 2.0 cm
SOLE_STANDS_PROUD = 0.45
SOLE_EDGE_IS_SHARP_WITHIN = 0.30      # cm of blend above the slab, so the step stays a step

# THE LAST. What a sneaker's silhouette is, station by station from the back of the heel (0.0)
# to the tip of the toe (1.0).
#
# Heights in cm above the floor, so the table reads as a side view. The peak at 0.12 is the heel
# counter, the dip at 0.34 is the throat where the topline drops, and the fall from there to the
# toe is the instep curve. 7.5 cm on a 26.4 cm shoe is 28% of its length - a low-top, against
# the 38% a 10 cm collar makes of it.
TOP_OF_THE_SHOE = (
    (0.00, 6.6), (0.12, 7.5), (0.24, 7.2), (0.34, 6.1), (0.48, 5.5),
    (0.62, 4.8), (0.78, 4.1), (0.90, 3.5), (1.00, 3.0),
)

# Half-width as a share of the widest point, which on a foot is the ball at about 0.62. Blunt at
# the tip on purpose: 0.50 keeps a toe box where the moccasin attempt narrowed it to 0.33.
WIDTH_OF_THE_SHOE = (
    (0.00, 0.44), (0.10, 0.70), (0.25, 0.82), (0.45, 0.92), (0.62, 1.00),
    (0.78, 0.94), (0.90, 0.78), (1.00, 0.50),
)

# THE COLLAR RIM: how far the shoe's opening stands proud of the ankle, and over what height the
# step blends away, both in cm.
#
# This is the one that matters, and every earlier pass missed it by working on the sole. Measured
# through the ankle, the shoe and the leg are ONE SMOOTH TUBE - the shoe tops out around 9-10 cm
# and the calf simply continues, tapering, with no edge anywhere. A shoe with no rim is a SOCK,
# which is exactly what it reads as. A sneaker's collar stands proud of the ankle inside it, and
# that edge is most of what says "shoe" in a silhouette.
RIM_STANDS_PROUD = 0.55
ANKLE_DRAWS_IN = 0.40
RIM_BLENDS_OVER = 1.8

# How far up the leg the pull-down has eased back to nothing, in cm above the floor. The trouser
# cuff sits well above this, so only bare ankle is stretched.
LEG_SETTLES_BY = 26.0

# What the result has to satisfy, stated as what a sneaker IS rather than as the constants that
# produced it - a factor can be applied in full and still not have done anything.
COLLAR_IS_LOW_ENOUGH = 0.32           # of the shoe's own length
SOLE_MUST_STILL_TOUCH = 0.05          # cm: the shoe stands on the floor
TOE_STAYS_BLUNT = 0.40                # of the widest half-width
# How much longer the shoe-to-leg junction's longest edge may get, as a multiple of what it was
# BEFORE the fit.
#
# Against the baseline, not against a number typed here. The first version of this guard used an
# absolute 2.2 cm and refused immediately - measured, that junction already carries edges of
# 7.96 cm on a 4.32 cm median, because the generator bridged the ankle with a few huge polygons.
# The fit had in fact SHORTENED the worst one to 7.33. A guard that does not know what it is
# comparing against reports the mesh's history as though it were your change.
THE_ANKLE_STAYS_JOINED = 1.25


def refuse(why):
    raise SystemExit(f"REFUSED: {why}")


def argv():
    return sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []


def smoothly(at):
    at = min(1.0, max(0.0, at))
    return at * at * (3.0 - 2.0 * at)


def along_the_table(table, at):
    """Reads a station table, straight-line between its entries."""
    at = min(1.0, max(0.0, at))
    for (x0, y0), (x1, y1) in zip(table, table[1:]):
        if at <= x1:
            return y0 + (y1 - y0) * (at - x0) / max(x1 - x0, 1e-9)
    return table[-1][1]


def named(mesh):
    """Which bone drives each vertex most, by name."""
    groups = {g.index: g.name for g in mesh.vertex_groups}
    who = {}
    for vertex in mesh.data.vertices:
        best, name = 0.0, ""
        for group in vertex.groups:
            if group.weight > best:
                best, name = group.weight, groups.get(group.group, "")
        who[vertex.index] = name
    return who


def shoe_vertices(mesh):
    """Which vertices belong to which shoe, by the bone that drives them most."""
    mine = {"L": [], "R": []}
    for index, name in named(mesh).items():
        if name.endswith("_Foot") or name.endswith("_ToeBase"):
            mine[name[0]].append(index)
    return mine


def frame_of(spots, forward):
    """The shoe's own axes, its length, its heel and its centreline."""
    long_way, wide_way, length, _ = slim_the_shoes.measure(spots, forward, UP)
    heel = min(p.dot(long_way) for p in spots)
    middle = (max(p.dot(wide_way) for p in spots) + min(p.dot(wide_way) for p in spots)) * 0.5
    return long_way, wide_way, length, heel, middle


def envelope(spots, long_way, wide_way, length, heel, middle, floor, bins=16):
    """How wide and how tall the shoe currently is at each station, smoothed.

    Smoothed because a station bin holds only a few dozen vertices, and the fit DIVIDES by these
    numbers - so noise in them comes out as scallops in the shoe.
    """
    wide = [0.0] * bins
    tall = [0.0] * bins
    for spot in spots:
        at = min(bins - 1, max(0, int((spot.dot(long_way) - heel) / max(length, 1e-9) * bins)))
        wide[at] = max(wide[at], abs(spot.dot(wide_way) - middle))
        tall[at] = max(tall[at], (spot.z - floor) * SCALE)
    for row in (wide, tall):
        for _ in range(2):
            row[:] = [(row[max(0, i - 1)] + row[i] * 2.0 + row[min(bins - 1, i + 1)]) / 4.0
                      for i in range(bins)]
    return wide, tall


def read_off(row, at):
    """A station's value from a binned row, straight-line between bin centres."""
    place = min(len(row) - 1.0, max(0.0, at * len(row) - 0.5))
    low = int(place)
    high = min(len(row) - 1, low + 1)
    return row[low] + (row[high] - row[low]) * (place - low)


def fit_to_the_last(mesh, indices, forward, floor, talk=True):
    """Remaps one shoe onto the sneaker last. Returns how far the furthest vertex moved."""
    into_mesh = mesh.matrix_world.inverted()
    spots = [mesh.matrix_world @ mesh.data.vertices[i].co for i in indices]
    long_way, wide_way, length, heel, middle = frame_of(spots, forward)
    standing = (max((mesh.matrix_world @ v.co).z for v in mesh.data.vertices) - floor) * SCALE

    # THE FOOTPRINT FIRST, so the last is fitted to a shoe that is already the right size.
    wants_long = standing * LONG_AS_A_SHARE_OF_HEIGHT
    wants_wide = wants_long * WIDE_AS_A_SHARE_OF_LENGTH
    was_wide = (max(p.dot(wide_way) for p in spots)
                - min(p.dot(wide_way) for p in spots)) * SCALE
    shrink_long = min(1.0, wants_long / max(length * SCALE, 1e-9))
    shrink_wide = min(1.0, wants_wide / max(was_wide, 1e-9))
    sole = wants_long * SOLE_AS_A_SHARE_OF_LENGTH
    if talk:
        print(f"    {length * SCALE:.1f} x {was_wide:.1f} cm -> {wants_long:.1f} x "
              f"{wants_wide:.1f}, collar to {along_the_table(TOP_OF_THE_SHOE, 0.12):.1f} cm, "
              f"sole {sole:.1f} cm")

    for index in indices:
        spot = mesh.matrix_world @ mesh.data.vertices[index].co
        spot += long_way * (spot.dot(long_way) - heel) * (shrink_long - 1.0)
        spot += wide_way * (spot.dot(wide_way) - middle) * (shrink_wide - 1.0)
        mesh.data.vertices[index].co = into_mesh @ spot

    # Re-measured, because the pass above moved everything it is about to be measured against.
    spots = [mesh.matrix_world @ mesh.data.vertices[i].co for i in indices]
    long_way, wide_way, length, heel, middle = frame_of(spots, forward)
    wide, tall = envelope(spots, long_way, wide_way, length, heel, middle, floor)
    widest = max(max(wide), 1e-9)

    worst = 0.0
    for index in indices:
        spot = mesh.matrix_world @ mesh.data.vertices[index].co
        was = spot.copy()
        at = (spot.dot(long_way) - heel) / max(length, 1e-9)

        # WIDTH onto the last's plan outline, but only ever INWARD.
        #
        # Fitting both ways splayed the sole: wherever the shoe was narrower than the last wants
        # - the toe, the heel - every vertex at that station was pushed OUT, sole included, and
        # the shoe came out standing on a wide flat slab like a clown shoe. The last is a
        # ceiling on this outline, not a target to be met from below. Narrowing what is too wide
        # is a slimming; widening what is too narrow is inventing a shoe that is not there.
        here = max(read_off(wide, at), 1e-9)
        pulls_in = min(1.0, widest * along_the_table(WIDTH_OF_THE_SHOE, at) / here)
        across = spot.dot(wide_way) - middle
        spot += wide_way * across * (pulls_in - 1.0)

        # HEIGHT onto the last's profile, THE WHOLE COLUMN, sole included.
        #
        # The first version held the sole at a fixed 2.0 cm and compressed only the upper onto
        # it, on the reasoning that lowering the collar should not thin the slab. Rendered, that
        # is worse, and the reason is the TEXTURE: the midsole stripe and the outsole are
        # painted, at fixed places in UV space, and they only line up while the geometry keeps
        # its proportions. Holding the sole while the upper shrank left the painted white band
        # riding above a black slab that had become half the shoe.
        #
        # A texture authored for one shape constrains how far that shape may be changed. Scaling
        # the whole column keeps every painted band where it belongs.
        above = (spot.z - floor) * SCALE
        top_now = max(read_off(tall, at), 0.1)
        top_wants = max(along_the_table(TOP_OF_THE_SHOE, at), 0.1)
        above *= min(1.0, top_wants / top_now)
        spot.z = floor + above / SCALE

        # THE SOLE EDGE. Scaled outward from the centreline rather than pushed by a fixed
        # amount, so the flat underside does not split down its middle and scallop - which is
        # what the first attempt at this did.
        sole_here = sole * top_wants / max(along_the_table(TOP_OF_THE_SHOE, 0.12), 0.1)
        if above < sole_here + SOLE_EDGE_IS_SHARP_WITHIN:
            fades = 1.0 - smoothly(max(0.0, above - sole_here) / SOLE_EDGE_IS_SHARP_WITHIN)
            across = spot.dot(wide_way) - middle
            spot += wide_way * across * (SOLE_STANDS_PROUD / SCALE / widest) * fades

        mesh.data.vertices[index].co = into_mesh @ spot
        worst = max(worst, (spot - was).length * SCALE)
    return worst


def cut_the_collar(mesh, who, side, floor, talk=True):
    """Puts an edge where the shoe ends and the ankle begins.

    The shoe's opening is pushed out and the ankle just above it drawn in, both fading over
    `RIM_BLENDS_OVER`, so there is a step between them instead of one continuous tube. Scaled
    from the ankle's own axis rather than pushed by a fixed amount, so the rim stays a ring
    rather than sliding sideways off the leg.
    """
    into_mesh = mesh.matrix_world.inverted()
    shoe = [i for i, n in who.items() if n in (f"{side}_Foot", f"{side}_ToeBase")]
    leg = [i for i, n in who.items()
           if n.startswith(side) and "Foot" not in n and "Toe" not in n]
    at = {i: mesh.matrix_world @ mesh.data.vertices[i].co for i in shoe + leg}
    rim = max((at[i].z - floor) * SCALE for i in shoe)
    axis = sum((mathutils.Vector((at[i].x, at[i].y, 0.0)) for i in shoe
                if (at[i].z - floor) * SCALE > rim - 1.0), mathutils.Vector())
    near = sum(1 for i in shoe if (at[i].z - floor) * SCALE > rim - 1.0)
    axis /= max(near, 1)

    moved = 0
    for index, by in [(i, RIM_STANDS_PROUD) for i in shoe] +                      [(i, -ANKLE_DRAWS_IN) for i in leg]:
        spot = at[index]
        above = (spot.z - floor) * SCALE
        gap = abs(above - rim)
        if gap > RIM_BLENDS_OVER:
            continue
        if (by > 0.0) != (above <= rim + 1e-6):
            continue
        out = mathutils.Vector((spot.x - axis.x, spot.y - axis.y, 0.0))
        if out.length < 1e-9:
            continue
        fades = 1.0 - smoothly(gap / RIM_BLENDS_OVER)
        spot = spot + out.normalized() * (by / SCALE) * fades
        mesh.data.vertices[index].co = into_mesh @ spot
        moved += 1
    if talk:
        print(f"    {side} rim at {rim:.1f} cm: {moved} vertices stepped, shoe out "
              f"{RIM_STANDS_PROUD:.2f} cm and ankle in {ANKLE_DRAWS_IN:.2f}")
    return rim


def the_collar_has_an_edge(mesh, who, side, floor, rim):
    """Refuses unless the shoe is measurably wider than the ankle just above it."""
    shoe = [i for i, n in who.items() if n in (f"{side}_Foot", f"{side}_ToeBase")]
    leg = [i for i, n in who.items()
           if n.startswith(side) and "Foot" not in n and "Toe" not in n]
    at = {i: mesh.matrix_world @ mesh.data.vertices[i].co for i in shoe + leg}
    axis = sum((mathutils.Vector((at[i].x, at[i].y, 0.0)) for i in shoe
                if (at[i].z - floor) * SCALE > rim - 1.0), mathutils.Vector())
    near = [i for i in shoe if (at[i].z - floor) * SCALE > rim - 1.0]
    axis /= max(len(near), 1)

    def spread(which, lo, hi):
        out = [mathutils.Vector((at[i].x - axis.x, at[i].y - axis.y, 0.0)).length * SCALE
               for i in which if lo <= (at[i].z - floor) * SCALE <= hi]
        return max(out) if out else 0.0

    opening = spread(shoe, rim - 1.2, rim + 0.1)
    ankle = spread(leg, rim + 0.1, rim + 1.6)
    step = opening - ankle
    print(f"    {side}: collar {opening:.2f} cm from the ankle's axis against {ankle:.2f} just "
          f"above it - a {step:+.2f} cm step")
    if step < 0.25:
        refuse(f"the {side} collar is only {step:+.2f} cm proud of the ankle above it, so there "
               f"is no edge where the shoe ends - it reads as a sock, which is the fault this "
               f"is here to fix")


def carry_the_leg_down(mesh, who, side, dropped, floor):
    """Brings the bottom of the leg down with the collar, easing back to nothing up the shin."""
    into_mesh = mesh.matrix_world.inverted()
    moved = 0
    for index, name in who.items():
        if not name.startswith(side) or "Foot" in name or "Toe" in name:
            continue
        spot = mesh.matrix_world @ mesh.data.vertices[index].co
        above = (spot.z - floor) * SCALE
        if above >= LEG_SETTLES_BY:
            continue
        spot.z -= dropped / SCALE * (1.0 - smoothly(above / LEG_SETTLES_BY))
        mesh.data.vertices[index].co = into_mesh @ spot
        moved += 1
    return moved


def looks_like_a_sneaker(mesh, indices, forward, floor, side):
    """Refuses unless the silhouette is a sneaker's. Statements about shoes, not about knobs."""
    spots = [mesh.matrix_world @ mesh.data.vertices[i].co for i in indices]
    long_way, wide_way, length, heel, middle = frame_of(spots, forward)

    def band(lo, hi):
        return [p for p in spots if lo <= (p.dot(long_way) - heel) / max(length, 1e-9) <= hi]

    def half(points):
        return max((abs(p.dot(wide_way) - middle) for p in points), default=0.0)

    stands = (max(p.z for p in spots) - floor) * SCALE
    sits = (min(p.z for p in spots) - floor) * SCALE
    share = stands / max(length * SCALE, 1e-9)
    blunt = half(band(0.93, 1.01)) / max(half(band(0.55, 0.72)), 1e-9)
    print(f"    {side}: {length * SCALE:.1f} cm long, {half(spots) * 2 * SCALE:.1f} wide, "
          f"{stands:.1f} tall ({share * 100:.0f}% of its length), toe {blunt * 100:.0f}% as "
          f"wide as the ball")
    if share > COLLAR_IS_LOW_ENOUGH:
        refuse(f"the {side} shoe stands {share * 100:.0f}% of its own length tall, past "
               f"{COLLAR_IS_LOW_ENOUGH * 100:.0f} - that is still a boot, not a low-top")
    if sits > SOLE_MUST_STILL_TOUCH:
        refuse(f"the {side} shoe floats {sits:.2f} cm off the floor")
    if blunt < TOE_STAYS_BLUNT:
        refuse(f"the {side} toe narrowed to {blunt * 100:.0f}% of the ball, under "
               f"{TOE_STAYS_BLUNT * 100:.0f} - that is a moccasin, which is the last mistake")


def the_ankle_join(mesh, who, side):
    """The longest edge bridging the shoe to the leg, in cm."""
    shoe = {i for i, n in who.items() if n in (f"{side}_Foot", f"{side}_ToeBase")}
    leg = {i for i, n in who.items()
           if n.startswith(side) and "Foot" not in n and "Toe" not in n}
    longest = 0.0
    for edge in mesh.data.edges:
        a, b = edge.vertices
        if (a in shoe and b in leg) or (b in shoe and a in leg):
            longest = max(longest, ((mesh.matrix_world @ mesh.data.vertices[a].co)
                                    - (mesh.matrix_world @ mesh.data.vertices[b].co)).length)
    return longest * SCALE


def the_ankle_stays_joined(mesh, who, side, was):
    """Refuses if lowering the collar stretched the junction into a funnel."""
    now = the_ankle_join(mesh, who, side)
    print(f"    {side}: longest edge across the ankle join {was:.2f} -> {now:.2f} cm")
    if now > was * THE_ANKLE_STAYS_JOINED:
        refuse(f"the {side} ankle join's longest edge went {was:.2f} -> {now:.2f} cm, more than "
               f"x{THE_ANKLE_STAYS_JOINED} - the collar has dropped away from the leg and the "
               f"ankle is a funnel")


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

    mine = shoe_vertices(mesh)
    floor = min((mesh.matrix_world @ v.co).z for v in mesh.data.vertices)
    stood = {}
    # Against the LAST'S OWN collar height, not against a share of the shoe's current length.
    # The delivered shoe is both long and tall, so that share already reads 31% before anything
    # has happened - the guard fired on the untouched asset. A ratio is only meaningful once the
    # footprint is the right size, which is after this pass, not before it.
    tallest = max(height for _, height in TOP_OF_THE_SHOE)
    for side in "LR":
        spots = [mesh.matrix_world @ mesh.data.vertices[i].co for i in mine[side]]
        stood[side] = (max(p.z for p in spots) - floor) * SCALE
        if stood[side] <= tallest + 0.5:
            refuse(f"the {side} shoe already stands {stood[side]:.1f} cm, at the last's own "
                   f"{tallest:.1f} cm collar - so this has been run before. Restore the asset "
                   f"from git rather than fitting it to the last twice.")

    joined = {side: the_ankle_join(mesh, named(mesh), side) for side in "LR"}
    belongs = set(mine["L"]) | set(mine["R"])
    inside = [p.index for p in mesh.data.polygons if all(v in belongs for v in p.vertices)]
    before = len(mesh.data.vertices)
    print(f"  {len(belongs)} shoe vertices in {len(inside)} faces, cutting them {cuts}x")
    prepare_rig.subdivide_these(mesh, inside, cuts)
    mine = shoe_vertices(mesh)
    print(f"  the body went from {before} to {len(mesh.data.vertices)} vertices")

    print("  fitting to the last:")
    for side in "LR":
        print(f"    {side} moved at most "
              f"{fit_to_the_last(mesh, mine[side], forward, floor):.2f} cm")
    mesh.data.update()

    who = named(mesh)
    print("  carrying the leg down:")
    for side in "LR":
        spots = [mesh.matrix_world @ mesh.data.vertices[i].co for i in mine[side]]
        dropped = stood[side] - (max(p.z for p in spots) - floor) * SCALE
        moved = carry_the_leg_down(mesh, who, side, dropped, floor)
        print(f"    {side} collar came down {dropped:.2f} cm; {moved} leg vertices followed")
    mesh.data.update()

    print("  cutting the collar:")
    rims = {side: cut_the_collar(mesh, who, side, floor) for side in "LR"}
    mesh.data.update()

    prepare_rig.reshade(mesh, set(mine["L"]) | set(mine["R"]))

    print("  after:")
    for side in "LR":
        looks_like_a_sneaker(mesh, mine[side], forward, floor, side)
        the_collar_has_an_edge(mesh, who, side, floor, rims[side])
        the_ankle_stays_joined(mesh, who, side, joined[side])

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
