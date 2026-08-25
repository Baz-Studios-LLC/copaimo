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

## The key poses of a run, and what the feet do in each

**SOURCE.** [The Key Poses of a Run Cycle - AnimSchool](https://blog.animschool.edu/2024/04/10/the-key-poses-of-a-run-cycle/)

Four poses, and the feet are doing something different in each:

| Pose | The feet |
|---|---|
| **Contact** | The lead foot meets the ground. Feet closer together, near the body's centre of mass. |
| **Down** | The lowest, most squashed pose. Knee and ankle bend to absorb it; the foot is flat. |
| **Push** | The foot rolls forward from the ball, the heel lifts, the ankle plantarflexes to drive off. |
| **Peak** | Airborne, both feet off the ground. Knees hold their shape from Push. |

**The article draws the distinction that decides the target.** A *realistic* run lands on the
BALL of the foot. An *exaggerated* one lands heel-first with the foot farther from the body.
Copaimo is stylised by policy, so heel-first is the target here.

**MEASURED (Copaimo, 2026-08-25).** The delivered run had no roll at all. Tracking the heel and
the toe separately, the toe sat lower than the heel through the WHOLE stance and the foot pitch
never went negative - he contacted toe-first and stayed on his toes to push-off - and every toe
key in every clip was an identity, so there was no metatarsal break anywhere.

`build_character::roll_the_feet` now rolls each planted foot: -8 degrees of pitch at contact
(heel down, toe up), flat by 35% of stance, +45 with a 35 degree toe break at push-off.

> **⚠ It only fixes the ANGLE.** A foot rolls about its contact point - the heel at contact, the
> ball at push-off - and rotating about the ANKLE instead leaves the ankle where the clip put it.
> The clip puts it high: the heel is still 3-4 cm off the floor at contact. Getting it down means
> moving the ankle, which means solving the leg, which is `src/ik.rs` and not a second copy of it
> in Python. **Open:** either make runtime planting absolute during stance, or emit targets for
> the Rust solver the way `solve_a_leg_for_blender` already does.

## Jogging, specifically  (researched 2026-08-25)

Everything above this section is about walks, runs and sprints. Copaimo ships a WALK and a JOG
and nothing else, and a jog is not a slow run - it has its own speed, its own cadence and its own
foot strike. This is what the sources say, and what ours measures against it.

### The speed tiers, in real units

**STANDARD.** ([MoCap Online, locomotion design](https://mocaponline.com/blogs/mocap-news/locomotion-animations-game-dev))

| Tier | Speed |
|---|---|
| Walk | 2–4 km/h |
| **Jog** | **6–8 km/h** |
| Run | 10–12 km/h |
| Sprint | 15+ km/h, usually forward-only |

**MEASURED (Copaimo, 2026-08-25).** `WALK_SPEED` 1.07 m/s = **3.9 km/h**, a walk, at the top of
the band. `JOG_SPEED` 4.00 m/s = **14.4 km/h**.

> **⚠ The jog is moving at sprint speed.** 14.4 km/h is past the whole RUN band and into the
> sprint's. It is not a jog by any definition in the source; it is a sprint being played by a
> clip authored as something else. A jog at 6–8 km/h is **1.67 to 2.22 m/s**, less than half of
> what is driven now.
>
> This is the same shape of fault as `covers` describing a clip it no longer belonged to: a
> number chosen by feel that nothing ever checked against what it claims to be. And it explains
> why the clip has to be stretched so far - see the cadence below.

### Cycle length

**STANDARD.** A run cycle is **16–20 frames at 30 fps**, about half a second per stride; a sprint
is shorter at 12–16. ([MoCap Online, run cycles](https://mocaponline.com/blogs/mocap-news/run-cycle-animation))
A jog is conventionally animated **"on tens"** - one foot strike every 10 frames, so 20 frames a
full cycle. ([LinkedIn Learning, 2D animation](https://www.linkedin.com/learning/2d-animation-character-attitude-walk-cycles/animating-a-jog-cycle-on-tens))
Walks follow an **8-count** structure: 8, 16 or 32 frames, left foot on count 1 and right on
count 5.

**MEASURED (Copaimo).** The jog clip is 25 frames at 24 fps = 1.04 s a cycle. Played at
`JOG_SPEED` it is 27.7 effective frames. Against a jog's 20-at-30fps - which is 0.67 s, or 16
frames at our 24 - **the cycle is roughly 1.7x too long**.

### Cadence

**STANDARD.** Recreational runners run at **150–170 steps a minute**; efficient trained adults sit
at **170–185**; elites exceed 180.
([Marathon Handbook](https://marathonhandbook.com/running-form-hub/),
[Princeton Sports Medicine](https://www.princetonmedicine.com/blog/unraveling-the-science-of-running-biomechanics))
Raising cadence 5–10% shortens stride and cuts braking force.

**MEASURED (Copaimo).** 108 steps a minute at `JOG_SPEED` 4.00, and 130 at the old 4.80. **Both
are below the bottom of the recreational band**, at a speed above the top of the run band. That
combination has one meaning: **the stride is far too long for the speed** - measured, 4.435 m a
cycle, which is 2.6x his own height where a jog is nearer 1.4–1.8.

### Ground contact, and what it implies for stance

**STANDARD.** Ground contact in running is **200–300 ms**. At 24 fps that is **5 to 7 frames per
foot**, and it is the same however long the cycle is.

**MEASURED (Copaimo).** The jog's stances run 2 to 5 frames. The short ones are why stance
detection has been so fragile - a two-frame contact has no middle, so a fade at each end swallows
it whole.

### Foot strike

**STANDARD.** Three patterns, and they are speed-dependent
([Frontiers in Sports and Active Living](https://www.frontiersin.org/journals/sports-and-active-living/articles/10.3389/fspor.2022.768801/full),
[Princeton](https://www.princetonmedicine.com/blog/unraveling-the-science-of-running-biomechanics)):

* **Heel strike** - the heel lands first. Common among RECREATIONAL runners, which is what a jog
  is. Higher knee impact.
* **Midfoot** - distributes force most evenly.
* **Forefoot** - the ball lands first. Sprinters. Less knee load, more calf and Achilles.

Stride length and foot strike are **coupled**: a longer stride pushes the foot out in front of
the centre of mass and toward a heel strike; shortening it moves the strike back toward midfoot.

> **→ For Copaimo.** A jog is a recreational pace, so heel-first is right, and that agrees with
> what was asked for. But note the coupling: this clip's 2.6x-height stride *forces* a heel
> strike whatever anyone intends. Fixing the stride and fixing the strike are the same job.

### Trunk lean

**STANDARD.** A run has a "slight forward lean"; the pronounced **15–30°** figure belongs to a
SPRINT and nothing slower. ([MoCap Online](https://mocaponline.com/blogs/mocap-news/run-cycle-animation))
This agrees with the biomechanics already recorded in `TROUBLESHOOTING.md`: real trunk flexion is
4–12°, most economical near 6.

**MEASURED (Copaimo).** The jog now sits at **+6.2° off vertical**, measured through the flesh.
Correct for the tier.

### Arm swing

**STANDARD.** Opposite arm to opposite leg, and it is a balance mechanism rather than decoration -
the counter-rotation cancels the torso rotation the legs produce. Runs show "stronger arm drive"
than walks; a sprint's "aggressive arm pump" is again a sprint trait.

**MEASURED (Copaimo).** 81.3° of shoulder swing with 56% of frames within 15% of an extreme,
which is a pump rather than a glide. Elbows carried at 85° with a 68–98° range.

## Foot rigging: the reverse-foot setup  (researched 2026-08-25)

**STANDARD, and Copaimo does not have it.** The industry answer for feet is the **reverse foot
rig**: a chain that runs BACKWARD from the toe tip through the ball and the heel to the ankle, so
the foot can pivot about whichever of those points is on the ground.
([Blender Artists](https://blenderartists.org/t/reverse-foot-ik-rig/430676),
[Whizzy Studios](https://www.whizzystudios.com/post/how-to-set-up-a-reverse-foot-lock-in-rigging),
[CAVE Academy](https://caveacademy.com/wiki/post-production-assets/rigging/rigging-training/introduction-to-rigging-course/07-rigging-the-feet-2/),
[BlenderNation](https://www.blendernation.com/2021/05/29/advanced-foot-rig-made-easy-pivot-and-roll/))

It gives, from a handful of bones: **heel roll, ankle pivot, ball roll, toe roll, toe pivot and
toe wiggle**. The IK is two handles - hip to ankle for the leg, ankle to ball for the toe lift.

> **⚠ This is the answer to a fault this project hit repeatedly.** A foot rolls about its CONTACT
> POINT - the heel at strike, the ball at push-off - and Copaimo's foot rotates about its ANKLE,
> which is why every roll correction lifted the heel off the floor instead of pivoting on it. The
> reverse foot is exactly the machinery that makes the pivot move to where the ground is.
>
> **MEASURED (Copaimo).** Two bones per foot, `Foot` and `ToeBase`. No heel bone, no ball bone,
> no reverse chain, and `docs/rigging.md`'s bone budget allows four per foot. There is room.
>
> It does NOT arrive for free: the export rig is FK-only (see `src/ik.rs`), so a reverse foot
> would be authored-through rather than shipped - the pivots drive the bake, and what leaves is
> still `Foot` and `ToeBase`. That is the normal arrangement and it is what makes it worth doing.

## Locomotion systems, as AAA builds them  (researched 2026-08-25)

**STANDARD.** ([MoCap Online, locomotion design](https://mocaponline.com/blogs/mocap-news/locomotion-animations-game-dev),
[MoCap Online, state machines](https://mocaponline.com/blogs/mocap-news/animation-state-machine-design-patterns),
[Lyra breakdown](https://www.jaydengames.com/posts/ue5-black-magic-game-core-animation/))

* **Speed-based blend spaces**, not switches. A typical axis: idle 0, walk 150 cm/s, jog 375,
  run 600.
* **Foot IK by line trace** - trace down from each foot bone, reposition it, and rotate the ankle
  to the surface normal. This is what `src/ik.rs` does.
* **Additive layers for everything that is not locomotion** - breathing applied additively at all
  times, recoil as an additive loop on the upper body, hit reactions additive by direction. Mesh
  space additives read better on a moving character than local space.
* **Clip counts.** Minimal loop-only 5–8. Production-complete, with starts, stops and turns,
  **40–80**. Full AAA across stances and tiers, **80–150+**.
* **Motion matching** (UE 5.4+) replaces the state machine entirely: pick the best-fitting clip
  from a database each frame by pose and trajectory, rather than naming transitions.

**MEASURED (Copaimo).** Three clips - idle, walk, jog - against a minimal set of 5–8. No starts,
no stops, no turns, no strafes. That is the honest scale of what is here, and it is worth having
in front of us before any more time goes into perfecting one of the three.

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

- [Run Cycle Animation: The Developer's Guide — MoCap Online](https://mocaponline.com/blogs/mocap-news/run-cycle-animation)
- [Locomotion Animations: Walk, Run, Blend Trees — MoCap Online](https://mocaponline.com/blogs/mocap-news/locomotion-animations-game-dev)
- [Animation State Machines: Patterns for 200+ States — MoCap Online](https://mocaponline.com/blogs/mocap-news/animation-state-machine-design-patterns)
- [The Key Poses of a Run Cycle — AnimSchool](https://blog.animschool.edu/2024/04/10/the-key-poses-of-a-run-cycle/)
- [Animating a jog cycle on tens — LinkedIn Learning](https://www.linkedin.com/learning/2d-animation-character-attitude-walk-cycles/animating-a-jog-cycle-on-tens)
- [The Coupling of Stride Length and Foot Strike in Running — Frontiers](https://www.frontiersin.org/journals/sports-and-active-living/articles/10.3389/fspor.2022.768801/full)
- [Unraveling the Science of Running Biomechanics — Princeton Sports Medicine](https://www.princetonmedicine.com/blog/unraveling-the-science-of-running-biomechanics)
- [Running Form: Cadence, Foot Strike + Drills — Marathon Handbook](https://marathonhandbook.com/running-form-hub/)
- [Reverse Foot IK rig — Blender Artists](https://blenderartists.org/t/reverse-foot-ik-rig/430676)
- [How to Set Up a Reverse Foot Lock in Rigging — Whizzy Studios](https://www.whizzystudios.com/post/how-to-set-up-a-reverse-foot-lock-in-rigging)
- [Rigging the Feet — CAVE Academy](https://caveacademy.com/wiki/post-production-assets/rigging/rigging-training/introduction-to-rigging-course/07-rigging-the-feet-2/)
- [Advanced Foot Rig Made Easy, Pivot and Roll — BlenderNation](https://www.blendernation.com/2021/05/29/advanced-foot-rig-made-easy-pivot-and-roll/)
- [Lyra Breakdown, Game Core Animation — ebp](https://www.jaydengames.com/posts/ue5-black-magic-game-core-animation/)
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
