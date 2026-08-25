"""Everything measurable about the character, in one run.

    dev/art/audit_character.sh
    blender --background --python audit_character.py -- [--model PATH]

Stage 00 of `docs/character-pipeline.md`: the instrument, before the subject. Every number this
prints was worked out ad hoc during the session that replaced the character, in scripts that
lived in a temp directory and are gone. A measurement you cannot repeat is not a measurement.

What it answers, in the order the answers matter:

  THE SURFACE    welded shells, holes, non-manifold edges, and the long edges that mean the
                 generator bridged two limbs that sat close together
  THE SKELETON   what is there against what a game biped needs, and what is missing
  THE SKIN       influences a vertex and whether the weights add up
  THE CLIPS      duration, travel, and whether each one returns to its opening POSE

Read-only. It changes nothing and writes nothing, so it can be run on anything at any time.

# Two things it does that the obvious version would not

It WELDS BY POSITION before asking any question about topology. glTF stores one set of
attributes per vertex, so every UV seam and hard edge duplicates the vertices along it - this
character's 7844 stored vertices are 2464 real ones. `is_boundary` on the unwelded mesh calls
every edge a boundary and answers no question at all, which once produced a confident, wrong
claim that a shoe could not be lowered without leaving the ankle in mid-air.

And it measures a POSE as local bone rotation rather than as world position. A clip with root
motion never repeats in world space, and `Root` - which does not travel - appears to swing the
whole distance relative to a hip that does.
"""
import collections
import math
import os
import sys

import bpy
import mathutils

ART = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, ART)

# The model is authored a unit high and `look::TALL` scales it, so a unit is 1.70 m in the game
# and everything here is reported in centimetres of the finished character.
SCALE = 170.0

# Coincident copies weld together at this grain; genuine neighbours never do. 3.4 microns at
# model scale.
WELD_WITHIN = 0.00002

# An edge this many times the median MIGHT be a bridge - but only if it also spans two body
# regions. On its own the test is wrong, and the way it was wrong is worth keeping.
#
# It counted 182 "bridges" on this character. Rendered with them picked out in red, they are the
# chest panel, the crotch and the shoulder caps: ordinary large polygons on a body that welds to
# 2464 vertices. A low-poly mesh has long edges everywhere and none of them is a defect.
#
# A bridge is a long edge that joins an ARM to a TRUNK, or a leg to a trunk - two surfaces that
# come close in the rest pose and separate when the character moves. There are nine of those,
# not 182, and the difference between the two numbers is the difference between a morning's work
# and a week of it.
A_BRIDGE_IS = 4.0

# What a game biped is expected to carry, by name. Missing entries are reported rather than
# refused - a character mid-pipeline is allowed to be incomplete, and the point is to say so.
WANTED = {
    "spine chain": ["Waist", "Spine01", "Spine02"],
    "neck and head": ["Head"],
    "clavicles": ["L_Clavicle", "R_Clavicle"],
    "arms": [f"{s}_{p}" for s in "LR" for p in ("Upperarm", "Forearm", "Hand")],
    "legs": [f"{s}_{p}" for s in "LR" for p in ("Thigh", "Calf", "Foot", "ToeBase")],
    "fingers": [f"{s}_{d}{n}" for s in "LR"
                for d in ("Thumb", "Index", "Middle", "Ring", "Pinky") for n in (1, 2, 3)],
}


def argv():
    return sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []


def the_body(objects):
    """The skinned mesh with the most vertices - the character rather than a prop or a widget."""
    skinned = [o for o in objects if o.type == "MESH" and o.vertex_groups]
    if not skinned:
        raise SystemExit("REFUSED: nothing in this file is a skinned mesh")
    return max(skinned, key=lambda o: len(o.data.vertices))


def welded(mesh):
    """Maps every vertex to one canonical index per POSITION. See the note in the docstring."""
    canon, seen = {}, {}
    for vertex in mesh.data.vertices:
        co = vertex.co
        key = (round(co.x / WELD_WITHIN), round(co.y / WELD_WITHIN), round(co.z / WELD_WITHIN))
        canon[vertex.index] = seen.setdefault(key, vertex.index)
    return canon


def play(rig, clip):
    """Assigns a clip so it actually drives the rig.

    From Blender 4.4 an action holds SLOTS, and until one is bound the action is attached and
    inert - it reports success and moves nothing. That is how a walk once measured as travelling
    0.0 cm, with its feet never leaving the ground.
    """
    if rig.animation_data is None:
        rig.animation_data_create()
    rig.animation_data.action = clip
    slots = getattr(clip, "slots", None)
    if slots:
        rig.animation_data.action_slot = slots[0]


def apart(first, other):
    """The largest angle between two poses, in degrees, by the SHORTEST arc.

    `Quaternion.rotation_difference(...).angle` can hand back more than 180 degrees, because a
    rotation and its negation are the same rotation and the difference may take the long way
    round. Measuring two standing poses that way reported 349.71 degrees between them when they
    are 10.29 apart - and 349 reads as "these are nothing like each other", which would have
    sent the clip join off after a problem that was not there.
    """
    worst = 0.0
    for a, b in zip(first, other):
        turn = math.degrees(a.rotation_difference(b).angle)
        worst = max(worst, min(turn, 360.0 - turn))
    return worst


def region_of(name):
    """Which part of the body a bone belongs to, for telling a bridge from a big face."""
    for key, part in (("Thigh", "leg"), ("Calf", "leg"), ("Foot", "leg"), ("Toe", "leg"),
                      ("Upperarm", "arm"), ("Forearm", "arm"), ("Hand", "arm"),
                      ("Clavicle", "trunk"), ("Spine", "trunk"), ("Waist", "trunk"),
                      ("Hip", "trunk"), ("Pelvis", "trunk"),
                      ("Neck", "head"), ("Head", "head")):
        if key in name:
            return part
    return None


def the_surface(mesh, owner):
    canon = welded(mesh)
    nodes = set(canon.values())
    touching = collections.defaultdict(set)
    for edge in mesh.data.edges:
        a, b = canon[edge.vertices[0]], canon[edge.vertices[1]]
        if a != b:
            touching[a].add(b)
            touching[b].add(a)

    seen, shells = set(), []
    for start in nodes:
        if start in seen:
            continue
        stack, group = [start], []
        seen.add(start)
        while stack:
            here = stack.pop()
            group.append(here)
            for other in touching[here]:
                if other not in seen:
                    seen.add(other)
                    stack.append(other)
        shells.append(len(group))

    faces_on = collections.Counter()
    for poly in mesh.data.polygons:
        corners = list(poly.vertices)
        for a, b in zip(corners, corners[1:] + corners[:1]):
            ca, cb = canon[a], canon[b]
            if ca != cb:
                faces_on[(min(ca, cb), max(ca, cb))] += 1
    holes = sum(1 for n in faces_on.values() if n == 1)
    odd = sum(1 for n in faces_on.values() if n > 2)

    lengths = sorted(
        ((mesh.matrix_world @ mesh.data.vertices[e.vertices[0]].co)
         - (mesh.matrix_world @ mesh.data.vertices[e.vertices[1]].co)).length * SCALE
        for e in mesh.data.edges
    )
    median = lengths[len(lengths) // 2]
    long_ones, bridges = 0, []
    for edge in mesh.data.edges:
        a, b = edge.vertices
        span = ((mesh.matrix_world @ mesh.data.vertices[a].co)
                - (mesh.matrix_world @ mesh.data.vertices[b].co)).length * SCALE
        if span <= median * A_BRIDGE_IS:
            continue
        long_ones += 1
        parts = {region_of(owner.get(a, "")), region_of(owner.get(b, ""))} - {None}
        if len(parts) > 1:
            bridges.append((span, owner.get(a, "?"), owner.get(b, "?")))
    bridges.sort(reverse=True)

    print("THE SURFACE")
    print(f"  {len(mesh.data.vertices)} stored vertices weld to {len(nodes)} real ones, "
          f"{len(mesh.data.polygons)} faces")
    print(f"  {len(shells)} shells, largest {sorted(shells, reverse=True)[:6]}")
    print(f"  {holes} open edges (holes), {odd} edges with more than two faces")
    print(f"  edge length: median {median:.2f} cm, longest {lengths[-1]:.2f} cm; "
          f"{long_ones} past {A_BRIDGE_IS:.0f}x the median")
    print(f"  of those, {len(bridges)} BRIDGE two body regions - the rest are just a coarse mesh")
    for span, a, b in bridges[:5]:
        print(f"    {span:6.2f} cm  {a} <-> {b}")
    if bridges:
        print("    ^ two surfaces that sit close at rest and separate when he moves. Those")
        print("      stretch, and that is what tearing is.")
    print(f"  custom split normals: {mesh.data.has_custom_normals}; "
          f"UV layers {[l.name for l in mesh.data.uv_layers]}")
    print(f"  colour attributes: {[l.name for l in mesh.data.color_attributes]}")


def owner_of(mesh):
    """Which bone drives each vertex most, by name."""
    groups = {g.index: g.name for g in mesh.vertex_groups}
    out = {}
    for vertex in mesh.data.vertices:
        best, who = 0.0, ""
        for group in vertex.groups:
            if group.weight > best:
                best, who = group.weight, groups.get(group.group, "")
        out[vertex.index] = who
    return out


def the_skeleton(rig):
    have = {bone.name for bone in rig.data.bones}
    twists = sorted(n for n in have if "Twist" in n)
    print("\nTHE SKELETON")
    print(f"  {len(have)} bones, {len(twists)} of them twist bones")
    for what, need in WANTED.items():
        missing = [n for n in need if n not in have]
        mark = "" if not missing else "   <- MISSING"
        print(f"  {what:<14s} {len(need) - len(missing):>2}/{len(need):<2} present{mark}")
        if missing and len(missing) <= 8:
            print(f"    {', '.join(missing)}")
    loose = sorted(n for n in have
                   if not any(n in need for need in WANTED.values()) and "Twist" not in n)
    if loose:
        print(f"  not on the list: {', '.join(loose)}")


def the_skin(mesh):
    counts = [sum(1 for g in v.groups if g.weight > 0.001) for v in mesh.data.vertices]
    sums = [sum(g.weight for g in v.groups) for v in mesh.data.vertices]
    print("\nTHE SKIN")
    print(f"  at most {max(counts)} bones drive a vertex "
          f"({'within' if max(counts) <= 4 else 'PAST'} the four glTF carries)")
    print(f"  weights add to 1 within {max(abs(s - 1.0) for s in sums):.6f}")
    if max(counts) > 4:
        print("    ^ anything past four is not a heavier vertex, it is a vertex whose smallest "
              "influences vanish silently at export")


def the_hands(rig, mesh, scene):
    """Which way the palms face, in the bind AND in every clip.

    # Why not the obvious plane fit

    The obvious measure is the direction the hand's vertex cloud varies LEAST in - the flat of
    the hand. On this character that returns a flatness of 0.45: the region carries the wrist and
    the sleeve cuff, so it is a lump rather than a plane, and the "normal" of a lump is whatever
    noise decides. It happily reported four different answers for four clips that pose the hand
    identically.

    Built instead from two things not in doubt: the forearm's own axis, and the KNUCKLE LINE, the
    widest direction of the hand once the forearm axis is projected out of it. The palm normal is
    perpendicular to both. Nothing to fit and no flatness required.

    # Every pose, not just the bind

    It read the bind only. The palm correction lives in the CLIPS - a bind change invalidates
    every clip authored against it - so the instrument reported the one pose the fix deliberately
    does not touch, and said "unchanged" about a change that had worked.
    """
    import numpy

    groups = {g.index: g.name for g in mesh.vertex_groups}
    owned = {"L": [], "R": []}
    for vertex in mesh.data.vertices:
        best, who = 0.0, ""
        for group in vertex.groups:
            if group.weight > best:
                best, who = group.weight, groups.get(group.group, "")
        if who.endswith("_Hand"):
            owned[who[0]].append(vertex.index)

    def palms():
        posed = mesh.evaluated_get(bpy.context.evaluated_depsgraph_get())
        for side in "LR":
            if len(owned[side]) < 12 or f"{side}_Hand" not in rig.pose.bones:
                print(f"    {side}: no hand to measure")
                continue
            wrist = rig.matrix_world @ rig.pose.bones[f"{side}_Hand"].head
            elbow = rig.matrix_world @ rig.pose.bones[f"{side}_Forearm"].head
            along = wrist - elbow
            if along.length < 1e-9:
                continue
            along.normalize()

            spots = [posed.matrix_world @ posed.data.vertices[i].co for i in owned[side]]
            middle = sum(spots, mathutils.Vector()) / len(spots)
            flat = [(p - middle) - along * (p - middle).dot(along) for p in spots]
            cloud = numpy.array([[v.x, v.y, v.z] for v in flat])
            _u, _s, axes = numpy.linalg.svd(cloud, full_matrices=False)
            knuckles = mathutils.Vector(axes[0]).normalized()
            palm = along.cross(knuckles).normalized()

            # The SIDE it faces is NOT measurable from this, and saying so is the point. A plane
            # has two sides and nothing here knows which is the palm - there are no finger bones.
            # An earlier version flipped the normal outward and then reported how far it was from
            # the thigh, which forces the answer: after that flip "toward the thigh" is negative
            # by construction. It read -83% and -85%, and both were arithmetic, not anatomy.
            inward = mathutils.Vector((wrist.x, wrist.y, 0.0))
            inward = inward.normalized() if inward.length > 1e-9 else mathutils.Vector((1, 0, 0))
            off_thigh = math.degrees(math.acos(min(1.0, abs(palm.dot(inward)))))
            off_flat = math.degrees(math.acos(min(1.0, abs(palm.z))))
            print(f"    {side}: palm plane {off_thigh:5.1f} deg off the thigh direction, "
                  f"{off_flat:5.1f} deg off vertical")

    print("")
    print("THE HANDS")
    # # No bind-pose row, on purpose
    #
    # Clearing the action and re-reading gives the LAST EVALUATED pose, not the rest pose: the
    # depsgraph keeps it, and stepping to the same frame is a no-op. The "bind" row came back
    # identical to whichever clip had been measured before it, every time.
    #
    # Rather than print a number that cannot be trusted, it is not printed. The palm correction
    # lives in the clips anyway - a bind change would invalidate every clip authored against it -
    # so the per-clip rows are the ones that answer the question, and those do move.
    for clip in sorted(bpy.data.actions, key=lambda a: a.name):
        play(rig, clip)
        scene.frame_set(int(round(clip.frame_range[0])))
        bpy.context.view_layer.update()
        print(f"  {clip.name}, frame {scene.frame_current}")
        palms()
    print("    palms ON the thighs means both angles small. WHICH SIDE faces in is not")
    print("    measurable here - render the hands in clay and look.")



# Poses the delivered clips do not contain, so deformation can be measured where it will
# actually be asked for. The arms barely leave the sides in an idle, a walk or a run - so
# strain measured on those alone says nothing about what happens when he reaches for something.
#
# (bone, axis, degrees) - applied on top of the rest pose, one pose at a time.
# The axes are MEASURED, not assumed: on this rig local Z abducts (the left wrist moves 0.23
# units away from the spine at +70 degrees), X flexes forward, and Y is the twist along the
# bone. The first version used X for "arms overhead" and was actually posing them forward.
TRIES = (
    ("arms out sideways", (("L_Upperarm", "Z", 85.0), ("R_Upperarm", "Z", -85.0))),
    ("arms forward", (("L_Upperarm", "X", 80.0), ("R_Upperarm", "X", 80.0))),
    ("a long stride", (("L_Thigh", "X", -55.0), ("R_Thigh", "X", 55.0))),
    ("a deep crouch", (("L_Thigh", "X", -95.0), ("R_Thigh", "X", -95.0),
                       ("L_Calf", "X", 110.0), ("R_Calf", "X", 110.0))),
)

# Past this much of its rest length an edge is tearing rather than stretching. 1.35 is the figure
# the skinning references use, and it is where a surface starts to read as rubber.
TEARS_PAST = 1.35


def the_deformation(rig, mesh, scene, owner):
    """How far every edge stretches, across every clip AND across poses the clips never reach.

    # Why this replaces the length test

    "An edge longer than four times the median" counted 182 faults on this character. Rendered
    with them picked out, they are the chest panel and the crotch - ordinary large polygons on a
    body that welds to 2464 vertices. Adding "and it spans two body regions" got it to 24, and
    the worst of those are `Waist <-> ThighTwist01` at the hip, where a trunk and a leg are
    SUPPOSED to be one surface.

    Length was never the question. A bridge is a bridge because it STRETCHES when the two things
    it joins move apart, and that is measurable directly rather than guessed at from geometry.

    # And why the clips are not enough on their own

    In an idle, a walk and a run the arms barely leave the sides, so armpit webbing never gets
    pulled and reports nothing. `TRIES` poses the character where the game will: reaching,
    striding, crouching.
    """
    def stand_at_rest():
        """Every bone back to identity, explicitly.

        Clearing the action is NOT enough - the depsgraph keeps the last evaluated pose, so the
        "rest" lengths came out as whatever had been posed most recently. Every strain figure
        below is divided by these, so getting them from a stale pose makes the whole section
        fiction. Setting each bone to identity is the one way that cannot be stale.
        """
        if rig.animation_data:
            rig.animation_data.action = None
        for bone in rig.pose.bones:
            bone.rotation_mode = "QUATERNION"
            bone.rotation_quaternion = mathutils.Quaternion()
            bone.location = mathutils.Vector((0.0, 0.0, 0.0))
            bone.scale = mathutils.Vector((1.0, 1.0, 1.0))
        bpy.context.view_layer.update()

    rest = {}
    stand_at_rest()
    posed = mesh.evaluated_get(bpy.context.evaluated_depsgraph_get())
    for edge in mesh.data.edges:
        a, b = edge.vertices
        rest[edge.index] = ((posed.matrix_world @ posed.data.vertices[a].co)
                            - (posed.matrix_world @ posed.data.vertices[b].co)).length

    worst = {}

    def look(what):
        posed = mesh.evaluated_get(bpy.context.evaluated_depsgraph_get())
        for edge in mesh.data.edges:
            a, b = edge.vertices
            now = ((posed.matrix_world @ posed.data.vertices[a].co)
                   - (posed.matrix_world @ posed.data.vertices[b].co)).length
            if rest[edge.index] < 1e-6:
                continue
            ratio = now / rest[edge.index]
            if ratio > worst.get(edge.index, (0.0, ""))[0]:
                worst[edge.index] = (ratio, what, a, b)

    for clip in sorted(bpy.data.actions, key=lambda a: a.name):
        play(rig, clip)
        first, last = (int(round(v)) for v in clip.frame_range)
        for frame in range(first, last + 1, 2):
            scene.frame_set(frame)
            bpy.context.view_layer.update()
            look(clip.name)

    for what, turns in TRIES:
        stand_at_rest()
        for name, axis, degrees in turns:
            if name in rig.pose.bones:
                rig.pose.bones[name].rotation_quaternion = mathutils.Quaternion(
                    {"X": (1, 0, 0), "Y": (0, 1, 0), "Z": (0, 0, 1)}[axis],
                    math.radians(degrees))
        bpy.context.view_layer.update()
        look(what)

    torn = sorted(((v[0], v[1], v[2], v[3]) for v in worst.values() if v[0] > TEARS_PAST),
                  reverse=True)
    print("")
    print("THE DEFORMATION")
    print(f"  measured over every clip and {len(TRIES)} poses they do not contain")
    print(f"  {len(torn)} of {len(rest)} edges stretch past {TEARS_PAST:.2f}x their rest length")
    # Broken down by WHERE, because a synthetic pose that bends a joint the wrong way produces
    # exactly the huge numbers a genuine fault does. If the clips - which are known-good
    # animation - tear little and only a made-up pose tears a lot, the pose is the suspect.
    from collections import Counter
    per = Counter(what for _, what, _, _ in torn)
    for what, count in per.most_common():
        print(f"    {count:4d} in {what}")
    for ratio, what, a, b in torn[:8]:
        print(f"    x{ratio:5.2f}  {owner.get(a, '?')} <-> {owner.get(b, '?')}   in {what}")
    if not torn:
        print("    nothing tears - the surface holds everywhere it was asked to bend")


def the_twists(rig, mesh):
    """Whether the twist bones are set up as twist bones, and whether anything drives them.

    Eighteen of this rig's forty-one bones are twists, and a twist bone that is mis-parented,
    unweighted or inert is dead weight in the budget. Three questions:

      does it LIE ALONG its parent   a twist rolls about its parent's length; angled, it swings
      does it CARRY SKIN             weights, or it cannot distribute anything
      does keying it MOVE skin       the only test that catches an inert one

    The third needs a FRESH depsgraph after the pose change. Re-using the one fetched before it
    reports 0.00 cm for every bone - which is what a broken rig looks like, and what this
    measured before the fix.
    """
    print("")
    print("THE TWISTS")
    groups = {g.index: g.name for g in mesh.vertex_groups}
    counts = collections.Counter()
    for vertex in mesh.data.vertices:
        best, who = 0.0, ""
        for group in vertex.groups:
            if group.weight > best:
                best, who = group.weight, groups.get(group.group, "")
        counts[who] += 1

    twists = [b for b in rig.data.bones if "Twist" in b.name]
    askew = [b for b in twists
             if b.parent and (b.tail_local - b.head_local).length > 1e-9
             and (b.parent.tail_local - b.parent.head_local).length > 1e-9
             and math.degrees((b.tail_local - b.head_local).angle(
                 b.parent.tail_local - b.parent.head_local)) > 5.0]
    bare = [b.name for b in twists if counts.get(b.name, 0) == 0]
    print(f"  {len(twists)} twist bones; {len(askew)} lie off their parent's length, "
          f"{len(bare)} carry no skin")
    if askew:
        print(f"    off-axis: {', '.join(b.name for b in askew[:6])}")
    if bare:
        print(f"    unweighted: {', '.join(bare[:6])}")

    if rig.animation_data:
        rig.animation_data.action = None
    for bone in rig.pose.bones:
        bone.rotation_mode = "QUATERNION"
        bone.rotation_quaternion = mathutils.Quaternion()
    bpy.context.view_layer.update()
    graph = bpy.context.evaluated_depsgraph_get()
    was = [(mesh.evaluated_get(graph).matrix_world @ v.co).copy()
           for v in mesh.evaluated_get(graph).data.vertices]
    inert = []
    for bone in twists:
        for other in rig.pose.bones:
            other.rotation_quaternion = mathutils.Quaternion()
        rig.pose.bones[bone.name].rotation_quaternion = mathutils.Quaternion(
            (0.0, 1.0, 0.0), math.radians(45.0))
        bpy.context.view_layer.update()
        graph = bpy.context.evaluated_depsgraph_get()
        posed = mesh.evaluated_get(graph)
        now = [(posed.matrix_world @ v.co) for v in posed.data.vertices]
        moved = max((a - b).length for a, b in zip(was, now)) * SCALE
        if moved < 0.1:
            inert.append(bone.name)
    print(f"  keying each one 45 degrees: {len(twists) - len(inert)} move skin, "
          f"{len(inert)} are inert")
    if inert:
        print(f"    inert: {', '.join(inert[:6])}")

    # And the thing that matters for the runtime: are they DRIVEN by anything but a clip?
    driven = [b.name for b in rig.pose.bones
              if "Twist" in b.name and b.constraints]
    print(f"  {len(driven)} have constraints driving them; the rest move only when a clip keys")
    if not driven:
        print("    ^ so a PROCEDURAL wrist or ankle rotation - IK, look-at, a grip pose - will")
        print("      crease at the joint instead of winding along the limb. Standard practice is")
        print("      a copy-rotation constraint at a fraction of the child's roll. Stage 05/06.")


def the_clips(rig, scene):
    if not bpy.data.actions:
        print("\nTHE CLIPS\n  none in this file")
        return
    bones = [b.name for b in rig.data.bones]
    print("\nTHE CLIPS")
    for clip in sorted(bpy.data.actions, key=lambda a: a.name):
        play(rig, clip)
        first, last = (int(round(v)) for v in clip.frame_range)
        lasts = (last - first) / scene.render.fps

        poses, travel = {}, 0.0
        began = ended = None
        for frame in range(first, last + 1):
            scene.frame_set(frame)
            bpy.context.view_layer.update()
            poses[frame] = [rig.pose.bones[n].rotation_quaternion.copy() for n in bones]
            hip = rig.matrix_world @ rig.pose.bones["Hip"].head
            if began is None:
                began = hip.copy()
            ended = hip.copy()
            travel = max(travel, (hip - began).length)

        def between(a, b):
            return apart(poses[a], poses[b])

        closes = between(first, last)
        verdict = "loops" if closes < 2.0 else f"DOES NOT LOOP"
        print(f"  {clip.name:<14s} {lasts:7.4f} s, frames {first}..{last}, "
              f"hip travels {travel * SCALE:6.1f} cm")
        print(f"  {'':<14s} first to last pose {closes:6.2f} deg  <- {verdict}")
        # Rotation closing is only half of a loop. A hip that ENDS somewhere other than where it
        # began snaps back the instant the clip wraps, and no angle reports that: `travel` is
        # the largest excursion from frame one, so a clip that sways 27 cm and comes home reads
        # the same as one that walks 27 cm away and stays there.
        drift = (ended - began).length * SCALE if began is not None else 0.0
        print(f"  {'':<14s} hip ends {drift:6.1f} cm from where it began  <- "
              f"{'lands' if drift < 1.0 else 'SNAPS BACK on the wrap'}")
        if closes >= 2.0:
            best = sorted(((between(first, f), f) for f in range(first + 4, last + 1)))[:1]
            if best:
                print(f"  {'':<14s} nearest repeat is frame {best[0][1]} at {best[0][0]:.2f} deg")


def main():
    args = argv()
    model = args[args.index("--model") + 1] if "--model" in args else os.path.join(
        os.path.dirname(os.path.dirname(ART)), "assets", "models", "person_ranger.glb")

    bpy.ops.wm.read_factory_settings(use_empty=True)
    for stale in list(bpy.data.objects):
        bpy.data.objects.remove(stale, do_unlink=True)
    bpy.ops.import_scene.gltf(filepath=model.replace("\\", "/"))

    rig = next((o for o in bpy.data.objects if o.type == "ARMATURE"), None)
    if rig is None:
        raise SystemExit("REFUSED: no armature in this file")
    mesh = the_body(bpy.data.objects)

    low = min((mesh.matrix_world @ v.co).z for v in mesh.data.vertices)
    high = max((mesh.matrix_world @ v.co).z for v in mesh.data.vertices)
    print(f"=== {os.path.basename(model)} ===")
    print(f"a figure {high - low:.4f} units tall, which is {(high - low) * SCALE:.1f} cm "
          f"once the game scales it\n")

    the_surface(mesh, owner_of(mesh))
    the_skeleton(rig)
    the_skin(mesh)
    the_hands(rig, mesh, bpy.context.scene)
    the_twists(rig, mesh)
    the_deformation(rig, mesh, bpy.context.scene, owner_of(mesh))
    the_clips(rig, bpy.context.scene)


if __name__ == "__main__":
    main()
