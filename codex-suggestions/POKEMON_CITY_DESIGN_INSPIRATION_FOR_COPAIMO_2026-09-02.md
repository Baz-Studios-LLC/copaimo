# Pokémon city-design research translated for Copaimo

Date: 2026-09-02  
Audience: Claude and the Copaimo design/implementation pass  
Status: inspiration research, not an instruction to reproduce any Pokémon location

## Non-copying boundary

Pokémon is useful here because its settlements often communicate an identity very quickly with limited
space. The goal is to learn the **design operations**, never the protected expression.

Do not reproduce:

- a Pokémon city's map, street geometry, skyline, landmark, façade, color combination or sequence of
  destinations;
- Pokémon names, symbols, creatures, shops, signage, props, battle facilities or narrative situations;
- a recognizable tower-at-the-center radial plan, pier arrangement, giant academy staircase, treetop rope-
  bridge town, floating-log settlement, mushroom-lit clearing, gear-covered industrial city, or other
  signature composition;
- the same relationship between a real-world reference and its Pokémon reinterpretation.

For every borrowed principle, change at least four of these five: **cause, topology, programme, material,
and player use**. Ground the result in Copaimo lore and combine it with real settlement logic. The test is
not “could this pass as a Pokémon town?” It is “does this feel unmistakably like Copaimo while achieving
the same clarity of place?”

## Executive synthesis

The recurring strength is **total commitment to one civic idea**. A memorable city is not “generic city
plus themed props.” Its terrain, street plan, economy, landmark, building types, color, traversal, public
life and quests all seem to have grown from the same cause.

The transferable pattern is:

`reason to exist -> movement structure -> civic landmark -> district programmes -> building fronts -> daily-life evidence -> gameplay`

This suggests six rules for Copaimo:

1. Give every city a one-sentence promise that describes what the player will experience, not only what it
   looks like.
2. Make that promise legible at three scales: skyline/arrival, street/district, and hand-touch/prop.
3. Let geography and livelihood explain the plan. The city should appear to have grown where it is for a
   reason.
4. Concentrate memorable functions into a small number of legible public places instead of spreading
   generic interest evenly.
5. Treat residents' routines and service flows as part of the architecture. A city is lived in when the
   player can infer who opens, repairs, cleans, delivers, teaches, worships and rests there.
6. Pair spectacle with navigational clarity and usable depth. A landmark is not enough if all intermediate
   streets are interchangeable scenery.

## What particular Pokémon settlements demonstrate

These are analytical examples only. The final column is deliberately abstract so it can be used without
copying the source city.

| Reference | What makes it memorable | Transferable operation | Risk to avoid |
|---|---|---|---|
| Goldenrod | Economy, communication, shopping, underground space and regional movement are concentrated in one metropolis. | Give a major city several mutually reinforcing civic systems; let one function continue below/behind another. | A checklist of disconnected “important buildings.” |
| Ecruteak | Traditional architecture, two historic towers, performance and the scar of a destroyed landmark tell one history. | Use **presence plus absence**: a functioning landmark and a visible ruin, repair, forbidden footprint or annual remembrance can make history spatial. | Copying Japanese motifs or the paired-tower story. |
| Slateport | Market, beach, museum, shipyard and harbor all express exchange at the coast. | Show the full economic chain: arrival of material, processing, selling, storage and departure. | A waterfront that is only scenic dressing. |
| Fortree | Homes and circulation respond to rain, trees and nearby wildlife. | Let local climate/ecology change foundations, routes, openings, drainage, roofs and daily routines together. | Recreating treehouses and rope bridges as the city's hook. |
| Pacifidlog | Construction, occupation and movement all respond to life on water. | For an extreme site, redesign ordinary life—delivery, sleep, waste, play and access—not just the silhouette. | Floating log platforms or the same maritime folklore. |
| Sootopolis | A crater explains enclosure, vertical circulation, water and its strong white city image. | Make landform a city-scale room and a traversal problem; use stairs, ramps, overlooks and drainage as identity. | A white crater city or copied arrival method. |
| Nimbasa | Entertainment, sport, performance and transit create a destination city. | Cluster compatible leisure programmes so activity spills between them and changes by time of day. | Theme-park props without back-of-house or residents. |
| Castelia | Named streets, piers, a central plaza, crowds, a narrow back lane and strong view corridors imply a metropolis larger than the playable footprint. | Compress scale through distinct route types, occlusion, named subplaces, crowd direction and glimpsed destinations. | Copying the fan/radial pier layout or filling wide streets with nonfunctional crowds. |
| Lumiose | A strong central beacon, boulevards, plazas, avenues and color-linked subareas support a radial capital. | Use landmark hierarchy and district wayfinding, but give routes distinct edge conditions and activities as well as color. | A central tower radial copy; same-looking streets that make the landmark do all navigation work. |
| Motostoke | Industry is expressed in machinery and a lower/upper-tier city structure. | Expose infrastructure and use grade changes to reveal how the place works. Connect tiers through overlapping views and several meaningful routes. | Decorative machinery or tiers with almost no relationship between them. |
| Ballonlea | A very small footprint feels unique through controlled environmental light, vegetation and color. | A town can be memorable through one tightly controlled atmosphere; density of intent matters more than size. | Copying luminous mushrooms/forest-fairy imagery or using colored light everywhere. |
| Hau'oli | Beachfront, shopping district and marina have different public roles, reinforced by local food, civic services and tourism. | Distinguish districts by **what people do there**, then support that use with local businesses, civic functions and thresholds. | Cosmetic district labels over identical streets. |
| Mesagoza | A central plaza and processional climb frame a dominant civic institution; local architecture and paving carry regional color. | Use a civic approach sequence—threshold, reveal, gathering room, climb and destination—while offering accessible alternate circulation. | Copying the monumental academy/stair composition or forcing every trip through one exhausting route. |
| Levincia | Coastal skyline, light and local energy production make infrastructure part of identity. | Give fantasy infrastructure a visible source, distribution system, workers, maintenance access and public consequence. | Neon/electric styling without a causal system. |

Two recent critical observations are also useful. Reviews of the city-scale *Pokémon Legends: Z-A* praise
dense side stories, alleys, rooftops and citizens whose dialogue relates to their location, while also
criticizing same-looking streets/roofs and residents without convincing daily lives. That is a valuable
AAA warning for Copaimo: **content density and visual density do not equal social simulation or spatial
variety**. Location-specific behavior, time-based state and distinct route grammar must carry through after
the first impression.

## The Copaimo city-identity grammar

The existing separation of `Plan` (`Rings`, `Grid`, `Spine`) from `Character` (`Capital`, `Works`, `Green`,
`Trade`) is the right foundation. Pokémon's lesson is to add a third, causal layer rather than another bag
of random variants.

Suggested conceptual record (names are illustrative, not a request for this exact API):

```text
CityIdentity
  promise                  // one sentence: why visit and what happens here
  founding_cause           // crossing, resource, sanctuary, institution, pilgrimage, defense...
  terrain_contract         // what water/slope/climate forced the builders to do
  visible_economy          // input -> transformation -> output
  primary_landmark_role    // civic, productive, sacred, social, ecological
  secondary_beacons[2]
  route_signature          // procession, exchange loop, terraces, passages, canals...
  public_room_sequence     // threshold -> street -> square/court -> destination
  district_material_family
  district_activity_family
  day_night_rhythm
  historic_mark            // repair, remnant, reused wall, abandoned foundation, memorial...
```

Derive this deterministically from character + plan + geography + seed. Coordinates can choose variants,
but causal rules decide what is allowed. The identity should then feed street details, building programmes,
yard types, props, light states, NPC anchors and quests.

### One-sentence promises for Copaimo's four characters

These are direction examples, not fixed lore:

- **Capital:** “A city where public life is ceremonial and the institutions that order the world are
  always visible.” The player experiences processions, archives, petitions, guild delegations, guarded
  thresholds, formal maintenance and old fabric adapted by newer authority.
- **Works:** “A city whose streets expose the conversion of raw fantasy resources into things the world
  needs.” The player sees deliveries, washing, sorting, heating/cooling, assembly, storage, inspection,
  repair and waste recovery.
- **Green:** “A city that treats water, shade, food and habitat as public infrastructure.” The player moves
  through planted courts, cisterns, productive gardens, nurseries and shaded social edges—not empty lawn.
- **Trade:** “A city built around face-to-face exchange and the movement of people and hand-carried goods.”
  The player reads arrival, bargaining, weighing, lodging, storage, porter routes and closure/opening
  rhythms.

These promises need a Copaimo-specific magical/fantasy cause. They should not default to generic modern
zoning.

### Twelve combinations that remain original

The plan should modify the promise rather than merely rearrange the same content:

| Combination | Original Copaimo direction |
|---|---|
| Rings + Capital | Civic wards accreted around an old oath-ground; successive eras remain visible as reused ring walls and changing paving. |
| Grid + Capital | A deliberately chartered administrative city, with formal view corridors broken by older terrain and informal service passages. |
| Spine + Capital | A ceremonial/pilgrimage route whose side courts house law, memory and hospitality; power is read as a sequence, not a central tower. |
| Rings + Works | Production encircles a shared heat, water or magical-pressure source; cleaner and dirtier processes occupy different arcs. |
| Grid + Works | Repeated workshop courts make an efficient production fabric, differentiated by exhaust, drainage, storage and shift rhythm. |
| Spine + Works | A resource channel or freight walk creates a linear chain from intake through making to dispatch, with parallel safe pedestrian routes. |
| Rings + Green | Water and cultivation radiate from a protected spring/cistern; inner public gardens become denser food and nursery belts outward. |
| Grid + Green | Cistern courts and shade passages occupy a rigorous framework gradually softened by use, repair and plant succession. |
| Spine + Green | A linear watercourse or windbreak supports public rooms, gardens and residences in a changing ecological sequence. |
| Rings + Trade | Exchange specializes by arc and time of day around a civic weighing/announcement ground, with porter shortcuts across rings. |
| Grid + Trade | Guild blocks expose public sales fronts and shared rear courts; cross-streets shift from bulk goods to fine goods and lodging. |
| Spine + Trade | A long arrival bazaar changes from animal/hoverboard dismount and lodging to bulk trade, daily market and specialized civic core. |

None of these requires a Pokémon landmark or recognizable map. They are causal combinations of Copaimo's
existing systems.

## Designing a car-free city that feels populated

Pokémon frequently uses cities as pedestrian game boards even when the fiction contains vehicles. Copaimo
can make the absence of cars a deeper strength by letting **human, hoverboard, mount and goods movement**
shape the urban form honestly.

### A movement hierarchy with visible users

- **Processional/civic promenade:** accommodates crowds, ceremonies, markets and emergency access. Its width
  is justified by periodic occupation, not by an absent carriageway.
- **Main exchange street:** clear through-band plus deep active edges for queues, displays, café/food use,
  performers and conversation.
- **Residential street:** narrower, quieter, more thresholds, household evidence, children/elders, water
  collection and shared sitting.
- **Porter/service lane:** handcarts, mounted deliveries where allowed, hoists, bins, wash-down and workshop
  access. Tougher paving and protected corners tell its use.
- **Passage/arcade:** weather-protected shortcut with compressed views and tiny specialist fronts.
- **Hoverboard/mount transition:** a dismount, tether/store, maintenance and rental/permit point at busy
  cores—never a reskinned car park.

Give each route type a unique cross-section, light spacing, edge activity, sound, prop family, movement
speed and closure rule. The player should know the route type without looking at the map.

### Crowd direction is urban storytelling

Do not scatter NPCs uniformly. Generate directional flows between believable attractors:

- homes -> food/water -> work in the morning;
- gates/lodging -> market/civic services during the day;
- workshops -> food courts/rest edges at shift changes;
- schools/training grounds -> homes/play courts later;
- public venues -> lodging/food after events;
- service workers -> rear courts before public opening and after closure.

Even before full NPC schedules exist, props can prefigure these flows: half-open shutters, swept thresholds,
stacked handcarts, delivery ledgers, queue rails, drying cloth, returned cups, repair ladders, refuse staged
for collection, extinguished versus lit stalls.

## Buildings: identity must survive close inspection

Pokémon often compresses a building to a readable icon and a single function. Copaimo can retain that
clarity while adding the physical truth expected at AAA fidelity.

### Three-scale read

Every important building should communicate at:

1. **100–300 m:** silhouette, roofline, massing and relationship to the city landmark;
2. **20–60 m:** programme through bay rhythm, entrances, signs, awnings, work openings and public space;
3. **1–15 m:** believable construction, drainage, wear, hardware, thresholds, interiors behind glass and
   evidence of an occupant.

### Address honesty

- One apparent shop unit needs one plausible customer entrance, storage relationship and opening state.
- Long housing slabs need repeated household addresses or clearly communicated shared lobbies.
- Public buildings need a formal front, accessible everyday entry, service side/rear, staff access and
  weather/drainage logic.
- Workshops need an input opening, making zone, output/storage zone, worker route and waste path.
- Roofs must reveal use: drainage, access, vents, repairs, drying, gardens or deliberate inaccessibility.

This directly reinforces the current `CityShops`, `CitySlab`, `CityWorks`, `CityBlock*` and replacement
`CityDeck` work. Pokémon-like readability should come from strong programme silhouettes, not false doors or
decorative machinery.

### History through controlled inconsistency

A believable city is not generated in one year. Within a district family, vary:

- foundation age and street alignment;
- later floor additions or infilled courtyards;
- repaired materials and altered openings;
- old civic fabric reused for homes/work;
- abandoned service hardware that still explains a previous economy;
- one historically important absence or protected remnant.

Keep the dominant material grammar consistent so this reads as time, not random kit mixing.

## Props: use tiny stories, not visual noise

Pokémon's strongest environmental moments are often small: a local specialty, a person using a location,
or a creature integrated into daily work. Copaimo should translate this into **activity clusters**.

Each cluster answers four questions:

1. Who owns or uses it?
2. What action happened or will happen?
3. Why is it exactly here?
4. What state is it in now?

Recommended cluster families:

| Anchor | Activity evidence | State variants |
|---|---|---|
| Shop entrance | display, price board, delivery basket, shade/rain cover, swept threshold | opening, busy, sold-down, closing, closed |
| Home threshold | seat, planter, shoes/boots, water vessel, repaired step, name/ward mark | occupied, visiting, celebration, mourning, away |
| Workshop front | material rack, offcuts, safety screen, tool return, finished-goods stand | active shift, cooling, inspection, maintenance, shutdown |
| Rear service court | handcart, sorting bins, drain, hoist, wash point, covered storage | delivery, cleanup, overflow, repair, empty |
| Public fountain/cistern | vessels, wet footprints, queue position, overflow drain, resting edge | morning demand, normal, dry/repair, festival |
| Food/social edge | tables, shade, trays, refuse return, musician space, conversation groups | setup, meal rush, quiet, event, cleanup |
| Gate/threshold | wayfinding, porter post, lodging notice, mount/board transition, inspection point | arrival peak, normal, closure, emergency |

Placement should follow semantic anchors supplied by the building/street programme, never a world-space
scatter pass. Preserve a clear route and collision envelope.

## Color and semi-cel-shaded presentation

Pokémon settlements often use bold regional palettes for immediate recognition. Copaimo needs the same
clarity with more material depth and less toy-like uniformity.

### Palette hierarchy

For each city define:

- **regional base:** local stone/brick/timber/plaster shared across most permanent fabric;
- **district modifier:** one restrained hue/value shift tied to an economic or historic cause;
- **civic accent:** a high-recognition color reserved for wayfinding, authority or shared infrastructure;
- **private accents:** lower-saturation household/shop colors with controlled variation;
- **state colors:** dirt, sun fade, wetness, soot, mineral staining, repair patches and vegetation.

Do not assign every building an unrelated bright hue. Color rhythm works when most surfaces support a few
meaningful accents. A practical target for a street frame is roughly 60–75% quiet structural family,
20–30% district/occupant variation and 5–10% high-attention wayfinding or interaction color.

### Outline-aware city art

- Reserve the strongest continuous outer ink for major silhouette separation and player-important
  interactables.
- Use thinner/broken internal edges for façade bays, frames and construction joints so dense streets do not
  become black grids.
- Keep dark windows from merging with outlines; lift window value or tint the ink locally.
- Separate adjacent buildings by value/material plane before relying on an outline.
- Reduce distant prop-line density by importance and projected size, not simply object class.
- At night, emissive signs/windows should retain shape and local value structure; they should not erase
  architectural edges or turn every district into the same neon scene.
- Use color-coded district cues on repeatable public elements—canopies, paving insets, lantern housings,
  ward marks—whose function is explained in-world. Avoid copying Lumiose's plaza/color system.

## Landmarks and navigation

Use a hierarchy, not one all-purpose monument:

- **one city beacon:** visible on arrival and from selected major spaces, expressing why the city exists;
- **two district beacons:** smaller silhouettes that orient movement after the main beacon is occluded;
- **route signatures:** repeated edge condition, paving detail, awning/lantern family or planting form;
- **micro-landmarks:** a repaired arch, public oven, unusual tree, mural, fountain, hoist or corner shop.

The city beacon should disappear and reappear deliberately. Constant visibility flattens discovery; total
absence causes disorientation. Build “decision-point views”: at each major junction the correct options
should be suggested by a beacon, street character, crowd flow, light or framed destination.

### Compressed scale without empty acreage

Castelia demonstrates that a small number of strongly differentiated streets can imply a much larger
metropolis. For Copaimo:

- use occlusion and bends to hide the full extent;
- name or visually code subplaces through in-world systems;
- imply continuation with gates, stairs, arcades, ferry/porter routes and occupied upper floors;
- make one narrow route, one broad public room and one service route feel qualitatively different;
- use background massing and sounds to imply non-playable city depth;
- prioritize accessible, meaningful interiors over hundreds of false doors.

## Gameplay belongs in the city identity

A city does not become memorable because the quest marker is located there. Give each character a native
play pattern:

- **Capital:** petitions, archives, ceremonial timing, access permissions, public debates, institutional
  shortcuts and consequences visible in civic space.
- **Works:** diagnose a broken production chain, route safe deliveries, learn craft processes, inspect
  failures and see production states change.
- **Green:** manage water/shade/habitat relationships, follow seasonal signs, cross layered public gardens
  and solve access without damaging living systems.
- **Trade:** compare information and goods, follow porter routes, negotiate timing/space, track a shipment
  through front and back stages, experience opening and closing transformations.

The same activity should change architecture or occupation state. This is the difference between a stage
set and a city the player affected.

## What a human–Copaimo city needs

Pokémon's recurring care center is valuable as a design pattern because it gives the player one reliable,
recognizable anchor in every settlement while its exact building can adapt to the region. Across games and
animation, the institution combines care with waiting, information, social gathering, supplies and often
lodging. Day-care/nursery and grooming systems separate longer-term care and personal treatment from
emergency recovery. The transferable principle is **a legible network of companion services**, not one
all-purpose copied center.

The Warden Guild already owns training, breeding and registry; keep those functions together instead of
spinning them into generic city businesses. Outside that specialist institution, the whole city must
support ordinary residents who live with Copaimo. Companion care, grooming, provisions, outfitting,
boarding, water, rest and social space are everyday urban services, not a Warden-only layer. A companion is
a resident and agent, not a horse-shaped prop: it needs routes, thresholds, surfaces, water, shade, privacy,
stimulation, recovery and social space that visibly fit its body and behavior.

### Establish a size-and-behavior envelope before making buildings

Claude should extract the actual companion roster/collision dimensions when available and define:

- `small`: can use household-scale openings and furniture edges;
- `partner`: the common accompanied-Copaimo envelope that all public Warden facilities support;
- `large`: needs double-leaf/service openings, broad turns, outdoor treatment and edge-of-core routes;
- `exceptional`: too large, dangerous, aquatic or flight-dependent for ordinary streets; served at a city-
  edge sanctuary, water gate, roof/perch network or specialist ground.

Do not invent one giant doorway for every creature. Give public facilities a human door, a partner opening
and, where programme requires it, a separate large court entrance. Separate frightened/injured arrivals
from noisy exercise and food queues. Use the real controller/collision envelopes plus comfortable turning
and handler clearance to set dimensions; the illustrative sizes below are block-planning ranges, not
standards.

### Original Warden-city institution roster

| Copaimo institution | Job and placement | Approximate programme/footprint | Street read | How a Copaimo actually uses it |
|---|---|---|---|---|
| **Warden Guild campus** | The existing Guild retains training, breeding, registry, dispatch, examinations and emergency coordination; it is a specialist civic landmark, not the owner of ordinary companion life. | 25–40 m hall frontage plus controlled training/breeding courts sized from the roster; public reception and private care/work zones. | Shared Guild emblem, public signal, notice arcade, arrivals and an unmistakable controlled campus threshold. | Registered/training/breeding routes remain inside Guild programme; public companion waiting, drinking and dispatch edges remain accessible without entering controlled areas. |
| **Bondhouse / companion clinic** | Triage, routine care, isolation and recovery; close to a gate and Warden Hall, but buffered from market noise. | 18×24 m building plus 20×25 m recovery court; more open/low than a hospital tower. | Sheltered intake canopy, washable apron, ventilation/light monitor, calm planted recovery edge. | Walks or is carried through a level intake; receives treatment in a body-appropriate bay; recovers in shade/water with controlled sightlines. |
| **Washhouse / groomer** | Coat, scale, hoof/claw and equipment care; near water/drainage and close to lodging or market, not beside food storage. | 10–16 m frontage, 15–22 m deep, plus 10×16 m drying yard. | Steam, drying racks, brushes/tools, textured non-slip threshold, wet/dry sides visibly separated. | Enters a broad wash bay, can stand/turn safely, drink, perch or rest while drying; runoff reaches a real drain/filter. |
| **Companion outfitter** | Harness, packs, protective wear, household/travel equipment and repairs for any resident, not only Wardens. | 8–14 m active frontage; 14–22 m depth; fitting court or side passage. | Sample equipment at several body heights, measuring frame, repair bench and material racks. | Uses a fitting stand/court with clearance and calm restraint, then exits without reversing through a crowd. |
| **Feed and provision hall** | Species-appropriate travel feed, human provisions and bulk delivery. Place near a service lane/market edge. | 10–18 m frontage; deep cool/dry storage; 4–6 m service opening to rear court where large delivery is relevant. | Scoops, sealed bins, ingredient samples, weigh point and strong input/output rhythm. | Smell/sample point outside the clear path, water station, controlled queue and no access to unsafe bulk storage. |
| **Guild training courts** | Safe practice, breeding assessment and Warden examinations remain part of the Guild campus, not a separate commercial institution. | Small skills yard 20×30 m; large/flight ground at the edge 50×80 m or terrain-derived, subject to actual roster. | Durable controlled perimeter, graded obstacles, observation edge, storage and visible reset/maintenance state. | Runs, climbs, balances, swims or perches on species-appropriate modules; public route never crosses the active envelope. |
| **Wayfarers' court** | Short-stay lodging for Wardens, residents and companions. Near gate/exchange street, away from late market noise. | 20×30 m courtyard building; mixed human rooms, companion bays, kitchen, tack/equipment store and wash court. | Deep covered entry, arrivals board, shared meal room, bedding airing and night staff light. | Enters directly to a secure court, rests near its Warden where safe, accesses water/toilet/wash without crossing dining or guest rooms. |
| **Sanctuary / long-term care ground** | Recovering, unbonded or specialist Copaimo care outside the Guild's breeding programme. Put at the quieter city edge with habitat access. | 40×60 m minimum concept parcel; scale from actual roster and habitat rather than a fixed building. | Habitat structure, keepers' service building, quarantine threshold and controlled public viewing/education edge. | Has retreat space, enrichment, separate care routes and habitat suited to behavior; not displayed continuously to crowds. |
| **Board-and-mount exchange** | Hoverboard issue/repair plus later mount transition, porter dispatch and storage. At approach/core thresholds. | Reuse an 18×24 m open structure plus 20×30 m maneuver/rest court; distributed small stations can be 6×10 m. | Repair benches, lockers, charging/magical service, racks, mounting blocks and clear speed-change paving. | Board is deployed/stowed; mounted users dismount; companions drink/rest; porters collect loads without entering the pedestrian clear band. |
| **Companion commons** | Everyday social, play, rest and acclimation space rather than formal training. Within every dense district. | 25×35 m neighborhood space; smaller 12×18 m rest pockets connected into the route network. | Shade, water, robust edge seating, multiple surface types, low play/enrichment elements. | Chooses sun/shade, soft/hard footing, water, separation or social contact; humans can supervise without occupying its whole space. |

Names must be checked against Copaimo lore; they are deliberately not Pokémon-derived. Not every city needs
every specialist building. The Guild keeps its established specialist responsibilities; basic care,
provisions and public water/rest form the everyday reliable network for all companion-owning citizens.
Character and geography decide which other functions combine or become regionally important.

### Adjacency rules

- Guild hall ↔ registry/dispatch ↔ training/breeding should remain one campus, but controlled active areas
  must not occupy the formal public square.
- Clinic ↔ gate/dispatch should be direct; clinic ↔ loud market/heavy workshop should be buffered.
- Groomer ↔ water/drainage ↔ lodging is useful; groomer ↔ food handling needs separation.
- Provisions ↔ market ↔ service lane ↔ porter exchange should form a visible logistics chain.
- Lodging ↔ arrival/exchange should be obvious on first entry, with a quiet interior court.
- Sanctuary/nursery ↔ habitat/open edge should be close; it needs its own service/quarantine route.
- Commons and water/rest nodes belong on daily paths, not isolated behind buildings.

### Open space is a network of rooms, not leftover grass

The user's “slightly too crowded” reaction should not be answered only by lowering
`HOUSES_IN_A_CITY`. Reserve buildable parcels for named public programmes before building placement, then
let frontage remain dense around their edges. That preserves urbanity while giving companions real space.

Starting **gross city-area** targets for visual iteration (design hypotheses, not universal planning
standards):

| City character | Programmed public/open-space target | What counts |
|---|---:|---|
| Capital | 12–18% | civic square, shaded petition courts, memorial garden, training court, companion rest pockets; exclude decorative setback lawn |
| Works | 10–15% | worker commons, safe cooling/wash courts, recovery green, drainage/water space and edge training; exclude fenced production yards |
| Green | 22–30% | productive gardens, habitat links, water, neighborhood commons, shade courts and exercise; do not make it one empty central park |
| Trade | 15–22% | market square when not occupied, porter courts, wayfarer commons, food/social spaces, water/rest and gate exchange; exclude private service yards |

Within those totals, aim for several scales:

- one city-scale destination (roughly 60–100 m across or terrain-equivalent), distinctive to the city's
  cause;
- one 25–50 m district commons/training/social room per major district;
- 12–20 m pocket rest/water spaces at important route decisions and every roughly 80–120 m along primary
  pedestrian movement;
- narrow ecological/service links where they genuinely connect water, shade, habitat or rear access.

The whole current city is about 340 m across, so access is not a distance problem; **variety, edge activity
and reason to occupy** are. Each open space needs at least four of: named users, activity, shade/shelter,
water, varied footing, supervised edge, active frontage, service access, nighttime state, seasonal state.
Grass without a use satisfies none of these.

### Multi-species spatial kit

Build a small shared kit that can appear in different materials:

- three-height drinking/washing basin with overflow and drainage;
- paired human and companion waiting/rest edges;
- sun, deep shade and rain shelter;
- soft, firm and washable footing zones;
- broad turning bay and a separate quiet retreat pocket;
- perch at low/medium/high levels, with droppings/cleaning logic below;
- mounting/fitting block and safe equipment tie point where culturally appropriate;
- double-gated/offset threshold for care or exercise areas;
- robust corner protection, paw/hoof wear and scratch/rub evidence;
- waste and shedding collection that joins the city's reuse/disposal economy.

Do not stamp the full kit everywhere. Choose by observed body/behavior needs and city programme.

## Current city-building roster: keep, change or drop

This answers the present generator rather than proposing a parallel asset library.

| Current element | Disposition | Shared-city programme |
|---|---|---|
| `CityTower` | Keep, use selectively | Capital offices/archives, dense housing or major guild commerce. Ground floor still needs multiple honest public/residential addresses and a companion-capable public threshold where relevant. |
| `CityBlock`, `CityBlockLow`, `CityBlockTall` | Keep | Mixed housing/institution fabric; vary court access, address rhythm, roof use and district material rather than treating height as identity. |
| `CitySlab` | Keep but repair frontage logic | Housing/wayfarer lodging with repeated household entries or clear shared lobbies, companion rest/wash court and service rear. |
| `CityShops` | Keep and prioritize | Outfitter, provisions, food and ordinary local retail. Each apparent unit needs its own address/opening state and rear logistics. |
| `CityWorks` | Keep | Visible craft/repair/feed processing/board workshop or care-support facility. Programme must show input, work, output, waste and safe public separation. |
| `CityDeck` | Drop the car-park identity completely; adapt the useful open frame | Best fit: board/mount/porter exchange, covered multi-level bazaar, Warden dispatch store or companion-care terrace. Remove parking bays, vehicle ramp language and anonymous empty decks; add human/companion vertical circulation, occupied edges, services and a real court. |
| `CityGreen` yard | Keep only as named programme | Pocket commons, water garden, clinic recovery court, shade/play space, memorial or productive garden. Never generic lawn. |
| `CityKiosk` yard | Keep | Feed/water point, Warden information, grooming pickup, repair booth or food/social anchor, attached to a sitting/queue target. |
| `CityForecourt` yard | Adapt | Public arrival, fitting, clinic intake, market spill or lodging threshold with movement/activity bands—not decorative emptiness. |
| `CityService` yard | Keep and clarify | Handcart/porter loading, sorting, waste/wash, workshop storage and companion-safe service route. Keep it behind/beside the public front. |

The depot survives where it explains a real no-car logistics chain. It should shrink or disappear in a
Green/residential district. The adapted deck/exchange is a city-edge or gateway facility, not a repeated
crafts-quarter filler. Large specialist companion facilities should replace several lots and create active
open space, which reduces crowding structurally without turning the remaining frontage sparse.

## Suggested implementation path for Claude

This is intentionally staged so the research does not become another broad rewrite.

### Step 1 — City cards, no geometry

For each generated city, emit a debug card containing:

- plan + character + founding cause;
- one-sentence promise;
- visible economy chain;
- landmark and two beacons;
- district material/activity cues;
- route signature;
- one historic mark;
- day/night rhythm.

Review the seven world cities as text first. If two cards describe the same experience with different
nouns, the identity space is not ready.

### Step 2 — One 60–100 m vertical slice

Use the existing AQ-032 Trade-city street target. Add only:

- one original city beacon framed at the end/side of selected views;
- one district beacon;
- one front-to-back economic chain;
- one historic layer;
- morning/day/closing occupation states;
- one route decision where architecture/crowd/props guide the player without a HUD marker.

Do not propagate until the short slice communicates its promise in screenshots and traversal.

### Step 3 — Data-driven semantic anchors

Let building and street programmes emit anchors such as:

`public_entry`, `home_entry`, `shop_display`, `delivery`, `service_door`, `drain_low`, `rest_view`,
`queue_edge`, `hoist`, `roof_access`, `sign`, `light`, `waste`, `repair`.

Activity clusters consume compatible anchors. This can extend the existing `Character::yard` and
`District::builds` logic without making prop placement guess building meaning from coordinates.

### Step 4 — Expand by combinations, not asset count

Prove one city per character with a different plan. Then prove that the same character on another plan
produces a different walk while retaining its civic promise. Only then add more assets.

## AAA acceptance checks

### Identity/readability

- In a five-second 250–400 m arrival view, reviewers can identify the city character and point to the
  feature that caused the judgment.
- At 25–40 m, reviewers can distinguish Market, Crafts and Outskirts without UI or unique hero props.
- The city has one beacon, two district beacons and multiple micro-landmarks; no single landmark bears all
  navigation responsibility.
- At every primary route decision, at least two non-UI cues agree: view, surface, edge activity, crowd flow,
  light, sound or landmark.
- A reviewer can sketch the main public-room sequence after one traversal.

### Lived-in evidence

- Every 40–60 m of main street contains at least one coherent activity cluster, one visible destination or
  social focus, and one clue to service/back-of-house life.
- Every major building exposes believable front, side/rear and roof roles.
- The economic chain shows input, transformation, output and waste/reuse somewhere in the district.
- Occupation visibly changes at two or more times/states; not every shutter, light and NPC switches in
  unison.
- Main-street blank frontage does not exceed about 20–25 m without a deliberate civic, garden, service or
  defensive reason.
- Props never reduce the required clear path or collide with doors, corners, stairs, camera or common NPC
  routes.

### Originality firewall

- Side-by-side thumbnails cannot be mistaken for any Pokémon location.
- No Pokémon signature topology, landmark, palette bundle or motif is present.
- Each city card names its Copaimo lore cause and at least two non-Pokémon real-world functional references
  to research before art production.
- The borrowed item can be stated as a verb/principle (“concentrate compatible public uses”, “make history
  visible through repair”) rather than an object (“build a central tower”).

## Recommended priority

1. Keep AQ-032's pedestrian-street vertical slice as the immediate target.
2. Add a city card and one-sentence promise before generating more building variants.
3. Replace vehicle-era programme with pedestrian/mount/hoverboard exchange and service logic.
4. Make one economic chain and one day/night rhythm visible through anchors and states.
5. Prove navigation with landmark hierarchy and route signatures.
6. Only then distribute the grammar across all plan × character combinations.

The highest-value Pokémon lesson is **clarity through coherence**. The highest-value improvement beyond
Pokémon is **physical and social follow-through**: every striking idea should survive a close walk, explain
daily work, guide movement and change over time.

## Sources

- [Castelia City — Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/Castelia_City)
- [Lumiose City — Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/Lumiose)
- [Goldenrod City overview — Bulbapedia walkthrough](https://bulbapedia.bulbagarden.net/wiki/Appendix:Crystal_walkthrough/Section_6)
- [Ecruteak City — Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/Ecruteak_City)
- [Slateport City — Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/Slateport)
- [Fortree City — Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/Fortree_City)
- [Pacifidlog Town — Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/Pacifidlog_Town)
- [Sootopolis City — Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/Sootopolis)
- [Nimbasa City — Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/Nimbasa_City)
- [Motostoke — Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/Motostoke)
- [Ballonlea — Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/Ballonlea)
- [Hau'oli City — Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/Hau%27oli_City)
- [Mesagoza — Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/Mesagoza)
- [Levincia — Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/Levincia)
- [Official Pokémon Sword/Shield Galar region overview](https://swordshield.pokemon.com/en-us/story/the-galar-region/)
- [Official Galar tour brochure — The Pokémon Company](https://assets.pokemon.com/assets/cms2/pdf/video-game/sword-shield/Galar_Tour_Brochure_US.pdf)
- [Pokémon Center — Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/Pokemon_Center)
- [Pokémon Day Care — Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/Pok%C3%A9mon_Day_Care)
- [Pokémon groomer — Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/Pokemon_grooming)
- [Pokémon Legends: Z-A review discussing dense urban content and rooftop sameness — GamesRadar](https://www.gamesradar.com/games/pokemon/pokemon-legends-z-a-review/)
- [Pokémon Legends: Z-A city-life critique discussing location-specific dialogue and static resident behavior — GamesRadar](https://www.gamesradar.com/games/pokemon/pokemon-legends-z-a-makes-the-world-feel-livelier-than-ever-before-but-its-not-because-of-lumioses-design/)

Bulbapedia is used here to compare documented layouts, functions and in-game descriptions across many
titles. Interpretive conclusions and all Copaimo proposals are Codex's synthesis, not claims from those
pages.
