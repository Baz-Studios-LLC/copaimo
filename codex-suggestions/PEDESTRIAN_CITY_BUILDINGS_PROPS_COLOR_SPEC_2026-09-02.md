# Copaimo Pedestrian City Production Specification

**Audience:** Claude and future Copaimo contributors  
**Purpose:** design and implementation guidance only  
**Reviewed state:** commit `815498e`, including the 420-building city pass  
**Hard fiction rule from the user:** there are no cars; city traffic is pedestrian, with hoverboards and mounts belonging to later mobility  

---

## Executive diagnosis

The new building roster and higher population are meaningful improvements. The city is no longer only
three heights of the same tower. The remaining problem is not raw object count: it is that the city is
still composed like a vehicle-era business park.

The current aerials show a road network subdividing broad green fields, with buildings placed as separate
objects inside them. The current eye-level view shows wide, mostly empty corridors and large gaps between
frontages. This creates three readings that work against the desired fiction:

1. **The street has a vehicle-shaped centre with no vehicle-shaped reason to exist.** `CITY_STREET_WIDE`
   is documented as a 6 m carriageway plus two 2 m pavements, even though nobody drives.
2. **The buildings occupy plots but do not consistently make urban rooms.** A city feels real when walls,
   entrances, windows, arcades and shop displays continuously address the public route; isolated objects in
   lawn read as a campus or procedural test field.
3. **Props are individually plausible but insufficiently tied to people, doors and daily tasks.** A bench,
   planter, skip or kiosk becomes life evidence only when it has an owner, a user, an access path and a
   reason for its orientation.

AAA quality here means replacing vehicle assumptions and random decoration with a coherent pedestrian
urban grammar. More polygons cannot substitute for that grammar.

## 1. Preserve what is already strong

Do not restart the city system. Keep:

- the three plan families (`Rings`, `Grid`, `Spine`);
- the four city characters (`Capital`, `Works`, `Green`, `Trade`);
- market/crafts/outskirts districts;
- deterministic frontage-based lot placement;
- the new slab, shop, works and deck silhouettes as evidence that programme can shape massing;
- landmarks beside genuine nodes;
- the shared semi-cel material and restrained vertex-colour workflow;
- explicit collision, doorway, footprint and ground-level tests.

The next pass should make those systems visible at walking height, not add another independent generator.

## 2. Replace the “road plus sidewalks” model with a pedestrian street hierarchy

The absence of cars is an artistic advantage. Streets can be social spaces instead of leftover margins
around a roadway. Do not make every city street the same 8–10 m kerbed ribbon.

### Recommended hierarchy

| Type | Total useful width | Surface/edge | What happens there |
|---|---:|---|---|
| Civic promenade / market high street | 8–12 m | mostly continuous surface; drainage/planting bands rather than two vehicle kerbs | dense movement, stalls, queues, café spill-out, events, banners, wayfinding |
| Main pedestrian street | 5.5–8 m | continuous or subtly crowned surface | two-way walking, hoverboards at low speed, deliveries by handcart, shopfront activity |
| Residential street | 4–6 m | continuous shared surface, frequent thresholds and planting | walking, play, neighbours, stoops, small household activity |
| Service lane | 3–4.5 m | tougher/darker paving, central drain or side channel | handcarts, refuse, repair access, goods doors, hoists, utility cabinets |
| Alley / passage | 1.8–3 m clear | hard wearing, wall-to-wall | shortcuts, rear access, discoveries, compressed camera moments |
| Arcade / covered walk | 2.5–4 m clear | repeated columns and protected threshold | weather shelter, retail, strong light rhythm |

These are game-space targets, not building regulations. The player controller and camera remain the final
authority. Still preserve an unobstructed route: 2 m is a useful normal clear band, 1.5 m a constrained
minimum, and 1.2 m should be treated as the absolute narrow pinch rather than the typical path. The U.S.
Access Board's public-right-of-way guidance requires a 1.22 m continuous accessible route and wider passing
space; UK guidance recommends 2 m under normal circumstances. Copaimo can use those as believability
anchors while keeping the camera comfortable.

### What this changes visually

- In pedestrian cores, remove the fake distinction between carriageway and two pavements. Use one
  continuous plane with a **clear movement band** and **edge activity bands** expressed by paving scale,
  drainage, lamps, trees, canopies and furniture.
- Keep kerbs where they perform a believable job: retaining planted beds, draining a grade change,
  protecting a mount/hoverboard route, or separating a service lane. A kerb around every foot-only street
  carries the visual history of cars even when no car exists.
- Make the high street recognisable by frontage, light, signs and population—not only width.
- Do not put a decorative circular node at every crossing. Reserve elaborate paving, monument geometry and
  gathering space for important nodes. Ordinary intersections should be quiet enough that landmarks remain
  special.
- Add short mid-block passages. The current large blocks need pedestrian permeability; a walker should
  discover a shortcut every 35–60 m on dense routes rather than always walking to the next major node.

Global Designing Cities Initiative guidance supports continuous-surface shared streets specifically where
pedestrian activity is high and vehicles are discouraged, with seating, vendors, artwork and landscape
forming the public space. Copaimo can go further because its vehicle count is exactly zero.

## 3. Make blocks, not objects in fields

The city needs a **street wall** and a **block interior**.

### Street wall rules

- Market core: target 70–90% of frontage occupied by building face, arcade, stall, wall or deliberately
  designed square edge.
- Crafts: 60–80%, broken by loading courts and service lanes.
- Residential: 45–70%, broken by stoops, narrow gardens, passages and courtyard entries.
- Outskirts: 30–55%, where the city intentionally dissolves into landscape.

These are not building-count ratios. Measure occupied metres along a street. A 24 m depot or 22 m shop
parade should count by the frontage it actually creates.

Align neighbouring fronts to a district building line. Allow small controlled offsets—roughly 0.2–1.2 m,
not an independent setback roll per building—to create bays without dissolving the wall. Where buildings
touch, use party walls or a tiny maintenance seam; do not leave unusable two-metre grass slots.

### Block interior rules

Every dense block should answer all four sides:

- public fronts face primary streets and squares;
- secondary residential fronts can face quieter streets;
- service doors, waste, plant, loading and repair face a rear lane or court;
- private/semi-private life goes in an interior courtyard, not on a leftover lawn around every tower.

Use one coherent interior programme per block: shared garden, service court, workshop court, market yard,
cloister, play court, rain garden or tiny shrine. The interior should have one or two connected entrances
from the street so it is spatially understandable.

The UK National Design Guide explicitly connects successful streets to consistent building lines, party
walls, front/back distinction, street-facing entrances/windows and active ground floors. Those are exactly
the missing relationships in the current aerial reading.

### Enclosure and height

Check street width against the height of the wall that frames it. A 6–10 m space framed by 12–22 m buildings
can feel satisfyingly urban if the base is articulated and light reaches it. A 60 m spire belongs at a node
or terminated vista, not randomly beside every narrow route. Step tall buildings back above a 2–4 storey
street base when necessary so the player's view reads doors and sky rather than only a vertical slab.

## 4. Correct the current building roster

### `CityDeck`: replace, do not cosmetically rename

The current implementation and comments call this a **car deck**, use open parking floors and include a
13-degree vehicle ramp. That directly contradicts the world. This should be treated as a P1 fiction fault.

Recommended replacement: a **multi-level pedestrian exchange / covered bazaar**. It can preserve the
valuable horizontal open-floor silhouette while changing the programme:

- broad public stairs or paired stair/escalator-like magical lifts instead of a vehicle ramp;
- arcaded edges with stalls, workshops or guild services;
- an open central void/light well;
- footbridges or terraces visible from the street;
- hoverboard racks or a future mount exchange at ground level if mobility needs a civic home;
- shade canopies, banners, planting and night light on occupied levels.

Alternative for `Works`: a porter depot with hoists, covered platforms and storage galleries. The key is
that each level visibly supports people and goods, not absent cars.

### `CityShops`: four units need four addresses

The model visually declares four shop units through signs and glazing but `_street_storey` provides one
central door. Give each unit a door or pair adjacent units around shared recessed entries. Add:

- individual sign/awning colour within a district-controlled family;
- display depth behind glass, not a flat blue plane;
- a 0.6–1.2 m threshold/spill zone for baskets, menu boards, goods or seating;
- shutters/grilles/closed state for some units;
- a rear service door or passage so goods have a believable route.

The market district should hit at least 60% visually active frontage: doors, displays, windows into public
uses, counters or arcades. A blue glass strip without an entrance is not active.

### `CitySlab`: express households at ground level

A 26 m residential bar with one central opening reads institutional. Keep the balcony rhythm but add two
or three address moments: paired entrances, stoops, lobby canopies, mail/notice panels, planted thresholds
and lit common rooms. Vary balcony occupation by household state—some empty, some planted, some with cloth,
chairs or drying racks—but keep the facade rhythm stronger than the clutter.

### Towers and blocks: give every side a role

The generic tower wraps curtain glazing around all four faces while only the street face has a real
entrance. Establish facade roles:

- front: entrance, canopy, name/address, active or transparent base;
- sides: structural rhythm, neighbouring wall condition, occasional secondary entry;
- rear: loading/service door, plant, waste, vents and less expensive material;
- roof: plant, drainage, access housing and a silhouette chosen by district.

The base/shaft/top model is sound. Improve the **base** before adding more crowns. The player spends nearly
all city time in the first two storeys.

### `CityWorks`: goods still move without cars

Large doors remain plausible for handcarts, mounts, bulky materials and machinery. Reframe the present
roller-door/apron language around human logistics:

- porter doors and covered loading platforms;
- handcarts, wheelbarrows and pallet dollies parked off the clear route;
- overhead hoists, cranes, rope, pulleys and goods lifts;
- stacked materials whose size fits the doorway and workshop;
- dirty/service paving that visibly connects the lane to the door.

The path of goods—from street, through threshold, to storage—should be readable in one image.

## 5. A prop is evidence of an action

Do not scatter a universal city-prop list. Generate **activity clusters** attached to anchors.

### Anchors

- door / threshold;
- shop display bay;
- balcony;
- service door;
- building corner;
- drain/downpipe;
- tree/planter edge;
- bench view target;
- junction decision point;
- courtyard centre;
- wall suitable for a sign, notice or repair;
- loading/porter bay.

### Cluster examples

| Programme | Minimum readable cluster | Optional variation |
|---|---|---|
| food shop | awning/sign + display + swept threshold + baskets/crates to one side | queue rail, menu, delivery bundle, closed shutter |
| maker/workshop | broad door + workbench/material + waste/offcuts + task light | hoist, repair under way, handcart, protective canopy |
| residence | named entrance + mat/step + planter/notice/mail point | laundry, child's toy, chair, repair patch, delivery |
| civic | sign/crest + clear approach + waiting seat + notice board | guard point, banner, fountain, public clock |
| park | shade + seats facing a reason + path + edge planting | game/play object, gardener tools, drink point, memorial |
| service court | refuse/storage + plant/vent + drainage + screened edge | pallets, maintenance cart, leaking pipe, locked cage |
| market node | stalls facing flow + central clear lane + storage/rear edge | musician, queue, shade cloth, seasonal decorations |

The cluster must preserve a clear route. Put furniture in a repeatable edge/furnishing zone, not the path
centre. Benches face a view, door, play area or other seats; they do not all face north or the nearest road
segment. Lamps illuminate a threshold or decision point. Waste stands near exits and service access, not
at a decorative random coordinate. Downpipe stains occur beneath downpipes.

### Occupation states

Give each frontage or household one coherent state instead of independent prop dice:

- open/busy;
- open/quiet;
- closed for the day;
- maintained/proud;
- repaired/adapted;
- neglected/empty;
- under construction;
- festival/seasonal.

The state controls props, window light, shutters, wear and material variation together. This is how a small
kit implies history without noise.

### Repetition control

- Do not repeat the exact `CityForecourt` stamp beside itself. Rotate is not enough; provide at least three
  compositions sharing the same programme.
- Avoid evenly alternating two prop types around an entire square. Build zones: arrival, sitting, vending,
  planting, ceremony and clear event space.
- Enforce minimum distances for identical silhouettes, but also limit consecutive empty frontages.
- Vary occupancy more than geometry: an identical balcony with different use reads as a household; a
  randomly scaled balcony reads as procedural damage.

## 6. Color: move from “modern grey” to district identity

The shared city palette is coherent but presently dominated by concrete `(0.72, 0.71, 0.68)`, secondary
grey, blue curtain wall and cyan neon. Against saturated green ground, this produces a clean prototype
reading rather than an inhabited city. Keep the cool modern age, but introduce controlled warm human
layers and district families.

### Value hierarchy first

- Reserve near-black for ink, deep openings and a few structural cavities.
- Keep ground and facade values separated enough that building bases remain readable in grayscale.
- Do not let pale paving, pale concrete and bright sky all occupy the same cel band in the city approach.
- Use the most saturated accents only on navigation, commerce, guild identity, magic and occupied light.

### Suggested district families

These are direction, not final constants; validate through the real tonemapper and cel bands.

| District/character | Main family | Secondary | Scarce accent | Human evidence |
|---|---|---|---|---|
| Capital/civic | warm light stone, quiet cool concrete | deep guild green / blue-black metal | brass, ceremonial red | banners, named doors, lit public interiors |
| Trade/market | cream, pale clay, warm grey | muted teal, dusty blue | saffron/coral signs and awnings | goods, canopies, open displays, warm pools of light |
| Works | charcoal blue-grey, brick/oxide, weathered steel | concrete and timber | safety ochre or magical cyan, never both everywhere | soot, repair, task lights, materials and hoists |
| Green/residential | warm grey/plaster, sage, dusty blue | timber/verdigris | flower/cloth colors in small household patches | balcony use, planting, stoops, laundry |
| Outskirts | city family mixed with local soil/stone | muted roof colors | route/door accents | gardens, sheds, repairs, softer edges |

Use a 60/30/10 discipline inside one view: dominant architectural family, supporting material family, and
scarce accent. Per-building color variation should stay inside the family; district-to-district change can
be larger.

### Surfaces must tell construction and maintenance

- concrete: panel/joint rhythm, formwork scale or aggregate only at close range; water streaks under edges;
- masonry/brick: course scale and lintels; darker repair patches should follow actual bays;
- metal: differentiate painted metal from exposed metal; place wear at handles and edges;
- glass: vary interior depth/light/curtain state rather than random pane color;
- paving: change module, edge and wear by street hierarchy; keep a stable clear band;
- planted ground: replace uniform electric green with canopy shadow, worn desire lines, dry bases and
  district-maintained beds.

The semi-cel style benefits from broad color blocks. Add medium-frequency construction rhythm and causal
wear, not photographic speckle.

## 7. Give each city a different pedestrian experience

The current `Character` system changes counts, tower shares and yard types. Extend that idea to how a player
moves and what happens at the first two storeys.

### Capital

- formal processional promenade terminating at guild/civic landmark;
- arcades and ordered tree/lamps rather than identical freestanding towers;
- fewer but larger civic squares, with strong sight lines;
- materials maintained and repairs discreet;
- street-level public counters, halls and ceremonies.

### Trade

- highest entrance/sign density;
- narrow commercial passages, covered market lanes, courtyards and cross-routes;
- goods cluster near service alleys, not across the shopping route;
- overlapping canopy/light rhythm and warmer accent range;
- daytime bustle and a visibly different closed/night state.

### Works

- low/wide skyline, service lanes, porter courts, gantries and chimneys;
- direct goods paths, robust darker paving and task lighting;
- smaller public squares but strong worker gathering/food nodes;
- visible repair, adaptation and material storage;
- no decorative lawn between depots.

### Green

- connected canopy and pocket courtyards, not towers isolated in grass;
- shaded walking loops and residential stoops;
- rain gardens and visible drainage;
- lower night intensity except community nodes;
- planted spaces framed by fronts on at least two sides so they feel safe and intentional.

## 8. Procedural implementation shape for Claude

This is a design proposal, not a request to implement everything at once.

### Add relationships before assets

Suggested data concepts:

- `StreetRole`: promenade, main, residential, service, alley;
- `FrontageRole`: active, residential, civic, workshop, service, blank/party;
- `BlockProgramme`: court, garden, service, market, institutional;
- `ActivityAnchor`: door, display, service, balcony, drain, corner, seat target;
- `OccupationState`: busy, quiet, closed, maintained, repaired, neglected, works;
- `PaletteFamily`: district + city character + maintenance state.

The key dependency is:

`city character -> district -> street role -> block/frontage role -> building programme -> anchors -> prop clusters -> material/occupation state`

Do not generate props directly from world coordinates. Coordinates may seed the result, but the programme
must decide what is allowed.

### First vertical slice

Use one 60–100 m section of a Trade city's main route:

1. continuous-surface pedestrian street with a 3 m minimum clear movement band;
2. 70%+ active frontage by metres;
3. one 4-unit shop parade with multiple real entries;
4. one mixed-use slab or block with household evidence;
5. one service passage connecting front to a rear court;
6. three coherent activity clusters and at least two occupation states;
7. district palette with a warm human layer;
8. day, dusk and night captures from identical cameras;
9. player/camera traversal and collision proof.

Do this before propagating across 588 buildings. If this short route does not feel inhabited, a full-city
scatter will only repeat the problem faster.

## 9. Acceptance tests and visual proof

### Measurable spatial checks

- every public entrance connects to a traversable clear route without clipping a kerb, prop or door leaf;
- 2.0 m preferred clear pedestrian band on ordinary routes; never below the controller/camera-proven hard
  minimum;
- no street furniture inside the path clearance envelope;
- market active-frontage ratio measured by frontage metres, not object count;
- no dense-core grass slot narrower than a usable passage or planted strip;
- every service prop has a service access path and every large goods door has usable clear ground;
- benches face a named target and remain reachable;
- drains occupy believable low edges and do not block traversal;
- no vehicle-only prop or marking exists: parking deck, parking bay, car ramp, traffic light, fuel station,
  unexplained six-metre carriageway, or road marking whose only meaning is cars;
- repeated cluster signatures have spacing/variant limits.

### Fixed capture set

For the same city seed and cameras:

1. 250–400 m arrival/skyline;
2. 80–120 m district threshold;
3. 25–40 m main street showing both sides;
4. 8–15 m shopfront and residential frontage;
5. service lane/rear court;
6. public square at human height;
7. overhead block diagram;
8. day/dusk/night versions of views 3–6;
9. grayscale versions of arrival and street views;
10. occupancy-debug view showing active, residential, service and blank frontage metres.

Ask of each image:

- Can the player tell why this street is wide?
- Can the player tell front from rear?
- Is there a reason to stop as well as a route to move?
- Can three human activities be inferred without NPC animation?
- Is the city character readable without labels?
- Does every prop have an owner and a believable location?
- Does any element imply cars?

## 10. Priority order

### P0/P1 coherence

1. Replace the car-specific `CityDeck` programme and remove vehicle-only visual language.
2. Define pedestrian street roles; stop treating every city route as carriageway plus sidewalks.
3. Make one representative market block form a continuous street wall with a real service interior.
4. Give `CityShops` one honest entrance per apparent unit and give long housing fronts multiple addresses.

### P1 lived-in quality

5. Add anchored activity clusters and occupation states.
6. Shift dense-core ground from uniform lawn to courtyards, hardscape, planting beds and service surfaces.
7. Add district palette families and active night-frontage logic.

### P2 expansion

8. Propagate to Capital, Works and Green slices.
9. Add arcades, passages, upper-level household states and seasonal/festival variants.
10. Expand asset variety only after the relationship/debug views prove the generator is using the current
    kit coherently.

## Sources used

- [Global Designing Cities Initiative — Shared Streets](https://globaldesigningcities.org/publication/global-street-design-guide/streets/shared-streets/)
- [Global Designing Cities Initiative — Changing Contexts](https://globaldesigningcities.org/publication/global-street-design-guide/designing-streets-for-place/changing-contexts-2/)
- [UK National Design Guide](https://www.gov.uk/government/publications/national-design-guide/national-design-guide-accessible-version)
- [UK National Model Design Code](https://www.gov.uk/government/publications/national-model-design-code)
- [UK Inclusive Mobility guidance](https://www.gov.uk/government/publications/inclusive-mobility-making-transport-accessible-for-passengers-and-pedestrians)
- [U.S. Access Board — Public Right-of-Way Accessibility Guidelines](https://www.access-board.gov/prowag/technical.html)

These real-world sources are dimensional and relational references, not instructions to reproduce a
contemporary Earth city. Copaimo's fantasy identity should control motifs, materials, magic and landmarks;
human scale, access, drainage, front/back logic and evidence of daily life should remain believable.

