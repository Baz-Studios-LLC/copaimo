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

### 01 - The surface is trustworthy  (webbing CUT both sides 2026-08-24; holes and shoulder skinning remain)

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

### 02 - The skeleton is complete  (open)

41 joints and **none of them are fingers**. Running, jumping, lifting, petting a monster,
grabbing, crouching - every one needs hands, and NPCs need them too.

* **30 finger bones** - three phalanges, five digits, two hands, placed from measured hand
  geometry. A thumb is NOT the short digit, the splayed one, or the odd one out; a pinky is all
  three, and four discriminators in a row picked it before anyone looked.
* **Chest** - the chain is `Waist -> Spine01 -> Spine02`. A fourth torso joint is what lets a
  look-at rotate the chest rather than snapping the neck.
* Build spine-outward, then limbs, then extremities.

*Unblocks:* stage 07 entirely.
*Refuses when:* a rest transform moved, or a clip no longer plays identically.

### 03 - Skinning that survives motion  (measure first)

Already correct where it counts: at most four bones a vertex, weights summing to 1.0000. What is
unmeasured is whether it DEFORMS well.

* Strain audit - every edge's deformed length against its rest length, across every frame of
  every clip. Turns "the shoulder looks wrong" into an edge and a frame number.
* Twist distribution - 18 of 41 joints are twists and all are keyed in the clips, so the
  distribution is baked rather than procedural. Needs checking under poses the clips lack.
* New finger bones arrive unweighted and need geometry to fold.

*Unblocks:* layering. An additive layer amplifies a weight fault rather than hiding it.
*Refuses when:* an edge stretches past ~1.35x its rest length in any frame.

### 04 - Locomotion that does not slide  (part done)

`covers` is 2.542 m walking and 4.964 m running, taken from the root motion detrended out of
each clip. Both play within 3% of native:

    walk  101 steps/min at 1.07 m/s, played at 0.98x
    jog   116 steps/min at 4.80 m/s, played at 0.97x

* **Close the run loop** - 22.19 deg apart, so it pops every cycle.
* Blend tree walk<->run on speed, handover at a measured crossover.
* **Distance matching** so the cycle's phase is driven by ground covered rather than by elapsed
  time. This is the solved answer to foot sliding and it survives acceleration.
* Turn-in-place, so standing rotation is not a skate.

This run bounds: 4.96 m a cycle on a 1.7 m figure is about 1.6x a human stride for its cadence.
That is a property of the clip, not a fault - and it is why the cadence guard now reports rather
than refuses.

*Unblocks:* ground contact. IK corrects a base pose, so the base has to be right first.
*Refuses when:* playback multiple leaves 0.80-1.25x, or planted-foot velocity spread > 0.

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

### 08 - Secondary motion  (open)

Without it, hair and clothing move rigidly with the body and the whole character reads as one
solid object. Spring bones are the standard answer: a mass-spring-damper per bone, with
colliders on the body.

* Chains for the jacket hem, the pack and the hood.
* Sphere colliders approximating torso and upper arms - cheap, and enough to stop clipping.
* Damping tuned so it settles rather than oscillates.

Last in the runtime stack, because it reads the final skeleton and must never feed back.

*Refuses when:* a chain does not settle, or a bone passes inside a collider.

### 09 - Presentation  (open)

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
