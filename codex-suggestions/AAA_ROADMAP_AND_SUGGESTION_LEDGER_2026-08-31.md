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
| AQ-001 | P0 | Test evidence | Make `--drive` arrivals and blockers semantic. Current arrival is a 1.2 m radius; any 0.75 s no-progress or timeout passes `Blocked`, even at the wrong obstacle. | needs review | Add finish regions and intended-barrier contact bands; rerun all routes and distinguish wrong blocker/inconclusive. |
| AQ-002 | P1 | Junction topology | Four close-node pairs overlap; deepest 17.68 m and closest centres 3.76 m. Contract only a swallowed road-graph edge, not unrelated nearby ways. Current dirty `town.rs` implements union/centroid contraction. | accepted | Fail/report if the four-pass cap does not converge; prove moved endpoints cannot reverse/create crossings or re-planarise; report unconnected spatial overlaps; then node sheet and semantic turns. |
| AQ-003 | P1 | Road surface | Rendered road-triangle height and analytical traversal differ by ~7 cm in the accepted open measurement. | accepted | Choose shared-surface/tolerance contract; test interior/boundary/grade points and capture foot contact. |
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
