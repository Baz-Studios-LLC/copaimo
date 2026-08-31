# Procedural Junction Nodes: Production Research and a Copaimo-Specific Build Contract

Date: 2026-08-31  
Audience: Claude, while the current `network` / `Node` rewrite is active  
Scope: research and suggestions only; no Copaimo game file was changed

## Executive recommendation

The current architectural direction is right: planarise the road graph, stop every road ribbon at a
mouth, and let one node own the complete intersection surface. The durable version of that idea is:

> **A junction is a bounded road object, not a patch.** It owns an ordered set of arm contacts, the
> closed boundary between those contacts, the connector paths through the interior, its height field,
> material coordinate field, traversal answer, and validation evidence.

That is also how a production road-network interchange format frames the problem. ASAM OpenDRIVE
models paths inside a junction as connecting roads, links incoming lanes to those connectors, requires
linked lanes to fit smoothly, and separately defines a closed junction boundary. Its 1.8 elevation-grid
model even gives incoming roads explicit transition regions into the junction height field. Copaimo
does not need OpenDRIVE's traffic complexity, but it benefits from the same division of ownership.

For this implementation, do the following before visual tuning:

1. Make `network` produce a stable planar graph. A crossing close to an existing sample must snap **and
   split**, not disappear.
2. Give every arm one explicit contact record at the final mouth. Sample that arm's complete
   `RoadSection` and `Arriving` state at the contact, not once at the node center.
3. Construct closed, ordered boundary loops from those contacts. Treat the current polar rings and
   center fan as a fast path only when the loop is demonstrably simple and star-shaped from `Node::at`.
4. Triangulate the actual loop when the fast-path guarantee fails. Validate the input polygon first;
   triangulation is not a polygon-repair system.
5. Make rendering and traversal consume one piecewise surface. Either query the rendered triangles or
   ensure the analytical surface is exactly the same interpolation the triangles use.
6. Represent at least centerline connectors through the node. They can initially serve only tests,
   material flow, and later automated driving; they need not become a traffic simulation.

The two P0 integration findings already placed in `CODEX_REPLY.md` remain the first blockers: country
nodes are rendered but not queried by country traversal, and near-vertex intersections can be removed
without splitting the whole `Way` at that vertex.

## What AAA and strong indie road systems have in common

AAA quality does not begin with more polygons or more decals. It begins with one coherent spatial
contract. A high-budget implementation can later add lanes, turn restrictions, drainage, curb ramps,
crossings, markings, LOD, traffic, and authored overrides. A strong indie implementation can stop much
earlier. Both still require the same five invariants:

- **Topology:** the graph contains every real connection and contains no phantom connection.
- **Ownership:** exactly one surface owns each point; arm and node do not overlap or leave gaps.
- **Continuity:** position, width, height, tangent, and material meet intentionally at every contact.
- **Shared truth:** rendering, collision/traversal, navigation, props, and tests read the same geometry.
- **Determinism:** identical inputs produce identical node IDs, arm order, vertices, triangles, and UVs.

StreetGen's research model makes the same broad case from procedural-city generation: useful street
generation is a coherent combination of network topology, road surface, and associated street objects,
not a bag of unrelated meshes. OpenDRIVE makes the connection model explicit. Neither standard needs
to be copied wholesale; they are evidence for the architecture.

## Recommended data contract

The following is conceptual, not a request to adopt these exact Rust names.

```text
JunctionNode
  id                 stable key derived from snapped graph vertex
  at                 graph-space node point
  arms[]             unique arms in deterministic bearing order
  boundary_loops[]   closed, consistently wound 2D loops
  surface_mesh       authoritative vertices + triangle indices
  connectors[]       permitted centerline/footway paths through interior
  bounds             cheap query/streaming envelope

ArmContact
  way_id / end       stable source edge and which end meets the node
  mouth              final road-space contact point
  tangent, side      unit frame at the contact
  section            complete per-arm RoadSection sampled at mouth
  arriving           material/construction state sampled at mouth
  bands[]            carriage, batter, kerb, seam, footway, shoulder contacts

Connector
  from_arm, to_arm
  curve              tangent-continuous centerline through the node
  width/profile      optional at first
  allowed            optional until traffic rules exist
```

The key difference from the active structure is that the node's `cut` should not be the widest arm's
single vertical truth. The widest section can still bound a cheap query radius. Each mouth should own
its own profile, while the interior surface blends between contact profiles.

## Construction pipeline

### 1. Planarise topology before making geometry

Treat this as graph construction rather than visual intersection detection:

1. Normalize every `Way`: remove duplicate consecutive points and reject non-finite or negligible
   segments.
2. Find all segment/segment contacts, including proper crossings, endpoint-on-segment T contacts, and
   contacts within the chosen snap tolerance.
3. Cluster contact points deterministically. Never let iteration order decide which coordinate wins;
   use a stable representative such as the lowest stable source key followed by a defined weighted or
   snapped coordinate rule.
4. Insert the representative into **every incident Way** at the correct arc-length position. If the
   representative coincides with an existing interior point, retain that point as a split site.
5. Split all affected Ways, remove sub-tolerance fragments, and deduplicate equivalent undirected
   edges.
6. Build node incidence from the resulting edge endpoints, not from the unsplit originals.

This matters directly to the active `planarise`: filtering a candidate because it lies within `SNAPS`
of a segment endpoint is safe only if that endpoint is also made a split site in every other incident
Way. “That crossing is this corner” must mean “use this corner as the graph vertex,” not “discard the
crossing.”

For present settlement sizes, the pairwise search can remain. Instrument it first: number of Ways,
segments, pair tests, contacts, fragments, and total construction time. If it becomes material during
streaming, add a uniform grid/spatial hash around segment AABBs before replacing the robust graph
contract.

### 2. Resolve arm contacts at the actual mouth

The active iterative mouth solve is a good pattern because clipped curved Ways determine their own
contact frames. Finish that contract by resolving every changing input at the winning mouth:

- tangent and side from the clipped Way;
- `RoadSection` including width wander;
- `Arriving::at(paved_here(mouth))`, rather than one `paved` sampled at `Node::at`;
- ground/support height and longitudinal grade at the contact;
- material class and road-relative distance/phase.

The 34 m arrival fade is long enough that a 10–20 m node can span meaningful construction change.
Sampling `Arriving` once at the center can therefore make a node's kerb, footway, shoulder, or material
state disagree with the arm precisely at the handoff. The solver can iterate contact frame and section
together until mouth displacement and every band offset change by less than a small tolerance, with a
fixed maximum iteration count and a diagnostic if it fails.

### 3. Form corners and curb returns from constraints

For each adjacent arm pair in bearing order:

1. Take the exiting edge/band point of arm A and entering edge/band point of arm B.
2. Intersect their tangent rays for the ideal corner/control point.
3. Clamp or replace implausible solutions using a street-class corner policy.
4. Build a tangent-continuous quadratic or cubic return between the contacts.
5. Subdivide by geometric error (maximum chord deviation or angular change), not only a fixed number of
   samples.

NACTO distinguishes the visible curb radius from the effective vehicle turning path and recommends
compact urban corners rather than automatically enlarging the entire intersection. Its examples put
common urban curb radii around 10–15 ft (about 3–4.6 m), with smaller radii possible. Copaimo should use
that as scale context, not a legal requirement. A stylized hierarchy might start around 1.5–2.5 m for
lanes/villages and 3–4.5 m for principal urban streets, then allow an explicit override for gates,
markets, or service approaches. The important rule is that radius comes from street purpose and space,
not a global multiple that allows acute arm pairs to create giant nodes.

Footways should normally follow the curb return as their inner constraint. Later curb ramps and
crossings should be explicit pieces with clear paths; they should not appear accidentally because a
wide blended band happened to become shallow.

### 4. Build and validate explicit boundary loops

The active node converts each band to a radius for a set of bearings, then draws rings from one central
point. This is efficient, but it assumes every radial ray from `Node::at` meets each boundary exactly
once. In computational-geometry terms, each loop must be star-shaped with respect to that point.

That guarantee is fragile at:

- acute or strongly skewed intersections;
- large differences in arm width;
- a bent arm whose mouth is not radial from the graph point;
- two nearby nodes whose mouth clips approach each other;
- outer shoulder/tie bands that use plain corners while inner bands use returns;
- any boundary that doubles back after corner clamping.

Do not repair a doubled-back loop by sorting all points by bearing and keeping the furthest point. That
computes a radial envelope, which can silently change intended mouth segments and erase concavity.
Instead, retain the actual ordered polyline assembled as “mouth, return, next mouth.” Then validate:

- at least three distinct vertices;
- no zero-length edges;
- no non-adjacent segment intersections;
- non-trivial signed area and expected winding;
- nested bands do not cross;
- every mouth segment is represented exactly;
- no triangle or boundary edge crosses outside the owned loop.

If the loop is simple and `Node::at` is inside its kernel, the current fan/ring fast path is valid. If
not, triangulate the explicit polygon. Constrained Delaunay triangulation is the rigorous option; a
lightweight ear-clipping implementation can be an indie-appropriate option for small valid loops.
Mapbox's Earcut documentation is unusually clear about the tradeoff: it is fast and practical, but it
assumes valid polygon input and does not guarantee correct results for arbitrary invalid/self-crossing
data. Its area-deviation check is also a useful model for a postcondition. In other words, validate
first regardless of triangulator.

### 5. Make one surface authoritative in 3D

ASAM's junction-boundary model includes transition areas that interpolate incoming-road height into a
junction elevation field. That suggests a clean Copaimo division:

- each mouth supplies position, cross slope, and longitudinal grade from its arm;
- a short contact strip inside the boundary preserves those constraints;
- the interior blends those strips without creating drainage-breaking pits or sharp creases;
- the outer shoulder/tie returns to the actual terrain.

The current analytical `Node::surface` and rendered triangles must not be two approximations that only
agree at vertices. A center fan linearly interpolates across each triangle. If `Node::surface` is
nonlinear inside that triangle, a player's support height can float above or sink through the mesh.
Choose one:

- **Mesh-authoritative:** locate the containing triangle and barycentrically interpolate the exact
  rendered heights for traversal; or
- **Function-authoritative:** tessellate sufficiently and define `Node::surface` with the identical
  piecewise interpolation and triangle partition used by the mesh.

The first is easier to prove. Cache the node AABB and triangle adjacency or a small local grid so the
query remains cheap. In either case, duplicate vertices where hard curb faces require separate normals.
Derive normals from the actual triangle geometry or from consistent along/across tangents, including
longitudinal grade.

### 6. Give the node a coordinate field, not world-axis paving

A continuous surface still looks procedural if its paving pattern abruptly changes direction or scale.
Road ribbons already have a natural longitudinal/lateral frame. A node needs an equivalent field.

An incremental solution:

1. Build a connector curve for each visually relevant arm pair, tangent to both contacts.
2. For a node vertex, choose or blend the closest relevant connector and compute along/across
   coordinates from it.
3. Preserve physical stone size; fade contrast/coverage with `Arriving`, not texture frequency.
4. Use an irregular cobble/noise treatment in ambiguous central regions rather than forcing a running
   bond through a many-way node.
5. Keep UV/world hashing stable across regeneration, but keep visible course direction road-relative.

For a first correct commit, a node-local planar field aligned with the dominant/through arm is better
than world axes and much simpler than full connector blending. Store the design limitation explicitly
and use irregular surfacing where the discontinuity would be visible.

## Copaimo-specific risks visible in the active tree

### P0: rendered country nodes and traversal do not share ownership

The town path asks `layout.nodes` first and suppresses street sections when a node answers. The country
path still derives support from raw planned Ways. Once country rendering trims arms and draws nodes,
country traversal must query an equivalent node set or it can retain invisible crowns/kerbs through the
rendered interior. This is already detailed in `CODEX_REPLY.md`.

### P0: snapping cannot mean dropping a graph split

An intersection near an interior polyline point must become a shared graph vertex. It is not enough to
filter the new cut as redundant, because node incidence currently comes from whole-Way endpoints. This
is already detailed in `CODEX_REPLY.md`.

### P1: one center-sampled `Arriving` state can disagree with all final mouths

`Node::new` currently receives one `paved` and uses that `Arriving` value while repeatedly moving the
mouths and resampling only width wander. Resolve `paved_here`/`Arriving` per final arm contact so the
six bands and road material meet exactly. A gateway node is the highest-value fixture because it puts
the fade gradient across the node.

### P1: a widest-arm vertical profile is not a mixed-width node surface

`Node.cut` is documented as the widest arm's section. That is useful for bounds but insufficient as the
height profile where a narrow lane meets a broad urban street. Test every arm contact independently:
each of its carriage, curb face/top, footway, and shoulder heights must equal its own ribbon. Blend only
after preserving those boundary constraints.

### P1: polar rings need an explicit validity gate

The current bearing sort, furthest-ray envelope, and center fan can be retained as an optimization for
simple star-shaped loops. Add the proof as a checked precondition. Acute/skew/mixed-width fixtures should
force the fallback if the condition is false, rather than silently changing the polygon to fit the fan.

### P1: analytical height needs triangle-level equivalence

The current node mesh samples the analytical surface at vertices. Add samples inside every triangle,
especially the center fan and curb-return wedges, and compare traversal height with barycentric rendered
height. Vertex-only agreement does not prove surface agreement.

### P1: gateway ownership needs one network boundary

Town and country roads are currently assembled through separate contexts. At the settlement gate,
confirm there is one graph vertex and one node owner, or an explicit contact seam between two node
systems. The `crossing` roads used for lot clearance are not by themselves proof that the town network
contains the country arm. Validate mesh overlap, traversal, material, and stable IDs at that boundary.

### P2: measure network generation before optimizing it

`planarise` is pairwise across Ways/segments and node queries scan node lists. That may be entirely fine
at current scale. Add timing/counters under the audit tool and cache by the same stable plan/edit key
used to build the mesh. Optimize only when settlement regeneration or streamed country redraw shows it
in evidence. A spatial hash is the likely first improvement, not a more complicated triangulator.

## Acceptance matrix

Use small deterministic fixtures before player-height beauty captures.

| Fixture | Required graph result | Required geometry result |
|---|---|---|
| straight continuation | one degree-2 node or intentionally collapsed pass-through | no bump, seam, or UV reset |
| T junction | three unique arms | side road stops at its mouth; no curb crosses through road |
| four-way crossing between samples | four arms and four split edges | one owner, no overlapping ribbons |
| crossing exactly at interior Way vertex | shared graph vertex | same result as nearby crossing |
| crossing just inside/outside snap tolerance | stable documented clustering | no tiny fragment or missing arm |
| skew junction | correct incidence | simple boundary, tangent returns, no long spike |
| acute junction | correct incidence | bounded corner radius; fallback triangulation if needed |
| mixed-width lane to street | distinct arm profiles | every mouth matches its own band heights/widths |
| dirt-to-city gateway | one connected graph | continuous `Arriving`, shoulder, kerb, material, traversal |
| two close nodes | two nodes or deterministic merge policy | neither swallows/inverts the intervening road |
| curved arm mouth | tangent from clipped Way | no gap at outer bands or normal discontinuity |
| country crossing | rendered and walk graph agree | no invisible curb/crown through node |

For every produced node, assert:

- finite coordinates and normals;
- deterministic arm order and IDs;
- no duplicate arm from the same edge end;
- boundary simplicity, winding, nonzero area, and band nesting;
- triangle indices valid, consistent winding, nonzero area, and area sum matching polygon area within
  tolerance;
- no arm triangles within the node-owned interior except the deliberate contact seam;
- each contact band's positions/heights exactly match its road ribbon;
- traversal height matches rendered triangle height at vertices, edge midpoints, triangle centroids,
  curb returns, center, mouth seams, and outer ties;
- no visible gap/overlap in a wireframe or ownership-debug render;
- regeneration produces byte-identical topology/vertex ordering for identical input.

## Staging: foundation first, AAA finish later

### Stage A — correctness gate for the active rewrite

- close the two P0 graph/traversal faults;
- per-mouth section and arrival sampling;
- explicit valid boundary loops plus fan validity gate/fallback;
- rendered/traversal surface equivalence;
- deterministic fixture matrix.

### Stage B — visually credible intersection

- street-class curb returns;
- controlled grade and cross slope;
- road-relative/node-local material coordinates;
- correct split normals at curb faces;
- one clear urban reference node captured in plan, wireframe, normal-debug, low oblique, player height,
  and night views.

### Stage C — AAA-capable extension points

- explicit lane and footway connectors;
- curb ramps, crossings, medians, drainage/gutters, markings, and street furniture bound to the node;
- authored overrides for landmarks and unusual civic spaces;
- LOD/streaming representation that preserves stable IDs and contact seams;
- wear/wetness/decal masks driven by flow and drainage;
- automated character routes that cross every node class and verify thresholds, support height, and
  stuck state.

The AAA standard here is not “implement Stage C now.” It is “do not make Stage A data impossible to
extend into Stage C.” Explicit contacts, loops, connectors, IDs, and shared surface ownership are the
small amount of foundation that preserves that path.

## Sources and how they apply

- [ASAM OpenDRIVE 1.8.1 — Connecting roads](https://publications.pages.asam.net/standards/ASAM_OpenDRIVE/ASAM_OpenDRIVE_Specification/v1.8.1/specification/12_junctions/12_04_connecting_roads.html): production interchange model for explicit internal junction paths, incoming-road contacts, lane links, and smooth fit.
- [ASAM OpenDRIVE 1.8.1 — Junction boundary](https://publications.pages.asam.net/standards/ASAM_OpenDRIVE/ASAM_OpenDRIVE_Specification/v1.8.1/specification/12_junctions/12_10_junction_boundary.html): closed counter-clockwise outer boundary including sidewalk-like lanes and transition length into a junction surface.
- [ASAM OpenDRIVE 1.8.1 — Junction elevation grid](https://publications.pages.asam.net/standards/ASAM_OpenDRIVE/ASAM_OpenDRIVE_Specification/v1.8.1/specification/12_junctions/12_11_junction_elevation_grid.html): explicit transition polygons and interpolation between incoming roads and junction elevation.
- [NACTO Urban Street Design Guide — Corner Radii](https://nacto.org/publication/urban-street-design-guide/intersection-design-elements/corner-radii/): compact curb-return design, visible versus effective turning radius, and context-driven corner geometry.
- [FHWA — Guide for the Planning, Design, and Operation of Pedestrian Facilities](https://highways.dot.gov/safety/pedestrian-bicyclist/safety-tools/33-guide-planning-design-and-operation-pedestrian): intersection concerns that later node data should support, including skew, crossing distance, sidewalks, curb treatments, and curb ramps.
- [CGAL — 2D Triangulations](https://doc.cgal.org/Manual/latest/doc_html/cgal_manual/Triangulation_2/Chapter_main.html): reference for constrained triangulation when a node loop cannot safely use a center fan.
- [Mapbox Earcut](https://github.com/mapbox/earcut): practical fast polygon triangulation, its valid-input limitations, consistent winding, and an area-deviation validation pattern.
- [StreetGen: In-base Procedural-based Road Generation](https://arxiv.org/abs/1801.05741): research model combining street topology, surface, and street objects into one coherent representation.

These sources are design references, not requirements that Copaimo become a civil-engineering or road-
simulation package. The useful common lesson is explicit topology, contacts, bounded ownership, smooth
surface transition, and testable geometry.
