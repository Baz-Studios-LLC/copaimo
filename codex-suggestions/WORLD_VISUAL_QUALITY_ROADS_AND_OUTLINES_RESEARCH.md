# Copaimo world visual-quality research: roads, settlement approaches, outlines, and environmental cohesion

**Audience:** Claude, as implementation guidance only  
**Scope:** research and recommendations; no game files were changed  
**Engine context reviewed:** Bevy 0.16, Copaimo's current terrain, route, settlement, town, material, and outline systems  
**Primary concern from the player:** the dirt-road-to-city-road transition is abrupt, missing, or visually unconvincing

## Executive conclusion

Copaimo does not principally need more texture noise. It needs **transitions with structure**.
The road problem is the clearest example. The current code already eases road color from earth to
stone over 34 metres, fades out width wander, blends the edge toward a kerb color, crowns the
ribbon, and fades its shoulders into the local ground. Those are good pieces. The visible result can
still feel instantaneous because only a small subset of the scene changes, and it changes over a
very short distance. A road becomes a street through a whole sequence of physical and social cues:
the verge is maintained, drainage changes, plots begin addressing the road, the width stabilizes,
edge stones appear, lights and signs begin, then kerbs, gutters, footways, and continuous frontage
arrive. Color is one cue among many.

The recommended solution is an **approach corridor with staged road states**, evaluated along the
actual incoming route rather than only by radial distance from a settlement center. Give each
incoming route a readable sequence:

1. open-country track;
2. managed rural approach;
3. inhabited fringe;
4. a conspicuous gateway or threshold;
5. an urban street with the complete street cross-section.

Some variables should change gradually; others should arrive at a meaningful threshold. A gradual
surface blend prevents a seam. A gate, bridge, lamp pair, tree avenue, wall opening, drainage
culvert, paving band, or first continuous frontage gives the player a memorable moment of arrival.
The combination solves both halves of the complaint: no accidental hard seam, but also no vague,
invisible transition.

The highest-return broader visual-quality changes follow the same rule:

- generate roadside and settlement-edge ecology from causes, not uniform scatter;
- compose journeys from player height with reveals, occlusion, landmarks, and repeated route
  rhythms;
- reserve outlines for silhouettes and important structural separations rather than tracing every
  polygon and every road edge;
- use material state to show wear, moisture, occupation, and maintenance;
- make weather, wind, light, particles, vegetation, and surface response feel like one system;
- validate all procedural work with fixed approach shots and traversal, not only aerial images.

## 1. What the current road implementation actually does

This diagnosis is based on read-only inspection of `src/world/route.rs`, `src/world/settle.rs`,
`src/world/town.rs`, `src/config.rs`, and the current art captures.

### 1.1 Strengths worth preserving

The current system is substantially better than a naive flat ribbon:

- the world route solver considers dry land, grade, shoreline cost, and bridges;
- settlement connections use actual route cost rather than only straight-line distance;
- settlement ground is shaped and eased rather than cut as an unblended disc;
- roads are drawn as connected `Way` ribbons with mitred bends;
- country road width is about 4.6 m and town street width is 6 m, which already gives a useful
  rural/urban distinction;
- the ribbon samples terrain every 2.5 m, follows height, has a crown, and tucks into its shoulders;
- road shoulders inherit the actual underlying terrain color;
- broad, fine, and close wear fields prevent a perfectly flat surface;
- the city surface adds stone-scale variation;
- `paved_here` uses a smoothstep rather than a binary color change;
- width wander and the visual kerb color are already tied to the paving factor.

Claude should preserve these. The fix is to extend their semantic reach, not discard them.

### 1.2 Why the transition can still look abrupt

The current paving transition is a single scalar driven by distance to the center of any city:

- it is fully rural outside `site.radius + 34 m`;
- it becomes fully paved at `site.radius`;
- it is radial, not aware of where the approach road meets the actual developed edge;
- it mainly affects vertex color, cobble variation, width wander, and edge color;
- it does not create a geometrical kerb, gutter, ditch, footway, verge, frontage ramp, street trees,
  drainage structure, or street-furniture ramp;
- it does not explicitly taper the 4.6 m country road into a 6 m city street;
- the locally generated city street mesh is independently forced fully paved.

Thirty-four metres is only a few seconds of movement. At ordinary third-person view it can pass
under the camera before the scene has enough distance to communicate a new place.

There is also a continuity risk in the current country-road selection. `lay_the_country_roads`
calls `dirt_roads_near`, and the shared selection helper classifies route legs by whether the
**leg midpoint** lies inside a city. The country pass asks only for the outside class. A segment
whose midpoint crosses the classification can therefore be omitted from that mesh rather than
carried continuously through the state transition. The unused `paved_roads_near` path is evidence
of the earlier two-mesh split. Even when the independently generated city street happens to cover
the missing area, the two systems can disagree about centerline, width, height, or endpoint. This
is a plausible cause of transitions that sometimes appear not to exist.

### 1.3 The core principle

Do not decide whether an entire route segment is dirt or paved. Include the complete visible route
chain, then evaluate a **road state at each sampling station**. Classification belongs to the
cross-section, not to segment inclusion.

## 2. The rural-to-city arrival grammar

The FHWA's real-world transition-zone guidance separates open rural road, a perception/reaction
area, a deceleration area, and the community zone. The value for Copaimo is perceptual rather than
automotive: players need advance warning, a developing change of character, a legible threshold,
and a stable urban condition. The same source explicitly notes that poorly delineated transitions
lack visual clues, while a transition that is too short feels abrupt. This maps almost exactly to
the current visual problem.

### 2.1 Five stages, not one blend

The exact distances must be tuned to movement speed and camera, but a useful first prototype for a
city is a 160–240 m arrival corridor. A smaller village can use 90–150 m.

| Stage | Suggested portion | Road surface | Edges and drainage | Roadside occupation | Arrival cue |
|---|---:|---|---|---|---|
| Open-country track | first 30–40% | packed earth, wheel wear, local aggregate | irregular soft shoulder, shallow ditch or natural runoff | field, woodland, scrub; no continuous frontage | distant landmark or smoke |
| Managed approach | next 20% | more aggregate, fewer soft patches, width begins to stabilize | verge cleared; ditch becomes deliberate; occasional culvert | first fence, orchard, milestone, utility or wayfinding post | settlement identity begins |
| Inhabited fringe | next 20% | gravel/cobble tongues and repairs; earth still visible | intermittent edge stones, driveway crossings, drainage blocks | gardens, yards, outbuildings, hedges, scattered lamps | first inhabited plot faces road |
| Gateway | narrow authored band | conspicuous paving/material band, bridge, compacted square, or full-width cobble | first continuous gutter/kerb or wall opening; width taper completes | paired trees, lamps, sign, gateposts, watch structure, or market marker | unmistakable threshold |
| Urban street | remainder | coherent paved surface with local wear | continuous kerb/gutter; footway or usable verge; controlled crown | continuous frontage, doors, lamps, deliveries, drains | landmark framed down street |

These should not become five visible texture stripes. They are overlapping systems with different
onset curves. The gateway is the only deliberately sharp event, and it must have a physical cause.

### 2.2 Gradual variables versus threshold variables

Gradual variables:

- earth-to-aggregate-to-stone surface coverage;
- width stability and centerline drift;
- verge maintenance;
- vegetation suppression;
- roadside prop density;
- repair frequency;
- frontage density;
- light spacing;
- road crown/crossfall character;
- ambient dust reduction and wetness retention.

Threshold variables:

- the first continuous kerb;
- a gate, sign, wall opening, or bridge;
- a deliberate material band across the road;
- the first sidewalk/footway;
- a drainage grate or built culvert;
- the first paired street lamps;
- a change from ditch drainage to gutter drainage;
- a settlement-specific emblem or palette accent.

A city entrance should contain at least one threshold variable. Otherwise an extremely smooth
transition can become visually nonexistent.

### 2.3 The transition should be route-relative

Radial distance is useful as a fallback but produces circular behavior around settlements even
where actual development is asymmetric. Derive the main transition from the incoming route:

1. identify each route chain that enters the settlement influence area;
2. compute cumulative distance along that chain;
3. find an entrance anchor from the first meaningful developed/frontage edge, wall opening, or
   generated gateway node;
4. evaluate signed distance before/after that anchor;
5. obtain a stable `arrival_t` through a smooth curve across the desired approach length;
6. let local parcel/frontage density and terrain constraints modulate the result;
7. fall back to radial `paved_here` only where a route chain has no resolved entrance.

This also allows two entrances to the same city to feel different without random inconsistency. A
river entrance may cross a bridge; a forest entrance may become a clipped tree avenue; a farm
entrance may pass orchards and walls. All still share the city's material and furnishing family.

## 3. Recommended road-state model for Claude

One `paved: f32` cannot express the road's full visual state. Keep a small semantic structure whose
fields are derived from route class, settlement type, arrival distance, biome, weather, and local
terrain. Conceptually:

```text
RoadState {
    surface_mix,          // earth -> gravel/aggregate -> stone/paving
    built_edge,           // natural shoulder -> edge stones -> kerb
    carriageway_width,    // route class plus a controlled entrance taper
    width_stability,      // worn wandering edge -> surveyed edge
    crown_or_crossfall,   // natural track -> engineered drainage profile
    ditch,                // rural drainage presence and side
    gutter,               // urban drainage presence
    footway,              // none -> intermittent -> continuous
    verge_maintenance,    // wild -> cut/managed
    vegetation_exclusion, // disturbed center/shoulder/verge masks
    frontage,             // roadside occupation intensity
    furniture,            // markers, lamps, benches, posts, drains
    damage_and_repairs,   // cause-based wear, not global random noise
    arrival_t,
    route_class,
    settlement_family,
}
```

This need not create many entities. Most fields can be sampled while building the existing single
road mesh. A few threshold props can be spawned from deterministic candidate points.

### 3.1 Use named deterministic variation

Variation should be stable and independently addressable. Use distinct seed channels for gateway
type, ditch side, repair patches, tree rhythm, furniture, and wetness. Adding a bench generator
must not reroll a city's entrance material or road shape.

### 3.2 Derive states from causes

- route class determines intended width, directness, maintenance, and bridge capacity;
- slope determines cut/fill, retaining edge, drainage side, and erosion;
- flow or local low points determine puddles, culverts, and damp staining;
- settlement age and wealth determine material completeness and repair language;
- district determines footway, delivery space, market spill, and light density;
- biome determines verge plants, dust color, aggregate color, walls, and tree species;
- traffic/access determines wheel wear, compacted center, and roadside disturbance;
- weather changes wetness/snow/dust state but does not replace material identity.

## 4. Road geometry: make the cross-section tell the story

### 4.1 Taper, do not snap

The 4.6 m rural road and 6 m town street should be joined by a controlled taper centered on the
gateway. Evaluate width per station rather than storing one width for the entire `Way`. Avoid
randomly widening the road in the open approach. The widening should look surveyed and purposeful,
often accompanied by the first edge stones, drainage, or frontage.

If the desired city design narrows traffic at the gateway, the apparent width can still become more
urban through kerbs, walls, trees, and footways while the actual carriageway narrows. FHWA guidance
calls this change in apparent roadside enclosure “optical width”; regularly spaced planting and
built edges communicate a change of character even before a large geometric change.

### 4.2 Transition the edge type

A color called `ROAD_KERB` is not yet a kerb. Use a small geometry vocabulary:

- natural shoulder: ribbon meets ground with no raised edge;
- worn shoulder: compacted band beyond wheel track;
- ditch: shallow depressed channel with an outer bank;
- intermittent edge stones: individual or broken low modules;
- flush stone gutter: visible material and slight crossfall, still traversable;
- raised kerb: reserved for the established urban condition;
- footway: distinct plane and function behind the kerb;
- driveway crossing: kerb/ditch continuity is intentionally interrupted.

The goal is not engineering simulation. Even a 2–4 vertex profile per side can clearly separate
these classes. Preserve collision simplicity unless the edge is genuinely gameplay-relevant.

### 4.3 Drainage makes the construction believable

Roads look “placed” when the world shows no response to them. Add a readable but economical
drainage grammar:

- rural road: crown or slight crossfall, ditch on appropriate sides, culvert where water or a
  driveway crosses;
- fringe: stabilized ditch, stone headwalls, intermittent channel blocks;
- urban street: gutter strip and sparse grate/downspout connections;
- wet weather: dampness and puddles in low edges, never uniformly on the crowned center;
- snow: wheels clear the main lanes first; shoulder accumulation lasts longer.

This is both visual quality and environmental storytelling. It explains why the road persists
through rain and terrain.

### 4.4 Use the current route polyline as a guide, then regularize it

Copaimo's coarse route search is appropriate for finding a corridor. For presentation geometry,
fit or relax the route into curves constrained by:

- maximum curvature for the route class;
- maximum grade and rate of grade change;
- water, cliff, and building clearance;
- bridge and gateway tangents;
- sightline to the next decision or landmark;
- deliberate connection to existing roads.

Do not add arbitrary sine-wave wobble. A curve should be caused by topography, ownership,
vegetation, water, or a historical obstacle. Random curvature reads like procedural noise.

### 4.5 Replace junction discs when they are visually obvious

The current discs are a sound emergency patch for ribbon gaps, but a visible round patch at every
polyline end can make the network look beaded. For important or close junctions, build a junction
polygon from incident road edges:

- identify the through/primary route;
- taper minor approaches into it;
- fillet or chamfer corners by route class;
- continue the crown/gutter logic through the node;
- place wear where turns actually pass;
- reserve a round form for an intentional plaza or roundabout.

The indie version can keep discs at distance and use proper junction patches only inside
settlements and at photographed entrances.

## 5. Road surfaces: layered, sparse, and causally placed

### 5.1 Keep broad transition in geometry/vertex data

The existing per-vertex color is a good place for macro coverage: earth, aggregate, stone, dampness,
and broad wear. It is stable, cheap, and follows the road. Use it for the transition that must never
break.

### 5.2 Use decals for semantic events, not for the entire surface

Bevy 0.16 officially added forward and clustered decals. Forward decals have broader platform
support; clustered decals are higher quality but have bindless/platform restrictions. A small
decal vocabulary can add high-value localized events:

- gravel tongues spreading out from partial paving;
- mud dragged from a field entrance;
- repaired stone/asphalt patches;
- cart braking/turn wear at a gateway;
- damp gutter runs and puddle rims;
- leaf litter caught at kerbs;
- cracks near drainage or settlement movement;
- painted or inlaid gateway bands if appropriate to the setting.

Avoid projecting generic grunge everywhere. Decals should answer “what happened here?” and should
be budgeted per visible corridor.

### 5.3 Three scales of road information

- **Macro, 30–200 m:** road class, width, value, curvature, settlement arrival, wet/dry state.
- **Meso, 5–30 m:** wheel paths, aggregate patches, gutters, repairs, verge management, drains.
- **Micro, under 5 m:** stone breaks, cracks, pebbles, footprints. Keep this sparse and optional.

The current code already has three noise frequencies, but frequency alone does not create meaning.
Mask those fields by lanes, low points, turn paths, edge distance, and nearby land use.

### 5.4 Road palette must belong to the region

Do not use one neutral dirt and one neutral paving color across the world. Inherit controlled
regional variants:

- local soil hue in the earth component;
- local stone/aggregate hue in paving and repairs;
- warmer, lower-value compacted wheel paths;
- a cooler or darker damp state;
- restrained settlement accents in edge stones, banners, lamps, or walls.

Keep value contrast strong enough that the road remains readable at gameplay distance. Hue nuance
cannot compensate for a route that vanishes into equal-value ground.

## 6. Settlement approaches and pathing between settlements

### 6.1 Road hierarchy should be visible

Research on procedural road generation consistently treats networks hierarchically. Copaimo should
distinguish at least:

- **primary inter-city route:** straighter, wider, better drained, more bridges, stronger roadside
  clearance, reliable gateway treatment;
- **secondary town route:** terrain-following, repaired selectively, intermittent markers and
  habitation;
- **local lane:** narrow, irregular, strong wheel wear, hedges/fences, more direct relationship to
  fields and homes;
- **foot/desire path:** shortest human connection, low width, no vehicular construction language.

Road class must affect topology, geometry, materials, maintenance, props, and destination type—not
just color.

### 6.2 The network should reuse roads and form selective loops

An MST is efficient but visually and functionally tree-like. Village-generation research shows the
value of lowering the cost of reusing an existing road so routes converge into a network rather
than independently drawing near-parallel lines. Preserve the efficient backbone, then consider a
small number of added edges where the benefit is high:

- connect two branches that otherwise require a long backtrack;
- create a secondary town entrance;
- connect a resource, bridge, or district that has no readable access;
- form one meaningful local loop around a settlement or landmark.

Do not maximize connectivity. A few loops create choice and resilience; too many erase hierarchy.

### 6.3 A journey needs beats

Long roads should not be equally detailed everywhere. Build a rhythm:

1. destination or landmark glimpse;
2. travel enclosure or terrain response;
3. a small foreground event;
4. temporary concealment or bend;
5. renewed reveal at a closer scale;
6. arrival sequence.

Suitable low-cost beats include a fork marker, culvert, bridgehead, shrine, wayside tree, abandoned
cart, pasture gate, quarry scar, overlook, road repair, creek crossing, or change in wind/particles.
Place them at decision points, biome thresholds, slope changes, water crossings, and major visual
reveals—not at uniform intervals.

### 6.4 Compose from the player's travel direction

The Horizon level-design material emphasizes placing content near travel routes, creating
sightlines from roads, and using framing, occlusion, light/dark, and parallax for reveal. For every
settlement entrance, compute or capture the approach direction and test:

- can the player see one destination cue before the first turn?
- does foreground terrain or vegetation frame it rather than expose everything at once?
- is the next road segment legible at each decision?
- does the gateway silhouette against readable ground or sky?
- is the landmark sometimes concealed and then revealed, instead of permanently centered?

Nintendo's documented “triangle rule” for Breath of the Wild similarly uses large, medium, and
small triangular forms to block, reveal, and offer paths around obstacles. The useful lesson is not
to fill the terrain with literal pyramids; it is to alternate visible goals, partial occlusion, and
route choice so travel is visually active.

## 7. Roadside ecology and settlement-edge quality

### 7.1 Roads need an influence field

Generate roadside environment from distance to the road and construction state:

- carriageway core: no vegetation;
- wear/shoulder band: sparse disturbed plants, stones, mud, hoof/cart marks;
- managed verge: short grass or deliberate planting near settlement;
- ditch/moisture band: taller or moisture-loving plants where appropriate;
- hedge/fence/wall band: aligned with parcels and gates, interrupted at access points;
- outer biome: returns gradually to local ecology.

This will do more for integration than another layer of random terrain noise.

### 7.2 Use vegetation clusters and negative space

The official Ghost of Tsushima world-art discussion describes limiting foliage types per biome,
pushing color, reducing texture noise, and using procedural tools in service of a strong art
direction. For Copaimo:

- use a small dominant species set per biome;
- cluster plants into readable masses;
- leave deliberate negative space around roads, landmarks, doors, and gameplay paths;
- make ecotones gradual, with a few transitional species;
- use height, slope, moisture, shade, grazing, fire, and road disturbance as placement causes;
- protect silhouette and landmark sightlines from procedural tree growth.

### 7.3 A settlement edge is occupied land

The edge should not be “last house, then untouched grass.” Ramp land use before building density:

- field boundary or orchard;
- ditch, low wall, fence, or hedge;
- garden and work yard;
- storage, drying, animals, wood piles, drainage, laundry, or refuse in controlled locations;
- first detached building;
- then continuous frontage where the settlement type supports it.

Break the edge where roads and designed sightlines pass through. Avoid another perfect circular
ring unless the settlement's history explicitly produced one.

## 8. Proper outlining for objects in the semi-cel-shaded style

The previous outline research remains the detailed implementation reference. The most important
world-scale refinement is **selectivity**.

### 8.1 Four line classes

Treat these separately:

1. outer silhouette;
2. object-object/contact separation;
3. major internal structural seam;
4. decorative surface detail.

Only the first two should usually be automatic. Major internal seams are best authored or marked.
Decorative detail should come from value/material design, not a black line around everything.

### 8.2 Recommended line placement by category

**Characters and creatures**

- strongest silhouette priority;
- controlled lines at face/hair/clothing overlaps;
- stable screen-space thickness;
- never allow foliage and environment ink to compete with faces.

**Buildings**

- silhouette against sky and clearly separate neighboring masses;
- roof-wall break, major eaves, door portals, and selected window-frame structure;
- omit lines across large coplanar wall panels unless they mark a true architectural boundary;
- reduce line density with distance before reducing the whole building to noisy hatching.

**Props**

- outline interactable, held, or hero props more strongly;
- use contact separation for stacked crates, carts, and furniture;
- background clutter receives weaker or no ink.

**Terrain and roads**

- do not outline the entire road edge as a black ribbon;
- do not outline every terrain triangle, rock crack, grass blade, or shore polygon;
- rely on value, material edge, vegetation exclusion, shadows, and local contact accents;
- use authored dark marks for culvert mouths, bridge undersides, retaining-wall joints, or
  significant stone edges;
- terrain silhouettes against sky may receive a subtle distance-faded edge if needed.

**Vegetation**

- outline canopy masses and selected near leaves/branches, not every alpha-card edge;
- weaken ink with distance and in shadow masses;
- keep wind-driven outline motion temporally stable.

### 8.3 Hybrid implementation

The strongest fit remains:

- inverted-hull or authored geometry for hero silhouettes requiring exact artistic control;
- image-space depth/normal discontinuities for broad silhouettes and object separation;
- authored seam geometry or metadata for important internal lines;
- category/object/material/section IDs to avoid treating every material boundary as equally
  important.

Arc System Works' production presentation stresses intentional vertex normals because even small
normal inconsistencies become obvious under cel bands. Good outlines cannot rescue incoherent
normals; model normals and cel shading must be authored together. Image-space outline research also
shows that silhouette, shadow, and texture boundaries are different signals, while temporal
coherence research identifies stable motion as a first-class requirement. Judge the system while
the camera, characters, foliage, and LODs move—not from a still frame only.

### 8.4 Ink hierarchy

Start with a category table, not one global thickness:

| Category | Near | Mid | Far |
|---|---|---|---|
| player/companion | strongest stable silhouette + selected internals | strong silhouette | simplified silhouette |
| interactable/hero prop | strong | medium | off unless landmark-relevant |
| building | medium silhouette + sparse authored seams | thinner silhouette | atmospheric fade/off |
| background prop | light | minimal | off |
| terrain | selective skyline/contact only | skyline only | generally off |
| vegetation | mass silhouette, sparse interior | mass edge only | off or palette separation |
| road | no global black edge | none | none |

Use a very dark hue related to the local palette rather than absolute black everywhere. Preserve
near-black for characters, hero objects, and the highest-priority separations. Fade environmental
ink toward atmospheric color with distance.

## 9. Other high-return visual-quality systems

### 9.1 Atmospheric depth without opaque fog

Copaimo has reason to avoid fog that hides the world or exposes streaming limits. Aerial
perspective can still be subtle and art-directed:

- lower distant contrast and saturation slightly;
- bias far color toward the sky/time-of-day palette;
- reduce outline weight with distance;
- keep landmarks protected from excessive fade;
- use height and humidity/biome only as gentle modifiers;
- test dusk and night independently from noon.

Bevy 0.16 includes procedural atmospheric scattering, but it changes sky and distance fog and has
documented reflection/direct-light limitations. Treat it as a measured prototype, not an automatic
upgrade. A simple palette-based distance grade may better preserve the semi-cel-shaded direction.

### 9.2 Lighting and contact

- establish a stable value hierarchy before adding more lights;
- reserve high contrast for characters, entrances, active windows, and landmarks;
- use subtle contact darkening where buildings and props meet the ground, but avoid black AO seams
  that imitate outlines everywhere;
- allow street-light pools to overlap enough to read as a route at night;
- vary occupied windows in coherent clusters, not independent random pixels;
- use bridge undersides, arcades, and tree canopies to make intentional shadow masses;
- keep emissive values and bloom restrained so window shape survives.

### 9.3 Wind and environmental motion

Ghost of Tsushima's official VFX discussion describes one global wind direction shared by foliage,
cloth, smoke, and particles, with layered local gusts. This is a powerful indie-friendly quality
multiplier. In Copaimo:

- share a global wind vector across grass, tree crowns, banners, clothing, chimney smoke, leaves,
  dust, and water response;
- add low-frequency gusts rather than independent oscillation;
- make biome-aware ambient particles follow the camera only as a streaming optimization, while
  their direction and density remain world-consistent;
- suppress particles inside interiors and sheltered spaces;
- use motion to point toward weather and open terrain, not as constant screen noise.

### 9.4 Water and shore integration

- use depth to control water value/hue;
- add a narrow wet-shore response rather than a universal bright foam line;
- place foam at actual contact, current change, obstacles, and steep shore segments;
- reflect or inherit sky color while preserving a readable stylized base;
- add bridge pier interaction and downstream wakes where visible;
- keep shore vegetation and mud causally tied to water level and slope.

### 9.5 Weather as material state

- rain darkens and smooths surfaces according to material class;
- puddles collect at road edges, ruts, and low terrain rather than by random circles;
- snow accumulates on upward-facing, sheltered surfaces and is worn/cleared on routes;
- dust comes from dry disturbed ground and follows traffic/wind;
- wetness strengthens reflections selectively but should not turn the cel-shaded world into uniform
  glossy PBR.

### 9.6 Camera and locomotion polish

Visual quality collapses if camera and character motion do not support it:

- keep the horizon and destination readable on roads;
- use subtle speed-based camera behavior, never constant dramatic FOV pumping;
- improve foot planting, acceleration/deceleration, turn anticipation, and slope response;
- prevent vegetation and outlines from vibrating against the camera;
- use foreground occluder fading only where necessary, with a style-consistent solution.

## 10. AAA standard versus sustainable indie implementation

### 10.1 AAA-style production standard

An AAA road/world pipeline commonly supports:

- hierarchical splines with road-class presets;
- authored and procedural overrides per entrance;
- several cross-section profiles, lane/edge/drainage rules, and intersection solvers;
- terrain cut/fill, retaining structures, bridge/culvert generation, and collision regeneration;
- biome- and district-specific roadside palettes;
- decals/virtual texturing or layered materials for repairs, dirt, and wetness;
- traffic/usage masks and flow-aware wear;
- HLOD, GPU placement, culling, and strict budgets;
- dedicated world-art, lighting, VFX, vegetation, and technical-art review;
- fixed cinematic and gameplay capture suites.

The quality lesson to copy is not feature count. It is the separation of semantic layers and the
ability to art-direct exceptions.

### 10.2 Indie target with most of the perceptual benefit

Copaimo can obtain much of the visible benefit with:

1. one continuous route chain and one station-based `RoadState`;
2. three cross-sections: rural, fringe, urban;
3. one entrance anchor plus a 160–240 m approach interval;
4. a controlled width taper from route to street;
5. four edge modules: shoulder, ditch, edge stone, kerb/gutter;
6. one deterministic gateway kit per settlement family;
7. vertex-color macro blending plus 8–12 reusable semantic decals;
8. road-driven vegetation/prop exclusion and verge bands;
9. selective hybrid outlines by category;
10. a fixed screenshot/traversal matrix and performance budget.

This is a coherent system, not a collection of heroic one-off assets.

## 11. Recommended implementation order

### Phase 0 — prove the cause of the current gap

- visualize every inter-settlement route segment, its midpoint city classification, and the
  independently generated city street centerline;
- capture one failing entrance from above and from player height;
- confirm whether any segment is omitted, overlapped, or laterally misaligned;
- measure the visible duration of the current 34 m blend at walking/riding speed.

**Exit condition:** Claude can state whether the complaint is primarily missing geometry, a short
color-only blend, centerline mismatch, or a combination.

### Phase 1 — continuous geometry and a longer route-relative blend

- include the entire relevant route chain in one mesh regardless of midpoint classification;
- compute `arrival_t` along the road;
- keep the current shoulder, crown, wear, and terrain following;
- taper width between country and city states;
- use 160–240 m as an initial city test, then tune from play speed;
- do not add props yet.

**Exit condition:** there is no hole, double ribbon, hard color seam, or sudden width snap.

### Phase 2 — three cross-sections and one gateway

- rural shoulder/ditch;
- fringe intermittent edge stone or stabilized ditch;
- urban kerb/gutter/footway;
- one settlement-family gateway at the threshold.

**Exit condition:** a grayscale player-height video communicates arrival even if surface colors are
temporarily neutralized.

### Phase 3 — roadside land-use ramp

- road-driven vegetation bands;
- first fields/fences/gardens/yards;
- sparse approach furniture and lamp-density ramp;
- preserve landmark sightlines.

**Exit condition:** the settlement begins before the first dense building row and does not end as a
perfect radial line.

### Phase 4 — semantic surface events

- aggregate tongues, repairs, mud, gutter dampness, leaf litter, turn wear;
- use Bevy 0.16 forward decals first if target-platform breadth matters;
- cap decal density and verify grazing angles.

**Exit condition:** close-range variation has a cause and disappears gracefully at distance.

### Phase 5 — broader world cohesion

- outline category table and distance policy;
- global wind and biome-aware ambient motion;
- subtle atmospheric depth;
- water/shore response;
- weather surface states;
- camera/locomotion polish.

## 12. Validation: what Claude should test

### 12.1 Named entrance capture matrix

For every city ingress, capture from the centerline at approximately:

- 250 m before threshold;
- 150 m before;
- 75 m before;
- 25 m before;
- threshold;
- 40 m inside;
- first important junction.

Capture each at:

- player height and ordinary gameplay FOV;
- one elevated debug view;
- noon, dusk, and night;
- dry and wet if weather exists;
- grayscale/value-only debug;
- road-state debug colors;
- outline-only/debug edge view when that pass exists.

### 12.2 Geometry assertions

- route centerline stays continuous through the settlement entrance;
- no included segment is dropped because its midpoint changes class;
- adjacent cross-sections share positions without gaps or overlaps;
- width change per metre stays below a chosen taper limit except at designed thresholds;
- road never sinks under or visibly floats above terrain;
- gutter is lower than crown and puddle candidates are not on the crown;
- culverts align with ditch/flow crossings;
- gateways leave gameplay and vehicle clearance;
- vegetation/props do not obstruct the traversable corridor or camera sightline;
- local city street and inter-settlement route do not double-cover with z-fighting.

### 12.3 Perceptual questions

- Can a new player point to where open country becomes managed approach?
- Can they identify the gateway without UI?
- Does the transition remain readable with color removed?
- Is the destination visible often enough to orient but not so constantly that travel is flat?
- Does each entrance belong to the same city family while retaining a geographic identity?
- Does the road remain the clearest navigational line at night and in bad weather?
- Do outlines help object separation without turning roads, grass, and terrain into black nets?

### 12.4 Performance guardrails

- generate road transition meshes when the streamed road cell changes, not every frame;
- batch static roadside modules by material where practical;
- use distance-based density and LOD for props, decals, vegetation, and ink;
- cap threshold props per entrance;
- measure decals and outline passes on target hardware;
- compare CPU route/mesh generation time, vertex count, draw calls, GPU time, and memory before and
  after each phase;
- do not enable Bevy's experimental occlusion culling on faith—the official 0.16 notes explicitly
  say it can be slower for small/simple scenes and should be measured.

## 13. Common failure modes to avoid

- Merely changing `PAVING_ARRIVES` from 34 m to a larger number without changing other cues.
- Five clearly visible, evenly spaced material stripes.
- A stone kerb color painted flat on a shoulder and treated as completed geometry.
- Random props at constant spacing down every road.
- Uniform decals or noise used as a substitute for wear logic.
- Every town entrance using the exact same gate object and composition.
- Every entrance using unrelated random pieces, destroying settlement identity.
- Paved and dirt meshes independently deciding which route segments exist.
- Road curves generated by arbitrary wobble instead of terrain and purpose.
- Perfect circular settlement boundaries.
- Black outlines on both edges of every road, every grass blade, every stone, and every window.
- Strong AO plus strong outlines, producing doubled black creases.
- Fog used to hide emptiness or streaming rather than designed atmospheric depth.
- Evaluating roads only from the aerial debug camera.
- Adding microdetail before the approach silhouette, cross-section, and route continuity work.

## 14. Priority list by visual return

1. Fix continuous inter-settlement route geometry through city entrances.
2. Replace radial, 34 m, color-dominant arrival with a route-relative multi-channel corridor.
3. Add a width taper and three readable cross-sections.
4. Add one conspicuous, settlement-specific gateway event.
5. Ramp roadside land use and vegetation maintenance before dense frontage.
6. Establish visible road hierarchy across the world.
7. Compose landmark reveals from each real approach direction.
8. Add cause-based road decals and wetness only after macro structure works.
9. Introduce selective category-based outlines; never outline roads globally.
10. Unify wind, foliage, smoke, particles, and weather direction.
11. Add restrained aerial perspective and distance ink fade.
12. Improve shore, drainage, and surface response so infrastructure belongs to terrain.

## 15. Research sources and practical takeaways

### Roads, networks, and settlement transitions

- [FHWA: Setting Transition Zones](https://highways.dot.gov/safety/speed-management/speed-management-eprimer-rural-transition-zones-and-town-centers/4-setting) — separates rural, perception/reaction, deceleration, and community zones; explains why short or poorly signaled transitions fail.
- [FHWA: Transition-zone countermeasures and gateways](https://highways.fhwa.dot.gov/safety/speed-management/speed-management-eprimer-rural-transition-zones-and-town-centers/5) — documents gateways, landscaping, apparent narrowing, kerb/gutter versus ditch conditions, and layered roadside cues.
- [FHWA: Speed management for rural road owners](https://highways.dot.gov/safety/local-rural/speed-management-manual-local-rural-road-owners/3-identifying-countermeasures) — defines gateways as combined geometry, surface, marking, structure, and other identifiable features rather than a sign alone.
- [NACTO: Gateway](https://nacto.org/publication/urban-street-design-guide/street-design-elements/curb-extensions/gateway/) — shows a gateway as a physical transition into a slower street and links it to greenery, street furniture, visibility, and stormwater treatment.
- [Galin et al.: Authoring Hierarchical Road Networks](https://perso.liris.cnrs.fr/egalin/Articles/2011-network.pdf) — road hierarchy, terrain-aware metrics, path merging, junctions, and parameterized geometry.
- [Emilien et al.: Procedural Generation of Villages on Arbitrary Terrains](https://perso.liris.cnrs.fr/egalin/Articles/2012-villages.pdf) — roads and buildings co-evolve; interest maps, route reuse, terrain adaptation, and parcel/building relationships matter.
- [Parish and Müller: Procedural Modeling of Cities](https://people.eecs.berkeley.edu/~sequin/CS285/PAPERS/Parish_Muller01.pdf) — establishes the importance of road patterns and environmental/population influences in city generation.

### Composition and travel

- [Nintendo Dream: Breath of the Wild triangle-rule/field-design report](https://www.ndw.jp/post-1121/) — documents occlusion, reveal, path choice, scale hierarchy, and reviewing actual player routes rather than only plans.
- [Guerrilla/GDC: Creating Environmental Puzzles in Horizon Forbidden West](https://media.gdcvault.com/gdc2024/Slides/GDC%2Bslide%2Bpresentations/Wewerinke_Daniel_CreatingEnvironmentalPuzzles.pdf) — road-based sightlines, framing, hiding/focusing, light/dark, and parallax reveal.
- [GDC: Genshin Impact—Crafting an Anime-Style Open World](https://www.gdcvault.com/play/1027539/-Genshin-Impact-Crafting-an) — relevant production reference for composition and a stylized open-world pipeline.

### Outlines and cel-shaded form

- [Arc System Works: Guilty Gear Xrd art-style presentation](https://www.ggxrd.com/Motomura_Junya_GuiltyGearXrd.pdf) — artist-controlled normals, silhouettes, lighting, and line treatment for a 3D cel-shaded production.
- [Arc System Works official presentation page](https://www.arcsystemworks.com/guilty-gear-xrds-art-style-the-x-factor-between-2d-and-3d-talk-from-gdc-2015-is-now-available-online/) — official context for the Xrd production presentation.
- [Mitchell, Brennan, and Card: Real-Time Image-Space Outlining for Non-Photorealistic Rendering](https://www.npcglib.org/paper.php?entryid=249) — distinguishes image-space silhouette, shadow, and texture-boundary signals.
- [Kalnins et al.: Coherent Stylized Silhouettes](https://pixl.cs.princeton.edu/pubs/Kalnins_2003_CSS/index.php) — establishes temporal coherence as a core requirement for stylized outlines.

### Vegetation, motion, rendering, and atmosphere

- [PlayStation: Crafting the World of Tsushima](https://blog.playstation.com/2020/07/09/crafting-the-world-of-tsushima/) — controlled biome species, pushed color, reduced noise, negative space, and procedural art direction.
- [PlayStation: Ghost of Tsushima VFX](https://blog.playstation.com/?p=345372) — unified wind direction, layered gusts, foliage/cloth/smoke cohesion, and biome-aware ambient particles.
- [Guerrilla: GPU-based Procedural Placement in Horizon Zero Dawn](https://www.guerrilla-games.com/read/gpu-based-procedural-placement-in-horizon-zero-dawn) — large-scale placement driven by masks, constraints, and GPU techniques.
- [Bevy 0.16 official release notes](https://bevy.org/news/bevy-0-16/) — exact project-version reference for decals, atmosphere, bloom, GPU-driven rendering, and the limitations/measurement requirements of experimental occlusion culling.

## Final direction to Claude

Treat the road as the settlement's first room. Its approach should tell the player—through surface,
cross-section, maintenance, drainage, vegetation, habitation, lighting, and composition—that the
world is becoming a place before a city street appears underfoot.

For the present complaint, first eliminate any route gap caused by midpoint segment
classification. Then replace the 34 m radial color blend with a longer route-relative arrival state.
Prove the change in grayscale from player height before adding decals or ornamental props. Once the
geometry and scene grammar communicate arrival, the existing material, lighting, and semi-cel
shading systems will have something coherent to describe.
