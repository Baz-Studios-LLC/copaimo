# Copaimo AAA-quality master audit

**Date:** 2026-08-31  
**Audience:** Claude, with enough context for the user to read directly  
**Authority:** advisory only. Codex did not change the game.  
**Scope:** current repository, current tools and evidence, not a hypothetical finished design.

## Executive verdict

Copaimo has a stronger *technical world foundation* than its current playability suggests. The
deterministic world plan, streamed terrain, authored-model contracts, rendered/traversal invariants,
photo matrix, audit mode, and real-input movement driver are unusually disciplined for this stage.
Claude is repeatedly turning visual complaints into measured contracts rather than hiding them with
local decoration. That is the right base for a high-quality game.

It is not yet meaningful to call the result AAA. That label should become a production standard:
one coherent art direction, a complete and polished player journey, target-hardware budgets,
accessibility, audio, stable release processes, and proof under movement—not simply more detail.
Today the project is an ambitious world foundation with a character controller and tools. Its declared
monster-catching/gameplay loop, player settings, gamepad path, audio, broad accessibility support, and
production content pipeline are not present yet.

The best route is therefore:

1. finish the current surface/junction/ink contracts;
2. make the evidence honest and repeatable;
3. establish target hardware and budgets;
4. produce one complete 10–15 minute vertical slice at final quality;
5. scale content only after that slice proves the pipeline.

That sequence does not diminish the AAA ambition. It prevents the project from polishing eight
kilometres of world before it knows what one excellent minute costs.

## What is already strong

- World generation is deterministic and contracts are expressed in code rather than tribal memory.
- Terrain, roads, settlements, buildings, doors, floors, collisions, lamps, seasons, weather, sky,
  bridges, vegetation, animation, foot planting, save/load, map, and a selective ink pass already
  form a real integrated foundation.
- The settlement system is not “ground only.” It currently has 19 building/yard/landmark kinds,
  district logic, street-facing doors, usable doorway gaps, interior floors and circulation checks.
- The photo matrix derives views from the generated plan instead of relying on fragile hand-entered
  coordinates. It declares lighting intent and writes a report beside its images.
- The movement driver presses the real controls and exercises real movement, grounding and collision
  at multiple update rates. This is much more valuable than teleport-based validation.
- Authored Blender outputs have metadata and geometry contracts. Recent removal of generic inverted
  hulls halved affected model geometry while preserving the intended picture through screen-space ink.
- Tools are feature-gated out of player releases, including the generation service that could spend
  credits. That is sound release hygiene.
- Recent work shows good change isolation: junction profile correction, ink correction, tree silhouette
  revision, and transparency-mask correction were committed separately and measured.

## The most important gaps, ranked

| Rank | Gap | Why it matters | Disposition now |
|---|---|---|---|
| P0 | Evidence verdict semantics | “33/33 routes” currently permits any prolonged stall to satisfy a `Blocked` route. Arrival is a radius only. The driver is useful, but its headline can claim more than it proves. | Needs review; fix the oracle before expanding the bot. |
| P1 | Close connected junctions overlap | Four measured pairs overlap; deepest overlap is 17.68 m. This can double pavement/kerbs and swallow a graph edge. | Accepted by Claude; contract the connected edge, not nearby unrelated nodes. |
| P1 | Rendered road and traversal surface differ | The accepted open terrain-drape delta is about 7 cm. Feet/collision can disagree with the visible surface. | Accepted/open; judge with foot-contact evidence and a shared-surface contract. |
| P1 | No target hardware or frame/memory budgets | “Optimized” has no falsifiable meaning without hardware, resolution, quality tier and percentile budgets. | Needs a user/Claude decision before broad optimization. |
| P1 | Temporal visual evidence is missing | Stills cannot prove stable outlines, filtered paving, shadow motion, streaming, LOD transitions or camera comfort. | Add short deterministic sweeps/traversals after the current shader pass. |
| P1 | Solid-object camera occlusion is not handled | The follow camera clears terrain height but does not query buildings, trees, canyon walls or other solids between focus and eye. Interiors and tight streets can put the camera inside geometry. | Needs review before interiors become gameplay spaces. |
| P1 | Routine CI is absent | CI runs only for tags/manual releases; format, lint, test and feature-combination regressions can accumulate between releases. | Add a lightweight push/PR workflow when current renderer work settles. |
| P2 | No distance representation strategy | The world streams to roughly 1.15 km, but there is no explicit LOD/HLOD/impostor system for trees, buildings or skyline continuity. | Design after baseline profiling; do not guess thresholds. |
| P2 | Player settings/input/accessibility architecture is absent | Controls are hard-coded keyboard/mouse; menu interaction is mouse-oriented; no gamepad/remap, scalable UI, alternate font, motion controls or quality settings exist. | Establish the settings schema during the vertical slice, before controls proliferate. |
| P2 | No audio layer | There is no music, ambience, footsteps, spatial settlement sound, UI feedback or caption plan. Audio is a major share of perceived quality and navigation. | Plan buses/events now; produce after core interactions exist. |
| P2 | Documentation has drifted from the game | `DESIGN.md` and `assets/models/README.md` still describe models, cities and the player as pending/placeholders despite extensive shipped work. | Create a short current-state snapshot; preserve old documents as history. |
| P2 | `src/world/town.rs` is a change-risk hotspot | At roughly 500 KB and ten thousand lines, topology, layout, rendering, collision, fixtures and tests share one file. | Extract by contract only after junction/material behavior is stable; no rewrite. |
| P3 | Content breadth and systemic gameplay are not present | Monsters, capture/battle/exams/missions/NPC life and a finished loop are the eventual game, not optional polish. | Deliberately defer until the foundation gate, then build one slice before world-wide content. |

## Rendering and art-direction audit

### What the renderer already establishes

- HDR camera, 4× MSAA, PBR-derived shared material extension, cel banding, weather/time lighting,
  procedural paving, and a post-tonemap ink pass provide a coherent stylized base.
- The selective ink architecture is now better aligned with the art goal: screen-space depth edges for
  generic world forms; sculpted hull lines only on the warden and authored landmarks.
- Grass and clouds opt out of the ink mask, junctions avoid black seams, and transparent water/river/
  glazing preserve authored alpha after `95dfba3`.
- Commit `2225578` moved pavement into a road-local coordinate frame and added derivative-aware joint
  filtering/fade. That is the right direction for directional continuity and anti-shimmer. Its code
  implementation is closed for that scope; moving-camera proof remains part of the temporal matrix.
- Current uncommitted `town.rs` work implements the accepted connected-edge contraction for swallowed
  close junctions. Treat it as active and review it against AQ-002 before proposing another topology fix.

### What “final-quality semi-cel-shaded” still requires

1. **Temporal line stability.** A correct still can still crawl in motion. Prove 1 px, 1.4 px and
   resolution-scaled sampling under a slow camera pan, diagonal movement, foliage motion, and the
   30/60/120 Hz presentation cases that are actually supported.
2. **Line hierarchy, not universal edge detection.** Preserve character/landmark silhouettes and large
   object boundaries. Suppress grass blades, cloud lobes, coplanar cobbles, gentle terrain folds,
   glazing interiors and ordinary kerb texture. A material/vertex eligibility mask plus relative depth
   and meaningful normal-angle thresholds is preferable to raising a global edge threshold.
3. **Material taxonomy.** Maintain a small art-directed library—earth, packed road, dressed stone,
   concrete, metal, timber, glass, foliage, water—each with controlled value range, roughness range,
   macro variation, wear logic and real-world pattern scale. Random hue/noise is not material richness.
4. **Distance behavior.** Every procedural pattern needs an explicit fade/filter response as its feature
   size approaches a pixel. The current `fwidth` direction is sound. The same review belongs to grass,
   facade repetition, thin window mullions, fences, branch tips and ink.
5. **Lighting matrix.** Noon and full night are not sufficient. Dawn, low sun, overcast, rain-wet value
   compression, snow, interiors looking out, and exterior-to-interior adaptation need controlled proof.
6. **Contact and footing.** Roads, buildings, props and trees should meet the terrain through readable
   bases, skirts, foundations, verge disturbance, dirt/debris and contact shadows—not appear lightly
   placed on a green sheet. Geometry/traversal agreement comes before decorative concealment.
7. **Quality scalability.** Shadow distance/cascades, MSAA, foliage density, fall particles, outline
   quality, view distance and distant proxies need named tiers. “Low” must preserve the composition and
   silhouettes rather than merely remove random features.

### LOD/HLOD direction

Epic’s HLOD guidance is useful as a principle rather than an engine prescription: distant unloaded
content can be represented by instanced, merged or simplified proxies to preserve mountains, trees,
cliffs and skylines while reducing draw calls. Unity likewise selects LOD by screen-space size and
supports fade transitions; identical trees and props are classic instancing candidates.

For Copaimo, begin with three measured families:

- vegetation: authored near mesh → simplified silhouette mesh → cluster/impostor;
- settlements: full building → facade/roof proxy → district skyline mass;
- terrain features: current chunks → lower-detail outer ring or authored horizon proxy.

Choose transitions from projected size and profiler data. Cross-fade/dither only if the stylized shader
and ink pass remain stable; an outlined dissolve can look worse than a deliberate hard silhouette swap.

## World, roads and settlements

The earlier road and building research remains valid and should be treated as the design library, not
repeated here. The current highest-value world criteria are:

- a country road becomes an approach, threshold, gate street and town street over distance—width,
  drainage, verge disturbance, material, kerb, furniture and building setback change in a coordinated
  sequence;
- graph nodes own their intersection surface once; connected close nodes are contracted deliberately;
- road-local material axes persist through ribbons and junctions without sudden 90-degree pattern turns;
- country paths connect meaningful origins/destinations and acquire desire-line wear, not arbitrary
  splines or evenly scattered props;
- settlements communicate district and function from silhouette, frontage, back-of-house treatment,
  open-space program and maintenance level—not just different colors;
- repeated buildings gain controlled variants in roofline, frontage rhythm, corner response, rear/service
  elevation, signs and attached volumes, while retaining a small coherent kit;
- each settlement needs a readable arrival, landmark, decision node, social/functional core, quiet edge
  and visible route back out;
- biome transitions need ecotones: density and species mix change over distance, with understory,
  deadfall, exposed soil and grove-edge structure budgeted as layers.

The present 19-kind settlement vocabulary is a real base. The next quality step is not a hundred unique
buildings. It is enough modular variation and lived-in program that repetition is perceived as a shared
building tradition rather than duplicated assets.

## Interiors

Current buildings already have walkable doors/floors and circulation guards. That changes the audit:
the problem is no longer “make interiors exist,” but “make a small number worth entering.”

For the vertical slice, choose only the ranch house, one ordinary dwelling/shop, and the guild hall.
Give each:

- a room program and gameplay purpose;
- a clear entry sequence with camera-safe dimensions;
- a primary circulation route kept free before furniture is placed;
- a light hierarchy (exterior threshold, task light, ambient fill) that works day and night;
- material transitions and wear based on use;
- acoustic identity and occlusion plan;
- interaction points and readable affordances;
- exterior/interior state and save rules;
- a camera solution for walls/ceilings/occluders;
- performance limits for lights, shadow casters, transparent glazing and prop count.

Populate from use outward: counter/work surface/hearth/bed/storage first, decorative filler last. A sparse
room with a legible purpose reads more finished than a crowded room whose path, camera and interaction
language have not been solved.

## Player, camera, UI and accessibility

The player foundation includes actual animation, movement, collision, foot placement and save position.
The missing production layer is control abstraction and configurability.

Create action-level input (`Move`, `Look`, `Walk`, `Interact`, `Map`, `Pause`) rather than spreading raw
keys. That enables keyboard, gamepad, remapping and automation to share one contract. The automation may
still inject action state or real key/button events; it should not own special movement logic.

The settings data model should eventually cover:

- mouse and stick sensitivity, independent X/Y invert, dead zones;
- remapping and conflict handling;
- hold/toggle options for sustained actions;
- FOV, speed-driven FOV, camera smoothing, shake/bob and motion-effect intensity;
- UI scale, readable sans-serif option, contrast mode and text-background opacity;
- master/music/effects/dialogue/ambience levels and dynamic range;
- window mode, resolution, frame cap/VSync and quality preset/custom controls.

Microsoft’s current accessibility guidance recommends at least 4.5:1 contrast for standard important
text/visuals, 3:1 for large elements, PC text around 18 px at 1080p by default, scalable text/icons, and
at least one sans-serif option. It also emphasizes action remapping, multiple input mechanisms, toggle
alternatives for holds, and configurable FOV/camera motion. These are useful design targets, not a legal
compliance claim.

## Audio

No audio implementation is visible in the current source. Plan a small event/bus contract before content:

- buses: master, music, ambience, effects, dialogue/creatures, UI;
- surface-aware footsteps using the same surface query movement trusts;
- settlement, forest, coast, canyon, weather and interior ambient zones;
- spatial emitters for lamps/fires/workplaces/water/wildlife;
- clear UI and interaction feedback;
- music states driven by place, threat and progression rather than a looping playlist;
- captions or visual equivalents for information-bearing sound;
- voice limiting, distance culling and streaming budgets.

Audio should enter the first complete slice early enough to shape pacing. It should not become hundreds
of clips attached after the world is otherwise “finished.”

## Performance and production engineering

The existing `CODE_OPTIMIZATION_AUDIT_2026-08-29.md` contains the code-level candidates and should remain
the detailed source. The master-level requirements are:

1. Name target hardware, resolution and presentation target.
2. Profile optimized player builds on that hardware; do not optimize editor/debug behavior as a proxy.
3. Track frame time distributions, not just average FPS: CPU main/render, GPU, p50/p95/p99, worst hitches,
   RAM, VRAM, startup, save time, stream-in time, draw calls/triangles and package size.
4. Use representative routes: ranch start, rural traversal, city entrance/node, interior transition,
   canyon, bridge, dense grove, night lamps, rain and snow.
5. Give background task integration and entity spawning a per-frame budget.
6. Capture traces before/after a change. Bevy’s official profiling guide supports built-in spans and
   Tracy in release builds; custom render work needs its own GPU timing or vendor capture.

The current asset folder is roughly 128 MB, dominated by two ~45 MB world layers and two large ranger
models. This is not automatically excessive, but the release packages the entire asset tree. Add an
explicit shipping manifest and duplication check before content scale makes accidental packaging costly.

The release workflow builds `--no-default-features` but runs tests with default features, and it runs
only for tags/manual dispatch. A future ordinary CI workflow should at minimum check formatting, lints,
tests, default/tools compilation and `--no-default-features` player compilation. The tag workflow should
also test the same feature set it ships.

## Documentation and collaboration health

`DESIGN.md`, `HANDOFF.md` and `assets/models/README.md` contain historical statements that no longer
describe the repository. That makes both agents spend time disproving old facts and risks resurrecting
closed work.

Recommended structure, for Claude to implement only when convenient:

- one short, dated `PROJECT_STATE.md` generated or manually refreshed from current facts;
- design documents explicitly labelled “intent” versus “current implementation”;
- historical handoffs kept as an archive, not the active queue;
- the ledger in `AAA_ROADMAP_AND_SUGGESTION_LEDGER_2026-08-31.md` as the current cross-agent queue;
- each material suggestion gets one disposition: accepted, adapted, deferred, rejected, needs review,
  or closed, with a reason and next proof;
- one change/invariant per commit when practical, with before/after evidence attached by name.

Claude is not expected to implement every suggestion. A reasoned deferral is successful collaboration;
silence is not, because neither the user nor Codex can tell whether the item was considered.

## Research sources

- [Microsoft Xbox Accessibility Guidelines](https://learn.microsoft.com/en-us/xbox/accessibility/guidelines)
- [XAG 101: Text display](https://learn.microsoft.com/en-us/gaming/accessibility/xbox-accessibility-guidelines/101)
- [XAG 102: Contrast](https://learn.microsoft.com/en-us/gaming/accessibility/xbox-accessibility-guidelines/102)
- [XAG 107: Input](https://learn.microsoft.com/en-us/gaming/accessibility/xbox-accessibility-guidelines/107)
- [XAG 117: Visual distractions and motion settings](https://learn.microsoft.com/en-us/gaming/accessibility/xbox-accessibility-guidelines/117)
- [Epic: Introduction to performance profiling and configuration](https://dev.epicgames.com/documentation/en-us/unreal-engine/introduction-to-performance-profiling-and-configuration-in-unreal-engine)
- [Epic: World Partition HLOD](https://dev.epicgames.com/documentation/en-us/unreal-engine/world-partition---hierarchical-level-of-detail-in-unreal-engine)
- [Unity: LOD Group](https://docs.unity3d.com/2022.3/Documentation/Manual/class-LODGroup.html)
- [Unity: GPU instancing](https://docs.unity3d.com/2023.2/Documentation/Manual/GPUInstancing.html)
- [Bevy: Profiling](https://github.com/bevyengine/bevy/blob/main/docs/profiling.md)

Engine documentation above supplies production principles, not code to transplant. Copaimo is pinned to
Bevy 0.16; any API-level implementation must be checked against that version rather than current `main`.
