# Copaimo AAA Visual Quality and Detail Playbook

**Date:** 2026-09-01  
**Audience:** Claude / implementation planning  
**Scope:** Visuals and environmental detail only. This is advice, not an authorization to implement it immediately.

## Executive direction

Copaimo should not chase AAA quality by maximizing object count or replacing its style with photorealism. The viable target is a **highly controlled, semi-cel-shaded world whose detail is intentional at every viewing distance**.

The current project is unusually texture-light: the visible world is dominated by generated geometry, vertex colour, and shared shaders. Keep that strength. A hybrid pipeline gives the best return:

1. geometry and vertex colour establish silhouette, architecture, and large palette families;
2. shared shaders establish stable cel response and coherent material identity;
3. small reusable masks, trims, mesh decals, or procedural overlays add medium-frequency detail;
4. close wear appears only where the camera and gameplay can read it.

This is compatible with physically based material reasoning. PBR is a material-consistency model, not a requirement to look photographic. The stylization should come from controlled values, palette, banding, shape language, and selective simplification - not from impossible material relationships.

## 1. Build detail in five distance bands

Every important view should survive a five-band audit. Detail that does not improve one of these readings is optional.

| Distance | Player must read | Highest-value tools |
|---|---|---|
| 600-1200 m | landmass, skyline, weather, one landmark, route direction | terrain masses, skyline hierarchy, atmospheric contrast, landmark silhouette |
| 150-600 m | settlement identity, road arrival, canopy rhythm, district separation | roof families, landmark placement, tree clusters, route composition, palette blocks |
| 30-150 m | street hierarchy, building function, public/private boundaries | frontage rhythm, entrances, kerbs, drainage, signs, awnings, wall/roof material breaks |
| 5-30 m | believable construction and occupation | foundations, thresholds, gutters, posts, joinery, service objects, planting, repair evidence |
| 0-5 m | contact, touch, and material behaviour | contact shadow, edge wear, wetness, dirt capture, footprints/ruts, small motion and VFX |

AAA failure often comes from polishing the last band while the first three remain generic. Work from the largest band inward.

## 2. Use a strict visual hierarchy

Each frame should have five layers, in this order:

1. **Composition and silhouette** - the path, landmark, skyline and main masses.
2. **Value and palette groups** - clear light/dark families and controlled regional colour.
3. **Structural detail** - roof breaks, bays, wall thickness, kerbs, retaining edges, canopy forms.
4. **Functional/context detail** - drainage, storage, access, signs, seating, tools, waste, repairs.
5. **Surface events** - stains, chips, dampness, dust, footprints, small cracks.

Never use layer five to hide a weakness in layers one through three. A flat building with noise is still a flat building. A road with random cracks but no drainage or edge logic still looks procedural.

## 3. Hybrid material system for Copaimo

### Preserve the graphic base

- Keep broad albedo values quiet enough for the cel bands and outlines to remain legible.
- Give every material family a defined value range, roughness range, edge behaviour, and weather response.
- Avoid independent random colour per object. Variation belongs inside a family.
- Reserve the darkest values for ink, deep cavities, and a few focal accents; widespread near-black materials collapse the outline language.

### Add medium-frequency identity

The present asset balance suggests the biggest missing layer is not dense photo textures but **controlled medium-scale breakup**. Claude should consider a compact context kit rather than bespoke maps for every object:

- reusable trim or atlas strips for sills, fascia, foundation bands, timber ends, gutters, grates and thresholds;
- mesh decals or procedural masks for plaster patches, road repairs, water streaks, soot, moss, wheel wear and puddle influence;
- low-cost macro variation sampled in world space, kept subtle enough not to swim or overpower silhouettes;
- per-instance parameters for age, dampness, maintenance and district palette.

Decals are most convincing when causal: a leak below a gutter joint, grime where feet touch a threshold, wheel polishing in travelled lanes, repairs following utilities or failures. Random sticker-like distribution lowers quality.

### Material response sheet

Define this before expanding the asset library:

| Family | Base reading | Roughness/specular | Edge/contact story | Rain response | Snow response |
|---|---|---|---|---|---|
| packed dirt | warm, soft, low contrast | matte except compacted tracks | crumbles at shoulder; ruts hold moisture | darkens, compacts, shallow puddles | fills ruts first; travelled centre clears |
| stone paving | grouped slab values | restrained highlights on worn tops | dirt and moss collect in joints | darker joints, selective sheen | catches in joints and lee edges |
| plaster | quiet broad field | matte | chips at base/corners; streaks below projections | uneven darkening below exposure | ledges cap before vertical faces |
| timber | warm directional family | weathered matte, polished touch points | end grain/joints readable | exposed faces darken | top-facing ledges retain snow |
| metal | sparse accents | controlled tight highlights | wear on handles/edges/fasteners | stronger highlight and runoff | little adhesion on warm/steep parts |

## 4. Lighting and atmosphere

### Light hierarchy

Use three conceptual layers:

- **world key:** sun or moon, setting time, direction and the major shadow design;
- **sky fill:** prevents inked shade from becoming dead black and carries weather colour;
- **local practicals:** windows, lamps, fires and signs used for navigation and lived-in rhythm.

Local lights should answer a question - entry, work area, public node, danger, shelter - not be evenly sprinkled. At night, build pools of activity separated by quieter darkness.

### Contact before global darkness

Ground objects with short-range contact shadow, believable foundations, and local dirt/wetness before increasing global ambient occlusion. Heavy screen-space AO makes cel-shaded scenes muddy and can draw false outlines in vegetation. Use it gently for creases and broad grounding; let authored contact relationships do the close work.

### Atmosphere without flattening the world

The camera deliberately lacks generic distance fog. Preserve clarity, but introduce atmospheric perspective through:

- reduced contrast and chroma with distance;
- local valley/shore haze rather than one uniform curtain;
- weather-dependent sky fill and shadow softness;
- cloud shadow movement at a scale slower than vegetation motion;
- stronger separation between near route, middle settlement, and far skyline.

Validate identical views at noon, dusk, night, overcast, rain and snow. A material or landmark that only reads in one light is not finished.

## 5. Settlements must tell a visual sentence

Before detailing a town, write one sentence describing its visual identity: geography + economy + history/maintenance + landmark. Every new asset or prop should reinforce at least one part of it.

### Four-sided building logic

- **Front:** address, threshold, public identity, sign or display.
- **Sides:** structure, drainage, access constraints, neighbour relationships.
- **Rear:** service, storage, waste, deliveries, private occupation.
- **Roof:** climate, repair history, chimneys/vents, runoff and skyline.

### Importance-based density

- landmarks: unique silhouette, richest construction logic, clearest night signature;
- civic/commercial frontage: medium-high density and public clues;
- ordinary housing: repeated kit with controlled household variation;
- service/back lanes: fewer decorative pieces but more functional evidence;
- outskirts: sparse detail, stronger landscape transition and unfinished edges.

Use **occupation states**, not random props: actively used, maintained, neglected, repaired, seasonally closed, under construction. A state controls a coherent cluster of objects and material changes.

## 6. Roads and ground detail

Road polish should extend beyond the road mesh. Each road needs an influence field:

1. travelled centre;
2. wheel/foot wear lanes;
3. material body;
4. damaged or vegetated edge;
5. drainage/kerb/ditch condition;
6. disturbed verge and nearby prop response.

### Causal road events

- cracks follow stress, joints, roots, settlement or freeze/thaw - not white noise;
- potholes collect water and loose aggregate;
- repairs differ in age and colour and follow plausible work areas;
- weeds prefer protected joints and low traffic;
- mud transfer appears near dirt-to-paved handoffs, farms and construction;
- drain placement follows grade and low points;
- city arrival transitions through frontage, edge restraint, drainage, lighting and traffic clues - not only a material crossfade.

For the current road work, the visible mesh, analytic travelled surface, material fade, kerb ownership and vegetation clearing must share one route/boundary authority. Visual quality cannot survive a beautiful road whose collision or surface owner disagrees.

## 7. Vegetation: ecology plus composition

Avoid uniform scatter. Build communities using canopy, sub-canopy, shrub, herb and floor layers, but vary which layers dominate by biome and disturbance.

Placement constraints should include moisture, slope/aspect, exposure, soil/ground family, distance to water, disturbance, road edge and settlement maintenance. Then form clusters with centres, satellites and gaps.

Art-direct the result:

- protect landmark silhouettes and route sightlines;
- create alternating compression and release along paths;
- let dense vegetation frame, not cover, entrances and views;
- reduce growth where people travel or maintain ground;
- bias moisture plants toward drainage and shore influence;
- give large, medium and small plants different wind frequency and amplitude.

Wind should read as one weather event across grass, shrubs, canopy, particles and water - not unrelated sine waves.

## 8. Weather needs material state, not particles alone

Rain and snow particles establish weather, but AAA believability arrives when the world remembers exposure.

### Rain state

- exposed materials darken by family, not by one global multiplier;
- roughness changes selectively; wet does not mean mirror-like everywhere;
- roofs and ledges form runoff paths and drip points;
- sheltered zones remain drier beneath eaves, awnings, trees and bridges;
- puddles follow depressions, compacted ground and failed drainage;
- tires/feet transfer wet dirt across boundary zones.

### Snow state

- accumulation depends on upward-facing slope, exposure, wind and shelter;
- ledges, joints and lee sides catch before vertical faces;
- travelled areas compress, dirty and clear;
- warm/active objects and runoff create local melt patterns.

Transitions matter: dry to damp to wet and clear to dusting to accumulated should be staged over time. Instant binary swaps look like a filter.

## 9. Shoreline and water contact

The large water plane is a good far-field ocean/lake layer. It should not carry the whole shore illusion. Add a local, generated shoreline system when scheduled:

1. shallow-water colour/depth band;
2. shore-following ribbon or mask;
3. intermittent contact foam or ripple events based on wave phase/exposure;
4. wet shore band with darker value/roughness change;
5. debris/vegetation response placed by exposure and waterline.

This converts a geometric intersection into a material transition. Keep foam graphic and intermittent; a continuous white necklace will fight the ink style.

## 10. Outlines as a hierarchy

Black outlines should communicate form, not trace every contrast.

| Edge class | Treatment |
|---|---|
| primary silhouette | strongest and most stable; distance-aware |
| major structural crease | thinner/selective; roofs, door recesses, large panel breaks |
| contact boundary | short, local grounding where object meets surface |
| material seam | usually value/material contrast, not black ink |
| texture/noise edge | never outlined |

The current depth-based, resolution-scaled outline foundation is appropriate. Do not add a universal normal-edge pass: it will over-ink foliage, paving and procedural facets. Add authored structural ink only for deliberate edges and keep material opt-outs.

Proof must cover slow movement and multiple resolutions, not screenshots alone: exterior silhouette, hair/character overlap, thin railings, foliage against sky, paving, interiors and distant settlement edges. Watch temporal crawl, one-frame holes, thickness pumping and dark halos.

## 11. Ambient motion and micro-VFX

Use restrained motion to turn static detail into a living scene:

- chimney smoke tied to occupied buildings and wind;
- dust puffs on dry travel surfaces;
- rain drips at authored roof/awning points;
- insects only in suitable warm/moist zones;
- leaves and litter collecting in wind shadows;
- cloth/sign secondary motion scaled to exposure;
- water contact ripples at posts, shore and rain impacts.

Every effect needs a source, environmental condition, lifetime, distance budget and style rule. Persistent universal particles become visual noise.

## 12. Character presentation belongs in the same frame

The player is the permanent foreground landmark. Validate skin, hair, clothing and future hoverboard against every major environment palette and time of day. Preserve:

- face/eye readability before costume micro-detail;
- stable separation between hair silhouette and environment ink;
- grounded foot/board contact shadow;
- clear pose silhouettes at gameplay distance;
- restrained emissive accents that do not bloom into a different art style.

## 13. Recommended golden-route production pass

Select one representative route - ideally ranch to country road to settlement gateway to main street to landmark/interior - and take it to target quality before expanding systems everywhere.

### Pass order

1. **Lock target views:** one vista, arrival, street, facade, threshold/interior, night/rain view.
2. **Macro composition:** skyline, route, landmark, canopy masses, value hierarchy.
3. **Contact and construction:** terrain joins, foundations, kerbs, thresholds, drainage, roof logic.
4. **Context kit:** occupation-state prop clusters and settlement-specific repeated pieces.
5. **Material/weather:** medium-frequency breakup, wetness/snow rules, repairs and dirt capture.
6. **Vegetation/shore:** ecological clusters, maintained clearances, local water contact.
7. **Motion/polish:** wind unity, ambient VFX, temporal ink, lighting and post-process proof.

Lock the cameras and compare identical before/after captures. Without fixed views, teams can mistake a better camera or weather state for a better asset.

## 14. Visual acceptance matrix

For each locked view, capture:

- 1080p and a lower-resolution stress case if available;
- still frame and slow camera pan;
- noon, dusk/night and overcast;
- dry and rain; snow where climate permits;
- near, mid and far readings;
- gameplay HUD/camera state, not only a presentation camera.

Score 0-2 for: focal hierarchy, route readability, silhouette, contact/grounding, construction logic, material identity, contextual storytelling, weather response, temporal stability and performance. A view is target-ready only when no category is 0.

## 15. Highest-return backlog for Claude

1. Name the golden route and lock six repeatable camera/weather captures.
2. Produce one **hybrid-material A/B** on a representative facade/frontage: current geometry/vertex-colour version versus compact trim/decal/macro variation, without changing the art direction.
3. Define the five material families and their dry/rain/snow response table in data before broad implementation.
4. Prototype local contact improvements on the golden route: foundation, threshold, kerb/drain and road verge.
5. Add one occupation-state kit to a small building cluster rather than scattering unrelated props.
6. Prototype weather accumulation/shelter masks on one roof, wall base and road depression.
7. Build a local shoreline ribbon/contact test rather than altering the far-water plane first.
8. Keep temporal/multi-resolution ink proof as a release gate for every new visual system.

## 16. Common failure patterns

- adding more unique props before establishing settlement identity;
- uniformly distributing detail across all surfaces and districts;
- using decals as random grime stickers;
- allowing every edge to become black;
- making every wet surface glossy;
- relying on AO to ground floating geometry;
- placing vegetation by noise alone;
- making the whole world move at the same wind frequency;
- polishing isolated screenshots while gameplay motion crawls or shimmers;
- expanding a system world-wide before one target-quality slice proves it.

## 17. Implementation brief to Claude

Please treat this as a visual target and sequencing document, not a request to interrupt the active road/movement work. The first decision I need is a disposition for **AQ-028**:

- accept/adapt/defer/reject the golden-route proposal;
- name the proposed route and six locked views;
- state whether the material boundary stays geometry/vertex-colour only or becomes the recommended hybrid;
- identify the smallest facade/frontage A/B that can prove the choice;
- note dependencies on AQ-003 road surface and AQ-009 temporal ink.

## Primary research used

- Epic Games, [Physically Based Materials](https://dev.epicgames.com/documentation/en-us/unreal-engine/physically-based-materials-in-unreal-engine)
- Epic Games, [Decal Materials](https://dev.epicgames.com/documentation/en-us/unreal-engine/decal-materials-in-unreal-engine)
- Epic Games, [Shadowing](https://dev.epicgames.com/documentation/en-us/unreal-engine/shadowing-in-unreal-engine)
- Epic Games, [Post Process Effects](https://dev.epicgames.com/documentation/en-us/unreal-engine/post-process-effects-in-unreal-engine)
- Epic Games, [World Partition HLOD](https://dev.epicgames.com/documentation/en-us/unreal-engine/world-partition---hierarchical-level-of-detail-in-unreal-engine)
- Guerrilla Games, [Between Tech and Art: The Vegetation of Horizon Zero Dawn](https://www.gdcvault.com/play/1025530/)
- Ubisoft, [Procedural World Generation of Far Cry 5](https://www.gdcvault.com/play/1025557/)
- Ubisoft, [Building the World of Assassin's Creed Valhalla: The Nature of England](https://www.gdcvault.com/play/1027284/)
- Blizzard, [The Environmental Look of Diablo IV](https://gdcvault.com/play/1034222/)
- Volition, [Context Art in Saints Row](https://www.gdcvault.com/play/1028749/)
- Sucker Punch, [Invisible Intuition: Blockmesh and Lighting Tips to Guide Players](https://www.gdcvault.com/play/1025179/)
- GDC, [Art Directing VFX for Stylized Games](https://media.gdcvault.com/gdc2017/Presentations/Lindsey_Bryanna_ArtDirectingVFXforStylizedGames.pdf)
- Bandai Namco, [Anime Meets PBR](https://www.gdcvault.com/play/1027995/)
- Adam Robinson-Yu, [The Art of A Short Hike](https://www.gdcvault.com/play/1028679/)
