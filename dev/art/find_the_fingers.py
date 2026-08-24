"""Finds the fingers in the hand mesh, so bones can be built for them.

The rig has 41 bones and only `L_Hand` and `R_Hand` among them - no fingers at all. That is
why the hands read as flat splayed slabs in every clip and why no amount of wrist or palm
tuning has helped: the shape is baked into the mesh and nothing can pose it. It also means the
character cannot hold anything, which is the reason to fix it properly rather than keep
adjusting the angle of a slab.

This does the measuring only. Fingers are found without assuming how many there are:

* take the vertices the hand drives, and the hand's own long axis
* keep the DISTAL part, past the knuckles, where fingers are separate from the palm
* cluster those by position, so each lump of connected vertices is one digit
* report each cluster's base, tip, direction and length

A thumb should fall out of that as the cluster whose direction differs most from the rest, and
the count tells us whether the sculpt has four fingers and a thumb or something simpler.

Read-only.
"""
import collections
import math
import sys

import bpy
import mathutils

ART = "C:/Users/jsull/Desktop/copaimo/dev/art"
sys.path.insert(0, ART)

GLB = "C:/Users/jsull/Desktop/copaimo/assets/models/person_ranger.glb"
SCALE = 170.0

bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=GLB)
rig = next(o for o in bpy.data.objects if o.type == "ARMATURE")

import prepare_rig

mesh = prepare_rig.the_body()
prepare_rig.reach_the_ends(rig, mesh)
across, forward, up = prepare_rig.body_frame(rig)
data = mesh.data

groups = {g.index: g.name for g in mesh.vertex_groups}
owned = {"L": [], "R": []}
for vertex in data.vertices:
    best, who = 0.0, ""
    for group in vertex.groups:
        if group.weight > best:
            best, who = group.weight, groups.get(group.group, "")
    if "_Hand" in who:
        owned[who[0]].append(vertex.index)

print(f"hand vertices: L {len(owned['L'])}, R {len(owned['R'])}")

# Which vertices touch which, so a finger can be found as a connected lump.
touching = collections.defaultdict(set)
for edge in data.edges:
    a, b = edge.vertices
    touching[a].add(b)
    touching[b].add(a)

for side in "LR":
    wrist = rig.matrix_world @ rig.pose.bones[f"{side}_Hand"].head
    tip = rig.matrix_world @ rig.pose.bones[f"{side}_Hand"].tail
    axis = (tip - wrist).normalized()
    spots = {i: mesh.matrix_world @ data.vertices[i].co for i in owned[side]}
    along = {i: (p - wrist).dot(axis) for i, p in spots.items()}
    reach = max(along.values())

    # Past 55% of the hand's length is finger rather than palm - chosen so the knuckles stay
    # out of it, since the palm is one lump and would join every finger into one cluster.
    distal = {i for i, a in along.items() if a > reach * 0.55}
    print(f"\n{side}: hand reaches {reach * SCALE:.1f} cm; {len(distal)} vertices past the "
          f"knuckles")

    seen, lumps = set(), []
    for start in distal:
        if start in seen:
            continue
        lump, stack = [], [start]
        seen.add(start)
        while stack:
            here = stack.pop()
            lump.append(here)
            for there in touching[here]:
                if there in distal and there not in seen:
                    seen.add(there)
                    stack.append(there)
        lumps.append(lump)

    # Split vertices mean a finger can arrive as several lumps at the same place; merge any
    # whose centres are within a finger's width of each other.
    centres = [
        (sum((spots[i] for i in lump), mathutils.Vector()) / len(lump), lump)
        for lump in lumps
    ]
    merged = []
    for centre, lump in sorted(centres, key=lambda c: -len(c[1])):
        for i, (other, group) in enumerate(merged):
            if (centre - other).length * SCALE < 1.6:
                group.extend(lump)
                merged[i] = (
                    sum((spots[j] for j in group), mathutils.Vector()) / len(group),
                    group,
                )
                break
        else:
            merged.append((centre, list(lump)))

    print(f"  {len(lumps)} raw lumps, {len(merged)} after merging ones closer than 1.6 cm")
    print(f"  {'verts':>6} {'length':>7} {'from the hand axis':>19}  direction")
    for centre, group in sorted(merged, key=lambda c: -len(c[1])):
        if len(group) < 6:
            continue
        base = min(group, key=lambda i: along[i])
        end = max(group, key=lambda i: along[i])
        line = spots[end] - spots[base]
        off = math.degrees(line.normalized().angle(axis))
        print(f"  {len(group):6} {line.length * SCALE:6.1f}cm {off:18.1f}deg  "
              f"fwd {line.normalized().dot(forward):+.2f} "
              f"lat {line.normalized().dot(across):+.2f} "
              f"up {line.normalized().dot(up):+.2f}")
