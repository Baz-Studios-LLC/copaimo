"""Measures how far a gait clip actually carries the warden, per cycle.

    blender --background --python dev/art/stride_measure.py -- <glb> <clip> [<clip>..]

# Why this cannot be arithmetic

`motion.rs` divides the warden's speed by how far one stride covers, so the clip plays
at the right cadence and the feet do not skate. That distance was first estimated as
`2 * leg * sin(stride angle)` and it was wrong enough to matter.

# And why the second attempt was still wrong

The estimate was then replaced by a MEASUREMENT of how far a foot travels front to
back relative to the hips, doubled, on the reasoning that a cycle is both feet taking
one step. That reasoning is wrong, and the way it is wrong differs between a walk and
a run — which is exactly the sort of error that hides.

The governing identity is exact: **speed = cadence x stride length**. A planted foot
is stationary on the GROUND, so relative to the hips it travels backward at precisely
the speed the character is moving. So if a foot moves `S` backward during a stance
lasting a fraction `f` of the cycle, the body advances `S / f` per cycle — not `2 S`.

* A walk has a foot down about 60% of the cycle, so `S / f` is about `1.7 S`.
* A run has one down about 35%, so it is about `2.9 S`.

Doubling therefore OVERSTATES a walk by a fifth and UNDERSTATES a run by a third.
Understating a run is what makes it churn: the clip is played fast enough to cover a
distance the poses do not reach, and the result is a cadence of 265 steps a minute
where a fast human runs at 180.

# What this measures instead

The stance fraction is not estimated at all. A planted foot moves backward relative to
the hips at the character's speed, so the RATE is the thing to measure: pick out the
frames where a foot is on the ground, fit a line to its fore-aft travel across them,
and the slope is metres per frame. Multiply by the frames in a cycle and that is how
far the cycle carries the warden.

The fit's straightness is worth having too. A planted foot must move at a CONSTANT
rate, because the ground does not accelerate; easing on it is what reads as limping
and twitching. So the residual is reported.
"""

import sys

import bpy
import mathutils

# How close to its lowest a foot must be to count as planted, as a share of how far
# that foot travels vertically over the cycle. A foot within a tenth of its lowest is
# on the ground; the toe-off and heel-strike frames at the edges of that are the ones
# where the ankle is rolling, so they are included deliberately.
PLANTED_WITHIN = 0.12

# How tall the model is authored, and what the game scales it to.
AUTHORED = 1.0
GAME_SCALE = 1.7

# The speeds the game uses, so the cadence a clip implies can be printed next to the
# range a person actually manages. Kept in step with `src/player.rs` by hand: this is
# a reporting convenience, not a source of truth.
WALK_SPEED = 1.8
SPRINT_SPEED = 3.6

# What a believable cadence is, in steps a minute. Walking sits at 100 to 115 and 140
# is itself the walk-to-run transition; recreational running is 150 to 180 and elites
# are 180 plus.
BELIEVABLE_WALK = (95.0, 140.0)
BELIEVABLE_RUN = (150.0, 200.0)


def argv():
    return sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []


def straight_line(xs, ys):
    """Least squares slope and intercept, plus the worst residual."""
    n = len(xs)
    if n < 2:
        return 0.0, 0.0, 0.0
    mean_x = sum(xs) / n
    mean_y = sum(ys) / n
    spread = sum((x - mean_x) ** 2 for x in xs)
    if spread < 1e-12:
        return 0.0, mean_y, 0.0
    slope = sum((x - mean_x) * (y - mean_y) for x, y in zip(xs, ys)) / spread
    intercept = mean_y - slope * mean_x
    worst = max(abs(y - (slope * x + intercept)) for x, y in zip(xs, ys))
    return slope, intercept, worst


def main() -> None:
    args = argv()
    if len(args) < 2:
        raise SystemExit("need <glb> <clip> [<clip>...]")
    src, clips = args[0], args[1:]

    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=src)
    rig = next(o for o in bpy.data.objects if o.type == "ARMATURE")
    if rig.animation_data:
        for track in rig.animation_data.nla_tracks:
            track.mute = True

    # Forward off the model's own toe rather than assumed, the same way
    # `verify_gait.py` does it.
    rig.animation_data.action = None
    for posed in rig.pose.bones:
        posed.rotation_mode = "QUATERNION"
        posed.rotation_quaternion = (1.0, 0.0, 0.0, 0.0)
        posed.location = (0.0, 0.0, 0.0)
    bpy.context.view_layer.update()
    toe = (rig.matrix_world @ rig.pose.bones["L_ToeBase"].tail) - (
        rig.matrix_world @ rig.pose.bones["L_Foot"].head
    )
    forward = mathutils.Vector((toe.x, toe.y, 0.0)).normalized()
    print(f"forward is ({forward.x:.3f}, {forward.y:.3f}, {forward.z:.3f})")

    scene = bpy.context.scene
    for name in clips:
        action = bpy.data.actions.get(name)
        if action is None:
            print(f"no clip {name!r}")
            continue
        rig.animation_data.action = action
        low, high = (int(v) for v in action.frame_range)

        # Each foot, every frame: how far ahead of the hips it is, and how high the
        # toe sits. The toe rather than the ankle, because it is the part that is
        # actually on the ground through most of a stance.
        tracks = {side: [] for side in ("L", "R")}
        for frame in range(low, high + 1):
            scene.frame_set(frame)
            bpy.context.view_layer.update()
            hips = rig.matrix_world @ rig.pose.bones["Hip"].head
            for side in tracks:
                foot = rig.matrix_world @ rig.pose.bones[f"{side}_Foot"].head
                toe_tip = rig.matrix_world @ rig.pose.bones[f"{side}_ToeBase"].tail
                tracks[side].append(
                    {
                        "frame": frame,
                        "ahead": (foot - hips).dot(forward),
                        "height": toe_tip.z,
                    }
                )

        # The cycle's length in frames. The last frame repeats the first, so it is
        # the span rather than the count.
        span = high - low
        if span <= 0:
            print(f"{name}: a single frame, so nothing to measure")
            continue

        rates, straightness, stances = [], [], []
        for side, path in tracks.items():
            floor = min(p["height"] for p in path)
            ceiling = max(p["height"] for p in path)
            travel = ceiling - floor
            near_ground = [
                p
                for p in path
                if travel < 1e-9 or (p["height"] - floor) <= PLANTED_WITHIN * travel
            ]
            # A stance is contiguous. A foot passing low twice in a cycle would give
            # two runs of frames, and fitting one line through both measures nothing,
            # so the LONGEST unbroken run is the stance.
            runs, current = [], []
            for p in near_ground:
                if current and p["frame"] != current[-1]["frame"] + 1:
                    runs.append(current)
                    current = []
                current.append(p)
            if current:
                runs.append(current)
            stance = max(runs, key=len) if runs else []
            if len(stance) < 3:
                print(
                    f"  {side}: only {len(stance)} frame(s) with the foot down — too "
                    f"few to fit a rate to, so this foot is skipped"
                )
                continue
            slope, _, worst = straight_line(
                [p["frame"] for p in stance], [p["ahead"] for p in stance]
            )
            # Backward relative to the hips is NEGATIVE ahead, so the magnitude is
            # the rate the ground goes by.
            rates.append(abs(slope))
            straightness.append(worst)
            stances.append(len(stance) / span)
            print(
                f"  {side}: down for {len(stance)}/{span} frames "
                f"({len(stance) / span:.0%} of the cycle), travelling "
                f"{abs(slope):.4f} units a frame, worst wobble {worst:.4f}"
            )

        if not rates:
            print(f"{name}: no foot was ever down, so this clip cannot be measured")
            continue

        rate = sum(rates) / len(rates)
        cycle = rate * span * GAME_SCALE / AUTHORED
        wobble = max(straightness) * GAME_SCALE
        stance_share = sum(stances) / len(stances)

        # And what that implies about cadence at the speeds the game uses. Two steps
        # to a cycle, sixty seconds to a minute.
        for label, speed, believable in (
            ("walk", WALK_SPEED, BELIEVABLE_WALK),
            ("run", SPRINT_SPEED, BELIEVABLE_RUN),
        ):
            if label not in name.lower():
                continue
            steps = 2.0 * 60.0 * speed / cycle if cycle > 1e-9 else float("inf")
            verdict = (
                "believable"
                if believable[0] <= steps <= believable[1]
                else ("TOO FAST — this is the frantic-little-steps read" if steps > believable[1] else "TOO SLOW — this will slide")
            )
            print(
                f"{name}: a cycle covers {cycle:.3f} m at game scale, so {speed} m/s "
                f"needs {steps:.0f} steps a minute ({believable[0]:.0f}-"
                f"{believable[1]:.0f} is {verdict.split(' ')[0]}): {verdict}"
            )
            break
        else:
            print(f"{name}: a cycle covers {cycle:.3f} m at game scale")

        print(
            f"  the foot is down {stance_share:.0%} of the cycle; doubling one foot's "
            f"swing would have said {2.0 * (max(p['ahead'] for p in tracks['L']) - min(p['ahead'] for p in tracks['L'])) * GAME_SCALE:.3f} m"
        )
        if wobble > 0.01:
            print(
                f"  WOBBLE: the planted foot's travel is {wobble:.3f} m off a straight "
                f"line. The ground does not accelerate, and easing on a planted foot "
                f"is what reads as limping and twitching."
            )


main()
