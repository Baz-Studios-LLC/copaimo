"""Where does the character's skin TEAR when he moves? Measured per edge, per frame.

A tear is not a hole in the mesh - the topology never changes. It is an edge being stretched
or crushed by the skinning until the surface pulls apart visually, and it happens where a
vertex is weighted to bones that swing away from each other. So the measurement is: for every
edge, how far its DEFORMED length strays from its rest length, across every frame of every
clip, and which bones own the vertices at each end.

The bone pair is the useful half of the output. An edge stretched 3x between `L_Upperarm` and
`Spine02` is not a modelling problem, it is a WEIGHTING problem - the two ends are being
driven by joints that move apart - and it names its own fix.

Read-only. Nothing is written and nothing is exported.
"""
import collections
import sys

import bpy

ART = "C:/Users/jsull/Desktop/copaimo/dev/art"
sys.path.insert(0, ART)

GLB = "C:/Users/jsull/Desktop/copaimo/assets/models/person_ranger.glb"
SCALE = 170.0
TORN = 1.35            # stretched or crushed past this and it reads as a tear
SHOW = 12

bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=GLB)
rig = next(o for o in bpy.data.objects if o.type == "ARMATURE")

import prepare_rig

mesh = prepare_rig.the_body()
data = mesh.data

groups = {g.index: g.name for g in mesh.vertex_groups}


def owner(vertex):
    best, who = 0.0, ""
    for group in vertex.groups:
        if group.weight > best:
            best, who = group.weight, groups.get(group.group, "")
    return who


owners = [owner(v) for v in data.vertices]

# Rest lengths, from the bind - which is what the skin is stretched away FROM.
rest = {}
for edge in data.edges:
    a, b = edge.vertices
    length = (data.vertices[a].co - data.vertices[b].co).length
    if length > 1.0e-6:
        rest[(a, b)] = length
print(f"{len(rest)} edges with a measurable rest length, of {len(data.edges)}")

worst = {}
by_pair = collections.Counter()
for clip in ("idle", "walk", "run", "sprint"):
    action = bpy.data.actions.get(clip)
    if action is None:
        continue
    if rig.animation_data is None:
        rig.animation_data_create()
    rig.animation_data.action = action
    if action.slots:
        rig.animation_data.action_slot = action.slots[0]
    lo, hi = (int(round(v)) for v in action.frame_range)

    for frame in range(lo, hi + 1):
        bpy.context.scene.frame_set(frame)
        got = mesh.evaluated_get(bpy.context.evaluated_depsgraph_get()).to_mesh()
        spots = [v.co.copy() for v in got.vertices]
        for (a, b), was in rest.items():
            now = (spots[a] - spots[b]).length / was
            strain = max(now, 1.0 / now) if now > 1.0e-6 else 99.0
            if (a, b) not in worst or strain > worst[(a, b)][0]:
                worst[(a, b)] = (strain, clip, frame)

for (a, b), (strain, clip, frame) in worst.items():
    if strain >= TORN:
        pair = tuple(sorted((owners[a], owners[b])))
        by_pair[pair] += 1

torn = [(s, e) for e, (s, *_) in worst.items() if s >= TORN]
print(f"\nedges straying past x{TORN}: {len(torn)} of {len(rest)} "
      f"({len(torn) / len(rest) * 100:.2f}%)")

ranked = sorted(((s, e) for e, (s, *_) in worst.items()), reverse=True)[:SHOW]
print(f"\n{'strain':>7} {'clip':8} {'fr':>3}  edge owners")
for strain, edge in ranked:
    s, clip, frame = worst[edge]
    a, b = edge
    print(f"{strain:7.2f} {clip:8} {frame:3d}  {owners[a]} <-> {owners[b]}")

print(f"\ntorn edges by the bone pair driving them:")
for pair, count in by_pair.most_common(SHOW):
    same = " (same bone - a modelling matter, not weighting)" if pair[0] == pair[1] else ""
    print(f"  {count:5d}  {pair[0]} <-> {pair[1]}{same}")
