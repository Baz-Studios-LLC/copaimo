# Copaimo Color, Materials, and Surface Art Direction

**Audience:** Claude and future Copaimo contributors  
**Purpose:** research-backed design and implementation guidance only  
**Project state reviewed:** August 29, 2026  
**Scope:** color scripting, material identity, cel-shaded lighting, terrain, architecture, glass, water, emissives, weather, accessibility, performance, and validation

---

## Executive recommendation

Copaimo already has the bones of a distinctive rendering language:

- linearized vertex colors rather than an uncontrolled collection of photo textures;
- four softened luminance bands rather than perfectly flat two-tone cel shading;
- blue-black silhouette ink;
- cloud shadows, biome color blending, day/night light, and warm occupied windows;
- clearly different old-world and modern architectural palettes.

The largest visual opportunity is **not more detail**. It is better hierarchy and stronger material identity.

At the moment, most opaque building surfaces share essentially one response to light. Plaster, wood, brick, stone, concrete, painted metal, exposed steel, brass, slate, and roof tile are distinguished mainly by vertex color and geometry. That makes the scene coherent, but it also makes large cities read as collections of similarly matte colored forms. In the daylight city capture, paving and façades converge toward bright gray-white; at night, large masses collapse close to black while lamps form isolated pools. The village reads more warmly, but its surfaces still depend heavily on color alone.

The recommended target is:

> **Illustrated color blocks at a distance, believable material response at middle distance, and selective authored surface evidence up close.**

Do this with a compact material vocabulary, not a photoreal texture library:

1. Lock a display and lighting calibration baseline before retuning more palette constants.
2. Define semantic color roles and value limits for world, route, interactable, danger, reward, and light.
3. Add approximately 8–10 reusable material response classes.
4. Preserve broad shapes; add only cause-based wear and construction detail.
5. Test every palette under day, dusk, night, and overcast conditions.
6. Validate in grayscale and with color-vision simulations, not only in full-color beauty shots.

AAA standards and good indie standards differ mainly in scale of content production. Their best practices are the same: a written visual grammar, calibrated inputs, reusable material systems, strong composition, deliberate budgets, and repeated testing in the actual camera and lighting conditions.

---

## 1. What the current Copaimo pipeline is doing

This section describes the reviewed implementation so recommendations stay compatible with the game rather than assuming a generic engine.

### 1.1 Color and shading

- `dev/art/masonry.py` authors human-readable sRGB palette colors and converts them once to linear vertex colors. That is correct in principle.
- Generated architecture uses vertex colors and a shared building material.
- The custom cloud shade material extends Bevy PBR and then applies four softened luminance bands. Current controls are approximately four bands, a soft edge of `0.055`, and a blend strength of `0.72`.
- Banding is luminance-based, then the RGB ratio is restored. This generally preserves hue better than quantizing channels independently.
- The HDR camera does not explicitly select a tonemapper or color grading. For the exact Bevy 0.16.1 dependency reviewed locally, the default is `TonyMcMapface`, a deliberately neutral transform that selectively desaturates bright colors.
- A vertical vertex-color foot shade multiplies the lower part of many buildings by about `0.74`.
- Generated building silhouettes use a blue-black inverted hull, approximately 7 cm before clamping.

### 1.2 Existing material response

- Opaque building surfaces share roughly `perceptual_roughness = 0.88`, `reflectance = 0.05`, and nonmetallic response.
- Glazing uses a separate translucent response, roughly `roughness = 0.25`, `reflectance = 0.5`.
- Terrain is highly rough and uses blended vertex colors with broad and fine mottling.
- Leaves, bark, cover grass, roads, sea, and rivers already have distinct material settings.
- Roads and paving contain comments showing that their source colors have been aggressively compensated for the current blue skylight, banding, and output transform. That is a warning sign: a color constant is being asked to solve a whole-pipeline problem.

### 1.3 Current image observations

These are art-direction observations, not bug reports.

- `dev/art/shots/day_city.png`: strong layout readability, but roads, paving, and many façades approach the same bright value family. Large areas feel equally matte. Material identity is weaker than shape identity.
- `dev/art/shots/night_city.png`: the warm light language is attractive, especially occupied windows. However, large ground and building masses lose internal value structure. Raising global exposure would damage the lights; the solution is controlled ambient/bounce, material response, and local composition.
- `dev/art/matrix/village_node.png`: the old-world palette is more emotionally legible—warm plaster, colored roofs, wood, shutters—but broad settled ground has limited tactile separation.
- `dev/art/matrix/canyon_inside.png`: very large gray and near-black masses dominate. This needs geological hue/value stratification and selective edge planes, not noisy rock textures.
- `dev/art/shots/path.png`: the dark route against a bright meadow reads well, but the foreground is a broad, almost equal green field. Large-scale value and temperature zones could add depth without fog.

### 1.4 Preserve these strengths

Do not discard the current system in pursuit of conventional PBR. Preserve:

- the graphic band structure;
- broad, designed color masses;
- blue-black rather than absolute-black ink;
- vertex-color efficiency;
- low visual noise;
- the legible warm-light/cool-night relationship;
- old-world versus modern architectural families;
- terrain variation at two scales rather than pixel-scale speckle.

---

## 2. Art-direction pillars

Claude should use these as decision filters.

### Pillar A — Readable before realistic

At the normal play camera, a route, doorway, landmark, hazard, and character must survive silhouette and grayscale tests. A physically plausible surface that hides navigation is the wrong surface.

### Pillar B — Color is structural

Color is not decoration applied after modeling. It establishes districts, climate, age, use, route priority, and emotional rhythm. A palette should explain the place even before props and signs appear.

### Pillar C — Materials differ by behavior, not noise

Wood should not be recognized merely because it is brown. It should have long construction rhythms, restrained directional variation, and an appropriate broad highlight. Stone should feel heavy and porous; tile should catch tighter light; painted metal should remain dielectric until its paint chips; exposed metal should carry metallic response.

### Pillar D — Detail has a cause

Darkening at the base comes from splash, soil, and capillary moisture. Roof streaks follow drainage. Hand contact polishes rails and handles. Sun-facing paint fades. Dust settles on upward planes. Damage concentrates at traffic and vulnerable edges. Random grime everywhere is not history; it is noise.

### Pillar E — One world across all lighting

The palette must work at noon, dusk, moonlight, overcast weather, indoors, and under warm lamps. There should not be a separate emergency set of arbitrary colors for each time unless it is an intentional authored state.

### Pillar F — Production scales through reuse

AAA quality comes from consistency and iteration more than unique texture count. Indie production reaches that standard by creating fewer, stronger modules: trim families, material classes, palette tables, and benchmark scenes.

---

## 3. Color system: build a hierarchy, not a collection of swatches

### 3.1 Establish semantic color roles

Every important color should have a job. Suggested roles:

| Role | Primary purpose | Typical treatment |
|---|---|---|
| World base | terrain and architecture | broad, restrained, locally harmonious |
| Traversable route | navigation | value or temperature separation from its surround |
| Landmark | orientation and memory | unique silhouette plus a reserved color family |
| Entrance/interaction | affordance | repeated accent or warm/cool contrast; never color alone |
| Character priority | gameplay | protected value/saturation range behind characters |
| Danger | rapid warning | high separation plus shape, motion, and/or iconography |
| Reward/magic | motivation and wonder | scarce high chroma and controlled emissive bloom |
| Occupancy/light | life and safety | warm, localized, patterned rather than every window |

When one bright accent color is used for flowers, shop signs, magical pickups, warning marks, and windows, it loses meaning. Reserve accent families.

### 3.2 Value organization comes first

Cel bands make value mistakes more visible because many nearby inputs collapse into the same output band. For each benchmark scene, decide:

- the dominant value mass;
- the supporting mass;
- the smallest high-contrast focal mass;
- which surfaces are allowed to reach the brightest diffuse band;
- which surfaces must retain information in shadow.

A practical composition target is not a rigid percentage but an **attention pyramid**: most of the screen quiet, a smaller region of contrast, and a very small focal accent. The exact ratio should follow the shot and gameplay.

For Copaimo:

- Do not let ordinary city paving live in the same brightest band as sky glints, emissives, pale landmark trim, or important signs.
- Do not let routine night walls fall to the same near-black as silhouette ink, deep occlusion, and voids.
- Keep playable routes separable in grayscale. Hue contrast is helpful but should be a second cue.

### 3.3 Use hue families, not isolated RGB fixes

Define palette families by region and material. Each family should contain at least:

- a lit local color;
- a shadow destination;
- a weathered variant;
- an accent/trim partner;
- a night-state expectation.

Examples:

- **Old village:** warm limestone/plaster, reddish or moss-muted roofs, low-chroma timber, cool slate punctuation, flower accents.
- **Modern city:** cool or neutral concrete, blue-gray glass, near-black mullions, restrained metal, warm inhabited windows, a small civic/signage accent family.
- **Canyon:** warm mineral midtones, cooler shadow planes, pale fractured edges, sparse vegetation accents. Avoid one neutral gray for all rock.
- **Meadow:** multiple related greens separated more by value and temperature than by random hue. Paths should belong to local geology.

### 3.4 Control chroma as a budget

Muted colors should dominate; selected saturated areas create interest and guidance. This is a recurring lesson in Valve’s *Illustrative Rendering in Team Fortress 2*: the world uses muted dominant colors with smaller saturated and complementary props, while excessive high-frequency detail is removed so intentional value changes can direct attention.

Suggested Copaimo chroma order:

1. gameplay-critical magical or warning state;
2. focal landmarks, signage, flowers, banners, or market goods;
3. inhabited windows and practical lights;
4. ordinary painted architecture and vegetation;
5. soil, unpainted masonry, distant landscape, and background support.

This does not mean all soil is gray. It means saturation is relative and deliberately allocated.

### 3.5 Temperature creates volume without extra darkness

Pure black shadows flatten cel-shaded scenes. A more illustrative solution is a subtle warm-to-cool relationship:

- sunlit faces may trend warmer;
- skylit and occluded faces may trend cooler;
- warm artificial lights can sit against cooler ambient night;
- overcast light compresses the temperature difference;
- canyon bounce may warm some shadow planes rather than making every shadow blue.

The TF2 paper explicitly identifies cool rather than black shadows, a slightly warmer/reddened terminator, and retained luminance variation as useful for form readability. Copaimo should borrow the principle, not the exact TF2 shader.

### 3.6 District and biome palettes need inheritance

Avoid inventing every town independently. Use a hierarchy:

```text
Global Copaimo palette rules
├─ climate/biome family
│  ├─ local geology and soil
│  ├─ vegetation family
│  └─ atmospheric shadow color
├─ settlement era
│  ├─ old-world construction palette
│  └─ modern construction palette
└─ district variation
   ├─ wealth/use/maintenance
   ├─ civic accent
   └─ landmark exception
```

The same stone should appear in local cliffs, foundations, walls, paving aggregate, and rubble. That single relationship makes a town feel built from its landscape.

### 3.7 Protect character and interaction readability

Build a “background exclusion” rule around the player character and major gameplay signals:

- avoid matching the character’s dominant value and hue in high-traffic backdrops;
- simplify contrast immediately behind entrances and important NPCs;
- use light pools, awnings, paving changes, door frames, and negative space as combined cues;
- ensure an interactive object is recognizable by form/state even if its accent hue is unavailable to the viewer.

The Xbox Accessibility Guidelines recommend sufficient contrast for gameplay-relevant visuals and additional cues rather than color alone. Treat that as baseline quality, not a late accessibility pass.

---

## 4. Color-management and output calibration

This is the highest-priority technical art task because palette work is unreliable until the display path is intentional.

### 4.1 Keep color spaces explicit

The current scripted sRGB-to-linear conversion is the right model. Maintain these rules:

- authored display colors are sRGB;
- lighting math and procedural interpolation operate in linear space;
- base-color textures are sampled as sRGB and decoded to linear by the renderer;
- roughness, metallic, normal, masks, and material IDs are data, not sRGB color;
- do not manually gamma-convert a value that the asset pipeline also decodes;
- do not compare linear RGB triplets by visual intuition in a text editor.

Khronos’s glTF PBR guidance and the glTF specification encode the same distinction: base color is an sRGB color input, while metallic/roughness are linear material data.

### 4.2 Make the final image path an authored choice

The conceptual pipeline is:

```text
authoring color → linear material inputs → PBR/light/cloud response
→ Copaimo luminance bands → HDR accumulation → tonemapping
→ optional restrained color grading → display
```

Claude should verify the exact placement of the custom band operation in Bevy’s extended material path, then document the intended ordering. Banding before versus after exposure/tonemapping produces substantially different stability.

### 4.3 Explicitly select the tonemapper

Do not rely indefinitely on the engine default. The exact current dependency defaults to Tony McMapface, but that default is an implementation detail and upgrades can change behavior.

Create a comparison capture using the same camera and lighting with the tonemappers available in Bevy 0.16.1—at minimum Tony, AgX, ACES Fitted, Somewhat Boring Display Transform, and no tonemapping if usable as a diagnostic. Compare:

- pale plaster and concrete;
- saturated flowers/signs;
- skin/character colors if applicable;
- blue sky and vegetation;
- warm emissives at night;
- metallic highlights and glass;
- shadow-band separation.

Select one based on the full day/night set, set it explicitly on the camera, and record the decision. Do **not** change tonemapping and every palette value in the same test; that hides cause and effect.

### 4.4 Use grading for global intent, not material repair

A small scene or weather grade may unify the image. It should not be used to make concrete stop looking like plaster or to rescue an unreadable route. Materials and local colors must work before grading.

If grading is introduced:

- keep it modest;
- test saturated accents for clipping and hue shifts;
- avoid crushed night blacks;
- compare UI and gameplay markers if they share the transform;
- store named presets by lighting condition, not arbitrary camera-by-camera values.

### 4.5 Prevent visible banding artifacts

Cel shading intentionally creates bands; HDR gradients in sky, fog-like haze, emissive bloom, and soft shadows should not acquire accidental 8-bit contouring. If visible:

- confirm render target and output precision;
- add subtle stable dithering at the final appropriate stage;
- avoid strong per-frame noise that crawls;
- distinguish intended four-band surface shading from unwanted output quantization.

Playdead’s *INSIDE* rendering presentation is a useful precedent for a sparse aesthetic that separately controls lighting terms and uses dithering to prevent technical banding.

---

## 5. A compact Copaimo material vocabulary

### 5.1 Why material classes are needed

Color answers “what family is this surface?” Material response answers “what happens when light touches it?” Copaimo currently has strong color families but too little second answer on buildings.

Do not assign a unique full material to every color or building. Define a small vocabulary. The following values are **starting ranges for visual tests**, not drop-in constants. Bevy’s `reflectance` property is a remapped dielectric specular control; `0.05` does not simply mean “5 percent reflectance.” The engine default around `0.5` represents the common dielectric F0 neighborhood. Judge changes in a calibration scene and against performance.

| Class | Example surfaces | Roughness direction | Metallic | Key stylized cue |
|---|---|---:|---:|---|
| Loose earth | dirt, silt, sand | very high | 0 | broad value zones, compacted route variation |
| Vegetation | grass, leaves, moss | high | 0 | controlled hue groups, readable clusters, minimal glitter |
| Porous mineral | plaster, chalk, raw concrete | high | 0 | wide soft response, subtle vertical weathering |
| Stone/brick | masonry, canyon rock, paving | high to medium-high | 0 | plane changes and joints; geology-led color |
| Timber | beams, boards, shingles | high when raw; lower when sealed | 0 | directional rhythm and end-grain/edge logic |
| Roof surface | tile, slate, thatch | material-dependent | 0 | clear repeating construction rhythm, restrained top-plane variation |
| Painted/coated | shutters, doors, painted steel | medium-high to medium | 0 | paint defines response; exposed chips may reveal substrate |
| Exposed metal | steel, brass, copper | medium to lower | 1 where truly bare | tight readable highlights; very limited screen area |
| Glass | windows, curtain wall | low to medium | 0 | coherent reflection/tint/opacity; interior state behind it |
| Liquid/wet | sea, river, puddle, wet surface state | lower | 0 | Fresnel and reflected environment, not noisy white sparkles |
| Emissive | lamps, occupied windows, magic | separate light-emission behavior | varies | shape and exposure-controlled luminance |

Ten or eleven conceptual classes can be implemented with fewer actual GPU materials if the shader consumes a material-class ID.

### 5.2 Possible implementation strategies

Claude should prototype and measure rather than committing to the most elaborate option.

#### Strategy A — A few material handles by response class

Split generated meshes or primitives into a small number of material groups: porous, timber, roof, metal, glass, and so on.

**Advantages:** simplest, uses standard Bevy material properties, easy to debug.  
**Costs:** additional mesh sections/material bindings and draw calls; scripted generation must preserve grouping.

This is the safest first vertical slice.

#### Strategy B — Material class in mesh data, one custom material

Keep a shared material but encode a small class index in an otherwise reserved vertex attribute or secondary UV, then map that class to roughness/reflectance/metallic and optional stylized controls in the shader.

**Advantages:** retains batching and the current vertex-color workflow.  
**Costs:** requires careful mesh/import support, shader plumbing, and testing; interpolated IDs must be quantized or flat; transparent surfaces still need separate handling.

If considering vertex-color alpha as the class channel, first prove that it is preserved by the Blender/glTF/export path and does not conflict with alpha behavior. A secondary UV or explicit custom attribute is conceptually safer.

#### Strategy C — Small trim/atlas material families

Use a few shared texture sets for high-value architectural elements: old-world masonry/trim, old roofs/wood, modern façade/mullions, and civic/signage. Use color masks or vertex tints to create controlled variation.

**Advantages:** strong mid-distance construction detail with reusable memory.  
**Costs:** UV authoring and pipeline complexity; easy to over-texture.

This is a later step after material classes, not the starting point. The *Sunset Overdrive* trim-sheet presentation is an excellent production reference for standardized layouts and reusable shader-driven variation.

### 5.3 Preserve a stylized response through cel bands

PBR is a model for stable light/material relationships; it does not require photoreal imagery. *Agents of Mayhem* and *Mirror’s Edge Catalyst* are relevant AAA examples of physically based pipelines bent toward deliberate stylization.

For Copaimo:

- diffuse banding may remain the dominant form language;
- specular response should be broader and simplified, but different across classes;
- consider separate artistic control of diffuse bands and specular intensity so a metal or glazed tile does not become merely another bright diffuse patch;
- do not quantize tiny high-energy speculars into distracting popping pixels;
- allow only selected materials to produce the sharpest highlight band;
- retain blue-black ink as the darkest graphic element.

### 5.4 Painted metal is not metallic paint by default

The visible paint layer is usually dielectric. Set metallic behavior for exposed metal, not every object made from metal underneath. A painted steel beam can use the painted/coated class; chips may reveal small metallic substrate areas if the style and scale justify them.

### 5.5 Material response should survive hue changes

A red shutter and a blue shutter should still feel like the same coated wood. A pale limestone and a red sandstone can share a porous stone response while keeping different local colors. This is why palette and material class should be independent systems.

---

## 6. Surface design at three scales

The best stylized environments organize detail by viewing distance.

### Macro: read from far away

- building color block;
- roof/wall/foundation separation;
- street versus sidewalk versus planted area;
- cliff strata and large plane breaks;
- district and landmark identity.

### Meso: read during ordinary play

- bays, frames, seams, masonry courses, beams, drainage paths;
- patch repairs, roof modules, curb transitions;
- restrained material-specific color breakup;
- broad roughness or normal variation.

### Micro: read only close up

- grain, fine cracks, aggregate, scratches, tiny chips.

Copaimo should spend most of its budget on macro and meso detail. Micro detail should be sparse and high-value. Valve’s TF2 work deliberately minimized high-frequency detail and repetitive geometry, conveying the impression of repetition without explicitly modeling every instance. That principle fits Copaimo’s current style and production constraints.

### Cause-based variation checklist

Before adding a stain, chip, or color patch, answer:

1. What caused it?
2. Which orientation or location would receive it?
3. What scale is visible from gameplay distance?
4. Does it clarify or compete with the form?
5. Does it repeat too evenly?

Recommended procedural masks, in descending usefulness:

- world height/base contact;
- upward-facing versus downward-facing surfaces;
- roof drainage direction;
- edge/convexity and crevice/concavity, used lightly;
- distance from doors, roads, or high-traffic areas;
- exposure to rain/wind/sun if the world supplies direction;
- low-frequency seeded variation per building or district.

Avoid uncorrelated RGB noise and uniform edge wear on every edge.

---

## 7. Surface-specific direction

### 7.1 Dirt, silt, sand, and paths

- Build large color/value patches before fine mottling.
- Show compaction on routes with slightly lower roughness, smoother normal response, and a value shift appropriate to local soil—not a universal dark stripe.
- At edges, transition through sparse stones, trampled grass, ruts, or a shoulder; avoid a perfectly stamped ribbon.
- Wetness should darken and reduce roughness in coherent drainage/traffic areas, not uniformly tint the entire biome.
- Keep path navigation readable in grayscale.

### 7.2 Grass and vegetation

- Group hues by species, moisture, exposure, and season rather than per-instance randomness.
- Use a few cluster-scale temperature/value variants; excessive hue jitter looks synthetic.
- Reduce sparkle from many small normals/specular responses.
- Let vegetation near settlements show maintained, trampled, planted, or disturbed states.
- Protect the silhouette and value of characters against grass.

### 7.3 Canyon and rock

- Start with geology: bedding direction, fractured planes, mineral bands, weathered top faces, and accumulated material at the base.
- Use warm midtone rock with cooler ambient shadow rather than neutral gray into black.
- Reserve pale edges for selected fractured planes and route readability.
- Add a few large plane-color changes; do not cover the canyon with high-frequency rock noise.
- Tie local stone architecture and road aggregate back to the same geology.

### 7.4 Plaster and stucco

- Keep broad, soft, porous response.
- Introduce subtle tone variation by wall panel or construction phase.
- Concentrate staining below sills, along caps, near grade, and at drainage paths.
- Repairs can be larger quiet patches with slightly different value/temperature.
- Do not use uniform dirty gradients on every building. The existing foot shade is useful but should be supplemented or varied by actual site logic so every façade does not share the same identical aging signature.

### 7.5 Timber

- Direction is essential: boards and beams need longitudinal rhythm, even if represented by geometry or a very restrained shared texture.
- Distinguish raw, aged, sealed, and painted wood through roughness and value—not just different browns.
- Darken end grain and sheltered joints selectively.
- Put polish on handrails, door edges, and traffic surfaces, not roof beams.
- Avoid thin black lines for every plank at distance; let a few construction divisions imply the rest.

### 7.6 Stone and brick masonry

- Prioritize block/course rhythm at meso scale.
- Mortar should not become a high-contrast grid that overwhelms the building.
- Use larger stone families and irregular grouping; tiny random bricks cause shimmer and noise.
- Slightly vary roughness and local color by stone group, with more coherent repairs or infill.
- Corners, openings, foundations, and caps deserve better stones because construction logic naturally emphasizes them.

### 7.7 Roof tile, slate, shingle, and thatch

- Each roof type needs a distinct silhouette and light response.
- Tile: organized highlights, clear overlap rhythm, low-frequency color families.
- Slate: cooler, flatter, sharper planar breaks, controlled edge glints.
- Wood shingle: directional, rough, muted, irregular at selected edges.
- Thatch: broad fibrous flow and soft highlights; avoid thousands of individually outlined strands.
- Weathering follows water paths and sun exposure. Roof variation should be stronger in broad patches than per-tile random color.

### 7.8 Concrete

- Separate cast panels, poured masses, and precast trim through seams and construction logic.
- Keep most concrete high-roughness but not identical to plaster; aggregate, panel scale, edge treatment, and subtle specular breadth can distinguish it.
- Use warm-neutral and cool-neutral families by district rather than one gray.
- Reserve strong dark streaks for real drainage points.

### 7.9 Painted and exposed metal

- Painted metal should read primarily through clean edges, manufactured repetition, and a controlled coated highlight.
- Exposed steel, brass, or copper should occupy limited areas and create useful highlight punctuation.
- Brass works well as a warm civic/luxury accent; do not scatter it uniformly.
- Weathered metal needs coherent oxidation states. Rust appears where paint fails and moisture stays; copper patina follows material and exposure.
- Avoid mirror-like metals in ordinary world clutter; they destabilize the cel image and cost visual attention.

### 7.10 Glass and windows

Glass needs an art-directed state machine more than a single universal transparent blue material.

Suggested states:

1. **Day reflective:** reflects sky/environment enough to read as glass; interior remains understated.
2. **Day transparent focal:** selected storefronts or entrances reveal a simplified interior.
3. **Night dark/unoccupied:** subdued reflection and deep but non-ink interior.
4. **Night occupied:** warm interior card/plane or simplified room, varied curtain/object silhouettes, controlled emissive.
5. **Special civic/magical:** reserved tint or pattern, still obeying exposure limits.

Guidelines:

- windows on one façade can share a reflection direction and value family;
- not every pane needs a separate random color;
- occupied windows should form designed rhythms and stories, not 50% independent noise;
- the emissive surface should have visible color and shape before bloom;
- place point/area-like light only where it meaningfully lights the exterior; do not attach a dynamic light to every glowing pane;
- curtain-wall mullions need enough value separation to express scale without turning into a black grid.

### 7.11 Water

- Keep sea and river responses distinct through movement scale, color depth, bank context, and roughness.
- Use Fresnel/environment reflection as the primary glassy cue.
- Avoid uniformly white highlights and excessive per-wave glitter under cel banding.
- Shallow water should inherit bed color and clarity; deep water should not simply be a darker saturated blue.
- Bank wetness, foam, and sediment belong only where flow and collision justify them.

### 7.12 Emissives and practical lights

The current warm lamp and occupied-window direction is promising. Formalize it:

- assign luminance tiers: indicator < occupied window < practical lamp < magic/focal source;
- maintain hue identity after tonemapping rather than driving everything to white;
- show a visible fixture/source shape;
- separate emissive appearance from actual world illumination;
- use light to reveal nearby material and navigation, not merely to decorate darkness;
- vary occupancy in coherent clusters—floors, rooms, districts, and time—not isolated random pixels;
- cap bloom so silhouettes and window framing survive.

At night, add controlled cool ambient or directional bounce to retain form. Do not solve black masses by raising the whole image until lamps lose contrast.

---

## 8. Outlines and materials must cooperate

The existing blue-black inverted hull supports the semi-cel look. Material improvements should not make every surface boundary require another line.

- Use silhouette ink for silhouette and selected major overlaps.
- Let material value, hue, roughness, and geometry separate internal surfaces.
- Keep outline darkest; do not allow ordinary shadow bands to routinely equal it.
- Suppress or reduce outlines across large internal coplanar divisions.
- Fine material detail should not receive independent black contours.
- Consider distance scaling/clamping so lines remain graphically consistent instead of becoming heavy nearby or disappearing far away.
- Use authored masks/weights where important profiles need thicker ink and visually fragile regions need less.

The Guilty Gear Xrd GDC material remains a useful implementation reference: its outline shell uses artist control for width and suppression, and treats internal lines separately. Copaimo need not copy its character solution exactly, but the principle—**ink is authored information, not an automatic border around everything**—applies to architecture.

---

## 9. Lighting, weather, and time-of-day contracts

### 9.1 Day

- Preserve separation among pale roads, plaster, cloud, concrete, and bright trim.
- Keep a controlled sky fill so cool-facing planes retain hue.
- Limit top-band occupancy to genuinely focal or directly lit light surfaces.
- Check foliage and roofs for noisy specular popping.

### 9.2 Dawn and dusk

- Warm key light plus cooler sky is useful, but avoid turning all local colors orange/blue.
- Maintain district and material identity under the color contrast.
- Let landmarks catch light selectively.

### 9.3 Night

- Set a minimum readable non-ink value for important architecture and routes.
- Keep warm practical lights distinct from warm materials.
- Use controlled ambient/bounce or a stylized cool shadow destination.
- Preserve sky silhouettes while retaining a few internal planes.
- Validate with bloom both on and off.

### 9.4 Overcast

- Compress contrast but preserve material distinction through roughness breadth, local color, edges, and construction patterns.
- Reduce harsh bands only if the weather system intentionally changes the light, not through a global gray overlay.
- Allow wetness selectively after rain; wet surfaces generally darken and become smoother, but enclosed walls and covered ground should differ.

### 9.5 Snow or bright weather states

- Snow cannot share one white with sky, concrete, clouds, UI, and emissives.
- Use cool ambient shadow, warm/sunlit planes, exposed ground breaks, and texture at broad drift scale.
- Avoid dazzling full-screen top-band coverage.

---

## 10. Depth without returning to opaque fog

Copaimo deliberately avoids conventional distance fog. Depth can still be strengthened through restrained aerial-perspective principles:

- lower contrast with distance;
- slightly shift distant landscape toward the sky/ambient hue;
- reduce distant saturation and micro-detail;
- group distant buildings into simpler value masses;
- use overlapping planes and designed skyline breaks;
- author distant LOD colors to be quieter;
- if a shader term is used, make it a subtle color/contrast term rather than a visibly opaque wall of fog.

This is especially useful for `path.png`, canyon exits, city vistas, and mountain/shore views. It should never erase landmarks or the clear horizon the no-fog decision is intended to protect.

---

## 11. The calibration courtyard Claude should build first

Before broad implementation, create a small developer-only benchmark scene or mode. It can be generated from primitives and reused indefinitely.

Include:

- a sphere, cube, bevelled block, vertical wall, roof pitch, and ground patch for each material class;
- all major Copaimo palette swatches on identical geometry;
- a pale-plaster/road/concrete/sky separation test;
- old-world façade fragment with timber, plaster, stone, roof, glass, and brass;
- modern façade fragment with concrete, painted metal, exposed metal, mullion, glass, and emissive interior;
- terrain strips for grass, dirt, sand, rock, snow, wet variants, and path edges;
- outline thickness targets at near/mid/far distances;
- one warm light, one cool ambient condition, and representative windows;
- a neutral middle-gray reference and controlled brightest diffuse reference.

Automate captures from fixed cameras for:

- noon clear;
- dawn/dusk;
- overcast;
- moonlit night;
- night with practical lights;
- optional rain/wet state.

Also capture:

- normal full color;
- grayscale;
- no outlines;
- outlines only if possible;
- flat albedo/vertex color;
- roughness/material-class debug view;
- no tonemapping diagnostic;
- each candidate tonemapper.

This scene turns subjective arguments into controlled comparisons and prevents a change that beautifies one screenshot from breaking the rest of the game.

---

## 12. Quantitative and human validation

Metrics are guardrails, not the art director.

### 12.1 Image checks

- **Value occupancy:** how much of the frame collapses into the darkest and brightest display ranges?
- **Band occupancy:** are three or four surface bands actually visible, or are most surfaces pinned to one?
- **Local contrast:** does the route/entrance/character separate from its immediate surround?
- **Saturation heat map:** are saturated pixels concentrated at intended accents?
- **Temporal stability:** do outlines, highlights, and material details shimmer during movement?
- **Night clipping:** do emissives retain hue and fixture shape?

### 12.2 Play tests

Ask players, without prompting:

- Where can you go?
- Which door matters?
- What is the landmark?
- Which area feels safe, inhabited, wealthy, old, industrial, or magical?
- What material is that wall/roof/ground?
- Can you track the character while moving through vegetation and city streets?

If players identify the material only when standing still close to it, the meso-scale cues are insufficient. If they identify it but cannot find the route, the surface is winning over gameplay.

### 12.3 Accessibility checks

- Test common color-vision deficiency simulations.
- Confirm gameplay meaning survives grayscale.
- Pair colors with silhouette, pattern, icon, animation, position, or text.
- Keep text/UI contrast independent from world grading where possible.
- Avoid rapid full-screen flashes and high-contrast repetitive patterns.
- Offer relevant brightness/contrast or effect controls when the game reaches settings work.

---

## 13. Recommended implementation sequence

### Phase 0 — Lock the image pipeline

1. Add/prepare the calibration courtyard and fixed capture matrix.
2. Explicitly compare and select the Bevy 0.16.1 tonemapper.
3. Verify color-space handling and band placement.
4. Establish value/chroma targets for day and night.
5. Record the selected baseline before retuning palette constants.

**Success:** the same authored swatch behaves predictably across controlled conditions.

### Phase 1 — Prove material identity on two façades

1. Prototype a few material response classes using the simplest implementation.
2. Apply them to one old-world fragment and one modern fragment.
3. Compare against the current shared opaque material.
4. Inspect cel-band and outline interaction.
5. Measure draw calls/frame time and visual stability.

**Success:** players can distinguish plaster, timber, roof, concrete, glass, coated metal, and exposed metal at play distance without more texture noise.

### Phase 2 — Correct the value hierarchy

1. Rebalance city paving, pale façades, concrete, cloud, and bright trim.
2. Establish the night minimum-readable value above ink.
3. Protect routes and interaction zones.
4. Test grayscale and color-vision variants.

**Success:** focal lights and landmarks retain headroom; ordinary bright ground does not dominate.

### Phase 3 — Terrain and canyon vertical slice

1. Add geology-led macro/meso color structure to canyon rock.
2. Improve path shoulders and compaction cues.
3. Group vegetation color variation more intentionally.
4. Add subtle distant contrast/color falloff if needed.

**Success:** natural surfaces feel distinct and deep without photographic noise or opaque fog.

### Phase 4 — Reusable surface modules

1. Add a small number of trim/atlas families only where meso detail is missing.
2. Add cause-based masks for base contact, drainage, top dust, and selected wear.
3. Establish consistent old/modern material libraries.

**Success:** new buildings inherit quality without unique texture production.

### Phase 5 — Weather and occupancy states

1. Introduce selective wet response.
2. Formalize window state patterns and emissive tiers.
3. Validate overcast, rain, snow, and seasonal palettes as applicable.

**Success:** conditions change mood while preserving color meaning and material identity.

---

## 14. Performance guardrails

- Prefer shared material classes and palette parameters over unique materials per building.
- Measure the draw-call cost of mesh splitting before applying it globally.
- Keep transparent glass limited and ordered deliberately; transparency is more expensive and troublesome than opaque surfaces.
- Use emissive-only windows for distant buildings; reserve real lights for places where illumination affects play or composition.
- Avoid a dynamic shadow-casting light per window or street fixture.
- Use shared trim sheets/atlases and repeatable UV conventions rather than unique high-resolution textures.
- Make debug views and capture tests cheap enough that they are used routinely.
- Test shimmering and aliasing in motion. A still screenshot can hide the biggest cost of fine stylized detail.
- Treat every added shader feature as optional until the vertical slice proves visual value per millisecond.

Riot’s “Better Living Through Materials” is a strong indie-compatible lesson despite its large-team context: a data-driven material system and rapid feedback support painterly style, gameplay clarity, and performance better than ad hoc one-off materials.

---

## 15. Common failure modes to avoid

- Retuning dozens of source RGB values before selecting and locking the output transform.
- Giving every surface the same roughness and relying on hue alone.
- Making the world “more detailed” with uniform texture noise.
- Treating all metal objects as metallic even when painted.
- Using absolute black for ordinary shadows and material crevices.
- Letting roads/paving consume the brightest diffuse band across large portions of the screen.
- Solving night readability with global exposure that washes out emissives.
- Randomly lighting individual windows with no building/floor/room logic.
- Adding a real-time light to every emissive surface.
- Drawing black lines around every brick, panel, pane, or plank.
- Using roughness variation as random static rather than material history.
- Making every district equally colorful.
- Depending on red/green or blue/purple distinction without secondary cues.
- Judging materials only in the Blender viewport, palette script, or isolated ball rather than inside Copaimo’s actual shader, sky, bands, and tonemapper.
- Applying photographic scans whose frequency, lighting, and artifacts fight the illustrated world.
- Upgrading to a more complicated shader system before a two-façade vertical slice proves the need.

---

## 16. Definition of done for the color/material pass

The work is successful when:

- ordinary players can distinguish major surface families at normal camera distance;
- the city, village, canyon, meadow, coast, and interiors have recognizable palette scripts but clearly belong to one game;
- old-world and modern construction differ through material behavior as well as hue and geometry;
- routes, entrances, landmarks, characters, hazards, and rewards remain readable in grayscale;
- day, dusk, overcast, and night preserve the intended hierarchy;
- night architecture retains form above the outline/void black level;
- emissives retain hue and shape without overwhelming bloom;
- outlines support silhouette and overlap rather than tracing all detail;
- moving camera footage is stable, without highlight glitter or fine-pattern shimmer;
- the material library remains compact and measurable;
- future buildings can be authored from documented palette/material families rather than one-off guesses.

---

## 17. Sources and why they matter

### Primary technical and production references

1. **Valve — “Illustrative Rendering in Team Fortress 2”**  
   https://www.riotgames.com/darkroom/original/87b07e8dde1ae968b72eb5e60c7ede9b%3A0ea751891424f001e471f06a521fabd8/npar07-illustrativerenderinginteamfortress2.pdf  
   Primary paper on aligning palette, silhouettes, low visual noise, cool-to-warm shading, artist-authored masks, and gameplay readability.

2. **Arc System Works — “Guilty Gear Xrd’s Art Style: The X Factor Between 2D and 3D”**  
   https://www.gdcvault.com/play/1022031/Guilty-Gear-Xrd-s-Art  
   https://www.ggxrd.com/Motomura_Junya_GuiltyGearXrd.pdf  
   Primary production reference for controlled toon shading, variable/suppressible inverted-hull outlines, and separating internal lines from silhouette ink.

3. **Tango Gameworks — “3D Toon Rendering in Hi-Fi RUSH”**  
   https://gdcvault.com/play/1034330/3D-Toon-Rendering-in-Hi  
   https://media.gdcvault.com/gdc2024/Slides/GDC%2Bslide%2Bpresentations/Tanaka_Kosuke_3D_Toon_Rendering.pdf  
   Modern AAA reference for carrying a toon language across characters and world geometry while retaining contemporary rendering features and performance.

4. **Volition — “Agents of Mayhem: Physically-Based Materials in a Stylized Open World”**  
   https://www.gdcvault.com/play/1024690/-Agents-of-Mayhem-Physically  
   https://media.gdcvault.com/gdc2017/Presentations/Taylor_James_Agents_of_Mayhem.pdf  
   Directly relevant precedent for PBR materials, color documentation, and performance in a stylized city.

5. **DICE — “Lighting the City of Glass: Rendering Mirror’s Edge Catalyst”**  
   https://www.gdcvault.com/play/1022987/Lighting-the-City-of-Glass  
   Useful AAA example of taming physically based sky, reflections, lighting, and grading to serve an unusually clean, stylized city.

6. **Playdead — “Low Complexity, High Fidelity: INSIDE Rendering”**  
   https://www.gdcvault.com/play/1023002/Low-Complexity-High-Fidelity-INSIDE  
   Reference for strong results from a sparse visual language, separately controlled lighting components, and dithering for technical gradients.

7. **Insomniac Games — “The Ultimate Trim: Texturing Techniques of Sunset Overdrive”**  
   https://www.gdcvault.com/play/1022323/The-Ultimate-Trim-Texturing-Techniques  
   Production reference for reusable trim layouts and shader variation in a large stylized city.

8. **Riot Games — “Better Living Through Materials”**  
   https://www.riotgames.com/en/news/better-living-through-materials  
   Primary studio account of building a data-driven material system around rapid feedback, painterly goals, gameplay clarity, and performance.

9. **Riot Games — Environment Art education page**  
   https://www.riotgames.com/en/artedu/environment-art  
   Concise environment-art framing: communicate gameplay and story while prioritizing production effort.

### Rendering and color references

10. **Khronos — Physically Based Rendering in glTF**  
    https://www.khronos.org/gltf/pbr  
    Accessible reference for metallic/roughness material behavior and color-versus-data texture treatment.

11. **Khronos — glTF 2.0 Specification**  
    https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html  
    Authoritative specification for base color, metallic/roughness, and texture color spaces in the asset exchange path used by the project.

12. **Walt Disney Animation Studios — “Physically-Based Shading at Disney”**  
    https://disneyanimation.com/publications/physically-based-shading-at-disney/  
    Foundational artist-facing material model research. Useful for understanding how a small, intuitive parameter set can cover many surfaces.

13. **Bevy — Tonemapping example**  
    https://bevy.org/examples/3d-rendering/tonemapping/  
    Official comparative example. The project’s exact Bevy 0.16.1 source should remain the final authority for available variants and defaults.

14. **Bevy — Color grading example**  
    https://bevy.org/examples/3d-rendering/color-grading/  
    Official reference for grading controls and their image effects.

15. **Bevy — `StandardMaterial` documentation**  
    https://docs.rs/bevy/latest/bevy/pbr/struct.StandardMaterial.html  
    Reference for meaning of roughness, metallic, reflectance, emissive, and related properties. Match it against the pinned 0.16.1 source before implementation because current online docs may describe a newer release.

### Accessibility references

16. **Xbox Accessibility Guideline 102 — contrast**  
    https://learn.microsoft.com/en-us/gaming/accessibility/xbox-accessibility-guidelines/102  
    Guidance for contrast of gameplay-relevant visual elements and user configuration.

17. **Xbox Accessibility Guideline 103 — additional cues and color**  
    https://learn.microsoft.com/en-us/gaming/accessibility/xbox-accessibility-guidelines/103  
    Guidance for avoiding color-only communication and supporting color-vision differences.

18. **Xbox Accessibility Guideline 118 — photosensitivity**  
    https://learn.microsoft.com/en-us/gaming/accessibility/xbox-accessibility-guidelines/118  
    Guidance relevant to emissive effects, flashing, contrast changes, and repetitive patterns.

---

## Final note to Claude

The immediate win is not to replace Copaimo’s look. It is to make the look more intentional and robust.

Start with the calibration courtyard and two façade fragments. Lock the tonemapper. Give a few surfaces distinct light behavior. Rebalance the city’s brightest masses and the night’s darkest masses. Only after those results are proven should the project invest in shared trim textures, procedural wear, wetness, or more elaborate shader channels.

The desired image is not “PBR with a cartoon filter.” It is an authored illustration whose colors remain stable, whose materials are believable enough to be tactile, and whose technical system protects composition and gameplay at every time of day.
