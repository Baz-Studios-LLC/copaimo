# Copaimo evidence, test and performance matrix

**Date:** 2026-08-31  
**Purpose:** turn “looks good,” “works,” and “runs well” into repeatable claims Claude can prove.  
**Authority:** advisory. All implementation remains with Claude.

## 1. Evidence rules

Every important visual or movement claim should have four parts:

1. **Scene:** a deterministic world seed and a position derived from the generated plan.
2. **State:** camera, resolution, time, weather, quality tier and build identity.
3. **Action:** still, camera sweep, traversal route, weather transition or stream route.
4. **Oracle:** what constitutes pass/fail, stated before viewing the result.

A screenshot is evidence for composition and a single rendered state. It is not evidence for temporal
stability, traversal, collision, streaming, LOD transitions or performance. A test assertion is evidence
for its exact predicate, not for the broader name placed over the test suite.

Each evidence run should write a manifest beside its outputs:

```text
commit / dirty diff marker
world seed and world-data hashes
build profile and feature set
OS, CPU, GPU, RAM and driver
window/render resolution and scale
quality preset and individual overrides
camera pose or derivation, FOV and motion settings
clock, weather and season
route/sweep identifier and update/presentation rates
pass/fail plus measured values
```

Dirty work is acceptable during development, but the manifest must say it is dirty. Otherwise two images
that look different cannot be traced to a source state.

## 2. Three quality gates

### Gate F — foundation coherent

This is the present gate. It does not require monsters or a finished loop.

- roads, junctions, footways and transitions have single ownership and stable materials;
- visible surface, traversal height and collision agree within a chosen contract;
- screen-space ink is selective, resolution-aware and stable in motion;
- transparent water, river and glazing remain transparent;
- settlement entrances, streets, doors and core interiors are traversable;
- the evidence driver reports honest semantic outcomes;
- a baseline performance capture exists on named hardware;
- no P0/P1 foundation finding is silent—accepted, adapted, deferred, rejected, needs review or closed.

### Gate V — one final-quality vertical slice

A 10–15 minute route from ranch through a settlement purpose and back demonstrates:

- final movement/camera/input feel on keyboard and gamepad;
- one meaningful creature/gameplay interaction;
- ranch, country route, settlement approach, town core and 2–3 interiors at target art quality;
- audio, UI, save/reload and basic accessibility settings;
- performance on target and minimum hardware;
- a complete start, objective, feedback, consequence and return.

### Gate P — scalable production

- content can be added through documented kits and data contracts rather than large bespoke code edits;
- LOD/HLOD/streaming and quality tiers meet budgets across representative scenes;
- CI, asset validation, save migration and regression evidence run routinely;
- localization/accessibility/audio pipelines are established;
- multiple vertical-slice-equivalent regions can be produced without degrading consistency or cost.

## 3. Core still-image matrix

The existing matrix already derives ranch, village, city, canyon, night and bridge views from the world
plan. Preserve those names when possible. Add the following only as they become relevant; do not turn
every ordinary run into a hundred images.

Notation:

- `S` = selected settlement centre.
- `R` = settlement radius.
- `O` = normalized outward approach direction returned by the plan.
- Camera “from” follows the current photo convention: eye is offset by `from * back` and looks at `at`.
- Player-eye evidence uses a 1.7–2.4 m height; plan evidence uses the existing higher oblique.

| Name | Target `at` | Camera from / height / back | State | What it proves | Pass criteria |
|---|---|---|---|---|---|
| `village_approach_far` | `S + O * 2.4R` | `O / 5 m / 40 m` | noon, clear | Country route and settlement silhouette begin as one composition. | Route points to a legible arrival; no isolated model scatter or abrupt paving. |
| `village_threshold` | `S + O * 1.02R` | `O / 1.7 m / 14 m` | noon, clear | Dirt/packed surface, drainage, verge, kerb and gateway meet at eye level. | No hard rectangular material cut; width/edge/grade changes read as staged and traversable. |
| `village_inside_gate` | `S + O * 0.85R` | `-O / 1.7 m / 14 m` | noon, clear | Looking back across the same transition catches one-sided fixes. | Same transition reads from inside; kerbs do not double or dead-end. |
| `city_approach_far` | `S + O * 2.4R` | `O / 6 m / 46 m` | noon, clear | Skyline hierarchy and urbanization sequence. | Spire/core visible without every building becoming equal visual noise. |
| `city_threshold` | `S + O * 1.02R` | `O / 1.7 m / 14 m` | noon, clear | The specific dirt-to-city-road problem. | Material, road width, footway and street furniture transition over distance; no absent or instant sidewalk. |
| `city_transition_graze` | `S + O * 1.0R` | normalized `O + O.perp()*0.55` / `1.7 m` / `18 m` | noon, clear | Grazing view exposes z-fighting, seams, UV rotation and anti-aliasing. | No bright/black seam, crawling border, doubled surface or sudden stone rotation. |
| `city_node_eye` | existing city node/hall derivation | hall-facing / `1.7 m` / `16 m` | noon, clear | Junction surface, kerb returns, doors and furniture as the player sees them. | One owned surface, continuous crossfall, clear path, no furniture in carriageway. |
| `city_node_oblique` | same node | existing `10 m / 46 m` | noon, clear | Topology and district composition. | Each arm connects once; no overlapping islands, doubled kerb or swallowed edge. |
| `mixed_gateway_node` | first node whose arms differ in `Arriving` state | align with unpaved arm / `2.2 m` / `18 m` | noon, clear | Mixed paved/unpaved ownership at a junction. | Transition follows arm state without radial rings or material leakage into the wrong mouth. |
| `road_surface_close` | straight paved segment inside city | 30° off axis / `1.7 m` / `8 m` | noon, clear | Stone scale, direction, filtering and roughness. | Pattern follows road direction, keeps believable scale, no subpixel sparkle or texture swimming. |
| `road_surface_distance` | same segment | along road / `3 m` / `70 m` | noon, clear | Procedural pattern fade with distance. | Detail simplifies smoothly before becoming noise; road identity remains. |
| `shoreline_alpha` | nearest accessible shore with oblique reflection | toward water / `1.7 m` / `15 m` | noon and dusk | Water alpha regression after ink mask. | Water remains translucent by authored amount; shoreline/underwater read remains intentional. |
| `window_alpha_day` | city/guild glazing from exterior | square to facade / `1.7 m` / `9 m` | noon | Glazing alpha and trim clarity. | Glazing is not opaque; ink does not fill the pane or swallow trim. |
| `window_alpha_night` | same facade | same camera | night, lamps on | Interior glow through glazing. | Warm interior reads through glass without a flat emissive rectangle or wall light leak. |
| `interior_threshold_out` | just inside chosen door | toward exterior / `1.7 m` / `6 m` | noon | Exposure, doorway clearance and camera behavior. | Exterior is readable, walls do not occlude the camera, threshold has no traversal lip. |
| `interior_threshold_in` | just outside same door | toward interior / `1.7 m` / `6 m` | night | Interior purpose, circulation and light hierarchy. | Primary route is obvious; room does not become a black box or flat amber wash. |
| `trees_close_sheet` | authored five-tree comparison site | orthographic plus player view | noon | Near silhouette, branch visibility and facet scale. | Each species reads by silhouette; leaf facets do not become distracting close-up plates. |
| `grove_edge` | first dense grove edge on representative route | toward grove / `1.7 m` / `25 m` | noon and overcast | Ecotone, density layers, contact and species rhythm. | Edge has understory/ground transition and uneven canopy; no lollipop grid or floating trunks. |
| `canyon_inside` | existing `way_through(0)` | existing / `2.4 m / 12 m` | noon | Wall geometry and path from player scale. | No edge comb, camera burial, ink chatter or false traversable opening. |
| `bridge_middle` | existing longest-span midpoint | existing | noon and storm | Repetition, horizon, weather and long-view stability. | Span has readable rhythm without copy-paste strobing; distant world does not visibly end. |
| `snow_settlement` | city/village street | street eye view | snow, chosen hour | Surface legibility under accumulation. | Road/footway/path remain distinguishable without relying only on hue. |

The exact settlement index and node should be resolved by semantic search (`first village`, `first city`,
`first mixed Arriving node`) and written to the manifest. Hard-coding current coordinates would make the
matrix silently point at the wrong place after generation changes.

## 4. Temporal evidence matrix

Still images are necessary but insufficient for the active ink and paving work. Add a small deterministic
capture mode or drive these manually until automation is worth its cost.

| Clip | Duration/action | Variants | Oracle |
|---|---|---|---|
| `ink_slow_pan_city` | 12 s constant 20° horizontal pan across roofline, windows, kerbs and foliage | 1080p/1440p; 30/60/120 presentation FPS | Silhouette weight appears constant; no one-frame gaps, crawling, MSAA stipple or grass/cloud ink. |
| `ink_dolly` | 15 s walk from 40 m to 5 m from a facade | same resolutions | Line grows/fades continuously; trim is not swallowed; no obvious sampling-radius step. |
| `paving_forward` | 20 s jog down city street | noon and low sun | Stones remain road-relative; distant pattern filters smoothly; no sparkle/moire/swimming. |
| `junction_turn` | traverse straight then turn through a 3+ arm node | 30/60/120/240 simulation Hz | Surface direction changes coherently; character does not snap vertically; no seam enters view. |
| `country_to_city` | full approach from unpaved country road to interior city street | walk and jog | Width, material, edge and furniture layers phase in as one arrival; no pop or instant state change. |
| `stream_grove_city` | high-speed maker route through grove into city, then player-speed route | target quality tiers | No large main-thread hitch, long empty patch, visible cell ring or delayed collision. |
| `camera_wall_interior` | orbit near exterior wall, enter and rotate inside | min/max camera distance | Camera never enters solid geometry; correction is smooth and does not oscillate. |
| `weather_transition` | clear→rain and clear→snow through representative street | target tier | Particle/entity count and lighting transition stay stable; no sudden material discontinuity. |
| `day_cycle_keyframes` | short held captures at dawn, noon, dusk, night | city node + interior | Exposure, silhouettes and navigation cues remain readable; lamps transition without popping/leaking. |

Presentation frame rate and simulation update rate are different variables. The existing `--drive`
30/60/120/240 Hz cases prove update-rate sensitivity; a video capture must also record the delivered frame
pacing. Do not label one as evidence for the other.

## 5. Movement-driver oracle correction

The current driver is an excellent actuator with an incomplete oracle:

- `Arrives` passes when the player centre is within 1.2 m of a point. A wall or kerb can separate that
  circle from the intended region.
- `Blocked` passes after 0.75 s without progress, wherever that occurs. A snag at the start, a tree, a
  wrong turn or the intended canyon wall all produce the same successful verdict.
- timing out also passes any `Blocked` route without proving which barrier stopped it.

Recommended contract:

### Arrives

An arrival route passes only if all required predicates hold:

- player centre enters a finish **region** (disc/rectangle/polygon meaningful to the test);
- player is on the intended side of any separating boundary;
- traversal height/surface kind matches the destination;
- route remained within its corridor;
- no prohibited collision penetration or excessive vertical snap occurred.

For a doorway, “inside the room beyond the threshold” is stronger than “near the door.” For a kerb,
“standing on the footway band” is stronger than “near the target coordinate.”

### Blocked

A blocked route passes only if:

- it first enters an authored **contact band** in front of the intended barrier;
- forward progress then remains below a threshold for the dwell time;
- it does not cross the barrier’s forbidden half-plane/volume;
- it does not leave the route corridor or stop on a different obstacle;
- the final point is within the intended along-barrier span and height band.

Otherwise the verdict is `Wrong blocker`, `Strayed`, `Timed out`, or `Inconclusive`, not pass.

This keeps the bot small. It does not require pathfinding or visual AI; each authored route declares the
semantic region it exists to prove.

## 6. Automated test layers

### Layer A — fast invariants on every change

- deterministic generation for fixed seeds;
- finite values, valid indices and non-degenerate mesh triangles;
- graph planarization and connected-edge ownership;
- exhaustive building/material/species registries;
- visible doorway equals collision doorway;
- no prop/furniture in reserved circulation, carriageway or door apron;
- alpha/ink material contract;
- save parsing, future-version refusal and atomic-write behavior;
- shader compilation and asset metadata presence where feasible.

### Layer B — representative procedural suites

- multiple fixed “problem seeds,” plus a rotating deterministic seed set;
- settlement reachability from approach → gateway → node → chosen door;
- close connected nodes contracted; close unconnected nodes reported, not silently merged;
- mixed paved/unpaved junction fixture;
- road ribbon and node share elevation/material-coordinate rules;
- triangle-interpolated rendered height compared with traversal height at interior samples, boundaries,
  curb returns and steep grades;
- building fronts address a reachable street and service backs do not invade reserved public space;
- LOD bounds/silhouette/material metadata remain valid after asset generation.

### Layer C — real-input integration

- kerbs, doorstep, slope, junction turn, bridge and canyon at walk/jog and multiple update rates;
- interior entry/exit and camera correction;
- save, exit, continue and position/surface restoration;
- map/pause/menu navigation by keyboard and gamepad once supported;
- country-to-settlement traversal with stream activity;
- negative routes with semantic blocker bands.

### Layer D — visual and performance regression

- still matrix and temporal clips;
- pixel metrics only for narrow invariants (alpha coverage, line coverage, missing/solid regions), not as
  a substitute for art review;
- release-profile CPU/GPU traces on representative routes;
- memory, startup, package and streaming reports;
- human review of composition, hierarchy, material believability and comfort.

## 7. Provisional performance contract

Claude and the user must name the actual target hardware before numbers become gates. Until then these are
planning envelopes, not claims:

| Target | Frame budget | Suggested internal headroom |
|---|---:|---:|
| 60 FPS presentation | 16.67 ms | aim p95 CPU frame ≤ 10–12 ms and GPU ≤ 13–14 ms in representative play so spikes/features have room |
| 30 FPS minimum tier | 33.33 ms | aim p95 CPU and GPU ≤ 26–28 ms, with quality reductions chosen deliberately |
| 120 FPS optional tier | 8.33 ms | treat as a separate high-end target, not an automatic promise |

Track at least:

- delivered frame-time p50/p95/p99 and worst 1-second window;
- main-thread, render-thread and GPU distributions;
- frames over 16.67/33.33/50 ms;
- stream task completion and main-thread integration time;
- draw calls, visible instances, triangles and transparent passes;
- total RAM, GPU memory where available and allocation spikes;
- cold start to interactive menu, new-game start to controllable player, save duration;
- package size and assets included but never loaded;
- capture conditions and whether profiler overhead was present.

Do not average away hitches. A steady 10 ms scene with one 120 ms cell integration every few seconds is
not a 10 ms experience.

## 8. Scene performance routes

| Route | Stresses |
|---|---|
| Ranch cold start | asset startup, first stream, character/animation, save state |
| Dense grove traversal | vegetation instances, shadows, alpha/foliage shading, streaming |
| City approach → node | settlement spawn, many materials/meshes, lights, ink and road shader |
| Night city orbit | lamps/windows, transparency, postprocess, shadow policy |
| Rain city traversal | precipitation churn, lighting, transparency/overdraw |
| Snow city traversal | particles/accumulation logic and surface readability |
| Canyon pass | terrain density, camera/ink on close walls, collision |
| Longest bridge | long sightline, repeated instances, water and horizon |
| Exterior → guild interior | camera occlusion, lights, props, asset load and acoustics later |
| Maker-speed world crossing | worst-case streaming throughput; diagnostic only, not player target |

Measure first, then use the candidates in `CODE_OPTIMIZATION_AUDIT_2026-08-29.md`. A code pattern that
looks expensive is a hypothesis until the trace assigns it meaningful frame time or memory.

## 9. CI/release evidence matrix

Recommended future checks, without changing the current release process until Claude chooses:

| Trigger | Checks |
|---|---|
| Pull request / ordinary push | formatting, Clippy, fast tests, default/tools compile, player `--no-default-features` compile |
| Nightly/manual foundation | procedural seed suite, semantic drive routes, shot manifest generation, asset contracts |
| Release candidate | optimized player build/tests with the exact shipped feature set, target-hardware performance routes, still/temporal matrix, package manifest |
| Tagged release | platform builds, exact-feature tests, tools-not-present proof, package/install smoke test, save compatibility check |

The current tag workflow builds the player without default features but runs `cargo test --release` with
defaults. Add a matching no-default-feature test/compile path when the workflow is next revised so the
tested product and shipped product do not silently diverge.

## 10. Review template

Claude can use this compact response for any ledger item:

```text
ID / disposition:
Evidence read:
Decision and reason:
Change/commit (if any):
Proof produced:
Remaining limitation or next gate:
```

“Deferred because the foundation phase excludes it” is a complete answer. “353 tests pass” is only a
complete answer when the item is actually covered by a named predicate in those tests.

