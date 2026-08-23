"""Does an arm pass THROUGH the torso? Tested against the torso's cross-section.

Two rulers were tried and thrown away before this one, both of which answered confidently:

* `closest_point_on_mesh` over the whole body, sign-tested against the normal. A joint centre
  is inside its own limb by construction, so the nearest surface it finds is the arm's own
  skin and every joint of every frame reads as buried.
* A BVH of torso polygons only, same sign test. The torso polys are an open PATCH, not a
  closed volume, and `find_nearest` normals on an open shell point whichever way the nearest
  face happens to face - it reported an arm 30 cm inside a torso 22 cm deep.

So no normals. The torso's SILHOUETTE at the arm's own height is the test: take the trunk
vertices in a thin band around the test point, project them onto the horizontal plane, and
ask whether the point is inside their convex hull. That is a closed 2D region by
construction, and a monotone chain gives it exactly.

Validated against a report rather than against itself: the sprint was described as having the
arm inside the body over frames 1-5, 10-19 and 22-25. A ruler that does not reproduce those
is not measuring what was seen.
"""
import sys

import bpy
import mathutils

ART = "C:/Users/jsull/Desktop/copaimo/dev/art"
sys.path.insert(0, ART)

GLB = "C:/Users/jsull/Desktop/copaimo/assets/models/person_ranger.glb"
SCALE = 170.0
BAND = 0.035          # how thick a slice of torso counts as "at this height"
# An arm's RADIUS, because what is sampled is its CENTRE LINE. At 0.012 (2 cm) the run
# measured only two marginal frames and read as clean, while the report was that both arms go
# into the body on the run as well - a centre line clearing the torso by 2 cm still has 2 cm
# of skin inside it. 0.024 is about 4 cm, which is what this character's forearm actually is.
CLEARS_BY = 0.024

bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=GLB)
rig = next(o for o in bpy.data.objects if o.type == "ARMATURE")

import prepare_rig

mesh = prepare_rig.the_body()
prepare_rig.reach_the_ends(rig, mesh)
across, forward, _ = prepare_rig.body_frame(rig)

TRUNK = ("Spine", "Waist", "Hip", "Pelvis", "Neck")
groups = {g.index: g.name for g in mesh.vertex_groups}


def owner(vertex):
    best, who = 0.0, ""
    for group in vertex.groups:
        if group.weight > best:
            best, who = group.weight, groups.get(group.group, "")
    return who


trunk = [v.index for v in mesh.data.vertices if any(p in owner(v) for p in TRUNK)]


def hull(points):
    """Convex hull of 2D points, monotone chain, counter-clockwise."""
    points = sorted(set(points))
    if len(points) < 3:
        return points

    def half(seq):
        out = []
        for p in seq:
            while len(out) >= 2:
                (ax, ay), (bx, by) = out[-2], out[-1]
                if (bx - ax) * (p[1] - ay) - (by - ay) * (p[0] - ax) > 0:
                    break
                out.pop()
            out.append(p)
        return out

    lower, upper = half(points), half(reversed(points))
    return lower[:-1] + upper[:-1]


def inside(point, ring, margin):
    """How far inside the ring the point is, along the nearest edge's inward normal."""
    if len(ring) < 3:
        return None
    worst = None
    for i, a in enumerate(ring):
        b = ring[(i + 1) % len(ring)]
        edge = mathutils.Vector((b[0] - a[0], b[1] - a[1]))
        if edge.length < 1e-9:
            continue
        # Inward normal of a counter-clockwise ring.
        normal = mathutils.Vector((-edge.y, edge.x)).normalized()
        gap = (mathutils.Vector((point[0] - a[0], point[1] - a[1]))).dot(normal)
        worst = gap if worst is None else min(worst, gap)
    return None if worst is None else worst - margin


print(f"torso: {len(trunk)} vertices\n")
print(f"{'clip':8} {'frames with an arm in the torso':46} deepest")
for clip in ("run", "sprint"):
    action = bpy.data.actions.get(clip)
    if action is None:
        continue
    if rig.animation_data is None:
        rig.animation_data_create()
    rig.animation_data.action = action
    if action.slots:
        rig.animation_data.action_slot = action.slots[0]

    hit, worst = set(), (0.0, 0, "")
    for frame in range(1, 26):
        bpy.context.scene.frame_set(frame)
        got = mesh.evaluated_get(bpy.context.evaluated_depsgraph_get()).to_mesh()
        spots = [mesh.matrix_world @ got.vertices[i].co for i in trunk]
        for side in "LR":
            elbow = rig.matrix_world @ rig.pose.bones[f"{side}_Forearm"].head
            wrist = rig.matrix_world @ rig.pose.bones[f"{side}_Hand"].head
            tip = rig.matrix_world @ rig.pose.bones[f"{side}_Hand"].tail
            for a, b in ((elbow, wrist), (wrist, tip)):
                for step in range(5):
                    at = a.lerp(b, step / 4.0)
                    slice2d = [
                        (p.dot(forward), p.dot(across))
                        for p in spots if abs(p.z - at.z) < BAND
                    ]
                    ring = hull(slice2d)
                    depth = inside((at.dot(forward), at.dot(across)), ring, CLEARS_BY)
                    if depth is not None and depth > 0.0:
                        hit.add(frame)
                        if -depth < worst[0]:
                            worst = (-depth, frame, side)
    shown = str(sorted(hit)) if hit else "none"
    print(f"{clip:8} {shown:46} {worst[0] * SCALE:+.2f} cm "
          f"{'at f' + str(worst[1]) + worst[2] if worst[1] else ''}")
