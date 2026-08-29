# Building Design for Copaimo: Exteriors, Interiors, Production, and Procedural Rules

**Audience:** Claude, as implementation and design guidance  
**Scope:** Building design only. This document does not request that every suggestion be implemented, and it does not modify the game.  
**Project fit:** Copaimo, Rust + Bevy 0.16, third-person camera, semi-cel-shaded rendering, procedural settlements, Blender-generated glTF buildings.

---

## 1. Executive recommendation

Copaimo should not pursue “a generator that can make any building.” It should pursue **a small number of coherent building grammars that can produce many believable, playable variants**.

The strongest practical model is:

1. **Program first:** decide what the building is for and who uses it.
2. **Plan second:** define rooms, their adjacencies, public/private/service zones, and vertical circulation as data.
3. **Envelope third:** derive doors, windows, chimneys, service openings, and façade rhythm from the plan instead of decorating a shell independently.
4. **Furnishing fourth:** place functional clusters around anchors while preserving explicit traversal and camera clearances.
5. **Art pass fifth:** apply one architectural family, controlled asymmetry, construction logic, wear, lighting, and selective ink.
6. **Runtime representation last:** choose a closed shell, enterable shell plus interior, or hero building based on gameplay value and measured cost.

For this project, the right indie-scale implementation is initially **authored plan templates plus controlled procedural variation**, not a general optimizer. A cottage, shop, townhouse, guild hall, and city lobby can each have two or three validated plan templates. The generator may mirror, stretch by full 1.5 m bays, change secondary rooms, choose compatible props, and alter façade dressing. It may not randomize structural facts such as whether the entrance reaches the public room, whether stairs reach every promised floor, or whether a shop counter blocks its own door.

The most important quality rule is:

> **Exterior variety may be broad; interior navigation grammar should be narrow and learnable.**

Embark reached this conclusion in production on *THE FINALS*. More than 30 fully traversable buildings with one to five floors and attics became confusing despite their visual variety. Their Monaco family adopted consistent entrance, hallway, stair, exit, and attic rules so that learning one building helped players read the next. They also removed furniture that impeded fast traversal. Copaimo is slower and can support richer interiors, but the lesson holds: a believable room is not automatically a playable room.

---

## 2. What Copaimo already gets right

The current building work is not a blank slate. Preserve these decisions unless playtesting disproves them:

- A **1.5 m module** connects procedural buildings to the workbench kit.
- The **3.6 m fantasy storey** is sized for the third-person camera, not only a 1.7 m character.
- A **1.9 × 2.45 m clear doorway** acknowledges the camera and avoids decorative-looking entrances the player cannot actually use.
- Walls have **real thickness**, doorways are real gaps, floors meet thresholds, and doorsteps bridge terrain to the interior.
- Façades are already split hierarchically into storeys, walls, bays, and opening types. This is consistent with the CGA/split-grammar literature.
- Windows have frames, sills, mullions, shutters, and surrounding construction, rather than being colored rectangles on a wall.
- Building silhouettes already use large, medium, and small shape hierarchy.
- Village/town and modern-city buildings use distinct architectural families, correctly making history legible through skyline and material vocabulary.
- Tall city buildings have a base, shaft, and top, and enterable towers provide a lobby instead of pretending every upper floor exists as playable space.
- Open and closed variants already recognize that not every door must open. Guild halls, shops, and selected homes can carry interiors; the rest can remain honest closed buildings.
- The game tests the relationship between Blender door orientation, world placement, collision gaps, and actual player traversal.
- Buildings are welded into whole glTF scenes rather than spawning every modeling box as an entity.

The central missing layer is not more façade decoration. It is a **shared semantic model connecting the lot, footprint, rooms, openings, furnishing, collision, lighting, and runtime visibility**.

Current interiors are useful proofs: a cottage has a hearth and bed, a shop has a counter and stock, a guild hall has a long table, and a tower has a lobby/core. They are not yet floor plans in the architectural or level-design sense. Most are one open volume per floor with semantic props. The next gain comes from relationships, not prop count.

---

## 3. AAA standards versus indie standards

“AAA quality” and “AAA scope” are different things. Copaimo can apply AAA decision standards without building AAA quantities of content or tooling.

| Concern | AAA production pattern | Appropriate Copaimo interpretation |
|---|---|---|
| Building tools | Node-based authoring, procedural feature stack, artist overrides, automated collision/occluders/LODs | Deterministic staged generator with small, testable functions and explicit override data |
| Variety | Many kits, architectural families, hero exceptions, regional material sets | Two strong families now; a few high-value archetypes and variants per family |
| Interiors | Complete room taxonomies, streaming cells, portals, navmesh, lighting scenarios, encounter markup | Validated plan templates; split exterior/interior assets before per-room streaming |
| Procedural plans | Constraint solver or graph-to-geometry system with artist correction | Authored adjacency graphs and rectangular/grid layouts; reject or fall back on invalid output |
| Art | Hero assets plus layered reusable kits, decals, trim sheets, material instances | Shared silhouette/structure modules, vertex-color palette, a small family-specific dressing set |
| Optimization | Platform budgets, HLOD, proxy meshes, occlusion data, automated reports | Measure representative scenes; closed/open tiers; distant proxies; cap visible lights and collision complexity |
| Validation | Automated asset, traversal, visibility, collision, and performance gates | Extend Copaimo's existing tests with plan/connectivity/camera/evidence checks |

The quality bar should be AAA in these areas:

- every building has a reason for its shape;
- the player can infer where to enter and where circulation continues;
- exterior and interior facts agree;
- reusable modules do not erase hierarchy or local identity;
- invalid procedural results fail loudly or fall back safely;
- performance claims are measured in the actual representative town/city views;
- hero exceptions are intentional rather than generator accidents.

The scope should remain indie in these areas:

- only selected interiors are playable;
- only spaces supporting play, story, service, or identity are modeled;
- templates and controlled variants precede a universal planner;
- high floors of towers can remain implied;
- rooms do not need dozens of unique props;
- room streaming and portal systems wait until whole-interior spawning is proven too expensive.

---

## 4. Treat a building as five coupled systems

A good building is not one mesh. It is five descriptions that must agree.

### 4.1 Program

The program states the building's purpose and users:

- residence, shop, guild hall, civic building, workshop, storage, office lobby;
- public, staff, resident, delivery, secure, and utility access;
- activities that occur there;
- required rooms and optional rooms;
- expected occupancy and gameplay;
- which spaces must be visible or reachable.

Program should determine plan and prominence. A guild hall needs a legible public threshold, reception/assembly space, controlled access to private/service areas, and vertical emphasis. A cottage needs a domestic hearth and sleeping/storage relationship. A shop needs a street-facing commercial room, a threshold display, a counter relationship, stock access, and preferably a service/back route if its footprint supports one.

### 4.2 Spatial plan

The plan is a graph before it is rectangles:

- nodes are rooms, halls, stairs, landings, exterior courts, and service areas;
- edges are doors, open passages, stairs, hatches, and exterior approaches;
- node properties include function, area range, privacy, daylight need, floor, and furnishing anchors;
- edge properties include width, visibility, lock/state, direction, and importance.

This ordering is supported by procedural floor-plan research: designer-controlled adjacency, area, reachability, and connectivity constraints are more valuable than unconstrained geometric novelty.

### 4.3 Structure and envelope

The envelope is the exterior shell and its construction logic:

- foundation/plinth and relation to terrain;
- load-bearing rhythm or implied frame;
- floors and roof support;
- front, side, back, corner, and roof conditions;
- openings derived from room needs;
- drainage, chimneys, vents, service entries, balconies, and additions.

Even in stylized work, plausible support matters. A heavy stone upper volume on tiny glass piers reads as an error unless the fiction clearly explains it. Timber bays should stack or visibly transfer load. Roof ridges, eaves, gutters, and chimneys should respond to the volume beneath them.

### 4.4 Experience

The building must communicate through movement and view:

- the approach frames an entrance;
- the threshold announces a state change;
- the first interior view gives orientation;
- circulation has a hierarchy;
- important destinations gain light, height, contrast, or axial position;
- private/service spaces recede from public circulation;
- repeated building families share wayfinding conventions.

### 4.5 Runtime representation

The same semantic building may have several representations:

- map/plan footprint;
- distant silhouette or block proxy;
- exterior shell;
- closed window treatment;
- enterable shell plus active interior;
- collision proxies;
- optional room cells, lights, interactables, audio zones, and navigation data.

Do not make the visual scene root the only truth. The semantic description should exist separately enough to validate entrances, rooms, portals, and budgets.

---

## 5. Exterior design: from lot to roof

### 5.1 Start with the site and approach

Buildings should respond to the lot rather than merely occupy it.

For every placed building, resolve:

1. **Street address:** which edge is public frontage?
2. **Approach path:** how does the player move from road or square to threshold?
3. **Ground transition:** paving, packed soil, steps, ramp, stoop, porch, forecourt, or garden path.
4. **Side access:** is there an alley, yard gate, delivery path, or no access?
5. **Rear condition:** garden, service yard, refuse/storage, loading, blank boundary, or another street.
6. **Topographic response:** stepped base, skirt, retaining edge, drainage, or footprint rejection.
7. **Neighbor response:** party wall, setback, shared court, firebreak, corner emphasis, or freestanding silhouette.

The entrance should not simply be the central door bay because the façade helper defaults to it. Centered doors suit formal halls and some cottages; corner shops, attached townhouses, workshops, and modern lobbies often need different entry logic.

Copaimo already orients doors toward streets and provides doorsteps. Extend that contract so the lot knows the path from street to clear threshold and can validate that yard objects, walls, signs, trees, and street furniture do not obstruct it.

### 5.2 Give every building a massing sentence

Before detail, describe the mass in one sentence. Examples:

- “Low one-room cottage under a roof larger than its walls, with a warm front and working rear.”
- “Narrow two-storey townhouse, public at ground, private above, with a rear stair volume.”
- “Shop gable addressing the street, display bay on one side, service lean-to behind.”
- “Stone civic hall with a broad public base and a tower marking the town from afar.”
- “Modern mid-rise with a transparent public base, repetitive occupied shaft, and mechanical crown.”

If the generator cannot state the sentence from its parameters, it is probably combining features without hierarchy.

Use a three-scale read:

- **Large:** footprint, height, roof, tower, wing, setback; readable at settlement approach.
- **Medium:** porch, bay window, arcade, buttress, dormer, canopy, balcony, service volume; readable from the street.
- **Small:** frame, mullion, sill, bracket, sign, hardware, seam, shingle course; readable near the threshold.

Do not compensate for a weak large shape with more small parts.

### 5.3 Model a front, sides, back, and top—not four interchangeable walls

Real buildings distribute attention and money unevenly.

**Front:** entrance, public identity, signage, display, best trim, social windows.  
**Sides:** structural rhythm, fewer openings near neighbors, downpipes, secondary entries, additions.  
**Back:** service door, stock, refuse, yard, vents, irregular extensions, cheaper material, repair history.  
**Corners:** either turn the façade deliberately or terminate it; do not let trim collide or stop arbitrarily.  
**Roof:** silhouette, drainage, chimneys/vents, access, mechanical/service logic, later additions.

This asymmetry is one of the cheapest ways to make a procedural building look inhabited. It is also more useful than random window omission because it follows program.

### 5.4 Make the façade answer the floor plan

The façade and plan need a two-way contract:

- a window belongs to a named room or circulation space;
- its sill and size suit that room's use and privacy;
- a door connects two valid spaces and has maneuvering clearance;
- a stair landing does not cut across a window at ankle height;
- a chimney aligns with a hearth, stove, or service riser;
- a shop display opens to the sales floor, not a bed or stair;
- a repeated office window band corresponds to a believable floor zone;
- a blank wall has a reason: party wall, service core, storage, structure, or defense.

Generation order should therefore be:

`program → footprint → rooms/core → candidate openings → façade composition → dressing`

not:

`shell → evenly spaced windows → interior partitions wherever they fit`

CGA shape grammar remains useful, but the façade grammar should consume semantic opening requests from rooms. It may regularize those requests into bays; it should not invent windows independently.

### 5.5 Use base, field, and termination at every scale

Copaimo already applies base–shaft–top to towers. Use the same logic more broadly:

- **Building base:** foundation, plinth, storefront, arcade, stoop, heavier material, splash/wear zone.
- **Wall field:** repeated bays, frame, panels, windows, occupied floors.
- **Termination:** eave, cornice, parapet, gable, roof, crown.

Individual windows also benefit from surround–field–termination: sill/base, pane/field, lintel/head. Doors benefit from threshold, opening/leaf, lintel/canopy/sign. This creates nested hierarchy without adding noise.

### 5.6 Variation should operate on causes, not arbitrary parts

Good procedural variation changes one of these causes:

- age or renovation;
- wealth and maintenance;
- climate exposure;
- corner versus mid-block lot;
- mixed use versus residence;
- public importance;
- construction family;
- household or business activity;
- extension history.

Then several visible effects change together. A poorer/older cottage may have a patched roof, smaller repaired panes, uneven but still structural framing, a lean-to addition, and less formal forecourt. Randomly choosing each of those independently produces visual noise and contradictions.

Use **correlated style tokens**, for example:

`old_repaired`, `prosperous_shop`, `civic_formal`, `industrial_service`, `modern_refit`.

Each token should influence a compatible bundle: materials, symmetry, trim density, window family, wear, additions, exterior props, and lighting state.

### 5.7 Make additions read as time

An indie-friendly way to get complexity is to start with a clear primary volume and add at most one or two subordinate volumes:

- porch;
- rear lean-to;
- stair tower;
- shopfront insertion;
- dormer;
- side workshop;
- rooftop plant room;
- modern canopy or service bay.

Subordinate additions should differ slightly in roof, material, alignment, or age while still joining cleanly. This produces a building with history at much lower cost than a wholly unique model.

### 5.8 Semi-cel-shaded exterior treatment

The detailed outline research already exists in `BUILDINGS_TOWNS_CITIES_AND_OUTLINES_RESEARCH.md`; retain that as the rendering reference. For building design, the critical placement rules are:

- strongest ink at outer silhouette, roofline against sky, major overlap, large depth break, and the public entrance;
- medium ink at major structural divisions, eaves, base/shaft/top changes, deep reveals, buttresses, and important window groups;
- light or no ink on every repeated seam, shingle, brick, mullion, and coplanar trim edge;
- do not outline the far side of transparent glass as if it were an opaque slab;
- avoid identical black weight around every window in a tower—the result becomes a grid texture rather than architecture;
- break or soften ink where strong daylight washes an edge; reinforce it where planes overlap or fall into shadow;
- prefer near-black family colors over absolute black where the palette needs atmosphere, while keeping the silhouette value stable enough to read;
- give old and modern families different secondary line vocabularies: irregular structural framing and eaves for old buildings; cleaner slab, mullion-group, canopy, and crown lines for modern ones.

Outlines should reveal **form and hierarchy**, not inventory every polygon.

---

## 6. Interior design: plan before props

### 6.1 Decide why the player enters

Every enterable building should answer at least one of these:

- core service: shop, guild, exam, mission, crafting, rest;
- narrative or character encounter;
- exploration/reward;
- route or shortcut;
- shelter, combat, stealth, or traversal;
- landmark interior that delivers the promise of the exterior.

If a building supports none of them, a good closed façade is usually the stronger use of budget. “Enterable” is not inherently higher quality if the interior is empty, repetitive, or confusing.

### 6.2 Use public–private–service gradients

Most believable plans are not arbitrary room collections. They organize access.

**Public:** threshold, lobby, shop floor, guild hall, waiting, reception.  
**Semi-private:** meeting room, dining, workshop, consultation, stairs to controlled floors.  
**Private:** bedroom, office, records, staff room, household storage.  
**Service:** stockroom, kitchen/work area, delivery, refuse, utilities, vertical core.

A simple graph should generally move from public to increasingly private spaces, while service access can connect street/yard to service spaces without crossing the public center unnecessarily.

For gameplay readability, the first room should usually reveal either the destination or the route toward it. Do not make the player enter into a blind dead-end vestibule unless suspense or security is the point.

### 6.3 Establish a circulation grammar per family

The family grammar is more important than per-building novelty. Suggested starting rules:

#### Old village/town family

- Main door opens to a recognizable hearth/public zone or short passage—not into furniture.
- The hearth or primary work surface becomes an orientation anchor.
- In two-storey buildings, stairs occupy a consistent rear-side zone and are visible or inferable from entry.
- A rear/service door, when present, continues toward a yard rather than a wall or prop cluster.
- Private sleeping/storage space lies beyond or above the public domestic/work zone.
- Roof access/attic is consistently located relative to the stair.

#### Modern city family

- Main entrance opens into a clear lobby sightline.
- Reception/security is visible from the threshold but does not block the route.
- Lift/stair core occupies a consistent back or side band.
- Service access is visually distinct and avoids the ceremonial frontage.
- Public route uses brighter values, taller openings, and stronger ceiling/light rhythm than staff/service space.
- Upper occupied floors may be implied; the lobby must not promise an elevator interaction that does nothing unless that fiction is intentionally closed.

Consistency lets the player transfer knowledge. Visual details, room proportions, and secondary routes can still vary.

### 6.4 Use room adjacency graphs

Suggested room graphs use `—` for required connection and `...` for optional/secondary connection.

#### Cottage

`street/stoop — common room — sleeping alcove or bed zone`  
`common room — hearth/work zone`  
`common room ... pantry/storage — rear yard`

The smallest cottage can remain one room, but it should contain zones rather than scattered props. The doorway-to-hearth path, hearth work clearance, bed privacy edge, and storage edge should be legible.

#### Townhouse

`street — entry/common or shop room — rear/service`  
`entry/common — stair — upper landing — private rooms`  
`rear/service — yard`  

Stairs should be reserved before upper rooms are partitioned. A stair squeezed into leftover space is the most common procedural-plan tell.

#### Shop

`street/display — sales floor — counter boundary — stock/work`  
`stock/work — service door — yard/alley`  
`sales floor ... stair — private upper room`  

The counter is a social/functional boundary, not a wall. Preserve a clear customer side, staff side, and route around/through it where appropriate.

#### Guild hall

`square/forecourt — ceremonial threshold — reception/assembly`  
`assembly — mission/exam/service point`  
`assembly — controlled stair — offices/records`  
`assembly ... meeting/training room`  
`service entry — storage/preparation — hall`  

The first view should deliver the institution: emblem, long axis, hall volume, desk/dais, or vertical glimpse. Do not make the building's exterior landmark status lead into a cottage-sized room.

#### City lobby

`street/canopy — vestibule/lobby — reception/security — core`  
`lobby ... public service/tenant room`  
`service street/alley — service corridor — core`  

The core can be a destination/set piece without making all tower floors playable. An intentionally inactive lift bank should read as part of the world, while any active control needs an honest outcome.

### 6.5 Gameplay metrics for Copaimo

Real regulations are useful reality anchors, not automatic game metrics. The 2010 ADA standards cite a 915 mm minimum clear walking route, an 815 mm minimum clear doorway in specified cases, and 2030 mm vertical clearance. Those are physical accessibility minima. Epic's current third-person blockout guidance recommends substantially larger game spaces—roughly 2–3 m wide and 3–4 m tall for halls—and explicitly warns that a realistic 0.8 m doorway may feel cramped or clip a third-person camera. Epic also suggests starting around 1.5× first-person scale and then playtesting.

Copaimo's existing 1.9 m doorway and 3.6 m storey are therefore defensible. Suggested **prototype metrics**, all to be verified in actual play, are:

| Element | Copaimo prototype target | Reason |
|---|---:|---|
| Main clear doorway | keep 1.9 m × 2.45 m | Existing traversal/camera contract |
| Secondary interior door | 1.5–1.9 m clear | Aligns to module; do not shrink below proven camera needs without testing |
| Primary hall/circulation | 2.4–3.0 m clear | Camera, passing, readable direction |
| Secondary passage | 1.8–2.25 m clear | Intimate but usable; widen at turns |
| Door landing/decision node | about 3.0 × 3.0 m clear | One full module around route decisions; accommodates camera swing |
| Stair clear width | 1.8–2.4 m | Character plus camera; exact rise/run must be playtested |
| Stair/landing headroom | at least current door height, preferably more | Prevent camera/outline clipping under stairs |
| Main room short dimension | generally 4.5 m or more | Three modules gives furniture perimeter plus central route |
| Primary route through furnished room | 1.8–2.4 m continuous | Avoid snagging and collision ambiguity |
| Clear interaction pad | at least 1.5 × 1.5 m, larger for camera-facing interaction | One module provides a predictable anchor |
| Threshold level change | keep near current 0.12 m treatment | Existing walkable entry solution |

These are not laws and should not become magic constants scattered through code. Put them in one building-metrics description, visualize them in debug views, and test them with the actual player controller and camera.

### 6.6 Reserve circulation before assigning rooms

The safe generation order for interiors is:

1. place main entrance and clear arrival zone;
2. reserve vertical core/stairs for all required floors;
3. reserve primary circulation spine or loop;
4. place required rooms against that circulation;
5. place service/back route;
6. divide remaining area among optional rooms/storage;
7. cut doors and validate full reachability;
8. request windows from room needs;
9. furnish around protected paths;
10. validate camera and collision in the final geometry.

Do not generate rooms first and then attempt to thread a hall or stair through the leftovers.

### 6.7 Prefer a loop or through-route where it serves play

Dead ends are appropriate for bedrooms, records, storage, and rewards. They are weaker as the dominant public circulation pattern.

A shop can have a short customer loop around a central display. A guild hall can connect entry, service point, hall, and side room before returning to the main axis. A modern lobby can lead visibly to core and side service. A rear door can make a building part of town traversal.

Do not force loops into tiny cottages. The rule is to avoid accidental dead ends, not to make every house a racetrack.

### 6.8 Furnish by functional cluster and anchor

Furniture research treats arrangement as a constraint problem involving accessibility, visibility, pairwise relationships, and clear door-to-door paths. For Copaimo, a full optimizer is unnecessary. Encode a small set of clusters:

- **Hearth cluster:** hearth/fireplace, work clearance, seat or tools, chimney alignment.
- **Sleep cluster:** bed head against protected wall, access along at least one side, chest/storage nearby, not in the main path.
- **Meal cluster:** table, seats, approach clearance, relation to hearth/kitchen.
- **Shop counter cluster:** counter facing customer zone, staff clearance behind, stock relation, queue/interaction pad in front.
- **Display cluster:** visible from street/entry, does not narrow required path.
- **Guild service cluster:** desk/dais/board/emblem, audience interaction pad, staff route.
- **Lobby cluster:** reception/security facing entrance, waiting edge, clear route to core.
- **Storage cluster:** shelves/crates against service walls, with an aisle rather than random gaps.

Each cluster should declare:

- allowed room types;
- preferred wall/corner/window/door relation;
- footprint and collision footprint;
- interaction side;
- protected clearance polygon;
- sightline preference;
- incompatible neighbors;
- optional decorative satellites.

Generation then chooses a valid anchor and places the whole relationship. It should not scatter a table, chairs, shelves, and crates independently.

### 6.9 Separate visual clutter from collision

Use at least three prop classes:

1. **Gameplay solid:** furniture or structure the player reads as substantial and routes around; needs honest, simple collision.
2. **Soft/forgiving:** small chairs, baskets, cloth, papers, minor crates; either no collision or a deliberately forgiving aggregate volume.
3. **Surface detail:** dishes, books, tools, signage, wall dressing; no player collision.

If a waist-high table is non-colliding, walking through it looks broken. If every stool and basket has exact collision, navigation becomes sticky. Choose based on visual promise and route importance.

Keep dense clutter on perimeter surfaces and dead-end story pockets. Keep primary routes visually cleaner. Use clutter to show occupation, not to fill empty floor.

### 6.10 Tell stories through state, not quantity

A convincing room needs a small number of related signs of activity:

- one task in progress;
- one storage system;
- one ownership/identity clue;
- one maintenance or age clue;
- one link to exterior context.

Examples:

- a shop counter with wrapped goods, open stock shelf, delivery crates at the rear door, and a street sign matching the trade;
- a cottage hearth with cooking tools, wood storage near the rear, one repaired chair, and garden produce from the lot outside;
- a guild hall with mission board, records near staff access, banners that continue exterior identity, and worn floor along the public route;
- a lobby with reception, directory, controlled core, cleaning/service cart near the service route, and tenant identity in window/sign rhythm.

This is more legible and cheaper than random prop density.

### 6.11 Interior lighting is navigation plus atmosphere

Build interior lighting in layers:

1. **Daylight:** windows and doors establish orientation and time; avoid making every pane an equal luminous rectangle.
2. **Motivated practicals:** hearth, lamp, ceiling fixture, sign, desk light. The visible source and illumination should agree.
3. **Navigation emphasis:** slightly brighter destination, stair, exit, service counter, or important interaction.
4. **Separation:** enough value difference between floor, walls, furniture, and doorway that cel shading does not collapse them into one dark mass.
5. **Night exterior read:** selected occupied windows communicate life and type; emissive appearance does not require every window to own a shadow-casting point light.

Copaimo already measured shadow rendering as a major frame cost. Therefore:

- do not attach a shadowed point light to every lit window;
- prefer a small number of important local lights;
- keep ranges tight enough to avoid hard cutoff appearing in visible open space;
- reserve shadows for lights where contact and occlusion materially improve the scene;
- share a building/room lighting state so exterior pane, interior practical, and occupancy agree;
- test noon, dusk, and night separately, because a good noon interior can become unreadable at night and a good night window can look self-illuminated in daylight.

Use light to reveal path and hierarchy, not merely to make every surface visible. The GDC “Invisible Intuition” material is a useful design reference: blockmesh, environment art, effects, audio, and lighting can guide players without a HUD marker.

### 6.12 Interior ink and cel shading

Interior outlines need different priorities than exteriors:

- strongest: doorway silhouette, foreground occluders, stair profile, major furniture overlap, character contact/readability;
- medium: room corners with meaningful depth, beams, counter edge, hearth opening, door/window reveal;
- weak/none: every wall-floor seam, every shelf item, every chair spindle, coplanar trim, dense background mullions;
- reduce line density in dark rooms, where black edges merge into shadow;
- use material/value separation for adjacent planes before adding more black lines;
- ensure important interactables have a stable silhouette or local contrast without making every prop equally outlined;
- watch camera-near inverted hulls or shell outlines, which can expand dramatically and clip through walls in confined rooms.

The interior should remain graphic and calm. Exterior skylines tolerate stronger silhouette ink; a small room filled with equally heavy edges becomes visual static.

### 6.13 Camera behavior is part of the floor plan

Every plan must be tested with:

- entry through the door while approaching off-center;
- a full camera orbit in the arrival room;
- backing through a doorway;
- turning at the bottom/top of stairs;
- standing near walls and large furniture;
- moving from bright exterior to darker interior;
- seeing through multiple aligned doorways;
- roof/ceiling obstruction and camera collision.

Possible future camera remedies—only if testing shows need—include closer indoor follow distance, softer collision push-in, selective roof/upper-wall fade, or cutaway groups. Do not design literal real-world-sized rooms and expect the camera system to repair them.

---

## 7. A procedural architecture Claude can implement incrementally

The following is a data/design recommendation, not a request to write all systems at once.

### 7.1 Semantic source data

A building description should eventually carry concepts equivalent to:

```text
BuildingProgram
  archetype                cottage | townhouse | shop | guild_hall | city_lobby
  family                   old_town | modern_city
  public_importance
  enterability             closed | lobby_only | full_selected_floors
  required_rooms[]
  optional_room_sets[]
  circulation_grammar
  exterior_style_token
  runtime_tier

RoomSpec
  id, kind, floor
  area_range, min_width, aspect_range
  privacy                  public | semi_private | private | service
  daylight_need
  exterior_edge_preference
  required_neighbors[]
  forbidden_neighbors[]
  furnishing_clusters[]

PortalSpec
  from, to
  kind                     open_passage | door | stair | hatch | exterior
  clear_width
  visual_priority
  state                    always_open | closed_visual | interactive | locked

FacadeRequest
  room_id, exterior_side
  kind                     main_door | service_door | display | window | vent
  preferred_bay
  privacy, prominence

RuntimeRepresentation
  distant_proxy
  exterior_scene
  closed_interior_mask
  interior_scene_or_groups
  collision_groups
  light_groups
```

Names and exact Rust layout can differ. The architectural boundaries are the important part.

### 7.2 Staged generation pipeline

Each stage should produce inspectable data and may reject the result:

1. **Choose program:** based on building type, district, lot, and authored importance.
2. **Choose validated template:** not a blank random graph.
3. **Fit footprint:** stretch only allowed axes by full modules; preserve minimum widths.
4. **Place entrance and vertical core:** satisfy street and all-floor access.
5. **Instantiate room graph:** required rooms first, compatible optional set second.
6. **Allocate room rectangles/zones:** maintain area, width, aspect, exterior-edge, and adjacency constraints.
7. **Create circulation and portals:** prove reachability from entrance and service routes.
8. **Generate structural shell/floors/roof:** floors align to inside wall faces; reserve stair openings.
9. **Resolve façade requests into bays:** regularize without breaking room meaning.
10. **Apply architectural family:** compatible window/door/eave/cornice/roof modules.
11. **Place furnishing clusters:** protect circulation, interaction pads, and door swings.
12. **Build collision/visibility/light groups:** derived from semantic elements, not manually guessed later.
13. **Validate:** plan, geometry, camera, path, art, runtime cost.
14. **Fall back:** mirrored template → simpler optional set → base template → closed variant. Never silently ship a broken plan.

This follows the best ideas from the procedural literature while limiting the solution space to something an indie project can tune.

### 7.3 Start with template transforms, not generated topology

Allowed early transformations:

- mirror left/right if street and neighboring lot permit;
- stretch a room or façade by one or more complete 1.5 m bays;
- swap compatible optional rooms;
- move a secondary partition along module lines within min/max limits;
- choose one of a few stair positions already supported by the template;
- choose front/side/rear opening patterns derived from room tags;
- add one compatible subordinate volume;
- vary furnishing clusters within protected anchor zones;
- apply correlated age/wealth/activity style token.

Disallowed early transformations:

- arbitrary room graph mutation;
- stairs placed after rooms;
- random doors between rooms;
- window generation independent of rooms;
- per-prop random scatter;
- arbitrary non-module scaling of authored door/window modules;
- unbounded retries until something happens to fit.

### 7.4 Stable, separated randomness

Use independent deterministic random streams or hashes for:

- plan variant;
- massing/addition variant;
- façade/style token;
- furnishing cluster variants;
- occupancy/light state;
- minor dressing.

A new chair variant must not change the stair side, façade, or whether the shop has a stockroom. Seed each choice from stable building identity plus a named channel, not enumeration order.

### 7.5 Author overrides are part of the system

AAA procedural tools succeed when artists can intervene. SideFX's Building Generator and Embark's Building Creator both emphasize staged features and custom modules/overrides. Copaimo's equivalent can be simple data:

- force a plan template;
- reserve a landmark axis or room;
- override a façade bay;
- mark a wall blank;
- replace a generated prop cluster;
- force an entrance or stair side;
- add a hero volume;
- close a building despite an archetype normally being open.

An override should change a semantic decision and let downstream stages rebuild. Avoid final-geometry patches that the next generator run erases.

---

## 8. Exterior/interior contracts by building type

### 8.1 Cottage vertical slice

**Exterior promise:** domestic, modest, one clear approach, oversized protective roof, hearth/chimney, working rear.  
**Interior:** common/hearth room plus sleeping/storage zones; optional pantry/alcove.  
**Contract checks:** chimney reaches hearth; front windows light common room; rear opening, if present, reaches yard; bed is not in entry path.  
**Variation:** gable orientation, porch, lean-to, repair token, flower/garden relation, left/right hearth where chimney remains plausible.  
**Do not:** create three tiny rooms just to claim a floor plan.

### 8.2 Shop vertical slice

**Exterior promise:** display, sign, public threshold, goods/activity near street, delivery clue at rear.  
**Interior:** sales floor, counter relationship, stock/work, optional upper/private access.  
**Contract checks:** display faces sales floor; clear entry-to-counter path; staff side connects to stock; service route does not cross customer interaction pad.  
**Variation:** trade cluster, counter position from validated choices, display width, side sign, awning/canopy, stock density.  
**Do not:** turn every shop into the same cottage with crates.

### 8.3 Townhouse vertical slice

**Exterior promise:** narrow frontage, stacked occupation, public/working ground floor, private upper level, rear life.  
**Interior:** entry/common or shop room, reserved stair, upper landing, one or two private zones, rear/service.  
**Contract checks:** upper windows belong to reachable floor; stair headroom; no landing through window; rear yard route; jettied upper floor still has a believable support/read.  
**Variation:** ground use, jetty/bay, stair side, rear addition, dormer, renovation token.  
**Do not:** model a visible second storey that has neither a reachable floor nor an intentional “not playable” contract.

### 8.4 Guild hall vertical slice

**Exterior promise:** town/city institution and landmark, formal threshold, identity visible at distance, public square relationship.  
**Interior:** ceremonial arrival, assembly/service point, controlled private/service branches, stair/vertical hint matching tower.  
**Contract checks:** entry view delivers the hall; emblem/banner repeats exterior identity; public route does not end behind furniture; tower/core relationship is spatially plausible; room volume matches exterior importance.  
**Variation:** mission/exam configuration, side rooms, meeting/training option, civic furnishing state, banner/event state.  
**Do not:** fill the grand hall with evenly scattered tables or make it visually identical to a house.

### 8.5 City lobby vertical slice

**Exterior promise:** transparent/taller base, canopy, address, occupied shaft, finished crown.  
**Interior:** vestibule/lobby, reception/security, core, optional public service room, separate service suggestion.  
**Contract checks:** core aligns through shaft; front glazing reveals lobby rather than empty void; exterior lit-window logic agrees with occupancy; service core corresponds to blanker façade/roof plant; active/inactive lift state is honest.  
**Variation:** core side, lobby material/value scheme, tenant identity, desk configuration, canopy, public room.  
**Do not:** generate dozens of inaccessible but visually promised upper interiors.

---

## 9. Runtime and performance strategy

### 9.1 Use explicit representation tiers

Suggested tiers:

| Tier | What exists | When |
|---|---|---|
| Distant | silhouette/proxy, coarse materials, no interior, simplest collision or none | skyline/approach |
| Street | full exterior, closed-window backing, exterior collision, limited exterior lights | normal town visibility |
| Near enterable | exterior plus interior shell/furnishing/collision; important local lights | near selected open building |
| Occupied/hero | interactables, characters, audio, special lighting/state, detailed collision only where needed | player inside or active mission |

The current “whole glTF scene per building” is a reasonable baseline. The first useful split is **exterior versus interior**, not individual room streaming. Per-room cells/portals only become worthwhile after profiling shows that several simultaneously loaded interiors are a real bottleneck.

### 9.2 Closed buildings need honest depth blockers

Closed variants should avoid windows revealing an empty shell or the world behind them. Options include:

- opaque/dark glass treatment;
- shallow interior card or curtain/blind layer;
- limited parallax-like backing geometry where worth the cost;
- occupancy color/emissive state without a real room;
- silhouettes of a few large interior planes, not full furniture.

Avoid a uniformly black pane in daylight and avoid every pane glowing at night. Window state should be patterned by floor/room/occupancy and correlated within a building.

### 9.3 Group meshes with culling in mind

One mesh per entire building reduces draw calls but makes interior visibility coarse. Thousands of independent pieces improve culling but increase entity/draw overhead. Use meaningful groups:

- exterior opaque shell;
- roof/upper cutaway group if needed;
- glazing/transparent group;
- interior architectural group per floor or major zone;
- static furnishing aggregate per room/floor;
- interactables separate;
- collision proxies separate from visual detail;
- lights and effects separate.

Group by **visibility and interaction behavior**, not by modeling convenience.

Epic's HLOD guidance explains the general principle: distant groups can be replaced by combined proxy meshes/materials to reduce object and draw-call cost. Bevy's `VisibilityRange` can support distance representation changes, but exact behavior must be verified against the project's Bevy 0.16 API. Do not design around latest-version documentation without checking 0.16.

### 9.4 Interior visibility

Potential progression:

1. Keep interiors absent for closed buildings.
2. Spawn/enable an enterable interior only inside a near radius.
3. Disable interior when sufficiently far and no camera/player is inside.
4. If profiling justifies it, divide large hero buildings into floor/room cells connected by door portals.
5. Determine visible cells from the camera cell plus open portals; keep a conservative margin to prevent popping.
6. Maintain separate collision/AI activation rules; rendering invisibility must not automatically delete gameplay state.

Avoid attempting portal culling before the building has stable semantic rooms and portals. The graph is the prerequisite.

### 9.5 Collision and navigation

Continue Copaimo's strong practice of testing actual traversal contracts. Add:

- architectural collision from simple wall segments, floors, stairs, counters, and large furniture;
- visual-only small detail;
- explicit doorway gaps from portal data;
- no invisible blocker in an opening the art presents as passable;
- no non-colliding major object the art presents as solid;
- traversal tests through every required portal and up/down every required stair;
- future navigation built from the final validated walkable geometry or semantic rooms/portals.

Recast/Detour is a useful future reference because it separates navmesh generation, runtime queries, tiled streaming, and crowds. It is not a recommendation to add that dependency now. The immediate value is conceptual: agent radius, step, slope, headroom, and portal width must be validated from the same geometry the player sees.

### 9.6 Budget from evidence, not folklore

Do not copy universal triangle, draw-call, light, or room-count budgets from another engine/platform. Establish a Copaimo measurement scene containing:

- one village approach and center;
- one city approach and center;
- several nearby open interiors;
- representative noon/dusk/night lighting;
- worst expected window/outlining density;
- normal player-height and high-overview views.

Record:

- frame time split, not only FPS;
- visible entities/meshes/material groups;
- triangles/vertices submitted;
- shadow pass cost;
- light count and overlapping ranges;
- asset memory and scene load time;
- interior activation spikes;
- collision query cost;
- generation/export time.

Then define budgets from the slowest supported hardware and leave headroom for characters, effects, UI, missions, and weather.

---

## 10. Validation and evidence gates

### 10.1 Semantic plan tests

- every required room exists exactly once or within allowed count;
- every playable room is reachable from a valid entrance;
- every promised upper floor is reachable by stair/lift/hatch logic;
- privacy gradient has no accidental public route through a bedroom/stock/service dead end;
- required adjacencies share a valid portal, not merely a wall;
- forbidden adjacencies do not occur;
- main entrance reaches the primary public space;
- service door reaches service space;
- no route depends on passing through a closed/static portal;
- room width, area, and aspect stay within template limits.

### 10.2 Exterior/interior contract tests

- every door connects valid spaces and has clear approach on both sides;
- every exterior window belongs to a room/floor or an explicit blind/spandrel condition;
- stair/landing does not intersect a window, beam, roof, or ceiling;
- chimneys/vents/cores align with their source through floors and roof;
- façade floor bands match actual floor heights;
- storefront display belongs to sales/public space;
- closed buildings cannot reveal missing interior geometry;
- roof addition/crown has support/service logic.

### 10.3 Traversal and camera tests

- walk from street to primary destination without jumping or snagging;
- walk through every required door at center and off-center approaches;
- traverse furniture route at normal and sprint speed;
- ascend/descend stairs while rotating camera;
- back out of small rooms;
- rotate camera at every decision node and landing;
- verify camera cannot see through roof/wall gaps or become trapped;
- verify collision gap and visible gap are the same;
- verify important interactable has usable approach and camera view.

### 10.4 Procedural robustness tests

- many seeds for every footprint class;
- tiny/large, corner/mid-block, steep/flat, mirrored lot cases;
- deterministic regeneration;
- adding a furnishing variant does not change plan/facade channels;
- invalid plan fails with a named reason;
- fallback reaches a known valid base template;
- no unbounded retry loop;
- exporter audit verifies required named groups/metadata/files;
- old and modern families cannot accidentally mix incompatible modules.

### 10.5 Visual review matrix

For each vertical-slice archetype, capture fixed-condition evidence:

- settlement approach silhouette;
- street approach at player height;
- front three-quarter;
- rear/service three-quarter;
- threshold looking in;
- entry looking back out;
- primary room from arrival;
- circulation decision/stair;
- night exterior and night interior;
- overhead plan/debug view with room names and protected paths;
- collision/path overlay;
- distant/near representation transition.

Judge each shot against a named claim. Example: “From the shop threshold, the counter, staff boundary, and route deeper inside are immediately readable.” This is stronger than “looks good.”

---

## 11. Recommended Copaimo rollout

### Phase 0 — Measure and expose current truth

- Document exact current cottage, townhouse, shop, guild hall, and lobby dimensions.
- Add debug views or exported diagrams showing floor, walls, openings, collision, player capsule/footprint, and camera clearance.
- Measure current building/interior cost in representative village/city views.
- Keep existing open/closed behavior intact.

**Exit criterion:** current strengths and failures are visible; no design work relies on guessed scale.

### Phase 1 — Introduce semantic templates without changing the look broadly

- Define room/zone, portal, entrance, and furnishing-anchor concepts.
- Express the existing cottage and shop as semantic templates.
- Generate the current or nearly current geometry from those templates.
- Add reachability, opening, and protected-path validation.

**Exit criterion:** the same building can explain why every door, window, and major prop is where it is.

### Phase 2 — Three-building vertical slice

Build and visually review:

1. cottage—domestic zoning and exterior/interior chimney contract;
2. shop—public/staff/service organization and readable interaction;
3. guild hall—landmark promise fulfilled by interior arrival and circulation.

Give each two or three validated variants, not dozens.

**Exit criterion:** variants feel related but not cloned; learning one helps navigate the others; all pass camera/traversal tests.

### Phase 3 — Runtime split

- Separate closed shell, open exterior, and interior representation where profiling justifies it.
- Add honest closed-window backing/state.
- Activate interior groups by distance/occupancy safely.
- Keep gameplay state independent from rendering state.

**Exit criterion:** several nearby buildings no longer pay for invisible interiors, with no door/window popping.

### Phase 4 — Townhouse and city lobby

- Reserve/test stairs and upper-floor promise for townhouse.
- Formalize lobby/core/service logic for towers.
- Ensure city façade floor/window logic agrees with implied occupancy.

**Exit criterion:** vertical circulation and partial enterability are honest and legible.

### Phase 5 — Art and occupation pass

- Add correlated style tokens, subordinate additions, room clusters, wear/activity states, and family-specific line weighting.
- Extend night occupancy/window patterns from room states.
- Keep prop collision classified and paths protected.

**Exit criterion:** buildings show different lives and ages without random clutter or mixed architectural vocabulary.

### Phase 6 — Advanced procedural planning only if needed

Consider constrained room growth/optimization, room-cell visibility, or broad plan synthesis only after templates prove too restrictive. Preserve authored constraints and fallbacks.

**Exit criterion:** the advanced system solves a measured content problem and produces fewer manual corrections, not merely more novelty.

---

## 12. High-priority decisions for Claude

Before implementing a broad interior system, settle these explicitly:

1. Which five building archetypes are allowed to be enterable in the next milestone?
2. For each, what is the player's reason to enter?
3. Which floors are physically playable versus implied?
4. What circulation facts remain consistent across each architectural family?
5. What is the standard indoor camera behavior and tested clearance envelope?
6. Which props are solid, forgiving, or visual-only?
7. Is the first runtime split whole building versus shell/interior, or is current whole-scene cost already acceptable?
8. Which room/light/window states must agree at day and night?
9. What debug/evidence outputs prove plan, collision, and façade consistency?
10. What is the safe closed-building fallback when generation fails?

My recommended answers for the first pass are:

- enterable: cottage, shop, selected townhouse, guild hall, city lobby;
- templates: 2–3 per archetype;
- playable floors: only those with a complete circulation and camera contract;
- runtime split: exterior/closed backing/interior, not rooms yet;
- furnishing: anchor clusters plus a protected 1.8–2.4 m primary route;
- validation: graph reachability, doorway/stair traversal, exterior/interior contracts, fixed visual matrix;
- failure: named diagnostic, then base template or honest closed variant.

---

## 13. Failure patterns to avoid

- A façade generator and interior generator that never exchange semantic data.
- Evenly spaced windows regardless of room function.
- Stairs inserted after rooms and furniture.
- A second storey visible outside but inaccessible or geometrically absent inside without an intentional contract.
- Every building open, producing many empty/repeated interiors.
- Random room graphs whose novelty defeats player learning.
- Furniture scatter used to disguise an unplanned room.
- Exact collision on every minor prop.
- No collision on large furniture or fences that visually promise solidity.
- A grand landmark exterior leading into a tiny generic room.
- Symmetrical front, sides, back, and roof with no service life.
- Mixing old-town and modern-city modules because each looked attractive alone.
- Strong black outline around every window, seam, mullion, shingle, and prop.
- A shadowed point light behind every lit window.
- Distant interiors, props, collision, or lights active when the player cannot perceive them.
- Infinite procedural retries or silent invalid output.
- Randomness tied to iteration order.
- Optimizing triangles while ignoring entity, material, draw, shadow, loading, and collision cost.
- Polishing assets before the blockout has passed player/camera traversal.

---

## 14. Research basis and sources

The recommendations above synthesize primary research, official engine/tool documentation, and published production talks. Sources are linked so implementation claims can be checked rather than treated as folklore.

### Procedural city, building, floor-plan, and furnishing research

- Parish & Müller, **“Procedural Modeling of Cities”**—transport networks under goals/constraints, lot subdivision, and building generation: [paper PDF](https://cgl.ethz.ch/Downloads/Publications/Papers/2001/p_Par01.pdf)
- Müller, Wonka, Haegler, Ulmer & Van Gool, **“Procedural Modeling of Buildings”**—CGA shape grammar and hierarchical building-shell generation: [ACM DOI](https://doi.org/10.1145/1141911.1141931), [ETH record](https://www.research-collection.ethz.ch/handle/20.500.11850/36290)
- Lopes, Tutenel, Smelik, de Kraker & Bidarra, **“A Constrained Growth Method for Procedural Floor Plan Generation”**—designer-controlled room areas, adjacency, reachability, and connectivity: [TU Delft PDF](https://publications.graphics.tudelft.nl/rails/active_storage/blobs/redirect/eyJfcmFpbHMiOnsibWVzc2FnZSI6IkJBaHBBbGNPIiwiZXhwIjpudWxsLCJwdXIiOiJibG9iX2lkIn19--cc2327652b4f8ecf389aff080c98c40465574f6f/LTSDB10a.pdf)
- Merrell, Schkufza & Koltun, **“Computer-Generated Residential Building Layouts”**—residential plans based on architectural programs and layout optimization: [author paper](https://paulmerrell.org/floorplan-final.pdf)
- Spitaler, **“Procedural Generation of 3D Building-Interiors”**—room distribution, cross-floor relationships, stairs/elevators, furnishing, and façade rules: [TU Wien record](https://www.cg.tuwien.ac.at/research/publications/2015/spitaler-2015-pbi/)
- Yu et al., **“Make It Home: Automatic Optimization of Furniture Arrangement”**—accessibility, visibility, spatial relationships, and door-to-door paths in furnishing: [UCLA PDF](https://web.cs.ucla.edu/~dt/papers/siggraph11/siggraph11.pdf), [ACM DOI](https://doi.org/10.1145/1964921.1964981)

### Shipped-game and production practice

- Embark/SideFX, **“Making the Procedural Buildings of THE FINALS”**—fully traversable interiors, staged feature nodes, artist modules, automated collision/occluders, consistent wayfinding rules, and furniture reduction for clean traversal: [SideFX production article](https://www.sidefx.com/community/making-the-procedural-buildings-of-the-finals-using-houdini/)
- SideFX Labs, **Building Generator**—blockout-to-floors/walls/corners/ledges/modules with overrides: [official documentation](https://www.sidefx.com/docs/houdini/nodes/sop/labs--building_generator-4.0.html), [tutorial](https://www.sidefx.com/tutorials/building-generator/)
- Bethesda, **“Fallout 4's Modular Level Design”**—modular art kits and iterative design enabling a relatively small content team to build a large world: [GDC Vault](https://www.gdcvault.com/play/1022930/-Fallout-4-s-Modular), [slides](https://media.gdcvault.com/gdc2016/Presentations/Burgess_Joel_Modular%20Level%20Design.pdf)
- Fullbright, **“The Level Design of Gone Home”**—researching and constructing authentic spaces, with layout and decoration tied to player psychology and emotion: [GDC Vault](https://gdcvault.com/play/1022112/Level-Design-in-a-Day), [slides](https://media.gdcvault.com/gdc2015/presentations/Craig_Kate_LevelDesignOf.pdf)
- Cyarron, **“An Architect's Guide to Creating Expressive Game Environments”**—architecture in relation to context and inhabitants: [GDC Vault](https://www.gdcvault.com/play/1023257/An-Architect-s-Guide-to)
- Hosking, **“Architecture in Level Design”**—geometry, material, texture, color, light, and construction creating mood before literal symbolism: [GDC Vault](https://www.gdcvault.com/play/1023554/Level-Design-Workshop-Architecture-in)
- Shaver & Yang, **“Invisible Intuition”**—blockmesh and lighting to guide players and establish mood: [GDC Vault](https://www.gdcvault.com/play/1025179/Level-Design-Workshop-Invisible-Intuition), [slides](https://media.gdcvault.com/gdc2018/presentations/DShaver_Invisible_Intuition_GDC2018.pdf)

### Scale, access, wayfinding, runtime, and navigation references

- Epic Games, **Level Blockout / Determine Your Scale**—third-person camera scale, 2–3 m hall guidance, player references, modular blockout, and early playtesting: [official Unreal documentation](https://dev.epicgames.com/documentation/en-us/unreal-engine/designer-01-project-setup-and-level-blockout-in-unreal-engine)
- U.S. Department of Justice, **2010 ADA Standards for Accessible Design**—real-world clear route, doorway, turning, threshold, and headroom reference values: [official standards](https://www.ada.gov/law-and-regs/design-standards/2010-stds/), [guidance](https://www.ada.gov/law-and-regs/design-standards/standards-guidance/)
- U.S. GSA, **P100 Facilities Standards**—wayfinding as the organization of circulation paths, signs, and visual cues: [official P100 PDF](https://origin-www.gsa.gov/system/files/2018_P100_Final_5-7-19_compressed.pdf)
- Epic Games, **Hierarchical Level of Detail**—combining distant objects into proxy meshes/materials to reduce object and draw-call cost: [official documentation](https://dev.epicgames.com/documentation/en-us/unreal-engine/hierarchical-level-of-detail-in-unreal-engine)
- Epic Games, **Real-Time Rendering Optimization**—material IDs, mesh counts, draw calls, visibility, and measurement: [official documentation](https://dev.epicgames.com/documentation/unreal-engine/guidelines-for-optimizing-rendering-for-real-time-in-unreal-engine)
- Bevy, **`VisibilityRange`** and **`PointLight`** concepts—distance visibility/HLOD and light intensity/range/shadows. Check exact Bevy 0.16 APIs locally before use: [current VisibilityRange docs](https://docs.rs/bevy/latest/bevy/camera/visibility/struct.VisibilityRange.html), [current PointLight docs](https://docs.rs/bevy/latest/bevy/light/struct.PointLight.html)
- Recast Navigation—navmesh generation, Detour runtime queries, tiled streaming, and crowd separation: [official repository/documentation](https://github.com/recastnavigation/recastnavigation)

---

## 15. Condensed implementation brief

If only one page of this research is used, use this:

1. Preserve Copaimo's 1.5 m module, 1.9 m doors, 3.6 m storeys, real wall thickness, door/road/collision contract, open/closed variants, and old/modern families.
2. Add a semantic plan layer: program, rooms/zones, portals, privacy, vertical core, façade requests, furnishing anchors.
3. Begin with 2–3 authored, validated plans each for cottage, shop, selected townhouse, guild hall, and city lobby.
4. Reserve entrance, stair/core, primary route, and service route before partitioning rooms.
5. Derive exterior doors/windows/chimneys from interior needs, then regularize them into the façade grammar.
6. Keep family navigation consistent even when massing and decoration vary.
7. Furnish using functional clusters; protect a continuous camera/player path; classify collision by visual promise.
8. Use lighting and selective ink to express hierarchy. Do not light or outline every repeated element equally.
9. First runtime split: closed shell versus exterior plus whole interior. Add room streaming only after profiling.
10. Validate connectivity, exterior/interior agreement, actual player/camera traversal, deterministic fallbacks, and fixed-condition visual evidence.

The goal is not maximum procedural freedom. It is a town where every enterable building feels designed, every closed building feels honest, and the player can read purpose, route, history, and importance before inspecting the details.
