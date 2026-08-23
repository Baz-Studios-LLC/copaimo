"""What is actually wrong with the character's mesh, measured.

Written before touching anything, because "the mesh fixes" needs to be a list and not an
impression - and because this file's history is of confident fixes to mis-measured problems.

Reports, on the PREPARED rig (which is what the clips are built from, so it is the mesh the
game ships):

* left-right symmetry, vertex by vertex against its mirror partner
* how many disconnected shells there are, and how big
* degenerate faces, and vertices sitting on top of each other
* vertices no bone drives, which deform not at all
* custom split normals, because their presence is why welding is forbidden here

Read-only. Nothing is written, nothing is exported.
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
NEAR = 0.004          # partner search radius, in model units
SAME_SPOT = 1.0e-5    # closer than this and two vertices are on top of each other

bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=GLB)
rig = next(o for o in bpy.data.objects if o.type == "ARMATURE")

import prepare_rig

mesh = prepare_rig.the_body()
across, _, _ = prepare_rig.body_frame(rig)
data = mesh.data

print(f"mesh: {len(data.vertices)} vertices, {len(data.polygons)} polygons, "
      f"{len(mesh.vertex_groups)} vertex groups")
print(f"custom split normals: {data.has_custom_normals}"
      f"  (if true, NEVER weld - see the pipeline notes)")

spots = [mesh.matrix_world @ v.co for v in data.vertices]

# Which axis is lateral, taken from the body frame rather than assumed.
lateral = max(range(3), key=lambda i: abs(across[i]))
print(f"lateral axis is index {lateral} (from body_frame, not assumed)")

# --- Symmetry, against an EXACT nearest neighbour.
#
# The first version bucketed by the two non-lateral coordinates and searched a 3x3
# neighbourhood of 0.68 cm cells, while accepting a partner up to 2.7 cm away. Those two do
# not agree: a partner 2 cm off sits four cells away and was never looked at, so it was
# counted as missing and every median came out inflated. A search radius has to be at least
# as large as the answer you are willing to accept.
#
# A KD-tree has no radius to get wrong.
tree = mathutils.kdtree.KDTree(len(spots))
for i, p in enumerate(spots):
    tree.insert(p, i)
tree.balance()

ACCEPT = 0.03           # 5.1 cm - generous, so "no partner" means genuinely none


def mirror_gap(p):
    """How far the nearest vertex is from where this one's mirror image should be."""
    want = mathutils.Vector(p)
    want[lateral] = -want[lateral]
    _, _, gap = tree.find(want)
    return gap


offs = []
unpartnered = 0
midline = 0
for p in spots:
    if abs(p[lateral]) < NEAR:
        midline += 1
        continue
    gap = mirror_gap(p)
    if gap is None or gap > ACCEPT:
        unpartnered += 1
    else:
        offs.append(gap * SCALE)

offs.sort()
if offs:
    print(f"\nsymmetry: {len(offs)} vertices matched to a mirror partner, "
          f"{unpartnered} with none within {ACCEPT * SCALE:.1f} cm, {midline} on the midline")
    print(f"  median off by {offs[len(offs) // 2]:.3f} cm, "
          f"90th {offs[int(len(offs) * 0.9)]:.3f}, worst {offs[-1]:.3f}")
    print(f"  over 1 mm: {sum(1 for o in offs if o > 0.1)}"
          f"   over 5 mm: {sum(1 for o in offs if o > 0.5)}"
          f"   over 2 cm: {sum(1 for o in offs if o > 2.0)}")

# --- Shells: how many disconnected pieces, via union-find over edges.
parent = list(range(len(data.vertices)))


def find(a):
    while parent[a] != a:
        parent[a] = parent[parent[a]]
        a = parent[a]
    return a


for edge in data.edges:
    a, b = find(edge.vertices[0]), find(edge.vertices[1])
    if a != b:
        parent[a] = b
sizes = collections.Counter(find(i) for i in range(len(data.vertices)))
print(f"\nshells: {len(sizes)} disconnected pieces; "
      f"largest {sizes.most_common(1)[0][1]} verts, "
      f"{sum(1 for n in sizes.values() if n < 8)} of them under 8 verts")

# --- Degenerate faces and coincident vertices.
tiny = sum(1 for p in data.polygons if p.area < 1.0e-9)
print(f"\ndegenerate polygons (zero area): {tiny}")

seen = collections.defaultdict(int)
for p in spots:
    seen[(round(p.x / SAME_SPOT), round(p.y / SAME_SPOT), round(p.z / SAME_SPOT))] += 1
stacked = sum(n - 1 for n in seen.values() if n > 1)
print(f"vertices sharing a position with another: {stacked} "
      f"(expected - glTF splits them for hard edges)")

# --- Vertices no bone drives.
driven = {v.index for v in data.vertices if any(g.weight > 0.0 for g in v.groups)}
adrift = len(data.vertices) - len(driven)
print(f"\nvertices no bone drives: {adrift}")
if adrift:
    lowest = min((spots[i].z for i in range(len(data.vertices)) if i not in driven))
    highest = max((spots[i].z for i in range(len(data.vertices)) if i not in driven))
    print(f"  they sit between z {lowest * SCALE:.1f} and {highest * SCALE:.1f} cm")

# --- Weight hygiene: totals that do not sum to one deform oddly under motion.
off_total = []
for v in data.vertices:
    total = sum(g.weight for g in v.groups)
    if abs(total - 1.0) > 0.01:
        off_total.append(total)
print(f"\nvertices whose weights do not sum to 1: {len(off_total)}")
if off_total:
    off_total.sort()
    print(f"  from {off_total[0]:.3f} to {off_total[-1]:.3f}")

# --- WHERE the asymmetry is. A blanket symmetrise would be wrong: the jacket, its pockets
# and the shoulder logo are asymmetric ON PURPOSE, and the shoe's sculpted toe sweep is
# character that a previous pass was told never to touch. What matters is the asymmetry that
# causes measurable ANIMATION faults - shoes sitting differently on their bones, landmarks
# that have to be shared between the sides to stop a limp. So this splits the offsets by the
# bone that owns each vertex.
groups = {g.index: g.name for g in mesh.vertex_groups}


def owner(vertex):
    best, who = 0.0, ""
    for group in vertex.groups:
        if group.weight > best:
            best, who = group.weight, groups.get(group.group, "")
    return who


# Per part, using the same exact search.
by_part = collections.defaultdict(list)
for i, p in enumerate(spots):
    if abs(p[lateral]) < NEAR:
        continue
    gap = mirror_gap(p)
    name = owner(data.vertices[i])
    # Fold L_/R_ together so a limb is one row.
    part = name[2:] if name[:2] in ("L_", "R_") else name
    by_part[part].append((gap * SCALE) if gap is not None and gap <= ACCEPT else None)

print(f"\n{'part':20} {'verts':>6} {'no partner':>11} {'median cm':>10} {'worst cm':>9}")
rows = []
for part, offs in by_part.items():
    got = sorted(o for o in offs if o is not None)
    missing = sum(1 for o in offs if o is None)
    if not got:
        rows.append((99.0, part, len(offs), missing, None, None))
        continue
    rows.append((got[len(got) // 2], part, len(offs), missing, got[len(got) // 2], got[-1]))
for med, part, n, missing, m, worst in sorted(rows, reverse=True)[:14]:
    if m is None:
        print(f"{part:20} {n:6} {missing:11} {'-':>10} {'-':>9}")
    else:
        print(f"{part:20} {n:6} {missing:11} {m:10.3f} {worst:9.3f}")
