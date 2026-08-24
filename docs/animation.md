# Animation

## The foot-sliding problem, and the solved answer

This is the one to read first, because we spent weeks on it and it is a named problem with named
solutions.

**STANDARD.** Two ways to move a character:

| | Root motion | In-place |
|---|---|---|
| Where velocity comes from | the animation | code |
| Responsiveness | delayed by the clip | instant |
| Tuning speed | re-export, re-time every clip | change a variable |
| Foot sliding | none by construction | **the standard failure** |
| Non-cyclic moves (lurches, turns) | bake them in | need code coordination |
| Networked / server-authoritative | awkward | strongly preferred |

In-place is right for anything wanting snappy control, and it comes with foot sliding: the legs
churn at one speed while the capsule moves at another. **This is not a bug to be got exactly
right once. It is a permanent tension with three standard mitigations.**

### 1. Distance matching

**STANDARD.** Drive the animation's playback position from **distance travelled** rather than
elapsed time. Author a distance curve alongside the clip — pose against distance — and at runtime
look up the pose for the distance actually covered.

The point: it "reduces the need to fine-tune animations and allows the character's speed to be
altered without disrupting animation playback." Distance matching effectively changes the play
rate to keep feet planted, continuously, rather than relying on a constant being right.

Also used for transitions — matching a stop or a start to the distance remaining, so the plant
lands where the capsule stops.

### 2. Stride warping (a.k.a. speed warping)

**STANDARD, and the number is the useful part.** Instead of speeding the clip up, **change the
stride length** — move the foot positions apart or together, and let the legs' timing alone. On
Paragon, Epic scaled motion **up to 60% by stride warping, with a further 15% by play rate**.

That ratio is the lesson. Play rate is the *small* adjustment; stride length is the big one.
Pushing everything through play rate is why a clip ends up churning.

### 3. Foot IK

**STANDARD.** Raycast down from each foot, put the IK target on the hit, solve two-bone IK on
hip/knee/ankle, and aim the ankle to the ground normal. Two-bone IK is cheap — dozens at
negligible cost.

Two details that separate a working system from a broken one:

- **Adjust the hips.** Without lowering or raising the pelvis by roughly the average foot
  offset, the body stays at its animated height while the feet stretch down to the ground, which
  reads as a character on stilts.
- **Only apply the offset at the moment of plant, then hold it until the foot lifts.** Applying
  IK continuously *during* the plant phase makes the foot slide — the exact thing it is there to
  prevent. Mark plant and swing phases with events.

> **→ For Copaimo.** The current design has none of these three. It computes a per-clip `covers`
> distance and sets `playback_rate = speed * lasts / covers`, so **the whole burden is on one
> constant being exactly right**, and being wrong by 28% is what caused the "running through
> water" feel for weeks. That is the fragile version of distance matching.
>
> Ranked by value for effort:
> 1. **Stride warping.** Biggest win, and it is what would let one clip cover a speed range
>    rather than needing the tier's native speed to line up. Paragon's 60/15 split says the
>    headroom is large.
> 2. **Foot IK.** The heightmap world makes this close to mandatory eventually — right now feet
>    plant at the clip's authored height, not the terrain's.
> 3. **Distance matching proper.** Removes `covers` from the critical path entirely: if playback
>    follows measured distance, a mis-measured constant stops mattering. Note the existing
>    `measure_covers.py` already produces the data a distance curve needs.

## Timing

**STANDARD, at 24 fps:**

| Cycle | Frames |
|---|---|
| Walk | 24–32 |
| Run | **12–16** |
| Sprint | shorter still |

People walk "on 12s" — one step every 12 frames, two steps a second.

A run's key poses are the same four as a walk — **contact, down, passing, up** — compressed, plus
a flight phase. What makes it a run rather than a fast walk is that both feet are off the ground
for at least two frames.

**MEASURED (Copaimo, 2026-08-23).** All three gaits are authored at **24 frames**, i.e. 1.0 s at
24 fps. Against the 12–16 standard for a run that is roughly double. Working the numbers through:

| Clip | `covers` | Native speed | Tier speed | Playback rate | Effective cycle |
|---|---|---|---|---|---|
| walk | 0.970 m | 0.97 m/s | `WALK_SPEED` 0.93 | 0.96x | 25 frames |
| run | 2.496 m | 2.50 m/s | `JOG_SPEED` 3.70 | **1.48x** | 16.2 frames |
| sprint | 3.283 m | 3.28 m/s | `SPRINT_SPEED` 5.90 | **1.80x** | 13.3 frames |

So the *effective* cycles land inside the standard — 16.2 and 13.3 frames — and the feel is right
for that reason. It is the **authored** frame count that is long, with playback rate making up
the difference.

The build refuses odd spans: a cycle is two identical steps, so the half-cycle must land on a
frame. 15 was tried and refused at 21% asymmetry between the halves, which is a limp.

> **⚠ The comments in `animate_ranger.py` around line 3146 do not match the code.** They state
> "twenty-four frames a cycle for a walk and sixteen for both the jog and the sprint", compute
> native speeds from 16 and 14 frames ("2.282 over 16 is 3.42, and 3.50 over 14 is 6.00"), and
> claim `src/motion.rs` "places each tier at its own clip's native speed". All three calls pass
> **24**, the real native speeds are 2.50 and 3.28 m/s, and the tiers sit at 1.48x and 1.80x of
> them. The same paragraph also appears twice, slightly reworded.
>
> Nothing is broken by this — `playback_rate` keeps the feet planted at any rate provided `covers`
> is right — but it is the *live* version of "a number stated in two places will disagree with
> itself", sitting in the exact file that cost weeks. Worth correcting before anyone reasons from
> it.

> **→ For Copaimo. OPEN.** Authoring the run at 16 and the sprint at 14 frames would put the
> authored timing where the effective timing already is, bringing playback rate near 1.0 and
> making `covers` far less load-bearing. Both are even, so the verifier would accept them. It
> re-times every arm and leg curve though, so it is a session of its own.

## The 12 principles, and which ones fight games

**STANDARD.** Squash and stretch, anticipation, staging, straight-ahead vs pose-to-pose,
follow-through and overlapping action, slow in and slow out, arcs, secondary action, timing,
exaggeration, solid drawing, appeal.

Two carry most of the weight in game locomotion:

- **Timing and spacing.** Timing is how many frames a motion takes; spacing is where each frame
  sits. Wide spacing reads as fast, tight spacing as slow. Same frame count, different feel.
- **Follow-through and overlapping action.** Different parts arrive at different times. This is
  what makes a body read as connected rather than a rig of independent levers.

**And the one that actively works against games: anticipation.** It is the biggest source of
awkwardness between film and game animation. A windup adds time before the action, and the player
pressed the button *now*. A long anticipation on a jump makes a character feel sluggish however
good the animation is.

The game resolution is: anticipation on things the *world* initiates (an enemy telegraphing), not
on things the *player* initiates. For player actions, put the character into the action on frame
one and pay the follow-through afterwards.

> **→ For Copaimo.** Relevant to the Genshin-feel target. Genshin's responsiveness comes partly
> from having almost no anticipation on player-initiated moves.

## Blending: state machine outside, blend tree inside

**STANDARD.** Every production locomotion system uses both, at different levels:

- **State machine** at the outer level — idle, locomotion, airborne, combat. Discrete states with
  transition rules.
- **Blend tree** inside the locomotion state — idle → walk → run → sprint, blended on speed.

A **2D blend space** takes speed on one axis and direction on the other, idle at the centre, walk
at the cardinals, run further out, and interpolates. That is how strafing and turning get covered
without authoring a clip per direction.

**Additive layers** put one animation on top of another — breathing over any locomotion state, a
carry pose over every gait — instead of authoring the cross product.

> **→ For Copaimo.** `motion.rs` currently picks a discrete gait from intent and sets a playback
> rate: effectively a state machine with no blend tree. Discrete selection was the right fix for
> the jitter (choosing from *intent* rather than a noisy measured velocity is what Genshin does),
> but it means gait changes are a hard cut rather than a blend. Bevy has `AnimationGraph` with
> blend and additive nodes, so the tree is available without new machinery.
>
> Additive layers are also the cheap answer to a deferred request: a **carry / holding** layer
> over the existing gaits, rather than a second set of walk-run-sprint clips.

## Motion matching

**STANDARD.** Keep a large database of motion frames; each frame pick the one best matching
current pose, velocity and future trajectory. Popularised by Ubisoft in 2015, native in Unreal 5.
It replaces the hand-built state machine with a nearest-neighbour search over a feature set.

Strength is that transitions come free and style iterates fast. Cost is a **large motion capture
dataset** — the whole method assumes one.

> **→ For Copaimo.** Not applicable. It needs a mocap corpus this project does not have and
> hand-authoring cannot substitute for. Noted so it is not mistaken for the ambitious version of
> what we are doing; the ambitious version here is distance matching plus stride warping.

## Secondary motion

**STANDARD.** Secondary motion is the parts that react but are not animated — capes, loose
clothing, hair, pouches, straps, dangling gear. Hand-keyframing it is expensive and nobody does.

The standard tool is **spring bones** (jiggle bones): a chain of bones with spring constraints
that trails the parent animation, with stiffness, damping and gravity as the knobs. Cheap, and
convincing for ponytails, small capes and hanging equipment. Full cloth simulation is the
expensive alternative and rarely justified for these.

> **→ For Copaimo.** This is the answer to "the jacket and necklace should move when the character
> runs, nothing crazy, just to give him more life instead of everything being fused." Two short
> spring chains — one on the necklace, one on the jacket hem — not cloth simulation. Needs bones
> in those places first, which is a rig change of the same kind as the fingers.

## Game feel

**STANDARD.** Responsiveness is engineered, not just animated:

- **Input buffering** — store an input for a few frames so a press slightly early still counts.
  Windows are not uniform per action: roughly **6–8 frames for attacks, 3–4 for dodges**.
  Over-buffering makes controls "sticky" — actions firing after the player let go.
- **Coyote time** — allow a jump a few frames after leaving a ledge.
- **Animation cancelling** — letting a new action interrupt a playing one. Reported as making
  more difference to feel than buffering, for less work, and worth having in anything fast.

The interaction to watch: if a buffered input cancels an animation early, it can look jarring.
Blend out rather than cut.

## Sources

- [Distance Matching in Unreal Engine — Epic](https://dev.epicgames.com/documentation/en-us/unreal-engine/distance-matching-in-unreal-engine)
- [Pose Warping in Unreal Engine — Epic](https://dev.epicgames.com/documentation/en-us/unreal-engine/pose-warping-in-unreal-engine)
- [Animation in the Lyra Sample Game — Epic](https://dev.epicgames.com/documentation/unreal-engine/animation-in-lyra-sample-game-in-unreal-engine?lang=en-US)
- [Most Inspiring Game Animation Tech Talks of 2016 — Game Developer](https://www.gamedeveloper.com/programming/most-inspiring-game-animation-tech-talks-of-2016)
- [Game Anim interview: Laurent Delayen (Paragon animation)](https://www.gameanim.com/2016/11/29/game-anim-interview-laurent-delayen/)
- [Root Motion vs In-Place Animation — MoCap Online](https://mocaponline.com/blogs/mocap-news/root-motion-vs-in-place-animation)
- [Locomotion System Design — MoCap Online](https://mocaponline.com/blogs/mocap-news/locomotion-system-design-guide)
- [Run Cycle Animation: The Developer's Guide — MoCap Online](https://mocaponline.com/blogs/mocap-news/run-cycle-animation)
- [Walk Cycle Animation Tips — AnimSchool](https://blog.animschool.edu/2024/03/14/walk-cycle-animation-tips/)
- [Blend Trees in Game Engines — MoCap Online](https://mocaponline.com/blogs/mocap-news/animation-blend-tree-guide)
- [Animation State Machine patterns — MoCap Online](https://mocaponline.com/blogs/mocap-news/animation-state-machine-design-patterns)
- [Motion Matching: The Future of Game Animation — MoCap Online](https://mocaponline.com/blogs/mocap-news/motion-matching-games-guide)
- [Foot IK sample — ozz-animation](https://guillaumeblanc.github.io/ozz-animation/samples/foot_ik/)
- [Inverse Kinematics in Games Guide — MoCap Online](https://mocaponline.com/blogs/mocap-news/inverse-kinematics-games-guide)
- [Procedural Animation and IK in UE5.7 — StraySpark](https://www.strayspark.studio/blog/procedural-animation-ik-ue57-guide)
- [Jiggle Physics implementation guide — Wayline](https://www.wayline.io/blog/jiggle-physics-implementation-guide)
- [12 Principles of Animation, Reframed for Games — Animworks](https://anim.works/the-12-principles-of-animation-reframed-for-games/)
- [12 Principles for Game Animation — Chris Totten](https://totter87.medium.com/12-principles-for-game-animation-a9137ef44345)
- [The Art of Input Buffering — Wayline](https://www.wayline.io/blog/art-of-input-buffering)
- [Input Buffering, Action Canceling — Yosi Spring](https://medium.com/@yosispring/input-buffering-action-canceling-and-also-forbidden-knowledge-47a3f8a95151)
- [bevy::animation API docs](https://docs.rs/bevy/latest/bevy/animation/index.html)
- [Bevy Animation System — DeepWiki](https://deepwiki.com/bevyengine/bevy/9.2-animation-system)
