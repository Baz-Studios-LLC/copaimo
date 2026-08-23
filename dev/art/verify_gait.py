"""Refuses a gait clip whose limbs bend or reach the wrong way, and scores the rest.

    blender --background --python dev/art/verify_gait.py -- <glb> <clip> [<clip>..]

# Why this exists

Three separate attempts shipped a walk with the knees bending backwards and the arms
swinging with the legs instead of against them, and every one was found by the person
playing the game. The cause was always a SIGN: `swing` takes degrees about an
armature axis, and whether positive is forward is a fact about the rig that was
reasoned about instead of measured. Reasoning got it exactly inverted, and inverted
twice reads as "the limbs are backwards" without saying which limb or which way.

# Two kinds of test, kept apart on purpose

**REFUSALS are signs.** A knee either folds forward or it does not, and there is no
amount of it that is acceptable. These fail the export.

**SCORES are amplitudes.** How far the hips rise, how much the planted foot slides,
how far the arms lag the legs — each has a target from the reference brief and a
tolerance, and none has a value that is simply wrong. These are measured and printed
as a `SCORE` line so one build can be compared against another rather than against a
threshold somebody guessed.

The split matters because a gate set to wherever the code happens to sit is
decoration, and a gate set to an aspiration blocks every candidate including the
good ones.

# The trap this file exists to avoid

Opposition survives a global sign flip. Flip both arms AND both legs and the arms
still oppose the legs, so a walk can be running entirely backwards and pass an
opposition test. The reference brief names this explicitly. So opposition is checked
alongside the ABSOLUTE direction of the lead foot in the same frame:

* the leading foot lands heel-down, TOES UP, while the trailing one is up on its
  toes,
* and the leading knee is straighter than the trailing one.

Both are asymmetries between the two legs at a contact pose, and both reverse under a
sign flip. That is what makes them able to catch one.
"""

import json
import math
import sys

import bpy
import mathutils

# How much of the legs fore-aft travel the hands must cover for the swing to read as
# a swing. The amplitude was once cut to six degrees as a workaround and stayed
# there, carrying the hands 9% of what the feet did.
ARMS_CARRY_AT_LEAST = 0.25

# How far off a straight line a joint must sit before its direction is called. Below
# this the limb is straight and its bend direction is not a fact about anything.
A_REAL_BEND = 0.004

# How alike the two halves of a cycle must bob. Below this it limps.
LIMPS_BELOW = 0.80

# How far the second half of a cycle may differ from the first, as a share of the
# bob. A cycle is two identical steps, so anything much above nought is a limp.
A_DIFFERENT_STEP = 0.12

# The most a planted foot may point away from the line of travel, in degrees.
#
# People walk with 7 to 10 degrees of toe-out. This model rests at 18.5 apiece, which
# is 37 between the two feet and reads as splayed at a glance. 14 leaves room for a
# stylised stance without letting it back to where it was.
TOES_OUT_AT_MOST = 14.0

# How far forward a running trunk must be flexed, and how nearly upright a walking one
# must stay. Both in degrees from the model's OWN resting posture.
#
# Four degrees is the bottom of the measured human range for running, which runs 4 to
# 12 with the most economical near 6. Game guidance quotes 15 to 30 for sprints, which
# is a two-to-four-times push and makes a character read as permanently accelerating.
LEANS_FORWARD_AT_LEAST = 4.0

# A walk may lean FORWARD up to this, and must never lean back at all. It was a
# symmetric 3 degrees either way, which is right for an unloaded stroll and wrong for
# this character: he carries a backpack, and its mass sits behind him, so an upright
# trunk reads as reclining - reported twice as "leaning back" while the number said
# +1.6 forward. A loaded walker leans into the load. Backwards still refuses at once,
# since that was the original fault this guard was written for.
WALK_MAY_LEAN_FORWARD = 8.0
WALK_MAY_LEAN_BACK = 0.5

# How far a thigh must extend BEHIND vertical somewhere in the cycle, in degrees.
# Walking reaches 10 to 20; this asks for most of the bottom of that, because a leg
# that never gets behind the body is what makes the hips look pushed out in front.
THIGH_REACHES_BACK = 12.0

# How far past a SMOOTH bob's own steepest step the hips may move between two frames.
#
# An absolute limit cannot work here and was tried: a sprint's hips complete two rises
# in 0.58 s, so its per-frame steps are legitimately about three times a walk's, and a
# figure that suited the walk refused the sprint at once. What is constant across gaits
# is the SHAPE - a cosine of amplitude A over a span N steps at most A * 4pi / N per
# frame - so the measured worst step is compared with that instead. A step function has
# one step far larger than its own amplitude implies, which is what this catches: the
# lurch that started this measured 3.8x, and a healthy walk measures 1.3.
HIP_STEPS_PAST_SMOOTH = 2.0

# How nearly the two steps of a cycle must match, with the legs swapped, each in its
# own unit. A limp that reads as one shows up far above these: the one this codebase
# actually shipped had the two halves bobbing 4.57 cm and 2.95, a 1.6 cm gap in a
# single quantity. The current clips sit at 1.09 cm and 4.05 deg, which is this asset's
# own left-right mesh asymmetry coming through the foot landmarks.
ANKLES_SWAP_WITHIN = 0.02      # model units, about 3.4 cm
THIGH_SWAP_WITHIN = 6.0        # degrees

# How nearly the first frame must equal the last. They are the same instant.
SEAM_CLOSES_WITHIN = 0.002

# The summed stance shares below which a gait has a genuine flight phase. One exactly
# would mean the two feet hand over with no overlap and no gap.
ALWAYS_ON_THE_GROUND = 1.0

# How near its lowest a sole must be to count as still on the ground, in model
# units. A foot rolls through stance rather than sitting at one height, so a contact
# has to be looked for over a window rather than at a single frame.
STILL_DOWN = 0.006

# How far off horizontal a foot must point for the tilt to be deliberate, in degrees.
# Below this the ankle is level and neither heel-strike nor toe-off is being said.
A_REAL_PITCH = 4.0


def argv():
    return sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []


def turning_points(series):
    """Where a looped series turns around, as (index, high|low) pairs.

    The series is a CYCLE, so the last sample repeats the first and the neighbours
    wrap. Counting turns is how the bob's frequency gets checked: a walk bobs twice
    per cycle, and once per cycle reads as a limp.
    """
    n = len(series) - 1 if len(series) > 1 and series[0] == series[-1] else len(series)
    found = []
    for i in range(n):
        here, before, after = series[i], series[(i - 1) % n], series[(i + 1) % n]
        if here > before and here >= after:
            found.append((i, "high"))
        elif here < before and here <= after:
            found.append((i, "low"))
    return found


def main() -> None:
    args = argv()
    if len(args) < 2:
        raise SystemExit("need <glb> <clip> [<clip>...]")
    # Each clip is given as `name:stance`, where stance is how many of the eight
    # poses each leg was authored on the ground. Five is a walk, three a jog, two a
    # sprint.
    #
    # # Why this is declared and not measured
    #
    # It was measured first, as the share of frames each foot spends within a few
    # millimetres of its lowest. On the walk that came out at 0.04 and 0.17 where the
    # authored answer is 0.62 - not because the authoring is wrong but because a keyed
    # FK leg has nothing holding its foot down BETWEEN keys, and the planted foot
    # drifts millimetres in the in-betweens. Foot IK is what fixes that, and it is not
    # built yet.
    #
    # So the duty factor is stated by the thing that knows it. The drift is still
    # reported, as `planted_foot_slides`, because it is a real fault worth watching -
    # it is just not a sound basis for deciding whether a clip is a walk.
    source = args[0]
    clips = {}
    for given in args[1:]:
        name, _, stance = given.partition(":")
        clips[name] = int(stance) if stance else 5

    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=source)
    rig = next(o for o in bpy.data.objects if o.type == "ARMATURE")
    if rig.animation_data:
        for track in rig.animation_data.nla_tracks:
            track.mute = True
    else:
        rig.animation_data_create()

    def head(bone):
        return rig.matrix_world @ rig.pose.bones[bone].head

    def tail(bone):
        return rig.matrix_world @ rig.pose.bones[bone].tail

    # Rest first: forward comes off the model's own toe rather than being assumed,
    # and the feet's rest pitch is the baseline heel-strike is measured against.
    rig.animation_data.action = None
    for posed in rig.pose.bones:
        posed.rotation_mode = "QUATERNION"
        posed.rotation_quaternion = (1.0, 0.0, 0.0, 0.0)
        posed.location = (0.0, 0.0, 0.0)
    rig.data.update_tag()
    bpy.context.view_layer.update()

    # Forward, from BOTH feet averaged.
    #
    # It used to come off the left foot alone, and that foot rests eighteen degrees
    # toed-out - so the reference axis was itself skewed by half the angle between the
    # feet. Every fore-aft number was measured against it: the left foot then read as
    # nearly straight because it DEFINED the axis, and the right read as 37 degrees
    # out, which is simply the angle between the two feet. It also cost about 5% on
    # every contact length and stride, by the cosine of the skew.
    #
    # Averaging the two cancels whatever toe-out the rest pose has, because it is
    # symmetric, and leaves the direction the body actually travels.
    both = mathutils.Vector((0.0, 0.0, 0.0))
    for side in "LR":
        span = tail(f"{side}_ToeBase") - head(f"{side}_Foot")
        both += mathutils.Vector((span.x, span.y, 0.0)).normalized()
    forward = both.normalized()

    def pitch(side):
        """How far a foot points up from HORIZONTAL, in degrees.

        In degrees from horizontal, which is the unit the authoring states these in, and
        NOT as a sine ratio against the rest pose. Measured against rest it was
        unreadable: this character's rest foot line already dips 31 degrees, so a
        correct 32-degree toe-off came out as -0.018 against a -0.02 threshold and was
        refused. The animation was hitting every pose exactly - 12 asked and 12 got, -32
        asked and -33 got - and the checker was the thing that was wrong.
        """
        span = tail(f"{side}_ToeBase") - head(f"{side}_Foot")
        flat = mathutils.Vector((span.x, span.y, 0.0))
        return math.degrees(math.atan2(span.z, max(1e-6, flat.length)))

    # # Zero is the BIND, and that is a change worth explaining
    #
    # This measured from horizontal and treated that as the zero, because back then the
    # foot bones ran horizontally through the shoe, so bind and horizontal were the same
    # thing. They are not any more: the ball joint moved to the shoe's real flex point
    # near the sole, so the bind's ankle-to-toe line now dips about ten degrees while
    # the SOLE is still flat on the floor. Against horizontal, a correct heel strike
    # then reads as roughly zero and gets refused.
    #
    # The bind is the pose in which the sole is flat - that is what the pipeline
    # guarantees, soles at 0.0000 cm - so it is the honest zero for "how far is this
    # foot tilted off the floor". The earlier note about a rest pose dipping 31 degrees
    # was true of the delivered CROUCH, which is no longer the bind.
    holding = rig.animation_data.action if rig.animation_data else None
    if rig.animation_data:
        rig.animation_data.action = None
    for posed in rig.pose.bones:
        posed.rotation_mode = "QUATERNION"
        posed.rotation_quaternion = (1.0, 0.0, 0.0, 0.0)
        posed.location = (0.0, 0.0, 0.0)
    bpy.context.view_layer.update()
    at_rest = {side: pitch(side) for side in "LR"}
    if rig.animation_data:
        rig.animation_data.action = holding
    bpy.context.view_layer.update()
    print(
        "feet at rest point "
        + ", ".join(f"{s} {at_rest[s]:+.1f} deg from horizontal (this is the zero)"
                    for s in "LR")
    )

    def toe_out(side):
        """Degrees a foot points away from the line of travel. Positive is flared.

        Only meaningful while the foot is roughly flat. Fold the knee a hundred
        degrees and the foot points backwards and upwards, its shadow on the ground
        shrinks to nothing, and the yaw of that shadow becomes noise - which is how a
        first attempt at this reported 146 degrees of flare on a foot that was simply
        in the air. So the caller checks the foot is DOWN before believing it.
        """
        span = tail(f"{side}_ToeBase") - head(f"{side}_Foot")
        flat = mathutils.Vector((span.x, span.y, 0.0))
        if flat.length < 1e-6:
            return 0.0
        flat.normalize()
        across = mathutils.Vector((-forward.y, forward.x, 0.0))
        yaw = math.degrees(math.atan2(flat.dot(across), flat.dot(forward)))
        # +across is the model's left, so positive yaw flares the left foot and the
        # right foot flares at negative. Flipped so that positive always means OUT.
        return yaw if side == "L" else -yaw

    def trunk():
        """How far the torso is flexed forward from vertical, in degrees.

        In degrees, and against the model's OWN rest posture, because both matter.
        Degrees so the answer can be held against the measured human range of 4 to 12;
        against rest because this figure stands with its chest a little behind its
        hips to begin with, and a walk is meant to leave that alone rather than
        correct it. An absolute threshold refused a run that had leant forward by a
        perfectly good twenty degrees of chest travel, purely because it started from
        behind.
        """
        along = head("Spine02") - head("Hip")
        return math.degrees(math.atan2(along.dot(forward), max(1e-6, along.z)))

    def under(side):
        return (
            head(f"{side}_Foot").z,
            head(f"{side}_ToeBase").z,
            tail(f"{side}_ToeBase").z,
        )

    # How high each of those three points sits above the ground in the REST pose.
    #
    # Without this the "sole" is whichever bone happens to poke lowest, and the ankle
    # sits higher off the ground than the toe does — so a foot with the toe pointed
    # measures LOWER than a flat one, and the frame that looked most like a contact
    # was actually toe-off, where the ankle is plantarflexed thirty degrees. Every
    # heel-strike check was then being applied to the wrong pose and every one failed.
    above = {side: tuple(z - min(under(side)) for z in under(side)) for side in "LR"}

    def sole(side):
        return min(z - high for z, high in zip(under(side), above[side]))
    trunk_at_rest = trunk()
    print(
        f"forward is ({forward.x:.3f}, {forward.y:.3f}, {forward.z:.3f}); "
        f"feet rest at pitch {at_rest['L']:+.3f}/{at_rest['R']:+.3f}; "
        f"the torso rests {trunk_at_rest:+.1f} deg from vertical"
    )

    def lead(bone):
        return (head(bone) - head("Hip")).dot(forward)

    def off_chord(top, joint, bottom):
        """How far the middle joint sits forward of the line from top to bottom."""
        a, b, c = head(top), head(joint), head(bottom)
        span = c - a
        if span.length < 1e-6:
            return 0.0
        along = span.normalized()
        return ((b - a) - along * (b - a).dot(along)).dot(forward)

    def folded(side):
        """How far a knee is from straight, as a fraction of the leg's length."""
        thigh = head(f"{side}_Thigh")
        knee = head(f"{side}_Calf")
        ankle = head(f"{side}_Foot")
        straight = (thigh - knee).length + (knee - ankle).length
        return 0.0 if straight < 1e-9 else 1.0 - (thigh - ankle).length / straight

    refused, scored = [], {}
    for name, stance in clips.items():
        action = bpy.data.actions.get(name)
        if action is None:
            refused.append(f"{name}: no such clip in the file")
            continue
        rig.animation_data.action = action
        low, high = (int(v) for v in action.frame_range)

        frames = []
        for frame in range(low, high + 1):
            bpy.context.scene.frame_set(frame)
            frames.append(
                {
                    "frame": frame,
                    "hip": head("Hip").z,
                    # How far the chest sits in front of the hips, which is the lean.
                    "chest": trunk() - trunk_at_rest,
                    "ground": min(tail(f"{s}_ToeBase").z for s in "LR"),
                    "along": {s: lead(f"{s}_Foot") for s in "LR"},
                    "sole": {s: sole(s) for s in "LR"},
                    "toes": {s: toe_out(s) for s in "LR"},
                    "legs": lead("L_Foot") - lead("R_Foot"),
                    "arms": lead("L_Hand") - lead("R_Hand"),
                    "pitch": {s: pitch(s) - at_rest[s] for s in "LR"},
                    "folded": {s: folded(s) for s in "LR"},
                    # Where the hips sit, and how far each thigh swings. Both were
                    # measured by hand for a long time before they earned refusals
                    # below; see `the_body_is_over_its_feet` and `thighs_swing_both_ways`.
                    "hipfwd": lead("Hip"),
                    "ankles": {s: lead(f"{s}_Foot") for s in "LR"},
                    "thigh": {
                        s: math.degrees(math.atan2(
                            lead(f"{s}_Calf") - lead(f"{s}_Thigh"),
                            max(1e-9, head(f"{s}_Thigh").z - head(f"{s}_Calf").z),
                        ))
                        for s in "LR"
                    },
                    "knees": {
                        s: off_chord(f"{s}_Thigh", f"{s}_Calf", f"{s}_Foot") for s in "LR"
                    },
                    "elbows": {
                        s: off_chord(f"{s}_Upperarm", f"{s}_Forearm", f"{s}_Hand")
                        for s in "LR"
                    },
                }
            )

        # How many frames a CYCLE is, which is not how many frames were sampled: the
        # last frame repeats the first so the clip loops, so a 25-frame clip is a
        # 24-frame cycle.
        #
        # This used to decide that by asking whether frame 1 and the last frame were
        # EXACTLY equal, as floats. They are equal in intent and differ in the last bit,
        # so it took a 24-frame cycle for a 25-frame one - and every modulo wrap after
        # that was off by one. The visible cost was `half_cycle_drift` reporting 25% on
        # hips that measure 0.0 cm apart, which sent a real morning of work chasing a
        # limp that was not there. The clip's own frame range says it exactly.
        span = int(round(high - low)) or len(frames)

        # --- Which frame is a contact, found by the FOOT rather than by the split.
        #
        # This used to take the frame where the legs are widest apart. That is a fair
        # description of a walk's contact and a wrong one for a sprint, where the
        # widest split falls just after toe-off, in mid-flight, with both feet off
        # the ground. Checked there, the sprint was refused for swinging its arms
        # with its legs on the strength of a left-hand lead of +0.015 - which is
        # noise, because mid-flight is exactly where the arms cross.
        #
        # Contact is the frame the clip was AUTHORED to land on, stated by the caller
        # rather than inferred from the geometry.
        #
        # # Three attempts at inferring it, and why the third failed too
        #
        # First the widest leg split (above). Then the lowest sole, which found
        # TOE-OFF instead, because a plantarflexed toe reaches lower than a flat foot.
        # Then the lowest sole that is also furthest forward - right in principle and
        # still the wrong frame, because a keyed FK leg has nothing holding its foot up
        # BETWEEN keys. The in-betweens dip below the keys, and in a flying clip the
        # deepest dip is under the leg reaching forward to land, not under the one that
        # has landed.
        #
        # Inferring it was never necessary. An eight-pose cycle lands the right foot at
        # pose 0 and the left half a cycle later, and the authoring knows that, so it
        # says so - the same way it states the stance count. Every check that depends
        # on "at contact" then looks at the pose the clip MEANT rather than at whichever
        # in-between happens to sag furthest.
        landing = {"R": frames[0], "L": frames[min(span // 2, len(frames) - 1)]}

        # Is this a walk or a run? Not a label — a measurement. A walk always has a
        # foot down; a run is airborne for part of its cycle, and the formal name for
        # the difference is the duty factor.
        #
        # It decides which rules apply. A walk lands HEEL first, which is the check
        # that catches a reversed cycle. A run lands on the FOREFOOT with the knee
        # already flexed, so demanding a heel strike of it is demanding a fault: both
        # the run and the sprint were refused for exactly that before this told them
        # apart.
        # Is this a walk or a run? Not a label but a measurement, and specifically the
        # DUTY FACTOR: what share of the cycle each foot spends on the ground. Above a
        # half per foot the two overlap and something is always down, which is a walk.
        # Below it there are moments with neither down, which is a run.
        #
        # It decides which rules apply. A walk lands HEEL first, and that is the check
        # which catches a reversed cycle. A run lands on the FOREFOOT with the knee
        # already flexed, so demanding a heel strike of one is demanding a fault, and
        # that is what refused both the run and the sprint before this told them apart.
        #
        # # Why the shares are summed rather than the airborne frames counted
        #
        # Counting frames where both soles sit above the floor called the WALK a run,
        # with fourteen airborne frames of twenty-four. Those frames are not flight:
        # they are the planted foot drifting upward BETWEEN keys, because a keyed FK
        # leg has nothing holding its foot down in the in-betweens. Summing each foot's
        # own stance share is immune to that, since a foot that drifts is still nearest
        # the ground for the same share of the cycle.
        #
        # It is also the quantity that sets the stride: contact length divided by the
        # stance fraction is how far a cycle carries the body, so it is worth reporting
        # whether or not anything is refused on it.
        # Two stance windows of `stance` poses each, out of eight. They overlap when
        # stance is above four, which is what makes a gait a walk.
        duty = 2.0 * stance / 8.0
        flies = duty < ALWAYS_ON_THE_GROUND

        # The right foot is authored down over poses 0 to stance-1, which on a clip of
        # `span` frames with eight poses puts that window between these two frames.
        planted_from = frames[0]["frame"]
        planted_to = frames[0]["frame"] + round((stance - 1) * span / 8)
        window = [f for f in frames if planted_from <= f["frame"] <= planted_to]
        contact_travel = (
            window[0]["along"]["R"] - window[-1]["along"]["R"] if len(window) > 1 else 0.0
        )
        # Of the two authored landings, the one whose foot is further forward is the
        # front foot of that contact.
        front = max("LR", key=lambda side: landing[side]["along"][side])
        back = "R" if front == "L" else "L"
        contact = landing[front]

        # The lean, averaged over the cycle: a run leans FORWARD and a walk stands up.
        leans = sum(f["chest"] for f in frames[:span]) / max(1, span)

        legs_travel = max(f["legs"] for f in frames) - min(f["legs"] for f in frames)
        arms_travel = max(f["arms"] for f in frames) - min(f["arms"] for f in frames)
        # How much a planted foot SLIDES.
        #
        # # Sliding is horizontal, and the first version of this measured vertical
        #
        # It took the spread of the lowest toe's height, which a walk with any foot
        # roll at all varies on purpose: at heel strike the toe is up. So the metric
        # punished exactly the thing that makes a step read as a step.
        #
        # A planted foot is still on the ground while the body moves over it, so
        # relative to the hips it travels BACKWARD at the body's speed - which is to
        # say linearly. Slide is therefore the departure from a straight line over
        # the frames the foot is down, and it is scale-free: expressed as a share of
        # how far the foot travels in that time.
        def slide(side):
            path = [f["along"][side] for f in frames]
            # Over the window that foot is AUTHORED to be down, which for the left is
            # the same window half a cycle along. Measuring over a fixed half-cycle
            # instead put swing frames inside the window, and a swinging foot is
            # supposed to depart from a straight line - so the metric punished the
            # gaits with the shortest stance hardest, reporting 1.34 for the sprint.
            downs = round((stance - 1) * span / 8) + 1
            start = 0 if side == "R" else span // 2
            walked = [path[i % span] for i in range(start, start + downs)]
            if len(walked) < 3:
                return 0.0
            travel = walked[0] - walked[-1]
            if abs(travel) < 1e-6:
                return 1.0
            worst = 0.0
            for step, here in enumerate(walked):
                straight = walked[0] - travel * step / (len(walked) - 1)
                worst = max(worst, abs(here - straight))
            return worst / abs(travel)

        rides = max(slide("L"), slide("R"))
        bobs = max(f["hip"] for f in frames) - min(f["hip"] for f in frames)
        highs = [i for i, kind in turning_points([f["hip"] for f in frames]) if kind == "high"]

        # --- Whether the two halves of the cycle match, which is whether it limps.
        #
        # A cycle is two steps, and the second is the first with the legs swapped. So
        # the hips must rise by the same amount in each half and peak half a cycle
        # apart. When they do not, every direction still measures correct and the
        # walk still reads as WRONG - which is exactly the report this came from:
        # "the legs and arms do not feel like they are moving correctly, even though
        # they are facing the right way now."
        #
        # Measured on the real clip, the pelvis yaw and obliquity were bobbing one
        # half 4.57 cm and the other 2.95, peaking ten frames apart instead of
        # twelve. Zeroing them made the halves exact, which is what identified them.
        middle = span // 2
        early = [f["hip"] for f in frames[: middle + 1]]
        late = [f["hip"] for f in frames[middle:]]
        rise = (
            min(max(early) - min(early), max(late) - min(late))
            / max(max(early) - min(early), max(late) - min(late))
            if max(early) > min(early) and max(late) > min(late)
            else 0.0
        )
        # --- Whether the bob REPEATS every half cycle, which is the actual invariant.
        #
        # Two earlier versions of this tried to infer it from where the peaks are:
        # first by comparing the argmax of each half, which breaks when a peak lands on
        # the boundary, then by differencing the two detected turning points, which
        # breaks when the top is flat and only one of a plateau gets counted. Both were
        # measuring a proxy.
        #
        # The property itself is simple and needs no peak-finding at all: a cycle is
        # two identical steps, so the hip height at any frame must equal the height
        # half a cycle later. So that is what is measured - the worst disagreement
        # across the cycle, as a share of the bob's own size.
        half = span // 2
        rises = max(f["hip"] for f in frames) - min(f["hip"] for f in frames)
        drift = max(
            abs(frames[i]["hip"] - frames[(i + half) % span]["hip"]) for i in range(span)
        )
        repeats = drift / rises if rises > 1e-9 else 0.0

        # Where the arms peak against where the LEGS peak, as a share of the cycle.
        # The brief puts the arm extremes 8 to 12 per cent behind the legs.
        #
        # Against the legs' own fore-aft extreme, not against the contact frame. In a
        # walk those nearly coincide; in a run the legs are widest apart in mid-flight,
        # a long way after the foot lands, so measuring from contact reported a lag of
        # minus forty-four per cent - the arms appearing to LEAD the legs by half a
        # cycle when nothing was wrong with them.
        leg_peak = frames.index(max(frames, key=lambda f: abs(f["legs"])))
        arm_peak = frames.index(max(frames, key=lambda f: abs(f["arms"])))
        # Measured against ANTI-PHASE, not against zero. The arms oppose the legs,
        # so their extremes are half a cycle from the legs' by construction, and a
        # correct ten per cent lag shows up as sixty per cent against a zero
        # baseline. The lag is what is left after taking the half turn out.
        lag = (((arm_peak - leg_peak) % span) / span - 0.5) if span else 0.0

        scored[name] = {
            "frames": span,
            "legs_travel": round(legs_travel, 4),
            "arms_carry": round(arms_travel / legs_travel, 3) if legs_travel else 0.0,
            "planted_foot_slides": round(rides, 3),
            "hip_rises_cm": round(bobs * 170.0, 2),
            "hip_highs_per_cycle": len(highs),
            "hip_high_at_percent": [round(100.0 * i / span) for i in highs],
            "flies": flies,
            "duty_factor": round(duty, 3),
            # How far a cycle carries the body: contact length over stance fraction.
            # `legs_travel` is the two feet's combined spread, so one foot's contact
            # length is half of it.
            # How far a cycle carries the body, by the one identity that is exact:
            # contact length divided by the stance fraction.
            #
            # The contact length is measured over the window the RIGHT foot is
            # authored to be down - poses 0 to stance-1 - because that is the only
            # stretch where the foot is on the ground and the identity applies. Taking
            # half the two feet's combined spread instead was an approximation, and it
            # disagreed with a line fitted to the whole cycle by 20 to 55%.
            "contact_length_m": round(1.7 * abs(contact_travel), 3),
            "covers_implied_m": round(1.7 * abs(contact_travel) / (stance / 8.0), 3),
            "leans_forward_deg": round(leans, 2),
            # At the contact, where the foot is flat and the number means something.
            "toe_out_deg": round(contact["toes"][front], 1),
            "arm_lag_percent": round(100.0 * lag),
            "arm_lag_wants": "8 to 12",
            "halves_bob_alike": round(rise, 3),
            "half_cycle_drift": round(repeats, 3),
            "contact_frame": contact["frame"],
        }
        print(
            f"\n{name}: {front} foot lands at frame {contact['frame']} "
            f"(sole {contact['sole'][front]:+.4f}), {back} trailing"
        )
        for key, value in scored[name].items():
            print(f"  {key}: {value}")

        # --- Refusals: signs, not amounts.
        if contact["legs"] * contact["arms"] >= 0.0:
            refused.append(
                f"{name}: the arms swing WITH the legs. At frame {contact['frame']} the "
                f"left leg leads by {contact['legs']:+.3f} and the left hand by "
                f"{contact['arms']:+.3f} - same sign. A walk is contralateral."
            )
        if legs_travel > 1e-6 and arms_travel / legs_travel < ARMS_CARRY_AT_LEAST:
            refused.append(
                f"{name}: the arms barely move - {arms_travel:.3f} against the legs "
                f"{legs_travel:.3f}, {arms_travel / legs_travel:.0%} where "
                f"{ARMS_CARRY_AT_LEAST:.0%} is the floor."
            )
        for side in "LR":
            if contact["knees"][side] < -A_REAL_BEND:
                refused.append(
                    f"{name}: the {side} knee sits {-contact['knees'][side]:.3f} BEHIND "
                    f"the hip-to-ankle line, so the leg folds like a birds."
                )
            if contact["elbows"][side] > A_REAL_BEND:
                refused.append(
                    f"{name}: the {side} elbow sits {contact['elbows'][side]:.3f} in "
                    f"FRONT of the shoulder-to-wrist line, so the arm folds backwards."
                )

        # --- And the asymmetries a global sign flip cannot survive.
        if not flies:
            # A walk: heel down in front, up on the toes behind.
            if contact["pitch"][front] < A_REAL_PITCH:
                refused.append(
                    f"{name}: the leading ({front}) foot is not presenting its heel - "
                    f"its pitch is {contact['pitch'][front]:+.3f} off rest where "
                    f"toes-up wants at least +{A_REAL_PITCH}. A front foot landing "
                    f"toe-first is the cleanest tell that the cycle runs backwards."
                )
            if contact["pitch"][back] > -A_REAL_PITCH:
                refused.append(
                    f"{name}: the trailing ({back}) foot is not up on its toes - pitch "
                    f"{contact['pitch'][back]:+.3f} off rest, wanting at most "
                    f"-{A_REAL_PITCH}. A trailing foot pushing off heel-first is the "
                    f"same tell from the other side."
                )
        elif contact["pitch"][front] > A_REAL_PITCH:
            # A run: the leading foot must NOT be presenting a heel. Toes level or
            # pointed is right; toes up means it is landing like a walk.
            refused.append(
                f"{name}: this clip has a flight phase, so it is a run - but its "
                f"leading ({front}) foot lands with the toes {contact['pitch'][front]:+.3f} "
                f"UP, which is a walk's heel strike. A run lands on the forefoot with "
                f"the knee already flexed."
            )
        # --- How far the planted foot points away from the line of travel.
        #
        # Measured at the contact, because that is where the foot is flat: a foot in
        # mid-swing with the knee folded has no meaningful yaw at all.
        if abs(contact["toes"][front]) > TOES_OUT_AT_MOST:
            refused.append(
                f"{name}: the landing ({front}) foot points "
                f"{contact['toes'][front]:+.1f} deg away from the line of travel, "
                f"where a person walks with 7 to 10 and {TOES_OUT_AT_MOST} is the "
                f"most this will pass. Splayed feet read as unnatural before anything "
                f"else does."
            )

        # --- Which way the torso leans.
        #
        # A runner's trunk is flexed FORWARD, between about 4 and 12 degrees. Leaning
        # back while running is not a matter of degree, it is a different action - so
        # this is a refusal, and it is the check that was missing when a clip shipped
        # doing exactly that. The cause was an axis constant measured on limbs, which
        # point down, being applied to a spine, which points up.
        if flies and leans < LEANS_FORWARD_AT_LEAST:
            refused.append(
                f"{name}: this clip flies, so it is a run - and its trunk is "
                f"{leans:+.1f} deg from where the model rests, where a runner's is "
                f"flexed FORWARD by {LEANS_FORWARD_AT_LEAST} or more. Leaning back "
                f"while running is not something people do."
            )
        # --- The body has to be over its feet.
        #
        # Not a matter of degree either: a walker whose hips are in front of BOTH feet
        # is falling, and it is what "the hips are sitting forward of the feet" was
        # seeing. Measured per frame rather than on average, because an average hides
        # exactly the few frames where it happens.
        adrift = [
            f["frame"] for f in frames
            if all(f["ankles"][s] < f["hipfwd"] for s in "LR")
        ]
        if adrift and not flies:
            refused.append(
                f"{name}: on {len(adrift)} frame(s) the hips are in front of BOTH "
                f"feet - {adrift[:6]} - so nothing is under the body. A walk always "
                f"has a foot to fall onto."
            )

        # --- And the thighs have to swing BOTH ways.
        #
        # A thigh that only ever points forward reads as the hips being pushed out in
        # front of the legs, whatever the trunk is doing: measured at one point it
        # ranged +13 to +34 degrees and reached behind vertical on a single frame of
        # twenty-five. Normal walking swings roughly -20 (extended at toe-off) to +30
        # or +35 (flexed at terminal swing), so the FORWARD end needs no guarding - it
        # is the extension that goes missing.
        for side in "LR":
            behind = min(f["thigh"][side] for f in frames)
            if behind > -THIGH_REACHES_BACK:
                refused.append(
                    f"{name}: the {side} thigh only reaches {behind:+.1f} deg from "
                    f"vertical, never extending {THIGH_REACHES_BACK} behind it. A leg "
                    f"that only swings forward puts the hips in front of the body."
                )

        # --- The hips must not lurch.
        #
        # They ride a smooth bob; a step between frames far larger than its neighbours
        # is the jitter that was reported as a bounce. It came from clamping the hip to
        # a reach limit frame by frame, which made its height a step function and
        # dropped it 5.95 cm in one frame.
        steps = [
            abs(frames[i]["hip"] - frames[i - 1]["hip"]) for i in range(1, len(frames))
        ]
        # `bob_height`, NOT `rise`: `rise` belongs to the limp check further down and
        # naming this one the same clobbered it, so a healthy walk was refused with a
        # message contradicting itself - "one half bobs 0.0224 and the other 0.0224, a
        # ratio of 0.02". A shadowed name inside one long function is not a small bug.
        bob_height = max(f["hip"] for f in frames) - min(f["hip"] for f in frames)
        smooth = bob_height / 2.0 * 4.0 * math.pi / max(1, len(frames) - 1)
        if steps and smooth > 0.0 and max(steps) > smooth * HIP_STEPS_PAST_SMOOTH:
            worst = frames[steps.index(max(steps)) + 1]["frame"]
            refused.append(
                f"{name}: the hips move {max(steps) * 100.0:.2f} cm between two frames "
                f"at frame {worst}, which is {max(steps) / smooth:.1f}x the steepest a "
                f"smooth bob of this height ({bob_height * 100.0:.1f} cm over "
                f"{len(frames) - 1} frames) would ever need. A step that size is seen "
                f"as a bounce."
            )

        # --- And the loop has to close.
        #
        # First frame and last are the same instant, so any difference is crossed every
        # cycle. It was seen as an arm jumping: frame 1 is the authoring loop's first
        # pass and settles differently from every frame after it.
        if len(frames) > 1:
            seam = max(
                abs(frames[0][what] - frames[-1][what]) for what in ("hip", "hipfwd")
            )
            for what in ("ankles", "thigh"):
                seam = max(
                    seam,
                    max(abs(frames[0][what][s] - frames[-1][what][s]) for s in "LR"),
                )
            if seam > SEAM_CLOSES_WITHIN:
                refused.append(
                    f"{name}: the first and last frames differ by {seam:.4f}, and they "
                    f"are the same instant of the cycle. Anything above "
                    f"{SEAM_CLOSES_WITHIN} is a discontinuity crossed every loop."
                )

        if not flies and not (
            -WALK_MAY_LEAN_BACK <= leans <= WALK_MAY_LEAN_FORWARD
        ):
            refused.append(
                f"{name}: the trunk is {leans:+.1f} deg off the model's own resting "
                f"posture, and a walk wants between {-WALK_MAY_LEAN_BACK:+.1f} and "
                f"{WALK_MAY_LEAN_FORWARD:+.1f}. Leaning BACK is the fault this checks "
                f"for; a loaded walker leans into the load."
            )

        # --- THE limp check: the two legs must do the same thing, half a cycle apart.
        #
        # A cycle is two steps, and the second is the first with the legs swapped. So
        # the LEFT leg at frame i must match the RIGHT leg at frame i + half, and the
        # arms likewise. That is the invariant, and it is measured on the legs.
        #
        # The three hip-height checks below cannot do this job any more and are kept
        # only for the shapes they still catch. The hip is no longer a consequence of
        # the legs: it is a closed-form cosine of phase, fitted under the reach ceiling,
        # so its two halves are identical whatever the legs are doing and a limp is
        # invisible to it. A guard has to watch something that responds to the fault.
        # In the quantities' OWN units, with a limit for each. A single blended
        # tolerance was tried first and it is not defensible: 0.01 of "scaled
        # disagreement" refused all three clips without saying what was wrong, and the
        # underlying numbers turned out to be 1.09 cm and 4.05 degrees - small, and
        # explained. This asset's two shoes sit about 4 cm differently on their own
        # bones, so the per-side foot landmarks differ and the derived ankle with them.
        # That is the mesh's asymmetry, not a limp in the motion, and it belongs to a
        # sculpting pass. The limits below are where a person starts to see a hitch.
        half = span // 2
        swap = {"ankles": 0.0, "thigh": 0.0}
        where = {"ankles": "", "thigh": ""}
        for i in range(len(frames) - half):
            here, later = frames[i], frames[i + half]
            for what in swap:
                off = max(
                    abs(here[what]["L"] - later[what]["R"]),
                    abs(here[what]["R"] - later[what]["L"]),
                )
                if off > swap[what]:
                    swap[what] = off
                    where[what] = f"frame {here['frame']}"
        scored[name]["ankles_swap_cm"] = round(swap["ankles"] * 100.0, 2)
        scored[name]["thigh_swap_deg"] = round(swap["thigh"], 2)
        if swap["ankles"] > ANKLES_SWAP_WITHIN:
            refused.append(
                f"{name}: it LIMPS. Swapped half a cycle apart the two ankles disagree "
                f"by {swap['ankles'] * 100.0:.2f} cm at {where['ankles']}, where "
                f"{ANKLES_SWAP_WITHIN * 100.0:.1f} cm is the most allowed."
            )
        if swap["thigh"] > THIGH_SWAP_WITHIN:
            refused.append(
                f"{name}: it LIMPS. Swapped half a cycle apart the two thighs disagree "
                f"by {swap['thigh']:.2f} deg at {where['thigh']}, where "
                f"{THIGH_SWAP_WITHIN} is the most allowed."
            )

        # A limp, stated as a refusal rather than a score, because a cycle whose two
        # steps differ is not a matter of degree - it is one step done twice, wrong.
        if rise < LIMPS_BELOW:
            refused.append(
                f"{name}: it LIMPS. One half of the cycle bobs "
                f"{max(early) - min(early):.4f} and the other "
                f"{max(late) - min(late):.4f}, a ratio of {rise:.2f} where "
                f"{LIMPS_BELOW} is the floor. A cycle is two steps and the second is "
                f"the first with the legs swapped, so the halves must match."
            )
        if repeats > A_DIFFERENT_STEP:
            refused.append(
                f"{name}: the hips do not repeat every half cycle - the worst "
                f"disagreement between a frame and its partner half a cycle later is "
                f"{repeats:.0%} of the whole bob, where {A_DIFFERENT_STEP:.0%} is the "
                f"most allowed. A cycle is two identical steps."
            )
        if contact["folded"][front] >= contact["folded"][back]:
            refused.append(
                f"{name}: at contact the leading ({front}) knee is folded "
                f"{contact['folded'][front]:.4f} and the trailing ({back}) one "
                f"{contact['folded'][back]:.4f}. The reaching leg is the STRAIGHT one; "
                f"bent in front and straight behind is the backwards read."
            )

    print("\nSCORE " + json.dumps(scored, sort_keys=True))
    if refused:
        print("\n" + "\n".join(f"REFUSED  {r}" for r in refused))
        raise SystemExit(1)
    print(
        "\nevery clip: arms oppose the legs, knees lead, elbows trail, "
        "the front heel presents and the back foot pushes off."
    )


main()
