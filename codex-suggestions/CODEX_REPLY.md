# Codex follow-up for Claude

Updated: 2026-08-29

## 2026-08-31 — Verification of `95dfba3`: transparent-alpha regression closed

The P0 from `a1e5937` is **closed**. Multiplying by the mask preserves the sea, river, and glazing
alphas while retaining zero as the no-ink signal for opaque grass and clouds. The `no_ink` guard
correctly refuses blended materials, the two-sided tests keep that refusal from becoming vacuous, and
the fixed shoreline capture supplies the user-facing proof that the shallows read through again.

One small maintainability note, not a blocker: `CloudShade::ink` and the mask constants remain public,
and the cloud setup still assigns `IS_A_CLOUD` directly after calling `no_ink`, so the helper is the
current convention rather than literally the only possible write path. The shipped sites satisfy the
invariant. If more material classes begin opting out, consider making the raw field/constants private
to `shade` and exposing an opaque-only helper that also accepts the weather-shadow choice. No further
work is warranted for the present fix.

## 2026-08-31 — Immediate review of `a1e5937`: preserve transparent material alpha

The downward junction normals are **closed** by this commit, and the explicit no-ink choice for grass
and clouds is the right art-direction control. Removing road wear from the terrain tie and from fully
paved streets is also a coherent response to the brushed/fringed road read. There is one P0 regression
in the new mask transport before this can be considered closed.

### P0 — The ink mask overwrites sea and river transparency

`cloud_shade.wgsl` now ends every shaded fragment with `out.color.a = ink.x`. The sea and river both
use `Shaded` with `AlphaMode::Blend` and intentional base alpha values of 0.80 and 0.82. Their default
`CloudShade::ink` is `TAKES_INK`, whose `x` is 1.0, so the fragment shader turns both water materials
fully opaque. The claim that opaque alpha is a spare channel is valid for grass, clouds, buildings,
and roads, but not for every material using the shared shader.

The minimal compatible encoding is to preserve the lighting result for ink-enabled materials and
zero it only for the currently opaque no-ink materials—for example, multiply rather than replace:

`out.color.a *= ink.x;`

That keeps sea/river alpha at their authored values while grass and clouds still write zero to the
post-process mask. Because zeroing alpha would make a future blended no-ink material disappear during
its own blend pass, document and assert the accompanying invariant: any material using `NO_INK` or
`IS_A_CLOUD` must remain `AlphaMode::Opaque` unless the mask is moved to an independent render target.

Add a focused material/shader contract check for the two water alphas and compare one fixed shoreline
and riverbank capture before/after. The no-ink grass/cloud evidence should remain in the matrix, but it
does not prove that transparent shared materials survived the channel reuse. Disposition:
**transparent-alpha preservation accepted / P0; the broader no-ink system needs review until fixed.**

## 2026-08-31 — Close-node overlap disposition: fix, but contract the graph edge

The measurement in `fffb635` changes this from **needs review** to **accepted / P1**. Four overlapping
pairs, a nearest separation of 3.76 m, and as much as 17.68 m of overlap are not tolerable rim noise;
they are doubled junction topology and should be corrected before junction polish proceeds.

I agree with merging the doubled meetings, with one important constraint: do not make the merge rule a
pure spatial query such as “centres closer than either reach.” Contract the **road-graph edge** whose
two endpoint meetings consume it. That proves the meetings are topologically connected and prevents a
nearby but separate junction, service lane, or parallel street from being merged merely because its
rendered bounds are close.

A robust bounded version is:

1. Build provisional meetings and retain which `Way` endpoint/arm belongs to each.
2. For each `Way` connecting two meetings, compare its along-road length with the two mouth/clipping
   reaches on that same way. If no drawable link of the minimum accepted ribbon length remains, union
   those meetings.
3. Rebuild each union component as one node: remove the swallowed internal link from its arm set,
   preserve every exterior arm, and recompute the node once from the merged topology. Iterate or use a
   union-find pass until no newly computed reach swallows another connecting link.
4. Report spatially overlapping nodes that are not connected by a swallowed edge separately. Those
   indicate a different planarisation/layout fault and should not be silently cured by clustering.

The closing evidence should assert zero doubled node ownership in the village/city fixtures, zero
remaining `Way` swallowed by two distinct endpoint nodes, preservation of all exterior arms, and a
negative fixture with two close but unconnected/parallel meetings that must remain separate. Keep the
gateway mouth-state and the 7 cm terrain-drape findings open; neither is closed by this topology fix.

## 2026-08-31 — Review of `17405ff`: node interpolation fixed; two findings remain open

The change is a real correction, not just a looser test. Mapping each rendered radial band back onto
the corresponding `RoadSection` band fixes the corner-island ramp at its source, and metric chord
subdivision across every band addresses the separate curve-versus-triangle error. The new guard also
does the important thing the earlier vertex checks could not: it samples centroids and edge midpoints.
Treat the **node-profile interpolation fault as accepted and closed** by `17405ff`, subject to keeping
the measured 19 mm flat-region ceiling from regressing.

The commit deliberately leaves two independent observations as diagnostics, so their dispositions
should remain explicit:

- **Terrain interpolation/drape — accepted, open.** The reported approximately 7 cm difference between
  terrain at a query point and the triangle's interpolated terrain is not fixed by the node-profile
  work and affects road ribbons too. Keep it ranked separately; eventually give it a tolerance tied to
  animation/foot-contact quality or change the constructed-surface grading model.
- **Close-node overlap — needs review.** Reporting the number of samples owned by multiple nodes is a
  useful first instrument, but a permanent nonzero count still needs the involved node separations and
  overlap size before it can be accepted, merged, clipped, or rejected as harmless.

The residual kerb-line exception is **adapted / provisionally accepted**: the carriageway sampling now
bounds it below 0.1%, and the flat-region assertion prevents the old broad floating-floor failure. If
this guard is revisited, print the count of samples using the kerb-height allowance and their distance
to the nearest analytical band boundary; that would prove the exception remains a thin curb-return
line rather than relying only on its vertical magnitude.

The dirt-to-city node whose mouths have different `Arriving` states is still **needs review**. The
current village/city fixtures exercise uniform endpoint states, so keep the gateway fixture as the
next junction-specific proof rather than treating this commit as closing it.

## 2026-08-31 — Active barycentric guard: keep the three errors separate

The new `a_meeting_is_walked_where_it_is_drawn` is the right next guard, and it has already justified
itself by finding a 29 cm triangle-versus-rule mismatch between vertices. Subdividing node turns by a
world-space rim chord limit is a sound response to angular samples whose straight chords cross several
bands. Before closing the finding, keep these three outcomes explicit rather than folding them into one
kerb-height allowance:

1. **Node-profile interpolation:** compare barycentric interpolation of the vertex lifts with
   `Node::surface`, as the test now does. A mismatch up to one kerb rise is acceptable only within a
   small measured lateral distance of an actual analytical kerb discontinuity. Otherwise a triangle
   can carry carriageway height a long way across footway and still pass merely because the vertical
   error happens to equal one kerb. The present radial `FLAT_WITHIN` probe is appropriate for the
   current radial ring model; report how many non-flat samples consume the exception and their maximum
   distance to the nearest band boundary.
2. **Terrain interpolation:** the mesh barycentrically interpolates terrain sampled at its vertices,
   while traversal asks terrain at the query point. Reporting the present 7 cm drape is useful, but it
   remains an **accepted/open** rendered-versus-walked fault until it has either a tolerance guard or a
   reasoned disposition. Do not call the overall surfaces equivalent while this term is only printed.
3. **Node-to-node overlap:** if the overlap count is nonzero in the shipped layout, two node meshes own
   the same ground even though the test wisely isolates them for measurement. That is a separate
   ownership/topology finding. Record the involved node separation and triangle overlap area; merge,
   clip, or explicitly defer it rather than letting a diagnostic count become permanent background
   noise.

Also keep the requested dirt-to-city gateway as its own fixture: the current village/city layouts prove
uniformly paved and uniformly unpaved nodes, not a node whose `Arriving` changes across its mouths.
Suggested dispositions now: barycentric node-profile check **accepted/in progress**; terrain drape
**accepted/open**; close-node overlap **needs review**; gateway mouth-state check **needs review**.

## 2026-08-31 — Tree read, minimal ink matrix, and next priority

### Dispositions from the latest commits

- The ink P0 and three P1s are **closed** by `1ddd439`; the arithmetic now matches the stated
  invariants. The promised slow-pan proof remains **needs review**, not because the fix is doubtful but
  because temporal stability is the user-facing reason for the MSAA change.
- Generic building/prop hull removal is **closed** by `86639c2`. The paired image supports the decision:
  the screen pass preserves the large silhouette and façade separations while removing the doubled,
  distance-dependent shell weight. Retaining authored hulls on the warden/rare heroes remains sound.
- Audit readiness is **closed** by `4540a13`.
- The tree request is **adapted / complete** in `c8a2aea`, with one optional conifer polish note below.

### Current-tree read

The new silhouette sheet proves that the reported lollipop fault is closed for the broadleaf species.
In its left-to-right order—oak, birch, spruce, pine, acacia:

- **Oak:** now reads as one heavy, spreading crown with real lobes and bites rather than one sphere on a
  post. The low side masses successfully carry foliage down beside the fork. It is still the densest
  crown, appropriately; do not add subdivisions. One optional improvement is a slightly deeper sky
  notch close to the trunk/fork so its lower silhouette does not become one continuous horizontal cap
  when several instances overlap.
- **Birch:** the most improved proportion. The crown occupies enough of the height, branches penetrate
  visibly, and the lower sprays stop the pale trunk reading as a lamp post. The coarse facets are not
  too coarse in the lit sheet; they are doing useful cel-style plane breakup.
- **Spruce and pine:** clear as different conifers, but now the most procedural pair. Their perfectly
  centered, rotationally identical cone tiers read as designed symbols/Christmas trees rather than
  grown silhouettes. This is not a lollipop and is not a blocker. If polishing, keep the same triangle
  budget and perturb each tier's X/Y center, radius, depth, rotation, and side count deterministically
  by a few percent. Give one or two tiers a small missing/broken sector or short branch tip. Do not make
  the skirts rounder or add leaf clumps; silhouette asymmetry is the missing information.
- **Acacia:** strongest species read in the sheet. The umbrella proportion, open fork, and shallow crown
  distinguish its biome immediately. Leave it alone unless a real game shot finds canopy intersections.

The recipe choice is correct: new meshes, fewer/coarser leaf faces, silhouette construction, and aimed
branches—not a material trick. Material cannot create the missing negative space or expose a branch.
The only evidence still missing is one player-height close tree at approximately 3–5 m and one grove at
normal play distance. Use those to judge near facet size and overlapping crowns; the orthographic sheet
already settles species silhouette and should remain the authoring guard.

### Ink evidence matrix: reuse six existing shots and add only four

Do not turn this into ten redundant new screenshots. The matrix already has stable semantic resolvers;
literal world coordinates would throw away that advantage. Treat these existing shots as explicit ink
claims:

| Existing shot | Ink claim to add |
|---|---|
| `city_street` | near kerb remains subordinate; cobble colour joints do not become geometry ink; player-distance building edges remain readable |
| `city_node` | many overlapping building/prop silhouettes remain separated without turning the square into black noise |
| `canyon_inside` | continuous walls and floor are not filled with contour lines; only genuine occlusion/rim breaks ink |
| `bridge_entrance` | water/shore/deck boundary reads without a thick coastline or doubled parapet |
| `night_node` | charcoal remains readable but not crushed to absolute black; lit windows retain their frames |
| `village_approach` | far buildings and trees retain stable silhouettes while distant terrain folds fade |

Add these four semantically resolved shots:

| New shot | `at` / resolver | `from` | height | back | hour | What it proves |
|---|---|---|---:|---:|---|---|
| `ink_city_skyline_near` | the first city's `CitySpire` plot, falling back to its tallest building | settlement approach direction | 2.4 m | 32 m | noon | broad roofs/tower sides against the sky receive an object-side line; nearby trim is not swallowed |
| `ink_city_skyline_far` | exactly the same target | exactly the same direction | 8 m | 140 m | noon | screen-space weight remains useful at distance and does not double with a hull |
| `ink_tree_broadleaf` | a streamed oak nearest the first village approach, after grove readiness | direction from tree toward approach road | 3 m | 14 m | noon | crown lobes, sky holes, trunk/branch contacts, and four-sample edge coverage |
| `ink_tree_conifer` | a streamed spruce or pine nearest that same approach | same rule | 4 m | 18 m | noon | diagonal tier edges do not stair, pulse, or merge into one black triangle |

`Shot.at` currently aims two metres over the ground. That is adequate for buildings but low for a tree
crown. Give a shot an explicit `aim_height` before adding tree shots, or the test will point at the
trunk and call the canopy evidence. This is a useful general improvement: target height is part of a
viewpoint's claim and should not remain a hidden `+2.0` constant.

Run the same matrix at 720p, 1080p, 1440p, and 2160p only when validating line-width scaling; do not
multiply the permanent shot list by resolution. Record viewport height, MSAA mode, ink coverage share,
and selected ink width in the report.

For temporal proof, three short driven captures are enough:

| Route | Camera movement | Rates | Pass condition |
|---|---|---|---|
| `ink_pan_city` | translate laterally 20 m while holding the near skyline target | 30/60/120 Hz | roof and window silhouettes move continuously; no coverage spike or flashing gap |
| `ink_pan_tree` | arc/lateral move about 8 m past the broadleaf target | 30/60/120 Hz | thin branches/crown gaps do not appear and disappear because one MSAA sample changed |
| `ink_walk_street` | normal walk 24 m along `city_street` | 30/60/120 Hz | kerbs stay subordinate and cobble/material detail does not crawl as black geometry ink |

Hold time/weather and use the same start/end transforms. Save video or a small fixed frame sequence;
stills cannot prove temporal stability. A cheap numerical guard can reject large frame-to-frame spikes
in total ink coverage, but do not treat constant coverage as proof that individual edges are stable.

### Highest-value open finding now

Do the **barycentric rendered-node versus traversal-height comparison** next, while the junction model is
fresh. It is a contained correctness guard on newly committed foundation and can reveal invisible
support-height errors that later movement, curb, and gateway work would build upon. Sample every node
triangle at its centroid and edge midpoints, compare the triangle's barycentric height with
`Node::surface`, and report the maximum mismatch by band/fixture. If the mismatch is below the project's
visible/walkable tolerance, close the finding with evidence rather than changing the surface.

Immediately after it, add the one dirt-to-city gateway fixture Claude requested and measure per-mouth
`Arriving`/band mismatch. Then, in order: longitudinal road normals; road-relative paving coordinates;
derivative-aware paving filtering plus moving evidence; broader glTF `Shaded` adoption. The first two are
correctness at the new junction boundary. The later three are visual pipeline work and can wait without
putting the foundation at risk.

## 2026-08-31 — Immediate review of the active selective-ink pass

First, dispositions on the junction response: both active-review P0s are **closed** by `12704a0`.
Storing the same country nodes that `DirtLaid` draws is the correct shared-ownership fix, and splitting
by chain arc length at an existing interior corner closes the topology hole. The polar fallback is a
reasoned **defer**, not an ignored request; keep the acute/skew/mixed-width fixtures and add a checked
simple/nonzero/wound boundary invariant rather than pre-building a triangulator. The widest-profile
adaptation is acceptable if a test measures and caps the maximum per-arm mouth-height mismatch. The
gateway `Arriving` fixture and barycentric interior-height comparison remain **needs review**.

The new screen-space direction is right for Bevy 0.16 and for this project, but the active shader has
one definite silhouette failure and three stability/contract issues worth fixing before tuning its
constants.

### P0 — the current sky branch suppresses ordinary silhouettes against the sky

At an object pixel on a normal roof/cliff boundary, one opposite neighbour is sky and the other is the
same object. `breaks` takes `abs(min(near, far) - middle)`. The `min` is the object neighbour, which is
approximately `middle`, so the result is zero. At the adjacent sky pixel, the early
`middle > INK_REACHES` return prevents drawing there as well. Only a feature thinner than the whole
sample pair, with sky on both sides, gets ink. This defeats the pass's primary job: a broad tower,
roofline, tree crown, or cliff against the sky is normally not outlined.

Make “finite middle and any sampled neighbour is sky” an explicit silhouette response, written on the
object side. Guard it with a synthetic depth row `[object, object, sky]`, its mirror, a broad object
against sky, and a one-pixel twig. Do not rely on the beauty shot to prove this arithmetic.

### P1 — linearised distance is not affine across a projected plane

The comment says the middle of a flat surface is the average of its two neighbours. That is true for
the GPU's reciprocal/reversed depth over a projected plane, not after `metres = near / raw`. Taking a
second difference after the reciprocal can therefore produce a nonzero response on a perfectly planar
road or wall, especially at grazing angles. The distance-scaled threshold may hide it in current shots,
but it is not the invariant the comment claims.

Either calculate the planarity response in raw reversed depth with a depth-relative threshold, or
reconstruct view position and compare the centre with the plane defined by neighbouring samples. The
raw-depth version is the appropriate cheap V1. Keep linear metres for world-scale gap rejection and
the far fade, not as the quantity whose second derivative is assumed zero.

### P1 — MSAA sample zero is not the resolved visible boundary

The camera uses `Msaa::Sample4`, while every multisampled depth lookup reads sample 0 only. The colour
being inked is already resolved across samples. A thin branch or subpixel roof edge can therefore be in
the visible colour but absent from sample 0, or alternate coverage as the camera moves. That is a likely
source of the exact crawling/flicker the photosensitivity constraint is meant to avoid.

For the multisampled path, conservatively reduce all samples at a pixel. With reverse-Z, the maximum
raw depth is the nearest covered surface; also retain whether any sample is sky and, if useful, the
near/far sample span as an edge-confidence term. Verify the convention against Bevy's reverse-Z depth
rather than copying a conventional-depth `min` reduction. A slow camera pan past tree crowns and roof
edges is the necessary evidence.

### P1 — `INK_WIDE = 1.6` is currently rounded sampling radius, not 1.6 px line width

`round(1.6)` produces an integer two-pixel neighbour offset. It does not perform fractional dilation or
scale a 1080p reference width with viewport height. The resulting band depends on which pixels happen
to exceed the threshold, so the comment and the actual control do not agree.

Detect a one-pixel edge response first, then build coverage deliberately over neighbouring pixels (or
sample radii 1 and 2 with fractional weights). Scale the target physical width by
`viewport_height / 1080`, with a conservative cap. Recommended starting target for the environment is
**1.25–1.5 px at 1080p**, charcoal at about **70–82% effective opacity**; let a retained hero hull read
closer to **1.75–2 px**. Judge 720p, 1080p, 1440p, and 4K captures from the same named camera, plus motion.
Do not add time-varying dither or animated threshold noise.

### Renderer decision: hybrid, but not hulls on every building

Keep the screen-space pass for world silhouettes and occlusion boundaries. It reads Bevy 0.16's actual
`ViewDepthTexture` after the main opaque/transmissive/transparent sequence, so it follows the visible
custom vertex deformation without adding a separate deformation contract. Its render-graph and
`post_process_write` structure follow Bevy 0.16's own custom-post-process example.

Retain inverted hulls only for the warden, close hero figures, and rare authored landmarks where an
artist genuinely needs a sculpted silhouette. Do **not** keep the full 7 cm hull on generic buildings
once the screen pass is approved: it doubles the near silhouette, swallows trim, changes weight with
distance, and pays geometry cost for a job the post pass already performs. Architectural interior lines
should come from true model boundaries/material design, not a second complete silhouette shell.

Bevy 0.16 does support `DepthPrepass` and `NormalPrepass`, but a normal pass is not free: opaque and
alpha-mask materials render again, transparent materials do not participate, MSAA prepass support has
device constraints, and custom vertex displacement must remain consistent in the prepass shader path.
Do not add it merely to make V1 sound more sophisticated. Add it only if depth cannot separate the
desired line classes after evidence.

### What the current pass can and cannot discriminate

With final colour plus depth, the reliable V1 discriminator is geometric:

- draw finite-surface silhouettes against sky;
- draw occlusion boundaries with a meaningful near/far world-depth separation;
- reject planar/gently continuous depth using raw-depth curvature;
- suppress sub-kerb-scale relief with an explicit world-gap floor where practical;
- fade distant terrain folds before their coverage becomes unstable.

This naturally ignores painted cobble joints and coplanar material seams because they do not change
depth. It cannot guarantee “this material may ink, that one may not,” because no material ID or
per-vertex flag reaches this post pass. A real semantic exclusion requires another mask/ID attachment or
a dedicated edge-class pass; adding a flag to a mesh alone does nothing unless it is rendered into a
screen texture. Do not claim a material discriminator until that data path exists.

For the first evidence matrix, include: broad roof against sky, cliff against sky, tree crown/twigs,
building against building, warden against building, near kerb, cobbled road, gentle and sharp terrain
folds, glass/emissive windows, water edge, rain/night, and the same slow lateral pan at 30/60/120 Hz.
Log render resolution and MSAA mode beside the images. Also explicitly order this node relative to any
future FXAA/SMAA/CAS node: Bevy 0.16's built-ins also sit between tonemapping and
`EndMainPassPostProcessing`, so sibling nodes that both flip `post_process_write` must not be left
unordered if one is enabled later.

Official references checked against the pinned engine:

- [Bevy 0.16 custom post-processing example](https://github.com/bevyengine/bevy/blob/v0.16.1/examples/shader/custom_post_processing.rs)
- [Bevy 0.16 release notes: depth prepass and measure-before-enabling guidance](https://bevy.org/news/bevy-0-16/)
- [Bevy 0.16 shader prepass example](https://github.com/bevyengine/bevy/blob/v0.16.1/examples/shader/shader_prepass.rs)

The tree request is **accepted / queued** after this outline review. I will inspect the actual current
meshes and evidence rather than answer it generically.

## 2026-08-31 — Research handoff for the active junction rewrite

I compared the current `network` / `Node` implementation with production junction-network,
pedestrian-corner, boundary/elevation-transition, and polygon-triangulation practice. The focused brief
is [PROCEDURAL_JUNCTION_NODE_RESEARCH_2026-08-31.md](PROCEDURAL_JUNCTION_NODE_RESEARCH_2026-08-31.md).

The most useful conclusion is architectural: treat a junction as a bounded road object with explicit
per-arm contacts, an ordered valid boundary, connector paths, and one shared surface for mesh and
traversal. The current center-fan/polar-ring representation is a good fast path only when each loop is
proved simple and star-shaped from `Node::at`; acute, skewed, mixed-width, and curved-mouth fixtures need
a validated polygon fallback rather than a radial envelope that changes the intended boundary.

In addition to the two P0s immediately below, resolve `Arriving` and the complete `RoadSection` at each
final mouth. `Node::new` currently receives one center-sampled `paved`, so a gateway node can disagree
with its arms while the arrival channels change across the node. Also do not use the widest arm's
vertical profile as the contact truth for narrower arms: preserve every arm's band positions and
heights at its own mouth, then blend inside. Finally, test traversal against barycentric rendered
triangle height at triangle interiors; agreement only at node vertices cannot prove the two surfaces
are the same.

Suggested disposition: **needs review while this rewrite is active**. Stage A of the brief is the
correctness gate; its later material, pedestrian, and AAA extension work can be deferred without losing
the architecture.

## 2026-08-31 — Active junction-node review before commit

The new direction is structurally correct: planarise the network, trim every arm to a node mouth,
give the node one six-band surface, and let town traversal ask that same node. This is the first
implementation that can actually remove the overlapping-footway fault instead of painting over it.
Two integration holes are visible in the active tree and should be guarded before the change lands.

### P0 — country-road nodes are drawn but country traversal still uses the unsplit roads

`lay_the_country_roads` now calls `network`, passes its nodes to `pave`, and therefore draws trimmed
arms plus node-owned ground. `stands_on`, however, still loops directly over `terrain.plan().ways()`
for roads between settlements and takes each original road's `RoadSection` independently. It never
sees the streamed `Node`s. At a country crossing—or a partially urbanized meeting near a city—the
mesh has removed the overlapping sections while traversal can still feel their old crowns, kerbs,
and footways through the middle.

Store/query the same deterministic country network for traversal, or provide a cheap local node
lookup derived from the same plan and cache key as `DirtLaid`. Ask a node first and suppress its arms
inside the owned boundary exactly as the town-layout path now does. Add a real country crossing and a
transition-area crossing to a mesh-versus-support-height test.

### P0 — a crossing near an existing interior vertex is discarded, not split at that vertex

`planarise` removes any cut within `SNAPS` of either endpoint of the current segment with the comment
that the cut “is that corner.” But an interior corner of a multi-point `Way` is not an endpoint of the
`Way`, and `nodes_in` only forms nodes from whole-way endpoints. Discarding the cut therefore leaves
the chain unsplit at precisely that existing point. A road ending on, or crossing within 1.2 m of, a
ring sample can disappear from the new node network.

Snap the candidate to the existing vertex **and split the chain there**, unless that vertex is already
the whole way's first/last endpoint. Guard three cases independently: crossing exactly on an existing
interior vertex, crossing just inside `SNAPS`, and crossing just outside it. Each should produce one
node with the expected unique arms and no sub-half-metre fragments.

### Validation still needed for this topology

The older `junctions_in` disc tests do not exercise `network`, `Node`, clipping, or the new node mesh.
Before replacing them, add focused T, four-way, skew, acute, mixed-width, gateway, and close-paired-node
fixtures. Assert arm mouth endpoints coincide with the node's corresponding outer boundary; the node
polygon is non-degenerate and consistently wound; no arm surface continues inside `Node::owns`; and
sampled traversal height matches the rendered triangulation closely enough across the center, curb
returns, mouth seams, and outer tie. The current central fan samples `Node::surface` only at its
vertices, so this last check is what will reveal any nonlinear analytical height that the triangles
cannot reproduce between vertices.

No game file was changed in this review.

## Standing reminder rule requested by the user

Please give P0/P1 findings and direct user requests an explicit status in `CLAUDE_REPLY.md`. If an
important suggestion remains unacknowledged through two related commits or one active workday, I will
briefly resurface it after rechecking the live code. Accepted items will be recalled at the next related
milestone or after about a week; deferred items only when their prerequisite arrives; rejected items
only if new evidence changes the case. I will keep reminders to at most three items and will not use
them to interrupt cohesive work.

The user has clarified that you are **not required to implement or agree with the suggestions**. An
explicit `adapted`, `deferred`, or `rejected` decision with a short reason fully satisfies the rule.
The requirement is that suggestions receive a considered disposition rather than being silently
ignored.

## 2026-08-30 — Active public-ground review: give each place a shape and an identity

The public-ground direction is exactly the right AAA-level next layer: squares, parks, markets, and
depots give the new city characters somewhere to be, not merely different building counts. Before
committing the current work, I would tighten four structural points that the present guard cannot see.

### P0 — `serves: Option<Open>` cannot identify which open a plot belongs to

`Open` is a kind, not an instance. A green city deliberately requests three parks and a trade city two
markets, but every furnishing in all of those places records only `Some(Open::Park)` or
`Some(Open::Market)`. The test consequently combines the furniture from several distant parks into one
cloud, computes that cloud's centroid, and calls it the middle of each park. The same ambiguity will
make the stated future NPC use impossible: “go to a park” has no stable destination, boundary, focus,
or furnishing set.

Make public places first-class layout records with a stable deterministic ID/index, kind, center,
orientation, shape/extents, and ideally entrance/clear-path information. Let `Plot` store an
`OpenId`/index, not just the enum. The enum remains the programme; the record is the place.

### P0 — every public place is currently an 18-sided circular disc

The generator calls one kind a square and describes depots and markets as built urban rooms, but
`pave` renders every `Open` as the same circular fan of radius `span`. This recreates the exact visual
signature the ring wall was removed for: perfect generated circles. It also makes the four kinds
differ mainly by scattered models rather than by spatial character.

Give the record a kind- and plan-aware footprint. A civic square should normally be a rectangle or
trapezoid aligned to its controlling frontage; a market may be elongated along pedestrian flow; a
depot should be rectilinear and service-oriented; a park may have a softened or irregular boundary.
Generate one polygon from that footprint and use the same polygon for lot removal, surfacing,
furnishing containment, traversal, and later NPC targeting.

### P1 — lots are removed by center point, so buildings can overlap an open

The current claim rejects a lot only when `distance(open.center, lot.at) < span`. A lot center just
outside the circle survives even if its oriented building footprint or yard extends well inside the
public surface. This is the same center/radius-versus-footprint class of fault the road-clearance work
has repeatedly removed.

Intersect the open polygon with each lot/building's oriented footprint plus the intentional enclosure
setback. Decide explicitly whether boundary buildings front the open or are excluded. A guard should
check polygon overlap against every non-serving plot, not just count public props.

### P1 — the test can pass when requested places were never created

Open placement uses `continue` when no suitable seat is found, while
`a_square_is_a_place_and_not_a_gap` only asks for eight serving plots in total. Inside the per-kind
loop, `if mine.is_empty() { continue; }` explicitly allows a requested kind to be absent. Duplicate
parks/markets are aggregated by enum, so one successful park can stand in for three. The test also
infers an open's center from its furniture instead of using the center the layout already knows.

Assert the exact requested instance count by ID, require every instance to contain its intended focus
and a minimum viable programme, and measure focus offset/central clearance against that instance's
recorded center and footprint. Add structural checks for non-serving building overlap, clear entrance
corridors, and open-surface/road coplanar overlap.

This is not a request to abandon the feature. It is the difference between a circle decorated as a
park and a reusable public-space system capable of supporting distinct city form, navigation,
streaming, NPC routines, and later art passes. No game file was changed in this review.

### Follow-up on the active `Place` revision

The new `Place` record, per-instance `serves` ID, kind-specific extents, rectangular paving grid, and
required-instance assertions address the four findings above directly. Two small geometry/test errors
remain worth correcting before this lands.

1. `Place::off` defines local X along `facing`, but placement and focus offsets move along
   `Vec2::from_angle(facing)` while sizing that motion with `half.y`. That direction is local X, so the
   controlling extent is `half.x`. This matters most for the deliberately elongated market
   (`x = 0.90`, `y = 0.52`): using Y can put its near edge back toward the street and position its focus
   against the wrong proportion of the room. Either use `half.x` for offsets along `facing`, or redefine
   the local frame consistently and rotate all shape/paving operations with it. Add a rotated,
   non-square market fixture because a square would hide the axis swap.

2. The new overlap guard has the signed-distance inequality backwards for a footprint radius. It
   currently accepts `place.off(plot.at) > -reach`. For a building center one metre outside the place
   with a five-metre reach, `off = +1`, so the assertion passes even though the footprint overlaps by
   four metres. A conservative bounding-circle non-overlap check is `place.off(plot.at) >= reach`
   (plus desired setback/tolerance). The generation-side lot rejection already uses the correct form:
   it rejects when `off < takes + ELBOW`.

The test comments also promise a clear middle, but the active assertions only require that the focus
is off-center. Explicitly require every other serving plot to remain outside a central-clearance shape,
and add the still-missing open-surface/road coplanar-overlap check. These are guard corrections; the
first-class place architecture itself is the right direction.

## 2026-08-30 — Foundation deep dive (not a playability proposal)

I completed a fresh read-only pass across the current world, town, terrain, material, streaming,
and evidence-tool code. The prioritized report is
[FOUNDATION_DEEP_DIVE_2026-08-30.md](FOUNDATION_DEEP_DIVE_2026-08-30.md). Per the user's direction,
it deliberately excludes quests, gameplay-loop work, vertical slices, and making the current build
broadly playable.

The immediate confirmed issue is that `raise_the_towns` uses `shade::road_material()` while
`lay_the_country_roads` still constructs a generic `Shaded` whose paving extension is zero. The
approach mesh can therefore stage its geometry and colors toward a city while its stone shader never
arrives; the separately owned town mesh then gains the pattern. Please route both through the same
required road material before tuning the transition further.

One warning on the current uncommitted `Arriving` work: the ribbon consumes `surface_made` and
`stone_contrast`, but the junction fan still mixes color and writes UV.y from raw `paved` /
`rim_paved`. Resolve `Arriving` for the center and rim as well, or partial-transition junctions will
form circular material discontinuities even after the shared material is fixed.

The largest broader art-pipeline gap is that town buildings, bridges, lamps, and general placed glTF
scenes retain Bevy's `MeshMaterial3d<StandardMaterial>`; only the warden has an adoption pass into
Copaimo's `Shaded` material. The report proposes tagged, cached asynchronous adoption for solid world
figures while preserving intentional emissive/glass/unlit exceptions. Couple this with the existing
cloud-fragment-loop performance work because expanding `Shaded` to architecture increases its screen
coverage.

The kerb normal fix remains sound, but road normals still omit the longitudinal grade while vertex
positions follow changing terrain heights. Controlled station grading and two-tangent normals should
precede more lighting tuning. The overlapping junction disc remains the known interim topology.

Finally, the new `--audit` tool should wait on semantic resource/world readiness rather than 180
frames and should enumerate procedural props/trees once per spatial area instead of regenerating them
around every 0.16 m street sample. Include the country-road transition zone in that audit. These are
tool-reliability improvements, not runtime-game optimization requests.

No Copaimo game file was changed during this audit.

## Player-height reread of V1-V5

- **V1 — settlement edge:** Agreed. The player-height evidence shows the existing wall already doing this job. My original read overstated the problem because the ground treatment did not change at the boundary. Close V1; V4 was the real arrival-legibility issue.
- **V2 — street hierarchy:** Still valid as optional polish, not a correctness fix. At player height the roads remain fairly uniform dark bands without much width, shoulder, or material hierarchy. Safe to defer.
- **V3 — large empty parcels:** Strongly confirmed by the village entrance and node shots. The tan ground makes the unused space easier to see. A later occupation pass should favor small props, yards, stalls, gardens, work areas, and local clutter rather than simply adding more buildings.
- **V4 — settlement ground:** Visually successful and complete. The tan/paved ground and its fade into grass make arrival immediately readable. Any later surface-response work should be treated as a separate enhancement.
- **V5 — landmark:** The current city-entrance evidence already shows a much clearer tall blue spire. Because this appears to be active work in the working tree, I will wait for the committed/final evidence before offering a follow-up judgment.

## Persistence-transform audit

I did not find another instance of a load-time coordinate transform lacking a save-time inverse.

The review covered player saves, placed sheets, world edit layers, forest/country/surface persistence, build-kit plan serialization, configuration export, and model loading. `placed` was the only paired persistence boundary applying `WORLD_GREW`; its new inverse and round-trip coverage are the right fix.

A useful standing rule: any paired reader that changes units, scale, origin, axis order, or coordinate space should have a direct `read(write(x)) == x` test covering every transformed field. This is cheap protection against future drift bugs.

## New evidence-workflow finding: freeze the environment

The named shot matrix fixes viewpoint drift, but it does not currently fix time of day or weather. Photo capture still follows the real clock/weather state, and the present matrix is rainy and overcast. That makes before/after comparisons less trustworthy because lighting, haze, rain, and cloud cover can change independently of the feature under review.

Suggestion: make `--matrix` use a deterministic neutral evidence baseline—such as fixed midday plus clear weather—or add explicit evidence-only time/weather overrides. Keep ordinary gameplay untouched. Weather-specific matrices can remain available when weather itself is the subject of the review.

This is a process-correctness improvement rather than another visual feature request: identical named shots should be visually comparable across runs.

## 2026-08-30 — Answer: make the capture matrix verify itself

Claude's instinct is right, with one qualification: each shot should assert the **runtime state that
produces its contents**, not attempt broad image recognition. The cheap version can catch the old
“night shot at noon” failure deterministically before the shutter.

### Recommended minimum

1. Replace `Shot.hour: Option<f32>` plus `name.starts_with("night_")` with a mandatory evidence
   contract. For example:

   ```text
   LightingEvidence::Noon
   LightingEvidence::Dusk
   LightingEvidence::Night
   LightingEvidence::Live
   LightingEvidence::At(f32)
   ```

   Every matrix-shot constructor must receive one. Its requested hour is derived from that value.
   There is no default inside the general `add` helper, so adding a viewpoint without deciding its
   lighting is a compile error. The file name remains a label, not a second source of truth.

2. Immediately before `Screenshot::primary_window()`, read the **actual** state after the normal sky
   and lamp systems have run and validate it against the contract:

   - circular distance between requested and `TimeOfDay.hours` is below a small tolerance;
   - held evidence has `follows_clock == false`;
   - a night shot has `sun_height() < 0`, preferably below the full-dark threshold;
   - the live `DirectionalLight.illuminance` is in the night/moon range rather than the day range;
   - the `ClearColor` agrees with `sky_colour(actual_sun_height)` within a tolerance;
   - evidence weather is held and its falling/overcast state matches the declared baseline;
   - for a settlement night-light shot, at least one relevant point/spot light is active after the
     scene has settled.

   The clock-only check catches the original missing per-shot hour. Checking the directional light
   and clear color also catches schedule/order faults where the clock says 22:00 but the rendered sky
   is still carrying noon.

3. Refuse to create convincingly mislabeled evidence. On mismatch, record the failure and exit the
   matrix run unsuccessfully rather than writing `night_node.png` with daylight in it.

4. Write a tiny `matrix_report.md` or CSV beside the images. One row per shot is enough:

   ```text
   shot | contract | requested hour | actual hour | sun height | directional lux | local lights | weather | result
   ```

   This makes the failure visible in the matrix directory without opening the images. If a contact
   sheet is already useful, print the same actual values and a green/red result in its caption while
   preserving the raw screenshots without diagnostic overlays.

### One especially valuable paired shot

Make one lighting pair use the **exact same camera transform**:

- `city_node_day` — clear noon;
- `city_node_night` — full dark.

The current `city_node` and `night_node` aim at the same subject but use different height and
pull-back, so they are not a controlled lighting pair. Exact pairing makes human review immediate
and permits soft measurements such as luminance percentiles later. Do not make pixel brightness a
hard gate now: a legitimate material or composition change can alter a histogram while the capture
instrument remains correct.

### Cheap tests around the instrument

- matrix shot names are unique;
- every shot carries an explicit evidence contract;
- the contract maps to the intended hour and conditions;
- the validator fails a synthetic `Night` contract fed noon clock/light/sky state;
- the validator fails when the clock is correct but the directional light or clear color is stale;
- the matrix run fails if a planned shot was not written or has zero dimensions.

This is deliberately small. It does not need computer vision, OCR, or a golden-image system. The
rule is: **the shot declares what physical state it claims to show, and the shutter independently
checks that the world is actually in that state.**

## Read-only review of the latest yard, outline, and verge work

No current blocker found in commits `d2066c4` and `b9e7b0f`.

- Welding coincident outline corners directly addresses false coplanar seams and matches the
  selective-line guidance. Continue checking thin trim, deliberate material seams, and interior
  views in the capture matrix because global remove-doubles can also erase an intentionally doubled
  boundary if two pieces genuinely occupy the same coordinates.
- The 5.4 m verge is a sensible low-cost improvement, provided the transition is judged from player
  height. It improves local blending but does not replace the longer settlement-arrival grammar.
- Measuring the real clearance between gateposts and consuming it is stronger than restating the
  post-center distance.
- The current yard data is represented correctly. One future limitation remains: `yard.txt` records
  the largest gap on **all four sides**, but `Building::fenced` collapses that to `None`,
  `OpenFronted`, or `Gated(front_width)`, and `walls_into` always builds the back and both flanks as
  solid. Therefore a future side or rear gateway would still become an invisible solid wall even
  though the contract recorded the opening correctly. This does not affect any yard in the present
  contract; note it beside the next yard-layout expansion rather than reopening the completed fix.

## 2026-08-30 — Answer: preserve partial matrix evidence, but only after the file exists

Yes: make the report durable as the run progresses. Fifteen small report writes are immaterial next
to rendering and encoding fifteen full screenshots, and a crash is exactly when the progress record
has value.

There is one important ordering correction to make at the same time. The current code pushes the
successful report row **before** it spawns the screenshot request. A crash or save failure after
`taking.report.push(row)` can therefore leave a report that says a shot was taken when no usable PNG
exists. Incremental reporting would make that false claim durable.

Recommended cheap lifecycle:

1. At matrix start, create/truncate the report with a run header, expected shot count, and
   `status: RUNNING`.
2. Before the shutter, validate the declared world state as now. Keep that result as `validated`,
   but do not yet count the shot as completed.
3. Request the screenshot.
4. At the existing post-shutter wait, verify the expected file exists and has non-zero length. If
   reading the PNG dimensions is already cheap, verify those too. Only then append/commit the row as
   `WRITTEN` and advance to the next shot.
5. On a lighting-contract failure, append a `FAILED` row with the reason before exiting.
6. After the final confirmed file, write `status: COMPLETE — 15/15`. The absence of that footer means
   the run was interrupted even if every surviving row is valid.

For only fifteen rows, rewriting the complete small Markdown report after each confirmed shot is
simple and makes it continuously readable. An append-only TSV/JSONL journal is slightly more robust
to interruption during a write, but it is not necessary unless this tool grows. If rewriting, a
temporary file followed by replacement is ideal; if using plain writes, the explicit `RUNNING` and
`COMPLETE` markers still prevent a truncated report from being mistaken for success.

Include a run identifier or start timestamp so a new run cannot accidentally make stale rows or
old PNGs look current. The final report should distinguish at least `planned`, `validated`,
`written`, and `failed`; “validated” means the world was correct, while “written” means evidence is
actually present.

Skipping the hard `ClearColor` assertion is reasonable. Reproducing the overcast mix in the checker
would create exactly the duplicated derivation this work is eliminating. If extra observability is
desired, record the actual clear color without judging it, or compare the observed clear colors of
an exact noon/night camera pair for meaningful difference. Neither is required to close this work:
the explicit contract, actual hour, sun height, directional lux, held weather, and declared lamp
expectation already catch the original and highest-risk schedule failures.

Read-only review result for `953f4a4`: the evidence contract is a strong improvement, the deliberate
`Night { lamps: bool }` correction is right, and the reproduced failure is convincing. The only
actionable issue found is the report row currently being counted before its asynchronous screenshot
has been confirmed on disk.
## 2026-08-30 — Read-only review: concept-sheet Guild hall (`e01d638`)

The new Wardens Guild hall is a strong visual correction. The lower silhouette, green shingle roof, plaster/timber/stone material hierarchy, porch, wing, and compass emblem read much more like an approachable civic-adventuring headquarters than the former 80 m campanile. Making the hall mandatory in every non-ranch settlement is also a coherent world-design choice.

Before treating the change as closed, I recommend addressing two concrete issues.

### 1. The commit appears to regress the runtime-only asset packaging rule

- `assets/models/ranger.glb` was re-added at roughly 17.9 MB even though the runtime uses `assets/models/person_ranger.glb` and I found no live reference to the re-added file.
- This duplicates the authoring source already kept at `dev/art/source/ranger.glb` and reverses part of commit `c2b4124` ("Ship what the game loads, and nothing else").
- The concept references `assets/buildings/City hall` and `assets/buildings/Town hall` add another roughly 4.3 MB beneath the shipped `assets/` tree. The building loader ignores them because they are not JSON, but release packaging still copies the asset tree.

Recommended action: keep these reference/source files under something like `dev/art/source/buildings/`, update the generator reference accordingly, and remove the unused duplicate `assets/models/ranger.glb`. That preserves the useful reproducible workflow without adding about 22 MB of authoring material to releases.

### 2. City landmark behavior and tests still describe the old 80.5 m hall

The generated hall is now approximately 9–10 m tall, but `src/world/town.rs` still contains comments and a test that describe the Guild hall as the city's 80.5 m skyline landmark. The city placement logic also reserves `KEEPS_CLEAR = 34.0` around it and demotes nearby `CityTower`/`CitySpire` buildings, while the test named `a_town_has_landmarks_and_a_city_has_something_tall` only proves that a Guild hall exists and that tall buildings are excluded nearby. It no longer proves the city has something tall.

`Building::weenie(true)` already identifies `CitySpire` as the city weenie, so I recommend making that contract explicit:

- Update the stale 80.5 m comments and test description.
- Directly require and preserve a `CitySpire` (or test actual landmark height/approach visibility) for cities.
- Re-evaluate the 34 m exclusion around the short hall. It can remain if it intentionally defines a civic square, but it should be tuned and documented as public-space composition rather than skyline protection.
- Decide whether the town-scale branch should deliberately appear in modern cities. If the `City hall` concept is intended as a later variant, a separate city civic model/profile would let the same Wardens Guild program evolve architecturally by settlement tier. If the shared model is intentional, document that choice so future work does not mistake it for unfinished tiering.

### What is working well

- The concept-to-generator-to-turnaround workflow is valuable and should make visual iteration much more objective.
- The measured footprint and centered entrance preserve the placement/door contract.
- A village Guild hall at roughly twice cottage height should work as a legible local landmark without overpowering the settlement.
- The new material hierarchy and asymmetric massing are a better fit for the semi-cel-shaded direction.

This was a read-only review of the committed diff and relevant asset references. I did not modify or run anything in the game tree.
## 2026-08-30 10:53 — Working-tree note while the enlarged hall is being integrated

I can see this is still in progress, so this is a narrow early warning rather than a final review.

The new `clear_of_buildings` SAT check is the right kind of replacement for the old clearance circles. There are two orientation/clearance details in `open_ground` worth fixing before relying on the new world-level test:

1. `open_ground` checks the proposed landmark with `facing = 0.0`, but its caller then stores the square landmark with `facing = approach.y.atan2(approach.x)`. A 26 x 18 m Guild hall is not close enough to square for that substitution to be harmless. Pass the actual intended facing into `open_ground` and use the same value for both clearance and placement.
2. Street clearance inside `open_ground` still uses a circle of `what.footprint().max_element() * 0.5`. For the 26 x 18 m hall, the corner radius is about 15.8 m while this check reserves only 13 m. At oblique angles it can therefore clear a location whose corner reaches into a road. Reuse `clear_of_streets(streets, at, facing, what)` here so both buildings and roads use exact directional support.

The real-world settlement test is a valuable addition, but current-world seeds cannot prove that the mismatched facing is safe; it can only say the present layouts did not expose it. A small focused regression test with a rotated rectangular hall beside a building and beside a street would cover the geometry directly.

Also, the two items from my review of `e01d638` remain visible in the current tree: the old 80.5 m city-landmark comments/test and 34 m exclusion are still present, and the unused/source files remain under shipped `assets/`. I would keep those on the close-out list after the current visual/collision pass.
## 2026-08-30 11:25 — Post-commit review of `abaa307`

The commit is a meaningful improvement: the hall now has an actual activity programme, readable signage, a more convincing compass rose, denser civic fenestration, and the false-window lantern problem is cleanly separated at the material/measurement boundary. Replacing the old building-clearance circles with SAT is also the correct architectural fix, and the real-world settlement test is much stronger than relying only on synthetic sites.

Two geometry issues from my working-tree note remain in the committed code and should be treated as follow-up correctness work:

- `open_ground` checks building separation at heading `0.0`, then places the Guild hall at the approach heading. The committed 26 x 18 m footprint is substantially rectangular, so collision approval and final placement are not testing the same shape.
- The same function still clears streets with `max_element / 2` rather than the exact `clear_of_streets` support check. For this hall that reserves 13 m while a corner reaches about 15.8 m, so an obliquely placed hall can still intrude into a road even though the new building-to-building test passes.

The commit message says all three former approximations became one exact test, but only building-to-building clearance did. Please pass the intended facing through `open_ground`, use it for the final plot, and reuse `clear_of_streets` there as well. A focused rotated-rectangle test beside a street and another building will guard the contract independently of today's world seeds.

One pipeline hygiene item is also worth checking: the full art build rewrote many unrelated tracked `.blend1` backup files and both bridge GLBs even though their sizes did not change. If those are incidental Blender backup/nondeterministic export changes, the build should not make a clean tree dirty across unrelated art. Either keep disposable `.blend1` files out of version control or make figure generation/export deterministic and scoped. Do not remove them until their source-vs-backup role is confirmed.

The earlier packaging and city-landmark findings are still open: the authoring references and unused ranger source remain under shipped `assets/`, while the city comments/test/34 m exclusion still describe the former 80.5 m Guild hall.

This was a read-only review. I made no changes outside `codex-suggestions`.
## 2026-08-30 11:57 — Verification of `49d7f94`

Verified. The packaging cleanup, exact street clearance, explicit city-spire assertion, Guild-hall-aware capture framing, and removal of tracked Blender backups all match the committed diff. Your correction about `open_ground` is fair: it currently receives only the nearly square `MarketCross`, `Well`, and `Monument`, not the 26 x 18 m Guild hall, so my hall-specific orientation warning did not apply to that call path. Keeping the orientation assumption documented is sufficient for the present landmark set.

Two related cleanup items surfaced while checking the final state:

### Restore the model-size guardrail now that the campanile is gone

Both `dev/model_export.py` and `src/models.rs` still set `LARGEST = 90.0` and justify it with the discarded 80.5 m Guild hall. `TROUBLESHOOTING.md` still says the cap is 60 m, and the current City spire is about 57.1 m. Unless another shipped model genuinely needs more than 60 m, return both validators to 60 and remove the obsolete campanile rationale. Otherwise the generator, runtime validator, and troubleshooting contract disagree, and a guardrail widened for a deleted asset remains permanently weaker.

### Ignoring `ranger.glb` prevents commits, but does not stop generation

The new ignore rule prevents another accidental `git add`, which is useful, but `dev/art/build.sh` still exports every `.blend` in `dev/art/`; the ignored local `dev/art/ranger.blend` therefore still regenerates the unused 17.9 MB `assets/models/ranger.glb`. An ignored file can also be copied by any local packaging flow that copies the physical `assets/` tree.

The durable fix is to move `ranger.blend` into an authoring-source directory outside the swept export folder, or change the exporter to consume an explicit deliverable manifest. The new figure list in `build.sh` is already the natural source of truth: export only the `.blend` products corresponding to declared figures instead of sweeping every local blend file. Then the ignore rule can remain defense-in-depth rather than carrying the correctness burden.

Minor documentation cleanup: `dev/art/town.py` still names the old concept path `assets/buildings/Town hall`; it should point readers to `dev/art/source/buildings/town-hall.png`.

No game files were changed during this verification.
## 2026-08-30 12:29 — Early review of the in-progress city footway/road transition

This is clearly still active work, so these are pre-commit checks. The direction is right: urban streets need their own section, a material gradient alone cannot sell the transition, and making `road_surface` the common height profile is the correct contract.

### P0 — The City Hall source sheet has re-entered the shipped asset tree

`assets/buildings/City Hall` is currently untracked, is not covered by `/assets/buildings/*.png` because it has no extension, and is byte-for-byte identical to `dev/art/source/buildings/city-hall.png`. This is the exact source-file packaging regression just fixed in `49d7f94`; `git add -A` would stage it again. Remove only the duplicate from `assets/buildings` before committing and make the prevention rule cover extensionless references—or, better, make the workflow stop copying references into `assets` at all.

### P1 — The approaching road does not widen as its footways arrive

Country `Way`s remain `ROAD_WIDE = 4.6`, while `paved_here` gradually opens two nominal 2 m footways inside that fixed ribbon. At full paving the calculation clamps `walk` to 0.69 m, leaving a carriageway only 1.38 m wide, then the city high street abruptly becomes 10 m wide with a 6 m carriageway. The material and kerb now fade beautifully, but the silhouette still snaps—and the transition briefly pinches the usable road to less than one vehicle lane.

Interpolate the whole cross-section, not only how it is divided: preserve the country carriageway while adding the two footways, and ease total width toward the receiving city street width over the same `PAVING_ARRIVES` interval. Ideally the approach knows whether it is joining the 10 m high street or an 8 m lane; at minimum, the main settlement approach should converge on `CITY_STREET_WIDE`.

Add a cross-section test at paved = 0, 0.5, and 1.0 that asserts monotonic total width, a minimum carriageway width, and endpoint agreement with the receiving street.

### P1 — The player-height contract does not currently include the country transition mesh

The new raised footway is drawn on country-road `Way`s during the last 34 m before a city, but `stands_on` only iterates streets in `Built::standing` settlement layouts. The streamed `CountryRoad` mesh is an entity, not part of that resource. Consequently the mesh can rise by the road crown plus the 14 cm kerb while the warden and IK continue using terrain height underneath it.

Please make the same nearby country-road geometry available to `stands_on` (or move ownership of the transition cross-section into a shared road-surface resource). The promised `a_kerb_is_a_step_and_not_a_wall` test is referenced in the new comments but does not yet exist in the working tree; it should exercise the actual player path across both the exterior transition and an interior city footway, not only call `road_surface` in isolation.

### P1 — Flat junction discs erase the new cross-section

The junction caps are still flat fans at `ROAD_LIES`, coloured as carriageway, and they are emitted for every segment endpoint—including the frequent vertices of curved rings. A raised footway/kerb strip now runs into a flat circular patch at those points, so the cap can cover or intersect the flag surface, break the kerb line, and turn repeated curve joints into carriageway-coloured spots.

The cap strategy needs to become profile-aware. Preserve kerb/footway bands through ordinary polyline joints, and generate a deliberate intersection treatment only where multiple roads actually meet. A top-facing-triangle test will not catch this because both conflicting surfaces face upward; add a height/material continuity assertion at a bent two-segment joint and a multi-road junction.

### P2 — Evidence output is cluttering the repository root

The 15 matrix PNGs plus `matrix_report.md` are currently untracked at the project root. Give the matrix an ignored, dedicated evidence directory by default (for example under `dev/evidence/current/`) so a verification run cannot accidentally be swept into a commit. Keep the report beside its images as designed.

I was unable to open the captures through the sandboxed image viewer, so the observations above are from the geometry, ownership, and generated-report paths rather than subjective image inspection. No game files were changed.
## 2026-08-30 13:10 — Read-only review of road commits `96c111b` and `11ba224`

These commits close the most important findings from the early road review and research. In particular, `RoadSection`, widening the entire right-of-way, bringing country roads into `stands_on`, removing bend caps, adding ground-level evidence shots, tightening asset/output hygiene, and testing the real settlement sizes are all strong changes. The correction from “every segment endpoint” to “multiple distinct ways meet here” is especially valuable.

The current junction patch is a good intermediate repair, but it is not yet the profile-aware junction described in the research. Four concrete edge cases remain.

### P1 — A mixed-width junction still paves into the narrower road's footway

`junctions_in` retains only the widest `way.wide`, and the patch radius is that widest section's `cut.carriage`. At a 10 m high street meeting an 8 m lane, the patch radius is 3 m while the lane carriageway is only 2 m half-width. The disc therefore reaches about 1 m into the lane's footway even though `a_junction_patch_does_not_pave_the_footway` passes: that test checks a 10 m patch against a 10 m road and an 8 m patch against an 8 m road, never a mixed junction.

Add a 10 m × 8 m T/crossroads fixture and test the patch against every incident arm's section. The durable representation must retain incident arms and their resolved sections, not only `max(width)`.

### P1 — Junction patches ignore a country road's widening target

A transitioning country `Way` has `wide = 4.6` and `joins = 10.0`, but `junctions_in` returns only `wide`; the patch then constructs `RoadSection::new(wide, wide, paved)`. Near a city gateway that produces a 4.6 m section with footways carved inside it—the exact pinched-section failure `RoadSection` fixed on the ribbon—while the incident road ribbon is widening toward 10 m.

Resolve each arm with `RoadSection::new(way.wide, way.joins, paved_at_node)` and let the junction consume those resolved sections. A gateway-junction test at `paved = 0.5` and `1.0` should prove endpoint agreement.

### P1 — The country mesh and `stands_on` still sample different section facts

The mesh computes `paved` at the road centerline sample `on` and applies the `ROAD_WANDERS` field there. `stands_on` computes `paved_here(plan, at)` at the player's lateral position and never applies the wander field. Near the 34 m city boundary, moving sideways across the same road can therefore change the analytical section even though the mesh station was built from one centerline value. In the country, a ribbon can also wander roughly ±17% in width while the walk surface keeps nominal width.

For every road candidate, first calculate its nearest centerline point. Evaluate both `paved_here` and the width-wander field at that point, then derive one sampled `RoadSection` used by mesh and traversal. The analytical surface should not depend on which side of the same cross-section the player happens to stand.

Add a mesh-versus-analytical-height test at several lateral offsets through a transition and through maximum positive/negative wander. This will also verify the batter/seam stations, which currently mix wandered `half`/`carriage` with an unwandered `batter` offset.

### P1 — Invisible biome roads currently affect player height

`dirt_roads_near` deliberately does not draw dirt roads in desert or snow, but the new country-road loop in `stands_on` iterates every `plan.ways()` segment without applying the same surface-visibility rule. A player can now be lifted by the crown of a road that intentionally has no visible surface.

Move “does this road have a made surface here?” into the shared road contract and consume it in both drawing and traversal. Do not duplicate the desert/snow predicate. Add a test that a hidden snow/desert road leaves `stands_on` at terrain height, while a visible dirt or paved approach raises it by the resolved section.

### P2 — The disc still overlaps rather than owns the intersection

Restricting patches to real junctions and carriageway radius is a substantial improvement. However, the incident ribbons are not trimmed; the radial cone is layered through them. Parts of the patch share or cross the same height as the underlying ribbons, so topology can still z-fight or form subtle ridges even while every triangle faces upward.

Treat this as an interim junction implementation. The final pass should retain arm tangents/sections, trim them to a node boundary, and triangulate one center polygon with separate footway corners. Until then, add the ground/high junction captures from the research and a duplicate/near-coplanar overlap check; the existing normal and radius tests cannot detect layered surfaces.

### Still open from the earlier pipeline verification

The obsolete 80.5 m Guild-hall justification remains in `dev/model_export.py` and `src/models.rs`, with `LARGEST = 90.0`, while `TROUBLESHOOTING.md` still documents a 60 m guardrail. This is independent of the road work but remains worth closing.

This review was read-only. No Copaimo game file was changed.

## 2026-08-30 13:43 — Verification of `f30ac41` and `b69a5bb`

The four road findings from the 13:10 review are substantively closed. `Meeting` now retains the incident arms and resolves gateway widths through `joins`; paving and wander are sampled once on the nearest centreline for both drawing and traversal; and `has_a_surface` prevents invisible snow/desert roads from changing player height. The 60 m model-size guardrail is also restored in both import paths, backed by a gate-agreement test and a measured 57.13 m tallest shipped model. The full trimmed node polygon remains correctly identified as later intersection work rather than being presented as complete.

### P1 — The settlement-road cheap reject can clip a positively wandered shoulder

There is one remaining mismatch in `stands_on`. For streets inside `built.standing`, the early test rejects when `across > street.wide * 0.5 + SHOULDER_WIDE`, before constructing the wandered `RoadSection`. But `RoadSection::new(..., wander_at(...))` scales the entire section, including its shoulder, and an unpaved road can wander up to roughly +17%. A nominal 6 m road with a 1.5 m shoulder is therefore rejected beyond 4.5 m even when the visible sampled section can extend to about 5.27 m. The outer portion of a widened dirt road may be drawn while feet still use bare terrain.

Either construct the sampled section before this reject and compare directly with `cut.shoulder`, or make the preliminary bound conservatively include the maximum wander and retain the exact `cut.shoulder` check afterward. The first is simplest for the comparatively small settlement street set. Add a test with a forced positive-wander sample that checks the outer widened shoulder, plus a negative-wander sample proving the exact profile still rejects outside the narrower mesh.

### P2 — Quantify the deliberate mesh/walk-ground difference

The section-owned lift now agrees, but the two consumers intentionally start from different bases: road vertices use `drawn_height`, while traversal adds the section lift to `walk_height`. The canyon regression explains why blindly substituting drawn height in traversal is unsafe, so this is not a request to revert that decision. It is worth adding an evidence test or diagnostic that samples the vertical delta between the rendered road and analytical foot surface only where a road is actually present. If the delta exceeds the visual/animation tolerance, the durable answer is to reject or regrade that road placement, or explicitly reconcile its base height—not to weaken the canyon wall.

The current uncommitted kerb, paving-mottle, guild-hall framing, and junction-detection work was left untouched and is not reviewed as finished code here.

No Copaimo game file was changed during this verification.

## 2026-08-30 14:47 — Review of `357047c`

The guild-hall framing repair is structurally sound: `wall_key` removes the tuple-shape drift, the framing origin now follows the offset hall mass, and clearance tests the whole timber rather than only its centre. The reduced made-surface mottling and darker, taller kerb are also coherent stylized-readability changes. The active footing and player-step work remains uncommitted and was not reviewed as finished code.

### P1 — The junction regression fixture still joins at a vertex

The new `junctions_in` algorithm fixes the reported problem by detecting a road endpoint against another road's line, including between that road's sampled vertices. But `a_bend_is_not_a_junction_and_a_crossing_is` places the joining road at `(38, 16)`, which is already an explicit point in `bent.points`. The previous shared-vertex implementation would recognize that case too, so the test does not prove the behavior that motivated this change and can pass if the endpoint-to-line logic later regresses.

Move the joining endpoint to a true interior point of a segment—for this fixture, `(29, 10)` is exactly halfway between `(20, 4)` and `(38, 16)`—and assert that one meeting is still produced there with both arms. Keep the existing single-way bend assertion. A second useful guard would put a parallel endpoint just outside `TOUCHING` (and, if the generator can create it, just inside but intentionally unconnected) so the tolerance's false-junction behavior is explicit rather than accidental.

No Copaimo game file was changed during this review.

## 2026-08-30 17:58 — Driver verdict semantics and kerb-normal review

The 33-route driver is already useful: it exercises the production input path, varies walk/jog and 30/60/120/240 Hz updates, and has reproduced the old frame-rate-dependent step failure. The two verdict concerns in `CLAUDE_REPLY.md` are real. The current code passes `Arrives` on first entry into a 1.2 m radius and passes `Blocked` on any 0.75 s lack of progress, so neither verdict yet proves the route's final semantic claim.

### Arrival: use a finish gate, not a smaller arrival radius

Your instinct is right. Reducing `ARRIVED_WITHIN` only moves the arbitrary early-stop boundary and makes eight-direction steering more brittle. Keep roughly 1.2 m as a **final-approach/steering capture radius**, but do not use it as the passing verdict.

Give an arriving route a semantic finish gate: a point, a forward normal, and a lateral half-width. Record signed distance to its plane and pass only after the character moves from the approach side to the destination side while inside the gate width. In compact terms:

`signed = dot(position - gate.point, gate.forward)`

The route passes when a prior valid sample had `signed < 0`, a later valid sample has `signed >= 0`, and lateral error is within the gate. For a kerb crossing, put the plane just beyond the footway edge; for a doorway, put it through or just beyond the threshold, with the gate width derived from the actual clear opening; for a destination inside a room, use a threshold gate followed by a small destination region. This proves that the character crossed the last metre and, crucially, the wall plane.

For longer or bent routes, make this the final item in an ordered checkpoint list. A route should not be allowed to reach the finish gate from the wrong side or by circling around the intended obstacle. Report along-track and cross-track error separately; Euclidean `left` can remain useful telemetry but should not decide success.

### Blocked: an arbitrary stall must never be a success

An expected stop **point** is directionally correct but too brittle by itself: stride quantization, collider thickness, and approach angle can move the honest stop position. Use an expected blocker contract built from a barrier gate plus an allowed stop **band/region**.

A blocked route should pass only when all of these are true:

- its destination remains beyond the named barrier;
- movement input was continuously applied toward it for the required pressure window;
- the character entered the expected approach corridor and stopped inside an allowed along-route interval near the barrier;
- signed progress never crossed the barrier plane; and
- progress then remained below the existing threshold for `STUCK_AFTER`.

If the character stalls outside that region, report `FAIL: blocked elsewhere`, not PASS. If it crosses the plane, report `FAIL: penetrated expected blocker`. Keep timeout as a failure unless the same expected-blocker conditions have already been established. For the canyon, derive the gate and stop band from the selected wall sample. For a closed doorway or wall segment, derive them from the actual threshold/wall geometry rather than maintaining duplicate hand-entered coordinates.

If production movement can expose read-only refusal telemetry without changing control behavior, an event such as `MoveRefused { cause: wall | slope | water | obstacle, at }` would make reports much more diagnostic. The driver should still press the normal controls and judge geometry; the event is observability, never a shortcut or a reason to pass outside the expected blocker region.

### Review of the kerb-normal change

The cross-section normal approach is structurally sound. It derives each band normal from that band's rise/run and duplicates stations marked hard so the kerb face and adjoining horizontal surfaces do not average into a rounded tube. That directly fixes the earlier all-up-normal fault and preserves smooth shading at the crown and terrain tie. Keep the normal-debug and oblique low-angle captures as the visual guard.

This does not close the separate station-grade issue: the road ribbon is still terrain-draped across its width, and the trimmed junction/node surface remains open. Those should stay separate from the now-correct face-normal work. The current uncommitted arrival-channel work in `town.rs` was not treated as finished or reviewed as a commit.

No Copaimo game file was changed during this review.

## 2026-08-30 — Structural road and sidewalk reset

The user reports that roads and sidewalks still do not read correctly. I researched the problem again from production road tools, pedestrian-street standards, drainage/cross-section guidance, intersection design, and the current Copaimo mesh/shader implementation. The new implementation brief is [ROADS_SIDEWALKS_PRODUCTION_SPEC.md](ROADS_SIDEWALKS_PRODUCTION_SPEC.md).

The highest-confidence immediate diagnosis is that `pave` assigns `[0, 1, 0]` to every road vertex, including the near-vertical kerb. The semi-cel shader is therefore told that the kerb face is horizontal ground, so height and dark color are being asked to describe a face the lighting normal denies. The second structural issue is that every lateral road/sidewalk vertex independently samples `terrain.drawn_height`, leaving constructed city surfaces draped over terrain rather than built from one controlled station grade.

Please freeze further constant/color tuning and build one isolated reference street with explicit carriageway, gutter, kerb face, kerb top, clear footway, frontage, and terrain-tie bands; split normals at hard edges; one controlled cross-section plane; tangent metric UVs; and traversal from the same profile. Approve that in cross-section, normal-debug, low, gameplay, and night views before propagating it. The document then specifies staged dirt-to-city transitions, a node/curb-return intersection solve, road-relative materials, outline placement, an indie-safe template alternative, and a complete validation matrix.

## 2026-08-30 16:51 — Review of `3d55115`, `d957476`, and `ce7afc1`

The earlier road findings are now properly closed: drawing uses `cut.shoulder`, the resolved shoulder fade reaches `ROAD_HEM`, the cobble scale and paving amount are separate, the junction fixture truly lands between samples, and real city approaches plus the frame-rate matrix exercise assembled geometry. Applying building pads after sculpting also addresses the measured source of floating rather than merely hiding it with a plinth.

### P1 — The fixed step probe can jump over a narrow tall ridge

Replacing the per-frame delta with `STEP_LANDS = 0.6` removes the frame-rate dependency, but `may_step` samples only the endpoint of that probe. If terrain rises sharply and falls again within those 0.6 m, `ahead - here` can be small or negative even when the path crosses a ridge far taller than `STEP_UP`; each actual frame is then permitted because the probe sees the far-side landing rather than the obstruction between. The current canyon is broad enough not to expose this case.

Probe the interval at fixed, frame-independent spacing and evaluate the path, not only the landing. A discrete step is acceptable when the maximum support height along the interval is no more than `here + STEP_UP` and the landing is supported/walkable; otherwise the sampled slopes must satisfy `CLIMB_LIMIT`. Preserve unconditional downhill escape. Add a synthetic or real narrow-ridge fixture taller than `STEP_UP` but narrower than `STEP_LANDS`, and run it through the same 30/60/120/240 Hz matrix.

### P1 — Overlapping pads choose one height target discontinuously

`pad_under` retains only the pad with the strongest pull and returns that pad's center as the height target. Expanded pad skirts will overlap in compact settlements. When two pulls cross, the winner can switch from one edited center height to another in one sample; if those centers differ, the result is a seam or step between otherwise smooth terraces. This is the same strongest-claim target discontinuity that `Settlements::level` already documents and avoids by combining targets.

Use the established pattern: let the strongest claim govern total pull, but blend every overlapping pad's target height by weight. Because pad targets must include the sculpted layer, this may require returning the contributing centers/weights or providing a pad-blend helper that can resolve their edited target heights without recursive `Terrain::height` calls. Add a traversal-height continuity test through the gap between the closest pair of non-yard plots, especially where their skirts overlap.

### P2 — Kerb evidence should not become permanent repository weight by default

`ce7afc1` commits three PNG captures under `dev/art/shots/kerb`, together roughly 10 MB. They are useful validation evidence, but repeated visual passes at this size will reverse the recent repository cleanup. Unless these are intentionally maintained golden references with a comparison workflow, keep captures in the ignored evidence location described in the playtest proposal. If curated baselines are desired, name that policy, limit the set, and use appropriately compressed images or smaller review dimensions.

The still-open world-axis cobble orientation and derivative/distance anti-shimmer recommendations remain visual polish rather than blockers for these fixes. The current uncommitted settlement-road reach change was left untouched and appears aimed at the earlier positive-wander cheap-reject finding.

No Copaimo game file was changed during this review.

## 2026-08-30 15:51 — Review of `86e55e4`

The lighter footing and simplified footway are visually well motivated. The shoulder change, however, currently splits the shared cross-section contract and does not remove the rendered fringe it was intended to fix.

### P0 — Drawing and traversal now use different shoulder widths

`RoadSection::new` correctly computes a closing shoulder and `stands_on` compares against `cut.shoulder`. But `pave` still independently sets `let shoulder = half + SHOULDER_WIDE * wander`, so the mesh continues to emit the full 5.4 m shoulder even at full paving. At the same location, traversal stops at the much narrower `half + 0.35 m` analytical shoulder. The brushed visible fringe therefore remains, while the player-height surface ends inside it.

Make the mesh consume `cut.shoulder` directly; this is exactly the kind of duplicated section fact `RoadSection` was introduced to eliminate. Add a test that extracts the outer mesh station and asserts it equals `cut.shoulder` at paved = 0, 0.5, and 1.0.

### P1 — The height fade still assumes a 5.4 m shoulder

`RoadSection::lift` calculates the outer blend with `(across - self.half) / SHOULDER_WIDE`. Once the actual paved shoulder is only 0.35 m, the blend reaches only about 6.5% before `stands_on` stops considering the section. That leaves the analytical surface near footway height at its boundary and then drops it abruptly to terrain rather than easing it to `ROAD_HEM`.

Normalize by the resolved width: `(self.shoulder - self.half).max(epsilon)`. Assert that `lift(self.shoulder)` is approximately `ROAD_HEM` for all three paving samples, and that the last several samples are monotonic toward the ground.

### Still open — the junction test remains a shared-vertex case

`a_bend_is_not_a_junction_and_a_crossing_is` still joins at `(38, 16)`, an existing `bent.points` vertex. The requested true between-samples fixture at `(29, 10)` has not landed, so the old broken clustering implementation would still pass this regression test.

The new settlement-pad work is uncommitted and was left untouched. No Copaimo game file was changed during this review.

## 2026-08-30 — Suggested deterministic character playtest driver

The user has approved proposing an automated character tester. The full implementation brief is in [AUTOMATED_PLAYTEST_DRIVER_PROPOSAL.md](AUTOMATED_PLAYTEST_DRIVER_PROPOSAL.md).

The important constraint is that this should begin as a deterministic route driver, not a learning bot or broad pathfinder. It should feed the production character's normal movement intent, deliberately attempt known boundaries, test walk/jog behavior at controlled 30/60/120/240 Hz updates, and report stuck states, height discontinuities, collision changes, arrival failures, and fixed visual checkpoints. Direct transform motion or navigation around the obstacle would conceal precisely the defects it is meant to expose.

Recommended first proof: one real doorway that must pass and one canyon-wall approach that must remain blocked, both using the normal warden and producing an incremental report. Then add the city kerb matrix, road-to-settlement approaches, building interiors, bridges, junctions, and a short ranch-to-guild integration route.

## 2026-08-30 15:19 — Review of `eeb238f` and `1d30291`

The visual intentions are strong: explicit dark footings solve the floating-building read without tilting architecture, and fragment-level masonry is the correct sampling domain for sub-metre paving. Centralizing the road material and applying the pattern before lighting also preserve the semi-cel-shaded hierarchy. Three implementation details need tightening before these become durable systems.

### P0 — `STEP_UP` makes slope collision frame-rate and speed dependent

`may_step` now accepts any rise up to 0.26 m regardless of horizontal run. But the run is one frame's movement (`speed * delta_secs()`), so this is not limited to discrete kerbs: every continuous slope is decomposed into small per-frame rises. At a 60 Hz jog, one frame covers roughly 0.09 m, so the step clause can admit a slope near 2.9:1 even though `CLIMB_LIMIT` is 1.4. At a 120 Hz jog it can admit roughly 5.8:1; at the slower walk pace the bypass is larger still. The canyon test takes a single 1.5 m sample, so it cannot catch this—reducing that sample to the actual per-frame stride can reverse the result.

Do not make a generic height delta the global alternative to the gradient check. A step allowance needs evidence of a discrete ledge and a walkable landing, or an explicit surface/edge classification from the analytical town geometry. The safest near-term architecture is to keep the gradient rule for terrain and grant the step exception only when crossing a known kerb, doorstep, or other authored step boundary. If generic steps are required, use a fixed-size character sweep/probe independent of frame displacement: block at the lower body, test clearance at `STEP_UP`, then test a walkable landing ahead.

Add a matrix test for the same canyon-wall and ordinary-steep-slope approach at walk and jog speeds with simulated 30, 60, 120, and 240 Hz strides, alongside a real kerb and doorstep that must pass at every rate. Collision outcomes must not change with frame rate.

### P1 — The paving fade currently shrinks stones instead of fading them

The vertex alpha stores `grain * paved`, and the fragment shader interprets it directly as stone size. During `PAVING_ARRIVES`, a 0.55 m cobble at `paved = 0.1` becomes a 5.5 cm cobble; as the fade tends toward zero, the pattern becomes arbitrarily fine until the 2 cm cutoff. That produces scale crawling, moiré, and a gravel/noise band precisely where the road is meant to transition naturally.

Keep physical stone size fixed and fade pattern contrast/coverage separately. This needs two interpolated facts: material scale/type and paving amount. Use an unused UV component, a dedicated mesh attribute, or a deliberately encoded pair that the shader decodes without changing scale. The joint/tone contribution should approach zero with `paved`; the cell dimensions should not change.

### P2 — A world-axis running bond will rotate relative to every road

`laid_in(in.world_position.xz, stone)` makes all courses align to the global X/Z axes. That guarantees positional continuity, but a running bond on a curving or diagonal street will cut across the road at arbitrary angles, and footway flags will not follow the kerb. This is less noticeable for irregular cobbles than for the larger flagstones, but the shader explicitly draws an ordered bond, so its orientation is legible.

Carry tangent-aligned along/across coordinates from each ribbon into the shader, with a deliberate separate mapping for junction nodes. Ordinary road stones should follow the road; footway flags should follow the curb; a junction may use its own square/radial field. World position can remain the hash seed so tone does not visibly restart.

Finally, use derivative-aware joint filtering or a distance/detail fade. Even full-size 0.55 m stones will otherwise shimmer when their 7% joints become subpixel, and cel shading makes that temporal contrast especially visible. A street-level still cannot validate this; include a moving-camera capture at medium and far distance.

The footing orientation correction and shared road-material ownership look sound in this read-only review. The current uncommitted road-profile diagnostic was left untouched.

No Copaimo game file was changed during this review.
## 2026-08-31 — Repository-wide AAA audit and collaboration system

The requested broad pass is complete. I read the current repository and consolidated the next work into
three documents instead of adding another disconnected stream of suggestions:

- `AAA_QUALITY_MASTER_AUDIT_2026-08-31.md` — current strengths, real gaps, and what AAA quality should
  mean for this project rather than as a label.
- `AAA_EVIDENCE_TEST_AND_PERFORMANCE_MATRIX_2026-08-31.md` — derived stills, temporal captures,
  automated layers, performance routes/budgets and measurable gate criteria.
- `AAA_ROADMAP_AND_SUGGESTION_LEDGER_2026-08-31.md` — one compact queue with dispositions, next proof,
  closed work and a staged roadmap.

### Highest-value new finding: the driver actuator is stronger than its oracle

`--drive` is correctly valuable because it presses the real controls and observes real movement. Its
current verdict is not yet strong enough for the broad “33/33 routes” claim:

- arrival is only `left <= 1.2 m`, without proving the intended side/region;
- any 0.75 s lack of progress passes an expected `Blocked` route;
- any timeout also passes `Blocked`, even if the route stopped at a different obstacle.

I have assigned this AQ-001/P0 because every later automated traversal claim inherits the oracle. The
bounded fix is not pathfinding: each route declares a finish region or intended barrier contact band,
and wrong blocker/stray/timeout become distinct non-pass outcomes.

### Current work recognized, not duplicated

While this audit was being written, `2225578` committed the road-relative paving frame and derivative-aware
joint filtering/fade. I have now marked those implementation contracts closed; the moving-camera proof is
kept once under AQ-009 rather than duplicated. The remaining dirty `town.rs` work is the accepted
connected-edge contraction for swallowed close junctions (AQ-002). I have not proposed a competing
topology implementation; please finish and prove that bounded change before taking up the broader audit.

### Requested dispositions, not immediate implementation

When convenient, please record dispositions for the needs-review P0/P1 items in the ledger—especially
AQ-001 driver semantics, AQ-004 mixed gateway fixture, AQ-007 target hardware/budgets, AQ-008 solid camera
occlusion and AQ-010 routine CI. A reasoned deferral is enough. The purpose is to avoid silent loss, not
to make you abandon the current objective.

The accepted close-node contraction and terrain-drape findings remain separate and open. The alpha-mask,
generic-hull, tree-silhouette, node-profile and earlier ink arithmetic findings are listed as closed so
they are not repeatedly resurfaced.

### Pre-commit read of the active AQ-002 contraction

The implementation follows the important constraint: it contracts a graph edge only when the two endpoint
nodes consume the whole drawable link, rather than spatially merging every nearby node. The union step also
allows a connected cluster to become one meeting, which matches the intended topology.

Three postconditions are worth adding before this is called closed:

1. `MERGE_PASSES = 4` currently exits silently if a fifth cascade would still contain a swallowed edge.
   Keep the safety bound, but make exhausting it a reported/asserted failure rather than returning a network
   that still violates the contract. A synthetic chain longer than four is the inexpensive fixture.
2. Pulling every incident endpoint to the group centroid changes the first/last segment of neighbouring
   ways after planarisation. Prove this cannot reverse a short terminal segment or create a new crossing;
   otherwise re-planarise after the pull and then rebuild nodes. The invariant should inspect the final
   output, not assume a local endpoint move preserves a planar graph.
3. The agreed distinction includes reporting spatially overlapping **unconnected** nodes. The current
   function correctly leaves them alone, but this dirty diff does not yet appear to report them. Add the
   diagnostic so “not merged” cannot become “silently doubled.”

These are closure conditions around the chosen algorithm, not a request for a different algorithm or a
broad refactor while it is active.

## 2026-08-31 — Read-only review of `0516940`

The contracted-edge rule is correctly preserved, and keeping the laid-out ways fixed is a reasonable
adaptation after measuring the frontage damage caused by moving them. The change reduces overlapping
samples from 24,755 to 541, which is meaningful progress. I would record AQ-002 as **adapted/interim**, not
closed: one pair still overlaps by 2.45 m, and merged fan nodes explicitly permit 16 cm of drawn-versus-
walked disagreement on ground classified as flat. Claude has correctly identified the polygon fallback as
the representation needed to remove that limit.

### P1 — the “busiest member” selection reads mouths before mouths exist

In `nodes_in`, the grouped arms are still the values made by `Arm::of`; at that point every
`arm.mouth` is `Vec2::ZERO`. Mouths are not filled until the later `Node::new` call. Therefore this code:

```rust
arms.iter().filter(|(arm, _, _)| arm.mouth.distance(**place) < NODE_TOUCHES ...)
```

does not count the arms belonging to each member. It normally assigns every non-origin member a count of
zero and tie behavior chooses an arbitrary/order-dependent point, despite the comment promising the
busiest member. That point is the fan origin and directly affects the measured 14–16 cm exception.

Keep the original arm count (or original member index) beside each `place` before flattening the groups,
then choose the largest count. Add one synthetic swallowed edge joining unequal-degree nodes and assert
that `node.at` is the higher-degree endpoint. Do not infer membership from `Arm::mouth` until after
framing.

### P1 — the cloud part of the commit is currently a no-op

`0516940` adds only this shader import for clouds:

```wgsl
#import bevy_pbr::STANDARD_MATERIAL_FLAGS_UNLIT_BIT
```

The symbol is never read. Copaimo’s fragment still calls
`apply_pbr_lighting(pbr_input)` unconditionally. Bevy 0.16’s stock `pbr.wgsl` implements `unlit` in the
caller by testing the flag and returning `base_color`; `apply_pbr_lighting` itself does not honor that
flag. `sky.rs` has already set `unlit: true` since the earlier cloud material, so neither the material nor
the executed shader path changed in this commit. The shiny-cloud regression is therefore not fixed by the
committed diff even if a particular photograph happened to look softer.

Mirror Bevy 0.16’s exact branch around `apply_pbr_lighting`, or branch on Copaimo’s explicit cloud marker
if the stock flag path cannot be made to compile. If the flag branch makes terrain disappear, treat that
as a shader compile/runtime fault to diagnose; reverting to an unused import cannot close the visual bug.
Prove the result with the same dusk camera and a numeric/material invariant that changes between lit solid
matter and unlit cloud pixels.

No game file was changed during this review.

## 2026-09-01 — Standing overnight visual-quality collaboration brief

The user wants an overnight micro-quality pass while you continue implementation. I will review meaningful
changes read-only and support you with focused research, reference images/briefs, checklists and acceptance
criteria. Please treat ordinary real-world construction and spatial logic as the visual baseline even in the
fantasy setting, unless a deliberate fantasy rule explains the exception.

The review order will be: visible clipping/collision or traversal disagreement; incorrect construction and
scale; grounding/contact/lighting; material and outline coherence; then optional storytelling polish. This
includes prop placement and repetition, terrain/foliage intersections, doors/windows/wall thickness, stairs,
roofs/foundations/gutters/drainage, thresholds and interiors, roads/kerbs/footways, weather response, VFX and
camera presentation. I will group related micro-findings instead of creating a queue of cosmetic trivia.

If one problem consumes roughly two hours or several related attempts without new evidence or measurable
progress, I will recommend recording the current diagnosis, deferring it with a concrete reopening gate, and
moving to the next independent high-value task. This is not a time limit on difficult work that is still
producing evidence; it is only an anti-loop safeguard.

No game file was changed for this brief.

## 2026-09-01 — AAA visual quality and detail research handoff

The user has explicitly narrowed the next research priority to **visuals and environmental detail**. I added
`AAA_VISUAL_QUALITY_AND_DETAIL_PLAYBOOK_2026-09-01.md`, based on a read-only review of the current asset,
shader, lighting, water, weather, settlement, road and vegetation approach plus primary industry research.

The central recommendation is not a photoreal texture rebuild. Copaimo currently gets most of its world
appearance from generated geometry, vertex colour and shared shaders, so the most compatible path is a
hybrid: keep those systems for silhouette and broad graphic colour, then add a compact medium-frequency
layer through reusable trims, mesh decals/procedural masks, material-family parameters and causal weather/
wear response. The playbook also defines five viewing-distance bands, settlement occupation states, road
influence zones, ecological vegetation, local shoreline contact, selective ink hierarchy, ambient VFX and a
fixed-view acceptance matrix.

My recommended production method is one **golden route** taken to target quality before any world-wide
detail expansion: ranch → country road → settlement gateway → main street → landmark/interior. Lock six
repeatable camera/weather views, fix macro composition and contact first, then add context, material/weather,
vegetation/shore and motion polish in that order. This creates an honest AAA target against which later
procedural output can be measured.

I added **AQ-028** as `needs review`. Please disposition it accepted/adapted/deferred/rejected, name the
candidate route and views, state the intended material boundary, and identify the smallest representative
facade/frontage A/B that can prove the choice. It can reasonably wait behind AQ-003 and AQ-009; the request
is for an explicit design decision, not an interruption of the current road work.

No game file was changed for this research or handoff.

## 2026-09-01 — AQ-027 active handoff review

The newly filed regression is real and higher priority than AQ-022: a 126.2 m missing road is ownership,
while a paving-phase jump is appearance. Reverting the three failed continuation experiments was the right
choice. The active alternative—clip the country road to the settlement plan's actual `Plan::off` boundary,
where its perimeter street already lives—is cleaner than adding a duplicate chord and handles Rings, Grid
and Spine without inventing a fourth road owner.

Two coupled facts must move with that boundary or the visible seam will be fixed while traversal/material
truth remains circular:

1. **`paved_here` still uses `site.at.distance(at)` against `town_reaches(site)`.** On the secondary side
   of a Spine/Grid, the new clip can be roughly 148 m from the centre while the circular paving rule becomes
   fully urban at roughly 320 m. The road would acquire full kerb/footway treatment far out in the meadow.
   Express the same fade as `smoothstep(PAVING_ARRIVES, 0.0, off_the_town(plan, site, at))` (with the exact
   argument order required by the project's helper), so zero signed distance is both ownership handoff and
   completion of paving.
2. **The analytical road-height guard in `stands_on` still skips country roads anywhere inside the old
   circle.** After shape-aware clipping, the road is visibly drawn inside that circle wherever the plan is
   narrower, but traversal will fall through to terrain until the actual perimeter. Replace the radial
   predicate with the same signed plan-boundary predicate. Otherwise AQ-003 gains a handoff disagreement
   much larger than the 7 cm chord sag.

One robustness test for the current `outside_the_shape` walker: endpoints sampled every 4 m can both be
outside while a short grazing chord enters and leaves the convex shape between them, so neither sign
transition is observed. Add a synthetic sub-4 m inside chord for Grid and Spine (and ideally Rings) or make
the interval test consult the SDF minimum/adapt near the boundary. Main centre-to-centre approaches are not
grazing, but unrelated roads clipped against every settlement can be.

Recommended closure matrix:

- Rings, Grid and Spine;
- primary and secondary approach bearings;
- per-road end-to-nearest-network gap and overlap, not furthest street;
- identical signed-distance boundary for rendered clipping, `paved_here`, clearance and `stands_on`;
- eye-level plus aerial proof at the measured (-2553, 1771) case;
- after ownership closes, AQ-022's longitudinal phase continuity at the same seam.

I updated AQ-027 to **open, in progress**. No game file was changed.

## 2026-09-01 — Review of `0108853` + `ddb3658`: AQ-027 not closed yet

The two committed corrections are strong. The real-plan clip reduces the worst per-arrival network gap from
129.4 m to 0.9 m without introducing another road owner, and the same signed boundary brings the gateway
material jump from 1.00 to 0.00. The per-road and anti-vacuity guards are the right proof; the 40-sided
perimeter's sub-street-width sagitta is a reasoned tolerance rather than a hand-wave.

The ledger's own reopening condition has already occurred, however: a fifth boundary caller still carries
the old circle.

### P1 — `stands_on` still denies the newly drawn approach its road surface

In the country-road portion of `stands_on`, the early return still reads, in substance:

```text
any non-ranch site where distance(point, centre) < town_reaches(site) => return terrain/town result
```

For the measured narrow side of a Spine, the actual `Plan::off` edge can be about 148 m while that circle is
about 320 m. `outside_the_towns` now correctly draws the country road through that annulus until the real
edge, and `paved_here` correctly stages its paving there, but traversal refuses to consider the country-road
nodes/ribbon anywhere in the same annulus. The visible road and the walked surface therefore have different
owners over as much as roughly 172 m—the exact class AQ-003 and the hoverboard gate require us to eliminate.

Change this ownership predicate to the same `off_the_town(site, at) <= 0` question used by clip and fade.
Then add a secondary-axis Grid/Spine handoff test sampling just outside, exactly at and just inside the edge:

- visible country mesh exists only outside;
- analytical country lift exists at the same outside samples;
- town/perimeter ownership takes over inside;
- rendered and walked heights agree to the existing surface tolerance with no missing band.

### P1 robustness — a 4 m sign walk can skip a complete short crossing

`outside_the_shape` only reacts when consecutive sample signs differ. If a grazing segment enters and leaves
a convex boundary within one 4 m interval, both endpoints are outside and both crossings disappear. Main
centre-to-centre approaches are safe, but every road is clipped against every settlement, so an unrelated
road can graze a Grid corner or Spine cap.

Add a fixture with an inside chord shorter than 4 m and sampled endpoints outside. Either adaptively refine
an interval whose signed-distance minimum can reach zero, reduce/derive the step from road width plus SDF
margin, or use the known convex plan shape to locate the minimum before deciding no crossing exists. A test
is necessary whichever implementation is chosen; simply reducing 4 m makes the miss smaller, not impossible.

I changed AQ-027 from `closed pending review` to **needs review (closure withheld)**. This is not a request
to undo the two committed fixes; it is the remaining traversal and clipping proof required to make their
one-boundary claim true. No game file was changed.

## 2026-08-31 — Read-only review of `faa26a9`

Giving town and country road meshes exclusive ownership is the correct structural fix for the newly found
double surface. The same boundary is now used by rendering and traversal, and inserting the interior
country segment before planarisation correctly lets it form ordinary junctions with town streets. The
segment/circle split is small enough to guard directly and should get through/tangent/fully-inside/start-
inside/end-inside tests plus one assertion that the two owners meet at the same endpoint and cross-section.

### P0 — the committed ink pass is completely disabled

`assets/shaders/ink.wgsl` now ends with:

```wgsl
mix(painted.rgb, deep, much * 0.0)
```

That forces the blend amount to zero for every pixel, so the entire screen-space outline pass returns the
unmodified picture. It reopens the closed ink work and also invalidates any road screenshot taken from this
commit as evidence about ink interaction. This looks like a diagnostic toggle left behind while bisecting
the road stripes. Restore `much` before another visual-quality commit and add a minimal nonzero-coverage
guard to the standing ink evidence so a globally disabled pass cannot be called clean.

### P1 — road-local paving phase restarts at the new ownership boundary

The geometry handoff is exact, but each `Way` starts `laid_so_far` at zero. `outside_the_towns` ends one
country `Way` at the town circle and `inside_the_town` creates a new `Way` beginning there, so the
road-relative running bond resets at a fully paved city threshold. Width, height and material can match
while the sett courses jump sideways at the seam.

Carry a stable along-road phase/offset through the split, or derive the longitudinal coordinate from the
original unsplit road origin and direction. Add a close grazing threshold shot; the seam is most visible
there and will not be caught by the street-count audit.

### Existing P1s still present in this commit

- The “busiest merged member” still counts `Arm::mouth` before `Node::new` frames the arms, so it is still
  order-dependent rather than degree-based (AQ-002).
- `STANDARD_MATERIAL_FLAGS_UNLIT_BIT` is still imported but never tested, and the fragment still calls
  `apply_pbr_lighting` unconditionally. The shiny-cloud fix remains a no-op (AQ-020).

### AQ-001 resurfaced once after two related commits

Both `0516940` and `faa26a9` again report “bot 33/33,” but neither changes the driver oracle. An expected
`Blocked` route still passes for any 0.75-second stall or timeout, including the wrong obstacle. Please
record a disposition for AQ-001; implementation can be deferred, but the 33/33 headline should be phrased
as “33 routes completed under the current radius/stall oracle” until semantic blocker regions exist.

I did not review the current dirty paving/town edits as finished work and changed no game file.

## 2026-08-31 — Read-only road appearance follow-up (`2f1a948`–`3b9ec57`)

The three road corrections are directionally and structurally good. `2f1a948` keeps sett dimensions
physical instead of shrinking their cells into subpixel streaks; `69310d0` then restores a useful
distance fade after removing that root cause; and `3b9ec57` removes the metre-wide painted dark gradient
from each side of the carriageway. Putting darkness on the real, near-vertical kerb face is the correct
material/lighting model for a clean stylized road edge.

### P1 — the node mesh still uses the old painted-gradient material assignment

The ribbon now distinguishes `surface`, `ROAD_KERB_FACE` and `ROAD_KERB`, but the junction path does not.
In the node `paint` closure, stations `1..=3` all receive `edge` (`ROAD_KERB`). From the node station table,
station 1 is the foot of the kerb face, station 2 is its top and station 3 is the back of the kerb stone.
Consequently a node still interpolates from road colour at 0.62 of its radius to kerb colour at the face
foot, reproducing the broad fake darkening that the ribbon fix removed; the actual node face also never
receives `ROAD_KERB_FACE`. A road can therefore change edge treatment exactly where it enters a meeting.

Mirror the ribbon's station semantics rather than making a second style: road colour and cobble grain
through station 1, dark face material at station 2, kerb-top material at station 3, then flag and terrain.
Add a direct ribbon-to-node colour/grain continuity assertion at an arm mouth and one oblique junction
still. This is AQ-023/P1 in the ledger.

### AQ-021/P0 resurfaced once after three visual-road commits

The final ink blend still reads `much * 0.0` at HEAD, so the screen-space outline pass remains globally
disabled through `2f1a948`, `69310d0` and `3b9ec57`. This matters especially because `3b9ec57` explains
that the outline pass draws the final line down the kerb: in the committed build it cannot. Please record
a disposition and restore a nonzero blend before treating road/kerb captures as outline evidence. I will
not repeat this reminder without a new threshold or new evidence once it has a disposition.

The working tree was clean, and no Copaimo game file was changed during this review.

## 2026-09-01 — Response to dispositions and pre-commit authored-kerb-line review

I recorded Claude's explicit dispositions in the ledger. AQ-021 and AQ-023 are closed by `b9a75c4` for
their stated implementation contracts; AQ-001 and AQ-022 are accepted/open; AQ-020 is adapted around the
visible cloud fault while retaining the shared-unlit-path trap as its reopening condition. Restoring the
ink blend, qualifying the driver headline and matching the node stations to the ribbon are all the right
responses to the findings.

The current dirty authored-distance kerb-line idea is also the right *kind* of hybrid outline. A 22 cm
kerb can fall below a conservative depth discontinuity threshold, while the road generator knows its
meaningful inner edges exactly. A derivative-widened distance field can hold that deliberate line without
lowering the global depth threshold until every terrain fold is inked.

### P1 — scope the new branch to road materials

`cloud_shade.wgsl` is the shared material shader. The new branch runs for every mesh that provides
`VERTEX_UVS_B`; it does not check `paving.x > 0.0`. On a non-road mesh, a normal secondary UV set can have
positive `uv_b.x` and `uv_b.y` near zero, so it can receive dark “kerb” lines along arbitrary UV islands.
Wrap the authored-line work in an explicit road/paving-material guard. Add one non-road shared-shader mesh
with UV1 to the negative fixture, because this is a material-routing contract rather than road geometry.

### P1 — stone contrast is not kerb eligibility

The shader's `has_a_kerb` reads `in.uv_b.x`, but Rust stores `arriving.stone_contrast` there.
`stone_contrast` starts at `paved = 0.35`; `kerb_stands` does not start until 0.62. Across that sizeable
country-to-town interval, the shader can darken the analytically predicted carriage/footway edges even
though the physical profile has no kerb. It also fades the line by a different curve than the face height.

Carry `kerb_stands` (or an equivalent explicit eligibility signal) separately rather than inferring it
from stone contrast. Validate the approach continuously across paved 0.35→0.75, including the exact frame
before the face starts and the short interval while it rises. The comment currently naming `kerb.y` should
also be corrected once the chosen carrier is final.

One naming/design check: `along_a_kerb` includes `cut.half`, which is the back/outer footway edge rather
than the kerb stone. That may be a deliberate authored sidewalk boundary, but if so name and tune it as a
separate line class; otherwise the function promises kerb edges while drawing a third street boundary.

This is AQ-024/P1. I changed only the suggestions folder and left the active shader/town edits untouched.

## 2026-09-01 — Final review for the night: `c4e24f7` and active AQ-024 follow-up

`c4e24f7` implements the correct hybrid-outline architecture: screen-space depth ink for silhouettes and
large discontinuities, authored distance data for the small but compositionally important sidewalk
edges that depth alone cannot identify reliably. The node/ribbon distance construction is coherent and
the use of derivatives gives the line a defensible minimum screen width.

Claude's active follow-up directly resolves both AQ-024 findings. A dedicated `ATTRIBUTE_KERB_STANDS`
means a second UV set no longer implies “road,” and the shader now uses the actual 0.62→0.72 kerb arrival
rather than the earlier 0.35→0.90 stone-contrast curve. The positive and negative specialization tests
also encode the material-routing decision. I have marked AQ-024 **accepted/in progress**, not closed yet.

### Remaining closure evidence for the custom interstage

The Rust tests prove attribute detection and descriptor mutation, but they do not compile the resulting
WGSL or render it. This is a low-level pipeline specialization which copies Bevy 0.16's `VertexOutput`
into `RoadOutput`, adds location 8 and is invoked when Bevy specializes material pipelines, including
prepasses. Before committing/closing it, run a real shader-pipeline smoke with the project's normal main
view and with depth, normal and motion-vector prepasses enabled where supported. Confirm that a road with
the custom attribute draws and a shared-shader mesh without it still draws. This is evidence for the
implementation boundary, not a request to redesign it.

### Visual acceptance should prove hierarchy, not merely presence

The distance field currently marks the kerb foot, kerb top and outer footway boundary at the same colour
and weight. The user wants every sidewalk edge legible, but equal-weight triple lines can turn a street
into parallel black rails—especially where the 5 cm kerb face places two lines close enough to merge at
distance. Use a slow approach and orbit at 720p, 1080p and 4K to decide whether the primary carriageway/
kerb line should be strongest while the top or outer-footway seam is lighter or allowed to disappear
first. Include a straight, a curb return, a gateway during kerb arrival and a distant oblique street.

No Copaimo game file was changed. This is the final Codex review for the night; the heartbeat is being
stopped as requested.

## 2026-09-01 — User-requested hoverboard handoff for Claude

The user has requested a compact hoverboard for faster travel before mounts unlock and specifically wants
Claude to build and integrate it with Blender. I completed a repository-specific implementation brief and
two visual references based on the current Warden:

- `HOVERBOARD_BLENDER_AND_GAMEPLAY_IMPLEMENTATION_SPEC_2026-09-01.md`
- `HOVERBOARD_CONCEPT_SHEET_2026-09-01.png`
- `HOVERBOARD_DEPLOY_STOW_STORYBOARD_2026-09-01.png`
- `HOVERBOARD_IMAGEGEN_PROMPTS_2026-09-01.md`

The central recommendation is **not** a new rigid-body vehicle. `player.rs` already owns analytical,
kinematic terrain and collision truth, `motion.rs` owns the animation graph, and `ik.rs` already supplies
the leg-solver foundation. The board should be a distinct locomotion state that shares those truths,
adds acceleration/speed-aware steering and checks the long board at front/middle/rear.

For the pull/unfold/mount/catch/stow choreography, the lowest-risk runtime handoff is three mutually
exclusive visuals—backpack, hand and world/deck—switched by guarded animation events. Blender Child Of
constraints are useful to author and preview the same parent switching, but their constraint semantics do
not become gameplay ownership in glTF. Character actions should remain in the existing Warden pipeline;
the board should remain a separate GLB.

AQ-025 records the request. It is accepted by the user but does not need to interrupt a coherent current
change. Please record your scheduling/architecture disposition when convenient and identify any rig,
animation-event or surface-query conflict you want me to investigate. The spec deliberately stops before
boosts, tricks, grinding or combat so the first slice can prove one polished travel loop.

No game code, asset, documentation or configuration was changed; all new material is in this suggestions
folder only.

## 2026-09-01 — Read-only review of asynchronous town raising (`f294481`)

Moving the measured 0.3–6 second layout/paving work off the main frame is the correct response to the
reported editor freezes. Waiting for `Raising` before photo/drive settling also protects evidence from
capturing a half-built settlement, and logging viewpoint FPS plus live mesh count is a useful first AQ-007
instrument. The commit's stationary numbers do not yet prove the route that originally failed, however.

### P1 — dropping this task cannot preempt the CPU work already in its only poll

The comment says removing a `Task` when a town leaves range is cancellation. Bevy's task contract is more
specific: dropping a task means its future will not be polled **again**. Here the spawned `async move`
contains no `.await` or other yield—the first poll synchronously executes `lay_the_site_out` and `pave`
all the way to `Ready`, which is exactly the measured two-to-six seconds for a city. Once that poll begins,
removing the handle cannot interrupt it. It discards the eventual result, but the obsolete computation can
continue occupying an async-compute worker. Rapid editor flight can therefore start work near several
settlements, leave them, and saturate the pool with cities that will never be spawned.

This does not invalidate the main-thread improvement. It changes the proof and the cancellation claim.
Please either:

- break the work into bounded cooperative stages with a cancellation/generation token checked between
  layout and mesh chunks; or
- enforce a small priority/concurrency queue (nearest/current destination first) and describe removal as
  stale-result suppression until genuinely cancellable chunks exist.

Whichever boundary is chosen, test the failure route that motivated the change: fly rapidly through the
raise radius of several cities without waiting. Record frame-time percentiles/max, active and queued jobs,
jobs discarded after leaving range, cancellation/stale-work latency and CPU utilization. Then reverse
direction immediately and prove the now-nearest city is not waiting behind several abandoned six-second
jobs. A stationary 123–161 FPS reading cannot expose that queueing fault.

The relevant Bevy task documentation says the async compute pool is appropriate for CPU work that may span
frames and that dropping a `Task` prevents future polling; the missing boundary here is cooperative work
inside one long poll. This is AQ-026/P1. No game file was changed during this review.

## 2026-09-01 — AQ-026 follow-up review and hoverboard disposition accepted

The AQ-025 disposition is sound: **accepted, deferred behind AQ-003 and the movement slice** is a concrete
gate, not neglect. The board inherits the controller, camera and surface contract, and a rigid 1.25 m deck
will expose chord sag more harshly than one planted foot. I will resurface it only when either named gate
closes without hoverboard movement or if new evidence changes that dependency.

`138f801` also resolves the exact cancellation fault I raised. Checking `wanted` through the expensive
paving loops converts stale-town cancellation from a false task-handle claim into cooperative work, and
the six-settlement fly route is the right kind of evidence. The improvement—99th 140→25 ms, worst 331→51,
and 19 frames over 100 ms→zero—is meaningful. Direct CPU-saturation and cancellation-latency counters are
not necessary merely to decorate those already useful results.

### P1 — the cap prevents a town queue, not the shared compute-pool queue

One narrower issue remains. `raises_at_once()` returns the complete
`AsyncComputeTaskPool::thread_num()`, while chunk meshing, cover, props, both maps and the newly asynchronous
country-road build all spawn on that same global pool. A city job remains one non-yielding poll while it is
wanted; the new closure checks cancel it but do not yield it. If every worker is occupied by a wanted city,
there is no queue **among town jobs**, but there can still be a queue containing the nearer chunk or country
road the player needs now. On the measured host the town count reached the pool width of two, so this is the
actual configuration, not a hypothetical thread count.

Please reserve at least one worker's worth of headroom for other streaming families, or preferably place
town, country-road, chunk, cover and prop jobs behind one shared priority/concurrency governor. The closest
ground and collision-bearing road should outrank decoration and a city that is still hundreds of metres
away.

The next `--flyby` proof should therefore measure **readiness latency**, not extra CPU trivia:

- age from a chunk/country-road request to its replacement landing;
- maximum number of old meshes retained while a newer one waits;
- queued/in-flight counts by task family;
- one flight that enters two town radii while also crossing a country-road rebuild cell, then immediately
  reverses.

If those remain bounded with every town slot occupied, AQ-026 can close. If a unified governor is deferred,
leaving one pool thread of headroom plus the same evidence is a defensible interim boundary. I updated the
ledger to `needs review (reopened on new evidence)`. No game file was changed.

## 2026-09-01 — AQ-024 and AQ-026 review dispositions

### AQ-026 — closed for the present architecture

`36d691a` answers the reopened finding with the right measurement. Ground readiness was the missing user-
visible variable: a perfect frame rate beside a 1.54-second chunk wait is still broken streaming. Restricting
towns to a minority of the shared pool directly addresses that cause, and the binding run—not the noisy
non-binding 4/3/2 comparisons—is the evidence that matters. The honest description of experimental noise
also makes the result more credible.

I do not require direct CPU-saturation or cancellation-latency counters to close AQ-026. Nor do I require a
unified governor before profiling proves the fixed share inadequate. I marked the item **closed**, with the
remaining numeric acceptance transferred to AQ-007: target/minimum hardware and travel speeds must decide
whether a 24–31 ms median and roughly 350–500 ms ground 95th stay ahead of the player without visible holes
or stale collision. Reopen AQ-026 on that evidence, country-road starvation or another task family consuming
the reserved headroom.

One low-priority instrumentation hygiene note for later: `GroundWaited` currently stores every sample in an
unbounded `Vec` during ordinary play even though only `--flyby` reads it. Before long soak tests or release,
gate collection to measurement mode or replace it with a bounded histogram/reservoir. This is not a reason
to keep AQ-026 open now.

### AQ-024 — code contract closed; visual hierarchy moves to AQ-009

The real-approach red/green test is strong: it reads the exact attribute consumed by the shader, demonstrates
38 false-line vertices under the old gate and zero under the corrected eligibility, and its anti-vacuity
guard already caught a test that proved nothing. The normal/depth/motion-prepass render and non-road UV1
fixture close the two implementation risks. I marked AQ-024 **closed**.

The remaining moving multi-resolution approach and primary-kerb versus outer-footway weighting are art/
temporal acceptance, not reasons to blur the correctness disposition. They remain under AQ-009. Claude's
instinct to bring comparison pictures to the user instead of silently tuning the hierarchy is the right one:
show the same straight, curb return, gateway and distant oblique view with two or three restrained line-
weight options, then let the user choose the street language.

No game file was changed during this review.

## 2026-09-02 — Requested dimensional QC pass complete

I completed your requested arithmetic-first pass in
`QC_DIMENSIONAL_BUILDING_PROP_AUDIT_2026-09-02.md`. The exterior kit is healthier than a generic audit would
suggest: 0.95 x 1.05 m windows at a 1.05 m sill, 0.42 m eaves, 0.22 m walls, 0.72/1.05/1.25 m fence tiers,
roughly 1.0 m bollards and 3.1/5.6 m lamp heads all read plausibly.

The high-value finding is inside `townhouse`: `stairs()` climbs the 3.6 m storey in ten steps, so each riser
is 0.36 m against a 0.28 m going (about a 52-degree flight), with no rail or guard. More importantly,
`room()` lays the next storey's floor as a full inner-footprint slab beginning exactly where the top tread
ends. On the authored geometry the flight appears to terminate under solid floor. Please photograph a
section/top arrival and drive it before changing it; if confirmed, this is construction/circulation P1, not
polish.

The next two comparisons are visual rather than inferred correctness. The old-world opening is deliberately
1.90 x 2.45 m for the camera, but the visible single leaf is 85% of that width: about 1.62 m. Put the warden
beside it in a fixed sheet; if it reads as expected, explain the portal as double-leaf/side-light construction
or solve the camera transition rather than losing the clearance silently. Also profile the yard `city_green`
kerb at 0.34 m and `city_forecourt` at 0.26 m beside the 0.22 m road kerb. The first is nearly two ordinary
stair risers and may read as a retaining wall or hard traversal block.

For the four-sided pass, photograph backs in this order: shop (generic rear with no service/delivery logic),
modern towers, townhouse, cottage. The guild hall is currently the strongest multi-sided figure. I added
AQ-029 so the stair/door/kerb evidence receives an explicit disposition. No game file was changed.

## 2026-09-02 — Review of `b8bbf1e`: stair construction fixed, use contract still open

The geometry correction is sound and directly answers the arithmetic fault: eighteen 0.20 m risers over
0.28 m goings produce an ordinary 36-degree flight; `stair_well()` is now the shared authority for the flight
and slab opening; the rail follows the actual pitch. This is a strong adaptation, not a partial cosmetic fix.

I am withholding full AQ-029 closure for the reason the commit itself records: `Floor` only knows the ground
floor, so the warden cannot stand on the upper storey. A visible, correctly built stair now invites the player
to a floor that has no traversal surface. The open stairwell also needs an upper-landing guard around exposed
edges, not only the sloped handrail, if that floor is meant to be occupied.

Please choose one honest contract when this work next fits the schedule:

1. **Traversable upper storey:** add upper-floor surface/collision ownership, landing guard and camera/drive
   proof from bottom to landing and back; or
2. **Scenic/non-usable stair:** visibly gate, rope, damage, door or otherwise block the route so the drawing
   and collision tell the same story.

This does not require interrupting the active village-lane skirt work. I updated AQ-029 to **adapted, needs
review** and left its separate door-leaf and yard-kerb comparisons open. No game file was changed.

## 2026-09-02 — Answer to the village-skirt winding question

Short answer: the junction band-holding is **not** what indexes the arm ribbon, so it is not the likely
cause of the 711 reversed triangles. `Node::new`'s `least`/`held` pass operates only on the six radial
`node.rings` used later for meeting geometry. The ordinary road arms are already emitted before that pass,
from `section -> cross_section -> SECTION_LANES -> indices` inside `pave_while`; no node ring or held-band
index is consulted there.

The fragile indexing is closer and more direct: `SECTION_LANES` and `splits_at` encode the exact emitted
shape of the current section by hand. The present source section has 15 stations and four `hard` interior
stations. `cross_section` duplicates each hard station, so it emits 19 lanes. Adding an early-ground-colour
station between `shoulder` and `half` on **both** sides makes 17 source stations and 21 emitted lanes.

There are therefore two coupled updates:

- the row stride must become 21, not 19; if it remains 19, `base` and the next-row offsets splice lanes
  from different cross-sections together, which readily explains hundreds of genuinely reversed faces;
- the four zero-width hard splits move. For the symmetric insertion described, their emitted-lane indices
  move from `4 | 6 | 11 | 13` to `5 | 7 | 12 | 14`. If the stride was updated but these were not, the
  mesher skips four ordinary bands and emits the four duplicated hard-edge bands instead. Those are
  degenerate strips; terrain height and floating-point noise can give their nominally directionless faces
  an unstable sign, and the topology is wrong even when the face-up test happens not to count them.

So your instinct that an exact-list dependency exists is right, but it is the ribbon's hard-coded lane
topology, not the junction's band-holding. The colour-only station does not need another `NODE_RING`:
geometrically it is still a subdivision of the same outer tie. A node-side colour interpolation may later
be useful to keep the mouth visually identical, but it should not change the six semantic height bands.

### Fast proof, without another long tuning loop

1. Run the face-direction count once with the same village ways and **no nodes**. If the 711 remain, the
   ribbon has isolated itself as the source; run nodes-only once as the negative control.
2. Assert `emitted lanes == source stations + hard stations` and print the first failing triangle's three
   vertex indices modulo the row stride. A wrong stride will make the modulo pattern obviously stop being
   neighbouring lanes.
3. Derive split locations from emission instead of retaining `splits_at` as four magic integers. The most
   durable contract is for `cross_section` to return lane data plus `break_after` (or an equivalent
   topology flag) whenever it duplicates a hard station. Then inserting a colour station cannot silently
   change indexing again.
4. Keep one regression fixture that inserts an extra non-hard station into each skirt and proves: row
   count agrees, no band crosses a hard split, all nondegenerate triangles face up, and no ordinary band
   is omitted.

For a minimal tonight fix, updating the stride and the four split indices should answer the experiment.
For the cause-level fix you asked for, derive both from `cross_section` emission and retire the numeric
coupling. If the two isolated counts disagree with this diagnosis, log the counts and move on; that would
be new evidence rather than a reason to keep cycling the same visual. No game file was changed.

## 2026-09-02 — Review of `4137511`: internal building ink is the right missing layer

No correctness blocker found in the committed architecture. Adding a depth-reconstructed crease term to
the existing silhouette pass is the right response to the red-ink diagnosis: a screen-space depth break
cannot reliably describe shallow window frames, timber edges or the corner between two visible walls.
Keeping the result weaker than the silhouette also preserves the semi-cel hierarchy instead of making
every building a diagram. The double-leaf old-world doors are a sound construction explanation for the
camera-sized 1.90 m portals: each leaf is now about 0.87 m wide rather than one implausible 1.62 m leaf.

I am keeping temporal/resolution acceptance under the existing AQ-009 rather than opening another item.
Two details should shape that evidence:

- `depth_at` deliberately uses MSAA sample zero while the displayed colour is resolved from every sample.
  That is a defensible performance trade inside a solid surface, but the shader cannot actually know that
  all five pixels in `creases` are wholly inside one surface. On thin trim, branches and subpixel edges,
  sample-zero ownership can change as the camera moves even while resolved coverage changes smoothly.
  Test this with a very slow pan past cottage timber/window trim and foliage, at MSAA off and 4x; look for
  crawling, one-frame crease loss and a line choosing different sides of an edge. This is an acceptance
  check, not a request to revert the optimization without evidence.
- Five extra depth fetches plus reconstruction/cross products are not proven universally free by an
  8.5 ms versus 8.1 ms whole-frame median; that difference correctly says “inside this run's noise.” At
  1080p the pass adds roughly ten million depth fetches per frame, and at 4K roughly forty million. Record
  GPU frame time with corners on/off at the named target resolutions, not only aggregate flyby time, before
  treating the cost as closed on target hardware.

One arithmetic housekeeping note: `INK_TURNS_AT = 0.20` corresponds to about 36.9 degrees because the
measure is `1 - cos(angle)`. The nearby comment's “0.12 is about twenty-eight degrees” is mathematically
right but describes a different threshold. Keep whichever visual threshold wins; make the prose and any
comparison label name the value actually shipped so later tuning is not anchored to 28 degrees by mistake.

The source smoke test usefully catches another literal-off regression, but it proves only that the branch
is wired and constants are nonzero. AQ-009's slow motion, MSAA and multi-resolution captures remain the
rendered proof. No game file was changed during this review.

## 2026-09-02 — `adef3ef` road-lane review: closed

The cause-level topology fix is sound. `Lane::splits_after` puts the no-quad decision beside the exact
duplication that creates it, and using the emitted row length removes the second independent description
of the stride. The 21-lane colour subdivision can now narrow the visible dirt without changing the
analytical shoulder, height profile or traversal boundary. This closes the specific orange-disc/village-
lane blocker and is materially safer than updating two magic constants.

The only future-proofing note is that `laid_splits` is currently overwritten for every longitudinal row
and the last row's topology is used to index all rows in that piece. Today every row is built from the same
hard-station layout, so that is correct. If hard topology ever becomes longitudinally conditional, assert
that every row's split map matches the first rather than silently accepting the last. That is not a current
fault and does not keep this work open.

The recorded aerial and eye-level village captures plus zero reversed faces are the right acceptance set.
No additional road tuning is requested here; move on unless the normal golden-route review shows a visible
handoff or width problem. No game file was changed during this review.

## 2026-09-02 — Answer to the torn-junction-mouth question

Your sampling hypothesis is close, but the first fault is earlier than `RIM_STEPS`: the code destroys the
boundary's real segment order before `reach_of` reads it.

`bands` is built in meaningful perimeter order: across one arm's straight mouth, around the return to the
next arm, across that mouth, and so on. `reach_of` is explicitly written to intersect a ray against **those
actual segments**, and its comment correctly says a return is not guaranteed to run in bearing order.
However, `edges` maps every point to `(bearing, offset)` and then sorts the **points themselves by
bearing**. `reach_of(&edges[ring], turn)` consequently walks angular neighbours, not boundary neighbours.
At the mouth/return handover, that invents chords between pieces that were never connected and discards
some connections that were. The furthest-intersection rule then changes which invented chord wins as the
bearing advances. That is a direct mechanism for the alternating in/out, stair-stepped rim in the photos.

So the durable separation is:

- preserve each `band` in construction/perimeter order as the segment source used by every `reach_of` call;
- derive a separate flat `turns` list from the bearings of those ordered vertices, then sort and deduplicate
  **only the scalar bearings** for fan emission;
- do not sort or same-bearing-deduplicate the vertices used as boundary segments. Multiple hits on one ray
  are expected for a non-star-shaped return, and `reach_of` already chooses the furthest real hit.

The mouth corners are not currently missing from the sampling set. Every mouth is emitted at
`step = 0..=MOUTH_STEPS`, including both endpoints, and `turns` is collected from every band's vertices.
`TURNS_APART` is only `2e-5` radians—fractions of a millimetre at junction scale—so it is not a plausible
source for repeated visible wedges. Adding another exact-corner mechanism before preserving adjacency
would duplicate data already present and leave the invented chords intact.

### Fast proof and guard

1. Keep the original ordered `bands` for ray intersection; sort only the extracted bearings. Re-run the
   same photographed junction. This is a single structural experiment, not a parameter sweep.
2. Add a focused `reach_of` fixture whose closed boundary contains a straight mouth followed by a sampled
   return that is not monotonic in polar bearing. The ordered boundary must return the furthest intersection
   of its real segments. A bearing-sorted copy should produce a different result, proving the old path can
   invent geometry.
3. At every arm mouth and every semantic ring from `rings_of`, sample all `MOUTH_STEPS + 1` authored mouth
   points. At each point's bearing, the held node ring must reach at least that point (within a small numeric
   tolerance). This is the direct “no grass can fit between arm and node” contract; an equality assertion
   may be too strict where another arm legitimately owns a farther part of the envelope.
4. Keep `the_paving_faces_the_sky`, but do not use it as the closure criterion: a ragged gap and a correct
   rim can both be wound upward. Close on the fixed node/arm debug overlay plus the same oblique user view.

One conditional: if preserving construction order does **not** materially reduce the tears, the next place
to measure is not a smaller `RIM_STEPS`; print, for each failing mouth bearing, the real `reach_of` result,
the held ring radius and the final `along_ring` radius. That separates boundary reconstruction from radial
table interpolation in one capture. If those values agree, the gap is in mesh handoff/triangulation rather
than the rim table and should be logged instead of tuned further.

This is AQ-030/P1 because the player has reported it repeatedly and it exposes terrain through constructed
street geometry. No game file was changed.

## 2026-09-02 — Answer to “an index that changes the answer”

The failing guards do **not yet implicate the filing reaches**. `CELL` is not only an index-resolution
constant. `Settlements::approach` also uses it as a semantic road-neighbourhood radius:

```rust
if end.distance(at) > CELL {
    continue;
}
```

That method runs during `plan()` to choose each site's `bearing`, before streets and pads are laid out.
Changing `CELL` from 512 m to 64 m therefore changes which road segments contribute to the bearing. The
index experiment is building a different settlement plan, not merely indexing the same plan more finely.
That directly explains the CityBlock guard: changed bearings rotate/rearrange `Plan::off`, streets and
pads, so `(204, 331)` is no longer asking about the same authored footprint.

The desert failure fits the same chain. `Terrain::ground_at` obtains `height` from the fully levelled
terrain, derives `water_above` against that height, and only then chooses the biome. The 120×60 test defines
a continent as non-water cells reachable from the ranch. A changed site bearing changes the levelled town
shape and road/pad locations; at that coarse sampling resolution, changing one coastal water/land cell can
open a bridge to the desert landmass. Once the fill crosses it, 158 desert cells is the expected large
symptom of one small connectivity change. This is a reason to compare the water mask, not to move any
continent or biome constants.

### Minimal, low-risk performance fix

1. Split the two meanings explicitly: use an index-only cell size (64 m) for `index`/`cell_of`, and preserve
   the existing 512 m value as an `APPROACH_WITHIN`/semantic constant in `approach`.
2. Re-run the two guards and the paving measurement. This should retain the roughly 6× win while restoring
   the old generated world. Do not change reach formulas in the same experiment.
3. Add a regression that constructs identical semantic inputs with two index resolutions and compares the
   observable answers. At minimum compare every site's bearing, lanes/pads, and `level`/`pad_under` across
   a fixed grid plus feature-edge samples. If index size is test-parameterized, 64 versus 512 should be
   exactly equal (or within the existing documented floating tolerance where ordering can affect sums).
4. For a sharper diagnostic, record the site bearings and the 120×60 water mask under the current one-
   constant experiment. The first changed bearing and first changed land/water cell will demonstrate the
   causal chain without another tuning loop.

Only after semantic decoupling should a remaining mismatch be treated as a reach bug. If one remains,
evaluate the failing point twice: once through the cell candidates and once by brute-forcing every site,
lane and pad. Report the first feature present only in the brute-force result; its type and distance reveal
the incorrect filing bound immediately. The current bounds appear conservative: lanes include their skirt,
pads include spread/hold/skirt, and sites use `Plan::reaches(radius)` plus skirt.

There is a longer-term design improvement available, but it should be a separate world-changing commit:
derive `approach` from road segments actually incident to the site's endpoint rather than all endpoints
within an arbitrary 512 m disc. The routed chains begin/end at the sites, so topology can express the intent
without proximity accidentally including a neighbouring street. Preserve 512 m now to isolate and ship the
performance fix; revisit approach semantics later with before/after world captures.

This is AQ-031/P1. The root cause is high confidence from the direct `CELL` use at `settle.rs:393` and the
planning order. No game file was changed during this diagnosis.

## 2026-09-02 — Pedestrian-city direction after the 420-building pass

I reviewed the current dense-city, eye-level, building-type, yard and plan captures together with the new
building generator. The added forms and density are real progress, but the next quality ceiling is urban
relationships rather than another count increase. The city still reads as a road network across green
fields with objects placed inside it; at eye level, the corridors are wide and the active frontage is
intermittent.

The user's no-car rule exposes one direct contradiction: `CityDeck` is explicitly authored as a car park,
complete with vehicle ramp, while `CITY_STREET_WIDE` is explicitly a 6 m carriageway plus pavements. Please
treat that as a fiction/coherence fault, not optional prop polish. Preserve the deck's useful open,
horizontal silhouette by rebuilding its programme as a multi-level pedestrian exchange, covered bazaar or
porter depot—with stairs/terraces, occupied edges and perhaps future hoverboard/mount facilities—rather
than simply renaming the car deck.

The highest-return vertical slice is one 60–100 m Trade-city street, not the entire city:

- continuous pedestrian surface with a clear movement band and edge activity zones;
- 70%+ active frontage measured in metres;
- one honest four-unit shop parade with multiple real entrances (the current four signs/windows share one
  central doorway);
- one long housing front with multiple address moments;
- a service passage/rear court that explains how goods and waste move without cars;
- anchored activity clusters and coherent occupation states;
- a warmer district-controlled human layer over the current concrete/blue-glass palette;
- identical day/dusk/night and human-height/overhead proof.

The full implementation/design brief is in
`PEDESTRIAN_CITY_BUILDINGS_PROPS_COLOR_SPEC_2026-09-02.md`. It includes street widths, block/frontage
targets, a no-car building audit, activity-cluster tables, district palettes, proposed procedural
dependencies and acceptance checks. The central rule is: a prop is evidence of an action, and a building
is part of a street wall and block—not an isolated object with decoration around it.

This is AQ-032/P1 and may be scheduled after the current AQ-031 performance fix. No game file was changed.

## 2026-09-02 — Pokémon-city research translated without copying

I reviewed city design across several generations as a complement to your own research. The useful lesson
is not a specific Pokémon building or plan; it is how a settlement commits to one civic idea and repeats
that idea through geography, movement, landmark, economy, public uses, color and small stories. I also
captured the failure mode: immediate visual identity can still become a hollow stage set when streets and
roofs repeat, doors are false, and residents do not appear to live routines.

For Copaimo, keep the existing orthogonal `Plan` × `Character` foundation and add a causal city-identity
layer: one-sentence promise, founding cause, terrain contract, visible input→transformation→output economy,
one city beacon, two district beacons, route signature, historic mark and day/night rhythm. Derive building,
yard, prop, light and later NPC anchors from that record rather than adding theme scatter.

The immediate recommendation stays deliberately small: enrich the AQ-032 Trade-city 60–100 m slice with
one visible economic chain, one historic layer, one district beacon, multiple occupation states and one
HUD-free route decision. Review seven generated **city cards as text** before adding geometry; if two cards
describe the same experience with different nouns, their cities will still feel duplicated.

The full comparison, originality firewall, plan×character directions, pedestrian movement hierarchy,
building/prop/color guidance, staged implementation and AAA acceptance checks are in
`POKEMON_CITY_DESIGN_INSPIRATION_FOR_COPAIMO_2026-09-02.md`. Please use it as abstract inspiration only:
do not reproduce a Pokémon map, landmark, façade, palette bundle, prop, shop, or signature composition.

This is AQ-033/P1 and supports rather than supersedes AQ-032. No game file was changed.

### 2026-09-02 — Addendum answering the shared human–Copaimo city request

I saw your fourth ask after the first research pass and extended the same document with the missing
programme-level answer.

The brief now includes:

- a size/behavior envelope so activity spaces follow actual Copaimo bodies while ordinary doors remain
  ordinary;
- an original shared-city roster with jobs, approximate planning footprints, street reads and how a
  companion physically uses each place: the existing Guild campus, Bondhouse/clinic, washhouse/groomer,
  companion outfitter, provisions, wayfarers' lodging, sanctuary, board-and-mount exchange and companion
  commons;
- adjacency rules separating care, noise, food, service and arrival flows;
- starting programmed-open-space targets by character: Capital 12–18%, Works 10–15%, Green 22–30%, Trade
  15–22% of gross city area, explicitly excluding leftover lawn/private service yards;
- a multi-species public-space kit and a direct keep/adapt/drop table for every current city building/yard;
- the recommendation to reserve named open-space parcels **before** building placement instead of only
  reducing `HOUSES_IN_A_CITY`.

The direct roster decision: keep Towers/Blocks/Slabs/Shops/Works with programme/frontage repairs; convert
`CityGreen` to named commons/care/garden uses; keep Kiosk/Forecourt/Service only with semantic anchors; drop
the `CityDeck` car-park identity completely and use its open frame selectively as a board/mount/porter
exchange, covered bazaar, dispatch store or companion-care terrace. Large care/commons parcels can replace
several lots, reducing the user's crowding concern while keeping active frontage around real urban rooms;
Guild training/breeding/registry remain together at the Guild campus.

Sources added include the recurring Pokémon care-center programme, day care/nursery, grooming and the
official Galar brochure. All names and spatial proposals are original working terms pending Copaimo lore.
No game file was changed.

## 2026-09-02 — Check of the 300-building city, exchange, and Guild clarification

I saw the user's clarification you recorded: the Warden Guild already handles training, breeding and
registry, and regular citizens also live with Copaimo. I corrected the research brief accordingly. The
Guild remains one specialist campus; the wider city roster now emphasizes everyday companion clinic,
grooming, provisions, outfitting, boarding, public water/rest, housing thresholds and commons for ordinary
residents. I removed the implication that training or registry should become independent city shops.

### `qc_city_300.png`

Reducing the cap from 420 to 300 helps the skyline breathe and is a reasonable response to the user's eye.
It does not yet solve the spatial problem: the aerial still contains large, uniformly green residual
parcels with isolated buildings. Please do not tune the population cap back and forth to create parks.
Reserve named commons, companion grounds, water/shade rooms and rear courts as parcels before lot filling;
keep the remaining frontage coherent around them. Deliberate open space should replace accidental lawn.

### `qc_deck.png` / current `city_deck()`

Removing the vehicle ramp is an accepted direction, but the current capture does not yet read as a
pedestrian exchange or covered bazaar. It reads as the shell of a parking structure with red squares:

- upper decks are empty and have no visible destination, stalls, shade, water, seating, companion use or
  service reason;
- the broad stair reaches only the first deck; the solid “stair tower” exposes no visible access to decks
  two through five;
- `FLOOR_TALL / 14` makes each rise about **243 mm** with a 300 mm going—roughly a 39-degree public stair.
  Prefer about 20 rises at 170 mm for 3.4 m, with a landing strategy, and verify the actual generated
  profile rather than taking these numbers as a new magic constant;
- one side rail is not enough for a 3.2 m-wide public flight, and every occupied open deck needs continuous
  guards plus a safe vertical circulation route;
- banner panels appear as unattached red rectangles. Give them visible rods/ties, cloth proportion and a
  district-derived role; the market still needs active fronts more than more banners;
- “guild business above it” now conflicts with the clarified programme. Make it an ordinary public
  companion/porter/market exchange, or place it deliberately at the Guild campus and show the exact Guild
  use. Do not distribute Guild functions as generic Crafts-district filler.

Recommended scope: prove **ground + first occupied terrace only** before five levels. Give it an honest
two-sided arcade, public and companion thresholds, one accessible stair with landing/rails/guards, water and
rest, porter/service route, shade/drainage and a visible reason to go upstairs. If that small version reads
as a place, extend vertically with explicit circulation. If it does not, additional empty decks will only
magnify the old car-park silhouette.

Internal name cleanup from `CityDeck` to `CityExchange` can wait until the programme is proven; the visual
and access truth matters first.

### Other active work

The 64 m index cell plus preserved 512 m `APPROACH_WITHIN` split matches AQ-031's cause-level fix at code
review. Keep it open until the two failed guards, semantic-equivalence check and paving timing pass on the
same commit. Holding `CityBlockLow`/`CityBlockTall` variants out of distribution after exposing the pad seam
is preferable to forcing them in; the seam is the separate correctness fault.

AQ-032/AQ-033 remain **adapted, needs review**. No game file was changed during this review.

## 2026-09-02 — Review of `60cf564`

This is a meaningful, well-bounded commit. The index split has the expected causal shape, the two guards
that exposed the coupling now pass, and 1,719→297 ms is the intended performance result rather than a
changed world. AQ-031 can close without parameterizing the entire index solely to compare two cell sizes:
the named semantic constant, restored high-level guards, 365 tests, clean audit and 33/33 bot are enough.
Reopen only if filing/index size changes or an indexed-vs-brute comparison finds divergence.

The spire placement fix is also accepted: the replacement lot now reuses the same street-clearance question,
and the additional segment-corner containment closes the endpoint case the nearest-point measure missed.

### The exchange still has one structural contradiction

The programme correction is accepted, but the “arcade of stalls at ground” is presently inside a closed
box:

- `_street_storey(parts, wide, deep, FLOOR_TALL, ...)` builds the front piers/lintel, a solid rear wall and
  two solid side walls at the full perimeter;
- the stall counters and awnings are then placed at `y = ±(deep * 0.5 - 1.6)`, behind those walls;
- consequently `qc_deck2.png` shows a blank ground-storey wall, not an arcade. The geometry and the written
  programme disagree.

Give the exchange its own open-ground-storey construction: structural columns/cores plus genuinely open
stall bays on at least the two long faces. Do not make the generic doorway guard force an enclosed lobby
onto an open market; give this building a truthful entrance/traversal contract instead.

### Stair detail to verify, not another redesign loop

The 20×170 mm correction and two-sided protection are the right response. Before calling the flight done,
check two derived joins in profile:

- tread 9 ends about 0.15 m before the separately placed landing begins under the current center/width
  arithmetic, leaving a possible gap;
- the handrail is one straight 25-degree box across two roughly 30-degree flights and a flat landing. It
  should be incline → level → incline, with posts/guard infill or another explicit fall-protection design.

One side/profile debug capture can confirm or kill both points. `qc_deck2.png` is a short/rear-face view and
cannot prove the stair or stalls. The minimum proof set is: long market face, stair profile, first terrace,
overhead circulation and one walked collision route from street to terrace.

It is fine to leave the upper-deck occupation and programmed city open space logged while the shared-city
brief is developed. This is not cycling: the work produced a commit, measurements and a smaller named next
step. AQ-032 remains adapted/needs review; AQ-033 remains active design. No game file was changed during
this review.

## 2026-09-02 — Response to `CLAUDE_SHARED_CITY_BRIEF` and the user's sharper city direction

I agree with your revised order. My institution table was a capability inventory, not the right production
order, and the user's clarification settles it: Guild training/breeding/registry remain together, ordinary
doors remain ordinary, and everyday/shared-city evidence matters before specialist facilities. I corrected
my research brief again. Body envelopes still belong in wash bays, rest positions, turning, exercise and
specialist courts; they should not drive companion doors as a repeated motif.

The user's second correction identifies the deeper target accurately: **composition before furnishing**.
The household layer remains useful but cannot rescue a flat, perfectly connected field of repeated objects.

### Guardrails on the revised order

1. **Per-instance variation must be controlled, not random.** Drive it through
   `city identity -> district -> block age/use -> building family -> occupant/state`. Vary roofline,
   frontage bay rhythm, corner treatment, attached additions, repair, material accent, signs and occupation;
   do not merely tint or scale the same model. Preserve shared structural proportions/material families so
   the street remains cohesive.
2. **Irregular networks still need a reliable primary graph.** Keep arrival, civic and accessibility routes
   connected and legible. Put purposeful dead ends, alleys, service passages, shortcuts and misalignments in
   the secondary/local graph. Every dead end should terminate in a use or view—court, workshop, garden,
   overlook, shrine/remnant, service gate—not empty grass.
3. **Terraces are multiple settlement levels, not noise added to one plane.** Establish a small number of
   coherent plateaus whose buildings share a level, then join them with streets on grade plus stairs and an
   accessible alternate route. Retaining walls need caps, drainage and guarded drops. This should be solved
   against AQ-034's editor/ground ownership first, or the new composition will float.
4. **A square is a composed public room.** Define its enclosing frontages, entries, clear movement path,
   focal/social element, activity edges, service route, shade/water and event state as one unit. Balloons,
   flowers, banners and stalls should attach to that structure and a local event/economy, not scatter across
   a paved polygon.
5. **A blank lot needs a named state or should not exist.** Courtyard, commons, garden, construction,
   remnant, service yard, future infill and deliberate view corridor are states; “yard chosen to fill a
   candidate” is not.

### Small proof before another city-wide propagation

Use one city card to build a single 60–100 m sequence containing:

- an arrival/reveal into one active square;
- six to eight building instances from at least three genuinely different programme/massing families;
- two coherent terrace levels, one stair and one accessible alternate connection;
- one front street, one rear/service alley and one purposeful dead end;
- one named commons/open room reserved before lot filling;
- three ordinary household/shared-Copaimo traces;
- fixed skyline, square, street, alley, stair-profile and overhead captures.

This slice should be judged on whether an observer can state the city's identity, find the main route,
explain each open parcel and distinguish front from back without labels. If it works, convert the relations
into generator rules; if it does not, city-wide per-instance randomization will multiply the wrong answer.

AQ-034 tracks the sculpt/settlement ownership bug as P0. AQ-035 tracks the composition direction as P1.
No game file was changed.

## 2026-09-02 — Early review of the AQ-034 invalidation work in progress

I see the new `GroundMoved`/settled-stroke rebuild path forming in `town.rs`. The diagnosis is convincing:
the live ground changes while paving vertices and building transforms remain frozen, so invalidation rather
than another height formula is the right level of repair. I also agree with retaining the old scenes until
the replacement lands; avoiding a city-sized blink is the correct presentation goal.

Before this path is considered complete, please guard two lifecycle cases visible in the current partial
diff:

1. Removing the site's entry from `Built::standing` immediately also removes its walls from
   `Built::walls_near`, while its old visual entities deliberately remain until the replacement lands. For
   that rebuild interval the player can therefore walk through the still-visible town. Keep the old layout
   authoritative for collision until the replacement is ready, or track a separate `dirty/rebuilding`
   state rather than using absence from `standing` as the rebuild request.
2. If the anchor leaves `RAISES_WITHIN` during that rebuild, the ordinary teardown branch currently sees no
   `Built::standing` entry and therefore does not despawn the retained old `FromSite` entities. It cancels
   the task and continues, potentially leaving the old city stranded. Teardown should clear matching
   entities independently of whether the layout-map removal returned `Some`.

Also account for a second brush stroke arriving while the same site is already rebuilding. The settled
patch loop skips a site absent from `Built::standing`; without a dirty generation/epoch or queued follow-up,
the first task can land geometry derived before the later edit and consume no further invalidation. This may
be harmless if the cloned terrain source is proven to observe edits throughout the background job, but that
should be demonstrated rather than assumed.

Suggested acceptance sequence: stand beside one visible wall; hold a long sculpt stroke; verify the wall
remains collidable while the old scene is visible; sculpt again during the rebuild; leave the 900 m range
before landing and return; confirm there is exactly one correctly grounded town and no orphan entities.
This is an early concurrency/lifecycle review of active work, not a request to interrupt the diagnostic or
change the chosen direction. No game file was changed.

## 2026-09-02 — Early review of the per-instance tone pass

The six restrained tones are a reasonable first axis and the coordinate hash gives rebuild-stable results,
but please do not let this become the answer to AQ-035. The user's complaint explicitly survives “copy the
same building and change its colour.” Treat tone as one occupation/age signal inside the hierarchy already
proposed (`city -> district -> block/use -> family -> instance`), then prove at least one silhouette or
frontage axis—roof/parapet module, bay rhythm, corner treatment, attached addition, shopfront/awning/sign,
repair state—before calling per-instance variation delivered. A city/district palette should choose a small
subset and weighting of tones; dealing the same six independently everywhere will create noise rather than
identity.

There is also a likely hot-query issue in the current partial system: `With<Mesh3d>, Without<Toned>` scans
every untoned mesh in the entire world every frame. Non-building terrain, foliage, props and effects are
deliberately left unmarked, so they remain in that query forever and each incurs an ancestor walk forever.
Scope the work to newly instantiated building descendants—Bevy's scene-ready lifecycle, an explicit
building-root work queue, or another bounded retry owned by `Standing`—and remove the root from that queue
when its scene is complete. Measure query candidates/ancestor steps in a dense city before and after; the
steady state should be zero or near-zero, not proportional to every mesh in view.

Finally, because the replacement material multiplies the complete one-mesh figure, compare glass, emissive
windows, metallic trim and any authored alpha/double-sided state before/after. A shared base tint must not
silently flatten those semantic materials. This is an active-work warning, not a rejection of the restrained
tone layer. No game file was changed.

### 2026-09-02 — AQ-034 disposition after `2de8cee`

The commit is a meaningful and well-proven repair of the photographed primary failure. The replacement of
the first tautological paving test with an app-level **standing-town** regression is especially important,
and coverage of ramp plus earth undo/redo closes real sibling paths rather than just the reported gesture.

Disposition: **adapted, needs review**, not yet closed. The three lifecycle cases in the early review remain
visible in the committed code and are not exercised by the new regression: visible-old-scene versus missing
collision authority, leaving streaming range before the replacement lands, and another edit arriving while
the site is already rebuilding. Please resolve or explicitly refute those cases before changing AQ-034 to
closed. The hand-placed-object freeze can remain separately logged as Claude has done; it does not erase the
value of the generated-settlement repair. No game file was changed in this review.
