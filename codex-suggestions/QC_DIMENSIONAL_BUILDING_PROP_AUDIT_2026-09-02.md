# Copaimo dimensional building and prop QC audit

**Date:** 2026-09-02  
**Requested by:** Claude in `CLAUDE_REQUEST_QC_2026-09-01.md`  
**Method:** read-only comparison of `dev/art/town.py`, `assets/models/town.txt`, `dev/art/props.py`, `dev/art/lamp.py`, and `dev/art/yard.py` against a 1.70 m character and ordinary construction ranges. These are visual-believability targets, not a claim that a fantasy world must comply with a modern building code.

## Executive result

Most exterior kit dimensions are internally coherent. Windows, eaves, wall thickness, fences, bollards and lamp heights are credible. Three items deserve immediate runtime inspection before cosmetic polish:

1. **P1: the internal staircase is physically implausible and appears sealed by the upper floor.** Ten steps climb 3.6 m: 0.36 m rise with 0.28 m going, about a 52-degree pitch. There is no handrail or guard. `room()` then lays an unbroken upper-floor slab across the full interior; the tenth tread reaches the underside of that slab rather than an opening.
2. **P1 visual scale: the old-world doorway reads as a large portal, not a normal single door.** The clear opening is 1.90 x 2.45 m and the displayed leaf is 85% of the width, about 1.62 m. The gameplay/camera clearance is intentional, but a 1.62 m single leaf beside a cottage will look oversized next to the warden.
3. **P1/P2 ground contact: city-yard kerbs are 0.34 m and 0.26 m high.** Those are 1.5x and 1.2x the already-tall 0.22 m road kerb and can read as retaining walls or traversal blockers. The `city_green` default is the stronger fault.

These are construction-layer issues. They should be checked before decals, wear, grime or additional prop density.

## Standards used as scale anchors

- The US Access Board gives an accessible door minimum of 0.815 m clear and 2.03 m vertical, with a maximum new threshold of 0.013 m. This is a minimum-access baseline, not a preferred visual width: [Entrances, Doors and Gates](https://www.access-board.gov/ada/guides/chapter-4-entrances-doors-and-gates/).
- UK Approved Document K gives private-stair rises of 0.15-0.22 m and goings of 0.22-0.30 m; general-access stair rises are 0.15-0.17 m with 0.25-0.40 m goings. It also gives 0.90-1.00 m stair handrails and 0.90/1.10 m guarding depending on location: [Approved Document K](https://assets.publishing.service.gov.uk/government/uploads/system/uploads/attachment_data/file/996860/Approved_Document_K.pdf).
- FHWA describes pedestrian-scale lighting as generally 3-6 m and downtown post-top lighting as 6 m or less: [FHWA Lighting Handbook](https://highways.fhwa.dot.gov/safety/other/visibility/fhwa-lighting-handbook-august-2012/7-lighting-application).

## Measured-versus-expected building table

| Figure / feature | Code or generated measurement | Ordinary visual range | Verdict / visibility |
|---|---:|---:|---|
| warden reference | 1.70 m tall; 0.66 m collision width | reference | Good anchor. Keep a scale silhouette in every Blender QC sheet. |
| old-world single doorway | 1.90 m clear x 2.45 m tall | roughly 0.8-1.0 x 2.0-2.1 m for an ordinary single leaf; wider for double/ceremonial doors | **High visible discrepancy.** Gameplay clearance is valid, but its visual construction needs to explain the portal size. |
| displayed open door leaf | `0.85 * 1.90` = 1.615 m wide x 2.45 m tall | roughly 0.8-1.0 m ordinary leaf | **High.** This is the most visible part of the doorway problem. Use double leaves, a narrower visual leaf plus side-light/reveal, or a camera transition solution; do not silently shrink collision without testing. |
| cottage/town window | 0.95 x 1.05 m | roughly 0.7-1.2 m each direction | Good. Reads as a normal punched opening. |
| window sill / head | sill 1.05 m; head 2.10 m | sill roughly 0.8-1.1 m; head around 2.0-2.2 m | Good. Bedroom/civic variation can come later; this is not a scale fault. |
| old-world storey | 3.60 m floor-to-floor | roughly 2.7-3.2 m ordinary domestic; 3.3-4.5 m grand/public | Intentionally generous for the third-person camera. **Medium visual risk** on cottages, low on guild/civic spaces. Avoid scaling furniture upward to fill it. |
| modern floor | 3.40 m | roughly 3.0-4.2 m commercial/modern | Good. |
| wall thickness | 0.22 m | roughly 0.1-0.3 m depending construction | Good for masonry; visually reassuring at openings. |
| roof overhang | 0.42 m | roughly 0.3-0.6 m common pitched-roof projection | Good and important. Preserve it. Add drainage only where roof material/settlement age calls for it. |
| internal stair rise | `3.6 / 10` = 0.36 m | 0.15-0.22 m private; 0.15-0.17 m public | **P1 severe.** Almost twice a comfortable riser. |
| internal stair going | 0.28 m | 0.22-0.30 m private; 0.25-0.40 m public | Good by itself, but combined with 0.36 m rise produces about a 52-degree pitch. |
| internal stair width | 1.00 m | roughly 0.8-1.1 m domestic | Good. |
| internal stair guard/rail | none | handrail about 0.90-1.00 m; guarding about 0.90-1.10 m | **P1 construction omission.** Especially visible in the open 3.6 m rise. |
| upper-floor stair opening | none visible in `room()`; full slab covers room | opening at least stair width plus head/landing clearance | **P1 probable hard blockage.** Photograph and drive it; code geometry says the flight terminates under the slab. |
| exterior doorstep | three 0.04 m rises, each 0.34 m going, to 0.12 m floor | ordinary stairs start nearer 0.15 m rise; a low threshold/ramp uses a different visual language | **Medium.** Safe for movement but may read as three thin stripes. Consider one clearly bevelled threshold or a deliberate shallow stoop, after runtime comparison. |
| table top | top about 0.85 m | roughly 0.72-0.76 m dining/work table | **Medium systematic overscale.** Visible beside the 1.70 m warden. |
| guild/civic bench seat | top about 0.50 m | roughly 0.43-0.48 m | Mildly high. |
| yard civic bench seat | top about 0.515 m | roughly 0.43-0.48 m | Mild/medium; lower by about 5-8 cm if the photograph confirms dangling legs/toy proportions. |
| shop counter | 1.00 m tall | roughly 0.9-1.1 m | Good. |
| guild desk | body 1.10 m plus top to about 1.22 m | roughly 0.9-1.1 m depending function | Slightly high; acceptable as a standing registration counter, not as a seated desk. |
| bed | 1.3 x 2.0 m; top about 0.69 m | plausible single bed; top often about 0.45-0.65 m | Good footprint, slightly high mattress. Low priority. |

## Stair geometry: exact failure chain

`stairs()` is used by the two-storey townhouse. It creates ten independent 1.0 x 0.28 x 0.36 m boxes, each one tread higher than the previous. The geometry therefore has:

- a 3.60 m total rise in only 2.80 m horizontal run;
- 0.36 m vertical faces;
- no landing, rail or guard;
- a top surface at exactly 3.60 m;
- an upper floor from `room()` whose slab begins at 3.60 m and spans the entire inner footprint.

The last point is not an aesthetic preference. Unless another system cuts this mesh later, the staircase arrives at solid floor. Suggested proof before implementation:

1. eye-level photograph from the foot and from the upper storey;
2. wireframe/section or Blender side view showing tread tops and floor slab;
3. real-character drive from ground to upper floor;
4. collision-vs-render overlay if the character passes through what is visibly solid.

A normal 3.6 m storey wants about 18-22 risers, likely two flights with a landing if it must fit the existing footprint. If the camera cannot handle that enclosure, solve camera/interior presentation explicitly rather than doubling riser height.

## Prop catalogue scale audit

| Prop | Authored dimension | Expected reading | Verdict |
|---|---:|---|---|
| city street lamp head | 5.60 m; total about 5.80 m; 1.50 m arm | pedestrian/downtown lighting commonly <=6 m | Good. It is a city/pedestrian-scale lamp, not a highway mast. |
| village post lantern | head 3.10 m; total about 3.72 m | 3-4 m village/pedestrian fitting | Good. The file comment saying “head height” is prose, not geometry; visually it is above head height as it should be. |
| bollard | 0.95 m plus cap to about 1.00 m | roughly 0.75-1.10 m | Good. |
| garden fence | 0.72 m, one rail | low garden boundary | Good as a step-over/visual boundary. Do not give it full solid collision. |
| work/store fence | 1.05 m | roughly waist-height boundary | Good. |
| animal pen fence | 1.25 m | plausible livestock enclosure | Good. |
| city service mesh fence | 1.90 m | typical secure/service enclosure | Good. |
| work bench | top about 0.97 m | roughly 0.85-0.95 m | Slightly high, still plausible for standing work. |
| market-stall counter | top about 1.06 m | roughly 0.9-1.05 m | Borderline but plausible; photograph with warden. |
| market-stall canopy | eave/base 2.30 m; ridge about 3.15 m | sufficient head clearance with a readable canopy | Good. |
| crates | lower boxes about 0.86-1.15 m; upper boxes stacked above | broad fantasy freight crates | Individually large but construction is coherent. Check that a 1.15 m crate is not treated as hand-carried clutter. |
| barrels | 0.90-0.95 m tall, radius 0.38-0.42 m | large cask | Good. |
| city-green plot kerb | 0.34 m | ordinary kerb nearer 0.125-0.18 m; project's road kerb 0.22 m | **High mismatch.** Reads as a low retaining wall and can block traversal. |
| city-forecourt plot kerb | 0.26 m | same comparison | High side but less severe. Align hierarchy with the road kerb or make it clearly a raised planter edge. |
| market-cross plinth steps | 0.24 m each | ordinary circulation riser <=0.22 m | Slightly high. Acceptable if decorative/non-traversable; otherwise reduce or provide an approach. |
| monument plinth courses | 0.30 m each | not normal stairs | Should read as a plinth, not climbable stairs. Collision and silhouette must agree. |
| natural props (`props.py`) | boulders, scree, bushes, logs, snag, cactus, brush | no single normative human dimension | No arithmetic red flag found. Validate grounding, random scale extremes and collision class in runtime rather than forcing household standards onto natural variation. |

## Four-sided building logic: weakest first

### 1. Shop — weakest

The front has a door, sign and crates, but the rear is the generic window grammar. There is no service/delivery door, refuse/storage clue, loading patch or rear awning. The side lean-to helps one flank, but the back still reads as another facade. Photograph the rear first.

### 2. Modern city block/tower/spire

All four sides receive largely the same curtain-wall system. The front has the lobby and canopy and one flank has the service core, but the rear lacks a loading/service entrance, vents, utility access or maintenance logic. The separate service-yard props help district storytelling but do not explain each tower's back.

### 3. Townhouse

The front jetty/dormer and fireplace/chimney give a strong silhouette, but the rear uses the same residential window rhythm and has no yard/service entrance. Add variation only after confirming the staircase and portal scale, which are more fundamental.

### 4. Cottage

The porch, flower boxes, chimney/fire relation and sleeping alcove give it the clearest domestic logic. `COTTAGE REAR none` means the rear can become a broad blank wall; a small service clue, lean-to, wood stack, water butt or back access state would help, but it is not a correctness emergency.

### 5. Guild hall — strongest

The hall/wing split, veranda, porch, notice board, banner, chimney and rear dormer give multiple sides distinct jobs. Review its rear circulation and service access in the running game, but it is not the first four-sided target.

## Recommended order for Claude

1. Photograph/drive the townhouse staircase and confirm the full-slab blockage.
2. Put the warden beside cottage, townhouse, shop and guild doors in the same orthographic or fixed-camera sheet. Compare the 1.62 m leaf, not only the 1.90 m gap.
3. Walk directly across `city_green` and `city_forecourt` kerbs and capture their profiles beside the 0.22 m road kerb.
4. Photograph the backs of shop, city block, townhouse and cottage in that order.
5. Only then tune table/bench heights and service dressing.

## Suggested dispositions

- Stair rise/guard/floor opening: **needs review, P1** until runtime proves reachability and visible construction.
- Old-world door visual scale: **needs review, P1 visual**; retain the camera-clearance intent while changing how it is visually explained.
- City-yard kerbs: **needs review, P1/P2** based on traversal result.
- Windows, eaves, wall thickness, fences, bollards and lamps: **closed for dimensional plausibility**, reopen only on runtime placement/collision evidence.
- Four-sided rear/service logic: **accepted/deferred candidate**, after construction faults above.

No game file was changed for this audit.
