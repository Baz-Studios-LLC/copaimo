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
| AQ-009 | P1 | Ink | Prove screen-space ink under slow movement, multiple resolutions and presentation frame rates; stills already support composition/coverage only. | accepted | Claude already identified slow-pan evidence as pending; capture named clips after paving commit. |
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
| AQ-026 | P1 | Town streaming performance | Closed for the current architecture in `36d691a`. `--flyby` proved the shared-pool fault despite perfect frame time: ground median 42 ms / 95th 615 / worst 1537 when towns could consume the pool. Towns now take a quarter of workers, never fewer than one; ground median falls to 24–31 ms and 95th to roughly 350–500 ms. Claude correctly recorded that caps 4/3/2 did not bind and their differences were noise. | closed | AQ-007 target/minimum-hardware work must decide whether the remaining readiness distribution is acceptable. Reopen here for visible holes/stale collision, country-road starvation, or a task family bypassing the reserved headroom. |
| AQ-027 | P1 | Road handoff | A country road now ENDS at the town boundary and nothing continues it: `790cae4` clips at a circular `town_reaches`, but non-ring plans meet their perimeter at a different distance on each bearing. Measured at (-2553, 1771): dirt ends 320 m out, nearest street point 126.2 m further on. Claude's active correction sensibly clips to the same `Plan::off` shape used by the perimeter. Review found the ownership fact still duplicated in `paved_here` and `stands_on`: both retain circular distance tests, which would respectively finish paving at the wrong non-round boundary and ignore the analytical country-road surface where the newly drawn road lies inside the old circle but outside the actual plan. | open, in progress | Use one signed `off_the_town` authority for clipping, paving arrival and traversal ownership. Add per-road gap/overlap and rendered-vs-walked handoff tests for Rings/Grid/Spine, a secondary (non-primary-axis) approach, and a short grazing chord that enters/exits within one 4 m sample interval. Then return to AQ-022 phase continuity. |

## Recently closed or explicitly settled

| ID | Area | Disposition/evidence |
|---|---|---|
| AQ-C01 | Ink sky/planarity/MSAA/width | closed in `1ddd439`; Claude measured line coverage and took noon/dusk/city/canyon images. Temporal sweep remains AQ-009. |
| AQ-C02 | Generic inverted hulls | closed in `86639c2`/`8bce408`; retained only on warden/authored landmarks, affected geometry roughly halved. |
| AQ-C03 | Tree lollipop silhouette | closed for the stated silhouette fault in `c8a2aea`, with five-species orthographic/silhouette sheet. Near-camera facet quality remains an art review, not a reopened silhouette bug. |
| AQ-C04 | Junction node-profile interpolation | closed in `17405ff`, with a 19 mm flat-region ceiling guard. This does not close AQ-003 terrain drape. |
| AQ-C05 | Ink alpha over water/river/glazing | closed in `95dfba3`; authored alpha is multiplied by the ink mask and blended no-ink misuse is guarded. |
| AQ-C06 | Longitudinal junction normals and road wear/grade | closed for the stated commit scope in `a1e5937`; road-relative paving and temporal filtering are separate AQ-005/AQ-006 items. |

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
