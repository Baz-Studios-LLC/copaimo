"""Measures how far a clip STRETCHES the skin, so a repair can be judged by numbers.

    "$BL" --background --python-exit-code 1 --python dev/art/ribbon_measure.py -- <glb> <clip> [<clip>...]

For every frame of each clip, the skin is evaluated for real — the actual armature
modifier, the actual exported weights — and each edge's length is compared with its
length at rest. Prints the worst edges by ABSOLUTE growth and by ratio, which piece
of cloth each is in, and the cross-limb weight bleed per piece.

# Why absolute growth as well as the ratio

Ratio alone sends you after phantoms. This mesh has edges 0.0017 long, and one of
those at fifteen times its rest length has moved 0.024 of a body height — invisible.
The ribbon is a 0.05-unit edge becoming 0.14. Both numbers are printed, and the
worst-by-growth list is the one that matches what the renders show.

# Why welded edges

The generator emits one vertex per UV corner, so this mesh has 7578 vertices on
about 2460 distinct positions and 1440 index-space islands. Index connectivity
therefore describes the UV atlas, not the surface: it reports a soup of fragments
where the surface is a couple of dozen clean garments. Everything here is measured
after welding by position, which is the only space in which "is this one piece of
cloth" has an answer.
"""

import sys
from collections import defaultdict

import bpy

ARM_BONES = ("Clavicle", "Upperarm", "Forearm", "Hand")
LEG_BONES = ("Thigh", "Calf", "Foot", "Toe")
WELD = 1e-5


def argv():
    return sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []


def limb_of(bone: str):
    if any(part in bone for part in ARM_BONES):
        return "arm"
    if any(part in bone for part in LEG_BONES):
        return "leg"
    return None


def welded(me):
    seen = {}
    ids = [0] * len(me.vertices)
    for v in me.vertices:
        spot = (round(v.co.x / WELD), round(v.co.y / WELD), round(v.co.z / WELD))
        ids[v.index] = seen.setdefault(spot, len(seen))
    return ids, len(seen)


def pieces(me, ids, spots):
    parent = list(range(spots))

    def root(x):
        while parent[x] != x:
            parent[x] = parent[parent[x]]
            x = parent[x]
        return x

    for e in me.edges:
        a, b = root(ids[e.vertices[0]]), root(ids[e.vertices[1]])
        if a != b:
            parent[a] = b
    return [root(s) for s in range(spots)]


def main():
    args = argv()
    if len(args) < 2:
        raise SystemExit("need <glb> <clip> [<clip>...]")
    src, clips = args[0], args[1:]

    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=src)

    rig = next((o for o in bpy.data.objects if o.type == "ARMATURE"), None)
    ob = max(
        (o for o in bpy.data.objects if o.type == "MESH" and o.vertex_groups),
        key=lambda o: len(o.data.vertices),
    )
    me = ob.data
    print(f"MESH {ob.name}: {len(me.vertices)} verts, {len(me.polygons)} faces")

    ids, spots = welded(me)
    piece_of_spot = pieces(me, ids, spots)
    piece_of = [piece_of_spot[ids[v.index]] for v in me.vertices]
    members = defaultdict(list)
    for v in me.vertices:
        members[piece_of[v.index]].append(v.index)
    print(f"WELD: {len(me.vertices)} verts -> {spots} positions -> {len(members)} pieces")

    # Weight sanity, and the cross-limb bleed per piece.
    named = {g.index: g.name for g in ob.vertex_groups}
    lightest, heaviest, most = 2.0, -1.0, 0
    held = {}
    for v in me.vertices:
        total = sum(g.weight for g in v.groups)
        lightest, heaviest = min(lightest, total), max(heaviest, total)
        most = max(most, len([g for g in v.groups if g.weight > 0.0]))
        arm = leg = 0.0
        for g in v.groups:
            fam = limb_of(named[g.group])
            if fam == "arm":
                arm += g.weight
            elif fam == "leg":
                leg += g.weight
        held[v.index] = (arm, leg)
    print(f"WEIGHTS: sums {lightest:.6f}..{heaviest:.6f}, max influences {most}")

    labels = {}
    for piece, verts in sorted(members.items(), key=lambda kv: -len(kv[1]))[:8]:
        arm = sum(held[i][0] for i in verts)
        leg = sum(held[i][1] for i in verts)
        lo = min(me.vertices[i].co.z for i in verts)
        hi = max(me.vertices[i].co.z for i in verts)
        labels[piece] = f"p{piece}({len(verts)}v z{lo:.2f}-{hi:.2f})"
        owner = "arm" if arm > leg else "leg"
        foreign = sum((held[i][1] if owner == "arm" else held[i][0]) > 0.01 for i in verts)
        print(
            f"  {labels[piece]:26s} arm {arm:7.1f} leg {leg:7.1f} -> {owner}, "
            f"{foreign} vert(s) carry the other limb"
        )

    # Welded edges, and which surviving index pair represents each.
    edges = {}
    for e in me.edges:
        a, b = ids[e.vertices[0]], ids[e.vertices[1]]
        if a == b:
            continue
        edges.setdefault((min(a, b), max(a, b)), (e.vertices[0], e.vertices[1]))
    print(f"EDGES: {len(me.edges)} index -> {len(edges)} welded")

    rest = {k: (me.vertices[i].co - me.vertices[j].co).length for k, (i, j) in edges.items()}

    for clip in clips:
        action = bpy.data.actions.get(clip)
        if action is None:
            print(f"CLIP {clip}: absent; have {[a.name for a in bpy.data.actions]}")
            continue
        if rig.animation_data is None:
            rig.animation_data_create()
        rig.animation_data.action = action
        try:
            if rig.animation_data.action_slot is None and action.slots:
                rig.animation_data.action_slot = action.slots[0]
        except AttributeError:
            pass
        lo, hi = (int(round(v)) for v in action.frame_range)

        peak = {k: 0.0 for k in edges}
        at = {k: lo for k in edges}
        for frame in range(lo, hi + 1):
            bpy.context.scene.frame_set(frame)
            deps = bpy.context.evaluated_depsgraph_get()
            ev = ob.evaluated_get(deps)
            posed = ev.to_mesh()
            co = [v.co.copy() for v in posed.vertices]
            ev.to_mesh_clear()
            for k, (i, j) in edges.items():
                length = (co[i] - co[j]).length
                if length > peak[k]:
                    peak[k] = length
                    at[k] = frame

        grew = sorted(edges, key=lambda k: -(peak[k] - rest[k]))
        ratio = sorted(edges, key=lambda k: -(peak[k] / max(rest[k], 1e-9)))
        over2 = sum(1 for k in edges if peak[k] > 2.0 * rest[k])
        over15 = sum(1 for k in edges if peak[k] > 1.5 * rest[k])
        print(f"\nCLIP {clip} frames {lo}..{hi}")
        print(
            f"  worst growth {peak[grew[0]] - rest[grew[0]]:.4f}  "
            f"worst ratio {peak[ratio[0]] / max(rest[ratio[0]], 1e-9):.2f}x  "
            f"edges >2x {over2}  >1.5x {over15}"
        )
        for title, order in (("BY GROWTH", grew), ("BY RATIO", ratio)):
            print(f"  {title}")
            for k in order[:6]:
                i, j = edges[k]
                where = (me.vertices[i].co + me.vertices[j].co) * 0.5
                print(
                    f"    {rest[k]:.4f} -> {peak[k]:.4f}  "
                    f"+{peak[k] - rest[k]:.4f} ({peak[k] / max(rest[k], 1e-9):.2f}x) "
                    f"f{at[k]:<3d} {labels.get(piece_of[i], 'p%d' % piece_of[i]):26s} "
                    f"at ({where.x:+.3f},{where.y:+.3f},{where.z:+.3f})"
                )


main()
