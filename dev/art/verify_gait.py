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

# And how far their peaks may drift from half a cycle apart, in frames.
A_FRAME_OR_TWO = 2

# How much a foot's pitch must change from its rest angle to count as deliberate.
# Below this the ankle is neutral and neither heel-strike nor toe-off is being said.
A_REAL_PITCH = 0.02


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
    source, clips = args[0], args[1:]

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

    toe = tail("L_ToeBase") - head("L_Foot")
    forward = mathutils.Vector((toe.x, toe.y, 0.0)).normalized()

    def pitch(side):
        """How much a foot points UP, as a fraction. Its rest angle is the zero."""
        span = tail(f"{side}_ToeBase") - head(f"{side}_Foot")
        return span.z / span.length if span.length > 1e-9 else 0.0

    at_rest = {side: pitch(side) for side in "LR"}
    print(
        f"forward is ({forward.x:.3f}, {forward.y:.3f}, {forward.z:.3f}); "
        f"feet rest at pitch {at_rest['L']:+.3f}/{at_rest['R']:+.3f}"
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
    for name in clips:
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
                    "ground": min(tail(f"{s}_ToeBase").z for s in "LR"),
                    "along": {s: lead(f"{s}_Foot") for s in "LR"},
                    "legs": lead("L_Foot") - lead("R_Foot"),
                    "arms": lead("L_Hand") - lead("R_Hand"),
                    "pitch": {s: pitch(s) - at_rest[s] for s in "LR"},
                    "folded": {s: folded(s) for s in "LR"},
                    "knees": {
                        s: off_chord(f"{s}_Thigh", f"{s}_Calf", f"{s}_Foot") for s in "LR"
                    },
                    "elbows": {
                        s: off_chord(f"{s}_Upperarm", f"{s}_Forearm", f"{s}_Hand")
                        for s in "LR"
                    },
                }
            )

        span = len(frames) - 1 if frames[0]["legs"] == frames[-1]["legs"] else len(frames)
        contact = max(frames, key=lambda f: abs(f["legs"]))
        front, back = ("L", "R") if contact["legs"] > 0 else ("R", "L")

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
            half = span // 2
            # The right foot is down over the first half of the cycle and the left
            # over the second, which is what the pose table says.
            window = range(0, half + 1) if side == "R" else range(half, span + 1)
            walked = [path[i] for i in window if i < len(path)]
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
        apart = (late.index(max(late)) + middle) - early.index(max(early))

        # Where the arms peak against where the legs peak, as a share of the cycle.
        # The brief puts the arm extremes 8 to 12 per cent behind the legs.
        leg_peak = frames.index(contact)
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
            "arm_lag_percent": round(100.0 * lag),
            "arm_lag_wants": "8 to 12",
            "halves_bob_alike": round(rise, 3),
            "halves_peak_frames_apart": apart,
            "halves_should_be_apart": middle,
            "contact_frame": contact["frame"],
        }
        print(f"\n{name}: contact at frame {contact['frame']}, {front} leg leading")
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

        # --- And the two asymmetries a global sign flip cannot survive.
        if contact["pitch"][front] < A_REAL_PITCH:
            refused.append(
                f"{name}: the leading ({front}) foot is not presenting its heel - its "
                f"pitch is {contact['pitch'][front]:+.3f} off rest where toes-up wants "
                f"at least +{A_REAL_PITCH}. A front foot landing toe-first is the "
                f"cleanest tell that the whole cycle is running backwards."
            )
        if contact["pitch"][back] > -A_REAL_PITCH:
            refused.append(
                f"{name}: the trailing ({back}) foot is not up on its toes - pitch "
                f"{contact['pitch'][back]:+.3f} off rest, wanting at most "
                f"-{A_REAL_PITCH}. A trailing foot pushing off heel-first is the same "
                f"tell from the other side."
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
        if abs(apart - middle) > A_FRAME_OR_TWO:
            refused.append(
                f"{name}: the two halves peak {apart} frames apart where half a cycle "
                f"is {middle}. The bob is not keeping time with the steps."
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
