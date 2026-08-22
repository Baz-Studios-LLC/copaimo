"""Authors the foot the way a foot works: the BALL is the pivot, the ankle follows.

Imported by `animate_ranger.py`.

# What was wrong, in one sentence

A bone rotates about its HEAD, so tilting the `Foot` bone pivots at the ANKLE - which
lifts the ball of the foot and drives the toe down through the floor. A real foot does
the opposite: the ball stays on the ground and the heel comes up over it.

# Why that mattered so much on this character

Measured, at rest:

    the shoe, front to back                      29.4 cm
    the bones inside it, ankle to toe tip         10.1 cm
    shoe reaching PAST the last bone              11.5 cm
    sole sitting BELOW the last bone               8.9 cm

Ten centimetres of bone in a twenty-nine centimetre shoe. The thing that hits the floor
is twelve centimetres in front of the last joint and nine below it, so a few degrees of
tilt at the ankle becomes centimetres of shoe through the ground - which is exactly what
it looked like.

# What this does instead

The rig already has a joint at the ball: `ToeBase.head`. Nothing was pivoting there.
So the path is authored for the BALL, the foot's tilt is authored as before, and the
ANKLE is then wherever those two put it - which is what the IK target is set to. The
ball stays planted through stance and the heel rises over it, because that is now what
the arithmetic says rather than something hoped for.

No bone is added and nothing is re-weighted, so a fresh export from the generator drops
straight in.

# And the shoe is rested on the floor, rather than the tilt being limited

Even pivoting correctly, a tilt puts one end of a rigid 29 cm shoe below the ground.
Clamping the tilt for that reason was the first attempt and it flattens the foot
completely - see `rest_the_shoe_on_the_floor`. The tilt is left alone and the HEIGHT is
solved instead, so the pivot moves from heel to ball to toe through the step without any
of them being named. That is what a foot roll is.
"""

import math

import bpy
import mathutils


def foot_landmarks(rig, mesh, feet, side: str):
    """Where the ankle, the ball and the sole are, with the foot as it rests.

    All three off the real thing: the joints from the rig, the sole from the DEFORMED
    mesh. The offsets between them are what let an ankle be derived from a ball.
    """
    evaluated = mesh.evaluated_get(bpy.context.evaluated_depsgraph_get())
    baked = evaluated.to_mesh()
    try:
        matrix = evaluated.matrix_world
        spots = [matrix @ baked.vertices[i].co for i in feet[side]]
        sole = min(p.z for p in spots)
        back = min(spots, key=lambda p: p.x)
        front = max(spots, key=lambda p: p.x)
    finally:
        evaluated.to_mesh_clear()

    ankle = rig.matrix_world @ rig.pose.bones[f"{side}_Foot"].head
    ball = rig.matrix_world @ rig.pose.bones[f"{side}_ToeBase"].head
    tip = rig.matrix_world @ rig.pose.bones[f"{side}_ToeBase"].tail
    return {
        "ankle": ankle,
        "ball": ball,
        "tip": tip,
        "sole": sole,
        "ball_above_sole": ball.z - sole,
        "ankle_from_ball": ankle - ball,
        "rest_direction": (tip - ankle).normalized(),
        "heel_behind_ball": (ball - back).length,
        "toe_ahead_of_ball": (front - ball).length,
    }


def where_the_balls_go(rig, facing, contact: float, share: float, phase: float,
                       leg_length: float, ground: float, landmarks,
                       swing_lift: float = 0.10, swing_shape: float = 1.0,
                       lands_ahead: float = 0.5, lead_by: float = 0.0):
    """Where each ball of the foot should be at this instant.

    Through STANCE the ball is still, on the ground, and the body travels over it - so
    relative to the body it slides backward at a constant rate. Through SWING it arcs
    forward and lifts.

    The ball rather than the ankle, because the ball is what a foot pivots on. Authoring
    the ankle's path and letting the ball fall where it may is what put the toe through
    the floor.
    """
    forward, _ = facing
    out = {}
    for side in "LR":
        own = (phase + (0.5 if side == "L" else 0.0)) % 1.0
        socket = rig.matrix_world @ rig.pose.bones[f"{side}_Thigh"].head
        # `lands_ahead` is the share of the sweep in FRONT of the hip at touchdown.
        # A walk's stance is near symmetric (0.5), a runner's is not: the leg lands
        # about a third of the sweep ahead and leaves two thirds behind, because a
        # rising heel extends reach BEHIND the body far more than a landing leg can
        # reach ahead. The symmetric window was the run's reach limit: asking for a
        # leg-length sweep centred on the hip puts the touchdown out of reach, while
        # the same sweep landed short and released long fits inside it - which is how
        # real runners sweep a full leg length per stance (0.99 +/- 0.08 m at every
        # speed, Weyand).
        if own < share:
            along = contact * (lands_ahead - own / share)
            lift = 0.0
        else:
            through = (own - share) / max(1e-6, 1.0 - share)
            along = contact * (through - (1.0 - lands_ahead))
            # `swing_lift` is per gait. One shared 0.10 left a run's swing foot on a
            # low flat arc, and a low far foot is a STRAIGHT leg - the swing knee read
            # dead straight (0.0000) where a runner's heel tucks up toward the seat.
            # The knee is never posed; raising the arc is what folds it.
            #
            # And `swing_shape` skews WHERE the arc peaks. Height alone was not enough:
            # measured, the leg still saturated for the first fifth of swing, because
            # sin(pi*through) is near zero right after toe-off - exactly when the ball
            # is furthest back and the ankle trails BEHIND it. A walk's foot does skim
            # the ground there (shape 1.0), but a runner's heel snaps up the moment it
            # leaves - so shapes below 1 pull the peak early: the foot rises fast, and
            # comes back down low ahead of touchdown where the reach needs it.
            lift = swing_lift * leg_length * math.sin(
                math.pi * through ** swing_shape
            )
        out[side] = mathutils.Vector(
            (
                # `lead_by` shifts the whole path forward by however far the ball
                # sits AHEAD of the ankle in the bind. The path is authored for the
                # ball, but the leg is hung from the ankle - so once the ball moved to
                # the shoe's real flex point, 14 cm ahead of the ankle, the same path
                # put the leading ankle almost directly under the hip and its knee
                # folded MORE than the trailing one, which is the backwards read. The
                # ankle's sweep is what has to sit symmetrically about the hip.
                socket.x + forward.x * (along + lead_by),
                socket.y + forward.y * (along + lead_by),
                ground + landmarks[side]["ball_above_sole"] + lift,
            )
        )
    return out


def rest_the_shoe_on_the_floor(rig, mesh, feet, targets, ground, planted, tilts,
                               toe_out, forward, across, aim, tries=16,
                               close_enough=0.00003):
    """Nudges each IK target vertically until the shoe's lowest point is on the floor.

    # Why the tilt is NOT clamped, which was the first attempt

    Clamping looked right: work out how far the foot may tilt before an end of the shoe
    breaks the floor, and go no further. It flattens the foot completely - measured, it
    removed 32 of the 32 degrees asked for - because with the ball on the ground ANY
    tilt puts one end of a rigid shoe underground. The arithmetic was not wrong; the
    model was.

    A rigid shoe does not pivot on its ball. It pivots on whichever END is touching:
    the heel at heel-strike, the toe at toe-off, and it is flat in between. That is
    what a foot roll IS, and it is three pivots rather than one.

    So the tilt is left alone and the HEIGHT is solved. Whatever the tilt, the lowest
    part of the shoe is put on the floor, and the pivot then emerges by itself - heel,
    flat, or toe, in that order, without any of them being named. Which is the thing
    that makes this general: it never needs to know which end is down.
    """
    # # The tilt has to be re-set every pass, or the two chase each other
    #
    # Moving a target makes the solver re-rotate the calf, and the foot's rotation is
    # relative to the calf - so nudging the target upward quietly re-tilts the foot,
    # which moves the sole again. Solving the height without re-fixing the tilt each
    # pass left 1.4 to 2.1 cm of shoe underground no matter how many passes were spent
    # on it: the two were converging against each other rather than together.
    # 16 passes with an early exit, not 10 flat: measured, one walk frame - the one
    # where the heel-rise changes fastest, right at toe-off - was still 1.47 cm under
    # the floor when 10 ran out, while every neighbouring frame had settled in 4.
    worst = 0.0
    for _ in range(tries):
        for side in "LR":
            pitch, toes_at = tilts[side]
            aim(rig, side, pitch, toe_out, forward, across, toes_at)
        bpy.context.view_layer.update()
        evaluated = mesh.evaluated_get(bpy.context.evaluated_depsgraph_get())
        baked = evaluated.to_mesh()
        try:
            matrix = evaluated.matrix_world
            lowest = {
                side: min((matrix @ baked.vertices[i].co).z for i in feet[side])
                for side in "LR"
            }
        finally:
            evaluated.to_mesh_clear()
        worst = 0.0
        for side in "LR":
            if planted.get(side):
                off = ground - lowest[side]
            else:
                # Airborne: only stop it going THROUGH, never pull it down to touch.
                off = max(0.0, ground - lowest[side])
            worst = max(worst, abs(off))
            targets[side].location = targets[side].location + mathutils.Vector(
                (0.0, 0.0, off)
            )
        if worst < close_enough:
            break
    return worst


def ankle_for(ball, tilt: float, toe_out: float, side: str, forward, across, landmarks):
    """Where the ankle must be to put the ball THERE with the foot tilted THAT far.

    The foot is a rigid body: fix the ball and choose an orientation, and the ankle has
    exactly one place it can be. So the IK target is not authored - it is derived, which
    is the whole point of pivoting on the ball.
    """
    hand = 1.0 if side == "L" else -1.0
    yaw = math.radians(toe_out * hand)
    heading = forward * math.cos(yaw) + across * math.sin(yaw)
    lift = math.radians(tilt)
    wanted = (
        heading * math.cos(lift) + mathutils.Vector((0.0, 0.0, 1.0)) * math.sin(lift)
    ).normalized()

    turn = landmarks["rest_direction"].rotation_difference(wanted)
    return ball + (turn @ landmarks["ankle_from_ball"]), wanted
