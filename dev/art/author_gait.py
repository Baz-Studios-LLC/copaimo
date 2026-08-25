"""Authors a gait by moving the FEET, and lets IK work out the legs.

    dev/art/author_gait.sh          author the jog onto the built character

# Where this came from, and why it is back

This is `gait()` and its dependency closure, recovered from `dev/art/animate_ranger.py` as it
stood one commit before that file was deleted on 2026-08-24. Twenty-six functions and
twenty-four constants, lifted with their comments intact, because those comments are the record
of what the pipeline learned - every measured contact length, heel-strike pitch and arm angle in
here was paid for once already.

That file retired this pipeline in favour of retargeting delivered clips, and said why, and said
what to do if it did not work out:

    "The authored gait pipeline is gone from the build - `gait()` and its pose tables are still
     in this file and still work, but nothing calls them, because the decision was to start
     fresh from the delivered presets... If the deliveries turn out not to be salvageable, that
     is what there is to go back to."

They did not turn out to be salvageable. The delivered jog points a foot 68 degrees down while
it is still loaded, passes its legs within 3 mm, covers 2.6 times its own height in a stride and
runs a 25-frame cycle at 108 steps a minute. Ten separate corrections were written against it -
toe hinge, toe flatten, stance roll, floor lift, trunk lean, sideways lean, head level, arm gain,
pump shaping, leg spread - and each one fixed its own measurement while breaking something else,
because they were all reaching in to bend one joint on a frame nobody authored.

# Why authoring cannot have those faults

The docstring on `gait` puts it better than a summary would: "Nothing here poses a knee or a hip.
The foot's path is stated - planted on the ground through stance, arcing forward through swing -
and the leg is whatever reaches that."

So a planted foot is planted BY CONSTRUCTION. It cannot slide, because its path says it does not
move. The ball is the authored point and the ankle is derived from it, which is the contact-point
pivot `docs/animation.md` describes - the thing that ten corrections kept failing to reproduce
from the outside.

# What it needs from the rig

A knee with some bend in the bind. `gait` uses no pole target - "the knee's direction comes from
the bind pose being BENT" - and a dead-straight two-bone chain is singular. `build_character`
eases the knees 2 degrees for exactly this.
"""
import math
import os
import sys

import bpy
import mathutils

ART = os.path.dirname(os.path.abspath(__file__))
if ART not in sys.path:
    sys.path.insert(0, ART)

import foot_roll  # noqa: E402
import ik_gait  # noqa: E402

ROOT = os.path.dirname(os.path.dirname(ART))
MODEL = os.path.join(ROOT, "assets", "models", "person_ranger.glb")


def refuse(why):
    raise SystemExit(f"REFUSED: {why}")


FOLDS_THE_ELBOW = (0.0, -1.0, 0.0)


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


# Frames of run-up authored BEFORE frame 1, solved, and then thrown away.
#
# The IK solve is iterative and starts from wherever the previous frame left the rig, so
# frame 1 used to be the one frame with no predecessor at all - solved cold, from rest.
# What was done about it made things worse: frame `span+1` skipped its own derivation and
# restored frame 1's pre-bake targets, and then `close_the_loop` copied the baked
# `span+1` back onto frame 1 on the stated grounds that the last frame "is computed with
# everything in place". It no longer was. The copy laundered frame 1's cold solve back
# onto itself, and measured, that left frame 1's ball 9.5 cm behind the path the other
# five stance frames sit on, with NOTHING touching the floor - the lowest point of the
# shoe 0.44 cm up - at the most visible pose in the cycle, crossed every 667 ms.
#
# The run-up removes the special case instead of compensating for it. `phase` is
# `((frame - 1) % span) / span`, and Python's modulo is positive for negative operands,
# so frame 0 is phase (span-1)/span - exactly what frame `span` is. Frame 1 and frame
# `span+1` therefore get identically posed predecessors, the solver converges to the
# same answer at both, and the seam closes BY CONSTRUCTION rather than by copying.
#
# Three is enough for the constraint stack to settle; the keys below frame 1 are deleted
# after the bake, so nothing ships.
LEAD_IN = 3


# How far the arm extremes fall BEHIND the leg extremes, as a share of the cycle.
#
# Two independent sources put it at two to three frames of a twenty-four frame
# cycle, which is where 0.10 comes from. Arms hitting their extremes on the same
# frame as the feet is a named failure - it reads as mechanical and synchronised,
# like a wind-up toy - and the old clip had no lag at all.
ARM_LAG = 0.03


# --- OVERLAPPING ACTION: how far up the body each part LAGS the one below it.
#
# Reported as feeling "stiff" three separate times, and as wanting Genshin's liveliness.
# The named cause, from the animation principle itself: a character reads as "a stiff toy
# robot" when its parts move in sync, and reads as alive when "the torso twists, the head
# bobs, and other elements move out of sync".
#
# Everything in this gait was moving on ONE phase. The pelvis sway, the shoulder twist and
# the pelvis yaw were all the same cosine or its exact negation; the trunk lean was a
# CONSTANT that never varied across the cycle; the head was a constant counter-rotation
# with no motion of its own. Nothing lagged anything, which is overlapping action's
# opposite.
#
# So the spine is a lag chain: the pelvis leads, the chest follows a fraction of a cycle
# later, the head later still. A fraction of a CYCLE rather than a count of frames, so the
# three clips stay consistent with each other at different frame counts.
#
# Small numbers on purpose. Overlap is a delay, not a wobble - too much and the segments
# visibly fight each other instead of flowing.
CHEST_LAGS_THE_HIPS = 0.04


# How much the trunk PITCHES across the cycle, in degrees either side of `lean`.
#
# The lean was a fixed angle, so the torso was welded at one attitude for the whole clip
# while the legs worked underneath it. A runner's trunk pitches a little as each push comes
# through - it is small, and it is the difference between a torso that is carried and one
# that is doing something.
# 4.0, not the 2.0 a measured runner shows. This is a fantasy game about collecting
# monsters, not a gait study, and the reference class is stylised action animation - where
# extremes are pushed past life because that is what reads at speed and at distance. The
# same goes for the head bob and the arm swing below.
# 2.5, down from 4.0. It was raised to 4 as a deliberate exaggeration, and on its own that
# was fine - but the trunk pitches the whole spine, so at 4 degrees over a ~70 cm spine it
# was adding about 5 cm of head travel UNDERNEATH the head bob. Two knobs pushing the same
# pixel is how an "extreme" head happens when neither number looks extreme.
TRUNK_PITCHES = 2.5


# How far the forearms are tucked ACROSS the front of the body, in degrees.
#
# Reported as "when people jog their forearms are more in front of the body. Here the
# forearms are more outward", and the cause is that the elbow folded about
# FOLDS_THE_ELBOW - a FIXED armature axis. A fixed hinge plane does not follow where the
# upper arm points, so with the arm hanging ARM_HANGS_AT out to the side the fold threw
# the hand laterally instead of forward.
#
# Anatomically the elbow is a hinge and cannot carry the hand inward by itself; what does
# is shoulder INTERNAL ROTATION, turning the hinge plane across the body. So the fold
# axis is derived per frame from where the hand should head - forward, and this many
# degrees toward the midline. See swing_the_arm.
# How far the forearms angle toward the midline, PER GAIT. It was one number for all three,
# and 24 degrees is what put the arms inside the torso - measured on the sprint at 7.09 cm
# through the chest wall, and reported on both clips.
#
# 24 came from a report about the JOG: "when people jog their forearms are more in front of
# the body. Here the forearms are more outward". That is still true of the jog, and it is the
# gait with the smallest swing, so the tuck has room. The faster gaits swing the shoulder
# much further, and a tuck that is fine at 94 degrees of swing drives the hand through the
# ribs at 127 - the two multiply rather than add. So the tuck now shrinks as the swing grows.
# How much the arm swing DWELLS at its extremes rather than gliding sinusoidally through
# them - see the note in swing_the_arm. 1.0 is a plain cosine. Only the sprint pumps.
# Extra degrees of inward tuck at the top of the forward swing, and how far ahead of the
# apex that peak lands, as a fraction of a cycle. See the note in swing_the_arm.
# How far behind the forearm the wrist drags, in degrees, and by how much of a cycle. See
# the note where it is applied.
#
# 26 degrees is a lot on paper and reads as very little, because the term is zero-mean: it
# is the gap between the arm angle now and a fortieth of a cycle ago. At 26 it produced only
# 4.6 deg of actual wrist travel, which still read as a dead hand; 70 gives about 13, which
# is a runner wrist rather than a flick.  Nothing at the extremes either way.
# ZERO, on research rather than taste. A wrist drag was added here on the animation principle
# of follow-through - "looser or heavier elements lag behind the main mass" - and for a running
# arm that is the wrong principle. Sprint coaching is explicit the other way: "you need to
# choose a hand position that keeps the WRIST LOCKED... you can't let your hands flop around
# when sprinting because you lose the power of the armswing, and in sprinting the arms work
# like levers".
#
# So the hand is held, not dragged. Kept as a constant at nought rather than deleted, because
# the drag is right for a WALK and for an idle - it is the sprint that wants it locked - and
# whoever comes to make it per-gait should find the machinery still here.
HAND_LAGS_THE_ARM_BY = 0.0


HAND_LAGS_THE_ARM = 0.025


CROSS_LEADS = 0.045


# How much of the trunk lean is taken back out at the NECK, so the head stays over the
# shoulders instead of riding the lean out in front.
#
# The gait leaned `Waist` and `Spine01` and countered nothing, so the head inherited the
# whole lean: measured, it sat 7.7 cm ahead of the hip on every frame of the run. A head
# that leads the body is a classic off-balance read - reported as "something still seems
# off about the characters balance" - and runners do the opposite, keeping the head
# stacked and the gaze up while the trunk pitches.
#
# Not 1.0. A dead-level head on a leaning trunk reads as stiff, and the neck genuinely
# does travel with the spine a little. The idle already did exactly this - Spine01 at
# +1.0 of its breath against NeckTwist01 at -0.8 - so the gait was the odd one out.
HEAD_HOLDS_BACK = 0.65


# How the trunk lean is DIVIDED between the two spine joints, lowest first.
#
# It was 0.4 at the waist and 0.6 at the chest, which bends the character mostly at the
# ribs - reported as the lean being "in the wrong place". A runner does not curl forward;
# the whole body tilts from low down and the trunk stays a straight line, which is why the
# reference poses read as a plank leaning rather than a hunch. Putting most of it at the
# waist gives that, and leaves a little at the chest so the spine is not dead rigid.
#
# These must sum to 1.0 or the total lean stops matching RUN_LEAN, and LEANS_FORWARD_AT
# LEAST in verify_gait is measured on the delivered angle.
LEAN_AT_THE_WAIST = 0.8


LEAN_AT_THE_CHEST = 0.2


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
# How far out from straight down the arms are HELD, in the frontal plane, through every
# clip. The bind pose holds them at prepare_rig.ARMS_OUT - 45 degrees, an A-pose, chosen
# for the skinning - and nobody walks like that, so every pose brings them down to this.
# Composed as ONE adduction of (ARM_HANGS_AT - prepare_rig.ARMS_OUT), so the two numbers
# cannot drift apart. Replaces ARM_OUT, which added abduction on top of the old
# near-hanging bind and would read as chicken wings on this one.
ARM_HANGS_AT = 12.0


# 80, up from 10. Reported as the hands being "angled backwards", and what that turned out
# to be is the ROLL and not the bend: the wrist's flexion measures plus or minus 14 degrees
# from the bind, symmetric between the sides and 1.4 at idle, so it was never the culprit.
# Rendered across the run, the hand sat FLAT with the palm down and the fingers splayed
# forward - a table top rather than a hand - and it is the roll that fixes that.
#
# There is a limit to how good this gets: the rig has 41 bones and only `L_Hand` and `R_Hand`
# among them, no fingers at all, so the splayed fingers are baked into the mesh and cannot be
# curled by posing. The roll is chosen to point them along the direction of travel, which is
# the most a hand can be helped without finger bones.
# 10, which is where this started and where it has been put back.
#
# A long detour ended here. Asked to fix the hands being "angled backwards", I read ANGLE as
# ROTATION and spent six measurements and five values - 80, 45, 80, -70, +90 - rolling the
# palm about the forearm, which is pronation and not angle at all. Every one of them was
# reported as no better or worse, correctly. The roll was never the complaint.
#
# What was actually wrong with the ANGLE is the wrist: a drag term had just been added that
# bends the hand back and forth against the forearm, and sprint coaching says the opposite -
# "keeps the WRIST LOCKED... you cannot let your hands flop around when sprinting". That is
# zeroed in HAND_LAGS_THE_ARM_BY, and it is the fix that belonged to this report.
#
# The value here is left at its original 10 rather than at any of the five, because none of
# them was solving the reported problem and the smallest is the one that was never complained
# about on its own.
PALM_IN = 10.0


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
# From `prepare_rig.TOE_OUT` on the deleted rig, brought across with its reason, because it is
# the previous character's answer to the complaint that started this:
#
#   "Zero: the feet point straight from the leg. 7 degrees of toe-out is anatomically ordinary,
#    but on this character's oversized shoes it read as flare from every angle and the user
#    called it twice. Straight is the read that works on the sculpt."
#
# This character has the same oversized shoes and the same complaint - "do you see how the toes
# angle to the side? they shouldn't be doing that" - so it stays at zero here too.
TOES_SIT_AT = 0.0


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


# How a closing hand divides between the three joints of a finger, as degrees at full fist.
# The middle joint travels furthest, the one at the fingertip least, which is what a hand does
# and what stops a fist reading as three straight segments hinged in the middle.
FIST_BENDS = (78.0, 92.0, 55.0)


# The thumb opposes rather than curls, so it folds less and mostly at its base.
THUMB_BENDS = (42.0, 46.0, 30.0)


# What a hand does when its owner is not using it: not flat, not fisted. A running hand held
# open flat is the single thing that most made this character read as a mannequin.
RELAXED = 0.26


DIGITS = ("Thumb", "Index", "Middle", "Ring", "Pinky")


def curl(rig, bone: str, degrees: float):
    """Bends a bone about ITS OWN X axis, which for a finger is the flexion axis.

    Deliberately not `swing`, which turns a bone about an axis of the ARMATURE. That is right
    for a shoulder or a spine, where one world axis means the same thing for every bone
    involved, and useless for fingers: five digits point five ways, and a thumb points across
    the palm, so no single armature axis flexes them all. add_finger_bones.py aligns each
    bone's roll to the hand's own palm plane precisely so that local X is flexion everywhere,
    and this is the helper that spends that.
    """
    posed = rig.pose.bones.get(bone)
    if posed is None:
        return
    posed.rotation_mode = "QUATERNION"
    posed.rotation_quaternion = mathutils.Quaternion(
        (1.0, 0.0, 0.0), math.radians(degrees)
    )


# Measured once per side and kept, because measuring it costs a depsgraph update per sample
# and the answer is a property of the rig, not of the frame.
CLOSES = {}


def hand_closes(rig, side: str) -> float:
    if side not in CLOSES:
        CLOSES[side] = which_way_closes(rig, side)
    return CLOSES[side]


def which_way_closes(rig, side: str) -> float:
    """+1 or -1: the sign of a local-X curl that CLOSES this hand. Measured, not chosen.

    The flexion reference is built from a cross product, and a cross product is a pseudovector
    - mirror its inputs and the result mirrors and flips - so which sign closes a hand is not
    something to reason about at a keyboard and be right about. It is also exactly the class of
    fault this rig has shipped before: a sign correct on one side and inverted on the other,
    which here would open one fist while closing the other.

    So it is measured. Curl every finger one way, see whether the fingertips move toward the
    wrist or away from it, and keep the way that brings them in.
    """
    wrist = rig.matrix_world @ rig.pose.bones[f"{side}_Hand"].head
    tips = [f"{side}_{digit}3" for digit in DIGITS
            if f"{side}_{digit}3" in rig.pose.bones]
    if not tips:
        return 0.0
    reach = {}
    for sign in (1.0, -1.0):
        for digit in DIGITS:
            for number in range(1, 4):
                curl(rig, f"{side}_{digit}{number}", sign * 45.0)
        bpy.context.view_layer.update()
        reach[sign] = sum(
            ((rig.matrix_world @ rig.pose.bones[name].tail) - wrist).length
            for name in tips
        ) / len(tips)
    for digit in DIGITS:
        for number in range(1, 4):
            curl(rig, f"{side}_{digit}{number}", 0.0)
    bpy.context.view_layer.update()
    return 1.0 if reach[1.0] < reach[-1.0] else -1.0


def close_the_hand(rig, side: str, closure: float, closes: float):
    """Folds one hand, 0 for open and 1 for a fist."""
    for digit in DIGITS:
        bends = THUMB_BENDS if digit == "Thumb" else FIST_BENDS
        for number, bend in enumerate(bends, start=1):
            curl(rig, f"{side}_{digit}{number}", closes * closure * bend)


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
        # `Head` is here for the follow-through bob. It was set in the pose and never
        # keyed, so the term measured EXACTLY no effect - head travel 6.29 cm before and
        # after, which is what sent me looking at the maths instead of the plumbing. Same
        # class of trap as the Pelvis being a connected bone: the code reads correctly and
        # nothing reaches the file.
        if posed.name in ("Hip", "Root", "Head"):
            posed.keyframe_insert("location", frame=frame)


# Everything a stride touches. Twist bones are left alone: they exist to spread a
# limb's roll and have no business being posed by hand.
DRIVEN = (
    "Hip",
    "Waist",
    "Spine01",
    "Spine02",
    # the chest, which is what the old Spine02 became when a joint was inserted below it
    "Chest",
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


def swing_the_arm(rig, side: str, hand: float, phase: float, reach: float,
                  back: float, elbow_held: float, elbow_swing: float, tuck_in: float,
                  crosses_in: float, pumps: float, facing) -> None:
    """One arm, for one instant of a cycle: swing, elbow, hang and palm.

    In ONE place because it used to be written out twice - once in `pose_the_body` and
    once in `gait` - and the A-pose bind change was applied to one copy and missed the
    other, which is the whole case against copies.

    An arm opposes the leg on its own side, so it reads the leg phase plus a half turn,
    lagged by `ARM_LAG`. The swing and the elbow are authored about armature axes; the
    adduction then brings the swung arm down from the bind pose's A (prepare_rig.ARMS_OUT
    degrees) to a natural hang, composed on top so the fore-aft plane is kept.
    """
    at = phase + (0.5 if hand > 0.0 else 0.0)
    swung = math.cos(2.0 * math.pi * (at - 0.5 - ARM_LAG))
    # `pumps` shapes that cosine without moving its extremes or its phase. Below 1 it
    # flattens the peaks and steepens the middle, so the arm DWELLS at the ends of its swing
    # and snaps between them - which is what "pumping" is, and what a pure sinusoid can
    # never be, since a sinusoid spends its time evenly. The references call a sprint's arms
    # an "aggressive pump" as a thing distinct from a run's swing; this is that distinction.
    # Kept as an odd-symmetric power so the two halves stay mirror images and the cycle
    # still closes.
    if pumps != 1.0:
        swung = math.copysign(abs(swung) ** pumps, swung)
    middle = (reach + back) / 2.0
    half = (reach - back) / 2.0
    swings_to = middle + half * swung

    # # The arm's direction is STATED, not composed from two turns
    #
    # It used to `swing` the fore-aft angle and then `turn_further` the adduction that
    # brings the arm down from the A-pose bind. Two rotations about different axes do
    # not add - they couple - so the authored numbers stopped meaning degrees: measured,
    # 14 forward and 22 back came out as +2.7 and -23, a range compressed to 72% and
    # shifted 6 degrees back. The hands barely passed the shoulder.
    #
    # So the wanted direction is built from the two angles and the arm is turned onto it
    # by the shortest arc, which is exactly how the feet are aimed and for the same
    # reason. `swings_to` now means what it says.
    forward, across = facing
    down = mathutils.Vector((0.0, 0.0, -1.0))
    out = math.radians(ARM_HANGS_AT)
    ahead = math.radians(swings_to)
    wanted = (
        down * (math.cos(out) * math.cos(ahead))
        + across * (hand * math.sin(out) * math.cos(ahead))
        + forward * math.sin(ahead)
    ).normalized()
    posed = rig.pose.bones[f"{side}_Upperarm"]
    posed.rotation_mode = "QUATERNION"
    posed.rotation_quaternion = (1.0, 0.0, 0.0, 0.0)
    bpy.context.view_layer.update()
    now = (posed.matrix.to_3x3() @ mathutils.Vector((0.0, 1.0, 0.0))).normalized()
    turn = now.rotation_difference(wanted)
    frame = posed.matrix.to_3x3()
    posed.rotation_quaternion = posed.rotation_quaternion @ mathutils.Quaternion(
        (frame.inverted() @ turn.axis).normalized(), turn.angle
    )
    # The elbow LAST, about the TRUE armature axis, with the upper arm already final.
    # `swing` converts its axis through the forearm's REST basis, which only means
    # "lateral" while the parent is AT rest - with the upper arm swung and adducted,
    # a third of the fold turned into forearm ROLL, invisible: 62 degrees asked, 31
    # to 54 measured. Same bug class that twisted the feet; same cure - state the
    # axis in the frame the bone is actually in.
    swing(rig, f"{side}_Forearm", 0.0, FOLDS_THE_ELBOW)
    bpy.context.view_layer.update()
    # Most bend at the forward extreme, straightest at the back one - an elbow cannot
    # fold the other way, so an arm going behind straightens.
    # The hinge plane is DERIVED, not fixed. `upper` is where the upper arm actually
    # points after the swing above; `heads` is where the hand should go from there -
    # forward, and `tuck_in` degrees toward the midline (`across * hand` points away from
    # the body, so inward is its negation). Rotating `upper` about `upper x heads` carries
    # it toward `heads`, which is the fold wanted. A constant axis cannot do this because
    # it does not know where the upper arm ended up.
    upper = (posed.matrix.to_3x3() @ mathutils.Vector((0.0, 1.0, 0.0))).normalized()
    # The tuck GROWS as the arm comes forward, peaking just before the forward apex, so the
    # hand arcs in toward the sternum rather than tracking a plane parallel to the body all
    # the way round. Suggested from watching it, and the biomechanics agrees: arm swing is
    # "characterised primarily by arm flexion/extension in the sagittal plane", but "the arms
    # don't move in parallel paths, rather in a coordinated pattern that helps stabilise the
    # torso and regulate rotational motion around the body's longitudinal axis". So a cross
    # belongs, as a SECONDARY component - which is why this is added on top of `tuck_in`
    # rather than replacing it, and why the sprint gets less of it than the jog despite
    # swinging further.
    #
    # `CROSS_LEADS` puts the peak a frame or so ahead of the apex. That is the half of the
    # suggestion that makes it read as motion rather than as a wider pose: the hand is already
    # arcing inward while it is still travelling forward, and unwinds as it goes back.
    # `max` keeps it to the forward half only, so nothing pulls the trailing arm across.
    swings_in = max(
        0.0,
        math.cos(2.0 * math.pi * (at + CROSS_LEADS - 0.5 - ARM_LAG)),
    )
    tuck = math.radians(tuck_in + crosses_in * swings_in)
    heads = (forward * math.cos(tuck) - across * (hand * math.sin(tuck))).normalized()
    hinge = upper.cross(heads)
    # With the upper arm lying along `heads` there is no plane; fall back rather than
    # normalise a zero vector.
    hinge = (
        hinge.normalized() if hinge.length > 1e-6
        else mathutils.Vector(FOLDS_THE_ELBOW)
    )
    turn_further_absolutely(
        rig, f"{side}_Forearm", elbow_held + elbow_swing * swung, hinge
    )
    bpy.context.view_layer.update()
    # The palms face the thighs and stay there: pronation through an arm swing is only
    # about fourteen degrees, so a palm that visibly rolls is wrong.
    #
    # About the FOREARM's own long axis, not a fixed armature axis - which is the bug
    # documented a dozen lines above, made again on the very next bone. A fixed (0,0,1)
    # means "up" only while the forearm hangs; once the elbow folds 88 degrees the
    # forearm points forward and inward, and turning the hand about world up stops being
    # pronation and becomes a wrist TWIST. Reported as "the hand/wrist twisted", and it
    # appeared the moment the fold went from 62 to 88.
    along = (
        rig.pose.bones[f"{side}_Forearm"].matrix.to_3x3()
        @ mathutils.Vector((0.0, 1.0, 0.0))
    ).normalized()
    swing(rig, f"{side}_Hand", PALM_IN * hand, axis=along)

    # # And then the wrist DRAGS behind the forearm
    #
    # Reported: "the wrists dont move so the hands feel off". They did not move at all -
    # `swing` SETS a rotation, so the line above was the whole of the hand's pose and it is
    # a constant. The arm swung 94 degrees and the hand went along as one rigid piece,
    # which is precisely the "mechanical" that follow-through exists to fix.
    #
    # The principle is drag: "when raising an arm, you might lead with the upper arm, then
    # elbow, then wrist, then fingers", and "looser or heavier elements lag noticeably
    # further behind the main mass". The wrist is the last joint before the fingers, so it
    # is the loosest thing on the arm and should lag the most.
    #
    # Written as the DIFFERENCE between where the arm is and where it was, which is the same
    # form the chest and the head already use here. That shape matters: it is zero-mean, so
    # it changes WHEN the hand arrives without moving the palm's neutral pose - the palm
    # stays facing the thigh, which took several rounds to get right and should not be
    # disturbed to add a wrist. It also peaks where the arm is moving fastest and vanishes
    # at both extremes, which is what drag physically is.
    #
    # About the elbow's own `hinge`, so the hand flexes in the plane the arm swings in. Any
    # fixed axis would have become a wrist TWIST the moment the elbow folded - the exact bug
    # documented on the pronation above, and on the elbow above that. Third time on this arm.
    trails = HAND_LAGS_THE_ARM_BY * (
        math.cos(2.0 * math.pi * (at - HAND_LAGS_THE_ARM - 0.5 - ARM_LAG))
        - math.cos(2.0 * math.pi * (at - 0.5 - ARM_LAG))
    )
    turn_a_bone_further(rig, f"{side}_Hand", trails, hinge)

    # And the fingers settle into a loose hold rather than staying flat.
    #
    # This is the payoff for building the finger rig at all, and it is worth being clear about
    # why it belongs here rather than in a pose somewhere. A hand carried through a run with
    # its fingers straight and splayed is the single strongest mannequin cue a character can
    # have - it was reported as the hands looking wrong perhaps a dozen times, and every reply
    # I made adjusted the wrist or the palm roll, because those were the only channels that
    # existed. None of them could have worked. A relaxed curl is what was missing.
    close_the_hand(rig, side, RELAXED, hand_closes(rig, side))


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

    # NO half-cycle enforcement here any more, and no seam line.
    #
    # There used to be `half = POSES // 2; out[i + half] = out[i]` followed by
    # `out[POSES] = out[0]`, which averaged/copied one half of the cycle onto the other
    # and closed the seam. Both were correct when this took a NINE-ENTRY list - eight
    # poses plus a closing seam - and both became corrupting the day it started taking a
    # per-FRAME list instead: on a 24-frame cycle they clobber indices 4-7 with 0-3, and
    # `out[POSES] = out[0]` zeroes index 8, which is a real airborne frame in the middle
    # of the cycle rather than a seam.
    #
    # Measured, that is exactly what it did: the arc came out
    # [.., 0.0, 3.59, 3.59, 2.39, ..] over the first flight and
    # [.., 2.39, 3.59, 3.59, 2.39, ..] over the second, so the hips sat 2.39 cm lower on
    # frame 9 than on frame 21 - the same instant of the same step - and the run refused
    # "the hips do not repeat every half cycle" by 27%. Four other fixes were aimed at
    # that asymmetry before it was traced here; none of them could have worked.
    #
    # Nothing is needed in its place. The caller builds the airborne mask from a periodic
    # phase, so the gaps are symmetric by construction - which the untouched second and
    # third flights prove, both filling to an identical 2.39/3.59/3.59/2.39.
    return out


def gait(rig, mesh, feet, ground: float, name: str, leg, span: int, contact: float,
         swing_lift: float, swing_shape: float, lands_ahead: float,
         arm_forward: float, arm_back: float, elbow_held: float, elbow_swing: float,
         lean: float, share: float, sinks: float, leads: float, bound: float,
         absorbs: float, tuck_in: float, crosses_in: float, pumps: float,
         twist: float, pelvis, facing):
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

    # The stance share arrives as a fraction of the cycle, and is the TRUE duty factor.
    #
    # It was once `min(stance / POSES, 0.5)`, and that cap removed DOUBLE SUPPORT from the
    # walk entirely - a walk is a walk precisely because stance exceeds half the cycle.
    # The cap is long gone; the division has now gone too, because whole eighths could not
    # express the 0.333 a jog needs. See WALK_SHARE.
    reach_of_leg = (
        rig.matrix_world @ rig.pose.bones["R_Foot"].head
        - rig.matrix_world @ rig.pose.bones["R_Thigh"].head
    ).length

    # No pole target: the knee's direction comes from the bind pose being BENT, which
    # `straighten_rig.py` bakes in. A pole was tried and it rotates the whole chain
    # about the hip-to-ankle axis, so putting the knees forward turned both feet 168
    # degrees away from the line of travel - the knee right and the foot backwards.
    flat = {side: ik_gait.rest_foot_pitch(rig, side, facing[0]) for side in "LR"}

    rest_bend = {side: rest_ankle_bend(rig, side) for side in "LR"}
    rigged = {side: ik_gait.add_leg_ik(rig, side) for side in "LR"}
    targets = {side: rigged[side][0] for side in "LR"}
    for side, (_, pole, hold) in rigged.items():
        turned = ik_gait.aim_the_pole(rig, side, pole, hold, facing[0], reach_of_leg)
        print(f"  {name}: {side} pole at {math.degrees(turned):+.0f} deg")

    # Where the ankle, ball and sole sit with the foot at rest. The offsets between
    # them are what let an ankle be derived from a ball.
    rest(rig)
    bpy.context.view_layer.update()
    landmarks = {
        side: foot_roll.foot_landmarks(rig, mesh, feet, side) for side in "LR"
    }
    # Shared between the sides before anything reads them, so the mesh's left-right
    # difference stays in the mesh instead of becoming a limp. See
    # foot_roll.make_the_landmarks_mirrors.
    foot_roll.make_the_landmarks_mirrors(landmarks, facing, mathutils.Vector((0.0, 0.0, 1.0)))
    print(
        f"  {name}: the ball sits "
        f"{landmarks['R']['ball_above_sole'] * 170.0:.1f} cm above the sole, with "
        f"{landmarks['R']['toe_ahead_of_ball'] * 170.0:.1f} cm of shoe ahead of it and "
        f"{landmarks['R']['heel_behind_ball'] * 170.0:.1f} cm behind"
    )

    # The joint's own limit, read once from the module that owns it.
    foot_roll_cap = ik_gait.TOES_BEND_UP_TO

    # How far the ball sits AHEAD of the ankle in the bind, measured, both feet the
    # same. The ball's authored path is shifted by this so the ANKLE - which is where
    # the leg hangs from - sweeps symmetrically about the hip. See where_the_balls_go.
    ball_leads_ankle = sum(
        -landmarks[side]["ankle_from_ball"].dot(facing[0]) for side in "LR"
    ) / 2.0
    print(f"  {name}: the ball leads the ankle by {ball_leads_ankle * 170.0:.1f} cm, "
          f"so the path is shifted by that")

    def reach_ceiling(at):
        """The highest the hips may sit at this phase, from the planted legs.

        A planted foot pins the hip to a sphere about its ankle, so the hip can be
        sqrt(reach^2 - horizontal^2) above it and no higher. Where two feet are down
        the lower limit wins, or a leg is asked past its reach.
        """
        allow = []
        for side in "LR":
            own = (at + (0.5 if side == "L" else 0.0)) % 1.0
            if not ik_gait.the_foot_is_down(own, share):
                continue
            tilt_here = smoothly(leg, own)
            balls_here = foot_roll.where_the_balls_go(
                rig, facing, contact, share, at, reach_of_leg, ground, landmarks,
                swing_lift, swing_shape, lands_ahead, ball_leads_ankle
            )
            ankle_here, _ = foot_roll.ankle_for(
                rig, balls_here[side], tilt_here, TOES_SIT_AT, side, facing[0], facing[1],
                landmarks[side],
            )
            socket = rig.matrix_world @ rig.pose.bones[
                f"{side}_Thigh"].bone.matrix_local.translation
            flat = mathutils.Vector(
                (ankle_here.x - socket.x, ankle_here.y - socket.y, 0.0)
            ).length
            allow.append(
                ankle_here.z
                + math.sqrt(max(0.0, reach_of_leg * ik_gait.STANCE_LEG_EXTENDS
                                * reach_of_leg * ik_gait.STANCE_LEG_EXTENDS
                                - flat * flat))
                - socket.z
            )
        return min(allow) if allow else 0.0

    # # The hips ride a SMOOTH double bob that stays under that ceiling
    #
    # Clamping the hip to the ceiling frame by frame made its height a step function:
    # pinned at the cap through double support, level at bind height through
    # mid-stance, and then a 5.95 cm COLLAPSE in a single frame the moment the leading
    # foot landed. Reported as a bounce and a jitter, and that is exactly what it was.
    #
    # A walk's hips rise and fall twice a cycle, lowest at each double support, and
    # that is a cosine - so one is fitted between the deepest the ceiling allows and
    # the height he stands at, phased to bottom out at contact. The ceiling is still
    # honoured frame by frame, but it now only trims a curve that is already close.
    # # Where the hips ride: a smooth bob, PLUS a ballistic arc while airborne.
    #
    # Two things that had to be separated, because doing either alone fails.
    #
    # The BASE is a cosine fitted between the deepest the reach ceiling allows and bind
    # height. Following the ceiling per frame instead was tried, on the reasoning that a
    # planted foot genuinely pins the hip so the ceiling is not a choice there. Measured,
    # it was much worse and every guard caught it: the hips rose 5.7 cm ABOVE bind height
    # where the ceiling sat above it, moved 5.59 cm in a single frame, and the two halves
    # of the cycle disagreed by 92% because the ceiling is not symmetric - the two shoes
    # sit about 4 cm differently on their bones, and a fitted curve averages that away
    # while a tracked one inherits it. The ceiling also jumps at LANDING, not only where
    # nothing is down, which is what that reasoning missed.
    #
    # The ARC is added on top, over the airborne stretches only, and it is what makes a
    # run read as a run: AnimSchool's Peak pose is "character is fully in the air", and
    # before this the flight frames had the feet 1.3 cm off the floor - the right duration
    # at the wrong height, which reads as one continuous pass rather than bounding.
    # `fill_in_the_flight` draws it as a parabola that is zero at both ends, so it cannot
    # introduce a step at the boundaries, and the airborne windows of a symmetric gait are
    # themselves symmetric - so unlike tracking the ceiling, this cannot make it limp.
    # WALK_BOUND is 0, so a walk is untouched.
    # `absorbs` is an EXTRA dip below whatever the reach ceiling demands, and it exists
    # because the ceiling is what binds here - which is why raising `sinks` from 0.056 to
    # 0.075 once produced byte-identical scores, `hip_rises_cm` stuck at 9.52. `max` takes
    # the SHALLOWER of the ceiling and the sink cap, so `sinks` is a limit that was never
    # being reached and turning it up did nothing.
    #
    # The `max` stays, because it is a real safety: where the ceiling would demand a huge
    # drop the hip holds still and the leading leg simply reaches its limit, rather than
    # lurching. Absorption is subtracted AFTER it, so it is a deliberate amount rather than
    # whatever the ceiling happened to ask for.
    #
    # Going deeper than the ceiling is always safe - a lower hip has more reach, not less.
    # And it is what the Survival Kit's second drawing is: "THE DOWN", the knee absorbing
    # the landing. Ours barely dipped, which is most of why the row read as flat.
    deepest = max(
        min(reach_ceiling(step / span) for step in range(span)),
        -sinks,
    ) - absorbs
    airborne = [
        not any(
            ik_gait.the_foot_is_down(
                ((step / span) + (0.5 if side == "L" else 0.0)) % 1.0, share
            )
            for side in "LR"
        )
        for step in range(span)
    ]
    # Filled over TWO cycles and then halved, because the cycle wraps and a list does
    # not. `fill_in_the_flight` arcs between known indices, so the airborne stretch that
    # straddles the seam - the last frames of the cycle, whose landing is frame 1 of the
    # next - has no known index after it and falls through to "hold the nearest", i.e. no
    # arc at all. That gave one bound of the cycle its full arc and the other none, which
    # measured as the hips failing to repeat every half cycle by 20% where 12% passes.
    # Over two cycles that stretch is interior, so both bounds get the same arc - and it
    # is the FIRST copy that has both of them interior, not the second, whose own tail is
    # then the unfilled one.
    twice = [None if up else 0.0 for up in airborne] * 2
    arc = fill_in_the_flight(twice, bound)[:span]
    # A WALK bottoms at CONTACT - an inverted pendulum, lowest in double support. A RUN
    # bottoms on the DOWN pose, one eighth in: it is a spring, and AnimSchool's Down is
    # "the lowest, most compressed position". Duty factor says which kind this is.
    bottoms_at = 0.0 if share > 0.5 else 1.0 / POSES
    print(f"  {name}: the hips bob {deepest * 170.0:+.1f} cm, plus up to "
          f"{max(arc) * 170.0:.1f} cm of ballistic arc over "
          f"{sum(airborne)} airborne frames of {span}")
    off_by, clamped = 0.0, 0.0
    for frame in range(1 - LEAD_IN, span + 2):
        phase = ((frame - 1) % span) / span
        # WHICH FOOT IS CARRYING: +1 for the right, -1 for the left. R is planted over own
        # phase 0..share, so its mid-stance is at share/2 and the left's half a cycle
        # later, and a cosine about that peaks where the weight is. Set HERE, at the top,
        # because both the pelvis block and the root block below want it - defining it
        # beside one of them put it after its first use and raised UnboundLocalError.
        weight = math.cos(2.0 * math.pi * (phase - share / 2.0))
        rest(rig)

        # The SPINE first, then the arms.
        #
        # The arms are aimed in world terms, and the spine is their parent - so leaning
        # it afterwards carried them with it and every arm ended up 7 degrees behind
        # where it was asked to be, while its amplitude stayed exactly right. Aiming
        # after the parent has moved is the whole point of stating a direction: it holds
        # whatever the chain above it does.
        # The trunk PITCHES across the cycle rather than holding one angle - see
        # TRUNK_PITCHES. Twice per cycle, because both pushes drive it.
        pitching = TRUNK_PITCHES * math.cos(4.0 * math.pi * (phase - share / 2.0))
        swing(rig, "Waist", (lean + pitching) * LEAN_AT_THE_WAIST,
              LEANS_THE_TORSO_FORWARD)
        swing(rig, "Spine01", (lean + pitching) * LEAN_AT_THE_CHEST,
              LEANS_THE_TORSO_FORWARD)
        # And taken back out at the neck, split over both segments so no single joint
        # kinks. See HEAD_HOLDS_BACK.
        for segment in ("NeckTwist01", "NeckTwist02"):
            swing(rig, segment, -(lean + pitching) * HEAD_HOLDS_BACK * 0.5,
                  LEANS_THE_TORSO_FORWARD)
        # And the shoulders twist against the hips, BEFORE the arms are aimed - see
        # SPRINT_TWIST. LAGGED behind the pelvis by CHEST_LAGS_THE_HIPS, which is what
        # makes it overlapping action rather than two parts moving as one piece.
        if twist:
            # The CHEST. The shoulders hang off it, so it is the bone a shoulder twist belongs
            # on; `Spine02` is a mid-back joint now - see dev/art/add_spine.py.
            swing(rig, "Chest",
                  twist * math.cos(
                      2.0 * math.pi
                      * (phase - 0.5 - ARM_LAG - CHEST_LAGS_THE_HIPS)
                  ),
                  (0.0, 0.0, 1.0))
        # # The head is NOT translated, and that is the point
        #
        # There was a lift here that damped the hips' vertical, on the reasoning that a
        # runner's head is the most stabilised part of the body. The reasoning is right and
        # the method was not: `Head` has skin between it and `NeckTwist02`, so translating it
        # does not hold the head steady, it pulls it off the neck and stretches what is in
        # between. Measured, the joint gap went from 2.59 cm at rest to 12.61 cm on the run -
        # a 386% stretch - and it was reported exactly as it looks: "the body moves up and
        # down during the jog giving the player a long neck".
        #
        # It very likely explains the ORIGINAL "the head bob is extreme" too. The term was
        # already live then, in the wrong basis, so the head was being displaced arbitrarily
        # against the neck. What read as a bobbing head was a detaching one. A body that
        # rises and falls as one rigid piece reads as bounce, which is what a run should do.
        #
        # A vertical offset cannot be spent as rotation, so there is no clever version of
        # this. The head's calm is bought at the SOURCE instead, by not bobbing the hips so
        # far - `absorbs` and `bound` were pulled back to pay for it.
        #
        # This does cost the head its own follow-through, which was the head's share of the
        # overlapping action: `HEAD_LAGS_THE_CHEST` had no other consumer. The chest keeps
        # its lag against the hips, which is the larger half of that effect and is carried
        # by rotation, so it moves no skin.
        #
        # Zeroed explicitly rather than left alone, so a value from an earlier pass cannot
        # survive into the export.
        head = rig.pose.bones.get("Head")
        if head is not None:
            head.location = mathutils.Vector((0.0, 0.0, 0.0))

        # # The pelvis, on all three axes - see WALK_PELVIS
        #
        # `weight` is +1 when the RIGHT foot is carrying and -1 when the left is. R is
        # planted over own phase 0..share, so its mid-stance is at share/2 and the left's
        # half a cycle later; a cosine about that puts the peak where the weight is.
        _, obliquity, yaw = pelvis
        if obliquity:
            # Rolls about the direction of travel: the hip rides UP over the support leg
            # and DROPS on the swing side.
            #
            # NEGATED `weight`, and it was measured rather than reasoned. The first sign
            # put the hip high on the swing side on 16 of 16 single-support frames of the
            # walk - exactly inverted, which reads as the body being thrown away from the
            # leg holding it up. The sway, on the same `weight`, was right first time at
            # 16 of 16, so the two axes genuinely take opposite signs and guessing them
            # together would have hidden one behind the other.
            turn_further_absolutely(
                rig, "Pelvis", -obliquity * weight, facing[0]
            )
        if yaw:
            # Against the shoulders. `Spine02` twists on the arm-swing phase above, so the
            # pelvis takes the same phase with the opposite sign - otherwise the torso has
            # nothing to counter-rotate against and the twist reads as the whole body
            # turning.
            turn_further_absolutely(
                rig, "Pelvis",
                -yaw * math.cos(2.0 * math.pi * (phase - 0.5 - ARM_LAG)),
                (0.0, 0.0, 1.0),
            )

        # The body: arms, hands and the ankles. No thigh, no knee.
        for side, hand in (("L", 1.0), ("R", -1.0)):
            swing_the_arm(
                rig, side, hand, phase, arm_forward, arm_back, elbow_held,
                elbow_swing, tuck_in, crosses_in, pumps, facing,
            )

        # The body's height, on ROOT - which carries no skin weight at all, so moving
        # it cannot shear anything. A deform bone would: translating one away from its
        # parent drags blended vertices with it.
        # # The body rides as high as the PLANTED LEG CAN REACH, worked out, not tuned
        #
        # This was a hand-set drop plus a cosine bob, and the drop had to cover the
        # WORST case - the moment the feet are furthest apart - so it crouched the
        # character through the whole cycle. Measured at mid-stance, where a walk
        # vaults over a straight leg, the stance leg sat at 94.5% extension with its
        # knee folded 38 degrees. A knee that bent is 12 cm off the hip-to-ankle line,
        # and forward-folding puts all of it in front, which is why both knees were
        # ahead of the hips on nearly every frame and nothing looked balanced.
        #
        # A planted foot pins the hip to a sphere: the hip can be exactly
        # sqrt((reach)^2 - (how far the ankle is forward)^2) above the ankle and no
        # higher. So that is computed from the foot path itself, and the bob is then
        # not authored at all - it FALLS OUT, low when the feet are spread and high
        # when they pass, which is the pendular rise a walk actually has.
        # And where the feet go. Keyed on the targets, so the bake can sample them.
        # --- Where each BALL of the foot goes, and the ankle derived from it.
        #
        # The ball is what a foot pivots on. Authoring the ankle's path and letting the
        # ball fall where it may is what drove the toe through the floor: a bone turns
        # about its HEAD, so tilting the foot lifted the ball instead of the heel.
        #
        # The balls come FIRST, before the body's height, because the height is derived
        # from where the planted ankle actually is - and the ankle rises as the heel
        # lifts. An earlier version set the height first, assuming the ankle stayed at
        # its bind height; at mid-stance the heel is already 11 degrees up, which lifts
        # the ankle 2.7 cm and ate exactly the extension the height was buying.
        balls = foot_roll.where_the_balls_go(
            rig, facing, contact, share, phase, reach_of_leg, ground, landmarks,
            swing_lift, swing_shape, lands_ahead, ball_leads_ankle
        )

        # # The body rides as high as the PLANTED LEG CAN REACH, worked out, not tuned
        #
        # A planted foot pins the hip to a sphere about its ankle: the hip can be
        # exactly sqrt(reach^2 - horizontal^2) above that ankle and no higher. So the
        # height is computed from the ankle the foot path actually produces, and the bob
        # is not authored at all - it FALLS OUT, low when the feet are spread and high
        # when they pass, which is the pendular rise a walk has.
        #
        # Where two feet are down, the hip goes to the LOWER of the two limits, so
        # BOTH legs can still reach their own foot. Taking the higher one puts a leg
        # past its reach, where IK stops tracking and the foot stops meeting the
        # ground. And a hip at its lowest during double support is what a walk does
        # anyway - that is where the pendulum bottoms out.
        # `stance_reach`, and the arm parameters are `arm_forward`/`arm_back`.
        #
        # This line used to assign to `reach`, which was ALSO the name of the arm's
        # forward swing angle in the signature - so from the second frame onward every
        # arm was posed with 0.45 (a length in model units) instead of the authored 14,
        # 34 or 46 degrees. The arms still swung, because ARM_BACK was untouched, but
        # only BEHIND the body: the hands never came forward of the hips in any clip.
        #
        # Second shadowed name in this file in one day - the other collided with the
        # limp check's `rise` and produced a refusal that contradicted itself. Both were
        # invisible because the code kept running and gave a plausible answer. That is
        # what a long function buys you, and the names carry their subject now.
        stance_reach = reach_of_leg * ik_gait.STANCE_LEG_EXTENDS
        socket_z = {
            side: (rig.matrix_world
                   @ rig.pose.bones[f"{side}_Thigh"].bone.matrix_local.translation).z
            for side in "LR"
        }
        allowed = []
        for side in "LR":
            own = (phase + (0.5 if side == "L" else 0.0)) % 1.0
            if not ik_gait.the_foot_is_down(own, share):
                continue
            tilt_here = smoothly(leg, own)
            ankle_here, _ = foot_roll.ankle_for(
                rig, balls[side], tilt_here, TOES_SIT_AT, side, facing[0], facing[1],
                landmarks[side],
            )
            flat = (ankle_here - balls[side])
            across_ground = mathutils.Vector(
                (balls[side].x - (rig.matrix_world
                                  @ rig.pose.bones[f"{side}_Thigh"].bone
                                  .matrix_local.translation).x + flat.x,
                 balls[side].y - (rig.matrix_world
                                  @ rig.pose.bones[f"{side}_Thigh"].bone
                                  .matrix_local.translation).y + flat.y, 0.0)
            ).length
            up = math.sqrt(
                max(0.0, stance_reach * stance_reach - across_ground * across_ground)
            )
            allowed.append(ankle_here.z + up - socket_z[side])
        # The reach limit is a CEILING, not a target. Tracking it exactly made the hip
        # inherit every wobble in the ankle's height - measured, it spiked from 4 cm
        # ABOVE the bind pose to 12 cm below it in a single frame, a 16 cm lurch, and
        # a leg whose ankle happened to sit under the hip with its heel up let the hip
        # rise above where it stands at rest, which nothing does while walking.
        #
        # So: never above the bind height, and never further down than one bob's worth.
        # Where the ceiling would demand more drop than that, the hip stays put and the
        # leading leg simply reaches its limit - which is a straight leg at heel
        # strike, exactly what a walk wants there. The floor solve then brings its
        # target to the ground, so nothing floats.
        rides = (
            deepest * (1.0 + math.cos(4.0 * math.pi * (phase - bottoms_at))) / 2.0
            + arc[(frame - 1) % span]
        )
        root = rig.pose.bones.get("Root")
        if root is not None:
            axes = root.bone.matrix_local.to_3x3().inverted()
            # Lateral sway goes HERE and not on the Pelvis, which is connected to its
            # parent - Blender ignores `location` on a connected bone, so the first
            # attempt measured 0.00 cm of sway in every clip while looking correct in the
            # code. The root already carries the bob and the forward lead, and the ball
            # path is stated against each thigh's REST socket, so moving the root carries
            # the hips without dragging the footfalls after them.
            #
            # `across` is +Y, the model's LEFT, so a positive `weight` - the right foot
            # carrying - has to move the pelvis the other way.
            root.location = axes @ (
                facing[0] * leads
                + facing[1] * (-pelvis[0] * weight)
                # Clamped against `sinks + absorbs`, not `sinks` alone - otherwise the
                # clamp would immediately undo the absorption it was just given.
                + mathutils.Vector((0.0, 0.0, max(rides, -sinks - absorbs)))
            )


        bpy.context.view_layer.update()
        if os.environ.get("DIAG") and frame == 1:
            bpy.context.view_layer.update()
            for side in "LR":
                hip_at = rig.matrix_world @ rig.pose.bones[f"{side}_Thigh"].head
                gap = (mathutils.Vector(balls[side]) - hip_at).length
                print(
                    f"  DIAG {name} f1 {side}: ball=({balls[side].x:+.3f},"
                    f"{balls[side].y:+.3f},{balls[side].z:+.3f}) hip_z={hip_at.z:+.3f} "
                    f"hip_to_ball={gap:.3f} reach={reach_of_leg:.3f}"
                )

        tilts = {}
        for side in "LR":
            own = (phase + (0.5 if side == "L" else 0.0)) % 1.0
            tilt = smoothly(leg, own)
            # # Toes flat ONLY while planted; airborne toes follow the foot
            #
            # The flat-toe rule used to look at pitch alone, so through early swing
            # the toe segment stayed parallel to the ground while the heel was up -
            # "the back foot isn't using toes": a foot that visibly never pushed
            # through them. On the ground the toes stay flat under the rising heel
            # (the crease); off it the bend eases out over the first quarter of
            # swing, so the foot leaves pointed - the push-off flick - and is rigid
            # again long before it presents the heel.
            flat_bend = min(-tilt, foot_roll_cap) if tilt < 0.0 else 0.0
            if ik_gait.the_foot_is_down(own, share):
                bend = flat_bend
            else:
                through = (own - share) / max(1e-6, 1.0 - share)
                bend = flat_bend * max(0.0, 1.0 - through / 0.25)
            toes_at = tilt + bend
            spot, aim = foot_roll.ankle_for(
                rig, balls[side], tilt, TOES_SIT_AT, side, facing[0], facing[1],
                landmarks[side],
            )
            targets[side].location = spot
            tilts[side] = (tilt, toes_at)
        # And the whole shoe rested on the floor, whatever the tilt turned out to be.
        #
        # "Planted" means THE PATH HAS THIS BALL ON THE GROUND - own phase inside the
        # same `share` the ball path used - because rest_the_shoe_on_the_floor PULLS a
        # planted sole down to the floor and only pushes an airborne one up. This used
        # to ask who_is_planted, passing a 0..1 phase where it expected a 0..7 pose
        # step; every float is below any stance count, so BOTH feet read as planted on
        # EVERY frame of every clip, and the swing foot was dragged out of its arc to
        # the ground. That one mismatch was the whole family of straight-swing-leg
        # refusals - and it silently erased every arc change made while it stood.
        planted = {
            side: ik_gait.the_foot_is_down(
                (phase + (0.5 if side == "L" else 0.0)) % 1.0, share
            )
            for side in "LR"
        }
        clamped = max(
            clamped,
            foot_roll.rest_the_shoe_on_the_floor(
                rig, mesh, feet, targets, ground, planted, tilts, TOES_SIT_AT,
                facing[0], facing[1], ik_gait.point_the_foot,
            ),
        )
        for side in "LR":
            targets[side].keyframe_insert("location", frame=frame)
            rig.pose.bones[f"{side}_Foot"].keyframe_insert(
                "rotation_quaternion", frame=frame
            )
        key(rig, frame, DRIVEN)

    # Solve, then turn the solution into plain keys and drop the helpers.
    #
    # The run-up is baked WITH the clip - that is the whole point, it is what gives
    # frame 1 a settled predecessor - and cut off afterwards.
    first, last = 1, span + 1
    bpy.context.scene.frame_start = first - LEAD_IN
    bpy.context.scene.frame_end = last
    ik_gait.bake_the_constraints(rig, first - LEAD_IN, last)
    ik_gait.drop_the_helpers(
        [part for parts in rigged.values() for part in parts[:2]]
    )

    baked = rig.animation_data.action
    dropped = forget_the_frames_before(baked, first)
    bpy.context.scene.frame_start = first

    # The ankles LAST, and only the airborne ones - see keep_the_swing_ankle_honest. It has
    # to be here: before the bake the legs are IK constraints and the shank it measures
    # against does not exist yet.
    straightened = keep_the_swing_ankle_honest(
        rig, span, share, facing[1], rest_bend
    )

    closed = close_the_loop(baked, 1, span + 1)
    turned = make_it_linear(baked)
    baked.name = name
    print(
        f"  {name}: {span + 1} frames, {turned} keys linear, {closed} closed at the "
        f"seam, {dropped} run-up keys dropped, legs solved by IK, "
        f"{straightened} swing ankles pulled back inside "
        f"{ANKLE_BENDS_BETWEEN[0]:.0f}..{ANKLE_BENDS_BETWEEN[1]:.0f} deg; "
        f"the worst the shoe missed the floor by was {clamped * 170.0:.2f} cm"
    )
    return baked


# How far the ankle may bend away from its rest angle, in degrees: plantarflexed, then
# dorsiflexed. A person runs inside about -25 to +30 and a shoe folds visibly well before
# either end.
ANKLE_BENDS_BETWEEN = (-30.0, 30.0)


# Where the pull toward the band starts, as a fraction of the limit. 0.75 means an ankle is
# left alone up to three quarters of the way to the edge and then eased, so the correction
# has no step in it - see the note where it is used.
EASES_FROM = 0.75


def rest_ankle_bend(rig, side: str) -> float:
    """The ankle's angle in the BIND, read from rest geometry and nothing else.

    This is the datum every ankle measurement is a deviation from, and taking it from
    `pose.bones` was a bug that hid the whole swing correction on one side. `gait()` runs
    once per clip and the rig keeps the last clip's pose between calls, so the "rest" datum
    was actually whatever frame the previous gait left behind - different for each clip, and
    different for the LEFT leg than the RIGHT, because the two are half a cycle apart.

    Measured, that put the sprint's left datum 46 degrees out: the correction pass read the
    left ankle at +19.98 where it truly sat at +65.8, saw it comfortably inside the band, and
    skipped every frame of it. The right leg's datum happened to be close enough to work, so
    the corrections looked like they were running - 13 of them, all on the right foot.
    Reported as a compressed foot at frames 2, 24 and 25, which are exactly left-swing.

    `data.bones` cannot be posed, so this cannot happen again.
    """
    knee = rig.data.bones[f"{side}_Calf"].head_local
    ankle = rig.data.bones[f"{side}_Foot"].head_local
    toe = rig.data.bones[f"{side}_ToeBase"].head_local
    shank = ankle - knee
    foot = toe - ankle
    if shank.length < 1e-9 or foot.length < 1e-9:
        return 0.0
    return math.degrees(shank.normalized().angle(foot.normalized()))


def ankle_bend(rig, side: str) -> float:
    """How far this ankle is from its rest angle. Positive is toes-up (dorsiflexion)."""
    knee = rig.matrix_world @ rig.pose.bones[f"{side}_Calf"].head
    ankle = rig.matrix_world @ rig.pose.bones[f"{side}_Foot"].head
    toe = rig.matrix_world @ rig.pose.bones[f"{side}_ToeBase"].head
    shank = (ankle - knee)
    foot = (toe - ankle)
    if shank.length < 1e-9 or foot.length < 1e-9:
        return 0.0
    return math.degrees(shank.normalized().angle(foot.normalized()))


def turn_a_bone_further(rig, bone: str, degrees: float, axis) -> None:
    """ADDS a turn about an axis of the armature, keeping whatever the bone already has.

    `swing` SETS a rotation, which is right for authoring and wrong for a correction - it
    would throw away the pose being corrected.
    """
    posed = rig.pose.bones[bone]
    rest = posed.bone.matrix_local.to_3x3()
    local = (rest.inverted() @ mathutils.Vector(axis)).normalized()
    posed.rotation_mode = "QUATERNION"
    posed.rotation_quaternion = (
        mathutils.Quaternion(local, math.radians(degrees)) @ posed.rotation_quaternion
    )


def keep_the_swing_ankle_honest(rig, span: int, share: float, across, rest_bend) -> int:
    """Pulls an airborne ankle back inside `ANKLE_BENDS_BETWEEN`, frame by frame.

    # Why this exists, and why the last fix for it did not hold
    #
    # `RUN_LEG` authors the sole's pitch against the FLOOR. While the foot is planted that
    # is exactly right - the floor is what the sole rests on. In SWING it is the wrong
    # frame: the shank sweeps most of a right angle, and a sole held to a floor-relative
    # angle leaves the ANKLE JOINT to absorb the whole difference. Measured, +63.5 degrees
    # of dorsiflexion on the run against a human range of about -25..+30 - the toes hauled
    # up into the shin, reported twice now as a "compressed back foot".
    #
    # It was fixed once by subtracting, by hand, the dorsiflexion each swing frame happened
    # to be carrying. That is a correction and not a cure, and it lasted exactly until the
    # knee moved: raising RUN_SWING_LIFT from 0.24 to 0.34 folded the shank further and
    # brought the whole fault straight back, at +63.5 where it had been +65.
    #
    # So this measures instead. It runs AFTER the bake, because the legs are solved by IK
    # constraints and the shank simply does not exist as a pose until then - which is also
    # why it could never have been done in the authoring loop.
    #
    # An airborne foot touches nothing, so its orientation is free: correcting it cannot
    # lift a sole off the floor or slide a plant. Only airborne frames are touched.
    #
    # The sign and the scale are PROBED rather than derived. Rotating a foot about the
    # body's lateral axis changes the shank-relative angle by some gain that depends on
    # where the shank happens to be pointing, and this file has a long history of sign
    # errors reasoned out and measured wrong afterwards. One 2-degree test rotation gives
    # both, and costs nothing.
    """
    fixed = 0
    for frame in range(1, span + 2):
        bpy.context.scene.frame_set(frame)
        for side in "LR":
            own = ((frame - 1) / span + (0.5 if side == "L" else 0.0)) % 1.0
            if ik_gait.the_foot_is_down(own, share):
                continue
            bone = f"{side}_Foot"
            bpy.context.view_layer.update()
            off = ankle_bend(rig, side) - rest_bend[side]
            low, high = ANKLE_BENDS_BETWEEN
            # Eased, not clamped. A hard clamp corrects nothing up to the band edge and then
            # the full excess just past it, which is a STEP in the correction curve - and a
            # step in a foot's orientation between two frames is a visible snap, reported as
            # "the lead foot doesn't land in the same spot, 12 shifts forward from 11". So
            # the pull starts before the limit and eases in: nothing at `EASE_FROM` of the
            # way to the edge, full correction at the edge and beyond.
            edge = high if off > 0.0 else low
            along = abs(off) / max(1e-6, abs(edge))
            pull = min(1.0, (along - EASES_FROM) / (1.0 - EASES_FROM))
            excess = (off - edge) * pull if pull > 0.0 else 0.0
            if abs(excess) < 0.25:
                continue

            posed = rig.pose.bones[bone]
            held = posed.rotation_quaternion.copy()
            turn_a_bone_further(rig, bone, 2.0, across)
            bpy.context.view_layer.update()
            gain = (ankle_bend(rig, side) - rest_bend[side] - off) / 2.0
            posed.rotation_quaternion = held
            if abs(gain) < 1e-3:
                continue

            turn_a_bone_further(rig, bone, -excess / gain, across)
            bpy.context.view_layer.update()
            posed.keyframe_insert("rotation_quaternion", frame=frame)
            fixed += 1

    # Read it back from the KEYS, not from the pose that was just set. If a correction did
    # not survive being keyed, that is the only way to see it.
    # Read back from the KEYS, not from the pose just set, and say which foot is which.
    # An airborne foot left out of band would mean a correction that did not survive being
    # keyed; a PLANTED one out of band is a different fault entirely and cannot be fixed
    # here - rotating a planted foot lifts its sole off the floor, and the floor solve ran
    # during authoring, long before this. That has to come off the authored pose instead.
    left = []
    for frame in range(1, span + 2):
        bpy.context.scene.frame_set(frame)
        bpy.context.view_layer.update()
        for side in "LR":
            off = ankle_bend(rig, side) - rest_bend[side]
            own = ((frame - 1) / span + (0.5 if side == "L" else 0.0)) % 1.0
            if abs(off) > abs(ANKLE_BENDS_BETWEEN[1]) + 2.0:
                left.append((abs(off), frame, side, off,
                             ik_gait.the_foot_is_down(own, share)))
    left.sort(reverse=True)
    for _, frame, side, off, planted in left[:4]:
        print(f"    ankle still {off:+.1f} deg at f{frame}{side} "
              f"({'PLANTED - fix the authored pose' if planted else 'AIRBORNE - a bug here'})")
    return fixed


def which_vertices_are_feet(mesh):
    """Which vertices belong to each foot. Lives in `ik_gait` now, so there is one of it."""
    return ik_gait.which_vertices_are_feet(mesh)


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


def where_the_curves_live(action):
    """The channelbag holding an action's f-curves, or None if it has no slot yet.

    Blender 5 has no `action.fcurves` - actions are slots, layers, strips and
    channelbags, and reaching for the old attribute raises `AttributeError`, which reads
    as a broken script rather than a moved API. Three functions below needed the same
    four lines to get at the curves, so it lives here once.
    """
    from bpy_extras import anim_utils

    if not action.slots:
        return None
    return anim_utils.action_ensure_channelbag_for_slot(action, action.slots[0])


def forget_the_frames_before(action, first: int) -> int:
    """Deletes every key earlier than `first`. Used to cut the run-up off a clip.

    See LEAD_IN: the frames before 1 exist only so the IK solver reaches frame 1 warm,
    with a predecessor at the same phase frame `span` has. Once baked they have done
    their job, and shipping them would put the cycle's start three frames late and hand
    the game a clip whose first key is not its first pose.
    """
    bag = where_the_curves_live(action)
    if bag is None:
        return 0
    cut = 0
    for curve in bag.fcurves:
        doomed = [p for p in curve.keyframe_points if p.co[0] < first - 0.01]
        for point in reversed(doomed):
            curve.keyframe_points.remove(point)
            cut += 1
        curve.update()
    return cut


def close_the_loop(action, first: int, last: int) -> int:
    """Makes the first frame identical to the last, which is what a cycle means.

    They are the same instant, so any difference between them is a discontinuity the
    player crosses every single cycle. Measured, the left arm's shoulder key at frame 1
    sat well off the trend its neighbours were on - 8.74 cm of hand travel in one frame
    against a median of 3.89 - and it was seen as the arm jumping.

    The FIRST frame is the one replaced, not the last. Frame 1 is the loop's first
    iteration: nothing has been keyed yet, so the pose it settles into is not the pose
    every later frame settles into once the action has curves to evaluate against. The
    last frame is computed with everything in place and lands exactly on the trend, so
    it is the one to trust.
    """
    bag = where_the_curves_live(action)
    if bag is None:
        return 0
    copied = 0
    for curve in bag.fcurves:
        source = next(
            (p for p in curve.keyframe_points if abs(p.co[0] - last) < 0.01), None
        )
        target = next(
            (p for p in curve.keyframe_points if abs(p.co[0] - first) < 0.01), None
        )
        if source is None or target is None:
            continue
        if abs(target.co[1] - source.co[1]) > 1e-9:
            copied += 1
        target.co[1] = source.co[1]
        target.handle_left[1] = source.co[1]
        target.handle_right[1] = source.co[1]
    for curve in bag.fcurves:
        curve.update()
    return copied


def make_it_linear(action) -> int:
    """Forces every key in an action to LINEAR interpolation.

    Blender 5.x has no `action.fcurves` - actions are slots, layers, strips and
    channelbags. And it matters more than it sounds: a planted foot slid 13.60 mm across
    a cycle on Bezier keys against 0.92 mm on linear ones, because Bezier auto-handles
    overshoot between them. glTF cannot carry Bezier anyway, so the exporter resamples
    the overshoot straight into the clip.
    """
    bag = where_the_curves_live(action)
    if bag is None:
        return 0
    done = 0
    for curve in bag.fcurves:
        for point in curve.keyframe_points:
            point.interpolation = "LINEAR"
            done += 1
        curve.update()
    done += keep_quaternions_on_one_side(bag)
    return done


def keep_quaternions_on_one_side(bag) -> int:
    """Flips keyed quaternions so neighbours never take the long way round.

    q and -q are the SAME orientation, and nothing in the maths that produces a pose
    prefers one sign. Interpolation is not so relaxed: between two keys on opposite
    sides of the hypersphere a linear blend swings almost all the way round and back,
    which is seen as a limb snapping for a frame - "the left arm is jumping". Measured,
    the left hand moved 8.74 cm between two frames where its own median step was 3.93.

    Every pose here is sampled per frame and correct AT each key, so the fault lives
    purely between them, which is why nothing that measured poses ever caught it. The
    cure is to walk each bone's four channels together and negate any key that faces
    away from the one before it.
    """
    from collections import defaultdict

    quads = defaultdict(dict)
    for curve in bag.fcurves:
        if curve.data_path.endswith("rotation_quaternion"):
            quads[curve.data_path][curve.array_index] = curve

    flipped = 0
    for channels in quads.values():
        if len(channels) != 4:
            continue
        ordered = [channels[i] for i in range(4)]
        keys = min(len(c.keyframe_points) for c in ordered)
        for at in range(1, keys):
            before = [c.keyframe_points[at - 1].co[1] for c in ordered]
            now = [c.keyframe_points[at].co[1] for c in ordered]
            if sum(a * b for a, b in zip(before, now)) < 0.0:
                for c in ordered:
                    point = c.keyframe_points[at]
                    point.co[1] = -point.co[1]
                    point.handle_left[1] = -point.handle_left[1]
                    point.handle_right[1] = -point.handle_right[1]
                flipped += 1
    for channels in quads.values():
        for curve in channels.values():
            curve.update()
    return flipped


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

# # The jog's own numbers, recovered with the pipeline
#
# Every one of these was measured or tuned on the deleted character and each carries the note
# that earned it. They are the inputs to a jog rather than corrections applied to one, which is
# the whole difference between this file and everything that came before it today.

RUN_SPAN = 24


# One leg length (0.46 units), for the run and the sprint alike: planted-foot travel is
# about a leg at EVERY running speed (0.99 +/- 0.08 m, Weyand), and the speeds differ by
# duty and cadence instead.
#
# This was cut to 0.345 and then 0.30 and then put back, and the round trip is the useful
# part. The symptom was real: the leg landed at 100.000% extension, the ankle was asked
# for 28.41 cm of forward reach that is not available at any legal hip height, and the
# heel sat 1.17 cm off the floor. Cutting the sweep looked like the fix and was not -
# 3.4 cm off the stride, which should have dropped the foot 1.2 cm by geometry, moved it
# 0.13 cm, because the hip bob was DERIVED from the reach ceiling and an easier reach
# simply raised the hip and lifted the foot back up.
#
# The stride was never the problem. The bob's PHASE was: spring-phased, it was deepest at
# mid-stance where the ceiling has 7 cm of slack, and shallowest at contact where the
# ceiling demands -5.39 cm - so it sat 4.79 cm above its own ceiling exactly where the
# leg was most stretched. Bottoming the bob at contact and letting the run sink 5.44 cm
# instead of a walk's 4.08 made a full leg length fit on the first try, with the heel
# planted, the thigh reaching back, and 30% less foot slide than before any of this.
#
# The lesson is the one this file keeps paying for: when a tuning knob does a tenth of
# what the geometry says it should, stop turning it - something downstream is cancelling
# it. See TROUBLESHOOTING.md.
# 0.48. Only reachable now that toe-off actually plantarflexes - see RUN_LEG's push-off
# row. At -32 degrees of toe-off this asked for sweep the leg could not give and the
# clipping came back as foot slide.
RUN_CONTACT = 0.48


# 0.46 for the run, up from 0.38: the ball joint moved forward to the shoe's real
# flex point, so the ankle now sits further behind the ball and the trailing leg has
# to stretch further for the same sweep - measured, it saturated dead straight at
# touchdown. Landing more of the sweep ahead buys that reach back.
RUN_LANDS_AHEAD = 0.30


# 0.34, up from 0.24. The legs half of "both the arms and legs need more movement".
#
# Williams' pass position has the swing knee well up in front with the heel tucked, and ours
# sits low - which reads as shuffling rather than running. This was tried once before and
# judged a failure because foot SEPARATION dropped, 49.9 to 42.3 cm. That was the wrong
# test: a high knee with a folded heel is meant to bring the foot closer to the body, and
# measuring how far apart the feet get penalises exactly the pose being asked for. The knee
# drove to +40.6 degrees, which is the thing that was wanted.
RUN_SWING_LIFT = 0.34


RUN_SWING_SHAPE = 0.6


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
    8.0,      # 0     contact - HEEL STRIKE, toes up, exactly as WALK_LEG does
    #                       Was -16 (forefoot). Two reasons it changed. Reported: "lead
    #                       leg shouldnt be landing toes first". Measured: at jog pace
    #                       Breine found ZERO forefoot strikers in 52 runners at 3.2 m/s
    #                       and 81% rearfoot, and the foot-floor angle here was -14.6
    #                       deg against a -1.6 threshold.
    #                       It was not editable before: the leg landed at 100.000%
    #                       extension, so the -16 was buying reach the leg did not have,
    #                       and pitching the toes up would have lifted the foot clear of
    #                       the floor. With RUN_CONTACT at 0.30 the leg lands at 99.84%
    #                       and a heel strike is actually the CHEAPER pose - toes up
    #                       raises the ankle off the contact point, which needs less
    #                       vertical reach and so frees up horizontal.
    0.0,      # 12.5  mid-stance - sole flat, weight loaded  [thigh 2.0, knee 38.0]
    -26.0,    # 25    push-off - leg extended hard behind, driving back  [thigh -38.0, knee 4.0]
    #                       -26, after -32, -48 and -38. This is the reach knob, not just
    #                       a pose - but it turned out to be a nearly free one, which is why
    #                       it moved four times. Going -48 -> -38 -> -26 cost covers 2.495
    #                       -> 2.493 -> 2.502 m, so the sweep does not come from HOW FAR the
    #                       ankle plantarflexes, only from it doing so at all, and the last
    #                       step even improved the planted foot's rate consistency (spread
    #                       4.01 cm to 0.26). Reach was never the thing being traded.
    #                       `where_the_balls_go` authors the BALL's path and `ankle_for`
    #                       derives the ankle from it, so how far the ball may sit behind
    #                       the hip depends on how far the ankle is pulled forward of the
    #                       ball - which is exactly what plantarflexion does. Toes down 32
    #                       degrees barely tilts it, so the reach budget was being spent as
    #                       though the foot were a stub; the ankle-to-ball segment is 15.9
    #                       cm and at -48 most of it counts toward reach behind the body.
    #                       A sprinter's ankle goes past -50 at toe-off, so this is not even
    #                       an exaggeration - it is the pose that was missing.
    #
    #                       This is why the earlier conclusion was wrong. Pushing
    #                       RUN_CONTACT alone saturated the leg and paid the difference in
    #                       foot slide, which read as "his legs are as long as they get".
    #                       He is normally proportioned - 50.1% of height, hip socket to
    #                       floor - so a person with his legs sweeps about a metre. The
    #                       shortfall was in the solve, not the skeleton.
    #
    #                       -48 went too far the other way, and the give was in the ANKLE:
    #                       this angle is the sole against the FLOOR, and at toe-off the
    #                       shank is steep, so -48 of sole came out as -60.5 of ankle bend.
    #                       That folds the shoe on a foot that is still on the ground, and
    #                       a planted foot cannot be corrected afterwards - rotating one
    #                       lifts its sole, and the floor solve is long past. So the limit
    #                       here is what the ankle can carry, not what the reach would like:
    #                       see ANKLE_BENDS_BETWEEN and keep_the_swing_ankle_honest, which
    #                       handles the SWING half of the same fault where a correction is
    #                       free.
    -30.0,    # 37.5  early flight - knee folding up behind  [thigh -30.0, knee 70.0]
    -36.0,    # 50    peak fold - heel toward the buttock  [thigh -6.0, knee 100.0]
    -34.0,    # 62.5  knee drive - thigh coming through high  [thigh 22.0, knee 92.0]
    -8.0,     # 75    reaching - the furthest the leg gets in front
    #                       -8, not -18. The last two swing rows turn the foot 26 degrees
    #                       toes-up to set the heel strike, and that rotation lifts the TOE
    #                       faster than the ankle is descending - so the sole's lowest point
    #                       stopped falling and rose again one frame before contact: 12.70,
    #                       8.11, 4.24, then back up to 5.10, then land. Reported exactly as
    #                       it measures - "frames 11 and 12, the lead foot doesnt land in the
    #                       same spot, 12 shifts forward from 11". Starting the toes-up
    #                       earlier spreads the same rotation over more frames, so the
    #                       descent stays monotonic.  [thigh 38.0, knee 50.0]
    8.0,      # 87.5  descending - foot coming back under to land  [thigh 34.0, knee 26.0]
    # The swing four were -14, -12, -6, -2, and they are FLOOR-relative, which is the
    # trap. While the foot is planted the floor is the right frame - it is what the sole
    # is resting on. In swing it is not: the shank sweeps through most of a right angle,
    # so a sole held near horizontal leaves the ANKLE JOINT to absorb the whole
    # difference. Measured, that put the swing ankle at +58 to +65 degrees of
    # dorsiflexion where human running stays inside about -25 to +30, which is the toes
    # hauled up against the shin - reported as a "compressed back foot" on frames 9-12,
    # and invisible to every other instrument: the shoe loses 0.7 mm of length and yaws
    # 0.0 degrees, so it is neither crushed nor turned.
    #
    # With the knee folded 100 degrees the shank points steeply down and back, and a real
    # runner's foot points back WITH it - which floor-relative means a strongly negative
    # sole. These numbers are set from the measured joint angle, not guessed: each one is
    # its old value minus the dorsiflexion that frame was carrying.
    # These four were -6, 0, +4, +4, and measured that left the swing foot within a few
    # degrees of FLAT while it was 16 to 19 cm off the ground - reported as "back leg is
    # flat while above the ground". A foot leaves the ground pointed and stays pointed
    # through the fold; the toes only come up in the last quarter of swing, to present
    # the heel. Ending at +8 also means it arrives at the contact pose's +8 without a
    # step in the curve.
)


# A run swings its arms far harder, and this is not a style choice: arm-swing
# amplitude is measured to scale up with gait velocity, and the check in
# `verify_gait.py` refuses a clip whose hands cover less than a quarter of what its
# feet do - which the run failed outright at a walk's amplitude once its stride grew.
# 24, down from 34, and RUN_ELBOW_HELD up from 62 - reported as "lead arm is
# extended too far", and measured as a wrist 37.5 cm ahead of the shoulder at
# 78-93% of full shoulder-to-wrist extension, climbing to near shoulder height. A
# jogger's elbows stay pinned near 90 deg and the hands travel from the hip to the
# lower chest in a tight arc, peaking maybe 15-20 cm ahead. `elbow_held` is the base
# fold and `elbow_swing` rides on it, so 62 +/- 18 held the arm between 44 and 80
# degrees of fold - never near 90, hence reaching rather than pumping.
#
# The SHOULDER swing is unchanged at 34/-46. It was cut to 24/-32 while the stride
# was temporarily down at 0.30, because arms unchanged against a third less leg
# travel measured 1.361 of it where they had been 0.916 - but the stride came back to
# 0.46 once the hip bob stopped fighting it, and the cut then read as under-swinging
# at 0.46 of the legs. The fold was always the fix for "extended too far"; the swing
# was never the problem.
# 22/-58, not 34/-46: the same 80 degrees of swing, moved BACK.
#
# Reported twice as the lead arm being "too extended". The elbow fold was the first half
# of that and is fixed (62 -> 88, so it now holds near a jogger's 90). What was left is
# that the whole arm sits too far forward - measured, the wrist was 31 cm ahead of the
# shoulder through the entire stance, barely varying, which is a held-out arm rather than
# a pumping one. Shrinking the swing was tried and was wrong: it made the arms read as
# under-swinging (0.46 of the legs' travel) without moving the hand back much. Shifting
# the WINDOW keeps the amplitude and moves the mean, exactly as LANDS_AHEAD does for the
# legs.
# 38 and -55, up from 21 and -29. Reported against the run cycle in The Animator's
# Survival Kit: "note the arms in the second image, they swing more than our character",
# and then "both the arms and legs need more movement".
#
# Measured, the old pair gave 50.8 degrees of upper-arm swing and - the telling number - the
# hand never got BEHIND the shoulder at all, travelling +1.6 to +27.7 cm and staying in
# front for every frame of the cycle. Williams' run has the trailing arm swept clearly back
# with the elbow past the hip, and a contact sheet of ours next to it shows both hands in
# front in all seven poses, with the arms barely changing shape across the row. That is the
# single biggest reason it read weak: a run is driven by the arms as much as the legs, and
# ours were held still in front of the chest.
#
# 93 degrees of swing now, which is Williams' territory. Worth noting what this measurement
# caught on the way: the SPRINT was already right at 118.7 degrees and got behind the body
# properly. It only got there by accident, through multipliers of 1.9 and 2.7 on values
# that were far too small - so the two clips disagreed by more than a factor of two, and
# the one that was reported as wrong was the one with the honest numbers.
# 28 forward, -55 back. Reported: "the jog the arms seem to go up too high". Two things
# stack into hand height and 38 was the wrong one to have raised - the shoulder driving 38
# degrees forward AND the elbow folding to 106 at that same extreme put the hand up by the
# chest. Cutting the shoulder swing is the cheaper of the two, because the elbow fold is
# what makes the arm read as ANGLED rather than dangling.
#
# Deliberately asymmetric now: a jogger drives further back than forward, and the back half
# is the half that was reported as too small in the first place. So the -55 stays.
# 20 forward. Reported: "on the run the lead hand goes too high and too extended".
# Measured, the hand reached 4.4 cm ABOVE the shoulder and 43 cm in front of it - which is a
# reach, not a jog's swing. The Animator's Survival Kit and the run-cycle references agree
# that at the top of the cycle the arm is BENT and not extended.
RUN_ARM_FORWARD = 20.0


RUN_ARM_BACK = -55.0


RUN_CROSSES_IN = 16.0


RUN_PUMPS = 1.0


RUN_TUCK_IN = 12.0


# 80 held with 36 of swing, from 88 and 18. The shoulder swing was fixed first and it was
# not enough on its own: a contact sheet still read as arms held still, because with the
# elbow pinned near 88 degrees the HAND orbits close to the chest however far the upper arm
# travels. Measured, the shoulder swung 93.9 degrees while the hand moved only 48 cm.
#
# `elbow_swing` rides on `elbow_held` against the same `swung` phase as the shoulder, so a
# positive value folds the arm MORE in front and opens it BEHIND - which is exactly the
# Survival Kit shape: the front arm comes up tight and the back arm extends and sweeps past
# the hip. At 18 the back arm still sat at 70 degrees of fold, which is not an extension.
# 80 +/- 36 gives 44 in front and 116 behind, so the contrast is doing the work.
# 76 +/- 30, so 46 in front and 106 behind. 80 +/- 36 was refused - "the R elbow sits 0.014
# in FRONT of the shoulder-to-wrist line, so the arm folds backwards" - because past about
# 110 degrees of fold the wrist swings through and the joint reads hyperextended. 106 is the
# most that has ever passed, so the front extreme is pinned there and the whole of the extra
# range is spent OPENING the back arm, which is the half that was missing anyway.
# 80 +/- 28, so 52 behind and 108 in front. Reported: the lead hand is "too extended".
# Cutting RUN_ARM_FORWARD from 28 to 20 brought the hand DOWN (+4.4 to -1.3 cm against the
# shoulder) and left it 43 cm in front, unchanged - at a forward-and-up angle the shoulder
# governs the hand's HEIGHT and the elbow governs its REACH, so the wrong knob was turned
# first. 108 is as tight as the front can go: 116 was refused for hyperextension and ~110 is
# where that starts.
RUN_ELBOW_HELD = 80.0


RUN_ELBOW_SWING = 28.0


RUN_TWIST = 10.0


RUN_PELVIS = (0.006, 5.0, 7.0)


RUN_LEADS = 0.012


RUN_SHARE = 8.0 / 24.0


RUN_SINKS = 0.056


# 0.030 and 0.040, up from 0.016 and 0.022. `bound` is how far above the straight line the
# hips arc while airborne - the Survival Kit's "THE UP", the fifth drawing, where the body
# is highest and both feet are clear. With the DOWN deepened by `absorbs` this is the other
# half of the vertical contrast that a run lives on.
#
# These could not be raised before without the head bob going with them, and the head bob
# had just been reported as extreme. It rides at 0.43 of the hips now that its damping is
# in the right basis, so the two are no longer in conflict.
# 0.022 and 0.030, back from 0.030 and 0.040. They were raised on the strength of a head
# damping that turned out to stretch the neck, so the bob they bought was never actually
# affordable - with the head rigid on the neck again, hip travel IS head travel, and 18.1 cm
# of it is well past the 14.74 that was called extreme.
# 0.017 and 0.023. With the neck rigid, hip travel IS head travel - there is no damping
# left to hide behind - so the vertical has to be set to what the HEAD can carry. At absorbs
# 0.010 and bound 0.022 the hip rose 14.21 cm and the head 15.27, which is past the 14.74
# that was called extreme in the first place. This lands the head near 12, below where the
# complaint started, while keeping a real rise and fall.
RUN_BOUND = 0.017


# Nought for the run. Absorption is the more expensive half of the vertical - it deepens the
# landing without the flight arc's payoff in shape - so when the budget had to shrink it went
# first, and the run keeps its DOWN from the reach ceiling alone.
RUN_ABSORBS = 0.0


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
# 15, up from 9. Straight off the caption on the reference this was matched against:
# "here's the same thing with a bit more vitality - MORE LEAN - bigger arm swing". The arm
# swing was the half that got reported out loud, but Williams names lean first, and measured
# ours sat at 8.4 degrees where his figure is nearer 15. Lean is what makes a run read as
# driving forward rather than bouncing on the spot, and it costs nothing in reach because
# it pitches the trunk, not the legs.
RUN_LEAN = 15.0

# How close to a foot's OWN lowest point it still counts as standing on the ground, in cm,
# widened until enough of the cycle is stance to be a gait.
#
# The same ladder `audit_character.the_footfalls` walks, and for the same reason it gives: one
# window does not fit both feet on a rig whose sides were delivered mirrored imperfectly. Measured
# on the delivered jog, the left sole never comes closer than 0.44 cm to the floor while the right
# reaches -1.51 cm through it, so a window measured from a shared floor calls one foot planted for
# twice as long as the other.
#
# It starts TIGHT. At 3 cm the left foot's stance took in the frame after toe-off, whose sole is
# already 3.27 cm up and climbing, and hauling a leaving foot back down to the floor is asking the
# leg to grow: the solver saturated and drove the toe in to make up the difference. A window is a
# contact tolerance, and a couple of centimetres is what that means on a 170 cm figure - the
# frames either side of it belong to the ramp, not to the plant.
STANDS_WITHIN = (2.0, 3.0, 4.5, 6.0, 8.0)

# The least of a clip one foot may be down before the window is widened. Running duty factors sit
# near a third per foot and fall with speed; a fifth is where it stops being a gait.
A_FOOT_IS_DOWN_FOR = 0.20

# How many frames the correction ramps over as a foot leaves and returns to the ground.
#
# Without it the ankle steps by the whole correction between the last stance frame and the first
# swing frame - 3 cm in one frame on this clip - which is a visible tick in the shin. The ramp
# lands on frames the foot is in the AIR, where a centimetre of ankle height is nobody's business.
EASES_OVER = 2

# The most a planted sole may still be off the floor once the leg has been solved, in cm. Past
# this the leg could not reach and the plant is a lie, so it is refused rather than shipped.
REACHES_TO_WITHIN = 0.60

# Whether a planted foot is also held to a straight line at a steady speed, and not merely to the
# floor.
#
# A foot on the ground does exactly one thing: it stays where it is while the world goes past. In a
# clip whose root motion has been detrended out that reads as travelling BACKWARD at precisely the
# character's speed, dead straight, every frame of the plant - `the_footfalls` is built on the same
# statement, and calls it `speed = cadence x stride`.
#
# The delivered jog does not do that. Measured over its own stance intervals, the planted foot goes
# backward between 13.6 and 19.6 cm a frame - a 2.4-fold swing, 1.4 m/s of it - and drifts up to
# 2.5 cm a frame SIDEWAYS as well, the left foot one way and the right the other, so the feet splay
# apart while they are supposed to be standing still. Height alone cannot touch any of that. It is
# the thing reported as "he moves like he's ice skating", and it is in the clip rather than in the
# game.
LOCKS_THE_FOOT = True

# Whether the lock holds a planted ball to a LINE as well as to a speed, and whether that line is
# HIS FACING rather than wherever his feet happen to average out.
#
# Both on. "Considering we want AAA/Indie quality he should just run straight in the direction the
# player wants him to go", and that is the whole of it: the game drives the warden along the way he
# points, so a clip whose feet travel anywhere else slides sideways for as long as it plays. Both
# delivered clips do - the walk runs 7.07 degrees off its own hips and the jog 7.03, which agree
# closely enough to be one systematic crab in the source rather than noise in either. Left in, that
# is about 60 cm of sideways foot travel per clip.
#
# The two switches are one idea. Holding the ball to a SPEED but not to a LINE leaves the heading
# free, so he keeps the crab; naming the line without holding it does nothing at all.
#
# The cost was weighed and is small. The correction is nought at mid-stance, where each plant is
# anchored, and grows to about 5 cm at the ends of a stance - so the width he actually stands at
# does not move, which is what "his legs are way further apart than they should be" was about.
HOLDS_SIDEWAYS = True
RUNS_ALONG_HIS_FACING = True

# How near his facing the held plants have to end up before the clip counts as running straight,
# in degrees. A quarter of a degree over a 2.4 m cycle is a centimetre of sideways travel, which is
# below anything an eye will find.
RUNS_STRAIGHT_WITHIN = 0.25

# The most a planted foot may be shifted sideways or along to stop it sliding, in cm.
#
# Twelve, raised from eight, and the reason is a decision rather than a measurement: "considering we
# want AAA/Indie quality he should just run straight in the direction the player wants him to go."
# Straightening a 5.36 degree crab over an 80 cm stance costs about 7.5 cm on its own, and stacked
# with the along-travel correction the worst frame asks 10.7. Eight was chosen when the lock was
# only ever going to fix speed, and holding a heading was explicitly not being attempted.
#
# It is still a real bound, and it still refuses. What it now allows is a foot moved by about
# two-thirds of its own length at the very ends of a stance, where the foot is unweighting anyway;
# at mid-stance, where each plant is anchored, the correction is nought and the width he stands at
# is untouched.
A_PLANT_MAY_SHIFT = 12.0

# How close the solved chain must end up to what it was asked for, in cm, before the ask is called
# unreachable. A leg that cannot get there has not planted the foot; it has leaned on it.
CHAIN_REACHES_TO_WITHIN = 0.50

# The most the body may be nudged to bring a plant within a leg's reach, in cm.
#
# The standard answer to a footplant a leg cannot quite make, and the reason it is available here
# is that a RUN has no double support: measured on this clip the two feet's stance frames do not
# share a single frame, so at any moment at most one foot is on the ground and the body can be
# moved to suit it without dragging the other. In a walk that would not hold and this would have to
# be weighted between the two.
#
# Four centimetres, because it is a repair and not a performance. Past that the clip is asking for
# a different step rather than the same step without slide in it.
HIPS_MAY_SHIFT = 4.0
CARRIES_THE_BODY = True

# How many times the body may be carried before the plants are taken as good as they will get.
HIP_TRIES = 4

# How many passes the ball-and-sole fixed point is given, and how close it has to land, in model
# units. Thirty is far more than the three or four a well-conditioned pass needs; it costs nothing
# on the frames that converge at once and buys the awkward ones room.
TRIES = 30
CLOSE_ENOUGH = 0.00002

# How many passes running the chain may fail to follow before it is called stuck rather than slow.
STALLS_BEFORE_STUCK = 3

# How much of a step the ankle must actually take before the step counts as followed.
#
# The guard against asking a straight leg to grow. Below this the chain has stopped tracking and
# every further centimetre of ask goes into PITCHING it instead, which drives the toe through the
# floor and reads as the stab this whole pass exists to remove. Half is generous - a chain with any
# reach left follows within a few percent - and it only has to separate "tracking" from "stuck".
FOLLOWS_BY = 0.5


def play_it(rig, clip):
    """Puts a clip on the rig, slot and all."""
    rig.animation_data.action = clip
    slots = getattr(clip, "slots", None)
    if slots:
        rig.animation_data.action_slot = slots[0]


def where_the_feet_are_down(rig, mesh, feet, clip, scene):
    """Which frames each foot stands on, measured off the deformed sole.

    The criterion is the sole's height against that foot's OWN lowest point, which is the test
    `the_footfalls` uses to decide what to measure, and is deliberately not "the foot is moving
    backward" - that is the thing the plant exists to fix, so deciding stance by it would make the
    fix agree with itself.
    """
    first, last = (int(round(v)) for v in clip.frame_range)
    soles = {"L": {}, "R": {}}
    for frame in range(first, last + 1):
        scene.frame_set(frame)
        bpy.context.view_layer.update()
        for side in "LR":
            soles[side][frame] = ik_gait.lowest_sole(rig, mesh, feet, side)

    # A looping clip's last frame repeats its first, so it is not a frame of its own: counted as
    # one, it inflates the stance duty and gives the seam two answers to the same question.
    loops = all(abs(soles[side][first] - soles[side][last]) < 1e-5 for side in "LR")
    ends = last - 1 if loops else last
    frames = ends - first + 1

    down, window = {}, STANDS_WITHIN[-1]
    for wide in STANDS_WITHIN:
        window = wide
        down = {
            side: [f for f in range(first, ends + 1)
                   if soles[side][f] <= min(soles[side].values()) + wide / 170.0]
            for side in "LR"
        }
        if all(len(down[side]) >= A_FOOT_IS_DOWN_FOR * frames for side in "LR"):
            break
    return soles, {side: without_gaps(down[side], first, ends, loops) for side in down}, \
        window, loops


def without_gaps(down, first, ends, loops):
    """Closes a one-frame hole in a contact, because a foot does not lift for a frame and land.

    A foot on the ground for four frames whose middle one happens to sit a millimetre outside the
    window is on the ground for four frames. Left as a hole it splits one plant into two, and the
    frame in the middle - a frame the foot is genuinely standing on - gets treated as flight and
    ramped, which is to say allowed to slide. Measured on the jog's right foot, whose contact runs
    24, 1, 2, 3 across the loop seam with frame 1 a tenth of a millimetre proud: it slid 6.00 cm
    sideways in one frame while the frames either side of it held to 0.00.

    Cyclically, since that is exactly where it bit - the seam is where a contact is most likely to
    look like two.
    """
    span = ends - first + 1
    was = set(down)
    filled = set(down)
    for frame in range(first, ends + 1):
        if frame in was:
            continue
        before = first + (frame - 1 - first) % span if loops else frame - 1
        after = first + (frame + 1 - first) % span if loops else frame + 1
        if before in was and after in was:
            filled.add(frame)
    return sorted(filled)


def a_run_of(frames, first=None, cycle=None):
    """The contiguous stretches in a list of frame numbers, joined across the seam if it cycles.

    A looping clip has no first or last plant, only plants - the right foot's stance on the jog
    runs from frame 24 through the seam to frame 2, and read as a line that is two separate
    contacts a frame apart. Ramping out of one and into the other put a different correction on
    the two frames that hold the SAME pose, which is a 1.05 cm step in the ankle at the exact
    moment the clip repeats.
    """
    runs = []
    for frame in sorted(frames):
        if runs and frame == runs[-1][-1] + 1:
            runs[-1].append(frame)
        else:
            runs.append([frame])
    if cycle and len(runs) > 1 and runs[0][0] == first and runs[-1][-1] == first + cycle - 1:
        runs = [runs[-1] + runs[0]] + runs[1:-1]
    return runs


def how_fast_the_ground_goes_by(spots, down, along=None):
    """Which way a planted contact travels and how far it goes each frame, off the BALLS.

    No axis is assumed and no facing is consulted: the direction is the sum of what the planted
    feet actually do, which is by definition the way the character goes. `the_footfalls` takes the
    travel direction the same way and for the same reason.

    The distance is the MEDIAN of the per-frame steps, not the mean. A stance's first and last
    intervals are half in the air and pull a mean down hard - measured on the jog, 8.28 cm against
    a 16.71 median over the same stance - and the median is what the middle of the plant, where the
    foot is unambiguously down, actually does.
    """
    steps = []
    for side in "LR":
        for frame in down[side]:
            if frame + 1 not in down[side]:
                continue
            went = spots[side][frame + 1] - spots[side][frame]
            went.z = 0.0
            steps.append(went)
    travel, each = ik_gait.the_line_of_travel(steps, along)
    if travel is None:
        refuse("no plant lasts two frames that move, so there is no line of travel to hold to")
    return travel, each


def where_a_planted_foot_belongs(run, spots, travel, each):
    """Where every frame of one plant puts the foot, if the foot is not to slide.

    Anchored on the MIDDLE of the plant rather than its start, so the correction is shared between
    the two ends instead of accumulating across the whole contact. Counted by position in the run
    and not by frame number, because a plant that crosses a looping clip's seam runs 24, 1, 2 and
    frame arithmetic on that is nonsense.
    """
    anchor = len(run) // 2
    held = spots[run[anchor]]
    return {frame: held + travel * (each * (step - anchor))
            for step, frame in enumerate(run)}


def read_the_feet(rig, scene, first, last):
    """Where the clip's own ankles and balls are, frame by frame.

    Read with any solver switched off, because the answer wanted here is what the CLIP does. Asking
    the rig while the IK is live returns where the solver last put things, which is the quantity
    this is trying to compute.
    """
    ankles, balls = {"L": {}, "R": {}}, {"L": {}, "R": {}}
    for frame in range(first, last + 1):
        scene.frame_set(frame)
        bpy.context.view_layer.update()
        for side in "LR":
            ankles[side][frame] = (
                rig.matrix_world @ rig.pose.bones[f"{side}_Foot"].head).copy()
            balls[side][frame] = (
                rig.matrix_world @ rig.pose.bones[f"{side}_ToeBase"].head).copy()
    return ankles, balls


def smoothly_between(known, first, ends, loops):
    """Fills the frames nobody measured by running a line between the ones somebody did.

    Cyclically when the clip loops, so the curve that comes out has no seam in it either - the
    frames after the last plant lead round into the frames before the first.
    """
    span = ends - first + 1
    at = sorted(known)
    if not at:
        return {f: mathutils.Vector((0.0, 0.0, 0.0)) for f in range(first, ends + 1)}
    out = {}
    for frame in range(first, ends + 1):
        if frame in known:
            out[frame] = known[frame].copy()
            continue
        before = [a for a in at if a <= frame]
        after = [a for a in at if a >= frame]
        if loops:
            low = before[-1] if before else at[-1] - span
            high = after[0] if after else at[0] + span
        else:
            low = before[-1] if before else at[0]
            high = after[0] if after else at[-1]
        if high == low:
            out[frame] = known[low % span + first if loops else low].copy()
            continue
        share = (frame - low) / (high - low)
        a = known[first + (low - first) % span] if loops else known[low]
        b = known[first + (high - first) % span] if loops else known[high]
        out[frame] = a.lerp(b, share)
    return out


def carry_the_body(rig, clip, shifts, scene, first, last, loops):
    """Moves the whole figure by a little, frame by frame, without touching a single joint.

    Written onto whichever bone the clip already travels on, in that bone's own rest frame - world
    up is not a bone's up, and `stand_on_the_floor` learned the same lesson lifting clips onto the
    floor. Every frame is read before any frame is written, because writing a key changes what the
    frames around it evaluate to, and reading through a curve that is being edited underneath gives
    each frame a different rig than the one it was measured on.
    """
    slot = rig.animation_data.action_slot if rig.animation_data else None
    for carrier in ("Root", "Hip", "Pelvis"):
        if carrier not in rig.pose.bones:
            continue
        rest = (rig.matrix_world @ rig.pose.bones[carrier].bone.matrix_local).to_3x3()
        turn = rest.inverted()
        was = {}
        for frame in range(first, last + 1):
            scene.frame_set(frame)
            bpy.context.view_layer.update()
            was[frame] = rig.pose.bones[carrier].location.copy()
        for frame in range(first, last + 1):
            at = first if (loops and frame == last) else frame
            scene.frame_set(frame)
            rig.pose.bones[carrier].location = was[frame] + (turn @ shifts[at])
            rig.pose.bones[carrier].keyframe_insert("location", frame=frame)
        return carrier
    refuse(f"{clip.name} travels on no bone this knows, so the body cannot be carried")


def how_hard_the_solver_pulls(down, lift, first, last, loops):
    """How far to move each ankle, and how much of that to actually apply, frame by frame.

    # Why the solver is switched off rather than fed the clip's own ankle

    The first version of this kept the IK live across the whole clip and simply aimed it at the
    ankle the animator had already chosen on the frames that needed nothing. That looks like a
    no-op and is not one. A two-bone chain reaching a point has a whole CIRCLE of solutions - the
    same degeneracy `add_leg_ik` describes - so hitting the same ankle does not mean reproducing
    the same LEG, and the pole picks its own answer. The foot hangs off the end of that, so its
    world tilt changes with it. Measured: the right foot's swing peak drifted from 16.63 cm to
    20.19 cm on frames whose correction was zero.

    So the solver is keyed OFF wherever nothing needs correcting, and the animator's leg is what
    plays there - which is the whole promise this pass is making. Where it is on, influence ramps
    it in and out over `EASES_OVER` frames of flight, which is also what does the smoothing: an
    influence of w lands the ankle w of the way from where the clip put it to where the floor
    wants it, so one curve serves as both the blend and the taper.
    """
    nowhere = mathutils.Vector((0.0, 0.0, 0.0))
    offset = {f: nowhere.copy() for f in range(first, last + 1)}
    strength = {f: 0.0 for f in range(first, last + 1)}
    cycle = (last - first) if loops else (last - first + 1)

    def inside(frame):
        """Where a frame lands once the seam is taken account of, or None if there is no seam."""
        if loops:
            return first + (frame - first) % cycle
        return frame if first <= frame <= last else None

    for run in a_run_of(down, first, cycle if loops else None):
        for frame in run:
            offset[frame] = lift[frame].copy()
            strength[frame] = 1.0
        for edge, way in ((run[0], -1), (run[-1], 1)):
            for step in range(1, EASES_OVER + 1):
                frame = inside(edge + way * step)
                if frame is None or frame in lift:
                    continue
                share = 1.0 - step / (EASES_OVER + 1.0)
                if share > strength[frame]:
                    offset[frame], strength[frame] = lift[edge].copy(), share
    if loops:
        # The seam frame IS the first frame, so it takes the first frame's answer by definition
        # rather than by arithmetic that happens to agree.
        offset[last], strength[last] = offset[first].copy(), strength[first]
    return offset, strength


def plant_a_clip(rig, mesh, feet, ground, clip, scene, facing):
    """Stands a DELIVERED clip's feet on the floor, leaving every angle in it exactly as authored.

    # Why this can work where ten per-frame corrections could not

    Every attempt at these feet before this one ROTATED the foot, because there was nothing here
    that could move an ankle. That is the wrong thing to turn, and `foot_roll` has said so all
    along:

        "A rigid shoe does not pivot on its ball. It pivots on whichever END is touching: the heel
         at heel-strike, the toe at toe-off, and it is flat in between. So the tilt is left alone
         and the HEIGHT is solved. Whatever the tilt, the lowest part of the shoe is put on the
         floor, and the pivot then emerges by itself - heel, flat, or toe, in that order, without
         any of them being named. Which is the thing that makes this general: it never needs to
         know which end is down."

    So nothing here touches a rotation. The IK goes on the CALF, so the solver drives the ankle and
    the foot keeps whatever local angle the animator gave it, and the single unknown is how high
    the ankle sits. A shoe through the floor comes up; a shoe hovering over it goes down; a shoe in
    the air is left alone, because a foot in flight is not this function's business.

    # What it is actually fixing, measured

    `stand_on_the_floor` lifts a whole clip until nothing penetrates, and says plainly what it
    leaves behind: "anything left over FLOATS, which is the side of the error a ground solver can
    pull back down." This is that solver, run once into the asset instead of every frame at
    runtime. On the delivered jog what floats is one whole foot - the left sole sits 0.44 to
    3.27 cm above the floor through all of its own stance while the right reaches 1.51 cm through
    it. A foot that hovers for its entire plant has nothing to push against, and two feet
    disagreeing by 4.8 cm about where the ground is, is what "runs with broken feet" looks like
    from outside.
    """
    first, last = (int(round(v)) for v in clip.frame_range)
    play_it(rig, clip)

    soles, down, window, loops = where_the_feet_are_down(rig, mesh, feet, clip, scene)
    frames = (last - first) if loops else (last - first + 1)
    print(f"    {clip.name} {'loops, so its last frame is its first' if loops else 'does not loop'}"
          f"; {frames} frames of cycle")
    for side in "LR":
        print(f"    {side} sole runs {min(soles[side].values()) * 170.0:+.2f}"
              f"..{max(soles[side].values()) * 170.0:+.2f} cm; down on {len(down[side])} of "
              f"{frames} frames ({len(down[side]) / frames * 100:.0f}%), within "
              f"{window:.1f} cm of its own lowest")
    if not all(down.values()):
        refuse(f"{clip.name} never puts a foot down, so there is nothing to plant")

    # Where the clip's own ankles are, read BEFORE any constraint exists. Reading them afterwards
    # asks the rig where the solver just put them, which is the answer this is trying to compute.
    ankles, balls = read_the_feet(rig, scene, first, last)

    rigged = {side: ik_gait.add_leg_ik(rig, side) for side in "LR"}
    targets = {side: rigged[side][0] for side in "LR"}
    reach = ((rig.matrix_world @ rig.pose.bones["R_Foot"].bone.head_local)
             - (rig.matrix_world @ rig.pose.bones["R_Thigh"].bone.head_local)).length
    for side, (_, pole, hold) in rigged.items():
        ik_gait.aim_the_pole(rig, side, pole, hold, facing[0], reach)

    # # Solving, then re-applying
    #
    # The lift is found frame by frame with the solver running, then set aside and re-keyed as one
    # eased curve. It cannot be keyed as it is found, because the ramp needs to know where a plant
    # ENDS before it can taper out of it, and that is not known until the plant is over.
    cycle = (last - first) if loops else (last - first + 1)
    faces = ik_gait.the_way_he_faces(rig)
    ran, _ = how_fast_the_ground_goes_by(balls, down)
    crabbed = math.degrees(ran.angle(-faces)) if ran is not None else 0.0
    travel, each = how_fast_the_ground_goes_by(
        balls, down, along=-faces if RUNS_ALONG_HIS_FACING else None)
    print(f"    he faces ({faces.x:+.3f}, {faces.y:+.3f}) and the delivered clip ran "
          f"{crabbed:.2f} deg off it"
          + ("; held straight" if RUNS_ALONG_HIS_FACING else "; left as delivered"))
    print(f"    the planted balls travel ({travel.x:+.3f}, {travel.y:+.3f}) at "
          f"{each * 170.0:.2f} cm a frame, so a {cycle}-frame cycle covers "
          f"{each * cycle * 1.70:.3f} m")

    # Where every planted ball belongs, worked out ONCE from the clip as delivered. It must not be
    # recomputed after the body has been carried: the target is a place on the ground, and a target
    # that moves whenever the figure does is not a constraint, it is an echo.
    held_at = {}
    for side in "LR":
        held_at[side] = {}
        for run in a_run_of(down[side], first, cycle if loops else None):
            held_at[side].update(
                where_a_planted_foot_belongs(run, balls[side], travel, each)
                if LOCKS_THE_FOOT else {f: balls[side][f] for f in run})

    lift, saturated, over = {"L": {}, "R": {}}, {"L": {}, "R": {}}, {"L": {}, "R": {}}
    for attempt in range(HIP_TRIES):
        held = held_at
        for side in "LR":
            for frame in down[side]:
                scene.frame_set(frame)
                for other in "LR":
                    targets[other].location = ankles[other][frame].copy()
                targets[side].location = ankles[side][frame].copy()
                bpy.context.view_layer.update()
                # # One fixed point, holding the ball still and resting the sole
                #
                # Both halves move the same thing - the IK target, which is the ankle - so they
                # are solved together rather than one after the other. Moving the ankle along the
                # ground drags the sole's contact with it, and tilting is not involved in either,
                # because nothing here touches a rotation.
                #
                # Converging at all rests on the foot being RIGID: with its rotation untouched,
                # shifting the ankle a centimetre shifts the ball the same centimetre, so the
                # horizontal correction is exact on the first pass and only the solver's own
                # rounding brings it back. Height is the same one-for-one argument
                # `solve_the_target` gives. What is left to iterate is the coupling: the chain
                # rolls a little as it reaches, which turns the foot and moves the ball again.
                #
                # Except when the leg runs out. Asking an ankle DOWN or further OUT asks the leg
                # to lengthen, and a leg already straight cannot; the target then walks away from
                # the foot for as long as it is allowed to, and the sole still arrives at the
                # floor - not because the ankle came down but because the saturated chain PITCHES,
                # driving the toe in. That is a stabbed toe dressed up as a solved frame, and it is
                # the exact fault this pass exists to remove. So every step is checked against what
                # the ankle actually did, and the moment the chain stops following, the ask stops
                # with it.
                stalled, left = 0, None
                for _ in range(TRIES):
                    ball = rig.matrix_world @ rig.pose.bones[f"{side}_ToeBase"].head
                    across = held[side][frame] - ball
                    across.z = 0.0
                    if not HOLDS_SIDEWAYS:
                        # Along the line of travel only. Pulling a ball back to a line as well as
                        # to a speed moves the ANKLE sideways to do it, and how far apart his legs
                        # stand is the animator's call, not this pass's.
                        across = travel * across.dot(travel)
                    short = ground - ik_gait.lowest_sole(rig, mesh, feet, side)
                    step = across + mathutils.Vector((0.0, 0.0, short))
                    left = step.length
                    if left <= CLOSE_ENOUGH:
                        break
                    was = (rig.matrix_world @ rig.pose.bones[f"{side}_Foot"].head).copy()
                    targets[side].location = targets[side].location + step
                    bpy.context.view_layer.update()
                    went = (rig.matrix_world @ rig.pose.bones[f"{side}_Foot"].head) - was
                    # A poor first step is not saturation. The largest asks here are 8 cm of
                    # sideways travel, and a chain that has to swing round to them follows perhaps
                    # half the way on the opening pass and the rest on the next two - which is
                    # ordinary damped iteration, not a leg running out. Treating one bad step as
                    # the end aborted the solve on the frame with the BIGGEST correction, and left
                    # its sole 1.74 cm in the air with every other frame exact. Only a chain that
                    # will not move for several passes running has actually stopped.
                    if went.length < step.length * FOLLOWS_BY:
                        stalled += 1
                        if stalled >= STALLS_BEFORE_STUCK:
                            targets[side].location = targets[side].location - (step - went)
                            bpy.context.view_layer.update()
                            break
                    else:
                        stalled = 0
                here = rig.matrix_world @ rig.pose.bones[f"{side}_Foot"].head
                # What is LEFT OVER, measured fresh, and not how far the chain missed its target:
                # a target that never converged is reached perfectly and still has the foot in the
                # wrong place. Measuring the leftover is also what gives the body something to be
                # moved BY, since the shortfall of a leg is exactly the nudge that would fix it.
                ball = rig.matrix_world @ rig.pose.bones[f"{side}_ToeBase"].head
                short = held[side][frame] - ball
                short.z = 0.0
                if not HOLDS_SIDEWAYS:
                    short = travel * short.dot(travel)
                short.z = ground - ik_gait.lowest_sole(rig, mesh, feet, side)
                over[side][frame] = short
                saturated[side][frame] = short.length
                lift[side][frame] = here - ankles[side][frame]

        worst = max(v for side in "LR" for v in saturated[side].values())
        if worst <= CHAIN_REACHES_TO_WITHIN / 170.0 or not CARRIES_THE_BODY:
            break
        if attempt == HIP_TRIES - 1:
            break

        # # Moving the body to the foot, when the foot cannot be moved to the ground
        #
        # A leg that comes up short has said precisely how short, and in a gait with no double
        # support that shortfall IS the nudge: shift the figure by it and the planted foot arrives
        # where it belongs without the leg being asked for anything it does not have. Only one foot
        # is ever down, so nothing else is being dragged; the airborne leg rides along, which is
        # what an airborne leg does.
        wanted = {}
        for side in "LR":
            for frame, short in over[side].items():
                wanted[frame] = wanted[frame] + short if frame in wanted else short.copy()
        shifts = smoothly_between(wanted, first, first + cycle - 1, loops)
        far = max(v.length for v in shifts.values()) * 170.0
        if far > HIPS_MAY_SHIFT:
            refuse(f"keeping the plants still would move the body {far:.2f} cm, past the "
                   f"{HIPS_MAY_SHIFT:.1f} cm this may nudge it - that is a different performance, "
                   "not the same one without slide in it")
        for side in "LR":
            rigged[side][2].influence = 0.0
        bpy.context.view_layer.update()
        carrier = carry_the_body(rig, clip, shifts, scene, first, last, loops)
        # Still switched off for the reading, and only switched back on afterwards. Read with the
        # solver live and every frame returns wherever the last solved frame left the target -
        # measured, that put the recorded ankle 54 cm from the clip's own.
        ankles, balls = read_the_feet(rig, scene, first, last)
        for side in "LR":
            rigged[side][2].influence = 1.0
        print(f"    a leg was {worst * 170.0:.2f} cm short, so the body was carried up to "
              f"{far:.2f} cm on {carrier} and the plants solved again")

    pull = {side: how_hard_the_solver_pulls(down[side], lift[side], first, last, loops)
            for side in "LR"}

    # A clip that loops must still loop: the first and last frames hold the same pose, so they must
    # take the same correction, and the same amount of it, or a seam opens where there was none.
    for side in "LR":
        offset, strength = pull[side]
        seam = (offset[first] * strength[first]
                - offset[last] * strength[last]).length * 170.0
        if seam > 0.02:
            refuse(f"the {side} plant breaks {clip.name}'s loop by {seam:.2f} cm")

    for frame in range(first, last + 1):
        scene.frame_set(frame)
        for side in "LR":
            offset, strength = pull[side]
            targets[side].location = ankles[side][frame] + offset[frame]
            targets[side].keyframe_insert("location", frame=frame)
            rigged[side][2].influence = strength[frame]
            rigged[side][2].keyframe_insert("influence", frame=frame)

    ik_gait.bake_the_constraints(rig, first, last)
    # The target and the pole only: the third of the three is the CONSTRAINT, which the
    # bake has already cleared, and which was never an object to remove.
    ik_gait.drop_the_helpers([h for r in rigged.values() for h in r[:2]])

    worst, missed, stood = 0.0, [], {"L": {}, "R": {}}
    play_it(rig, clip)
    for frame in range(first, last + 1):
        scene.frame_set(frame)
        bpy.context.view_layer.update()
        for side in "LR":
            stood[side][frame] = (
                rig.matrix_world @ rig.pose.bones[f"{side}_ToeBase"].head).copy()
            if frame not in lift[side]:
                continue
            off = abs(ground - ik_gait.lowest_sole(rig, mesh, feet, side)) * 170.0
            worst = max(worst, off)
            if off > REACHES_TO_WITHIN:
                missed.append(f"{side}{frame} by {off:.2f} cm")

    # # Checking the heading that was asked for, over the frames it was asked on
    #
    # Measured off the BAKED clip, so what is being read is what ships and not what the solver
    # believed. Over the locked frames only: a stance window measured from outside admits the
    # frames either side, where the correction is deliberately part-applied, and those carry the
    # crab this is meant to have removed. Read at a 2 cm window from outside, the delivered jog's
    # 5.36 degrees came back as 3.08 and it was not obvious whether that was residue or ruler.
    went = []
    for side in "LR":
        for run in a_run_of(down[side], first, cycle if loops else None):
            for at in range(len(run) - 1):
                if run[at + 1] != run[at] + 1:
                    continue
                step = stood[side][run[at + 1]] - stood[side][run[at]]
                step.z = 0.0
                went.append(step)
    heading, _ = ik_gait.the_line_of_travel(went)
    if heading is not None:
        drifts = math.degrees(heading.angle(-faces))
        print(f"    over the frames it was held on, the ground now goes by {drifts:.2f} deg off "
              f"his facing, against {crabbed:.2f} as delivered")
        if RUNS_ALONG_HIS_FACING and drifts > RUNS_STRAIGHT_WITHIN:
            refuse(f"the plants were held to his facing and still run {drifts:.2f} deg off it, "
                   f"past the {RUNS_STRAIGHT_WITHIN:.2f} that counts as straight")
    shifts = [(pull[side][0][f] * pull[side][1][f]) for side in "LR" for f in pull[side][0]]
    moved = max(v.length for v in shifts) * 170.0
    sideways = max(math.hypot(v.x, v.y) for v in shifts) * 170.0
    frames_over = frames * 2
    solving = sum(1 for side in "LR" for f in range(first, first + frames)
                  if pull[side][1][f] > 0.0)
    stuck = max(v for side in "LR" for v in saturated[side].values()) * 170.0
    print(f"    planted {sum(len(d) for d in down.values())} foot-frames; the ankle moved at "
          f"most {moved:.2f} cm, {sideways:.2f} cm of that across the ground, and the chain "
          f"followed to within {stuck:.2f} cm")
    if sideways > A_PLANT_MAY_SHIFT:
        refuse(f"holding a plant still would move a foot {sideways:.2f} cm across the ground, "
               f"past the {A_PLANT_MAY_SHIFT:.1f} cm this may restyle - that is a different step, "
               "not a step with slide in it")
    if stuck > CHAIN_REACHES_TO_WITHIN:
        refuse(f"a plant was left {stuck:.2f} cm from where it belongs, so the foot is being "
               "leaned on rather than stood on")
    print(f"    the solver is off for {frames_over - solving} of {frames_over} foot-frames, so "
          f"those legs play exactly as delivered; every planted sole sits within {worst:.2f} cm "
          f"of the floor")
    if missed:
        refuse(f"{len(missed)} planted sole(s) stayed more than {REACHES_TO_WITHIN:.2f} cm off "
               f"the floor - the leg could not reach, so the plant would be a lie: "
               + ", ".join(missed))
    return clip


def the_body(things):
    """The skinned mesh, which is the one with vertex groups and the most vertices."""
    skinned = [o for o in things if o.type == "MESH" and o.vertex_groups]
    if not skinned:
        refuse("no skinned mesh in this file")
    return max(skinned, key=lambda o: len(o.data.vertices))


def main():
    args = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    model = args[args.index("--model") + 1] if "--model" in args else MODEL
    called = args[args.index("--name") + 1] if "--name" in args else "jog"
    if not os.path.isfile(model):
        refuse(f"{model} is missing - build the character first")

    bpy.ops.wm.read_factory_settings(use_empty=True)
    for stale in list(bpy.data.objects):
        bpy.data.objects.remove(stale, do_unlink=True)
    bpy.ops.import_scene.gltf(filepath=model.replace("\\", "/"))

    rig = next((o for o in bpy.data.objects if o.type == "ARMATURE"), None)
    if rig is None:
        refuse("no armature in this file")
    body = the_body(bpy.data.objects)
    if rig.animation_data is None:
        rig.animation_data_create()
    scene = bpy.context.scene
    scene.render.fps = 24
    doing = "authoring" if "--author" in args else "planting"
    print(f"{doing} '{called}' onto {os.path.basename(model)}: "
          f"{len(rig.data.bones)} bones, {len(body.data.vertices)} vertices")

    # The knee has to be bent or there is no telling which way it folds - see the header.
    hip = rig.matrix_world @ rig.pose.bones["L_Thigh"].bone.head_local
    knee = rig.matrix_world @ rig.pose.bones["L_Calf"].bone.head_local
    ankle = rig.matrix_world @ rig.pose.bones["L_Foot"].bone.head_local
    straight = (knee - hip).length + (ankle - knee).length
    stands = (ankle - hip).length / straight if straight > 1e-9 else 1.0
    print(f"  the bind stands at {stands * 100:.1f}% of straight")
    if stands > 0.999:
        refuse("the bind's knees are dead straight, which is singular for the solver - "
               "build_character's KNEE_EASE is what puts the bend in")

    rest(rig)
    bpy.context.view_layer.update()
    facing = across_the_body(rig)
    print(f"  forward is ({facing[0].x:+.3f}, {facing[0].y:+.3f}) at rest")

    feet = which_vertices_are_feet(body)
    print(f"  {len(feet['L'])} left-foot vertices, {len(feet['R'])} right")
    rest(rig)
    bpy.context.view_layer.update()
    rests_at = min(sole_of(rig, body, feet, side) for side in "LR")
    # The floor is zero, never wherever this model's sole happens to rest. The old file learned
    # that the hard way: taking the rest sole as ground solved every clip onto a floor 5.7 cm
    # underground and reproduced it faithfully for eight passes.
    ground = 0.0
    print(f"  the floor is z=0; this model's sole rests at {rests_at * 170.0:+.1f} cm")

    if "--author" in args:
        made = gait(
            rig, body, feet, ground, called, RUN_LEG, RUN_SPAN, RUN_CONTACT,
            RUN_SWING_LIFT, RUN_SWING_SHAPE, RUN_LANDS_AHEAD,
            RUN_ARM_FORWARD, RUN_ARM_BACK, RUN_ELBOW_HELD, RUN_ELBOW_SWING,
            RUN_LEAN, RUN_SHARE, RUN_SINKS, RUN_LEADS, RUN_BOUND, RUN_ABSORBS,
            RUN_TUCK_IN, RUN_CROSSES_IN, RUN_PUMPS, RUN_TWIST, RUN_PELVIS, facing,
        )
        if made is None:
            refuse("gait() authored nothing")
    else:
        # # Planting the delivered clip, rather than authoring over it
        #
        # Authoring produces a gait with no measurable faults in it, and it produced one: planted
        # soles on every frame, no slide, a stride and cadence inside the published bands. What it
        # cannot produce is a PERFORMANCE. Reported on the authored jog - "his legs are way further
        # apart than they should be and he runs at an angle, his hips are pointed the wrong way and
        # dont move" - and every one of those is a thing `gait` states from a constant, which is
        # only ever as good as the guess behind it.
        #
        # The delivered clip has an animator's answer to all three already. What it does not have
        # is contact with the floor. So the clip is kept and only the contact is solved, which is
        # the smaller and far better-posed half of the problem. See `plant_a_clip`.
        made = next((a for a in bpy.data.actions if a.name == called), None)
        if made is None:
            refuse(f"there is no clip called '{called}' to plant; this file has "
                   + ", ".join(sorted(a.name for a in bpy.data.actions)))
        print(f"  planting the delivered '{called}', frames {made.frame_range[0]:.0f}"
              f"..{made.frame_range[1]:.0f}")
        plant_a_clip(rig, body, feet, ground, made, scene, facing)
    made.use_fake_user = True
    print(f"  {'authored' if '--author' in args else 'planted'} '{made.name}', frames {made.frame_range[0]:.0f}"
          f"..{made.frame_range[1]:.0f}")

    # The delivered clip of the same name goes, or the file ships two and the game plays one of
    # them at random.
    for stale in [a for a in bpy.data.actions if a.name == called and a is not made]:
        print(f"  dropped the delivered '{stale.name}'")
        bpy.data.actions.remove(stale)
    made.name = called

    rig.animation_data.action = made
    slots = getattr(made, "slots", None)
    if slots:
        rig.animation_data.action_slot = slots[0]
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.export_scene.gltf(
        filepath=model.replace("\\", "/"), export_format="GLB", use_selection=True,
        export_yup=True, export_apply=False, export_animations=True,
        # Not resampled, for the same reason build_character does not resample: the exporter's
        # default bakes every action onto the scene rate and that cost the run 25 degrees of
        # loop accuracy on its own.
        export_force_sampling=False,
    )
    print(f"wrote {model}")


if __name__ == "__main__":
    main()
