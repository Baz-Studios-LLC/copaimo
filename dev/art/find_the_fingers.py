"""Separates the hand mesh into digits, so a bone chain can be built down each one.

The rig has 41 bones and, among them, `L_Hand` and `R_Hand` and nothing past the wrist. That
is why the hands read as flat splayed slabs in every clip, and why no amount of wrist or palm
tuning helped: the splay is baked into the mesh and there is nothing to pose it with. It also
means the character cannot hold, lift, grab or pet anything, which is the reason to fix it
properly rather than keep adjusting the angle of a slab.

Rendering the hand on its own settled the question that decides whether any of this is
possible: the sculpt has FOUR FINGERS AND A THUMB, separated, each a distinct tube. Bones
cannot pose a painted-on groove, but they can pose these.

# Finding them, and three attempts it took

CLUSTER BY EDGE CONNECTIVITY: 22 lumps per hand merging to 6 by proximity, candidate lengths
1.1 to 5.6 cm. Unusable. glTF splits a vertex wherever the normals are hard, so one finger
arrives as several pieces sharing positions but no edges, and merging them afterwards by
distance is guesswork.

WELD BY POSITION FIRST, then cut at a fraction of the hand's length along its own axis. The
weld is right and worth keeping - 309 vertices become the 95 real ones, and each finger is one
piece again. The cut is not: sweeping it from 30% to 84% of reach, the count of separate lumps
rises to FOUR and never five. The reason is the thumb. It leaves the hand near the wrist and
points across the palm rather than down it, so it barely advances along the axis the fingers
run down, and any cut deep enough to clear the palm has already discarded it.

MEASURE ALONG THE SURFACE, AND CLAIM EACH DIGIT WHEN IT SEPARATES. Distance is now geodesic -
step by step through the mesh from the wrist - under which the thumb IS far away, because the
distance runs down the thumb rather than across the hand. And there is no single radius that
suits every digit anyway: the thumb has separated and is running out of tip while the fingers
are still joined at the knuckles. So the radius sweeps, and a digit is claimed at the radius
where it FIRST comes away, which is where most of it survives. Five claims, five digits, no
threshold anybody had to pick.

Read-only. Refuses rather than report a bad split, because the bone builder downstream has no
business running on one.
"""
import collections
import heapq
import math
import os
import sys

import bpy
import mathutils

ART = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, ART)

SCALE = 170.0                  # centimetres per model unit
WELD_WITHIN = 0.00002          # 3.4 microns at model scale: split copies, never neighbours
A_REAL_LUMP = 4                # welded nodes; below this it is a stray, not a digit
DIGITS = 5                     # four fingers and a thumb, confirmed by looking
STEPS = 60                     # radii tried between the wrist and the far end

# A digit has to be at least this long to be believed. The shortest real one is the thumb;
# anything under this is a bump on the palm.
A_DIGIT_IS_AT_LEAST = 2.0      # cm


def refuse(why):
    raise SystemExit(f"REFUSED: {why}")


def hand_vertices(mesh, side):
    """The vertices the hand drives, by their strongest weight."""
    groups = {g.index: g.name for g in mesh.vertex_groups}
    mine = []
    for vertex in mesh.data.vertices:
        best, who = 0.0, ""
        for group in vertex.groups:
            if group.weight > best:
                best, who = group.weight, groups.get(group.group, "")
        if who == f"{side}_Hand":
            mine.append(vertex.index)
    return mine


def welded(mesh, wanted):
    """Maps each wanted vertex to a canonical index shared by every copy at its position.

    The export splits vertices for hard normals, so a finger is several components that touch
    nowhere. Welding by POSITION restores the connectivity the sculpt has and the file does
    not - the same move that turned 1362 apparent boundary loops into the 10 real ones.
    """
    canon, seen = {}, {}
    for i in wanted:
        co = mesh.data.vertices[i].co
        key = (round(co.x / WELD_WITHIN), round(co.y / WELD_WITHIN),
               round(co.z / WELD_WITHIN))
        canon[i] = seen.setdefault(key, i)
    return canon


def graph(mesh, canon):
    """Who touches whom, between welded nodes."""
    among = set(canon)
    touching = collections.defaultdict(set)
    for edge in mesh.data.edges:
        a, b = edge.vertices
        if a in among and b in among:
            ca, cb = canon[a], canon[b]
            if ca != cb:
                touching[ca].add(cb)
                touching[cb].add(ca)
    return touching


def surface_distance(spots, touching, seed):
    """Distance from `seed` to every node, stepping along the mesh rather than through it.

    This is what lets the thumb be found. Straight-line distance from the wrist, or advance
    along the hand's long axis, both put the thumb tip nearer than the finger knuckles, so no
    cut separates it. Along the SURFACE the thumb is properly far away, because the route
    there goes down the thumb.
    """
    far = {node: math.inf for node in spots}
    far[seed] = 0.0
    queue = [(0.0, seed)]
    while queue:
        here_far, here = heapq.heappop(queue)
        if here_far > far[here]:
            continue
        for there in touching[here]:
            step = here_far + (spots[there] - spots[here]).length
            if step < far[there]:
                far[there] = step
                heapq.heappush(queue, (step, there))
    return far


def span(digit, spots):
    """How long a branch is, tip to base, in centimetres."""
    here = [spots[c] for c in digit["nodes"]]
    return max((a - b).length for a in here for b in here) * SCALE if len(here) > 1 else 0.0


def lumps_past(cut, nodes, far, touching):
    """Connected components of everything further along the surface than `cut`."""
    past = {n for n in nodes if far[n] > cut}
    seen, found = set(), []
    for start in past:
        if start in seen:
            continue
        lump, stack = [], [start]
        seen.add(start)
        while stack:
            here = stack.pop()
            lump.append(here)
            for there in touching[here]:
                if there in past and there not in seen:
                    seen.add(there)
                    stack.append(there)
        found.append(lump)
    return [lump for lump in found if len(lump) >= A_REAL_LUMP]


def separate(rig, mesh, side, talk=True):
    """The digits of one hand. Each is a list of welded vertex indices, base first."""
    mine = hand_vertices(mesh, side)
    canon = welded(mesh, mine)
    nodes = set(canon.values())
    spots = {c: mesh.matrix_world @ mesh.data.vertices[c].co for c in nodes}
    touching = graph(mesh, canon)

    wrist = rig.matrix_world @ rig.pose.bones[f"{side}_Hand"].head
    seed = min(nodes, key=lambda c: (spots[c] - wrist).length)
    far = surface_distance(spots, touching, seed)
    # Nodes the surface never reaches are strays - a detached scrap of glove, say - and they
    # would otherwise read as an extra digit at every radius.
    stranded = [c for c in nodes if math.isinf(far[c])]
    if stranded:
        nodes -= set(stranded)
    reach = max(far[c] for c in nodes)

    if talk:
        print(f"\n{side}: {len(mine)} vertices weld to {len(nodes)}"
              f"{f' ({len(stranded)} stranded, set aside)' if stranded else ''}; "
              f"the surface runs {reach * SCALE:.1f} cm from the wrist")

    # Sweep INWARD from the fingertips and freeze each digit where it would join its
    # neighbour. There is no one radius that suits all five - the thumb has separated and is
    # losing its tip while the fingers are still joined at the knuckles - so this tracks each
    # branch instead: it appears alone at some radius, grows as the radius drops, and stops the
    # moment its component swallows a second branch. That freeze point is the knuckle, so each
    # digit comes out whole.
    #
    # Sweeping outward instead claimed the entire hand as one digit at the first radius tried,
    # which is what a component test says when nothing has branched yet.
    growing, frozen, palm = [], [], set()
    for step in range(STEPS - 1, 0, -1):
        cut = reach * step / STEPS
        for lump in lumps_past(cut, nodes, far, touching):
            here = set(lump)
            if here & palm:
                # Below the knuckle everything is palm, and it stays palm. Without this the
                # merged blob went on growing as a fresh branch and finished as a 94-node
                # "digit" 17.4 cm long, which is the whole hand.
                palm |= here
                continue
            mine = [d for d in growing if here & d["nodes"]]
            if not mine:
                growing.append({"nodes": here, "cut": cut})     # a new fingertip
            elif len(mine) == 1:
                mine[0]["nodes"], mine[0]["cut"] = here, cut     # the same digit, longer
            else:
                # Two branches in one component: this radius is past the knuckle between
                # them. A stub too short to be a digit is absorbed rather than frozen, so a
                # bump on the palm cannot cut a real finger short.
                real = [d for d in mine if span(d, spots) >= A_DIGIT_IS_AT_LEAST]
                if len(real) <= 1:
                    keep = max(mine, key=lambda d: span(d, spots))
                    for other in mine:
                        if other is not keep:
                            growing.remove(other)
                    keep["nodes"], keep["cut"] = here, cut
                else:
                    for one in mine:
                        growing.remove(one)
                        frozen.append(one)
                    palm |= here
    frozen.extend(growing)
    claimed = [(one["cut"], sorted(one["nodes"])) for one in frozen]

    # Order them the way a hand is read: across the palm, so index to little with the thumb at
    # one end, rather than in the order the sweep happened to find them.
    across, _forward, _up = prepare_rig.body_frame(rig)
    claimed.sort(key=lambda one: sum(spots[c].dot(across) for c in one[1]) / len(one[1]))

    kept = []
    for cut, lump in claimed:
        base = min(lump, key=lambda c: far[c])
        tip = max(lump, key=lambda c: far[c])
        length = (spots[tip] - spots[base]).length * SCALE
        good = length >= A_DIGIT_IS_AT_LEAST
        if talk:
            print(f"    {len(lump):3} nodes, {length:5.1f} cm, came away at "
                  f"{cut * SCALE:5.1f} cm  {'' if good else '<- too short, dropped'}")
        if good:
            kept.append(sorted(lump, key=lambda c: far[c]))

    if len(kept) != DIGITS:
        refuse(f"the {side} hand separated into {len(kept)} digits, not {DIGITS}")
    return kept, spots, far


def main():
    glb = os.environ.get(
        "FINGER_GLB", os.path.join(ART, "ranger_apose.glb").replace("\\", "/"))
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=glb)
    rig = next(o for o in bpy.data.objects if o.type == "ARMATURE")

    mesh = prepare_rig.the_body()
    prepare_rig.reach_the_ends(rig, mesh)
    across, forward, up = prepare_rig.body_frame(rig)

    for side in "LR":
        digits, spots, far = separate(rig, mesh, side)
        print(f"  {'nodes':>6} {'length':>8}  where it points")
        for lump in digits:
            base, tip = spots[lump[0]], spots[lump[-1]]
            line = tip - base
            way = line.normalized() if line.length else mathutils.Vector()
            print(f"  {len(lump):6} {line.length * SCALE:7.1f}cm  "
                  f"fwd {way.dot(forward):+.2f} lat {way.dot(across):+.2f} "
                  f"up {way.dot(up):+.2f}")


import prepare_rig  # noqa: E402  (after sys.path, and only needed once bpy is up)

if __name__ == "__main__":
    main()
