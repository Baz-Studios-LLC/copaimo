# Stored City 01: dependency audit and re-bake merge research

**For:** Claude  
**From:** Codex  
**Date:** 2026-09-17  
**Scope:** read-only audit of the current tree plus source-backed production research. No game file was changed by Codex.

## Executive answer

Storing City 01 is the right decision. The safest production model is not “merge two anonymous lists by
position.” It is:

1. store a stable settlement ID;
2. give every generator-owned source row a stable semantic ID before it becomes geometry;
3. keep authored overrides and authored additions as a separate layer;
4. regenerate the base, then apply overrides by source ID;
5. preserve deletions as tombstones;
6. report a missing source as an orphan requiring an explicit keep/detach/delete decision;
7. derive all geometry, indices, spatial caches and runtime entities after composition.

The current draft gets the most important architectural split right—`ways`, `opens` and placed objects are
source data while streets/nodes/walls/stairs are rebuilt—but its 6 m proximity merge cannot carry a moved
edit forward. Once an authored building moves more than 6 m from the new generator's version, the old
generated building can return beside it. A way matched only by its first point has the same problem plus
false matches between roads sharing a mouth.

The production evidence is unusually consistent: shipped teams expose authored inputs to procedural tools,
replace generated outputs by stable name/ID, and provide an explicit “branch/unlink/bake” boundary when an
artist needs to own the result. I found no credible shipped-game source describing automatic three-way
merging of arbitrary generated objects by quantised position. Quantised position is useful as a migration
hint, not as canonical identity.

## P0 findings in the current draft

These are correctness issues, not optional architecture preferences.

### P0.1 — settlement file identity is an unstable vector index

- `src/world/bake.rs:197` names files `settlement_{key}.json` and says the key is an index in
  `config::SETTLEMENTS`.
- The key passed to `lay_the_site_out` is actually the index in `Settlements::sites`. The ranch is inserted
  first at `src/world/settle.rs:1245` before the thirteen configured settlements, so those indices are
  already offset from `config::SETTLEMENTS`.
- City 01 is selected by “nearest city to the ranch” around `src/world/settle.rs:1400`; inserting/reordering a
  site can make another row inherit the existing file.
- `FromSite(u32)` and `Built::standing: HashMap<u32, Layout>` (`src/world/town.rs:6504` and `:5936`) are fine
  transient runtime keys. They are not durable content identity.

**Required contract:** add a permanent string/UUID-like `SettlementId` to the authored configuration (for
example `ardwen_harbour_city`) and name the file from it. Keep the numeric index only as an in-process lookup.

### P0.2 — proximity is conflict suppression, not identity

- `src/world/bake.rs:146` carries authored rows and drops fresh rows whose anchor is within
  `AUTHORED_CLEARS`.
- Rows have no stable ID (`src/world/bake.rs:58`).
- Plots and lamps use their current position. Moving one more than 6 m loses the relationship to its source,
  so the fresh original reappears.
- Ways use only the first point as their anchor. Two roads can share a mouth and be different roads; one road
  can keep its identity while its first point moves.
- A fixed 6 m distance does not even prove non-overlap: a large authored guild hall and a generated building
  can overlap with centres farther apart, while unrelated small props within 6 m can suppress one another.

**Required contract:** generated rows need stable `source_id`s. An authored override carries the same
`source_id`; an authored addition has its own `authored_id` and no source. Use footprint/curve geometry for a
separate collision-resolution pass. Never use collision proximity as identity.

### P0.3 — deletion has no representation

Deleting a generated JSON row does not mean “keep this deleted”; the next re-bake regenerates it. Provenance
needs a third persistent operation: a tombstone keyed by `source_id` (or an explicit `Suppressed` override).
Without it, hand deletions cannot survive the first generator improvement.

### P0.4 — `Place.id` and `Plot.serves` persist a list index

- `Place.id` is documented as its own layout index at `src/world/town.rs:2819` and is assigned from
  `opens.len()` around `:4835`.
- `Plot.serves: Option<usize>` at `src/world/town.rs:1450` stores that value; assignment sites include
  `:5101`, `:5220`, and `:5237`.
- The relationship is consumed today by layout validation (`:14101`) and is intended as a future NPC
  destination.
- Reordering/removing/inserting `opens` during editing or re-bake can silently attach market furniture to a
  different place while all coordinates remain valid.

**Required contract:** give each place a stable ID and make `serves` a foreign key to it. Do not serialize
“my index in this Vec.” Validate uniqueness and referential integrity on load. If list indices remain useful
at runtime, derive an ID→index map after composition.

### P0.5 — the file stamp cannot detect the inputs that materially change generation

`Stamp` at `src/world/bake.rs:80` records only `at`, `radius`, and `seed`; `stored()` does not compare even
those fields before loading. Layout also depends on at least:

- settlement ID and schema version;
- generator algorithm/version;
- `Site::bearing`, `shape`, `plan`, `era`, `character`, `city`, and `first`;
- roads crossing the settlement from `roads_through`;
- constants and building footprints used for lots, clearance and thinning;
- coastline/terrace facts for derived walls and stairs.

The existing `version` is a file-schema version, not a generator version, and `stored()` currently accepts
any numeric value it can deserialize. Record a generator fingerprint and the relevant input fingerprint.
Unknown schema versions need an explicit migration/rejection path. A stale layout may still be intentionally
loadable, but it must be reported as stale before a re-bake—not silently described as current.

### P0.6 — malformed/missing stored City 01 silently becomes a different generated city

`stored()` logs malformed JSON and returns `None`, after which `lay_the_site_out` generates. That is a useful
development fallback for an optional bake, but dangerous once saves, quests or authored references target
stable city objects. Shipping City 01 should fail closed (or visibly quarantine the city) on a malformed
required asset. Otherwise the player can load a structurally different city with no hard failure.

## What is source data and what must remain derived

| Data | Current owner | Stored? | Reason / re-derivation |
|---|---|---:|---|
| Settlement identity | `Site` / config | **Yes, stable ID** | Durable file and reference key; never vector position. |
| Ways: points, width, role, join width | `Layout::ways` | **Yes** | Primary authored street intent. Each way needs stable ID. |
| Public places: programme, transform, extent | `Layout::opens` | **Yes** | Primary authored city-room intent. Replace `Place.id` index with stable ID. |
| Plots: transform, building kind, district, place relationship | `Layout::plots` | **Yes** | Primary authored placement. Each plot needs stable ID/source ID; `serves` uses stable place ID. |
| Lamp overrides/additions | currently `Layout::lamps` | **Only overrides/additions** | Default lamps are derived by `light_the_streets` at `src/world/town.rs:2483` from streets and plots. Storing the full generated lamp list freezes stale lamps when a way/plot is hand-edited without a re-bake. |
| Street segments | `Layout::streets` | **No** | Derived from way chains. |
| Junction nodes/mouths/rings | `Layout::nodes` | **No** | Derived by `network()` at `src/world/town.rs:8525`; depends on all current ways and paving rules. |
| Retaining walls and stairs | `Layout::walls`, `stairs` | **No** | Derived by `retain_the_terraces` at `src/world/town.rs:2333`; depends on current streets, crossings, site/terrain rules. |
| Town levelling lanes | `Settlements::lanes` | **No** | Rebuilt from `layout.streets` in `lay_the_town_out`, `src/world/settle.rs:1540`. |
| Building pads | `Settlements::pads` | **No** | Rebuilt from plot footprint/facing in the same function. |
| Spatial cells | `Settlements::cells` | **No** | Index over sites/lanes/pads, rebuilt by `Settlements::index`. |
| Paving mesh | `pave_while` | **No** | Built asynchronously from ways/nodes/opens at `src/world/town.rs:9688`. |
| Scene entities and `FromSite` | `raise_the_towns` | **No** | Streaming result; derived from current layout and site key. |
| Building floor/step contract | `FLOORS` / `TOWN_CONTRACT` | **No** | Asset-model contract parsed at runtime around `src/world/town.rs:6162`; not city content. |
| Foundations/footings and wall collision | `under`, `walls_near` | **No** | Derived from terrain, footprints and current walls. |
| Dock placement/deck surfaces | `moor_the_harbour`, `Built::docks` | **No for current design** | Derived from finished terrain and the first site's coast. If the harbour is later hand-authored, promote a separate harbour source record; do not smuggle runtime `Built` data into the layout file. |
| Terrain coastline field, harbour bearing, terrace height | `Site`/`Settlements` | **No** | World-plan facts derived before layout. A stored city consumes them; it should not duplicate them. |

### Important correction to the current `finish()` split

The new `finish()` at `src/world/town.rs:4096` correctly rebuilds ways/nodes/streets/walls/stairs, but it
leaves `laid.lamps` untouched. In generated layouts, lamps come from `light_the_streets(streets, plots,
city)` near `src/world/town.rs:5600`. A directly edited stored way can therefore move while its generated
lamps stay on the old kerb. Either derive all default lamps in `finish()` and apply stable authored lamp
overrides afterward, or explicitly declare every stored lamp authored and require the editor to move them.
The first model is safer and matches the stated “derived data is never baked” rule.

Also, the generator still performs its own network/wall/lamp finishing inside `lay_out`. Keep one finishing
function for generated and stored layouts or this will become a two-derivations drift point.

## Caller audit: what assumes generation, purity or repeatability

Line numbers refer to the working tree observed on 2026-09-17 and may move as the active branch changes.

### Runtime-critical callers

1. **Terrain planning / levelling** — `src/world/settle.rs:1540–1562`  
   `Settlements::lay_the_town_out` calls `lay_the_site_out` for every site and derives lanes and pads. This is
   the earliest and most important consumer: terrain heights, biome suppression and later meshes assume its
   answer is the same layout that will be spawned. The stored snapshot must be available before
   `Terrain::new` finishes.

2. **Streaming and paving** — `src/world/town.rs:9687–9696`  
   An async compute task calls the same entry point, then paves ways/nodes/opens. It assumes the call is pure,
   thread-safe, repeatable and cancellable around the expensive mesh step. Any hot reload after terrain
   creation would make render/collision/levelling disagree unless the entire terrain plan and affected
   stream caches are invalidated together.

3. **Street lamps** — `src/world/lamp.rs:340`  
   The lamp system consumes `layout.lamps` separately from town entity creation, under the same site key.
   It requires the same immutable layout snapshot; it exposes stale stored lamps immediately.

4. **World audit** — `src/audit.rs:119`  
   Regenerates/loads each layout to compare roads, buildings and props. It assumes deterministic equality
   with terrain claims and spawned content. It should audit the composed stored layout, which is valuable,
   but tests/tools need a deterministic asset root.

5. **Movement driver** — `src/drive.rs:320` and `:353`  
   Finds real terrace stairs/walls and city kerbs from layout data. If finishing is incomplete, automated
   movement validates a different world than the player sees.

6. **Photo matrix** — `src/photo.rs:440–450`  
   Finds the first guild hall from `layout.plots`. An authored deletion/rename can make a planned shot vanish;
   that should be a clear missing-landmark result, not a fallback to a generated city.

### Test/debug callers

- `src/world/town.rs:6270` (`a_paved_street`) and the calls around `:11348`, `:11440`, `:11548`, `:11832`,
  `:11895`, `:12002`, `:12187`, `:12532`, `:12797`, and `:15020` regenerate real-world layouts in guards,
  reports and audits.
- Direct `lay_out` tests with fabricated `Site`s intentionally bypass storage and remain generator tests.
- Whole-world tests that construct `Terrain::new` will now consume City 01's file. Their result depends on
  the process working directory because `path_of` is relative. A test launched from a different directory
  can silently exercise generated City 01 instead of stored City 01.

### Availability and threading conclusion

The `OnceLock<HashMap<usize, Baked>>` at `src/world/bake.rs:210` gives one thread-safe immutable snapshot per
process, which preserves agreement among terrain, streaming, audit, driver and photos after the first call.
The tradeoffs need to be explicit:

- the first caller performs blocking filesystem IO, potentially from a worker;
- relative `assets/world/...` lookup depends on current working directory and packaging layout;
- `write()` calls `stored()` and then writes, but cannot update the initialized `OnceLock`; the current
  process continues using the old/absent version until restart;
- a malformed file is cached as absent for the whole process;
- live editing requires a versioned `Arc<StoreSnapshot>` plus coordinated terrain/stream invalidation, not
  a mutable global file read. If live editing is not a requirement, document “restart after bake/edit” and
  make the bake tool a separate one-shot process.

## Index and reference stability audit

### Persisted references that must change

- settlement numeric key → permanent `SettlementId`;
- `Place.id: usize` → stable `PlaceId`;
- `Plot.serves: Option<usize>` → `Option<PlaceId>`;
- every way/plot/place/lamp row → stable row ID plus optional generator `source_id`.

### Runtime-only indices that are safe if always rebuilt

- `Lane.site: u16` (`src/world/settle.rs:239`) indexes the current site vector;
- feature indices in `Settlements::cells` index the current sites/roads/lanes/pads concatenation;
- `FromSite(u32)`, `Built::standing`, and `Raising::working` key streamed instances;
- node and street vector indices inside network/paving internals.

These are safe only because they are derived afresh from one composed snapshot and never serialized or
referenced by a save/quest. Do not let the editor expose them as durable IDs.

## Recommended re-bake data model

This is a contract, not an implementation demand:

```text
SettlementFile
  schema_version
  settlement_id
  base_generator_version
  base_input_fingerprint
  authored_additions[]       // permanent authored_id, complete record
  overrides[]                // source_id + changed fields (or complete replacement)
  suppressions[]             // source_id tombstones

GeneratedRow
  source_id                  // stable semantic ID from generator construction path
  kind
  data
```

### Source ID design

Prefer a semantic construction path generated before world transforms, then hash/store it as text or 128
bits. Examples:

- `market/main`;
- `way/high_street`;
- `way/ring/02/arc/05`;
- `place/park/01`;
- `lot/ring/02/spoke/05/frontage/03`.

The exact vocabulary can change, but the rule is that unrelated insertion must not renumber existing IDs.
Do not use vector index, PRNG consumption order, current position, or the first point of a polyline.

For generator changes that intentionally replace a semantic entity, ship a small ID migration map
(`old_source_id -> new_source_id`) with the generator version. This is reviewable in source control and far
safer than a global nearest-neighbour guess.

### Re-bake algorithm

1. Generate a fresh base keyed by `source_id`; reject duplicate IDs.
2. Apply explicit ID migrations for this generator version.
3. Apply suppressions by `source_id`.
4. Apply overrides to matching sources.
5. Add independent authored additions.
6. For an override whose source no longer exists, emit an **orphan conflict**.
7. Validate references and geometry: place foreign keys, way shape, finite numbers, duplicate IDs, building
   footprints, road/building/open-ground intersections and world bounds.
8. Derive streets, nodes, default lamps, walls, stairs, lanes, pads, cells, paving and runtime entities.
9. Write a merge report and only then replace the file atomically.

### Orphan policy

Do not silently delete an authored edit and do not silently attach it to the nearest new object. Default to:

- preserve the authored record in an `orphaned` section (or keep it enabled if it is independently valid);
- fail/flag the re-bake with the old source ID and reason;
- offer explicit resolutions: **detach as authored addition**, **retarget to source ID**, or **delete**;
- block shipping if an orphan has broken foreign keys or overlaps critical traversal.

This mirrors the real production boundary exposed by procedural tools: linked outputs are replaced by the
generator; intentionally independent work is unlinked/branched and becomes authored content.

### Quantised position

Use it only for a one-time migration assistant when adding IDs to old files. Candidate matching should also
consider type, dimensions, orientation, topology/adjacency and parent place—not position alone. If the best
and second-best candidates are close in score, report ambiguity instead of guessing. Once a human confirms
a match, write the permanent source ID and never repeat the spatial inference.

## What shipped practice actually supports

I did not find evidence that AAA teams routinely merge arbitrary edited procedural output rows back onto a
new generation using quantised position. The documented practices point elsewhere:

1. **Keep art direction in persistent inputs; regenerate disposable outputs.** Ubisoft's Far Cry 5 team
   integrated Houdini tools into its editor so game artists could craft water, power lines, cliffs, biomes,
   fog and maps through the procedural tools. That is artist-authored control data feeding regeneration,
   not hand-edited anonymous outputs being spatially merged.  
   Source: [SideFX — Far Cry 5](https://www.sidefx.com/community/far-cry-5/).

2. **Make every result handcraftable and permit a deliberate branch.** Insomniac's *Marvel's Spider-Man 2*
   procedural-tool talk explicitly frames production use by completion level—greybox, iterate to final or
   polish, and “branch from proceduralism”—and describes environment artists replacing greybox building
   prefabs. That is strong shipped evidence for an explicit ownership handoff rather than an invisible merge.  
   Source: [GDC 2024 — Procedural Tool Development for Marvel's Spider-Man 2](https://media.gdcvault.com/gdc2024/Slides/GDC%2Bslide%2Bpresentations/Santiago_David_ProceduralToolDev_with_notes.pdf).

3. **Replace generated output by stable name/identifier.** Houdini Engine's official Unreal API replaces a
   previous bake when names match, and can bake one output by output index plus identifier. It does not claim
   to infer identity from proximity.  
   Source: [SideFX — Houdini Engine Unreal Public API](https://www.sidefx.com/docs/houdini/unreal/publicapi.html).

4. **Use explicit IDs because element numbers change.** Houdini's geometry documentation defines `id` as a
   unique element ID used to track elements even when point counts/numbers change; `name` is used to find
   primitives by name. This is the closest direct technical precedent for Copaimo rows.  
   Source: [SideFX — Geometry attributes](https://www.sidefx.com/docs/houdini/model/attributes.html).

5. **Linked generated output is replaceable; detached output is authored.** Houdini Engine describes baked
   output as independent, offers “Unlink Bake,” deletes linked baked PCG actors/assets on regeneration, and
   says clearing the PCG link prevents that deletion. This is a concrete orphan/ownership model: stay linked
   and accept replacement, or explicitly detach and own the result.  
   Source: [SideFX — Houdini Engine details panel](https://www.sidefx.com/docs/houdini/unreal/detailspanel.html).

6. **Final baking commonly ends procedural participation.** SideFX's Unreal packaging documentation says
   Bake and Replace can remove Houdini actors and that modifying procedural content again then requires
   manually recreating the Houdini assets. So the honest “many teams bake once/finalize” answer is partly
   true: robust tools support iterative recooks, but final detached content is often no longer auto-merged.  
   Source: [SideFX — Packaging Houdini assets for Unreal](https://www.sidefx.com/docs/houdini/unreal/packaging.html).

7. **AAA hybrid worlds still preserve manual layers.** Ubisoft's Ghost Recon Wildlands material describes
   procedural roads/cities/terrain while noting that some decals could also be added manually. The scalable
   pattern is separate generated and manual layers, not pretending they are one provenance-free list.  
   Source: [SideFX — Ghost Recon Wildlands production articles](https://www.sidefx.com/community/80-level-ghost-recon-wildlands/).

## Acceptance checklist for City 01 storage

- [ ] City file is addressed by permanent settlement ID, not site/config index.
- [ ] Every stored row has a unique stable ID; generated rows also have stable `source_id`.
- [ ] Moving an authored plot 50 m and re-baking does not respawn its generated original.
- [ ] Deleting a generated row creates a tombstone and it stays gone after re-bake.
- [ ] Inserting an unrelated generated lot does not change existing source IDs.
- [ ] Reordering JSON arrays changes nothing.
- [ ] Removing a source with an attached override emits an orphan conflict; no silent deletion/reattachment.
- [ ] `Plot.serves` resolves a stable `PlaceId`; invalid references fail validation.
- [ ] Editing a way re-derives streets, nodes, paving, default lamps, terrain lanes, walls and stairs.
- [ ] Editing a plot re-derives its pad, grounding/footing, walls/collision and any default lighting relation.
- [ ] Runtime terrain, streamed town, lamps, audit, drive and photo all consume one immutable store snapshot.
- [ ] Malformed/unknown-version required City 01 data cannot silently become generated content in shipping.
- [ ] Asset lookup is independent of process working directory and works in the packaged build.
- [ ] Re-bake writes a diff/report, validates, and atomically replaces the file only on success.
- [ ] A same-process bake either refreshes a versioned snapshot and rebuilds dependents or explicitly requires restart.

## Disposition

- Stored City 01 direction: **accepted**.
- “Derived data is never baked”: **accepted, with default lamps added to the derived set**.
- Provenance from the first version: **accepted, but needs stable IDs, tombstones and orphan policy**.
- Quantised-position identity: **rejected as canonical identity; accepted only as migration assistance**.
- Current 6 m proximity merge: **P0 needs redesign before authored edits are trusted**.
- Current vector-index settlement/place references: **P0 needs stable IDs**.

