"""Diffs two builds of the same character: geometry, weights, clips.

    "$BL" --background --python-exit-code 1 --python dev/art/compare_skin.py -- <before.glb> <after.glb>

Weights are compared by joint NAME, never by joint index: the two files store the
armature in different orders, so an index-wise comparison mislabels every bone.
"""

import sys

import bpy

ARM = ("Clavicle", "Upperarm", "Forearm", "Hand")
LEG = ("Thigh", "Calf", "Foot", "Toe")


def limb_of(b):
    if any(p in b for p in ARM):
        return "arm"
    if any(p in b for p in LEG):
        return "leg"
    return None


def load(path):
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=path)
    ob = max(
        (o for o in bpy.data.objects if o.type == "MESH" and o.vertex_groups),
        key=lambda o: len(o.data.vertices),
    )
    named = {g.index: g.name for g in ob.vertex_groups}
    weights = [
        {named[e.group]: e.weight for e in v.groups if e.weight > 0.0} for v in ob.data.vertices
    ]
    return (
        [v.co.copy() for v in ob.data.vertices],
        weights,
        len(ob.data.polygons),
        sorted(a.name for a in bpy.data.actions),
        {b.name: b.head_local.copy() for b in next(
            o for o in bpy.data.objects if o.type == "ARMATURE").data.bones},
    )


a, b = sys.argv[sys.argv.index("--") + 1 :][:2]
co_a, w_a, f_a, clips_a, bones_a = load(a)
co_b, w_b, f_b, clips_b, bones_b = load(b)

print(f"verts {len(co_a)} -> {len(co_b)}   faces {f_a} -> {f_b}")
print(f"clips {clips_a}\n   -> {clips_b}")
if len(co_a) != len(co_b):
    raise SystemExit("vertex counts differ; nothing further is comparable")

moved = max((x - y).length for x, y in zip(co_a, co_b))
print(f"largest vertex MOVEMENT: {moved:.9f}")
bone_moved = max((bones_a[n] - bones_b[n]).length for n in bones_a if n in bones_b)
print(f"bones {len(bones_a)} -> {len(bones_b)}, largest bone head movement {bone_moved:.9f}")

changed = []
for i, (x, y) in enumerate(zip(w_a, w_b)):
    keys = set(x) | set(y)
    gap = max(abs(x.get(k, 0.0) - y.get(k, 0.0)) for k in keys)
    if gap > 1e-4:
        changed.append((gap, i))
print(f"vertices whose WEIGHTS changed: {len(changed)} of {len(co_a)}")

# Which bones lost or gained influence overall.
totals = {}
for x, y in zip(w_a, w_b):
    for k in set(x) | set(y):
        totals[k] = totals.get(k, 0.0) + (y.get(k, 0.0) - x.get(k, 0.0))
print("joint total weight shift (|shift| > 0.5):")
for name, shift in sorted(totals.items(), key=lambda kv: -abs(kv[1])):
    if abs(shift) > 0.5:
        print(f"    {name:22s} {shift:+8.2f}   ({limb_of(name)})")

# Where the changed vertices are, so the change can be shown to be local.
if changed:
    zs = [co_a[i].z for _g, i in changed]
    ys = [co_a[i].y for _g, i in changed]
    xs = [co_a[i].x for _g, i in changed]
    print(
        f"changed vertices live in x {min(xs):+.3f}..{max(xs):+.3f} "
        f"y {min(ys):+.3f}..{max(ys):+.3f} z {min(zs):+.3f}..{max(zs):+.3f}"
    )
    still = sum(
        1
        for _g, i in changed
        if min(
            sum(v for k, v in w_b[i].items() if limb_of(k) == "arm"),
            sum(v for k, v in w_b[i].items() if limb_of(k) == "leg"),
        )
        > 1e-6
    )
    print(f"of those, {still} still hold weight on both limb chains")
