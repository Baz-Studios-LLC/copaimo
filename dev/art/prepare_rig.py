"""Turns the delivered character into a rig fit to animate, reproducibly.

    blender --background --python prepare_rig.py -- <source.glb> <out.glb>

Every number in here was measured on the asset, in Blender, and every step ends in a check
that will REFUSE rather than write a rig that fails it. What follows is what was wrong and
how each of them is dealt with, because none of it was guessable.

# What arrived, measured

    the two sides were never mirrors     shins 9.6 deg apart, feet 12.6 deg apart, the
                                         left ankle 5.7 cm behind the right
    the legs were bent at rest           thigh 6.0 deg back, shin 23.3, a 17.5 deg crouch
    leaf bones had invented lengths      Head 2.6 cm, Hand 25.4 (longer than the forearm),
                                         ToeBase 6.1 inside a shoe 29.4 long
    Root and Hip were 84.9 cm each       directions invented too - Root flat along the
                                         floor, Hip angled up and out through the back
    the mesh was 1442 disconnected       7582 verts, 7475 non-manifold edges, largest
      shells                             shell 37 verts
    every joint wore a sphere widget     41 of 41 bones, so the skeleton was unreadable
    the character stood 5.7 cm under     the floor, in its own bind pose

The asymmetry is the one that mattered most. It lives in the REST pose, so every clip
inherits it, which is why "the shins and feet flare out" survived being fixed several
times: each fix was a per-frame correction for a constant, and correcting a constant per
pose is what twisted the feet.

# Two lessons about method, which cost more than any of the above

A GUARD MUST COMPARE AGAINST THE SPECIFICATION, NOT AGAINST ITS OWN INPUT. The bake step
was checked by comparing the result to the shape fed into it. When the input turned out to
be wrong the check passed happily and wrote a mesh in one pose bound to a skeleton in
another. Every check below is absolute: soles at zero, arms at 45 degrees, sides mirrored
to within half a millimetre.

AND THE BUILD MUST RUN FROM THE SOURCE, NOT FROM A SESSION. All of this was first done by
hand across a dozen probes in a live Blender that a person was also clicking in, and the
A-pose quietly went missing somewhere in the middle. Whatever cleared it, a shared session
cannot be a substrate for a multi-step build. That is what this file is for.

# Nothing here needs the mesh to be re-weighted

Bone LENGTH is stored apart from `matrix_local`, so extending a leaf bone along its own
axis leaves the skinning basis untouched - checked, 0.000000 degrees. REDIRECTING a tail
does rotate matrix_local, which IS the skinning basis, and doing that to `Hip` earlier tore
this mesh into spikes. Only Root and Hip are redirected here, and only because both were
measured to drive zero vertices.
"""

import math
import os
import sys

import bmesh
import bpy
import mathutils

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import unfuse  # noqa: E402  (the cross-limb weight repair, shared with animate_ranger)

# --- The A-pose. Arms halfway through the shoulder's range rather than at either end,
# so the weights are wrong by half as much at both extremes as a T-pose leaves them.
ARMS_OUT = 45.0
LEGS_OUT = 3.0
# Zero: the feet point straight from the leg. 7 degrees of toe-out is anatomically
# ordinary, but on this character's oversized shoes it read as flare from every angle
# and the user called it twice. Straight is the read that works on the sculpt.
TOE_OUT = 0.0
# The knee's ease: thigh aimed this many degrees forward and the calf the same amount
# back, so the knee rests bent 2x this, folding FORWARD. Not a style choice - the first
# bake made the legs perfectly straight and every knee froze at exactly 0.0000 through
# whole clips: a dead-straight two-bone chain is singular to an IK solver, which cannot
# tell from it which way the joint folds. The standard fix is a slight bend in the bind.
# 2.0, so the knee rests bent 4 degrees rather than 8. The ease exists only to tell an
# IK solver which way the joint folds - a dead-straight chain is singular and froze
# every knee at exactly 0.0000 - and 4 degrees does that just as well as 8. The 8 was
# costing real posture: it seats the knee forward in the BIND, so every stance pose
# inherited a forward knee and the thigh could never reach behind vertical.
KNEE_EASE = 2.0

# --- Tolerances. Each one is the point past which a person would see it.
MIRRORED_WITHIN = 0.0005       # 0.5 mm
AIMED_WITHIN = 0.01            # degrees
BASIS_MUST_NOT_MOVE = 1e-4     # degrees
ON_THE_FLOOR_WITHIN = 0.0001   # 0.1 mm at model scale
WELD_MAY_MOVE = 0.000001       # a micron of model scale
SHAPE_KEPT_WITHIN = 0.0001

# Weights below this move a vertex too little for where its bone is to matter.
MATTERS = 0.05
ITS_OWN = 0.5
NEARLY_ALL = 0.98

SCALE = 170.0  # centimetres per model unit, for readable reports only


def argv():
    return sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []


def refuse(why):
    raise SystemExit(f"REFUSED: {why}")


# ----------------------------------------------------------------------------- the frame


def body_frame(rig):
    """Across and forward, from every left/right bone pair at once.

    Taking either from a single bone makes that bone read as perfect by construction and
    dumps the whole error onto its partner: the first version of this measurement derived
    forward from `R_ToeBase` and duly reported the right foot at 0.0 degrees of toe-out
    and the left at 16.0, which said nothing about the character and everything about the
    axis. Using every pair uses both sides equally and is immune to however the asset
    happens to be yawed in the scene.
    """
    spread = mathutils.Vector((0.0, 0.0, 0.0))
    for bone in rig.data.bones:
        partner = f"R_{bone.name[2:]}"
        if bone.name.startswith("L_") and partner in rig.data.bones:
            spread += (rig.matrix_world @ bone.matrix_local.translation) - (
                rig.matrix_world @ rig.data.bones[partner].matrix_local.translation
            )
    spread.z = 0.0
    across = spread.normalized()
    up = mathutils.Vector((0.0, 0.0, 1.0))
    return across, across.cross(up).normalized(), up


def mirror_pairs(rig):
    return sorted(
        bone.name[2:]
        for bone in rig.data.bones
        if bone.name.startswith("L_") and f"R_{bone.name[2:]}" in rig.data.bones
    )


def rest_head(rig, name):
    return rig.matrix_world @ rig.data.bones[name].matrix_local.translation


def rest_tail(rig, name):
    bone = rig.data.bones[name]
    return rig.matrix_world @ (
        bone.matrix_local @ mathutils.Vector((0.0, bone.length, 0.0))
    )


def sole_of(mesh):
    """The lowest point of the DEFORMED mesh - what actually touches the floor."""
    evaluated = mesh.evaluated_get(bpy.context.evaluated_depsgraph_get())
    baked = evaluated.to_mesh()
    try:
        return min((evaluated.matrix_world @ v.co).z for v in baked.vertices)
    finally:
        evaluated.to_mesh_clear()


def deformed(mesh):
    evaluated = mesh.evaluated_get(bpy.context.evaluated_depsgraph_get())
    baked = evaluated.to_mesh()
    try:
        return [v.co.copy() for v in baked.vertices]
    finally:
        evaluated.to_mesh_clear()


def rest_the_pose(rig):
    for posed in rig.pose.bones:
        posed.rotation_mode = "QUATERNION"
        posed.rotation_quaternion = (1.0, 0.0, 0.0, 0.0)
        posed.location = (0.0, 0.0, 0.0)
        posed.scale = (1.0, 1.0, 1.0)
    bpy.context.view_layer.update()


def in_edit_mode(rig):
    class Editing:
        def __enter__(self):
            self.was = bpy.context.view_layer.objects.active
            bpy.context.view_layer.objects.active = rig
            bpy.ops.object.mode_set(mode="EDIT")
            return rig.data.edit_bones

        def __exit__(self, *_):
            bpy.ops.object.mode_set(mode="OBJECT")
            bpy.context.view_layer.objects.active = self.was
            bpy.context.view_layer.update()

    return Editing()


# ------------------------------------------------------------------------------ the work


def drop_the_widgets(rig):
    """Every bone wore a sphere, so the skeleton could not be read.

    The armature's `display_type` was already OCTAHEDRAL - a custom shape overrides it, and
    all 41 bones carried one. Hiding the Icosphere OBJECT does nothing, because the bones
    reference the mesh DATABLOCK and that renders whether or not an object uses it.
    """
    cleared = 0
    for posed in rig.pose.bones:
        if posed.custom_shape is not None:
            posed.custom_shape = None
            cleared += 1
    rig.data.display_type = "OCTAHEDRAL"
    rig.show_in_front = True
    for obj in list(bpy.data.objects):
        if obj.type == "MESH" and not obj.vertex_groups:
            bpy.data.objects.remove(obj, do_unlink=True)
    print(f"  dropped {cleared} sphere widgets")


def make_the_sides_mirrors(rig):
    """Averages each left/right pair across the body's midline.

    Neither side is the correct one, so taking the right leg as truth would move the left
    twice as far as it needs to go and shift the whole character sideways. Each pair is
    replaced by its own average: reflect the left, average with the right, send half back.

    Rolls last. A pair with mirrored ends but unmirrored roll still hands its children a
    flipped frame, and with `use_rotation` off an IK solver leaves a shin's roll to the
    bone - which is how one foot came to point backwards through most of a run.
    """
    rest_the_pose(rig)
    pairs = mirror_pairs(rig)
    across, _, _ = body_frame(rig)
    centre = mathutils.Vector((0.0, 0.0, 0.0))
    for part in pairs:
        centre += (rest_head(rig, f"L_{part}") + rest_head(rig, f"R_{part}")) / 2.0
    plane = (centre / len(pairs)).dot(across)

    def reflect(spot):
        return spot - across * (2.0 * (spot.dot(across) - plane))

    def gap(part):
        return max(
            (reflect(rest_head(rig, f"L_{part}")) - rest_head(rig, f"R_{part}")).length,
            (reflect(rest_tail(rig, f"L_{part}")) - rest_tail(rig, f"R_{part}")).length,
        )

    was = max(gap(part) for part in pairs)

    wanted = {}
    for part in pairs:
        for which, get in (("head", rest_head), ("tail", rest_tail)):
            middle = (reflect(get(rig, f"L_{part}")) + get(rig, f"R_{part}")) / 2.0
            wanted[(f"R_{part}", which)] = middle
            wanted[(f"L_{part}", which)] = reflect(middle)

    inverse = rig.matrix_world.inverted()
    turn = rig.matrix_world.to_3x3()
    with in_edit_mode(rig) as edit:
        for part in pairs:
            for side in "LR":
                bone = edit[f"{side}_{part}"]
                bone.head = inverse @ wanted[(f"{side}_{part}", "head")]
                bone.tail = inverse @ wanted[(f"{side}_{part}", "tail")]
        for part in pairs:
            z = turn @ edit[f"R_{part}"].matrix.to_3x3().col[2].normalized()
            edit[f"L_{part}"].align_roll(
                turn.inverted() @ (z - across * (2.0 * z.dot(across)))
            )

    now = max(gap(part) for part in pairs)
    print(f"  worst mirror gap {was * SCALE:.2f} cm -> {now * SCALE:.4f} cm")
    if now > MIRRORED_WITHIN:
        refuse(f"the sides are still {now * SCALE:.2f} cm from mirrored")


def owned_reach(rig, mesh, name):
    """How far along its own axis the geometry a bone drives extends.

    At the 98th percentile, not the furthest vertex: this mesh has weights that cross the
    body, so a single stray one would set a bone's length on its own. Where the two agree
    the geometry really does reach that far.
    """
    index = mesh.vertex_groups.get(name)
    if index is None:
        return None
    index = index.index
    bone = rig.data.bones[name]
    head = rest_head(rig, name)
    along = (rig.matrix_world.to_3x3() @ bone.matrix_local.to_3x3().col[1]).normalized()
    far = []
    for vertex in mesh.data.vertices:
        for group in vertex.groups:
            if group.group == index and group.weight >= ITS_OWN:
                far.append(((mesh.matrix_world @ vertex.co) - head).dot(along))
                break
    if not far:
        return None
    far.sort()
    return far[min(len(far) - 1, int(len(far) * NEARLY_ALL))]


def centre_the_skeleton(rig, mesh):
    """Puts the skeleton's midline on the mesh's, and the central bones on that midline.

    Measured on the delivered rig: the spine chain sat 1.67 cm to one side of the mesh's
    own middle, and 0.63 cm off even the midline its own limb pairs define. Seen head-on
    that is a skeleton visibly off to the right of the body it drives - reported exactly
    that way - and it means every spine rotation swings the torso about an axis that is
    not the torso's.

    Two moves, both pure lateral translation, so no bone changes direction and no
    skinning basis rotates:

      the CENTRAL bones (root, hips, spine, neck, head) go onto the midline the limb
        pairs define, because that is what "central" means;
      then EVERY bone shifts together so that midline lands on the mesh's own silhouette
        centre, which keeps the mirroring intact - a rigid shift cannot break it.

    The mesh is not touched. Its own left-right asymmetry is a sculpting matter; this
    only stops the skeleton adding to it.
    """
    MIDDLE = ("Root", "Hip", "Pelvis", "Waist", "Spine01", "Spine02",
              "NeckTwist01", "NeckTwist02", "Head")

    across, _, _ = body_frame(rig)
    pairs = mirror_pairs(rig)
    midline = 0.0
    for part in pairs:
        midline += (rest_head(rig, f"L_{part}").dot(across)
                    + rest_head(rig, f"R_{part}").dot(across)) / 2.0
    midline /= len(pairs)

    spots = [mesh.matrix_world @ v.co for v in mesh.data.vertices]
    sideways = [p.dot(across) for p in spots]
    mesh_middle = (max(sideways) + min(sideways)) / 2.0

    spine_was = sum(
        rest_head(rig, n).dot(across) for n in MIDDLE if n in rig.data.bones
    ) / sum(1 for n in MIDDLE if n in rig.data.bones)
    print(f"  the limb midline is at {midline * SCALE:+.2f} cm, the mesh's middle at "
          f"{mesh_middle * SCALE:+.2f}, the central bones at {spine_was * SCALE:+.2f}")

    inverse = rig.matrix_world.inverted()
    turn = rig.matrix_world.to_3x3()
    with in_edit_mode(rig) as edit:
        # The central bones onto the limb midline.
        for name in MIDDLE:
            bone = edit.get(name)
            if bone is None:
                continue
            for which in ("head", "tail"):
                spot = turn @ getattr(bone, which) + rig.matrix_world.translation
                setattr(bone, which,
                        inverse @ (spot - across * (spot.dot(across) - midline)))
        # Then everything together onto the mesh's middle.
        shift = across * (mesh_middle - midline)
        for bone in edit:
            bone.head = bone.head + (turn.inverted() @ shift)
            bone.tail = bone.tail + (turn.inverted() @ shift)

    midline_now = 0.0
    for part in pairs:
        midline_now += (rest_head(rig, f"L_{part}").dot(across)
                        + rest_head(rig, f"R_{part}").dot(across)) / 2.0
    midline_now /= len(pairs)
    spine_now = sum(
        rest_head(rig, n).dot(across) for n in MIDDLE if n in rig.data.bones
    ) / sum(1 for n in MIDDLE if n in rig.data.bones)
    print(f"  now: limb midline {midline_now * SCALE:+.3f} cm, central bones "
          f"{spine_now * SCALE:+.3f}, mesh middle {mesh_middle * SCALE:+.3f}")
    if abs(spine_now - mesh_middle) * SCALE > 0.2:
        refuse(f"the central bones are {abs(spine_now - mesh_middle) * SCALE:.2f} cm "
               "off the mesh's middle")
    if abs(midline_now - mesh_middle) * SCALE > 0.2:
        refuse(f"the limb midline is {abs(midline_now - mesh_middle) * SCALE:.2f} cm "
               "off the mesh's middle")



def reach_the_ends(rig, mesh):
    """Makes each leaf bone as long as the thing it drives, both sides alike.

    glTF stores joint POSITIONS and no lengths. An interior bone gets its length for free
    because its tail is its child's head; a LEAF has no child, so the importer invents one.
    That is why nothing pivoted at the toe: the bone stopped 11.5 cm behind the front of
    the shoe, so there was no joint where the shoe meets the floor.

    One length per pair, or making each side reach its own geometry re-breaks the symmetry
    that was just established - measured, it put L_ToeBase at 16.3 cm and R_ToeBase at 21.1.
    """
    rest_the_pose(rig)
    leaves = sorted(b.name for b in rig.data.bones if not b.children)
    reach = {name: owned_reach(rig, mesh, name) for name in leaves}

    settled = {}
    for part in {n[2:] for n in leaves if n[:2] in ("L_", "R_")}:
        left, right = reach.get(f"L_{part}"), reach.get(f"R_{part}")
        if left is None or right is None:
            continue
        shared = max(left, right)
        settled[f"L_{part}"] = shared
        settled[f"R_{part}"] = shared
    for name in leaves:
        if name[:2] not in ("L_", "R_") and reach.get(name):
            settled[name] = reach[name]

    kept = {}
    with in_edit_mode(rig) as edit:
        for name, length in settled.items():
            bone = edit[name]
            kept[name] = (bone.matrix.to_3x3().col[1].normalized().copy(), bone.roll)
            bone.length = length
    with in_edit_mode(rig) as edit:
        worst = 0.0
        for name, (direction, roll) in kept.items():
            bone = edit[name]
            worst = max(
                worst,
                math.degrees(direction.angle(bone.matrix.to_3x3().col[1].normalized())),
                math.degrees(abs(bone.roll - roll)),
            )

    print(f"  {len(settled)} leaf bones lengthened to their own geometry; "
          f"the skinning basis moved {worst:.6f} deg")
    for name in ("Head", "L_Hand", "L_ToeBase"):
        if name in settled:
            print(f"    {name} is now {rig.data.bones[name].length * SCALE:.1f} cm")
    if worst > BASIS_MUST_NOT_MOVE:
        refuse(f"changing leaf lengths moved the skinning basis by {worst:.6f} deg")


def sane_root_and_hip(rig, mesh):
    """Root and Hip arrived 84.9 cm long with invented directions.

    Both drive zero vertices, so neither carries any deformation and both may be pointed
    anywhere. That is checked rather than assumed, because redirecting a bone that DOES
    drive geometry rotates its matrix_local, which is the skinning basis.
    """
    rest_the_pose(rig)
    names = {g.index: g.name for g in mesh.vertex_groups}
    driven = {}
    for vertex in mesh.data.vertices:
        for group in vertex.groups:
            name = names.get(group.group)
            if name and group.weight >= 0.001:
                driven[name] = driven.get(name, 0) + 1
    for name in ("Root", "Hip"):
        if driven.get(name):
            refuse(f"{name} drives {driven[name]} vertices, so its direction is a "
                   "skinning basis and must not be changed here")

    _, forward, _ = body_frame(rig)
    pelvis = rest_head(rig, "Pelvis")
    up_the_spine = (rest_tail(rig, "Waist") - pelvis).normalized()
    ground = mathutils.Vector((pelvis.x, pelvis.y, 0.0))
    inverse = rig.matrix_world.inverted()

    with in_edit_mode(rig) as edit:
        root = edit["Root"]
        root.head = inverse @ ground
        root.tail = inverse @ (ground + forward * (20.0 / SCALE))
        root.roll = 0.0
        hip = edit["Hip"]
        hip.head = inverse @ pelvis
        hip.tail = inverse @ (pelvis + up_the_spine * (12.0 / SCALE))
        hip.roll = 0.0

    print(f"  Root is now {rig.data.bones['Root'].length * SCALE:.1f} cm at the feet "
          f"pointing forward; Hip {rig.data.bones['Hip'].length * SCALE:.1f} cm up the spine")


def check_the_skin(mesh):
    """Checks the skin. Deliberately does NOT weld the coincident vertices.

    # Why the weld is gone

    It was here, and it looked free: the mesh arrives as 1442 disconnected shells with
    7475 non-manifold edges because glTF stores one set of attributes per vertex, so
    every UV seam and every hard normal edge duplicates the vertices along it. Merging
    them gave 19 clean shells, moved the surface 0.0005 mm, and kept the face count.

    It also WRECKED THE SHADING. Those split vertices ARE the hard-edge encoding, and
    the custom split normals that ride on them end up describing a topology that no
    longer exists: the character is lit as if it were a different shape, and the shoes
    read as shards, seams and melted forms. Rendered from the source with the weld as
    the only change, the damage is unmistakable - and no numeric guard here saw it,
    because shoe edge lengths and dimensions still matched the source to 0.01 cm.

    And the weld bought nothing that was not already had. `unfuse.cloth_pieces` welds
    VIRTUALLY - it rounds coordinates into buckets and unions across edges - so the
    garment pieces are identified without the mesh being altered at all. Everything
    downstream here selects shoe geometry by POSITION for the same reason.

    So the checks stay and the edit goes.
    """
    sums = [sum(g.weight for g in v.groups) for v in mesh.data.vertices]
    astray = sum(1 for s in sums if abs(s - 1.0) > 0.01)
    most = max(sum(1 for g in v.groups if g.weight > 0.001) for v in mesh.data.vertices)
    print(f"  {len(mesh.data.vertices)} vertices, {len(mesh.data.polygons)} faces, "
          f"custom split normals: {mesh.data.has_custom_normals}")
    print(f"  every vertex's weights add to 1 within "
          f"{max(abs(s - 1.0) for s in sums):.6f}; at most {most} bones drive any vertex")
    if astray:
        refuse(f"{astray} vertices have weights that do not add to 1")
    if most > 4:
        refuse(f"{most} bones drive one vertex; glTF carries 4 and drops the rest")


def put_the_ball_where_the_shoe_bends(rig, mesh):
    """Moves the ball joint to the shoe's real flex point, near the sole.

    # The fault this fixes, and why nothing else could

    Measured on the delivered rig: `ToeBase` runs HORIZONTALLY at ankle height, and its
    head - the joint the whole foot roll pivots on - sits 46% along the left shoe and
    33% along the right, 8.4 cm above the floor. The ball of a real foot is the first
    metatarsophalangeal joint, at 70 to 79% of foot length from the heel (Fernández et
    al. on shod feet: 70-72% of last length), and it is the part that touches the
    ground when you rise onto your toes.

    So "rolling onto the ball" was pivoting about a point a third of the way along the
    shoe, high in the air. The shoe see-sawed about its own arch and twenty centimetres
    of it stayed off the ground - which reads as heel-walking however the pitch curve
    is authored, and five rounds of tuning the pitch could not touch it because the
    pitch was never the problem.

    # What it does

    The station is taken per shoe from its own geometry and then SHARED between the two
    sides, so the rig stays mirror-exact. The two shoes disagree by about 4 cm on where
    they sit relative to their bones - the mesh asymmetry, still unsculpted - so each
    ball lands a little off its own 72%, and the guard below allows 62-84% rather than
    pretending otherwise.

    The height comes from the shoe too: a third of the way up the shoe's own section at
    that station, which puts the joint in the flex line of the sole rather than up by
    the ankle.

    Moving a rest bone changes `matrix_local`, which is the skinning basis - safe here
    because at rest the deformation is identity either way, and because the shoe's
    weights are reassigned from scratch by position immediately after this runs. What
    changes is how a POSED foot deforms, which is the entire point.
    """
    BALL_AT = 0.72          # of shoe length from the heel
    UP_THE_SECTION = 0.33   # of the shoe's own height at that station
    TOE_TIP_AT = 0.97       # the tail reaches nearly the front of the shoe

    balls = {s: rest_head(rig, f"{s}_ToeBase") for s in "LR"}
    shoes = {}
    for side in "LR":
        ankle = rest_head(rig, f"{side}_Foot")
        heading = balls[side] - ankle
        heading.z = 0.0
        heading.normalize()
        other = balls["L" if side == "R" else "R"]
        picked = []
        for vertex in mesh.data.vertices:
            spot = mesh.matrix_world @ vertex.co
            if spot.z > 0.14:
                continue
            flat = mathutils.Vector(((spot - balls[side]).x, (spot - balls[side]).y, 0.0))
            away = mathutils.Vector(((spot - other).x, (spot - other).y, 0.0))
            if flat.length < 0.25 and flat.length < away.length:
                picked.append(spot)
        if len(picked) < 40:
            refuse(f"only {len(picked)} vertices found for the {side} shoe")
        along = [(p - ankle).dot(heading) for p in picked]
        shoes[side] = {
            "ankle": ankle,
            "heading": heading,
            "back": min(along),
            "front": max(along),
            "spots": picked,
        }

    # One station for both sides, so the bones stay mirrors.
    station = sum(
        s["back"] + (s["front"] - s["back"]) * BALL_AT for s in shoes.values()
    ) / 2.0
    tip_station = sum(
        s["back"] + (s["front"] - s["back"]) * TOE_TIP_AT for s in shoes.values()
    ) / 2.0

    inverse = rig.matrix_world.inverted()
    with in_edit_mode(rig) as edit:
        for side in "LR":
            shoe = shoes[side]
            near = [
                p for p in shoe["spots"]
                if abs((p - shoe["ankle"]).dot(shoe["heading"]) - station) < 0.03
            ]
            if not near:
                refuse(f"the {side} shoe has no geometry at the ball station")
            sole = min(p.z for p in near)
            top = max(p.z for p in near)
            height = sole + (top - sole) * UP_THE_SECTION

            ball = (shoe["ankle"] + shoe["heading"] * station)
            ball.z = height
            tip = (shoe["ankle"] + shoe["heading"] * tip_station)
            tip.z = height

            edit[f"{side}_Foot"].tail = inverse @ ball
            edit[f"{side}_ToeBase"].head = inverse @ ball
            edit[f"{side}_ToeBase"].tail = inverse @ tip
            edit[f"{side}_ToeBase"].roll = edit[f"{side}_Foot"].roll

    for side in "LR":
        shoe = shoes[side]
        ball = rest_head(rig, f"{side}_ToeBase")
        at = ((ball - shoe["ankle"]).dot(shoe["heading"]) - shoe["back"]) / (
            shoe["front"] - shoe["back"]
        )
        print(f"  {side}: ball now at {at * 100:.0f}% of shoe length "
              f"(anatomy 70-79), {ball.z * SCALE:.1f} cm above the floor, "
              f"toe bone {rig.data.bones[f'{side}_ToeBase'].length * SCALE:.1f} cm long")
        if not 0.62 <= at <= 0.84:
            refuse(f"the {side} ball landed at {at * 100:.0f}% of the shoe")

    worst = 0.0
    for part in ("Foot", "ToeBase"):
        for get in (rest_head, rest_tail):
            left, right = get(rig, f"L_{part}"), get(rig, f"R_{part}")
            worst = max(worst, abs(abs(left.y) - abs(right.y)),
                        abs(left.x - right.x), abs(left.z - right.z))
    print(f"  the two feet are still mirrors to within {worst * SCALE:.3f} cm")
    # In CENTIMETRES, matching the message. The first version compared model units to
    # 0.02 and so tolerated 3.4 cm while printing "mirrors" - a threshold in different
    # units from its own report is a guard that does not guard.
    if worst * SCALE > 2.0:
        refuse(f"moving the ball broke the mirror by {worst * SCALE:.2f} cm")


def the_feet_own_their_shoes(rig, mesh):
    """Moves shin weights off the shoes, so a pitching foot pitches its whole shoe.

    Measured on the delivered skin: of 418 vertices below ankle height, 104 - the heel
    cuff block, a quarter of each shoe - were DOMINATED by CalfTwist02. The foot bone
    would pitch heel-up and the heel geometry would stay with the vertical shin: on
    screen the shoe rotated toes-up about its own glued heel, in the exact frames the
    bones reported heel-up. Bones and skin disagreeing is a weight fault, nothing else.

    Every calf- or thigh-chain weight on a vertex below the ankle is transferred to
    that side's Foot or ToeBase - by whether the vertex sits behind or ahead of the
    ball - with a blend band up the cuff so the shoe top still follows the shin a
    little. Weight moves between groups on the same vertex, so the sums stay exactly 1.
    """
    names = {g.name: g.index for g in mesh.vertex_groups}
    by_index = {i: n for n, i in names.items()}

    def is_foreign(name, side):
        """Any weight that is not this side's own Foot or ToeBase.

        This began as "this side's calf and thigh chain" and that was not enough: the
        cross-limb audit had already counted 137 vertices in one leg driven by the
        OTHER leg at weight 1.00, and the arm/leg unfuse never touches leg-on-leg. A
        right-shoe vertex holding left-calf weight follows the swinging left leg
        through the right foot's stance - measured, it lifted the ball region 1.6 cm
        and tilted the visible shoe +10 degrees toes-up while both of its own bones
        sat exactly at bind. Below the ankle there is no honest owner except the foot.
        """
        if name in (f"{side}_Foot", f"{side}_ToeBase"):
            return False
        return name[:2] in ("L_", "R_") and any(
            part in name for part in ("Calf", "Thigh", "Foot", "ToeBase")
        )

    ankle = {side: rest_head(rig, f"{side}_Foot").z for side in "LR"}
    ball = {side: rest_head(rig, f"{side}_ToeBase") for side in "LR"}
    heading = {}
    for side in "LR":
        line = rest_tail(rig, f"{side}_ToeBase") - rest_head(rig, f"{side}_Foot")
        line.z = 0.0
        heading[side] = line.normalized()

    blend_top = 5.0 / SCALE   # this far above the ankle the shin keeps its say
    moved, touched = 0.0, 0
    for vertex in mesh.data.vertices:
        spot = mesh.matrix_world @ vertex.co
        side = "L" if (spot - ball["L"]).length < (spot - ball["R"]).length else "R"
        over = spot.z - ankle[side]
        if over >= blend_top:
            continue
        share = 1.0 if over <= 0.0 else 1.0 - over / blend_top
        taking = 0.0
        for group in vertex.groups:
            name = by_index.get(group.group, "")
            if is_foreign(name, side) and group.weight > 0.0:
                taking += group.weight * share
                group.weight -= group.weight * share
        if taking <= 0.0:
            continue
        wearer = (f"{side}_ToeBase"
                  if (spot - ball[side]).dot(heading[side]) > 0.0 else f"{side}_Foot")
        already = sum(
            g.weight for g in vertex.groups if by_index.get(g.group, "") == wearer
        )
        mesh.vertex_groups[wearer].add([vertex.index], already + taking, "REPLACE")
        moved += taking
        touched += 1

    astray = sum(
        1 for v in mesh.data.vertices
        if abs(sum(g.weight for g in v.groups) - 1.0) > 0.01
    )
    print(f"  moved {moved:.1f} weight from the shin chains onto the feet, across "
          f"{touched} shoe vertices; {astray} left with weights not summing to 1")
    if astray:
        refuse("the shoe re-weight broke normalisation")

    still = 0
    for vertex in mesh.data.vertices:
        spot = mesh.matrix_world @ vertex.co
        side = "L" if (spot - ball["L"]).length < (spot - ball["R"]).length else "R"
        if spot.z - ankle[side] > 0.0:
            continue
        best, name = 0.0, ""
        for group in vertex.groups:
            if group.weight > best:
                best, name = group.weight, by_index.get(group.group, "")
        if is_foreign(name, side):
            still += 1
    print(f"  below the ankle, {still} vertices still follow a foreign bone (must be 0)")
    if still:
        refuse(f"{still} shoe vertices still follow something that is not their foot")

    # # And the crease is made SHARP: Foot behind the ball, ToeBase ahead, blended
    # # only across a 4 cm band at the joint itself
    #
    # Measured on the skin (segment pitches taken from sole-ring marker verts), the
    # toe box tracked the foot at about HALF amplitude everywhere: planted heel-rise
    # asked the toes to stay flat at 0 and the visible toe box swung to -11 with the
    # foot; flat mid-stance carried a +2-3 degree hover that kept the toe pad a
    # couple of centimetres off the floor. That is what a 50/50 Foot/ToeBase smear
    # across the whole box does - the visible shoe always splits the difference, so
    # it can neither hold flat nor follow. The bend belongs AT the ball.
    CREASE_BAND = 2.0 / SCALE  # half-width of the blend across the ball
    sharpened = 0
    for vertex in mesh.data.vertices:
        spot = mesh.matrix_world @ vertex.co
        side = "L" if (spot - ball["L"]).length < (spot - ball["R"]).length else "R"
        if spot.z - ankle[side] > 0.0:
            continue
        foot_g = names[f"{side}_Foot"]
        toe_g = names[f"{side}_ToeBase"]
        held = {g.group: g.weight for g in vertex.groups}
        pair = held.get(foot_g, 0.0) + held.get(toe_g, 0.0)
        if pair <= 1e-6:
            continue
        ahead = (spot - ball[side]).dot(heading[side])
        toe_share = min(1.0, max(0.0, (ahead + CREASE_BAND) / (2.0 * CREASE_BAND)))
        want_toe = pair * toe_share
        if abs(want_toe - held.get(toe_g, 0.0)) < 1e-4:
            continue
        mesh.vertex_groups[f"{side}_ToeBase"].add([vertex.index], want_toe, "REPLACE")
        mesh.vertex_groups[f"{side}_Foot"].add(
            [vertex.index], pair - want_toe, "REPLACE"
        )
        sharpened += 1
    astray = sum(
        1 for v in mesh.data.vertices
        if abs(sum(g.weight for g in v.groups) - 1.0) > 0.01
    )
    print(f"  crease sharpened across {sharpened} vertices; {astray} left with "
          f"weights not summing to 1")
    if astray:
        refuse("sharpening the crease broke normalisation")


def stand_in_an_a_pose(rig):
    """States each limb bone's world direction outright and back-solves the basis.

    Not a correction added to whatever the bone was doing. A world-axis rotation applied in
    a bone's own basis becomes a time-varying screw the moment its parent moves, which is
    how asking for 8 degrees of toe-out produced 28, 51, 134 and 117 at different moments
    with up to 59 degrees of roll. Stating the answer cannot drift.

    `Bone.convert_local_to_pose(..., invert=True)` is the sanctioned back-solve. Parents
    first, because a child's basis is measured against its parent's POSE.
    """
    rest_the_pose(rig)
    across, forward, up = body_frame(rig)
    down = -up

    def aim(side, out_deg, forward_deg):
        hand = 1.0 if side == "L" else -1.0
        out, ahead = math.radians(out_deg), math.radians(forward_deg)
        return (
            down * (math.cos(out) * math.cos(ahead))
            + across * (hand * math.sin(out) * math.cos(ahead))
            + forward * math.sin(ahead)
        ).normalized()

    wanted = {}

    # The torso, stood up plumb. The stack arrives leaning 6.5 degrees BACKWARD - the
    # neck 5.4 cm and the head 6.3 cm behind the pelvis - and every clip inherits the
    # bind, so every gait leaned back with it. Reported twice ("human spines dont lean
    # back when we run"), and the first repair only measured lean RELATIVE TO REST,
    # which is exactly the number that cannot see a leaning rest. The whole stack is
    # tipped forward rigidly, so the spine keeps its own curve and the neck lands
    # plumb over the pelvis; the neck and head are children and come along.
    pelvis = rest_head(rig, "Pelvis")
    neck = rest_head(rig, "NeckTwist01")
    leans_back = math.atan2((neck - pelvis).dot(forward), max(1e-9, neck.z - pelvis.z))
    tip = mathutils.Quaternion(across, -leans_back)
    for part in ("Waist", "Spine01", "Spine02"):
        direction = (rest_tail(rig, part) - rest_head(rig, part)).normalized()
        wanted[part] = tip @ direction

    for side in "LR":
        arm = aim(side, ARMS_OUT, 0.0)
        for part in ("Upperarm", "UpperarmTwist01", "UpperarmTwist02",
                     "Forearm", "ForearmTwist01", "ForearmTwist02", "Hand"):
            wanted[f"{side}_{part}"] = arm
        thigh = aim(side, LEGS_OUT, KNEE_EASE)
        for part in ("Thigh", "ThighTwist01", "ThighTwist02"):
            wanted[f"{side}_{part}"] = thigh
        calf = aim(side, LEGS_OUT, -KNEE_EASE)
        for part in ("Calf", "CalfTwist01", "CalfTwist02"):
            wanted[f"{side}_{part}"] = calf
        # NOT the feet. They are handled below by holding their BIND orientation.
        #
        # This used to aim both foot bones along a horizontal heading, which was right
        # while the bones ran horizontally through the shoe. Once the ball moved to the
        # shoe's real flex point the Foot bone legitimately angles DOWN about 14 degrees
        # from ankle to ball - so "make it horizontal" yanked every foot up by that
        # much, and against the hard weight split at the ball it sheared the shoes into
        # shards. The bake then froze the shards into the geometry, which is why the
        # damage showed at REST and no amount of animation work could touch it.
        #
        # At identity a bone deforms nothing, so the shoe is exactly as sculpted. The
        # only thing the feet need is to be held there while the leg above them turns.

    ordered = []

    def walk(bone):
        ordered.append(bone)
        for child in bone.children:
            walk(child)

    for bone in rig.pose.bones:
        if bone.parent is None:
            walk(bone)

    for posed in ordered:
        target_direction = wanted.get(posed.name)
        if target_direction is None:
            continue
        bpy.context.view_layer.update()
        world = posed.matrix.copy()
        now = (world.to_3x3() @ mathutils.Vector((0.0, 1.0, 0.0))).normalized()
        target = now.rotation_difference(target_direction).to_matrix().to_4x4() @ world
        target.translation = world.translation
        bone = posed.bone
        if posed.parent is None:
            posed.matrix_basis = bone.convert_local_to_pose(
                target, bone.matrix_local, invert=True
            )
        else:
            posed.matrix_basis = bone.convert_local_to_pose(
                target,
                bone.matrix_local,
                parent_matrix=posed.parent.matrix,
                parent_matrix_local=posed.parent.bone.matrix_local,
                invert=True,
            )
    bpy.context.view_layer.update()

    # --- The feet: hold the orientation they were BOUND with, whatever the leg did.
    #
    # A direction is not enough for a foot: aiming its Y axis leaves the bank about
    # that axis to whatever the parent handed down, and the sole tilts. The whole
    # orientation is restored instead, so the sole sits exactly as sculpted.
    world3 = rig.matrix_world.to_3x3()
    # # Holding the bind is not the same as holding the bind's MISTAKES
    #
    # This held each foot's bind orientation outright, which fixed the A-pose shearing
    # the shoes - and quietly stopped TOE_OUT being applied to anything. Measured, the
    # baked feet toed out 17.65 degrees apiece, 35 between them, while the constant read
    # 0.0 and the report said so. The flare that was reported twice was never fixed;
    # a number nothing reads had been set to zero.
    #
    # So the bind orientation is held and then YAWED about world up by the difference
    # between the toe-out it has and the toe-out asked for. Yaw about up cannot tilt a
    # sole, so the flatness the block exists to preserve is untouched.
    yaw_by = {}
    for side in "LR":
        hand = 1.0 if side == "L" else -1.0
        ankle = rest_head(rig, f"{side}_Foot")
        tip = rest_tail(rig, f"{side}_ToeBase")
        line = tip - ankle
        line.z = 0.0
        has = math.degrees(math.atan2(line.dot(across) * hand, line.dot(forward)))
        yaw_by[side] = mathutils.Quaternion(
            up, math.radians((TOE_OUT - has) * hand)
        ).to_matrix()
        print(f"  the {side} foot is bound {has:+.2f} deg out; yawing it "
              f"{TOE_OUT - has:+.2f} to reach the {TOE_OUT} asked for")

    for side in "LR":
        for part in ("Foot", "ToeBase"):
            posed = rig.pose.bones[f"{side}_{part}"]
            bone = posed.bone
            bpy.context.view_layer.update()
            target = (
                yaw_by[side] @ world3 @ bone.matrix_local.to_3x3()
            ).to_4x4()
            target.translation = posed.matrix.translation
            posed.matrix_basis = bone.convert_local_to_pose(
                target,
                bone.matrix_local,
                parent_matrix=posed.parent.matrix,
                parent_matrix_local=posed.parent.bone.matrix_local,
                invert=True,
            )
    bpy.context.view_layer.update()

    off = 0.0
    for side in "LR":
        for part in ("Foot", "ToeBase"):
            posed = rig.pose.bones[f"{side}_{part}"]
            bound = yaw_by[side] @ world3 @ posed.bone.matrix_local.to_3x3()
            now = world3 @ posed.matrix.to_3x3()
            off = max(off, math.degrees(
                (bound.inverted() @ now).to_quaternion().angle
            ))
    print(f"  the feet hold that orientation to within {off:.4f} deg")
    if off > 0.05:
        refuse(f"the feet are {off:.2f} deg off the orientation they were aimed at")

    worst, worst_name = 0.0, ""
    for name, asked in wanted.items():
        got = (rig.pose.bones[name].matrix.to_3x3()
               @ mathutils.Vector((0.0, 1.0, 0.0))).normalized()
        off = math.degrees(got.angle(asked))
        if off > worst:
            worst, worst_name = off, name
    print(f"  arms {ARMS_OUT} deg out, legs {LEGS_OUT} deg out with the knee eased "
          f"{2 * KNEE_EASE} deg forward, feet flat with {TOE_OUT} deg toe-out")
    print(f"    worst any bone missed its stated direction: {worst:.6f} deg ({worst_name})")
    if worst > AIMED_WITHIN:
        refuse(f"{worst_name} is {worst:.4f} deg off the direction it was told to take")


def put_it_on_the_floor(rig, mesh):
    """Lifts the whole character until the soles rest on z=0.

    The character arrived standing 5.7 cm into the ground in its own bind pose, and the
    animation tool was taking that rest sole AS the floor and faithfully reproducing it.
    A constant error is a datum error, not a solver that has not converged.
    """
    basis = rig.data.bones["Root"].matrix_local.to_3x3()
    turn = rig.matrix_world.to_3x3().inverted()
    root = rig.pose.bones["Root"]
    for _ in range(8):
        off = 0.0 - sole_of(mesh)
        root.location = root.location + (
            basis.inverted() @ (turn @ mathutils.Vector((0.0, 0.0, off)))
        )
        bpy.context.view_layer.update()
    left = sole_of(mesh)
    print(f"  the soles now rest at {left * SCALE:+.4f} cm")
    if abs(left) > ON_THE_FLOOR_WITHIN:
        refuse(f"the soles are {left * SCALE:+.3f} cm off the floor")


def bake_the_pose_as_rest(rig, mesh):
    """Makes the A-pose the bind pose, geometry first and bones second.

    `pose.armature_apply()` moves the BONES and nothing else. Run on its own it leaves the
    mesh where it was while the skeleton walks off, and the character then deforms by the
    difference. So the deformation is written into the geometry first, and only then are the
    bones told this is their new rest.

    The checks afterwards are ABSOLUTE - the arms must read 45 degrees, the knees 0, the
    soles 0 - and not "the same as what was fed in". Comparing a result to its own input is
    what let a mesh in one pose bound to a skeleton in another pass a green check.
    """
    if mesh.data.shape_keys:
        refuse("the mesh has shape keys, which baking deformation into it would fight")

    posed = deformed(mesh)
    if len(posed) != len(mesh.data.vertices):
        refuse("the modifier stack changes the vertex count, so the shapes cannot be "
               "matched one to one")

    # # Applied through the MODIFIER, not by writing vertex coordinates
    #
    # Writing `vertex.co` moves the geometry and leaves the custom split normals glTF
    # brings in exactly where they were - describing a shape that no longer exists. The
    # mesh is then lit as if it were still in its old pose, and it looks destroyed:
    # shards, seams, melted forms. It cost most of a day, because every numeric guard
    # here passed while it happened - shoe edge lengths and dimensions matched the
    # source to 0.01 cm - and a guard on geometry cannot see shading.
    #
    # `modifier_apply` evaluates the deform and writes the RESULT, normals included, so
    # the same shape arrives correctly lit. The modifier is then rebuilt, since applying
    # it consumes it.
    # The custom split normals have to be TURNED, not just carried.
    #
    # glTF ships per-corner normals, and they are what smooths a sculpt this low-poly.
    # Moving vertices does not touch them, so after the bake they describe a shape that
    # no longer exists and the surface is lit as if it were still in the old pose: it
    # reads as shards, seams and melted forms. That cost most of a day, because every
    # numeric guard here passed while it happened - shoe edge lengths and dimensions
    # matched the source to 0.01 cm - and no guard on geometry can see shading.
    # Applying through `modifier_apply` did not help either; the normals came along
    # unrotated.
    #
    # So each corner normal is rotated by the same blended skinning rotation that moves
    # its own vertex. Dropping them instead would work and lose the artist's smoothing,
    # which on this asset is most of how the shoes read.
    was_smooth = [n.vector.copy() for n in mesh.data.corner_normals]
    turn_of = {}
    for vertex in mesh.data.vertices:
        blended = mathutils.Matrix(((0.0,) * 3,) * 3)
        total = 0.0
        for group in vertex.groups:
            name = mesh.vertex_groups[group.group].name
            bone = rig.pose.bones.get(name)
            if bone is None or group.weight <= 0.0:
                continue
            skin = (bone.matrix @ bone.bone.matrix_local.inverted()).to_3x3()
            for row in range(3):
                for col in range(3):
                    blended[row][col] += skin[row][col] * group.weight
            total += group.weight
        if total > 1e-9:
            for row in range(3):
                for col in range(3):
                    blended[row][col] /= total
            turn_of[vertex.index] = blended

    posed_shape = deformed(mesh)
    for vertex, spot in zip(mesh.data.vertices, posed_shape):
        vertex.co = spot
    mesh.data.update()

    turned = []
    for loop in mesh.data.loops:
        was = was_smooth[loop.index]
        skin = turn_of.get(loop.vertex_index)
        fresh = (skin @ was) if skin is not None else was
        if fresh.length < 1e-9:
            fresh = was
        turned.append(fresh.normalized())
    mesh.data.normals_split_custom_set(turned)
    bpy.context.view_layer.update()

    was = bpy.context.view_layer.objects.active
    bpy.context.view_layer.objects.active = rig
    bpy.ops.object.mode_set(mode="POSE")
    bpy.ops.pose.armature_apply(selected=False)
    bpy.ops.object.mode_set(mode="OBJECT")
    bpy.context.view_layer.objects.active = was
    rest_the_pose(rig)

    across, forward, _ = body_frame(rig)
    print("  the rest pose is now, measured off matrix_local:")
    for part, asked in (("Upperarm", ARMS_OUT), ("Forearm", ARMS_OUT),
                        ("Thigh", LEGS_OUT), ("Calf", LEGS_OUT)):
        got = []
        for side in "LR":
            hand = 1.0 if side == "L" else -1.0
            along = (rest_tail(rig, f"{side}_{part}")
                     - rest_head(rig, f"{side}_{part}")).normalized()
            got.append(math.degrees(math.atan2(along.dot(across) * hand, -along.z)))
        print(f"    {part:<10} asked {asked:>5.1f} out, got L {got[0]:>6.1f} R {got[1]:>6.1f}")
        for side, value in zip("LR", got):
            if abs(value - asked) > 0.1:
                refuse(f"{side}_{part} rests at {value:.1f} deg out, not {asked:.1f}")

    for side in "LR":
        hip = rest_head(rig, f"{side}_Thigh")
        knee = rest_head(rig, f"{side}_Calf")
        ankle = rest_head(rig, f"{side}_Foot")
        bend = math.degrees((knee - hip).angle(ankle - knee))
        print(f"    {side} knee bends {bend:.3f} deg at rest, asked {2 * KNEE_EASE}")
        if abs(bend - 2 * KNEE_EASE) > 0.1:
            refuse(f"the {side} knee rests bent {bend:.2f} deg, not the "
                   f"{2 * KNEE_EASE} the IK needs to know its fold direction")
        ahead = (knee - hip)
        if ahead.dot(forward) < 0.0:
            refuse(f"the {side} knee ease points BACKWARD, which an IK pole in front "
                   "of the knee would fight every frame")

    pelvis, neck = rest_head(rig, "Pelvis"), rest_head(rig, "NeckTwist01")
    plumb = math.degrees(math.atan2((neck - pelvis).dot(forward),
                                    max(1e-9, neck.z - pelvis.z)))
    print(f"    the pelvis-to-neck line rests {plumb:+.2f} deg from plumb")
    if abs(plumb) > 0.3:
        refuse(f"the torso still leans {plumb:+.1f} deg in the bind pose")

    # The toe-out, measured on the BAKED bind rather than trusted. This is the check
    # that was missing while the feet toed out 17.65 degrees and the log said 0.0.
    for side in "LR":
        hand = 1.0 if side == "L" else -1.0
        line = rest_tail(rig, f"{side}_ToeBase") - rest_head(rig, f"{side}_Foot")
        line.z = 0.0
        out = math.degrees(math.atan2(line.dot(across) * hand, line.dot(forward)))
        print(f"    the {side} foot rests {out:+.3f} deg out, asked {TOE_OUT}")
        if abs(out - TOE_OUT) > 0.1:
            refuse(f"the baked {side} foot toes out {out:+.2f} deg, not {TOE_OUT}")

    left = sole_of(mesh)
    print(f"    the soles rest at {left * SCALE:+.4f} cm with no pose on the rig")
    if abs(left) > ON_THE_FLOOR_WITHIN:
        refuse(f"after the bake the soles sit {left * SCALE:+.3f} cm off the floor; "
               "the mesh and the skeleton are in different poses")

    moved = max((a - b).length for a, b in zip(deformed(mesh), posed))
    print(f"    the shape held to {moved * SCALE * 10.0:.6f} mm")
    print(f"    custom split normals survived: {mesh.data.has_custom_normals}")
    if moved > SHAPE_KEPT_WITHIN:
        refuse(f"the bake moved the surface by {moved * SCALE * 10.0:.4f} mm")


def report_the_fit(rig, mesh):
    """How far each bone runs from the middle of what it drives.

    Not a refusal. This is the measurement behind "the rig and the mesh are offset", and it
    is reported per side on purpose: the bones are mirror-exact after this runs, so any
    left-versus-right difference here is the MESH being asymmetric, which is a sculpting
    job and not a rigging one.
    """
    rest_the_pose(rig)
    groups = {g.name: g.index for g in mesh.vertex_groups}
    owned = {name: [] for name in groups}
    for vertex in mesh.data.vertices:
        spot = mesh.matrix_world @ vertex.co
        for group in vertex.groups:
            for name, index in groups.items():
                if group.group == index and group.weight >= 0.35:
                    owned[name].append(spot)

    def offset(name):
        spots = owned.get(name) or []
        if not spots or name not in rig.data.bones:
            return None
        centre = mathutils.Vector((0.0, 0.0, 0.0))
        for spot in spots:
            centre += spot
        centre /= len(spots)
        head, tail = rest_head(rig, name), rest_tail(rig, name)
        along = tail - head
        if along.length_squared < 1e-12:
            return (centre - head).length
        share = max(0.0, min(1.0, (centre - head).dot(along) / along.length_squared))
        return (centre - (head + along * share)).length

    worst_part, worst_gap = "", 0.0
    for part in mirror_pairs(rig):
        left, right = offset(f"L_{part}"), offset(f"R_{part}")
        if left is None or right is None:
            continue
        if abs(left - right) > worst_gap:
            worst_gap, worst_part = abs(left - right), part
    everything = sorted(o for o in (offset(n) for n in groups) if o is not None)
    print(f"  bones sit a median {everything[len(everything) // 2] * SCALE:.2f} cm from "
          f"the middle of what they drive, worst {everything[-1] * SCALE:.2f} cm")
    print(f"  the widest left-vs-right difference is {worst_part} at "
          f"{worst_gap * SCALE:.2f} cm -- that is the MESH being asymmetric, since the "
          f"bones are mirror-exact")


def main():
    where = argv()
    if len(where) < 2:
        refuse("usage: prepare_rig.py -- <source.glb> <out.glb>")
    source, out = where[0], where[1]

    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=source)
    rig = next(o for o in bpy.data.objects if o.type == "ARMATURE")
    mesh = max(
        (o for o in bpy.data.objects if o.type == "MESH" and o.vertex_groups),
        key=lambda o: len(o.data.vertices),
    )
    if rig.animation_data:
        rig.animation_data.action = None
        for track in rig.animation_data.nla_tracks:
            track.mute = True
    print(f"{source}: {len(rig.data.bones)} bones, {len(mesh.data.vertices)} vertices")
    rest_the_pose(rig)
    print(f"  as delivered the soles sit {sole_of(mesh) * SCALE:+.2f} cm against z=0")

    print("\nthe sphere widgets:")
    drop_the_widgets(rig)
    print("\nmirroring the two sides:")
    make_the_sides_mirrors(rig)
    print("\ncentring the skeleton on the mesh:")
    centre_the_skeleton(rig, mesh)
    print("\nthe leaf bones:")
    reach_the_ends(rig, mesh)
    print("\nRoot and Hip:")
    sane_root_and_hip(rig, mesh)
    print("\nthe skin as delivered:")
    check_the_skin(mesh)
    print("\nthe cross-limb weights:")
    # BEFORE the A-pose is struck or baked. A hip pocket holding a stray arm weight is
    # dragged sideways the moment the arms move, and baking then writes the dragged
    # shape into the geometry for good - which is exactly what happened: a flat plate
    # off the left hip, in the bind and in every frame of every clip after it.
    unfuse.unfuse_the_gloves_from_the_pockets(rig, mesh)
    # NO TORSO RE-WEIGHT. There was a step here that took the chest back off the
    # arms - the delivered skin drives it 61% from the upperarm twists and 12% from
    # Spine02 - on the theory that this was why leaning the spine did not read as a
    # lean. Measured with the step and without it, the trunk sits in EXACTLY the same
    # place: head +6.0 cm ahead of the hip either way. The posture came from centring
    # the skeleton and carrying the pelvis forward, not from the weights.
    #
    # And it cost a great deal: the jacket is loose cloth in its own shell, and
    # splitting its weights between the arm that used to carry it and the spine tore
    # its front panels into triangles. Rendered at a frame with the arm forward, the
    # damage is unmistakable. The chest really is arm-driven and that really is odd,
    # but it is a WEIGHTING decision about a garment, which belongs in a paint tool
    # with a person looking at it - not in a script inferring it from bone distance.
    print("\nthe shoes:")
    put_the_ball_where_the_shoe_bends(rig, mesh)
    the_feet_own_their_shoes(rig, mesh)
    print("\nthe A-pose:")
    stand_in_an_a_pose(rig)
    print("\nstanding it on the floor:")
    put_it_on_the_floor(rig, mesh)
    print("\nbaking the A-pose as the bind pose:")
    bake_the_pose_as_rest(rig, mesh)
    # NO SOLE FLATTENING. There was a step here that pulled sole vertices down onto
    # a plane, and it TORE THE MESH: it moved vertices individually, with a fade,
    # across coarse 2 cm slices, so neighbours got very different pulls and the
    # surface ripped into shards and holes. Every one of its guards passed -
    # flatness 0.00 cm, weights normalised, heel and toe heights correct - because
    # not one of them asked whether the shoe was still a shoe. Rendered large, it
    # was obviously wrecked.
    #
    # It is not coming back in this form. A sole's SHAPE is a sculpting decision and
    # belongs in a sculpting tool with a person looking at it. With the ball joint at
    # the shoe's real flex point the foot rolls correctly on the sculpted sole, which
    # is what made this step look necessary in the first place.
    print("\nhow well the rig fits the mesh:")
    report_the_fit(rig, mesh)

    for obj in bpy.data.objects:
        obj.select_set(obj in (rig, mesh))
    bpy.ops.export_scene.gltf(
        filepath=out,
        export_format="GLB",
        use_selection=True,
        export_yup=True,
        export_apply=False,
        export_animations=False,
    )
    print(f"\nwrote {out}")


if __name__ == "__main__":
    # Guarded so the A-pose numbers can be imported by the animation authoring without
    # running the whole build - the animator must ask for the SAME angles this bakes,
    # or it silently rotates every leg at the hip on every frame to fight the bind.
    main()
