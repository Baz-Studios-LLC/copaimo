# The warden character pipeline

Eleven stages from *imported and walking* to shippable, ordered so each one hands the next
something it needs. Every "why" is a number something printed, not a judgement.

There is a formatted copy of this at
<https://claude.ai/code/artifact/205aeaf6-2f00-4941-a6bc-f0efed471102>. **This file is the one to
edit** — tick stages off here, and the other is a snapshot.

## Where the character is now

Measured 2026-08-24 on `assets/models/person_ranger.glb`, built by `dev/art/build_character.py`
from the three files in `assets/character/`.

    mesh          2464 real vertices, 4899 triangles, 6 shells
    skeleton      41 joints, 18 of them twist bones
    fingers       0 of 30                      <- no hand articulation at all
    clips         idle 15.58 s, walk 2.375 s, run 1.033 s
    walk loop     0.04 deg first to last       <- clean
    run loop      22.19 deg                    <- does not close, pops every cycle
    skin          at most 4 bones a vertex, weights sum to 1.0000
    surface       10 open edges, 5 non-manifold edges
    straddling    260 faces across two body regions, 5.3% of the surface
    edges         median 2.70 cm, longest 27.79 cm

## Two bodies, one skeleton

The creator plans a male and a female body sharing this rig, which makes `the_skeletons_match`
load-bearing rather than a nicety: it refuses if two files' binds differ by more than a micron,
and that is the thing stopping a clip authored on one body from silently meaning something else
on the other. Any future bind change has to land on BOTH, which is an argument for leaving the
export rig as delivered wherever possible.

It also splits the pipeline's work in two by how it survives a second body:

* **Derived per build** - finger bones, bone lengths, the digit web - reads each mesh's own
  geometry and adapts to a new body with no intervention. This is the payoff for deriving.
* **Recorded** - the armpit webbing centroids - is a list of positions on THIS mesh. On another
  body it matches nothing and refuses, which is correct behaviour but means re-measuring per
  body rather than copying.

## Two orders, and they are not the same

The stages below are a BUILD order - what to make first so nothing has to be redone. That is a
different sequence from the RUNTIME stack, which is what evaluates each frame. Confusing them is
how foot IK ends up fighting an additive layer that was applied after it.

The runtime stack, bottom up:

    1  base locomotion      blend tree on speed, distance-matched
    2  additive layers      breathing, lean - added on top, never replacing
    3  masked overrides     upper body, so the legs keep walking underneath
    4  IK fixup             foot planting, hand attach, look-at
    5  secondary physics    spring bones, riding on the final skeleton

Each tier can only correct what is beneath it. That is why IK sits above the layers and physics
above everything - and why the export rig stays FK-ONLY, with every IK chain in the engine
rather than baked into a clip.

## The build order

### 00 - Instruments before subjects  (DONE 2026-08-24)

Every day lost on the previous character was a measurement failure. A shoe was reshaped seven
times because no render stripped the texture off it, and a guard refused an ankle at 2.2 cm on a
mesh that already carried 7.96 cm edges there.

* `inspect_glb.py` - reads a file's own JSON, so it reports what the FILE says rather than what
  an importer made of it.
* `render_clay.sh` - every material stripped to grey, plus `--silhouette`. A textured render
  cannot show form. Clay is the default and `--textured` is the exception.
* `audit_character.sh` - surface, skeleton, skin and clips in one run, welded by position.
  The pose-loop measure lives in here.
* `see_the_character.sh` - one Blender window, all clips in the Action Editor, skeleton in
  front. A render is one angle at one instant; half of what is wrong only shows when it moves.
* Contact sheet - **still to do**, for a fault reported twice.

*Unblocks:* everything. No later stage may claim a result one of these did not print.
*Refuses when:* a claim in a commit message has no measurement behind it.

### 01 - The surface is trustworthy  (DONE 2026-08-24, with the armpit re-scoped to 03)

Closed: the original 10 open edges were 3 closed loops, filled by `close_the_holes` with faces
over their own rim vertices - welded by position first, because on a split mesh an open loop is
not a chain of stored edges. The audit reads 0 open edges.

Named, deliberately left: the 5 edges with more than two faces are the hair meeting the head (3)
and the backpack meeting the back (2) - accessory attachment junctions, not defects. Altering
the ear or the strap to satisfy a manifold counter would be damage.

RE-SCOPED: the armpit webbing is NOT cut. Three builds proved the recorded faces are the ONLY
surface there - the "walls behind them" were backfaces, and cutting made real chest holes. The
46-face record stays in `build_character.py` as stage 03's measured worklist: reweight the chest
vertices the generator hung on the forearm twists, and model a gusset where the membrane is.

Bones are placed against geometry and weights are painted onto it, so faults here propagate into
every stage after.

    10 open edges, 5 non-manifold edges
    648 of 11217 edges stretch past 1.35x their rest length:
       315 in the run clip
       201 with the arms overhead      <- the armpit
        85 in the idle
        47 in a deep crouch
    worst: R_Clavicle <-> Spine02 at x6.76, arms overhead

**"An edge longer than 4x the median" was the wrong test and cost two rounds.** It counted 182
faults; rendered with them picked out in red they are the chest panel, the crotch and the
shoulder caps - ordinary large polygons on a body that welds to 2464 vertices. Adding "and it
spans two body regions" got it to 24, whose worst were `Waist <-> ThighTwist01` at the hip, where
a trunk and a leg are SUPPOSED to be one surface.

Length was never the question. A bridge is a bridge because it STRETCHES when the two things it
joins move apart, and that is measurable directly. `the_deformation` measures it over every clip
AND over four poses the clips never reach - in an idle, a walk and a run the arms barely leave
the sides, so armpit webbing never gets pulled and reports nothing.

**Do the mesh work once and commit it.** Re-deriving repairs on every build asks a classifier to
make the same judgement forever and never once get it wrong. It cut sleeve cuffs, holed a
trouser leg and took part of a shoulder. Rig repair is derived per build; sculpting is done
once, checked by eye, and kept.

Also: no `TANGENT` attribute, so the normal map has nothing to light against.

*Unblocks:* skeleton placement, weight painting, every deformation judgement after them.
*Refuses when:* open edges > 0, non-manifold > 0, any edge past 4x the median.

### 02 - The skeleton is complete  (DONE 2026-08-25)

Done: 30 finger bones, derived per build by `add_the_fingers`. Digits found by graph distance
from the wrist over the position-welded surface; the five furthest-and-mutually-apart vertices
are the tips, each VALIDATED by the digit it produces - the left hand's first pick was a
sleeve-cuff vertex whose "digit" held one vertex, and it was banned and replaced rather than
trusted. The thumb is named by the one fact that cannot lie: its base branches off nearest the
wrist. Verified by curling single digits by name and looking: thumb curls the innermost digit,
pinky the outermost, on both hands.

UNFUSED by DEEPENING, 2026-08-25. Deleting the 36 inter-digit faces was the wrong operation: it
left 45 open edges that read as holes on the hands. A web between fingers is ANATOMY - every
hand has one, down to about the crotch - and what was wrong here was that it sat almost level
with the digits, so the hand read as a paddle. Nothing is deleted now; shared vertices sink
toward the wrist along their digit's own axis, faded to nothing by the fingertip. The digits
stand clear, and deleting nothing cannot open anything: 0 open edges.

Two measured corrections on the way. 0.30 of the digit length sank the crotch 4.14 cm on a 9 cm
hand and tore the left hand into ribbons - it is 0.08 now, 0.83 cm. And the "pinch toward the
seam" was not merely too large but WRONG in direction: pulling shared vertices toward the seam
between two digits drags both digits into each other, fusing them harder. Removed.

Still open for stage 03, stated plainly: a hard 45-degree curl still tears the left hand, whose
digits share more geometry than the right's. No clip curls the fingers today, so nothing shows.
Fixing it properly is the gusset work - real walls between the digits, modelled not deleted.

REVERTED 2026-08-25: the examine-hands beat. Three poses, three failures - hands at the belly,
hands through the jacket, elbows into each other. The elbow hinge on this rig sweeps ACROSS the
body, so the pose needs shoulder twist coordinated per arm against a hand TARGET, which is what
hand IK does and what composing fixed axis offsets cannot. It returns at stage 07, posed by the
solver. The measured axis record and the envelope machinery are kept behind `EXAMINES`.

41 joints and **none of them are fingers**. Running, jumping, lifting, petting a monster,
grabbing, crouching - every one needs hands, and NPCs need them too.

* **30 finger bones** - three phalanges, five digits, two hands, placed from measured hand
  geometry. A thumb is NOT the short digit, the splayed one, or the odd one out; a pinky is all
  three, and four discriminators in a row picked it before anyone looked.
* **Chest: NOT ADDED, because it already exists under another name.** Measured, `Spine02` spans
  113 to 130 cm and is the parent of both clavicles and the neck - it IS the chest joint. Rotating
  it 18 degrees swings the shoulder 5.5 cm, and a 60-degree look split 30/20/20/30 across
  `Spine02` / `NeckTwist01` / `NeckTwist02` / `Head` turns the gaze the full 68 degrees with the
  chest carrying its share. A fourth torso bone would duplicate that function and add a joint
  every clip would have to be re-authored around.

  Renaming `Spine02` to `Chest` was also rejected: clips address bones by NAME, and a rename
  silently kills every channel path that targets it. Stage 06's look-at uses the chain as it is.
* Build spine-outward, then limbs, then extremities.

*Unblocks:* stage 07 entirely.
*Refuses when:* a rest transform moved, or a clip no longer plays identically.

### 03 - Skinning that survives motion  (DIAGNOSED 2026-08-25; weights are not the fault)

Already correct where it counts: at most four bones a vertex, weights summing to 1.0000. And
measured now, the deformation is ALSO not a weighting fault.

Diagnosed with the arms forward - 307 tearing edges - by asking of each whether it is a hard
weight transition or too little geometry:

    246  both mild      short edge, small weight jump
     33  long edge      too little geometry to share the bend
     28  weights jump   a hard transition

WEIGHT SMOOTHING WAS TRIED AND MADE IT WORSE at every strength: 0.20 gave 504 tearing edges,
0.35 gave 488, 0.50 gave 476, against 452 before. Blurring pulls the clavicle's influence onto
spine vertices and the spine's onto the clavicle, so MORE vertices end up partly driven by a
swinging bone - it widens the affected region rather than easing the gradient, and on a body of
2464 vertices there is nowhere for a gradient to spread. Kept behind `SMOOTHS_WEIGHTS`, off,
because it is the right tool on a denser mesh.

AND THE TEARING IS NOT VISIBLE - checked on ALL THREE clips, not one. Rendered in clay at each
clip's own worst frame: run 10 (261 edges, peak 3.77), idle 428 (80 edges, peak 3.81), walk 49
(60 edges, peak 2.03). All three read clean - good poses, no distortion. The first pass checked
only the run and generalised, which was an inference wearing the clothes of a measurement.

Most of those edges are a JACKET stretching, which is what a jacket does. The threshold is a
screening tool, not a verdict.

The hole fills were also checked TEXTURED, not only in clay: a fill with wrong UVs smears, and
these do not - the jacket, the necklace and the trim all read continuous.

So stage 03's real content is the two MODELLING jobs, not weight painting:

* Strain audit - every edge's deformed length against its rest length, across every frame of
  every clip. Turns "the shoulder looks wrong" into an edge and a frame number. **Exists.**
* Twist distribution - MEASURED 2026-08-25 and correct as a rig: all 18 lie along their
  parent's length, all 18 carry skin (54-361 vertices each), and keying one 45 degrees moves
  skin 6.3 cm - more than the wrist itself. `audit_character.the_twists` checks all three every
  run, with a fresh depsgraph after each pose; a stale one reports 0.00 cm for every bone, which
  is what a broken rig looks like and what this measured before the fix.

  THE GAP IS NOT THE RIG, IT IS THE DRIVING: none of the 18 has a constraint, so they move only
  when a clip keys them. A PROCEDURAL wrist or ankle rotation - IK, look-at, a grip pose - will
  crease at the joint instead of winding along the limb. Standard practice is a copy-rotation
  constraint at a fraction of the child's roll. That is stage 05/06 work and is now on their
  lists rather than assumed handled.
* THE ARMPIT, from stage 01: reweight the chest vertices the generator hung on the forearm
  twists, and model a gusset where the arm-to-ribs membrane is. The 46-face record in
  `build_character.py` is the worklist; cutting it is proven wrong (real holes), so the fix is
  weights and geometry, not deletion.
* Twist distribution - 18 of 41 joints are twists and all are keyed in the clips, so the
  distribution is baked rather than procedural. Needs checking under poses the clips lack.
* New finger bones arrive unweighted and need geometry to fold.

*Unblocks:* layering. An additive layer amplifies a weight fault rather than hiding it.
*Refuses when:* an edge stretches past ~1.35x its rest length in any frame.

### 04 - Locomotion that does not slide  (DONE 2026-08-25)

`covers` is 2.542 m walking and 4.964 m running, taken from the root motion detrended out of
each clip. Both play within 3% of native:

    walk  101 steps/min at 1.07 m/s, played at 0.98x
    jog   116 steps/min at 4.80 m/s, played at 0.97x

* ~~Close the run loop~~ - **DONE.** The audit now reports every clip closing at 0.00-0.04 deg
  and every hip landing 0.0 cm from where it began. The 22.19 deg figure above is stale.
* ~~Distance matching~~ - **DONE 2026-08-25.** The phase is no longer integrated by the
  animation player from a rate handed to `set_speed`. `Strides` accumulates the gait CYCLES the
  warden has actually covered, the clip is seeked there, and its own speed is pinned at zero so
  the player integrates nothing. See `strides_over` in `src/motion.rs` for why rate matching had
  no feedback and this does.

  Counted in cycles rather than metres or clip fractions on purpose: the walk clip holds two
  cycles and the run one, so the same clip fraction means opposite feet, and
  `a_gait_change_lands_on_the_same_foot` is the test that holds it.

  This is also the fix for both remaining in-game reports - the jitter while running, and the
  stop that read as sliding backwards. **Needs a playtest to confirm; the tests cover the
  arithmetic, not the feel.**
* ~~Blend tree walk<->run on speed~~ - **DONE.** The gaits cross-fade instead of switching.
  `weights_for` is the tree, as a function of asked speed alone so it tests without an app;
  `eased` ramps toward the target so a blend takes time; `as_bevy_weights` converts intended
  shares into what Bevy actually needs. Distance matching is what made this safe to build - both
  clips agree on where in the cycle they are, so the cross-fade blends two poses of the same gait
  phase rather than two arbitrary ones.

  The **crossover is measured**: the speed at which both clips are stretched by the same factor,
  `sqrt(walk_native x run_native)` = 2.167 m/s. It was `halfway(WALK_SPEED, JOG_SPEED)`, which is
  a fact about the speeds a player is driven at and says nothing about the clips.

  `AnimationTransitions` is gone. It owned the cross-fade by declining weights over time, and two
  things writing the same weights is how a blend goes wrong invisibly.
* ~~Turn-in-place~~ - **DONE.** Through the same accumulator, with no new clip and no second
  mechanism: a pivot swings the feet along an arc about the turn axis, and that arc is fed in as
  distance. Pivot radius 0.119 m, measured off the idle rather than the A-pose bind, which stands
  half again as wide. It also falls out for free while walking a curve, where the arc is real
  ground the outside foot has to cover.

This run bounds: 4.96 m a cycle on a 1.7 m figure is about 1.6x a human stride for its cadence.
That is a property of the clip, not a fault - and it is why the cadence guard now reports rather
than refuses.

*Unblocks:* ground contact. IK corrects a base pose, so the base has to be right first.
*Refuses when:* playback multiple leaves 0.80-1.25x, or planted-foot velocity spread > 0.

**Both halves of the refusal are now built and green.** The playback multiple is asserted by
`neither_gait_plays_at_a_blur` - walk 1.01x, jog 1.08x against a 0.80-1.25x bound. The
planted-foot check is `the_footfalls` in `dev/art/audit_character.py`, which did not exist until
now, and finding it changed a number: see below.

#### What the footfall guard found, first time it ran

`covers` was taken from each clip's ROOT MOTION, and the root is the wrong source. It is what the
animator moved the hips by; what `covers` has to be is what the GROUND supports, because `covers`
is the divisor that turns distance covered into cycle phase. Measured off the planted feet:

    walk   feet 1.06 m/s   root claimed 1.09 m/s    -2.8%    ->  2.542 becomes 2.471
    run    feet 4.44 m/s   root claimed 4.96 m/s   -10.6%    ->  4.964 becomes 4.435

The run's root overshot its feet by more than a tenth, which is a 10% skate at every speed, and
nothing in the pipeline would have said so. Two instrument faults had to be fixed before that
number could be trusted - a planted test loose enough to admit swing frames, and a velocity read
in model units and compared against metres, which made both clips look 44% wrong by the same
fraction. A systematic gap that size on two independently authored clips is always the instrument.

A note on what the 0.80-1.25x bound now means. It described the multiple handed to `set_speed`,
and nothing is handed to `set_speed` any more. The bound still describes the cadence that emerges
from covering ground at a given speed, because that cadence is the same `speed / covers` the old
rate computed - `neither_gait_plays_at_a_blur` asserts on exactly that. It is a statement about
the clips, not about the playback mechanism, and it survived the change intact.

### 05 - Ground contact  (solver exists)

The two-bone solver, the reach budget and the sole-alignment maths are in `src/ik.rs` with tests
behind them. Generic code, written for the last character - it needs re-measuring against this
skeleton's proportions, not rewriting.

* Foot planting - trace to the ground, solve the leg, aim the ankle so the sole lies on the
  slope rather than the character standing on his toes.
* Hip drop, bounded, so a leg that cannot reach lowers the body instead of snapping straight.
* Step-up and slope limits that agree with the movement code's own climb limit.

*Unblocks:* the open world being walkable rather than a flat plane with scenery.
*Refuses when:* a sole is off the ground on a slope, hips drop past budget, a knee bends back.

### 06 - Layered motion  (open)

Where the character stops being a mannequin playing clips. Bevy's `AnimationGraph` has both
pieces already: add nodes for additive layers, masks for restricting a node to a set of bones.

* Additive idle - breathing and weight shifts over locomotion, so standing still is never still.
* Look-at - head and chest aiming, distributed across the chain rather than dumped on the neck.
  The Chest joint from stage 02 is what makes this read.
* Upper-body mask - carrying, petting and reacting while the legs keep walking underneath.

*Unblocks:* interactions that do not stop the character dead to play a clip.
*Refuses when:* a layer cannot be switched off independently, or a bone is transformed twice.

### 07 - The interaction set  (needs hands)

Grab, lift, carry, pet, crouch, jump, land. Blocked entirely on stage 02.

* Hand IK to attach points, so a carried object is held rather than intersected.
* Grip poses per object class, blended in rather than switched.
* Jump and land as additive over the falling state, landing depth driven by fall distance.

*Unblocks:* the monster-companion loop, which is the game.
*Refuses when:* a contact point misses its target by more than a set tolerance.

### 08 - Secondary motion  (SHRUNK 2026-08-25 - hair will not be animated)

The character creator plan settles this stage's scope: hair is swappable styles, static, so the
main customer for spring bones is gone. What remains is the jacket hem and the pack, and both
are arguable. **Reconsider whether this stage earns its place at all** before building it.

The collider set is still worth having IF springs are built - a few spheres on `Spine01`,
`Spine02`, the upper arms and thighs. That is NOT body self-collision, which games essentially
never do on a character: authoring fixes self-intersection, and every one seen on this character
so far was an authoring fault that collision would have hidden rather than shown.

Without it, hair and clothing move rigidly with the body and the whole character reads as one
solid object. Spring bones are the standard answer: a mass-spring-damper per bone, with
colliders on the body.

* Chains for the jacket hem, the pack and the hood.
* Sphere colliders approximating torso and upper arms - cheap, and enough to stop clipping.
* Damping tuned so it settles rather than oscillates.

Last in the runtime stack, because it reads the final skeleton and must never feed back.

*Refuses when:* a chain does not settle, or a bone passes inside a collider.

### 09 - Presentation  (open - and the skin split is now LOAD-BEARING)

**Decide the skin/clothing split before painting anything.** The creator offers skin colour
options, and everything is currently in ONE atlas - skin, jacket, trousers and shoes painted
together. Recolouring skin needs either a mask channel marking skin pixels or skin on its own
material. Both are easy now and awkward once there is a texture worth keeping.

**A head socket for hair.** Swappable styles want an attachment point they parent to, so a
change is one transform rather than a re-skin.

Three textures shipped with this character against the last one's single base colour. Cheap
relative to everything above, and should not start before it: a better-lit wrong deformation is
still wrong.

* Verify what the three maps are and that the material reads them.
* Tangents, without which the normal map does nothing.
* LODs - not urgent at 4899 triangles, urgent the moment a crowd of NPCs shares the rig.
* Shadow and contact-shadow quality, which is what sells him as standing ON the ground.

*Refuses when:* a map is bound to the wrong slot, or normals light as a different shape.

### 10 - Regression safety  (habit exists)

Three gaits shipped with backwards knees before anything could refuse them, and every one was
caught by the person playing the game rather than by the build.

* Golden images - a clay render of fixed poses, diffed every build. Form faults are invisible to
  every numeric guard.
* Every audit runs on every build, printing loudly, refusing where a refusal is honest.
* A guard compares against the SPEC, never against its own input, and knows its baseline.

*Refuses when:* a golden image drifts, or an audit is skipped to make a build pass.

## Why this order and not another

* **Instruments first**, because the failures worth avoiding are believing something without a
  number behind it.
* **Surface before skeleton**, because bones are placed against geometry.
* **Skeleton before skinning**, because weights are per-bone.
* **Skinning before layering**, because additive motion amplifies a weight fault.
* **Base locomotion before IK**, because IK corrects a pose - it cannot invent one.
* **Hands before interactions**, which is why stage 02 sits as early as it does.
* **Physics last**, because it reads the final skeleton and must never feed back.

The dependency worth watching is 02 against 04: adding finger bones changes the skeleton, and a
clip cannot be copied across a bind change. Either the fingers go on before any new clips are
authored, or every clip after them needs retargeting. **Stage 02 next is the cheaper of those.**

## Sources beyond `docs/`

* [MoCap Online, game rig setup and FK-only export](https://mocaponline.com/blogs/mocap-news/character-rigging-game-dev-guide)
* [StraySpark on the modern locomotion stack](https://www.strayspark.studio/blog/motion-matching-control-rig-ue5-animation)
* [The Lyra animation breakdown](https://www.jaydengames.com/posts/ue5-black-magic-game-core-animation/)
* [Wayline on spring-bone implementation](https://www.wayline.io/blog/jiggle-physics-implementation-guide)
* [Bevy AnimationGraph: masks and add nodes](https://docs.rs/bevy/latest/bevy/prelude/struct.AnimationGraph.html)

## Roll distribution landed early (2026-08-25)

Stage 05/06 work, pulled forward because the elbow twist was reported three times and it was the
fix. The rig's roll bones - `ForearmTwist01/02`, `UpperarmTwist01/02` - carry ALL the arm skin,
and the bend bones `Forearm` and `Upperarm` carry none. The delivered clips left every roll bone
at exactly 0.0, so the forearm's twist was inherited whole and the whole forearm turned as a
rigid block.

`spread_the_twist` now decomposes each forearm key into swing and twist and grades the twist down
the chain, with shares measured from where each roll bone's skin actually sits. What remains of
the original Stage 05/06 note is the CONSTRAINT version of this: the export rig stays FK-only, so
the distribution is baked at build time rather than driven by a constraint, and anything that
rotates the arm PROCEDURALLY at runtime - an aim offset, an IK fixup - will still crease, because
it will not be spread. That is the piece still outstanding.

Two other things landed with it, both pose-level rather than geometry-level, and both worth
keeping in mind for the character creator because pose fixups adapt to a second body and mesh
edits do not:

* `lift_the_arms` holds a floor of 16 degrees of abduction at the shoulder, because the delivered
  idle rested the right arm 4 degrees tighter to the torso than the left and it read as attached.
* `move_the_arms_more` amplifies the idle's arm motion 1.45x about the clip's own mean pose, so
  the swing grows without moving where he rests.

Order matters and is enforced by the build: amplify, then lift, then spread. Amplifying after the
lift would push the inner extreme back into the ribs; spreading before amplifying would
redistribute a twist that then gets scaled.


## Honest limits of stage 04 (2026-08-25)

Written down because they are properties of the CLIP SET, not bugs, and the next person will
otherwise rediscover them.

* **There is no jog clip, and the gap shows at the crossover.** The walk tops out near 1.32 m/s
  and the run bottoms out near 3.55, so between them neither clip is inside `PLAYS_BETWEEN`. At
  the 2.167 m/s crossover both sit about 2.05x from their own native rate. The driven speeds are
  chosen near the clips' natives so the gap is only crossed while accelerating, and the blend is
  what makes crossing it bearable - but a jog clip is the real fix and nothing here can invent
  one. `neither_gait_plays_at_a_blur` checks the DRIVEN speeds, not the crossover, and that is
  deliberate rather than an oversight.
* **Turning in place plays the walk.** The feet step round rather than skate, which was the
  fault. An authored turn-in-place clip would pivot properly instead of stepping; this is the
  version that needs no new asset.
* **The run's stance is 10 frames inside 3 cm of the floor**, so its footfall mean rests on a
  thinner sample than the walk's 44. The median makes it robust rather than exact.
* **None of it has been playtested.** The tests cover arithmetic and the audit covers geometry.
  Whether the jitter and the abrupt stop are gone needs eyes on the running window.

## Deferred, on purpose (2026-08-25)

Not forgotten, and not blocked - held back because the user asked to keep moving and do another
pass over the whole character later.

* **The fingers do not move.** "his fingers should move too but lets move on to the next step."
  The 30 finger bones from stage 02 are rigged, weighted and unfused, and every clip leaves them
  static. Two separate wants sit behind this and they want different work: a hand that RESPONDS
  (grip poses, an additive finger layer over locomotion) is stage 06/07, while a hand that simply
  is not frozen during an idle is a clip edit of the same kind as `move_the_arms_more` - a small
  authored curl-and-release, amplified about the mean.
* **Another pass over everything.** Explicitly requested. Worth doing after stage 06 exists,
  because a layered-motion system changes what is worth authoring into a clip by hand.
