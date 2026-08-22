"""Refuses a gait clip whose limbs bend the wrong way.

    blender --background --python dev/art/verify_gait.py -- <glb> <clip> [<clip>..]

# Why this exists

Three separate attempts shipped a walk with the knees bending backwards and the arms
swinging with the legs instead of against them, and each time it was found by the
person playing the game rather than by anything here. The cause was always a SIGN:
`swing` takes degrees about an armature axis, and whether positive is forward is a
fact about the rig that was reasoned about instead of measured. Reasoning got it
exactly inverted, and inverted twice reads as "the limbs are backwards" without
saying which limb or which way.

So the signs are measured once, in `animate_ranger.py`, and this checks the RESULT —
the exported file, posed over its own clips. A wrong sign now fails the export
instead of reaching a player.

# The three things a walking person does

**Arms oppose legs.** Left leg forward, right arm forward. Every walking animal with
four limbs on two of them does this, and its absence is the single loudest wrongness
in a gait.

**A knee leads.** Bending a knee puts it in FRONT of the line from hip to ankle. Put
it behind and the leg is a bird's.

**An elbow trails.** Bending an elbow puts it BEHIND the line from shoulder to wrist.

Each is a sign, measured off the geometry, so each is a test.

# And the arms have to actually move

A swing too small to see passes the opposition test on a rounding error. So the arms
must carry a real fraction of what the legs do — measured, not assumed, because the
amplitude was once cut to six degrees as a workaround and stayed there.
"""

import sys

import bpy
import mathutils

# How much of the legs' fore-aft travel the hands must cover for the swing to read
# as a swing. Measured on the fixed clips: hands 0.24 against feet 0.46 is a half.
# A quarter is the floor — below that the shoulders look pinned.
ARMS_CARRY_AT_LEAST = 0.25

# How far off a straight line a joint must sit before its direction is called. Below
# this the limb is straight and its bend direction is not a fact about anything.
A_REAL_BEND = 0.004


def argv():
    return sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []


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

    def at(bone):
        return rig.matrix_world @ rig.pose.bones[bone].head

    # Forward is taken off the model's own toe, not assumed. A toe points forward
    # from a foot on every rig, whatever the exporter did to the axes.
    for posed in rig.pose.bones:
        posed.rotation_mode = "QUATERNION"
        posed.rotation_quaternion = (1.0, 0.0, 0.0, 0.0)
        posed.location = (0.0, 0.0, 0.0)
    toe = (rig.matrix_world @ rig.pose.bones["L_ToeBase"].tail) - at("L_Foot")
    forward = mathutils.Vector((toe.x, toe.y, 0.0)).normalized()
    print(f"forward is ({forward.x:.3f}, {forward.y:.3f}, {forward.z:.3f})")

    def lead(bone):
        return (at(bone) - at("Hip")).dot(forward)

    def off_chord(top, joint, bottom):
        """How far the middle joint sits forward of the line from top to bottom."""
        a, b, c = at(top), at(joint), at(bottom)
        span = c - a
        if span.length < 1e-6:
            return 0.0
        along = span.normalized()
        return ((b - a) - along * (b - a).dot(along)).dot(forward)

    complaints = []
    for name in clips:
        action = bpy.data.actions.get(name)
        if action is None:
            complaints.append(f"{name}: no such clip in the file")
            continue
        rig.animation_data.action = action
        low, high = (int(v) for v in action.frame_range)

        frames = []
        for frame in range(low, high + 1):
            bpy.context.scene.frame_set(frame)
            frames.append(
                {
                    "frame": frame,
                    "ground": min(
                        (rig.matrix_world @ rig.pose.bones[f"{s}_ToeBase"].tail).z
                        for s in ("L", "R")
                    ),
                    "legs": lead("L_Foot") - lead("R_Foot"),
                    "arms": lead("L_Hand") - lead("R_Hand"),
                    "knees": (
                        off_chord("L_Thigh", "L_Calf", "L_Foot"),
                        off_chord("R_Thigh", "R_Calf", "R_Foot"),
                    ),
                    "elbows": (
                        off_chord("L_Upperarm", "L_Forearm", "L_Hand"),
                        off_chord("R_Upperarm", "R_Forearm", "R_Hand"),
                    ),
                }
            )

        # How much the LOWER foot rides over the cycle. A planted foot should stay
        # planted, so this wants to be near zero; it is reported rather than
        # enforced because the number it should be has not been established, and a
        # check whose threshold is set to wherever the code happens to sit is
        # decoration. See TROUBLESHOOTING.md — it currently rides 0.095 m walking.
        rides = max(f["ground"] for f in frames) - min(f["ground"] for f in frames)

        peak = max(frames, key=lambda f: abs(f["legs"]))
        legs_travel = max(f["legs"] for f in frames) - min(f["legs"] for f in frames)
        arms_travel = max(f["arms"] for f in frames) - min(f["arms"] for f in frames)
        print(
            f"\n{name}: peak stride at frame {peak['frame']}, "
            f"legs {peak['legs']:+.3f} arms {peak['arms']:+.3f}"
        )
        print(
            f"  travel over the cycle: legs {legs_travel:.3f}, arms {arms_travel:.3f} "
            f"({arms_travel / legs_travel:.0%} of the legs)"
        )
        print(f"  the lower foot rides {rides:.4f} units, {rides * 1.7:.3f} m at game scale")

        if peak["legs"] * peak["arms"] >= 0.0:
            complaints.append(
                f"{name}: the arms swing WITH the legs. At frame {peak['frame']} the "
                f"left leg leads by {peak['legs']:+.3f} and the left hand by "
                f"{peak['arms']:+.3f} — same sign. A walk is contralateral: left leg "
                f"forward, RIGHT arm forward. Flip the sign on the Upperarm swing."
            )
        if legs_travel > 1e-6 and arms_travel / legs_travel < ARMS_CARRY_AT_LEAST:
            complaints.append(
                f"{name}: the arms barely move — {arms_travel:.3f} against the legs' "
                f"{legs_travel:.3f}, {arms_travel / legs_travel:.0%} where "
                f"{ARMS_CARRY_AT_LEAST:.0%} is the floor. Raise the arm amplitude."
            )
        for which, knee in zip("LR", peak["knees"]):
            if knee < -A_REAL_BEND:
                complaints.append(
                    f"{name}: the {which} knee sits {-knee:.3f} BEHIND the line from "
                    f"hip to ankle, so the leg folds like a bird's. A knee leads that "
                    f"line. Turn the Calf about FOLDS_THE_KNEE, positive."
                )
        for which, elbow in zip("LR", peak["elbows"]):
            if elbow > A_REAL_BEND:
                complaints.append(
                    f"{name}: the {which} elbow sits {elbow:.3f} in FRONT of the line "
                    f"from shoulder to wrist, so the arm folds the wrong way. An elbow "
                    f"trails. Turn the Forearm about FOLDS_THE_ELBOW, positive."
                )

    if complaints:
        print("\n" + "\n".join(f"REFUSED  {c}" for c in complaints))
        raise SystemExit(1)
    print("\nevery clip: arms oppose the legs, knees lead, elbows trail.")


main()
