# Copaimo AAA roadmap and active suggestion ledger

**Date:** 2026-08-31  
**Purpose:** one short queue shared by Claude, Codex and the user.  
**Rule:** a suggestion does not have to be implemented, but it should receive an explicit disposition.

## Dispositions

- **accepted** — Claude agrees; implementation/evidence may still be pending.
- **adapted** — Claude is pursuing the intent through a different implementation.
- **deferred** — valid, intentionally scheduled later, with the gate that reopens it.
- **rejected** — considered and declined, with a reason.
- **needs review** — no decision yet or evidence is incomplete.
- **closed** — implemented and sufficiently proven for the stated contract.

“In progress” belongs in the note, not in place of a disposition. A reasoned deferral/rejection is not
resurfaced without new evidence. An unacknowledged P0/P1 should be raised after two related commits or one
active workday, then left alone once Claude records a disposition.

## Active ledger

| ID | Pri | Area | Suggestion / current evidence | Disposition | Next proof or reopening gate |
|---|---|---|---|---|---|
| AQ-001 | P0 | Test evidence | Make `--drive` arrivals and blockers semantic. Current arrival is a 1.2 m radius; any 0.75 s no-progress or timeout passes `Blocked`, even at the wrong obstacle. Claude accepts the limitation and now qualifies 33/33 claims. | accepted | Add finish regions/intended-barrier contact bands when scheduled; until then retain the explicit radius/stall qualification. |
| AQ-002 | P1 | Junction topology | `0516940` keeps ways fixed and merges swallowed-edge meetings within 6 m, reducing overlap samples 24,755→541. One pair remains; merged fan nodes allow 16 cm flat-surface disagreement. Its “busiest member” choice currently reads zero-valued `Arm::mouth` before framing, so fan origin is order-dependent. | adapted | Store original member degree and test an unequal-degree merge; then polygon fallback (or equivalent representation) must remove the remaining overlap and merged-fan surface exception before closure. |
| AQ-003 | P1 | Road surface | Two faults were confused in the 7 cm. `drawn_height` claimed a bilinear patch while `build_chunk` draws two triangles per quad, so roads and feet agreed with each other and both stood off the ground actually drawn - fixed in `6e97b0e`. Measured before and after, that moved the worst chord sag 7.09 cm -> 7.07, so it was NOT the drape; reported as a null result rather than claimed. The rest is chord sag over the levelling skirt, and node rim steps are already 1.2 m against a 2 m grid, so uniform refinement is not the answer. | accepted, cause now named | Conform the ribbon to the heightfield: subdivide where a segment crosses the grid and refine by curvature, not distance. Not started - it is ribbon-builder work, not a constant. |
| AQ-004 | P1 | Junction materials | Add a fixture where `Arriving` differs across a node’s mouths. Uniform paved/unpaved fixtures cannot prove the gateway case. | needs review | Named mixed gateway node test and eye-level/oblique stills. |
| AQ-005 | P1 | Road materials | Use road-relative longitudinal/across coordinates through ribbons and junctions, not world-aligned paving. | closed | Implemented in `2225578`; straight/node/grazing regression views remain useful but do not reopen the coordinate contract without evidence. |
| AQ-006 | P1 | Temporal quality | Filter procedural paving as its feature size approaches a pixel. | closed | Derivative-aware joint width and distance fade implemented in `2225578`; moving-camera validation is tracked once under AQ-009. |
| AQ-007 | P1 | Performance | Name target/minimum hardware, resolutions, presentation targets and frame/memory/startup budgets. | needs review | User/Claude decision; first optimized baseline on named hardware. |
| AQ-008 | P1 | Camera | Follow camera clears terrain only; add solid-geometry occlusion/collision behavior before interiors/tight streets are final. | needs review | Exterior wall, tree, canyon and interior orbit clips at min/max distance. |
| AQ-009 | P1 | Ink | `4137511` adds the missing internal-crease layer from reconstructed depth normals while retaining stronger silhouette ink. Architecture accepted; rendered temporal/resolution acceptance remains open. The crease path samples only MSAA sample zero although displayed colour is resolved, so thin trim/foliage coverage needs motion proof. | accepted, needs visual review | Slow pan across cottage timber/window trim and foliage with MSAA off/4x at target resolutions/frame rates; compare GPU frame time corners on/off and check crawling, one-frame loss, terrain-facet noise and line hierarchy. |
| AQ-010 | P1 | CI | Add routine push/PR format/lint/test and feature-matrix checks; test the player feature set that release actually ships. | needs review | Reopen after current shader/junction churn; ordinary CI green on Windows/macOS where practical. |
| AQ-011 | P2 | LOD/HLOD | Add screen-size-driven vegetation, settlement and horizon representation families after profiling. | deferred | Reopen once AQ-007 baseline shows distance/draw-call/stream cost and final silhouettes are stable. |
| AQ-012 | P2 | Settings/input | Introduce action-level input, gamepad/remapping, sensitivity/invert/toggle and accessible UI/motion settings. | deferred | Reopen at vertical-slice start, before combat/interact inputs proliferate. |
| AQ-013 | P2 | Audio | Define buses/events/zones, surface footsteps and information-bearing cue/caption policy. | deferred | Reopen when the vertical slice has stable interactions and locations. |
| AQ-014 | P2 | Documentation | Replace contradictory status claims with one short dated project-state snapshot; preserve long docs as design/history. | needs review | Snapshot says what exists, what is active and what is deliberately absent; link it from normal handoff. |
| AQ-015 | P2 | Maintainability | Split `world/town.rs` by contracts—graph/profile, layout, surface/render, traversal/collision, fixtures/tests—without redesigning behavior. | deferred | Reopen after AQ-002–006 stabilize and the existing tests can protect mechanical extraction. |
| AQ-016 | P2 | Settlements | Expand controlled modular variation and lived-in programs, not raw unique-building count. | deferred | Reopen after Gate F; prove one village and one city block at player height. |
| AQ-017 | P2 | Interiors | Finish only ranch, guild hall and one ordinary shop/home first, including camera, circulation, lighting, interaction and audio purpose. | deferred | Reopen for Gate V; full enter/use/save/exit evidence. |
| AQ-018 | P2 | Vertical slice | Build one final-quality 10–15 minute ranch→route→settlement purpose→return loop before scaling world-wide gameplay/content. | deferred | Reopen when Gate F is achieved or the user explicitly changes phase. |
| AQ-019 | P2 | Packaging | Ship from an explicit asset manifest and check duplicate/unused large assets; `assets/` is currently ~128 MB and is copied wholesale. | needs review | Package inventory and loaded-vs-shipped report before major content growth. |
| AQ-020 | P1 | Clouds | Claude accepts that the imported unlit flag did nothing, but adapted the visible fix at material level: zero reflectance, full roughness and emissive cloud colour, with noon/dusk photographs. The shared shader's ignored `unlit` flag remains a latent trap. | adapted | Keep the material fix; add a focused unlit-material fixture before another shared-shader user relies on that flag. |
| AQ-021 | P0 | Ink regression | The committed `much * 0.0` disabled every screen-space outline for five commits. `b9a75c4` restores `much` and adds a source-level smoke alarm for the exact literal-zero failure. | closed | Temporal/rendered coverage remains tracked once under AQ-009; reopen only if the pass can again produce zero coverage without failing evidence. |
| AQ-022 | P1 | Road handoff | Town/country meshes now have good single ownership, but both new `Way`s restart longitudinal UV at zero, so running-bond paving can phase-jump at the fully paved boundary. Claude accepts this and leaves it open. | accepted | Preserve phase from the original unsplit road and capture a close grazing boundary shot; add direct circle-split/handoff tests. |
| AQ-023 | P1 | Kerb continuity | `b9a75c4` makes junction stations mirror ribbon semantics: cobbled road to the face foot, dark face, then kerb top. This removes the node's broad painted gradient and collapsing stone-size band. | closed | An oblique junction still remains useful evidence, but the material/station contract is corrected; reopen on a measured mouth discontinuity. |
| AQ-024 | P1 | Authored kerb ink | Closed for implementation correctness. Non-road UV1 contamination is guarded by road-only specialization and two tests; depth+normal+motion prepasses compile and render together on the real camera; the real 0.35→0.75 approach test reads the shader's vertex attribute and proves red/green (38 false-line vertices with `stone_contrast`, zero with `kerb_stands`). | closed | Moving multi-resolution appearance and primary-kerb/outer-footway hierarchy are visual acceptance, tracked under AQ-009 and best decided with user-facing comparison pictures. Reopen AQ-024 only for a routing/eligibility/prepass regression. |
| AQ-025 | P1 | Player traversal | User has confirmed the hoverboard is happening and is explicitly not today's work. Accepted whole — the spec is not being shrunk. But a rideable board is a second MOVEMENT MODE, not a prop: it inherits the walk/jog tuning, the camera and the collision contract, and will reopen all three if it lands before they settle. Its own closing conditions (multi-point collision, invalid-surface recovery) also read straight onto AQ-003's ~7 cm drape — a plank held rigid at speed shows what a foot forgives. | accepted, deferred | After AQ-003 closes and the vertical slice's movement is settled. Raise it if either gate closes and nothing has moved on the board. |
| AQ-026 | P1 | Town streaming performance | You were right to reopen it: the cap was the pool's whole width, and the pool is shared with chunks, cover, props, the map and the country roads. `--flyby` now reports the GROUND's wait, which is the cross-family number you asked for, and it measured the starvation plainly - chunks waiting a 95th of 615 ms with the frame rate perfect throughout. Letting towns take fewer workers walks it down monotonically: 615 ms at full width, 602 at all-but-one, 509 at half, 322 at a quarter. Towns now take a quarter, never less than one. With the paving speedup on top (`a6da67f`), the ground's 95th is 274 ms and six of seven towns land before the player leaves them where five of eight did. | closed pending your review | Cancellation latency and CPU saturation are still inferred rather than named numbers; say if you want them reported directly. |
| AQ-027 | P1 | Road handoff | `0108853` + `ddb3658` correctly unify visible clipping and gateway fade on `Plan::off`: worst network gap 129.4→0.9 m across 25 arrivals and material jump 1.00→0.00. Review found the fifth caller the row anticipated: `stands_on` still suppresses the analytical country-road surface anywhere inside the old circular `town_reaches`, including the newly drawn road inside that circle but outside a narrow Grid/Spine shape. The 4 m sign-sampling clip can also miss a short enter+exit grazing chord whose sampled endpoints are both outside. | needs review (closure withheld) | Replace traversal's circular owner test with the same signed `off_the_town` authority and prove drawn-vs-walked height through a secondary Spine/Grid handoff. Add a sub-4 m grazing-intersection fixture or adaptive/SDF-minimum interval check. Then AQ-027 can close; AQ-022 phase continuity remains separate. |
| AQ-028 | P1 | Visual target/detail | The user explicitly prioritizes AAA visuals and detail. The repo is texture-light and already has strong generated geometry/shared shading; the highest-return next target is a controlled hybrid detail layer proven on one representative route rather than broad asset-count expansion. | needs review | Name a ranch→road→settlement→interior golden route and six locked views; choose the geometry/vertex-colour versus hybrid material boundary; make one identical-camera facade/frontage A/B. May defer behind AQ-003/AQ-009, but record the decision. |
| AQ-029 | P1 | Building/prop QC | Adapted strongly in `b8bbf1e`: townhouse stairs are now eighteen 0.20 m risers at a 36-degree pitch, a shared `stair_well` cuts the upper slab, and a sloped rail/newels were added. The visible stair still invites traversal onto an upper floor with no collision and the open landing still lacks a guard. `4137511` also replaces each camera-sized old-world 1.62 m door leaf with a plausible pair of roughly 0.87 m leaves; construction logic accepted pending the comparison view. The 0.34/0.26 m city-yard kerbs remain unreviewed. | adapted, needs review | Either make the upper floor/stair honestly traversable with landing guard, collision and camera proof, or visibly block the scenic stair. Then make the warden-to-double-door sheet and traverse/profile both yard kerbs. |
| AQ-030 | P1 | Junction mouth topology | User-reported grass wedges/ragged tears repeat at city junction mouths. `Node::new` constructs each band in real perimeter order, then `edges` sorts its vertices by polar bearing before `reach_of` treats neighbours as segments. This invents chords at non-bearing-monotonic mouth/return handovers despite `reach_of`'s own warning. | needs review | Preserve ordered band adjacency for ray intersections; sort only a separate scalar-bearing fan list. Prove with a non-monotonic return fixture, mouth-point containment checks and the same oblique node/arm overlay. |
| AQ-031 | P1 | Spatial index | Closed in `60cf564`: `settle::CELL` sized the index AND was `approach()`'s semantic search radius. `CELL = 64` and `APPROACH_WITHIN = 512` restore the original plan while cutting worst-city paving 1,719→297 ms. Both previously failing guards pass, with audit clean, bot 33/33 and 365 tests. | closed | No two-resolution refactor/test required before moving on; the named split, restored high-level guards and measured result are sufficient. Reopen if index size/filing semantics change or indexed-vs-brute evidence diverges. |
| AQ-032 | P1 | Pedestrian city coherence | Adapted in `60cf564`: population cap 420→280, vehicle ramp removed, stair revised, banner rods and first-terrace guards added. The claimed ground arcade is not visually/structurally open: `_street_storey` builds four perimeter walls while counters/awnings sit inside them. Upper decks remain empty and visibly inaccessible; `qc_deck2.png` does not show the stair or market face. The aerial still has broad accidental lawn rather than named open-space parcels. | adapted, needs review | Open real stall bays instead of enclosing the “arcade”; prove street face, full stair/landing/rails, first terrace and traversal from fixed cameras. Then add companion water/rest and service logic. Reserve programmed commons before lot filling rather than tuning population alone. |
| AQ-033 | P1 | Original shared-city identity | Pokémon research suggests memorable settlements commit to one civic idea across geography, plan, landmark, economy, public life, palette and gameplay; strong theming becomes hollow when repeated streets, false fronts and static crowds lack social/physical follow-through. User clarification: the Guild retains training, breeding and registry, while ordinary citizens also live with Copaimo. | adapted, needs review | Keep Guild duties in one campus; design everyday clinic, grooming, provisions, outfitting, boarding, housing thresholds, public water/rest and commons for all residents. Add original text-only city cards above Plan × Character and prove the first on AQ-032's Trade slice. Full brief/originality firewall: `POKEMON_CITY_DESIGN_INSPIRATION_FOR_COPAIMO_2026-09-02.md`. |
| AQ-034 | P0 | Editor/settlement sculpt coherence | Adapted strongly in `2de8cee`: `GroundMoved` now invalidates generated settlements after sculpt/ramp/earth undo/redo, clears the country-road cache and retains old scenes until replacement paving lands. The corrected app-level regression proves a standing town follows a 39.8 m ground edit; 366 tests, audit clean, bot 33/33. Closure is withheld because the old visible town loses `Built::walls_near` collision during rebuild, leaving range while rebuilding can skip despawning retained entities, and a second edit while the site is absent from `Built::standing` may not queue a follow-up. Hand-placed objects are explicitly logged separately. | adapted, needs review | Keep old visible geometry and collision authoritative together until replacement lands; make off-range teardown independent of layout-map presence; prove a second stroke during rebuild cannot land stale geometry. Acceptance: collide with a visible wall during rebuild, edit twice, leave range before landing, return to exactly one grounded/collidable town with no orphan entities. |
| AQ-035 | P1 | City personality and composition | User's sharper city brief: buildings remain copy-paste at different sizes, filler lots dominate, squares lack composed shops/decor/flowers/stalls, networks are too perfect, and every city is flattened despite desired hills, stairs, plateaus, alleys and meaningful dead ends. | needs review | Build one city-card-driven 60–100 m slice across two terraces: controlled per-instance building variation, composed active square, service alley/back lane, one purposeful dead end, programmed open ground and ordinary human–Copaimo household evidence. Preserve a connected primary graph and accessible route; irregularity belongs mainly in secondary/local paths. |

## Recently closed or explicitly settled

| ID | Area | Disposition/evidence |
|---|---|---|
| AQ-C01 | Ink sky/planarity/MSAA/width | closed in `1ddd439`; Claude measured line coverage and took noon/dusk/city/canyon images. Temporal sweep remains AQ-009. |
| AQ-C02 | Generic inverted hulls | closed in `86639c2`/`8bce408`; retained only on warden/authored landmarks, affected geometry roughly halved. |
| AQ-C03 | Tree lollipop silhouette | closed for the stated silhouette fault in `c8a2aea`, with five-species orthographic/silhouette sheet. Near-camera facet quality remains an art review, not a reopened silhouette bug. |
| AQ-C04 | Junction node-profile interpolation | closed in `17405ff`, with a 19 mm flat-region ceiling guard. This does not close AQ-003 terrain drape. |
| AQ-C05 | Ink alpha over water/river/glazing | closed in `95dfba3`; authored alpha is multiplied by the ink mask and blended no-ink misuse is guarded. |
| AQ-C06 | Longitudinal junction normals and road wear/grade | closed for the stated commit scope in `a1e5937`; road-relative paving and temporal filtering are separate AQ-005/AQ-006 items. |
| AQ-C07 | Village lane visual width / ribbon topology | closed in `adef3ef`; emitted lane count and hard splits now come from `cross_section` output, zero reversed faces remain, and an extra colour-only skirt station makes the 4 m lane read at its intended width without changing geometry or traversal. |

## Roadmap

### Stage 0 — finish the active close-node contraction

Do not interrupt the current connected-edge contraction with a broad town-file refactor.

Exit criteria:

- current dirty `town.rs` work becomes a reviewable commit;
- only swallowed connected edges contract, while nearby unconnected roads are reported and preserved;
- cascade passes are bounded and no valid route disappears;
- node sheet/audit/semantic traversal evidence names what each proves.

### Stage 1 — close foundation contracts (Gate F)

Recommended order:

1. AQ-001 honest driver semantics;
2. AQ-002 connected close-node contraction;
3. AQ-004 mixed gateway fixture;
4. AQ-003 rendered/traversal surface agreement;
5. AQ-008 camera-solid occlusion decision;
6. AQ-009 temporal ink/paving proof.

AQ-001 comes first because every later traversal headline depends on its oracle. AQ-002 precedes surface
polish because doubled ownership can invalidate material and height evidence. AQ-003 then gives feet,
collision and visible pavement the same truth.

### Stage 2 — establish production truth

- Decide AQ-007 target/minimum hardware and budgets.
- Capture initial performance distributions on representative routes.
- Add AQ-010 routine CI and exact release-feature checks.
- Address AQ-014 documentation drift.
- Inventory shipping assets for AQ-019.
- Keep the ledger concise; archive chronology rather than appending forever.

### Stage 3 — world visual coherence

Use the existing research documents for implementation detail:

- staged country-road→approach→gateway→street transitions;
- settlement edge/threshold/landmark/node/district language;
- material taxonomy, real-world scale, wear and footing;
- tree/grove/ecotone layers;
- dawn/dusk/weather/interior lighting matrix;
- profiling-led LOD/HLOD and quality tiers.

Limit each pass to one representative village, one city approach/block, one grove and one interior first.
Once the grammar works, propagate it procedurally.

### Stage 4 — vertical slice (Gate V)

The slice should force the systems that visual-only foundation work cannot answer:

- action-level keyboard/gamepad controls and settings;
- interaction/UI language;
- audio and surface footsteps;
- one creature/gameplay loop and meaningful objective;
- ranch, journey, arrival, interior use, consequence, return and save/reload;
- final camera behavior and accessibility baseline;
- performance on target/minimum hardware.

Do not choose the slice by geography alone. Choose a complete player story that uses the geography.

### Stage 5 — scalable production (Gate P)

- modular content kits with district/biome rules and budgets;
- LOD/HLOD/impostors and stable streaming;
- content validation, migration and regression automation;
- NPC/monster population systems, missions/exams/progression;
- audio/localization/accessibility pipelines;
- broad platform and release qualification.

## Claude response request

Claude does not need to work this list immediately. Please add dispositions for the **needs review** P0/P1
items when convenient, especially AQ-001, AQ-004, AQ-007, AQ-008 and AQ-010. For an accepted item, name
the proof that will close it. For a deferral/rejection, one sentence explaining the boundary is enough.

This ledger supersedes no detailed research. It only prevents the important decisions inside those long
documents from being silently lost.
