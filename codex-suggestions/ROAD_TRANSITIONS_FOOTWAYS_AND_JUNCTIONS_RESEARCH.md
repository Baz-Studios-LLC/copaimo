# Road transitions, footways, kerbs, and junctions

## A Claude-facing production brief for Copaimo

This document focuses on the current road pass in `src/world/town.rs`: the dirt-to-city threshold, the new urban street widths and footways, the shared height profile, and the junction geometry. It combines real street-design guidance, AAA procedural-world practices, indie-scaled alternatives, and a code-specific implementation sequence.

The short version is:

> A road transition is not a material fade. It is a gradual change in the whole right-of-way: total width, usable carriageway, walking space, drainage edge, terrain disturbance, street furniture, surface grain, collision, and the way intersections are resolved.

Copaimo already has the right foundation: one centerline network, sampled cross-sections, vertex colors, a paving factor, and a shared `road_surface` function. The next step is to make one semantic cross-section drive every consumer and to stop treating junctions as flat discs.

---

## 1. What AAA and indie standards actually mean here

There is no single formal “AAA road standard.” The production distinction is one of scale, tooling, and review:

| Concern | AAA/open-world practice | Strong indie practice | Copaimo-sized answer |
|---|---|---|---|
| Layout | Art-directable spline graph with typed arteries, locals, service roads, and junction nodes | One curve graph with a small number of explicit road classes | Keep `Way`, but add a semantic road class and receiving-settlement role |
| Geometry | Cross-section assemblies sampled along splines; special intersection generators | A constant-row ribbon plus a few authored intersection/gateway types | Keep the 13-row ribbon; generate rows from a shared `RoadSection` |
| Materials | Layered terrain masks, road shaders, decals, wear, and artist overrides | Vertex colors plus a compact shader or atlas | Continue vertex color now; reserve semantic blend channels for later |
| Terrain | Spline deforms terrain and paints a falloff mask | Grade the terrain once and feather a shoulder | Keep the existing graded corridor; make its width follow the same section |
| Collision/navigation | Derived from the same spline/cross-section data | Analytical height query or simplified collider from the same samples | Make town and country roads share one surface query |
| Intersections | Dedicated node solver, curb corners, crossings, markings, drainage, props | Trim road arms and build one clean center polygon plus sidewalk corners | Replace endpoint discs with joint and junction builders |
| Validation | Automated topology/width/collision checks plus named screenshot matrix | The same checks on fewer representative cases | Add cross-section, seam, overlap, and walk-height assertions to `--matrix` |
| Art direction | Procedural base with local artist overrides | A small gateway kit and deterministic exceptions | One city gateway grammar, parameterized and seeded |

Epic's City Sample is a useful AAA reference because its procedural city data drives not just visible roads but also traffic, AI, audio, lots, and downstream simulation. Its road-network workflow explicitly includes cleanup of short or crowded roads and full-preview review before generation. That is the same “one graph, several consumers, then inspect the result” architecture Copaimo should aim for, without copying City Sample's tool complexity. See [Epic's City Sample Houdini workflow](https://dev.epicgames.com/documentation/unreal-engine/city-sample-quick-start-for-generating-a-city-and-freeway-using-houdini).

Insomniac described a similar production order for *Marvel's Spider-Man*: road curves feed ground, streets, sidewalks, design layout, modular art, pedestrian/traffic data, and finally procedural approval. In other words, the road is upstream world structure, not decorative mesh. See the [GDC presentation, “Procedurally Crafting Manhattan”](https://media.gdcvault.com/gdc2019/presentations/santiago_david_procedurally_crafting_manhattan.pdf).

For an indie-sized analogue, SideFX's PDG course builds paths as an artist-editable system, lets them modify terrain, and stresses recooking only affected areas. The transferable lesson is not “add Houdini”; it is to keep road intent as data and derive local output deterministically. See [PDG for Indie Gamedev, Paths & Roads](https://www.sidefx.com/learn/collections/pdg-for-indie-gamedev/).

---

## 2. The current Copaimo cross-section problem, numerically

The in-progress code has these relevant values:

- Country road width: `ROAD_WIDE = 4.6 m`
- City high street width: `10.0 m`
- City lane width: `8.0 m`
- Intended footway: `2.0 m` each side
- Kerb rise: `0.14 m`
- Dirt-to-paved arrival: `34.0 m`

The current country ribbon keeps a fixed half-width of `2.3 m` while the paving factor opens footways inside it. At full paving:

```text
half width       = 2.30 m
footway request  = 2.00 m per side
walk clamp       = max(2.30 - 2.00, 2.30 × 0.30) = 0.69 m
carriageway      = 2 × 0.69 = 1.38 m
```

That means the approach briefly becomes two sizable footways surrounding a carriageway narrower than one normal travel lane, then snaps to a 10 m city street whose carriageway is 6 m. The surface color can fade perfectly while the silhouette and usable space remain visibly wrong.

Real street references help establish believable proportions. NACTO considers roughly 3.0 m/10 ft an appropriate urban travel-lane width, while UK inclusive-mobility guidance treats 2.0 m as a normal minimum footway width that allows two wheelchair users to pass. These are not mandatory simulation targets for Copaimo, but they confirm that a 6 m two-way carriageway plus two 2 m footways is a coherent 10 m high-street section. See [NACTO's lane-width guidance](https://nacto.org/publication/urban-street-design-guide/street-design-elements/lane-width/) and the UK Department for Transport's [Inclusive Mobility guidance](https://assets.publishing.service.gov.uk/government/uploads/system/uploads/attachment_data/file/1044542/inclusive-mobility-a-guide-to-best-practice-on-access-to-pedestrian-and-transport-infrastructure.pdf).

The implementation consequence is simple:

> Do not subtract urban footways from a fixed country road. Add the urban right-of-way around a carriageway that remains usable.

---

## 3. One semantic `RoadSection` should own the transition

The safest architecture is a pure function that returns a fully resolved cross-section at a point:

```text
RoadSection {
    paved: 0..1,
    total_half,
    carriage_half,
    footway_width,
    kerb_height,
    kerb_run_or_step,
    shoulder_width,
    surface_family,
    hierarchy,
}
```

Inputs should be semantic rather than consumer-specific:

```text
section_at(
    world_position,
    road_class,
    receiving_site,
    distance_to_site_edge,
    local_wear,
) -> RoadSection
```

The following systems should consume that same resolved section:

- Mesh station positions
- Vertex colors/material blend values
- Terrain grading and foliage exclusion width
- `stands_on` and IK height
- Building setback from the street edge
- Lamp, bollard, drain, sign, and tree placement
- Junction-arm geometry
- Capture-report metadata and tests

This is the road equivalent of the window and fence contracts already adopted elsewhere in the project: one fact is calculated once and used, rather than re-described by several systems.

Epic's Landscape Spline interface exposes start width, end width, start/end falloff, roll, and subdivisions as first-class spline inputs. That is strong evidence for treating width and terrain falloff as functions along the road rather than constants attached to an entire segment. See [Epic's Landscape Splines documentation](https://dev.epicgames.com/documentation/en-us/unreal-engine/landscape-splines-in-unreal-engine) and the [Editor Apply Spline parameters](https://dev.epicgames.com/documentation/en-us/unreal-engine/BlueprintAPI/Landscape/Editor/EditorApplySpline).

### Recommended transition equations

Use a smooth, monotonic easing value `p` from `paved_here`. Then derive, rather than clamp:

```text
country_half      = ROAD_WIDE / 2                     # 2.3
urban_half        = receiving_width / 2               # 5.0 high street, 4.0 lane
total_half        = lerp(country_half, urban_half, p)

footway           = lerp(VERGE_LEAST, FOOTWAY_WIDE, p)
carriage_half     = total_half - footway
```

For the high street at `p = 1`:

```text
total width       = 10.0 m
footways          = 2.0 m + 2.0 m
carriageway       = 6.0 m
```

For a lane at `p = 1`:

```text
total width       = 8.0 m
footways          = 2.0 m + 2.0 m
carriageway       = 4.0 m
```

If 4 m feels too narrow for two-way visual language, make lane footways 1.5 m or treat the lane as a shared street. Do not silently let a clamp decide its programme.

Use the receiving city road type as the target. The principal country connection should normally target `CITY_STREET_WIDE`; secondary approaches can target `CITY_LANE_WIDE`. This makes the transition land exactly on the section it joins.

### Keep the stable 13-row topology

The current choice to retain a constant row count through the blend is good. Changing vertex count along a strip creates topology problems. Keep 13 semantic stations, but recompute their offsets at every longitudinal sample:

```text
outer terrain hem
outer edge of right-of-way
back of footway
footway/kerb seam
top of kerb
foot of kerb
carriageway interior
center
mirrored stations
```

At `p = 0`, the footway rows may collapse visually toward a narrow verge, but they must not share identical coordinates. The existing `VERGE_LEAST` and `SEAM` approach is sound for avoiding zero-area triangles.

Bevy supports standard per-vertex colors and custom vertex attributes, so this semantic data can remain on the procedural mesh without introducing a texture-heavy pipeline. See Bevy's [`Mesh` documentation](https://docs.rs/bevy/latest/bevy/prelude/struct.Mesh.html) and [custom vertex attribute example](https://docs.rs/crate/bevy/latest/source/examples/shader_advanced/custom_vertex_attribute.rs).

---

## 4. Transition choreography: geometry, material, and world dressing

The current 34 m fade is about seven seconds of travel at the recorded 4.8 m/s player pace. That can work for the surface itself, but a convincing arrival should begin earlier through environmental cues.

Use two overlapping distances:

- **Broad approach corridor: 60–90 m.** Changes vegetation, maintenance, drainage, props, and sightline composition.
- **Road construction transition: 30–45 m.** Changes width, section, surface, kerb, and footway.

These are art-direction starting points, not external regulations. Tune them with the player camera and travel speed.

### Five stages

#### Stage A — Country road

- 4.6 m worn track
- Soft shoulder and irregular width
- Ruts/wear concentrated toward traveled lines
- Grass and low cover encroach asymmetrically
- No formal curb
- Few or no black lines across the surface

#### Stage B — Maintained approach

- Wander begins to reduce
- Ditches become cleaner or stone-lined
- Loose stone appears at wet/soft edges
- Vegetation is cut back from the traveled way
- Occasional marker, culvert end, milestone, or repaired patch
- Total width begins increasing subtly

#### Stage C — Settlement threshold

- Width expansion becomes legible
- Surface moves dirt → compacted aggregate → setts/cobbles
- First drains or edge stones appear
- Footways open from verges without taking width from the carriageway
- Lighting cadence begins
- A gateway sign, walls, trees, or paired posts announces civic control

NACTO calls a curb extension at the entrance to a slower street a “gateway” treatment because it visibly marks a change in street regime. Copaimo does not need a literal modern curb extension, but the perceptual principle is directly useful: the boundary should be composed, not merely recolored. See [NACTO's gateway guidance](https://nacto.org/publication/urban-street-design-guide/street-design-elements/curb-extensions/gateway/).

#### Stage D — City entrance

- Full target width
- Continuous kerb and footway
- One deliberate crossing or dropped-kerb pair
- First urban lamp pair
- Facades or boundary elements tighten the sightline
- Surface module changes are complete
- Street furniture aligns to a clear furnishing strip, not the walking path

#### Stage E — Interior city street

- Stable section and cadence
- Local variation comes from use: patched paving, drains, thresholds, loading areas, planted sections, and intersections
- Do not continue the transition noise indefinitely; an urban street needs to feel maintained and intentional

Epic's Fortnite road guidance explicitly notes that different meshes can be assigned to individual spline sections for forks or dirt-to-paved transitions, with scale adjusted so adjoining segments match. Copaimo's procedural equivalent is a continuous cross-section function with exact endpoint agreement. See [Creating Roads and Pathways in UEFN](https://dev.epicgames.com/documentation/fortnite/creating-roads-and-pathways-in-unreal-editor-for-fortnite).

---

## 5. Kerbs: visible step, gameplay step, and accessible crossings

The current `KERB_RISE = 0.14 m` is visually plausible. The problem is using `CLIMB_LIMIT` to turn the entire curb into a shallow wedge so the player can cross it anywhere.

A curb and a ramp are different architectural objects:

- The curb should read as a near-vertical boundary.
- The footway should remain a stable walking plane.
- Crossings, gates, and important desire lines should receive explicit curb cuts or blended transitions.
- Player locomotion should distinguish a small step from an impassable slope if free curb traversal is desired.

Real accessibility guidance is useful for scale and intent. The U.S. Access Board specifies curb-ramp running slopes no steeper than 1:12 (8.3%), flush grade breaks, and clear landings; the point is that a pedestrian route crosses a curb through a designed transition rather than through a continuous steep batter. See the [Public Right-of-Way Accessibility Guidelines](https://www.access-board.gov/prowag/technical.html). The [2010 ADA design standards](https://www.ada.gov/law-and-regs/design-standards/2010-stds/) similarly distinguish ordinary walking surfaces from curb ramps.

For a 0.14 m rise, a realistic 1:12 ramp run is about 1.68 m. In Copaimo's stylized scale, 1.4–1.8 m will read clearly and is large enough for a player to intentionally find.

### Three implementation options

#### Preferred: visible curb plus explicit ramps

- Render a near-vertical 0.14 m curb face.
- Add dropped kerbs at intersections, Guild-hall approaches, major doors, and mid-block crossings where useful.
- Let navigation and `stands_on` follow the ramp geometry.
- Keep normal curb faces as small steps if locomotion supports stepping, or as collision boundaries if it does not.

#### Acceptable indie compromise: rendered step, simplified collision

- Render a crisp curb.
- Use a slightly wider invisible/analytical traversal ramp only for player motion.
- Document the visual/collision divergence and bound it tightly.
- Do not use this if IK makes the mismatch conspicuous.

#### Current approach: continuous climbable batter

- Cheapest and fully analytical.
- Weakest visually: the whole street edge reads as a long wedge rather than a curb.
- Removes the level-design value of crossings because every point is equally crossable.

For Copaimo, the preferred option adds useful navigation language: the dropped curb, crosswalk, lamp, and doorway can form a readable path to the Guild hall.

---

## 6. Country-road collision and foot placement must join the contract

The new transition is drawn by the streamed `CountryRoad` mesh, but `stands_on` currently queries only streets inside `Built::standing` settlement layouts. This means the last section of approach road can visibly grow a crown, kerb, and raised footway while the player and IK still stand on the terrain beneath it.

That is not only foot clipping. It affects:

- Step acceptance in `may_step`
- Warden vertical position
- Ankle IK
- Companion movement if it shares terrain height
- Camera bob/contact perception
- Whether a kerb behaves like a barrier

### Recommended ownership

Keep a small `RoadSurfaceCache` alongside the streamed country-road entity:

```text
RoadSurfaceCache {
    cell,
    ways: Vec<Way>,
    bounds,
}
```

When `lay_the_country_roads` rebuilds the mesh, it updates the same cache. `stands_on` queries:

1. Terrain/bridge height
2. Nearby settlement street sections
3. Nearby cached country-road sections
4. Building floors

The height must come from `RoadSection`/`road_surface`, not sampled back from the render mesh. That keeps the analytical query cheap and deterministic while still deriving both outputs from the same facts.

If performance becomes a concern, bucket the cached segments into the same streaming cell or a small spatial grid. The player only needs the nearest few roads.

---

## 7. Junctions: why flat discs fail once roads have structure

The old junction fan solved one problem: two flat ribbons meeting at different angles left a triangular notch. A flat disc covered the hole. Once the cross-section contains carriageway, curb, raised footway, and shoulder, the disc becomes destructive:

- It paints carriageway color over footways.
- It intersects or covers the raised kerb.
- It creates coplanar or near-coplanar surfaces.
- It flattens the crown locally.
- It appears at every polyline endpoint, including ordinary curve subdivisions.
- On a ring, repeated discs can read as beads or patches.

The industry solution is to distinguish an ordinary curve joint from an actual intersection.

### 7.1 Ordinary two-segment bend

Do not cap it. Share one cross-section row between both segments.

At the shared point:

1. Compute incoming and outgoing unit tangents.
2. Compute their normals.
3. Use the normalized sum for the joint normal.
4. Scale lateral offsets by the miter factor `1 / dot(joint_normal, incoming_normal)`.
5. Clamp with a miter limit; if exceeded, use a bevel with one extra row.
6. Reuse the exact same vertex indices for both adjoining pieces where practical.

This preserves every semantic band through the bend and removes overlap.

### 7.2 True junction with three or more arms

Build a node, not a disc:

1. Cluster endpoints within the existing junction tolerance.
2. Gather incident arms and sort them by angle.
3. Sample each arm's `RoadSection` at a trim distance from the node.
4. Emit a left and right carriageway boundary for each arm.
5. Intersect or bevel adjacent boundary rays, with a maximum corner radius/miter.
6. Triangulate one central carriageway polygon.
7. Build footway corner wedges outside that polygon.
8. Insert curb ramps/crossings where the street programme calls for them.
9. Build the outer shoulder/terrain blend last.

This does not require a general polygon-boolean library for the first version. Copaimo's junction valence is small and roads are nearly planar. A sorted radial polygon with clamped corner intersections is enough for a robust indie implementation.

### 7.3 Transition at a junction

Never decide the junction surface from only one arm. Resolve a node section intentionally:

- Inside a city: paved intersection, urban width, footway corners.
- At a gateway: transition node with the country arm widening into the urban receiving arm.
- Outside: dirt intersection with soft shoulders and no curb.

Epic's City Sample specifically calls out road-network cleanup, short-road handling, and merging near intersections to keep traffic flow coherent. That is the data-level counterpart to building junctions as explicit nodes rather than visual patches. See [City Sample road network options](https://dev.epicgames.com/documentation/unreal-engine/city-sample-quick-start-for-generating-a-city-and-freeway-using-houdini).

---

## 8. Material transition for the semi-cel-shaded look

Copaimo's vertex-color approach is a good fit for its art style. More textures are not automatically more quality. The goal is readable material families under cel bands.

### Establish a value hierarchy

At noon, test in grayscale:

- Terrain: local biome value
- Dirt track: slightly darker/warmer than nearby soil or grass
- Compacted threshold: reduced saturation and slightly higher value variation
- Urban carriageway: darkest large horizontal surface
- Footway flags: lighter and warmer than carriageway
- Kerb top/face: the clearest narrow separator
- Wet/drain areas: localized darker accents, not a global gloss layer

The footway should read first from scale and edge, then from color. The current larger `FLAG_IS` relative to cobble grain is sound.

### Blend by construction stage, not only RGB

A good threshold changes several signals together, but at slightly offset distances:

```text
p_surface     dirt -> aggregate -> stone
p_width       narrow -> broad
p_footway     verge -> flags
p_kerb        none -> fragments -> continuous
p_order       wander -> straight edge
p_dressing    wild -> maintained -> civic
```

They can all derive from one `paved`, but remap it:

```text
p_surface = smoothstep(0.00, 0.75, p)
p_width   = smoothstep(0.10, 1.00, p)
p_footway = smoothstep(0.25, 1.00, p)
p_kerb    = smoothstep(0.45, 1.00, p)
p_order   = smoothstep(0.00, 0.85, p)
```

This makes the road feel built in stages rather than cross-faded between two complete objects.

### Keep outlines selective

For the road system:

- Outline the true curb silhouette or give the curb face a dark material band.
- Do not apply inverted hulls to the whole paving ribbon.
- Do not ink every flag or cobble boundary; cel shading turns that into visual static.
- Use a few expansion joints, drain edges, crosswalk boundaries, and damaged patches as authored line events.
- Suppress lines across coplanar ribbon seams and ordinary curve rows.
- At distance, preserve the road edge and gateway silhouette; drop small paving detail first.

The road should look graphic, not diagrammed.

### Future shader path, only if needed

AAA workflows often blend road actors into terrain through landscape masks or virtual texturing; Epic's runtime-virtual-texture example specifically uses a road spline as an actor composited into the landscape. See [Epic's Runtime Virtual Texturing Quick Start](https://dev.epicgames.com/documentation/unreal-engine/runtimevirtual-texturing-quick-start-in-unreal-engine).

Copaimo does not need that machinery now. If vertex colors eventually become limiting, add custom mesh attributes such as:

- paving factor
- normalized lateral position
- semantic band ID
- wear amount
- distance to nearest junction

Then a small Bevy material can add edge dirt, curb shading, or wear without changing topology. Keep the vertex-color version as the baseline and evidence target.

---

## 9. Settlement gateway kit: maximum quality per asset

The most efficient indie upgrade is one procedural gateway kit rather than dozens of unique road meshes.

### Minimum gateway elements

- Paired or asymmetrical entrance markers
- First lamp or lantern pair
- One drainage cue: culvert, gutter inlet, or stone channel
- Surface repair/threshold band
- A maintained verge or planted strip
- A readable sign or civic emblem where appropriate
- One safe crossing/dropped curb if the footway begins there
- A background landmark aligned with the approach when possible

### Variation knobs

- City age/material family
- Wet/dry biome drainage
- Wealth/maintenance
- Main gate versus secondary entrance
- Damage/repair seed
- Tree versus wall versus bollard framing
- Lamp spacing and banner use

### Placement rules

- Keep the clear walking corridor free.
- Place furnishings in a dedicated band behind the curb.
- Avoid perfect bilateral symmetry except at deliberately formal civic entrances.
- Preserve a long view to the Guild hall, spire, or another navigation anchor.
- Use props to reinforce the direction of travel, not to decorate every empty patch.

The real-world “gateway” idea is useful because it pairs road geometry with a perceptual threshold. NACTO notes that gateway treatments visually announce a slower street regime; in game terms, this is also where the player understands “I have entered a city.” See [NACTO Gateway](https://nacto.org/publication/urban-street-design-guide/street-design-elements/curb-extensions/gateway/).

---

## 10. Performance and scalability

The 13-row ribbon almost doubles the cross-road vertex count from the former 7-row version. That is acceptable if the system remains disciplined.

### Keep

- One mesh per raised settlement
- One streamed mesh for nearby country roads
- Vertex colors rather than one material/draw call per band
- Fixed topology per longitudinal station
- Deterministic sampling spacing

### Avoid

- Separate mesh entities for every curb stone or flag
- Flat junction discs layered over profile meshes
- Decal entities for every wear spot
- Recomputing all roads when only one streamed cell changes
- High longitudinal subdivision on straight segments
- Outlines on every paving component

### Adaptive longitudinal sampling

Sample based on curvature and grade, not only distance:

- Long straight/flat road: coarse spacing
- Tight bend, gateway taper, curb ramp, or steep grade: finer spacing
- Require exact samples at transition start/end, site boundary, road-class change, junction trim, and crossing limits

Epic's spline API exposes subdivision count with the explicit warning that higher values improve fidelity but cost performance and can cause artifacts. That supports an error-bounded adaptive sampler rather than uniformly increasing density. See [Editor Apply Spline](https://dev.epicgames.com/documentation/en-us/unreal-engine/BlueprintAPI/Landscape/Editor/EditorApplySpline).

---

## 11. Automated proof: what the matrix should assert

The screenshot matrix should remain visual evidence, but the road generator can cheaply prove its own contracts before the shutter.

### Cross-section invariants

For `p = 0.0, 0.25, 0.5, 0.75, 1.0`:

- `total_half` is monotonic toward the target street.
- `carriage_half >= minimum_for_class / 2`.
- `footway_width >= 0` and is monotonic where a footway is intended.
- All 13 station offsets are strictly ordered, allowing only declared seam tolerances.
- No adjacent stations are closer than a geometric epsilon.
- Kerb top is exactly `KERB_RISE * remap(p)` above its foot.
- At `p = 0`, the section matches the existing country profile within tolerance.
- At `p = 1`, offsets match the receiving city street exactly.

### Mesh topology

- Every triangle has area above epsilon.
- Every rendered road triangle faces upward except deliberately vertical curb faces.
- Shared bend rows are position-identical.
- No duplicate coplanar triangles at joints.
- No central junction patch overlaps the footway polygon.
- UV/color/semantic attributes have exactly one value per vertex.

### Traversal

- Analytical `stands_on` height agrees with the generated section at sampled points.
- Test interior city roads and streamed country transition roads.
- The difference between analytical height and barycentrically sampled mesh height stays below a small tolerance, for example 1–2 cm.
- A curb ramp is traversable under the configured movement rule.
- A normal curb behaves according to the explicit design decision: step, barrier, or simplified ramp.
- IK sole height uses the same result.

### Junctions

- Two-segment bends generate no cap.
- Three- and four-arm nodes generate one central polygon.
- Arms of different widths meet without a gap.
- Footway bands remain continuous around corners or terminate at an explicit crossing.
- Very acute angles trigger the bevel/miter limit rather than creating a long spike.

### Visual captures

Add named shots:

- `city_transition_70m`
- `city_transition_35m`
- `city_transition_threshold`
- `city_kerb_oblique`
- `city_junction_ground`
- `city_junction_high`
- `city_crossing`
- `city_transition_night`

The report should include:

```text
paved factor
total road width
carriageway width
left/right footway width
kerb rise
road class
receiving street class
nearest junction valence
country/town surface owner
```

Then a screenshot labeled “transition” cannot silently depict a 1.38 m carriageway or a country mesh with no traversal owner.

---

## 12. Recommended implementation order

### Phase 0 — Close the asset/output hazards

1. Remove the duplicate extensionless `assets/buildings/City Hall` source reference.
2. Make the source-asset prevention rule cover extensionless files.
3. Put matrix PNGs and reports in a dedicated ignored evidence directory rather than the repository root.

### Phase 1 — Define the section, without changing visuals again

1. Introduce pure `RoadSection`/`section_at` data.
2. Reproduce the current city and country cross-sections from it.
3. Add numeric cross-section tests.
4. Make mesh generation and `stands_on` consume it.

### Phase 2 — Fix the city approach

1. Mark which city street receives each country approach.
2. Interpolate total width to the receiving section.
3. Preserve minimum carriageway width while footways grow.
4. Make terrain grading and foliage clearance follow total width.
5. Add the four transition captures.

### Phase 3 — Bring country roads into traversal

1. Cache nearby country `Way`s with the streamed mesh.
2. Query them in `stands_on` and IK.
3. Verify mesh/analytical height agreement through the transition.

### Phase 4 — Replace discs

1. Remove caps from ordinary two-segment bends.
2. Add miter/bevel row sharing.
3. Build one central polygon for true junctions.
4. Add footway corner wedges.
5. Add one explicit crossing/ramp type.

### Phase 5 — Art-direction pass

1. Add the broad maintained-verge corridor.
2. Add gateway props and first-lamp cadence.
3. Refine material remaps and value hierarchy.
4. Add selective curb ink/dark edge.
5. Tune at noon, night, and motion speed rather than from a still overhead view alone.

### Phase 6 — Performance proof

1. Record road mesh vertices/triangles per settlement and streamed cell.
2. Record regeneration time.
3. Add adaptive longitudinal samples if needed.
4. Confirm one road material/draw path remains.

---

## 13. Acceptance checklist

The road pass is ready when all of these are true:

- [ ] A dirt road broadens continuously into the receiving city street.
- [ ] The carriageway never becomes narrower as footways appear unless an intentional gateway pinch point is authored.
- [ ] Material, width, kerb, and environmental order change over overlapping distances.
- [ ] The city boundary has a composed visual threshold.
- [ ] The player and IK stand on the same raised profile that is rendered.
- [ ] Country transition roads participate in the surface query.
- [ ] Ordinary curve joints have no flat discs.
- [ ] True junctions preserve carriageway and footway semantics.
- [ ] Curb crossings are explicit and legible.
- [ ] Kerb faces create a restrained graphic edge without outlining every paving unit.
- [ ] No zero-area, downward, overlapping, or coplanar junction triangles remain.
- [ ] The screenshot report records cross-section state as well as lighting state.
- [ ] Evidence files do not dirty the project root.
- [ ] The authoring concept sheet remains outside shipped `assets/`.

---

## Sources

- Epic Games, [Landscape Splines in Unreal Engine](https://dev.epicgames.com/documentation/en-us/unreal-engine/landscape-splines-in-unreal-engine)
- Epic Games, [Editor Apply Spline](https://dev.epicgames.com/documentation/en-us/unreal-engine/BlueprintAPI/Landscape/Editor/EditorApplySpline)
- Epic Games, [Creating Roads and Pathways in UEFN](https://dev.epicgames.com/documentation/fortnite/creating-roads-and-pathways-in-unreal-editor-for-fortnite)
- Epic Games, [City Sample — Generating a City and Freeway with Houdini](https://dev.epicgames.com/documentation/unreal-engine/city-sample-quick-start-for-generating-a-city-and-freeway-using-houdini)
- Epic Games, [Runtime Virtual Texturing Quick Start](https://dev.epicgames.com/documentation/unreal-engine/runtimevirtual-texturing-quick-start-in-unreal-engine)
- Insomniac Games/GDC, [Procedurally Crafting Manhattan for *Marvel's Spider-Man*](https://media.gdcvault.com/gdc2019/presentations/santiago_david_procedurally_crafting_manhattan.pdf)
- Insomniac Games/GDC, [Procedural and Automation Techniques for *Sunset Overdrive*](https://www.gdcvault.com/play/1022216/Procedural-and-Automation-Techniques-for)
- SideFX, [PDG for Indie Gamedev](https://www.sidefx.com/learn/collections/pdg-for-indie-gamedev/)
- NACTO, [Lane Width](https://nacto.org/publication/urban-street-design-guide/street-design-elements/lane-width/)
- NACTO, [Gateway Treatments](https://nacto.org/publication/urban-street-design-guide/street-design-elements/curb-extensions/gateway/)
- UK Department for Transport, [Manual for Streets](https://www.gov.uk/government/publications/manual-for-streets)
- UK Department for Transport, [Inclusive Mobility](https://assets.publishing.service.gov.uk/government/uploads/system/uploads/attachment_data/file/1044542/inclusive-mobility-a-guide-to-best-practice-on-access-to-pedestrian-and-transport-infrastructure.pdf)
- U.S. Access Board, [Public Right-of-Way Accessibility Guidelines — Technical Requirements](https://www.access-board.gov/prowag/technical.html)
- U.S. Department of Justice, [2010 ADA Standards for Accessible Design](https://www.ada.gov/law-and-regs/design-standards/2010-stds/)
- Bevy, [`Mesh` and vertex color documentation](https://docs.rs/bevy/latest/bevy/prelude/struct.Mesh.html)
- Bevy, [Custom mesh vertex attribute example](https://docs.rs/crate/bevy/latest/source/examples/shader_advanced/custom_vertex_attribute.rs)

This research is advisory. No Copaimo game file was changed.
