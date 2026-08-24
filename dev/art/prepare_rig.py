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

import json
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


def shorten_the_controls(rig, mesh):
    """Takes Root and Hip down to a sane length, without changing where they point.

    `sane_root_and_hip` is the version for the BUILD, and it redirects them. This one is
    for anything that opens an already-animated file, where redirecting is unsafe:
    changing a bone's direction rotates `matrix_local`, and that is the basis the glTF
    importer has already converted the clip's keys into, so a redirect after import
    silently corrupts the pose. Length is stored apart from `matrix_local`, so shortening
    along a bone's own axis cannot.

    Both bones arrive 85.0 cm long, which is not the importer inventing: `Root` sits on
    the floor and its only child is at the pelvis, so its tail genuinely reaches the
    whole way up. It is still unreadable, which is the "what is this long angled bone".
    """
    names = {g.index: g.name for g in mesh.vertex_groups}
    driven = {}
    for vertex in mesh.data.vertices:
        for group in vertex.groups:
            name = names.get(group.group)
            if name and group.weight >= 0.001:
                driven[name] = driven.get(name, 0) + 1

    wanted = {"Root": 20.0 / SCALE, "Hip": 12.0 / SCALE}
    before = {}
    for name in wanted:
        if name not in rig.data.bones:
            continue
        if driven.get(name):
            print(f"  NOT shortening {name}: it drives {driven[name]} vertices, so its "
                  "length is not free to change here")
            continue
        bone = rig.data.bones[name]
        before[name] = (
            bone.matrix_local.to_3x3().col[1].normalized().copy(), bone.length
        )

    with in_edit_mode(rig) as edit:
        for name in before:
            edit[name].length = wanted[name]

    worst = 0.0
    for name, (direction, was) in before.items():
        now = rig.data.bones[name].matrix_local.to_3x3().col[1].normalized()
        worst = max(worst, math.degrees(direction.angle(now)))
        print(f"  {name}: {was * SCALE:.1f} -> "
              f"{rig.data.bones[name].length * SCALE:.1f} cm")
    print(f"  the two controls kept their direction to within {worst:.6f} deg")
    if worst > BASIS_MUST_NOT_MOVE:
        refuse(f"shortening the controls turned one by {worst:.6f} deg")


def make_the_import_readable(rig, mesh):
    """Everything a fresh glTF import needs before a person can look at the skeleton.

    THE POINT OF THIS FUNCTION IS THAT IT IS ONE FUNCTION. Every tool that opens a GLB
    needs all of it, none of it is optional, and it has to be redone on EVERY import
    because none of it can be exported - glTF stores joint positions and nothing else.
    Two of these faults have now been reported, fixed, and reported again:

      the sphere widgets - the importer's own `armature_display` builds an Icosphere and
        hangs it off all 41 bones, and a custom shape overrides `display_type`, so the
        skeleton reads as a bag of balls whatever the armature says;
      the leaf lengths - `Head` arrives 2.6 cm on a 27.8 cm head and both hands arrive
        8 cm past the fingertips, which is "the bones don't reach the top of the head
        and the ends of the feet and hands";
      and Root and Hip at 85 cm, which is "what is this long angled bone".

    Order is not free: `drop_the_widgets` disposes of the Icosphere by deleting meshes
    with no vertex groups, so anything else unskinned in the scene - a floor, marker
    bars, a reference prop - must be added AFTER this runs, not before.

    Safe on animated files: every step here changes bone LENGTHS or a display flag, and
    the two that touch lengths each re-read the directions afterwards and refuse if any
    skinning basis moved.
    """
    drop_the_widgets(rig)
    reach_the_ends(rig, mesh)
    shorten_the_controls(rig, mesh)
    left = [b.name for b in rig.pose.bones if b.custom_shape is not None]
    if left:
        refuse(f"{len(left)} bones still wear a widget ({left[:3]})")


def the_body(objects=None):
    """The character's own mesh, out of however many skinned meshes are in the scene.

    Every tool that opens the GLB used to take the FIRST skinned mesh it found. That was
    fine while there was one. Splitting the backpack out makes two, and "first" is
    whatever order the importer happened to use - so a tool could measure the ranger's
    gait against a 370-vertex bag. Largest wins instead, which is 7261 against 370 and
    needs no names: glTF suffixes duplicate names on round trip, so a name test would rot.
    """
    skinned = [
        o for o in (objects if objects is not None else bpy.data.objects)
        if o.type == "MESH"
        and (o.vertex_groups or any(m.type == "ARMATURE" for m in o.modifiers))
    ]
    if not skinned:
        refuse("no skinned mesh in this scene")
    return max(skinned, key=lambda o: len(o.data.vertices))


def split_out_the_backpack(rig, mesh):
    """Separates the backpack into its own object, rigidly bound to one bone.

    # Why it is separate

    Asked for directly - swappable bags for players later - and it also fixes a measured
    fault. The pack was skinned across `Spine01` (49%), `Spine02` (20%), `Waist` and even
    `Head`, and a RIGID object spread over four bones that rotate differently has to
    shear: measured, its own bounding diagonal changed 3.25 cm on a 73 cm object, 4.4%,
    over the run. That is what read as the pack moving oddly. Bound to a single bone it
    cannot deform at all, only be carried.

    # What it selects, and what it deliberately leaves

    Vertices the SPINE chain owns that sit behind the spine in the torso band, then the
    faces all of whose corners are in that set. Checked by rendering both halves before
    this was written: the pack comes away as a recognisable bag and the jacket back is
    left INTACT, no hole - the pack is additive geometry over the garment rather than a
    panel cut into it.

    The STRAPS stay on the body. They go over the shoulders and round the front, so they
    are not separable by "behind the spine", and for a first pass a fixed harness with a
    swappable pack is a normal way to build this. Moving them onto the bag is garment work
    for a paint tool, like the jacket weights above.
    """
    CARRIES = ("Waist", "Spine01", "Spine02", "NeckTwist", "Clavicle")
    BEHIND = 0.055
    HOLDS_IT = "Spine02"

    rest_the_pose(rig)
    _, forward, _ = body_frame(rig)
    forward = mathutils.Vector((forward.x, forward.y, 0.0)).normalized()
    spine = rig.data.bones["Spine02"].head_local
    names = {g.index: g.name for g in mesh.vertex_groups}

    chosen = set()
    for vertex in mesh.data.vertices:
        spot = mesh.matrix_world @ vertex.co
        best, who = 0.0, ""
        for group in vertex.groups:
            if group.weight > best:
                best, who = group.weight, names.get(group.group, "")
        if not any(part in who for part in CARRIES):
            continue
        if (spot - spine).dot(forward) < -BEHIND and 0.45 <= spot.z <= 0.92:
            chosen.add(vertex.index)
    faces = [
        f.index for f in mesh.data.polygons if all(i in chosen for i in f.vertices)
    ]
    print(f"  {len(chosen)} vertices behind the spine, {len(faces)} whole faces")
    if len(faces) < 100:
        refuse(f"only {len(faces)} faces look like a backpack; the rule has drifted")

    was = len(mesh.data.vertices)
    stem = mesh.name
    bpy.ops.object.select_all(action="DESELECT")
    mesh.select_set(True)
    bpy.context.view_layer.objects.active = mesh
    # The DESELECT has to go through the edit-mode operator, not through
    # `polygon.select = False`. Clearing the polygon flags in object mode leaves the
    # VERTEX selection untouched, and `separate(SELECTED)` reads that - so a freshly
    # imported mesh, which arrives fully selected, separates whole. Measured: the pack
    # came out with 7578 vertices and the body with 0.
    bpy.ops.object.mode_set(mode="EDIT")
    bpy.ops.mesh.select_all(action="DESELECT")
    bpy.ops.object.mode_set(mode="OBJECT")
    for index in faces:
        mesh.data.polygons[index].select = True
    bpy.ops.object.mode_set(mode="EDIT")
    # DUPLICATE first, then separate the copy. Separating alone MOVES the faces out of the
    # body, and measured that leaves it with six small holes - 0 real open loops before this
    # step and 6 after - which you then see the interior through, reported as pale patches
    # round the lower back. The note above says the jacket back is left intact and was
    # checked by rendering; a render will not show a four-vertex hole, and the topology does.
    #
    # Duplicating costs the body a few hundred faces it keeps under the pack, which is
    # cheaper than the alternatives: capping the rim afterwards needs a closed loop that the
    # split representation does not provide (fill_holes selected all 30 edges and added 0
    # faces), and leaving it open is the bug.
    bpy.ops.mesh.duplicate()
    bpy.ops.mesh.separate(type="SELECTED")
    bpy.ops.object.mode_set(mode="OBJECT")

    made = [
        o for o in bpy.data.objects
        if o.type == "MESH" and o is not mesh and o.name.startswith(stem.split(".")[0])
    ]
    if not made:
        refuse("the separate produced no new object")
    pack = min(made, key=lambda o: len(o.data.vertices))
    pack.name = pack.data.name = "Backpack"

    # RIGID: one group, weight 1, everything else cleared. That is the whole point - a
    # single bone cannot shear what it carries.
    for group in list(pack.vertex_groups):
        pack.vertex_groups.remove(group)
    held = pack.vertex_groups.new(name=HOLDS_IT)
    held.add(list(range(len(pack.data.vertices))), 1.0, "REPLACE")
    astray = sum(
        1 for v in pack.data.vertices
        if abs(sum(g.weight for g in v.groups) - 1.0) > 1e-6
    )
    print(f"  Backpack: {len(pack.data.vertices)} vertices, rigid on {HOLDS_IT}, "
          f"{astray} not summing to 1; the body keeps {len(mesh.data.vertices)}")
    if astray:
        refuse(f"{astray} backpack vertices are not rigidly bound")
    # Not equality: `separate` DUPLICATES the seam, so every vertex shared between a kept
    # face and a separated one ends up in both objects. Measured here as 47 extra on 7578,
    # which is the boundary of the pack and exactly what should happen. What must never
    # happen is LOSS, and the total can only grow by the seam.
    together = len(mesh.data.vertices) + len(pack.data.vertices)
    if together < was:
        refuse(
            f"the split lost geometry: {len(mesh.data.vertices)} + "
            f"{len(pack.data.vertices)} against {was}"
        )
    if together > was + len(chosen):
        refuse(
            f"the split grew by {together - was} vertices, more than the "
            f"{len(chosen)}-vertex selection could have on its boundary"
        )
    print(f"  the seam duplicated {together - was} vertices into both halves")
    if len(pack.data.vertices) >= len(mesh.data.vertices):
        refuse(
            f"the 'backpack' has {len(pack.data.vertices)} vertices against the body's "
            f"{len(mesh.data.vertices)} - the separate went the wrong way"
        )
    return pack


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


def whole_shells(mesh):
    """Every connected piece, after a VIRTUAL weld.

    glTF splits vertices to encode hard edges, so edge connectivity alone calls almost
    every face its own island - it reports 1396 pieces for a mesh that has 19. Unioning by
    POSITION first is what `unfuse` already does to find garment pieces, and it is the only
    way a "shell" here means what it sounds like.
    """
    import collections

    at = collections.defaultdict(list)
    for vertex in mesh.data.vertices:
        at[(round(vertex.co.x, 5), round(vertex.co.y, 5), round(vertex.co.z, 5))].append(
            vertex.index
        )
    parent = list(range(len(mesh.data.vertices)))

    def find(a):
        while parent[a] != a:
            parent[a] = parent[parent[a]]
            a = parent[a]
        return a

    def join(a, b):
        a, b = find(a), find(b)
        if a != b:
            parent[a] = b

    for together in at.values():
        for other in together[1:]:
            join(together[0], other)
    for edge in mesh.data.edges:
        join(edge.vertices[0], edge.vertices[1])

    shells = collections.defaultdict(set)
    for vertex in mesh.data.vertices:
        shells[find(vertex.index)].add(vertex.index)
    return list(shells.values())


def make_the_shoes_mirrors(rig, mesh):
    """Replaces the right shoe with a mirrored copy of the left.

    # Why the shoes and not the whole mesh
    #
    # `mesh_audit.py` measures the body at a median 1.32 cm from its own mirror image, and
    # the worst of it is the feet: ToeBase 3.29 cm, CalfTwist01 2.73, Foot 2.37. That is the
    # asymmetry with consequences - it is the reason `foot_roll` has to SHARE its landmarks
    # between the sides to stop the clips limping, and the reason the reach ceiling cannot be
    # tracked per frame. Elsewhere the asymmetry is either small or deliberate: the jacket's
    # zip, its pockets and the shoulder logo are meant to be one-sided.
    #
    # # Why a mirrored duplicate, and not the two obvious repairs
    #
    # Both were tried and measured in `shoe_symmetry_trial.py`, and both are worse than the
    # fault:
    #
    # * Moving each vertex onto its mirror PARTNER. The shoes are differently tessellated -
    #   179 right vertices find their nearest partner among only 41 of the left's 190 - so
    #   77% of the mapping collides and the shoe collapses onto a fortieth of its detail.
    # * Projecting each vertex onto the mirrored SURFACE, which does not care about
    #   tessellation. It distorts instead: median travel 1.96 cm, and edge lengths running
    #   x0.17 to x3.15 with 28 edges past half or double.
    #
    # A mirrored duplicate has neither problem, because the new shoe carries the LEFT's
    # topology rather than trying to fit the right's to a new shape. It is available at all
    # only because each shoe is a self-contained shell: 318 vertices on the left and 311 on
    # the right, each its own shoe plus its ankle cuff and nothing else. Nothing has to be
    # stitched back to a trouser leg.
    #
    # # Why this is done with object operators rather than bmesh
    #
    # The custom split normals. This mesh's hardest-won rule is that glTF encodes hard edges
    # by splitting vertices, so normals that stop describing the surface light the character
    # as a different shape - shoes read as shards - and NO geometry guard can see it.
    # Blender's own mirror-and-apply maintains those normals and the winding through the
    # transform; hand-rolled bmesh surgery would mean tracking every loop's normal myself and
    # mirroring it, which is exactly the bookkeeping that goes wrong quietly.
    #
    # Runs after `centre_the_skeleton`, because the midline it mirrors about is the one that
    # establishes, and before `put_the_ball_where_the_shoe_bends`, which reads shoe geometry.
    """
    across, _, _ = body_frame(rig)
    lateral = max(range(3), key=lambda i: abs(across[i]))

    def owns(vertex):
        best, who = 0.0, ""
        for group in vertex.groups:
            if group.weight > best:
                best, who = group.weight, mesh.vertex_groups[group.group].name
        return who

    def shoe_shell(side):
        wanted = {
            v.index for v in mesh.data.vertices
            if owns(v).startswith(f"{side}_")
            and ("Foot" in owns(v) or "ToeBase" in owns(v))
        }
        hit = [shell for shell in whole_shells(mesh) if shell & wanted]
        if len(hit) != 1:
            refuse(
                f"the {side} shoe spreads over {len(hit)} shells, so it is not the "
                f"separable piece this depends on"
            )
        return hit[0]

    keep, drop = shoe_shell("L"), shoe_shell("R")
    if keep & drop:
        refuse("the two shoes share vertices, so one cannot be replaced without the other")
    print(f"  the left shoe is {len(keep)} vertices, the right {len(drop)}")

    # # Only ever ONE object in edit mode
    #
    # The first attempt deleted the entire body - 7264 vertices down to 318 - because
    # `bpy.ops.mesh.*` acts on every selected object at once. Blender enters MULTI-OBJECT
    # edit mode when more than one is selected, and a duplicate is left selected alongside
    # its original, so a delete meant for the right shoe took the whole mesh with it. The
    # operators reported success throughout.
    #
    # So every edit-mode block below is fenced: deselect everything, select one, make it
    # active, and check the vertex count afterwards against what was asked for. A destructive
    # operator with no count check is how a mesh gets quietly emptied.
    def alone(obj):
        bpy.ops.object.mode_set(mode="OBJECT")
        bpy.ops.object.select_all(action="DESELECT")
        obj.select_set(True)
        bpy.context.view_layer.objects.active = obj

    def cut_down_to(obj, wanted, why):
        """Deletes everything except `wanted`, and refuses if the count is not what it should be."""
        before = len(obj.data.vertices)
        alone(obj)
        for vertex in obj.data.vertices:
            vertex.select = vertex.index not in wanted
        bpy.ops.object.mode_set(mode="EDIT")
        bpy.ops.mesh.delete(type="VERT")
        bpy.ops.object.mode_set(mode="OBJECT")
        after = len(obj.data.vertices)
        if after != len(wanted):
            refuse(
                f"{why}: asked to keep {len(wanted)} of {before} vertices and ended with "
                f"{after} - the selection did not do what it said"
            )
        return after

    alone(mesh)
    bpy.ops.object.duplicate()
    copy = bpy.context.view_layer.objects.active
    if copy is mesh:
        refuse("the duplicate did not become the active object, so nothing can be trusted")

    # On the copy, keep ONLY the left shoe.
    cut_down_to(copy, keep, "trimming the copy to the left shoe")

    # Mirror it, and apply - which is where Blender takes care of the normals and the
    # winding that a negative scale would otherwise leave inside out.
    alone(copy)
    scale = [1.0, 1.0, 1.0]
    scale[lateral] = -1.0
    copy.scale = scale
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    bpy.ops.object.mode_set(mode="EDIT")
    bpy.ops.mesh.select_all(action="SELECT")
    bpy.ops.mesh.flip_normals()
    bpy.ops.object.mode_set(mode="OBJECT")

    # It is a LEFT shoe wearing left weights; it has to become a right one.
    swapped = 0
    for group in copy.vertex_groups:
        if group.name.startswith("L_"):
            group.name = "R_" + group.name[2:]
            swapped += 1
    print(f"  {swapped} vertex groups renamed from L_ to R_ on the copy")

    # Now take the old right shoe out of the body - keep everything that is NOT it.
    body = {v.index for v in mesh.data.vertices} - drop
    cut_down_to(mesh, body, "cutting the old right shoe out of the body")

    # And put the mirrored one in its place.
    bpy.ops.object.mode_set(mode="OBJECT")
    bpy.ops.object.select_all(action="DESELECT")
    copy.select_set(True)
    mesh.select_set(True)
    bpy.context.view_layer.objects.active = mesh
    bpy.ops.object.join()

    print(f"  the body is now {len(mesh.data.vertices)} vertices, "
          f"split normals {mesh.data.has_custom_normals}")
    if len(mesh.data.vertices) != len(body) + len(keep):
        refuse(
            f"after joining, the body has {len(mesh.data.vertices)} vertices where "
            f"{len(body)} + {len(keep)} were expected"
        )
    if not mesh.data.has_custom_normals:
        refuse(
            "the join lost the custom split normals, which is the fault that makes the "
            "shoes read as shards - see the note on welding"
        )


def reshade(mesh, vertices, within: float = 40.0):
    """Rebuilds the custom split normals over a region, welded by position, split by angle.

    # Why a region ever needs this

    Subdividing is geometrically exact - measured, the 1234 new vertices sat 0.000 cm off the
    original surface and the old ones did not move at all. The shoe still came out looking
    melted, in lobes, and the reason is the shading:

        glTF splits a vertex at every hard edge, so this mesh has NO connectivity to smooth
        across. The custom split normals are the entire carrier of smooth shading.

    Subdivision interpolates those normals onto the new loops, and interpolating a normal field
    that was authored for one topology across a finer one gives mush. The character is lit as a
    shape it is not - the exact fault `check_the_skin` refuses to weld because of, arriving from
    the other direction. Rendered with the normals REMOVED, the subdivided shoe and the original
    are pixel-alike, which is what proved it was shading and not geometry.

    # What this does instead

    Recomputes them the way an angle-based auto-smooth would, on the connectivity the mesh
    actually has by POSITION rather than by index:

      * weld the region's vertices into buckets by position
      * at each bucket, average the normals of the faces meeting there, area-weighted
      * average only across faces within `within` degrees of the one being shaded, so a genuine
        crease - the sole's edge, the shelf - stays a crease instead of being rounded off

    Only the region's loops are rewritten. Everything else keeps the normals it shipped with,
    which is the point: this is for geometry that has just been changed, not for the asset.
    """
    import collections

    region = set(vertices)
    faces = [p for p in mesh.data.polygons if all(v in region for v in p.vertices)]
    if not faces:
        raise SystemExit("REFUSED: no whole face lies inside the region to reshade")

    grain = 0.00002   # the weld bucket from find_the_fingers: split copies, never neighbours
    def bucket(index):
        co = mesh.data.vertices[index].co
        return (round(co.x / grain), round(co.y / grain), round(co.z / grain))

    meeting = collections.defaultdict(list)
    for poly in faces:
        normal = mathutils.Vector(poly.normal)
        if normal.length_squared < 1e-12:
            continue
        normal.normalize()
        for index in poly.vertices:
            meeting[bucket(index)].append((normal, poly.area))

    cosine = math.cos(math.radians(within))
    was = [mathutils.Vector(c.vector) for c in mesh.data.corner_normals]
    now = list(was)
    for poly in faces:
        mine = mathutils.Vector(poly.normal)
        if mine.length_squared < 1e-12:
            continue
        mine.normalize()
        for loop in poly.loop_indices:
            here = bucket(mesh.data.loops[loop].vertex_index)
            blended = mathutils.Vector((0.0, 0.0, 0.0))
            for normal, area in meeting[here]:
                if normal.dot(mine) >= cosine:
                    blended += normal * area
            now[loop] = blended.normalized() if blended.length_squared > 1e-12 else mine

    mesh.data.normals_split_custom_set([tuple(n) for n in now])
    turned = sum(1 for a, b in zip(was, now) if (a - b).length > 0.001)
    print(f"  reshaded {len(faces)} faces: {turned} of {len(now)} loop normals rebuilt, "
          f"creases kept past {within:.0f} deg")
    if not mesh.data.has_custom_normals:
        raise SystemExit("REFUSED: reshading dropped the custom split normals entirely")


def it_only_cut_what_was_asked_for(mesh, before, asked, cuts):
    """Refuses if the mesh grew by far more than the region it was given could account for.

    Cutting one quad `c` times makes (c+1)^2 of them, so a region of N faces can add at most
    about N((c+1)^2 - 1) vertices, plus fan triangles along its own border. Doubling that and
    allowing a flat 200 leaves plenty of room for the border and none at all for the failure
    this exists to catch, which is not a near miss - it is the WHOLE MESH, nine times over.

    Written against the arithmetic of subdivision rather than against vertex indices, which
    subdivision itself invalidates. See the note in `subdivide_these`.
    """
    grew = len(mesh.data.vertices) - before
    room = asked * ((cuts + 1) ** 2 - 1) * 2 + 200
    print(f"  cutting {asked} faces {cuts}x added {grew} vertices, against {room} allowed")
    if grew > room:
        raise SystemExit(
            f"REFUSED: subdividing {asked} faces added {grew} vertices where {room} is the most "
            f"that region could account for - the selection did not take and the whole mesh has "
            f"been cut")


def subdivide_these(mesh, polygons, cuts: int = 1):
    """Cuts the named polygons `cuts` times, and puts the skinning back within glTF's limits.

    Pulled out of `add_room_where_it_tears` when the shoes needed the same operation on a
    different set of faces. Everything in here is about the two ways this can go wrong
    silently, so it is worth having in ONE place rather than two.

    # The selection has to be made INSIDE edit mode, and this is why

    Setting `poly.select` and `vertex.select` in object mode and then entering edit mode does
    NOT carry the selection in. Measured, on this mesh:

        object mode:                       226 faces selected, 335 verts
        edit mode, immediately after:     7139 faces selected, 9190 verts

    Everything arrives selected, so `bpy.ops.mesh.subdivide` cuts the WHOLE BODY. That is not
    a subtlety - it is a silent 9x, and it is what actually happened to `add_room_where_it_tears`
    when it "took the body from 7578 to 18532 vertices and tearing did not fall". It was never
    subdividing the 121 straddling polygons it named; it was subdividing all of them. The
    conclusion drawn from that experiment was about a different experiment.

    So the faces are picked through bmesh once edit mode is already open, and
    `it_only_cut_what_was_asked_for` counts the untouched faces afterwards rather than trusting
    any of this.

    Subdividing BLENDS the weights of the corners it interpolates between, which is the point -
    a new vertex in an armpit inheriting some spine and some upperarm is what stops it tearing.
    But blending STACKS influences, and the build refused: "7 bones drive one vertex; glTF
    carries 4 and drops the rest". Four is the format's limit, so anything above it is not a
    heavier vertex, it is a vertex whose smallest influences vanish silently at export - and
    silently is the word that matters, since the mesh would look right in Blender and wrong in
    the game. Trimming to the four largest and renormalising is what the exporter would do
    anyway, done here where it can be seen and where the guard can check it.

    Smoothness is 0: this only TESSELLATES, it does not round anything off. Any shaping is a
    separate, measurable step afterwards.
    """
    import bmesh

    wanted = set(polygons)
    started_with = len(mesh.data.vertices)

    bpy.ops.object.mode_set(mode="OBJECT")
    bpy.ops.object.select_all(action="DESELECT")
    mesh.select_set(True)
    bpy.context.view_layer.objects.active = mesh
    bpy.ops.object.mode_set(mode="EDIT")
    bpy.ops.mesh.select_mode(type="FACE")
    bpy.ops.mesh.select_all(action="DESELECT")
    working = bmesh.from_edit_mesh(mesh.data)
    working.faces.ensure_lookup_table()
    for index in wanted:
        working.faces[index].select = True
    bmesh.update_edit_mesh(mesh.data)
    bpy.ops.mesh.subdivide(number_cuts=cuts, smoothness=0.0)
    bpy.ops.object.mode_set(mode="OBJECT")

    bpy.ops.object.vertex_group_limit_total(limit=4)
    bpy.ops.object.vertex_group_normalize_all(lock_active=False)
    it_only_cut_what_was_asked_for(mesh, started_with, len(wanted), cuts)


def add_room_where_it_tears(rig, mesh, cuts: int = 1):
    """Subdivides the polygons that straddle two body regions, so the skin has room to bend.

    # What tears, and why it is not webbing
    #
    # `tear_audit.py` measures every edge's deformed length against its rest length across
    # every frame of every clip. 894 of 10432 edges - 8.57% - stray past x1.35, and the worst
    # reaches x21 on a single edge running from `Spine01` to `R_ForearmTwist01`.
    #
    # An edge from the chest to the forearm looks exactly like generated webbing, and the
    # first guess was that these were spurious bridging faces to cut. Measured, they are not:
    # of the 121 polygons that straddle two regions only EIGHT are slivers, their median area
    # is 15.7 cm2 against a 2.79 cm median edge length, the largest is 120 cm2, and together
    # they are 8.56% of the whole surface. That figure sitting on top of the 8.57% of edges
    # that tear is the whole diagnosis: these are not artefacts, they are ordinary quads that
    # are simply far too big, each with its corners on bones that swing apart.
    #
    # A few huge polygons across an armpit means a few long edges taking the entire
    # deformation. Cutting them would open holes in a surface that is meant to be there. The
    # fix for coarse geometry at a joint is not to remove it, it is to give it more of itself.
    #
    # # Why subdividing is safe here where welding never is
    #
    # This mesh's standing rule is that glTF splits vertices to encode hard edges, so MERGING
    # them leaves the custom split normals describing a surface that no longer exists and the
    # character is lit as a different shape. Subdividing is the opposite operation: it adds
    # vertices between existing ones and interpolates, so no split is collapsed and no normal
    # is orphaned. `has_custom_normals` is checked afterwards regardless, because that whole
    # class of fault is invisible to any geometry measurement.
    #
    # Vertex groups are interpolated onto the new vertices by Blender, which is what makes
    # this worth doing at all - the new geometry inherits a BLEND of the bones at each corner,
    # and a blend is exactly what a joint needs.
    #
    # Runs before `put_the_ball_where_the_shoe_bends` and the shoe re-weight, both of which
    # assign by position and would otherwise be working from geometry that is about to change.
    """
    def owns(vertex):
        best, who = 0.0, ""
        for group in vertex.groups:
            if group.weight > best:
                best, who = group.weight, mesh.vertex_groups[group.group].name
        return who

    def region(name):
        for part in ("Forearm", "Upperarm", "Hand"):
            if part in name:
                return "arm"
        for part in ("Thigh", "Calf", "Foot", "Toe"):
            if part in name:
                return "leg"
        for part in ("Spine", "Waist", "Hip", "Pelvis", "Neck", "Head"):
            if part in name:
                return "trunk"
        return None

    owners = [region(owns(v)) for v in mesh.data.vertices]
    straddling = [
        poly.index for poly in mesh.data.polygons
        if len({owners[i] for i in poly.vertices} - {None}) > 1
    ]
    if not straddling:
        print("  nothing straddles two regions, so there is nothing to open up")
        return

    before = len(mesh.data.vertices)
    subdivide_these(mesh, straddling, cuts)
    after = len(mesh.data.vertices)
    print(f"  {len(straddling)} polygons straddled a joint; subdividing them {cuts}x "
          f"took the body from {before} to {after} vertices")
    if after <= before:
        refuse(
            f"the subdivide added nothing - {before} vertices before and {after} after - so "
            f"the selection did not reach the operator"
        )
    if not mesh.data.has_custom_normals:
        refuse(
            "subdividing lost the custom split normals, which is the fault that lights the "
            "character as a different shape - see the note on welding"
        )


def cut_the_fusions(rig, mesh, hops: int = 4, times_median: float = 4.0,
                    only=("arm", "trunk")):
    """Cuts the faces where the generator welded a limb to the body, and caps what is left.

    # What is being cut, and how it is told apart from real clothing
    #
    # `tear_audit.py` finds edges stretching x21 under animation, and the worst run from
    # `Spine01` to `R_ForearmTwist01` - the chest to the forearm. That is not geometry. In the
    # A-pose the forearm hangs beside the hip, and the generator has fused the two where they
    # nearly touched, leaving a band of faces bridging the gap. When the arm swings the band
    # has to stretch, and that stretching IS the tearing.
    #
    # Telling a fusion from ordinary clothing is the whole difficulty, because for a dressed
    # character a sleeve genuinely IS continuous with the jacket at the shoulder. Two tests
    # together, and neither works alone:
    #
    #   * BONE DISTANCE. How many joints apart the bones driving a face's corners are. An
    #     upperarm and a spine are close in the hierarchy and a face spanning them is a
    #     shoulder. A forearm and a spine are five joints apart and nothing legitimate spans
    #     them. Distance alone is not enough because a single mis-weighted vertex - there are
    #     `Waist` weights down at knee height - makes an innocent face look far-flung.
    #   * SIZE. The median face here is 2.70 cm2. A fusion has to span a real gap, so it is
    #     enormous by comparison: the worst is 120.5 cm2, forty-five times the median. A
    #     mis-weighted vertex sits in a face of ordinary size.
    #
    # At five hops and five times the median that is 32 faces. Deliberately the tightest
    # defensible set rather than the largest - the same tests at four hops would take 101.
    #
    # # Why the holes are capped SEPARATELY, and never bridged
    #
    # Cutting the band leaves two boundary loops: one where it left the arm, one where it left
    # the torso. Filling each on its own closes the arm as an arm and the torso as a torso.
    # `bridge_edge_loops` would join the two loops back together, which is the fusion again -
    # it is the obvious operator to reach for here and it is exactly wrong.
    #
    # Only the NEW boundaries are filled. This mesh is not watertight to begin with - it is 19
    # shells and the garments have open hems - so filling every boundary would sew the jacket
    # shut. The edges that were already open are recorded first and left alone.
    """
    def owns(vertex):
        best, who = 0.0, ""
        for group in vertex.groups:
            if group.weight > best:
                best, who = group.weight, mesh.vertex_groups[group.group].name
        return who

    joined = {}
    for bone in rig.data.bones:
        if bone.parent:
            joined.setdefault(bone.name, set()).add(bone.parent.name)
            joined.setdefault(bone.parent.name, set()).add(bone.name)

    known = {}

    def apart(a, b):
        """How many joints lie between two bones."""
        if a == b:
            return 0
        key = (a, b) if a < b else (b, a)
        if key in known:
            return known[key]
        seen, edge, far = {a}, [a], 0
        while edge and far < 14:
            far += 1
            nxt = []
            for here in edge:
                for there in joined.get(here, ()):
                    if there == b:
                        known[key] = far
                        return far
                    if there not in seen:
                        seen.add(there)
                        nxt.append(there)
            edge = nxt
        known[key] = 99
        return 99

    owners = [owns(v) for v in mesh.data.vertices]

    # # Judged on the EDGE, not on the face's area
    #
    # The first version picked faces by area - over five times the median - and cut 31 of
    # them, and the tearing did not move: 8.57% of edges past x1.35 before, 8.53% after, with
    # the very same worst edge at x21.11. Area is a symptom. What actually tears is a long
    # EDGE spanning bones that swing apart, and those edges survived the cut because they were
    # shared with a neighbouring face that happened to fall under the area threshold.
    #
    # So the test is on the edge itself: longer than `times_median` times the median edge, and
    # its two ends driven by bones at least `hops` joints apart. Any face carrying such an
    # edge goes, which is the only way to be sure the edge goes with it.
    lengths = sorted(
        (mesh.data.vertices[e.vertices[0]].co - mesh.data.vertices[e.vertices[1]].co).length
        for e in mesh.data.edges
    )
    median = lengths[len(lengths) // 2]
    big = median * times_median
    print(f"  the median edge is {median * 170.0:.2f} cm, so 'long' here is over "
          f"{big * 170.0:.2f} cm")

    # # Only between the regions named in `only`
    #
    # This is the restriction that was missing when this last ran and holed the trousers. The
    # long-edge-and-far-apart test is right, but it catches ordinary TROUSER geometry too,
    # because there are `Waist` weights sitting down at knee height and a single mis-weighted
    # vertex makes an innocent face look like a bridge. Leg-to-trunk is therefore off limits.
    #
    # Arm-to-trunk has no such trap: nothing legitimate spans a forearm and a spine, and these
    # are the long flat ribbons reported as straps attached to the arm from the back.
    def region(name):
        for part in ("Forearm", "Upperarm", "Hand", "Clavicle"):
            if part in name:
                return "arm"
        for part in ("Thigh", "Calf", "Foot", "Toe"):
            if part in name:
                return "leg"
        for part in ("Spine", "Waist", "Hip", "Pelvis", "Neck", "Head"):
            if part in name:
                return "trunk"
        return None

    torn = set()
    for edge in mesh.data.edges:
        a, b = edge.vertices
        span = (mesh.data.vertices[a].co - mesh.data.vertices[b].co).length
        if span <= big or apart(owners[a], owners[b]) < hops:
            continue
        if {region(owners[a]), region(owners[b])} != set(only):
            continue
        torn.add(edge.key)

    fused = [
        poly.index for poly in mesh.data.polygons
        if any(
            tuple(sorted((poly.vertices[i], poly.vertices[(i + 1) % len(poly.vertices)])))
            in torn
            for i in range(len(poly.vertices))
        )
    ]
    print(f"  {len(torn)} edges are long AND span {hops}+ joints, carried by "
          f"{len(fused)} faces")

    if not fused:
        print("  nothing is fused by both tests, so there is nothing to cut")
        return

    faces_before = len(mesh.data.polygons)
    print(f"  {len(fused)} faces are {hops}+ joints across AND over "
          f"{big * 170.0 * 170.0:.1f} cm2")

    # # The cut and the cap in ONE bmesh session
    #
    # The first version recorded which edges were open before the cut, cut, recorded them
    # again, and filled the difference. That is wrong in a way nothing complained about:
    # deleting faces RENUMBERS the vertices, so an edge key taken before the cut names a
    # different pair of vertices afterwards. The "difference" came out as 7007 edges, and
    # filling them sewed up 240 faces worth of boundary all over the mesh - including the
    # garment hems, which are open on purpose. The mesh is not watertight to begin with: 7475
    # of its 10432 edges are boundary edges, because glTF splits vertices for hard edges.
    #
    # Holding the boundary as bmesh REFERENCES avoids the whole problem. An element that
    # survives a delete keeps its identity, so the edges around the cut can be picked out
    # before and still be the same edges after. And staying inside one edit-mode session is
    # what preserves the custom split normals - `bm.to_mesh` would drop them.
    bpy.ops.object.mode_set(mode="OBJECT")
    bpy.ops.object.select_all(action="DESELECT")
    mesh.select_set(True)
    bpy.context.view_layer.objects.active = mesh
    bpy.ops.object.mode_set(mode="EDIT")
    working = bmesh.from_edit_mesh(mesh.data)
    working.faces.ensure_lookup_table()
    going = [working.faces[i] for i in fused]
    doomed = set(going)

    # The rim: edges of a doomed face that some surviving face also uses. These are what the
    # cut leaves open, and they are exactly what should be capped.
    rim = [
        edge for face in going for edge in face.edges
        if any(other not in doomed for other in edge.link_faces)
    ]
    rim = list(dict.fromkeys(rim))
    print(f"  they are rimmed by {len(rim)} edges that survive the cut")

    for face in working.faces:
        face.select_set(False)
    for edge in working.edges:
        edge.select_set(False)
    for vertex in working.verts:
        vertex.select_set(False)
    bmesh.ops.delete(working, geom=going, context="FACES")

    # `rim` still refers to live edges - only the faces were removed.
    capped = 0
    for edge in rim:
        if edge.is_valid:
            edge.select_set(True)
            capped += 1
    bmesh.update_edit_mesh(mesh.data)
    print(f"  {capped} of the rim survived and is selected for capping")
    bpy.ops.mesh.fill_holes(sides=0)
    bpy.ops.object.mode_set(mode="OBJECT")

    cut = faces_before - len(mesh.data.polygons)
    print(f"  net {cut:+d} faces; the body is {len(mesh.data.vertices)} vertices and "
          f"{len(mesh.data.polygons)} faces")
    if cut > len(fused):
        refuse(
            f"{cut} faces went for {len(fused)} asked - more was cut than chosen"
        )
    if not mesh.data.has_custom_normals:
        refuse(
            "the cut lost the custom split normals, which lights the character as a "
            "different shape - see the note on welding"
        )


# How far from a limb's own axis a small piece has to sit before it counts as hanging OFF the
# limb rather than being worn on it, in model units - about 17 cm. See `hangs_off`.
# How far off a limb's own axis a small piece has to sit, in CENTIMETRES, before it counts as
# hanging off the limb rather than being worn on it.
#
# In centimetres on purpose. It was in model units, compared against a figure measured in
# world centimetres, and the mismatch silently spared the straps this exists to remove -
# twice, because the second attempt lowered the number instead of fixing the units.
#
# The cuff sits at 6.7 cm and the straps at 16.1, 18.5 and 29.4, so 11 sits in the middle of
# a wide gap rather than near either edge.
CLEARS_THE_ARM = 11.0


def remove_the_hanging_straps(rig, mesh, biggest: int = 120):
    """Deletes the loose strap pieces the generator hung off the forearms.

    # What these are
    #
    # The character wears a backpack, and the generator gave it straps - but it also left
    # four small pieces of strap floating on the FOREARMS, where no strap belongs. They dangle
    # off the wrists and swing with the arms, which is what they were reported as.
    #
    # They are identifiable without any guesswork, because after a virtual weld the mesh is 19
    # shells and each of these is one of them:
    #
    #   44 verts, 6.8 cm across, z 110-117, every vertex on L_ForearmTwist01
    #   49 verts, 10.0 cm,       z 103-113, L_ForearmTwist01 and 02
    #   60 verts, 10.3 cm,       z 102-112, R_ForearmTwist01 and 02
    #   73 verts, 5.3 cm,        z 103-108, every vertex on R_ForearmTwist01
    #
    # 226 vertices between them. Small, thin, self-contained, and driven only by forearm
    # bones - which is exactly what a strap that has ended up on an arm looks like, and
    # nothing else in this mesh looks like it.
    #
    # # What is deliberately NOT touched
    #
    # The 62-vertex shell at z 122-126 is a real strap, correctly driven by `Spine02` and
    # `L_Clavicle`: it crosses the chest, where a strap should be, and it moves with the
    # torso. And the 268-vertex shell on `R_UpperarmTwist01` spans 35 cm - that is a SLEEVE,
    # not a strap. The size cap and the forearm-only test keep both.
    #
    # The shoes are 311 and 318 vertices, the gloves 406 and 471, so nothing near them is at
    # risk from a 120-vertex cap either.
    """
    def owns(vertex):
        best, who = 0.0, ""
        for group in vertex.groups:
            if group.weight > best:
                best, who = group.weight, mesh.vertex_groups[group.group].name
        return who

    owners = [owns(v) for v in mesh.data.vertices]

    # # It has to HANG OFF the arm, not sit on it
    #
    # Size and forearm-ownership alone took the SLEEVE CUFFS with the straps. Four shells
    # qualified and one of them was the ribbed band at the end of the sleeve; removing it left
    # the sleeve an open tube with the forearm passing through, reported as the forearm not
    # being connected to the upper arm. The raw export has the band and the built asset did
    # not, which is what settled it.
    #
    # Encircling was the first test tried and it fails: the cuff covers only 54 degrees round
    # the arm, because most of it is the sleeve's own shell and only a fragment is separate.
    #
    # DISTANCE from the arm's own axis works. Measured on the four: 6.7 cm for the cuff, and
    # 16.1, 18.5 and 29.4 for the straps. A cuff is worn ON the arm so it sits within a few
    # centimetres of the bone; a strap dangling off is three times that or more.
    def how_far_off_the_arm(shell):
        """Mean distance from the forearm's own axis, in CENTIMETRES.

        In world space and in real units, both deliberately. The first version worked in
        mesh-local while the threshold had been derived from a world-space probe, and the two
        are not the same space - so it silently spared the straps it was written to remove:
        pieces measured at 16.1 and 18.5 cm by the probe came out under a 12 cm limit here.
        Nudging the number would have papered over a units bug.
        """
        side = owners[next(iter(shell))][0]
        elbow = rig.matrix_world @ rig.pose.bones[f"{side}_Forearm"].head
        wrist = rig.matrix_world @ rig.pose.bones[f"{side}_Hand"].head
        along = wrist - elbow
        if along.length < 1e-9:
            return 0.0
        along.normalize()
        out = 0.0
        for i in shell:
            spoke = (mesh.matrix_world @ mesh.data.vertices[i].co) - elbow
            spoke -= along * spoke.dot(along)
            out += spoke.length
        return out / len(shell) * SCALE

    candidates = [
        shell for shell in whole_shells(mesh)
        if len(shell) <= biggest
        and all("Forearm" in owners[i] for i in shell)
    ]
    # Printed every build, because the whole difficulty here is telling a cuff from a strap and
    # the number that does it should not be invisible.
    loose = []
    for shell in candidates:
        out = how_far_off_the_arm(shell)
        keeps = out <= CLEARS_THE_ARM
        print(f"    {len(shell):3} verts sit {out:5.1f} cm off the arm axis -> "
              f"{'worn on the arm, KEPT' if keeps else 'hangs off, removed'}")
        if not keeps:
            loose.append(shell)
    if not loose:
        print("  no forearm-only pieces small enough to be a stray strap")
        return

    going = {i for shell in loose for i in shell}
    print(f"  {len(loose)} loose forearm pieces, {len(going)} vertices: "
          + ", ".join(str(len(shell)) for shell in sorted(loose, key=len)))

    before = len(mesh.data.vertices)
    bpy.ops.object.mode_set(mode="OBJECT")
    bpy.ops.object.select_all(action="DESELECT")
    mesh.select_set(True)
    bpy.context.view_layer.objects.active = mesh
    bpy.ops.object.mode_set(mode="EDIT")
    working = bmesh.from_edit_mesh(mesh.data)
    working.verts.ensure_lookup_table()
    for face in working.faces:
        face.select_set(False)
    for edge in working.edges:
        edge.select_set(False)
    for vertex in working.verts:
        vertex.select_set(False)
    bmesh.ops.delete(
        working, geom=[working.verts[i] for i in going], context="VERTS"
    )
    bmesh.update_edit_mesh(mesh.data)
    bpy.ops.object.mode_set(mode="OBJECT")

    went = before - len(mesh.data.vertices)
    print(f"  {went} vertices went; the body is {len(mesh.data.vertices)}")
    if went != len(going):
        refuse(
            f"asked to remove {len(going)} vertices and {went} went - the delete did not "
            f"do what it said"
        )
    if not mesh.data.has_custom_normals:
        refuse("removing the straps lost the custom split normals")


def face_the_right_way_out(rig, mesh):
    """Flips the faces the generator wound backwards.

    # What they look like, and what they are
    #
    # Reported as pale angular shards around the lower back and hip, in game and in a render.
    # They are faces whose winding is reversed: the surface is in the right place, but it is
    # facing into the body, so it is lit from behind and reads as a bright shard sitting on
    # top of the clothing.
    #
    # 283 of 4429 faces, clustered at z 90-130 - the torso - with the worst single one 73 cm2.
    #
    # # How they are identified without guessing
    #
    # A ray each way from the face's own centre. If the OUTWARD direction is blocked by solid
    # geometry while the inward direction is open, then what the face calls out is in and what
    # it calls in is out, and it is backwards. The two exclusions matter: an ordinary surface
    # has its outward side open and its inward side blocked, and a thin garment panel - a
    # jacket flap, a pocket - is open BOTH ways. Neither can be mistaken for backwards.
    #
    # # Why FLIP and not recalculate
    #
    # `normals_make_consistent` is the obvious operator and it is the wrong one here. It
    # recomputes normals from the geometry, which throws away the custom split normals that
    # encode this mesh's hard edges - the documented shards/melted-shoe fault, traded for the
    # shards being fixed. Flipping reverses the winding AND carries the stored normal with it,
    # which is precisely what a backwards face needs and nothing more.
    """
    from mathutils.bvhtree import BVHTree

    solid = BVHTree.FromObject(mesh, bpy.context.evaluated_depsgraph_get())
    step = 0.004

    backwards = []
    for poly in mesh.data.polygons:
        centre = mesh.matrix_world @ poly.center
        out = (mesh.matrix_world.to_3x3() @ poly.normal).normalized()
        blocked = solid.ray_cast(centre + out * step, out, 2.0)[0] is not None
        open_behind = solid.ray_cast(centre - out * step, -out, 2.0)[0] is None
        if blocked and open_behind:
            backwards.append(poly.index)

    if not backwards:
        print("  every face is already facing out")
        return

    area = sum(mesh.data.polygons[i].area for i in backwards) * 170.0 * 170.0
    print(f"  {len(backwards)} of {len(mesh.data.polygons)} faces are wound backwards, "
          f"{area:.0f} cm2 between them")

    bpy.ops.object.mode_set(mode="OBJECT")
    bpy.ops.object.select_all(action="DESELECT")
    mesh.select_set(True)
    bpy.context.view_layer.objects.active = mesh
    bpy.ops.object.mode_set(mode="EDIT")
    bpy.ops.mesh.select_mode(type="FACE")
    working = bmesh.from_edit_mesh(mesh.data)
    working.faces.ensure_lookup_table()
    for face in working.faces:
        face.select_set(False)
    for edge in working.edges:
        edge.select_set(False)
    for vertex in working.verts:
        vertex.select_set(False)
    for index in backwards:
        working.faces[index].select_set(True)
    bmesh.update_edit_mesh(mesh.data)
    bpy.ops.mesh.flip_normals()
    bpy.ops.object.mode_set(mode="OBJECT")

    if not mesh.data.has_custom_normals:
        refuse(
            "flipping lost the custom split normals - see the note on why this flips rather "
            "than recalculating"
        )

    # Measured again, on the result. A flip that leaves them backwards is a flip that did not
    # happen, and this file has had several of those.
    solid = BVHTree.FromObject(mesh, bpy.context.evaluated_depsgraph_get())
    left = 0
    for poly in mesh.data.polygons:
        centre = mesh.matrix_world @ poly.center
        out = (mesh.matrix_world.to_3x3() @ poly.normal).normalized()
        if (
            solid.ray_cast(centre + out * step, out, 2.0)[0] is not None
            and solid.ray_cast(centre - out * step, -out, 2.0)[0] is None
        ):
            left += 1
    print(f"  {left} still backwards afterwards")
    if left > len(backwards) // 4:
        refuse(
            f"{left} faces are still backwards of {len(backwards)} flipped - the selection "
            f"did not reach the operator"
        )


def close_the_holes_round_the_waist(rig, mesh, biggest: int = 8):
    """Caps the small punctures round the waist and hip that you can see inside through.

    # Why this is capping and not bridging
    #
    # It was reported as the legs not being joined to the torso, and asked for as a bridge
    # across the waist. Measured, that is not the shape of it - and the first two
    # measurements said otherwise because they were taken on the wrong topology.
    #
    # glTF splits vertices to encode hard edges, so on the mesh as it arrives 6975 of 10131
    # edges look like boundary and there are 1362 "boundary loops", the largest 29 vertices.
    # None of that is real. Welded by position first - 7062 split vertices are 2302 real ones
    # - there are 6710 edges of which only 140 are genuinely open, in TEN loops.
    #
    # And most of those ten are meant to be open:
    #
    #   41 verts, z 92-138, an open chain on Spine01/Spine02  - the jacket's front zip
    #   38 verts, z 127-143, closed, Clavicle and Neck        - the collar
    #   26 verts, z 157-160, an open chain on Head            - the hairline
    #
    # What is left is three small closed punctures: six vertices at the waist, four at the
    # hip on L_ThighTwist01, four beside the arm. Those are the holes you see the interior
    # through, and there is no gap between the leg and the torso to bridge - the trouser
    # shell has no opening at its top at all.
    #
    # So: cap closed loops of at most `biggest` vertices below `below`. The size cap is what
    # protects the jacket front and the collar, and the height cap keeps the hairline out of
    # it. Filling the jacket's zip shut would be a far worse bug than the one being fixed.
    """
    weld, spot = {}, {}
    for vertex in mesh.data.vertices:
        where = (
            round(vertex.co.x, 5), round(vertex.co.y, 5), round(vertex.co.z, 5)
        )
        spot.setdefault(where, len(spot))
        weld[vertex.index] = spot[where]

    carried = {}
    for index, welded in weld.items():
        carried.setdefault(welded, index)

    faces = {}
    for poly in mesh.data.polygons:
        ring = [weld[i] for i in poly.vertices]
        for i in range(len(ring)):
            a, b = ring[i], ring[(i + 1) % len(ring)]
            if a != b:
                faces[tuple(sorted((a, b)))] = faces.get(tuple(sorted((a, b))), 0) + 1

    open_edges = [pair for pair, count in faces.items() if count < 2]
    beside = {}
    for a, b in open_edges:
        beside.setdefault(a, set()).add(b)
        beside.setdefault(b, set()).add(a)

    seen, wanted = set(), set()
    loops = 0
    for start in list(beside):
        if start in seen:
            continue
        group, stack = [], [start]
        seen.add(start)
        while stack:
            here = stack.pop()
            group.append(here)
            for there in beside[here]:
                if there not in seen:
                    seen.add(there)
                    stack.append(there)
        closed = all(len(beside[i]) == 2 for i in group)
        # Size and closedness alone protect everything that is meant to stay open: the
        # jacket's zip is a 41-vertex open CHAIN, the collar is closed but 38 vertices and so
        # far over the cap, and the hairline is a 26-vertex chain. A height limit was in here
        # as well and was doing no work at all.
        if closed and len(group) <= biggest:
            loops += 1
            for i in group:
                for j in beside[i]:
                    wanted.add(tuple(sorted((i, j))))

    if not wanted:
        print("  no small closed holes down here to cap")
        return

    print(f"  {loops} small closed holes, {len(wanted)} welded edges between them")

    bpy.ops.object.mode_set(mode="OBJECT")
    bpy.ops.object.select_all(action="DESELECT")
    mesh.select_set(True)
    bpy.context.view_layer.objects.active = mesh
    bpy.ops.object.mode_set(mode="EDIT")
    bpy.ops.mesh.select_mode(type="EDGE")
    working = bmesh.from_edit_mesh(mesh.data)
    for face in working.faces:
        face.select_set(False)
    for vertex in working.verts:
        vertex.select_set(False)
    picked = 0
    for edge in working.edges:
        pair = tuple(sorted((
            weld[edge.verts[0].index], weld[edge.verts[1].index]
        )))
        take = pair in wanted
        edge.select_set(take)
        picked += 1 if take else 0
    bmesh.update_edit_mesh(mesh.data)
    faces_before = len(mesh.data.polygons)
    bpy.ops.mesh.fill_holes(sides=biggest)
    bpy.ops.object.mode_set(mode="OBJECT")
    print(f"  {picked} split edges selected, {len(mesh.data.polygons) - faces_before} "
          f"faces added")

    if not mesh.data.has_custom_normals:
        refuse("capping the holes lost the custom split normals")


PICKED_JUNK = os.path.join(os.path.dirname(os.path.abspath(__file__)), "junk_to_remove.json")


def remove_the_picked_junk(rig, mesh):
    """Deletes the vertices picked by hand in `pick_the_junk.sh`.

    # Why by hand and not by rule

    Five rules were written to find the generator's stray geometry - by shell size, by bone
    ownership, by face area, by long edges between distant bones, by distance from the limb
    axis - and three of them removed real parts of the character: the trouser leg, the sleeve
    cuffs, and a chunk of the shoulder. They were not badly tuned. "A long thin face spanning
    bones that are far apart" IS a hanging strap, and it is also a SHOULDER, where one quad
    legitimately runs from the clavicle out to the upper arm. Nothing in the geometry tells
    the two apart, so no threshold can, and each attempt cost a rebuild and a piece of him.

    So the junk is named once, by eye, in Blender, and removed by identity from then on.

    # Identity is POSITION, not index

    Indices shift the moment anything is deleted. Positions do not, and the pick is taken on
    the RAW export, which is exactly what this sees - so this runs FIRST, before mirroring,
    centring, the A-pose or the bake move anything.

    An absent list is the ordinary case for a fresh checkout and is not news.
    """
    if not os.path.exists(PICKED_JUNK):
        print("  no junk_to_remove.json - nothing has been picked yet")
        return

    with open(PICKED_JUNK) as handle:
        wanted = {tuple(spot) for spot in json.load(handle)["positions"]}
    if not wanted:
        print("  the pick list is empty")
        return

    going = [
        v.index for v in mesh.data.vertices
        if tuple(round(c, 5) for c in v.co) in wanted
    ]
    print(f"  {len(wanted)} positions picked, {len(going)} found in this mesh")
    if not going:
        refuse(
            "none of the picked positions are in this mesh - the pick was taken against a "
            "different file, so it would silently remove nothing"
        )

    before = len(mesh.data.vertices)
    bpy.ops.object.mode_set(mode="OBJECT")
    bpy.ops.object.select_all(action="DESELECT")
    mesh.select_set(True)
    bpy.context.view_layer.objects.active = mesh
    bpy.ops.object.mode_set(mode="EDIT")
    working = bmesh.from_edit_mesh(mesh.data)
    working.verts.ensure_lookup_table()
    for face in working.faces:
        face.select_set(False)
    for edge in working.edges:
        edge.select_set(False)
    for vertex in working.verts:
        vertex.select_set(False)
    bmesh.ops.delete(working, geom=[working.verts[i] for i in going], context="VERTS")
    bmesh.update_edit_mesh(mesh.data)
    bpy.ops.object.mode_set(mode="OBJECT")

    went = before - len(mesh.data.vertices)
    print(f"  {went} vertices removed; the body is {len(mesh.data.vertices)}")
    if went != len(going):
        refuse(f"asked to remove {len(going)} vertices and {went} went")
    if not mesh.data.has_custom_normals:
        refuse("removing the picked junk lost the custom split normals")


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
    # NOT CALLED YET. `make_the_shoes_mirrors` is written below and its approach is measured
    # and sound, but the operator-driven surgery is not landing the selection it asks for, so
    # it refuses rather than guessing. A refusing build is worse than an unfinished feature,
    # so the call waits here until two things are solved:
    #
    #   1. `bpy.ops.mesh.delete` is not removing what the object-mode select flags say. The
    #      multi-object edit-mode trap is fenced off already and was not the whole of it.
    #   2. Custom split normals need REFLECTING, not flipping. Mirroring geometry across a
    #      plane wants the normal with its lateral component negated; `flip_normals` negates
    #      all three, and those are different vectors. Getting it wrong is the documented
    #      shards/melted-shoe fault that no geometry guard can see.
    # NOT CALLED. `add_room_where_it_tears` is kept below because its measurements are worth
    # having, but subdividing was the WRONG FIX and the evidence says so:
    #
    #   * It took the body from 7578 to 18532 vertices and tearing did not fall.
    #   * It cannot even be shown to have helped or hurt, because the strain metric it was
    #     judged by is RESOLUTION-DEPENDENT: the same 1 cm of displacement is x1.33 on a 3 cm
    #     edge and x2 on a 1 cm one, so a finer mesh scores worse for free. 8.57% before
    #     against 8.68% after is not a comparison, it is two different rulers.
    #   * And the premise was wrong anyway. The weighting is not the fault - the median weight
    #     jump across an edge is 0.006, joints blend as they should, and the only hard seams
    #     are the 11 deliberate ones at the shoe's ball. Adding resolution to weights that are
    #     already correct cannot fix a problem that is geometric.
    #
    # The actual fault is that the generator FUSED limbs to the body where they sat close:
    # 165 edges bridge two body regions, up to 36.74 cm long against a 2.79 cm median, over
    # 121 polygons that are 8.56% of the whole surface. Those bridges stretch when the limbs
    # separate, and that is the tearing. Cutting them is modelling work rather than a
    # pipeline step, because the surface behind a fusion does not exist and the hole left
    # would need closing.
    print("\nturning the backwards faces the right way out:")
    face_the_right_way_out(rig, mesh)
    print("\nremoving the straps that ended up on the forearms:")
    remove_the_hanging_straps(rig, mesh)
    # AFTER the surgery, not before it. The raw export has only three open loops and every one
    # of them is the collar, the neck or the hairline - there are no holes at the waist as it
    # arrives. They are made HERE: removing a forearm strap takes with it any face that
    # bridged the strap to the arm, and the backpack split leaves a rim of its own. Capping
    # before that ran found nothing to cap, which was correct and useless.
    print("\ncapping the small holes the surgery leaves:")
    close_the_holes_round_the_waist(rig, mesh)
    # ARM-TO-TRUNK ONLY, which is the whole reason this is called again.
    #
    # The first time it ran unrestricted it HOLED THE TROUSERS: the long-edge-and-far-apart
    # test also catches ordinary trouser geometry, because there are `Waist` weights sitting
    # down at knee height and one mis-weighted vertex makes an innocent face look like a
    # bridge. It opened a gap on the thigh you could see skin through and a diamond hole in
    # the shin. Worse, the render that would have shown it WAS taken and I read the dark
    # angular shapes on the thighs as trouser design.
    #
    # Leg-to-trunk is therefore off limits, and arm-to-trunk has no equivalent trap: nothing
    # legitimate spans a forearm and a spine. Those faces are the long flat ribbons reported
    # as "straps attached to the arm from the back" - 12 of them, 468 cm2, highlighted in red
    # and agreed before being cut. They render as small patches because most of each blade is
    # buried inside the body, which is why they are easy to see in a wireframe and easy to
    # miss in a render.
    # NOT CALLED, and it should not be again in this form.
    #
    # Restricting it to arm-and-trunk did keep the trousers whole, but it took part of the ARM
    # instead - a chunk out of the shoulder - and left the hanging straps behind. That is
    # three times a rule over faces has removed real geometry here: the trouser leg, the
    # sleeve cuffs, and now the shoulder.
    #
    # The lesson is about the METHOD and not the thresholds. "A long edge between distant
    # bones" describes a strap, and it equally describes a SHOULDER, where one big quad
    # legitimately spans from the clavicle out to the upper arm. A rule that cannot tell a
    # strap from a deltoid will not be fixed by tightening it, and every attempt costs a
    # rebuild and a piece of the character.
    #
    # What these need is to be NAMED once, by eye, and removed by identity afterwards.
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
    # LAST, so every step above sees one whole mesh and nothing upstream has to know
    # about the split. Everything downstream picks the character with `the_body`.
    print("\nsplitting out the backpack:")
    pack = split_out_the_backpack(rig, mesh)

    # AFTER the split, because the split is what makes the holes. `mesh.separate` duplicates
    # the seam into both objects, so the body is left with a rim where the pack used to join
    # it - and that rim is what you see the interior through, reported as pale patches round
    # the lower back and as "the legs not connected to the torso".
    #
    # It took three goes to put this call in the right place, and the reason is worth keeping:
    # the holes were measured on the EXPORTED asset and the fix was first wired into the top
    # of this pipeline, where it correctly found nothing, because the raw export has no holes
    # at the waist at all - only a collar, a neck and a hairline. Measuring one artefact and
    # fixing another is easy to do when both are called "the mesh".
    print("\ncapping the holes the split leaves:")
    close_the_holes_round_the_waist(rig, mesh)

    for obj in bpy.data.objects:
        obj.select_set(obj in (rig, mesh, pack))
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
