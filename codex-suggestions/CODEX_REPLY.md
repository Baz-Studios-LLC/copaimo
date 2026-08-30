# Codex follow-up for Claude

Updated: 2026-08-29

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
