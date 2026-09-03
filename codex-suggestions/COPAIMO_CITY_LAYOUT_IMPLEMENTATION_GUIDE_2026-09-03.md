# Copaimo City Layout Implementation Guide

Date: 2026-09-03  
Audience: Claude  
Priority: P1 visual direction, immediately after blocking correctness work  
Disposition: **accepted direction; needs one prototype before propagation**

## The user's approved target

The user approved these two original concept images:

- [Beauty concept](COPAIMO_CITY_CONCEPT_MARKET_TERRACES_2026-09-03.png)
- [Layout masterplan](COPAIMO_CITY_LAYOUT_MASTERPLAN_2026-09-03.png)

The beauty image is the emotional and visual target. The masterplan makes the spatial relationships easier
to read. Neither is an exact construction drawing, and neither authorizes copying a Pokémon location. The
numeric and relational rules below are the implementation contract.

The intended lesson from creature-collecting JRPG cities is abstract: one memorable civic idea, a legible
landmark and route, strongly programmed public rooms, and creatures integrated into ordinary life. Do not
copy any map, landmark, façade, symbol, palette bundle, creature, shop, or signature composition from an
existing franchise.

## What must survive implementation

1. **No cars and no car-shaped leftovers.** No traffic lanes, parking lots, parking decks, traffic lights,
   petrol stations, vehicle ramps, or road-width voids waiting for cars. Deliveries use people, handcarts and
   suitable Copaimo.
2. **Three coherent levels, not noisy terrain.** The city has a lower arrival quarter, a middle market
   terrace and an upper civic/garden terrace. Buildings share intentional plateaus. Stairs, retaining walls
   and long accessible routes explain every height change.
3. **One dominant public room.** A large market square is visibly enclosed by active mixed-use fronts. It is
   not a disc with repeated objects around it and not a paved gap left after lot placement.
4. **Open space is programmed first.** The park, square, pocket gardens, courts, overlook and service yards
   reserve their ground before ordinary lots are dealt.
5. **Buildings differ in mass and use, not merely scale or tint.** Tone is one minor axis. The street needs
   different rooflines, bay rhythms, footprints, corners, wings, shopfronts, balconies, repair states and
   relationships to neighbouring buildings.
6. **The city has fronts and backs.** Public doors face streets and squares. Deliveries, drains, bins,
   storage, laundry and workshop spill face alleys and courts. `Carries::Service` is the right beginning.
7. **Humans and Copaimo visibly share ordinary life.** Copaimo use ordinary doors. Shared life is shown by
   actions and spaces: drinking, resting, carrying, playing, waiting, working, perching and accompanying.
8. **Urban fauna exists at several scales.** Copaimo coexist with birds, insects and small urban wildlife;
   fauna placement follows habitat and food/water opportunities rather than uniform random spawning.

## A buildable normalized plan

Use settlement-local `(x, z)` coordinates with the arrival at negative `z`. Treat the numbers as a first
prototype range, not universal world constants.

| Zone | Suggested local bounds / centre | Elevation | Programme |
|---|---:|---:|---|
| Lower arrival quarter | `x -105..-25`, `z -125..-35` | `h = 0 m` | arrival court, inn, provisions, narrow homes, first view of landmark |
| Middle market terrace | centre `(-10, 10)`, roughly `135 × 105 m` | `h = +4 m` | market square, mixed-use fronts, workshops west, park entrance east |
| Central market square | centre `(-10, 10)`, `65–75 × 55–65 m` | `h = +4 m` | trading, water, shade, events and meeting; 60–70% clear centre |
| East park room | centre `(75, 25)`, `45–60 × 60–75 m` | `h = +4 to +5 m` | canopy, water, play/rest, perches, varied footing and Copaimo activity |
| Upper civic/garden terrace | centre `(20, 95)`, roughly `140 × 70 m` | `h = +9 m` | civic hall/landmark, formal garden, quiet court and overlook connection |
| West workshop court | centre `(-85, 25)`, `25–35 × 30–40 m` | `h = +3 to +4 m` | purposeful service dead end, repair, materials and deliveries |
| East overlook | around `(105, 95)` | `h = +9 m` | purposeful pedestrian dead end, view, seating and shade/perch |

Do not stamp this footprint into every city. Prove it once as the **Trade city reference slice**, then turn
the relationships—not the silhouette—into reusable rules.

## Movement hierarchy

### Primary route

- One continuous route from arrival through the market to the upper civic terrace.
- Target clear width: `6.5–8 m`; widen deliberately at entries and gathering nodes.
- It may bend and reveal the square gradually. It must not be a ruler-straight axial boulevard.
- Preserve a readable landmark sightline at two or three chosen points, not everywhere.
- Use pedestrian paving with drainage and frontage thresholds, not a fake carriageway plus sidewalks.

Suggested prototype polyline:

`(-75,-125) -> (-70,-85) -> (-48,-48) -> (-25,-20) -> (-10,10) -> (8,48) -> (18,82) -> (20,105)`

### Secondary and service movement

| Route class | Clear width | Rules |
|---|---:|---|
| Neighbourhood lane | `3.5–5 m` | fronts may address it; bends, short offsets and changing enclosure are welcome |
| Service alley | `2.2–3.0 m` | no public fronts; connects stores, drains, bins and courts; still collision-clear |
| Foot shortcut | `1.6–2.2 m` | brief and visible at entry; never the only accessible route |
| Broad public stair | `3–5 m` | `0.15–0.18 m` risers, `0.30–0.35 m` treads, level landings and truthful guards |
| Accessible slope | preferably `1:16`, never steeper than `1:12` for the reference route | needs the real horizontal run: a 4 m rise requires at least 48 m at `1:12` |

Keep the primary graph connected. Imperfection belongs mainly in secondary routes. Include at least:

- one rear alley that rejoins another lane;
- one workshop/service dead end with a visible use at its end;
- one scenic dead end at the overlook;
- one small court reached through a narrow threshold;
- one optional shortcut that rewards exploration.

Every dead end ends in work, rest, discovery, a view or a door. None ends in generic grass.

## Terrace construction and physical truth

- `Site::height` can remain the settlement datum, but it cannot be the height of every plot. Add a small
  authored/generated terrace set whose level is explicit and inherited by its ways, open rooms and plots.
- Prefer three large coherent plateaus to per-building height noise.
- Transition bands belong between plateaus. Keep ordinary buildings away from the fold until foundations,
  retaining walls and traversal agree.
- Retaining walls need caps, believable thickness, drainage outlets or channels, and guards wherever the
  drop is reachable.
- A stair and its accessible alternative must arrive at the same useful destinations.
- Foundation/footing, visible mesh height and collision/traversal height must derive from the same
  terrace/ground authority.
- AQ-034's invalidation lifecycle must remain valid when plots can occupy more than one settlement level.

## Compose the market square as one object

The square should be generated as a **public-room composition**, not as a polygon followed by a furniture
ring.

### Boundary

- Enclose roughly 70–85% of its perimeter with active frontages.
- Use 3–5 deliberate entries of different character: arrival mouth, narrow shop lane, broad upper stair,
  park passage and service gap.
- Place at least three building families around it: arcaded shops, an inn/mixed-use house and a civic or
  guild-related face. Corners should turn or terminate the frontage intentionally.

### Interior zones

- Keep 60–70% of the centre open for circulation, gatherings and event state changes.
- Put the focal water/monument slightly off-centre so movement can pass beside it.
- Place stalls in 2–4 clusters along activity edges, not an even ring.
- Give one edge shade and seating, one edge produce/craft trade, one edge food/water and one quieter meeting
  edge.
- Keep a clear delivery route from a service lane to the backs of stall clusters.
- Attach balloons, ribbons, flowers and banners to entrances, stall clusters, shade structures and event
  anchors. Never distribute them uniformly over the square.

The same square should support at least `quiet`, `ordinary market` and `festival` states through occupancy
and removable dressing, without rebuilding its permanent geometry.

## Compose the park as a public room

The approved image uses the park as a second destination, not leftover green space.

- Give it a clear edge: frontage, retaining wall, garden wall, water or tree belt.
- Provide 3–4 entrances tied to real routes.
- Aim for 35–50% tree-canopy coverage, leaving sunny and shaded zones.
- Use at least three ground families: durable path, soft planted/grass area and water/rock edge.
- Include a social edge, quiet edge, play/exercise pocket and water/rest node.
- Place perches at several heights and drinking/rest positions at several body scales, but do not invent
  special companion doors.
- `Open::Park => (CityGreen, CityForecourt)` is a vocabulary improvement, not the final composition. Replace
  repeated alternation with a small set of named zones and relations.

## Building grammar: controlled difference

Do not ask one finished GLB to become a city by changing scale, tint or yaw. Build each family from a stable
base plus bounded modules/states.

### Minimum families for the prototype

1. narrow house / townhouse;
2. broad shop with arcade or awning;
3. corner mixed-use building with two addressed faces;
4. inn or lodging with balcony and rear court;
5. workshop with service yard;
6. civic/landmark family.

### Variation axes

For each instance, choose coherently from:

- footprint/massing: straight, L-wing, shallow court, attached side addition;
- roof: gable direction, hip, stepped parapet, dormer/chimney group, roof terrace where appropriate;
- frontage: bay count/rhythm, arcade, shopfront, stoop, balcony, awning, corner entrance;
- use: home, shop, workshop, inn, storage, civic;
- age/maintenance: repaired render, patched roof, repaint, scaffolding, closed/open shutters;
- occupation: signs, goods, laundry, plants, lights, seating and delivery evidence;
- tone/material accent chosen from a **city and district palette**, not six globally equal random tones.

Recommended dependency:

`city identity -> district -> block age/use -> building family -> occupant/state`

Hash only inside the allowed set at the final two levels. Correlated variation reads as culture and history;
independent random choices read as AI noise.

## Human, Copaimo and fauna placement

Build activity nodes into the layout even before final character models exist.

| Node | Human action | Copaimo action | Placement |
|---|---|---|---|
| Market trade | browse, sell, carry | wait, sniff, carry basket/pack | stall edge, never central clear lane |
| Water/rest | sit, refill, supervise | drink, rest, cool | park and square edges, shaded where possible |
| Household threshold | talk, sweep, unload | accompany, wait, enter ordinary door | stoop/front court |
| Workshop | repair, stack, deliver | pull/carry/watch | rear service court/dead end |
| Play/exercise | supervise, socialize | play, climb, chase | park pocket away from quiet/rest area |
| Roof/perch | maintain, garden | perch/watch | roofs with access logic; not every roof |
| Urban ecology | feed/ignore | observe/chase | food, water, canopy and wall habitat nodes |

Use paired and small-group routines rather than a uniform crowd scatter. Leave breathing room around doors,
stairs, water and route decisions. Fauna density should respond to canopy, water, food and quiet; do not roll
birds and insects evenly per square metre.

## Generator order

The current generator should move toward this dependency order:

1. Choose an original `CityCard`: civic idea, geography stance, economy, public-life signature, palette,
   landmark and time/event state.
2. Establish settlement datum plus 2–3 terrace levels and transition bands.
3. Lay the connected primary route and accessible level changes.
4. Reserve the market square, park, civic court, workshop court, pocket gardens and overlook.
5. Lay secondary frontage lanes and service alleys, including purposeful dead ends.
6. Form coherent blocks and label each edge `public front`, `service back`, `party/shared wall` or `open edge`.
7. Deal programme/massing families to blocks, then apply controlled instance modules/states.
8. Place landmarks and active frontage entrances.
9. Compose public-room zones and semantic props from their anchors.
10. Add human/Copaimo activity nodes, fauna habitat nodes, lighting and event dressing.
11. Derive collision, paving, drainage, foundations and streaming representation from the same layout.

`Plan::{Rings, Grid, Spine}` may remain the coarse settlement history, but it is not enough to compose the
approved city. `Open::wanted`, `Open::spans` and `Open::fills` need programme/boundary/entry/zoning data; a
list of repeated models around a shape cannot express the target.

## How to avoid another city-wide wrong answer

Build and review one `60–100 m` slice first:

- one market-square edge and its opposite focal view;
- 6–8 buildings from at least 3 massing/programme families;
- two levels, a stair and the real accessible alternative;
- one public front, one service alley and one meaningful dead end;
- one park/commons edge;
- three visible human–Copaimo shared-life nodes.

Only propagate after the slice passes. Do not spend another world-wide commit multiplying tint, yards or
furniture before the composition reads at player height.

## Evidence and acceptance

Capture identical fixed views before and after:

1. distant arrival skyline/reveal;
2. market entry at player height;
3. square centre looking toward active fronts;
4. park entrance and water/rest node;
5. public front versus rear service alley pair;
6. stair profile showing landings/guards plus the accessible route;
7. overhead plan with public rooms and route hierarchy visible;
8. dusk view proving windows, stalls and route lighting create activity without flattening materials.

Ask five questions without labels:

- Can a viewer identify the city's main destination?
- Can they distinguish front from back?
- Can they explain what every open parcel is for?
- Can they name at least three building differences that are not size or colour?
- Can they see ordinary humans and Copaimo living together rather than Copaimo used as decoration?

Also prove:

- every public destination has a connected walkable route and accessible alternative;
- visible walls, stairs, bridges and retaining edges agree with collision;
- no building/prop floats, sinks or clips at terrace transitions;
- tone/scene processing reaches near-zero steady-state work instead of scanning every unowned mesh forever;
- a repeated-seed rebuild produces identical layout and activity anchors;
- the city stays within an explicit scene/material/light/CPU budget.

## Definition of done

This direction is not done when a park alternates two models, yards move closer to kerbs, or buildings take
six tones. Those are useful ingredients. It is done when the reviewed prototype visibly matches the
relationships in the approved images, passes the player-height questions above, and its rules can generate
a second city that is equally coherent but unmistakably different.
