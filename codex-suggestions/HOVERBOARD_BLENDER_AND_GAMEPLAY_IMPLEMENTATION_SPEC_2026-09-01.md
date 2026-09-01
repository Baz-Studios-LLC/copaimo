# Hoverboard: Blender, animation, traversal and integration specification

**Date:** 2026-09-01  
**Audience:** Claude  
**User decision:** add a personal hoverboard that gives the player faster travel before mounts unlock.  
**Status:** requested feature; implementation remains Claude's responsibility. Codex changed no game file.

## 1. Product intent

The hoverboard should solve early-game travel without stealing the fantasy, utility or progression value
of mounts. It is the Warden's compact personal vehicle: quick on roads and firm open ground, always
available after its introduction, and visually integrated with the existing backpack. Mounts should later
remain faster and/or offer terrain access, cargo, abilities, personality and relationship progression.

The first deliverable should be a polished traversal vertical slice, not a trick system:

- toggle from on-foot to board with an authored take-out/unfold/mount sequence;
- ride with convincing acceleration, steering, braking, lean and terrain contact;
- request stow, brake to a safe speed, step off, catch/fold and return it to the backpack;
- handle slopes, curbs, walls, water and interrupted transitions without duplication or teleport pops;
- use the same character, terrain and collision truths already in Copaimo.

Do **not** add jumping, grinding, combat-on-board, boost meters or a full physics vehicle before this
basic loop is visually and mechanically proven.

## 2. Visual target supplied

- `HOVERBOARD_CONCEPT_SHEET_2026-09-01.png` — consistent top/side/front/stowed/unfolding views, scale,
  palette and character relationship.
- `HOVERBOARD_DEPLOY_STOW_STORYBOARD_2026-09-01.png` — eight readable key poses for the complete
  take-out, unfold, mount, ride and catch/stow sequence.

These are design references, not texture or mesh source. Preserve the current Warden's proportions,
silhouette and in-game rig. The board target is approximately **1.25 m long × 0.34 m wide × 0.10 m
thick**, with a three-section Z/accordion fold into a backpack-mounted module.

Art direction:

- olive/off-white deck, charcoal chassis, burnt-orange mechanical accents and restrained cyan hover light;
- broad intentional planes, visible bevels and clean silhouette, matching Copaimo's semi-cel treatment;
- ink on the outer silhouette and meaningful hinge/panel separations, not black bands painted around every
  bevel;
- a dark contact shadow and faint dust/grass response are essential to make the hovering object feel
  grounded;
- emissive glow should be a low, readable value—not a white bloom slab that erases the underside design.

## 3. Fit to the actual Copaimo project

This proposal is based on the current repository, not a generic controller:

- the game is Rust + Bevy 0.16;
- the current Warden asset is `assets/models/person_ranger.glb`, built through
  `dev/art/build_character.py` from the existing character sources;
- the character already has a backpack and named idle/walk/jog clips;
- `src/player.rs` owns camera-relative **kinematic** movement and uses analytical world queries rather
  than a rigid-body character controller;
- jog is currently 5.35 m/s, terrain height is resolved through `stands_on`, and obstacles are protected
  through `may_step` and the existing town/tree/building checks;
- `src/motion.rs` owns a custom named-clip animation graph, distance-driven phase and blend weights;
- `src/ik.rs` already contains measured two-bone leg chains and the foundation for foot planting.

Therefore, implement the board as a second kinematic locomotion mode sharing the existing world queries.
Do not introduce an independent rigid-body truth for the same player. A physics board would immediately
disagree with the roads, collision and surface heights the on-foot controller uses and would make smooth
mount/dismount handoffs much harder.

## 4. Required runtime state machine

Use one authoritative state machine. Suggested shape:

```text
Stowed
  -> Deploying
  -> Riding
  -> Stowing
  -> Stowed

Any transition state -> Recovering -> Stowed (load failure, interruption or invalid surface)
```

The state, animation, board visibility and control enablement must not be separate booleans that can drift.
At minimum track:

- current phase;
- phase time or normalized animation time;
- requested toggle/buffered intent;
- current speed and steering state;
- exactly one visible board representation;
- why deployment or stowing is blocked.

An action-level `RideToggle` is preferable. `B` appears available during normal exploration and is a
reasonable provisional keyboard binding, but avoid making the feature permanently depend on a raw key;
Copaimo will eventually need remapping and gamepad support.

### Eligibility

Deployment should require all of the following:

- player grounded on a valid rideable surface;
- not in water/wading, an interior, map/editor/fly-camera mode, a cutscene or another exclusive action;
- enough overhead/rear/side clearance for the deploy pose and board;
- slope and local surface samples within the board's ride limits.

If the toggle is pressed at an unsafe moment, buffer it briefly or display a quiet blocked response. Do
not silently enter half of the state machine.

### Suggested event timing

Author a 20-frame deploy at 24 fps (about 0.83 s) as the initial target:

| Normalized time | Event / visible contract |
|---:|---|
| 0.00 | request accepted; on-foot gait suppressed; movement damped |
| 0.12 | right hand reaches the backpack board anchor |
| 0.28 | stowed representation hides; hand representation appears |
| 0.42 | board leaves the hand; world representation appears and unfolds |
| 0.50 | hover lift and cyan emission ramp begin |
| 0.62 | lead foot contacts the deck |
| 0.78 | both feet are locked; ride loop begins; steering becomes authoritative |

Author stowing as its own 20–24 frame clip if possible. Reversing deploy is acceptable for a throwaway
prototype, but a convincing dismount has different body mechanics: the rider brakes, steps clear, catches
the moving object, lets it fold toward the hand and then rotates it into its cradle.

For a ride-to-stow request above roughly 1.5 m/s, remain in Riding, apply controlled braking, and enter
Stowing only when the board can be caught cleanly. Never snap from full speed into the catch pose.

## 5. Prop ownership without transform pops

The robust game implementation is three mutually exclusive visual instances referencing the same board
art, not one entity repeatedly reparented mid-frame:

1. `BoardStowed` — attached to the backpack anchor;
2. `BoardHand` — attached to the right-hand anchor for the pull/catch interval;
3. `BoardWorld` — placed under the feet and driven by board movement/ground alignment.

At each animation event, hide one representation and show the next in the same update. Assert that the
visible count is exactly one while the board is present. This avoids ECS hierarchy changes, one-frame
global-transform lag and inverse-transform mistakes, while remaining visually indistinguishable from one
physical prop.

In Blender, it is still useful to preview the choreography with keyed **Child Of** constraint influence:
Blender explicitly supports animating a Child Of influence to switch a prop between parent targets. Use
Set Inverse before animating the targets so the prop does not jump. This relationship is an authoring
aid—not the runtime ownership protocol. glTF exports object transforms, pose bones and shape-key values;
it does not export Blender constraint semantics as gameplay state. See the
[Blender 4.4 Child Of documentation](https://docs.blender.org/manual/id/4.4/animation/constraints/relationship/child_of.html)
and [Blender 4.4 glTF animation documentation](https://docs.blender.org/manual/en/4.4/addons/import_export/scene_gltf2.html).

## 6. Blender board construction

### Geometry and transforms

- Work in metres and validate against the final exported Warden, not a differently scaled proxy.
- Follow the current project axis convention: Blender +Y must arrive as Bevy -Z.
- Put the deployed board origin on its centre line, centred longitudinally; define hover height separately
  in gameplay rather than baking world clearance into the mesh origin.
- Apply scale before export. Keep transforms clean and give every hinge a deliberate pivot.
- Use three rigid deck pieces. Each can be a rigid object or be 100% assigned to one board bone; do not
  soft-skin structural panels across a hinge.
- A minimal board armature can use `BoardRoot`, `DeckFront`, `DeckRear` and optional left/right lift-unit
  bones. If only the fold moves, rigid objects with keyed transforms are also sufficient.
- Target roughly 2k–6k triangles for the hero board. Spend geometry on silhouette, bevels and folding
  readability, not hidden micro-detail.

### Stowed volume

The three panels should accordion into a vertical module approximately 0.46 m tall, 0.30 m wide and
0.12 m deep, centred on the backpack. Check it against:

- both arms through the whole existing jog cycle;
- hair/head clearance during turns;
- door frames and the follow camera silhouette;
- the right-hand reach and a plausible grip point;
- the full deploy sweep so the panel does not pass through the torso.

If the reference module is too large in the actual shot, preserve the deployed 1.25 m length but use
overlapping telescoping rails or a slightly denser fold. Do not simply scale the riding deck down until the
feet no longer fit.

### Materials and ink

Prefer Copaimo's existing compact material language over a new photoreal PBR asset family:

- 3–5 material roles: deck, stripe/pad, chassis/hinges, orange accents, hover emissive;
- mostly high roughness (about 0.72–0.9) and low metallic response; reserve metallic response for exposed
  hinge/lift hardware;
- actual bevel normals and separate face planes should create most form changes;
- let the global screen-space ink own the outer silhouette;
- use modeled/material-value separations for hinges and panel gaps only where the design needs an internal
  line. Avoid coplanar black decal strips that can shimmer or double the screen-space outline.

Make glow optional by quality setting and prove it in noon, dusk and heavy weather. A board that is readable
only because it blooms is not finished.

### Character and board source separation

Keep the board asset separate from the character GLB. Put new character actions through the existing
character build pipeline and export the board mesh/fold actions independently. Do not add a new deform rig
or silently replace the production skeleton. In Blender, attach the preview prop to the actual right-hand
bone and author against the delivered proportions.

The current pipeline requires the mesh to use an Armature modifier, every character vertex weighted, no
more than four influences and linear rotation interpolation before export. Preserve those rules. Blender
4.4's glTF exporter only exports actions that are active or stashed/associated appropriately, and its
slotted-action behavior changed in 4.4; verify the named actions after export instead of assuming they were
included.

## 7. Animation deliverables

Minimum authored set:

| Clip | Type | Purpose |
|---|---|---|
| `board_deploy` | one-shot, in-place | reach, pull, throw/unfold, step onto deck |
| `board_ride_idle` | looping, in-place | balanced base pose with restrained life |
| `board_stow` | one-shot, in-place | brake/step, catch/fold, backpack return |

High-value second pass:

| Clip | Use |
|---|---|
| `board_accel` | forward intent and hips settling under acceleration |
| `board_brake` | hips back, heel pressure and upper-body counterbalance |
| `board_turn_left`, `board_turn_right` | authored turn silhouette or maskable pose layers |

All clips should be in-place. The gameplay controller owns translation and facing. Do not bake the player
travelling metres through the scene into the animation root.

### Stance

- Feet approximately shoulder-width along the deck, lead foot angled 20–35°, rear foot 70–90°.
- Knees visibly soft, pelvis low enough to absorb terrain but not in a permanent deep crouch.
- Chest and head remain readable in the travel direction; arms balance asymmetrically.
- Under acceleration, hips lag slightly and torso counters forward; under braking, hips shift rearward.
- Keep hand silhouettes clear of backpack and hair.

Use the existing IK work to plant ankle targets on two authored deck sockets. The base ride clip supplies
style; IK corrects contact and board tilt. Do not ask IK to invent the entire riding pose. Limit correction
weight during the mount/dismount one-shots, then blend to full deck planting after the second foot contact.

### Procedural lean

Start conservatively:

- roll lean from lateral acceleration, clamped to roughly ±12°;
- pitch from acceleration/braking, clamped to roughly ±8°;
- board and lower body lead the lean; upper torso/head counter slightly for readability;
- smooth all values critically or with an exponential response so frame rate does not change the look.

## 8. Bevy animation integration

Do not let `src/motion.rs` continue to drive the idle/walk/jog branch while the board action branch also
writes animation weights. Riding should explicitly suppress on-foot gait and select one authoritative
board/action branch in the graph.

Recommended pattern:

- extend the named motion lookup with deploy, ride and stow clips;
- create an action/board branch with full-body weight during one-shots;
- cross-fade to `board_ride_idle` only after `FeetLocked`;
- blend optional accel/brake/turn poses in one place, not from several systems writing graph nodes;
- return to idle only after `BoardStowed` is visible and the one-shot completes.

Animation events should carry the exact prop/control handoffs. Bevy animation clips support timed events;
the official examples attach events to clips/targets before playback. Since imported glTF clips will not
automatically contain Copaimo gameplay events, attach them programmatically after the named clips load,
using the authored times as data. Guard every event by the current board phase so repeats or a blend restart
cannot duplicate the prop. See Bevy's
[animated-mesh/event examples](https://github.com/bevyengine/bevy/tree/v0.16.1/examples/animation)
and the [AnimationClip API](https://docs.rs/bevy/0.16.1/bevy/animation/struct.AnimationClip.html).

Suggested event names:

```text
TakeStowed
ReleaseFromHand
BeginHover
LeadFootContact
FeetLocked
EnableRideControl
BeginCatch
CatchInHand
ReturnStowed
```

Use normalized-time data in one configuration structure even if the Bevy event API takes seconds. That
keeps timing changes tied to clip duration and makes the contract testable.

## 9. Movement model

### Initial tuning target

These are starting values for playtesting, not design absolutes:

| Property | Initial target |
|---|---:|
| maximum forward speed | 8.5 m/s (about 1.6× current jog) |
| acceleration | 10–12 m/s² |
| braking | 14–16 m/s² |
| reverse | cap around 2.5 m/s |
| safe automatic stow entry | ≤1.5 m/s |
| visual hover gap | about 0.20 m, adjusted to art |

This is meaningfully faster than 5.35 m/s jogging while leaving clear room for mounts around 11–14+ m/s
or for mounts that win through terrain-specific abilities rather than raw speed.

Use acceleration and velocity rather than multiplying the current walk vector. Steering should become
less angular at speed and should not permit instant full-speed strafing. Preserve camera-relative intent,
but rotate the board through a speed-aware yaw response so it describes a curve.

### Surface following

Sample the rideable surface at least at front and rear deck contacts (three contacts are better on uneven
ground). Derive a smoothed board normal and pitch from those samples. Keep the player root on the same
analytical `stands_on` surface that collision uses; apply the visual hover gap and smoothed normal to the
board visual. This prevents a cosmetic hover offset from becoming a collision loophole.

The board should be road/firm-ground biased:

- reject slopes beyond a chosen ride angle;
- use a smaller step/climb tolerance than walking so curbs remain meaningful and roads feel preferable;
- slow strongly on rough/off-road surfaces if surface classification is available;
- disallow deep water and define shallow-water behavior explicitly;
- never average across two sides of a wall or curb when sampling ground.

### Collision footprint

The current player root/width check is not enough for a 1.25 m-long board. At minimum sweep/check front,
middle and rear points along the intended movement and during turns. Otherwise the player's centre may be
clear while the board nose passes through a wall. A later capsule/oriented-footprint sweep is preferable,
but a deterministic three-point contract is enough for the first slice.

On ordinary collision, decelerate or stop; do not eject the player in v1. Emergency dismount can wait until
the base loop is reliable.

## 10. Camera, sound and effects

- Pull the camera back or widen FOV subtly as speed rises, smoothed over roughly 0.3–0.5 s.
- Do not add constant camera shake. Offer reduced/disabled speed-FOV motion when accessibility settings
  arrive.
- Add a low hover hum whose pitch follows speed, a short hinge/unfold whoosh, a catch/lock sound and quiet
  surface-dependent contact texture (stone, dirt, grass).
- Use faint particulate response near the contact shadow, not a continuous smoke exhaust.
- Let the cyan underside brighten during deploy, stabilize while riding and decay only after catch.

## 11. Failure and save behavior

- If asset or animation loading fails, recover to on-foot `Stowed`; never leave movement disabled.
- If a surface becomes invalid during deployment, reverse/recover before control unlock.
- If a stow request occurs while airborne or over invalid ground, brake/hold the request and provide feedback.
- Save the unlock/equipment decision, not a half-completed animation phase. Until the save system deliberately
  supports vehicle state, loading should restore the player safely on foot with the board stowed.
- Despawning/reloading the player must remove all three board representations and all timed events.

## 12. Definition of done for the first vertical slice

### Asset and animation proof

- deployed dimensions and stowed dimensions measured against the production Warden;
- top/side/front/fold turntable plus silhouette at gameplay distance;
- exported GLBs inspected for scale, axes, origins, clip names and first/last poses;
- no missing actions from Blender 4.4 slots/NLA export;
- no torso, backpack, hair or arm intersections in deploy/stow at normal and slow speed.

### Runtime invariants

- exactly one board representation visible through every valid phase;
- each handoff event is idempotent and cannot spawn a second board;
- on-foot gait has zero authority while board actions/riding are active;
- deploy from idle and from a jog; stow request at rest and at top speed;
- board nose/rear do not pass through walls during forward motion or turning;
- slope, curb, doorway, water, bridge edge and interrupted-load cases recover safely;
- travel distance and transition timing are stable at 30, 60 and 120 Hz.

### Visual acceptance

- one uncut capture: on-foot jog → deploy → road travel → broad turn → brake → catch/stow → on-foot jog;
- slow-motion side and rear-quarter captures prove both foot contacts and all three prop handoffs;
- noon, dusk and weather stills prove deck value separation, ink hierarchy, emissive restraint and contact
  shadow;
- 720p, 1080p and 4K frames prove outlines and panel gaps do not merge into black noise;
- compare road and grass travel so the vehicle's intended surface preference is readable without UI text.

## 13. Recommended implementation order

1. Model and rig the board in its deployed and stowed states; export a static GLB and prove scale/ink.
2. Add the board state machine and three mutually exclusive visual representations with no animation.
3. Add kinematic acceleration, speed-aware steering, multi-point collision and terrain alignment.
4. Author `board_ride_idle`, plant the two feet to deck sockets and tune procedural lean.
5. Author deploy/stow one-shots and connect prop/control events.
6. Add camera, sound, restrained effects and recovery/save rules.
7. Capture the complete evidence route before adding boosts, tricks or combat.

This order isolates art, state, movement and choreography faults. It also gives Claude useful proofs at
every step rather than requiring the entire feature to exist before anything can be judged.

## 14. Source basis

- [Blender 4.4 glTF 2.0 manual](https://docs.blender.org/manual/en/4.4/addons/import_export/scene_gltf2.html) — supported animated properties, Actions/NLA export, slotted actions, sampling and reset-between-action behavior.
- [Blender 4.4 Child Of constraint](https://docs.blender.org/manual/id/4.4/animation/constraints/relationship/child_of.html) — keyed influence, parent switching and Set Inverse behavior.
- [Bevy 0.16.1 animation examples](https://github.com/bevyengine/bevy/tree/v0.16.1/examples/animation) — animation graphs, masks and clip events.
- [Bevy 0.16.1 AnimationClip API](https://docs.rs/bevy/0.16.1/bevy/animation/struct.AnimationClip.html) — attaching timed events to a clip/target.
- [Unreal Engine Animation Notifies](https://dev.epicgames.com/documentation/en-us/unreal-engine/animation-notifies-in-unreal-engine) — production precedent for synchronizing gameplay/VFX/audio events to authored animation timing. Copaimo should implement the principle with Bevy clip events, not import an Unreal architecture.

## 15. Decisions requested from Claude

Please record an explicit disposition and, if accepted, answer these before implementation expands:

1. Which provisional action/key will own `RideToggle` and where will input gating live?
2. Will the three-representation handoff be used, or is there a tested alternative that guarantees no
   hierarchy pop and exactly one visible board?
3. Which current analytical surface/collision functions will the board share, and how will the 1.25 m
   footprint be checked?
4. Will character actions remain in the existing Warden GLB pipeline while the board stays a separate GLB?
5. What proof closes the first slice before tricks/boost/combat are considered?

The feature is user-requested, but it does not need to interrupt a coherent current change. A scheduling
disposition is enough until Claude reaches an appropriate boundary.
