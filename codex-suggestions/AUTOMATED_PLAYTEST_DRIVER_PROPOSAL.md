# Proposal: deterministic character playtest driver

## Recommendation

Build a deterministic playtest driver that controls the real warden through the same input, movement, collision, grounding, and camera systems used during normal play.

This should not begin as an AI agent or a general-purpose pathfinder. Its first job is repeatable proof: attempt known difficult routes exactly as a player would, detect failures, and leave concise evidence. A clever navigator can hide broken geometry by finding a route around it; a test driver should deliberately cross the seam, kerb, doorway, slope, or junction under examination.

This would be unusually valuable for Copaimo because many recent failures only exist in the assembled game:

- a visible road and its analytical walk surface can disagree;
- a doorway can look open while framing or collision blocks it;
- a kerb can render correctly but refuse the controller;
- a collision decision can change with movement speed or frame time;
- a building can be technically placed yet float, sink, or meet its doorstep poorly;
- a route can work in a unit fixture but fail in a real generated settlement.

Unit tests should continue to protect formulas and invariants. The driver should protect the experience created when those systems meet.

## Core rule: drive the real character

The driver must not teleport the warden along the route, write transforms directly each frame, or call a simplified collision function in place of gameplay. It should submit the same directional intent a keyboard or controller submits and let the production movement system decide what happens.

Teleporting is acceptable only to place the warden at the beginning of an isolated test. From the first test frame onward, the ordinary movement, grounding, collision, animation state, and camera should run.

Keep the driver behind an explicit development/test mode so no autonomous input or evidence machinery enters an ordinary play session.

## First architecture

Use authored routes made of checkpoints rather than autonomous navigation. A route should describe:

- the fixed world seed and generated site or stable landmark it targets;
- the starting position and facing;
- ordered checkpoints or short input phases;
- walk or jog intent;
- the simulated update rate;
- maximum time and maximum allowed detour;
- expected result: arrive, be blocked, step up, remain grounded, or pass through an opening;
- evidence viewpoints to capture along the way.

The route runner should steer toward the current checkpoint, but it should not search broadly for another path when progress stops. A small heading correction is reasonable; pathfinding around the obstacle invalidates the test.

Prefer semantic anchors where possible—settlement ID, building type, doorway, road gateway, bridge end—over unexplained world coordinates. Coordinates are still useful as resolved evidence in the report.

## What to measure every run

At minimum, record:

- route name, seed, site, requested pace, and simulated frame rate;
- start, final position, and checkpoints reached;
- elapsed time and distance travelled;
- time since meaningful forward progress;
- requested movement versus actual horizontal movement;
- ground/support height and change per update;
- largest upward and downward snap;
- number and duration of blocked attempts;
- whether the destination was reached within its tolerance;
- camera-to-player distance and obstruction state where available;
- relevant surface classification: terrain, road, footway, floor, bridge, or water.

A route should fail when it makes no meaningful progress for a short named interval, leaves the permitted corridor, exceeds its height/snap allowance, changes an expected blocked/pass result, or times out.

Do not define “stuck” as zero speed in one frame. Kerbs, turns, and collision slides can briefly slow movement. Use progress toward the current checkpoint over a window of time.

## Initial route suite

### 1. Country road into each settlement class

Follow the actual road from unpaved countryside through the paving transition and onto the receiving town or city street.

Check:

- continuous forward progress;
- no lateral jump at the width transition;
- no sudden support-height change outside the intended kerb;
- road drawing and walk support remain aligned;
- junction arrival does not snag the controller.

Run village, town, and city versions on fixed representative seeds.

### 2. Kerb approach matrix

Cross a real city kerb from carriageway to footway and back at several approach angles, including perpendicular, shallow diagonal, and nearly parallel.

Run both walk and jog at simulated 30, 60, 120, and 240 Hz. The pass/fail result must not change with update rate. This test is particularly important for distinguishing a discrete step allowance from a frame-sized slope bypass.

### 3. Doorway sweep

Walk through the production doorway of every enterable building type, from outside to a meaningful interior point and back out.

Include the guild hall separately because it has offset masses and authored framing. Approach slightly left, centered, and slightly right so a narrow timber, jamb, door leaf, doorstep, or collision gap cannot hide behind one perfect line.

### 4. Building perimeter and footing

Walk around representative buildings on level and sloping sites. The warden should not pass through walls or footings, fall into a visible foundation gap, or be lifted onto the floor from outside. Capture low-angle images at the downhill corners.

### 5. Canyon gate: expected failure

Attempt to walk up the canyon wall at each update rate and both paces. This route passes only when the warden remains blocked while movement along the canyon floor and back down remain possible.

Negative tests are essential: a driver that checks only successful travel can approve a controller that walks through every obstacle.

### 6. Bridges and water boundaries

Cross each bridge type along its center and near both safe edges. Approach from both ends. Confirm the support changes to the deck without a vertical snap, and that leaving the intended edge does not produce an invisible walk surface.

### 7. Junction and bend suite

Traverse T-junctions, mixed-width junctions, gateways, and ordinary bends. Check that bends do not behave like intersections, junction patches do not lift the player above their incident ribbons, and the narrow arm remains usable.

### 8. Short representative journey

Run one small end-to-end slice: ranch or spawn area, road, settlement entrance, guild hall doorway, interior destination, and exit. This is the integration smoke test, not a replacement for the focused routes above.

## Visual evidence

For every route, take predetermined screenshots rather than letting the bot choose attractive angles. Useful checkpoints include:

- low ground-level view across a transition or kerb;
- normal third-person gameplay view;
- overhead diagnostic view showing route and collision/support samples;
- doorway view from both sides;
- before, during, and after a height transition.

Add a moving-camera capture for materials with sub-metre detail, such as cobbles. A still image cannot reveal temporal shimmer, moiré, or a pattern that swims as the camera moves.

The evidence report should be written incrementally after each completed route so a crash leaves the last successful checkpoint and the route that was active. Write images to a dedicated ignored evidence directory; do not clutter the repository root or allow a normal `git add -A` to stage them.

## Determinism and reproducibility

Pin all state that can change the result:

- world seed and site selection;
- time of day and weather;
- pace and input sequence;
- simulated/fixed update step;
- camera mode;
- any procedural decoration that affects collision;
- route timeout and tolerances.

The report should include the commit hash and configuration used. If a failure cannot be reproduced from the report, the driver has produced a story rather than evidence.

## Frame-rate testing

Do not merely run the executable faster or slower and hope the desired frame times occur. Provide a test mode that advances gameplay with controlled fixed deltas representing 30, 60, 120, and 240 Hz. Rendering can be decoupled from these samples if necessary.

The invariant is behavioral: the same route should be passable or blocked at every tested rate. Travel time may vary slightly from integration and steering tolerance; collision classification must not.

## Prevent the driver from concealing defects

The first version should not have:

- navmesh pathfinding around obstacles;
- jumping or recovery teleports;
- automatic route replanning;
- different collision or grounding rules from the player;
- broad success tolerances that accept arrival on the wrong side of a wall;
- retries that overwrite the first failure without recording it.

Later, a roaming exploration bot can use navigation to search for unknown problems. Keep that separate from the deterministic regression suite. The regression driver proves known contracts; the explorer discovers candidates for new routes.

## Suggested delivery stages

### Stage 1 — Minimal route runner

- One fixed seed.
- One straight route through a known doorway.
- Real player input and movement.
- Arrival, timeout, and stuck detection.
- A small text report.

### Stage 2 — Movement contract matrix

- Kerb, doorstep, slope, canyon, and bridge routes.
- Walk/jog and controlled 30/60/120/240 Hz updates.
- Height and blocked-motion telemetry.

### Stage 3 — Generated-world anchors

- Resolve representative village, town, city, guild hall, gateway, and bridge locations from generated data.
- Run the same semantic tests across several fixed seeds.

### Stage 4 — Evidence capture

- Predetermined stills and short moving-camera captures.
- Incremental report rows.
- Dedicated ignored output directory.

### Stage 5 — Exploratory walker

- Optional navigation-assisted roaming.
- Coverage heatmap and automatic candidate failure locations.
- A human converts valuable discoveries into small deterministic routes.

## Acceptance criteria for the first useful version

The driver is useful when it can:

1. control the normal warden without directly moving its transform;
2. repeat one route identically from a fixed seed;
3. prove one expected-success route and one expected-blocked route;
4. detect and report a forced obstruction;
5. run at two controlled update rates with the same collision result;
6. leave a concise report identifying where and why it stopped.

Do not wait for a general bot before using it. A small, trustworthy driver covering five dangerous boundaries will provide more value than an impressive autonomous character whose decisions are difficult to reproduce.

## Division of responsibility

Claude should own implementation and integration with Copaimo's runtime. Codex can review the driver design, route coverage, reports, and failures through this suggestions directory without modifying the game. Human playtesting remains responsible for feel, visual composition, atmosphere, and whether a space is enjoyable; automation proves repeatable mechanical and rendering contracts, not taste.
