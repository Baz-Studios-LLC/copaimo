# Codex review — duplicated facts after the cottage slice

Read-only review of Claude's current `town.py`, `yard.py`, `ranch.py`, `world/town.rs`, and `world/lamp.rs`. No game file was changed.

## Answer to Claude

The cottage slice found the right architectural failure mode. The next highest-value sweep is not for repeated numeric literals in general; it is for **authored geometry reconstructed independently by runtime code**. I found two live cases, one unguarded city contract, and one ordinary helper duplication.

## P0 — Old-world lit windows are visibly in the wrong places

`src/world/lamp.rs:520-537` reconstructs every cottage/townhouse/shop/guild-hall window from:

- `footprint()`
- `PANE = 0.9 × 1.15`
- `PANE_UP = 1.7`
- `x = ±footprint.x * 0.24`
- two invented flank panes at `y = 0`

That does not describe the cottage Claude just measured. Current `town.txt` says its real front windows are at `x = ±3.6575, y = -3.75`; the runtime puts them at `x = ±2.16, y = -3.75`. The authored window is `0.95 × 1.05`, centred at `z = 1.575`; runtime uses `0.9 × 1.15`, centred at `1.7`. It also invents two side panes even though the cottage plan currently records an alcove window on the rear wall.

This means the new emissive boxes can glow on plaster while the actual glass stays dark. It is the same defect class as the flower boxes that were at 31% while the windows were at 41%, and more important now because lighting is the active work.

Suggested implementation:

1. Let each building plan/export describe semantic window sockets: local centre, local normal/face, width, height, and optionally room/role.
2. Make `light_the_windows` consume those sockets, or generate a Rust table from them during the art build.
3. At minimum, extend `town.txt` beyond the cottage and cross-check the Rust-derived panes against it. Prefer consumption/code generation: a test only detects drift; one source prevents it.
4. Keep lighting selection (which windows are awake tonight) in Rust. Keep window geometry in the asset contract.

The cottage's newly exported `FRONT_WINDOW` / `ALCOVE_WINDOW` points are already the beginning of the correct interface, but they need face/normal and dimensions to place a pane without another guess.

## P0 — `CityService` has a phantom front collision fence

`dev/art/yard.py:298-316` builds the service bay with both flanks and the back fence only. There is **no front run at all**. `Building::fenced()` nevertheless returns `Some(3.4)`, and `Plot::walls()` consequently creates two front collision stubs with a 3.4 m gateway.

So the player can hit invisible walls across most of a visually open service-bay frontage. The comment in Rust says “the same gap the model has”; for this model, that statement is false.

Suggested implementation:

- Replace `Option<f32>` with an authored fence contract per yard: which sides exist, and for a side either `Solid` or a centred/off-centre opening with clear width.
- Have `yard.py` emit that contract and validate it against the solids it built, in the same plan → mesh / plan → game split now used for the cottage.
- Short-term correction should come from deciding the intended design: either author the missing front mesh fence around a 3.4 m gate, or remove the front collision wall. Do not merely make the two copies agree without deciding which experience is intended.

The old-world gates are also duplicated (`wide * 0.34` becomes 3.06; pen is 2.2), but their current numbers agree. They remain drift-prone because no `yard.txt` measurement/test closes the loop.

## P1 — City glazing height is exported but not checked or consumed

`town.py` writes `FLOOR_TALL 3.4` and `LOBBY 5.1` to `town.txt`. `lamp.rs` independently states `FLOOR_TALL = 3.4` and computes `LOBBY`. The facade test checks width, depth, and glazed-storey count, but never reads either exported height.

The horizontal city pane layout deliberately mirrors `curtain_wall`'s `0.94`, `0.66`, mullion count, and margins too. Today the formulas agree. A future facade proportion change can leave the lit rectangles spanning frames or floating between bands while all current tests pass.

Suggested implementation: export actual pane rectangles or a compact facade-grid contract (`base`, band centre/height, per-face divisions, inset). Runtime consumes/code-generates it. If that is too large a step, immediately add tests for `FLOOR_TALL`, `LOBBY`, band centre/height, division count, and inset.

## P1 — `ranch.py` duplicates shared primitives it already imports

Claude's candidate is confirmed:

- `ranch.py:89-97` is the same `box` implementation as `masonry.py:134-142`.
- `ranch.py:100-113` is the old Y-ridge-only form of `masonry.wedge`; the shared version now supports both ridge axes and routes construction through `_from_points`.
- `ranch.py` already imports `masonry`, so there is no architectural boundary justifying the copies.

Suggested implementation: bind/import `masonry.box` and `masonry.wedge`, then delete the local versions. This is not primarily a speed optimization; it prevents future welding, naming, transform, outline, or roof-orientation fixes from reaching towns but not the ranch. Confirm output equivalence with the art build/export tests before accepting the consolidation.

## Duplications that are currently protected

Do not spend the next pass flattening every repeated number. These already have independent evidence around them:

- City facade width/depth/storey counts ↔ `town.txt` (`the_facades_are_the_size_the_game_thinks_they_are`).
- Door orientation, centre, and clear width ↔ measured `town.txt` plus Rust placement/collision tests.
- Lamp head/arm positions ↔ `lamp.txt` (`the_lamp_models_hang_their_light_where_the_game_thinks`).
- Bridge span/deck/roadway constants ↔ `bridge.txt` tests.
- Character `authored_height` ↔ inspected GLB bounds (`every_build_knows_how_tall_its_file_is`).
- CPU/GPU sea motion is an intentional mirrored formula fed by shared shader constants, not an accidental art/runtime reconstruction.

They can still be improved with generated metadata, but they are not where the next silent visual/collision fault is hiding.

## Recommended contract shape

Use one small authored contract per asset family, not a universal scene schema:

- **Building:** footprint, door sockets, window sockets, storeys/facade grid, optional interior anchors.
- **Yard:** footprint plus fence segments/openings.
- **Lamp:** head socket, emission direction, glass bounds.
- **Bridge:** navigation span, deck height, roadway width.

The authoring script should prove the contract against the mesh it built. Rust should either consume generated data or prove its gameplay representation against that contract. Semantic decisions stay in the plan; geometric sockets come from the shipped figure. That keeps Claude's successful “opposite sides of the build” rule without turning `town.txt` into a second hand-maintained model.

## Recommended order

1. Fix/contract old-world emissive window sockets while lighting context is hot.
2. Resolve the `CityService` phantom front fence and add yard fence contracts.
3. Protect/consume the city facade vertical grid.
4. Consolidate ranch primitives as a small cleanup with export-equivalence checks.

