# Rigging

## Bone budgets

**STANDARD.** A current-generation humanoid game rig runs **70–100 bones**, or 100–120 if you
count a full set of fingers. Body without fingers is 50–80. The breakdown usually given:

| Part | Bones |
|---|---|
| Spine | 5 (pelvis + spine_01..03 + chest, roughly) |
| Neck and head | 2 |
| Clavicles | 2 |
| Arms | 4 (upper + lower, each side) |
| Hands | 2 |
| Fingers | **30** — 5 digits x 3 joints x 2 hands |
| Legs | 4 |
| Feet and toes | 4 |
| Twist / roll helpers | the remainder |

Two things fall out of that table that are easy to miss. **Fingers are about half the rig.** And
the spine gets five joints, not two — a character that lifts, crouches and carries needs the
torso to bend in more than one place.

**MEASURED (Copaimo, 2026-08-23).** 71 bones. 41 as delivered plus 30 finger bones. Against the
standard that is a complete hand set and a **thin spine**: `Waist → Spine01 → Spine02` is three
where the standard is five, and there are no separate chest or upper-chest joints.

> **→ For Copaimo.** Lifting and crouching will want more spine. Adding one joint between
> `Spine02` and the clavicles is the cheap version and is the usual place for it. Not urgent —
> it is a real change to the bind, so batch it with other rig work rather than doing it alone.

**STANDARD.** For crowds and distance, strip finger bones in the LOD rig. Since fingers are half
the bone count, that halves the skinning cost of a background character for free. Three
animation LODs is the usual setup: full rig, simplified rig, imposter.

## Naming

**STANDARD.** There is no cross-engine standard, and that is itself the important fact.

- Unreal: `lowercase_with_underscores`, side as a suffix — `upperarm_l`, `hand_r`, `spine_01`.
- Unity: `PascalCase`, and its humanoid system maps *any* naming onto a normalised rig instead.
- Mixamo: `mixamorig:LeftForeArm`.

Retargeting between rigs depends on near-identical hierarchy **and** naming, plus similar
proportions. Renaming later is cheap in a file and expensive in every clip that references it.

**MEASURED (Copaimo).** `L_`/`R_` prefix, CamelCase parts: `L_Upperarm`, `R_ForearmTwist01`,
`L_Index2`. Consistent, and the finger bones were added to match. Not Unreal's convention, which
does not matter for a Bevy game and would matter if animations were ever bought in.

## Twist bones

**STANDARD.** A single bone taking a whole limb's rotation produces the **candy-wrapper**
artefact — the mesh pinches to nothing at the twist. The fix is 1–3 twist joints between elbow
and wrist (and shoulder and elbow) carrying a share of the roll each. Without them a 180° forearm
rotation collapses the mesh.

**MEASURED (Copaimo).** The delivered rig has twist bones and they carry **all** the arm and leg
skin — `L_Upperarm`, `L_Forearm`, `L_Thigh` and `L_Calf` each drive **zero** vertices; every one
of those vertices is on a `...Twist01`/`...Twist02`. That works, because the twists are children
and inherit the parent's rotation, but it means:

- posing `L_Forearm` moves the mesh only via its children;
- any tool that asks "which bone drives this vertex" gets a twist bone, never the main one;
- deleting or re-parenting a twist bone would remove skin, not just refine it.

## Skinning

**STANDARD.** Real-time engines use **linear blend skinning** (LBS) and have for twenty years,
despite two well-known artefacts:

- **candy wrapper** — volume collapse under twist;
- **elbow collapse** — volume loss under bending.

The alternatives each trade one artefact for another:

| Method | Fixes | Introduces |
|---|---|---|
| Dual quaternion (DQS) | candy wrapper, volume loss | joint **bulging** on bends |
| Delta mush / direct delta mush | most of both, cheaply | needs a smoothing precompute |
| Corrective shape keys | anything, exactly | authoring cost per pose |
| Spherical blend, stretchable-twistable bones | specific cases | complexity |

In practice: **LBS plus enough twist bones plus corrective shapes where it still shows.** That is
what shipped games do.

**STANDARD, and a trap.** Blender's armature modifier has a *Preserve Volume* checkbox, which is
dual-quaternion skinning. Engines generally do **not** support it. Leaving it on means Blender
shows you a deformation the game will not produce.

### The hard limits

**STANDARD.** Four bone influences per vertex. glTF stores four; a fifth is dropped **silently**
at export, so the model looks right in Blender and wrong in the game. Bevy is explicit about the
same limit. Some engines do eight; assume four.

**STANDARD.** In glTF every vertex must be assigned to at least one bone. Blender treats an
unassigned vertex as un-skinned and it exports wrong.

**STANDARD.** Weights should sum to 1 per vertex. Trim to the largest four and renormalise
yourself, where you can see it, rather than letting the exporter do it quietly.

**MEASURED (Copaimo).** All three are guarded in `prepare_rig.check_the_skin` and refuse the
build. Worth keeping: subdividing *blends* weights, which is what stops a new armpit vertex
tearing, and it also stacks influences past four. One subdivide took a vertex to seven bones.

## Bind pose

**STANDARD.** A-pose or T-pose. A-pose puts the shoulder halfway through its range, so weights
are wrong by half as much at both extremes; T-pose is easier to model and to retarget from.

**STANDARD, and it bites.** Applying a pose as the rest pose permanently changes the armature,
and *every* clip authored against the old rest pose now means something different. A clip cannot
be copied across a bind change — only retargeted. Shape keys, drivers, constraints and custom
properties on the armature modifier can all be lost or need rebuilding.

**MEASURED (Copaimo).** The delivered rig arrived with a 17.5° crouch, the two sides 5.45 cm from
mirrored, and the character 5.7 cm under the floor. All three are rest-pose constants, which is
why per-frame corrections kept failing: **correcting a constant per pose is what twisted the
feet**, three separate times. Fixed once in the bind, and the authoring has no correction step at
all now.

A dead-straight two-bone chain is **singular** to an IK solver — it cannot tell which way the
joint folds, and every knee froze at exactly 0.0000. Standard fix is a few degrees of bend in the
bind. Copaimo uses `KNEE_EASE = 2.0`, so the knee rests bent 4°.

## Hands

**STANDARD.** Three joints per finger — proximal, middle, distal — and three for the thumb. Some
game rigs drop to two per finger to save bones. Three is what lets a fist have two joints to fold
at, and a fist with one fold reads as a mitten.

**STANDARD.** Phalanx proportions: the proximal phalanx is roughly as long as the middle and
distal together. Copaimo uses 45 / 30 / 25 of digit length.

**STANDARD, and the thing we got wrong.** *The thumb does not share the fingers' axes.* Its
movement is at an angle to theirs, so the rotation axis has to be adjusted — the usual figure is
the second joint's axis rotated about **45° outward, perpendicular to the thumbnail**. The thumb
wants its own chain and its own orientation, not the palm plane the fingers use.

> **→ For Copaimo.** This is exactly the open item: the thumb opposes weakly because all five
> digits were rolled to the same palm normal. 45° off the palm plane on the thumb, perpendicular
> to where a nail would be, is the standard answer.

**STANDARD.** Hand topology: concentric edge loops around each joint, radial flow from palm into
fingers, joint locations centred in a clean ring. Loops in the *middle* of a straight segment do
nothing for deformation and can be removed. Triangles are fine on the back of the hand and the
wrist cuff where nothing bends. A test rig of three bones per finger is the recommended way to
preview it.

**MEASURED (Copaimo).** The sculpt is triangle soup with no edge flow — normal for a generated
mesh — and each finger arrived with barely one ring of vertices per bone, so a fist read as
blocks. Subdividing the finger faces twice (7534 → 9189 vertices, silhouette unchanged at
smoothness zero) gave each digit 114–160 welded nodes. Both hands close 5.3 and 6.1 cm with zero
tearing across 698 shared vertex positions.

**Identifying digits, the hard way.** Four separate geometric tests for "which digit is the
thumb" all named the **pinky**, because on this sculpt the pinky is shorter (12.2 vs 14.0 cm), its
knuckle shallower (9.5 vs 11.0), and it splays further (25.2° vs 23.3°). A thumb is not the short
one, the splayed one, or the odd one out. What worked: the digit pointing *across* the palm rather
than down it, established once on the delivered mesh, then carried through remeshing by tip
position. See `dev/art/add_finger_bones.py`, which records all four failures.

## Weight painting

**STANDARD.** The loop is **pose, observe, paint, re-observe** — test after each meaningful
stroke by rotating the joint, not at the end. Bind with the harshest settings first, so each
joint drives only its own section, then soften. Stress-test with a series of extreme poses as
soon as the mesh is skinned, not once it is finished.

Attention goes to elbows, knees, shoulders, hips, wrists and fingers — where deformation actually
happens. Everything else is a formality.

## A validation checklist

What studios enforce automatically, and what is worth a guard here:

- [ ] naming convention holds, both sides, no typos
- [ ] hierarchy matches the standard (retargeting depends on it)
- [ ] no vertex has more than 4 influences
- [ ] no vertex has zero influences
- [ ] weights sum to 1
- [ ] left and right are mirrors, to a stated tolerance
- [ ] custom split normals survive every mesh edit
- [ ] the rest pose does not move when it is not meant to
- [ ] extreme poses do not tear — measured on coincident vertices, not eyeballed
- [ ] bone count inside budget for the platform

The last-but-one is worth spelling out. A mesh built for hard-edge shading holds its surface
together with **duplicate vertices at every seam**. If two copies of a point end up on different
bones they come apart, and under a texture a seam and a hole look identical. Weld by position in
the rest pose, pose the extreme, and measure whether each welded group is still in one place.
On Copaimo's hands: 698 shared positions, worst split **0.000 mm**.

## Sources

- [Character Rigging for Games: Setup & Skinning — MoCap Online](https://mocaponline.com/blogs/mocap-news/character-rigging-game-dev-guide)
- [Character Rigging Basics — MoCap Online](https://mocaponline.com/blogs/mocap-news/character-rigging-basics-guide)
- [Skeleton Hierarchy: Why Your Retargets Keep Breaking — MoCap Online](https://mocaponline.com/blogs/mocap-news/skeleton-hierarchy-animation-guide)
- [Human Rig Bone Names — CG Typhoon](https://cgtyphoon.com/rigging/bones-naming-in-the-human-character-rig/)
- [Skeletons in Unreal Engine — Epic](https://dev.epicgames.com/documentation/en-us/unreal-engine/skeletons-in-unreal-engine)
- [Mecanim humanoids — Unity](https://unity.com/blog/engine-platform/mecanim-humanoids)
- [Avatar Muscle & Settings — Unity Manual](https://docs.unity3d.com/Manual/MuscleDefinitions.html)
- [The skinning in character animation: a survey (PDF)](https://francis-press.com/uploads/papers/cG4LVr6lQiYEurCzBzzuVCkbV6taxZ4kEYJKgLAz.pdf)
- [Velocity Skinning for Real-time Stylized Skeletal Animation (arXiv)](https://arxiv.org/pdf/2104.04934)
- [Rigging the Fingers and Thumbs — CAVE Academy](https://caveacademy.com/wiki/post-production-assets/rigging/rigging-training/introduction-to-rigging-course/08-rigging-the-fingers-and-thumbs/)
- [3D Hand Model Rigging guide — Tripo](https://www.tripo3d.ai/blog/explore/3d-hand-model-rigging-guide)
- [Low-poly hand topology — Tripo](https://www.tripo3d.ai/blog/explore/smart-mesh-topology-for-hands-fingers-low-poly)
- [Apply pose as rest pose — Blender Manual](https://docs.blender.org/manual/en/latest/animation/armatures/posing/editing/apply.html)
- [Polygon budgets by platform 2026 — low-poly.com](https://low-poly.com/blog/polygon-budgets-by-platform-2026)
- [Creating models for optimal performance — Unity Manual](https://docs.unity3d.com/Manual/ModelingOptimizedCharacters.html)
- [Weight painting best practices — Whizzy Studios](https://www.whizzystudios.com/post/best-practices-for-weight-painting-in-character-rigging)
