"""How far the CONTACT PATCH actually slips, as opposed to how far the ankle wanders.

`verify_gait.slide` fits a straight line to a planted landmark's travel and reports the
worst deviation as a fraction. That is a linearity test on ONE point, and a foot that rolls
correctly moves that point non-linearly on purpose - the heel comes up, the pivot walks
forward to the ball, and the ankle arcs over it. So the metric conflates rolling with
slipping, and it says so loudest exactly when the roll is most correct. It rose from 0.102
to 0.200 the moment toe-off was given a real plantarflexion, which is the opposite of what
it was meant to catch.

The naive version of this was wrong in the way everything in this pipeline is wrong at
least once: FRAME OF REFERENCE. It asked whether the contact patch was stationary, which
would be right for a clip that carries its own root motion - but these are IN-PLACE clips.
The body sits at the origin and the planted foot travels BACKWARD across the floor on
purpose; the game adds the forward motion at runtime. So it measured the intended sweep and
called it slip, and reported 53 cm of failure on the walk, which is signed off.

The invariant is not that the patch holds still. It is that the patch moves backward at a
CONSTANT rate, equal to the speed the body is meant to be travelling - `covers / span` per
frame. A foot going backward faster than that is over-running the ground; slower, and it is
dragging. So this sums the per-frame deviation from that rate, over the vertices in contact
in both frames of a pair, and reports centimetres. A correctly rolling foot does not inflate
it, because rolling moves the pivot but not the patch's rate.

Read-only. Nothing is rebuilt or exported.
"""
import math
import sys

import bpy
import mathutils

ART = "C:/Users/jsull/Desktop/copaimo/dev/art"
sys.path.insert(0, ART)

GLB = "C:/Users/jsull/Desktop/copaimo/assets/models/person_ranger.glb"
SCALE = 170.0
TOUCHING = 0.012  # ~2 cm of sole, in model units

# Measured per clip by verify_gait, in metres a cycle. Kept here rather than recomputed so
# this probe checks the clip against the number the GAME is driven by - if they disagree,
# that disagreement is the bug, and a probe that derives its own expectation would hide it.
COVERS = {"walk": 0.881, "run": 1.801, "sprint": 2.111}

bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=GLB)
rig = next(o for o in bpy.data.objects if o.type == "ARMATURE")

import prepare_rig

mesh = prepare_rig.the_body()
prepare_rig.reach_the_ends(rig, mesh)
_, forward, _ = prepare_rig.body_frame(rig)

# Which vertices belong to which foot, by dominant weight.
groups = {g.index: g.name for g in mesh.vertex_groups}
owns = {"L": [], "R": []}
for vertex in mesh.data.vertices:
    best, who = 0.0, None
    for group in vertex.groups:
        name = groups.get(group.group, "")
        if group.weight > best and ("Foot" in name or "ToeBase" in name):
            best, who = group.weight, name
    if who:
        owns[who[0]].append(vertex.index)


def contact_patch(side, floor):
    """The vertices of this foot touching the floor, and where they are."""
    got = mesh.evaluated_get(bpy.context.evaluated_depsgraph_get()).to_mesh()
    where = {}
    for i in owns[side]:
        at = mesh.matrix_world @ got.vertices[i].co
        if at.z - floor <= TOUCHING:
            where[i] = at
    return where


print(f"{'clip':8} {'side':>4} {'stance':>8} {'slip cm':>9} {'worst step':>11}  verdict")
for clip in ("walk", "run", "sprint"):
    action = bpy.data.actions.get(clip)
    if action is None:
        continue
    if rig.animation_data is None:
        rig.animation_data_create()
    rig.animation_data.action = action
    if rig.animation_data.action_slot is None and action.slots:
        rig.animation_data.action_slot = action.slots[0]
    lo, hi = (int(round(v)) for v in action.frame_range)

    # The floor is the lowest the whole body ever gets - the clips are authored onto it.
    floor = None
    for frame in range(lo, hi):
        bpy.context.scene.frame_set(frame)
        got = mesh.evaluated_get(bpy.context.evaluated_depsgraph_get()).to_mesh()
        low = min((mesh.matrix_world @ v.co).z for v in got.vertices)
        floor = low if floor is None else min(floor, low)

    # What one frame of correct travel is: the cycle carries `covers` over `span` frames.
    span = hi - lo
    covers = COVERS[clip]
    per_frame = covers * 100.0 / span
    print(f"  ({clip}: {covers:.3f} m a cycle over {span} frames "
          f"= {per_frame:.2f} cm of backward travel a frame)")

    for side in "LR":
        was = None
        slipped, worst, planted = 0.0, 0.0, 0
        for frame in range(lo, hi):
            bpy.context.scene.frame_set(frame)
            now = contact_patch(side, floor)
            if now:
                planted += 1
            if was and now:
                shared = set(was) & set(now)
                if shared:
                    moved = sum(
                        (now[i] - was[i]).dot(forward) for i in shared
                    ) / len(shared)
                    # Backward is negative along `forward`, so the wanted step is -per_frame.
                    off = abs(moved * SCALE + per_frame)
                    slipped += off
                    worst = max(worst, off)
            was = now
        verdict = "good" if slipped < 2.0 else ("watch" if slipped < 5.0 else "SLIPPING")
        print(
            f"{clip:8} {side:>4} {planted:5d} fr {slipped:9.2f} {worst:11.2f}  {verdict}"
        )
