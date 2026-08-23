"""What each clip ACTUALLY carries per cycle, measured from the planted foot's own rate.

`motion::{WALK,RUN,SPRINT}_COVERS` decide the playback rate - `speed * lasts / covers` -
so an error in them is an error in how fast the legs turn for a given ground speed. Get it
too small and the clip plays too fast: the legs churn at a rate implying more speed than
the body has, which is the "running through water" / "Scooby Doo" read. It is the single
most consequential number in the pipeline and it was being taken from the wrong place.

It came from `verify_gait`'s `covers_implied_m`, which is `contact_length / stance_share`,
where `contact_length` is the AUTHORED sweep. Two ways that goes wrong. The authored sweep
is what was asked for, and the reach solve clips it, so the ask is not the outcome. And the
achieved sweep is measured between two landmark extremes, which misses travel the foot does
while rolling past them. Measured on the run, `contact_length` said 0.60 m where the foot
genuinely swept 0.83, so `covers` came out 1.80 against a true 2.50 - the clip was played
39% too fast.

This measures the outcome instead, and from the invariant that actually defines `covers`:
through stance, the planted contact patch travels backward at a constant rate equal to the
body's forward speed. So `covers = that rate x span`. Vertices in contact in both frames of
a pair give the rate directly, and the MEDIAN across stance is taken rather than the mean,
because the first and last planted frames are the patch arriving and leaving - their vertex
sets are still changing and they read wild (+6.17 cm on a frame that should be -7.50).

Print this, then paste the numbers into `motion.rs`. It is deliberately not automatic: a
covers change retunes every speed and cadence downstream, so it should be a decision.
"""
import statistics
import sys

import bpy

ART = "C:/Users/jsull/Desktop/copaimo/dev/art"
sys.path.insert(0, ART)

GLB = "C:/Users/jsull/Desktop/copaimo/assets/models/person_ranger.glb"
SCALE = 170.0
TOUCHING = 0.012

bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=GLB)
rig = next(o for o in bpy.data.objects if o.type == "ARMATURE")

import prepare_rig

mesh = prepare_rig.the_body()
prepare_rig.reach_the_ends(rig, mesh)
_, forward, _ = prepare_rig.body_frame(rig)

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


def patch(side, floor):
    got = mesh.evaluated_get(bpy.context.evaluated_depsgraph_get()).to_mesh()
    return {
        i: (mesh.matrix_world @ got.vertices[i].co)
        for i in owns[side]
        if (mesh.matrix_world @ got.vertices[i].co).z - floor <= TOUCHING
    }


print(f"{'clip':8} {'span':>5} {'rate cm/fr':>11} {'TRUE covers m':>14} {'spread':>8}")
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
    span = hi - lo

    floor = None
    for frame in range(lo, hi):
        bpy.context.scene.frame_set(frame)
        got = mesh.evaluated_get(bpy.context.evaluated_depsgraph_get()).to_mesh()
        low = min((mesh.matrix_world @ v.co).z for v in got.vertices)
        floor = low if floor is None else min(floor, low)

    rates = []
    for side in "LR":
        was = None
        for frame in range(lo, hi):
            bpy.context.scene.frame_set(frame)
            now = patch(side, floor)
            if was and now:
                shared = set(was) & set(now)
                if shared:
                    moved = sum((now[i] - was[i]).dot(forward) for i in shared) / len(shared)
                    # Backward is negative along `forward`; report it as positive travel.
                    rates.append(-moved * SCALE)
            was = now
    if not rates:
        print(f"{clip:8} {span:5d}  no contact found")
        continue
    rate = statistics.median(rates)
    keep = [r for r in rates if abs(r - rate) < abs(rate) * 0.5]
    print(
        f"{clip:8} {span:5d} {rate:11.2f} {rate * span / 100.0:14.3f} "
        f"{max(keep) - min(keep):8.2f}"
    )
