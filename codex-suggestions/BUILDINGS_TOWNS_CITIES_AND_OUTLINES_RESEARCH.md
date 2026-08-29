# Production research: buildings, settlements, cities, and ink outlines

Audience: Claude, for Copaimo implementation decisions  
Prepared: 2026-08-29  
Status: research and recommendations only; Codex did not alter game code or assets

## Executive conclusion

There is no single formal “AAA standard” or “indie standard” for generating a town. There is a shared quality bar, and two different production strategies for reaching it.

- **AAA production** normally combines procedural layout, modular kits, artist-authored landmarks, local overrides, streaming/LOD work, and automated validation. The generator is a tool for artists and designers, not an unattended content slot machine.
- **Strong indie production** narrows the possibility space: fewer architectural families, fewer settlement types, deterministic seeds, compact kits, high-impact hero assets, simple collision, aggressive reuse, and screenshot-driven testing. It spends authorship where the player will notice it.
- **Both** succeed when a place is legible, traversable, coherent, useful to gameplay, performant, and apparently inhabited. They fail when random variation replaces composition.

Copaimo already has more of the correct foundation than a generic research document would assume: road-first layout, parcels/lots, districts, stable seeds, player-height evidence, real building interiors, a modular building grammar, semantic building types, landmarks, streaming, collision checks, and a shared four-band near-cel material. Do not discard those systems.

The next production layer should be:

1. Turn every retained or omitted lot into an intentional **plot program**: building, yard, garden, work area, stall, service area, civic space, or deliberate breathing room.
2. Give streets a semantic hierarchy and a readable arrival sequence.
3. Move variation upward from independent cosmetic rolls to settlement-level “genes” that make a whole place belong together.
4. Preserve an authored override path for landmarks, entrances, squares, and story-critical routes.
5. Add outlines as a selective **ink system**, not as a universal edge filter. For Copaimo, prototype a screen-space depth/normal pass first, then use inverted hulls only for characters or hero objects that require hand-tuned line weight.

The highest-value principle is this:

> Generate structure, author emphasis, validate the result at player height.

## 1. What the industry evidence actually says

### 1.1 City generation is hierarchical

The classic Parish–Müller city system is not “scatter buildings and connect them.” It generates a transportation network under global goals and local constraints, divides land into lots, and then creates suitable building geometry. The later CGA building work separates mass, storeys, façade bays, and components into a shape grammar. Epic’s City Sample likewise begins with a city boundary, arterial splines, zones, block/lot processing, buildings, and finally city furniture.

That gives a dependable hierarchy:

1. World context and site
2. Approach routes and arterial structure
3. Street network and public-space nodes
4. Blocks/parcels
5. Frontage lots
6. District and land-use program
7. Building mass
8. Façade grammar
9. Plot dressing and street furniture
10. Gameplay, navigation, collision, streaming, and validation

Each layer should consume semantic information from the layer above. A shop is not merely a cottage chosen by a different random number; it belongs on trade frontage, wants a readable door and sign, produces a different street edge, and implies nearby activity.

Copaimo already correctly owns layers 2–8 across `src/world/town.rs` and `dev/art/town.py`. The conspicuous gap is layer 9, plus more explicit street hierarchy in layers 2–3.

### 1.2 Modularity is a production system, not just a grid

Bethesda’s Fallout 4 modular-level-design presentation frames kits and iterative level design as how a relatively small content team built a very large world. SideFX’s production building generator turns blockout volumes into floors, wall faces, corners, ledges, and artist-supplied modules, but it also includes floor-specific and hand-placed overrides. Epic’s artist-driven procedural-building talk similarly emphasizes split/repeat rules, occlusion tests, edge-angle and height tests, trim, roofs, non-rectangular faces, and LOD generation.

The standard pattern is therefore:

- A common measurement system and reliable pivots
- A small set of semantically named pieces
- Rules that know corners, floors, openings, roofs, and occlusion
- Preview/debug views
- Artist overrides that do not require destroying the generator
- A cheap proxy/LOD path

Copaimo’s 1.5 m module, measured footprints, grounded origins, real openings, and export gates are aligned with this. The next improvement is not an explosion of new modules. It is to expose more semantic variation through the existing grammar and to keep the variation correlated by place.

### 1.3 Procedural output still needs art direction

Red Hook’s Darkest Dungeon 2 production account explicitly describes an art-led generator and tools that let artists control the system. Ubisoft’s Ghost Recon Wildlands pipeline used procedural layers for roads, terrain work, vegetation, village terraforming, and building placement; it did not imply that every layer was visually acceptable without review. Sucker Punch’s inFamous talk describes footprint standardization and layout shortcuts, while also saying individual streets need high-profile elements for identity and navigation.

The common AAA compromise is:

- Procedural systems fill the broad field.
- Semantic rules keep the field coherent.
- Artists author or override high-salience moments.
- Automated reports find broken content.
- Human review judges composition.

For an indie project, “artist” may mean one developer and “tool” may be a Rust data structure plus a nine-shot matrix. The production logic remains the same.

### 1.4 Validation is part of generation

Ubisoft’s Assassin’s Creed Origins world-data talk describes daily automated tests and visual reports because open-world art, AI, design, and generated content are too interdependent to validate manually. Copaimo’s invariant tests and shot matrix are already moving in this direction.

The implication for Claude is important: a generator feature is incomplete until there is a cheap way to prove it across seeds and at player height.

## 2. A production architecture suited to Copaimo

Do not make one enormous `lay_out` function responsible for every visual decision. Keep a staged intermediate representation whose records explain why something exists.

### 2.1 Settlement profile: the place’s shared DNA

Create or conceptually maintain a settlement-level profile before individual plots are resolved. It should be deterministic from the site and stable named seed channels.

Suggested fields:

- Settlement kind: ranch, hamlet, village, guild town, modern city
- Economy or role: farming, herding, craft, trade, administration, transport
- Architectural family: roof pitch range, massing rules, framing style, civic motifs
- Material palette: wall, structure, roof, trim, paving, accent
- Age and prosperity: controls repair, additions, formal frontage, clutter, vacancy
- Density curve: center, intermediate ring, edge
- Street language: widths, surfacing, shoulders, kerbs, lamps, trees
- Landmark family and skyline target
- Vegetation and plot-dressing family
- One or two exceptions that give the place identity

The goal is covariance. If prosperity rises, several related features should change together: more paved frontage, repaired roofs, stronger shop signs, denser market furniture, fewer broken fences. Independent random switches produce variety but not identity.

### 2.2 Stable named randomness

Use separate deterministic streams or hashes for logically separate decisions:

- `roads`
- `districts`
- `plot_programs`
- `building_masses`
- `facades`
- `dressing`
- `landmarks`

Adding a flowerpot variant must not rotate every house or move a landmark because it consumed one more random number. Derive a choice from stable keys such as `(world_seed, site_id, plot_id, "roof")` rather than a mutable global sequence. This is one of the cheapest ways to keep procedural work reviewable.

### 2.3 Street classes, not one road with different paint

Suggested semantic classes:

- **Approach/main street:** connects the world route to the main node; highest storefront and landmark priority.
- **Secondary street:** connects districts and secondary nodes.
- **Local lane:** gives residential/workshop frontage.
- **Service lane or alley:** narrow, low-status, connects backs and work areas.
- **Footpath:** informal desire line through yards, gardens, or edge spaces.

These should differ in more than width. Vary surface, shoulder/kerb, setback, frontage occupation, furniture, vegetation, and lighting. A player should identify the main route without a map.

For Copaimo, introduce the classification before adding many meshes. Existing paving can first use class-driven width and color. Dressing can follow later.

### 2.4 Plot programs: make empty ground intentional

Every lot or omitted lot should resolve to a purpose, even when it contains no building:

- Residence
- Shop or stall
- Workshop with outdoor work yard
- Civic building or square
- Kitchen garden/orchard
- Animal pen
- Storage or service yard
- Small green, shrine, well court, or seating
- Construction/repair site
- Intentional empty buffer around a landmark

This is the direct answer to Copaimo’s visible empty tan parcels. Do not solve them only by raising `HOUSES_IN_A_VILLAGE`. The current decision to leave air can be correct; the missing piece is evidence of use.

Each program needs a small kit and two or three compositions, not dozens of unique props. A workshop composition might be a lean-to, bench, stacked material, barrel, and beaten-earth patch. A garden might be a low fence, rows, shed, tree, and path to the door. Those clusters read as authored because their parts imply a relationship.

### 2.5 Constraint, fallback, and provenance

Every placement rule should have:

- A bounded number of attempts
- A clear constraint set
- A deterministic fallback
- A debug reason when it fails

For example, a garden that cannot fit should become a smaller yard, not silently vanish. A shop that cannot obtain road frontage should downgrade to a residence or move to the next eligible lot. A landmark that cannot obtain skyline clearance should fail a test loudly.

In debug views, color by street class, district, plot program, building family, failure/fallback state, and collision footprint. The generator should be able to answer “why is this here?”

### 2.6 Authored overrides are a required feature

Provide a small site-specific override record rather than branching deeply on coordinates. It may specify:

- Force or forbid a plot program
- Reserve a view corridor
- Replace a generated building with a hero building
- Fix a landmark position or facing
- Add a square, gate, bridge approach, or story prop cluster
- Preserve an entrance sequence

An override is not a failure of procedural generation. It is how procedural generation becomes production art.

## 3. Building design: the quality bar

### 3.1 Design in three visual scales

Copaimo already documents large, medium, and small shape. Keep that model as a formal acceptance test.

- **Large scale, read from approach:** footprint, height, roof shape, tower, wing, courtyard, silhouette.
- **Medium scale, read across a street:** porch, jetty, dormer, chimney, buttress, balcony, shopfront, canopy, service lean-to.
- **Small scale, read at the door:** frame, sill, sign, hinge, planter, courses, shingles, stock, threshold wear.

If two buildings differ only at the small scale, they are the same building from normal play distance. Variation should be spent first on mass and silhouette, then on façade rhythm, and last on decoration.

### 3.2 Use semantic massing, not arbitrary noise

A believable building mass follows purpose and construction:

- Cottage: one primary volume, perhaps one later addition; chimney near hearth; garden/service side.
- Shop: strong public front, display/opening, sign/canopy, storage or residence behind/above.
- Workshop: broad access, work yard, lean-to, stacks and ventilation/chimney appropriate to craft.
- Civic hall: larger approach, axial or deliberately framed entrance, public forecourt, skyline feature.
- City block: base/shaft/top, active ground floor, repeated middle, termination at roof.
- Tower/spire: lower mass that meets the street, a legible shaft, and a crown visible against sky.

This is why SideFX exposes floor overrides and localized overrides: the ground floor, corner, roof, and hero entrance are not interchangeable repeated bays.

### 3.3 Façade grammar rules

For each frontage:

1. Identify public front, side, rear/service side, and party wall.
2. Reserve the entrance and protect its path to the street.
3. Establish structural bay rhythm.
4. Place openings according to interior/storey logic.
5. Treat corners explicitly.
6. Add base and top termination.
7. Add a controlled exception.

Good variation changes a pattern while keeping its grammar. Bad variation rolls every window, material, roof, and prop independently.

Recommended Copaimo variation levers, in priority order:

- Footprint proportions and secondary mass
- Storey count within district limits
- Roof orientation/pitch/profile
- Entrance or shopfront composition
- Bay count and grouped opening rhythm
- One medium silhouette feature
- Palette variant from the settlement family
- Small dressing cluster

Keep window and trim dimensions modular so pieces still align. Use palette families rather than unrestricted RGB perturbation.

### 3.4 Front, side, back, and ground contact

Generated buildings often look like props because every side has equal importance and the model meets the terrain with no occupation.

- **Front:** door, sign, path, threshold, public-facing detail.
- **Side:** fewer openings, drainage, storage, small addition, fence return.
- **Back:** service door, bins/barrels, fuel, washing, work surface, private yard.
- **Ground contact:** foundation/plinth, dirt or paving response, drainage strip, doorstep wear, vegetation suppression.

The ground-contact pass matters as much as another façade variant. It makes the building look built on the site rather than spawned onto it.

### 3.5 Interiors and closed buildings

Copaimo’s policy that essential buildings and some homes open while most doors remain shut is production-sensible. Maintain a clear visual contract:

- Openable/enterable: stronger approach path, readable threshold, slightly brighter or revealing interior, interaction affordance consistent with the game.
- Closed but inhabited: complete door and signs of life, but no invitation that looks mechanically identical to an enterable door.
- Pure background shell: use only where the player cannot reasonably reach it, otherwise the mismatch will be noticed.

Do not let outline thickness become the only signal for interactivity; line art should describe form, not replace game-language cues.

### 3.6 Architectural families

An architectural family needs a “shape language sheet,” even if it is expressed as constants:

- Module and storey height
- Typical width/depth ratios
- Roof pitch and allowed profiles
- Wall-to-roof color/value relationships
- Structural pattern
- Opening proportions and rhythm
- Foundation and eave treatment
- One civic motif
- Allowed exception list

Within one town, hold most of these constant and vary a minority. Between towns, change several together. That creates place identity more efficiently than making every house unique.

### 3.7 Hero assets and repetition budget

Use unique or heavily overridden assets for:

- Arrival landmark
- Primary civic/guild destination
- Central node
- Important shop or story location
- Skyline anchor

Ordinary buildings can repeat, but avoid obvious immediate sequences: identical silhouette, palette, and orientation should not occur on adjacent lots unless the intended style is a terrace. Measure recurrence along a player path, not only total variety in the settlement.

## 4. Town and city design: from plan to experience

### 4.1 The player experiences a sequence, not a plan view

Design and validate these beats:

1. **Distant read:** a skyline or landmark says a settlement exists.
2. **Approach:** roads, fields, fences, traffic signs, outbuildings, or vegetation changes announce its influence.
3. **Threshold:** ground material, walls, gate, bridge, compression, or a change in frontage says “you have arrived.”
4. **Reveal:** the first meaningful view presents a node or landmark.
5. **Choice:** the street structure offers two or three understandable directions.
6. **Destination:** important doors and spaces are visually distinct.
7. **Exit:** routes back to the world remain easy to recover.

Copaimo’s new settled-ground transition solves much of threshold. The next pass should strengthen approach influence and the first reveal.

### 4.2 Legibility and identity

Lynch’s paths, nodes, landmarks, districts, and edges remain useful because they describe how people build mental maps. Copaimo already encodes all five conceptually. The production test is whether they are perceptually distinct at player height.

- Paths must differ by destination and class.
- Nodes need open space plus a reason to gather.
- Landmarks need silhouette, contrast, and view corridors.
- Districts need clustered visual rules, not merely enum labels.
- Edges need a perceptible change, not necessarily a wall.

Sucker Punch’s inFamous team used signage, infrastructure, damage, “weenies,” shoreline, and parks to keep streets from feeling repetitive and to aid navigation. Copaimo can do the same at smaller scope with a unique tree group, market canopy, guild banner, chimney cluster, wall type, or road furniture family.

### 4.3 Functional adjacency

Believability comes from relationships:

- Shops seek main flow and nodes.
- Workshops want access and outdoor work area, often near but not at the ceremonial center.
- Homes cluster on quieter streets.
- Gardens and animal uses become more common at the edge.
- Civic buildings own forecourts or view corridors.
- Service uses face backs or alleys.
- Inns/stables belong near arrival routes.

These do not need simulation. A small adjacency score is enough. Rules that encode why uses sit together create more believable output than more decorative meshes.

### 4.4 Density is frontage occupation, not just building count

Control density through:

- How continuously the street edge is occupied
- Setback
- Building width and party-wall behavior
- Number of storeys
- Frequency of vacant/green/work plots
- Public-space size
- Prop and pedestrian density

A city can contain 34 buildings and still read sparse if each is isolated in a large tan field. A village can read dense with 12 buildings if fences, gardens, lean-tos, trees, and yards carry the frontage between them.

Use district-specific street-wall targets. For example, a market street should maintain a high percentage of visually occupied frontage; outskirts should deliberately break into gardens and yards.

### 4.5 Skyline composition

The skyline should have:

- A broad base mass
- Secondary peaks or roof rhythm
- One dominant anchor
- Clear negative sky around the dominant anchor from intended approaches

Do not let random towers compete evenly. Copaimo’s recent spire/clearance logic is the correct direction. Validate dominance from several approach samples, not just by comparing model heights.

### 4.6 Public space must have use and edges

A square is not a large empty patch with an object in its center. It needs:

- An enclosing edge or strong frontage on most sides
- Multiple entries with one dominant arrival
- A focal object offset or centered intentionally
- Activity zones: stalls, seating, queueing, announcements, water, shade
- Clear traversal lanes
- Enough open space for gameplay and camera movement

Treat the landmark’s protected space as programmed public ground, not merely “no towers within radius.”

### 4.7 Gameplay-first urbanism

Every settlement pass should ask:

- Can the player and trailing camera enter every intended door?
- Are paths, thresholds, and steps compatible with movement and collision?
- Can the player identify main and secondary routes without UI?
- Do corners reveal enough before collision or encounter?
- Are plazas large enough for intended interaction but not so large that they feel vacant?
- Are props outside traversal envelopes?
- Do NPC routes have destinations and recovery points?
- Can important buildings be found from multiple entry directions?

Realistic urban form is subordinate to readable play.

## 5. AAA practice versus a sustainable indie scope

### 5.1 AAA-oriented pipeline

Typical expectations:

- Interactive generator with immediate preview
- Modular kit validation and naming contracts
- Multiple architecture/style sets
- Local and hand-placed overrides
- Semantic data shared with navigation, traffic, audio, quests, and VFX
- HLOD, impostors, instancing, occlusion, streaming, and memory budgets
- Daily automated world checks and reports
- Dedicated art, design, tech-art, and QA review
- Hero passes on all critical routes

The visual result is not produced by complexity alone. It comes from specialists repeatedly reviewing the same places from different disciplines.

### 5.2 Indie-oriented pipeline

Recommended Copaimo scope:

- Two strong architectural ages/families rather than many weak ones
- Three settlement scales with shared rules
- One compact plot-dressing kit per economic role
- One hero landmark and one secondary node family per settlement kind
- Deterministic offline model generation plus runtime placement
- Simple per-site overrides in data/code
- Seed sweeps and a fixed evidence matrix
- Aggressive reuse with correlated palette/massing variation
- Simple collision proxies and conservative draw-call budgets
- Polish the ranch-to-first-guild route before broadening the world

AAA quality is best interpreted here as **clarity, finish, and consistency**, not AAA volume.

## 6. Cel outlines: what an outline is and where it belongs

### 6.1 Distinguish four kinds of line

1. **Occluding contour / silhouette:** where the visible surface turns away or an object ends against the background.
2. **Occlusion boundary:** where one object visibly overlaps another.
3. **Structural crease:** a sufficiently sharp change in surface normal, such as roof-to-wall or a deep recess.
4. **Authored interior line:** a deliberate mark that describes a seam, fold, panel, trim, facial feature, or design motif.

Research on suggestive contours demonstrates that sparse, selected lines can convey form better than silhouette alone, but this is not permission to draw every geometric edge. The relevant phrase is **sparse lines**. A triangle edge is not automatically a meaningful drawing line.

### 6.2 Where black ink should appear on Copaimo buildings

Primary ink:

- Roofline and major outer silhouette against sky or terrain
- Eaves and large overhang boundaries
- Building-to-building occlusion boundaries
- Deep door/window recess boundaries
- Strong roof/wall or tower/crown creases
- Hero landmark silhouette

Secondary ink, lighter/thinner/selective:

- Major corners that turn far enough to describe mass
- Base/plinth contact where it improves grounding
- Porch, balcony, canopy, dormer, chimney, buttress silhouettes
- Large window/door frames when their geometry creates a true depth or normal break

Usually no automatic ink:

- Every stone course, shingle, mullion, or timber strip
- Coplanar palette changes
- Every edge of road mesh or terrain triangulation
- Individual grass blades, tiny leaves, rain, clouds, water ripples, or distant litter
- Soft rolling terrain normals
- Internal polygon boundaries on curved or triangulated surfaces

Small architectural detail should be carried by geometry, value grouping, vertex color, or a deliberately authored interior-line mechanism. If the edge pass outlines every shingle, the building will turn into a black mesh at distance.

### 6.3 Line hierarchy

Start with three conceptual weights:

- **Primary silhouette:** strongest and widest
- **Structural crease/occlusion:** roughly half to two-thirds the strength
- **Decorative authored line:** controlled per asset and often absent at distance

Do not commit to world-unit widths. Judge output in pixels at target resolutions. A reasonable prototype range is approximately 1–2 output pixels for environmental silhouettes at 1080p, with structural lines narrower or lower opacity. This is a tuning start, not a standard.

Maintain a minimum coherent width after antialiasing; Unreal’s TSR guidance notes that lines thinner than about a pixel can become discontinuous. Scale the sampling radius with output resolution, then cap it so 4K does not turn every outline into a heavy band.

### 6.4 Ink color

“Black outline” should normally mean black **to the eye**, not necessarily mathematical zero.

- Start with a very dark cool or neutral ink.
- Consider blending a small amount of base color into ink, as Unity Toon Shader supports.
- Preserve true black for the strongest focal accents if desired.
- Fade/tint ink into atmospheric fog at distance.
- Reduce or tint ink in deep night so dark objects do not collapse into holes.

Suggested art-direction experiment: compare pure black, cool near-black, and material-tinted near-black in the fixed matrix. The winner should be chosen under noon, overcast, dusk, and interior lighting.

## 7. Outline implementation options

### 7.1 Inverted hull

Method: draw a second copy of the mesh, expand vertices along a controlled normal or scale direction, cull front faces, and render the back faces in ink.

Strengths:

- Clean, stable object silhouette
- Precise per-object and per-vertex control
- Variable width or complete suppression through vertex data/width maps
- Works well for characters and hero assets
- Arc System Works used it for Guilty Gear Xrd because artists could preview and control line width, including erasing lines in selected areas

Weaknesses:

- A second geometry draw for every outlined mesh
- Poor fit for thousands of small environment parts
- Split normals, hard edges, open meshes, concavity, and nonuniform scale can create spikes/gaps
- Constant world expansion produces distance-dependent screen width unless corrected in clip/view space
- Does not create interior lines by itself
- Needs deliberate depth bias and should not cast shadows

Use in Copaimo for:

- Player and important creatures
- Possibly a few hero props/landmarks
- Assets where the line must be manually sculpted

Do not start by duplicating every town, tree, and grass mesh.

### 7.2 Screen-space depth/normal outline

Method: sample neighboring depth and normal values in a fullscreen pass, detect discontinuities, then composite ink.

Strengths:

- Cost is primarily screen resolution and sample count rather than total triangle count
- Works across the whole visible scene
- Captures occlusion boundaries and structural normal changes
- Consistent pixel-space width
- Natural fit for a streamed environment

Weaknesses:

- Without masks, it outlines unwanted terrain, grass, and tiny clutter
- Depth thresholds must account for perspective/reversed depth
- Normal thresholds can expose triangulation or normal-map noise
- Transparent materials are normally absent from the prepass
- Temporal shimmer and MSAA interaction need testing
- It knows pixels, not artistic semantics

Use in Copaimo for:

- Buildings, bridges, large props, and possibly major tree masses
- General environmental ink after a category mask exists

### 7.3 Explicit authored line geometry or line data

Method: model a narrow strip/tube, encode a line through special UVs, or author a decal/mark.

Strengths:

- Complete semantic control
- Best for inner lines the silhouette methods cannot infer
- Can be art-directed around openings and hero details

Weaknesses:

- Asset cost
- Z-fighting and LOD concerns
- Texture lines can pixelate in extreme close-up unless authored carefully; Guilty Gear used UV-aligned line data to keep internal lines crisp

Use sparingly for hero façades, characters, signs, and design motifs.

### 7.4 Recommended hybrid for Copaimo

1. **Screen-space selective environment outline** for major opaque forms.
2. **Inverted hull** only for the player/creatures and exceptional hero assets.
3. **Existing geometry and vertex colors** for most architectural inner detail.
4. **Explicit authored lines** only where neither method communicates the intended form.

This gives the semi-cel world a coherent ink layer without paying a second draw for every streamed object or turning the environment into wireframe.

## 8. A Bevy 0.16 implementation brief for Claude

This section is intentionally architectural, not drop-in code.

### 8.1 Keep ink separate from `CloudShade`

`assets/shaders/cloud_shade.wgsl` currently sees one visible fragment at a time. It can band lighting or add a view-angle rim, but it cannot expand a silhouette into pixels outside the mesh and cannot compare a building with neighboring objects without screen textures.

Create an `InkPlugin` or equivalent renderer concern separate from `ShadePlugin`. The existing shared `Shaded = ExtendedMaterial<StandardMaterial, CloudShade>` should remain responsible for surface shading and deformation.

### 8.2 Use the official Bevy patterns as the starting point

Bevy 0.16.1 includes:

- `examples/shader/shader_prepass.rs` for `DepthPrepass`, `NormalPrepass`, and motion-vector textures
- `examples/shader/custom_post_processing.rs` for a fullscreen render-graph node and source/destination `ViewTarget` handling
- `MaterialExtension::prepass_vertex_shader` and `prepass_fragment_shader` hooks

Copy the architecture from the version pinned by Copaimo, not `main`, because Bevy rendering APIs move quickly.

### 8.3 Prepass correctness with Copaimo’s moving geometry

This is the most important technical trap.

`CloudShade` displaces grass and sea vertices in its main vertex shader. If the depth/normal prepass uses Bevy’s default undeformed vertex path, the edge detector will see geometry where the visible grass/water is not. That creates halos, detached lines, or missing edges.

Choose one:

- Exclude sea, grass, and other non-ink surfaces from the ink mask/prepass.
- Or provide a matching `prepass_vertex_shader` that mirrors the exact deformation.

For the recommended art direction, excluding water, grass, rain, clouds, and tiny foliage from ink is correct and cheaper. Do not duplicate deformation code unless those surfaces truly need outlines.

### 8.4 MSAA compatibility

Copaimo’s main camera currently uses `Msaa::Sample4`. Bevy 0.16’s prepass example disables MSAA for maximum compatibility and notes that a shader prepass with MSAA requires the GPU’s `MULTISAMPLED_SHADING` capability.

Therefore:

1. Prototype with MSAA off to establish correctness.
2. Measure the target GPU capability path.
3. Compare MSAA, FXAA/SMAA/TAA options supported by the pinned version.
4. Make sure the final ink itself is antialiased and temporally stable.

Do not silently trade clean geometry edges for a working outline pass.

### 8.5 Selective ink classes

A global depth/normal Sobel pass will likely outline grass and terrain noise. Production needs an inclusion mask or class buffer.

Suggested logical classes:

- 0: no ink — sky, clouds, water, terrain, roads, grass, particles, rain
- 1: environment silhouette — buildings, bridges, large props
- 2: environment silhouette plus structural creases — hero architecture
- 3: character/creature — reserved for stronger treatment or inverted hull
- 4: authored accent — explicit control

Possible implementation paths:

- A custom render phase that writes a small mask/object-class target for tagged entities
- A selected-object prepass with depth testing against the visible scene
- A material split between inkable and non-inkable surfaces if that is simpler and remains batch-friendly

The mask is not optional polish. It is the control that prevents the edge detector from outlining every blade and terrain triangle.

### 8.6 Edge equations

For each pixel, sample a small cross or 3×3 neighborhood.

- Depth signal: maximum view-space or linear-depth difference, normalized relative to center depth.
- Normal signal: maximum `1 - dot(center_normal, neighbor_normal)` among neighbors with compatible depth.
- Silhouette/occlusion mask: selected-object/class boundary, with scene depth used to prevent ink showing through foreground occluders.
- Combined edge: weighted maximum, not a sum that makes two weak noises look like one strong line.
- Final coverage: `smoothstep(low_threshold, high_threshold, edge)` followed by a small controlled dilation for primary silhouettes.

Important details:

- Do not compare raw nonlinear depth with a single world-independent threshold.
- Place an external silhouette primarily into the farther/background pixel so the visible object does not shrink.
- Keep normal-derived creases on the object surface.
- Gate normal edges by class and depth agreement so an occlusion discontinuity is not double-thickened.
- Use nearest or carefully controlled sampling for depth/normal data; filtering can invent boundaries.

### 8.7 Render ordering

The official Bevy custom-post-process example inserts its pass after tonemapping. That is a sensible first experiment because ink color is predictable in display space. It also means ink will not automatically inherit scene fog or HDR response.

For version 1:

- Detect edges from depth/normal/mask.
- Composite after tonemapping for predictable black.
- Explicitly attenuate/tint by linear depth and Copaimo’s fog treatment.
- Ensure UI/map rendering is not outlined.

Only move the pass earlier if there is a demonstrated visual need and the HDR/fog ordering is understood.

### 8.8 Distance and resolution policy

Use a screen-space target width with distance-based class fading:

- Near: full primary and structural ink.
- Mid: primary silhouette plus strongest creases.
- Far: one-pixel silhouette or no ink, letting value and atmosphere carry the form.

Base transition distances on the matrix and `SIGHT`, not arbitrary genre numbers. At 720p, 1080p, 1440p, and 4K, compare the same composition. At narrow and wide FOV, verify perceived weight.

### 8.9 Outline acceptance tests

Automated/render tests should cover:

- Building against sky, grass, another building, and deep shadow
- Door/window recess at near and street distance
- Roof shingles/courses do not become black noise
- Terrain triangles do not show
- Grass and rain remain uninked
- Player/creature remains readable against dark and light backgrounds
- No halo from displaced grass/water
- No line visible through an occluder
- Stable line while orbiting camera slowly
- 720p, 1080p, 1440p, 4K
- MSAA/AA path on supported and fallback hardware
- Noon, overcast, dusk, night, interior, and fog
- Photo matrix remains deterministic

Capture an “ink debug” view showing depth edge, normal edge, class mask, and final composite separately.

## 9. Performance standards

### 9.1 Buildings and towns

- Reuse mesh/material handles so Bevy can batch/instance where possible.
- Keep ordinary buildings to a small number of materials; vertex colors are already a strong Copaimo choice.
- Use simple collision walls/footprints rather than render-mesh collision.
- Stream at settlement granularity when practical, as Copaimo already does, and avoid per-prop systems that wake every frame.
- Establish LOD or simplified silhouette assets before the modern city grows substantially.
- Preserve the rule that decorative detail must justify its draw/triangle cost at the distance it is visible.
- Do not let outline geometry cast shadows.

### 9.2 Outline pass

Measure GPU time separately for:

- Additional prepass or selected mask phase
- Fullscreen edge pass
- Extra bandwidth for depth, normal, and mask targets
- Any character hull draws

Test worst-case views: city center, forest edge, rain, many transparent elements, and maximum view distance. Report median and high-percentile frame time, not only FPS in one quiet shot.

The screen-space pass should use the smallest neighborhood that achieves the line. Nine samples of multiple full-resolution targets can become bandwidth-heavy; start with a cross pattern and expand only when diagonals visibly fail.

## 10. Validation suite for generated settlements

Copaimo already tests many structural invariants. Extend the philosophy rather than duplicating rules.

### 10.1 Structural checks

- No building intersects carriageway, another footprint, protected landmark space, or ranch exclusion
- Every non-landmark front door faces and can reach a route
- Enterable thresholds satisfy player and camera clearance
- Every building model exists and passes export gates
- Streets remain within valid terrain grade/level treatment
- Determinism: same input produces the same semantic layout
- Stable named random streams: adding a dressing variant does not alter roads/buildings

### 10.2 Semantic checks

- Every retained and omitted lot has a plot program
- Shops/workshops/homes satisfy frontage and district rules
- Each district contains enough of its defining programs to be perceptually different
- Every city/town has a dominant landmark and at least one secondary node
- Landmark view corridors are clear from sampled approach points
- Market/public space has enclosing frontage and activity zones
- Arrival route reaches a meaningful first node

### 10.3 Repetition checks

Along each likely route, measure:

- Longest run of identical building types
- Adjacent identical silhouette/palette/roof combinations
- Repeated prop-cluster signatures
- Façade rhythm recurrence
- Percentage of frontage with no building, fence, planting, furniture, or program

The point is not to ban repetition. It is to keep it from becoming the first thing the player sees.

### 10.4 Visual reports

For a fixed set of representative seeds:

- Plan view with semantic colors
- Player-height approach, threshold, node, side street, landmark, and exit
- Skyline mask
- Plot-program overlay
- Ink mask and edge-channel views
- Performance counters

Freeze time and weather for comparison, then run separate weather-specific evidence when weather is the feature under review.

## 11. Recommended implementation order

### Phase A — settlement occupation, highest visual return

1. Add plot programs without adding many new buildings.
2. Build a minimal yard/garden/work/stall kit from existing geometric language and vertex colors.
3. Give main, secondary, and local streets semantic classes.
4. Dress one village and one city across the current matrix.
5. Add semantic and repetition tests.

Success: no large parcel reads accidentally empty; approach and main route are readable; the place looks inhabited.

### Phase B — architectural family and controlled variation

1. Add settlement-level family/profile data.
2. Separate stable random streams.
3. Add two or three high-impact mass/façade variants per ordinary building role.
4. Add site override records.
5. Validate adjacent repetition and skyline composition.

Success: buildings clearly belong to the same town without reading as clones.

### Phase C — ink prototype

1. Define an art target with three reference matrix shots.
2. Build an `InkPlugin` from pinned Bevy 0.16 examples.
3. Prototype depth-only silhouettes with MSAA off.
4. Add selective ink classes/mask.
5. Add normal-derived structural creases for hero architecture.
6. Add resolution, distance, fog, and temporal controls.
7. Compare near-black treatments.

Success: buildings and the player separate cleanly from the world, while grass, terrain, roads, rain, and small detail remain uncluttered.

### Phase D — hero hulls and authored lines only if needed

1. Inverted hull for player/creatures.
2. Per-vertex width/suppression data where silhouette quality demands it.
3. Explicit inner lines for a small number of hero assets.

Success: focal subjects have deliberate line weight without making the entire world pay the cost.

### Phase E — production hardening

1. Restore/select final AA path and capability fallback.
2. Measure GPU/CPU cost in worst-case scenes.
3. Add ink debug captures and regression shots.
4. Establish simplified distant building silhouettes/LOD if city cost requires it.

## 12. Things not to do

- Do not add random props uniformly to fill space.
- Do not solve empty parcels only by increasing building count.
- Do not give every building independently random colors and roof parts.
- Do not let a generator silently skip failed placements.
- Do not treat artist overrides as technical debt.
- Do not judge settlement quality only from plan view.
- Do not put true black around every triangle, shingle, grass blade, or road edge.
- Do not attempt a silhouette inside the existing single-surface fragment shader.
- Do not add hull duplicates to every streamed environment mesh as the first outline solution.
- Do not enable a normal prepass without checking Copaimo’s custom vertex deformation.
- Do not keep `Msaa::Sample4` by assumption; verify capability and image stability.
- Do not composite post-tonemap black without distance/fog treatment.

## 13. Source notes

Primary research and official production references used:

- Parish and Müller, [Procedural Modeling of Cities](https://cgl.ethz.ch/Downloads/Publications/Papers/2001/p_Par01.pdf) — road networks under goals/constraints, lots, and buildings.
- Müller et al., [Procedural Modeling of Buildings](https://www.researchgate.net/publication/220183823_Procedural_Modeling_of_Buildings) — CGA shape grammar and hierarchical façade generation.
- Epic, [City Sample: generating a city and freeway with Houdini](https://dev.epicgames.com/documentation/en-us/unreal-engine/city-sample-quick-start-for-generating-a-city-and-freeway-using-houdini) — city boundary, arterials, zones, lots, buildings, furniture, caching/PDG.
- SideFX, [Labs Building Generator](https://www.sidefx.com/docs/houdini/nodes/sop/labs--building_generator-4.0.html) and [Building Generator workflow](https://www.sidefx.com/tutorials/building-generator/) — blockout-to-modules, corners, floor overrides, and hand-placed/volume overrides.
- Bethesda, [Fallout 4’s Modular Level Design](https://gdcvault.com/play/1023202/-Fallout-4-s-Modular) — modular kits and iterative production for a large open world.
- Sucker Punch, [Building an Open-World Game Without Hiring an Army](https://www.gdcvault.com/play/1012233/Building-an-Open-World-Game) — footprint standardization, layout shortcuts, street identity, and navigation landmarks.
- Ubisoft, [Ghost Recon Wildlands terrain technology and tools](https://media.gdcvault.com/gdc2017/Presentations/WERLE_MARTINEZ_GRWterrainTechnologyTools.pdf) — procedural layers for roads, terrain, vegetation, villages, and placement.
- Ubisoft, [Assassin’s Creed Origins: Monitoring and Validation of World Design Data](https://www.gdcvault.com/play/1025054/-Assassin-s-Creed-Origins) — daily automated checks and visual reports for generated and authored world data.
- Freehold Games, [End-to-End Procedural Generation in Caves of Qud](https://www.gdcvault.com/play/1026313/Math-for-Game-Developers-End) — linked multi-tier village generation including history, culture, architecture, NPCs, and quests.
- Red Hook Studios, [The Art-Led Procedural Generation of Darkest Dungeon 2](https://gdcvault.com/play/1035493/Evolving-Worlds-from-the-Crumbling) — generator built around art direction and artist-facing tools.
- Arc System Works, [Guilty Gear Xrd’s Art Style: The X Factor Between 2D and 3D](https://www.ggxrd.com/Motomura_Junya_GuiltyGearXrd.pdf) — controlled normals, vertex data, inverted-hull variable outlines, and separate interior-line treatment.
- Tango Gameworks, [3D Toon Rendering in Hi-Fi RUSH](https://gdcvault.com/play/1034251/3D-Toon-Rendering-in-Hi) — full-world deferred toon rendering and stylized render-pass integration.
- Unity, [Unity Toon Shader outline settings](https://docs.unity3d.com/ja/Packages/com.unity.toonshader%400.8/manual/Outline.html) — normal/scale hull modes, width maps, color maps, baked normals, and camera-distance width control.
- Epic, [Stylized Rendering Post Processing](https://dev.epicgames.com/documentation/en-us/unreal-engine/stylized-rendering-post-processing?application_version=4.27) — scene-depth neighbor sampling and masking unwanted internal edges.
- DeCarlo et al., [Suggestive Contours for Conveying Shape](https://gfx.cs.princeton.edu/pubs/DeCarlo_2003_SCF/DeCarlo2003.pdf) — sparse contour families for shape communication.
- NVIDIA, [Blueprint Rendering and Sketchy Drawings](https://developer.nvidia.com/gpugems/gpugems2/part-ii-shading-lighting-and-shadows/chapter-15-blueprint-rendering-and-sketchy) — silhouette, border, and crease edges as visually meaningful line classes.
- Bevy 0.16.1 source examples: [shader prepass](https://github.com/bevyengine/bevy/blob/v0.16.1/examples/shader/shader_prepass.rs) and [custom post processing](https://github.com/bevyengine/bevy/blob/v0.16.1/examples/shader/custom_post_processing.rs) — the exact release family Copaimo pins.

## Final recommendation to Claude

Keep the current generator. Add meaning between its layers.

For settlements, the next breakthrough will come from plot programs, street hierarchy, correlated architectural families, and authored emphasis—not from raw building count. For rendering, add an ink system that selects forms worth drawing rather than interpreting all geometry as line art. The semi-cel shader already organizes light; outlines should now organize attention.
