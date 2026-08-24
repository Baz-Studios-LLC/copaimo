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

import bmesh
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


# How many times to cut the finger faces before weighting them. Three bones a digit needs
# vertices for each to move, and the sculpt arrives with barely one ring per bone - a closed
# fist read as a handful of blocks hinged in three places. Cutting does not change the
# silhouette at all with smoothness at zero; it only gives the skinning something to work with.
CUT_THE_FINGERS = 2


def refuse(why):
    raise SystemExit(f"REFUSED: {why}")


def start_clean(rig, mesh):
    """Takes out any finger rig already present, so this can be run again.

    Without this the second run refuses on `L_Thumb1 already exists`, which makes every
    iteration a hunt through git for the asset as it was before the last one. The weights go
    back onto the hand rather than being dropped - those vertices are all well past the wrist,
    so the hand is where they belong when there is nothing finer to hold them.
    """
    known = [f"{side}_{digit}{number}"
             for side in "LR" for digit in (THUMB_NAME,) + FINGER_NAMES
             for number in range(1, len(PHALANX_SHARES) + 1)]
    present = [name for name in known if name in rig.data.bones]
    if not present:
        return 0

    for side in "LR":
        hand = mesh.vertex_groups.get(f"{side}_Hand")
        if hand is None:
            continue
        mine = [mesh.vertex_groups[n] for n in known
                if n.startswith(f"{side}_") and n in mesh.vertex_groups]
        for vertex in mesh.data.vertices:
            owed = sum(g.weight for g in vertex.groups
                       if any(g.group == one.index for one in mine))
            if owed > 0.0:
                held = sum(g.weight for g in vertex.groups if g.group == hand.index)
                hand.add([vertex.index], min(1.0, held + owed), "REPLACE")
    for name in known:
        group = mesh.vertex_groups.get(name)
        if group is not None:
            mesh.vertex_groups.remove(group)
    with prepare_rig.in_edit_mode(rig) as edit:
        for name in present:
            bone = edit.get(name)
            if bone is not None:
                edit.remove(bone)
    print(f"  removed {len(present)} finger bones from a previous run; their weights went "
          f"back to the hands")
    return len(present)


def add_room_in_the_fingers(mesh, digits_by_side, cuts=CUT_THE_FINGERS):
    """Cuts up the finger faces so three bones a digit have vertices to move.

    The rig works without this - both hands close, measured - but a fist reads as a handful of
    blocks, because a bone with one ring of vertices in it can only swing that ring. Cutting at
    smoothness zero adds density and moves nothing: the original vertices stay exactly where
    they are, which is checked below.

    Only the faces whose every corner belongs to a digit, so the palm and the wrist are left
    alone. And ONLY THIS OBJECT is selected first: `bpy.ops.mesh.*` acts on everything in edit
    mode, and taking that for granted once deleted this entire body, 7264 vertices down to 318.
    """
    wanted = set()
    for side, digits in digits_by_side.items():
        for one in digits:
            wanted |= set(one["all"])
    if not wanted:
        refuse("no finger vertices to cut")

    faces = [p.index for p in mesh.data.polygons if all(v in wanted for v in p.vertices)]
    if not faces:
        refuse("no face has all its corners in a finger, so there is nothing to cut")

    before = len(mesh.data.vertices)
    kept = [mesh.matrix_world @ v.co.copy() for v in mesh.data.vertices]

    bpy.ops.object.mode_set(mode="OBJECT")
    bpy.ops.object.select_all(action="DESELECT")
    bpy.context.view_layer.objects.active = mesh
    mesh.select_set(True)

    # Selected through BMESH, in edit mode, rather than by setting `poly.select` in object
    # mode. Object-mode face selection is DERIVED from the vertex selection, so setting it
    # there does not survive the mode switch - the first version of this did exactly that and
    # subdivided the entire body, 7534 vertices to 34037, while reporting "293 finger faces
    # cut". Nothing in the numbers said fingers; only the total did.
    bpy.ops.object.mode_set(mode="EDIT")
    bpy.ops.mesh.select_mode(type="FACE")
    working = bmesh.from_edit_mesh(mesh.data)
    working.faces.ensure_lookup_table()
    for element in list(working.verts) + list(working.edges) + list(working.faces):
        element.select = False
    for i in faces:
        working.faces[i].select = True
    working.select_flush(True)
    bmesh.update_edit_mesh(mesh.data)
    picked = sum(1 for f in working.faces if f.select)
    if picked != len(faces):
        refuse(f"asked for {len(faces)} faces and the mesh has {picked} selected, so the "
               f"selection did not take")

    bpy.ops.mesh.subdivide(number_cuts=cuts, smoothness=0.0)
    bpy.ops.object.mode_set(mode="OBJECT")

    after = len(mesh.data.vertices)
    print(f"  {len(faces)} finger faces cut {cuts}x: {before} vertices to {after}")
    if after <= before:
        refuse(f"the cut added nothing - {before} vertices before and {after} after - so the "
               f"selection never reached the operator")
    if not mesh.data.has_custom_normals:
        refuse("cutting the fingers lost the custom split normals, which lights the character "
               "as a different shape - see the note on welding in prepare_rig")

    # Linear subdivision leaves every original vertex exactly where it was, so this is an
    # absolute check and not a comparison against its own input: every position the mesh had
    # before must still be a position the mesh has.
    now = {(round(p.x, 6), round(p.y, 6), round(p.z, 6))
           for p in (mesh.matrix_world @ v.co for v in mesh.data.vertices)}
    lost = [p for p in kept if (round(p.x, 6), round(p.y, 6), round(p.z, 6)) not in now]
    if lost:
        refuse(f"{len(lost)} vertices moved while being cut; the shape must not change")
    return after - before


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


def hand_frame(digits, spots, across, wrist, known=None):
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

    # # Which one is the thumb
    #
    # Established ONCE, on the mesh as delivered, and then carried by position - never worked
    # out again on a mesh that has been cut up. That is the whole lesson of this function, and
    # it cost four discriminators to learn, so they are all recorded: each was argued from
    # numbers, each looked clean somewhere, and every one of them was wrong.
    #
    #   THE DIGIT POINTING ACROSS THE PALM. Correct on both hands as delivered, and by a wide
    #   margin - 0.96 against 0.49 for the next. It is the test kept below. It broke only when
    #   the finger faces were subdivided, because a watershed branch's direction depends on
    #   where the branch stopped, and that moves with the density: the left thumb went from
    #   0.96 to 0.41 and no digit was lateral enough to be one.
    #
    #   THE DIGIT REACHING LEAST FAR. Wrong. The pinky is shorter - 12.2 cm against the thumb's
    #   14.0 - so this names the pinky.
    #
    #   THE DIGIT WHOSE KNUCKLE IS NEAREST THE WRIST. Wrong, and for the same reason: the
    #   pinky's knuckle is shallower, 9.5 cm against 11.0.
    #
    #   THE DIGIT MOST OPPOSED TO THE OTHERS. Wrong again - the pinky splays further, 25.2
    #   degrees against 23.3.
    #
    #   THE DIGIT WHOSE REMOVAL LEAVES THE OTHERS IN A LINE. Wrong, and the most plausible of
    #   the lot. Dropping the LONGEST finger from a fan straightens what remains more than
    #   dropping the thumb does: 14.69:1 without the middle finger against 5.56:1 without the
    #   thumb.
    #
    # A thumb is not the short one, the splayed one, or the odd one out - a pinky is all three.
    # Rendering the five digits in five colours is what settled it, after all four arguments;
    # looking took one render.
    if known:
        for one in told:
            one["name"] = min(known, key=lambda k: (k[0] - one["tip"]).length)[1]
        given = sorted(one["name"] for one in told)
        if given != sorted((THUMB_NAME,) + FINGER_NAMES):
            refuse(f"carrying the names over by position gave {given}, so two digits claimed "
                   f"the same name and the tips did not match up")
        thumb = next(one for one in told if one["name"] == THUMB_NAME)
    else:
        thumbs = [one for one in told if abs(one["way"].dot(across)) >= A_THUMB_POINTS_ASIDE]
        if len(thumbs) != 1:
            refuse(f"{len(thumbs)} digits point across the palm; exactly one should - "
                   f"the thumb")
        thumb = thumbs[0]
        thumb["name"] = THUMB_NAME

    rest = [one for one in told if one is not thumb]
    if len(rest) != len(FINGER_NAMES):
        refuse(f"{len(rest)} fingers beside the thumb; expected {len(FINGER_NAMES)}")

    # One path from here, whether the names were just worked out or carried in. The two used to
    # derive the frame separately and they disagreed: `out` came out negated and the right
    # hand's index and pinky swapped, which the mirror check caught at 173.7 degrees.
    down = sum((one["way"] for one in rest), mathutils.Vector()).normalized()
    # From the TIPS, not the branch bases: a tip is where the sculpt ends and does not move when
    # the mesh is cut up, where a branch base is wherever the watershed happened to stop.
    knuckle_line, straightness = spread_along([one["tip"] for one in rest], down)
    if straightness < 1.8:
        refuse(f"the four fingertips are not spread along a line - {straightness:.2f} times as "
               f"far one way as the other - so there is no knuckle line to read")

    # Pointed at the thumb from the MIDDLE of the four fingers. Pointing it from `rest[0]`
    # instead made the whole frame depend on which finger happened to be first in a list, and
    # that order is not fixed - it flipped `out` between two runs of the same code.
    middle = sum((one["tip"] for one in rest), mathutils.Vector()) / len(rest)
    if (thumb["tip"] - middle).dot(knuckle_line) < 0.0:
        knuckle_line = -knuckle_line
    rest.sort(key=lambda one: -one["tip"].dot(knuckle_line))

    if known:
        # The carried names must still run in the order the hand does. If they do not, the tips
        # were matched to the wrong digits and everything downstream is mislabelled.
        carried = [one["name"] for one in rest]
        if carried != list(FINGER_NAMES):
            refuse(f"the carried names run {carried} across the hand, not {list(FINGER_NAMES)}, "
                   f"so a tip was matched to the wrong digit")
    else:
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


def discover(rig, mesh, side, across, forward, up, known=None):
    """Everything about one hand that the bones and weights are built from.

    Its own function because it runs TWICE: once on the mesh as delivered, to learn
    which faces belong to fingers and so which to cut, and again afterwards on the
    denser mesh, where every index has changed and the digits have to be found afresh.
    """
    digits, spots, far = fingers.separate(rig, mesh, side, talk=False)

    mine = fingers.hand_vertices(mesh, side)
    canon = fingers.welded(mesh, mine)
    touching = fingers.graph(mesh, canon)
    nodes = set(canon.values())
    node_at = {c: mesh.matrix_world @ mesh.data.vertices[c].co for c in nodes}
    wrist = rig.matrix_world @ rig.pose.bones[f"{side}_Hand"].head

    named, out, knuckle_line, straightness = hand_frame(
        digits, spots, across, wrist, known)
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
        # Every ORIGINAL vertex of the digit, split copies and all, which is what the cutting
        # step selects faces from and what the weighting writes to.
        one["all"] = [v for c in one["own"] for v in canon_of[c]]
        print(f"    {one['name']:<7} {len(one['own']):3} nodes, {len(one['all']):3} vertices, "
              f"{(one['tip'] - one['knuckle']).length * SCALE:5.1f} cm knuckle to tip{how}")
    return named, out, canon_of


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
    start_clean(rig, mesh)

    # Find the digits on the mesh as it stands, to learn which faces are finger, then cut those
    # and find them again. Two passes rather than one because cutting renumbers every vertex -
    # the same trap that made deleting faces reshuffle the indices a later step was holding.
    print("\n  finding the fingers, to know what to cut:")
    found = {side: discover(rig, mesh, side, across, forward, up)[0] for side in "LR"}
    # The NAMES are settled here, on the mesh as delivered, and carried through the cut by tip
    # position. Tips do not move when the mesh is subdivided - measured, 14.0 cm before and
    # 14.0 cm after - whereas every test that re-derives which digit is the thumb from the cut
    # mesh has been wrong. See hand_frame for the four of them.
    named_by_tip = {side: [(one["tip"], one["name"]) for one in digits]
                    for side, digits in found.items()}
    add_room_in_the_fingers(mesh, found)

    # AFTER the cut, because the cut deliberately changes the vertex count and this guard
    # compares vertex against vertex. The cut has its own absolute check: every position the
    # mesh had before is still a position it has.
    was = deformed_now(mesh)

    everything, hands = [], {}
    for side in "LR":
        named, out, canon_of = discover(rig, mesh, side, across, forward, up,
                                        known=named_by_tip[side])
        made = build(rig, side, named, out)
        touched = weigh(mesh, rig, side, named, out, canon_of)
        print(f"  {touched} vertices moved onto the finger bones")
        everything.append((side, named, made))
        hands[side] = {"named": named, "out": out}

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
