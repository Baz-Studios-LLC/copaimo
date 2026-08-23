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
import unfuse  # noqa: E402
import prepare_rig  # noqa: E402  (the A-pose numbers; guarded, so importing runs nothing)
import foot_roll  # noqa: E402
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

# MEASURED 2026-08-23: the hip socket sits at 85.2 cm on a 170.2 cm figure, which is
# 50.1% - dead in the adult human 50-52% band. The '45%' below is WRONG and it misled
# hours of tuning, because it made every reach limit look anatomical and therefore
# unfixable. The real limit is a constant: the bind stands at 99.7% leg extension
# (hip-to-ankle 78.1 cm on a 78.35 cm leg), so ik_gait.STANCE_LEG_EXTENDS = 0.98 caps
# usable reach BELOW what standing upright needs, and the crouch that follows is what
# eats the stride. The genuine oddity is that he is 5.7 heads tall against 7.5
# realistic - a large head, not short legs.
#
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
# PER GAIT, because ground sweep and speed part ways the moment the feet leave the
# ground. A walk covers ground only while a foot is on it, so its sweep is long. A run
# covers most of its ground IN FLIGHT: ask its stance for a walk-length sweep and the
# leg simply cannot reach - measured, the solver clamped the run to 0.56 m and the
# sprint to 0.44 m of the 0.88 asked, saturating the knee dead straight at touchdown
# (folded 0.0000) to do even that. These author what the legs were measured to do,
# so touchdown keeps its pre-flex instead of spending it on an unreachable span.
# 0.34, and NOT the one-leg-length that human data gives.
#
# The research figure is right for humans and wrong for this character. Contact length
# is about one leg length in people - but a person's leg is 52% of their height, and
# this character's is 45% (the user's stylisation, and not up for changing). A 78 cm
# leg reaching a full leg length ahead has to crouch to do it: the hip can only ever be
# sqrt(reach^2 - ahead^2) above a planted ankle, so 44 cm of forward reach dropped the
# hip 16.5 cm, the stance leg sat at 79% extension, and both knees were ahead of the
# hips on every frame. Reported, correctly, as "no way he'd be balanced".
#
# So the stride is set by what the legs can carry upright, which is the honest
# constraint for these proportions. It costs ground speed and buys a walk that stands
# up straight - and speed is recovered by cadence in the game's playback rate.
# 0.34, and 0.30 was tried and reverted.
#
# The thigh swings -23 to +30 degrees from vertical here. That was read against a
# remembered "-20 to +25" and trimmed, which was wrong twice over: normal walking's
# PEAK HIP FLEXION is 30 to 35 degrees at terminal swing, so +30 is ordinary, and the
# trim did not even move it - the forward extreme is set by the swing arc, not the
# stance sweep, so 0.30 took 8 degrees off the REAR extension (-23 to -14), put the
# hip ahead of both feet on two frames, and cost a tenth of the walking speed.
#
# The forward knee travel that goes with +30 degrees of flexion - about 20 cm in front
# of the hip at heel strike - is what a real stride looks like on a 42 cm thigh.
WALK_CONTACT = 0.34
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
SPRINT_CONTACT = RUN_CONTACT * 1.0

# Where each gait's foot lands, as the share of its sweep ahead of the hip. See
# where_the_balls_go: walks land near half way; runners land short and release long.
# 0.54, not the symmetric 0.5, since the toes learned to stay flat: with the pad on
# the ground the trailing ankle no longer rides up over the shoe tip, so the rear
# extreme of the old window sat beyond the leg's reach and the trailing knee froze
# straight. Landing a little further ahead and releasing a little sooner behind is
# also what people do - the ball leaves just after the opposite heel lands.
# 0.44: more of the sweep goes BEHIND the hip than in front.
#
# The thigh, measured, sat forward of vertical for all but one frame of the cycle and
# reached only -10 degrees behind, where a person swings -20 to +25 and spends about a
# third of the cycle with the thigh extended behind them. That is what reads as the
# hips sitting forward of the legs. Two things follow from moving the window back: the
# trailing foot travels further behind, so the thigh extends; and the LEADING foot
# reaches less far forward, which is what the hip drop is paid for, so the hip rides
# higher and the stance leg is straighter - a straighter leg puts its knee less far
# forward, which lets the thigh get behind vertical at all.
WALK_LANDS_AHEAD = 0.48
# 0.46 for the run, up from 0.38: the ball joint moved forward to the shoe's real
# flex point, so the ankle now sits further behind the ball and the trailing leg has
# to stretch further for the same sweep - measured, it saturated dead straight at
# touchdown. Landing more of the sweep ahead buys that reach back.
RUN_LANDS_AHEAD = 0.30
SPRINT_LANDS_AHEAD = RUN_LANDS_AHEAD

# Stance counts are what separate the gaits: five of eight poses on the ground is a
# walk (some foot always down), three is a run and two a sprint (neither down - the
# flight phase). Duty factor is the formal line between walking and running, and it is
# also where a long stride comes from: planted-foot travel stays near one leg length
# at EVERY speed (0.99 +/- 0.08 m from 6.2 to 11.1 m/s), so stride grows by spending
# less of the cycle on the ground, never by reaching further - reaching further is
# what made 42 degrees of thigh swing read as the splits.
#
# How high each gait's swing foot arcs, as a share of leg length. The arc is what
# folds the swing knee - see where_the_balls_go - so a run's is far higher: a walker
# clears the ground, a runner's heel tucks toward the seat.
# 0.14, up from 0.08. Measured, 0.08 cleared the swing foot only 5.0 cm off the floor
# and it read as barely leaving the ground - reported. Real walking clears the toe by a
# famously small 1-2 cm, so anatomy is no argument for staying low here: the reason to
# lift more is that a 171 cm character seen at game distance needs the swing to be
# legible, and 0.14 gives about 11 cm.
WALK_SWING_LIFT = 0.14
# 0.34, up from 0.24. The legs half of "both the arms and legs need more movement".
#
# Williams' pass position has the swing knee well up in front with the heel tucked, and ours
# sits low - which reads as shuffling rather than running. This was tried once before and
# judged a failure because foot SEPARATION dropped, 49.9 to 42.3 cm. That was the wrong
# test: a high knee with a folded heel is meant to bring the foot closer to the body, and
# measuring how far apart the feet get penalises exactly the pose being asked for. The knee
# drove to +40.6 degrees, which is the thing that was wanted.
RUN_SWING_LIFT = 0.34
SPRINT_SWING_LIFT = RUN_SWING_LIFT * 1.15

# Where each arc peaks - the exponent on the swing's progress inside its sine, see
# where_the_balls_go. 1.0 peaks at mid-swing; below 1 pulls it earlier.
# 5 degrees. Two was measured at +1.6 of actual trunk flexion and still read as
# leaning BACK, twice - because the backpack's mass sits behind him, so upright is not
# neutral on this character. A person carrying a load leans into it.
# 7 degrees. Five measured at +4.0 of trunk flexion and STILL read as leaning back -
# the third report of it. The numbers were never wrong: the head sits 4.3 cm ahead of
# the hip. What they miss is the BACKPACK, whose mass hangs behind him and dominates
# the silhouette, so a trunk that leans forward still reads as reclining. The lean has
# to beat the pack, not just beat vertical.
WALK_LEAN = 7.0
WALK_SWING_SHAPE = 1.0
RUN_SWING_SHAPE = 0.6
SPRINT_SWING_SHAPE = RUN_SWING_SHAPE

# The rig this authors against, produced by `prepare_rig.py`. Named once, here, because
# two producers writing two filenames is how the pipeline came to be reading a stale file
# without saying so.
PREPARED_RIG = "ranger_apose.glb"

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

# One leg's ANKLE PITCH through a full cycle, in degrees at each eighth, positive
# toes-up. Degrees the sole is tilted off the floor, with the bind pose as the zero.
#
# It used to be a (thigh, knee, ankle) triple, and the first two columns are gone: IK
# solves the thigh and the knee from the foot's path, so nothing ever read them. They
# were sampled by `pose_the_body`, which had no caller left. The authored values are
# kept in each row's comment, because they record what the pose was designed to be and
# that is worth having if the legs are ever hand-posed again - but they are a NOTE now,
# not data pretending to be used.
#
# Originally: one leg through a full cycle,
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
# The pitch column IS the rollover: strike toes-up, slap FLAT by the first eighth,
# stay flat while the body passes over, then the heel rises into toe-off. The first
# version put +5 and +12 toes-up at passing and up - the number contradicting its own
# comment - so mid-stance the foot rocked back onto its heel, and the whole walk read
# as heel-walking. A sole is flat for the middle half of stance; only the ends tip.
#
# The heel RISE is spread over the back half of stance (0 -> -4 -> -15 -> -32 from a
# quarter in) rather than held flat to a third and then dumped: the compressed rise
# read as "mostly heel". Anatomy holds the sole flat to ~30% and this eases a touch
# sooner - a read choice, like the strike below.
#
# The strike angle is 8, BELOW the measured human mean on purpose. Gait analysis puts
# the foot-floor angle at initial contact at 18.7 +/- 2.8 degrees in healthy adults,
# flat by ~12% of the cycle (which this clip already matches) - but this character's
# shoe is a 29 cm stylised block, so every degree of toe-up lifts a big visible box:
# 12 read as an exaggerated heel-first stomp. The read outranks the chart here.
WALK_LEG = (
    8.0,      # 0     contact - heel strike, toes up, knee nearly straight  [thigh 38.0, knee 4.0]
    0.0,      # 12.5  down - slapped flat, weight loaded  [thigh 26.0, knee 20.0]
    -8.0,     # 25    passing - HEEL-OFF: it happens at half of stance,  [thigh 6.0, knee 16.0]
    #                       which is a quarter of the cycle here (clinical heel rise
    #                       sits at ~30% of the gait cycle, and this path's stance is
    #                       compressed to half). Holding it flat past this point left
    #                       the trailing heel visibly planted while the other leg
    #                       swung by - a stretched leg on a glued heel.
    -22.0,    # 37.5  up - the heel well up, hips highest  [thigh -10.0, knee 0.0]
    # -22, not -38. The heel lift IS the toe-off, but it raises the ankle: measured at
    # -38 the trailing ankle sat 15.3 cm up with its ball on the floor, which shortens
    # the hip-to-ankle chord and folded that knee 68 degrees where a person is nearer
    # 40. This character cannot have both - its hip is 85.5 cm over a 78 cm leg, so a
    # 40-degree trailing fold at this stride would need the hip ABOVE where it stands
    # at rest. Given the choice, a leg that reads straight beats a deeper heel lift.
    -22.0,    # 50    toe-off - rolling off the creased toes  [thigh -27.0, knee 24.0]
    -8.0,     # 62.5  initial swing - knee folding to its peak  [thigh -6.0, knee 64.0]
    0.0,      # 75    mid-swing - clearing the ground  [thigh 14.0, knee 68.0]
    8.0,      # 87.5  terminal swing - presenting the heel at the strike angle  [thigh 33.0, knee 22.0]
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
    -48.0,    # 25    push-off - leg extended hard behind, driving back  [thigh -38.0, knee 4.0]
    #                       Was -32. This is the reach knob, not just a pose.
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
    -30.0,    # 37.5  early flight - knee folding up behind  [thigh -30.0, knee 70.0]
    -36.0,    # 50    peak fold - heel toward the buttock  [thigh -6.0, knee 100.0]
    -34.0,    # 62.5  knee drive - thigh coming through high  [thigh 22.0, knee 92.0]
    -18.0,    # 75    reaching - the furthest the leg gets in front  [thigh 38.0, knee 50.0]
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
# THE SPRINT IS THE JOG, MORE AGGRESSIVE - and stated as the jog's values so that every
# fix to the jog carries over rather than being re-tuned here twice.
#
# Its own table was the old forefoot shape: -18 at contact, then -34/-30. That is why it
# refused on "the front foot is no more toes-up than the back one" - measured, its leading
# foot sat at -8.99 and its trailing at -6.00, so the roll had no direction at all and a
# reversed cycle would have looked identical. The jog's table is heel -> flat -> toe with a
# real direction, and AnimSchool is explicit that heel contact is the STYLISED convention
# while ball-of-foot is the realistic one. A real sprinter is a forefoot striker; if that
# is wanted later it needs the reach budget checked first, which is exactly the trap that
# cost five rounds on the walk and another on the jog.
SPRINT_LEG = RUN_LEG

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
RUN_ARM_FORWARD = 38.0
RUN_ARM_BACK = -55.0

# A sprint drives the arms harder still, and the check in `verify_gait.py` refuses a
# clip whose hands cover less than a quarter of what its feet do - which a sprint's
# much longer stride makes easy to fail.
# 1.9, up from 1.45. Arm swing scales with speed, and this is the knob asked for twice.
# There IS a ceiling: at an outright 46/-58 the arm went so far back that the elbow read as
# in FRONT of the shoulder-to-wrist line and refused. That was with the elbow held at 62,
# leaving 44 degrees of fold at the back extreme; at 88 it keeps 70, so there is more room
# now than there was - but the refusal is the thing to watch if this goes higher.
# 1.1 and 1.55, down from 1.9 and 2.7. These were not chosen, they were compensation: the
# run's own values were less than half what they should have been, so the sprint needed a
# huge multiple to look like a sprint. Now that the run is honest, the same multiples would
# put the sprint at 72 and -148 degrees. Retuned to hold the sprint at the 42/-85 that
# measured well - about 127 degrees of swing against the run's 93.
SPRINT_ARM_FORWARD = RUN_ARM_FORWARD * 1.1
# The BACK swing is scaled harder than the forward one, and separately from it.
#
# Reported as "the elbows dont go back far enough". They can be pushed on their own: what
# refused earlier was the FORWARD fold crossing the shoulder-to-wrist line near 106
# degrees, and the back extreme is nowhere near any limit - measured, the elbow sits
# 15.05 cm BEHIND that line there, and travelled only 21.6 cm behind the shoulder.
# A sprinter's arms are asymmetric anyway: the drive back is the powerful half and the
# forward recovery is shorter.
SPRINT_ARM_BACK = RUN_ARM_BACK * 1.55

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
HEAD_LAGS_THE_CHEST = 0.10

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

# How far the head bobs vertically, in model units, lagging the hips.
#
# The hips already rise and fall; the head followed them rigidly because nothing else
# touched it. A real head lags the body's bob and settles after it, which is follow-through
# - and the head is the part an eye tracks, so this is the cheapest liveliness there is.
# 0.005, down from 0.014. Reported: "the head bob is extreme". It was, and the number is
# not what changed - keying it is. `key()` only wrote a `location` channel for Hip and Root,
# so the head term was set on the pose and thrown away, and every value tried measured the
# same 6.29 cm of travel. Adding "Head" to that list connected a knob that had been turned
# up blind, and travel jumped to 13.50 cm at once. This is what it should have been set to
# had it ever been measurable.
HEAD_BOBS = 0.005

# How much of the hips vertical the head cancels. 0.85 lands the head near 65% of the
# what the pelvis does, which is the right way round - see `head_rides_less` at the point of
# use for why it was the wrong way round before.
HEAD_RIDES_LESS = 0.85

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
FOREARMS_TUCK_IN = 24.0

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

# The elbow bends more at the forward extreme and straightens toward the back one,
# because an elbow cannot fold backward: an arm swinging behind the body has to
# straighten. Base 25 with 12 either way gives 37 in front and 13 behind, inside
# the brief's 35-45 front and 10-20 back.
ELBOW_HELD = 25.0
ELBOW_SWING = 12.0
RUN_ELBOW_HELD = 88.0
RUN_ELBOW_SWING = 18.0

# The sprint's elbows are tight and stay near constant, rather than opening and closing.
#
# # Which end of the swing was the problem, because the obvious answer is wrong
#
# The refusal is "the elbow sits in FRONT of the shoulder-to-wrist line, so the arm folds
# backwards", and the intuition is that an arm thrown too far BEHIND causes it - an elbow
# cannot fold the other way, so a back-swung arm straightens. That was the first fix tried
# here, tightening the hold to 96 to protect the back extreme, and it made things WORSE.
#
# Measured per frame, the offending arm is the FORWARD one. At the back extreme the elbow
# sits 15.05 cm BEHIND the line, which is entirely healthy. At the forward extreme it sits
# 1.69 cm in front, and the fold there is 107.8 degrees - because `elbow_swing` ADDS at the
# forward extreme. The crossing point is near 106: measured, 105.9 degrees of fold put the
# elbow 1.32 cm behind the line and 107.3 put it 0.94 in front.
#
# So the swing has to come DOWN, not the hold up, and the ceiling is the forward total.
# 94 + 4 = 98 at the front and 90 at the back - tight at both ends, which is a sprinter's
# arm anyway, and clear of the crossing with room to spare.
SPRINT_ELBOW_HELD = 94.0
SPRINT_ELBOW_SWING = 4.0

# How far the SHOULDERS counter-rotate against the hips, in degrees each way.
#
# Reported as "the elbows dont go back far enough" - and more shoulder extension cannot
# fix that, because the elbow sits one upper-arm length from the shoulder and so its
# backward travel is capped at 26 cm. Measured, it reached 25.8 of that 26 with the swing
# already at 2.7x, which is 99% of the geometric limit. Pushing the angle further just
# rotates the arm past horizontal and buys nothing.
#
# What was missing is that the TORSO never rotated at all. A runner's shoulders twist
# against the pelvis, and that carries the whole shoulder - and the elbow hanging off it -
# further back on the retreating side. It is also what makes a sprint read as driving
# rather than pedalling.
#
# Applied to Spine02 about world up, in phase with the arm swing, so the shoulder goes
# back with its own arm. The arms are aimed in WORLD terms AFTER the spine moves, so their
# angles are unchanged by this - only where they start from.
# Measured elbow travel against the PELVIS, which is the frame that shows it - against the
# shoulder it is invisible, because counter-rotation carries the shoulder and the elbow
# together:
#
#     walk    0 deg -> the elbow reaches  4.0 cm behind the hips, 16.2 cm of travel
#             5 deg ->                    5.3 cm behind,          17.5 cm
#     jog     0 deg ->                    5.3 cm behind,          22.1 cm
#            10 deg ->                    8.0 cm behind,          27.6 cm
#     sprint  0 deg ->                    8.9 cm behind,          42.6 cm
#            18 deg ->                   13.4 cm behind,          51.8 cm
#
# 5 / 10 / 18 across the three tiers. The walk's is the smallest because a walk really does
# rotate least, and it was checked against the approved clip rather than assumed: lean,
# slide and thigh symmetry all measured identical to before it was added.
WALK_TWIST = 5.0
RUN_TWIST = 10.0
SPRINT_TWIST = 18.0

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

# --- The PELVIS, on all three axes. This is what stops a gait reading as stiff.
#
# Reported as "still feels too stiff" and "from the side the run resembles the old Scooby
# Doo character run" - legs cycling under a torso that does not answer them. The cause was
# that the pelvis was not posed AT ALL: `RUN_DROP` was set to 0.0 and `PELVIS_SWAY` was
# defined and never referenced, so lateral sway measured exactly 0.0000 cm in every clip
# and there was no obliquity and no pelvic yaw either. A rigid pelvis on cycling legs is
# the cartoon run.
#
# Walking reference, from gait analysis: the pelvis inclines side to side about 7 degrees,
# rotates about 5, and sways laterally toward whichever foot carries the weight - about
# 3 cm at this height, which is what PELVIS_SWAY already said before anything read it.
# Faster gaits carry more of all three.
#
# The three are separate motions and each does a different job:
#
#   SWAY moves the pelvis over the planted foot, which is what balancing on one leg
#     actually requires - without it the body hangs between its feet.
#   OBLIQUITY drops the hip on the SWING side and lifts it over the support leg, which
#     is the single clearest read of weight being carried.
#   YAW rotates the pelvis against the shoulders. The shoulders already counter-rotate
#     (see SPRINT_TWIST); with the pelvis fixed, that twist had nothing to twist against.
#
# Obliquity was previously given up - "the least visible thing in a clip whose legs are
# already dramatic" - because it made the run limp, the two thighs' local axes not being
# mirror images. That diagnosis was wrong: the limp was `out[POSES] = out[0]` zeroing one
# frame of the flight arc, and it is fixed. So this is worth trying again, and the limp
# guards will say whether it holds.
#
# SWAY SHRINKS WITH SPEED, and the first version had it backwards.
#
# Reported as "he seems to just be swaying side to side", and two things were wrong. The
# amplitudes were about double - the ~3 cm the reference quotes for a walk is the TOTAL
# excursion, so the amplitude is half of it - and the scaling ran the wrong way: I gave the
# sprint the most sway when it should have the least. Lateral displacement of the centre of
# mass DECREASES with velocity, because faster gaits land closer to the midline; the same
# literature treats excess lateral movement as a pathology marker rather than a flourish
# ("every centimetre increase ... associated with a 30% reduction in running ability").
#
# So sway falls walk -> jog -> sprint. Yaw still rises with speed - pelvic rotation grows
# with the stride, and it is the axis that reads as drive rather than wobble.
#
# OBLIQUITY IS KEPT SMALL, and 11 degrees was too much for a reason worth writing down.
# Contralateral pelvic drop is a clinical measure: healthy running carries a little of it,
# and MORE of it is the Trendelenburg pattern - weak hip abductors failing to hold the
# pelvis level over the stance leg. Bramah 2018 puts it starkly: for each 1 degree of extra
# pelvic drop, the odds of a runner being classified as injured rose by 80%. So a big drop
# does not read as effort, it reads as a laboured runner about to get hurt, which is the
# opposite of what this character should look like.
#
# The direction is still drop-on-the-swing-side - that is what the gluteus medius acting
# eccentrically over the stance leg produces - but the size is now the modest one a strong
# runner shows. Two sources disagreed on the direction (an animation tutorial has the hip
# dropping on the bent-leg side, the clinical literature has a healthy pelvis level or
# slightly up on the lifted side); the mechanism settles it, and the difference between
# them is magnitude rather than sign.
#
# (sway in model units, obliquity in degrees, yaw in degrees)
WALK_PELVIS = (0.009, 4.0, 5.0)
RUN_PELVIS = (0.006, 5.0, 7.0)
SPRINT_PELVIS = (0.004, 6.0, 9.0)

# How far the hips are carried FORWARD of where they rest, in model units, PER GAIT.
#
# This replaces a single `PELVIS_LEADS = 0.012` that was defined and never referenced -
# orphaned the same way `fill_in_the_flight` still is - so nothing has been carrying its
# hips forward, in any clip. Per-gait rather than shared because the walk's look is
# settled: WALK_LEADS stays at zero, so wiring this up cannot move it, and only the
# faster gaits lean into their stride.
#
# Safe on `Root` despite it parenting everything, because the ball path is stated against
# each thigh's REST socket (`bone.matrix_local.translation`), not its posed one. So the
# hips move and the footfalls do not follow - which is the whole point, and would
# silently cancel if the path used the posed socket.
WALK_LEADS = 0.0
RUN_LEADS = 0.012
SPRINT_LEADS = RUN_LEADS * 1.5

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
# Given as the SHARE of the cycle each foot is down, not as a count of poses.
#
# It was `stance / POSES` - a whole number of eighths - and that granularity is what
# capped the flight phase. The reference breakdown (Schubert's 24-frame run, a key every
# three frames) puts exactly ONE of the four poses per step in the air, which is a 25%
# flight fraction and so a 0.333 duty per foot. Eighths cannot say 0.333: 3/8 is 0.375
# and 2/8 is 0.25. At 0.375 the closed stance interval covers 10 of 24 frames per foot,
# leaving 24 - 20 = 4 airborne, and four is not enough for the ballistic arc to BE an
# arc - it plateaus across two frames and then drops, which the bounce guard refuses. At
# 0.333 it is 9 per foot and 6 airborne, three per stretch, which is the shape the
# reference describes and the room the arc needs.
#
# The walk and sprint keep their old values exactly, written as the fractions they were.
WALK_SHARE = 5.0 / 8.0
RUN_SHARE = 8.0 / 24.0
SPRINT_SHARE = 2.0 / 8.0

# And how far the body rises above the straight line between two planted poses while
# it is airborne. Nought for a walk, which never leaves the ground.
#
# A run's vertical oscillation is 6 to 9 cm at recreational pace; below about 5 it
# reads as a shuffle that never reaches flight. In model units on a figure one unit
# tall, 0.022 is 3.7 cm of arc on top of whatever the planted geometry already gives,
# and 0.034 is 5.8.
# How far the hips may sink below standing, PER GAIT.
#
# ik_gait.HIP_DROPS_AT_MOST is 4 cm and its own comment says why: "four is also what
# real walking's whole vertical hip travel measures". That is a WALK figure, and it was
# being applied to the run and the sprint as well. A jog's hips oscillate 8-10 cm across
# three independent labs, so the cap was holding the run to less than half a jog's travel
# on the strength of a measurement of walking.
#
# It also made the heel strike impossible. The run's reach ceiling at contact is
# -5.39 cm - that is how far the hip MUST drop for the leg to put its foot on the floor
# out in front - and a 4.08 cm cap cannot get there, so the foot stayed 1.17 cm up
# however the stride was trimmed.
#
# The run gets 5.44 cm, which is what its own ceiling asks for and no more. Note this is
# the drop AT CONTACT, where the knee is nearly straight; it is NOT licence to deepen
# mid-stance, where the knee already folds 56-59 deg against a human jog's 40-45.
WALK_SINKS = 0.024
RUN_SINKS = 0.056
SPRINT_SINKS = RUN_SINKS

WALK_BOUND = 0.0
RUN_BOUND = 0.016
SPRINT_BOUND = 0.022

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
# 2.3, up from 1.55: the reference range for a SPRINT is 15-30 degrees of trunk lean and
# 1.55 delivered 13.01, which read as under-committed. verify_gait has a floor on forward
# lean and no ceiling for a flying gait, so the only bound here is taste.
# 1.45, down from 2.3 - the third multiplier found to be compensation rather than a choice,
# after the two on the arms. It was tuned when RUN_LEAN was 9, so it had to be large to make
# a sprint look like one; against an honest 15 the same factor asks for 34.5 degrees, which
# is a fall rather than a lean. Held at about 22, which is sprinter territory.
SPRINT_LEAN = RUN_LEAN * 1.45

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
# FROM THE BIND POSE, not a number of this file's own. prepare_rig.py bakes the legs
# at LEGS_OUT and the feet at TOE_OUT, so asking for exactly those angles means the
# authoring applies ZERO correction at rest - any other value here quietly rotates
# every leg at the hip on every frame to fight the bind.
LEGS_SIT_AT = prepare_rig.LEGS_OUT

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
TOES_SIT_AT = prepare_rig.TOE_OUT
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
    """Moves a bone along an axis of the armature, ON TOP of where it already is.

    It used to ASSIGN, which is fine while one caller owns a bone and a silent bug the
    moment two do - the pelvis now carries both its side-to-side sway and a forward
    offset, and assignment would have kept only whichever ran last. Same trap that
    `turn_further` exists for on the rotation side.
    """
    posed = rig.pose.bones.get(bone)
    if posed is None:
        return
    rest = posed.bone.matrix_local.to_3x3()
    posed.location = posed.location + (
        rest.inverted() @ (mathutils.Vector(axis) * along)
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
                  back: float, elbow_held: float, elbow_swing: float, facing) -> None:
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
    # forward, and FOREARMS_TUCK_IN toward the midline (`across * hand` points away from
    # the body, so inward is its negation). Rotating `upper` about `upper x heads` carries
    # it toward `heads`, which is the fold wanted. A constant axis cannot do this because
    # it does not know where the upper arm ended up.
    upper = (posed.matrix.to_3x3() @ mathutils.Vector((0.0, 1.0, 0.0))).normalized()
    tuck = math.radians(FOREARMS_TUCK_IN)
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


def idle_breathing(rig, facing):
    """Authors the idle: standing easy, breathing, weight settling side to side.

    Authored rather than inherited, for two reasons that arrived together. The shipped
    idle is written in the OLD rest basis, so on the prepared rig every quaternion in
    it means something else - a clip cannot be copied across a bind change, only
    retargeted. And it was reported wrong anyway ("the idle pose is still wrong", "the
    beginning part of the idle is terrible"), so there is nothing worth retargeting.

    One breath and one settle per loop, four seconds each. The settle is on the WAIST
    and not the hips: the hips carry the legs, so swaying there slides planted feet.
    """
    span = 96  # four seconds at 24 fps: a resting breath, about fifteen a minute
    action = bpy.data.actions.new("idle")
    if rig.animation_data is None:
        rig.animation_data_create()
    rig.animation_data.action = action
    for frame in range(1, span + 2):
        phase = ((frame - 1) % span) / span
        breath = math.sin(2.0 * math.pi * phase)
        settle = math.sin(2.0 * math.pi * phase + math.pi / 3.0)
        rest(rig)
        for side, hand in (("L", 1.0), ("R", -1.0)):
            swing(rig, f"{side}_Upperarm", 3.0, REACHES_FORWARD)
            swing(rig, f"{side}_Forearm", 8.0, FOLDS_THE_ELBOW)
            # About the FOREARM's own axis, as swing_the_arm does - a fixed world axis
            # stops being pronation and becomes a wrist twist the moment the elbow folds.
            # This was the second copy of that bug; the idle only hid it because its elbow
            # barely bends.
            bpy.context.view_layer.update()
            along = (
                rig.pose.bones[f"{side}_Forearm"].matrix.to_3x3()
                @ mathutils.Vector((0.0, 1.0, 0.0))
            ).normalized()
            swing(rig, f"{side}_Hand", PALM_IN * hand, axis=along)
            # The breath lifts the arms a little with the ribs.
            turn_further(
                rig, f"{side}_Upperarm",
                (ARM_HANGS_AT + 0.6 * breath - prepare_rig.ARMS_OUT) * hand,
                (1.0, 0.0, 0.0),
            )
        swing(rig, "Spine01", 1.0 * breath, LEANS_THE_TORSO_FORWARD)
        swing(rig, "Spine02", 0.7 * math.sin(2.0 * math.pi * phase - 0.5),
              LEANS_THE_TORSO_FORWARD)
        # The head stays level while the chest moves under it.
        swing(rig, "NeckTwist01", -0.8 * breath, LEANS_THE_TORSO_FORWARD)
        turn_further(rig, "Waist", 0.8 * settle, (1.0, 0.0, 0.0))
        key(rig, frame, DRIVEN)
    turned = make_it_linear(action)
    print(f"  idle: {span + 1} frames, {turned} keys linear, one breath a loop")
    return action


def gait(rig, mesh, feet, ground: float, name: str, leg, span: int, contact: float,
         swing_lift: float, swing_shape: float, lands_ahead: float,
         arm_forward: float, arm_back: float, elbow_held: float, elbow_swing: float,
         lean: float, share: float, sinks: float, leads: float, bound: float,
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
    deepest = max(
        min(reach_ceiling(step / span) for step in range(span)),
        -sinks,
    )
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
            swing(rig, "Spine02",
                  twist * math.cos(
                      2.0 * math.pi
                      * (phase - 0.5 - ARM_LAG - CHEST_LAGS_THE_HIPS)
                  ),
                  (0.0, 0.0, 1.0))
        # The head: a bob that FOLLOWS the body's, lagged again - see HEAD_BOBS. The head
        # is what an eye tracks, so it is the cheapest liveliness available.
        head = rig.pose.bones.get("Head")
        if head is not None and HEAD_BOBS:
            axes = head.bone.matrix_local.to_3x3().inverted()
            behind = phase - CHEST_LAGS_THE_HIPS - HEAD_LAGS_THE_CHEST
            # The DIFFERENCE between where the body is and where the head has got to,
            # not a bob of its own. A lagged wave ADDED to the body's just rides along
            # with it - measured, the head went on peaking at frame 11 with the hip
            # however far it was delayed, because 1 cm of extra bob cannot move the peak
            # of a 5.8 cm ride. Subtracting the body's own phase and adding the lagged
            # one is what a follow-through actually is: the head is held back while the
            # body rises, and catches up after. Zero-mean, so it changes when the head
            # arrives rather than how high it goes.
            # Set further down, once the hip's own vertical is known - the head has to
            # DAMP that rather than be authored blind against it. See `head_rides_less`.

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
                elbow_swing, facing,
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
                + mathutils.Vector((0.0, 0.0, max(rides, -sinks)))
            )

        # # The head rides LESS than the hips
        #
        # Reported: "the head bob is extreme". Measured, head travel was 14.74 cm against a
        # hip rise of 11.6 - the head was AMPLIFYING the hip, when the head is the most
        # stabilised part of a running body. A runner's gaze holds steady while the pelvis
        # does the work; the spine and neck spend the difference. So the head gets a
        # negative gain on the hip's own vertical, and it has to be applied here rather than
        # up with the rest of the head work, because `rides` does not exist yet up there.
        # Authoring a damping term without the thing it damps is how it came out amplifying.
        #
        # The zero-mean follow-through stays: it changes WHEN the head arrives, which is the
        # overlapping-action part, and is a different job from how far it travels.
        if head is not None:
            # `axes` is NOT the right basis here, and using it is why raising
            # HEAD_RIDES_LESS from 0.4 to 0.85 moved head travel 12.08 cm to 11.76 - a
            # knob doing a twentieth of what it should. A pose bone's `location` is in its
            # OWN rest space, and `axes` was built for Root; applied to the Head it pushes
            # in some arbitrary direction that happens to be nearly horizontal. The lift
            # has to be expressed in the head's own basis, so build it from world up and
            # take it there.
            amount = (
                HEAD_BOBS * (
                    math.cos(4.0 * math.pi * (behind - share / 2.0))
                    - math.cos(4.0 * math.pi * (phase - share / 2.0))
                )
                - HEAD_RIDES_LESS * max(rides, -sinks)
            )
            skyward = rig.matrix_world.to_3x3().inverted() @ mathutils.Vector(
                (0.0, 0.0, 1.0)
            )
            head.location = (
                head.bone.matrix_local.to_3x3().inverted()
                @ (skyward.normalized() * amount)
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

    closed = close_the_loop(baked, 1, span + 1)
    turned = make_it_linear(baked)
    baked.name = name
    print(
        f"  {name}: {span + 1} frames, {turned} keys linear, {closed} closed at the "
        f"seam, {dropped} run-up keys dropped, legs solved by IK; "
        f"the worst the shoe missed the floor by was {clamped * 170.0:.2f} cm"
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
    # # ONE source, always, and it is the prepared rig
    #
    # This used to scan the project root for any .glb with "ranger" in the name and, if it
    # found one, read the RAW export instead - the reasoning being that preset clips have
    # to be read against the file they were authored in. The effect was that dropping a
    # walk export into the folder silently threw away every rig repair: the mirroring, the
    # bind pose, the welded mesh, all of it, with nothing in the log to say so.
    #
    # Nothing here needs the raw export, and reading it is only ever a mistake: its
    # rest pose is the crouch this rig was repaired out of.
    source = os.path.join(here, PREPARED_RIG)
    out = os.path.join(root, "assets", "models", "person_ranger.glb")
    if not os.path.isfile(source):
        raise SystemExit(
            f"the prepared rig is not there: {source}\n"
            "run dev/art/prepare_rig.py first - see dev/art/animate_ranger.sh"
        )
    print(f"reading {PREPARED_RIG}, the prepared rig")

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
    # The cross-limb weights are repaired in prepare_rig.py now, BEFORE the bind pose
    # is baked - baking first froze a half-dragged pocket into the mesh for good.
    # Running the same repair here again is the CHECK: on a clean mesh it strips
    # nothing, counts what is left, and refuses on its own if anything is.
    unfuse.unfuse_the_gloves_from_the_pockets(rig, body)

    # The generator's own walk and run, if they were exported alongside the idle.
    #
    # These are preset animations from the tool that rigged the character, which means
    # they are made by people who could see the result - and the whole of the authoring
    # below exists only because the first export happened to have the IDLE preset
    # selected and nobody asked what else was on offer.
    print("looking for the generator's own gaits:")
    # NO PRESET-CLIP GATHERING. There was a step here that imported every other .glb
    # in the project root looking for the generator's own walk and run, and it could
    # never succeed: it accepts a sibling only if its rest pose matches the primary's to
    # within half a millimetre, and the primary is the PREPARED rig, whose rest pose was
    # deliberately mirrored, straightened and A-posed away from every raw export. So it
    # re-imported 3.5 MB on every build to reject it - a check that can only ever fail.
    #
    # Preset clips are still worth having one day, and the honest way is a WORLD-SPACE
    # retarget: sample each bone's world matrix per frame from the source rig and
    # replay it against this rest pose. Copying quaternions cannot work across a bind
    # change, which is what the rejected comparison was correctly refusing to do.

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
    rests_at = min(sole_of(rig, body, feet, side) for side in "LR")

    # # The floor is z=0, and it is NOT wherever this model's sole happens to rest
    #
    # This used to take the rest sole as the ground, which sounds careful and is a datum
    # error: the character as delivered stands 5.7 cm INTO z=0, so every clip was then
    # solved onto a floor 5.7 cm underground and reproduced that faithfully. Measured,
    # the penetration was a constant -5.7 cm on 25 of 25 walk frames, 17 of 17 run and
    # 17 of 17 sprint - and a constant error is a datum, never a solver that has not
    # converged. Eight passes of iteration were spent on a number that was never going
    # to move.
    #
    # Zero is the floor because zero is what Blender's ground plane and the game's
    # terrain both use. How far the model's own sole sits from it is worth printing,
    # since it is the amount the character would sink if anything trusted the rest pose.
    ground = 0.0
    print(
        f"the floor is z=0; this model's sole rests at {rests_at * 170.0:+.1f} cm, so "
        f"it would sink that far if the rest pose were taken as the ground"
    )

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
    # The idle first: it is the state every other clip blends from.
    if idle is None:
        idle_breathing(rig, facing).use_fake_user = True

    gait(
            rig, body, feet, ground, "walk", WALK_LEG, 24, WALK_CONTACT, WALK_SWING_LIFT, WALK_SWING_SHAPE, WALK_LANDS_AHEAD,
            ARM_FORWARD, ARM_BACK,
            # 2 degrees of trunk lean, not 0. A walk is upright, but dead plumb
            # reads as being carried along rather than walking; a couple of degrees is
            # what a person leans to actually go somewhere.
            ELBOW_HELD, ELBOW_SWING, WALK_LEAN, WALK_SHARE, WALK_SINKS, WALK_LEADS, WALK_BOUND, WALK_TWIST, WALK_PELVIS, facing,
        ).use_fake_user = True
    gait(
            # An EVEN span, always: a cycle is two identical steps, so the half
            # cycle must land exactly on a frame. 15 was tried for the cadence and the
            # verifier refused it - the two halves sample different phases and the
            # hips disagree with themselves by 21%, which is a limp.
            rig, body, feet, ground, "run", RUN_LEG, 24, RUN_CONTACT, RUN_SWING_LIFT, RUN_SWING_SHAPE, RUN_LANDS_AHEAD,
            RUN_ARM_FORWARD, RUN_ARM_BACK,
            RUN_ELBOW_HELD, RUN_ELBOW_SWING, RUN_LEAN, RUN_SHARE, RUN_SINKS, RUN_LEADS, RUN_BOUND, RUN_TWIST, RUN_PELVIS, facing,
        ).use_fake_user = True
    gait(
            # 14 frames, not 16: a sprint cycle is ~0.58 s where a run's is ~0.67.
            rig, body, feet, ground, "sprint", SPRINT_LEG, 24, SPRINT_CONTACT, SPRINT_SWING_LIFT, SPRINT_SWING_SHAPE, SPRINT_LANDS_AHEAD,
            SPRINT_ARM_FORWARD, SPRINT_ARM_BACK, SPRINT_ELBOW_HELD, SPRINT_ELBOW_SWING, SPRINT_LEAN, SPRINT_SHARE,
            SPRINT_SINKS, SPRINT_LEADS, SPRINT_BOUND, SPRINT_TWIST, SPRINT_PELVIS, facing,
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
