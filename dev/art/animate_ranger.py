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

# Which way is forward, and how that was settled

The model faces +X (0.969, 0.249, 0) taken off its own toe. Up is +Z. A limb swinging
fore-and-aft turns about the armature's **Y**, and the body bobs along **Z**.

The sentence that used to stand here said a positive Y swing is FORWARD, and that
knee flexion is therefore negative and elbow flexion positive. **Every part of that
was backwards, and it is why the limbs bent like a bird's for three attempts.** It
was reasoning, not measuring.

Measured instead, three ways that agree:

1. **Directly.** +10 degrees about +Y moves the end of every limb BACKWARD — all
   twelve bones, both sides, between 0.002 and 0.078 units. So forward is **−Y**.
2. **The hinge test.** Bending a joint must FOLD the limb — shorten the line from
   its root to its tip. Twists and swings do not. Of six candidate axes, the knee
   folds on +Y with the knee LEADING the hip-to-ankle line by 0.082, and the elbow
   folds on −Y with the elbow TRAILING the shoulder-to-wrist line by 0.074. Those
   are the two human directions, and the other four axes give one or the other but
   never both.
3. **The model's own idle.** The clip that shipped in `Ranger_Rig_Idle.glb` is an
   authored pose by someone who could see it, so it is ground truth in the same
   file. Its knees turn about +Y by 42 degrees and its elbows trail by 0.037. It
   agrees with the hinge test on both.

So the axes below are NAMED for what they do, not for their sign, and the sign is
recorded once where it was measured rather than re-derived at each call site. That is
the whole fix: `bend the knee 40 degrees` cannot be written backwards.
"""

import math
import os
import sys

import bpy
import mathutils

# Blender does not put a --python script's own directory on the path, so a sibling
# module is not importable without this.
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import gather_clips  # noqa: E402
import ik_gait  # noqa: E402

# Which armature axis each joint turns about, and which way is positive. Measured —
# see the module docstring. Named so that a caller states an intention and cannot
# state a sign.
REACHES_FORWARD = (0.0, -1.0, 0.0)
FOLDS_THE_KNEE = (0.0, 1.0, 0.0)
FOLDS_THE_ELBOW = (0.0, -1.0, 0.0)
LIFTS_THE_TOE = (0.0, -1.0, 0.0)

# And the same axis the other way round, for the SPINE.
#
# # A limb hangs down; a spine stands up
#
# `REACHES_FORWARD` was measured on thighs and upper arms, which point DOWN from their
# joints. A spine points UP. The identical rotation therefore carries a thigh's foot
# forward and a spine's head BACKWARD, and using the constant named "reaches forward"
# on the waist leant the torso back at every speed - reported as "human spines don't
# lean back when we run", which is exactly right.
#
# Measured, not reasoned, like the rest of them: a positive ten degrees about
# `REACHES_FORWARD` moves the head 0.054 units BACKWARD from the hips and the left
# foot 0.079 FORWARD. `Waist`, `Spine01` and `Spine02` all point +Z; `L_Thigh` points
# -Z.
#
# The lesson is about the NAME. An axis named for what it does cannot be written
# backwards at a call site, which is what these constants are for - but the name is
# only true for bones of the orientation it was measured on. So there are two, and
# each says which way its bones point.
LEANS_THE_TORSO_FORWARD = (0.0, 1.0, 0.0)

# How far a planted foot travels while it is down, in model units.
#
# ONE number for every gait, which is not a simplification - it is the finding. Planted
# foot travel stays near one leg length from a jog to a world-class sprint, measured at
# 0.99 plus or minus 0.08 m across speeds from 6.2 to 11.1 m/s. What changes with speed
# is how long the foot stays down, not how far it goes while it is there.
#
# So the stride comes from the STANCE SHARE and nothing else: covers = contact / share,
# which is 0.83 m a cycle over 0.625 for the walk, 1.39 over 0.375 for the jog and 2.09
# over 0.25 for the sprint. 0.52 units is 0.88 m at 1.7 m scale, or 1.14 of this
# character's own 0.455-unit leg - the stylised 14% over, stated rather than stumbled
# into.
CONTACT = 0.52

# --- The eight poses of a cycle
#
# CONTACT, DOWN, PASSING, UP for one step, then the same four for the other leg. A
# walk authored on four poses is the classic "cheap walk": contact and passing only,
# no recoil and no high point, which is why it read as a mannequin gliding on rails
# with its legs cycling underneath. The reference brief is blunt that four per CYCLE
# is half of what a weighted walk needs, and that down and up are the two carrying
# the weight.
#
# Eight lands on frames 1, 4, 7, 10, 13, 16, 19, 22 and 25 of a twenty-four frame
# clip, which is exactly the published breakdown for a 25-frame cycle.
POSES = 8

# One leg through a full cycle: (thigh, knee, ankle) in degrees at each eighth,
# thigh forward-positive, knee fold-positive, ankle toe-up-positive. The other leg
# reads the same table half a cycle along, which is what makes the two legs
# genuinely out of phase rather than mirrored — mirroring one step to make the other
# is a named cause of a limp in three-quarter view.
#
# Off the clinical table in the reference brief, which gives hip, knee and ankle by
# percentage of the cycle from heel strike. Four departures, all deliberate:
#
# 1. The thigh swing is ASYMMETRIC, +32 forward against -20 back, because a symmetric
#    swing "reads slightly wrong even when everything else is right".
# 2. It is also LONGER than clinical. Stride length is half of `speed = cadence x
#    stride`, and a stride authored short forces a frantic cadence to reach the
#    game's speed - the failure mode the run brief calls "frantic little steps".
# 3. The peak fold is 68, at the top of the clinical 60-70. It is what makes a step
#    read as a step, and "the knees do not bend as much as they should" was the note
#    that moved it.
# 6. The stance window spans 74 degrees of thigh in every running clip, and that
#    number is measured rather than chosen. A sweep of thigh span against the resulting
#    contact length gives 0.880 m at 74 degrees, and contact divided by the stance
#    fraction IS the stride: 0.880/0.375 is 2.35 m for the jog and 0.880/0.25 is 3.52
#    for the sprint, which at 180 steps a minute carry 3.53 and 5.28 m/s - against
#    Palworld's 3.50 jog and Skyrim's 5.29 run.
#
#    Worth recording that this character's leg is 0.774 m hip to ankle, not the 0.90 a
#    1.7 m human has: a stylised build with proportionally short legs, 45% of its
#    height against a human's 52%. "Contact length is about one leg length" therefore
#    means 0.774 m HERE, and 0.880 is already reaching 14% past it. That is a
#    stylisation, stated rather than stumbled into.
#
# 7. The ankle at a running CONTACT is plantarflexed, -16 for the jog and -18 for the
#    sprint, where a walk's is +12. A run lands on the forefoot and a walk on the heel,
#    and the number has to fight the chain above it: at contact the thigh is 36 degrees
#    forward and the knee folded 12, which leaves the shin pitched 24 degrees forward,
#    so an ankle at +4 still points the toe UP. Measured against the ground rather than
#    against the shin - the check in verify_gait.py refuses a flying clip that lands
#    heel-first, which is what caught it.
#
# 5. The knee at TOE-OFF is 24, not the clinical 40. Rendered and looked at: at 40
#    the trailing foot came fully off the ground at the moment the other foot lands,
#    and the whole thing read as bounding rather than walking. A real walk keeps the
#    ball of that foot loaded through double support, and this rig has no toe joint to
#    keep it down with while the knee folds - so the fold gives way and the ankle
#    takes over, at -30 of plantarflexion.
# 4. The knee holds 16 degrees at PASSING and goes dead straight at UP, where the
#    clinical curve is nearer 8 and 3. That pair is what puts the hips highest at the
#    up pose rather than at passing. Planting the stance foot means hip height IS the
#    stance leg's vertical extent, so the high point lands wherever that leg is
#    longest - and a knee still carrying a bend at midstance is what moves it later.
WALK_LEG = (
    (38.0, 4.0, 12.0),  # 0     contact - heel strike, toes up, knee nearly straight
    (26.0, 20.0, -6.0),  # 12.5  down - the recoil, foot flat, weight loaded
    (6.0, 16.0, 5.0),  # 25    passing - body over the foot, knee still carrying
    (-10.0, 0.0, 12.0),  # 37.5  up - leg dead straight, heel lifting, hips highest
    (-27.0, 24.0, -32.0),  # 50    toe-off - ball of the foot still loaded
    (-6.0, 64.0, -8.0),  # 62.5  initial swing - knee folding to its peak
    (14.0, 68.0, 0.0),  # 75    mid-swing - clearing the ground
    (33.0, 22.0, 9.0),  # 87.5  terminal swing - reaching, presenting the heel
)

# And a run, which is not a bigger walk. It has a FLIGHT phase, so both feet leave
# the ground; it lands on the forefoot with the knee already flexed rather than on a
# straight-legged heel; and its knee folds far further - a sprinter's heel comes up
# toward the buttock, which is past 90 degrees, where a walk peaks near 62.
# The first RUN_STANCE rows are the ones with the foot DOWN, and the rest are the
# swing. Getting that alignment wrong put the push-off on a row that `who_is_planted`
# had already sent airborne, so the body pushed off nothing and the hips took their
# shape from the absorption instead - four vertical peaks per cycle where a run has
# two, and a limp on top.
RUN_LEG = (
    (36.0, 12.0, -16.0),  # 0     contact - forefoot lands, knee lightly loaded
    (2.0, 38.0, -6.0),  # 12.5  mid-stance - deepest absorption, the lowest point
    (-38.0, 4.0, -32.0),  # 25    push-off - leg extended hard behind, driving back
    (-30.0, 70.0, -14.0),  # 37.5  early flight - knee folding up behind
    (-6.0, 100.0, -6.0),  # 50    peak fold - heel toward the buttock
    (22.0, 92.0, 0.0),  # 62.5  knee drive - thigh coming through high
    (38.0, 50.0, 4.0),  # 75    reaching - the furthest the leg gets in front
    (34.0, 26.0, 4.0),  # 87.5  descending - foot coming back under to land
)

# And a sprint, which differs from the run mostly in how LITTLE of it is spent on
# the ground. The leg angles are not far off the run's - planted-foot travel stays
# near one leg length at every speed, measured at 0.99 plus or minus 0.08 m from
# 6.2 m/s to 11.1 - so the extra stride comes from `SPRINT_STANCE`, not from reaching
# further. What does change is the knee: a sprinter's heel comes right up to the
# buttock, past 125 degrees, and the thigh drives through high in front.
# Two stance rows only, so that pair carries the whole of contact, absorption and
# push-off between them - which is what a 25% duty factor means.
#
# And they must span a whole CONTACT LENGTH between them, which is the thing that was
# got wrong first. Planted-foot travel is about one leg length in every gait from a
# jog to a world-class sprint (0.99 plus or minus 0.08 m measured across 6.2 to
# 11.1 m/s), so a faster gait covers the same ground in LESS TIME rather than covering
# less ground. Shrinking the thigh travel along with the pose count is what dropped the
# measured contact length from 0.80 m in the walk to 0.37 in the sprint, and with it
# the stride: covers = contact / stance, so a short contact undoes the whole point of
# a short stance.
SPRINT_LEG = (
    (36.0, 12.0, -18.0),  # 0     contact - forefoot, knee lightly loaded
    (-38.0, 4.0, -34.0),  # 12.5  stance - absorbs and drives through in ONE pose
    (-30.0, 46.0, -30.0),  # 25    early flight - leaving the ground behind
    (-34.0, 90.0, -16.0),  # 37.5  flight - knee folding hard
    (-4.0, 125.0, -6.0),  # 50    peak fold - heel at the buttock
    (30.0, 112.0, 0.0),  # 62.5  knee drive - thigh through high
    (48.0, 62.0, 4.0),  # 75    reaching - the furthest in front
    (44.0, 28.0, 2.0),  # 87.5  descending - coming back under to land
)

# --- The arms
#
# Sampled from a curve rather than a table, because an arm swing really is close to
# a sinusoid and because the LAG has to be a fraction of a cycle rather than a row
# in a table. Asymmetric, more extension than flexion: +10 forward against -17 back
# is 27 degrees total, inside the 20-30 the brief gives for a normal walk.
ARM_FORWARD = 14.0
ARM_BACK = -22.0

# A run swings its arms far harder, and this is not a style choice: arm-swing
# amplitude is measured to scale up with gait velocity, and the check in
# `verify_gait.py` refuses a clip whose hands cover less than a quarter of what its
# feet do - which the run failed outright at a walk's amplitude once its stride grew.
RUN_ARM_FORWARD = 34.0
RUN_ARM_BACK = -46.0

# A sprint drives the arms harder still, and the check in `verify_gait.py` refuses a
# clip whose hands cover less than a quarter of what its feet do - which a sprint's
# much longer stride makes easy to fail.
SPRINT_ARM_FORWARD = 46.0
SPRINT_ARM_BACK = -58.0

# How far the arm extremes fall BEHIND the leg extremes, as a share of the cycle.
#
# Two independent sources put it at two to three frames of a twenty-four frame
# cycle, which is where 0.10 comes from. Arms hitting their extremes on the same
# frame as the feet is a named failure - it reads as mechanical and synchronised,
# like a wind-up toy - and the old clip had no lag at all.
ARM_LAG = 0.10

# The elbow bends more at the forward extreme and straightens toward the back one,
# because an elbow cannot fold backward: an arm swinging behind the body has to
# straighten. Base 25 with 12 either way gives 37 in front and 13 behind, inside
# the brief's 35-45 front and 10-20 back.
ELBOW_HELD = 25.0
ELBOW_SWING = 12.0
RUN_ELBOW_HELD = 62.0
RUN_ELBOW_SWING = 18.0

# --- The pelvis, expressed on the LEGS
#
# # Why not on the pelvis bone
#
# These were rotations of `Pelvis`, which carries both thighs, and they made the walk
# LIMP: one half of the cycle bobbed 4.57 cm and the other 2.95, peaking ten frames
# apart instead of twelve. Zeroing them made the halves exact, which is what named
# them; halving them was not enough.
#
# The cause is in the asset. This rig is not mirror-symmetric - `L_Thigh`'s local X
# runs (-0.007, -0.999, -0.044) against `R_Thigh`'s (+0.007, -0.992, +0.125) - so one
# shared rotation on their parent cannot move the two legs alike, and whatever it does
# to the STANCE leg's length is a change `plant` then feeds straight into the hips.
#
# So the pelvis's two rotations are stated on the legs instead, where each side is
# independent and the stance leg's vertical extent stays a pure function of its own
# three angles. That makes the two halves identical by construction rather than by
# hoping the rig is symmetric, and it is faithful to the thing being modelled: the
# brief's own reason for wanting hip yaw is that "the hips and legs are a unit" and
# that hip rotation is part of what lengthens a stride, which is exactly what adding
# it to a thigh's reach does.
#
# The obliquity's sign was ALSO backwards, which is worth recording because the brief
# warns it is the one people invert. Armature +X is forward, the left bones sit at +Y,
# and a positive turn about +X carries +Y onto +Z - so the old `+PELVIS_DROP` raised
# the SWING hip, giving the hip-hitch strut the brief describes instead of the drop.
#
# The old comment, kept because the hierarchy fact in it is true and useful: all of
# them were set on `Pelvis`, which carries the legs
# and NOT the spine - `Hip` parents `Pelvis` and `Waist` as siblings - so the torso
# counter-rotation the brief asks for falls out of the hierarchy instead of needing
# a spine twist to cancel a hip twist.
#
# YAW, as extra reach for the swinging leg and a little less for the stance one.
# Skipping it gives "tiny steps" and a mechanical look.
PELVIS_YAW = 6.0

# OBLIQUITY, as adduction of the swinging leg - the swing side hangs DOWN and in,
# the stance side rides high. Up on the swing side reads as a hip-hitch, with the leg
# looking yanked clear rather than clearing naturally.
PELVIS_DROP = 4.0

# And nought for the gaits that fly.
#
# The drop is applied as adduction of each thigh with the sign flipped by side, and
# this rig's thighs are not mirror images: `L_Thigh`'s local X runs
# (-0.007, -0.999, -0.044) against `R_Thigh`'s (+0.007, -0.992, +0.125). So the two
# sides do not receive equal treatment, and the leftover asymmetry lands in the hips
# through the planting.
#
# A walk carries it - five stance poses a leg spread the error thinly and the halves
# still match to 0.89 - but a run has three and a sprint two, so the same error weighs
# twice as much and the run limped at 0.59 against a floor of 0.80. The obliquity is
# also the least visible thing in a clip whose legs are already dramatic, which makes
# it the right thing to give up.
RUN_DROP = 0.0

# SWAY, once per cycle, toward whichever foot is bearing the weight. In model units
# on a figure one unit tall, so about 3 cm at 1.7 m.
PELVIS_SWAY = 0.018

# --- How much of the cycle is spent on the ground, and how far the body arcs
# while none of it is
#
# `STANCE` is how many of the eight poses EACH LEG is planted for. It is the formal
# difference between a walk and a run: stance above half the cycle is a walk, below
# it is a run, and that quantity has a name - the duty factor.
#
# It is also the stride. Planted-foot travel stays near one leg length whatever the
# speed, and stride is that contact length divided by the stance fraction, so a
# longer stride is bought by spending LESS time on the ground. This is the knob that
# was missing: reaching further instead is what made 42 degrees of thigh swing read
# as the splits, and it is why the run could not carry the speed it was asked to.
#
# A walk overlaps at five, so some foot is always down and there is nothing to arc.
WALK_STANCE = 5
RUN_STANCE = 3
SPRINT_STANCE = 2

# And how far the body rises above the straight line between two planted poses while
# it is airborne. Nought for a walk, which never leaves the ground.
#
# A run's vertical oscillation is 6 to 9 cm at recreational pace; below about 5 it
# reads as a shuffle that never reaches flight. In model units on a figure one unit
# tall, 0.022 is 3.7 cm of arc on top of whatever the planted geometry already gives,
# and 0.034 is 5.8.
WALK_BOUND = 0.0
RUN_BOUND = 0.022
SPRINT_BOUND = 0.034

# How far the torso leans forward, in degrees, spread over the waist and lower spine.
#
# Real runners sit between 4 and 12 degrees of trunk flexion, most economically near 6,
# and game guidance routinely overstates it: a "pronounced 15 to 30 degree lean" is
# quoted for sprints against a measured 4 to 8. That is a two-to-four-times push, and
# the cost of taking it is that the character reads as ACCELERATING permanently rather
# than as running.
#
# And the sprint leans LESS than the jog, which is the opposite of the obvious guess
# and was wrong here before: maximum-velocity sprinting is more upright than jogging,
# because a big lean belongs to acceleration - 45 degrees at a sprinter's block exit,
# nearly nothing at top speed. Keeping the sprint a shade under the jog is what stops
# it reading as a permanent launch.
RUN_LEAN = 9.0
SPRINT_LEAN = 8.0

# The arms hang a little OUT and the palms turn IN, in every frame, so the hands
# clear the pockets. The generator's bind pose parks them ON the pockets — glove and
# trouser come within 0.003 of touching — so a few degrees of abduction is what keeps
# the fingers out of the cloth.
#
# This pair used to be a workaround as well, and the arm swing used to be 6 degrees
# rather than 20 to hide a mesh fault under a motion too small to tear it. That fault
# is repaired at its cause now — see `unfuse_the_gloves_from_the_pockets` — so the
# swing amplitudes are back to being nothing but a gait choice, and they come off
# `ARM_FORWARD` and `ARM_BACK` above.
ARM_OUT = 5.0
PALM_IN = 10.0

# --- Two repairs to the idle the model shipped with. See `mend_the_shipped_idle`.
#
# A half-turn out of each forearm, which is what puts the palms back against the
# thighs instead of facing forward with the fingers splayed.
UNTWIST = 180.0
FOREARMS = ("L_Forearm", "R_Forearm")

# What angle each leg is stood at, in degrees out from the hip toward the foot.
#
# # Why an angle rather than a gap, and why it is asked for rather than added
#
# The hips are 30 cm apart at this rig's scale and a person stands with the heels 10 to
# 15 apart, so a real leg CONVERGES slightly on its way down - a few degrees inward,
# which is what this negative number is.
#
# It replaced two mistakes at once. The first was `STANCE_OPENS`, six degrees applied
# about armature Y - which is the LATERAL axis, so it swung the legs fore-and-aft
# instead of opening them. Measured afterwards, the idle held its feet 4.6 cm apart
# across the body and 12.0 cm apart front to back: not a wider stance, a split one. It
# was rendered and accepted at the time because the feet did look further apart, and
# nobody asked along WHICH AXIS they had separated.
#
# The second was correcting each leg by its rest-pose splay. That is an open loop: it
# assumes the clip's poses are near rest, and the shipped idle shifts its weight from
# foot to foot, so the constant left the legs 8 and 11 degrees INWARD at the start -
# knock-kneed.
#
# So the angle is ASKED FOR and the correction is measured against what is really
# there. See `stand_the_leg_up`.
LEGS_SIT_AT = -3.5

# What angle each foot is pointed at, in degrees out from the line of travel.
#
# The model rests with 18.5 degrees of toe-out apiece - 37 between the two feet - where
# a person has 7 to 10. Reported as "feet flair out when running seems unnatural", and
# measured to be the asset's own pose rather than anything the clips do.
#
# Asked for rather than added, for the same reason as `LEGS_SIT_AT`: a fixed eleven
# degrees of correction was right until the legs started being stood up too, at which
# point it landed at 6 degrees on one clip and 18 on another, because rotating a leg
# also turns the foot it carries.
TOES_SIT_AT = 8.0
FEET = ("L_Foot", "R_Foot")


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


def turn_further(rig, bone: str, degrees: float, axis):
    """Adds a turn ON TOP of whatever the bone is already holding.

    `swing` REPLACES a bone's rotation, which is what a pose wants. Layering a
    second motion onto the same bone - the pelvis's yaw and its obliquity, or an
    arm's swing and its abduction - needs composition instead, and doing it by
    calling `swing` twice silently discards the first.
    """
    posed = rig.pose.bones.get(bone)
    if posed is None:
        return
    rest = posed.bone.matrix_local.to_3x3()
    local = (rest.inverted() @ mathutils.Vector(axis)).normalized()
    posed.rotation_mode = "QUATERNION"
    posed.rotation_quaternion = (
        mathutils.Quaternion(local, math.radians(degrees)) @ posed.rotation_quaternion
    )


def under_the_foot(rig, side: str):
    """The three points that stand in for one foot's sole, in armature Z.

    The heel is not a bone, so the bones that ARE there stand in for the sole: the
    ankle, the ball and the toe. At heel strike the ankle end is the low one and at
    toe-off it is the toe.
    """
    return (
        (rig.matrix_world @ rig.pose.bones[f"{side}_Foot"].head).z,
        (rig.matrix_world @ rig.pose.bones[f"{side}_ToeBase"].head).z,
        (rig.matrix_world @ rig.pose.bones[f"{side}_ToeBase"].tail).z,
    )


def where_each_sole_rests(rig):
    """Each of those points' height above the ground, measured in the REST pose.

    # Why a flat minimum will not do

    Taking the lowest of the three directly gives a sole that jumps. The ankle sits
    higher off the ground than the toe does, so the instant the lowest point switches
    from one bone to another - which is exactly what happens between heel strike and
    toe-off - the measured sole steps by the difference between them, and planting
    against it steps the hips with it.

    Measured, that produced a walk whose hips rose 10.6 cm with THREE high points per
    cycle instead of two: the real one at the up pose, and two artefacts where the
    lowest bone changed hands.

    So each point carries its own rest height. Subtracting it makes all three agree in
    the rest pose - a flat foot on flat ground - and each then tracks the sole
    correctly as the ankle rolls, which is the whole point.

    # And each FOOT gets its own ground, which matters more than it sounds

    This model does not stand level: the right sole rests 1.4 cm higher than the left.
    Planting both feet against one shared ground therefore made the two halves of the
    cycle geometrically different, and the hips peaked at 21% of the cycle in one half
    and 62% in the other where a symmetric walk peaks at 25 and 75. An asymmetric bob
    is a LIMP, and it is the most likely thing behind "the legs do not feel like they
    are moving correctly" while every direction measures correct.

    Per-foot grounds fix it exactly, and the arithmetic says why: at rest, each sole's
    height plus that leg's vertical extent equals the same hip height, because that is
    what the rest pose IS. So planting each foot to its own rest level makes both
    halves agree pose for pose, while planting both to a shared level forces the
    difference into the hips.
    """
    rest(rig)
    bpy.context.view_layer.update()
    # ONE ground for both feet, and it is the lower of the two rest soles so that
    # nothing sinks through it.
    #
    # # Per-foot grounds were tried, and they are the limp
    #
    # It seemed right: this model does not stand level, the right sole resting 1.4 cm
    # higher than the left, so planting each foot to its own rest level looked like
    # respecting the asset. The argument was that a sole's height plus its leg's
    # extent equals the hip height at rest, so both would agree.
    #
    # That argument is wrong, and the run made it obvious - one half of the cycle
    # bobbing 0.024 and the other 0.034. The rest extents are only unequal BECAUSE
    # the soles are: pose both legs to the same angles and their extents become
    # equal, at which point two different grounds put the hips at two different
    # heights, once per step. That is a limp of exactly the 1.4 cm the rest pose was
    # out by.
    #
    # A shared ground makes the two halves identical by construction. The per-point
    # offsets stay per-foot, because those really are properties of each foot - and
    # measured, they agree to within 0.6 mm anyway.
    ground = min(min(under_the_foot(rig, side)) for side in "LR")
    return {
        side: (ground, tuple(z - min(under_the_foot(rig, side)) for z in under_the_foot(rig, side)))
        for side in "LR"
    }


def across_the_body(rig):
    """The unit vector along the body's travel, and the one across it.

    # Call this at REST, once, and pass the answer around

    It takes the direction off the feet, so it is only the body's own forward while the
    feet are still pointing that way. Called mid-pose it reads whatever the stride is
    doing - and a stride has one foot folded up behind the hip, whose shadow on the
    ground points nowhere useful.

    Doing exactly that is what made the toe-in closed loop miss: it asked for eight
    degrees of toe-out against a reference recomputed from the posed feet each time,
    and landed at -21, -46 and -57. The loop was right and its yardstick was moving.
    """
    toward = mathutils.Vector((0.0, 0.0, 0.0))
    for side in "LR":
        span = (rig.matrix_world @ rig.pose.bones[f"{side}_ToeBase"].tail) - (
            rig.matrix_world @ rig.pose.bones[f"{side}_Foot"].head
        )
        toward += mathutils.Vector((span.x, span.y, 0.0)).normalized()
    forward = toward.normalized()
    return forward, mathutils.Vector((-forward.y, forward.x, 0.0))


def how_far_the_leg_splays(rig, side: str, across) -> float:
    """How far one whole leg leans OUT across the body, hip to ankle, in degrees.

    Hip to ankle rather than segment by segment, because that is what reads: a thigh
    angled out and a shin angled back in look straight, and the eye follows the line
    of the whole limb.
    """
    along = (rig.matrix_world @ rig.pose.bones[f"{side}_Foot"].head) - (
        rig.matrix_world @ rig.pose.bones[f"{side}_Thigh"].head
    )
    sideways = along.dot(across) * (1.0 if side == "L" else -1.0)
    return math.degrees(math.atan2(sideways, max(1e-6, -along.z)))


def point_the_foot_along(rig, side: str, want: float, forward, across) -> float:
    """Yaws one foot until it points `want` degrees out from the line of travel.

    Closed, like `stand_the_leg_up` and for the same reason. A fixed eleven degrees of
    toe-in worked until the legs started being stood up as well - rotating a leg about
    the forward axis also turns the foot it carries, by an amount that depends on how
    far that leg happens to be swung. The fixed correction then landed at 6 degrees on
    one clip and 18 on another.
    """
    span = (rig.matrix_world @ rig.pose.bones[f"{side}_ToeBase"].tail) - (
        rig.matrix_world @ rig.pose.bones[f"{side}_Foot"].head
    )
    flat = mathutils.Vector((span.x, span.y, 0.0))
    if flat.length < 1e-6:
        return 0.0
    flat.normalize()
    hand = 1.0 if side == "L" else -1.0
    was = math.degrees(math.atan2(flat.dot(across), flat.dot(forward))) * hand
    # Positive about armature up carries a foot toward +Y, which is OUT on the left
    # and IN on the right - hence the hand again.
    turn_further_absolutely(rig, f"{side}_Foot", (want - was) * hand, (0.0, 0.0, 1.0))
    return was


def stand_the_leg_up(rig, side: str, want: float, across) -> float:
    """Rotates one leg until it splays by `want` degrees. Returns what it was.

    # A closed loop, because an open one cannot know what it is correcting

    This was a fixed correction: measure each leg's splay once in the rest pose and
    subtract it everywhere. That works on a clip whose own poses are near rest and
    fails on one that is not - the shipped idle shifts its weight from foot to foot,
    so subtracting a constant left the legs 8 and 11 degrees INWARD at the start of it
    and knock-kneed on screen.

    Measuring per pose costs a depsgraph flush and cannot be wrong about what it is
    starting from. It also means the amount is not a number anybody has to maintain:
    ask for the angle you want the leg to sit at and the arithmetic is done against
    whatever is actually there.
    """
    was = how_far_the_leg_splays(rig, side, across)
    hand = 1.0 if side == "L" else -1.0
    # Positive about armature forward carries a downward-pointing limb toward +Y,
    # which is OUT on the left and IN on the right - hence the hand.
    turn_further_absolutely(rig, f"{side}_Thigh", (want - was) * hand, (1.0, 0.0, 0.0))
    return was


def turn_further_absolutely(rig, bone: str, degrees: float, axis) -> None:
    """Rotates a bone's posed orientation about a TRUE armature axis.

    # The arithmetic, because getting it nearly right is worse than not trying

    A pose bone's final orientation in armature space is `M @ chan`, where `chan` is
    `rotation_quaternion` and `M` is everything above it - the parent's pose, the
    offset, and the bone's own rest matrix. To turn that final orientation by `R`:

        R @ (M @ chan) = M @ chan @ Q      =>      Q = (M @ chan)^-1 @ R @ (M @ chan)

    which is `R` expressed in the bone's FULLY POSED frame, and `chan` is then
    POST-multiplied by it. `pose_bone.matrix` is exactly `M @ chan`, so the frame is
    there to be read.

    Two earlier versions were wrong in two different ways, and each looked plausible:

    * `turn_further` conjugates with the bone's REST matrix and pre-multiplies. That is
      correct for a rotation meant to be relative to the parent - a knee bending
      against its thigh - and wrong for one meant to be absolute, because the axis gets
      carried around by every posed ancestor. Ten degrees of yaw asked for that way
      moved one foot twenty-seven degrees and the other nine the wrong way.
    * Then the parent's pose was folded in but the multiplication left on the wrong
      side. Pre-multiplying applies the turn BEFORE the bone's own rotation instead of
      after it, so a foot already pitched thirty degrees at the ankle received
      something that was neither the yaw asked for nor nothing: the toe-in overshot to
      21, 46 and 57 degrees the wrong way.

    Both are the same mistake at bottom - assuming a rotation composes the way it reads
    - and both are why this docstring carries the algebra instead of a description.
    """
    posed = rig.pose.bones.get(bone)
    if posed is None:
        return
    frame = posed.matrix.to_3x3()
    if frame.determinant() == 0.0:
        return
    local = (frame.inverted() @ mathutils.Vector(axis)).normalized()
    posed.rotation_mode = "QUATERNION"
    posed.rotation_quaternion = posed.rotation_quaternion @ mathutils.Quaternion(
        local, math.radians(degrees)
    )



def sole_of_the_foot(rig, side: str, resting) -> float:
    """How low one foot's sole reaches, with each point's rest height taken out."""
    return min(z - high for z, high in zip(under_the_foot(rig, side), resting[side][1]))


def raise_the_hips(rig, by: float) -> None:
    """Moves the hips vertically, on top of whatever sway is already on them."""
    hips = rig.pose.bones.get("Hip")
    if hips is None:
        return
    rest_axes = hips.bone.matrix_local.to_3x3().inverted()
    up = (rest_axes @ mathutils.Vector((0.0, 0.0, 1.0))).normalized()
    hips.location = hips.location + up * by


def plant(rig, resting, stance: str) -> float:
    """Drops the hips so that the lower foot rests on the ground.

    # The bob is not authored. It is what planting a foot leaves behind.

    A hip bob added on top of posed legs is a SECOND source of the same motion, and
    the two disagree: the authored curve says the body rises at the passing pose
    while the geometry says it rises wherever the stance leg is straightest. The
    planted foot then slid 9.5 cm per cycle, which is the skating read.

    So nothing is authored. The legs are posed, the lower foot is measured, and the
    hips are moved by exactly the difference. `Hip` is above every leg bone, so the
    correction translates the feet by precisely the amount it was asked to - one
    measurement, one shift, no iteration.

    What comes out is the bob a real walk has, for the reason a real walk has it:
    the body is higher when the stance leg is extended, because a straighter leg is
    a longer leg. Its amplitude and its phase are then consequences rather than
    choices, and `verify_gait.py` reports whether they landed where the brief says.

    # Which foot to plant is KNOWN, not discovered

    Planting whichever foot measures lowest sounds more robust and is worse. The
    model's rest pose is not level - the right sole sits 1.4 cm higher than the left -
    so the lower foot changes hands at moments that have nothing to do with the gait,
    and the hips step each time it does. That produced THREE high points per cycle
    where a walk has two.

    An eight-pose cycle already says which foot is down: the right lands at the start
    and pushes off halfway, so it carries the first half and the left carries the
    second. Passing that in is exact, and it is what an animator does - you plant the
    foot you know is on the ground.
    """
    bpy.context.view_layer.update()
    return resting[stance][0] - sole_of_the_foot(rig, stance, resting)


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


def who_is_planted(step: int, stance: int):
    """Which foot is on the ground at this pose, or None if the body is airborne.

    # Stance is a COUNT, and it is what separates a walk from a run

    `stance` is how many of the eight poses each leg spends on the ground. A walk has
    five, so the two legs overlap and some foot is always down. A run has three and a
    sprint two, so there are poses where neither leg is planted — that is the flight
    phase, and it is not a detail. Duty factor is the formal difference between
    walking and running: stance above half the cycle is a walk, below it is a run.

    It is also where a long stride comes from. Planted-foot travel stays near one leg
    length at every speed — measured at 0.99 plus or minus 0.08 m from 6.2 m/s to
    11.1 — and stride is that contact length divided by the stance fraction. So the
    way to a 3.5 m stride is to spend LESS of the cycle on the ground, not to reach
    further. Reaching further is what made 42 degrees of thigh swing read as the
    splits.
    """
    right = step % POSES
    left = (right + POSES // 2) % POSES
    if right < stance:
        return "R"
    if left < stance:
        return "L"
    return None


def pose_the_body(rig, leg, step: int, phase: float, reach: float, back: float,
                  elbow_held: float, elbow_swing: float, lean: float,
                  drop: float, facing) -> None:
    """Sets every bone for one pose, hips excepted.

    Factored out because the hips need TWO passes over the cycle — one to find out
    where planting puts them, another to key them once the airborne stretches have
    been filled in — and posing the body twice from one description is the only way
    those passes can agree about what they are looking at.
    """
    rest(rig)
    for side, hand in (("L", 1.0), ("R", -1.0)):
        # The right leg leads at phase nought and the left half a cycle later.
        # `hand` is +1 on the left, so the left reads the table offset by half.
        at = (step + (POSES // 2 if hand > 0.0 else 0)) % POSES
        thigh, knee, ankle = leg[at]

        # The pelvis's yaw and obliquity, on this leg. `swinging` runs from -1 when
        # the leg is planted to +1 when it is carrying itself through, which is a
        # leg's own half of the cycle: it reads the table at index `at`, so the first
        # four rows are its stance and the last four its swing.
        swinging = -1.0 if at < POSES // 2 else 1.0
        swing(rig, f"{side}_Thigh", thigh + PELVIS_YAW * swinging, REACHES_FORWARD)
        swing(rig, f"{side}_Calf", knee, FOLDS_THE_KNEE)
        swing(rig, f"{side}_Foot", ankle, LIFTS_THE_TOE)
        # And the drop, as adduction: the swinging leg hangs down and in toward the
        # midline while the standing one holds the hip up.
        turn_further(rig, f"{side}_Thigh", drop * swinging * hand, (1.0, 0.0, 0.0))

        # An arm opposes the leg on its OWN side, so the left arm is forward when the
        # left leg is back: the left leg's phase plus a half turn. The lag is
        # subtracted from the phase, which is what puts the arm's extreme later in
        # time than the leg's.
        same_leg = phase + (0.5 if hand > 0.0 else 0.0)
        swung = math.cos(2.0 * math.pi * (same_leg - 0.5 - ARM_LAG))
        middle = (reach + back) / 2.0
        half = (reach - back) / 2.0
        swing(rig, f"{side}_Upperarm", middle + half * swung, REACHES_FORWARD)
        # Most bend at the forward extreme, straightest at the back one - an elbow
        # cannot fold the other way, so an arm going behind straightens.
        swing(rig, f"{side}_Forearm", elbow_held + elbow_swing * swung, FOLDS_THE_ELBOW)
        # Abduction, so the hands clear the pockets, composed on top of the swing
        # rather than replacing it.
        turn_further(rig, f"{side}_Upperarm", ARM_OUT * hand, (1.0, 0.0, 0.0))
        # The palms face the thighs and stay there: pronation through an arm swing is
        # only about fourteen degrees, so a palm that visibly rolls is wrong.
        swing(rig, f"{side}_Hand", PALM_IN * hand, axis=(0.0, 0.0, 1.0))

    # Sway ONCE per cycle, toward whichever foot carries the weight: the right at a
    # quarter, the left at three quarters. This one stays on `Hip` because a pure
    # translation moves both legs by the same vector, so it cannot make the two
    # halves differ.
    shift(rig, "Hip", -PELVIS_SWAY * math.sin(2.0 * math.pi * phase), (0.0, 1.0, 0.0))
    # The legs stood up under the body and the feet yawed in toward the line of
    # travel, both measured against what is actually there rather than applied blind,
    # and both about true armature axes - by this point a foot's own rest frame has
    # been carried a long way off by its thigh and knee. See `stand_the_leg_up` and
    # `turn_further_absolutely`.
    forward, across = facing
    for side in "LR":
        bpy.context.view_layer.update()
        stand_the_leg_up(rig, side, LEGS_SIT_AT, across)
        bpy.context.view_layer.update()
        point_the_foot_along(rig, side, TOES_SIT_AT, forward, across)

    # The lean into a run, on the waist and lower spine rather than the hips so the
    # legs keep their own frame. Spread across two joints so the back curves into it
    # instead of hinging at one point.
    #
    # `LEANS_THE_TORSO_FORWARD`, NOT `REACHES_FORWARD` - see the constant. The spine
    # points up where a limb points down, so the two need opposite signs to do the
    # same thing, and using the limb's constant here leant the torso backwards.
    swing(rig, "Waist", lean * 0.4, LEANS_THE_TORSO_FORWARD)
    swing(rig, "Spine01", lean * 0.6, LEANS_THE_TORSO_FORWARD)


def fill_in_the_flight(lift, bound: float):
    """Fills the airborne poses with a ballistic arc between the planted ones.

    A body with no foot down is a projectile: it leaves the ground going up, slows,
    falls, and lands. So the airborne stretches are filled with a parabola that
    matches the planted heights at each end and rises `bound` above the straight line
    between them.

    Leaving them flat instead is what made the old run read as a bouncy walk — the
    hips held still exactly where a runner is rising fastest. Leaving them to an
    authored sine wave was worse, because its phase was a walk's: highest at
    midstance, where a runner is at their LOWEST.
    """
    known = [i for i, v in enumerate(lift) if v is not None]
    if not known:
        return [0.0] * len(lift)
    out = list(lift)
    for gap_start, gap_end in zip(known, known[1:]):
        if gap_end - gap_start < 2:
            continue
        low, high = out[gap_start], out[gap_end]
        for i in range(gap_start + 1, gap_end):
            along = (i - gap_start) / (gap_end - gap_start)
            # A parabola that is nought at both ends and one in the middle.
            arc = 4.0 * along * (1.0 - along)
            # Scaled by how much ROOM the gap has. Across a single airborne pose
            # there is nowhere for an arc to be an arc: the full bump lands on one
            # frame and reads as a spike, which showed up as four hip peaks per
            # cycle where a run has two.
            room = min(1.0, (gap_end - gap_start - 1) / 2.0)
            out[i] = low + (high - low) * along + bound * arc * room
    # Anything still unknown is outside the planted range: hold the nearest.
    for i, v in enumerate(out):
        if v is None:
            out[i] = out[known[0]] if i < known[0] else out[known[-1]]

    # --- And now make the two halves identical, because they are the same step.
    #
    # A cycle is two steps and the second is the first with the legs swapped, so the
    # hips must follow a curve that repeats every HALF cycle. Anything else is a limp.
    #
    # Up to here they only nearly did. The planted heights come out of measuring a rig
    # whose two legs are not mirror images - `L_Thigh`'s local X runs
    # (-0.007, -0.999, -0.044) against `R_Thigh`'s (+0.007, -0.992, +0.125) - so the
    # same pose measured on the left and the right differs by a little, and Blender's
    # default bezier handles then interpolate the difference unevenly. On the walk that
    # left the halves matching to 0.89; on the run, with three stance poses instead of
    # five to spread it over, 0.57 against a floor of 0.80.
    #
    # Averaging each pose with its half-cycle partner ENFORCES the invariant instead of
    # hoping the asset supports it. It is the right move rather than a patch, because
    # the difference being averaged away is measurement noise about a quantity that is
    # periodic by definition - and it costs nothing, since both values describe the
    # same moment of the same step.
    # COPIED rather than averaged, and the difference matters. Averaging the two
    # halves moved BOTH of them, which took the planted poses off the ground they had
    # just been measured onto - the right foot ended up sinking 4 cm in the sprint and
    # the contact checks started failing on the wrong frame.
    #
    # Copying the first half onto the second leaves every value that was measured
    # against a planted right foot exactly where planting put it, and hands the left
    # the same curve. That curve is CORRECT for the left too, because both legs read
    # the same pose table half a cycle apart, so the same pose deserves the same hip
    # height. Whatever the rig's own left-right difference is then shows up as a small
    # constant offset on one foot instead of as a limp, and a limp is far the worse of
    # the two to look at.
    half = POSES // 2
    for i in range(half):
        out[i + half] = out[i]
    out[POSES] = out[0]
    return out


def gait(rig, mesh, feet, ground: float, name: str, leg, span: int, reach: float,
         back: float, elbow_held: float, elbow_swing: float, lean: float,
         stance: int, bob: float, facing):
    """One cycle, authored by moving the FEET, with IK solving the legs.

    Nothing here poses a knee or a hip. The foot's path is stated - planted on the
    ground through stance, arcing forward through swing - along with how far the body
    rises and falls, and the leg is whatever reaches that. See `ik_gait` for why the
    other way round could not work: with the sole planted, hip height is not a free
    choice but the leg's vertical extent, so stating both over-determines it, and the
    thing it over-determined was the 30 cm hip drop reported as a disconnected hip.

    The ankle is still stated, because IK on the shin drives the ANKLE and leaves the
    foot's own orientation alone - which is right, since ankle flexion is relative to
    the shin anyway. Heel strike and toe-off are therefore still authored, and still
    read off the leg table's third column.
    """
    action = bpy.data.actions.new(name)
    rig.animation_data.action = action
    rest(rig)

    share = min(stance / POSES, 0.5)
    reach_of_leg = (
        rig.matrix_world @ rig.pose.bones["R_Foot"].head
        - rig.matrix_world @ rig.pose.bones["R_Thigh"].head
    ).length

    # No pole target: the knee's direction comes from the bind pose being BENT, which
    # `straighten_rig.py` bakes in. A pole was tried and it rotates the whole chain
    # about the hip-to-ankle axis, so putting the knees forward turned both feet 168
    # degrees away from the line of travel - the knee right and the foot backwards.
    flat = {side: ik_gait.rest_foot_pitch(rig, side, facing[0]) for side in "LR"}

    rigged = {side: ik_gait.add_leg_ik(rig, side) for side in "LR"}
    targets = {side: rigged[side][0] for side in "LR"}
    for side, (_, pole, hold) in rigged.items():
        turned = ik_gait.aim_the_pole(rig, side, pole, hold, facing[0], reach_of_leg)
        print(f"  {name}: {side} pole at {math.degrees(turned):+.0f} deg")

    off_by = 0.0
    for frame in range(1, span + 2):
        phase = ((frame - 1) % span) / span
        rest(rig)

        # The body: arms, spine, hands and the ankles. No thigh, no knee.
        for side, hand in (("L", 1.0), ("R", -1.0)):
            at = phase + (0.5 if hand > 0.0 else 0.0)
            swung = math.cos(2.0 * math.pi * (at - 0.5 - ARM_LAG))
            middle = (reach + back) / 2.0
            half = (reach - back) / 2.0
            swing(rig, f"{side}_Upperarm", middle + half * swung, REACHES_FORWARD)
            swing(rig, f"{side}_Forearm", elbow_held + elbow_swing * swung, FOLDS_THE_ELBOW)
            turn_further(rig, f"{side}_Upperarm", ARM_OUT * hand, (1.0, 0.0, 0.0))
            swing(rig, f"{side}_Hand", PALM_IN * hand, axis=(0.0, 0.0, 1.0))

        swing(rig, "Waist", lean * 0.4, LEANS_THE_TORSO_FORWARD)
        swing(rig, "Spine01", lean * 0.6, LEANS_THE_TORSO_FORWARD)

        # The body's height, on ROOT - which carries no skin weight at all, so moving
        # it cannot shear anything. A deform bone would: translating one away from its
        # parent drags blended vertices with it.
        rides = ik_gait.how_high_the_body_rides(share, phase, bob)
        root = rig.pose.bones.get("Root")
        if root is not None:
            axes = root.bone.matrix_local.to_3x3().inverted()
            root.location = axes @ mathutils.Vector(
                (0.0, 0.0, rides - ik_gait.KNEES_STAY_BENT)
            )
        bpy.context.view_layer.update()

        # And where the feet go. Keyed on the targets, so the bake can sample them.
        # Where each SOLE should be, and then the target solved so it lands there.
        wanted = ik_gait.where_the_soles_go(
            rig, facing, CONTACT, share, phase, reach_of_leg, ground
        )
        # Straight off the table, in degrees from HORIZONTAL - not added to the rest
        # pose's own tilt, because the foot's direction is now built from scratch rather
        # than nudged from wherever it was.
        pitches = {
            side: smoothly(
                [row[2] for row in leg], phase + (0.5 if side == "L" else 0.0)
            )
            for side in "LR"
        }
        for side, spot in wanted.items():
            targets[side].location = spot
        left = ik_gait.solve_the_target(
            rig, mesh, feet, targets, wanted, pitches,
            forward=facing[0], across=facing[1], toe_out=TOES_SIT_AT,
        )
        off_by = max(off_by, left)
        for side in "LR":
            targets[side].keyframe_insert("location", frame=frame)
            rig.pose.bones[f"{side}_Foot"].keyframe_insert(
                "rotation_quaternion", frame=frame
            )

        key(rig, frame, DRIVEN)

    # Solve, then turn the solution into plain keys and drop the helpers.
    first, last = 1, span + 1
    bpy.context.scene.frame_start, bpy.context.scene.frame_end = first, last
    ik_gait.bake_the_constraints(rig, first, last)
    ik_gait.drop_the_helpers(
        [part for parts in rigged.values() for part in parts[:2]]
    )

    baked = rig.animation_data.action

    turned = make_it_linear(baked)
    baked.name = name
    print(
        f"  {name}: {span + 1} frames, {turned} keys linear, legs solved by IK; "
        f"the worst the sole missed its path by was {off_by * 170.0:.2f} cm"
    )
    return baked


def which_vertices_are_feet(mesh):
    """Which vertices belong to each foot, by dominant weight. Computed once.

    Weights never change, so recomputing this per frame is waste.
    """
    groups = {g.index: g.name for g in mesh.vertex_groups}
    feet = {"L": [], "R": []}
    for vertex in mesh.data.vertices:
        heaviest = max(vertex.groups, key=lambda g: g.weight, default=None)
        if heaviest is None:
            continue
        name = groups.get(heaviest.group, "")
        for side in "LR":
            if name.startswith(side + "_") and ("Foot" in name or "Toe" in name):
                feet[side].append(vertex.index)
    return feet


def sole_of(rig, mesh, feet, side: str) -> float:
    """How low the DEFORMED foot actually reaches, in armature Z.

    Off the evaluated mesh, not off bone positions. Three bone points used to stand in
    for the sole and they sit 2.7 to 8.4 cm above it depending on how the foot is
    pitched - an error that swung 9.7 cm across a cycle and put the feet through the
    floor on 22 of 25 walk frames while every number involved looked self-consistent.
    The mesh is the thing that has to touch the floor, so the mesh is what to measure.
    """
    evaluated = mesh.evaluated_get(bpy.context.evaluated_depsgraph_get())
    baked = evaluated.to_mesh()
    try:
        matrix = evaluated.matrix_world
        return min((matrix @ baked.vertices[i].co).z for i in feet[side])
    finally:
        evaluated.to_mesh_clear()


def make_it_linear(action) -> int:
    """Forces every key in an action to LINEAR interpolation.

    Blender 5.x has no `action.fcurves` - actions are slots, layers, strips and
    channelbags. And it matters more than it sounds: a planted foot slid 13.60 mm across
    a cycle on Bezier keys against 0.92 mm on linear ones, because Bezier auto-handles
    overshoot between them. glTF cannot carry Bezier anyway, so the exporter resamples
    the overshoot straight into the clip.
    """
    from bpy_extras import anim_utils

    if not action.slots:
        return 0
    bag = anim_utils.action_ensure_channelbag_for_slot(action, action.slots[0])
    done = 0
    for curve in bag.fcurves:
        for point in curve.keyframe_points:
            point.interpolation = "LINEAR"
            done += 1
        curve.update()
    return done


def smoothly(column, at: float) -> float:
    """Samples a per-pose table anywhere in the cycle, smoothly and periodically.

    Catmull-Rom, so the authored poses are still passed through exactly while the
    frames between them are a curve this script controls rather than one Blender
    guesses.
    """
    n = len(column)
    x = at * n
    i = math.floor(x)
    f = x - i
    a, b, c, d = (column[(i + k) % n] for k in (-1, 0, 1, 2))
    return 0.5 * (
        2.0 * b
        + (-a + c) * f
        + (2.0 * a - 5.0 * b + 4.0 * c - d) * f * f
        + (-a + 3.0 * b - 3.0 * c + d) * f * f * f
    )


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


def mend_the_shipped_idle(rig, facing) -> None:
    """Fixes two things about the idle the model arrived with.

    # The hands came facing the wrong way, and it was not our doing

    Both forearms in `preset:biped:idle` are turned about 170 degrees from their rest
    pose, which lands the palms FORWARD with the fingers splayed outward — the
    reversed-hand read. It is the model's own authored pose: the original file and the
    game's export measure identical on every hand and forearm value, and a render of
    the untouched file shows it plainly. Nothing downstream did it.

    A roll of 180 degrees about a forearm's own length is exactly what swaps
    palm-forward for palm-back, and a bone runs along its local Y in Blender, so
    rolling about local Y is pronation and supination and cannot move the wrist.
    Rendered against 0, +90 and -90 before being chosen: the half-turn is the one that
    hangs the hand naturally, and it leaves about ten degrees of residual pronation,
    which is a believable authored amount.

    # And the legs were too close together

    The same idle holds the feet 12.7 cm apart for most of its length against 30.9 at
    rest, and in baggy trousers that reads as one leg rather than two. See
    `LEGS_SIT_AT` for the angle the legs are stood at, and why it is an angle asked
    for rather than a rotation added.

    # And the feet pointed outwards

    18.5 degrees of toe-out apiece against a human 7 to 10. Corrected here as well as
    in the gaits, because it is a fact about the asset and not about one clip.

    Both repairs ride in ONE pass over the clip. Two passes would mean two full
    reads, two rebuilds and the second one reading what the first wrote, which is the
    trap the last note below is about.

    # Why this bakes the clip rather than editing it

    Blender 5.x actions are LAYERED and have no `action.fcurves` — curves live under
    `action.layers[].strips[].channelbag(slot)`. Rather than reach into that, this
    steps the clip, applies the correction and keys a new action, which is the same
    shape as `gait()` and applies the fix to the POSE, which is the thing that was
    wrong.

    Two traps, both paid for once:

    * **An assigned action is re-evaluated by the depsgraph** and puts the keyed value
      straight back over a pose edit. So the pose is read, the action is dropped, and
      only then is anything changed. Four identical renders came out before that was
      understood.
    * **Read the whole clip before writing any of it.** Otherwise the action being
      read from is the action being written to.
    """
    if rig.animation_data is None or rig.animation_data.action is None:
        print("no idle to untwist")
        return
    idle = rig.animation_data.action
    was_called = idle.name
    low, high = (int(v) for v in idle.frame_range)

    held = []
    for frame in range(low, high + 1):
        bpy.context.scene.frame_set(frame)
        bpy.context.view_layer.update()
        held.append(
            {
                posed.name: (
                    posed.rotation_quaternion.copy(),
                    posed.location.copy(),
                    posed.scale.copy(),
                )
                for posed in rig.pose.bones
            }
        )

    rig.animation_data.action = None
    fixed = bpy.data.actions.new("untwisted")
    rig.animation_data.action = fixed
    half = mathutils.Quaternion(mathutils.Vector((0.0, 1.0, 0.0)), math.radians(UNTWIST))

    for offset, pose in enumerate(held):
        frame = low + offset
        for posed in rig.pose.bones:
            turn, where, scale = pose[posed.name]
            posed.rotation_mode = "QUATERNION"
            posed.rotation_quaternion = turn
            posed.location = where
            posed.scale = scale
        for name in FOREARMS:
            posed = rig.pose.bones.get(name)
            if posed is not None:
                posed.rotation_quaternion = posed.rotation_quaternion @ half
        forward, across = facing
        for side in "LR":
            bpy.context.view_layer.update()
            stand_the_leg_up(rig, side, LEGS_SIT_AT, across)
            bpy.context.view_layer.update()
            point_the_foot_along(rig, side, TOES_SIT_AT, forward, across)
        # Every bone, every frame — see `key`.
        for posed in rig.pose.bones:
            posed.keyframe_insert("rotation_quaternion", frame=frame)
            posed.keyframe_insert("location", frame=frame)

    bpy.data.actions.remove(idle)
    fixed.name = was_called
    fixed.use_fake_user = True
    print(
        f"mended '{fixed.name}': {len(held)} frames, forearms rolled {UNTWIST:+.0f}, "
        f"legs at {LEGS_SIT_AT:+.1f} deg, toes at {TOES_SIT_AT:+.1f}"
    )


def main() -> None:
    here = os.path.dirname(os.path.abspath(__file__))
    root = os.path.dirname(os.path.dirname(here))
    # The generator's own export, as it arrived.
    #
    # # Why not the straightened copy any more
    #
    # `straighten_rig.py` exists to serve HAND-AUTHORED gaits: it repairs a rest pose
    # so that poses written against it come out standing properly. The generator's own
    # clips were authored against the generator's own rest pose, so changing that rest
    # pose would make every quaternion in them mean something else. Where the clips
    # come from decides whether the rest pose may be touched, and the clips win.
    # The straightened copy when the gaits are AUTHORED here, the generator's own
    # export when they are not. Which one is right depends entirely on where the clips
    # come from: `straighten_rig.py` repairs a rest pose so that poses written against
    # it stand properly, and a clip authored against the OLD rest pose would be
    # corrupted by that same repair. So the generator's presets, if it ever ships any,
    # must be read against the untouched export; anything written here wants the clean
    # one.
    presets = [
        name
        for name in os.listdir(root)
        if name.lower().endswith(".glb")
        and "ranger" in name.lower()
        and name.lower() != "ranger_rig_idle.glb"
    ]
    source = (
        os.path.join(root, "Ranger_Rig_Idle.glb")
        if presets
        else os.path.join(here, "ranger_straight.glb")
    )
    print(f"reading {os.path.basename(source)}" + (f"; presets present: {presets}" if presets else "; no preset gaits, so the straightened rig"))
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

    # The generator's own walk and run, if they were exported alongside the idle.
    #
    # These are preset animations from the tool that rigged the character, which means
    # they are made by people who could see the result - and the whole of the authoring
    # below exists only because the first export happened to have the IDLE preset
    # selected and nobody asked what else was on offer.
    print("looking for the generator's own gaits:")
    given = gather_clips.take_the_clips(rig, source)
    if given:
        gather_clips.how_long_each_is(given)
    else:
        print("  none found; the gaits below are authored instead")

    # Which way the body faces, taken once from the rest pose. Everything that needs
    # to know is handed this, rather than working it out from feet that have moved.
    rest(rig)
    bpy.context.view_layer.update()
    facing = across_the_body(rig)
    print(f"forward is ({facing[0].x:+.3f}, {facing[0].y:+.3f}) at rest")

    idle = rig.animation_data.action if rig.animation_data else None
    if idle:
        print(f"keeping '{idle.name}'")
        idle.use_fake_user = True
        # Before the gaits: `gait` takes over the active action, and this needs the
        # idle to still be it.
        mend_the_shipped_idle(rig, facing)

    # Where the ground is, and how high each point of each sole sits above it in the
    # rest pose. Measured rather than assumed, because `plant` has to put a foot back
    # exactly where the model's own feet already rest.
    resting = where_each_sole_rests(rig)
    for side, (ground, above) in resting.items():
        print(f"{side} sole rests at z={ground:+.4f}, points above it {above}")

    # Which vertices are feet, and where the floor is. Both off the REST pose, once,
    # and off the MESH rather than off bone positions - three bone points used to stand
    # in for the sole and they sit 2.7 to 8.4 cm above it depending on how the foot is
    # pitched, an error that swung 9.7 cm across a cycle.
    feet = which_vertices_are_feet(body)
    rest(rig)
    bpy.context.view_layer.update()
    ground = min(sole_of(rig, body, feet, side) for side in "LR")
    print(f"the floor is at z={ground:+.5f}")

    # Twenty-four frames a cycle for a walk, sixteen for a run. Eight poses either
    # way: a run is not a walk with bigger numbers, but it does have the same four
    # poses per step.
    # Twenty-four frames a cycle for a walk and sixteen for both the jog and the
    # sprint. Eight poses each way: a run is not a walk with bigger numbers, but it
    # does have the same four poses per step.
    #
    # The frame counts are what set each clip's NATIVE speed, which is the one speed at
    # which its feet do not slide - `covers x fps / frames`. 1.935 over 24 is 1.93 m/s,
    # 2.282 over 16 is 3.42, and 3.50 over 14 is 6.00. `src/motion.rs` places each tier
    # at its own clip's native speed for exactly that reason.
    if "walk" in given:
        print(f"  keeping the walk that came with the model")
    else:
        gait(
            rig, body, feet, ground, "walk", WALK_LEG, 24, ARM_FORWARD, ARM_BACK,
            ELBOW_HELD, ELBOW_SWING, 0.0, WALK_STANCE, ik_gait.WALK_BOB, facing,
        ).use_fake_user = True
    if "run" in given:
        print(f"  keeping the run that came with the model")
    else:
        gait(
            rig, body, feet, ground, "run", RUN_LEG, 16, RUN_ARM_FORWARD, RUN_ARM_BACK,
            RUN_ELBOW_HELD, RUN_ELBOW_SWING, RUN_LEAN, RUN_STANCE, ik_gait.RUN_BOB, facing,
        ).use_fake_user = True
    if "sprint" in given:
        print(f"  keeping the sprint that came with the model")
    else:
        gait(
            rig, body, feet, ground, "sprint", SPRINT_LEG, 16, SPRINT_ARM_FORWARD,
            SPRINT_ARM_BACK, RUN_ELBOW_HELD, RUN_ELBOW_SWING, SPRINT_LEAN, SPRINT_STANCE,
            ik_gait.RUN_BOB, facing,
        ).use_fake_user = True

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
