"""Puts the three delivered clips onto one character and writes the game's asset.

    blender --background --python build_character.py

Reads `assets/character/*.glb` and writes `assets/models/person_ranger.glb` with the mesh, the
skeleton and three clips named `idle`, `walk` and `run`.

# Why there is no retargeting here

Measured off the files themselves, all three carry the SAME mesh - `tripo_node_eafb5436`, 7844
vertices, 4899 triangles - and the SAME 41-joint skeleton in the same order. A clip cannot be
copied across a bind change, so the first thing this does is prove there is no bind change:
`the_skeletons_match` compares joint names, parents AND rest transforms, and refuses if any of
them differ. If a later delivery breaks that, this stops rather than quietly producing a
character whose arms are in the wrong place.

# What is measured rather than described

The clips are authored at different frame rates - walk's first key lands at 1/24 s and run's at
1/30 - so a frame count is not a shared unit and nothing here uses one. Durations come from the
clip's own range, which is what the animation player will use.

How far the body travels in one cycle is the single most consequential number in movement,
because playback rate is `lasts * speed / covers`. It is measured here and printed. A value
belonging to a different animation is exactly what running through water looks like.
"""
import math
import os
import sys

import bpy
import mathutils

ART = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(ART))
SOURCE = os.path.join(ROOT, "assets", "character")
OUT = os.path.join(ROOT, "assets", "models", "person_ranger.glb")

# The delivered file, and what the game calls the clip in it. `lookAround` becomes the idle.
DELIVERED = (
    ("idle.glb", "idle"),
    ("lookAround.glb", "look_around"),
    ("walk.glb", "walk"),
    ("run.glb", "run"),
)

# Clips laid end to end into one, and what the result is called.
#
# Standing still runs one long clip rather than a loop plus a break the game has to schedule.
# Simpler in every direction: no timer, no state, no chance of two wardens breaking on the same
# frame, and the animator decides how often he looks around by where they put it in the clip.
#
# Measured, the two joins are 10.29 and 10.60 degrees apart - small, but not nothing, so
# `join_the_clips` bends the start of each segment to meet the end of the one before it.
JOIN_INTO = (("idle", ("idle", "look_around")),)

# Over how many frames a join is absorbed. Half a second at 24 fps: long enough that a ten-degree
# correction is not a visible snap, short enough that it does not eat the motion it is bending.
JOIN_OVER = 12

# # The examine-hands moment: ATTEMPTED, REVERTED, and what it taught
#
# Three poses were authored by composing measured axis rotations onto the baked idle, and every
# one failed a different way: hands at the belly with forearms crossed, hands through the
# jacket, elbows driven into each other. The root cause is the rig itself - the elbow hinge
# sweeps ACROSS the body, so a natural "hands up, palms toward the face" needs coordinated
# shoulder twist per arm, and composing fixed axis offsets cannot coordinate anything. This is
# precisely the job of hand IK: solve where the hands should BE and let the arms follow. The
# beat returns at stage 07, posed by the solver instead of by arithmetic.
#
# EXAMINES stays False until then. The constants below are the measured record - which axes do
# what on this rig - and the envelope/compose machinery is sound and reused when it returns.
EXAMINES = False
#   toward the face   upper-arm Y twist points the hinge forward (L-50/R+50), then forearm X
#   look down         Head X negative (the crown tips toward his own forward)
#   palms to face     the hands roll back along the palm-correction axis
EXAMINES_AT = (140, 284)     # frames of the joined idle; a calm stretch of the plain stand
EXAMINE_EASES = 30           # frames of ease at each edge of the window
# (axis, degrees, LAG in frames). The lags are what makes it read as a person rather than a
# machine: the upper arms lead, the forearms follow a quarter-second later, the hands turn over
# as they arrive, and the head comes down last to meet them. Everything still eases to zero
# inside the window, lag included.
# A LIST, because a bone may take two turns. The anatomy of examining your hands: the elbows
# STAY AT THE RIBS and bend to ninety-odd degrees, which puts the hands a forearm's length in
# front of the chest - they cannot clip what they are held away from. The shoulders barely move:
# a touch of forward flexion, almost no swing. The second pass had this backwards - it swung the
# arms inward from the shoulder, which drags the forearms across the torso, and the render was
# read too kindly. `the_hands_stay_off_the_chest` now measures what the eye excused.
# The elbow hinge on this rig sweeps ACROSS the body, not forward - both earlier poses folded
# the forearms over the belly because of it. The upper arm must TWIST (its own Y) to point the
# hinge forward before the elbow bends; measured, L wants -50 and R +50, which lands the hands
# 20-28 cm ahead of the chest at 118-119 cm up. Sequence per side: lift a little (X), twist the
# hinge forward (Y), bend the elbow (forearm X), turn the palm (hand Y).
EXAMINE = (
    ("L_Upperarm", (1.0, 0.0, 0.0), 18.0, 0),
    ("L_Upperarm", (0.0, 1.0, 0.0), -50.0, 0),
    ("R_Upperarm", (1.0, 0.0, 0.0), 18.0, 2),
    ("R_Upperarm", (0.0, 1.0, 0.0), 50.0, 2),
    ("L_Forearm", (1.0, 0.0, 0.0), 85.0, 6),
    ("R_Forearm", (1.0, 0.0, 0.0), 81.0, 8),
    ("L_Hand", (0.0, 1.0, 0.0), -60.0, 10),
    ("R_Hand", (0.0, 1.0, 0.0), 60.0, 12),
    ("Head", (1.0, 0.0, 0.0), -20.0, 14),
)
# The fingers SPLAY - he spreads them to look at them, which is what a person does, and it is
# also the pose that shows whether the digits are truly separate. Spread is about local Z (the
# palm normal the bone rolls were aligned to), base phalanx only, fanning outward from the
# middle finger: the middle stays, the index and ring lean away a little, the thumb and pinky
# a lot. A touch of straightening on every phalanx opens the hand flat.
FINGERS_SPLAY_TO = 17.0
FANS = {"Thumb": -2.2, "Index": -1.0, "Middle": 0.0, "Ring": 1.0, "Pinky": 2.0}
SPLAY_SIGNS = {"L": 1.0, "R": -1.0}
FINGERS_FLATTEN_BY = -7.0
DIGITS_TRAIL_BY = 3          # frames each digit lags the one before it, thumb first

# Which clips are supposed to carry the character somewhere. Everything else is a standing
# motion, and a standing motion with no travel is correct rather than broken - the refusal below
# is there to catch a gait whose channels never bound, which is what an unbound action slot
# looks like from the outside.
TRAVELS = ("walk", "run")

# How far two rest transforms may differ before the skeletons are called different. Tight: this
# asks whether two exports of the same rig agree, not whether two rigs are similar.
RESTS_MATCH_WITHIN = 1e-5

# # The armpit is NOT cut, and the record below is why it stays anyway
#
# Three builds taught this the hard way. The recorded faces both join an arm to the trunk and
# tear when the arm lifts - but cutting them made real holes ("his chest is full of holes"),
# because the "walls behind them" that justified cutting were BACKFACES: clay renders both sides
# of a surface, so an armpit gap showing tidy surface behind it was showing the inside of the far
# wall. The membrane is the ONLY surface there. Deleting it means holes or fan-caps that read as
# fins; the honest fix is stage 03's - reweight the mis-weighted chest vertices off the forearm
# twists, and model a proper gusset where the membrane is.
#
# The record stays because it is the measured worklist for that stage, face by face.
CUT_THE_WEBBING = False

# # The armpit webbing, face by face, both sides
#
# The generator webbed the inner arms to the ribs where they rested close: no daylight under
# either arm in any idle frame, and 201 edges tearing past 1.35x with the arms overhead. These
# are the faces that both JOIN an arm to the trunk and STRETCH when the arm lifts, sitting clear
# below the shoulder joint - measured by `webbing.py`'s criteria, rendered in red, and agreed by
# eye before anything was cut. The 23 faces at shoulder height that also matched are the deltoid
# cap, and they stay: a correct shoulder joins arm to trunk too.
#
# Recorded as CENTROIDS rather than indices, so the cut finds each face by where it is. A
# re-delivered file with a different face order then refuses loudly instead of cutting somebody's
# chest out. The two sides are not mirror images - 18 faces against 28 - so each got its own
# record and its own inspection: the left was cut first, looked at with the arm out, and agreed;
# the right followed. Re-measured after the left cut, the finder reports left 0, right 28, which
# is the cut and the record confirming each other.
WEBBING = {
    "L": (
        (0.084475, 0.072733, 0.660144),
        (0.072733, 0.083170, 0.656230),
        (0.081213, 0.063601, 0.695369),
        (0.074038, 0.070124, 0.707111),
        (0.075342, 0.066862, 0.719505),
        (0.072733, 0.053816, 0.735812),
        (0.049250, 0.096217, 0.653621),
        (0.034247, 0.099478, 0.646445),
        (0.030333, 0.098826, 0.636660),
        (0.012068, 0.097521, 0.630137),
        (0.008154, 0.094260, 0.636008),
        (0.000978, 0.093607, 0.630137),
        (0.000326, 0.092303, 0.645793),
        (-0.013372, 0.086432, 0.641227),
        (-0.021200, 0.088389, 0.664710),
        (-0.019243, 0.097521, 0.687541),
        (-0.025114, 0.105349, 0.709068),
        (-0.064905, 0.096217, 0.729289),
    ),
    "R": (
        (0.030333, -0.102740, 0.547293),
        (0.030333, -0.098174, 0.542074),
        (0.006849, -0.087737, 0.544031),
        (-0.006197, -0.085780, 0.543379),
        (0.002935, -0.104044, 0.577299),
        (-0.008806, -0.092955, 0.570776),
        (0.034899, -0.094912, 0.634703),
        (0.056425, -0.083170, 0.660796),
        (0.042074, -0.087736, 0.657534),
        (0.020548, -0.090998, 0.643183),
        (0.009459, -0.089041, 0.632094),
        (0.079909, -0.049250, 0.634051),
        (0.081213, -0.057730, 0.653620),
        (0.080561, -0.047945, 0.652316),
        (0.078604, -0.034247, 0.633399),
        (0.077299, -0.019896, 0.658839),
        (0.057078, -0.072733, 0.695368),
        (0.052511, -0.060992, 0.735812),
        (0.059035, -0.049250, 0.734507),
        (-0.030333, -0.074038, 0.578604),
        (-0.033594, -0.068167, 0.593607),
        (-0.038160, -0.064253, 0.610567),
        (-0.014025, -0.078604, 0.622961),
        (-0.044684, -0.057730, 0.610568),
        (-0.044684, -0.054468, 0.617091),
        (-0.040117, -0.062296, 0.621657),
        (-0.040117, -0.062948, 0.641879),
        (-0.047945, -0.062296, 0.664710),
    ),
}

# How near a face's centroid must be to its recorded position to be the recorded face. Half a
# millimetre at model scale; the faces themselves are centimetres apart.
THE_SAME_FACE_WITHIN = 5e-4

# Coincident split copies weld together at this grain; genuine neighbours never do.
WELD_WITHIN = 0.00002

# # Finger bones: three per digit, five digits, both hands
#
# How each phalanx shares its digit's length, base to tip. Anatomical averages; on a stylised
# 9 cm hand the difference from perfect is invisible, and the joints land where the mesh has
# vertices to bend.
PHALANX_SHARES = (0.45, 0.30, 0.25)

# A digit's vertices are the ones past this share of the way from its knuckle to its tip,
# measured as graph distance from the wrist. Below it is palm.
A_DIGIT_STARTS = 0.52

# Half-width of the blend at each joint, as a share of the digit's length. A hard weight
# boundary creases; a blend this wide folds.
JOINT_BLENDS = 0.09

# The names, in anatomical order from the thumb. The THUMB is identified by the one fact about
# it that cannot lie: its base branches off the palm nearest the wrist. The last character
# taught this the expensive way - four discriminators in a row (shortest, most splayed, oddest
# angle, outlier) each confidently picked the PINKY, because a pinky is all of those things and
# a thumb is none of them.
DIGITS = ("Thumb", "Index", "Middle", "Ring", "Pinky")

# How far to roll each hand inward, in degrees, and which way that is per side.
#
# The delivered character stands SUPINATED - palms facing out, which no relaxed human does. It is
# in the bind, so every clip inherits it and no clip corrects it: the audit measures bind pose
# and idle frame 1 as identical.
#
# Corrected in the CLIPS rather than in the bind. A bind change invalidates every clip authored
# against it, and these were authored against this one; rolling the hand on each key preserves
# whatever the clip does with the arm and only changes where the hand rests while it does it.
#
# Rolled about the bone's own Y, which is along its length - that is the axis a forearm pronates
# about. Opposite signs per side because pronation is a mirror.
PALMS_ROLL_IN = 90.0
ROLLS = {"L": 1.0, "R": -1.0}

# How the roll is SHARED along the forearm, and why it has to be shared at all.
#
# Rolling only the hand puts the whole ninety degrees into one joint, and the wrist shreds into
# shards - visible in a clay render long before any number complains. The twist bones exist for
# exactly this, but the hierarchy here is not the obvious one:
#
#     L_Forearm -> L_ForearmTwist01 -> L_ForearmTwist02
#     L_Forearm -> L_Hand
#
# The hand is a SIBLING of the twists, not their child, so rolling the twists does not move it
# and rolling it does not twist the forearm. Both are needed.
#
# The shares are cumulative down the chain: a third at Twist01, a third more at Twist02 - which
# rides on Twist01, so it reaches two thirds - and the full amount on the hand, which hangs off
# the forearm and therefore carries no inherited roll. That ramps the skin from nothing at the
# elbow to everything at the wrist, which is what a forearm does.
SHARED_ALONG = (("ForearmTwist01", 1.0 / 3.0), ("ForearmTwist02", 1.0 / 3.0), ("Hand", 1.0))


def refuse(why):
    raise SystemExit(f"REFUSED: {why}")


def cut_the_webbing(rig, mesh):
    """Removes the recorded webbing faces and closes each side onto its own surface.

    # Found by position, refused on any doubt

    Every recorded centroid must match exactly one face within `THE_SAME_FACE_WITHIN`, or
    nothing at all is cut. Two mesh removals on the last character took the wrong thing, and
    both began with a selection that was almost right.

    # The caps do not re-bridge what was just cut

    Deleting the webbing leaves ONE boundary running around the hole - along the sleeve, across
    to the ribs, along the ribs, and back. Capping that loop as it stands would stitch the arm
    straight back onto the torso. So the boundary vertices are split by which REGION owns them,
    and each side is closed onto its own centre: the sleeve gets an inner wall, the ribs get a
    side wall, and the gap between them is the daylight this exists to create.

    # Two things Blender does here that would be silent

    A bmesh round trip DROPS custom split normals, and on a fully split mesh those carry all of
    the smooth shading - the melted-shoe fault, from a new direction. Every surviving loop's
    normal is snapshotted first, keyed by face centroid and corner position, and put back after;
    cap faces take their face normal, which is what a flat wall wants anyway.

    And the mesh is SPLIT, so a stored edge on the hole's rim may have a coincident twin that
    still has faces. Boundaries are found by welded position - an edge is on the rim only if ALL
    copies of it together now border exactly one face.
    """
    import bmesh
    from collections import defaultdict

    groups = {g.index: g.name for g in mesh.vertex_groups}

    def owner_of(vertex):
        best, who = 0.0, ""
        for group in vertex.groups:
            if group.weight > best:
                best, who = group.weight, groups.get(group.group, "")
        return who

    def side_of(name):
        return "arm" if any(k in name for k in ("Upperarm", "Forearm", "Hand")) else "trunk"

    def key_of(co):
        return (round(co.x / WELD_WITHIN), round(co.y / WELD_WITHIN), round(co.z / WELD_WITHIN))

    # The faces, by where they are.
    wanted = []
    for which, centroids in WEBBING.items():
        for spot in centroids:
            aim = mathutils.Vector(spot)
            near = [p for p in mesh.data.polygons
                    if (p.center - aim).length < THE_SAME_FACE_WITHIN]
            if len(near) != 1:
                refuse(f"the recorded {which} webbing face at {spot} matches {len(near)} faces "
                       f"- this is not the mesh the record was measured on, so nothing was cut")
            wanted.append(near[0].index)
    if len(set(wanted)) != len(wanted):
        refuse("two recorded centroids found the same face - nothing was cut")

    # # Only faces with a WALL BEHIND them are webbing. The rest are the chest.
    #
    # The record's criteria - joins an arm to the trunk, tears when the arm lifts - also caught
    # chest-surface faces whose vertices the generator mis-weighted to the forearm twists. They
    # join regions because their WEIGHTS are wrong, not because they bridge anything, and cutting
    # them holed the chest: 78 open edges against 10, reported as "his chest is full of holes"
    # within minutes of the build. Webbing is a layer OVER existing walls; the chest is the only
    # surface where it is. So each face must have another, non-neighbouring face close behind it
    # or it is skin and it stays, with a line saying so.
    from mathutils.bvhtree import BVHTree

    tree = BVHTree.FromPolygons(
        [v.co.copy() for v in mesh.data.vertices],
        [tuple(p.vertices) for p in mesh.data.polygons],
    )
    def key_ring(poly):
        return {key_of(mesh.data.vertices[v].co) for v in poly.vertices}

    backed, skin = [], []
    for index in wanted:
        poly = mesh.data.polygons[index]
        mine = key_ring(poly)
        # Everything within 1.5 cm of this face's centre that shares no welded corner with it.
        near = tree.find_nearest_range(poly.center, 0.009)
        others = [hit for hit in near
                  if hit[2] is not None and hit[2] != index
                  and not (key_ring(mesh.data.polygons[hit[2]]) & mine)]
        (backed if others else skin).append(index)
    if skin:
        print(f"    {len(skin)} recorded face(s) have NOTHING behind them - they are the chest, "
              f"not webbing, and they stay")
    wanted = backed

    # The shading, before bmesh forgets it.
    kept_normals = {}
    for poly in mesh.data.polygons:
        for loop_index in poly.loop_indices:
            loop = mesh.data.loops[loop_index]
            co = mesh.data.vertices[loop.vertex_index].co
            kept_normals[(key_of(poly.center), key_of(co))] = tuple(
                mesh.data.corner_normals[loop_index].vector)

    before_verts = len(mesh.data.vertices)
    bm = bmesh.new()
    bm.from_mesh(mesh.data)
    bm.faces.ensure_lookup_table()
    bm.verts.ensure_lookup_table()
    doomed = [bm.faces[i] for i in wanted]
    rim_candidates = {v.index for f in doomed for v in f.verts}
    bmesh.ops.delete(bm, geom=doomed, context="FACES")

    # The hole's rim, welded. An edge is on it only if its position-pair borders one face now.
    bm.verts.ensure_lookup_table()
    faces_on = defaultdict(int)
    edge_at = defaultdict(list)
    for edge in bm.edges:
        pair = tuple(sorted((key_of(edge.verts[0].co), key_of(edge.verts[1].co))))
        faces_on[pair] += len(edge.link_faces)
        edge_at[pair].append(edge)
    rim = set()
    for pair, count in faces_on.items():
        if count == 1:
            for edge in edge_at[pair]:
                for vert in edge.verts:
                    if vert.index in rim_candidates:
                        rim.add(vert)

    # Each side closes onto its own middle - IF there is a hole to close.
    #
    # Measured on this mesh, there is not: 32 candidate vertices produced a rim of ONE. Almost
    # every edge on the cut's boundary still borders a face at its welded position, which means
    # the webbing was an extra layer OVER existing sleeve and rib walls rather than the only
    # surface there - the 0.33 cm nearest-approach between arm and trunk was those walls. So a
    # side with no ring is the good outcome, reported and left alone; the caps exist for the day
    # a delivery genuinely has nothing behind its webbing.
    deform = bm.verts.layers.deform.verify()
    made_faces = []
    for which in ("arm", "trunk"):
        ring = [v for v in rim if side_of(owner_of(mesh.data.vertices[v.index])) == which]
        # One representative per welded position, or the fan doubles up on split copies.
        one_per_spot = {}
        for vert in ring:
            one_per_spot.setdefault(key_of(vert.co), vert)
        ring = list(one_per_spot.values())
        if len(ring) < 3:
            print(f"    the {which} side has {len(ring)} open rim vertices - the surface "
                  f"behind the webbing is already there, so there is nothing to close")
            continue
        middle = sum((v.co for v in ring), mathutils.Vector()) / len(ring)

        # Ordered around the ring's own plane, so the fan walks the rim instead of jumping it.
        away = [v.co - middle for v in ring]
        normal = mathutils.Vector((0.0, 0.0, 0.0))
        for a, b in zip(away, away[1:]):
            normal += a.cross(b)
        if normal.length < 1e-12:
            normal = mathutils.Vector((0.0, 0.0, 1.0))
        normal.normalize()
        east = away[0] - normal * away[0].dot(normal)
        if east.length < 1e-12:
            east = mathutils.Vector((1.0, 0.0, 0.0))
        east.normalize()
        north = normal.cross(east)
        ring.sort(key=lambda v: math.atan2((v.co - middle).dot(north),
                                           (v.co - middle).dot(east)))

        centre = bm.verts.new(middle)
        weights = defaultdict(float)
        for vert in ring:
            for group_index, weight in vert[deform].items():
                weights[group_index] += weight / len(ring)
        top = sorted(weights.items(), key=lambda kv: -kv[1])[:4]
        total = sum(w for _, w in top) or 1.0
        for group_index, weight in top:
            centre[deform][group_index] = weight / total

        for here, there in zip(ring, ring[1:] + ring[:1]):
            try:
                made_faces.append(bm.faces.new((here, there, centre)))
            except ValueError:
                pass  # a rim pair that already shares a face; the fan simply skips it
        print(f"    closed the {which} side with {len(ring)} rim vertices onto one centre")

    bmesh.ops.recalc_face_normals(bm, faces=made_faces)
    bm.to_mesh(mesh.data)
    bm.free()
    mesh.data.update()

    # The shading back on: restored where it survived, taken from the face where it is new.
    normals = []
    for poly in mesh.data.polygons:
        for loop_index in poly.loop_indices:
            loop = mesh.data.loops[loop_index]
            co = mesh.data.vertices[loop.vertex_index].co
            normals.append(kept_normals.get((key_of(poly.center), key_of(co)),
                                            tuple(poly.normal)))
    mesh.data.normals_split_custom_set(normals)

    print(f"  cut {len(wanted)} of the "
          f"{sum(len(c) for c in WEBBING.values())} recorded faces "
          f"({' + '.join(f'{len(c)} {s}' for s, c in WEBBING.items())} recorded); "
          f"vertices {before_verts} -> {len(mesh.data.vertices)}")
    if not mesh.data.has_custom_normals:
        refuse("the cut dropped the custom split normals - the whole body would be lit as a "
               "different shape")


# A hole bigger than this many rim vertices is not filled, it is reported: something that large
# is an intentional opening - a collar, a cuff - and capping one of those is its own bug.
A_HOLE_IS_SMALL = 30


def add_the_fingers(rig, mesh):
    """Gives each hand fifteen bones, placed on the digits the mesh actually has.

    # Finding the digits without guessing

    Graph distance from the wrist, along the hand's own surface (welded by position first -
    stored connectivity is shredded by UV seams). The five vertices furthest from the wrist,
    kept apart from each other, are the fingertips; every hand vertex then belongs to the tip
    it is nearest along the surface, and the far span of each of those basins is a digit.

    # Naming them without guessing

    The thumb's BASE - the nearest-to-wrist vertex of its basin - sits closer to the wrist than
    any finger's, because a thumb branches off the palm early. That is the whole test. The four
    fingers then take their names in order along the knuckle line, starting beside the thumb.
    Nothing here asks which digit is short, splayed or odd: all three of those confidently name
    the pinky, and it cost the last character four wrong hands in a row to learn it.

    # The bones follow the mesh; the weights follow the bones

    Each digit's spine is a polyline through the centroids of its distance-bands, so a curved
    digit gets bones that follow the curve. Joints land at the anatomical shares, weights move
    from the hand bone onto whichever phalanx spans each vertex, blended at the joints, and the
    palm keeps the hand bone. Only the `X_Hand` share of a vertex moves - a vertex the forearm
    also drives keeps that influence untouched, so sums stay at one without renormalising.
    """
    import heapq
    from collections import defaultdict

    groups = {g.index: g.name for g in mesh.vertex_groups}

    def key_of(co):
        return (round(co.x / WELD_WITHIN), round(co.y / WELD_WITHIN), round(co.z / WELD_WITHIN))

    added = 0
    assigned = {}
    for side in "LR":
        hand_bone = f"{side}_Hand"
        owned = []
        for vertex in mesh.data.vertices:
            best, who = 0.0, ""
            for group in vertex.groups:
                if group.weight > best:
                    best, who = group.weight, groups.get(group.group, "")
            if who == hand_bone:
                owned.append(vertex.index)
        if len(owned) < 60:
            print(f"    {side}: only {len(owned)} hand vertices - no fingers added")
            continue

        # The welded surface graph of this hand.
        canon, seen_at = {}, {}
        for index in owned:
            canon[index] = seen_at.setdefault(key_of(mesh.data.vertices[index].co), index)
        nodes = set(canon.values())
        at = {n: (mesh.matrix_world @ mesh.data.vertices[n].co) for n in nodes}
        touching = defaultdict(set)
        for edge in mesh.data.edges:
            a, b = edge.vertices
            if a in canon and b in canon and canon[a] != canon[b]:
                touching[canon[a]].add(canon[b])
                touching[canon[b]].add(canon[a])

        wrist = rig.matrix_world @ rig.pose.bones[hand_bone].head
        start = min(nodes, key=lambda n: (at[n] - wrist).length)
        dist = {start: 0.0}
        queue = [(0.0, start)]
        while queue:
            so_far, here = heapq.heappop(queue)
            if so_far > dist.get(here, 1e9):
                continue
            for other in touching[here]:
                step = so_far + (at[here] - at[other]).length
                if step < dist.get(other, 1e9):
                    dist[other] = step
                    heapq.heappush(queue, (step, other))
        unreached = nodes - set(dist)
        if unreached:
            print(f"    {side}: {len(unreached)} hand vertices are not connected to the wrist "
                  f"- left on the hand bone")
            nodes -= unreached

        # Five tips: furthest first, each at least a fifth of the hand apart from the others -
        # and each VALIDATED by the digit it produces. The left hand's first pick included a
        # sleeve-cuff vertex: far from the wrist along the surface (the long way round the
        # cuff), so it looked like a fingertip and its "digit" held one vertex. A tip whose
        # basin has no body is banned and the next candidate takes its place.
        span = max(dist.values())
        banned = set()
        digits = None
        for _ in range(8):
            tips = []
            for node in sorted(nodes, key=lambda n: -dist[n]):
                if node in banned or dist[node] < span * 0.55:
                    continue
                if all((at[node] - at[t]).length > span * 0.20 for t in tips):
                    tips.append(node)
                if len(tips) == 5:
                    break
            if len(tips) < 5:
                digits = None
                break

            # Every vertex joins the tip it is nearest along the surface.
            basin, queue = {}, []
            best_to = {tip: 0.0 for tip in tips}
            for tip in tips:
                basin[tip] = tip
                heapq.heappush(queue, (0.0, tip, tip))
            while queue:
                so_far, here, whose = heapq.heappop(queue)
                if so_far > best_to.get(here, 1e9):
                    continue
                basin[here] = whose
                for other in touching[here]:
                    step = so_far + (at[here] - at[other]).length
                    if step < best_to.get(other, 1e9):
                        best_to[other] = step
                        basin[other] = whose
                        heapq.heappush(queue, (step, other, whose))

            # A digit is the far span of its basin; a tip that cannot produce one is no tip.
            digits, impostor = {}, None
            for tip in tips:
                mine = [n for n in nodes if basin.get(n) == tip]
                body = [n for n in mine if dist[n] > dist[tip] * A_DIGIT_STARTS]
                if len(body) < 6:
                    impostor = tip
                    break
                digits[tip] = body
            if impostor is None and len(digits) == 5:
                break
            if impostor is not None:
                banned.add(impostor)
                print(f"    {side}: banned a fingertip candidate at "
                      f"{tuple(round(v, 3) for v in at[impostor])} - its digit had no body")
            digits = None
        if not digits:
            print(f"    {side}: could not settle five digits - no fingers added")
            continue

        # THE THUMB: the digit whose base sits nearest the wrist. Then the fingers in order
        # along the knuckle line, starting beside the thumb.
        base_of = {tip: min((dist[n] for n in body)) for tip, body in digits.items()}
        thumb = min(digits, key=lambda t: base_of[t])
        fingers = [t for t in digits if t is not thumb]
        thumb_spot = sum((at[n] for n in digits[thumb]), mathutils.Vector()) / len(digits[thumb])
        first = min(fingers, key=lambda t: (at[t] - thumb_spot).length)
        last = max(fingers, key=lambda t: (at[t] - thumb_spot).length)
        knuckles = (at[last] - at[first])
        knuckles = knuckles.normalized() if knuckles.length > 1e-9 else mathutils.Vector((1, 0, 0))
        fingers.sort(key=lambda t: (at[t] - at[first]).dot(knuckles))
        named = dict(zip(DIGITS, [thumb] + fingers))

        # Palm normal, for bone roll: perpendicular to the knuckle line and the hand's reach,
        # so every phalanx hinges about the same axis and a curl is one rotation per bone.
        reach = (sum((at[t] for t in tips), mathutils.Vector()) / 5 - wrist).normalized()
        palm = reach.cross(knuckles).normalized()

        into_rig = rig.matrix_world.inverted()
        bpy.context.view_layer.objects.active = rig
        bpy.ops.object.mode_set(mode="EDIT")
        for called, tip in named.items():
            body = digits[tip]
            low = min(dist[n] for n in body)
            top = dist[tip]
            length = max(top - low, 1e-9)

            def spot_at(share):
                aim = low + length * share
                near = sorted(body, key=lambda n: abs(dist[n] - aim))[:6]
                return sum((at[n] for n in near), mathutils.Vector()) / len(near)

            joints = [spot_at(0.0), spot_at(PHALANX_SHARES[0]),
                      spot_at(PHALANX_SHARES[0] + PHALANX_SHARES[1]), at[tip]]
            parent = rig.data.edit_bones[hand_bone]
            for count in range(3):
                bone = rig.data.edit_bones.new(f"{side}_{called}{count + 1}")
                bone.head = into_rig @ joints[count]
                bone.tail = into_rig @ joints[count + 1]
                bone.parent = parent
                bone.use_connect = count > 0
                bone.align_roll(into_rig.to_3x3() @ palm)
                parent = bone
                added += 1
        bpy.ops.object.mode_set(mode="OBJECT")

        # The weights: each digit vertex hands its X_Hand share to the phalanx that spans it,
        # blended at the joints. Palm vertices keep the hand bone.
        for called, tip in named.items():
            body = set(digits[tip])
            low = min(dist[n] for n in body)
            top = dist[tip]
            length = max(top - low, 1e-9)
            cuts = (PHALANX_SHARES[0], PHALANX_SHARES[0] + PHALANX_SHARES[1])
            lanes = [mesh.vertex_groups.new(name=f"{side}_{called}{n + 1}") for n in range(3)]
            hand_lane = mesh.vertex_groups[hand_bone]

            for index in owned:
                node = canon[index]
                if node not in body:
                    continue
                share = (dist[node] - low) / length
                assigned[index] = (side, called, share)
                had = 0.0
                for group in mesh.data.vertices[index].groups:
                    if groups.get(group.group, "") == hand_bone:
                        had = group.weight
                if had <= 0.0:
                    continue
                # Which phalanx, and how much of the neighbour at a joint.
                takes = [0.0, 0.0, 0.0]
                if share < cuts[0] - JOINT_BLENDS:
                    takes[0] = 1.0
                elif share < cuts[0] + JOINT_BLENDS:
                    blend = (share - (cuts[0] - JOINT_BLENDS)) / (2 * JOINT_BLENDS)
                    takes[0], takes[1] = 1.0 - blend, blend
                elif share < cuts[1] - JOINT_BLENDS:
                    takes[1] = 1.0
                elif share < cuts[1] + JOINT_BLENDS:
                    blend = (share - (cuts[1] - JOINT_BLENDS)) / (2 * JOINT_BLENDS)
                    takes[1], takes[2] = 1.0 - blend, blend
                else:
                    takes[2] = 1.0
                hand_lane.remove([index])
                for lane, take in zip(lanes, takes):
                    if take > 0.001:
                        lane.add([index], had * take, "REPLACE")
        print(f"    {side}: 15 bones on 5 digits; the thumb's base sits "
              f"{base_of[thumb] * 170.0:.1f} cm along the surface against "
              f"{min(base_of[t] for t in fingers) * 170.0:.1f} for the nearest finger")

    print(f"  added {added} finger bones")
    if added not in (0, 30):
        refuse(f"{added} finger bones is neither none nor all thirty - one hand failed after "
               f"the other succeeded, and half-fingered is worse than either")
    return assigned


def close_the_holes(rig, mesh):
    """Fills every open loop in the surface with faces over its own rim vertices.

    # Welded first, or there is nothing to find

    The mesh is split at every UV seam and hard edge, so an open loop that is obvious once
    welded is not a closed chain of stored edges - `fill_holes` selects everything and adds
    nothing, which is documented from the last character's trouser leg. Open edges are found as
    welded PAIRS bordering exactly one face, chained into loops by position, and filled with a
    fan over one representative stored vertex per position.

    # New faces inherit what their vertices already know

    Every rim vertex already sits in a kept face, so it has weights and a UV. The fan reuses the
    vertices themselves - no new vertex, no new weight - and each new corner copies its UV from
    an existing corner of the same vertex, so the texture continues across the fill instead of
    smearing from zero.
    """
    import bmesh
    from collections import defaultdict

    def key_of(co):
        return (round(co.x / WELD_WITHIN), round(co.y / WELD_WITHIN), round(co.z / WELD_WITHIN))

    # The shading, before bmesh forgets it - same move as the cut.
    kept_normals = {}
    for poly in mesh.data.polygons:
        for loop_index in poly.loop_indices:
            loop = mesh.data.loops[loop_index]
            co = mesh.data.vertices[loop.vertex_index].co
            kept_normals[(key_of(poly.center), key_of(co))] = tuple(
                mesh.data.corner_normals[loop_index].vector)

    bm = bmesh.new()
    bm.from_mesh(mesh.data)
    bm.verts.ensure_lookup_table()
    uv_layer = bm.loops.layers.uv.active

    faces_on = defaultdict(int)
    for edge in bm.edges:
        pair = tuple(sorted((key_of(edge.verts[0].co), key_of(edge.verts[1].co))))
        faces_on[pair] += len(edge.link_faces)
    crowded = sum(1 for n in faces_on.values() if n > 2)

    # One representative stored vertex per welded position, preferring one that sits in a face
    # so the fill can copy its UV.
    stands_for = {}
    for vert in bm.verts:
        spot = key_of(vert.co)
        if spot not in stands_for or (vert.link_faces and not stands_for[spot].link_faces):
            stands_for[spot] = vert

    # Open pairs, chained into loops by position.
    joins = defaultdict(set)
    for (a, b), count in faces_on.items():
        if count == 1:
            joins[a].add(b)
            joins[b].add(a)
    loops, seen = [], set()
    for start in joins:
        if start in seen:
            continue
        walk, here, came = [start], start, None
        seen.add(start)
        closed = False
        while True:
            following = [n for n in joins[here] if n != came]
            if not following:
                break
            came, here = here, following[0]
            if here == start:
                closed = True
                break
            if here in seen:
                break
            seen.add(here)
            walk.append(here)
        loops.append((walk, closed))

    filled, left = 0, 0
    for walk, closed in loops:
        ring = [stands_for[spot] for spot in walk]
        low = min((mesh.matrix_world @ v.co).z for v in ring)
        if not closed or len(ring) < 3 or len(ring) > A_HOLE_IS_SMALL:
            left += 1
            print(f"    left a {'loop' if closed else 'CHAIN'} of {len(ring)} open edges alone "
                  f"at {low * 170.0:.0f} cm up"
                  + ("" if closed else " - it does not close, which wants eyes"))
            continue
        # Each corner's UV, from any face its vertex already sits in.
        wears = {}
        for vert in ring:
            for loop in vert.link_loops:
                wears[vert] = loop[uv_layer].uv.copy()
                break
        for here, there in zip(ring[1:], ring[2:]):
            try:
                face = bm.faces.new((ring[0], here, there))
            except ValueError:
                continue
            face.smooth = True
            if uv_layer:
                for loop in face.loops:
                    if loop.vert in wears:
                        loop[uv_layer].uv = wears[loop.vert]
        filled += 1

    bm.to_mesh(mesh.data)
    bm.free()
    mesh.data.update()

    normals = []
    for poly in mesh.data.polygons:
        for loop_index in poly.loop_indices:
            loop = mesh.data.loops[loop_index]
            co = mesh.data.vertices[loop.vertex_index].co
            normals.append(kept_normals.get((key_of(poly.center), key_of(co)),
                                            tuple(poly.normal)))
    mesh.data.normals_split_custom_set(normals)

    print(f"  closed {filled} hole(s), left {left} alone; "
          f"{crowded} welded edge(s) still carry more than two faces")
    if not mesh.data.has_custom_normals:
        refuse("closing the holes dropped the custom split normals")


def rig_of(objects):
    return next((o for o in objects if o.type == "ARMATURE"), None)


def skeleton_of(rig):
    """Name, parent and rest matrix for every bone, in order - what a clip is authored against."""
    return [
        (bone.name,
         bone.parent.name if bone.parent else None,
         tuple(round(v, 6) for row in bone.matrix_local for v in row))
        for bone in rig.data.bones
    ]


def the_skeletons_match(first, other, called):
    """Refuses unless two rigs are the same skeleton, so clips can simply be moved across."""
    if len(first) != len(other):
        refuse(f"{called} has {len(other)} bones against {len(first)} - not the same skeleton, "
               f"so its clip cannot be copied over without retargeting")
    for mine, theirs in zip(first, other):
        if mine[0] != theirs[0] or mine[1] != theirs[1]:
            refuse(f"{called} has bone {theirs[0]} under {theirs[1]} where the base has "
                   f"{mine[0]} under {mine[1]} - the skeletons differ")
        off = max(abs(x - y) for x, y in zip(mine[2], theirs[2]))
        if off > RESTS_MATCH_WITHIN:
            refuse(f"{called} rests bone {theirs[0]} {off:.6f} away from the base - a clip "
                   f"authored against one bind does not mean the same thing on another")


def play(rig, clip):
    """Assigns a clip so it actually drives the rig.

    Assigning `animation_data.action` alone is not enough from Blender 4.4 on: an action holds
    SLOTS, and until one is bound the action is attached and inert. It reports success and moves
    nothing, which is how this first measured every clip as travelling 0.0 cm - a walk whose feet
    never left the ground, and a number that would have gone straight into `covers`.
    """
    if rig.animation_data is None:
        rig.animation_data_create()
    rig.animation_data.action = clip
    slots = getattr(clip, "slots", None)
    if slots:
        rig.animation_data.action_slot = slots[0]
    elif not hasattr(clip, "slots"):
        pass  # older Blender: the action drives the rig on its own


def fcurves_of(clip, slot):
    """Every fcurve in a clip, on Blender 5 and on what came before.

    From 4.4 an action is slots, layers, strips and channelbags rather than a flat
    `action.fcurves`, and reaching for the old attribute finds nothing and raises nothing.
    """
    if hasattr(clip, "fcurves") and len(clip.fcurves):
        return list(clip.fcurves)
    out = []
    for layer in getattr(clip, "layers", []):
        for strip in layer.strips:
            bag = strip.channelbag(slot) if slot else None
            if bag is None and getattr(strip, "channelbags", None):
                bag = strip.channelbags[0]
            if bag is not None:
                out.extend(bag.fcurves)
    return out


def stand_still(rig, clip, scene):
    """Takes the travel out of a clip and leaves the sway in. Returns how far it removed.

    These clips carry ROOT MOTION - the walk moves its root 1.50 units over the clip and the run
    2.81. The game moves the warden in code, so a clip that also translates him would move him
    twice, and the classic symptom is a character skating away from under himself.

    Detrended, not zeroed: a straight line from the first key to the last is subtracted, so the
    travel goes and the side-to-side sway and the bob a real gait has are kept. Zeroing the
    channel outright would take those with it and the walk would go rigid.

    What is subtracted is measured and returned, because it IS `covers` - the distance the clip
    carries him - and that is the number playback rate divides by.
    """
    play(rig, clip)
    slot = rig.animation_data.action_slot if rig.animation_data else None
    first, last = (int(round(v)) for v in clip.frame_range)
    curves = [c for c in fcurves_of(clip, slot)
              if c.data_path.endswith(".location") or c.data_path == "location"]
    if not curves:
        return 0.0, None

    # Whichever channel actually carries the travel, rather than an assumption about which bone
    # or which axis is forward.
    worst, moved = None, 0.0
    for curve in curves:
        keys = [k.co[1] for k in curve.keyframe_points]
        if not keys:
            continue
        drift = abs(keys[-1] - keys[0])
        if drift > moved:
            worst, moved = curve, drift
    if worst is None or moved < 1e-4:
        return 0.0, None

    who = worst.data_path.split('"')[1] if '"' in worst.data_path else "object"
    took = 0.0
    for curve in curves:
        if curve.data_path != worst.data_path:
            continue
        keys = curve.keyframe_points
        if len(keys) < 2:
            continue
        began, ended = keys[0].co[0], keys[-1].co[0]
        low, high = keys[0].co[1], keys[-1].co[1]
        span = max(ended - began, 1e-9)
        took += (high - low) ** 2
        for key in keys:
            slide = low + (high - low) * (key.co[0] - began) / span
            key.co[1] -= slide - low
            key.handle_left[1] -= slide - low
            key.handle_right[1] -= slide - low
        curve.update()
    return took ** 0.5, who


def roll_the_hands(rig, clip, degrees):
    """Rolls each hand inward by a constant on every key, so the palms rest on the thighs.

    Composed onto the keyed rotation rather than replacing it: `keyed * offset` in the bone's
    own space, which leaves the clip's motion exactly as authored and moves only the frame it
    happens in.
    """
    if abs(degrees) < 1e-6:
        return 0
    slot = rig.animation_data.action_slot if rig.animation_data else None
    curves = fcurves_of(clip, slot)
    turned = 0
    for side, way in ROLLS.items():
        for bone, share in SHARED_ALONG:
            path = f'pose.bones["{side}_{bone}"].rotation_quaternion'
            parts = {c.array_index: c for c in curves if c.data_path == path}
            if len(parts) != 4:
                continue
            offset = mathutils.Quaternion((0.0, 1.0, 0.0),
                                          math.radians(degrees * way * share))
            for at in range(len(parts[0].keyframe_points)):
                keyed = mathutils.Quaternion(
                    [parts[i].keyframe_points[at].co[1] for i in range(4)])
                rolled = keyed @ offset
                for i in range(4):
                    point = parts[i].keyframe_points[at]
                    was = point.co[1]
                    point.co[1] = rolled[i]
                    point.handle_left[1] += rolled[i] - was
                    point.handle_right[1] += rolled[i] - was
            for curve in parts.values():
                curve.update()
            turned += 1
    return turned


def sample(rig, clip, scene):
    """Every bone's local rotation, location and scale, frame by frame.

    Baked rather than re-keyed from the source curves, because the two clips are authored at
    different rates and against different key times - and a join has to happen on a single
    timeline whatever the pieces were written on.
    """
    play(rig, clip)
    first, last = (int(round(v)) for v in clip.frame_range)
    out = []
    for frame in range(first, last + 1):
        scene.frame_set(frame)
        bpy.context.view_layer.update()
        out.append({
            bone.name: (bone.rotation_quaternion.copy(),
                        bone.location.copy(),
                        bone.scale.copy())
            for bone in rig.pose.bones
        })
    return out


def bend_to_meet(poses, offsets, over, backwards=False):
    """Distributes a pose offset across `over` frames so a seam closes without a snap.

    Full correction at the seam itself, easing to none by the far end of the window, so the
    segment arrives exactly where the one before it left off and is back on its own motion
    within half a second.
    """
    for step in range(min(over, len(poses))):
        share = 1.0 - (step / over)
        at = -(step + 1) if backwards else step
        for name, (turn, shift) in offsets.items():
            if name not in poses[at]:
                continue
            was_turn, was_shift, was_scale = poses[at][name]
            poses[at][name] = (
                mathutils.Quaternion().slerp(turn, share) @ was_turn,
                was_shift + shift * share,
                was_scale,
            )


def join_the_clips(rig, scene, pieces, called):
    """Lays clips end to end into one, closing both seams and the loop.

    Two seams matter, not one: where the second piece starts on the first, and where the whole
    thing wraps back to its own beginning. A merged idle that closes the first and forgets the
    second pops once every time round.
    """
    frames = []
    for clip in pieces:
        frames.append(sample(rig, clip, scene))
    poses = frames[0]
    for after in frames[1:]:
        offsets = {}
        for name, (turn, shift, _) in poses[-1].items():
            if name not in after[0]:
                continue
            their_turn, their_shift, _ = after[0][name]
            offsets[name] = (turn @ their_turn.inverted(), shift - their_shift)
        bend_to_meet(after, offsets, JOIN_OVER)
        poses = poses + after

    # And the wrap: the last frame has to meet the first, or it pops once a lap.
    offsets = {}
    for name, (turn, shift, _) in poses[0].items():
        if name not in poses[-1]:
            continue
        their_turn, their_shift, _ = poses[-1][name]
        offsets[name] = (turn @ their_turn.inverted(), shift - their_shift)
    bend_to_meet(poses, offsets, JOIN_OVER, backwards=True)

    made = bpy.data.actions.new(called)
    made.use_fake_user = True
    rig.animation_data.action = made
    slots = getattr(made, "slots", None)
    if slots is not None:
        slot = made.slots.new(id_type="OBJECT", name="Armature")
        rig.animation_data.action_slot = slot
    for at, pose in enumerate(poses):
        scene.frame_set(at + 1)
        for bone in rig.pose.bones:
            if bone.name not in pose:
                continue
            bone.rotation_mode = "QUATERNION"
            bone.rotation_quaternion, bone.location, bone.scale = pose[bone.name]
            bone.keyframe_insert("rotation_quaternion", frame=at + 1)
            bone.keyframe_insert("location", frame=at + 1)
            bone.keyframe_insert("scale", frame=at + 1)
    return made, len(poses)


# A vertex this far along its digit is past the natural crotch, where real fingers join; only
# webbing above it is fused wrongly. Below it, connection is anatomy.
THE_CROTCH_ENDS = 0.22

# # Unfusing by DEEPENING the web, not by deleting it
#
# The first attempt deleted the 36 inter-digit faces and tried to wall the flanks. It left 45
# open edges that read as visible holes on the hands, and it was the wrong operation anyway.
#
# A web between fingers is ANATOMY. Every hand has one; on a real hand it runs down to about the
# crotch of the digits. What is wrong here is not that the web exists but that it is SHALLOW -
# it sits almost level with the digit surfaces, so the fingers read as one paddle. The
# production fix, and what the manual workflow does by hand, is to trim and then PULL THE
# FINGERS APART; the pulling is the half that was missing.
#
# So nothing is deleted. Vertices shared between two digits are pushed back toward the wrist
# along the hand's own reach axis, and pulled in toward the line between the two digits they
# sit between. The valley deepens, the digits stand clear of each other, and the surface stays
# exactly as watertight as it was - deleting nothing cannot open anything.
UNFUSES = True

# How far a shared vertex sinks toward the wrist, as a share of the digit's length, and how far
# it is drawn toward the seam between its two digits. Faded by how far along the digit it sits:
# full at the crotch, nothing by the fingertips, so the web deepens without narrowing the tips.
# Measured, not guessed at twice: 0.30 sank the crotch 4.14 cm on a 9 cm hand and tore the left
# hand into ribbons. The pinch is worse than too-large - it is WRONG: pulling shared vertices
# toward the seam between two digits drags both digits into each other, which is fusing them
# harder rather than parting them. Sink only, gently.
WEB_SINKS_BY = 0.08
WEB_PINCHES_BY = 0.0


def unfuse_the_digits(rig, mesh, assigned):
    """Deepens the web between fused fingers, so the digits read as separate. Deletes nothing.

    Every vertex that a face shares between two digits is sunk toward the wrist and pinched
    toward the seam between those digits, faded to nothing by the fingertip. That turns a
    shallow sheet into a valley, which is what a hand actually has.

    Coincident split copies move TOGETHER - the position is what is welded, so shifting one copy
    and not its twin would tear the surface open along a UV seam, which is the same class of
    fault as everything else on this mesh.
    """
    from collections import defaultdict

    def key_of(co):
        return (round(co.x / WELD_WITHIN), round(co.y / WELD_WITHIN), round(co.z / WELD_WITHIN))

    def digit_of(index):
        row = assigned.get(index)
        if row is None or row[2] < THE_CROTCH_ENDS:
            return None
        return (row[0], row[1])

    # Which vertices sit between two digits, and which two.
    between = defaultdict(set)
    for poly in mesh.data.polygons:
        each = [digit_of(v) for v in poly.vertices]
        parts = set(each) - {None}
        if len(parts) < 2 or len({p[0] for p in parts}) != 1:
            continue
        for vertex in poly.vertices:
            if assigned.get(vertex) is not None:
                between[vertex] |= parts
    if not between:
        print("  no inter-digit webbing found - the fingers are already separate")
        return

    # Each digit's own axis and tip, for the sink and the pinch.
    tips, bases = {}, {}
    for index, (side, digit, share) in assigned.items():
        spot = mesh.matrix_world @ mesh.data.vertices[index].co
        key = (side, digit)
        if share > tips.get(key, (0.0, None))[0]:
            tips[key] = (share, spot)
        if share < bases.get(key, (9.9, None))[0]:
            bases[key] = (share, spot)

    into_mesh = mesh.matrix_world.inverted()
    moved_by = {}
    for index, parts in between.items():
        row = assigned.get(index)
        if row is None or len(parts) < 2:
            continue
        side, digit, share = row
        pair = sorted(parts)[:2]
        if any(p not in tips or p not in bases for p in pair):
            continue
        spot = mesh.matrix_world @ mesh.data.vertices[index].co

        # Toward the wrist, along this digit's own axis.
        along = (tips[(side, digit)][1] - bases[(side, digit)][1])
        length = along.length
        if length < 1e-6:
            continue
        along = along / length
        fades = max(0.0, 1.0 - (share - THE_CROTCH_ENDS) / max(1.0 - THE_CROTCH_ENDS, 1e-9))
        sink = along * (-WEB_SINKS_BY * length * fades)

        # And toward the seam between the two digits it sits between.
        seam = (tips[pair[0]][1] + tips[pair[1]][1]) * 0.5
        toward = seam - spot
        toward -= along * toward.dot(along)
        pinch = toward * (WEB_PINCHES_BY * fades)

        moved_by[key_of(mesh.data.vertices[index].co)] = sink + pinch

    # Applied by POSITION, so every split copy of a shared vertex moves with its twins.
    shifted = 0
    for vertex in mesh.data.vertices:
        shift = moved_by.get(key_of(vertex.co))
        if shift is None:
            continue
        vertex.co = into_mesh @ ((mesh.matrix_world @ vertex.co) + shift)
        shifted += 1
    mesh.data.update()
    deepest = max((v.length for v in moved_by.values()), default=0.0) * 170.0
    print(f"  deepened the web between {len(between)} shared vertices "
          f"({shifted} stored copies), sinking the crotch up to {deepest:.2f} cm")


def examine_the_hands(rig, clip, scene):
    """Authors the examine-hands beat into the baked idle. See the constants above.

    The joined idle is baked - one key per frame on every bone, fingers included - so this is
    per-frame COMPOSITION on existing keys: `keyed @ offset(angle * envelope)`, the same move as
    the palm roll. The envelope is a smoothstep in and out that reaches exactly zero at the
    window's edges, which is what keeps the loop at 0.00 degrees: frames outside the window are
    not touched at all, and the window's first and last frames are touched by nothing.
    """
    slot = rig.animation_data.action_slot if rig.animation_data else None
    curves = fcurves_of(clip, slot)
    first, last = EXAMINES_AT
    span = max(last - first, 1)

    def envelope(frame, lag=0.0):
        t = frame - first - lag
        room = span - lag
        if t <= 0 or t >= room:
            return 0.0
        rise = min(1.0, t / EXAMINE_EASES)
        fall = min(1.0, (room - t) / EXAMINE_EASES)
        eased = min(rise, fall)
        return eased * eased * (3.0 - 2.0 * eased)

    def compose(path, axis, degrees_of):
        parts = {c.array_index: c for c in curves if c.data_path == path}
        if len(parts) != 4:
            return 0
        for at in range(len(parts[0].keyframe_points)):
            frame = parts[0].keyframe_points[at].co[0]
            if frame <= first or frame >= last:
                continue
            angle = degrees_of(frame)
            if abs(angle) < 1e-4:
                continue
            keyed = mathutils.Quaternion(
                [parts[i].keyframe_points[at].co[1] for i in range(4)])
            turned = keyed @ mathutils.Quaternion(axis, math.radians(angle))
            for i in range(4):
                point = parts[i].keyframe_points[at]
                was = point.co[1]
                point.co[1] = turned[i]
                point.handle_left[1] += turned[i] - was
                point.handle_right[1] += turned[i] - was
        for curve in parts.values():
            curve.update()
        return 1

    touched = 0
    for bone, axis, degrees, lag in EXAMINE:
        touched += compose(f'pose.bones["{bone}"].rotation_quaternion', axis,
                           lambda frame, d=degrees, l=lag: d * envelope(frame, l))

    # The splay: each digit fans from the middle finger and straightens a touch, a few frames
    # behind the digit before it, thumb first.
    splayed = 0
    for row, digit in enumerate(DIGITS):
        lag = 16.0 + row * DIGITS_TRAIL_BY
        for side in "LR":
            fan = FINGERS_SPLAY_TO * FANS[digit] * SPLAY_SIGNS[side]
            splayed += compose(
                f'pose.bones["{side}_{digit}1"].rotation_quaternion',
                (0.0, 0.0, 1.0), lambda frame, f=fan, l=lag: f * envelope(frame, l))
            for count in (1, 2, 3):
                splayed += compose(
                    f'pose.bones["{side}_{digit}{count}"].rotation_quaternion',
                    (1.0, 0.0, 0.0),
                    lambda frame, l=lag: FINGERS_FLATTEN_BY * envelope(frame, l))
    print(f"  examine-hands authored over frames {first}..{last}: {touched} body bones, "
          f"{splayed} phalanx channels splayed")
    if touched < len(EXAMINE):
        refuse(f"only {touched} of the {len(EXAMINE)} examine turns have curves in the idle - "
               f"the moment would play half-posed")


def the_hands_stay_off_the_chest(rig, mesh, scene):
    """Refuses if the raised hands pass behind the chest's own front - the clip-into-the-jacket
    fault, measured instead of squinted at.

    At the window's peak, every vertex a hand or forearm drives must sit FORWARD of the chest's
    forwardmost surface, along the direction he faces. Reported as centimetres of daylight, so
    a pass says how much room there is and not just that there is some.
    """
    groups = {g.index: g.name for g in mesh.vertex_groups}
    def owner(vertex):
        best, who = 0.0, ""
        for group in vertex.groups:
            if group.weight > best:
                best, who = group.weight, groups.get(group.group, "")
        return who

    peak = (EXAMINES_AT[0] + EXAMINES_AT[1]) // 2
    scene.frame_set(peak)
    bpy.context.view_layer.update()
    posed = mesh.evaluated_get(bpy.context.evaluated_depsgraph_get())

    # Which way he faces, from the toes - the same measured answer render_clay uses.
    toe = rig.pose.bones["L_ToeBase"]
    forward = (rig.matrix_world @ toe.tail) - (rig.matrix_world @ toe.head)
    forward.z = 0.0
    forward.normalize()

    chest, hands = [], []
    for vertex in posed.data.vertices:
        who = owner(mesh.data.vertices[vertex.index])
        spot = (posed.matrix_world @ vertex.co).dot(forward)
        if who in ("Spine01", "Spine02", "Waist"):
            chest.append(spot)
        # HANDS and digits only. The first version included the forearms, and an elbow held at
        # the ribs is LEGITIMATELY behind the chest's front plane - the guard refused anatomy.
        # What must stay forward of the chest is what is held up to be looked at.
        elif "Hand" in who or any(f"_{d}" in who for d in DIGITS):
            hands.append((spot, vertex.index, who))
    front = max(chest)
    hands.sort()
    nearest, index, who = hands[0]
    clear = (nearest - front) * 170.0
    where = posed.matrix_world @ posed.data.vertices[index].co
    print(f"  at frame {peak} the rearmost hand vertex sits {clear:+.1f} cm forward of the "
          f"chest's front: owned by {who}, at {where.z * 170.0:.0f} cm up")
    if clear < 1.0:
        print(f"  *** the raised hands sit behind the chest front - LOOK at the peak render "
              f"before trusting either the pose or this number")


def travels(rig, clip, scene):
    """How far the body moves through one cycle, hips and feet separately.

    Two numbers, because they answer different questions. The HIPS moving is root motion, which
    a game either uses or strips. The planted FOOT sliding is how far the character covers when
    the clip is played in place, and that is what playback rate needs.
    """
    play(rig, clip)
    first, last = (int(round(v)) for v in clip.frame_range)
    scene.frame_set(first)
    bpy.context.view_layer.update()

    def at(name):
        return (rig.matrix_world @ rig.pose.bones[name].head).copy()

    began = {n: at(n) for n in ("Hip", "L_Foot", "R_Foot")}
    hips, feet = 0.0, {"L_Foot": 0.0, "R_Foot": 0.0}
    for frame in range(first, last + 1):
        scene.frame_set(frame)
        bpy.context.view_layer.update()
        hips = max(hips, (at("Hip") - began["Hip"]).length)
        for foot in feet:
            feet[foot] = max(feet[foot], (at(foot) - began[foot]).length)
    return hips, max(feet.values())


def main():
    bpy.ops.wm.read_factory_settings(use_empty=True)
    for stale in list(bpy.data.objects):
        bpy.data.objects.remove(stale, do_unlink=True)

    base_rig, base_mesh, skeleton = None, None, None
    wanted = {}
    for filename, called in DELIVERED:
        path = os.path.join(SOURCE, filename)
        if not os.path.exists(path):
            refuse(f"{path} is missing")
        before = set(bpy.data.objects)
        known = set(bpy.data.actions)
        bpy.ops.import_scene.gltf(filepath=path.replace("\\", "/"))
        fresh = [o for o in bpy.data.objects if o not in before]
        rig = rig_of(fresh)
        if rig is None:
            refuse(f"{filename} has no armature")
        clips = [a for a in bpy.data.actions if a not in known]
        if len(clips) != 1:
            refuse(f"{filename} carries {len(clips)} clips, and this expects exactly one")

        if base_rig is None:
            base_rig = rig
            base_mesh = next(o for o in fresh if o.type == "MESH" and o.vertex_groups)
            skeleton = skeleton_of(rig)
            print(f"  {filename}: the base - {len(rig.data.bones)} bones, "
                  f"{len(base_mesh.data.vertices)} vertices")
            if CUT_THE_WEBBING:
                cut_the_webbing(rig, base_mesh)
            close_the_holes(rig, base_mesh)
            assigned = add_the_fingers(rig, base_mesh)
            if UNFUSES:
                # After the closer, never before it - see unfuse_the_digits on why.
                unfuse_the_digits(rig, base_mesh, assigned)
        else:
            the_skeletons_match(skeleton, skeleton_of(rig), filename)
            print(f"  {filename}: same skeleton, so its clip moves across unchanged")
            for thing in fresh:
                bpy.data.objects.remove(thing, do_unlink=True)

        clips[0].name = called
        clips[0].use_fake_user = True
        wanted[called] = clips[0]
        play(base_rig, clips[0])
        rolled = roll_the_hands(base_rig, clips[0], PALMS_ROLL_IN)
        if rolled:
            print(f"    rolled {rolled} hand(s) in by {PALMS_ROLL_IN:.0f} deg")

    # Anything else the imports brought in: spare meshes, the widget the importer invents.
    for thing in list(bpy.data.objects):
        if thing not in (base_rig, base_mesh):
            print(f"  dropped {thing.name} ({thing.type})")
            bpy.data.objects.remove(thing, do_unlink=True)
    for spare in [a for a in bpy.data.actions if a not in wanted.values()]:
        bpy.data.actions.remove(spare)

    scene = bpy.context.scene
    if base_rig.animation_data is None:
        base_rig.animation_data_create()

    for called, pieces in JOIN_INTO:
        have = [wanted[p] for p in pieces if p in wanted]
        if len(have) < 2:
            continue
        print("")
        print(f"  joining {' + '.join(pieces)} into one '{called}'")
        for piece in have:
            piece.name = f"{piece.name}_piece"
        made, frames = join_the_clips(base_rig, scene, have, called)
        for piece in have:
            wanted.pop(piece.name.replace("_piece", ""), None)
            bpy.data.actions.remove(piece)
        wanted[called] = made
        print(f"    {frames} frames, {frames / scene.render.fps:.2f} s, joins bent over "
              f"{JOIN_OVER} frames")
        if called == "idle" and EXAMINES:
            play(base_rig, made)
            examine_the_hands(base_rig, made, scene)
            the_hands_stay_off_the_chest(base_rig, base_mesh, scene)
    low = min((base_mesh.matrix_world @ v.co).z for v in base_mesh.data.vertices)
    high = max((base_mesh.matrix_world @ v.co).z for v in base_mesh.data.vertices)
    print(f"\n  a {(high - low) * 100:.1f} cm figure at scene scale")

    print("\n  clips, measured off the file:")
    for called in sorted(wanted):
        clip = wanted[called]
        first, last = clip.frame_range
        lasts = (last - first) / scene.render.fps
        hips, foot = travels(base_rig, clip, scene)
        # Named, and printed AFTER its own summary. It read the other way round, and a clip with
        # no travel prints no line at all - so every remaining line sat above the clip it was
        # about and the whole column looked shifted by one. It was not; it was unlabelled.
        print(f"    {called:<12s} frames {first:.0f}..{last:.0f}, {lasts:.4f} s at "
              f"{scene.render.fps} fps; hips travel {hips * 100:.1f} cm, "
              f"the furthest foot {foot * 100:.1f} cm")
        covers, who = stand_still(base_rig, clip, scene)
        if covers:
            after, _ = travels(base_rig, clip, scene)
            print(f"    {called:<12s} carried {covers:.4f} units on {who}; taken out, the root "
                  f"moves {after:.4f} -> COVERS = {covers:.4f}")
        elif called in TRAVELS:
            refuse(f"the {called} clip has no travel to take out, which means either it is "
                   f"already in place or the channel carrying it was not found")
        if called in ("walk", "run") and foot < 0.05:
            refuse(f"the {called} clip moves its feet {foot * 100:.1f} cm, which is not a "
                   f"gait - either the clip is empty or it is not driving the rig")

    play(base_rig, wanted["idle"])
    scene.frame_set(int(wanted["idle"].frame_range[0]))

    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.export_scene.gltf(
        filepath=OUT, export_format="GLB", use_selection=True, export_yup=True,
        export_apply=False, export_animations=True,
        # NOT resampled. The clips are authored at different rates - the walk's keys land on 24
        # fps and the run's on 30 - and the exporter's default is to bake every action at the
        # SCENE rate. Measured, that cost the run 25 degrees of loop accuracy on its own: its
        # opening and closing poses went from 22.19 degrees apart in the delivered file to 47.13
        # in the export, purely from being resampled onto a grid its keys do not sit on.
        export_force_sampling=False,
    )
    print(f"\nwrote {OUT}")


if __name__ == "__main__":
    main()
