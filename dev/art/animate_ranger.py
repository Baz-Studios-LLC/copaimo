"""Adds walk and run clips to the made ranger, and writes the game's copy of it.

    dev/art/animate_ranger.sh

Reads `Ranger_Rig_Idle.glb` from the repository root — the file as it arrived, kept
untouched — and writes `assets/models/person_ranger.glb` with its idle plus a walk
and a run. Re-runnable: the source is never modified, so this can be thrown away and
done again, and a new export from the generator drops straight in.

# Rotating a bone you did not build

The rig came from a generator, so nothing about its rest orientations is known: a
bone's local X might point along the limb, across it, or anywhere. Guessing an axis
and looking at the result is how the scripted figure's knees ended up bending
backwards like a bird's.

So nothing here guesses. Every pose is stated in the ARMATURE's own axes — swing
this leg forward, lift these hips — and converted into whatever local axis that
happens to be for the bone in question:

    axis_in_bone = bone.matrix_local.to_3x3().inverted() @ world_axis

That is exact for any rig, and it means this same script would work on a different
skeleton with different rest poses.

# Which way is forward

The model faces +X. Up is +Z (Blender). So a limb swinging forward turns about the
armature's **Y** axis, and the body bobs along **Z**. A positive swing is forward,
which makes a knee's flexion negative and an elbow's positive — the same anatomy the
scripted figure needed, just expressed in axes that hold for any rig.
"""

import math
import os
import sys

import bpy
import mathutils

# How far a thigh reaches at full stride, in degrees, and how far an arm answers it.
WALK_STRIDE = 26.0
WALK_ARM = 6.0
# 28, not 42. Rendered and looked at: 42 degrees each way is 84 between the legs,
# and with the knees straight it read as the splits — mid-air, no foot ever planted.
# A stylised run wants a modest stride and a lot of KNEE.
RUN_STRIDE = 28.0
RUN_ARM = 10.0

# How much a knee bends as the leg passes under the body.
WALK_KNEE = 38.0
RUN_KNEE = 62.0

# How far the hips rise and fall, in the model's own units — it stands one unit
# tall, so this is a share of its height rather than metres.
WALK_BOB = 0.014
RUN_BOB = 0.020

# How far the body leans into a run, and how much the spine counter-twists.
RUN_LEAN = 6.0
SPINE_TWIST = 4.0

# The arms hang a little OUT and the palms turn IN, in every frame, so the hands
# clear the pockets. The generator's bind pose parks them ON the pockets — glove and
# trouser come within 0.003 of touching — so a few degrees of abduction is what keeps
# the fingers out of the cloth.
#
# This pair used to be a workaround as well, and `WALK_ARM` used to be 6 rather than
# 20 to hide a mesh fault under a swing too small to tear it. That fault is repaired
# at its cause now — see `unfuse_the_gloves_from_the_pockets` — so the swing
# amplitudes are back to being nothing but a gait choice. They are left where they
# are: how far a walk swings its arms is a question about how the walk should LOOK,
# and answering it is not this repair's business. A wider swing is available to
# whoever wants to look at one; the repair was measured at twenty as well as at six.
ARM_OUT = 5.0
PALM_IN = 10.0


def swing(rig, bone: str, degrees: float, axis=(0.0, 1.0, 0.0)):
    """Turns a bone about an axis of the ARMATURE, whatever its own rest pose is."""
    posed = rig.pose.bones.get(bone)
    if posed is None:
        return
    rest = posed.bone.matrix_local.to_3x3()
    local = rest.inverted() @ mathutils.Vector(axis)
    posed.rotation_mode = "QUATERNION"
    posed.rotation_quaternion = mathutils.Quaternion(
        local.normalized(), math.radians(degrees)
    )


def shift(rig, bone: str, along: float, axis=(0.0, 0.0, 1.0)):
    """Moves a bone along an axis of the armature — used for the bob."""
    posed = rig.pose.bones.get(bone)
    if posed is None:
        return
    rest = posed.bone.matrix_local.to_3x3()
    posed.location = rest.inverted() @ (mathutils.Vector(axis) * along)


def rest(rig) -> None:
    """Puts every bone back to its rest pose.

    # Keys were baking in the idle's pose

    Nothing did this, and it showed. The clip that came with the rig leaves the
    armature posed, and a bone that is keyframed WITHOUT being set first records
    whatever it happened to be holding — so the torso came out pitched back and the
    arms hung across the body, and none of it was in the keyframes I wrote.

    Every bone, not just the driven ones: a bone left posed by the idle and never
    keyed here holds that pose for the whole clip.
    """
    for posed in rig.pose.bones:
        posed.rotation_mode = "QUATERNION"
        posed.rotation_quaternion = (1.0, 0.0, 0.0, 0.0)
        posed.location = (0.0, 0.0, 0.0)
        posed.scale = (1.0, 1.0, 1.0)


def key(rig, frame: int, bones):
    """Writes the current pose of EVERY bone at this frame.

    # Un-keyed bones freeze at whatever the last clip left

    Only the driven bones were keyed at first, and the clips looked right in Blender
    and wrong in the game. In Blender the test started from rest, so the other
    twenty-seven bones sat at rest. In the game an animation player only moves the
    bones a clip has curves for — everything else HOLDS ITS LAST POSE, which after
    blending from the idle is some frame of the idle. The result was a chimera:
    walking legs under an idle's pelvis, twists and toes, which read as knees
    bending the wrong way.

    The `bones` argument is gone: it existed to say which bones to key, and keying
    a subset is exactly the bug this comment describes.
    """
    _ = bones
    for posed in rig.pose.bones:
        posed.keyframe_insert("rotation_quaternion", frame=frame)
        if posed.name in ("Hip", "Root"):
            posed.keyframe_insert("location", frame=frame)


# Everything a stride touches. Twist bones are left alone: they exist to spread a
# limb's roll and have no business being posed by hand.
DRIVEN = (
    "Hip",
    "Waist",
    "Spine01",
    "Spine02",
    "L_Thigh",
    "R_Thigh",
    "L_Calf",
    "R_Calf",
    "L_Foot",
    "R_Foot",
    "L_Upperarm",
    "R_Upperarm",
    "L_Forearm",
    "R_Forearm",
)


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


def gait(rig, name: str, stride: float, arm: float, knee: float, bob: float, lean: float, span: int):
    """One cycle: contact, pass, contact, pass, contact.

    A gait is four poses and the fifth is the first again, so it loops. What makes
    it read as walking rather than as legs waving is the BOB — the body falls onto
    each foot and rises over it, twice per cycle — and the arms answering the
    opposite leg.
    """
    action = bpy.data.actions.new(name)
    rig.animation_data.action = action
    rest(rig)
    quarter = span // 4

    for step in range(5):
        frame = 1 + step * quarter
        phase = step % 4
        # Which leg is forward: +1 means the left.
        lead = 1.0 if phase == 0 else (-1.0 if phase == 2 else 0.0)
        passing = phase in (1, 3)
        # On a pass, the swinging leg is the one that was behind.
        swinging = 1.0 if phase == 1 else -1.0

        for side, hand in (("L", 1.0), ("R", -1.0)):
            if passing:
                forward = swinging * hand
                swing(rig, f"{side}_Thigh", (10.0 if forward > 0 else -14.0))
                swing(rig, f"{side}_Calf", -knee if forward > 0 else -6.0)
                swing(rig, f"{side}_Foot", 12.0 if forward > 0 else -6.0)
                swing(rig, f"{side}_Upperarm", 0.0)
            else:
                forward = lead * hand
                swing(rig, f"{side}_Thigh", stride * forward)
                # The reaching leg is nearly straight; the trailing one keeps a bend
                # because it is pushing off.
                swing(rig, f"{side}_Calf", -8.0 if forward > 0 else -26.0)
                swing(rig, f"{side}_Foot", -10.0 if forward > 0 else 16.0)
                # Arms answer the OPPOSITE leg.
                swing(rig, f"{side}_Upperarm", -arm * forward)
            # A held bend, so the arms are not two planks.
            swing(rig, f"{side}_Forearm", 18.0 if name == "walk" else 62.0)

        # The arms out a little and the palms in, every frame — see ARM_OUT.
        # `shoulder`, NOT `arm`: `arm` is this function's swing amount, and naming
        # the bone the same thing handed a PoseBone to a subtraction.
        for side, hand in (("L", 1.0), ("R", -1.0)):
            shoulder = rig.pose.bones.get(f"{side}_Upperarm")
            if shoulder is not None:
                # Composed ON TOP of the swing set above: abduction about the
                # forward axis, so the arm hangs clear of the pocket.
                rest_axes = shoulder.bone.matrix_local.to_3x3().inverted()
                out_axis = (rest_axes @ mathutils.Vector((1.0, 0.0, 0.0))).normalized()
                shoulder.rotation_quaternion = (
                    mathutils.Quaternion(out_axis, math.radians(ARM_OUT * hand))
                    @ shoulder.rotation_quaternion
                )
            swing(rig, f"{side}_Hand", PALM_IN * hand, axis=(0.0, 0.0, 1.0))

        # The spine counter-twists against the hips, and leans into a run. The
        # WAIST is set too — it was in the keyed list and never posed, so every
        # frame recorded whatever the idle had left in it.
        swing(rig, "Waist", lean * 0.3, axis=(0.0, 1.0, 0.0))
        swing(rig, "Spine01", lean * 0.4, axis=(0.0, 1.0, 0.0))
        swing(rig, "Spine02", SPINE_TWIST * (lead if not passing else 0.0), axis=(0.0, 0.0, 1.0))
        # Lowest at each contact, highest over the standing leg.
        shift(rig, "Hip", 0.0 if not passing else bob)
        key(rig, frame, DRIVEN)

    # Nothing is done about interpolation here. Bezier is Blender's own default for
    # a new key, and reaching for `action.fcurves` to set it does not work on 5.x
    # anyway — actions are LAYERED now, and the curves live under
    # `action.layers[].strips[].channelbag(slot)`. Easing a gait by hand would be
    # the wrong move regardless: a walk wants to slow into each contact, which is
    # what bezier already does.
    return action


def use_the_calmed_texture() -> None:
    """Points the material at the base-colour map with the eye whites brought down.

    The pixel work is `dev/art/ranger_texture.py`, deliberately not here. Doing it in
    Blender — editing `image.pixels` and calling `pack()` — reported success and
    exported the ORIGINAL bytes: peak luminance through the eye read 254 before and
    254 after. Blender keeps the packed file it already has. So the texture is
    prepared outside, where the result can be measured, and this step has one job it
    cannot get wrong.
    """
    here = os.path.dirname(os.path.abspath(__file__))
    calmed = os.path.join(here, "ranger_basecolor.png")
    if not os.path.isfile(calmed):
        print(f"no {os.path.basename(calmed)}; the eyes keep their glare")
        return

    replacement = bpy.data.images.load(calmed, check_existing=True)
    swapped = 0
    for material in bpy.data.materials:
        if not material.use_nodes:
            continue
        for link in material.node_tree.links:
            if link.to_socket.name != "Base Color":
                continue
            node = link.from_node
            if node.type == "TEX_IMAGE":
                node.image = replacement
                swapped += 1
    print(f"USING {os.path.basename(calmed)} on {swapped} material(s)")


def main() -> None:
    root = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    source = os.path.join(root, "Ranger_Rig_Idle.glb")
    out = os.path.join(root, "assets", "models", "person_ranger.glb")
    if not os.path.isfile(source):
        raise SystemExit(f"the source is not there: {source}")

    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=source)

    rig = next((o for o in bpy.data.objects if o.type == "ARMATURE"), None)
    if rig is None:
        raise SystemExit("no armature in the source")
    print(f"rig '{rig.name}' with {len(rig.data.bones)} bones")
    missing = [b for b in DRIVEN if b not in rig.pose.bones]
    if missing:
        raise SystemExit(f"the rig has no {missing} — the gait cannot be written")

    use_the_calmed_texture()

    # The skinned body, and not the stray unskinned `Icosphere` of radius 1 that the
    # generator leaves at the origin.
    body = max(
        (o for o in bpy.data.objects if o.type == "MESH" and o.vertex_groups),
        key=lambda o: len(o.data.vertices),
        default=None,
    )
    if body is None:
        raise SystemExit("no skinned mesh in the source")
    print(f"unfusing '{body.name}'")
    unfuse_the_gloves_from_the_pockets(rig, body)

    idle = rig.animation_data.action if rig.animation_data else None
    if idle:
        print(f"keeping '{idle.name}'")
        idle.use_fake_user = True

    # Twenty-four frames a cycle for a walk, sixteen for a run: a run is the same
    # shape at a quicker cadence, and reads wrong if it is only bigger.
    gait(rig, "walk", WALK_STRIDE, WALK_ARM, WALK_KNEE, WALK_BOB, 0.0, 24).use_fake_user = True
    gait(rig, "run", RUN_STRIDE, RUN_ARM, RUN_KNEE, RUN_BOB, RUN_LEAN, 16).use_fake_user = True

    bpy.ops.export_scene.gltf(
        filepath=out,
        export_format="GLB",
        export_yup=True,
        # Every action as its own clip, rather than only whatever is active.
        export_animation_mode="ACTIONS",
        export_animations=True,
        export_skins=True,
        export_morph=False,
        export_apply=False,
    )
    print(f"WROTE {out}")
    print("clips:", [a.name for a in bpy.data.actions])


main()
