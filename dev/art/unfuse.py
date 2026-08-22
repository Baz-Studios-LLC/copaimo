"""Weight repair: stops one limb driving another limb's cloth.

Its own module because TWO scripts need it in ONE order. `prepare_rig.py` must run it
BEFORE baking the A-pose as the bind pose - baking with a stray arm weight on a hip
pocket froze the pocket mid-drag into the geometry itself, a plate off the left hip in
every frame of every clip. And `animate_ranger.py` used to own it, which put it AFTER
that bake. Moving it is the fix; sharing it is what keeps the two callers agreeing.

Everything below is as it was in animate_ranger.py, where its history is told.
"""

import bpy
import mathutils

# Bone-name fragments that make up each limb chain. Matched by SUBSTRING, and the
# substrings are the whole point: this rig's hand bones are `L_Hand` and `R_Hand`, so
# a filter written as "arm" matches Upperarm, Forearm and Clavicle and MISSES the
# hands — and no bone in this rig has "leg" in its name at all. An earlier repair
# searched for "arm" and "leg", found nothing where the fault actually is, and
# reported that there was nothing to fix. The Twist bones are covered on purpose:
# `*_ThighTwist01` is the bone that carries most of the bleed.
ARM_BONES = ("Clavicle", "Upperarm", "Forearm", "Hand")
LEG_BONES = ("Thigh", "Calf", "Foot", "Toe")

# Vertices this close together are the same point on the surface.
WELD = 1e-5

# A piece of cloth is only a limb's to claim if the two limb chains together drive at
# least this much of it. Measured on this asset: the trousers come out at 0.96 and each
# glove at 1.00, while the jacket-and-sleeves piece is 0.32, because most of the jacket
# is driven by the SPINE. The gap between 0.32 and 0.96 is what this gate sits in.
#
# It exists because the jacket needs excluding and nothing else excludes it. By limb
# weight the jacket looks arm-owned — 1316.7 against 71.6 — and by nearness too, 4158
# vertices to 153, so both of the votes below would happily claim it and strip the
# thigh influence out of its hem.
#
# That influence belongs there: a jacket hem hangs over a thigh and should follow it a
# little. Taking it away is not a catastrophe, and the honest numbers are worth having
# rather than a scary guess — rebuilt with this gate at 0.0, 286 further vertices are
# stripped and the walk barely moves, worst growth 0.0323 either way. But it is worse
# in two ways that were measured and looked at. The RUN gets worse, not better: edges
# over 1.5x go from 26 to 38 and over 2x from 3 to 6. And rendered, the hem stiffens —
# the front corner that swept in a curve becomes an angular faceted flap, because it is
# now rigid against a leg still moving underneath it. Repairing a fault is no licence
# to restyle a garment that was working.
LIMB_PIECE = 0.5


def limb_of(bone: str):
    """Which limb chain a bone drives — or None for the spine, hips, head and root."""
    if any(part in bone for part in ARM_BONES):
        return "arm"
    if any(part in bone for part in LEG_BONES):
        return "leg"
    return None


def cloth_pieces(mesh):
    """Numbers each vertex with the piece of cloth it belongs to.

    A piece is what is connected AFTER welding by position, and welding first is what
    makes the question answerable at all. In the file's own index space this mesh is
    1440 disconnected islands, the largest of them 37 vertices, because the generator
    splits every UV seam — 7578 vertices sit on 2463 distinct positions. So index
    connectivity describes the texture atlas, not the garment, and counting it is how
    an earlier reading arrived at "432 disconnected arm-weighted fragments" and
    concluded the mesh was a soup nothing could reach. Welded, it is 19 clean pieces,
    and the trousers, the left glove and the right glove are three of them.

    The weld decides GROUPING only. No weight is ever averaged across coincident
    vertices, and that restraint matters: 20 coincident groups on this mesh really do
    carry different weights, by as much as 0.84. All 20 are in the jacket piece, which
    this repair leaves alone, but merging them would have been an invention either way.
    """
    spot_of = {}
    for vertex in mesh.vertices:
        spot_of[vertex.index] = (
            round(vertex.co.x / WELD),
            round(vertex.co.y / WELD),
            round(vertex.co.z / WELD),
        )
    numbered = {}
    for spot in spot_of.values():
        numbered.setdefault(spot, len(numbered))

    parent = list(range(len(numbered)))

    def root(x):
        while parent[x] != x:
            parent[x] = parent[parent[x]]
            x = parent[x]
        return x

    for edge in mesh.edges:
        a = root(numbered[spot_of[edge.vertices[0]]])
        b = root(numbered[spot_of[edge.vertices[1]]])
        if a != b:
            parent[a] = b
    return [root(numbered[spot_of[v.index]]) for v in mesh.vertices]


def nearer_chain(rig, ob):
    """Per vertex, which limb chain's bones it physically sits closer to.

    Distance to each bone's SEGMENT, not to its head. Head distance would be wrong
    here for the one bone that matters most: `L_Hand`'s head is at the wrist and its
    tail is out at the fingers, so a fingertip is far from the head of the very bone
    that owns it.
    """
    chains = {"arm": [], "leg": []}
    for bone in rig.data.bones:
        chain = limb_of(bone.name)
        if chain:
            chains[chain].append((bone.head_local.copy(), bone.tail_local.copy()))
    into_rig = rig.matrix_world.inverted() @ ob.matrix_world

    def to_bone(point, head, tail):
        along = tail - head
        span = along.dot(along)
        if span < 1e-12:
            return (point - head).length
        how_far = max(0.0, min(1.0, (point - head).dot(along) / span))
        return (point - (head + along * how_far)).length

    nearer = []
    for vertex in ob.data.vertices:
        point = into_rig @ vertex.co
        arm = min(to_bone(point, head, tail) for head, tail in chains["arm"])
        leg = min(to_bone(point, head, tail) for head, tail in chains["leg"])
        nearer.append("arm" if arm < leg else "leg")
    return nearer


def unfuse_the_gloves_from_the_pockets(rig, ob) -> None:
    """Stops the gloves and the trouser pockets driving each other's surface.

    # A sail of cloth from the fingertips to the pocket

    Move a limb and a flat blade of surface is dragged between the glove and the top
    of the thigh cargo pocket, as if the fingers were stitched to it, with the
    pocket's orange piping smeared out along the blade. Measured on the walk clip by
    posing the real rig and comparing each welded edge with its rest length, the worst
    edge goes 0.0505 -> 0.1389, a growth of 0.0884 on a character that stands one unit
    tall. Eleven edges pass twice their rest length. On the run it is 0.1976 and 55
    edges. It is worst on the RIGHT, at walk frame 1, which is a view no camera preset
    in `gait_look.py` could see until `tqfront_r` was added.

    # What it is not

    Four earlier readings of this fault were wrong, and each wrong reading cost a
    repair. They are written out here because every one of them is a plausible thing
    to conclude again.

    * "The generator outputs one fused skin — the fingertips SHARE geometry with the
      pocket." No. Welded by position, this mesh is 19 separate pieces of cloth. The
      gloves are two of them and the trousers are another, and no edge joins them:
      they approach to 0.0028 on the right and 0.0044 on the left and never touch.
      Nothing is fused, so there is no weld to tear.
    * "The arm-weighted geometry is 432 disconnected pieces, so nothing can reach it."
      That count is index space, where every UV seam is a split. See `cloth_pieces`.
    * "There are no edges joining a hand-dominant vertex to a leg-dominant one, so
      there is nothing to cut." True, and beside the point. A tear does not need
      dominance to flip: a vertex at hand 0.73 beside one at hand 0.11 stretches 2.75x
      with both of them leg-dominant. Dominance was never the quantity that matters.
    * "Reducing the arm swing will settle it." It cannot. The pocket is pulled by
      where the hand HANGS, not by how far it swings, and the fingers are pulled by
      the thigh whatever the arm does.

    # What it is

    Reciprocal cross-limb weight bleed between two pieces of cloth that interpenetrate
    at bind pose. The gloves hang inside the pockets, so the generator's radius-based
    auto-skin reached across that 0.003 gap in BOTH directions:

        trousers     113 verts carry `*_Hand` weight     up to 0.728
        left glove   243 verts carry thigh weight        up to 0.497
        right glove   68 verts carry thigh weight        up to 0.267

    So the pocket rim rides the hand and the fingers ride the thigh, and when the two
    limbs part, the cloth between them has to span the difference.

    # The repair, and why it is a fact rather than a threshold

    No piece of cloth may be driven by two different limbs. A glove is worn on an arm
    and a trouser leg on a leg; that is a statement about the character, not a number
    tuned until a render looked acceptable. So each piece is given to one limb chain
    and every weight it holds on the other chain is deleted, with what remains
    renormalised.

    Ownership is settled per PIECE and never per vertex, and the difference is not
    subtle. The resting hand's bone axis runs right alongside the pocket, so 150 of
    the trousers' 832 vertices are physically nearer an arm bone than a leg bone —
    18 per cent of the garment, including the whole outer face of the right pocket.
    Believe that per vertex and the pocket leaves with the hand and the trouser leg is
    left with a hole in it. Believe it per piece and the trousers are leg cloth, which
    is what they are.

    Two votes have to agree before a piece is touched: total limb weight, and how many
    of its vertices sit nearer each chain. On every piece this claims they agree with
    room to spare — trousers leg by weight 45:1 and by nearness 682:150, the gloves arm
    by weight 13:1 and 51:1 and by nearness 465:6 and 406:0. If a re-export ever makes
    them disagree, the two readings no longer describe the same garment, and guessing
    which is right is worse than leaving the fault in: the piece is skipped and said so.

    That guard is not untested code. Dropping LIMB_PIECE to 0.0 to see what the gate
    was holding back also let the vote reach the eyes and the hair, which carry no limb
    weight at all, and it refused all four of them rather than assigning cloth to a limb
    on a nearness reading with nothing to corroborate it.

    # Why this is a strip and not a re-skin

    Nothing is invented. The generator's own weights are what carry the surface
    afterwards; the repair only deletes the ones that name the wrong limb. That is
    also what keeps it seam-free by construction. Partial repairs measured WORSE than
    no repair at all — one earlier attempt made the worst triangle sixteen times
    worse — because stopping a weight edit part-way through a garment leaves a
    discontinuity in the weights, and a discontinuity in the weights is itself a tear.
    A piece of cloth has no interior boundary to leave one at, and separate pieces
    share no edge to leave one across, so doing this per piece cannot produce one.
    """
    mesh = ob.data
    named = {group.index: group.name for group in ob.vertex_groups}
    groups = ob.vertex_groups

    piece_of = cloth_pieces(mesh)
    nearer = nearer_chain(rig, ob)
    members = {}
    for vertex in mesh.vertices:
        members.setdefault(piece_of[vertex.index], []).append(vertex.index)
    print(f"  {len(mesh.vertices)} verts in {len(members)} piece(s) of cloth")

    stripped = removed = orphaned = 0
    claimed = set()
    for piece, verts in sorted(members.items(), key=lambda kv: -len(kv[1])):
        held = {"arm": 0.0, "leg": 0.0}
        for index in verts:
            for entry in mesh.vertices[index].groups:
                chain = limb_of(named[entry.group])
                if chain:
                    held[chain] += entry.weight
        driven = (held["arm"] + held["leg"]) / len(verts)
        if driven < LIMB_PIECE:
            continue

        by_weight = "arm" if held["arm"] > held["leg"] else "leg"
        votes = {"arm": 0, "leg": 0}
        for index in verts:
            votes[nearer[index]] += 1
        by_nearness = "arm" if votes["arm"] > votes["leg"] else "leg"
        report = (
            f"    piece of {len(verts):4d} verts, limb-driven {driven:.2f}: "
            f"weight {by_weight} ({held['arm']:.1f}/{held['leg']:.1f}), "
            f"nearness {by_nearness} ({votes['arm']}/{votes['leg']})"
        )
        if by_weight != by_nearness:
            print(f"{report} -> DISAGREE, left alone")
            continue

        claimed.add(piece)
        foreign = "leg" if by_weight == "arm" else "arm"
        here = 0
        for index in verts:
            vertex = mesh.vertices[index]
            before = [(entry.group, entry.weight) for entry in vertex.groups]
            after = [
                (group, weight)
                for group, weight in before
                if limb_of(named[group]) != foreign
            ]
            if len(after) == len(before):
                continue
            total = sum(weight for _group, weight in after)
            if total <= 1e-6:
                # The foreign chain was all that held this vertex up. Renormalising
                # would divide by zero and clearing it would drop the vertex at the
                # origin, so it keeps what it has and gets counted out loud.
                orphaned += 1
                continue
            for group, _weight in before:
                groups[group].remove([index])
            for group, weight in after:
                groups[group].add([index], weight / total, "REPLACE")
            removed += len(before) - len(after)
            here += 1
        stripped += here
        print(f"{report} -> {by_weight}, {here} vert(s) had {foreign} weight removed")

    # Proved, not assumed. A silent weight edit is indistinguishable from a no-op, and
    # two of the earlier repairs were reported as no-ops when they were not.
    #
    # Counted separately for the pieces this claimed and the pieces it did not, because
    # the two numbers mean opposite things. In a claimed piece any remaining cross-limb
    # weight is a bug in this function and has to be zero. In an UNclaimed piece it is
    # left there on purpose: the jacket really does blend its hem between the spine and
    # the thigh, and that is a garment hanging over a leg, not a fault.
    left, worst, spared, lightest, heaviest = 0, 0.0, 0, 2.0, -1.0
    for vertex in mesh.vertices:
        total = 0.0
        held = {"arm": 0.0, "leg": 0.0}
        for entry in vertex.groups:
            total += entry.weight
            chain = limb_of(named[entry.group])
            if chain:
                held[chain] += entry.weight
        lightest, heaviest = min(lightest, total), max(heaviest, total)
        if min(held["arm"], held["leg"]) <= 1e-6:
            continue
        if piece_of[vertex.index] in claimed:
            left += 1
            worst = max(worst, min(held["arm"], held["leg"]))
        else:
            spared += 1
    print(
        f"  stripped {removed} weight(s) off {stripped} vertices, {orphaned} orphan(s); "
        f"weight sums {lightest:.6f}..{heaviest:.6f}"
    )
    print(
        f"  cross-limb weight left in the {len(claimed)} claimed piece(s): {left} vertex/vertices "
        f"(worst {worst:.3f}) — must be 0; deliberately left in unclaimed cloth: {spared}"
    )
    if left:
        raise SystemExit("the strip did not take; refusing to export a half-repaired skin")


def spine_frame(rig):
    """Across, forward and up, from every left/right bone pair at once.

    Its own helper because deriving it from one bone makes that bone perfect by
    construction - see prepare_rig's body_frame, which this mirrors deliberately
    rather than importing, since unfuse is called by both scripts.
    """
    spread = mathutils.Vector((0.0, 0.0, 0.0))
    for bone in rig.data.bones:
        partner = f"R_{bone.name[2:]}"
        if bone.name.startswith("L_") and partner in rig.data.bones:
            spread += (rig.matrix_world @ bone.matrix_local.translation) - (
                rig.matrix_world @ rig.data.bones[partner].matrix_local.translation
            )
    spread.z = 0.0
    across = spread.normalized()
    up = mathutils.Vector((0.0, 0.0, 1.0))
    return across, across.cross(up).normalized(), up


def the_spine_owns_the_torso(rig, mesh) -> None:
    """Takes the torso back off the arms, so leaning the spine leans the body.

    # What was wrong

    Measured on the delivered skin, the chest - a band from 110 to 125 cm on a 171 cm
    character, below the shoulders - was driven 61% by the four UPPERARM TWIST bones and
    only 12% by Spine02. The visible torso followed the arms. Two consequences, both
    reported and neither guessable from looking at the bones:

      leaning the spine barely moved the body, because the spine owned an eighth of it,
        so three separate increases of the walk's lean all measured as forward flexion
        and all still read as leaning back;
      and the torso's apparent centreline sat away from the skeleton, which is what
        "the skeleton isn't centred on the mesh" was seeing - the mesh was being pulled
        around by bones that are nowhere near its middle.

    # What this does

    For every vertex, the distance to the ARM chain is compared with the distance to the
    SPINE chain, both as segments. Where the spine is nearer, arm-chain weight is moved
    to the nearest spine bone. Weight moves between groups on the same vertex, so the
    sums stay exactly 1 and nothing is invented.

    The shoulder keeps its blend: up there the arm chain really is nearer, so nothing
    moves. This only reclaims what the arms had taken further in.
    """
    ARMS = ("Clavicle", "Upperarm", "Forearm", "Hand")
    SPINE = ("Waist", "Spine01", "Spine02", "NeckTwist01", "NeckTwist02", "Head")

    names = {g.name: g.index for g in mesh.vertex_groups}
    by_index = {i: n for n, i in names.items()}
    into_rig = rig.matrix_world.inverted() @ mesh.matrix_world

    def segments(of):
        out = []
        for bone in rig.data.bones:
            if any(part in bone.name for part in of):
                out.append((bone.name, bone.head_local.copy(), bone.tail_local.copy()))
        return out

    arm_bones, spine_bones = segments(ARMS), segments(SPINE)

    def how_far(spot, head, tail):
        along = tail - head
        span = along.dot(along)
        if span < 1e-12:
            return (spot - head).length
        share = max(0.0, min(1.0, (spot - head).dot(along) / span))
        return (spot - (head + along * share)).length

    # # The TRUNK CORE by region, not by which bone is nearest
    #
    # Nearest-bone cannot do this job: the upperarm twist bones run down the outside of
    # the ribcage, so for most of the chest an arm bone is genuinely the closest thing
    # there is. Measured, that test reclaimed the chest only from 12% spine to 22%.
    #
    # The rule that does work is anatomical: the arms hang from sockets about 16 cm out
    # from the midline, so geometry well INSIDE that, between the hips and the neck, is
    # trunk whatever happens to be near it. Outside it, the arm keeps everything - the
    # shoulder's blend is untouched.
    across, _, _ = spine_frame(rig)
    midline = 0.0
    pairs = [b.name[2:] for b in rig.data.bones
             if b.name.startswith("L_") and f"R_{b.name[2:]}" in rig.data.bones]
    for part in pairs:
        midline += (
            (rig.matrix_world @ rig.data.bones[f"L_{part}"].matrix_local.translation)
            .dot(across)
            + (rig.matrix_world @ rig.data.bones[f"R_{part}"].matrix_local.translation)
            .dot(across)
        ) / 2.0
    midline /= len(pairs)
    socket_out = abs(
        (rig.matrix_world @ rig.data.bones["L_Upperarm"].matrix_local.translation)
        .dot(across) - midline
    )
    # The boundary is the SHOULDER JOINT plus an arm's thickness, not a fraction of it.
    #
    # Measured, the arm bones driving the chest are genuinely the NEAREST bones to it -
    # median 9.2 cm, against 18.9 cm for the spine - because in an A-pose the arms hang
    # right alongside the ribs. So proximity cannot decide this and never could; the
    # generator weighted by proximity and that is exactly how the chest ended up on the
    # arms. Trunk surface belongs to the spine as a decision, and the line to draw it
    # on is where the arm begins: inboard of the socket is body, outboard is limb. At
    # the chest the torso reaches about 20 cm out and the arms are past 29, so there is
    # real clearance between them.
    core = socket_out + 0.035   # the socket, plus about 6 cm of arm thickness
    low = (rig.matrix_world @ rig.data.bones["Waist"].matrix_local.translation).z
    high = (rig.matrix_world
            @ rig.data.bones["NeckTwist01"].matrix_local.translation).z
    print(f"  the trunk core is within {core * 170.0:.1f} cm of the midline, "
          f"between z {low * 170.0:.0f} and {high * 170.0:.0f} cm")

    moved, touched = 0.0, 0
    for vertex in mesh.data.vertices:
        spot = into_rig @ vertex.co
        world = mesh.matrix_world @ vertex.co
        if abs(world.dot(across) - midline) > core or not low <= world.z <= high:
            continue
        best = min(((how_far(spot, h, t), n) for n, h, t in spine_bones))
        taking = 0.0
        for group in vertex.groups:
            name = by_index.get(group.group, "")
            if any(part in name for part in ARMS) and group.weight > 0.0:
                taking += group.weight
                group.weight = 0.0
        if taking <= 0.0:
            continue
        already = sum(
            g.weight for g in vertex.groups if by_index.get(g.group, "") == best[1]
        )
        mesh.vertex_groups[best[1]].add([vertex.index], already + taking, "REPLACE")
        moved += taking
        touched += 1

    astray = sum(
        1 for v in mesh.data.vertices
        if abs(sum(g.weight for g in v.groups) - 1.0) > 0.01
    )
    print(f"  moved {moved:.1f} weight off the arms onto the spine, across {touched} "
          f"torso vertices; {astray} left with weights not summing to 1")
    if astray:
        raise SystemExit("REFUSED: taking the torso back broke normalisation")

    # Proved, not assumed: the spine must now be the majority owner of the chest.
    band, whole = {}, 0.0
    for vertex in mesh.data.vertices:
        spot = mesh.matrix_world @ vertex.co
        if not 0.647 <= spot.z <= 0.735:   # 110 to 125 cm at this scale
            continue
        for group in vertex.groups:
            if group.weight <= 0.0:
                continue
            name = by_index.get(group.group, "?")
            band[name] = band.get(name, 0.0) + group.weight
            whole += group.weight
    spine_share = sum(
        w for n, w in band.items() if any(part in n for part in SPINE)
    ) / max(1e-9, whole)
    arm_share = sum(
        w for n, w in band.items() if any(part in n for part in ARMS)
    ) / max(1e-9, whole)
    print(f"  the chest is now {spine_share:.0%} spine and {arm_share:.0%} arm "
          f"(it was 12% and 61%)")
    if spine_share < 0.5:
        raise SystemExit(
            f"REFUSED: the spine still only drives {spine_share:.0%} of the chest"
        )
