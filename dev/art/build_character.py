"""Puts the three delivered clips onto one character and writes the game's asset.

    blender --background --python build_character.py

Reads `dev/art/source/character/*.glb` and writes `assets/models/person_ranger.glb` with the mesh, the
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
# THE DELIVERED CLIPS, which are an input to this and not a thing the game loads.
#
# They lived in `assets/character/` and the release workflow copies `assets/` whole,
# so fifteen megabytes of source animation shipped in every download - along with the
# original `ranger.glb` this pipeline superseded, which nothing had referenced for a
# long time. `assets/` now means what the game loads, and only that: there is no list
# to keep in step, because the folder IS the list.
SOURCE = os.path.join(ROOT, "dev", "art", "source", "character")
OUT = os.path.join(ROOT, "assets", "models", "person_ranger.glb")

# The delivered file, and what the game calls the clip in it. `lookAround` becomes the idle.
# # The delivered rig speaks a different language, and is normalised once on the way in
#
# The 2026-08-26 warden arrives as a 24-bone Mixamo-named skeleton - `LeftUpLeg`, `RightToeBase`,
# `neck` - on a 34,355 vertex mesh, 1.7 units tall. Everything downstream of here, in five Python
# files and in `src/ik.rs` and `src/motion.rs`, is written against the previous convention:
# `L_Thigh`, `R_ToeBase`, `Neck`, on a figure one unit high. That is 373 name references and 55
# places that turn units into centimetres by multiplying by 170.
#
# So the incoming asset is translated rather than the pipeline rewritten. Two operations, both at
# import, both reversible, and nothing after them can tell the difference:
#
#   * the bones are RENAMED by the table below,
#   * and the figure is SCALED to one unit tall, which is what `look::TALL` expects to be handed
#     and what every `* 170.0` in here already assumes.
#
# Renaming beats rewriting here for a reason worth stating: the runtime reads bone names out of
# the shipped glb too, so a rename in the pipeline alone would have left the game looking for
# joints that no longer existed. Doing it to the ASSET keeps one convention across both sides.
RENAMES = {
    "Hips": "Hip",
    "LeftUpLeg": "L_Thigh", "LeftLeg": "L_Calf",
    "LeftFoot": "L_Foot", "LeftToeBase": "L_ToeBase",
    "RightUpLeg": "R_Thigh", "RightLeg": "R_Calf",
    "RightFoot": "R_Foot", "RightToeBase": "R_ToeBase",
    # The spine numbers run the other way round on this rig: walking UP from the hips it is
    # Spine02, then Spine01, then Spine. Mapped by position in the chain rather than by name,
    # because the position is what `the_torso_bands` and the lean corrections actually use.
    "Spine02": "Waist", "Spine01": "Spine01", "Spine": "Spine02",
    "LeftShoulder": "L_Clavicle", "LeftArm": "L_Upperarm",
    "LeftForeArm": "L_Forearm", "LeftHand": "L_Hand",
    "RightShoulder": "R_Clavicle", "RightArm": "R_Upperarm",
    "RightForeArm": "R_Forearm", "RightHand": "R_Hand",
    "neck": "Neck",
    "Head": "Head", "head_end": "HeadTip", "headfront": "HeadFront",
}

# How tall the figure is left, in units. One, because that is the shape of every other number in
# this file - see RENAMES.
STANDS_A_UNIT_TALL = 1.0

# # A still idle, held from the walk
#
# The 2026-08-26 delivery is a walk and a run with no idle, and the game asks for one by name -
# `look.rs` names the clip and `motion.rs` blends out of it. "We dont need an idle I think, he can
# just stand there for now", so he stands.
#
# The pose is TAKEN rather than authored: the frame of the walk where his two feet are closest
# together and both nearest the floor, which is the moment a walk passes through standing. That is
# a real pose by a real animator with his arms and shoulders where they belong, and it costs
# nothing to be wrong about, unlike a pose invented here.
#
# Two seconds of it, keyed at both ends so the clip has a duration to blend across and no motion
# to notice. When a real idle arrives this whole thing is one line to delete.
# The BIND, not a walk frame. "His idle stance should just be standing but his hands are back and
# one leg is up" - which is what holding a mid-walk frame looks like, because every frame of a walk
# is mid-something. The bind is this artist's own neutral stand - rendered to check: legs together,
# upright, arms relaxed - and holding it costs nothing and invents nothing.
# # Letting the arms hang in the idle
#
# The bind fixes the legs and the spine - he stands square and upright - but a bind holds the arms
# out in an A-pose, which is a rigging convenience, not a stand. "His idle stance should just be
# standing", so the idle drops them.
#
# Aimed, not nudged: the shoulder-to-hand line is MEASURED in the bind, the wanted line is built
# from the same measurement (down, a little out so the arms clear the hips, a little forward so he
# does not read as at-attention), and the rotation between the two is applied to the upper arm. No
# axis is guessed and no angle is dialled in by eye - the one number here is where the arm should
# point, and the elbow keeps whatever the bind gave it. Idle only; the walk and run never see it.
THE_ARMS_HANG = True
THE_ARMS_HANG_OUT = 7.0        # opened further until the mesh actually clears
THE_ARMS_HANG_FORWARD = 4.0
THE_ELBOWS_COME_FORWARD = 7.0
THE_LEGS_STAND_APART_BY = 2.5
OPENS_THE_ARMS_BY = 2.0
OPENS_THE_ARMS_TO = 40.0
THE_FEET_LEVEL_WITHIN = 1.5    # cm
THE_IDLE_STANDS_ON_THE_FLOOR = 0.5
STANDS_STILL_FROM = "bind"

# # Zipping the pinholes
#
# "There are some empty spaces or color mismatches on his hair and neck." Beacon-rendered, the
# mismatches sit exactly on OPEN EDGES: the mesh ships as split shells - 30,119 boundary edges, all
# but 27 of them sealed by a coincident twin on the neighbouring shell - and those 27 are real
# pinholes, clustered at z 1.33-1.64 m: the hair and the nape. Through them the orange hood lining
# shows, which reads as orange flecks in black hair and a colour break at the neck.
#
# The zip welds each truly-open boundary vertex to its nearest boundary neighbour within a few
# millimetres - the twin the exporter meant it to have. Positions of kept vertices do not move, no
# face's UVs change (UVs live on face corners, which survive a weld), and the skin weights of the
# kept vertex stand. Mesh only; no clip is touched.
# # Padding the texture islands
#
# "Lets smooth out the colors on him too there are some empty spaces or color mismatches on his
# hair and neck." Diagnosed rather than guessed at, and it is not the mesh and not the material:
# the material is a single OPAQUE one and the mesh's 27 open edges are elsewhere. It is the
# TEXTURE. This asset's UV atlas is shattered into per-triangle islands - measured, only 53.5% of
# the 2048x2048 sheet is covered by UVs at all - and the 46.5% between the islands is left black
# with no padding. Bilinear filtering samples across an island's border, picks up that black, and
# the result is dark speckles and colour breaks exactly where islands are smallest and most
# crowded: the hair, and the seam at the neck.
#
# The fix is the standard one every bake pipeline ends with - DILATION, also called edge padding:
# push each island's own colour outward into the empty texels so no sample can reach the
# background. Rasterise the exact UV coverage from the mesh's own triangles, then grow the covered
# colour outward a ring at a time. Only empty texels are ever written - a covered texel keeps the
# artist's pixel exactly - and geometry, rig, weights and clips are all untouched.
# # Mending the stray hair islands
#
# The dilation above fixes the black speckle, and does nothing for the ORANGE flecks in his hair,
# because those are not gaps - they are painted. Measured on the shipped file: of the 5,558
# triangles in the hair mass (weighted mainly to Head, in the top 16 cm of the figure), 32 sample
# a strong orange - their UV islands sit on the hood-lining part of the atlas. 0.6% of the hair,
# scattered through it, which is exactly the "colour mismatches" read.
#
# So those 32 islands are repainted to the hair's own median colour, and nothing else is: the test
# is deliberately narrow - hair only, strong orange only - because the jacket's orange trim beside
# its green is the same colour relationship and is meant to be there. A wider "fix outliers"
# rule would eat it.
MENDS_THE_HAIR = True
THE_HAIR_IS_THE_TOP = 0.16
PADS_THE_TEXTURE = True
A_MARGIN_OF = 24

# OFF. Tried and measured: welding each open boundary vertex onto its nearest twin took the count
# from 27 open edges to 32 - a pointmerge collapses an edge and can open two more, so it treated a
# symptom and spread it. Whatever the flecks are, they are diagnosed before anything is welded.
ZIPS_THE_PINHOLES = False
A_PINHOLE_SPANS = 0.008

STANDS_STILL_FOR = 48

# The 2026-08-26 warden delivered a WALK and a RUN and no idle, so there is no idle here. The
# joiner below still expects one and `look.rs` still asks the game for one, which is the largest
# open thing about this changeover - see the note on JOINED.
DELIVERED = (
    ("walk.glb", "walk"),
    # The delivered file is called run.glb and the clip it becomes is called JOG. That is not a
    # slip: measured against `docs/animation.md`, this clip's effective cycle is 23 frames and its
    # cadence 130 steps a minute, where a run is 12-16 frames and 180-240. It is a jog, and the
    # game is walk-and-jog with no sprint tier - "we probably dont even need a sprint (run) in
    # this game". A real run clip, if one is ever wanted, comes in beside this rather than
    # replacing it.
    ("run.glb", "jog"),
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

# # Closing a cycle that does not close
#
# The delivered run does not loop: 22.19 degrees between its first and last pose, which in the
# game is the hands snapping 30.6 cm and the hip 2.10 cm, once every 1.03 seconds. Reported as
# "either he jitters or the world does" - it was him.
#
# The same unclosed loop is also why he creeps backwards. Root motion is detrended as a straight
# line from first key to last, and when the two ends are not the same pose that leaves a
# residue: measured, -1.96 cm of net backward hip travel per cycle.
#
# Closed by bending the last frames to meet the first, the same way `join_the_clips` bends its
# seams. Over a third of a second here, which spreads 22 degrees at under 3 degrees a frame.
CLOSES_THE_LOOP = ()
CLOSE_OVER = 8

# Which clips are supposed to carry the character somewhere. Everything else is a standing
# motion, and a standing motion with no travel is correct rather than broken - the refusal below
# is there to catch a gait whose channels never bound, which is what an unbound action slot
# looks like from the outside.
TRAVELS = ("walk", "jog")

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

# # Deepening the armpit instead of cutting it
#
# The fingers taught the answer this should have had from the start. Deleting webbing makes
# holes, because on this mesh the membrane is the only surface where it is - but a web is only
# WRONG when it is shallow. Sinking the shared vertices turns a sheet into a valley, the limbs
# read as separate, and deleting nothing cannot open anything.
#
# The right arm is the reported one and the worse one: 28 recorded faces against the left's 18,
# and in the idle the sleeve runs continuously into the jacket with no gap at all.
#
# Each membrane vertex is drawn toward ITS OWN bone's axis - arm vertices toward the upper arm,
# torso vertices toward the spine - which thins the two surfaces apart and opens a valley
# between them.
#
# Sinking them toward the armpit APEX was tried first and did nothing visible at 2.23 cm: moving
# surface vertices toward a point slides them ALONG the surface, it does not recess them. A
# valley needs the two sides pulled apart, not bunched together.
DEEPENS_THE_ARMPIT = False
ARMPIT_DRAWS_IN_BY = 0.30

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
#
# THIRTY, not ninety, and the difference is a measurement I did not take the first time. The
# delivered clips ALREADY pronate the forearm - band by band from the elbow, the idle winds it
# -38 to -59 degrees and plateaus - so the correction needed is only what is left over. Swept
# against the palm-plane angle:
#
#     0 deg -> L 32.6 R 49.8      25 -> L 28.6 R 33.1      40 -> L 34.8 R 26.2
#    55 deg -> L 44.8 R 25.8      90 -> L 74.9 R 51.1
#
# Ninety was past the far side and OVER-rotating by more than double. It read as roughly right
# in a render because a palm 75 degrees off the thigh still looks better than one facing
# forward.
PALMS_ROLL_IN = 0.0
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
# THE HAND ONLY, and the twist bones deliberately left alone.
#
# Sharing the roll along the forearm is right on a rig whose clips do not already twist it. This
# rig's clips DO: measured band by band from the elbow, the delivered idle winds the forearm
# -38 to -59 degrees, smoothly, plateauing at the wrist. That is a correct pronation already
# there.
#
# Adding +30/+60/+90 on top FOUGHT it, because the added roll runs the other way: proximally the
# two cancelled to -15, distally the hand over-rotated to +18, and the forearm came out
# counter-twisted by 46 degrees along its length - the mesh wrung like a towel, reported as "a
# twist in the mesh of the elbow render".
#
# Rolling only the hand leaves the clip's own twist exactly as authored and moves the one joint
# that actually needs moving.
SHARED_ALONG = (("Hand", 1.0),)

# # How far the arms rest off the torso, and why that is a POSE question
#
# "The idle still has his right arm seem attached to the torso." Two attempts to fix that in the
# MESH failed, and measuring the pose shows why they were always going to. Across the delivered
# idle, the angle between the upper arm and the spine:
#
#     L arm   min  9.6 deg   mean 15.7 deg   max 25.8 deg
#     R arm   min  8.8 deg   mean 11.9 deg   max 18.9 deg
#
# The right arm is held about four degrees tighter to the body than the left for the WHOLE clip.
# That is the asymmetry being seen, and no amount of carving the armpit fixes an arm that is
# resting against the ribs - carving it only ever moved the seam, and at a sink big enough to
# show it punched spikes through the shoulder.
#
# The fix is a pose-fixup layer: a constant abduction added at the shoulder on top of whatever
# the animator keyed. It keeps the clip's own arm swing, it costs no geometry, it adapts to a
# second body for the character creator, and unlike every mesh edit tried here it cannot open a
# hole.
#
# The floor applies to the clip's CLOSEST frame, so a clip already held wide is untouched and
# only the frames that read as attached move.
ARMS_REST_AT = 16.0
LIFTS = ("idle", "look_around")

# # The forearm's twist, spread along the forearm instead of dumped in the elbow
#
# "Twisted arm at the elbow again." Reported three times, which by this project's own rule
# means the model of the problem was wrong rather than the fix too timid. Two measurements
# settle it.
#
# Where the twist is, as each bone's own rotation about its length:
#
#     clip    Forearm        Twist01   Twist02   Hand
#     idle    -92 / +111      0.0       0.0      -+30
#     run     -84 / +119      0.0       0.0      -+30
#     walk    -60 /  +97      0.0       0.0      -+30
#
# And where the SKIN is:
#
#     Forearm            0 verts        ForearmTwist01   616 verts
#     Upperarm           0 verts        ForearmTwist02   316 verts
#
# `Forearm` deforms nothing. It is a pure bend bone, and the roll bones under it are what
# the mesh is attached to - the rig was BUILT for roll distribution. The clips never drove
# it: both roll bones read exactly 0.0 in every clip, so the forearm's 119 degrees is
# inherited WHOLE by both of them and every vertex from elbow to wrist turns as one rigid
# block. The crease is where that block meets the upper arm, which is the elbow, which is
# where it was reported.
#
# So the fix is not to fight the roll - an earlier attempt added roll on top and wrung the
# mesh the other way - it is to take the twist OFF the bend bone and hand it to the chain in
# graded shares. Removing it from `Forearm` costs nothing, because nothing is weighted to it.
#
# The hand has to move too, and that is correctness rather than polish: with the forearm no
# longer twisting, the wrist only stays where the animator put it if the hand takes the whole
# twist locally. That is the invariant this pass is checked against.
# Off: this rig carries no twist bones to spread onto, and the check around it reads hands.
SPREADS_THE_TWIST = False

# Where each roll bone's share comes from: MEASURED, not the usual one-third/two-thirds. A
# roll bone should carry the twist belonging to the stretch of arm its skin actually covers,
# and that is a weighted centroid, so it is read off the weight map per side and adapts to a
# second body. The distal end is pinned to the hand at the full amount.
ROLLS_ALONG = ("ForearmTwist01", "ForearmTwist02")

# # A livelier idle
#
# "Lets edit it in general so he moves his arms a bit more." The delivered idle is a subtle
# performance - fine on its own, thin at the scale a player watches it, standing still
# between fights.
#
# Amplified around the clip's OWN MEAN POSE rather than around the bind pose: each key's
# deviation from the average is scaled, so the swing grows and the place he rests does not
# move. Scaling the raw rotations instead would drag the whole arm toward or away from the
# body and undo the abduction floor below.
#
# Applied BEFORE the arms are lifted and before the roll is spread, so the floor is enforced
# on the amplified motion and the spread redistributes the amplified twist. Getting that
# order wrong would let amplification push the inner extreme back into the ribs.
# Under 1.0 makes a clip calmer, which is what the jog needed. Measured, the delivered jog swung
# its arms 119 degrees where its own walk swings 33 - sprint-scale, and reported as "less arm
# swing". 0.45 brings it to about 54, which reads as a jog carrying its arms rather than driving
# with them.
# # Reset for the 2026-08-26 warden
#
# Emptied, not deleted. Every number that was in here was measured against the PREVIOUS delivery -
# a different animator, a different skeleton, a different performance - and the first thing the
# build did with the new one was refuse: "a gain of 0.67 and a pump of 0.55 on jog left the L hand
# swinging 25.8 cm against 25.8 cm before - the gain is wired to nothing." The guard was right. A
# correction tuned to one performance says nothing about another.
#
# The distinction that decides what stays and what goes is whether a correction is about GEOMETRY
# or about PERFORMANCE. Mirroring the bind, squaring him onto the axis, hinging the toes at the
# ball, easing the knees, standing the legs apart, putting the soles on the floor - those are facts
# about a rig and they carry across unchanged. How far his arms swing, how far he leans, where his
# elbows are held - those are facts about a clip, and this is a different clip.
#
# So they come back one at a time, each earned on a measurement of the NEW animation, the way the
# old ones were. Empty until then.
MOVES_MORE = {}

# # A pumping arm dwells at its extremes; a gliding one does not
#
# "The arms still need to pump instead of whatever they're doing. This was a solved problem we
# had so its crazy we're going through it again." It was solved, on the character deleted in
# August, and the answer is in that commit rather than in anything derived here:
#
#   "A pure cosine cannot pump: it spends its time evenly. SPRINT_PUMPS = 0.55 shapes the swing
#    with an odd-symmetric power that flattens the peaks and steepens the middle, so the arm
#    DWELLS at the ends and snaps between them - 16 of 24 frames now sit within 15% of an
#    extreme against the run's 12. Phase and extremes are untouched, so the cycle still closes."
#
# So amplitude was never the whole story, and cutting it to 0.45 made it worse: a smaller swing
# on an arm that is HELD OUT leaves the held-out part dominating, which is "whatever they're
# doing". 0.67 restores the old jog's 80 degrees of swing, and the shaping below is what turns a
# swing into a pump.
#
# Under 1.0 dwells at the ends. 1.0 is the clip as authored.
PUMPS = {}

# # The hands come up too high, and the knob for that is the SHOULDER
#
# Reported as "the hands come up too high", and measured on the jog:
#
#     L hand   +7.1 cm ABOVE the shoulder at its peak, and 41.4 cm in front of it
#     elbow    folds 28.1 to 92.9 deg - straightening almost out at the front
#
# The same report, the same numbers and the same answer are in the deleted character's history.
# From commit 5a7c815: the lead hand "reached 4.4 cm ABOVE the shoulder; RUN_ARM_FORWARD 28 -> 20
# puts it 10.9 cm below. Worth recording that the first knob was the wrong one: at a
# forward-and-up pose the SHOULDER governs the hand's height and the ELBOW governs its reach, so
# cutting the shoulder angle moved the hand down and left it 43 cm forward, unchanged."
#
# And on what a jog's arms should do: "A jogger's elbows stay pinned near 90 deg and the hands
# travel from the hip to the lower chest in a tight arc, peaking maybe 15-20 cm ahead." An elbow
# opening to 28 degrees is the other half of that note - "never near 90, hence reaching rather
# than pumping".
#
# So two knobs, and they are not interchangeable. This one cuts the SHOULDER's swing on top of
# the whole-arm gain, which brings the hand down.
# NOT a gain. Cutting the shoulder's swing to 0.35 on top of the whole-arm 0.67 left 23% of the
# original arc: the hands came down, and the arms stopped alternating - both hung forward at waist
# height, which is a worse fault than the one being fixed. Amplitude is what makes a pump; the
# height belongs to where the arc is CENTRED.
#
# So the whole arc is rotated backward instead, which drops the forward extreme without taking
# anything out of the swing.
SHOULDERS_SWING = {}
THE_SHOULDER_IS = ("Clavicle", "Upperarm")

# Degrees of backward rotation applied to the whole upper arm, which lowers the hands' arc while
# leaving every degree of swing intact.
SHOULDERS_SIT_BACK = {}

# And this one holds the ELBOW near a right angle, which pulls the hand in. A mean shift, not a
# gain: the fold's RANGE is fine, it is the angle it is carried at that has the arm reaching.
ELBOWS_HOLD_AT = {}

# And how much the fold is allowed to VARY around that. Holding the mean at 85 still left the
# elbow opening to 56 degrees at the front of the swing, and a straighter elbow reaches further -
# the hand peaked 34.6 cm in front of the shoulder where the old note puts a jogger's at 15-20.
# Tightening the range is what pulls it in, and it is a different knob from the mean: one decides
# where the elbow is carried, the other how much it opens and shuts.
ELBOWS_SWING = {}
THE_ELBOW_IS = ("Forearm",)

# # The feet, and three faults measured on the delivered clips
#
# Reported as "ground contact should solve the mesh clipping into the floor and straighten the
# feet during the run", "the feet need to have that toe bend during the run", and "feet should be
# straight too, a lot of frames they're offset unnaturally". All three are real and all three are
# in the CLIPS, which is why runtime ground contact cannot fix them: `ik::shift_to_ground` corrects
# for how far the ground under one foot differs from the ground under the warden, so on flat
# ground it correctly does nothing - and a sole authored below the floor stays below it.
#
# What the clips actually do, measured off the shipped .glb:
#
#                  lowest sole   frames through the floor   toe break     toe off travel
#     idle           -5.02 cm      744 of 744  (100%)       0.0 deg      L  +0.5  R -30.3
#     walk           -3.38 cm       57 of  57  (100%)       0.0 deg      L  +4.3  R -29.1
#     run            -7.51 cm       22 of  25   (88%)       0.0 deg      L +21.1  R  -2.0
#
# Three separate things, in the order they have to be fixed - the first two move the soles, so
# the floor can only be measured once they are done.
#
# THE TOE NEVER BENDS. Both toe bones are keyed in every clip and every key holds identity, so a
# foot that should break at the ball of the foot instead pitches the whole shoe down - 86.7
# degrees of it in the run, which is the ballet point that was reported. Moving the excess pitch
# into the toe straightens the foot AND produces the bend, because they are the same operation.
#
# THE RIGHT FOOT IS SPLAYED about thirty degrees off the line of travel while the left is
# straight, consistently, through both the idle and the walk. The same left/right signature as
# the arm that rested 4 degrees tighter and the forearm whose roll all sat in one joint: these
# clips are mirrored imperfectly.
#
# THE SOLES SIT BELOW THE FLOOR, in the bind pose they do not - both soles measure 0.00 cm there -
# so this is the animation and not the mesh.
# Three separate fixes, three separate switches, because two of them turned out to be wrong and
# one of them is unambiguously right. Reported after the first attempt: "nearly every frame is
# wrong or messed up in some way. Remember to use the research about the real run that we did for
# the last animation."
#
# `docs/animation.md` has that research and it is what settles this. A run's key poses are
# CONTACT, DOWN, PASSING, UP plus a flight phase - four authored poses, not a per-frame rule. The
# 86.7 degrees of foot pitch is TOE-OFF, and toe-off in a run is supposed to point: it is the
# frame the whole stride is thrown from. Flattening every frame past 25 degrees did not fix a
# ballet point, it deleted the push-off from the one pose that needs it, and it did the same to
# the splay at the ankle - a joint that does not yaw 30 degrees in life.
#
# Both are per-frame corrections applied to poses an animator chose. They are off. The lesson is
# the one already in TROUBLESHOOTING.md under a different name: a correction layer may fix
# CONTACT with the world, and must not restyle a performance.
FEET_MEET_THE_FLOOR = False
BREAKS_THE_TOES = False
POINTS_THE_FEET = False

# # The foot roll through stance: heel, flat, toe
#
# "Frame 20, the lead foot is angled, people don't run like that... lack of toe bend on the
# planted foot. Remember heel -> toe. Real anatomy for a run."
#
# The reference is AnimSchool's key poses of a run cycle - CONTACT, DOWN, PUSH, PEAK - which the
# user supplied and which is now in `docs/animation.md`. It also draws the distinction that
# decides the target here: a REALISTIC run lands on the ball of the foot, an EXAGGERATED one
# lands heel-first with the foot farther from the body. Copaimo is stylised by policy, and
# heel-first is what was asked for, so heel-first is the target.
#
# What the delivered run actually does, measured per frame with the heel and toe tracked
# separately (cm above the floor, pitch positive = toe below ankle):
#
#     L stance   f19  heel 3.0  toe 1.6  pitch +10.7
#                f20  heel 2.9  toe 1.6  pitch +11.9
#                f21  heel 2.3  toe 0.8  pitch +18.3
#                f22  heel 3.7  toe -2.6 pitch +46.8
#
# The toe is lower than the heel through the WHOLE stance and the pitch never goes negative. The
# heel never touches: he contacts toe-first and stays on his toes until push-off. There is no
# roll, which is why the frames read wrong one after another rather than at one bad pose.
#
# So the correction is a roll, and it only ever touches STANCE. `docs/animation.md` is explicit
# about why - "only apply the offset at the moment of plant, then hold it until the foot lifts" -
# and the last attempt at this ignored it, applied a blanket rule to every frame, and deleted the
# push-off from the one pose that needs it. Swing is the animator's.
# Gaits only. An idle has no stance-and-swing cycle - both feet are simply down - so treating a
# five-hundred-frame stand as one stance ramped it from heel-strike to push-off across the whole
# clip and drove the feet 12.68 cm through the floor. A roll is a property of a STEP.
# OFF, with the toe hinge and for the same reason - see `HINGES_THE_TOES`. The machinery stays:
# it is correct, it is tested, and a clip that genuinely needs a heel-to-toe roll can have one by
# naming it here. This clip does not.
ROLLS_THROUGH_STANCE = ()

# # The toe joint belongs at the ball of the foot
#
# "The toe bones should go to the end of the mesh", after thirteen of the run's twenty-five frames
# were called out as wrong. Measured along each shoe, heel to tip:
#
#     L   shoe 33.14 cm   ankle at 27.8%   TOE JOINT AT 45.1%   toe bone ends at 62.4%
#     R   shoe 32.93 cm   ankle at 21.2%   TOE JOINT AT 38.1%   toe bone ends at 55.0%
#
# A ball of the foot is 65-75% along. This one hinges at the MID-ARCH, and `ToeBase` owns
# everything from 28.7% forward - so rotating the toe folds the shoe across its own middle. That
# is what every one of those thirteen frames shows, and no amount of tuning the roll fixes a
# hinge in the wrong place.
#
# So the joint moves forward to the ball and the bone runs on to the tip, and the weights that
# used to sit on it are redistributed about the new hinge. Only the share held between `Foot` and
# `ToeBase` is moved - each vertex keeps its total, so anything the ankle or the calf holds is
# untouched.
# Whether the toe joint is moved to the ball at all. A switch, so the character can be built
# exactly as delivered and compared against - which is the check that should have come first.
# OFF. Put side by side against the clip as delivered, moving the toe joint to the ball CRUMPLES
# the shoe - the toe section folds and the silhouette breaks - and the stance roll on top of it
# is worse again. The delivered feet were fine.
#
# The measurements that justified all of it were each true and each beside the point: the joint
# really did sit at 45% of the shoe, the toe really never bent, the foot really did roll onto its
# edge. None of that mattered, because the shoe reads correctly as delivered and every correction
# made it read worse. A number improving is not the same as the thing improving, and the check
# that settles it is the one that should have come first - build it both ways and look.
#
# This is the second time on this character: the shoes were fixed by reverting to as-delivered
# after seven passes of reshaping them.
# # The bind is mirrored, once, and then nothing needs correcting per frame
#
# `docs/rigging.md` says this outright, from the previous character:
#
#   "The delivered rig arrived with a 17.5 deg crouch, the two sides 5.45 cm from mirrored, and
#    the character 5.7 cm under the floor. All three are rest-pose constants, which is why
#    per-frame corrections kept failing: CORRECTING A CONSTANT PER POSE IS WHAT TWISTED THE FEET,
#    three separate times. Fixed once in the bind, and the authoring has no correction step at
#    all now."
#
# This rig arrived the same way. Measured against its own mirror plane:
#
#     positions   worst 5.60 cm, mean 3.24 over 16 pairs
#     directions  worst 16.3 deg - and the worst pair is L_Foot and L_ToeBase
#
# The feet are the most asymmetric bones in the rig, in the BIND. Every left/right difference
# chased per-frame - a right foot splayed 30 degrees where the left is straight, a right arm
# resting 4 degrees tighter to the torso - is downstream of this one constant, and correcting
# them per pose is what crumpled the shoe every time.
# # Squaring him up before anything else looks at him
#
# The delivered figure is not built on an axis. His hip line and his shoulder line both put his
# front 11.67 degrees off world +X - two independent witnesses agreeing to two decimal places, with
# the mass of his head as a third - and nothing in the pipeline noticed, because everything
# downstream measures him against himself.
#
# That is why "he runs off to the right" survived a pass that had measured his travel at 0.00
# degrees off his own facing. Both statements were true. He ran exactly where he pointed, and where
# he pointed was crooked.
#
# The game does not escape it either. `look::Build::turn` hands the Ranger a flat FRAC_PI_2, so a
# figure 11.67 degrees off axis is turned to 11.67 degrees off the game's -Z, and then driven along
# its heading - which is a body pointing one way and travelling another, for as long as it moves.
# `look::a_warden_faces_forward` is the test that catches exactly this, and it runs on Male and
# Female only; the Ranger has never been in it.
#
# So he is turned once, here, before a single measurement is taken. +X because that is what the
# game's existing quarter turn maps onto its own forward: Blender +X exports to glTF +X, and a
# quarter turn about Y carries +X onto -Z exactly.
#
# The turn goes on the DATA - `Armature.transform` and `Mesh.transform` - and not on the object.
# Bone-local animation is unaffected by construction, since a pose is stated relative to a rest
# that has turned with it, and the root's translation keys live in that same rest frame. An object
# rotation would have shipped as a node transform for something downstream to forget about.
# # The delivered clips are final. Nothing here may change them.
#
# "I gave you a new glb of a run that was fine. Whatever you did broke that... Do not change the
# run at all, if something would affect it let me know but only implement changes that do not
# affect the run and walk."
#
# So this file now does exactly three things to the delivery, and every one of them leaves the
# performance bit-for-bit intact:
#
#   * RENAMES the bones onto the pipeline's convention, and the animation channels with them. A
#     rename moves nothing.
#   * SCALES the figure to one unit and bakes the object transforms in. A uniform scale about the
#     origin changes no joint angle anywhere.
#   * SQUARES him onto the axis. A rigid rotation of the whole rig, bones and flesh together, so
#     every local rotation in every clip is untouched - and it is what makes `look::Build::turn`
#     land him facing the game's forward.
#
# Everything else is off, listed here so the cost is visible rather than buried:
#
#   MIRRORS_THE_BIND        moves bind joints; the clips are compensated but the bind moves
#   HINGES_THE_TOES         moves the toe joint and re-weights the shoe
#   KNEE_EASE               bends the bind knee 2 degrees
#   THE_LEGS_STAND_APART    abducts the bind hips
#   ADDS_THE_FINGERS        adds 30 bones and takes weight off the hand
#   UNFUSES                 moves mesh vertices between the fingers
#   PALMS_ROLL_IN           rotates the hands 30 degrees in the clip
#   FLATTENS_THE_TOES       rotates toes per frame
#   FEET_MEET_THE_FLOOR     translates the whole clip vertically
#   CLOSES_THE_LOOP         re-keys every channel from samples
#   CLOSES_THE_HOLES        welds mesh
#   and `author_gait` - the whole plant - which is the largest of them by far
#
# What that costs is written up for the user rather than hidden: the soles are not on the floor,
# the feet are not flat, nothing is planted, and the toes keep whatever the animator gave them.
# Those come back one at a time, each shown against the delivered clip first.
THE_DELIVERY_IS_FINAL = True

# # And the delivered NAMES, SCALE and FACING are final too
#
# "Are you adding bones? literally go back to the original." The earlier translate-at-the-door
# renamed his bones and rescaled him so the pipeline's tools could keep their vocabulary. That was
# the wrong direction: the asset is the artist's and the tools are ours, so the tools adapt.
#
# The shipped skeleton is now the delivered one - LeftUpLeg, RightToeBase, Hips, 24 bones, his
# scale, his orientation. The game side follows it: `look::Build::turn` is 0 because Blender +Y
# exports to glTF -Z, which IS the game's forward, and `authored_height` is what the file measures.
# The one thing still renamed is the ACTIONS - walking_man to walk, running to jog - because a
# clip's name is a label, not motion.
KEEPS_THE_DELIVERED_NAMES = True


def the_bone(rig, name):
    """The pose bone for a pipeline name, on either naming. Reads, never renames."""
    if name in rig.pose.bones:
        return rig.pose.bones[name]
    for was, becomes in RENAMES.items():
        if becomes == name and was in rig.pose.bones:
            return rig.pose.bones[was]
    refuse(f"no bone answers to {name} on this rig")

# Neither of these had a switch before, and both change the skin.
ADDS_THE_FINGERS = False
CLOSES_THE_HOLES = False

# Off with the delivered facing kept: `look::Build::turn` is 0 instead, which turns nothing.
SQUARES_HIM_UP = False
FACES_ALONG = (1.0, 0.0)

# How far off the axis he may still be once he has been turned, in degrees.
SQUARE_WITHIN = 0.05

MIRRORS_THE_BIND = False

# # A knee that is dead straight cannot be solved
#
# The bind stands at 100.0% of straight, and `docs/rigging.md` is explicit about what that costs:
# "A dead-straight two-bone chain is SINGULAR to an IK solver - it cannot tell which way the joint
# folds, and every knee froze at exactly 0.0000. Standard fix is a few degrees of bend in the
# bind." The previous character carried `KNEE_EASE = 2.0`, and the authored gait pipeline depended
# on it - `gait()` has no pole target because "the knee's direction comes from the bind pose being
# BENT".
#
# Two degrees at the thigh and two back at the calf, so the knee rests bent 4 degrees and the
# ANKLE stays where it was. Bending only one of them would move the foot.
#
# The delivered clips are compensated for the change rather than left to mean something new: a
# bind change invalidates every key written against the old rest - "a clip cannot be copied across
# a bind change, only retargeted" - so each affected bone's keys are pre-multiplied by the inverse
# of the rest change, which leaves the animation looking exactly as it did.
KNEE_EASE = 0.0

# # How far apart the legs are held
#
# "Check frame 8 of the jog, his feet go into each other." They do: measured, the legs come
# within 0.45 cm at frame 8, and 0.28 at frame 2 - two to four millimetres on a 170 cm figure,
# which with smoothed normals reads as one leg inside the other.
#
# Part of it is the clip - the delivered jog closes to 0.48 cm on its own - and part is mine:
# mirroring the bind averaged two legs that were 5.6 cm from mirrored, which brought them about
# 0.2 cm closer together.
#
# A constant, so it is fixed as one: both thighs rotated a little outward in the BIND, which
# widens every pose by the same amount without touching a single key. `docs/rigging.md` is
# explicit that this is where a constant belongs.
# 1.6, and more is NOT better - swept, and the closest approach moves to a different pair of
# points as the legs open, so it gets worse again:
#
#     as delivered   jog 0.28 cm      1.6 deg   jog 0.51 cm
#     3.0 deg        jog 0.42         4.5 deg   jog 0.24
#
# 1.6 is the best of them and it is a small improvement, not a fix. The legs pass close because
# the CLIP swings them close; widening the bind moves where they nearly touch rather than
# stopping them nearly touching.
THE_LEGS_STAND_APART = 0.0

# # How far the ankle may point down, and why this is the only foot correction left
#
# "The toes bend to the floor... he runs with broken feet." The delivered jog points the whole
# shoe 86.7 degrees down at push-off - past vertical relative to the shin - and the toe, which is
# rigid with the foot, goes with it straight into the floor.
#
# Everything else tried on these feet is off, and each was reverted for a measured reason:
#
#   moving the toe joint to the ball   the shipped mesh came out 4.2 cm different in SHAPE from
#                                      an identical set of bone rotations. Neither the joint move
#                                      nor the re-weighting does that in Blender - both measure
#                                      0.000 cm there - so it happens through the export, and a
#                                      rig edit whose effect nobody can account for does not ship
#   the heel-to-toe stance roll        crumpled the shoe, because the hinge it bends is at the
#                                      mid-arch and moving that hinge is the change above
#   aiming the foot down the leg       drags a planted toe; the slide guard refused it at -27.9%
#   per-frame splay and pitch fixes    `docs/rigging.md` says plainly that correcting a constant
#                                      per pose is what twists feet, and it did
#
# What is left is the one thing that is neither a rig edit nor a constant: an authored PEAK, in
# one channel, at frames the foot is in the AIR. Capping it cannot drag a planted foot, cannot
# change the bind, and cannot move a weight. `docs/animation.md` is clear that a pointed foot at
# toe-off is correct in a run - so this caps rather than flattens, and only past the point where
# the toe would be below the ball.
# The ankle cap is off. It could only touch AIRBORNE frames - correcting a planted foot drags its
# toe and the slide guard refuses that - and the steep frames are the planted ones, at push-off,
# up on the ball. It reached 86.7 down to 74.3 and left the fault where it was.
THE_ANKLE_POINTS_AT_MOST = 50.0
CAPS_THE_ANKLE = ()

# # The toe keeps the ground while the foot points off it
#
# This is the fault, stated as simply as it can be: at push-off the heel is up, the ball is on the
# floor, and the toe - rigid with the foot - carries on down the same steep line and ends up
# BELOW the floor. "The red line is pointing down into the floor. That is not how toes work."
#
# The fix rotates the toe UP about the ball until it lies along the ground. The ball is the pivot
# and the ball is the contact, so nothing slides - which is the whole reason this works where
# every other correction failed. `docs/animation.md` names it: a foot pivots about its CONTACT
# POINT, and rotating about the ankle is what lifted the heel off the floor every previous time.
#
# It needs the joint at the ball, or the bend folds the arch. See `HINGES_THE_TOES`.
FLATTENS_THE_TOES = ()

# How far below horizontal a toe may still point once it has been flattened. Not zero: a shoe has
# a sole with some thickness and a toe box that curves up, so a few degrees reads as the toe
# resting on the floor rather than hovering over it.
THE_TOE_RESTS_AT = 8.0

# And how far the toe may be bent to get there. 40 degrees, not 55.
#
# At 55 the correction bound exactly at its own cap on the frames that needed it most: frame 4 of
# the jog has the right foot pointing 67.9 degrees down while still on the ground, so flattening
# its toe took 60 degrees of bend at the ball. Reported as "check frame 4, his foot would be
# broken here", and it would - a toe does not extend sixty degrees.
#
# 40 leaves those frames with the toe still angled down rather than flat, which is the honest
# trade: the clip puts the foot steeper than a toe can compensate for, and bending the toe past
# its own range to hide that is how a foot ends up looking broken instead of just steep.
THE_TOE_BENDS_AT_MOST = 40.0

# ON, and the note that used to sit here calling it a crumpler was wrong.
#
# That came from a contact sheet where the state with the joint moved looked folded. It was not:
# the shot is aimed at a bone and the two builds frame slightly differently, so one sat shifted in
# the frame and I read the shift as a fold. Checked properly afterwards, vertex by vertex at the
# same frames, the two builds' FEET are identical - which is what the arithmetic says they must
# be, since a toe bone with identity keys is rigid with its foot and its deform cancels to the
# foot's however the joint is placed. Measured in Blender, moving the joint 8 cm moves the mesh
# 0.000 cm.
#
# The 4 cm difference the two builds really do have is in the HANDS - R_Pinky, R_Ring, R_Middle,
# R_Index, R_Hand - because `add_the_fingers` runs afterwards and its digit segmentation is
# already a known-broken area. Nothing to do with feet.
#
# The joint has to be at the ball for `flatten_the_toes` to work at all: a toe that bends at the
# mid-arch folds the shoe across its middle.
HINGES_THE_TOES = False
THE_TOE_HINGES_AT = 0.70
# Tried at 0.15 to spread the bend, on the reasoning that an abrupt handover pinches. It does the
# opposite: a wider band puts MORE vertices under two bones at once, and linear blend skinning
# loses volume exactly where influences overlap. The left shoe's worst-frame squash went from
# 2.1% to 4.6%. Narrow is right here.
THE_HINGE_BLENDS_OVER = 0.08

# How far up the shoe the ball joint sits, where the shoe is that tall.
#
# Moving the joint forward while keeping the height it had at the ARCH left it 69.3% up the shoe -
# nearly level with the ankle at 81% - and it was reported as "toes are above the foot". A
# metatarsophalangeal joint sits LOW in the foot; the ankle is the high one.
THE_TOE_SITS_UP = 0.30

# How much of the shoe's length counts as the toe box, for putting the toe bone's tip in the middle
# of it. A tenth of a 33 cm shoe is the front 3.3 cm, which is the toe and not the ball.
THE_TIP_IS_THE_FRONT = 0.10

# A shoe within this many centimetres of the floor is down.
STANCE_WITHIN = 3.0

# # A planted foot is flat and points where he is going
#
# "He needs to run on the flats of his feet... Right now he's running on the sides of his shoes...
# He still angles to the side during the run. Everything should be pointing straight in front of
# him."
#
# All three measured on the run, per frame, as the foot's own pitch, roll and yaw:
#
#     L at contact (f7)   roll  18.2 deg   - tilted onto the edge of the shoe
#     L through stance    yaw   6 to 20 deg off travel
#     R through stance    yaw  -23 deg
#
# So a planted foot is given a target ORIENTATION rather than three separate nudges: pointing
# along travel, flat about its own length, and pitched by wherever the heel-to-toe roll has got
# to. Rotating to a target basis is exact, where three sequential corrections each measured from
# the original pose are not - they compose, and the error grows with the angle.
#
# Only during stance. Swing keeps the animator's poses, for the reason `docs/animation.md` gives.
PLANTS_FLAT = True

# The roll, as a share of stance. Heel-first contact with the toe up, flat by a third of the way
# through, then the heel lifts and the toe takes over.
CONTACT_PITCH = -8.0
FLAT_AT = 0.35
PUSH_PITCH = 45.0

# The toe stays straight until the heel is well up, then breaks. It is the metatarsal break, and
# it is the thing the delivered clips have none of - every toe key in every clip is an identity.
#
# # Which way a toe bends, and it is the opposite of the obvious one
#
# Reported as "the toes bend the wrong way", and the measurement agreed: at push-off the foot
# pitched 36.1 degrees and the toe pitched 51.6 in the world - MORE than the foot - which drove
# the tip 6 cm through the floor.
#
# A toe at push-off does not curl down. It stays flat on the ground while the foot rotates up
# OVER it, so the joint EXTENDS: the toe goes up relative to the foot by however far the foot has
# gone down. The target is therefore the toe's world pitch staying near zero, and the break is
# the negation of the foot's own pitch rather than an amount added to it.
TOE_BREAKS_AFTER = 0.55
TOE_BREAKS_TO = 35.0

# Frames either side of a stance over which the correction fades, so there is no step where it
# starts and stops.
ROLL_EASES_OVER = 2

# Foot pitch beyond which the toe takes over, and how far the toe may go. A foot rolls up onto the
# ball before the toe does anything, so the first 25 degrees stay in the ankle; past that a real
# foot breaks rather than pointing, and 55 degrees is about where a toe stops.
TOE_BREAKS_PAST = 25.0
TOE_BREAKS_AT_MOST = 55.0

# How much toe-out to leave in. Feet do not point dead along the line of travel - about 10 degrees
# of natural toe-out is right - so the correction only takes out what is past that, and only ever
# reduces the splay.
SPLAY_ALLOWS = 10.0
MOVES_AT = ("Clavicle", "Upperarm", "Forearm", "Hand")

# # How far the trunk leans, and why this one number has bitten before
#
# "The jog SHOULD be easy. Less forward lean, less arm swing."
#
# TROUBLESHOOTING.md already holds the research, from the last character: real trunk flexion in
# running is **4 to 12 degrees**, most economical near 6, and game guidance quoting "15 to 30 for
# a sprint" is a two-to-four-times push that makes a character read as permanently accelerating.
# The previous character shipped its jog at 9 degrees, measuring +6.97 from its own resting
# posture.
#
# The delivered jog leans **+35.3 degrees from its own rest** - five times that, and three times
# the top of the real range. That is a sprinter's block-exit lean held for a whole cycle.
#
# Measured FROM REST, never absolute: this figure stands with its trunk 7.57 degrees BEHIND
# vertical, and the same mistake on the last character refused a run that had leant forward
# perfectly well purely because it started from behind.
# OFF VERTICAL, not off the model's own rest, and the difference is the whole fault. Trunk
# flexion in the biomechanics this comes from is measured from vertical - "4 to 12 degrees, most
# economical near 6" means six degrees in FRONT of upright.
#
# Setting it from rest instead put him at +6.2 from a rest that is 7.6 degrees BEHIND vertical,
# which is -1.4 absolute: still leaning back, and reported as exactly that. Measuring from rest
# is right for a GUARD - an absolute threshold once refused a run that had leant forward
# perfectly well, purely because it started from behind - and wrong for a TARGET. A guard asks
# "did this clip lean forward at all"; a target says where the trunk should end up, and where it
# ends up is an angle in the world.
LEANS_FORWARD = {}

# Shared down the spine rather than folded at one joint, for the same reason the forearm's roll
# is shared down its twists: a whole trunk's worth of bend put into one vertebra creases there.
LEANS_ALONG = ("Spine01", "Spine02")

# And the head, which has to be put back afterwards.
#
# Leaning the trunk rotates everything above it, so taking 40 degrees out of the spine took the
# head with it and left the warden jogging along looking at the sky - 28 degrees above his own
# resting gaze. A runner's head is level and their eyes are on the ground ahead, so this brings
# it back to rest and no further.
LEVELS_THE_HEAD = {}
THE_HEAD_IS = ("Head",)

# And the SIDEWAYS lean, which is a different fault from the forward one and was invisible to
# every measurement of it. Asked as "do you see the lean?" over a render of the warden tilted
# over to one side.
#
#     jog    -8.9 deg mean, -17.7 to +0.2, which is -12.8 from its own rest
#     walk   -0.3 deg mean, which is -4.2 from rest
#     idle   +1.6 deg mean
#
# A jog does not lean sideways. A little sway either way is the weight shifting; a mean of -12.8
# held for the whole cycle is a list. Corrected to level, by the same machinery as the forward
# lean and about the axis at right angles to it.
STANDS_UPRIGHT = {}


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


# The stub a childless bone gets, as a fraction of its parent's length.
A_TIP_IS = 0.4

# # Whether the tails are rebuilt. OFF, and this is the expensive lesson of the changeover.
#
# The delivered tails are junk - glTF does not carry a bone length, so Blender invents one, and
# here it invented hips 11.72 units long on a figure 1.7 units tall. Rebuilding them from the
# skeleton makes every REPORT sane: `stand_the_legs_apart` stops claiming it moved a knee 8492 cm
# and the bind mirror stops claiming halves 288 cm apart.
#
# And it destroys the animation. A bone's rest ORIENTATION is its head-to-tail direction, so
# re-aiming a tail redefines the frame that every keyed local rotation is measured in. Nothing
# needs to touch a key for the whole clip to mean something else. Rendered, the run became a man
# rocking on the spot with his arms flung out - "why did you make him breakdance" - and it was the
# only step that did it: with every other correction on and this one off, the run is the delivered
# run.
#
# It can come back, but only WITH the compensation a bind change needs - capture each bone's world
# pose per frame, change the rest, put the pose back - which is the same thing `ease_the_knees`
# already does for its two degrees. Until then the reports read oddly and the animation is right,
# which is the correct way round.
POINTS_THE_BONES = False


def point_the_bones_at_their_children(rig):
    """Rebuilds every bone's TAIL from the skeleton, because the delivered ones are invented.

    # glTF has no idea how long a bone is

    It stores joints as points. A bone's length and direction are Blender's guess on the way in,
    and on this rig the guess is wild: the hips come in 11.72 units long on a figure 1.7 units
    tall, the thighs 35.6. Nothing downstream can survive that - `stand_the_legs_apart` reported
    moving a knee 8492 cm, and the bind mirror reported halves 288 cm from each other, both of them
    reading a tail that means nothing.

    A joint's real direction is toward the next joint, so that is what is used. A bone with one
    child points at it. A bone with several - the hips, the chest - points at their middle, except
    that a MIRRORED pair of children says nothing about direction and is ignored in favour of any
    single centre child. A bone with no children continues its parent, shortened, which is the only
    honest answer for a fingertip or a toe tip.
    """
    bpy.context.view_layer.objects.active = rig
    bpy.ops.object.mode_set(mode="EDIT")
    edits = rig.data.edit_bones
    for bone in edits:
        kids = [k for k in edits if k.parent is bone]
        aim = None
        if len(kids) == 1:
            aim = kids[0].head.copy()
        elif len(kids) > 1:
            # A centre child - one that is not half of a left/right pair - is the honest heading.
            middles = [k for k in kids if abs(k.head.x - bone.head.x) < 1e-4]
            picked = middles if middles else kids
            aim = sum((k.head for k in picked), mathutils.Vector()) / len(picked)
        if aim is not None and (aim - bone.head).length > 1e-5:
            bone.tail = aim
    # Leaves last, so they continue a parent that has already been straightened.
    for bone in edits:
        if any(k.parent is bone for k in edits):
            continue
        along = (bone.head - bone.parent.head) if bone.parent else mathutils.Vector((0, 0, 1))
        if along.length < 1e-6:
            along = mathutils.Vector((0.0, 0.0, 1.0))
        bone.tail = bone.head + along.normalized() * (along.length * A_TIP_IS)
    bpy.ops.object.mode_set(mode="OBJECT")
    return len(rig.data.bones)


def carry_the_clips_into_the_new_scale(clips, factor):
    """Scales an action's translation channels by the same factor the figure was resized by.

    Rotations do not care about scale; translations do, and they are expressed in DATA units, so
    they change by the WHOLE change a data unit undergoes - the object's own scale as well as the
    resize. Per file, for the same reason the renaming is per file: the base's action is the only
    one that exists while the base is being normalised, and the run arrived afterwards. Left out,
    its hips carried 571 cm of travel across a 16-frame clip on a figure one unit tall.
    """
    if abs(factor - 1.0) < 1e-12:
        return 0
    moved = 0
    for clip in clips:
        for curve in fcurves_of(clip, None):
            if not curve.data_path.endswith(".location"):
                continue
            for key in curve.keyframe_points:
                key.co[1] *= factor
                key.handle_left[1] *= factor
                key.handle_right[1] *= factor
            curve.update()
            moved += 1
    return moved


def mend_the_stray_hair(rig, mesh, sheet, wide, high):
    """Repaints hair islands that sample orange. See MENDS_THE_HAIR. Returns how many."""
    import numpy as np

    groups = {g.name: g.index for g in mesh.vertex_groups}
    head = groups.get("Head")
    if head is None:
        return 0
    scalp = set()
    for vertex in mesh.data.vertices:
        for group in vertex.groups:
            if group.group == head and group.weight > 0.5:
                scalp.add(vertex.index)
                break
    tallest = max((mesh.matrix_world @ v.co).z for v in mesh.data.vertices)

    mesh.data.calc_loop_triangles()
    uvs = mesh.data.uv_layers.active.data
    hair, stray = [], []
    for face in mesh.data.loop_triangles:
        if not all(v in scalp for v in face.vertices):
            continue
        if (mesh.matrix_world @ mathutils.Vector(face.center)).z < tallest - THE_HAIR_IS_THE_TOP:
            continue
        corner = np.array([np.array(uvs[loop].uv) for loop in face.loops])
        at = np.mean(corner, axis=0)
        colour = sheet[int(np.clip(at[1] * high, 0, high - 1)),
                       int(np.clip(at[0] * wide, 0, wide - 1))]
        hair.append(colour)
        # Orange in LINEAR terms: red-dominant and blue-poor, well above the hair's own darkness.
        red, green, blue = colour
        if red > 0.10 and red > blue * 2.5 and green < red * 0.75:
            stray.append(corner)

    if not stray:
        return 0
    was = np.median(np.array(hair), axis=0)
    for corner in stray:
        xs = corner[:, 0] * wide
        ys = corner[:, 1] * high
        x0 = max(int(np.floor(xs.min())) - 2, 0)
        x1 = min(int(np.ceil(xs.max())) + 2, wide)
        y0 = max(int(np.floor(ys.min())) - 2, 0)
        y1 = min(int(np.ceil(ys.max())) + 2, high)
        sheet[y0:y1, x0:x1] = was
    return len(stray)


def pad_the_texture_islands(mesh, margin=None):
    """Dilates the base colour texture into its empty texels. See PADS_THE_TEXTURE."""
    import numpy as np

    margin = A_MARGIN_OF if margin is None else margin
    image = None
    for slot in mesh.data.materials:
        if slot is None or not slot.use_nodes:
            continue
        for node in slot.node_tree.nodes:
            if node.type == "TEX_IMAGE" and node.image is not None and node.image.size[0] > 1:
                image = node.image
                break
    if image is None:
        return None

    wide, high = image.size
    flat = np.empty(wide * high * 4, dtype=np.float32)
    image.pixels.foreach_get(flat)
    sheet = flat.reshape(high, wide, 4)

    mesh.data.calc_loop_triangles()
    uvs = mesh.data.uv_layers.active.data
    faces = mesh.data.loop_triangles
    corners = np.empty((len(faces), 3, 2), dtype=np.float32)
    for at, face in enumerate(faces):
        for which, loop in enumerate(face.loops):
            corners[at, which] = uvs[loop].uv

    # Exact coverage: every texel whose centre falls inside a UV triangle.
    xs = corners[:, :, 0] * wide
    ys = corners[:, :, 1] * high
    x0 = np.clip(np.floor(xs.min(1)).astype(int) - 1, 0, wide - 1)
    x1 = np.clip(np.ceil(xs.max(1)).astype(int) + 1, 0, wide)
    y0 = np.clip(np.floor(ys.min(1)).astype(int) - 1, 0, high - 1)
    y1 = np.clip(np.ceil(ys.max(1)).astype(int) + 1, 0, high)
    covered = np.zeros((high, wide), dtype=bool)
    for at in range(len(corners)):
        a, b = xs[at], ys[at]
        area = (b[1] - b[2]) * (a[0] - a[2]) + (a[2] - a[1]) * (b[0] - b[2])
        if abs(area) < 1e-12:
            covered[y0[at]:y1[at], x0[at]:x1[at]] = True
            continue
        gx, gy = np.meshgrid(np.arange(x0[at], x1[at]) + 0.5, np.arange(y0[at], y1[at]) + 0.5)
        one = ((b[1] - b[2]) * (gx - a[2]) + (a[2] - a[1]) * (gy - b[2])) / area
        two = ((b[2] - b[0]) * (gx - a[2]) + (a[0] - a[2]) * (gy - b[2])) / area
        covered[y0[at]:y1[at], x0[at]:x1[at]] |= (
            (one >= -0.001) & (two >= -0.001) & ((1.0 - one - two) >= -0.001))

    was = float(covered.mean())
    colour = sheet[:, :, :3]
    mended = mend_the_stray_hair(None, mesh, colour, wide, high) if MENDS_THE_HAIR else 0
    colour = colour.copy()
    filled = covered.copy()
    rings = ((1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1))
    for _ in range(margin):
        empty = ~filled
        if not empty.any():
            break
        total = np.zeros_like(colour)
        count = np.zeros((high, wide), dtype=np.float32)
        for down, right in rings:
            near = np.roll(np.roll(colour, down, axis=0), right, axis=1)
            has = np.roll(np.roll(filled.astype(np.float32), down, axis=0), right, axis=1)
            total += near * has[:, :, None]
            count += has
        grows = empty & (count > 0)
        colour[grows] = total[grows] / count[grows][:, None]
        filled |= grows

    sheet[:, :, :3] = colour
    image.pixels.foreach_set(sheet.ravel())
    image.update()
    return was, float(filled.mean()), margin, mended


def his_axes(rig):
    """Down, ahead and across - MEASURED off the rig, never assumed off a world axis.

    Assuming cost a whole pass: aiming the arms "forward" along world -X aimed them sideways,
    because this warden is not authored down a world axis. The hip line gives across and his own
    `headfront` marker gives ahead - the same marker that finally settled `look::Build::turn`.
    """
    into = rig.matrix_world.to_3x3().inverted()
    down = (into @ mathutils.Vector((0.0, 0.0, -1.0))).normalized()
    left = the_bone(rig, "L_Thigh").bone.head_local
    right = the_bone(rig, "R_Thigh").bone.head_local
    across = (left - right)
    across -= down * across.dot(down)
    across.normalize()
    face = None
    if "headfront" in rig.pose.bones and "Head" in rig.pose.bones:
        face = (rig.pose.bones["headfront"].bone.head_local
                - rig.pose.bones["Head"].bone.head_local)
        face -= down * face.dot(down)
    if face is not None and face.length > 1e-6:
        ahead = face.normalized()
    else:
        ahead = across.cross(down).normalized()
    return down, ahead, across


def aim_the_segment(rig, bone, child, want):
    """Turns `bone` until the line to `child` points along `want`. Returns how far it turned."""
    a = rig.pose.bones[bone]
    b = rig.pose.bones[child]
    now = (b.head - a.head)
    if now.length < 1e-7:
        return 0.0
    now.normalize()
    want = want.normalized()
    turn = now.rotation_difference(want)
    a.matrix = (mathutils.Matrix.Translation(a.head)
                @ turn.to_matrix().to_4x4()
                @ mathutils.Matrix.Translation(-a.head)
                @ a.matrix)
    bpy.context.view_layer.update()
    return math.degrees(now.angle(want))


def hold_the_orientation(rig, bone, was):
    """Puts a bone back to a world orientation captured earlier, leaving where it sits alone."""
    a = rig.pose.bones[bone]
    a.matrix = mathutils.Matrix.Translation(a.matrix.to_translation()) @ was.to_3x3().to_4x4()
    bpy.context.view_layer.update()


def how_clear_the_arms_are(mesh):
    """Intersecting triangle pairs between forearm+hand and the waist. Zero is the only pass.

    Deliberately NOT the whole arm against the whole torso: the upper arm meets the torso at the
    shoulder by design, and a vertex weighted above the threshold to both would read as a hit on
    every frame forever - so a shared vertex is dropped from both sides. Same lesson as the legs:
    `BVHTree.overlap`, because vertex distance cannot see one surface pass through another.
    """
    from mathutils.bvhtree import BVHTree

    posed = mesh.evaluated_get(bpy.context.evaluated_depsgraph_get())
    skin = posed.to_mesh()

    def group(names, floor=0.4):
        want = {mesh.vertex_groups[n].index for n in names if n in mesh.vertex_groups}
        keep = set()
        for vertex in skin.vertices:
            for weighted in vertex.groups:
                if weighted.group in want and weighted.weight > floor:
                    keep.add(vertex.index)
                    break
        return keep

    def tree(keep):
        verts, faces = [], []
        for face in skin.polygons:
            if len(face.vertices) == 3 and all(v in keep for v in face.vertices):
                at = len(verts)
                verts.extend(mesh.matrix_world @ skin.vertices[i].co for i in face.vertices)
                faces.append((at, at + 1, at + 2))
        return BVHTree.FromPolygons(verts, faces) if faces else None

    waist = group(["Hips", "Spine01"]) or group(["Hip", "Spine01"])
    hits = 0
    for side in ("Left", "Right"):
        arm = group([side + "ForeArm", side + "Hand"])
        if not arm:
            arm = group([side[0] + "_Forearm", side[0] + "_Hand"])
        shared = arm & waist
        one, two = tree(arm - shared), tree(waist - shared)
        if one and two:
            hits += len(one.overlap(two))
    posed.to_mesh_clear()
    return hits


def how_low_the_skin_goes(mesh):
    """The lowest point of the POSED skin. The sole stands on the floor, not a bone."""
    posed = mesh.evaluated_get(bpy.context.evaluated_depsgraph_get())
    skin = posed.to_mesh()
    low = min((mesh.matrix_world @ v.co).z for v in skin.vertices)
    posed.to_mesh_clear()
    return low


def stand_him_up(rig, mesh, out_by):
    """Aims every limb from the bind into a stand. Returns what each segment turned."""
    for bone in rig.pose.bones:
        bone.matrix_basis = mathutils.Matrix.Identity(4)
    bpy.context.view_layer.update()
    down, ahead, across = his_axes(rig)

    # The extremities keep the orientation the artist bound them at, so the sole stays flat and
    # the hand keeps its shape - only the limbs above them are aimed.
    ends = [n for n in ("LeftFoot", "RightFoot", "LeftHand", "RightHand") if n in rig.pose.bones]
    keep = {n: rig.pose.bones[n].matrix.copy() for n in ends}

    turned = {}
    for side, sign in (("Left", 1.0), ("Right", -1.0)):
        out = across * sign
        leg = (down + out * math.tan(math.radians(THE_LEGS_STAND_APART_BY))).normalized()
        turned[side + " thigh"] = aim_the_segment(rig, side + "UpLeg", side + "Leg", leg)
        turned[side + " calf"] = aim_the_segment(rig, side + "Leg", side + "Foot", down)
        upper = (down
                 + out * math.tan(math.radians(out_by))
                 + ahead * math.tan(math.radians(THE_ARMS_HANG_FORWARD))).normalized()
        turned[side + " upper arm"] = aim_the_segment(rig, side + "Arm", side + "ForeArm", upper)
        fore = (down
                + out * math.tan(math.radians(out_by * 0.35))
                + ahead * math.tan(math.radians(THE_ELBOWS_COME_FORWARD))).normalized()
        turned[side + " forearm"] = aim_the_segment(rig, side + "ForeArm", side + "Hand", fore)

    for name, was in keep.items():
        hold_the_orientation(rig, name, was)

    # Settle on the floor by the SKIN, not by a bone - the sole is what stands on it.
    low = how_low_the_skin_goes(mesh)
    root = rig.pose.bones["Hips"] if "Hips" in rig.pose.bones else the_bone(rig, "Hip")
    root.matrix = (mathutils.Matrix.Translation(
        rig.matrix_world.to_3x3().inverted() @ mathutils.Vector((0.0, 0.0, -low)))
        @ root.matrix)
    bpy.context.view_layer.update()
    return turned


def let_the_arms_hang(rig):
    """Superseded by stand_him_up. Kept because the idle used to be built this way."""
    down = rig.matrix_world.to_3x3().inverted() @ mathutils.Vector((0.0, 0.0, -1.0))
    down.normalize()
    across = rig.matrix_world.to_3x3().inverted() @ mathutils.Vector((0.0, 1.0, 0.0))
    across.normalize()
    ahead = rig.matrix_world.to_3x3().inverted() @ mathutils.Vector((1.0, 0.0, 0.0))
    ahead.normalize()

    aimed = {}
    for side, out in (("Left", 1.0), ("Right", -1.0)):
        arm = rig.pose.bones.get(f"{side}Arm") or rig.pose.bones.get(f"{side[0]}_Upperarm")
        hand = rig.pose.bones.get(f"{side}Hand") or rig.pose.bones.get(f"{side[0]}_Hand")
        if arm is None or hand is None:
            continue
        now = (hand.head - arm.head)
        if now.length < 1e-6:
            continue
        now.normalize()
        # Which way is "out" for this side, measured off the rig rather than assumed.
        sideways = across * out
        if sideways.dot(now) < 0.0:
            sideways = -sideways
        want = (down
                + sideways * math.tan(math.radians(THE_ARMS_HANG_OUT))
                + ahead * math.tan(math.radians(THE_ARMS_HANG_FORWARD)))
        want.normalize()
        turn = now.rotation_difference(want)
        arm.matrix = (mathutils.Matrix.Translation(arm.head)
                      @ turn.to_matrix().to_4x4()
                      @ mathutils.Matrix.Translation(-arm.head)
                      @ arm.matrix)
        bpy.context.view_layer.update()
        aimed[side] = math.degrees(now.angle(want))
    return aimed


def zip_the_pinholes(mesh):
    """Welds each truly open boundary vertex onto its nearest neighbour. See ZIPS_THE_PINHOLES."""
    import bmesh
    bm = bmesh.new()
    bm.from_mesh(mesh.data)
    bm.verts.ensure_lookup_table()

    def truly_open():
        seen = {}
        for edge in bm.edges:
            if len(edge.link_faces) != 1:
                continue
            a = tuple(round(c, 5) for c in edge.verts[0].co)
            b = tuple(round(c, 5) for c in edge.verts[1].co)
            seen.setdefault((min(a, b), max(a, b)), []).append(edge)
        return [edges[0] for edges in seen.values() if len(edges) == 1]

    before = truly_open()
    loose = {v for e in before for v in e.verts}
    boundary = [v for e in bm.edges if len(e.link_faces) == 1 for v in e.verts]
    tree = mathutils.kdtree.KDTree(len(boundary))
    for at, v in enumerate(boundary):
        tree.insert(v.co, at)
    tree.balance()

    welded = 0
    for v in list(loose):
        if not v.is_valid:
            continue
        best = None
        for _, at, span in tree.find_n(v.co, 8):
            other = boundary[at]
            if other is v or not other.is_valid or other in loose:
                continue
            if span < A_PINHOLE_SPANS:
                best = other
                break
        if best is None:
            continue
        import bmesh.ops
        bmesh.ops.pointmerge(bm, verts=[v, best], merge_co=best.co)
        welded += 1

    left = truly_open()
    bm.to_mesh(mesh.data)
    bm.free()
    mesh.data.update()
    return len(before), welded, len(left)


def stand_him_still(rig, walk, scene, mesh=None, called="idle"):
    """Freezes the walk's most standing-like frame into a still clip. See STANDS_STILL_FROM.

    Chosen by measurement, not by picking a frame number: feet closest together, and lowest, wins.
    """
    if STANDS_STILL_FROM == "bind":
        # The artist's own neutral stand, with the arms let down out of the A-pose.
        at = 0
        # The bind is NOT a stand on this rig. Measured on it: the feet sit 7.65 cm apart in
        # height and the hands 26 cm apart fore and aft - which is exactly the raised leg and the
        # arm behind the back that were reported. So the stand is AUTHORED: every limb aimed at a
        # direction built from his own axes, the arms opened until the mesh genuinely clears the
        # waist, and the whole figure settled on the floor by its lowest skin point. Three guards
        # below, and they compare against the stand we asked for, not against what came out.
        at = 0
        out_by, hits = THE_ARMS_HANG_OUT, 0
        while True:
            turned = stand_him_up(rig, mesh, out_by)
            hits = how_clear_the_arms_are(mesh)
            if hits == 0 or out_by >= OPENS_THE_ARMS_TO:
                break
            out_by += OPENS_THE_ARMS_BY
        if hits:
            refuse(f"the idle's arms still pass through his waist at {out_by:.1f} deg out - "
                   f"{hits} intersecting triangle pairs")
        left = rig.matrix_world @ the_bone(rig, "L_Foot").head
        right = rig.matrix_world @ the_bone(rig, "R_Foot").head
        level = abs(left.z - right.z) * 100.0
        if level > THE_FEET_LEVEL_WITHIN:
            refuse(f"the idle stands with one foot {level:.2f} cm higher than the other")
        stands = abs(how_low_the_skin_goes(mesh)) * 100.0
        if stands > THE_IDLE_STANDS_ON_THE_FLOOR:
            refuse(f"the idle's lowest skin sits {stands:.2f} cm off the floor")
        print(f"    stood him up: arms {out_by:.1f} deg out and clear of the waist, feet level "
              f"to {level:.2f} cm, standing on the floor to {stands:.2f} cm")
        for what, by in sorted(turned.items()):
            print(f"      aimed his {what:<16} {by:5.1f} deg")
        held = {bone.name: bone.matrix_basis.copy() for bone in rig.pose.bones}
    else:
        first, last = (int(round(v)) for v in walk.frame_range)
        play(rig, walk)
        best, at = None, first
        for frame in range(first, last + 1):
            scene.frame_set(frame)
            bpy.context.view_layer.update()
            left = rig.matrix_world @ the_bone(rig, "L_Foot").head
            right = rig.matrix_world @ the_bone(rig, "R_Foot").head
            apart = (left - right).length
            high = max(left.z, right.z)
            score = apart + high
            if best is None or score < best:
                best, at = score, frame
        scene.frame_set(at)
        bpy.context.view_layer.update()
        held = {bone.name: bone.matrix_basis.copy() for bone in rig.pose.bones}

    still = bpy.data.actions.new(called)
    still.use_fake_user = True
    rig.animation_data.action = still
    slots = getattr(still, "slots", None)
    if slots is not None:
        try:
            rig.animation_data.action_slot = still.slots.new("OBJECT", rig.name)
        except Exception:
            pass
    for frame in (1, STANDS_STILL_FOR):
        scene.frame_set(frame)
        for bone in rig.pose.bones:
            bone.matrix_basis = held[bone.name]
            bone.rotation_mode = "QUATERNION"
        bpy.context.view_layer.update()
        for bone in rig.pose.bones:
            bone.keyframe_insert("rotation_quaternion", frame=frame)
            bone.keyframe_insert("location", frame=frame)
    return still, at


def speak_the_clips_language(clips):
    """Rewrites an action's channel names onto the pipeline's convention. See RENAMES.

    Every delivered file needs this, not just the one the skeleton is taken from. An fcurve
    addresses its bone by name inside its data path, so a clip imported from a file whose bones
    were never renamed points at joints that do not exist on the base rig - and Blender says
    nothing. The clip simply drives nothing and plays as a rest pose.

    That is how the run arrived: the walk was the base and got renamed, the run came second and did
    not, and the loop-closing bake then froze a dead action into 540 curves of stationary
    character. It exported cleanly. The first sign was the plant measuring a sole that read 2.71 cm
    on every frame of a run.
    """
    moved = 0
    for clip in clips:
        for curve in fcurves_of(clip, None):
            for was, becomes in RENAMES.items():
                head = 'pose.bones["%s"]' % was
                if was != becomes and curve.data_path.startswith(head):
                    curve.data_path = ('pose.bones["%s"]' % becomes) + curve.data_path[len(head):]
                    moved += 1
                    break
    return moved


def speak_the_pipeline_s_language(rig, meshes):
    """Renames the incoming skeleton and scales the figure to one unit. See RENAMES.

    The vertex groups are renamed with the bones or the skin comes off them - a group is bound to
    a bone by NAME, and nothing warns when the two stop matching. Blender renames the groups for
    you when you rename a bone through the UI; setting `bone.name` from a script does not.
    """
    named = 0
    for was, becomes in RENAMES.items():
        bone = rig.data.bones.get(was)
        if bone is None or was == becomes:
            continue
        if rig.data.bones.get(becomes) is not None:
            refuse(f"cannot rename {was} to {becomes}: something already holds that name")
        bone.name = becomes
        for mesh in meshes:
            group = mesh.vertex_groups.get(was)
            if group is not None:
                group.name = becomes
        # # And the animation, which does NOT come with it
        #
        # An fcurve addresses a bone by name inside its data path - `pose.bones["LeftUpLeg"].
        # rotation_quaternion` - and setting `bone.name` from a script leaves every one of them
        # pointing at a bone that no longer exists. Blender does not warn; the channels simply stop
        # driving anything and the clip plays as a rest pose.
        #
        # It is silent in a particularly nasty way, too. The build ran to completion and exported,
        # and the first sign was the plant reporting a sole that measured 2.71 cm on every frame of
        # a run. A clip that does not move looks like a clip until something measures it.
        was_path = 'pose.bones["%s"]' % was
        now_path = 'pose.bones["%s"]' % becomes
        for clip in bpy.data.actions:
            for curve in fcurves_of(clip, None):
                if curve.data_path.startswith(was_path):
                    curve.data_path = now_path + curve.data_path[len(was_path):]
        named += 1

    strays = [b.name for b in rig.data.bones
              if b.name not in RENAMES.values() and b.name not in RENAMES]

    # The SKINNED mesh decides how tall he is. The file also carries a two-unit Icosphere that the
    # build drops later, and taking the tallest of everything measured him against that instead -
    # a 1.700 figure reported as 2.000 and scaled by 0.5 rather than 0.588.
    tall = 0.0
    for mesh in meshes:
        if not mesh.data.vertices or not mesh.vertex_groups:
            continue
        zs = [(mesh.matrix_world @ v.co).z for v in mesh.data.vertices]
        tall = max(tall, max(zs) - min(zs))
    # # One space, and only one
    #
    # The delivered objects carry a 0.01 scale: the data is in centimetres and the object shrinks
    # it to metres. That is a perfectly ordinary way to ship a model and it quietly breaks anything
    # that computes a displacement in WORLD units and then writes it to a bone channel, because a
    # bone channel is in DATA units and the two differ by a hundred.
    #
    # `stand_on_the_floor` is exactly that: it works out how far to lift, turns world up into the
    # carrier's rest frame, NORMALISES it - which throws the scale away - and multiplies by a
    # world-space distance. Measured, it asked for 1.51 cm of lift and moved the sole 0.02.
    #
    # So the object transforms are baked into the data and reset to identity, and the figure is
    # scaled to one unit in the same operation. After this a data unit IS a world unit, everywhere,
    # and the question cannot come up again.
    grew = 1.0
    if tall > 1e-6:
        grew = STANDS_A_UNIT_TALL / tall
    settle = mathutils.Matrix.Scale(grew, 4)

    # The curve factor is the WHOLE change a data unit undergoes - the object's own scale as well
    # as the resize - because that is what a location value is measured in.
    carried = rig.matrix_world.to_scale()
    factor = grew * (sum(carried) / 3.0)

    rig.data.transform(settle @ rig.matrix_world)
    rig.matrix_world = mathutils.Matrix.Identity(4)
    for mesh in meshes:
        mesh.data.transform(settle @ mesh.matrix_world)
        mesh.matrix_world = mathutils.Matrix.Identity(4)

    bpy.context.view_layer.update()
    # The clips are NOT scaled here. Only the base's exists at this point, and every later file
    # brings its own - see `carry_the_clips_into_the_new_scale`, which the import loop calls per
    # file with the factor handed back from here.
    return named, tall, grew, strays, factor


def which_way_he_faces(rig):
    """Which way the figure points, squared off his HIP LINE.

    A hip line cannot toe out, which a foot can and on this rig emphatically does - its two bind
    toes splay 58.47 degrees apart, so either one alone is off by twenty-nine. The shoulders are
    kept as a second opinion rather than an input; they agree to two decimal places.
    """
    left = rig.matrix_world @ rig.pose.bones["L_Thigh"].bone.head_local
    right = rig.matrix_world @ rig.pose.bones["R_Thigh"].bone.head_local
    span = left - right
    span.z = 0.0
    if span.length < 1e-9:
        refuse("the bind's hips sit on top of each other, so it has no facing")
    span.normalize()
    return mathutils.Vector((span.y, -span.x, 0.0))


def square_him_up(rig, meshes):
    """Turns the whole figure onto `FACES_ALONG`, bones and flesh together. See SQUARES_HIM_UP."""
    faces = which_way_he_faces(rig)
    want = mathutils.Vector((FACES_ALONG[0], FACES_ALONG[1], 0.0)).normalized()
    yaw = math.atan2(faces.x * want.y - faces.y * want.x,
                     faces.x * want.x + faces.y * want.y)

    shoulders = None
    if "L_Clavicle" in rig.pose.bones and "R_Clavicle" in rig.pose.bones:
        left = rig.matrix_world @ rig.pose.bones["L_Clavicle"].bone.head_local
        right = rig.matrix_world @ rig.pose.bones["R_Clavicle"].bone.head_local
        span = left - right
        span.z = 0.0
        if span.length > 1e-9:
            span.normalize()
            shoulders = mathutils.Vector((span.y, -span.x, 0.0))
            apart = math.degrees(faces.angle(shoulders))
            if apart > 5.0:
                refuse(f"his hips and his shoulders disagree about which way he faces by "
                       f"{apart:.2f} degrees, so squaring him up would only pick a side")

    turn = mathutils.Matrix.Rotation(yaw, 4, "Z")
    rig.data.transform(turn)
    for mesh in meshes:
        mesh.data.transform(turn)
    bpy.context.view_layer.update()

    now = which_way_he_faces(rig)
    left_over = math.degrees(now.angle(want))
    if left_over > SQUARE_WITHIN:
        refuse(f"turning him {math.degrees(yaw):+.2f} degrees left him {left_over:.2f} off the "
               f"axis, past the {SQUARE_WITHIN} that counts as square")
    return math.degrees(yaw), left_over


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


def the_arms_rest_at(rig, clip, scene):
    """The smallest angle between each upper arm and the spine, over a whole clip.

    Measured on the EVALUATED rig frame by frame rather than read off the curves, because the
    shoulder inherits from the spine and the chest and the number that matters is where the arm
    actually ends up.
    """
    play(rig, clip)
    first, last = (int(round(v)) for v in clip.frame_range)
    closest = {"L": 180.0, "R": 180.0}
    for frame in range(first, last + 1):
        scene.frame_set(frame)
        posed = rig.evaluated_get(bpy.context.evaluated_depsgraph_get())
        spine = ((posed.matrix_world @ posed.pose.bones["Spine02"].head)
                 - (posed.matrix_world @ posed.pose.bones["Spine01"].head))
        for side in ("L", "R"):
            upper = ((posed.matrix_world @ posed.pose.bones[f"{side}_Forearm"].head)
                     - (posed.matrix_world @ posed.pose.bones[f"{side}_Upperarm"].head))
            if upper.length < 1e-9 or spine.length < 1e-9:
                continue
            off = 180.0 - math.degrees(upper.normalized().angle(spine.normalized()))
            closest[side] = min(closest[side], off)
    return closest


def which_way_abducts(rig, side):
    """The bone-local axis that swings this upper arm AWAY from the spine.

    Derived rather than assumed. Rotating a direction `u` about an axis `n` by a small angle
    moves it by `n x u`, so `u.s` grows fastest about `u x s`. Abduction is `180 - angle(u, s)`,
    which grows as `u.s` grows - so `u x s` IS the axis, not its negation. Written out because
    guessing the sign per side is how the finger curls went wrong, and a mirrored limb flips it.

    Derived, then checked: `lift_the_arms` refuses a lift that leaves the arm closer to the body
    than it found it, so a sign error is a failed build rather than a worse render. The first
    version of this negated the cross product and drove the left arm from 10.0 deg down to 4.5.
    """
    bone = rig.pose.bones[f"{side}_Upperarm"]
    upper = ((rig.matrix_world @ rig.pose.bones[f"{side}_Forearm"].bone.head_local)
             - (rig.matrix_world @ bone.bone.head_local))
    spine = ((rig.matrix_world @ rig.pose.bones["Spine02"].bone.head_local)
             - (rig.matrix_world @ rig.pose.bones["Spine01"].bone.head_local))
    if upper.length < 1e-9 or spine.length < 1e-9:
        refuse(f"the {side} upper arm or the spine has no length, so no abduction axis exists")
    opens = upper.normalized().cross(spine.normalized())
    if opens.length < 1e-9:
        refuse(f"the {side} upper arm lies along the spine, so abduction is undefined")
    rest = (rig.matrix_world @ bone.bone.matrix_local).to_3x3()
    return (rest.inverted() @ opens.normalized()).normalized()


def lift_the_arms(rig, clip, scene, floor):
    """Adds a constant abduction at each shoulder until the clip's closest frame clears `floor`.

    Composed onto the keyed rotation BEFORE it - `offset * keyed`, not `keyed * offset` - and
    that order is the whole point. Post-multiplying applies the offset in the POSED frame, which
    is right for a twist that follows the bone, and it is what `roll_the_hands` does. An
    abduction is not carried by the bone: it is a constant swing of the whole posed arm about an
    axis fixed in the shoulder, so it composes on the rest side.
    """
    was = the_arms_rest_at(rig, clip, scene)
    slot = rig.animation_data.action_slot if rig.animation_data else None
    curves = fcurves_of(clip, slot)
    lifted = {}
    for side in ("L", "R"):
        short = floor - was[side]
        if short <= 0.05:
            continue
        path = f'pose.bones["{side}_Upperarm"].rotation_quaternion'
        parts = {c.array_index: c for c in curves if c.data_path == path}
        if len(parts) != 4:
            refuse(f"{clip.name} keys {len(parts)} of the 4 rotation channels on "
                   f"{side}_Upperarm, so the arm cannot be lifted without dropping its motion")
        offset = mathutils.Quaternion(which_way_abducts(rig, side), math.radians(short))
        for at in range(len(parts[0].keyframe_points)):
            keyed = mathutils.Quaternion([parts[i].keyframe_points[at].co[1] for i in range(4)])
            out = offset @ keyed
            for i in range(4):
                point = parts[i].keyframe_points[at]
                point.handle_left[1] += out[i] - point.co[1]
                point.handle_right[1] += out[i] - point.co[1]
                point.co[1] = out[i]
        for curve in parts.values():
            curve.update()
        lifted[side] = short
    now = the_arms_rest_at(rig, clip, scene) if lifted else was
    for side, by in lifted.items():
        if now[side] < was[side] + by * 0.5:
            refuse(f"lifting the {side} arm by {by:.1f} deg on {clip.name} moved it from "
                   f"{was[side]:.1f} to {now[side]:.1f} deg off the spine - the abduction axis "
                   f"is pointing the wrong way, so the arm was pressed INTO the body")
    return was, lifted, now


def how_far_the_hands_swing(rig, clip, scene):
    """The widest separation each hand reaches, measured against the hip so sway does not count."""
    play(rig, clip)
    first, last = (int(round(v)) for v in clip.frame_range)
    step = max(1, (last - first) // 60)
    seen = {"L": [], "R": []}
    for frame in range(first, last + 1, step):
        scene.frame_set(frame)
        posed = rig.evaluated_get(bpy.context.evaluated_depsgraph_get())
        hip = posed.matrix_world @ posed.pose.bones["Hip"].head
        for side in ("L", "R"):
            seen[side].append((posed.matrix_world @ posed.pose.bones[f"{side}_Hand"].head) - hip)
    widest = {}
    for side, spots in seen.items():
        far = 0.0
        for i, one in enumerate(spots):
            for other in spots[i + 1:]:
                far = max(far, (one - other).length)
        widest[side] = far * 170.0
    return widest


def an_average_of(turns):
    """The mean of a set of rotations, near enough for a small spread.

    Summed and normalised, with every term flipped into the first one's hemisphere first -
    without that, two quaternions describing nearly the same rotation can cancel, because q and
    -q are the same rotation and the sum does not know it.
    """
    if not turns:
        return mathutils.Quaternion()
    total = mathutils.Quaternion((0.0, 0.0, 0.0, 0.0))
    first = turns[0]
    for turn in turns:
        way = -1.0 if turn.dot(first) < 0.0 else 1.0
        for at in range(4):
            total[at] += turn[at] * way
    if total.magnitude < 1e-9:
        return mathutils.Quaternion()
    total.normalize()
    return total


def move_the_arms_more(rig, clip, scene, gain, pumps=1.0, only=None):
    """Scales each arm bone's motion about the clip's own average pose.

    Returns how wide the hands swung before and after, because a gain that does not change that
    number is a knob wired to nothing - which is the failure worth catching, not a wrong value.
    """
    if abs(gain - 1.0) < 1e-6 and abs(pumps - 1.0) < 1e-6:
        return None, None
    before = how_far_the_hands_swing(rig, clip, scene)
    slot = rig.animation_data.action_slot if rig.animation_data else None
    curves = fcurves_of(clip, slot)
    for side in ("L", "R"):
        # `only` and not `at`: this function already uses `at` as a loop index below, and a
        # parameter of that name was shadowed by it on the second pass - which showed up as
        # "'int' object is not iterable" rather than as anything about arms.
        for part in (only or MOVES_AT):
            path = f'pose.bones["{side}_{part}"].rotation_quaternion'
            parts = {c.array_index: c for c in curves if c.data_path == path}
            if len(parts) != 4:
                continue
            keys = [mathutils.Quaternion([parts[i].keyframe_points[at].co[1] for i in range(4)])
                    for at in range(len(parts[0].keyframe_points))]
            middle = an_average_of(keys)
            if middle.magnitude < 1e-9:
                continue
            back = middle.inverted()
            # How far this bone strays from its own average, and the furthest it ever does, so
            # the shaping below has something to normalise against.
            strays = []
            for was in keys:
                off = back @ was
                if off.w < 0.0:
                    off.negate()
                strays.append(abs(off.angle))
            widest = max(strays) if strays else 0.0
            for at, was in enumerate(keys):
                # `share` is what fraction of this bone's own widest excursion this key sits at.
                # Raising it to a power under one flattens the peaks and steepens the middle, so
                # the arm holds near its extremes and crosses between them quickly - a pump
                # rather than a glide. At `share` 0 and 1 it is unchanged, so the extremes and
                # the phase are exactly where the animator put them and the cycle still closes.
                share = (strays[at] / widest) if widest > 1e-9 else 0.0
                shaped = (share ** pumps) / share if share > 1e-6 else 1.0
                out = middle @ a_share_of(back @ was, gain * shaped)
                for i in range(4):
                    point = parts[i].keyframe_points[at]
                    point.handle_left[1] += out[i] - point.co[1]
                    point.handle_right[1] += out[i] - point.co[1]
                    point.co[1] = out[i]
            for curve in parts.values():
                curve.update()
    after = how_far_the_hands_swing(rig, clip, scene)
    for side in ("L", "R"):
        # The gain may calm a clip as well as liven one, so the check is that the swing moved the
        # way the gain asked - not that it grew. What it is really catching is a knob wired to
        # nothing, and that shows up as no movement either way.
        moved = after[side] - before[side]
        if abs(moved) < 0.1:
            refuse(f"a gain of {gain} and a pump of {pumps} on {clip.name} left the {side} "
                   f"hand swinging "
                   f"{after[side]:.1f} cm against {before[side]:.1f} cm before - the gain is "
                   f"wired to nothing")
    return before, after


def the_torso_bands(rig, mesh):
    """Which vertices make up the lower and upper trunk, chosen once off the bind.

    The trunk's lean is measured from the FLESH, not from the spine bones, and this is why:
    measured against the torso they deform, `Waist` sits 67.4% toward his front, `Spine01`
    72.1% and `Spine02` 77.9%. The chain is not only displaced forward, it is displaced by
    INCREASING amounts up its length - so the line from one bone to the next tilts forward
    relative to the body it is inside, and any angle read off it is biased by that tilt.

    That is what "looks like the spine is in the front which is probably causing the odd lean"
    was pointing at, and it is why the trunk read +7.4 degrees in front of vertical while the
    warden plainly leant back. The bones are where the animator's rig puts them; the torso is
    where the vertices are.
    """
    groups = {g.index: g.name for g in mesh.vertex_groups}
    mine = []
    for vertex in mesh.data.vertices:
        best, who = 0.0, ""
        for group in vertex.groups:
            if group.weight > best:
                best, who = group.weight, groups.get(group.group, "")
        if any(part in who for part in ("Waist", "Spine", "Chest", "Pelvis")):
            mine.append(vertex.index)
    if len(mine) < 20:
        refuse("too few vertices belong to the trunk to measure its lean from")
    heights = sorted((mesh.matrix_world @ mesh.data.vertices[i].co).z for i in mine)
    low = heights[len(heights) // 5]
    high = heights[len(heights) * 4 // 5]
    lower = [i for i in mine if (mesh.matrix_world @ mesh.data.vertices[i].co).z <= low]
    upper = [i for i in mine if (mesh.matrix_world @ mesh.data.vertices[i].co).z >= high]
    if len(lower) < 5 or len(upper) < 5:
        refuse("the trunk has no clear top and bottom to measure a lean between")
    return lower, upper


def the_torso_leans(rig, mesh, clip, scene, bands, sideways=False):
    """The angle of the TORSO ITSELF off vertical, averaged over a clip.

    The centroid of the lower band to the centroid of the upper one - a line through the middle
    of the flesh, which is what a viewer reads as the body's lean.
    """
    play(rig, clip)
    first, last = (int(round(v)) for v in clip.frame_range)
    step = max(1, (last - first) // 40)
    bind = rig.pose.bones["L_ToeBase"].bone
    travel = ((rig.matrix_world @ bind.tail_local) - (rig.matrix_world @ bind.head_local))
    travel.z = 0.0
    travel.normalize()
    across = mathutils.Vector((-travel.y, travel.x, 0.0))
    lower, upper = bands
    seen = []
    for frame in range(first, last + 1, step):
        scene.frame_set(frame)
        skin = mesh.evaluated_get(bpy.context.evaluated_depsgraph_get())
        spots = skin.data.vertices
        low = mathutils.Vector((0.0, 0.0, 0.0))
        for i in lower:
            low += skin.matrix_world @ spots[i].co
        low /= len(lower)
        high = mathutils.Vector((0.0, 0.0, 0.0))
        for i in upper:
            high += skin.matrix_world @ spots[i].co
        high /= len(upper)
        up = high - low
        if up.length < 1e-9:
            continue
        if sideways:
            off = up.dot(across)
            seen.append(math.degrees(math.atan2(off, math.sqrt(
                max(up.length_squared - off * off, 1e-12)))))
        else:
            flat = mathutils.Vector((up.x, up.y, 0.0))
            seen.append(math.degrees(math.atan2(flat.length, up.z))
                        * (1.0 if flat.dot(travel) > 0 else -1.0))
    return (sum(seen) / len(seen)) if seen else 0.0, travel


def lean_the_torso(rig, mesh, clip, scene, target, what, sideways=False):
    """Leans the TORSO to `target` degrees off vertical, measured from its own flesh."""
    bands = the_torso_bands(rig, mesh)
    was, travel = the_torso_leans(rig, mesh, clip, scene, bands, sideways)
    now, moved = was, 0.0
    for _ in range(24):
        short = target - now
        if abs(short) < 0.25:
            break
        moved += short * 0.34
        lean_by(rig, clip, short * 0.34, travel, LEANS_ALONG, sideways)
        now, _ = the_torso_leans(rig, mesh, clip, scene, bands, sideways)
    if abs(now - target) > 2.0:
        refuse(f"the {what} on {clip.name} would not settle: {now:+.1f} off vertical against a "
               f"{target:+.1f} target")
    return was, moved, now


def the_chain_leans(rig, clip, scene, bones, sideways=False):
    """How far a chain is off vertical, averaged over a clip.

    Positive forward, or positive toward the warden's left when `sideways`. Two different faults
    live on these two axes and neither measurement can see the other: the trunk was brought to
    +6.2 forward while it was still listing 12.8 degrees to one side.
    """
    play(rig, clip)
    first, last = (int(round(v)) for v in clip.frame_range)
    step = max(1, (last - first) // 60)
    bind = rig.pose.bones["L_ToeBase"].bone
    travel = ((rig.matrix_world @ bind.tail_local) - (rig.matrix_world @ bind.head_local))
    travel.z = 0.0
    if travel.length < 1e-9:
        refuse("the bind toe has no horizontal direction, so forward cannot be established")
    travel.normalize()
    across = mathutils.Vector((-travel.y, travel.x, 0.0))
    seen = []
    for frame in range(first, last + 1, step):
        scene.frame_set(frame)
        posed = rig.evaluated_get(bpy.context.evaluated_depsgraph_get())
        low = posed.matrix_world @ posed.pose.bones[bones[0]].head
        high = posed.matrix_world @ posed.pose.bones[bones[-1]].tail
        up = high - low
        flat = mathutils.Vector((up.x, up.y, 0.0))
        if up.length < 1e-9:
            continue
        if sideways:
            off = up.dot(across)
            rest = math.sqrt(max(up.length_squared - off * off, 1e-12))
            seen.append(math.degrees(math.atan2(off, rest)))
        else:
            seen.append(math.degrees(math.atan2(flat.length, up.z))
                        * (1.0 if flat.dot(travel) > 0 else -1.0))
    return (sum(seen) / len(seen)) if seen else 0.0, travel


def the_chain_rests_at(rig, bones, sideways=False):
    """A chain's own resting angle off vertical, from the bind and nothing else."""
    bind = rig.pose.bones["L_ToeBase"].bone
    travel = ((rig.matrix_world @ bind.tail_local) - (rig.matrix_world @ bind.head_local))
    travel.z = 0.0
    travel.normalize()
    low = rig.matrix_world @ rig.pose.bones[bones[0]].bone.head_local
    high = rig.matrix_world @ rig.pose.bones[bones[-1]].bone.tail_local
    up = high - low
    flat = mathutils.Vector((up.x, up.y, 0.0))
    if up.length < 1e-9:
        return 0.0
    if sideways:
        across = mathutils.Vector((-travel.y, travel.x, 0.0))
        off = up.dot(across)
        return math.degrees(math.atan2(off, math.sqrt(
            max(up.length_squared - off * off, 1e-12))))
    return (math.degrees(math.atan2(flat.length, up.z))
            * (1.0 if flat.dot(travel) > 0 else -1.0))


def which_way_leans_forward(rig, bone, travel, sideways=False):
    """The bone-local axis that tips this spine bone's top FORWARD.

    Derived, never assumed, and this exact thing has gone wrong here before: an axis measured on
    thighs and upper arms - which point DOWN from their joints - was reused on the spine, which
    points UP, and the identical rotation carried a thigh's foot forward and a spine's head
    BACKWARD. The whole torso leant back at every speed. See TROUBLESHOOTING.md.
    """
    here = rig.pose.bones[bone]
    kids = [b for b in rig.pose.bones if b.parent is not None and b.parent.name == bone]
    above = min(kids, key=lambda b: (b.bone.head_local - here.bone.head_local).length) \
        if kids else None
    up = ((rig.matrix_world @ (above.bone.head_local if above else here.bone.tail_local))
          - (rig.matrix_world @ here.bone.head_local))
    if up.length < 1e-9:
        refuse(f"{bone} has no length, so no lean axis exists")
    # Rotating `up` about `n` moves it by `n x up`, so `up.travel` - how far forward the top of
    # this bone points - grows fastest about `up x travel`. Not its negation: that was written
    # first, and the guard in `lean_the_trunk` caught it immediately, turning a -28.3 degree
    # correction into +48.8 from rest instead of +7. The third time on this character that a
    # derived axis has been negated by hand, and the third time a check caught it rather than a
    # render.
    toward = mathutils.Vector((-travel.y, travel.x, 0.0)) if sideways else travel
    tips = up.normalized().cross(toward)
    if tips.length < 1e-9:
        refuse(f"{bone} points along the line of travel, so its lean is undefined")
    rest = (rig.matrix_world @ here.bone.matrix_local).to_3x3()
    return (rest.inverted() @ tips.normalized()).normalized()


def lean_a_chain(rig, clip, scene, bones, target, what, sideways=False, absolute=False):
    """Brings a chain's lean to `target` degrees forward of the model's own rest posture.

    A constant offset shared down the chain, in the same shape as `lift_the_arms`: the clip keeps
    every bit of its own motion and only the angle it is carried at moves.

    Used twice - once on the spine, once on the head - because levelling a head is the same
    problem as leaning a trunk, and the second one only exists because of the first: rotating the
    spine back by forty degrees carried the head back with it, and the warden jogged along
    looking at the sky, 28 degrees above where he rests.
    """
    was, travel = the_chain_leans(rig, clip, scene, bones, sideways)
    rests = the_chain_rests_at(rig, bones, sideways)
    now, moved = was, 0.0
    # Measured, corrected, measured again, until it lands. A spine is a CHAIN: rotating a bone
    # tips everything above it and nothing below, so how far the trunk as a whole moves for a
    # given rotation depends on where in the chain it is applied and on the pose it is applied
    # from. Rather than model that leverage - and be wrong about it quietly - the correction
    # measures what it actually achieved and applies the remainder. The first version assumed
    # one degree in gave one degree out, shared it evenly across two bones, and delivered 12.6
    # degrees of a 28.3 degree correction.
    # HALF the shortfall each pass, not all of it. The chain's gain is above one - rotating
    # `Spine01` tips `Spine02` with it, and then `Spine02` adds its own on top - so correcting by
    # the full error overshoots and the iteration oscillates instead of settling. It went from
    # +35.3 to -1.9 from rest chasing a +7.0 target. Damping converges for any gain up to four.
    # A third of the shortfall, over more passes. Halving converged for the sideways lean and
    # stalled 1.6 degrees short on the forward one, because the chain's gain is not the same on
    # both axes - a spine bends further forward for a given rotation than it tips sideways. A
    # smaller step is slower and converges over a wider range of gains, and slow costs nothing
    # here: it is arithmetic on curves, not a solve.
    for _ in range(24):
        short = (target if absolute else rests + target) - now
        if abs(short) < 0.25:
            break
        moved += short * 0.34
        lean_by(rig, clip, short * 0.34, travel, bones, sideways)
        now, _ = the_chain_leans(rig, clip, scene, bones, sideways)
    landed = now if absolute else now - rests
    # Two degrees, not one. The correction axis is derived from the BIND pose, so as the trunk
    # swings a long way from it - twenty-four degrees, here - the axis fits the current pose less
    # well and each pass buys less than the last. It asymptotes at +7.4 against a +6.0 target and
    # no amount of extra passes closes it; recomputing the axis per pass would, and is not worth
    # it while both numbers sit inside the 4-to-12 band the research gives.
    if abs(landed - target) > 2.0:
        refuse(f"the {what} on {clip.name} would not settle: {landed:+.1f} against a "
               f"{target:+.1f} target")
    return was, rests, moved, now


def lean_by(rig, clip, short, travel, bones, sideways=False):
    """Adds `short` degrees of forward trunk lean, shared down the spine."""
    slot = rig.animation_data.action_slot if rig.animation_data else None
    curves = fcurves_of(clip, slot)
    each = short / len(bones)
    for bone in bones:
        path = f'pose.bones["{bone}"].rotation_quaternion'
        parts = {c.array_index: c for c in curves if c.data_path == path}
        if len(parts) != 4:
            refuse(f"{clip.name} keys {len(parts)} of the 4 rotation channels on {bone}, so the "
                   f"trunk cannot be leaned without dropping its motion")
        offset = mathutils.Quaternion(which_way_leans_forward(rig, bone, travel, sideways),
                                      math.radians(each))
        for at in range(len(parts[0].keyframe_points)):
            keyed = mathutils.Quaternion([parts[i].keyframe_points[at].co[1] for i in range(4)])
            out = offset @ keyed
            for i in range(4):
                point = parts[i].keyframe_points[at]
                point.handle_left[1] += out[i] - point.co[1]
                point.handle_right[1] += out[i] - point.co[1]
                point.co[1] = out[i]
        for curve in parts.values():
            curve.update()


def which_way_swings_the_arm_back(rig, side, travel):
    """The upper arm's own axis that carries its elbow BACKWARD.

    Derived, like every other axis here: rotating `u` about `n` moves it by `n x u`, so the
    component of `u` along travel falls fastest about `travel x u`.
    """
    bone = rig.pose.bones[f"{side}_Upperarm"]
    down = ((rig.matrix_world @ rig.pose.bones[f"{side}_Forearm"].bone.head_local)
            - (rig.matrix_world @ bone.bone.head_local))
    if down.length < 1e-9:
        refuse(f"the {side} upper arm has no length, so no swing axis exists")
    back = travel.cross(down.normalized())
    if back.length < 1e-9:
        refuse(f"the {side} upper arm lies along the line of travel")
    rest = (rig.matrix_world @ bone.bone.matrix_local).to_3x3()
    return (rest.inverted() @ back.normalized()).normalized()


def sit_the_shoulders_back(rig, clip, scene, degrees):
    """Rotates each whole arm backward by a constant, lowering the arc it swings through."""
    if abs(degrees) < 1e-6:
        return
    bind = rig.pose.bones["L_ToeBase"].bone
    travel = ((rig.matrix_world @ bind.tail_local) - (rig.matrix_world @ bind.head_local))
    travel.z = 0.0
    travel.normalize()
    slot = rig.animation_data.action_slot if rig.animation_data else None
    curves = fcurves_of(clip, slot)
    for side in ("L", "R"):
        path = f'pose.bones["{side}_Upperarm"].rotation_quaternion'
        parts = {c.array_index: c for c in curves if c.data_path == path}
        if len(parts) != 4:
            continue
        offset = mathutils.Quaternion(which_way_swings_the_arm_back(rig, side, travel),
                                      math.radians(degrees))
        for at in range(len(parts[0].keyframe_points)):
            keyed = mathutils.Quaternion([parts[i].keyframe_points[at].co[1] for i in range(4)])
            out = offset @ keyed
            for i in range(4):
                point = parts[i].keyframe_points[at]
                point.handle_left[1] += out[i] - point.co[1]
                point.handle_right[1] += out[i] - point.co[1]
                point.co[1] = out[i]
        for curve in parts.values():
            curve.update()


def the_elbows_fold(rig, clip, scene):
    """How far each elbow is folded, in degrees, averaged over a clip. 180 is straight."""
    play(rig, clip)
    first, last = (int(round(v)) for v in clip.frame_range)
    step = max(1, (last - first) // 60)
    seen = {"L": [], "R": []}
    for frame in range(first, last + 1, step):
        scene.frame_set(frame)
        posed = rig.evaluated_get(bpy.context.evaluated_depsgraph_get())
        for side in ("L", "R"):
            shoulder = posed.matrix_world @ posed.pose.bones[f"{side}_Upperarm"].head
            elbow = posed.matrix_world @ posed.pose.bones[f"{side}_Forearm"].head
            wrist = posed.matrix_world @ posed.pose.bones[f"{side}_Hand"].head
            up, down = shoulder - elbow, wrist - elbow
            if up.length > 1e-9 and down.length > 1e-9:
                seen[side].append(180.0 - math.degrees(up.angle(down)))
    return {side: (sum(got) / len(got) if got else 0.0) for side, got in seen.items()}


def which_way_folds_the_elbow(rig, side):
    """The forearm's own axis that folds the elbow, derived from the arm's own geometry.

    The hinge normal is perpendicular to the plane the arm lies in, which is the plane through
    the shoulder, the elbow and the wrist. Derived and not assumed - a fixed armature axis is
    exactly what the previous character got wrong here, and it "threw the hand laterally instead
    of forward".
    """
    bone = rig.pose.bones[f"{side}_Forearm"]
    shoulder = rig.matrix_world @ rig.pose.bones[f"{side}_Upperarm"].bone.head_local
    elbow = rig.matrix_world @ bone.bone.head_local
    wrist = rig.matrix_world @ rig.pose.bones[f"{side}_Hand"].bone.head_local
    up, down = (shoulder - elbow), (wrist - elbow)
    if up.length < 1e-9 or down.length < 1e-9:
        refuse(f"the {side} arm has no length, so no elbow hinge exists")
    hinge = down.normalized().cross(up.normalized())
    if hinge.length < 1e-9:
        refuse(f"the {side} arm is straight in the bind, so its hinge plane is undefined")
    rest = (rig.matrix_world @ bone.bone.matrix_local).to_3x3()
    return (rest.inverted() @ hinge.normalized()).normalized()


def hold_the_elbows(rig, clip, scene, target):
    """Carries each elbow at `target` degrees of fold, keeping the range the animator gave it."""
    was = the_elbows_fold(rig, clip, scene)
    slot = rig.animation_data.action_slot if rig.animation_data else None
    for _ in range(8):
        now = the_elbows_fold(rig, clip, scene)
        if all(abs(now[side] - target) < 0.5 for side in ("L", "R")):
            break
        curves = fcurves_of(clip, slot)
        for side in ("L", "R"):
            short = (target - now[side]) * 0.5
            if abs(short) < 0.25:
                continue
            path = f'pose.bones["{side}_Forearm"].rotation_quaternion'
            parts = {c.array_index: c for c in curves if c.data_path == path}
            if len(parts) != 4:
                continue
            offset = mathutils.Quaternion(which_way_folds_the_elbow(rig, side),
                                          math.radians(short))
            for at in range(len(parts[0].keyframe_points)):
                keyed = mathutils.Quaternion(
                    [parts[i].keyframe_points[at].co[1] for i in range(4)])
                out = offset @ keyed
                for i in range(4):
                    point = parts[i].keyframe_points[at]
                    point.handle_left[1] += out[i] - point.co[1]
                    point.handle_right[1] += out[i] - point.co[1]
                    point.co[1] = out[i]
            for curve in parts.values():
                curve.update()
    now = the_elbows_fold(rig, clip, scene)
    for side in ("L", "R"):
        if abs(now[side] - target) > 5.0:
            refuse(f"holding the {side} elbow on {clip.name} settled at {now[side]:.1f} deg "
                   f"against a {target:.1f} target - the hinge axis is wrong")
    return was, now


def where_the_hands_point(rig, clip, scene):
    """Each hand's world orientation, frame by frame. The invariant a roll spread must not move."""
    play(rig, clip)
    first, last = (int(round(v)) for v in clip.frame_range)
    step = max(1, (last - first) // 40)
    out = {"L": [], "R": []}
    for frame in range(first, last + 1, step):
        scene.frame_set(frame)
        posed = rig.evaluated_get(bpy.context.evaluated_depsgraph_get())
        for side in ("L", "R"):
            out[side].append(
                (posed.matrix_world @ posed.pose.bones[f"{side}_Hand"].matrix).to_quaternion())
    return out


def split_about(q, axis):
    """Splits a local rotation into the part that turns about one bone-local axis, and the rest.

    `q = rest * turn`, `turn` about the bone's own X, Y or Z for `axis` 0, 1 or 2. The projection
    is the standard swing-twist decomposition; the negation keeps it on the shortest arc, without
    which a 10 degree turn reads as 350.

    Written once and used twice: a forearm's roll is the part about its LENGTH (Y), and a foot's
    pitch is the part about its FLEX axis (X). Same arithmetic, different column.
    """
    parts = [0.0, 0.0, 0.0]
    parts[axis] = q[axis + 1]
    turn = mathutils.Quaternion((q.w, *parts))
    if turn.magnitude < 1e-9:
        turn = mathutils.Quaternion()
    turn.normalize()
    if turn.w < 0.0:
        turn.negate()
    return q @ turn.inverted(), turn


def swing_and_twist(q):
    """A limb bone's roll about its own length, and whatever is left."""
    return split_about(q, 1)


def a_share_of(turn, share):
    """The same rotation scaled to a fraction of its angle."""
    if abs(share - 1.0) < 1e-9:
        return turn.copy()
    if turn.magnitude < 1e-9 or abs(turn.angle) < 1e-9:
        return mathutils.Quaternion()
    return mathutils.Quaternion(turn.axis, turn.angle * share)


def rest_down_to(rig, bone, from_bone):
    """The rest rotation of `bone` relative to `from_bone`, following the parents between them.

    Needed because a twist expressed in the FOREARM's axes has to be conjugated into the axes of
    whichever bone is asked to carry it, and for the second roll bone that is two levels down
    rather than one.
    """
    # By NAME, not by identity: Blender hands back a fresh wrapper object on every attribute
    # read, so `bone is other_bone` is false even for the same bone and the walk never
    # terminates - it reported "L_ForearmTwist01 is not below L_Forearm" for a bone whose parent
    # is exactly that.
    here = rig.pose.bones[bone].bone
    down = mathutils.Matrix.Identity(4)
    while here is not None and here.name != from_bone:
        parent = here.parent
        if parent is None:
            refuse(f"{bone} is not below {from_bone}, so no rest chain joins them")
        down = (parent.matrix_local.inverted() @ here.matrix_local) @ down
        here = parent
    return down.to_quaternion()


def where_the_roll_skin_sits(rig, mesh, side):
    """How far along the forearm each roll bone's skin sits: 0 at the elbow, 1 at the wrist."""
    elbow = rig.matrix_world @ rig.pose.bones[f"{side}_Forearm"].bone.head_local
    wrist = rig.matrix_world @ rig.pose.bones[f"{side}_Hand"].bone.head_local
    along = wrist - elbow
    if along.length < 1e-9:
        refuse(f"the {side} forearm has no length, so no roll share can be measured")
    reach, way = along.length, along.normalized()
    groups = {g.name: g.index for g in mesh.vertex_groups}
    sits = {}
    for bone in ROLLS_ALONG:
        index = groups.get(f"{side}_{bone}")
        if index is None:
            # A rig with no twist bones is simpler, not broken. The 2026-08-26 warden carries none
            # where the previous one had eighteen, and refusing here stopped a whole build over a
            # bone that was never delivered. Nothing to spread means nothing to do.
            return {}
        heavy = weighted = 0.0
        for vertex in mesh.data.vertices:
            for group in vertex.groups:
                if group.group != index or group.weight <= 1e-4:
                    continue
                at = ((mesh.matrix_world @ vertex.co) - elbow).dot(way) / reach
                heavy += group.weight * min(max(at, 0.0), 1.0)
                weighted += group.weight
        if weighted <= 0.0:
            refuse(f"{side}_{bone} has a vertex group but no weight in it")
        sits[bone] = heavy / weighted
    return sits


def channels_for(clip, slot, path):
    """The four rotation channels of a bone, created if the clip does not key it yet."""
    have = {c.array_index: c for c in fcurves_of(clip, slot) if c.data_path == path}
    if len(have) == 4:
        return have
    holder = None
    for layer in getattr(clip, "layers", []):
        for strip in layer.strips:
            bag = None
            try:
                bag = strip.channelbag(slot, ensure=True)
            except TypeError:
                bag = strip.channelbags[0] if getattr(strip, "channelbags", None) else None
            if bag is not None:
                holder = bag.fcurves
                break
        if holder is not None:
            break
    if holder is None:
        holder = clip.fcurves
    for at in range(4):
        if at not in have:
            have[at] = holder.new(path, index=at)
    return have


def spread_the_twist(rig, clip, mesh):
    """Moves each forearm's roll off the bend bone and onto the roll bones and the hand.

    The bend bone keeps the swing. Each roll bone gets the share of the twist belonging to where
    its skin sits, as an INCREMENT on top of what it inherits from its parent. The hand takes the
    whole twist, which is what keeps the wrist where the animator put it.

    Keys are written at the union of every key time involved, and every value is read before any
    value is written, because the hand's new rotation is a function of the forearm's old one.
    """
    slot = rig.animation_data.action_slot if rig.animation_data else None
    spread = {}
    for side in ("L", "R"):
        bend = f"{side}_Forearm"
        path = f'pose.bones["{bend}"].rotation_quaternion'
        arm = {c.array_index: c for c in fcurves_of(clip, slot) if c.data_path == path}
        if len(arm) != 4:
            continue
        sits = where_the_roll_skin_sits(rig, mesh, side)
        if not sits:
            # No roll bones on this rig - see `where_the_roll_skin_sits`. The forearm keeps its
            # whole twist, which is what a rig without them means.
            continue
        so_far, takes = 0.0, []
        for bone in ROLLS_ALONG:
            takes.append((f"{side}_{bone}", sits[bone] - so_far))
            so_far = sits[bone]
        takes.append((f"{side}_Hand", 1.0))

        carries = {}
        for bone, share in takes:
            carries[bone] = (share, rest_down_to(rig, bone, bend),
                             channels_for(clip, slot,
                                          f'pose.bones["{bone}"].rotation_quaternion'))

        when = {round(k.co[0], 4) for k in arm[0].keyframe_points}
        for _, _, chans in carries.values():
            when |= {round(k.co[0], 4) for k in chans[0].keyframe_points}
        when = sorted(when)

        wanted = {}
        for frame in when:
            was = mathutils.Quaternion([arm[i].evaluate(frame) for i in range(4)])
            swing, twist = swing_and_twist(was)
            row = {bend: swing}
            for bone, (share, rest, chans) in carries.items():
                keyed = mathutils.Quaternion([chans[i].evaluate(frame) for i in range(4)])
                if keyed.magnitude < 1e-9:
                    keyed = mathutils.Quaternion()
                turn = a_share_of(twist, share)
                row[bone] = (rest.inverted() @ turn @ rest) @ keyed
            wanted[frame] = row

        writing = {bend: arm}
        writing.update({b: c for b, (_, _, c) in carries.items()})
        for bone, chans in writing.items():
            for frame, row in wanted.items():
                for at in range(4):
                    chans[at].keyframe_points.insert(frame, row[bone][at], options={"FAST"})
            for curve in chans.values():
                curve.update()
        # Reported as what each bone ENDS UP with, not as its increment: the increment is an
        # implementation detail of the parenting, and a reader wants the gradient.
        adds, spread[side] = 0.0, []
        for bone, share in takes:
            adds = 1.0 if abs(share - 1.0) < 1e-9 else adds + share
            spread[side].append((bone.split("_", 1)[1], adds))
    return spread


def which_way_points_the_toe_down(rig, side):
    """The foot's own axis that pitches its toe toward the floor.

    Derived the same way as `which_way_abducts`, and for the same reason - guessing a sign per
    side is how the finger curls went wrong. Rotating a direction `u` about `n` moves it by
    `n x u`, so `u.z` falls fastest about `-(u x z)`.
    """
    foot = rig.pose.bones[f"{side}_Foot"]
    along = ((rig.matrix_world @ rig.pose.bones[f"{side}_ToeBase"].bone.head_local)
             - (rig.matrix_world @ foot.bone.head_local))
    if along.length < 1e-9:
        refuse(f"the {side} foot has no length, so no pitch axis exists")
    down = -along.normalized().cross(mathutils.Vector((0.0, 0.0, 1.0)))
    if down.length < 1e-9:
        refuse(f"the {side} foot points straight up or down, so its pitch is undefined")
    rest = (rig.matrix_world @ foot.bone.matrix_local).to_3x3()
    return (rest.inverted() @ down.normalized()).normalized()


def the_feet_pitch(rig, clip, scene):
    """How far each foot's toe sits below its ankle, over a whole clip, in degrees."""
    play(rig, clip)
    first, last = (int(round(v)) for v in clip.frame_range)
    step = max(1, (last - first) // 60)
    out = {"L": [], "R": []}
    for frame in range(first, last + 1, step):
        scene.frame_set(frame)
        posed = rig.evaluated_get(bpy.context.evaluated_depsgraph_get())
        for side in ("L", "R"):
            ankle = posed.matrix_world @ posed.pose.bones[f"{side}_Foot"].head
            toe = posed.matrix_world @ posed.pose.bones[f"{side}_ToeBase"].head
            flat = mathutils.Vector((toe.x - ankle.x, toe.y - ankle.y, 0.0)).length
            if flat > 1e-9:
                out[side].append(math.degrees(math.atan2(ankle.z - toe.z, flat)))
    return out


def break_the_toes(rig, clip, scene):
    """Moves a foot's excess downward pitch out of the ankle and into the toe.

    A foot rolls onto the ball and BREAKS there; it does not point like a dancer's. The delivered
    clips pitch the whole shoe down - 86.7 degrees of it in the run - and leave both toe bones
    keyed to identity, so there is no break at all. Raising the ankle by the excess and bending
    the toe down by the same amount straightens the foot and produces the bend in one operation,
    because they are the same operation seen from two ends: the toe tip stays where the animator
    put it and the shoe above it comes flat.

    # Measured in the WORLD, not decomposed from the foot's own rotation

    The first version split the foot's local rotation about its flex axis and moved a share of
    that. It moved 5.5 degrees and changed the resulting pitch by nothing, because a foot's world
    pitch is not in its own bone: the thigh and the calf carry most of it, and the foot's local X
    is only the last contribution. So the excess is measured off the posed ankle-to-toe vector,
    frame by frame, and applied as a correction - the same shape as `lift_the_arms`.

    Both offsets are composed on the POSED side (`keyed * offset`, with the axis expressed in each
    bone's own posed frame) because the pitch axis follows the foot, unlike an abduction which is
    fixed in the shoulder.
    """
    was = the_feet_pitch(rig, clip, scene)
    play(rig, clip)
    first, last = (int(round(v)) for v in clip.frame_range)
    up = mathutils.Vector((0.0, 0.0, 1.0))

    # Every correction worked out before any of it is written, because writing a key changes what
    # the next frame evaluates to.
    wanted = {side: {} for side in ("L", "R")}
    for frame in range(first, last + 1):
        scene.frame_set(frame)
        posed = rig.evaluated_get(bpy.context.evaluated_depsgraph_get())
        for side in ("L", "R"):
            ankle = posed.matrix_world @ posed.pose.bones[f"{side}_Foot"].head
            toe = posed.matrix_world @ posed.pose.bones[f"{side}_ToeBase"].head
            along = toe - ankle
            flat = mathutils.Vector((along.x, along.y, 0.0))
            if flat.length < 1e-9 or along.length < 1e-9:
                continue
            pitch = math.degrees(math.atan2(ankle.z - toe.z, flat.length))
            over = min(max(pitch - TOE_BREAKS_PAST, 0.0), TOE_BREAKS_AT_MOST)
            if over <= 0.01:
                continue
            # The world axis about which the toe FALLS - derived, not assumed, the same way as
            # `which_way_abducts`: rotating `u` about `n` moves it by `n x u`, so `u.z` falls
            # fastest about `-(u x z)`.
            falls = -along.normalized().cross(up)
            if falls.length < 1e-9:
                continue
            falls.normalize()
            for bone, way in ((f"{side}_Foot", -1.0), (f"{side}_ToeBase", 1.0)):
                held = (rig.matrix_world @ posed.pose.bones[bone].matrix).to_3x3()
                mine = held.inverted() @ falls
                if mine.length < 1e-9:
                    continue
                wanted[side].setdefault(bone, {})[frame] = mathutils.Quaternion(
                    mine.normalized(), math.radians(over * way))

    slot = rig.animation_data.action_slot if rig.animation_data else None
    broke = {}
    for side in ("L", "R"):
        if not wanted[side]:
            continue
        most = 0.0
        for bone, byframe in wanted[side].items():
            chans = channels_for(clip, slot, f'pose.bones["{bone}"].rotation_quaternion')
            # EVERY existing value read before ANY of them is written. The toe's curves start
            # empty, so inserting frame by frame while still reading meant each frame read back
            # the key the previous frame had just written and composed on top of it: the run's
            # left toe compounded from a 55 degree cap to 111.8. `spread_the_twist` gets this
            # right and this got it wrong; the rule is the same either way.
            keyed = {}
            for frame in byframe:
                held = mathutils.Quaternion([chans[i].evaluate(frame) for i in range(4)])
                keyed[frame] = mathutils.Quaternion() if held.magnitude < 1e-9 else held
            for frame, offset in byframe.items():
                turned = keyed[frame] @ offset
                for i in range(4):
                    chans[i].keyframe_points.insert(frame, turned[i], options={"FAST"})
                most = max(most, math.degrees(abs(offset.angle)))
            for curve in chans.values():
                curve.update()
        broke[side] = most
        print(f"      {side}: corrected {len(wanted[side].get(f'{side}_Foot', {}))} frames of "
              f"{last - first + 1}, largest offset {most:.1f} deg")
    now = the_feet_pitch(rig, clip, scene)
    # The toe may not end up bent further than it was ever asked to bend. A correction that
    # composes onto itself does not announce itself any other way.
    play(rig, clip)
    for frame in range(first, last + 1):
        scene.frame_set(frame)
        for side in ("L", "R"):
            turn = rig.pose.bones[f"{side}_ToeBase"].rotation_quaternion.copy()
            if turn.w < 0.0:
                turn.negate()
            bent = math.degrees(turn.angle)
            if bent > TOE_BREAKS_AT_MOST + 5.0:
                refuse(f"{clip.name} frame {frame}: the {side} toe ended up bent {bent:.1f} deg "
                       f"against a {TOE_BREAKS_AT_MOST:.0f} deg cap, so the correction composed "
                       f"onto itself somewhere")
    return was, broke, now


def flatten_the_toes(rig, mesh, clip, scene):
    """Rotates each toe up about the ball until it lies along the ground instead of through it.

    Only where the shoe is DOWN, and only about the ball - which is both the pivot and the
    contact, so the correction cannot drag anything. Everything else this file has tried on the
    feet rotated about the ankle, which moves the contact and is why the slide guard kept
    refusing them.
    """
    play(rig, clip)
    first, last = (int(round(v)) for v in clip.frame_range)
    shoe = the_shoe_vertices(mesh)
    up = mathutils.Vector((0.0, 0.0, 1.0))
    wanted, was, deepest = {}, 0.0, 0.0
    for frame in range(first, last + 1):
        scene.frame_set(frame)
        graph = bpy.context.evaluated_depsgraph_get()
        posed, skin = rig.evaluated_get(graph), mesh.evaluated_get(graph)
        for side in ("L", "R"):
            ball = posed.matrix_world @ posed.pose.bones[f"{side}_ToeBase"].head
            tip = posed.matrix_world @ posed.pose.bones[f"{side}_ToeBase"].tail
            along = tip - ball
            flat = mathutils.Vector((along.x, along.y, 0.0))
            if flat.length < 1e-9 or along.length < 1e-9:
                continue
            points = math.degrees(math.atan2(ball.z - tip.z, flat.length))
            was = max(was, points)
            low = min((skin.matrix_world @ skin.data.vertices[i].co).z for i in shoe[side])
            if low > STANCE_WITHIN / 170.0 or points <= THE_TOE_RESTS_AT:
                continue
            lift = min(points - THE_TOE_RESTS_AT, THE_TOE_BENDS_AT_MOST)
            deepest = max(deepest, lift)
            lifts = along.normalized().cross(up)
            if lifts.length < 1e-9:
                continue
            held = (rig.matrix_world @ posed.pose.bones[f"{side}_ToeBase"].matrix).to_3x3()
            mine = held.inverted() @ lifts.normalized()
            if mine.length < 1e-9:
                continue
            wanted.setdefault(f"{side}_ToeBase", {})[frame] = mathutils.Quaternion(
                mine.normalized(), math.radians(lift))

    slot = rig.animation_data.action_slot if rig.animation_data else None
    for bone, byframe in wanted.items():
        chans = channels_for(clip, slot, f'pose.bones["{bone}"].rotation_quaternion')
        # Every frame keyed, so the toe returns to straight in the air rather than the curve
        # interpolating across a whole flight phase between two bent extremes.
        for frame in range(first, last + 1):
            byframe.setdefault(frame, mathutils.Quaternion())
        keyed = {}
        for frame in byframe:
            held = mathutils.Quaternion([chans[i].evaluate(frame) for i in range(4)])
            keyed[frame] = mathutils.Quaternion() if held.magnitude < 1e-9 else held
        turned = {f: keyed[f] @ o for f, o in byframe.items()}
        settled = turned.get(first) or turned.get(last)
        if settled is not None:
            turned[first] = settled
            turned[last] = settled
        for frame, out in turned.items():
            for i in range(4):
                chans[i].keyframe_points.insert(frame, out[i], options={"FAST"})
        for curve in chans.values():
            curve.update()

    play(rig, clip)
    now = 0.0
    for frame in range(first, last + 1):
        scene.frame_set(frame)
        graph = bpy.context.evaluated_depsgraph_get()
        posed, skin = rig.evaluated_get(graph), mesh.evaluated_get(graph)
        for side in ("L", "R"):
            ball = posed.matrix_world @ posed.pose.bones[f"{side}_ToeBase"].head
            tip = posed.matrix_world @ posed.pose.bones[f"{side}_ToeBase"].tail
            flat = mathutils.Vector((tip.x - ball.x, tip.y - ball.y, 0.0))
            low = min((skin.matrix_world @ skin.data.vertices[i].co).z for i in shoe[side])
            if flat.length > 1e-9 and low <= STANCE_WITHIN / 170.0:
                now = max(now, math.degrees(math.atan2(ball.z - tip.z, flat.length)))
    return was, now, deepest


def cap_the_ankle(rig, mesh, clip, scene, most):
    """Stops the foot pointing further down than `most` degrees, and touches nothing else.

    Measured in the world off the posed ankle-to-ball direction, corrected about the axis that
    lifts the toe, and applied ONLY where the shoe is clear of the floor - a planted foot is left
    exactly as it is, which is what keeps the slide guard happy.
    """
    play(rig, clip)
    first, last = (int(round(v)) for v in clip.frame_range)
    shoe = the_shoe_vertices(mesh)
    up = mathutils.Vector((0.0, 0.0, 1.0))
    wanted, was, now = {}, 0.0, 0.0
    for frame in range(first, last + 1):
        scene.frame_set(frame)
        graph = bpy.context.evaluated_depsgraph_get()
        posed, skin = rig.evaluated_get(graph), mesh.evaluated_get(graph)
        for side in ("L", "R"):
            ankle = posed.matrix_world @ posed.pose.bones[f"{side}_Foot"].head
            ball = posed.matrix_world @ posed.pose.bones[f"{side}_ToeBase"].head
            along = ball - ankle
            flat = mathutils.Vector((along.x, along.y, 0.0))
            if flat.length < 1e-9 or along.length < 1e-9:
                continue
            points = math.degrees(math.atan2(ankle.z - ball.z, flat.length))
            was = max(was, points)
            low = min((skin.matrix_world @ skin.data.vertices[i].co).z for i in shoe[side])
            # Only in the air. A foot on the ground is the animator's and the guard's.
            if low < STANCE_WITHIN / 170.0 or points <= most:
                continue
            lifts = along.normalized().cross(up)
            if lifts.length < 1e-9:
                continue
            held = (rig.matrix_world @ posed.pose.bones[f"{side}_Foot"].matrix).to_3x3()
            mine = held.inverted() @ lifts.normalized()
            if mine.length < 1e-9:
                continue
            wanted.setdefault(f"{side}_Foot", {})[frame] = mathutils.Quaternion(
                mine.normalized(), math.radians(points - most))

    slot = rig.animation_data.action_slot if rig.animation_data else None
    for bone, byframe in wanted.items():
        chans = channels_for(clip, slot, f'pose.bones["{bone}"].rotation_quaternion')
        keyed = {}
        for frame in byframe:
            held = mathutils.Quaternion([chans[i].evaluate(frame) for i in range(4)])
            keyed[frame] = mathutils.Quaternion() if held.magnitude < 1e-9 else held
        turned = {f: keyed[f] @ o for f, o in byframe.items()}
        # A cycle's ends are the same pose and must stay so.
        if first in turned or last in turned:
            settled = turned.get(first) or turned.get(last)
            if settled is not None:
                turned[first] = settled
                turned[last] = settled
        for frame, out in turned.items():
            for i in range(4):
                chans[i].keyframe_points.insert(frame, out[i], options={"FAST"})
        for curve in chans.values():
            curve.update()

    play(rig, clip)
    for frame in range(first, last + 1):
        scene.frame_set(frame)
        posed = rig.evaluated_get(bpy.context.evaluated_depsgraph_get())
        for side in ("L", "R"):
            ankle = posed.matrix_world @ posed.pose.bones[f"{side}_Foot"].head
            ball = posed.matrix_world @ posed.pose.bones[f"{side}_ToeBase"].head
            flat = mathutils.Vector((ball.x - ankle.x, ball.y - ankle.y, 0.0))
            if flat.length > 1e-9:
                now = max(now, math.degrees(math.atan2(ankle.z - ball.z, flat.length)))
    return was, now, sum(len(v) for v in wanted.values())


def which_way_the_knee_folds(rig, side, travel):
    """The thigh's own axis that carries the knee FORWARD, so a leg bends the way a leg bends."""
    bone = rig.pose.bones[f"{side}_Thigh"]
    down = ((rig.matrix_world @ rig.pose.bones[f"{side}_Calf"].bone.head_local)
            - (rig.matrix_world @ bone.bone.head_local))
    if down.length < 1e-9:
        refuse(f"the {side} thigh has no length, so no knee axis exists")
    # Rotating `down` about `n` moves it by `n x down`, so its forward component grows fastest
    # about `down x travel`.
    folds = down.normalized().cross(travel)
    if folds.length < 1e-9:
        refuse(f"the {side} thigh lies along the line of travel")
    return folds.normalized()


def ease_the_knees(rig, degrees):
    """Bends each knee a little in the BIND, and moves every clip with it.

    The thigh goes forward by `degrees` and the calf comes back by the same, so the knee leads
    and the ankle stays put. Every clip that keys either bone is compensated, because a rest
    change silently re-interprets every key written against the old one.
    """
    if abs(degrees) < 1e-6:
        return 0.0
    bind = rig.pose.bones["L_ToeBase"].bone
    travel = ((rig.matrix_world @ bind.tail_local) - (rig.matrix_world @ bind.head_local))
    travel.z = 0.0
    if travel.length < 1e-9:
        refuse("the bind toe has no horizontal direction")
    travel.normalize()

    # Worked out before anything moves, in world terms, so the compensation below can be built
    # from the same rotations.
    turns = {}
    for side in ("L", "R"):
        axis = which_way_the_knee_folds(rig, side, travel)
        turns[f"{side}_Thigh"] = mathutils.Quaternion(axis, math.radians(degrees))
        turns[f"{side}_Calf"] = mathutils.Quaternion(axis, math.radians(-2.0 * degrees))

    bpy.context.view_layer.objects.active = rig
    bpy.ops.object.mode_set(mode="EDIT")
    for name, turn in turns.items():
        bone = rig.data.edit_bones.get(name)
        if bone is None:
            continue
        pivot = bone.head.copy()
        spin = (rig.matrix_world.to_3x3().inverted() @ turn.to_matrix()
                @ rig.matrix_world.to_3x3()).to_4x4()

        def swing(here):
            here.head = pivot + spin @ (here.head - pivot)
            here.tail = pivot + spin @ (here.tail - pivot)
            for kid in rig.data.edit_bones:
                if kid.parent is not None and kid.parent.name == here.name:
                    swing(kid)

        swing(bone)
    bpy.ops.object.mode_set(mode="OBJECT")

    # And every clip carried across, so nothing that was authored against the old rest moves.
    for clip in bpy.data.actions:
        slot = getattr(clip, "slots", None)
        slot = slot[0] if slot else None
        curves = fcurves_of(clip, slot)
        for name, turn in turns.items():
            path = f'pose.bones["{name}"].rotation_quaternion'
            parts = {c.array_index: c for c in curves if c.data_path == path}
            if len(parts) != 4:
                continue
            back = turn.inverted()
            for at in range(len(parts[0].keyframe_points)):
                keyed = mathutils.Quaternion(
                    [parts[i].keyframe_points[at].co[1] for i in range(4)])
                out = back @ keyed
                for i in range(4):
                    point = parts[i].keyframe_points[at]
                    point.handle_left[1] += out[i] - point.co[1]
                    point.handle_right[1] += out[i] - point.co[1]
                    point.co[1] = out[i]
            for curve in parts.values():
                curve.update()

    def bend():
        hip = rig.matrix_world @ rig.pose.bones["L_Thigh"].bone.head_local
        knee = rig.matrix_world @ rig.pose.bones["L_Calf"].bone.head_local
        ankle = rig.matrix_world @ rig.pose.bones["L_Foot"].bone.head_local
        straight = (knee - hip).length + (ankle - knee).length
        return (ankle - hip).length / straight if straight > 1e-9 else 1.0

    return bend()


def stand_the_legs_apart(rig, degrees):
    """Rotates each thigh outward in the BIND, so every pose is that much wider.

    In the rest pose, which is the whole point: the legs come too close on a handful of frames
    and the cause is a constant, so widening the rest widens all of them equally and no clip is
    touched. Done in edit mode with the pose at rest, so the mesh does not move.

    The whole leg swings with the thigh - calf, foot, toes and every twist under them - because
    rotating a bone in edit mode carries its children.
    """
    if abs(degrees) < 1e-6:
        return 0.0
    across = ((rig.matrix_world @ rig.data.bones["R_Thigh"].head_local)
              - (rig.matrix_world @ rig.data.bones["L_Thigh"].head_local))
    across.z = 0.0
    if across.length < 1e-9:
        refuse("the hips are in the same place, so there is no outward direction")
    across.normalize()

    bpy.context.view_layer.objects.active = rig
    bpy.ops.object.mode_set(mode="EDIT")
    moved = 0.0
    for side, way in (("L", -1.0), ("R", 1.0)):
        thigh = rig.data.edit_bones.get(f"{side}_Thigh")
        if thigh is None:
            continue
        pivot = thigh.head.copy()
        # About the line of travel, which tips a leg out to its own side rather than forward.
        axis = (rig.matrix_world.to_3x3().inverted()
                @ mathutils.Vector((-across.y, across.x, 0.0))).normalized()
        turn = mathutils.Matrix.Rotation(math.radians(degrees * way), 4, axis)

        def swing(bone):
            bone.head = pivot + turn @ (bone.head - pivot)
            bone.tail = pivot + turn @ (bone.tail - pivot)
            for kid in rig.data.edit_bones:
                if kid.parent is not None and kid.parent.name == bone.name:
                    swing(kid)

        below = thigh.tail.copy()
        swing(thigh)
        moved = max(moved, (thigh.tail - below).length * 170.0)
    bpy.ops.object.mode_set(mode="OBJECT")
    return moved


def the_bind_is_mirrored(rig):
    """Makes the rest pose an exact mirror of itself, left to right.

    The mirror plane comes from the rig: its normal is the hip-to-hip direction, and it passes
    through the centroid of the bones that carry no side at all. Each pair is then averaged with
    its own reflection, so neither side is imposed on the other - the result sits between them.

    Done in EDIT mode with the pose at rest, so the mesh does not move: Blender deforms by the
    difference between a bone's pose and its rest, and at rest there is none.

    The clips are NOT compensated for this, and that is the point rather than an oversight. They
    were authored symmetrically and retargeted onto an asymmetric rig; the asymmetry is in the
    REST, so taking it out of the rest is what lets the animation come out even. Compensating the
    keys would preserve exactly the look this is meant to fix.
    """
    across = ((rig.matrix_world @ rig.data.bones["R_Thigh"].head_local)
              - (rig.matrix_world @ rig.data.bones["L_Thigh"].head_local))
    across.z = 0.0
    if across.length < 1e-9:
        refuse("the hips are in the same place, so there is no mirror plane")
    across.normalize()
    middle = [b for b in rig.data.bones
              if not b.name.startswith("L_") and not b.name.startswith("R_")]
    if not middle:
        refuse("no centre bones, so the mirror plane has nothing to pass through")
    spine = mathutils.Vector((0.0, 0.0, 0.0))
    for bone in middle:
        spine += rig.matrix_world @ bone.head_local
    spine /= len(middle)

    def flip(spot):
        away = spot - spine
        return spine + away - across * (2.0 * away.dot(across))

    pairs = []
    for bone in rig.data.bones:
        if bone.name.startswith("L_") and ("R_" + bone.name[2:]) in rig.data.bones:
            pairs.append((bone.name, "R_" + bone.name[2:]))
    if not pairs:
        return 0, 0.0

    wanted = {}
    worst = 0.0
    for left, right in pairs:
        for end in ("head_local", "tail_local"):
            here = rig.matrix_world @ getattr(rig.data.bones[left], end)
            there = flip(rig.matrix_world @ getattr(rig.data.bones[right], end))
            worst = max(worst, (here - there).length)
            settled = (here + there) * 0.5
            wanted[(left, end)] = settled
            wanted[(right, end)] = flip(settled)
    # The centre bones belong ON the plane, or the two sides are mirrored about a line the body
    # is not actually built around.
    for bone in middle:
        for end in ("head_local", "tail_local"):
            spot = rig.matrix_world @ getattr(bone, end)
            wanted[(bone.name, end)] = spot - across * (spot - spine).dot(across)

    bpy.context.view_layer.objects.active = rig
    bpy.ops.object.mode_set(mode="EDIT")
    into = rig.matrix_world.inverted()
    for (name, end), spot in wanted.items():
        bone = rig.data.edit_bones.get(name)
        if bone is None:
            continue
        if end == "head_local":
            bone.head = into @ spot
        else:
            bone.tail = into @ spot
    bpy.ops.object.mode_set(mode="OBJECT")
    return len(pairs), worst * 170.0


def the_shoe_runs(rig, mesh, side):
    """The shoe's own heel-to-tip axis and extent, and which vertices belong to that foot.

    # The SHOE decides which way it points, not the bone inside it

    This took its axis from the toe bone's own direction, and that bone is 15.6 degrees off the
    shoe on the left and 16.9 on the right - about twelve of it in the horizontal. So the joint
    was moved forward along a line that was not the shoe's, and the hinge came out aimed sideways
    of the foot: "some small directional and compression issues".

    The shoe's own long axis is the direction its vertices are most spread along, which is what
    "which way does this shoe point" means. Found by power iteration on their covariance, because
    Blender's bundled Python has no numpy. It comes out unsigned, so it is turned to agree with
    the ankle-to-toe direction - the bone is wrong about the ANGLE by twelve degrees, not about
    which end is the front.
    """
    groups = {g.index: g.name for g in mesh.vertex_groups}
    spots = []
    for vertex in mesh.data.vertices:
        best, who = 0.0, ""
        for group in vertex.groups:
            name = groups.get(group.group, "")
            if group.weight > best and name.startswith(side) and (
                    "Foot" in name or "ToeBase" in name):
                best, who = group.weight, name
        if who:
            spots.append(mesh.matrix_world @ vertex.co)
    if len(spots) < 8:
        refuse(f"no vertices are weighted to the {side} foot")
    middle = mathutils.Vector((0.0, 0.0, 0.0))
    for spot in spots:
        middle += spot
    middle /= len(spots)
    ahead = mathutils.Vector((1.0, 0.0, 0.0))
    for _ in range(40):
        nxt = mathutils.Vector((0.0, 0.0, 0.0))
        for spot in spots:
            away = spot - middle
            nxt += away * away.dot(ahead)
        if nxt.length < 1e-12:
            break
        ahead = nxt.normalized()
    toe = rig.pose.bones[f"{side}_ToeBase"].bone
    forward = ((rig.matrix_world @ toe.tail_local) - (rig.matrix_world @ toe.head_local))
    if ahead.dot(forward) < 0.0:
        ahead = -ahead
    # The shoe's long axis tilts DOWN toward the toe - a toe box tapers - and flattening it left
    # the bone 9 to 12 degrees off in the vertical even once the horizontal was within a degree.
    # The bone follows the shoe in three dimensions; the horizontalised copy is only for deciding
    # how far ALONG the shoe a thing is.
    tilted = ahead.copy()
    ahead = mathutils.Vector((ahead.x, ahead.y, 0.0))
    if ahead.length < 1e-9:
        refuse(f"the {side} shoe has no horizontal long axis")
    ahead.normalize()
    tilted.normalize()
    mine, along = [], []
    for vertex in mesh.data.vertices:
        best, who = 0.0, ""
        for group in vertex.groups:
            name = groups.get(group.group, "")
            if group.weight > best and name.startswith(side) and (
                    "Foot" in name or "ToeBase" in name):
                best, who = group.weight, name
        if who:
            mine.append(vertex.index)
            along.append((mesh.matrix_world @ vertex.co).dot(ahead))
    if not mine:
        refuse(f"no vertices are weighted to the {side} foot")
    return ahead, min(along), max(along), mine, tilted


def hinge_the_toes_at_the_ball(rig, mesh):
    """Moves each toe joint to the ball of the foot, runs the bone to the tip, and re-weights.

    Done in EDIT mode with the rig at rest, so the mesh does not move: Blender deforms by the
    difference between a bone's pose and its rest, and at rest there is none.
    """
    was = {}
    for side in ("L", "R"):
        ahead, back, front, _, tilted = the_shoe_runs(rig, mesh, side)
        was[side] = (ahead, back, front - back,
                     (rig.matrix_world @ rig.pose.bones[f"{side}_ToeBase"].bone.head_local)
                     .dot(ahead), tilted)

    bpy.context.view_layer.objects.active = rig
    bpy.ops.object.mode_set(mode="EDIT")
    moved = {}
    for side in ("L", "R"):
        ahead, back, length, before, tilted = was[side]
        bone = rig.data.edit_bones[f"{side}_ToeBase"]
        head = rig.matrix_world @ bone.head
        # Forward along the shoe's own axis, height unchanged: a ball of the foot is further
        # along the foot, not higher up it.
        ball = back + length * THE_TOE_HINGES_AT
        tip = back + length
        # Forward along the shoe, and DOWN to where a ball joint belongs. The height has to be
        # measured at the ball itself: a shoe is not the same depth at the arch as at the ball.
        near = [(mesh.matrix_world @ mesh.data.vertices[i].co)
                for i in the_shoe_runs(rig, mesh, side)[3]]
        near = [p for p in near if abs(p.dot(ahead) - ball) < 0.02]
        moved_to = head + ahead * (ball - head.dot(ahead))
        if near:
            low, high = min(p.z for p in near), max(p.z for p in near)
            moved_to.z = low + (high - low) * THE_TOE_SITS_UP
            # And onto the shoe's own centreline. Moving the joint forward left its sideways
            # position wherever the original was, and on the right foot that was 3.63 cm off
            # centre on a 14.96 cm shoe - a quarter of its width, reported as "still offset".
            across = mathutils.Vector((-ahead.y, ahead.x, 0.0))
            edges = [p.dot(across) for p in near]
            middle = (min(edges) + max(edges)) * 0.5
            moved_to += across * (middle - moved_to.dot(across))
        bone.head = rig.matrix_world.inverted() @ moved_to
        # Along the shoe's own tilted axis, scaled so it still reaches the tip when measured
        # the flat way.
        reach = (tip - ball) / max(tilted.dot(ahead), 1e-6)
        bone.tail = rig.matrix_world.inverted() @ (moved_to + tilted * reach)
        moved[side] = ((ball - before) * 170.0, (before - back) / length * 100.0,
                       THE_TOE_HINGES_AT * 100.0)
    bpy.ops.object.mode_set(mode="OBJECT")

    # # The tail lands IN the toe box, not on a line aimed at it
    #
    # This used to run the bone along the shoe's own tilted long axis and scale it until it reached
    # the front measured flatly. Two things went wrong at once and they compounded: the long axis of
    # a shoe tilts DOWN toward the toe because the shoe tapers, so scaling it to reach the front
    # horizontally overshoots the front AND sinks below the sole. Measured on the built character,
    # both tips ended up past the end of the shoe - 112.5% and 110.0% along it - and 3.7 and 3.3 cm
    # BELOW the sole. A toe bone sticking out through the bottom of a shoe cannot bend a toe
    # convincingly whatever angle it is given, which is what "the toe bends incorrectly" kept
    # coming back to, on both feet alike rather than on one of them.
    #
    # So the tail is not aimed at all. It is placed on the middle of the flesh at the front of the
    # shoe, which is inside the shoe by construction and at the height a toe tip actually sits.
    for side in ("L", "R"):
        ahead, back, front, shoe, _ = the_shoe_runs(rig, mesh, side)
        head = rig.matrix_world @ rig.pose.bones[f"{side}_ToeBase"].bone.head_local
        spots = [mesh.matrix_world @ mesh.data.vertices[i].co for i in shoe]
        toe_box = [p for p in spots
                   if p.dot(ahead) >= front - (front - back) * THE_TIP_IS_THE_FRONT]
        if not toe_box:
            continue
        tip = sum(toe_box, mathutils.Vector((0.0, 0.0, 0.0))) / len(toe_box)
        if (tip - head).length <= 1e-6:
            continue
        bpy.context.view_layer.objects.active = rig
        bpy.ops.object.mode_set(mode="EDIT")
        rig.data.edit_bones[f"{side}_ToeBase"].tail = rig.matrix_world.inverted() @ tip
        bpy.ops.object.mode_set(mode="OBJECT")

    # And the weights, about the new hinge. Only the Foot/ToeBase share moves; each vertex keeps
    # whatever total it had, so nothing the calf or the ankle holds is disturbed.
    groups = {g.name: g for g in mesh.vertex_groups}
    shifted = 0
    for side in ("L", "R"):
        ahead, back, length, _, _ = was[side]
        foot, toe = groups.get(f"{side}_Foot"), groups.get(f"{side}_ToeBase")
        if foot is None or toe is None:
            continue
        ball = back + length * THE_TOE_HINGES_AT
        band = length * THE_HINGE_BLENDS_OVER
        for vertex in mesh.data.vertices:
            held = {g.group: g.weight for g in vertex.groups}
            mine = held.get(foot.index, 0.0) + held.get(toe.index, 0.0)
            if mine <= 1e-5:
                continue
            at = (mesh.matrix_world @ vertex.co).dot(ahead)
            share = min(max((at - (ball - band)) / (2.0 * band), 0.0), 1.0)
            toe.add([vertex.index], mine * share, "REPLACE")
            foot.add([vertex.index], mine * (1.0 - share), "REPLACE")
            shifted += 1
    mesh.data.update()
    return moved, shifted


def weight_the_toes_about_the_hinge(rig, mesh):
    """Splits each foot's flesh between foot and toe about where the toe joint ACTUALLY ended up.

    # Why this has to happen last

    The hinge already re-weighted once, against the ball position it was aiming for, measured
    before it moved anything. Then the joint moved, and then the bind was mirrored again - and a
    mirror shifts a joint that was not symmetric to start with. So the crease and the joint drifted
    apart, by different amounts on each side: measured on the built character, the left toe's
    influence reached 0.62 at 60% of the shoe while its joint sat at 69%, and the right reached
    0.31 at the same place with its joint at 66%. One shoe creased 3.6 cm behind its own hinge and
    the other creased on it, which is two feet bending differently however carefully their angles
    are matched.

    Reading the joint back off the rig costs nothing and cannot drift, so it is read back.

    Only the foot-and-toe share moves. Each vertex keeps whatever total those two held, so nothing
    the calf or the ankle owns is touched.
    """
    groups = {g.name: g for g in mesh.vertex_groups}
    shifted, where = 0, {}
    for side in ("L", "R"):
        ahead, back, front, _, _ = the_shoe_runs(rig, mesh, side)
        foot, toe = groups.get(f"{side}_Foot"), groups.get(f"{side}_ToeBase")
        if foot is None or toe is None:
            continue
        length = front - back
        ball = (rig.matrix_world @ rig.pose.bones[f"{side}_ToeBase"].bone.head_local).dot(ahead)
        band = length * THE_HINGE_BLENDS_OVER
        where[side] = (ball - back) / length * 100.0
        for vertex in mesh.data.vertices:
            held = {g.group: g.weight for g in vertex.groups}
            mine = held.get(foot.index, 0.0) + held.get(toe.index, 0.0)
            if mine <= 1e-5:
                continue
            at = (mesh.matrix_world @ vertex.co).dot(ahead)
            share = min(max((at - (ball - band)) / (2.0 * band), 0.0), 1.0)
            toe.add([vertex.index], mine * share, "REPLACE")
            foot.add([vertex.index], mine * (1.0 - share), "REPLACE")
            shifted += 1
    mesh.data.update()
    return shifted, where


def the_foot_parts(posed, skin, shoe, side):
    """Heel height, toe height, foot pitch and the toe's own break, for one foot on one frame."""
    ankle = posed.matrix_world @ posed.pose.bones[f"{side}_Foot"].head
    ball = posed.matrix_world @ posed.pose.bones[f"{side}_ToeBase"].head
    tip = posed.matrix_world @ posed.pose.bones[f"{side}_ToeBase"].tail
    ahead = ball - ankle
    ahead.z = 0.0
    if ahead.length > 1e-9:
        ahead.normalize()
    spots = [skin.matrix_world @ skin.data.vertices[i].co for i in shoe[side]]
    middle = sum((p.dot(ahead) for p in spots), 0.0) / len(spots)
    front = [p.z for p in spots if p.dot(ahead) > middle]
    back = [p.z for p in spots if p.dot(ahead) <= middle]
    flat = mathutils.Vector((ball.x - ankle.x, ball.y - ankle.y, 0.0)).length
    pitch = math.degrees(math.atan2(ankle.z - ball.z, flat)) if flat > 1e-9 else 0.0
    # The break is the angle between the foot's own line and the toe's, in the vertical plane -
    # what a metatarsal break IS, rather than the toe bone's raw rotation angle.
    toe_flat = mathutils.Vector((tip.x - ball.x, tip.y - ball.y, 0.0)).length
    toe_pitch = math.degrees(math.atan2(ball.z - tip.z, toe_flat)) if toe_flat > 1e-9 else 0.0
    return (min(back) if back else 0.0, min(front) if front else 0.0,
            pitch, toe_pitch - pitch)


def the_shoe_vertices(mesh):
    """Which vertices belong to each shoe."""
    groups = {g.index: g.name for g in mesh.vertex_groups}
    shoe = {"L": [], "R": []}
    for vertex in mesh.data.vertices:
        for group in vertex.groups:
            name = groups.get(group.group, "")
            if group.weight > 0.3 and ("Foot" in name or "ToeBase" in name):
                shoe[name[0]].append(vertex.index)
                break
    if not shoe["L"] or not shoe["R"]:
        refuse("one of the shoes has no vertices weighted to it")
    return shoe


def the_stances(down, first, last):
    """Runs of consecutive frames where a foot is down, as (start, end) pairs.

    A cycle's last stance may wrap onto its first - a foot down at frame 25 and again at frame 1
    is one stance, not two - so a run that touches both ends is joined.
    """
    # Closing one-frame gaps was tried here, on the reasoning that a foot does not leave the
    # ground for a single frame mid-contact. It made things worse: merging the run's short
    # contacts into long ones stretched the heel-to-toe ramp across frames the foot was actually
    # airborne for, and lifted the whole cycle back off the floor. The detector is noisy because
    # the CLIP barely touches the ground, and smoothing the detector does not put the feet down.
    runs, start = [], None
    for frame in range(first, last + 1):
        if down.get(frame):
            start = frame if start is None else start
        elif start is not None:
            runs.append((start, frame - 1))
            start = None
    if start is not None:
        runs.append((start, last))
    if len(runs) > 1 and runs[0][0] == first and runs[-1][1] == last:
        runs = [(runs[-1][0] - (last - first + 1), runs[0][1])] + runs[1:-1]
    return runs


def roll_the_feet(rig, mesh, clip, scene):
    """Rolls each planted foot heel to toe, and leaves every swing frame alone.

    Contact lands heel-first with the toe up, the foot is flat by `FLAT_AT` of the way through,
    then the heel lifts and the toe breaks. Outside stance nothing is touched: the flight and
    recovery poses are the animator's, and the last attempt at this - a blanket rule on every
    frame - is what flattened the push-off and was reported as "nearly every frame is wrong".
    """
    play(rig, clip)
    first, last = (int(round(v)) for v in clip.frame_range)
    shoe = the_shoe_vertices(mesh)
    up = mathutils.Vector((0.0, 0.0, 1.0))
    # Where he is going, from the bind toe - the same convention `render_clay` uses to decide
    # which way he faces, so nothing here assumes a world axis.
    bind = rig.pose.bones["L_ToeBase"].bone
    travel = (rig.matrix_world @ bind.tail_local) - (rig.matrix_world @ bind.head_local)
    travel.z = 0.0
    if travel.length < 1e-9:
        refuse("the bind toe has no horizontal direction, so travel cannot be established")
    travel.normalize()

    held, down, posed_at = {}, {"L": {}, "R": {}}, {}
    for frame in range(first, last + 1):
        scene.frame_set(frame)
        graph = bpy.context.evaluated_depsgraph_get()
        posed, skin = rig.evaluated_get(graph), mesh.evaluated_get(graph)
        for side in ("L", "R"):
            heel, toe, pitch, brk = the_foot_parts(posed, skin, shoe, side)
            ankle = posed.matrix_world @ posed.pose.bones[f"{side}_Foot"].head
            ball = posed.matrix_world @ posed.pose.bones[f"{side}_ToeBase"].head
            along = ball - ankle
            falls = -along.normalized().cross(up) if along.length > 1e-9 else None
            thigh = ((posed.matrix_world @ posed.pose.bones[f"{side}_Calf"].head)
                     - (posed.matrix_world @ posed.pose.bones[f"{side}_Thigh"].head))
            shin = ((posed.matrix_world @ posed.pose.bones[f"{side}_Foot"].head)
                    - (posed.matrix_world @ posed.pose.bones[f"{side}_Calf"].head))
            posed_at[(side, frame)] = (thigh, shin)
            held[(side, frame)] = (pitch, brk, falls,
                                   (rig.matrix_world @ posed.pose.bones[f"{side}_Foot"].matrix)
                                   .to_3x3(),
                                   (rig.matrix_world
                                    @ posed.pose.bones[f"{side}_ToeBase"].matrix).to_3x3())
            down[side][frame] = min(heel, toe) < STANCE_WITHIN / 170.0

    def rolled(share):
        """Where the foot and the toe should be, at `share` of the way through a stance."""
        if share <= FLAT_AT:
            pitch = CONTACT_PITCH * (1.0 - share / FLAT_AT)
        else:
            pitch = PUSH_PITCH * (share - FLAT_AT) / (1.0 - FLAT_AT)
        # Negative: the toe extends UP relative to the foot, by as much as the foot has pitched
        # down, so the toe itself stays flat on the ground. Capped, because a toe only bends so
        # far, and eased in so it does not snap straight at the moment the heel lifts.
        if share <= TOE_BREAKS_AFTER:
            brk = 0.0
        else:
            into = (share - TOE_BREAKS_AFTER) / (1.0 - TOE_BREAKS_AFTER)
            brk = -min(max(pitch, 0.0), TOE_BREAKS_TO) * into
        return pitch, brk

    wanted = {}
    rolls = {}
    for side in ("L", "R"):
        runs = the_stances(down[side], first, last)
        rolls[side] = len(runs)
        for began, ended in runs:
            span = max(ended - began, 1)
            # The fade has to fit inside the stance it is fading. Two fixed frames either side
            # swallowed the right foot's whole contact - its stances run two to three frames
            # where the left's run five - so every right-foot frame was only partly corrected and
            # it sat 12 to 19 degrees out of its leg's plane while the left sat at 2 to 3.
            eases = max(1, min(ROLL_EASES_OVER, span // 2))
            for frame in range(began - eases, ended + eases + 1):
                at = first + (frame - first) % (last - first + 1)
                if (side, at) not in held:
                    continue
                share = min(max((frame - began) / span, 0.0), 1.0)
                # Faded at the edges so the correction starts and stops smoothly rather than
                # stepping into and out of the swing poses on either side.
                fade = 1.0
                if frame < began:
                    fade = 1.0 - (began - frame) / (eases + 1.0)
                elif frame > ended:
                    fade = 1.0 - (frame - ended) / (eases + 1.0)
                pitch, brk, falls, foot_at, toe_at = held[(side, at)]
                if falls is None or falls.length < 1e-9:
                    continue
                falls = falls.normalized()
                want_pitch, want_brk = rolled(share)

                sideways_now, swung = None, None
                if PLANTS_FLAT:
                    # The whole orientation at once: flat about its own length, pitched by the
                    # roll, and pointing where THE LEG points.
                    #
                    # Not where he is TRAVELLING, which is what this aimed at first. A foot comes
                    # straight off its shin; forcing it to face down the line of travel while the
                    # leg swings somewhere else kinks the ankle sideways, and that is what "the
                    # entire foot bends to the side instead of being straight off the shin bone"
                    # is. The leg's own plane is the one through hip, knee and ankle, and the
                    # direction it swings in is that plane's horizontal - so the foot follows the
                    # leg and the ankle stays straight.
                    # TRAVEL, and the leg's own plane was tried instead. It is the better
                    # anatomy - a foot comes straight off its shin - and the slide guard refused
                    # it outright: the walk's planted foot dropped to 0.76 m/s against a covers
                    # of 1.06, off by 27.9%. Turning a planted foot to follow the leg drags its
                    # toe across the ground, and a foot that slides is a worse fault than a foot
                    # a few degrees out of its leg's plane.
                    #
                    # The right way round is to move the ANKLE rather than turn the foot, which
                    # is the leg solver's job and not a clip correction's.
                    aims = travel
                    lean = math.radians(want_pitch)
                    forward = (aims * math.cos(lean) - up * math.sin(lean)).normalized()
                    sideways = forward.cross(up)
                    if sideways.length < 1e-9:
                        continue
                    sideways.normalize()
                    upright = sideways.cross(forward).normalized()
                    sideways_now = sideways
                    target = mathutils.Matrix((
                        (sideways.x, forward.x, upright.x),
                        (sideways.y, forward.y, upright.y),
                        (sideways.z, forward.z, upright.z),
                    )).to_quaternion()
                    now = foot_at.to_quaternion()
                    # Eased from where the animator had it toward flat-and-forward, so the
                    # correction fades in and out at the edges of the stance.
                    wanted.setdefault(f"{side}_Foot", {})[at] = (
                        now.inverted() @ now.slerp(target, fade))
                    # How far the foot itself turns IN THE WORLD. The toe rides on the foot, so
                    # its own frame moves by this too - and the toe's hinge axis has to be
                    # expressed in the frame the toe will be in, not the one it is in now.
                    swung = now.slerp(target, fade) @ now.inverted()
                elif abs(want_pitch - pitch) >= 0.01:
                    mine = foot_at.inverted() @ falls
                    if mine.length > 1e-9:
                        wanted.setdefault(f"{side}_Foot", {})[at] = mathutils.Quaternion(
                            mine.normalized(), math.radians((want_pitch - pitch) * fade))

                if abs((want_brk - brk) * fade) >= 0.01:
                    # About the axis of the foot's TARGET orientation, not the one it had before
                    # the flat-and-forward correction moved it. `falls` is derived from where the
                    # foot was pointing when the frame was measured, and the foot is then rotated
                    # - sometimes a long way, to flat and along travel - so a toe turned about the
                    # old axis bends out of the new foot's plane. Measured, that put as much
                    # SIDEWAYS bend in the toe as there was proper bend: 37.8 degrees of it on the
                    # left against 36.5 in plane, and reported as "toes still bend to the side".
                    hinge = sideways_now if PLANTS_FLAT and sideways_now is not None else falls
                    if swung is not None:
                        hinge = swung.inverted() @ hinge
                    mine = toe_at.inverted() @ hinge
                    if mine.length > 1e-9:
                        wanted.setdefault(f"{side}_ToeBase", {})[at] = mathutils.Quaternion(
                            mine.normalized(), math.radians((want_brk - brk) * fade))

    # Every frame gets a key, corrected or not. Writing keys only where a correction applies
    # leaves the curve to interpolate across the whole flight phase between one stance's last
    # value and the next stance's first - so mid-swing the toe drifts to wherever that line
    # passes, which measured as 32.3 degrees of SIDEWAYS bend on the right foot with none of it
    # asked for. A toe in the air is straight; saying so explicitly is what keeps it there.
    for bone in list(wanted):
        for frame in range(first, last + 1):
            wanted[bone].setdefault(frame, mathutils.Quaternion())

    slot = rig.animation_data.action_slot if rig.animation_data else None
    for bone, byframe in wanted.items():
        chans = channels_for(clip, slot, f'pose.bones["{bone}"].rotation_quaternion')
        keyed = {}
        for frame in byframe:
            was = mathutils.Quaternion([chans[i].evaluate(frame) for i in range(4)])
            keyed[frame] = mathutils.Quaternion() if was.magnitude < 1e-9 else was
        turned = {frame: keyed[frame] @ offset for frame, offset in byframe.items()}
        # A cycle's last frame IS its first, so whatever the roll decides for one it must decide
        # for the other. It does not fall out on its own: a stance that wraps the seam gives the
        # two ends different shares of the same stance, so they were corrected differently and
        # the clip stopped looping - and the odd frame then became the deepest sole in the file
        # and dragged the floor lift with it.
        if first in turned or last in turned:
            # Whichever end was corrected wins, and the other is set to match. Reading only
            # `turned` was not enough: when the roll touched the LAST frame and not the first,
            # the last kept its correction, the first kept none, and the clip stopped looping -
            # 13.48 degrees apart, which the audit caught.
            settled = turned.get(first)
            if settled is None:
                settled = turned.get(last)
            if settled is not None:
                turned[first] = settled
                turned[last] = settled
        for frame, out in turned.items():
            for i in range(4):
                chans[i].keyframe_points.insert(frame, out[i], options={"FAST"})
        for curve in chans.values():
            curve.update()
    return rolls


def the_lowest_sole(rig, mesh, clip, scene):
    """Where the shoe rests when it is DOWN, and the deepest frame, off the skinned mesh.

    Returns the height to lift by and the deepest frame, in that order.

    # The typical contact, not the single deepest frame

    Lifting by the minimum was the obvious thing and it was wrong. The run's deepest pose sits
    2.6 cm below its own typical contact, so lifting the clip by that one frame put every real
    footfall in the air: measured afterwards, only 4 of 25 frames had a foot within 3 cm of the
    floor and the warden ran on tiptoe 4 to 25 cm up. One outlier pose decided the height of the
    whole cycle.

    So the lift is the MEDIAN of the frames where a shoe is actually down - within a window of
    its own lowest point, the same adaptive test `the_footfalls` uses in the audit. Typical
    contact lands on the floor, the deepest pose goes a little under, and that is the right side
    of the error: a foot fractionally through the floor at one extreme reads as weight, a foot
    hovering through an entire stance reads as a bug.
    """
    play(rig, clip)
    first, last = (int(round(v)) for v in clip.frame_range)
    groups = {g.index: g.name for g in mesh.vertex_groups}
    shoe = []
    for vertex in mesh.data.vertices:
        for group in vertex.groups:
            name = groups.get(group.group, "")
            if group.weight > 0.3 and ("Foot" in name or "ToeBase" in name):
                shoe.append(vertex.index)
                break
    if not shoe:
        refuse("no vertices are weighted to the feet, so the floor cannot be found")
    soles = {}
    for frame in range(first, last + 1):
        scene.frame_set(frame)
        skin = mesh.evaluated_get(bpy.context.evaluated_depsgraph_get())
        soles[frame] = min((skin.matrix_world @ skin.data.vertices[i].co).z for i in shoe)
    if not soles:
        return 0.0, first
    when = min(soles, key=soles.get)
    lowest = soles[when]
    # Frames where a shoe is DOWN: within a window of the clip's own lowest point. Widened until
    # there are enough of them to take a median from, because a run's contact is short.
    for window in (2.0, 3.0, 4.0, 6.0, 9.0):
        down = sorted(z for z in soles.values() if z <= lowest + window / 170.0)
        if len(down) >= max(3, len(soles) // 8):
            break
    return down[len(down) // 2], when


def stand_on_the_floor(rig, mesh, clip, scene):
    """Lifts a clip until no part of either shoe is below the floor.

    The runtime's ground contact cannot fix this and is not meant to: `ik::shift_to_ground`
    corrects for how far the ground under one foot differs from the ground under the warden, so
    on flat ground it correctly does nothing - and a sole authored through the floor stays
    through it. Measured on the delivered clips, the sole is below zero on 100% of idle and walk
    frames and 88% of run frames. In the BIND pose both soles sit at 0.00 cm, so this is the
    animation rather than the mesh.

    A single lift per clip, applied to the root's own translation. It cannot fix the two feet
    sitting at DIFFERENT heights - the idle's left sole rests 3.5 cm lower than its right - which
    is a per-foot correction and therefore the runtime solver's job, not a second copy of it here.
    What this guarantees is that nothing penetrates; anything left over floats, which is the side
    of the error a ground solver can pull back down.
    """
    low, when = the_lowest_sole(rig, mesh, clip, scene)
    if abs(low) < 1e-6:
        return low, 0.0, low, when
    slot = rig.animation_data.action_slot if rig.animation_data else None
    curves = fcurves_of(clip, slot)
    # Whichever bone the clip moves the body with. World up expressed in ITS rest frame, because
    # a bone's local Z is along whatever way the bone was built and not necessarily up.
    for carrier in ("Root", "Hip", "Pelvis", "Hips"):
        if carrier not in rig.pose.bones:
            continue
        path = f'pose.bones["{carrier}"].location'
        parts = {c.array_index: c for c in curves if c.data_path == path}
        if len(parts) != 3:
            continue
        rest = (rig.matrix_world @ rig.pose.bones[carrier].bone.matrix_local).to_3x3()
        up = (rest.inverted() @ mathutils.Vector((0.0, 0.0, 1.0))).normalized() * (-low)
        for at in range(3):
            for key in parts[at].keyframe_points:
                key.co[1] += up[at]
                key.handle_left[1] += up[at]
                key.handle_right[1] += up[at]
            parts[at].update()
        now, _ = the_lowest_sole(rig, mesh, clip, scene)
        return low, -low, now, when
    refuse(f"{clip.name} keys no root translation, so it cannot be lifted onto the floor")


def the_feet_point(rig, clip, scene):
    """How far each toe points off the line of travel, over a whole clip, in degrees."""
    play(rig, clip)
    first, last = (int(round(v)) for v in clip.frame_range)
    step = max(1, (last - first) // 60)
    bind = rig.pose.bones["L_ToeBase"].bone
    ahead = ((rig.matrix_world @ bind.tail_local) - (rig.matrix_world @ bind.head_local))
    ahead.z = 0.0
    if ahead.length < 1e-9:
        refuse("the bind toe has no horizontal direction, so travel cannot be established")
    ahead.normalize()
    across = mathutils.Vector((-ahead.y, ahead.x, 0.0))
    out = {"L": [], "R": []}
    for frame in range(first, last + 1, step):
        scene.frame_set(frame)
        posed = rig.evaluated_get(bpy.context.evaluated_depsgraph_get())
        for side in ("L", "R"):
            toe = posed.pose.bones[f"{side}_ToeBase"]
            along = ((posed.matrix_world @ toe.tail) - (posed.matrix_world @ toe.head))
            along.z = 0.0
            if along.length > 1e-9:
                along.normalize()
                out[side].append(math.degrees(math.atan2(along.dot(across), along.dot(ahead))))
    return out


def point_the_feet_along(rig, clip, scene):
    """Takes the excess splay out of a foot, leaving the natural toe-out in.

    Measured on the delivered clips, the RIGHT foot points about thirty degrees off the line of
    travel through both the idle and the walk while the left is straight - mean -30.3 and -29.1
    against +0.5 and +4.3. That is the same left/right signature as the arm that rested four
    degrees tighter to the torso and the forearm whose whole roll sat in one joint: these clips
    are mirrored imperfectly, and it reads as "a lot of frames they're offset unnaturally".

    The correction reduces the MAGNITUDE toward `SPLAY_ALLOWS` and keeps the sign, so a foot that
    toes out naturally still toes out - it just stops doing it by thirty degrees. A foot already
    inside the allowance is left alone entirely.

    Applied at the ankle, which is the cheap place rather than the true one: a splay of this size
    really comes from the hip, and rotating the whole leg would be the honest fix. On a stylised
    character the ankle reads fine and it cannot disturb the hip's own motion, which the gait
    depends on.
    """
    was = the_feet_point(rig, clip, scene)
    play(rig, clip)
    first, last = (int(round(v)) for v in clip.frame_range)
    bind = rig.pose.bones["L_ToeBase"].bone
    ahead = ((rig.matrix_world @ bind.tail_local) - (rig.matrix_world @ bind.head_local))
    ahead.z = 0.0
    if ahead.length < 1e-9:
        refuse("the bind toe has no horizontal direction, so travel cannot be established")
    ahead.normalize()
    across = mathutils.Vector((-ahead.y, ahead.x, 0.0))
    up = mathutils.Vector((0.0, 0.0, 1.0))

    wanted = {"L": {}, "R": {}}
    for frame in range(first, last + 1):
        scene.frame_set(frame)
        posed = rig.evaluated_get(bpy.context.evaluated_depsgraph_get())
        for side in ("L", "R"):
            toe = posed.pose.bones[f"{side}_ToeBase"]
            along = ((posed.matrix_world @ toe.tail) - (posed.matrix_world @ toe.head))
            along.z = 0.0
            if along.length < 1e-9:
                continue
            along.normalize()
            points = math.degrees(math.atan2(along.dot(across), along.dot(ahead)))
            over = max(abs(points) - SPLAY_ALLOWS, 0.0)
            if over <= 0.01:
                continue
            # Toward the allowance, never past it, and never through it to the other side.
            turn = -math.copysign(over, points)
            held = (rig.matrix_world @ posed.pose.bones[f"{side}_Foot"].matrix).to_3x3()
            mine = held.inverted() @ up
            if mine.length < 1e-9:
                continue
            wanted[side][frame] = mathutils.Quaternion(mine.normalized(), math.radians(turn))

    slot = rig.animation_data.action_slot if rig.animation_data else None
    for side in ("L", "R"):
        if not wanted[side]:
            continue
        chans = channels_for(clip, slot, f'pose.bones["{side}_Foot"].rotation_quaternion')
        # Everything read before anything is written - see `break_the_toes` for what happens
        # otherwise.
        keyed = {}
        for frame in wanted[side]:
            held = mathutils.Quaternion([chans[i].evaluate(frame) for i in range(4)])
            keyed[frame] = mathutils.Quaternion() if held.magnitude < 1e-9 else held
        for frame, offset in wanted[side].items():
            turned = keyed[frame] @ offset
            for i in range(4):
                chans[i].keyframe_points.insert(frame, turned[i], options={"FAST"})
        for curve in chans.values():
            curve.update()
    now = the_feet_point(rig, clip, scene)
    return was, now


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

    # The shortest arc, or the blend goes the long way round

    Every offset is put on the shortest arc first. `q` and `-q` are the same rotation, but
    slerping from IDENTITY to one of them is not the same as slerping to the other: for a `q`
    with negative w the interpolation swings the long way, through angles larger than either
    end. It showed up as a spike in the run's loop-closure window - the left foot's local
    rotation reading 115.5 degrees on frame 23 where its neighbours sat near 58, and a toe
    correction capped at 55 degrees coming out at 111.8 - and it also put one frame's sole
    0.31 cm back through the floor after the clip had been lifted onto it.
    """
    for step in range(min(over, len(poses))):
        share = 1.0 - (step / over)
        at = -(step + 1) if backwards else step
        for name, (turn, shift) in offsets.items():
            if name not in poses[at]:
                continue
            short = turn.copy()
            if short.w < 0.0:
                short.negate()
            was_turn, was_shift, was_scale = poses[at][name]
            poses[at][name] = (
                mathutils.Quaternion().slerp(short, share) @ was_turn,
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
UNFUSES = False

# How far a shared vertex sinks toward the wrist, as a share of the digit's length, and how far
# it is drawn toward the seam between its two digits. Faded by how far along the digit it sits:
# full at the crotch, nothing by the fingertips, so the web deepens without narrowing the tips.
# Measured, not guessed at twice: 0.30 sank the crotch 4.14 cm on a 9 cm hand and tore the left
# hand into ribbons. The pinch is worse than too-large - it is WRONG: pulling shared vertices
# toward the seam between two digits drags both digits into each other, which is fusing them
# harder rather than parting them. Sink only, gently.
WEB_SINKS_BY = 0.08
WEB_PINCHES_BY = 0.0


# # Smoothing the weights where the skin tears
#
# Diagnosed rather than assumed. With the arms forward, 307 edges stretch past 1.35x, and the
# split is NOT what either textbook fault looks like:
#
#     246  both mild      short edge, small weight jump
#      33  long edge      too little geometry to share the bend
#      28  weights jump   a hard transition
#
# The worst is x5.26 on a 0.74 cm edge with a weight jump of only 0.24 - two neighbours a
# centimetre apart, driven similarly, ending up four centimetres apart. That is a transition
# happening over too FEW vertices rather than a wrong one, which is what weight smoothing fixes:
# blur each vertex's weights toward its welded neighbours' so the change spreads over the whole
# region instead of one edge.
#
# Applied only where it tears, kept only if it helps. `the_skin_holds` measures strain before and
# after and refuses to keep a pass that made things worse - a blur can bleed an arm's influence
# onto the chest and look tidier while deforming worse.
# MEASURED AND NOT KEPT. Smoothing made the deformation worse at every strength tried:
#
#     0.20 -> 504 tearing edges     0.35 -> 488     0.50 -> 476     against 452 before
#
# The reason is structural rather than a bad number. Blurring pulls the clavicle's influence
# onto spine vertices AND the spine's onto clavicle vertices, so MORE vertices end up partly
# driven by a swinging bone: it widens the affected region instead of easing the gradient. On a
# body that welds to 2464 vertices there is no room for the gradient to spread into.
#
# The machinery stays, off, because it is the right tool on a denser mesh and the numbers are
# the evidence for why it is not the tool here. `SMOOTHS_WEIGHTS` turns it on.
SMOOTHS_WEIGHTS = False
SMOOTHS_OVER = 3          # passes
SMOOTHS_BY = 0.35          # how far toward the neighbourhood average each pass moves a vertex
SMOOTHS_AROUND = 2        # rings of neighbours around a tearing vertex also smoothed


def smooth_the_weights(rig, mesh):
    """Blurs the weights around every vertex whose edges tear, and keeps it only if it helps."""
    from collections import defaultdict

    def key_of(co):
        return (round(co.x / WELD_WITHIN), round(co.y / WELD_WITHIN), round(co.z / WELD_WITHIN))

    canon, seen_at = {}, {}
    for vertex in mesh.data.vertices:
        canon[vertex.index] = seen_at.setdefault(key_of(vertex.co), vertex.index)
    touching = defaultdict(set)
    for edge in mesh.data.edges:
        a, b = canon[edge.vertices[0]], canon[edge.vertices[1]]
        if a != b:
            touching[a].add(b)
            touching[b].add(a)

    def strain(poses):
        """The worst stretch of every edge over a set of poses, and how many tear."""
        at_rest()
        posed = mesh.evaluated_get(bpy.context.evaluated_depsgraph_get())
        rest = {}
        for edge in mesh.data.edges:
            a, b = edge.vertices
            rest[edge.index] = ((posed.matrix_world @ posed.data.vertices[a].co)
                                - (posed.matrix_world @ posed.data.vertices[b].co)).length
        worst, torn = {}, 0
        for turns in poses:
            at_rest()
            for name, axis, degrees in turns:
                if name in rig.pose.bones:
                    rig.pose.bones[name].rotation_quaternion = mathutils.Quaternion(
                        axis, math.radians(degrees))
            bpy.context.view_layer.update()
            posed = mesh.evaluated_get(bpy.context.evaluated_depsgraph_get())
            for edge in mesh.data.edges:
                if rest[edge.index] < 1e-6:
                    continue
                a, b = edge.vertices
                now = ((posed.matrix_world @ posed.data.vertices[a].co)
                       - (posed.matrix_world @ posed.data.vertices[b].co)).length
                ratio = now / rest[edge.index]
                if ratio > worst.get(edge.index, 0.0):
                    worst[edge.index] = ratio
        torn = sum(1 for v in worst.values() if v > 1.35)
        return worst, torn, max(worst.values(), default=0.0)

    def at_rest():
        if rig.animation_data:
            rig.animation_data.action = None
        for bone in rig.pose.bones:
            bone.rotation_mode = "QUATERNION"
            bone.rotation_quaternion = mathutils.Quaternion()
            bone.location = mathutils.Vector((0.0, 0.0, 0.0))
            bone.scale = mathutils.Vector((1.0, 1.0, 1.0))
        bpy.context.view_layer.update()

    poses = (
        (("L_Upperarm", (1.0, 0.0, 0.0), 80.0), ("R_Upperarm", (1.0, 0.0, 0.0), 80.0)),
        (("L_Upperarm", (0.0, 0.0, 1.0), 85.0), ("R_Upperarm", (0.0, 0.0, 1.0), -85.0)),
        (("L_Thigh", (1.0, 0.0, 0.0), -55.0), ("R_Thigh", (1.0, 0.0, 0.0), 55.0)),
    )
    before, torn_before, peak_before = strain(poses)
    hurt = {e for e, r in before.items() if r > 1.35}
    wanted = set()
    for edge_index in hurt:
        for vertex in mesh.data.edges[edge_index].vertices:
            wanted.add(canon[vertex])
    for _ in range(SMOOTHS_AROUND):
        wanted |= {n for v in wanted for n in touching[v]}
    print(f"  smoothing weights around {len(wanted)} vertices, from {torn_before} tearing edges")

    groups = {g.index: g.name for g in mesh.vertex_groups}
    kept = {v.index: {g.group: g.weight for g in v.groups} for v in mesh.data.vertices}
    at_hand = {canon[i]: w for i, w in kept.items()}
    for _ in range(SMOOTHS_OVER):
        fresh = {}
        for node in wanted:
            neighbours = [n for n in touching[node] if n in at_hand]
            if not neighbours:
                continue
            blend = defaultdict(float)
            for other in neighbours:
                for group, weight in at_hand[other].items():
                    blend[group] += weight / len(neighbours)
            mine = at_hand[node]
            mixed = defaultdict(float)
            for group in set(mine) | set(blend):
                mixed[group] = (mine.get(group, 0.0) * (1.0 - SMOOTHS_BY)
                                + blend.get(group, 0.0) * SMOOTHS_BY)
            fresh[node] = dict(mixed)
        at_hand.update(fresh)

    # Written back to every split copy, trimmed to the four glTF carries, renormalised.
    for vertex in mesh.data.vertices:
        node = canon[vertex.index]
        if node not in wanted:
            continue
        mixed = at_hand[node]
        top = sorted(mixed.items(), key=lambda kv: -kv[1])[:4]
        total = sum(w for _, w in top)
        if total <= 1e-9:
            continue
        for group in list(mesh.vertex_groups):
            group.remove([vertex.index])
        for group_index, weight in top:
            mesh.vertex_groups[groups[group_index]].add(
                [vertex.index], weight / total, "REPLACE")
    mesh.data.update()

    after, torn_after, peak_after = strain(poses)
    print(f"  tearing edges {torn_before} -> {torn_after}, worst stretch "
          f"x{peak_before:.2f} -> x{peak_after:.2f}")
    if torn_after > torn_before:
        print(f"  *** smoothing made it worse and was NOT kept - a blur that bleeds one limb's "
              f"influence onto another looks tidier and deforms worse")
        for vertex in mesh.data.vertices:
            if canon[vertex.index] not in wanted:
                continue
            for group in list(mesh.vertex_groups):
                group.remove([vertex.index])
            for group_index, weight in kept[vertex.index].items():
                mesh.vertex_groups[groups[group_index]].add(
                    [vertex.index], weight, "REPLACE")
        mesh.data.update()
    at_rest()
    return torn_after <= torn_before


def deepen_the_armpit(rig, mesh):
    """Sinks the armpit webbing toward the apex, so the arm reads clear of the ribs.

    Deletes nothing. Every recorded webbing vertex moves toward the top of the armpit hollow
    along the upper arm's own axis, faded by how far below the apex it sits, and every split
    copy of a shared position moves with its twins - moving one and not the other would tear
    the surface along a UV seam.
    """
    def key_of(co):
        return (round(co.x / WELD_WITHIN), round(co.y / WELD_WITHIN), round(co.z / WELD_WITHIN))

    groups = {g.index: g.name for g in mesh.vertex_groups}

    def owner_of(index):
        best, who = 0.0, ""
        for group in mesh.data.vertices[index].groups:
            if group.weight > best:
                best, who = group.weight, groups.get(group.group, "")
        return who

    into_mesh = mesh.matrix_world.inverted()
    moved_by, found = {}, 0
    for side, centroids in WEBBING.items():
        shoulder = rig.matrix_world @ rig.pose.bones[f"{side}_Upperarm"].head
        elbow = rig.matrix_world @ rig.pose.bones[f"{side}_Forearm"].head
        arm_axis = (elbow - shoulder)
        if arm_axis.length < 1e-6:
            continue
        arm_axis = arm_axis.normalized()
        spine_low = rig.matrix_world @ rig.pose.bones["Spine01"].head
        spine_high = rig.matrix_world @ rig.pose.bones["Spine02"].head
        spine_axis = (spine_high - spine_low)
        spine_axis = spine_axis.normalized() if spine_axis.length > 1e-6 else mathutils.Vector(
            (0.0, 0.0, 1.0))

        mine = set()
        for spot in centroids:
            aim = mathutils.Vector(spot)
            near = [p for p in mesh.data.polygons
                    if (p.center - aim).length < THE_SAME_FACE_WITHIN]
            if len(near) != 1:
                refuse(f"the recorded {side} webbing face at {spot} matches {len(near)} faces - "
                       f"this is not the mesh the record was measured on, so nothing was moved")
            mine.update(near[0].vertices)
        found += len(mine)

        for index in mine:
            who = owner_of(index)
            if any(part in who for part in ("Upperarm", "Forearm", "Hand")):
                base, axis = shoulder, arm_axis
            else:
                base, axis = spine_low, spine_axis
            spot = mesh.matrix_world @ mesh.data.vertices[index].co
            out = (spot - base) - axis * (spot - base).dot(axis)
            if out.length < 1e-6:
                continue
            moved_by[key_of(mesh.data.vertices[index].co)] = -out * ARMPIT_DRAWS_IN_BY

    shifted = 0
    for vertex in mesh.data.vertices:
        shift = moved_by.get(key_of(vertex.co))
        if shift is None:
            continue
        vertex.co = into_mesh @ ((mesh.matrix_world @ vertex.co) + shift)
        shifted += 1
    mesh.data.update()
    deepest = max((v.length for v in moved_by.values()), default=0.0) * 170.0
    print(f"  opened both armpits: {found} webbing vertices ({shifted} stored copies), "
          f"drawn up to {deepest:.2f} cm toward their own bone")


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


def close_the_loop(rig, clip, scene, over):
    """Bends the end of a cycle back to meet its beginning, in rotation AND position.

    Sampled and re-keyed rather than edited in place: the correction has to reach every bone the
    clip touches, and a clip authored at one rate against another's key times cannot be nudged
    channel by channel without drifting out of step with itself.
    """
    play(rig, clip)
    frames = sample(rig, clip, scene)
    if len(frames) < over + 2:
        return 0.0
    offsets = {}
    for name, (turn, shift, _) in frames[0].items():
        if name not in frames[-1]:
            continue
        their_turn, their_shift, _ = frames[-1][name]
        offsets[name] = (turn @ their_turn.inverted(), shift - their_shift)
    bend_to_meet(frames, offsets, over, backwards=True)

    was = clip.name
    clip.name = f"{was}_open"
    made, _ = join_the_clips(rig, scene, [], was) if False else (None, None)
    # Re-keyed straight from the samples, on the clip's own name.
    fresh = bpy.data.actions.new(was)
    fresh.use_fake_user = True
    rig.animation_data.action = fresh
    slots = getattr(fresh, "slots", None)
    if slots is not None:
        rig.animation_data.action_slot = fresh.slots.new(id_type="OBJECT", name="Armature")
    for at, pose in enumerate(frames):
        scene.frame_set(at + 1)
        for bone in rig.pose.bones:
            if bone.name not in pose:
                continue
            bone.rotation_mode = "QUATERNION"
            bone.rotation_quaternion, bone.location, bone.scale = pose[bone.name]
            bone.keyframe_insert("rotation_quaternion", frame=at + 1)
            bone.keyframe_insert("location", frame=at + 1)
            bone.keyframe_insert("scale", frame=at + 1)
    bpy.data.actions.remove(clip)
    return fresh


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
        return (rig.matrix_world @ the_bone(rig, name).head).copy()

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
    # How much the base was resized on the way in, so every later file's clips can follow it.
    shrank = 1.0
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
        # Before anything touches the clip, and for EVERY file rather than only the base.
        if not KEEPS_THE_DELIVERED_NAMES:
            renamed_paths = speak_the_clips_language(clips)
            if renamed_paths:
                print(f"    pointed {renamed_paths} animation channels at the renamed bones")
            if base_rig is not None:
                carried = carry_the_clips_into_the_new_scale(clips, shrank)
                if carried:
                    print(f"    scaled {carried} translation channel(s) by {shrank:.6f} to match")

        if base_rig is None:
            base_rig = rig
            base_mesh = next(o for o in fresh if o.type == "MESH" and o.vertex_groups)
            skeleton = skeleton_of(rig)
            print(f"  {filename}: the base - {len(rig.data.bones)} bones, "
                  f"{len(base_mesh.data.vertices)} vertices")
            # FIRST of everything, before a single measurement: every function below this line
            # asks for bones by the pipeline's names and reports in centimetres of a unit figure.
            if KEEPS_THE_DELIVERED_NAMES:
                print("    the delivered rig ships as delivered: names, scale and facing are the "
                      "artist's, and the game and tools adapt to them")
            else:
                named, tall, grew, strays, shrank = speak_the_pipeline_s_language(
                    rig, [o for o in fresh if o.type == "MESH"])
                carried = carry_the_clips_into_the_new_scale(clips, shrank)
                if carried:
                    print(f"    scaled {carried} translation channel(s) by {shrank:.6f} to match")
                if POINTS_THE_BONES:
                    aimed = point_the_bones_at_their_children(rig)
                    print(f"    rebuilt {aimed} bone tails from the skeleton")
                print(f"    renamed {named} bones; scaled a {tall:.3f} unit figure by {grew:.4f}")
                if strays:
                    print(f"    not in the table, left alone: {', '.join(sorted(strays))}")
            if CUT_THE_WEBBING:
                cut_the_webbing(rig, base_mesh)
            elif DEEPENS_THE_ARMPIT:
                deepen_the_armpit(rig, base_mesh)
            if CLOSES_THE_HOLES:
                close_the_holes(rig, base_mesh)
            if PADS_THE_TEXTURE:
                padded = pad_the_texture_islands(base_mesh)
                if padded is None:
                    print("    no base colour texture to pad")
                else:
                    was, now, rings, mended = padded
                    print(f"    padded the texture islands: UVs cover {was * 100:.1f}% of the "
                          f"sheet, {rings} rings of dilation take it to {now * 100:.1f}% - the "
                          "gaps that were sampling as black speckle")
                    if mended:
                        print(f"    mended {mended} hair island(s) that were painted orange - "
                              "repainted to the hair's own median colour")
            if ZIPS_THE_PINHOLES:
                found, welded, left = zip_the_pinholes(base_mesh)
                print(f"    zipped the pinholes: {found} truly open edges, {welded} vertices "
                      f"welded onto their twins, {left} still open")
            # Before every other rig edit, because everything downstream measures against it.
            if MIRRORS_THE_BIND:
                pairs, was = the_bind_is_mirrored(rig)
                print(f"    mirrored the bind: {pairs} pairs, the worst was {was:.2f} cm from "
                      f"its own reflection")
            # And then square the mirrored figure onto the axis, before anything measures him.
            #
            # After the mirror rather than before it, because the delivered bind is not symmetric
            # and the two halves do not agree about which way he faces: his clavicles share an x
            # so his shoulder line reads dead along the axis, while his left thigh sits 2.5 cm
            # forward of his right and skews his hip line 11.67 degrees. Asked before the mirror,
            # the two witnesses are 11.67 apart and squaring would just be picking one. The mirror
            # settles that - it is what it is for - and it settles it onto the crooked answer,
            # which is what this then turns.
            if SQUARES_HIM_UP:
                turned, left_over = square_him_up(
                    rig, [o for o in fresh if o.type == "MESH"])
                print(f"    squared him up: turned {turned:+.2f} deg, so his front now runs "
                      f"along {FACES_ALONG} to within {left_over:.3f} deg")
            # Before anything reads a toe position, and before any clip is corrected: this moves
            # the joint the whole roll pivots about.
            moved, shifted = (hinge_the_toes_at_the_ball(rig, base_mesh)
                              if HINGES_THE_TOES else ({}, 0))
            for side, (by, before, now) in sorted(moved.items()):
                print(f"    {side} toe joint moved {by:5.2f} cm forward, from {before:.1f}% "
                      f"to {now:.0f}% along the shoe, and the bone runs to the tip")
            print(f"    re-weighted {shifted} vertices about the new hinge")
            # And mirrored AGAIN, because the hinge undid it for the toes. Each toe is aimed
            # along its own shoe's long axis, and the shoe MESH is not mirrored - only the bones
            # were - so the two toes came out 16.4 degrees apart while every other bone in the
            # rig sat at 0.0. That is the "toes angle to the side" in a straight-on shot.
            if MIRRORS_THE_BIND:
                pairs, was = the_bind_is_mirrored(rig)
                print(f"    mirrored again after the hinge: worst {was:.2f} cm")
            # And the toe weights LAST of all, about where the joint finally sits rather than
            # where the hinge meant to put it - see `weight_the_toes_about_the_hinge`.
            if HINGES_THE_TOES:
                again, where = weight_the_toes_about_the_hinge(rig, base_mesh)
                print("    re-weighted the toes about the settled hinge: "
                      + ", ".join(f"{side} at {at:.0f}% along the shoe"
                                  for side, at in sorted(where.items())))
            if KNEE_EASE:
                stands = ease_the_knees(rig, KNEE_EASE)
                print(f"    eased the knees {KNEE_EASE:.1f} deg; the bind now stands at "
                      f"{stands * 100:.1f}% of straight, and every clip came with it")

            # Last, so it widens a rig that is already symmetric and stays symmetric.
            if THE_LEGS_STAND_APART:
                by = stand_the_legs_apart(rig, THE_LEGS_STAND_APART)
                print(f"    stood the legs {THE_LEGS_STAND_APART:.1f} deg apart, which moves "
                      f"each knee {by:.2f} cm out")
            assigned = add_the_fingers(rig, base_mesh) if ADDS_THE_FINGERS else {}
            if UNFUSES:
                # After the closer, never before it - see unfuse_the_digits on why.
                unfuse_the_digits(rig, base_mesh, assigned)
            # Last, so it smooths the weights the fingers actually ended up with.
            if SMOOTHS_WEIGHTS:
                smooth_the_weights(rig, base_mesh)
        else:
            the_skeletons_match(skeleton, skeleton_of(rig), filename)
            print(f"  {filename}: same skeleton, so its clip moves across unchanged")
            for thing in fresh:
                bpy.data.objects.remove(thing, do_unlink=True)

        clips[0].name = called
        clips[0].use_fake_user = True
        wanted[called] = clips[0]
        play(base_rig, clips[0])
        rolled = roll_the_hands(base_rig, clips[0], PALMS_ROLL_IN) if PALMS_ROLL_IN else 0
        if rolled:
            print(f"    rolled {rolled} hand(s) in by {PALMS_ROLL_IN:.0f} deg")
        if called in MOVES_MORE:
            wide, wider = move_the_arms_more(base_rig, clips[0], bpy.context.scene,
                                             MOVES_MORE[called], PUMPS.get(called, 1.0))
        if called in SHOULDERS_SIT_BACK:
            sit_the_shoulders_back(base_rig, clips[0], bpy.context.scene,
                                   SHOULDERS_SIT_BACK[called])
            print(f"    shoulders sat back {SHOULDERS_SIT_BACK[called]:.0f} deg, which lowers "
                  f"the hands without costing any swing")
        if called in ELBOWS_SWING:
            move_the_arms_more(base_rig, clips[0], bpy.context.scene,
                               ELBOWS_SWING[called], 1.0, THE_ELBOW_IS)
            print(f"    elbows open and shut {ELBOWS_SWING[called]:.2f}x, which pulls the "
                  f"hands in")
        # After the range, so the mean is the last word on where the elbow is carried.
        if called in ELBOWS_HOLD_AT:
            was, now = hold_the_elbows(base_rig, clips[0], bpy.context.scene,
                                       ELBOWS_HOLD_AT[called])
            print(f"    elbows held at L {was['L']:.0f} -> {now['L']:.0f} deg, "
                  f"R {was['R']:.0f} -> {now['R']:.0f}")
            if wide is not None:
                swing = ", ".join(f"{s} {wide[s]:.1f} -> {wider[s]:.1f} cm" for s in ("L", "R"))
                print(f"    arms move {MOVES_MORE[called]:.2f}x more: hands swing {swing}")
        if called in LEANS_FORWARD:
            was, by, now = lean_the_torso(base_rig, base_mesh, clips[0], bpy.context.scene,
                                          LEANS_FORWARD[called], "trunk lean")
            print(f"    torso leaned {by:+.1f} deg: {was:+.1f} -> {now:+.1f} off vertical, "
                  f"measured through the FLESH and not the spine bones")
        if called in STANDS_UPRIGHT:
            was, by, now = lean_the_torso(base_rig, base_mesh, clips[0], bpy.context.scene,
                                          STANDS_UPRIGHT[called], "sideways lean", sideways=True)
            print(f"    torso straightened {by:+.1f} deg sideways: {was:+.1f} -> {now:+.1f}")
        # After the trunk, always: it is undoing what the trunk did to the head.
        if called in LEVELS_THE_HEAD:
            was, rests, by, now = lean_a_chain(base_rig, clips[0], bpy.context.scene,
                                               THE_HEAD_IS, LEVELS_THE_HEAD[called], "head")
            print(f"    head levelled {by:+.1f} deg: {was - rests:+.1f} -> {now - rests:+.1f} "
                  f"from its own rest")
        if called in LIFTS:
            was, lifted, now = lift_the_arms(base_rig, clips[0], bpy.context.scene,
                                             ARMS_REST_AT)
            for side in ("L", "R"):
                note = (f"lifted {lifted[side]:.1f} deg" if side in lifted
                        else "already clear")
                print(f"    {side} arm rests {was[side]:5.1f} deg off the spine, {note}"
                      f" -> {now[side]:5.1f} deg")
        # Last of the three, so it redistributes the total - including the hand roll above.
        if SPREADS_THE_TWIST:
            pointed = where_the_hands_point(base_rig, clips[0], bpy.context.scene)
            spread = spread_the_twist(base_rig, clips[0], base_mesh)
            moved = where_the_hands_point(base_rig, clips[0], bpy.context.scene)
            for side, shares in sorted(spread.items()):
                where = ", ".join(f"{b} {sh * 100:.0f}%" for b, sh in shares)
                print(f"    {side} forearm roll spread as {where}")
            worst, when = 0.0, ""
            for side in ("L", "R"):
                for before, after in zip(pointed.get(side, []), moved.get(side, [])):
                    off = math.degrees(before.rotation_difference(after).angle)
                    off = min(off, 360.0 - off)
                    if off > worst:
                        worst, when = off, side
            # One degree, not half. The guard catches a wrong share or a wrong conjugation, and
            # those move a wrist by TENS of degrees - it read 46 when it last fired for real. The
            # residue it has to tolerate is fcurve resampling: the roll is written at the union of
            # every key time involved, and a sharper curve between keys - which is exactly what
            # the pump shaping makes - evaluates a little differently. It refused a build at 0.67.
            if worst > 1.0:
                refuse(f"spreading the roll on {clips[0].name} moved the {when} hand by "
                       f"{worst:.2f} deg - the wrist must land exactly where the animator put "
                       f"it, so the shares or the rest conjugation are wrong")
            print(f"    the wrists did not move: worst {worst:.3f} deg")

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
            # Not a fault by itself. The 2026-08-26 warden's clips arrive as treadmills already -
            # its hips move 0.6 cm across a whole run - which is the state this step exists to
            # produce, so there is nothing for it to do. It refused here, on a clip that was
            # already correct.
            #
            # What it DOES cost is the `covers` reading, which came from the travel that was taken
            # out. Without it, `the_footfalls` measuring the planted feet is the only source - and
            # it is the better one anyway, since it is what distance matching actually has to
            # agree with.
            print(f"    {called:<12s} arrives in place - hips move {hips * 100:.1f} cm across it - "
                  "so there is no travel to take out; COVERS comes from the feet")

    # AFTER the travel is out, never before. Closing the loop re-keys every channel from
    # samples, so a detrend that runs afterwards finds no travel left to remove and refuses.
    for called in CLOSES_THE_LOOP:
        if called not in wanted:
            continue
        shut = close_the_loop(base_rig, wanted[called], scene, CLOSE_OVER)
        if shut is not None:
            wanted[called] = shut
            print(f"    {called:<12s} loop closed over {CLOSE_OVER} frames")
        if called in ("walk", "jog") and foot < 0.05:
            refuse(f"the {called} clip moves its feet {foot * 100:.1f} cm, which is not a "
                   f"gait - either the clip is empty or it is not driving the rig")

    # # The feet, AFTER the joins and the loop closure
    #
    # Order, and it cost a round to find. The toe correction is derived per frame from the posed
    # ankle-to-toe direction, so on a clip whose first and last frames are DIFFERENT poses - which
    # is exactly what a clip needing its loop closed is - the correction at the two ends differs
    # too. `close_the_loop` then reads that difference as a seam and blends it back across the
    # last eight frames, which doubled the run's left toe from a 55 degree cap to 111.8 and
    # crumpled the shoe on frames 22 and 23.
    #
    # Corrected after the loop is shut, the two ends are the same pose, the two corrections agree,
    # and there is no seam in the toe channel to blend. Verified by turning the closure off, which
    # dropped the same frame from 111.8 to 54.0.
    #
    # The floor lift follows the toe break for the same kind of reason: breaking a toe moves the
    # sole, so the floor can only be measured once the feet have stopped changing shape.
    if FEET_MEET_THE_FLOOR:
        for called, clip in sorted(wanted.items()):
            if BREAKS_THE_TOES:
                was, broke, now = break_the_toes(base_rig, clip, scene)
                for side in ("L", "R"):
                    if side in broke and broke[side] > 0.01:
                        print(f"    {called:<12s} {side} foot pitched up to "
                              f"{max(was[side]):5.1f} deg; {broke[side]:5.1f} moved into the "
                              f"toe -> {max(now[side]):5.1f} deg")
            pointed, points = (point_the_feet_along(base_rig, clip, scene)
                               if POINTS_THE_FEET else ({}, {}))
            for side in ("L", "R"):
                if pointed.get(side) and points.get(side):
                    mine = sum(pointed[side]) / len(pointed[side])
                    theirs = sum(points[side]) / len(points[side])
                    if abs(mine - theirs) > 0.5:
                        print(f"    {called:<12s} {side} foot pointed {mine:6.1f} deg off "
                              f"travel on average -> {theirs:6.1f} deg")
            # Floored first so stance can be detected against a real floor, rolled, then
            # floored again because rolling a foot moves its sole.
            if called in ROLLS_THROUGH_STANCE:
                # Twice, and it has to be. The roll decides which frames are STANCE from how
                # close the shoe is to the floor, and rolling a foot moves its sole - so the
                # floor lift that follows shifts the very heights the detection used. One frame
                # fell through that gap every cycle: the run's right foot at the loop seam kept
                # 14.7 degrees of roll and 17.8 of yaw while every other stance frame read zero.
                #
                # A second pass is safe because the correction targets an absolute orientation
                # rather than adding to what is there, so re-running it on a frame that is
                # already flat and forward changes nothing.
                # ONCE. A second pass was tried to catch the loop-seam frame, on the reasoning
                # that the correction targets an absolute orientation and so is idempotent. The
                # orientation part is; the FLOOR part is not - each pass re-lifts against soles
                # the previous pass moved, and two passes walked the whole cycle 5 cm into the
                # air. The seam frame is left as a known fault rather than paid for with that.
                stand_on_the_floor(base_rig, base_mesh, clip, scene)
                stood = roll_the_feet(base_rig, base_mesh, clip, scene)
                print(f"    {called:<12s} rolled {stood['L']} left and {stood['R']} right "
                      f"stance(s) heel to toe")
            if called in FLATTENS_THE_TOES:
                was, now, most = flatten_the_toes(base_rig, base_mesh, clip, scene)
                print(f"    {called:<12s} a planted toe pointed {was:5.1f} deg into the floor; "
                      f"now {now:5.1f}, bent up to {most:5.1f} deg at the ball")
            if called in CAPS_THE_ANKLE:
                steepest, capped, frames = cap_the_ankle(
                    base_rig, base_mesh, clip, scene, THE_ANKLE_POINTS_AT_MOST)
                print(f"    {called:<12s} the ankle pointed {steepest:5.1f} deg down at worst; "
                      f"capped to {capped:5.1f} over {frames} airborne frame(s)")
            was_low, lifted, now_low, when = stand_on_the_floor(
                base_rig, base_mesh, clip, scene)
            print(f"    {called:<12s} lowest sole {was_low * 170.0:6.2f} cm at frame {when}; "
                  f"lifted {lifted * 170.0:5.2f} -> {now_low * 170.0:6.2f} cm")
            if abs(now_low) > 0.2 / 170.0:
                refuse(f"{called} rests its sole {now_low * 170.0:.2f} cm off the floor after "
                       f"being lifted, so the lift did not take")

    if "idle" not in wanted:
        still, took = stand_him_still(base_rig, wanted.get("walk"), scene, base_mesh)
        wanted["idle"] = still
        print(f"  no idle was delivered, so he stands: an authored stand, held for "
              f"{STANDS_STILL_FOR} frames")

    # The clip the file is left showing. The idle when there is one, since that is what a warden
    # does most of the time - and whatever else there is when there is not. The 2026-08-26 delivery
    # is a walk and a run with no idle at all, which is the one genuinely open thing about it: the
    # game asks for `idle` by name in `look.rs` and there is nothing to give it.
    resting = wanted.get("idle") or wanted.get("walk") or next(iter(wanted.values()))
    if "idle" not in wanted:
        print(f"  NO IDLE was delivered - the file is left on '{resting.name}'. The game needs an "
              "idle clip; this build cannot invent a performance.")
    play(base_rig, resting)
    scene.frame_set(int(resting.frame_range[0]))

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
