# Copaimo City 02: modern transition and outward progression

Status: **user-approved visual direction and progression; implementation suggestion for Claude**

Visual reference: `COPAIMO_CITY_02_TRANSITIONAL_MODERN_CONCEPT_2026-09-03.png`

Authority order: user constraints -> this written contract -> image as mood/composition reference

## The decision in one sentence

City 02 is the seam between Copaimo's older built world and its advancing one: a warm historic pedestrian
quarter grows into a contemporary stone, glass, copper and planted civic district, while every later city
becomes progressively more advanced with distance from the ranch and the farthest city is almost futuristic.

The image is not a literal plan to trace. Its useful information is the relationship between old and new,
the sequence of public rooms, the visible vertical circulation, the mixed building eras, and the density of
ordinary shared life.

## Non-negotiable world rule

The seven cities have an authored progression order based on **distance from the ranch**, nearest first.
Technology level must not be derived from `SETTLEMENTS` declaration order, `Character::of`, `Plan::of`, a
random seed, or whichever city happens to stream first.

Current ranch position resolves to approximately `(-4595.712, 988.321)`. The current city sites sort as:

| City rank | Site `(x, z)` | Ranch distance | Broad visual era |
|---:|---:|---:|---|
| 01 | `(-2553, 1771)` | 2187.5 m | heritage market / earliest urban layer |
| 02 | `(-321, 1593)` | 4317.3 m | old city plus contemporary civic growth |
| 03 | `(223, 385)` | 4856.3 m | planned modern trade city |
| 04 | `(1341, -63)` | 6029.1 m | mature mechanical/civic city |
| 05 | `(408, 4388)` | 6049.4 m | vertical ecological city |
| 06 | `(-708, 6211)` | 6510.8 m | advanced research/culture city |
| 07 | `(3401, -1370)` | 8337.2 m | almost-futuristic high-technology city |

This catches a present mapping trap: `(3401, -1370)` is the fifth city in `SETTLEMENTS`, but it is the
farthest city in play-space. Declaration order therefore cannot express the user's progression.

The user has also approved moving city sites when necessary. These coordinates are the current measured
order, not immutable placements. If terrain, routes, biome fit, composition or an arrival experience
materially improves by relocating a city, update the site and then regenerate the distance order and its
tests. Preserve the ranch-relative 1→7 advancement ladder; relocation must not silently swap narrative
eras.

### Recommended data separation

Keep four facts orthogonal:

`City = progression rank/era × Character × Plan × biome/context`

- **Progression rank** controls technological maturity, infrastructure, fabrication and new-building share.
- **Character** controls what the city is for: capital, works, green or trade.
- **Plan** controls its large-scale street silhouette: rings, grid or spine.
- **Biome/context** controls local weathering, planting, shelter, foundation and material response.

Do not replace `Character` with era. A high-tech Works city and high-tech Green city should still be clearly
different. Do not replace `Plan` with era either; progression expressed only as progressively straighter
roads will make later cities less believable rather than more advanced.

Prefer an explicit stable `city_rank`/`progression` fact assigned during world planning:

1. collect only `city == true` sites with their original stable index;
2. sort by squared distance to `RANCH_AT`, tie-breaking with that stable index;
3. assign ranks 1 through 7 once;
4. carry the rank on the site or on an authored city-definition table;
5. test the seven coordinate-to-rank mappings above.

If the sites are intended to remain hand-authored, an explicit coordinate/id-to-rank table is also valid
and makes the narrative order impossible to perturb accidentally. What matters is that this is authored
world progression, not procedural luck.

## Progression must read as accumulated history

Increasing technology does **not** mean erasing everything old, making every building glass, or increasing
height at every stop. Each city should contain layers:

- the oldest route, water edge, market, shrine, guild or surviving block;
- an established middle layer showing the city's main period of growth;
- a smaller leading edge showing what its society can build now.

The farther cities preserve less old fabric by share, but never zero. Even City 07 needs a conserved lane,
foundation, civic object or old quarter so the advanced world feels inherited rather than generated
yesterday. Conversely, City 01 can possess maintained lighting, drainage and skilled construction; older
does not mean incompetent or dirty.

### Seven-city visual ladder

| Rank | Approx. old/new frontage share | Infrastructure that tells the era | Avoid |
|---:|---:|---|---|
| 01 | 80/20 | masonry drains, warm lamps, hand-worked signs, stairs and simple ramps | depicting poverty as age |
| 02 | 55/45 | public lift, covered galleries, integrated civic lighting, green roofs, refined drainage | a modern skin over the same old blocks |
| 03 | 40/60 | modular mixed-use bays, organized service courts, larger glass spans, better wayfinding | perfect grid or shopping-mall sterility |
| 04 | 30/70 | visible mechanical systems, fabricated bridges, workshops, public utility halls | pipes/gears scattered as decoration |
| 05 | 25/75 | stacked gardens, water recovery, climate shade, linked terraces and lifts | empty eco-utopia lawns |
| 06 | 15/85 | advanced research/cultural envelopes, responsive shade/light, precise public interfaces | neon overload or unreadable forms |
| 07 | 10/90 | near-future composites, quiet energy systems, seamless vertical mobility, luminous restrained wayfinding | generic science-fiction skyline |

The proportions are composition targets, not per-building dice weights. Place an old quarter and a new
quarter deliberately; do not hash each lot independently and produce visual salt-and-pepper.

## City 02 identity

Working identity: **The Terrace Exchange** — an old trade settlement whose prosperity funded a newer civic,
horticultural and fabrication district uphill.

Player read from the approach:

> I enter through an older inhabited quarter, cross a lively seam where the market changed generations,
> descend into a contemporary civic forum, then rise by stair, ramp or lift into a planted modern district.

This city should feel optimistic and capable, but not futuristic. City 02 introduces the player to the fact
that Copaimo's societies advance; it must leave obvious visual headroom for Cities 03–07.

### City 02 composition ratio

- 45–55% historic or adapted frontage in the arrival/market quarter.
- 30–40% clearly contemporary frontage around the forum and upper district.
- 10–20% hybrid buildings where an old masonry/timber base supports a newer light structure.
- No more than one local vertical landmark and one long contemporary roof gesture in the prototype view.
- Most buildings remain 2–5 occupied floors; modernization is expressed by systems, spans, access and
  construction, not by filling the skyline with towers.

## Spatial sequence: reserve these before parcels

Use the city's approach bearing to orient a local frame. Preserve the sequence even if the current `Plan`
changes its geometry.

### 1. Arrival promenade — old fabric

- A 6.5–8.0 m pedestrian primary route enters between attached or closely spaced older buildings.
- Frontages are irregular but maintain a readable walking channel.
- Thresholds, stoops, shutters, laundry, small awnings, repair patches and household Copaimo traces show
  daily habitation.
- The city is revealed in stages. Do not expose every plaza and modern building at the boundary.

### 2. Market seam — old meets new

- The old market widens asymmetrically into a 35–50 m public room.
- Historic shop houses occupy one or two sides; adapted masonry arcades and a contemporary canopy occupy
  another.
- Stalls cluster along serviced edges and leave a 4 m minimum continuous movement route.
- Paving records generations: repaired stone near old fronts, larger precise slabs near the new concourse,
  joined by a deliberate 3–6 m transition band rather than a hard material line.
- Drainage remains continuous across both eras.

### 3. Sunken civic forum — contemporary heart

- Reserve an oval or softened-polygon room approximately 42–58 m across and 1.2–2.0 m below the main
  promenade, not a perfect circular arena.
- Provide at least three entries: broad social steps, an accessible ramp at 1:16 preferred, and a short route
  from the public lift.
- Keep 55–65% of the centre clear for gathering, performance and events.
- Put activity on the edge: guild/civic counter, food, shaded seating, water, creature rest pockets and a
  small performance edge.
- Guards are required where the drop exceeds the game's safe unguarded threshold. Do not use benches or
  planters as accidental fall protection.

### 4. Lift-and-gallery seam — visible modernization

- One public lift tower is the city landmark: pale stone or masonry base, dark structural frame, restrained
  teal glass, visible landings and a roof cap that sheds water.
- The lift must connect useful levels, not exist as an ornamental glass shaft.
- Every destination also has a stair; the principal destination has an accessible alternate route.
- Covered galleries connect active frontages and create rain/shade shelter without becoming enclosed tubes.
- One or two lightweight pedestrian bridges may cross an actual gap. Never add a bridge merely to signal
  technology.

### 5. Upper garden/innovation district

- Mixed-use buildings face a planted terrace rather than freestanding in lawns.
- The park is a programmed urban room with a through-route, play/training patch, quiet shaded edge, water,
  creature-safe planting and a pavilion/greenhouse.
- Green roofs are accessible or maintainable and visibly drained; they are not green rectangles pasted on
  roofs.
- A small fabrication/work courtyard provides a purposeful service destination and prevents the district
  from reading as a luxury campus.

### 6. Back network

- 2.2–3.4 m service alleys run behind frontages and connect screened bins, vendor servicing, repair yards,
  kitchen doors and handcart storage.
- Front doors do not address service lanes. Service doors may.
- At least one secondary lane ends deliberately in a workshop gate, shared garden, overlook or small court.
- Avoid a fully connected diagram: primary routes connect; local/service routes may stop, bend and misalign.

## City 02 building families

Build or compose at least these eight massing families before relying on prop variation:

1. **Old attached shop house** — plaster/timber or masonry, pitched roof, 2–3 floors, narrow bay rhythm.
2. **Adapted market hall** — older load-bearing shell with a contemporary canopy, repaired openings and an
   active service side.
3. **Hybrid courtyard housing** — masonry base, lighter upper additions, balconies and a shared inner court.
4. **Contemporary civic block** — pale engineered stone, clear structural bays, deep shaded entrance and
   limited large glazing.
5. **Lift/wayfinding tower** — a singular vertical circulation landmark, not repeated as a generic tower.
6. **Stepped mixed-use terrace** — shops/workrooms below, homes above, planted roof or occupied terrace.
7. **Greenhouse/pavilion** — transparent but visibly framed, ventilated and founded; belongs to the park.
8. **Fabrication/work hall** — low, broad, ventilated, robust doors, yard and visible practical servicing.

For each family define controlled variants in this order:

`family massing -> structural bay pattern -> corner/end condition -> roof/parapet -> ground-floor use -> occupation/repair -> restrained color accent`

Color is last. A tint is never a substitute for a different family.

## Material and color contract

### Historic layer

- warm ivory and sand plaster;
- weathered local stone with readable joints at close range;
- terracotta, slate and dark timber roofs;
- aged timber, cloth awnings and limited painted ceramic accents.

### Contemporary layer

- pale precise stone panels with believable joints and anchors;
- charcoal or dark bronze structural metal;
- desaturated teal/blue-green glass, never uniformly mirror-blue;
- restrained copper, colored ceramic or enamel accents;
- timber soffits and planted surfaces to retain warmth.

### Transition rule

Old and new share two anchors—local stone and warm timber—so the city is cohesive. They differ in span,
joint precision, glazing area, roof construction, drainage integration and public lighting. Do not make the
eras legible only by hue.

### Semi-cel-shaded outline rule

- strongest black/dark ink on skyline silhouettes, primary object separation and major overlapping forms;
- medium ink on doors, principal window frames, balcony edges and major material boundaries;
- little or no ink on every paving joint, leaf, glass mullion or tiny prop;
- use contact shadow, value separation and AO for grounding rather than outlining every ground contact;
- glass edges need selective structure lines, not a black rectangle around every pane.

## Construction credibility checklist

Every contemporary visual feature needs a physical answer:

- **Glass:** frame, mullion rhythm, opaque floor/spandrel zone, corner/end detail and maintenance access.
- **Canopy:** support or credible cantilever, drainage edge, downpipe/scupper and clearance above heads.
- **Green roof:** parapet/guard where occupied, soil depth cue, drainage/overflow and access.
- **Retaining wall:** cap, weep/drainage logic, guarded drop and no floating façade across level changes.
- **Bridge:** landings on both ends, depth appropriate to span, guard/handrail and route purpose.
- **Lift:** door at every served landing, weather enclosure, visible base and paired stair/escape logic.
- **Ramp:** continuous clear width, landings, edge protection and grade tested by the player controller.
- **Mixed-use frontage:** door reaches floor, shop threshold meets paving, rear servicing does not cross the
  principal public room.

Fantasy technology may change the mechanism, but the city still has to show support, access, weathering,
safety and maintenance. If magic removes a normal requirement, communicate that deliberately with a device
or material behavior rather than an unexplained omission.

## Humans, Copaimo and fauna are part of the plan

Do not place living things as a final random scatter. Author activity sockets with the public rooms:

- market vendor + customer + small companion waiting/resting at the edge;
- household threshold + water bowl/rest mat + resident task;
- forum performance/training edge with spectators and clear circulation behind them;
- shaded park rest group sized for a human and at least two different Copaimo body envelopes;
- workshop/service gate with handcart, worker, material stack and safe passing width;
- roof-garden tending activity with guarded access;
- urban fauna at food, water, canopy and masonry niches, away from the centre of travel paths.

Use different time states—opening, peak, evening cleanup, rain shelter—so the same city can feel occupied
without increasing permanent clutter.

## Implementation order for Claude

1. Add/test a stable city progression rank derived from ranch distance or an explicit authored table.
2. Keep progression, `Character`, `Plan` and biome as independent inputs to a city card.
3. Create City 02's city card and its old/new/hybrid district masks; never roll era per lot.
4. Reserve the arrival, market seam, forum, lift seam, upper park and service court before lot subdivision.
5. Establish 2–3 coherent terrace heights and all connecting stairs/ramps/lift landings.
6. Lay the connected primary promenade, then secondary streets, then back/service lanes and the purposeful
   dead end.
7. Place only gray-box volumes from the eight families and validate silhouette, spacing, door address and
   collision.
8. Build one 80–120 m vertical slice spanning old -> seam -> modern, rather than generating the whole city.
9. Add materials, structural detail and semantic occupation only after the gray-box composition reads.
10. Capture and review the fixed proof set below before propagation.

## Fixed proof set and acceptance gate

Capture the same seed, time and camera positions after every relevant iteration:

1. approach reveal from outside the city;
2. high oblique showing old/new district composition;
3. eye-level old market seam looking toward the modern district;
4. forum centre showing all access routes;
5. lift base and one upper landing;
6. park edge with humans, Copaimo and fauna sharing it;
7. rear/service alley from human eye height;
8. side profile showing terrace elevations, retaining walls, stairs and accessible route;
9. night/rain view proving shelter, lighting and wet-material hierarchy;
10. collision/debug overlay for the same 80–120 m slice.

City 02 passes when an unlabeled observer can answer all of these:

- Which area is old, which is new, and where do they meet?
- What is the primary route and where does it lead?
- Can I reach every public level without guessing?
- Does each open space have a named use and an enclosing edge?
- Can I distinguish at least eight building families by massing rather than color?
- Is there an obvious front, back and service condition?
- Do humans and Copaimo appear to share ordinary life rather than occupy separate attractions?
- Could a builder explain how the visible glass, canopy, roof garden, bridge, lift and wall stand up and shed
  water?
- Does it look more advanced than City 01 while leaving substantial room for Cities 03–07?

If any answer is no, do not compensate with more props. Correct the route, room, level, massing or building
grammar that caused the failure.

## Performance and determinism guardrails

- District/era assignment should be computed once per city layout, not queried on every mesh every frame.
- Building family and variant must be deterministic for world seed + stable city/block/lot identity.
- Reuse modular meshes/material families, but vary compositions and end conditions; cloning complete
  buildings is the repetition the user is rejecting.
- Use LOD/HLOD or equivalent aggregation for distant glazing frames, balcony rails, market clutter and roof
  planting; ink should not preserve sub-pixel geometry into the skyline.
- Activity density needs distance and visibility budgets; empty sockets are preferable to permanently
  spawning every narrative prop.
- Modern glass, lighting and vegetation must enter the existing performance evidence route before City 02
  is propagated.

## Scope reminder

This document and its image are design/implementation guidance. Codex has not modified the game. Claude
owns implementation, validation, disposition and any adaptation required by Copaimo's actual systems.
