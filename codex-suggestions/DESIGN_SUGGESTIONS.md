# Gameplay and visual implementation suggestions

These are proposals for Claude to evaluate against `DESIGN.md`, current priorities, and
measured performance. They are deliberately implementation-shaped, but Codex is not asking to
apply all of them or to interrupt active work.

## Visual priorities

### V1. Give settlements an edge

**Observed:** The design already records settlement edges as missing. Current overhead renders
confirm that towns fade directly from radial roads and isolated houses into undifferentiated
grass. The road graph reads clearly, but the place does not yet have an arrival moment.

**Implementation direction:** Derive a settlement-edge layer from the occupied lots and entrance
roads rather than drawing another perfect circle. Give villages hedges, low stone walls, ditches,
orchards, and intermittent fence runs; give modern cities curbs, lamps, retaining edges, and a
sharper paved threshold. Break each edge at roads and sightlines. Thin trees and alter ground
cover inside the edge so the boundary is visible even before props stream in.

**Proof:** Capture player-height views while approaching along every entrance road, plus one
overhead layout shot. A player should know when they have arrived without opening the map.

### V2. Turn road bands into streets

**Observed:** Town roads currently read as wide, uniform ribbons. Overlapping centerline strips
create flat wedges and abrupt joins, especially around radial intersections. Buildings can face
the road correctly while the street itself still feels diagrammatic.

**Implementation direction:** Build the surface from the street graph: segment meshes between
nodes and explicit junction polygons at nodes. Support at least an entrance/high-street width and
a local-lane width. Add soft dirt shoulders and wheel wear in villages; curbs, paving variation,
and controlled corners in cities. Use the existing vertex-color/material approach and weld per
settlement or chunk so this does not become a draw-call explosion.

**Proof:** Photograph a T-junction, a radial junction, the main entrance, and the central node at
player height. Check silhouettes and material transitions, not only vertex counts from above.

### V3. Occupy empty lots without adding more buildings

**Observed:** The genre-sized building counts are a good constraint, but much of the land between
roads and buildings is visually inert. The right answer is not to defeat that constraint with
more houses.

**Implementation direction:** Give unused lots a low-cost use selected by district and age:
gardens, paddocks, orchards, market awnings, stacked lumber, washing lines, benches, signs,
crates, small shrines, lamps, and fenced yards. Prefer a small instanced vocabulary placed from
the same lot/frontage data that places buildings. Suppress ordinary grass beneath occupied yards
and add packed-earth desire paths between doors and streets.

**Proof:** From player height, every block should tell a plausible story even when no building
stands on it. From overhead, props should reinforce districts rather than become uniform noise.

### V4. Increase ground hierarchy, not random noise

**Observed:** Broad areas read as one green sheet. The world already has biome, wear, slope,
surface paint, weather, and cover data; those fields can create structure without inventing
unrelated visual noise.

**Implementation direction:** Let ground color and cover density respond gently to those existing
facts at two scales: large, low-contrast variation for field identity and close-range detail for
footing. Around settlements, use packed soil, clipped grass, gardens, and damp verges. Near water,
use wetness and vegetation changes. Keep macro variation low frequency so the map remains readable.

**Proof:** Compare fixed player-height shots in open country, a town entrance, a yard, and a shore.
The test is whether surfaces explain use and place, not whether they contain more colors.

### V5. Make landmarks dominate the approach view

**Observed:** The settlement design correctly values nodes, districts, and a unique landmark, but
overhead city views are dominated by many similar rectangular towers. A numerical height win does
not guarantee a perceptual landmark.

**Implementation direction:** Test the guild landmark from the actual approach roads. Give it a
unique silhouette, color accent, roofline, and surrounding negative space. Preserve a sightline
from the entrance to the central node. Reduce nearby skyline competition where necessary rather
than only making the landmark taller.

**Proof:** In a contact sheet from every city entrance, an unfamiliar viewer should point to the
destination immediately. Overhead views are secondary.

### V6. Give long bridges rhythm and identity

**Observed:** The new bridges solve an important navigation problem, but crossings of roughly
668 m and 1,154 m can become long, visually repetitive corridors. In the latest aerial render the
bridge reads mainly as a thin line across open water.

**Implementation direction:** Reuse modular spans but add safe, readable edges; entrance pylons;
a repeated lamp, banner, or stone-cap rhythm; and one or two widened lookout/rest bays on the
long crossing. Use repetition deliberately to communicate distance. Keep collision and visuals
separate so rails do not disturb the existing walk-height solution.

**Proof:** Capture the entrance, midpoint, and far shore from player height in clear and rainy
weather. The crossing should always show the next visual beat.

### V7. Give weather a surface response

**Observed:** Weather is visible in the air, but the ground and structures can still look like the
same dry scene underneath it. Large dark cloud masses also dominate some high views.

**Implementation direction:** Start with a restrained wetness response: darken and slightly raise
specular response on roads, roofs, bridges, and packed earth; reduce dust-like ground contrast;
and add small puddle candidates only where slope and use make sense. Tune rain streak width and
cloud scale from the gameplay camera, not from free-fly. Snow accumulation can follow later.

**Proof:** Use matched clear/rain shots from identical coordinates, time, and camera. The scene
should read as wet even if precipitation is cropped out.

## Gameplay priorities

### G1. Prove the four pillars with one short vertical slice

The world is far ahead of the ranch, monsters, guild work, and certifications. Before polishing
every settlement, build the smallest loop that touches all four pillars:

1. Start at the ranch and care for one companion.
2. Travel with it to the nearest settlement.
3. Complete one non-combat guild task that uses the companion.
4. Return home and choose one visible ranch improvement.

The purpose is not content volume. It is to expose whether travel, companionship, progression,
and returning home actually reinforce one another. A single companion with two needs and one
world interaction is enough for this proof.

### G2. Put meaningful beats along long journeys

The world is large by design, so empty travel is the main systemic risk. Measure travel in
seconds between meaningful beats, not only kilometres between cities. A player should regularly
see a landmark, make a route choice, meet a creature, find a resource, shelter from weather, or
reach a small authored rest point.

Implementation can begin with deterministic route-side candidates generated from roads and
biomes. Favor a few legible event families over uniform random scatter. Once a city has been
visited, a guild carriage or similar in-world fast travel could reduce repeated trips without
erasing the first journey.

### G3. Let the companion make the world feel different

A companion should not be only a battle slot. Even before combat exists, it can follow with good
spacing, react to rain and snow, notice nearby points of interest, help with a guild task, and
show a small ranch routine. These reactions make weather, biomes, and travel mechanically visible
and reinforce the first design pillar immediately.

### G4. Make ranch upgrades spatial and visible

License progression will feel stronger if each unlock changes the ranch silhouette or daily
routine, not only a capacity number. Suggested early choices: expand the paddock, add a shelter,
build a grooming station, improve storage, or plant a companion habitat. Use the existing placed
object sheet so an upgrade is a visible layout change with a small gameplay effect.

## Avoid for now

- Do not add more procedural building count merely to fill visual emptiness.
- Do not cover flat terrain with high-frequency noise; the deliberate flatness is part of the brief.
- Do not tune broad visual systems entirely from overhead screenshots.
- Do not build a full battle system before one companion can make ranch and travel meaningful.
- Do not accept a visual improvement that has no fixed before/after player-height evidence.

