"""Gives the character hands that work: five bone chains per hand, weighted to the sculpt.

    blender --background --python add_finger_bones.py -- [--dry-run] [<in.glb> <out.glb>]

The rig arrived with 41 bones and nothing past the wrist, so the hands were a fixed splayed
slab. That is not a tuning problem - it is why weeks of adjusting the palm roll and the wrist
drag never helped, because the splay is in the MESH and no channel existed to pose it. It also
means the character cannot hold, lift, grab, pet or carry, which is most of what the game asks
of him, and it means no NPC can either.

This writes the finger rig ONCE, into the committed source asset, for the reason set out in
bootstrap_rig.sh: sculpting and rigging decisions re-derived on every build are decisions a
classifier has to re-make correctly forever, and this pipeline has already cut a sleeve cuff,
a trouser leg and part of a shoulder that way.

# What is measured rather than assumed

WHICH DIGITS EXIST. find_the_fingers.py separates the hand by watershed on surface distance -
see its notes for the two attempts that failed first. Both hands come out as five, and the
thumb identifies itself: on each hand exactly one digit points across the palm rather than down
it (lateral 0.96 left, -0.96 right, correctly mirrored) and it is the shortest.

WHICH FINGER IS WHICH. The four non-thumb digits are ordered by where they sit across the palm,
and the one nearest the thumb is the index. Nothing is hard-coded per side; the mirrored sign
of the thumb direction is what orients the whole naming, and it is checked.

WHERE EACH DIGIT ENDS AND THE PALM BEGINS. The watershed freezes a branch where it would meet
its neighbour, which is NOT the knuckle - and it froze at radii from 10.1 to 15.6 cm, giving
digit lengths of 2.3 to 8.4 cm on hands that are mirror images. So the branches are used only
as seeds. The split is then a geodesic Voronoi: six sources, the five fingertips and the wrist,
and every vertex goes to whichever is nearest ALONG THE SURFACE. The boundary lands at the
knuckles by itself, and the palm is simply what the wrist won.

HOW THE BONES SIT. Three per digit at 45/30/25 of its length, which is roughly how a real
phalanx divides and what every game rig uses, so a fist has two joints to fold at. Each bone's
roll is aligned to the palm normal, so its local X is the flexion axis and a curl is one
rotation on one axis - the difference between a hand an animator can pose and a puzzle.

# What is guarded

The rest pose must not move. Adding bones and reassigning weights is meant to add
CAPABILITY and change no shape at all, so the deformed mesh is compared vertex by vertex
before and after and the whole thing is refused if anything shifts. That is the guard the
earlier bake step did not have, when it compared its result against its own input and
happily wrote a mesh in one pose bound to a skeleton in another.

Then the format's own limits, which are silent failures otherwise: four bone influences a
vertex, because glTF carries four and drops the rest without a word; weights summing to one;
and the custom split normals surviving, because losing those lights the character as a
different shape and no numeric check on positions ever sees it.
"""
import collections
import heapq
import math
import os
import sys

import bpy
import mathutils

ART = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, ART)

import find_the_fingers as fingers  # noqa: E402
import prepare_rig  # noqa: E402

SCALE = 170.0

# Three bones a digit, at these shares of its length from knuckle to tip. A real proximal
# phalanx is about the same length as the middle and distal together, and every game rig
# divides a finger this way, so a fist has two joints to fold at rather than one.
PHALANX_SHARES = (0.45, 0.30, 0.25)

# Names in the rig's own style - it uses L_/R_ prefixes and CamelCase parts - and in the order
# a hand is read. Which mesh digit gets which name is MEASURED, never assumed by index.
FINGER_NAMES = ("Index", "Middle", "Ring", "Pinky")
THUMB_NAME = "Thumb"

# A digit pointing this much across the palm rather than down it is the thumb. The measured
# gap is wide - 0.96 for the thumb against 0.49 for the next most lateral finger - so this
# sits in the middle of it rather than near either edge.
A_THUMB_POINTS_ASIDE = 0.75

# How far either side of a joint the weights blend, as a share of the shorter bone. Zero
# creases the finger at a single ring of vertices; too much and the fingertip drags the
# knuckle with it. A third is the usual starting point for a smooth-skinned chain.
JOINT_BLENDS = 0.34

REST_MUST_NOT_MOVE = 1e-6      # model units, so a micron at this scale
WEIGHTS_ADD_UP_WITHIN = 0.001
CARRIES_INFLUENCES = 4         # what glTF stores; a fifth vanishes silently


def refuse(why):
    raise SystemExit(f"REFUSED: {why}")


def argv():
    return sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []


# --------------------------------------------------------------------- naming what was found


def spread_along(points, ignoring):
    """The direction a handful of points spread out in most, within the plane across `ignoring`.

    Their principal axis, so it does not matter which point comes first in the list. Doing this
    as first-minus-last is what made the earlier palm plane unstable.
    """
    import numpy

    flat = [p - ignoring * p.dot(ignoring) for p in points]
    cloud = numpy.array([[p.x, p.y, p.z] for p in flat])
    cloud = cloud - cloud.mean(axis=0)
    _u, spread, axes = numpy.linalg.svd(cloud, full_matrices=False)
    straightness = spread[0] / max(spread[1], 1e-12)
    return mathutils.Vector(axes[0]).normalized(), straightness


def hand_frame(digits, spots, across):
    """Names the digits and derives the hand's own axes, from the four fingers.

    Three directions, and each one comes from the fingers rather than from the body:

      DOWN, the way the fingers run - their mean direction.
      THE KNUCKLE LINE, the way their bases are spread out - the principal axis of those four
        points once DOWN is projected out, pointed toward the thumb so that "first" means
        "nearest the thumb" on either hand.
      OUT OF THE PALM, the two crossed.

    Two earlier versions of this got it wrong in ways only a comparison between the hands
    revealed, and both are worth recording because each looked fine on one hand alone.

    Ordering the fingers along the BODY's left-right axis fails because, with the arm hanging
    and the palm turned to the thigh, the knuckles run front-to-back: the projection was near
    zero for all four and their order was noise, so the left hand got a ring finger longer than
    its middle and the right the other way about.

    Taking the palm normal as the thinnest direction of the whole hand's vertex cloud fails
    because a hand with splayed fingers is not a slab - measured, its thinnest direction was
    0.53 of its widest, so "thinnest" was very nearly arbitrary, and the two hands' normals
    mirrored only to 0.348. The four finger bases ARE nearly a line, and that is what to use.
    """
    told = []
    for lump in digits:
        base, tip = spots[lump[0]], spots[lump[-1]]
        line = tip - base
        if line.length < 1e-9:
            refuse("a digit has no length, so its direction cannot be measured")
        told.append({"nodes": lump, "base": base, "tip": tip,
                     "way": line.normalized(), "length": line.length})

    thumbs = [one for one in told if abs(one["way"].dot(across)) >= A_THUMB_POINTS_ASIDE]
    if len(thumbs) != 1:
        refuse(f"{len(thumbs)} digits point across the palm; exactly one should - the thumb")
    thumb = thumbs[0]
    thumb["name"] = THUMB_NAME

    rest = [one for one in told if one is not thumb]
    if len(rest) != len(FINGER_NAMES):
        refuse(f"{len(rest)} fingers beside the thumb; expected {len(FINGER_NAMES)}")

    down = sum((one["way"] for one in rest), mathutils.Vector()).normalized()
    knuckle_line, straightness = spread_along([one["base"] for one in rest], down)
    if straightness < 1.8:
        refuse(f"the four finger bases are not in a line - they spread {straightness:.2f} times "
               f"as far one way as the other - so there is no knuckle line to read")
    if (thumb["base"] - rest[0]["base"]).dot(knuckle_line) < 0.0:
        knuckle_line = -knuckle_line
    rest.sort(key=lambda one: -one["base"].dot(knuckle_line))
    for one, name in zip(rest, FINGER_NAMES):
        one["name"] = name

    out = down.cross(knuckle_line)
    if out.length < 1e-9:
        refuse("the fingers run along the line of their own knuckles; no palm plane")
    return [thumb] + rest, out.normalized(), knuckle_line, straightness


# ------------------------------------------------------------------ splitting hand from palm


def voronoi(spots, touching, sources):
    """Nearest source to every node, measured along the surface.

    Multi-source Dijkstra. The five fingertips compete with the wrist, so the boundary between
    finger and palm is wherever the walk from the wrist stops being the shorter route - which
    puts it at the knuckles without anyone choosing a radius. Replaces using the watershed's
    own freeze points, which sat anywhere from 10.1 to 15.6 cm and gave two mirror-image hands
    digits of quite different lengths.
    """
    mine = {}
    far = {node: math.inf for node in spots}
    queue = []
    for who, seed in sources.items():
        far[seed] = 0.0
        mine[seed] = who
        heapq.heappush(queue, (0.0, seed, who))
    while queue:
        here_far, here, who = heapq.heappop(queue)
        if here_far > far[here] + 1e-12:
            continue
        for there in touching[here]:
            step = here_far + (spots[there] - spots[here]).length
            if step < far[there]:
                far[there] = step
                mine[there] = who
                heapq.heappush(queue, (step, there, who))
    return mine


# ------------------------------------------------------------------------- building the bones


def chain_of(one, out):
    """Where the three bones of a digit start and stop, knuckle to tip."""
    base, tip = one["knuckle"], one["tip"]
    run = tip - base
    joints, at = [base], 0.0
    for share in PHALANX_SHARES:
        at += share
        joints.append(base + run * at)
    return joints


def build(rig, side, named, out):
    """Adds the bones for one hand, and returns what was made."""
    made = []
    with prepare_rig.in_edit_mode(rig) as edit:
        hand = edit.get(f"{side}_Hand")
        if hand is None:
            refuse(f"no {side}_Hand to parent the fingers to")
        inverse = rig.matrix_world.inverted()
        for one in named:
            joints = chain_of(one, out)
            parent = hand
            for number, (head, tail) in enumerate(zip(joints, joints[1:]), start=1):
                name = f"{side}_{one['name']}{number}"
                if name in edit:
                    refuse(f"{name} already exists - the fingers have been built before")
                bone = edit.new(name)
                bone.head = inverse @ head
                bone.tail = inverse @ tail
                bone.parent = parent
                bone.use_connect = number > 1
                bone.use_deform = True
                # Roll so the bone's Z faces out of the back of the hand. That makes local X
                # the flexion axis, so a curl is one rotation on one axis for every finger.
                bone.align_roll(rig.matrix_world.to_3x3().inverted() @ out)
                parent = bone
                made.append(name)
    print(f"  {side}: {len(made)} bones - " + ", ".join(
        f"{one['name']} x{len(PHALANX_SHARES)}" for one in named))
    return made


# ------------------------------------------------------------------------------- the weights


def along_the_chain(spot, joints):
    """Which bone of a chain a point belongs to, and how far through it, 0 to 1.

    Projected onto each segment in turn rather than onto the straight line knuckle-to-tip, so
    a digit that is not dead straight still divides correctly.
    """
    best = (math.inf, 0, 0.0)
    for number, (head, tail) in enumerate(zip(joints, joints[1:])):
        run = tail - head
        if run.length < 1e-12:
            continue
        through = max(0.0, min(1.0, (spot - head).dot(run) / run.length_squared))
        off = (spot - (head + run * through)).length
        if off < best[0]:
            best = (off, number, through)
    return best[1], best[2]


def weigh(mesh, rig, side, named, out, canon_of):
    """Puts every digit vertex on its own bone chain, blended across the joints.

    A vertex belongs to the bone it sits in, and near a joint it is shared with the neighbour
    so the finger bends in a curve rather than creasing at one ring of vertices. Whatever the
    hand held before is replaced, then everything is renormalised - so weights still add to one
    and the vertex is not left half-driven by a palm that no longer owns it.
    """
    groups = {}
    for one in named:
        for number in range(1, len(PHALANX_SHARES) + 1):
            name = f"{side}_{one['name']}{number}"
            groups[name] = mesh.vertex_groups.get(name) or mesh.vertex_groups.new(name=name)
    hand_group = mesh.vertex_groups.get(f"{side}_Hand")

    touched = 0
    for one in named:
        joints = chain_of(one, out)
        lengths = [(b - a).length for a, b in zip(joints, joints[1:])]
        for node in one["own"]:
            for vertex in canon_of[node]:
                spot = mesh.matrix_world @ mesh.data.vertices[vertex].co
                which, through = along_the_chain(spot, joints)
                share = collections.Counter()
                share[which] += 1.0
                # Blend across the joint into the neighbouring bone, by however far into the
                # blend window this vertex sits.
                if through > 1.0 - JOINT_BLENDS and which + 1 < len(lengths):
                    over = (through - (1.0 - JOINT_BLENDS)) / JOINT_BLENDS
                    share[which] -= 0.5 * over
                    share[which + 1] += 0.5 * over
                elif through < JOINT_BLENDS and which > 0:
                    over = (JOINT_BLENDS - through) / JOINT_BLENDS
                    share[which] -= 0.5 * over
                    share[which - 1] += 0.5 * over
                elif through < JOINT_BLENDS and which == 0 and hand_group is not None:
                    # The knuckle end keeps a share of the hand, so the base of a finger is
                    # not a hard seam where the palm stops driving it.
                    over = (JOINT_BLENDS - through) / JOINT_BLENDS
                    share[which] -= 0.5 * over
                    share[-1] += 0.5 * over

                for group in list(mesh.data.vertices[vertex].groups):
                    for name, existing in mesh.vertex_groups.items():
                        if existing.index == group.group:
                            existing.remove([vertex])
                            break
                for which_bone, amount in share.items():
                    if amount <= 0.0:
                        continue
                    if which_bone == -1:
                        hand_group.add([vertex], amount, "REPLACE")
                    else:
                        groups[f"{side}_{one['name']}{which_bone + 1}"].add(
                            [vertex], amount, "REPLACE")
                touched += 1
    return touched


# ------------------------------------------------------------------------------------- guards


def deformed_now(mesh):
    """Every vertex where it actually ends up, armature and all."""
    depsgraph = bpy.context.evaluated_depsgraph_get()
    evaluated = mesh.evaluated_get(depsgraph)
    got = evaluated.to_mesh()
    spots = [mesh.matrix_world @ v.co.copy() for v in got.vertices]
    evaluated.to_mesh_clear()
    return spots


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
    across, forward, up = prepare_rig.body_frame(rig)

    print(f"reading {source}")
    print(f"  {len(rig.data.bones)} bones before")
    was = deformed_now(mesh)

    everything, hands = [], {}
    for side in "LR":
        digits, spots, far = fingers.separate(rig, mesh, side, talk=False)

        mine = fingers.hand_vertices(mesh, side)
        canon = fingers.welded(mesh, mine)
        touching = fingers.graph(mesh, canon)
        nodes = set(canon.values())
        node_at = {c: mesh.matrix_world @ mesh.data.vertices[c].co for c in nodes}

        named, out, knuckle_line, straightness = hand_frame(digits, spots, across)
        # `out` comes from DOWN crossed with a knuckle line already pointed at the thumb, and
        # the thumb is on opposite sides of the two hands - so this is mirror-consistent by
        # construction rather than by a per-side sign anybody had to choose. It is checked
        # against the other hand below, and seen curling in the viewer, because a flexion axis
        # flipped on one hand only would open one fist while closing the other.
        print(f"\n{side} hand: thumb points {named[0]['way'].dot(across):+.2f} across; "
              f"knuckles in line to {straightness:.1f}:1; out of the palm is "
              f"fwd {out.dot(forward):+.2f} lat {out.dot(across):+.2f} up {out.dot(up):+.2f}")

        # The full digits, by geodesic Voronoi against the wrist.
        wrist = rig.matrix_world @ rig.pose.bones[f"{side}_Hand"].head
        sources = {"palm": min(nodes, key=lambda c: (node_at[c] - wrist).length)}
        for one in named:
            sources[one["name"]] = one["nodes"][-1]
        owner = voronoi(node_at, touching, sources)

        canon_of = collections.defaultdict(list)
        for original, c in canon.items():
            canon_of[c].append(original)

        # The knuckle is the SEAM: the middle of the digit's own vertices that touch a vertex
        # the palm won. Taking the point furthest from the tip instead let a single stray node
        # the Voronoi handed over drag the knuckle deep into the hand, which is how a ring
        # finger came out 10.4 cm on one hand and 6.3 cm on its mirror.
        for one in named:
            one["own"] = [c for c in nodes if owner.get(c) == one["name"]]
            if len(one["own"]) < 3:
                refuse(f"{side} {one['name']} won only {len(one['own'])} vertices")
            one["seam"] = [c for c in one["own"]
                           if any(owner.get(n) == "palm" for n in touching[c])]

        # A digit hemmed in by its neighbours touches no palm at all - the middle finger, on
        # both hands. Knuckles sit at much the same distance from the wrist, so the fallback is
        # the others' typical seam distance rather than that digit's own far end, which is a
        # measurement of where the Voronoi boundary happened to land.
        seam_far = sorted(far[c] for one in named for c in one["seam"])
        typical = seam_far[len(seam_far) // 2] if seam_far else 0.0
        for one in named:
            if one["seam"]:
                one["knuckle"] = sum((node_at[c] for c in one["seam"]),
                                     mathutils.Vector()) / len(one["seam"])
                how = ""
            else:
                near = min(one["own"], key=lambda c: abs(far[c] - typical))
                one["knuckle"] = node_at[near]
                how = f"  (hemmed in by its neighbours; knuckle set at the usual depth)"
            print(f"    {one['name']:<7} {len(one['own']):3} nodes, "
                  f"{(one['tip'] - one['knuckle']).length * SCALE:5.1f} cm knuckle to tip{how}")
        hands[side] = {"named": named, "out": out}

        made = build(rig, side, named, out)
        touched = weigh(mesh, rig, side, named, out, canon_of)
        print(f"  {touched} vertices moved onto the finger bones")
        everything.append((side, named, made))

    # The two hands should be a mirrored pair, so comparing them checks the WHOLE derivation
    # at once - the naming, the palm plane and the knuckle seam - against something no step of
    # it was given. This is the check that caught the unstable palm normal and the stray-node
    # knuckle, both of which looked perfectly reasonable on one hand alone.
    #
    # A report and a loose bound rather than a refusal on any difference: the sculpt is
    # generated and its two hands are not held to half a millimetre of each other. What would
    # be a real fault is a digit NAMED differently on the two sides, which shows up here as a
    # length that disagrees wildly with its partner.
    print("\n  the two hands, compared - a mirrored pair should agree:")
    # The flexion reference is a CROSS PRODUCT, and a cross product is a pseudovector: mirror
    # its two inputs and the result mirrors AND flips sign. So on a properly mirrored pair the
    # right hand's reference lands opposite the left's mirror image, not on it - 180 degrees
    # apart, not 0.
    #
    # Checked at 180 for that reason. The first version of this check expected 0, measured
    # 165.2, and refused a derivation that was correct.
    #
    # What that leaves is a TRUE MIRROR, and it is worth being exact about the consequence,
    # because the first version of this note claimed the opposite. Z mirrors and flips, Y just
    # mirrors, so X = Y x Z mirrors cleanly - and a mirrored axis means a curl about it runs the
    # other way round. So the sign that closes a fist is NOT the same on the two hands.
    # Measured on the built rig it is -1 on the left and +1 on the right, which is what
    # animate_ranger.which_way_closes finds by curling the fingers and seeing which way the
    # tips go. No code has to know the sign, which is the point: reasoning about the handedness
    # of a cross product is how the earlier claim here came to be wrong.
    left_out = hands["L"]["out"]
    mirrored = left_out - across * (2.0 * left_out.dot(across))   # across the body's midline
    apart = 180.0 - math.degrees(mirrored.angle(hands["R"]["out"]))
    print(f"    the flexion reference is {abs(apart):.1f} deg off a true mirror "
          f"(0 is perfect; a cross product mirrors to 180, see the note)")
    if abs(apart) > 25.0:
        refuse(f"the two hands' flexion references are {abs(apart):.1f} deg from mirroring, so "
               f"a curl would not match between them")
    worst = 0.0
    for left, right in zip(hands["L"]["named"], hands["R"]["named"]):
        if left["name"] != right["name"]:
            refuse(f"the hands named their digits in different orders: {left['name']} "
                   f"against {right['name']}")
        a = (left["tip"] - left["knuckle"]).length * SCALE
        b = (right["tip"] - right["knuckle"]).length * SCALE
        apart = abs(a - b) / max(a, b)
        worst = max(worst, apart)
        print(f"    {left['name']:<7} {a:5.1f} cm and {b:5.1f} cm, {apart * 100:4.1f}% apart")
    if worst > 0.45:
        refuse(f"a digit differs {worst * 100:.0f}% between the hands, which is a naming or "
               f"knuckle fault rather than an asymmetric sculpt")

    # Four influences and weights that add up, as the format requires.
    bpy.context.view_layer.objects.active = mesh
    bpy.ops.object.vertex_group_limit_total(limit=CARRIES_INFLUENCES)
    bpy.ops.object.vertex_group_normalize_all(lock_active=False)

    print(f"\n  {len(rig.data.bones)} bones after")
    prepare_rig.check_the_skin(mesh)

    # The rest pose must be untouched: this adds capability, not shape.
    now = deformed_now(mesh)
    if len(now) != len(was):
        refuse(f"the vertex count changed, {len(was)} to {len(now)}")
    moved = max((a - b).length for a, b in zip(was, now))
    print(f"  the rest pose moved {moved * SCALE * 10000:.4f} microns at most")
    if moved > REST_MUST_NOT_MOVE:
        refuse(f"the rest pose moved {moved * SCALE:.4f} cm; adding bones must change "
               f"nothing about the shape")

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
