"""Authors a locomotion cycle by moving the FEET, with IK solving the legs.

Imported by `animate_ranger.py`. Nothing here poses a knee or a hip directly.

# Why this replaces posing the joints

Every leg angle was stated by hand and the hips were then moved to put the stance foot
on the ground. That is over-determined, and the thing it over-determines is the one
thing a player notices: with the sole planted, hip height IS the leg's vertical extent
and nothing else. A thigh 38 degrees back is 21% shorter vertically than a vertical one,
so reaching for stride while planted dropped these hips 30 cm and swung them 38 cm a
cycle - reported as "the hip becomes disconnected". Tuning the pose table cannot fix it,
because the table is specifying the quantity that is over-determined; and solving the
knee for it by bisection ran the joint to 45 degrees the wrong way and inverted it.

Inverting the problem removes it. The foot's PATH is stated - planted on the ground
through stance, arcing forward through swing - along with how far the body rises and
falls. Then the knee and hip are whatever reaches that, which is what a leg is. The bob
becomes an input, the planted foot is exactly planted at every frame, and the knee bend
through stance is solved rather than guessed - which is precisely the job knee flexion
does in a real gait.

# What IK needs to be told, and the traps

**The constraint goes on the SHIN, not the foot.** Blender's IK drives the constrained
bone's TAIL to the target, and the shin's tail is the ankle. On the foot with
`chain_count=2` the chain is Foot plus Calf, the thigh stays rigid, and the ankle is
simply unreachable - measured at 0.47 units of tracking error.

**The rest leg must not be dead straight.** A leg extended 99.3% of the way cannot be
solved: there is no bend for the solver to work with and it fails to track at all. A
small root drop keeps a little bend in it, and measured, that took stance slide from
unusable to 0.01 mm.

**Bake with `visual_keying=True`.** `clear_constraints` without it keys the pre-constraint
pose, which is to say the wrong one. Blender's own RNA documentation says so.

**Purge the helpers before export.** The glTF exporter writes every object as a node and
every action as a clip, so the IK targets would ship as part of the character.
"""

import math

import bpy
import mathutils

# How high a swinging foot lifts, as a share of the leg's length. A toe needs very
# little to clear the ground and a lifted knee reads as high-stepping.
CLEARS_BY = 0.10

# How far the body rises and falls, in model units on a figure one unit tall.
#
# An INPUT now, which is the whole point of authoring this way round. Real walking
# oscillates 2.7 to 4.8 cm and running 6 to 9; at 1.7 m scale those are 0.016 to 0.028
# and 0.035 to 0.053. Twice per cycle for both, lowest just after each foot lands.
WALK_BOB = 0.020
RUN_BOB = 0.030

# How far the hips sit below where a straight leg would put them.
#
# # This is what sets the STRIDE, which is not obvious
#
# It reads like a detail about keeping a little bend in the knee for the solver, and it
# started life as that. It is really the thing that decides how far the foot can reach,
# and getting it wrong throttled the run to almost no stride at all.
#
# The arithmetic is exact. This leg is 0.455 units hip to ankle. To put the ankle
# `contact/2` = 0.26 units in FRONT of the hip and still on the ground, the hip may sit
# at most sqrt(0.455^2 - 0.26^2) = 0.373 above it. At full extension it sits at 0.455,
# so the drop has to be at least 0.082 or the target is simply out of reach - and an
# unreachable target does not fail loudly, it clamps, and the foot lands short:
#
#     drop 0.03 -> the leg reaches 0.33 units of contact, 0.55 m
#     drop 0.06 -> 0.45 units, 0.77 m
#     drop 0.09 -> 0.54 units, 0.92 m
#
# Measured before this was understood: 0.49 m on the run against the 0.88 asked for, and
# a run with visibly no stride. 0.09 clears 0.082 with a little margin.
KNEES_STAY_BENT = 0.090


def add_leg_ik(rig, side: str):
    """Puts an IK target under one foot, a pole in front of the knee, and constrains.

    On the CALF with a chain of two, so the chain is Calf plus Thigh and the solver
    drives the ankle. On the Foot with a chain of two it would be Foot plus Calf, the
    thigh would stay rigid, and the ankle would be unreachable - measured at 0.47 units
    of tracking error.

    # The pole target, and why a knee needs one

    A two-bone chain reaching a point has a whole CIRCLE of solutions - the knee can sit
    anywhere on it - and nothing in the goal position says which. Blender picks one, and
    without being told it picked a BACKWARD knee on the sprint: the verifier caught it
    as "the R knee sits 0.011 behind the hip-to-ankle line, so the leg folds like a
    bird's". It is the same degeneracy that makes IK knees flip in every engine, and a
    pole target is the standard answer to it.

    The pole goes well in FRONT of the knee, at hip height, so the only solution it
    admits is a knee pointing forward. `pole_angle` then corrects for however this rig's
    thigh happens to be rolled: it is measured rather than assumed, by asking where the
    knee actually ended up and turning the pole until it points there.
    """
    target = bpy.data.objects.new(f"IK_{side}", None)
    target.empty_display_size = 0.04
    bpy.context.collection.objects.link(target)

    knee = rig.matrix_world @ rig.pose.bones[f"{side}_Calf"].head
    pole = bpy.data.objects.new(f"IKPole_{side}", None)
    pole.empty_display_size = 0.03
    pole.location = knee + mathutils.Vector((0.0, 0.0, 0.0))
    bpy.context.collection.objects.link(pole)

    hold = rig.pose.bones[f"{side}_Calf"].constraints.new("IK")
    hold.target = target
    hold.pole_target = pole
    hold.chain_count = 2
    hold.use_rotation = False
    return target, pole, hold


def aim_the_pole(rig, side: str, pole, hold, forward, leg_length: float) -> float:
    """Sets the pole so the FOOT points forward, by trying every angle and measuring.

    # What a pole is actually needed for here

    With `use_rotation` off, IK places the shin's tail and resolves the chain's roll
    itself. The two legs' bases are mirrored, and a reflection flips HANDEDNESS - so the
    solver can settle that roll 180 degrees apart on the two sides from bases that are
    otherwise perfectly matched. Measured: the LEFT foot pointed backwards on 13 of 17
    run frames while the right did so on none.

    Nothing is wrong with the rig. The solver simply has a free degree of freedom and no
    reason to prefer one answer, so it must be given one.

    `pole_angle` is an offset that depends on the bone's roll, so it cannot be assumed.
    An earlier attempt searched it for the angle that put the KNEE furthest forward, and
    that is the wrong thing to optimise: it found -75 and -45 degrees on the two sides
    and turned both feet 168 degrees off the line of travel. The knee was right and the
    foot was backwards, which is not an improvement.

    So the thing measured is the thing that was wrong: the foot's heading. The knee is
    checked too, as a veto rather than a target - an angle that points the foot forward
    and the knee backwards is no good either.
    """
    hip = rig.matrix_world @ rig.pose.bones[f"{side}_Thigh"].head
    pole.location = hip + forward * (leg_length * 1.5)

    best, best_at = None, 0.0
    for step in range(36):
        angle = -math.pi + step * (2.0 * math.pi / 36.0)
        hold.pole_angle = angle
        bpy.context.view_layer.update()

        thigh = rig.matrix_world @ rig.pose.bones[f"{side}_Thigh"].head
        knee = rig.matrix_world @ rig.pose.bones[f"{side}_Calf"].head
        ankle = rig.matrix_world @ rig.pose.bones[f"{side}_Foot"].head
        toe = rig.matrix_world @ rig.pose.bones[f"{side}_ToeBase"].tail

        # The foot must point forward. That is the measurable that was wrong.
        points = (toe - ankle).normalized().dot(forward)
        # And the knee must not fold backwards, which is a veto and not a score.
        span = ankle - thigh
        if span.length > 1e-6:
            along = span.normalized()
            out = (knee - thigh) - along * (knee - thigh).dot(along)
            if out.dot(forward) < 0.0:
                continue
        if best is None or points > best:
            best, best_at = points, angle

    hold.pole_angle = best_at
    bpy.context.view_layer.update()
    return best_at


def ankle_rests_above_the_sole(rig, mesh, feet, side: str) -> float:
    """How high the ankle sits above the sole with the foot flat, in model units.

    The IK target drives the ANKLE, and an ankle is not a sole - it sits about 4.3 cm
    up inside the shoe on this character. Aiming the target at the floor therefore
    buries the foot by exactly that much before anything else goes wrong, which is part
    of why every frame of the walk had a foot through the ground.
    """
    evaluated = mesh.evaluated_get(bpy.context.evaluated_depsgraph_get())
    baked = evaluated.to_mesh()
    try:
        matrix = evaluated.matrix_world
        sole = min((matrix @ baked.vertices[i].co).z for i in feet[side])
    finally:
        evaluated.to_mesh_clear()
    return (rig.matrix_world @ rig.pose.bones[f"{side}_Foot"].head).z - sole


def where_the_soles_go(rig, facing, contact: float, share: float, phase: float,
                       leg_length: float, ground: float):
    """Where each ankle should be at this instant, in armature space.

    Through STANCE the foot is still on the ground and the body travels over it, so
    relative to the body it slides backward at a constant rate - linearly, because the
    body's speed is constant and any easing here is a limp. Through SWING it arcs
    forward and lifts.

    The path is stated relative to each hip socket, so the two legs get the same path
    half a cycle apart and cannot disagree.
    """
    forward, _ = facing
    out = {}
    for side in "LR":
        own = (phase + (0.5 if side == "L" else 0.0)) % 1.0
        socket = rig.matrix_world @ rig.pose.bones[f"{side}_Thigh"].head
        if own < share:
            # Planted: front to back, linearly, on the ground.
            along = contact * (0.5 - own / share)
            lift = 0.0
        else:
            through = (own - share) / max(1e-6, 1.0 - share)
            along = contact * (through - 0.5)
            lift = CLEARS_BY * leg_length * math.sin(math.pi * through)
        # Where the SOLE should be, not the ankle. An ankle is 14 cm up inside this
        # character's shoe with the foot flat, and further as the foot tilts - up to 7
        # cm either way over a 17 cm foot. Aiming the ankle at a fixed height therefore
        # buries or floats the foot by an amount that changes every frame, which is
        # what put the feet 20 cm through the floor. The sole is what touches the
        # ground, so the sole is what the path is for; `solve_the_target` then works
        # out where the ankle has to be to put it there.
        out[side] = mathutils.Vector(
            (
                socket.x + forward.x * along,
                socket.y + forward.y * along,
                ground + lift,
            )
        )
    return out


def how_high_the_body_rides(share: float, phase: float, bob: float) -> float:
    """How far the body sits above its own average, at this instant.

    Twice per cycle and lowest just after each foot lands, which is the loading
    response - the body falling onto the leg that has just taken it. A bob at the wrong
    frequency reads as a limp and one peaking AT contact reads as a hop.
    """
    return -bob * math.cos(4.0 * math.pi * (phase - share * 0.5))


def solve_the_target(rig, mesh, feet, targets, wanted, pitches, tries: int = 4,
                     forward=None, across=None, toe_out: float = 0.0):
    """Moves each IK target until the SOLE lands where the path asks.

    # Why this is solved and not computed

    The ankle-to-sole distance is not a constant. It is 14 cm with the foot flat and
    changes by up to 7 cm either way as the foot tilts, over a foot 17 cm long, and the
    tilt changes every frame. A closed form would need the shoe's geometry; two facts
    about it are enough to iterate instead.

    Raising the target raises the sole very nearly one for one, so this converges in
    three or four passes: solve the leg, set the foot's tilt, measure the sole off the
    DEFORMED MESH, and move the target by whatever is left over.

    The foot's tilt is set INSIDE the loop, because tilting the foot moves the sole and
    the sole is the thing being aimed. Doing it afterwards - which was the first attempt -
    changes the answer after it has been found.
    """
    worst = 0.0
    for _ in range(tries):
        bpy.context.view_layer.update()
        for side in "LR":
            point_the_foot(rig, side, pitches[side], toe_out, forward, across)
        bpy.context.view_layer.update()
        worst = 0.0
        for side in "LR":
            sole = lowest_sole(rig, mesh, feet, side)
            off = wanted[side].z - sole
            worst = max(worst, abs(off))
            targets[side].location = targets[side].location + mathutils.Vector(
                (0.0, 0.0, off)
            )
    return worst


def lowest_sole(rig, mesh, feet, side: str) -> float:
    """How low the deformed foot reaches, in armature Z."""
    evaluated = mesh.evaluated_get(bpy.context.evaluated_depsgraph_get())
    baked = evaluated.to_mesh()
    try:
        matrix = evaluated.matrix_world
        return min((matrix @ baked.vertices[i].co).z for i in feet[side])
    finally:
        evaluated.to_mesh_clear()


def rest_foot_pitch(rig, side: str, forward):
    """How far the sole tilts from horizontal in the REST pose, in degrees.

    The zero that every authored ankle angle is measured against, because in the rest
    pose the sole is flat on the floor - which is the only definition of "flat" that
    means anything on a rig nobody here built.
    """
    heel = rig.matrix_world @ rig.pose.bones[f"{side}_Foot"].head
    toe = rig.matrix_world @ rig.pose.bones[f"{side}_ToeBase"].tail
    span = toe - heel
    flat = span - mathutils.Vector((0.0, 0.0, span.z))
    return math.degrees(math.atan2(span.z, max(1e-6, flat.length)))


def point_the_foot(rig, side: str, pitch: float, toe_out: float, forward, across):
    """Aims one foot outright: heading AND pitch, both stated, neither read back.

    # A foot is CONTROLLED, not corrected

    Every earlier version nudged the foot from wherever it happened to be - set its
    pitch relative to the shin, or measure its heading and turn it by the difference.
    Both fail for the same reason, which is that the shin is solved by IK and swings
    more than forty degrees a frame, so "wherever it happened to be" is not a fixed
    starting point and there is nothing stable to correct FROM.

    Worse, measuring the foot's own heading breaks exactly when it matters. A foot
    arriving vertical from the solver has a horizontal component of 0.02 units on a 0.17
    unit foot, and the SIGN of that is noise - which is why the run read as a foot
    pointing 179 degrees backwards when it was really a foot pointing straight down.
    Giving up in that case left it pointed forever; using the body's forward as a
    stand-in flipped it the rest of the way.

    So nothing is read back. The direction the foot should point is built from the
    travel direction, the toe-out and the pitch - all three known, none of them
    degenerate - and the foot is turned onto it by the shortest arc, which adds no roll.
    That is what a foot control does in a hand-built rig: the foot's orientation is an
    input, not a consequence.
    """
    hand = 1.0 if side == "L" else -1.0
    yaw = math.radians(toe_out * hand)
    heading = forward * math.cos(yaw) + across * math.sin(yaw)
    lift = math.radians(pitch)
    wanted = (heading * math.cos(lift) + mathutils.Vector((0.0, 0.0, 1.0)) * math.sin(lift)).normalized()

    heel = rig.matrix_world @ rig.pose.bones[f"{side}_Foot"].head
    toe = rig.matrix_world @ rig.pose.bones[f"{side}_ToeBase"].tail
    now = toe - heel
    if now.length < 1e-9:
        return
    turn = now.normalized().rotation_difference(wanted)
    if turn.angle < 1e-9:
        return
    posed = rig.pose.bones[f"{side}_Foot"]
    frame = posed.matrix.to_3x3()
    local = (frame.inverted() @ turn.axis).normalized()
    posed.rotation_mode = "QUATERNION"
    posed.rotation_quaternion = posed.rotation_quaternion @ mathutils.Quaternion(
        local, turn.angle
    )


def bake_the_constraints(rig, first: int, last: int) -> None:
    """Turns the solved poses into plain keyframes and removes the constraints.

    `visual_keying` is what samples the constraint RESULT. Without it,
    `clear_constraints` keys the pose as it was before the solver ran, which is the
    wrong one - Blender's own RNA documentation says as much.
    """
    bpy.context.view_layer.objects.active = rig
    if bpy.context.object.mode != "POSE":
        bpy.ops.object.mode_set(mode="POSE")
    bpy.ops.pose.select_all(action="SELECT")
    bpy.ops.nla.bake(
        frame_start=first,
        frame_end=last,
        step=1,
        only_selected=True,
        visual_keying=True,
        clear_constraints=True,
        clear_parents=False,
        use_current_action=True,
        bake_types={"POSE"},
    )
    bpy.ops.object.mode_set(mode="OBJECT")


def drop_the_helpers(helpers) -> None:
    """Removes the IK helpers and any action they picked up along the way.

    Both halves matter. The glTF exporter writes every OBJECT as a node, so a target
    left behind ships as part of the character; and it writes every ACTION as a clip,
    so the targets' own location keys ship as animations. Measured on an uncleaned
    scene: six stray clips called IK_LAction, IK_RAction and numbered variants, next to
    the three that were wanted.
    """
    for helper in helpers:
        if helper is None:
            continue
        if helper.animation_data and helper.animation_data.action:
            spent = helper.animation_data.action
            helper.animation_data.action = None
            if spent.users == 0:
                bpy.data.actions.remove(spent)
        if helper.name in bpy.data.objects:
            bpy.data.objects.remove(helper, do_unlink=True)
