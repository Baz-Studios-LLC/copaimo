# Copaimo foundation deep dive — 2026-08-30

## Scope

This is a read-only review of the foundation that exists now: procedural world generation,
settlement geometry, roads and footways, terrain agreement, streaming, rendering, materials,
visual composition, and developer evidence tools. It deliberately does **not** recommend a
gameplay loop, vertical slice, quest work, or making the current build broadly playable. Those
are later concerns.

No game file was changed. The line numbers below describe the live working tree and may move
while the current uncommitted road/audit work is being developed; the named functions and
types are the durable references.

## Executive reading

Copaimo's main architectural strength is that it increasingly derives drawing, traversal, and
placement from the same procedural facts. `RoadSection`, deterministic site layouts, model
measurements, streamed chunk generation, and the large regression-test suite are unusually
good foundations. The project does not need a rewrite. The highest-value work is closing a few
integration gaps where two otherwise sound systems still disagree.

Recommended order:

1. Make town and country roads use the same actual material, not merely the same vertex data.
2. Make junction patches consume the same named arrival channels as the adjacent ribbon.
3. Decide which authored glTF world objects belong in `Shaded`, and adopt them consistently.
4. Finish the constructed-road geometry contract: station grade, full surface normals, then a
   trimmed node/intersection mesh.
5. Make audit tools wait for semantic world readiness and scan spatially rather than repeatedly
   regenerating candidates at 16 cm intervals.
6. After those facts agree, spend art time on settlement ground hierarchy, thresholds, massing,
   furnishing, and distance composition.

## Finding 1 — P0 visual correctness: country roads do not use `road_material`

**Status:** confirmed in the current working tree.

`shade::road_material` is the one material description that enables the paving controls:

- `src/shade.rs`, `road_material`: `road.extension.paving` receives the stone variation, joint,
  and joint-darkening values.
- `src/world/town.rs`, `raise_the_towns`: town paving uses `road_material()`.
- `src/world/town.rs`, `lay_the_country_roads`: the streamed country-road mesh still creates a
  fresh generic `shaded(StandardMaterial { ... })` whose `CloudShade::paving` remains zero.

This is not merely duplication. The road mesh carries fixed stone size and paving amount in its
vertex attributes, but a material with `paving == Vec4::ZERO` never activates the shader's stone
surface. The approach can therefore widen, gain a kerb, change color, and report that paving is
arriving while its physical stone pattern does not arrive. The separately spawned town mesh then
uses the special material and the pattern appears at that ownership boundary. That directly fits
the repeated symptom of an absent or abrupt dirt-road-to-city-road transition.

There is already a `RoadSurface` resource and `mix_the_road_surface`, but the resource is not
registered or consumed. Two sound solutions exist:

- simplest: have both local material handles call `shade::road_material()`; or
- better ownership: initialize one `RoadSurface` handle and make both town and country draw paths
  require it, with no optional/silent skip.

The guard should exercise the assembled material path, not only `road_material` in isolation.
For example, assert that both road-spawn systems receive the same handle, or factor material
creation behind one required resource and prove its `paving.x > 0`. A shader screenshot at the
city ownership boundary should show no pattern pop.

## Finding 2 — P0 active-work warning: junction discs bypass the new arrival channels

**Status:** found in Claude's current uncommitted transition work; review before committing.

The new `Arriving` type is the right architecture. It separates surface making, carriageway
widening, kerb rise, footway formation, outer tie, stone contrast, and width wander so the approach
can become urban in stages. The road ribbon already consumes those named channels.

The junction-disc portion of `pave` still uses the old raw paving amount:

- center color mixes `ROAD_EARTH` to `ROAD_STONE` with `paved`;
- center UV writes `[0.0, paved]`;
- rim color and UV do the same with `rim_paved`.

That means a transition-area disc can change color and reveal stones according to the old curve
while the adjoining ribbon uses `surface_made` and `stone_contrast`. The likely visual result is a
circular made-road patch appearing earlier than its arms, or a ring where the stone contrast does
not match.

Resolve `Arriving::at(paved)` at the center and `Arriving::at(rim_paved)` at each rim point. Use
`surface_made` for surface color and `stone_contrast` for the shader attribute. The disc can remain
an interim topology while still honoring the transition contract. A focused test should compare
the channel-derived attributes on a ribbon station and its coincident junction center/rim at
several partial paving values.

## Finding 3 — P1 art-direction consistency: authored world glTF scenes bypass `Shaded`

**Status:** confirmed by Copaimo's spawn paths and Bevy 0.16's glTF loader.

`shade.rs` describes `Shaded = ExtendedMaterial<StandardMaterial, CloudShade>` as the material the
world is made of. Procedural terrain, roads, vegetation, and the warden's repainted parts use it.
However, authored world scenes are spawned directly with `SceneRoot` in:

- `world/town.rs` for town buildings;
- `world/bridge.rs` for bridge figures;
- `world/lamp.rs` for lamp figures; and
- `build/mod.rs` for placed models.

Bevy 0.16's glTF loader attaches `MeshMaterial3d<StandardMaterial>` to each primitive, including a
generated default material when the glTF does not explicitly list one. The sampled Copaimo town,
bridge, and lamp GLBs carry vertex colors but no explicit glTF material. There is no world-object
equivalent of `look::paint_the_warden`, which removes the standard material and inserts `Shaded`.

Consequences:

- cloud shadows and custom light banding do not treat major authored buildings like the surrounding
  procedural world;
- the semi-cel treatment changes by asset pipeline rather than by art-direction intent; and
- future global shader controls will appear to work on the ground and vegetation but not on the
  architecture that occupies much of the frame.

Add an explicit policy, not a blanket conversion by accident. Tag roots that should be part of the
solid world, then run an asynchronous scene-adoption system patterned on `paint_the_warden`:

1. find descendant meshes under tagged roots as they arrive;
2. preserve vertex colors and intended base properties;
3. replace their `StandardMaterial` handle with a cached `Shaded` equivalent; and
4. mark converted entities so the scan terminates.

Keep intentional emissive lamp panes, glass, water, particles, UI, and any genuinely unlit ink
geometry in their appropriate specialized materials. A structural audit should assert that every
solid descendant of a tagged town/bridge root has `MeshMaterial3d<Shaded>` and no standard material
after scene loading settles.

**Performance coupling:** moving buildings and bridges into `Shaded` increases screen coverage of
the custom fragment shader. The existing optimization audit already identifies the 30-cloud-disc
fragment loop as a likely GPU hotspot. Benchmark or reduce that loop before or alongside full world
material adoption; otherwise visual consistency may reveal a cost that was previously avoided only
because much of the scene bypassed the shader.

## Finding 4 — P1 lighting correctness: road normals describe only the cross-section

**Status:** confirmed; separate from the recently fixed kerb-face normal.

The new `cross_section` normals correctly derive hard and smooth bands from lateral rise/run. That
fixes the old all-up kerb and is worth keeping. The normal calculation has no longitudinal term,
though. Meanwhile each station's vertices are positioned using terrain height at their own world
locations. A climbing road therefore changes height along its tangent, but its authored normal only
knows about the crown/kerb slope across the tangent.

On a grade, provided normals and geometric face normals disagree. In a cel shader this is more
visible than in soft PBR: a lighting band can remain unnaturally level, flip at station boundaries,
or make the road look pasted over the hill.

Do this in the correct order:

1. establish a controlled station center height/grade for constructed sections instead of allowing
   every lateral lane to independently drape over arbitrary terrain;
2. compute the longitudinal derivative from neighboring station centers;
3. combine it with the lateral profile derivative to form the actual surface tangents; and
4. derive the normal from their cross product, preserving deliberate hard splits at kerb faces.

Add a graded and turning road fixture. For non-hard faces, compare the supplied vertex/face normal
to the geometric triangle normal with a useful dot-product threshold. Keep a low oblique cel-band
capture because a numeric normal can be geometrically valid yet aesthetically unstable.

## Finding 5 — P1 structural debt: the junction patch is still an overlapping disc

**Status:** explicitly acknowledged by Claude as interim; prioritize after the transition contract.

The ten-segment fan successfully hides notches and now handles mixed-width arms more carefully, but
it remains a circular carriageway patch laid over untrimmed ribbons. Its all-up normals, fan topology,
and overlap cannot own:

- curb returns;
- corner footways;
- gutter/drainage continuity;
- ordered paving coordinates; or
- a proof that no coplanar faces remain.

This is why further radius and color tuning has diminishing returns. The production endpoint remains
the one already described in the road research: trim each incident arm to a node boundary, construct
one center polygon, generate curb returns and footway corners from incident sections, and give the
node a deliberate local material field. Preserve the disc only as a named temporary fallback so it
does not quietly become the permanent intersection system.

## Finding 6 — P1 developer-tool reliability: `--audit` waits for frames, not readiness

**Status:** current uncommitted tool; improve before relying on its report.

`audit.rs` starts with `settling: 180`, then audits once the app has spent that many frames in the
playing state. Asset and world readiness are not measured in frames. Three seconds at 60 Hz is not
equivalent to 180 frames on a slow development machine, and an authored-woods replacement already
has a semantic readiness condition elsewhere in the project.

Use explicit readiness predicates/resources for the terrain, prop pool, grove/authored woods, site
layouts, and any loaded model metadata the audit consumes. The report should print the readiness
facts and world seed it observed. A timed safety ceiling may report “incomplete world” but should not
silently substitute for readiness.

The current search is also much more expensive than its evidence requires. Every street is sampled
at 0.16 m; every sample regenerates/query-builds props and trees within an 8 x 8 m box; duplicates
are removed only after those candidates are produced. Across all settlements this repeatedly asks
for the same procedural objects thousands of times.

Invert the work:

1. determine each settlement/approach audit AABB;
2. enumerate the candidate trees and props once per spatial tile or AABB;
3. put them in the existing/proposed spatial buckets;
4. test each candidate against nearby road segments and their resolved `RoadSection`; and
5. classify a definite carriageway obstruction separately from visual verge/footway encroachment.

Also include country-road approach segments through the transition zone. Auditing only
`layout.streets` misses the exact area currently under visual construction.

## Finding 7 — P2 production boundary: evidence drivers are compiled outside `tools`

**Status:** confirmed in the current working tree; low urgency while foundation development is active.

`main.rs` declares and installs `photo`, `drive`, and `audit` unconditionally. The release contract in
`Cargo.toml` says `--no-default-features` removes the maker tools, but these evidence systems are not
behind that feature. They can modify input/time, write evidence, take pictures, or terminate the app
when invoked.

Give evidence tooling its own feature (for example `evidence`) or put it under `tools` if that is the
intended boundary. Enable it in development/CI capture builds and omit it from packaged player builds.
This is primarily about keeping the production state graph and binary honest, not about treating the
tools as unsafe during development.

## Visual reading: what will buy the most quality after correctness

The existing art already has useful strengths: clear large silhouettes, readable old-versus-city
building language, banded vegetation, explicit architectural dark-line geometry, atmospheric light,
and deterministic composition. Historical screenshots also show why settlements can still read as
models placed on a broad field rather than places made by people.

### Build a ground hierarchy from facts already present

Avoid adding uniform texture noise. Derive a small set of low-frequency ground classes from procedural
meaning:

- carriageway;
- gutter/kerb;
- clear footway;
- frontage apron and doorway connection;
- worked yard/market square;
- parcel green/garden;
- drainage/verge; and
- untouched surrounding biome.

Tint, roughness, small wear, and prop eligibility should follow those masks. This will create more
visual information than another global detail texture because it explains how each space is used.
Every non-landmark entrance should receive an explicit route to the footway or yard rather than a door
that terminates in generic parcel ground.

### Treat settlement arrival as an authored sequence

The new arrival channels are a strong foundation. Pair them with deterministic threshold events after
the geometry is correct:

- drainage ditch narrows or closes;
- first constructed edge/kerb terminal appears;
- verge planting becomes maintained;
- lamp and street-tree rhythm begins;
- frontage setbacks tighten; and
- a small transverse material or drainage detail marks the threshold without forming a hard full-width
  texture seam.

These are correlated changes driven by one approach coordinate, not independently randomized props.
The player should read increasing settlement stewardship before the first dense block appears.

### Increase city variety through massing, not noisy decoration

The present city vocabulary relies heavily on a small block/tower/spire family with repeated facade
logic. Add deterministic correlated variants at the silhouette level:

- podium width and depth;
- corner versus mid-block form;
- setbacks/step-backs above selected floors;
- roof/service crowns;
- narrow-through-block versus broad courtyard mass; and
- district-controlled height bands.

Choose a family per parcel/district and let details follow it. Randomizing each window or trim piece
independently would weaken the authored look.

### Keep outlines selective

Continue using black or near-black geometry lines for architectural separations and important object
silhouettes. Do not outline roads, terrain, grass blades, every prop, or every contact edge. The image
needs quiet surfaces to make inked buildings and characters legible. Before adding more outlines,
bring authored world solids into a consistent material pipeline; otherwise outline tuning will be
compensating for inconsistent lighting.

### Add distance separation without sacrificing the maker view

The deliberate no-fog maker view is valuable for inspecting generation. A beauty/release presentation
can still reduce distant contrast and saturation slightly, bias distant values toward the sky, and
soften small ink detail without hiding terrain. Keep the inspection mode exact and let the presentation
mode carry subtle aerial perspective.

## Evidence sequence for Claude

Each step should produce a structural check and a visual check before the next one starts:

1. **Shared road material:** one city approach capture centered on the town/country ownership boundary;
   material-handle or extension-value assertion.
2. **Arrival channels at junctions:** partial-transition fixture at several paving values; attribute
   comparison between arm and node.
3. **World material adoption:** same town/bridge under sun, cloud shade, and night light before/after;
   descendant-material inventory.
4. **Station grade and normals:** straight climb, cambered climb, and climbing bend; geometric-normal
   comparison plus moving cel-band capture.
5. **Trimmed node:** T, four-way, skew, mixed-width, and city-gateway cases; no coplanar overlap and no
   carriageway material over footway.
6. **Ground hierarchy:** one settlement edge, one street at player height, one doorway/frontage, one
   high composition view. Judge use-zones, not texture density.

Do not use the old screenshots as proof that a current uncommitted change failed. They are useful only
as historical evidence of the visual categories above. Capture new matched views after each structural
change.

## What not to do next

- Do not tune more road colors to hide a material-routing error.
- Do not make stones shrink with paving amount; fade contrast/coverage while scale stays physical.
- Do not add a screen-space outline over the whole scene before material ownership is consistent.
- Do not distribute random clutter before ground-use masks decide where clutter is plausible.
- Do not optimize the 0.16 m audit loop by merely increasing the interval; change the spatial query.
- Do not broaden this phase into quests, combat, content loops, or general playability work.

The project is strongest when one fact has one owner. The deep-dive findings are the remaining places
where material ownership, transition ownership, surface orientation, or readiness still has two
answers.
