# Copaimo Static Code Optimization Audit

**Audience:** Claude and future Copaimo contributors  
**Date:** August 29, 2026  
**Mode:** read-only static audit; no code, assets, configuration, or game files changed  
**Engine reviewed:** Rust, Bevy 0.16.1  

---

## Executive summary

Copaimo is not an unoptimized project. The code contains many deliberate, evidence-based performance decisions already: asynchronous terrain generation, chunk streaming, shared mesh/material handles, welded grass and props, GPU sea displacement, one-mesh stars, shadow distance gates, task caps, coarse spatial rebuild gates, dirty checks before material writes, release LTO, and removal of maker tools from player builds.

The strongest remaining opportunities are concentrated rather than systemic.

### Highest-confidence findings

1. **Disabled rivers still pay for a full river sampling pass on every generated terrain chunk.** `RIVERS` is false, but `build_chunk` still calls `build_river`. Across the normal 253-chunk view disc, that is about 1,068,925 `river_surface` calls for a feature guaranteed to produce no mesh. Each call begins with `drawn_height`, which samples four terrain heights. This can add roughly 4.28 million expensive height evaluations during initial streaming, in addition to the roughly 1.14 million needed by the actual terrain grid.

2. **The cloud-shadow shader is a likely major GPU hotspot.** Every visible fragment using `Shaded` can loop over 30 cloud discs, performing wrapping, distance, and smoothstep math. This applies to terrain, vegetation, buildings, the player, and most solid world surfaces. The loop is skipped uniformly when cloud strength is zero, but in ordinary daylight it multiplies with screen resolution, overdraw, and MSAA.

3. **Precipitation performs unnecessary work in clear weather and scales poorly when active.** Eight hundred separate entities are retained. Clear weather writes `Visibility::Hidden` to every one each frame. Rain/snow updates hundreds of transforms and visibility components individually each frame.

4. **Lighting systems repeatedly rediscover static state.** `stand_the_lamps` scans lamp entities to learn whether each settlement already has lamps; `light_the_windows` performs a linear scan of all awake panes for each tower; active-light selection scans lamps, allocates vectors, sorts, and performs repeated linear membership searches each frame. The visual design is sound; bookkeeping needs persistent state and spatial/time gating.

5. **Background generation can still hitch on the main thread when work completes.** Mesh computation is asynchronous, but all ready results are consumed in one frame. Completion attaches assets, uploads meshes, despawns/replaces children, plants trees, and spawns large entity hierarchies. If several chunk, cover, or prop tasks finish together, `collect_*` has no completion or time budget.

6. **The visible terrain has no distance LOD.** The normal radius contains 253 chunks, about 1,068,925 terrain vertices and 2,072,576 terrain triangles before water, trees, grass, props, settlements, outlines, characters, or shadow passes. A uniform two-metre grid is excellent nearby and wasteful near the 1.15 km horizon.

### Recommended order

1. Add a reproducible release-profile performance capture matrix and establish CPU/GPU baselines.
2. Skip river construction entirely while `RIVERS` is false.
3. Stop clear-weather writes to 800 precipitation entities.
4. Profile the cloud-disc fragment loop; prototype a low-resolution cloud-shadow field if it is significant.
5. Replace lighting discovery scans with explicit state markers and spatial/time gates.
6. Budget main-thread completion work across frames.
7. Prototype terrain/tree/building LOD only after the preceding changes are measured.

Do not perform all suggestions simultaneously. Each change should be isolated, captured, and compared by median and p95/p99 frame time.

---

## 1. Audit boundaries and confidence labels

This report did not compile or run the game because the user prohibited changes outside `codex-suggestions`, and compilation would create build artifacts. It is a static review of the current source, assets, pinned Bevy source, configuration, and existing measurements/comments.

Findings use these labels:

- **Confirmed work:** the code unquestionably performs the described work.
- **Likely hotspot:** workload is large enough to prioritize for profiling, but only a capture can establish its frame-time share on target hardware.
- **Conditional:** worthwhile only if a measured budget—startup, package size, memory, CPU, or GPU—fails.

Optimization target should be a **release build without default `tools`**, matching the actual workflow. Development builds include the editor, workbench, kiln, and HUD and are not representative of player performance.

---

## 2. What is already well optimized

Preserve these decisions unless measurements contradict them.

### World streaming and generation

- Terrain chunks are generated through `AsyncComputeTaskPool` rather than synchronously in the frame.
- Cover and prop geometry are also generated in background tasks.
- Pending counts are capped (`MAX_PENDING_CHUNKS`, `MAX_PENDING_COVER`, `MAX_PENDING_PROPS`).
- `ChunkMap` prevents duplicate work and provides explicit lifetime ownership.
- Terrain uses `RenderAssetUsages::RENDER_WORLD`, allowing CPU mesh data to be dropped after upload.
- Grass and props are welded into one mesh per dressed chunk rather than thousands of entities.
- Trees share a small pool of meshes and materials rather than receiving unique assets.
- Empty cover/prop results record an answered state, preventing barren chunks from being regenerated forever.
- Cover and props have much shorter ranges than horizon terrain.
- Unload/dress systems include hysteresis.

### Rendering

- The sea is displaced in the vertex shader rather than rewriting and uploading a 26,000-vertex mesh every frame.
- Grass does not cast cascaded shadows.
- Distant trees have shadow casting disabled at chunk granularity and change only when crossing a ring.
- Clouds do not enter the engine shadow pass; custom cloud shading avoids stretching cascades to the cloud ceiling.
- Stars are combined into one mesh rather than 900 star entities.
- Clouds share one material.
- Materials are commonly shared by world category.
- Several material mutations compare the desired value before calling `get_mut`, avoiding needless GPU preparation.
- Sun transform movement is quantized to stabilize cascades.
- Point and spot lamp shadows remain disabled.

### World population and tools

- Villages/cities have explicit scene budgets rather than spawning every geometrically possible lot.
- Country roads and bridges rebuild on coarse cell changes, not every frame.
- The world map paints asynchronously and only when requested.
- Debug HUD work returns before formatting or terrain queries when hidden.
- Maker-only code is feature-gated out of player builds.

### Build profile

- Project code uses light dev optimization while dependencies use level 3.
- Release uses thin LTO and one codegen unit.
- CI builds player releases with `--no-default-features`.

These choices already follow Bevy’s official release-profile guidance.

---

## 3. P0: establish an optimization measurement contract

Do this before any architectural rewrite.

### 3.1 Fixed performance scenes

Use deterministic locations, hours, weather, and camera motion. Suggested captures:

| Scene | Stresses |
|---|---|
| Noon city walk | buildings, outlines, material count, lamps as geometry, paved ground |
| Night city walk | 20 real lights, emissive panes, transparency, bloom if present |
| Dense meadow at noon | grass overdraw, cloud fragment loop, terrain |
| Dense meadow in rain | precipitation updates/draws plus grass |
| Dense forest traversal | transform/entity count, foliage overdraw, shadow ring crossings |
| Canyon vista | far terrain, high screen coverage, shadow cascades |
| Coast panorama | transparent sea, far terrain, clouds |
| Fast free-fly traversal | task production/completion, mesh uploads, town/bridge staging |
| Menu → play → bench → play | retained world memory and state transition cost |

Capture at the same resolution, present mode, camera path, and graphics settings.

### 3.2 Record more than average FPS

For each capture record:

- CPU frame median, p95, p99, worst one-percent cluster;
- GPU frame median, p95, p99;
- main-world system spans;
- render prepare/queue/pass spans;
- loaded and pending chunks;
- completed chunks consumed per frame;
- meshes/materials/images and estimated memory;
- visible entities, draw calls/batches, triangles where available;
- active point/spot lights;
- visible transparent meshes;
- active shadow casters and cascade time;
- allocation spikes during stream crossings.

The existing `FrameTimeDiagnosticsPlugin` is useful for a HUD but cannot attribute cost.

### 3.3 Use Bevy’s profiler support

Bevy 0.16 connected render diagnostics to Tracy. Use a release capture with the appropriate Bevy tracing feature and add named spans around Copaimo’s own large units:

- `build_mesh` and `build_river` separately;
- task polling versus result attachment;
- tree planting;
- cover/prop mesh conversion and upload;
- town layout, scene spawning, and road paving;
- `light_them_at_night` and `light_the_windows`;
- collision gathering and IK;
- map painting;
- custom render passes if a cloud-shadow field is introduced.

Official Bevy profiling guide:  
https://github.com/bevyengine/bevy/blob/main/docs/profiling.md

Bevy 0.16 rendering and GPU timestamp context:  
https://bevy.org/news/bevy-0-16/

### 3.4 Define budgets before optimizing

Example target envelopes—not assumptions about current results:

- 60 Hz frame: 16.67 ms total;
- CPU main app: aim below roughly 8 ms median with headroom for spikes;
- GPU: aim below roughly 13–14 ms on minimum target hardware;
- streaming p99: no single completion frame above the agreed hitch threshold;
- memory and package targets chosen for minimum platform;
- separate 30 Hz quality tier if minimum hardware requires it.

Use the project’s real hardware matrix rather than treating these examples as requirements.

---

## 4. P1 confirmed: disabled river generation still samples every chunk

### Evidence

- `src/config.rs:235`: `pub const RIVERS: bool = false`.
- `src/world/chunk.rs:67–69`: `build_chunk` always calls both `build_mesh` and `build_river`.
- `src/world/chunk.rs:79–93`: `build_river` samples a 65×65 surface grid.
- `src/world/terrain.rs:1162`: `river_surface` starts with `drawn_height` before discovering there is no water.
- `src/world/terrain.rs:1029–1041`: `drawn_height` evaluates four height corners.
- Normal view disc: 253 chunks at `VIEW_CHUNKS = 9`.

### Cost shape

```text
253 chunks × 65 × 65 river samples = 1,068,925 river_surface calls
1,068,925 × 4 height corners ≈ 4,275,700 height evaluations
```

That work produces `None` for every chunk while rivers are disabled. The terrain mesh itself samples a padded 67×67 height grid, approximately 1,135,717 height evaluations for the same disc. Therefore the disabled river path can perform several times the terrain-height work of the visible mesh.

### Recommendation

Immediate safe shape:

- In `build_chunk`, do not call `build_river` when the compile-time/game configuration says rivers are disabled.
- Prefer making the absence structural: either `if RIVERS { build_river(...) } else { None }`, or construct a terrain capability once and query it cheaply.

When rivers return:

- Reuse the terrain height grid already built for the 64×64 chunk because river and ground resolutions currently match.
- At exact grid vertices, the rendered ground height does not need a four-corner bilinear lookup; the existing grid height is the answer.
- Refactor river-surface evaluation to accept the already-known ground height.
- Avoid allocating `Option<Vec<f32>>` for every wet quad; use a four-element stack array or direct pattern match.

### Validation

- Compare initial world-ready time and total terrain task CPU with rivers off.
- Ensure `RIVERS = true` still produces identical water edges.
- Keep the existing river geometry tests.

**Priority:** highest-value low-risk fix.

---

## 5. P1 likely GPU hotspot: thirty cloud tests per shaded fragment

### Evidence

- `assets/shaders/cloud_shade.wgsl:249–278`: `sunlight_on` loops through `weather.w` cloud discs.
- `src/config.rs:328`: 30 clouds.
- `src/shade.rs`: supports 32 disc uniforms.
- `assets/shaders/cloud_shade.wgsl:326+`: the fragment path applies this after PBR to all non-sea `Shaded` surfaces when cloud strength is active.
- `Shaded` is used across terrain, cover, props, trees, architecture, roads, rivers, sea, and character surfaces.

Each loop iteration includes world-space wrapping, division/rounding, vector length, and smoothstep. This is elegant and visually coherent, but it scales as:

```text
shaded fragments × overdraw × active cloud count
```

At 1600×900, the screen contains 1.44 million pixels before overdraw. Even one shaded layer over most pixels implies tens of millions of cloud-disc iterations. Vegetation and transparent surfaces can add substantial overdraw. Four-sample MSAA adds coverage and bandwidth costs even where the shader invocation rate is implementation-dependent.

### Profile first

Use three otherwise identical captures:

1. current 30-disc shadows;
2. cloud shadow strength forced to zero while keeping visible clouds;
3. a temporary small cloud count such as 4 or 8.

Compare GPU time in terrain, opaque, alpha/vegetation, and total render passes. Do not judge only FPS if VSync is active.

### Preferred architectural solution if significant

Build a low-resolution, world-space **cloud coverage/shadow field** around the viewer:

- Rasterize the same cloud discs into a small R8/R16 texture or compute target.
- Update it only when the sun slant/weather quantization changes or as its world origin crosses a texel/cell threshold.
- Scroll cloud motion analytically or update the field at a modest rate.
- Sample once per shaded fragment.
- Use a stable world-space mapping and a padded/repeating region so camera movement does not expose seams.
- Preserve the “look up and see the cloud casting it” contract by using the exact same cloud list as the field source.

This trades 30 expensive analytic tests per visible fragment for one texture sample plus modest field generation.

### Lower-complexity alternatives

- Reduce active shading clouds while retaining 30 visible cloud meshes.
- Merge overlapping discs into a smaller set on the CPU at the slow existing update cadence.
- Divide the world field into coarse tiles and pass only the few relevant discs per tile/draw, though material/draw complexity may erase the gain.
- Use a scrolling authored coverage texture only if the exact-cloud correspondence is no longer an art requirement.

### Do not

- Recompute and upload a full high-resolution texture every frame on the CPU.
- Add screen-space noise that swims with the camera.
- Replace the current system before an A/B GPU capture proves its cost.

---

## 6. P1 confirmed: precipitation entity churn

### Evidence

- `src/fall.rs:41`: `DROPS = 800`.
- `src/fall.rs:235–289`: `let_it_fall` queries and updates every drop.
- `src/fall.rs:256–260`: clear weather loops over all 800 entities and assigns `Visibility::Hidden` every frame.
- Active weather updates visibility, material handle, translation, rotation, and scale for each visible drop.

### Immediate safe fix

Track precipitation state in a local/resource:

- If last frame was also clear, return without querying/mutating drops.
- On transition into clear, hide the pool once.
- On rain ↔ snow transition, switch material/size only when the state actually changes.
- For the active-count boundary, show/hide only the index range that changed rather than assigning visibility to the whole pool every frame.

This preserves all visuals and removes the worst inactive-state work.

### Architectural improvement if rain remains costly

Options in increasing complexity:

1. One or a few welded meshes whose vertices are moved in the shader from per-drop seed data.
2. GPU instancing/instance data with one mesh and material.
3. A camera-local particle shader using vertex IDs/seeds and global time.
4. A dedicated GPU particle system only if future weather needs collisions, splashes, or richer state.

The current procedural formula already derives positions from seed and elapsed time, which is well suited to a shader. CPU gameplay does not appear to need individual drop entities.

### Validation

- Clear-day CPU capture should show almost no precipitation cost.
- Rain should preserve camera wrapping, wind lean, snow/rain size, and no edge-on disappearance.
- Compare draw calls, transform propagation, extract/prepare work, and overdraw.

---

## 7. Immediate review for Claude’s current lighting work

The visual strategy—emissive glass everywhere, only 20 real nearby lights, no local shadows, spotlights for streets, points for lanterns—is performance-conscious. The main remaining cost is repeated discovery.

### 7.1 `stand_the_lamps` rescans existing lamps

`src/world/lamp.rs:183–250`

For each built settlement, the system runs:

```rust
standing.iter().any(|(_, from)| from.0 == *key)
```

every frame. The file comment says there can be hundreds of fittings. This is static lifetime state being rediscovered through an entity scan.

**Suggestion:** put an explicit `LampsRaised` marker/state against the settlement key or retain lamp-root entities in a small map. Spawn when a new `Built` key appears; remove state when the settlement leaves. Prefer one settlement lamp root with every fitting as a descendant, allowing one root despawn.

### 7.2 `light_the_windows` performs tower × pane searches

`src/world/lamp.rs:390–532`, especially line 452.

For every tower, every frame:

```rust
awake.iter().any(|(_, of)| of.parent() == entity)
```

The `Awake` query counts panes, not buildings. Modern tower bands can create many panes, making this an avoidable O(towers × awake panes) scan.

**Suggestion:** mark the tower parent with `WindowsAwake { night }` when children are created. Query only towers without the current state. On day transition/new night, despawn panes and remove/update the parent marker once. This converts pane discovery to direct ECS state.

Alternative: construct a `HashSet<Entity>` of awake parents once per frame. That is better than nested scans but still rebuilds information that a component can remember.

### 7.3 Real-light selection does more work than needed

`src/world/lamp.rs:285–385`

Per frame it:

- collects active parents;
- scans every lamp and computes square-root distance;
- allocates and sorts a vector;
- repeatedly uses linear `contains`/`find` checks.

The active set changes only when the player moves meaningfully, a settlement streams, or a hysteresis boundary is crossed. Sun height changes intensity, not membership.

**Suggestion:** separate two jobs:

1. **Selection:** recompute candidate membership only after the anchor moves a chosen threshold or lamp topology changes. Use squared distance and `select_nth_unstable_by`/bounded nearest selection if candidate count becomes large.
2. **Intensity:** update only the at-most-20 active light components for day/night fade and distance falloff.

Keep the existing 62 m admission / 85 m retention hysteresis.

### 7.4 `open_the_glass` scans all glass every frame

The system already avoids assigning if visibility matches, which is good. It can still be gated on a cached day/night emissive state because the answer changes only at the threshold. A run condition/resource state transition would remove the hundreds-entry scan during the rest of day or night.

### 7.5 Keep shadows off

Do not enable dynamic shadows on these 20 local lights without a new budget and capture. The current directional cascades already consumed most of a measured frame in an earlier code comment. Use fixture geometry, spot direction, range, building occluder proxies, cookies, or light placement to manage leaks before shadowed locals.

---

## 8. P1 likely hitch source: unbudgeted main-thread task completion

### Evidence

- `src/world/stream.rs:120–194`: all ready chunk tasks can be consumed in one frame; each result adds meshes, replaces children, creates river geometry, and plants trees.
- `src/world/cover.rs:187–237`: all ready cover tasks can add meshes and replace children in one frame.
- `src/world/prop.rs:159–199`: all ready prop tasks can do the same.
- `MAX_PENDING_CHUNKS = 24`, cover = 6, props = 6.

Task computation is off-thread; result integration is not. Several tasks finishing together can cause:

- main-world allocation and command work;
- `Assets<Mesh>::add` work;
- render asset extraction/preparation and GPU buffer upload;
- entity hierarchy creation for trees;
- simultaneous cover and prop replacement.

The current comment that chunk generation is “off the frame budget entirely” is too strong. Sampling is off-frame; integration and upload remain in-frame.

### Recommendation

Introduce an integration budget:

- cap ready chunk results consumed per frame;
- independently cap cover and prop completions;
- optionally use a small time budget measured with a monotonic timer, but deterministic count caps are easier to test;
- prioritize near chunks and terrain before decoration;
- stage tree planting separately if it dominates attachment;
- avoid integrating terrain, cover, props, and a whole settlement in the same frame when possible.

Because tasks are already complete, leaving them pending for another frame is cheap and does not waste computation.

### Suggested priority order

```text
near ground mesh
→ near collision/walkable continuity
→ river if enabled
→ trees/major silhouettes
→ cover
→ props
```

### Capture

Measure p95/p99 during fast travel, not only stationary FPS. Add counters for “ready results waiting” and “results integrated this frame.”

---

## 9. P1/P2: distance LOD and visibility

### Current terrain cost

With `VIEW_CHUNKS = 9`, `CHUNK_QUADS = 64`:

- 253 chunks in the radius-nine disc;
- 4,225 visible grid vertices per chunk;
- 8,192 triangles per chunk;
- approximately 1.07 million terrain vertices;
- approximately 2.07 million terrain triangles.

This excludes chunk overlap/hysteresis and every other world object. The same two-metre spacing is used under the player and at the horizon.

### Terrain LOD recommendation

Prototype concentric mesh resolution rings:

- near: current 64×64 quads;
- middle: 32×32;
- far: 16×16 or comparable;
- preserve chunk world size so streaming bookkeeping remains stable.

Crack control options:

- skirts on lower-resolution chunk edges;
- edge vertices constrained to the coarser neighbor grid;
- limited neighbor-level difference of one;
- geomorph/crossfade if needed.

Because Copaimo uses a two-metre near grid and broad stylized terrain, far reduction should be visually forgiving. Recalculate normals/colors at each LOD rather than naïvely dropping vertices if biome/road edges depend on samples.

### Existing Bevy support

Bevy 0.16.1 includes `VisibilityRange` and a visibility-range plugin. It can support high/low mesh ranges and dithered transitions for individual entities. The exact pinned source confirms availability. It may be useful for trees, props, bridge modules, and architecture. Terrain still needs crack-aware geometry logic.

### Tree and prop LOD

- Near trees: current trunk + canopy.
- Mid trees: simplified trunk/canopy or a welded cluster per chunk.
- Far trees: cluster impostor/card or canopy mass; no shadow casting.
- Small props: disappear earlier with range hysteresis or combine into chunk HLOD.

The current shared meshes help memory, but each tree remains a hierarchy with trunk and leaf entities. Bevy 0.16 has improved static transform propagation and GPU preprocessing, so profile before combining everything. Focus first on visible entity extraction, draw batches, foliage overdraw, and shadow submission.

### Building LOD

Generated town GLBs are already one node, one mesh, one primitive each, which is good for submission. Reviewed examples range roughly from 4,100 to 14,300 triangles, including the scripted style/outline geometry. For a 34-building city this is manageable on many GPUs, but far buildings do not need window recesses, fine trim, and full outline geometry.

Possible levels:

- near: current mesh;
- mid: remove small façade relief and internal edge detail while keeping silhouette/major ink;
- far: one simple colored volume per roof/wall family or cluster block HLOD.

Do not make LOD solely a triangle exercise: reducing overdraw, entities, material sections, and shadow casters may matter more.

---

## 10. P2: avoid disposable mesh-data clones

### Evidence

`src/world/stream.rs:588–612` converts `terrain_core::Geometry` by reference and clones positions, normals, UVs, indices, and optional colors into a Bevy mesh.

For cover and props, the completed `Geometry` is owned locally and discarded immediately after conversion. Cloning doubles peak memory traffic for large welded meadow meshes. Tree geometry at startup is similarly generated, cloned into Bevy meshes, then dropped.

### Recommendation

Provide consuming conversion functions where ownership is available:

```text
Geometry → Mesh by moving places/normals/uvs/indices/colours
```

Keep a borrowing/cloning version only for the few cases where source geometry must remain reusable.

If `terrain_core::Geometry` is in the shared crate, add an engine-neutral `into_parts` method or make its public vectors destructurable without binding Bevy into that crate.

### Benefit

- lower task-completion memory spikes;
- less CPU memory bandwidth;
- faster cover/prop integration;
- no visual or gameplay change.

Profile allocations around `collect_cover` and `collect_props` to quantify.

---

## 11. P2: movement collision repeatedly allocates and regenerates nearby data

### Evidence

While movement input is active, `src/player.rs:662–777`:

- calls `standing_near`, which creates a new vector and regenerates nearby trees/props from procedural lattices;
- calls `Built::walls_near`, which creates a new vector;
- `walls_near` loops layouts and plots, uses square-root distances, calls `Plot::walls`, and extends from additional newly allocated vectors;
- candidate collision is then tested up to three times against the gathered arrays.

The code correctly gathers once for three candidate steps. That prior optimization should remain.

### Options

1. **Reuse buffers:** keep `Local<Vec<Trunk>>` and `Local<Vec<Wall>>`, clear and refill rather than allocate each moving frame.
2. **Avoid nested vectors:** add walls directly to a caller-provided buffer or use an iterator/small fixed array. A normal building has about five slabs; a yard fence about five.
3. **Cache settlement collision geometry:** when a `Layout` is created, compute its wall slabs once and store a spatially bucketed collision representation.
4. **Spatial index:** bucket static walls, trunks, and solid props by chunk/coarse cell, then query nearby buckets.
5. **Movement-cell cache:** regenerate procedural tree/prop collisions only when the player crosses a small cell boundary or the editor changes nearby content.
6. **Squared distance:** use squared comparisons where exact distance is not otherwise needed.

### Caution

There is one player and the query box is only 8×8 metres. This may be inexpensive relative to rendering. Do not introduce a complicated physics engine or dynamic broadphase until Tracy identifies collision gathering as meaningful. Buffer reuse and cached town walls are low-risk; a full spatial index is conditional.

---

## 12. P2: IK hierarchy reconstruction allocates repeatedly

### Evidence

- `src/ik.rs:525–552`: `world_of` creates `Vec::new`, walks the parent chain, then reverses it.
- `plant_the_feet` calls `world_of` repeatedly for both legs and for later bone writes—multiple heap allocations per frame for a hierarchy that does not change after the character scene is loaded.

### Recommendation

- Cache the entity chains for thigh, calf, foot, and required parents inside `Legs` when bones are discovered.
- Or use a small fixed-capacity stack/small-vector because skeletal chains are short.
- Reuse scratch storage rather than heap-allocating per call.
- If safe with animation ordering, evaluate the full needed chain once per side and derive all joint transforms from that pass.

Maintain the current scheduling relative to Bevy animation and transform propagation; correctness is more valuable than the allocation savings.

---

## 13. P2: slow clocks and weather run at frame rate

### Evidence

- `sky::read_the_clock` calls local time every frame and writes `TimeOfDay`.
- `weather::read_the_weather` calls `chrono::Local::now`, terrain region/height, and weather functions every frame.
- `drive_the_sky` mutably accesses directional light, ambient light, and clear color every frame, even though sun transform is separately quantized.
- Some downstream systems contain their own quantization because resources are written continuously.

Cloud and sea motion already use Bevy’s global elapsed time in shaders, so they do not require a system-clock read each frame.

### Recommendation

- Resync wall-clock time at a modest interval such as 0.25–1 second and integrate smoothly from `Time` between syncs if needed.
- Evaluate weather at a slower fixed interval because its inputs change on hour/geographic scales.
- Quantize or compare desired clear/ambient/light values before assignment.
- Expose deliberate state transitions: day, twilight, night, precipitation type, occupancy night ID.
- Keep photo-mode/manual hour controls as immediate invalidations.

### Benefit

The raw clock call is not likely a dominant cost. The real benefit is letting lighting, glass, windows, weather, and material systems run on meaningful state changes rather than all rediscovering slow state every frame.

---

## 14. P2: streaming scans can be event/cell driven

### Current per-frame work

While the world is visible:

- `queue_chunks` counts pending, scans the radius disc, allocates and sorts missing chunks;
- `unload_chunks` scans the loaded map;
- `shade_far_wood` scans loaded chunks;
- cover and prop dress systems scan their small rings;
- cleanup systems scan dressed/answered entities.

The individual counts are modest—253 loaded chunks, 25 cover cells, 49 prop cells—so this is not urgent. Much of it can nevertheless stop after the world settles.

### Recommendation

- Track the anchor’s current chunk and run spatial membership work when it changes.
- Also invalidate when a task completes, a chunk is rebuilt, a season state requires redressing, or editing changes content.
- Precompute radius-nine chunk offsets in distance order once rather than allocating and sorting them repeatedly during initial filling.
- Keep task polling each frame, but separate polling from spatial discovery.

### Scheduling note

`WorldPlugin` currently chains the full sequence of chunk, shadow, cover, and prop systems. Some ordering is real—ground before decoration and deferred component state before dependent systems—but the chain also prevents unrelated work from overlapping.

Use Tracy’s schedule view before changing it. If meaningful:

- define focused system sets such as spatial discovery, task polling, integration, and cleanup;
- order only true dependencies;
- allow independent cover/prop discovery or cleanup to run in parallel when resource access permits;
- remember that the three collectors all mutably access `Assets<Mesh>`, so they will serialize unless integration is redesigned.

Do not simply remove `.chain()`; that risks one-frame state bugs and duplicate work.

---

## 15. P2/P3: materials and change detection

### `part_the_grass`

`src/shade.rs:251–280` creates a new `HashMap` each frame, replaces the previous map, and mutates the cover material every frame.

For the current one-player game:

- a fixed small array or retained map is sufficient;
- update only while a mover changes or the trailing point is still settling;
- once caught up and stationary, stop touching the material until movement resumes;
- remove departed movers by retaining known entity IDs rather than building a fresh map.

If future NPCs use `Wades`, the fixed `MOST_MOVERS` limit already bounds data size.

### Material mutation contention

Multiple systems request `ResMut<Assets<Shaded>>` or `ResMut<Assets<StandardMaterial>>`. Bevy must serialize conflicting systems, even when they mutate unrelated handles. Most updates are slow, but this can become a schedule bottleneck as material systems grow.

Possible future direction:

- centralize slow environment-uniform updates;
- keep frequently changing grass/sea data in per-view/global resources where practical rather than duplicating it per material;
- use an extracted global/environment uniform if a custom render path is already being revised for cloud shadows.

Do not undertake a render architecture rewrite solely for borrow parallelism; measure first.

---

## 16. P2/P3: shadows, MSAA, transparency, and quality tiers

### Directional shadows

Existing comments report cascades at 16.7 ms of a 23.8 ms frame in an earlier configuration. Current code already:

- limits distance to 400 m;
- uses three cascades;
- disables grass and distant tree casting;
- parks shadows at grazing light angles.

Further options if GPU captures still show a shadow bottleneck:

- quality tiers for cascade count/resolution/distance;
- reduce far-cascade caster categories;
- separate static scenery shadow policy from animated/important subjects;
- verify outline hull geometry is not needlessly contributing to shadow maps;
- consider simplified shadow-caster meshes for complex buildings/trees;
- preserve player and near landmark shadows before distant detail.

### MSAA

The HDR main camera uses `Msaa::Sample4`. This is aesthetically defensible for cel edges and geometry outlines but expensive in bandwidth, depth, and some raster workloads.

Capture 1×, 2×, and 4× at the target resolutions. Compare:

- outline stability;
- foliage edges;
- distant building shimmer;
- GPU opaque/transparent pass time;
- memory/bandwidth pressure.

If 2× preserves the look, it is a valuable quality-tier option. Do not remove antialiasing based only on one still image.

### Transparency

Sea, rivers, glass, rain, snow, and emissive panes can cause sorting and overdraw. Focus on:

- actual screen coverage;
- transparent pass timing;
- whether a surface truly needs alpha blending versus opaque/dithered treatment;
- keeping layered panes from occupying the same pixels;
- avoiding distant transparent detail.

The sea’s single large mesh and disabled shadow casting are already good choices.

---

## 17. Memory, loading, and package size

### 17.1 Runtime character texture

`assets/models/person_ranger.glb`, referenced by the active `Build::Ranger`, is about 8.7 MB and contains one embedded 2048×2048 PNG. Decoded RGBA8 is about 16 MiB before mip overhead. This is reasonable for a hero character but should have mipmaps and ideally platform-appropriate GPU compression.

### 17.2 Unused/source assets ship in releases

The release workflows copy the entire `assets` directory:

- Windows: `.github/workflows/release.yml:117`.
- macOS: `packaging/macos-app.sh:30`.

The current asset tree is about 160.6 MiB. It includes apparent source/retired files not referenced at runtime:

- `assets/models/ranger.glb`: ~17.9 MB, including three 4096×4096 maps. If loaded, those decode to about 192 MiB as RGBA8 before mips; no runtime source reference was found.
- `assets/character/walk.glb` and `run.glb`: ~15.5 MB combined; used by authoring/build scripts, while runtime references `person_ranger.glb`.
- `assets/character/retired/*`: roughly 13 MB.

Together these obvious candidates are about 46 MB—nearly 29% of the current asset tree.

### Recommendation

- Package from an explicit runtime manifest/allowlist rather than copying all authoring sources.
- Keep source and retired assets outside the runtime `assets` tree if practical.
- Add CI validation that every packaged runtime reference exists and every large packaged file is intentionally referenced.
- Produce a package inventory with compressed and uncompressed sizes.

This improves download, install, patching, scanning, and package creation without touching frame time.

### 17.3 Texture compression and mipmaps

For runtime textured characters or future materials:

- export mip chains;
- use KTX2/Basis or platform-native GPU compression supported by the pinned engine configuration;
- choose compression by data role: base color, normal, and ORM have different quality needs;
- do not put normal maps through lossy color JPEG workflows;
- use 2K/1K where screen-space evidence shows 4K adds nothing.

Bevy’s current default feature set includes the KTX2/Zstd support pulled by tonemapping LUTs, but verify the exact export/loading path before changing assets.

### 17.4 Dense world layers

`edits.bin` and `surface.bin` are each approximately 47.1 MB. They likely dominate synchronous startup reads and CPU-resident world data. Their formats are owned by `terrain-core` and may be intentionally dense for constant-time lookup.

Conditional investigation if startup or memory exceeds budget:

- measure actual load/decode time and resident memory;
- inspect painted-cell occupancy;
- compare dense, tiled, sparse, RLE, quantized, or compressed-on-disk representations;
- preserve fast random sampling after load;
- version the file format and keep strict refusal of mismatched worlds.

Do not optimize these solely from file size: a dense array can be the fastest runtime representation.

### 17.5 Synchronous `Terrain::new`

The world resource is constructed during plugin build because the player and first tasks need synchronous height queries. That guarantees correctness but delays first interactive frame while world files and settlement planning complete.

If measured startup is poor, introduce a real Loading state:

- show a window/loading presentation first;
- load/decode/plan on a worker;
- enter Menu/Playing only when `TerrainSource` is ready;
- keep all current consumers guarded by the state/resource.

This improves perceived startup but is a larger state-flow change, so make it conditional on measurement.

---

## 18. Release and dependency configuration

### Already good

```toml
[profile.release]
lto = "thin"
codegen-units = 1
```

This matches official Bevy recommendations.

### Conditional dependency slimming

The Bevy dependency uses default features, which include audio, gamepad, picking backends, gizmos, sprites, default font, and other capabilities. Some may be unused by the player build.

An explicit Bevy feature list can reduce compile time, binary size, startup plugin work, and dependency surface. However it increases maintenance and can silently omit required loaders/plugins.

Only do this with:

- a documented required-feature list;
- player and tools feature sets;
- CI checks for both default/tools and `--no-default-features` release;
- startup smoke tests for glTF, JPEG/PNG, animation, UI, windowing, HDR, tonemapping, PBR, and platform support.

### Diagnostics in release

`FrameTimeDiagnosticsPlugin` is added unconditionally, but the HUD consumer is tools-only. Its cost is small; compiling/adding it only for tools is a simple cleanup if release captures show any diagnostic overhead or binary concern.

### Do not upgrade Bevy as an optimization shortcut

Bevy releases do contain major rendering improvements, but the project is intentionally pinned to 0.16 and uses copied/internal shader interfaces. An engine upgrade is a migration project with visual and correctness risk, not a substitute for profiling the current game.

---

## 19. Smaller safe cleanups

These are secondary and should not distract from the top items.

### Use squared distance for threshold-only tests

Several per-frame paths compute square-root distance only to compare against a radius:

- lamp candidate range;
- tower awake range;
- town streaming range;
- wall broadphase;
- some bridge/town checks.

Use `distance_squared` and squared thresholds where the true distance is not needed for falloff or reporting. Lamp intensity still needs actual distance for `carries`, but sorting and inclusion can often use squared values.

### Preallocate predictable vectors

- `queue_chunks::wanted` can reserve the precomputed disc size.
- active lights can reserve `MOST_LIT` or candidate estimates.
- town wall buffers can reserve based on nearby plots.
- river positions/indices can reserve wet-grid estimates.

### River vertex reuse

`build_river` emits four new vertices per wet quad. A dense wet chunk could use up to 16,384 vertices rather than sharing the 4,225 grid vertices. If rivers return and water mesh memory/draw cost matters, build an indexed remap grid. This is irrelevant while rivers are disabled.

### State transitions instead of repeated scans

Good candidates:

- glass emissive day/night state;
- precipitation type/active state;
- occupied-window night state;
- lamps-raised settlement state;
- last streaming anchor chunk;
- last light-selection anchor cell.

### Avoid unconditional component writes

When using `&mut Component`, assignment generally marks it changed. Preserve the existing pattern of comparing before assigning, especially for visibility, transforms that may already match, materials, and UI nodes.

---

## 20. What not to optimize yet

- Do not replace the deterministic terrain system with baked meshes before proving generation/loading is the limiting experience.
- Do not introduce a general physics engine only to avoid small collision buffers.
- Do not merge all buildings into one giant mesh; streaming, culling, interaction, material behavior, and editing need useful boundaries.
- Do not enable local-light shadows as a visual fix without a GPU budget.
- Do not remove the custom cloud/outline style merely because a conventional shader benchmarks faster; first preserve the visual contract with a cheaper representation.
- Do not turn every ECS query into a cache. Queries over one player, one camera, or 20 lights are often already cheap.
- Do not reduce view distance before trying LOD if the visible horizon is part of the game’s identity.
- Do not optimize tests, editor-only image generation, or one-time catalogue work ahead of player frame time.
- Do not judge streaming solely by average stationary FPS.
- Do not make several performance changes in one commit; attribution matters.

---

## 21. Proposed implementation batches for Claude

### Batch A — No-visual-change wins

1. Skip `build_river` while `RIVERS` is false.
2. Hide the precipitation pool only on transition to clear.
3. Add state markers for settlement lamps and awake towers.
4. Gate glass scanning on day/night threshold changes.
5. Consume completed `Geometry` rather than cloning where ownership permits.
6. Reuse collision and IK scratch buffers.

**Proof:** identical screenshots/behavior; reduced system time and allocations.

### Batch B — Streaming smoothness

1. Add completion counters and spans.
2. Cap chunk/cover/prop integrations per frame.
3. Stage tree planting if captures identify it.
4. Precompute sorted chunk offsets.
5. Gate spatial discovery on chunk changes/invalidation.

**Proof:** lower traversal p95/p99 without a noticeable increase in holes or pop-in.

### Batch C — Lighting CPU pass

1. Separate active-light selection from intensity updates.
2. Recompute selection only after meaningful movement/topology change.
3. Use direct parent state for occupied windows.
4. Preserve the 20-light cap, fade, hysteresis, and unshadowed lights.

**Proof:** same night captures and slow-walk behavior; lower `LampPlugin` CPU time.

### Batch D — GPU experiments

1. A/B cloud shadows off/current/reduced count.
2. Prototype low-resolution world-space cloud-shadow field if proven hot.
3. Compare MSAA 1×/2×/4×.
4. Capture directional shadow cost by cascade and caster class.
5. Inspect transparent pass and vegetation overdraw.

**Proof:** GPU median and p95 improve without loss of the semi-cel art contract.

### Batch E — LOD vertical slice

1. One terrain ring with lower resolution and crack treatment.
2. One tree species with near/mid/far forms.
3. One old and one modern building with near/mid forms.
4. Test camera motion, shadow transitions, and outlines.

**Proof:** meaningful GPU/entity reduction in canyon/forest/city scenes with controlled transition artifacts.

### Batch F — Packaging and startup

1. Runtime asset manifest.
2. Exclude source/retired character files.
3. Add package inventory validation.
4. Measure dense world-layer startup/memory.
5. Add a Loading state only if measured startup warrants it.

**Proof:** smaller release artifact and/or faster perceived startup with identical runtime content.

---

## 22. Suggested acceptance metrics

Claude should replace placeholders with target-hardware numbers after the baseline capture.

| Change | Required evidence |
|---|---|
| Skip disabled rivers | initial stream CPU/task time lower; no output difference |
| Clear-weather transition gate | `let_it_fall` near zero while clear; correct first rain/clear frame |
| Lighting state markers | lower LampPlugin time; same active-light set and window patterns |
| Integration budget | lower traversal p99; acceptable chunk/decor arrival latency |
| Consuming geometry conversion | fewer allocation bytes/peak; identical mesh counts and visuals |
| Cloud shadow field | opaque/terrain GPU time lower; cloud/shadow correspondence retained |
| Terrain LOD | lower triangles/GPU time; no cracks or unacceptable morphing |
| Tree/building LOD | lower visible entities/triangles/draw work; stable silhouettes/outlines |
| MSAA tier | measured GPU gain; accepted edge quality in motion |
| Runtime asset manifest | package smaller; all runtime paths validated in CI |

For every batch, test:

- noon and night;
- city, meadow/forest, canyon, and coast;
- stationary and moving camera;
- normal play and fast traversal;
- player release without tools;
- at least one lower-end and one representative target machine.

---

## 23. First message to act on

If Claude wants one contained optimization to implement between visual tasks, choose the disabled-river gate. It is:

- directly proven by current constants and call flow;
- isolated to generation;
- visually inert while rivers are off;
- likely to remove millions of expensive height samples from initial streaming;
- easy to validate with timing and existing tests;
- easy to revert.

For the current lighting objective, the equivalent contained win is adding explicit parent/settlement state so `stand_the_lamps` and `light_the_windows` stop scanning child entities to rediscover whether they already did their work.

---

## 24. Final assessment

Copaimo’s performance architecture is fundamentally sound for its current scope. The code repeatedly demonstrates the right instincts: share, weld, stream, move procedural work off-thread, avoid shadows on detail, compare before mutating, and keep expensive tools out of releases.

The next performance level comes from making those principles fully consistent:

- inactive features should cost almost nothing;
- slow state should update on slow events;
- background tasks should integrate under a frame budget;
- static relationships should be remembered, not rediscovered;
- far geometry should become cheaper with distance;
- screen-wide stylized effects should not perform dozens of analytic tests per fragment when a stable field can encode the same answer;
- release packages should contain runtime content, not the authoring archive.

Measure those changes in representative release captures, preserve the visual contract, and optimize p95/p99 experience rather than chasing an impressive average FPS in an empty field.
