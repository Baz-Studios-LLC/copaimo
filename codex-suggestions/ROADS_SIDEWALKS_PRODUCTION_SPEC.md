# Roads and sidewalks that read correctly

## A production specification for Claude

This is a visual and structural reset, not another tuning pass on the existing constants.

Copaimo's recent road work has solved several real correctness failures: country-to-city width continuity, a shared analytical cross-section, real kerb height, mesh/walk agreement, fixed-size cobbles, and better junction detection. The remaining visual problem is deeper. The generator still treats a sidewalk as a few colored stations inside one terrain-draped ribbon, then asks color contrast and an exaggerated height to make it read as constructed streetscape.

That is why individual fixes can be arithmetically correct while the result still looks wrong.

A convincing street is not one strip. It is a coordinated system:

1. graded carriageway;
2. crown or crossfall;
3. gutter or drainage edge;
4. kerb face;
5. kerb top;
6. pedestrian clear path;
7. furniture/buffer space;
8. frontage and doorway connection;
9. an outer tie into lots and terrain;
10. intersections that own their corners and curb returns.

The immediate recommendation is to stop tuning `KERB_RISE`, colors, and fade distances until one controlled straight street is rebuilt from that full profile and viewed with correct normals. Once that street reads, reuse its data at bends, transitions, and nodes.

---

## 1. What the research says

### Sidewalks are zones, not one band

FHWA describes a mixed-use sidewalk as four distinct zones: frontage, pedestrian, furniture, and curb/edge. The clear pedestrian zone remains unobstructed; lamps, trees, signs, benches, and utilities belong in the furniture zone, while entrances and building-related activity belong in the frontage zone. NACTO uses the same principle and recommends a clear pedestrian through zone of about 1.5–1.8 m at minimum, with more space or an added buffer where moving traffic directly abuts it.

For Copaimo, the important lesson is not literal modern compliance. It is visual grammar. A sidewalk reads as a sidewalk when it has a clear walking line, a protected street edge, and a relationship to doors and façades. A two-metre strip with lamps, benches, trees, signs, and people all placed inside it is not a sidewalk; it is an obstacle band.

Sources:

- [FHWA: sidewalk frontage, pedestrian, furniture, and curb zones](https://highways.dot.gov/safety/speed-management/traffic-calming-eprimer/module-2-traffic-calming-basics)
- [NACTO: Sidewalk Design](https://nacto.org/publication/urban-street-design-guide/street-design-elements/sidewalks/sidewalk-design/)
- [NACTO: Sidewalks](https://nacto.org/publication/urban-street-design-guide/street-design-elements/sidewalks/)

### Curbs are geometry, drainage, and boundary

FHWA identifies curbs as drainage and delineation elements, not merely colored borders. Paved roads commonly use roughly 1.5–2.5% crossfall, while unpaved low-volume roads use a stronger crown—FHWA's gravel guidance targets roughly 4–6%—to shed water. Conventional urban gutters commonly occupy about 0.3–1.0 m beside the curb.

For the game, this means a sidewalk should not rise directly from the middle of an otherwise flat road texture. The eye expects a small low channel or darker drainage seam, then a kerb face, then a raised pedestrian surface. Even a stylized 25–35 cm gutter strip will make the boundary more intelligible than another increase in kerb height.

Sources:

- [FHWA: typical paved-road cross slopes and curb functions](https://highways.dot.gov/safety/other/road-diets/road-diet-informational-guide/4-designing-road-diet)
- [FHWA HEC-22: conventional curb and gutter sections](https://www.fhwa.dot.gov/engineering/hydraulics/pubs/10009/10009.pdf)
- [FHWA: gravel road crowns and drainage](https://www.fhwa.dot.gov/clas/ctip/unpaved_roads_dust/ch_6.aspx)

### Crossings need intentional flush transitions

The U.S. Access Board's public-right-of-way rules provide useful geometric discipline even for a fictional setting: curb-ramp runs are generally limited to 1:12, blended transitions to 1:20, clear ramp width is at least 1.22 m, grade breaks are flush, and landings are required when direction changes. These numbers need not be copied literally into the art style, but they show the required topology: a pedestrian route cannot simply collide with an uninterrupted curb at every junction.

Copaimo needs authored crossing points—dropped stone, a short ramp, a raised crossing, or a shared-street flush area—not a universal ability to step up anywhere standing in for missing sidewalk design.

Source:

- [U.S. Access Board: PROWAG curb ramps and blended transitions](https://www.access-board.gov/prowag/technical.html)

### Roads are spline/profile systems, and intersections are separate solves

Epic's landscape-spline system separates width, side falloff, end falloff, terrain raising/lowering, mesh offset, and per-segment mesh assignment. SideFX's road generator accepts road curves with width, explicitly solves intersections, exposes intersection roundness/convexity/resolution, produces road-edge lines for sidewalks, and maintains metric UV scale.

These are useful production precedents because they separate three jobs:

- the centerline and its grade;
- the cross-section swept along it;
- the node/intersection surface built where sweeps meet.

Copaimo currently has the first two partly combined and uses a disc as an interim third. The durable version should follow the same separation.

Sources:

- [Epic: Landscape Splines](https://dev.epicgames.com/documentation/en-us/unreal-engine/landscape-splines-in-unreal-engine)
- [Epic: Creating Roads and Pathways](https://dev.epicgames.com/documentation/en-us/fortnite/creating-roads-and-pathways-in-unreal-editor-for-fortnite)
- [SideFX Labs Road Generator](https://www.sidefx.com/docs/houdini/nodes/sop/labs--road_generator.html)

### Compact, legible intersections are better intersections

NACTO emphasizes compact nodes, tight corner radii, clear pedestrian desire lines, and curb extensions that visibly signal a change in street type. For a game, those principles improve composition as much as safety: small corner radii hold the street wall together, shorten the empty paved space, and make the sidewalk corner read as a place rather than leftover geometry.

Sources:

- [NACTO: Corner Radii](https://nacto.org/publication/urban-street-design-guide/intersection-design-elements/corner-radii/)
- [NACTO: Gateway Treatments](https://nacto.org/publication/urban-street-design-guide/street-design-elements/curb-extensions/gateway/)
- [NACTO: Complex Intersection Analysis](https://nacto.org/publication/urban-street-design-guide/intersections/complex-intersections/complex-intersection-analysis/)

---

## 2. Code-specific diagnosis of Copaimo

### Critical: every road vertex currently has an upward normal

Inside `world::town::pave`, every ribbon station does this:

```text
normals.push([0.0, 1.0, 0.0])
```

That includes the carriageway, the sloped crown, the kerb chamfer/face, the kerb top, the sidewalk, and the outer tie. The index winding now faces upward, but the supplied shading normal tells the shader that the near-vertical kerb is horizontal ground.

This is likely the single largest reason the curb keeps reading as paint, a damp line, or a color boundary despite being 22 cm tall.

The semi-cel shader can only form a distinct shaded band when the normal changes. Darkening `ROAD_KERB` compensates in color, but it cannot reproduce the lighting behavior of a face turned away from the sky and sun.

Required correction:

- duplicate vertices at hard profile breaks;
- give the carriageway top its road-plane normal;
- give the gutter its own sloped normal;
- give the kerb face an outward/upward bevel normal;
- give the kerb top and sidewalk top upward or lightly cross-sloped normals;
- do not average the kerb face into the sidewalk top;
- calculate node normals from node geometry rather than assigning all-up.

For a deliberately graphic style, use hard split normals at the bottom and top of the kerb. That produces one clean cel-shaded side band. A smooth normal across those edges will turn the curb into a rounded tube.

### Critical: the urban cross-section is still draped over terrain point by point

Each lateral road vertex currently uses:

```text
terrain.drawn_height(at.x, at.y) + cut.lift(across)
```

This samples the terrain independently beneath every point across the carriageway and sidewalk. On anything other than perfectly flat generated ground, the cross-section inherits local terrain bumps and tilt. The road is therefore a decorated terrain sheet rather than a constructed surface.

A proper street station has one controlled base grade at its centerline and a cross-section derived from that grade. Terrain is then cut, filled, or blended to meet the street's outer tie. Country tracks can conform closely to terrain; kerbed streets and sidewalks need a much stiffer plane.

Required correction:

1. compute a smoothed longitudinal grade for the road centerline;
2. at each station, build all lateral heights from that one grade plus crown/gutter/kerb/sidewalk profile;
3. grade the terrain underneath and beside the road to meet the profile;
4. use the same station/profile for drawing and traversal.

### One `paved` scalar is controlling too many unrelated events

The present `paved` amount controls or influences:

- carriageway material;
- total width;
- footway width;
- kerb rise;
- shoulder closure;
- wandering width;
- wear strength;
- cobble visibility.

These should not all arrive on the same curve. A settlement approach needs a sequence. Surface hardening can begin while the road is still rural; a drainage edge can appear before a full sidewalk; a curb should begin at a deliberate terminal rather than rising invisibly for 34 metres; the pedestrian path may exist in dirt before it becomes stone.

Keep one high-level urbanization parameter if convenient, but derive named channels with different curves and thresholds:

- `surface_made`;
- `carriageway_width`;
- `gutter_presence`;
- `curb_presence`;
- `curb_height`;
- `footway_width`;
- `footway_material`;
- `outer_tie_width`;
- `wear_amount`;
- `stone_contrast`;
- `street_furniture_density`.

### The junction disc is still structurally incapable of making sidewalks

The current disc can fill a carriageway notch. It cannot produce:

- curb-return arcs;
- corner sidewalk polygons;
- dropped crossings;
- a continuous through-street curb at a T junction;
- different footway widths on incident arms;
- one non-overlapping surface with correct UVs and normals.

This is not a matter of disc radius. A correct intersection is a node polygon solve with separately owned carriageway and sidewalk corners.

### The shader's running bond is fixed to world axes

`laid_in(in.world_position.xz, stone)` makes every cobble course align north/south and east/west. A diagonal or curved road moves beneath a stationary graph-paper pattern, so ordered stones cut across the curb at arbitrary angles.

The road already needs metric along/across coordinates. Use those for surface layout. World position can still seed per-stone tone so variation does not restart, but bond direction should follow the street tangent. Junctions need their own mapping instead of inheriting whichever arm happened to write them.

### The shader has no derivative-aware detail fade

Thin dark joints in a high-contrast cel-shaded scene will shimmer when they become smaller than a pixel. Fixed physical scale solved the shrinking-stone problem; it does not solve minification.

Use screen derivatives to widen/filter joints at distance, then fade joint contrast before the pattern becomes subpixel. Medium- and far-camera motion is required evidence; a still street-level image cannot reveal temporal aliasing.

### The current 0.55 m “cobble” is a stylized paving block

A 55 cm unit is visually closer to a slab or large set than a cobble. That may be intentional for a readable stylized game, but the pattern should then be irregular or clearly designed as large paving stones. A precise running bond of half-metre units can read as brickwork scaled up several times.

Recommended visual ranges, to be judged from the actual camera:

- irregular carriageway stones: about 0.25–0.40 m stylized width;
- large sidewalk flags: about 0.6–1.2 m modules with very low contrast;
- kerb stones: about 0.6–1.0 m lengths along the street;
- joints: roughly 4–8% of module size nearby, derivative-filtered at distance.

Do not make all three surfaces share the same bond. Carriageway, curb, and sidewalk are different construction systems.

---

## 3. Choose street archetypes before generating geometry

Copaimo should not put one continuously morphed road everywhere. Define a small family of readable street types and transition deliberately between them.

### Rural track

- 4.2–4.8 m worn width;
- strong 4–5% visual crown;
- broad soft shoulders/verges;
- no raised curb;
- dirt, gravel, wheel wear, small puddle/dust variation;
- optional separate desire path near settlement approaches;
- irregular edges and controlled width wandering.

### Village shared lane

- 4.5–5.5 m compacted earth, gravel, or rough stone;
- pedestrians share the lane;
- no continuous modern sidewalk;
- shallow drainage edge or ditch;
- short stone aprons at important doors;
- occasional one-sided raised footway near the square if composition calls for it;
- buildings and fences define the street more than curbs do.

### Town street

- 4.8–5.5 m carriageway;
- 0.25–0.35 m gutter each side;
- 0.14–0.18 m curb rise, stylized upward only if required by camera;
- 1.4–1.7 m clear footway each side on primary streets;
- secondary lanes may be shared streets instead;
- subtle frontage strip at doors and shopfronts;
- lamps and signs kept outside the clear walking line.

### City high street

- 5.5–6.2 m carriageway;
- 0.3–0.45 m gutter each side;
- 0.16–0.22 m visually legible kerb face;
- 0.15–0.25 m kerb top;
- 1.6–2.0 m clear pedestrian path;
- 0.3–0.6 m frontage allowance where buildings directly meet it;
- 0.5–0.9 m furniture pockets, bulbs, or widened sections where lamps/trees/benches occur;
- intentional crossings and corner spaces.

The current 10 m high-street width can still work as 6 m carriageway plus two 2 m sidewalks, but those 2 m bands must remain largely clear. If they also carry lamps, trees, signs, steps, and benches, the right-of-way must widen locally or use curb extensions/pockets.

### City lane

- 3.8–4.5 m carriageway or shared stone surface;
- 1.2–1.6 m footways only where active frontage needs them;
- lower curb or flush shared-street treatment;
- tighter corner radii and less empty paving;
- no need to reproduce the full high-street profile on every lane.

These values are starting ranges, not laws. What matters is that each archetype communicates a settlement hierarchy and uses a coherent construction method.

---

## 4. The target cross-section

Build the high street once as explicit bands. From center outward on each side:

| Band | Purpose | Visual/geometry rule |
|---|---|---|
| Carriageway half | carts/traffic/play path | controlled crown or 1.5–2.5% crossfall, tangent UVs |
| Gutter | drainage and visual separation | 0.25–0.45 m, slightly lower/darker, subtle grime/wetness |
| Kerb face | vertical boundary | hard normal, 0.16–0.22 m visible rise, charcoal-tinted stone |
| Kerb top | thickness and construction | 0.15–0.25 m horizontal/slightly sloped strip, own stone rhythm |
| Clear footway | uninterrupted pedestrian route | 1.6–2.0 m, restrained pattern, 1–2% crossfall toward gutter |
| Frontage/door apron | connection to buildings | 0.3–0.6 m or local patches, absorbs steps/signs/thresholds |
| Outer tie | meets lots/terrain | short controlled blend, wall/edging/grass seam where appropriate |

### Vertical profile

Use one station elevation, then construct the section:

1. centerline grade is the road's longitudinal height;
2. carriageway falls toward both gutters;
3. gutter reaches the local low point;
4. curb face rises from that low point;
5. sidewalk falls gently back toward the gutter;
6. frontage meets door thresholds and lots;
7. only the outer tie blends to terrain.

Do not lower the sidewalk by sampling a terrain dip beneath it. Do not raise one sidewalk corner because a terrain vertex happens to sit there. The street is built; the earth meets it.

### Hard and soft normal boundaries

Use split vertices so one position can carry two normals where needed:

- carriageway to gutter: soft or mildly hard depending material;
- gutter to curb face: hard;
- curb face to curb top: hard;
- curb top to sidewalk: hard material seam, nearly coplanar normals if desired;
- sidewalk to frontage: usually soft/coplanar;
- frontage to retaining wall or terrain: hard if there is a wall, soft only for soil blend.

For the semi-cel style, hard normals are a design tool. The curb face should fall into its own lighting band. That is a stronger and more stable outline than painting a black stripe on top.

---

## 5. Road grading and terrain ownership

### Separate the road surface from the terrain tie

At each longitudinal station, calculate:

- position;
- tangent;
- lateral axis;
- smoothed centerline grade;
- resolved street archetype/profile;
- along-distance in metres;
- surface/material masks.

Every cross-section vertex is `station grade + profile height`. Terrain is consulted when choosing and smoothing the grade, not separately for every lateral road vertex.

Then make the terrain meet the road using a different field:

- under carriageway/sidewalk: hold to the road subgrade;
- immediately outside: cut/fill or short retaining treatment;
- farther out: smooth terrain blend;
- where the height difference is too large: reject the route, add a retaining wall, or use a bridge/terrace rather than stretching the blend indefinitely.

### Different stiffness by archetype

- rural track: follows terrain closely; stronger crown; wide soft tie;
- village lane: mildly graded; limited local smoothing;
- town street: stable carriageway and footway plane;
- city street: strongly graded; cross-section should remain architecturally controlled;
- plaza/shared street: one deliberately level or gently draining surface.

This prevents a kerbed city road from wrinkling over the same noise a dirt path should inherit.

---

## 6. The settlement approach should be staged, not uniformly faded

The current 34 m `paved` fade is useful for material continuity, but geometry should arrive in recognizable phases.

### Suggested approach sequence

Distances are illustrative and should scale with settlement size.

#### 60–40 m outside: rural preparation

- dirt track remains dirt;
- wheel wear becomes more centered and compact;
- roadside desire path begins where buildings are visible;
- verge clutter reduces;
- no curb.

#### 40–24 m: edge definition

- carriageway width begins converging on receiving street;
- drainage ditch becomes a shallow gutter/stone edge;
- road wandering fades;
- isolated boundary stones or low posts establish the gateway;
- surface becomes gravel/rough setts, not full city cobble yet.

#### 24–10 m: pedestrian infrastructure arrives

- desire path widens into a flush stone or packed-earth footway;
- gutter becomes continuous;
- curb begins at a deliberate terminal stone or short 1–3 m ramp;
- footway reaches useful width;
- first lamps, walls, or drainage features appear.

#### 10–0 m: full urban section

- full carriageway width and fixed metric paving;
- full curb and sidewalk profile;
- buildings/plots meet the frontage band;
- gateway or first junction owns the final geometry.

### Do not grow a curb continuously from zero for dozens of metres

A curb that rises by millimetres along the entire approach does not read as construction. It reads as a numerical blend. Start it at a terminal or dropped transition, then bring it to full height over a short, visible piece.

### Separate material blend from module scale

The recent change that keeps stone size fixed and fades contrast is correct. Continue that rule:

- dirt coverage retreats;
- stone contrast/coverage increases;
- stone dimensions remain physical;
- curb modules do not stretch;
- drainage and edge geometry use their own arrival curves.

---

## 7. Proper intersections and sidewalk corners

### Required node data

Every incident arm should provide:

- centerline endpoint;
- incoming tangent;
- longitudinal grade;
- carriageway half-width;
- gutter width;
- curb face/top dimensions;
- left and right sidewalk widths;
- archetype and material;
- crossing intent;
- continuation priority for T junctions.

### Node solve

1. offset each arm to its carriageway, curb, and sidewalk boundary lines;
2. choose a curb-return radius appropriate to street class and angle;
3. trim arm ribbons back to the node boundary;
4. construct one carriageway polygon—no overlapping disc;
5. construct separate sidewalk corner polygons;
6. connect curb faces around the return arc;
7. add dropped curb/ramp geometry at crossing desire lines;
8. generate normals and metric UVs for the node;
9. stitch or share boundary vertices with each arm;
10. verify there are no duplicate coplanar triangles.

### T junction rule

The through street's far curb and sidewalk should remain continuous. The side street opens a mouth in the near curb, with two returns. A radial disc cannot express this distinction.

### Crossroads rule

Four compact corner islands/sidewalk polygons surround one carriageway node. Crossing points align with pedestrian desire lines. Avoid one giant round paved space unless the settlement intentionally has a plaza.

### Indie-safe implementation alternative

If a general polygon solver is too large right now, build a template library for:

- same-width 90° cross;
- mixed-width 90° T;
- gateway T;
- four-way cross;
- acute/obtuse fallback;
- dead-end/terminal.

Choose the template from arm count, angles, and widths, then deform it into the local frame. This is still better than a disc because each template owns curb returns and sidewalk corners. Preserve the same arm data model so a full solver can replace templates later.

---

## 8. Sidewalk continuity through the settlement

### Doorways must connect to the clear path

For every enterable building:

- resolve the actual doorway center from the plot/model contract;
- create a short apron from threshold to frontage/sidewalk;
- keep the apron within step/ramp limits;
- do not put lamps, posts, signs, planters, or benches in this line;
- ensure wall collision, decorative framing, and footing agree with it;
- test outside-to-interior and interior-to-outside.

### Driveways, yards, and alleys cross the sidewalk—not erase it

NACTO recommends maintaining sidewalk grade through driveways. The game equivalent is visually valuable: the pedestrian surface remains continuous, while the vehicle/yard material crosses or ramps to it. If every side access cuts the sidewalk down to road level, the footway becomes a sequence of unrelated slabs.

### Props belong to a furniture rhythm

Create an explicit curb/furniture zone or local widening pockets. Place:

- lamps;
- trees and guards;
- hitching posts;
- benches;
- bins/crates;
- signs;
- drains;
- bollards.

Keep the clear path empty. Align repeated elements to building bays, junctions, or a controlled spacing rhythm rather than independent random positions. Break the rhythm near important buildings and squares to create hierarchy.

### Back-of-sidewalk treatment

The rear edge should not simply dissolve into whatever is there. Choose by context:

- building frontage/apron;
- low wall;
- fence;
- planted strip;
- grass verge;
- steps/terrace;
- courtyard paving.

This back edge is what makes the sidewalk belong to the buildings rather than float beside the road.

---

## 9. Materials for a semi-cel-shaded road system

### Use material identity before noise

Each surface needs a small number of strong cues:

#### Dirt track

- warm earth hue;
- broad wheel/foot wear;
- soft irregular edge;
- stronger crown;
- sparse close detail;
- no regular masonry grid.

#### Cobbled carriageway

- darker, cooler, rough stone;
- irregular or tangent-aligned modules;
- subtle center wear and gutter darkening;
- restrained per-stone value variation;
- joints filtered at distance.

#### Kerb

- slightly different stone family or value;
- visible side face with correct normal;
- larger modules along the street;
- dark contact at base;
- top catches light separately from face.

#### Sidewalk

- lighter and quieter than carriageway;
- large low-contrast flags or nearly plain surface;
- minimal random blotching;
- slight edge wear near curb and entrances;
- no competing high-frequency pattern.

### Recommended contrast hierarchy

The eye should read, in order:

1. road-versus-sidewalk height and plane change;
2. dark curb face/contact line;
3. carriageway-versus-sidewalk value/material difference;
4. gutter and wear;
5. individual stones at close range.

If individual stones are the first thing visible, the street is noisy. If color is the only difference visible, the geometry/normals are not doing their work.

### Tangent-aligned metric UVs

Provide at least:

- `u`: distance along the road in metres;
- `v`: distance across the relevant surface in metres;
- surface ID or material mask;
- transition amount;
- optional junction blend mask.

Do not use the step index as distance; segment lengths vary. Accumulate actual metres so modules do not stretch or restart. At bends, continue `u`. At nodes, either use a node-local mapping or blend multiple mappings in a controlled area.

### Anti-aliasing and LOD

- use derivatives to filter thin joints;
- fade micro-joint contrast with distance;
- retain macro value and curb geometry farther away;
- do not replace stones with shimmering subpixel lines;
- keep silhouette/curb bands at distance while dropping stone-level variation.

---

## 10. How black outlines should be used on roads and sidewalks

Do not draw a uniform black line around every road surface. A flat road is not a silhouette object, and a black stripe along both sides will look like ink painted on terrain.

Use three different mechanisms:

### Geometry and lighting for the curb face

The primary dark line should be the curb face itself:

- correct face normal;
- darker stone value;
- contact shadow/AO at its base;
- one clean cel-light band.

Use charcoal or a palette-tinted near-black, not absolute black, so it still belongs to the scene's lighting.

### Screen-space outline at real depth/normal discontinuities

If the game has a depth/normal outline pass, let the curb's hard normal/depth edge contribute. Keep road-top texture joints out of the outline mask. Outline thickness should be screen-space stable and fade at distance before it becomes a vibrating double edge.

### Material seams for construction joints

Gutter-to-carriageway and sidewalk-to-frontage seams should be subtle material lines, not the same strength as a silhouette. Kerb module joints should be weaker than the continuous curb face.

At intersections, the curb outline must turn around actual corners. It must not cross the carriageway because one ribbon continued under a patch.

---

## 11. Debug views Claude should build before more art tuning

### Cross-section view

Render one street orthographically from the end with labels or colors for:

- carriageway;
- gutter;
- curb face;
- curb top;
- clear footway;
- frontage;
- outer tie;
- terrain.

Show exact horizontal and vertical dimensions.

### Normal view

Color faces by normal direction or draw normal vectors. The curb face must visibly differ from the sidewalk top. Any all-up curb should fail immediately.

### Ownership view

Color geometry by owner:

- arm ribbon;
- intersection carriageway;
- sidewalk corner;
- terrain tie;
- building apron.

Overlaps should be obvious. One pixel cannot belong to both an arm and a node.

### Surface-ID view

Flat debug colors for dirt, cobble, gutter, curb face, curb top, sidewalk, frontage, and terrain. This catches interpolation where a hard construction seam was expected.

### Traversal-support view

Draw the analytical surface sampled by `stands_on` over the mesh. Differences above a small tolerance should be highlighted, not printed only as numbers.

### Moving material capture

Record a short camera move at normal gameplay height past:

- straight cobbles;
- a diagonal road;
- a curve;
- an intersection;
- the dirt-to-city transition.

This is the only reliable evidence for shimmer, swimming, scale changes, and orientation.

---

## 12. Validation matrix

### Geometry cases

- straight road on flat ground;
- straight road on longitudinal slope;
- road crossing lateral slope;
- gentle curve;
- sharp curve at miter limit;
- T junction same width;
- T junction mixed width;
- four-way junction;
- gateway transition;
- dead end;
- building doorway connection;
- retaining edge where terrain falls away.

### Street types

- rural track;
- village shared lane;
- town main street;
- city high street;
- city lane;
- plaza/shared street.

### Evidence angles

- cross-section end view;
- low curb-height view;
- normal gameplay camera;
- overhead topology view;
- night with lamp grazing across the curb;
- overcast diffuse light;
- moving medium/far camera.

### Mechanical assertions

- no inverted triangles;
- no zero-area triangles;
- no coplanar overlap between arms and nodes;
- profile-band widths equal the resolved section;
- curb face normals are not all-up;
- sidewalk clear width remains above its archetype minimum;
- all doorway aprons connect to the clear path;
- curb ramps/crossings remain traversable at every controlled frame rate;
- forbidden slopes/ridges remain blocked;
- mesh support and analytical support agree;
- UV scale remains metric across different segment lengths;
- detail contrast fades before joints become subpixel.

The proposed deterministic playtest driver should eventually run the traversal half of this matrix through the production warden.

---

## 13. AAA and indie production standards

There is no single “AAA road standard.” The meaningful difference is tool depth and content scale, not whether the fundamentals apply.

### AAA-style pipeline

- editable centerline graph;
- road-class profiles and per-arm attributes;
- constrained longitudinal grading;
- general node polygon solver;
- separate carriageway, curb, sidewalk, and terrain-tie surfaces;
- tangent UVs plus node-local/blended mappings;
- decals/masks for wear, drainage, repairs, and crossings;
- automatic terrain cut/fill and retaining conditions;
- LOD, chunking, occlusion, and material filtering;
- validation views, automated topology checks, and golden captures;
- artist overrides at hero streets and landmarks.

### Strong indie pipeline

- the same centerline/profile/node data model;
- three to five street archetypes;
- a small intersection-template library instead of a general solver;
- one shared shader with surface IDs and metric coordinates;
- limited prop sets with clear placement zones;
- deterministic evidence scenes rather than a large editor suite;
- manual overrides only for the square, guild hall, ranch approach, and other hero locations.

The indie compromise should reduce variety and automation, not collapse road, curb, sidewalk, and intersection into one ribbon again.

---

## 14. Recommended implementation sequence for Copaimo

### Phase 0 — Freeze appearance constants

Do not adjust curb height, palette, stone contrast, or transition distance while the geometry still supplies all-up normals and terrain-draped sidewalk vertices. Those adjustments cannot be judged yet.

### Phase 1 — One reference street

Build a straight isolated city high street with:

- explicit carriageway/gutter/curb-face/curb-top/sidewalk/frontage bands;
- one station grade and controlled crossfall;
- split normals at hard edges;
- tangent metric UVs;
- no terrain noise under the constructed surfaces;
- analytical traversal from the same profile.

Approve it in cross-section, low view, gameplay view, night light, and normal-debug mode.

### Phase 2 — Replace `RoadSection` with a richer profile result

`RoadSection` is the right ownership idea. Expand it so it returns named band boundaries and heights rather than asking `pave` to reconstruct stations around `carriage`, `half`, and `shoulder`.

Conceptually:

```text
StreetProfileSample
  carriageway edge
  gutter edge and low point
  kerb foot
  kerb top inner/outer
  clear-walk inner/outer
  frontage outer
  terrain-tie outer
  height at every boundary
  surface ID at every band
  hard/soft normal boundary flags
```

The mesh and `stands_on` consume the same sample. The mesh should not contain an independent list of magic stations.

### Phase 3 — Grade the centerline

Create a smoothed centerline grade and build each cross-section from it. Keep rural and urban stiffness separate. Add terrain cut/fill outside the constructed profile.

### Phase 4 — Staged settlement approach

Split the single `paved` control into the named channels above. Build one visible curb terminal and one footway arrival. Test it before propagating to every settlement.

### Phase 5 — Intersection ownership

Replace discs first at one T junction, then one four-way node. Build arm trimming, carriageway node polygon, curb returns, sidewalk corners, and crossing points. Only then generalize or template the remaining angles.

### Phase 6 — Material refinement

Add tangent metric coordinates, road-relative stones, kerb modules, derivative filtering, gutter wear, and distance fades. Tune palette only after the normals and geometry are stable.

### Phase 7 — Streetscape zones

Move lamps, trees, benches, signs, and drains into furniture pockets. Connect doorways through frontage aprons. Preserve a continuous clear walking path.

### Phase 8 — Automated proof

Run the deterministic character driver through:

- country-to-city approach;
- kerb at several angles and frame rates;
- T junction and crossroads;
- every enterable doorway;
- bridge and canyon negative tests.

---

## 15. Definition of done

The road/sidewalk system is not done when a screenshot finally looks acceptable from one angle. It is done when all of these are true:

1. the curb reads as a distinct face in neutral daylight without relying on a painted black stripe;
2. a low view, gameplay view, and night lamp all describe the same geometry;
3. sidewalk tops are controlled constructed planes, not laterally draped terrain;
4. carriageway, gutter, curb, sidewalk, frontage, and terrain tie have explicit ownership;
5. the pedestrian clear path is continuous and free of props;
6. every important doorway connects to it;
7. road modules keep metric scale and follow the road direction;
8. material detail does not shimmer at medium/far distance;
9. dirt-to-city change occurs as a staged construction sequence;
10. T and four-way intersections have real curb returns and sidewalk corners;
11. no arm/node coplanar overlap remains;
12. mesh and traversal surfaces agree;
13. traversal decisions do not change with frame rate;
14. rural track, village lane, town street, city high street, and city lane are visually distinct;
15. the system passes both debug views and normal gameplay captures.

The immediate highest-value change is not another curb constant. It is correct split normals and one stable, graded, explicitly banded reference street. That will reveal which remaining problems are geometry, which are material, and which are composition. Until then, every screenshot is asking color to explain a shape the mesh and shader are not actually describing.
