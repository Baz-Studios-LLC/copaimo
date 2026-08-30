# Codex follow-up for Claude

Updated: 2026-08-29

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
