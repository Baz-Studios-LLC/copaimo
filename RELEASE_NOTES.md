## Copaimo: The Wardens Guild

### This release: the packaged build finds its own world

A fix, and a quiet one worth naming. The world's layers — the map, the sculpting,
the woods, the surfacing, the countries, whatever is placed — were read against the
**working directory**. Run from the repository that is the crate root and all of it
is found; but macOS launches a `.app` from `/`, which is how the studio launcher
starts it, and from there every one of those paths missed. Nothing errored: the map
fell back to procedural and each missing layer looked like an unpainted world, so
the packaged mac build drew a world nobody had made and looked fine doing it.

Assets are now found beside the binary when the working directory has none. Bevy's
own asset server and the window icon each already did this; the files read by hand
were the set that did not.

Everything below shipped in v0.1.10 and is unchanged.

---

Still a world to walk and nothing yet to do in it — no monsters, no battles, no
guild exams. This release is about the **road east**, the workbench that furnishes
the world, and a long list of things that used to look wrong.

### The canyon gates the road east

The way from the desert to the green country is now **one winding slot canyon**
through a flat-topped massif. The top is a mesa, above the treeline and too high
to walk over. The walls are sheer — a hundred and seventy metres of rise over
fifty-five of run — and jagged, so the skyline is crags and buttresses rather than
a drawn line. The way through swings two hundred metres side to side, so no
straight line crosses it and no sightline reaches the far country: you cannot see
what is on the other side until you have walked it.

It **braids**, the way a real slot canyon does. A fork leaves the main way and
rejoins it a quarter-kilometre later around an island of true rock — both ways go
through, and neither is signposted. Two spurs open off the route and pinch shut at
a headwall, so a junction is a choice with a wrong answer, and the world has
alcoves to hide things in later. The floor is fifty-two metres wall to wall,
opening to ninety at the junctions: room for a party, oncoming traffic, and a
camera swinging behind them.

The whole massif is desert, floor included. The handover to the green world happens
on the plain past the eastern mouth, not halfway down the canyon.

**And the walls are walls now.** Terrain is this game's only geometry, so a walker
could stroll straight up a seventy-degree face — the canyon gated nothing. A step
that climbs more than 1.4 m per metre travelled is refused; only the step *up*, so
no slope is ever a trap, and a refused step retries along each axis so brushing a
wall slides along it instead of sticking.

### A tunnel was built here first, and it was the wrong idea

Worth saying plainly, because it is most of what happened this cycle. The road east
was going to be a tunnel: a bore tool, a cave mesh, two-level walking, a carve that
opened the surface at each mouth, holes cut out of the terrain, and a stone
doorframe to make the holes findable. All of it worked, and it still read wrong on
screen five times running.

The reason is one fact the whole world is built on: **a heightfield has exactly one
height at every point**. Anything under the ground needs a parallel world beside
it, and each patch bought another. So the tunnel was removed — every line of it —
and replaced by a gate the terrain is actually good at: walls that go up, a floor
that stays down, sky overhead. The whole story is written down in
`TROUBLESHOOTING.md` so nobody re-derives it.

### The workbench builds things you can stand in the world

The bench was a wall of text over a grey grid. It now has a camera you can fly, a
shelf of parts you click, colour swatches you click, and a floor that ends where
the work is.

* **Floors read as wood** — planks with grain and no gaps between them, and
  stretching across the grain adds *more planks* instead of stretching the ones
  there.
* **New parts**: stone foundations, dark-brown stairs, and beds.
* **Walls sit on the floor** rather than clipping through it, and they snap to a
  stretched floor's real edge.
* **What you select stays selected** until you pick something else, with a glow on
  its border — so reaching for a floor under a wall no longer loses it to whatever
  the handles were over.
* **Work can be named**, which is the whole pipeline from bench to world.

In the world itself, placed buildings can now be **turned** and **moved** after
they are put down. Not everything has to face north.

### The world looks like itself

* **Biome edges blend.** Desert into grass and grass into snow were stepped bands;
  they are gradients now. Three separate faults were stacked there, and the worst
  colour jump across a boundary went from 0.81 to under 0.06.
* **The ground is not one flat colour** — mottled at two scales, and the grass is
  broken up without touching the tufts you walk through.
* **No more faint streaks over everything**, worst at night: shadow acne at
  grazing light, which needs a bias that grows as the light drops.
* **Cloud shadows keep off the water.** They were physically right and read as
  stains on a flat blue sea.
* **Rocks sit on the ground** instead of hovering, and the world stops popping as
  you cross it.

### It runs better

The frame was mostly shadow work: every tree in a 254-chunk disc was resubmitted to
three shadow cascades every frame. Trees stop casting past a two-chunk ring, the
sea's 26,000 vertices moved to the GPU instead of being rewritten and re-uploaded
each frame, barren chunks stop being re-dressed forever, and the hidden debug
overlay stops sampling terrain while nobody is looking at it.

### The maker's tools

The terrain tool's panel is clickable throughout — every action has a row, each row
prints its own key, and no key means two things any more (`B` was the biome brush
*and* the tunnel bore). The mouse looks around with nothing held; **ALT** frees the
pointer to reach the panel. The brush ring lands where the cursor is, panels scroll,
and sliders and the minimap click in the right place on a scaled display.

There is also a **`TROUBLESHOOTING.md`** now: forty-odd faults from this project
arranged by *symptom*, with the shape each one turned out to take. Six shapes cover
most of them — one question with two answers, written but never registered, a proxy
that stopped being true, tests asserting the old semantics, invisible by design, and
granularity mismatches.

### None of it ships

The terrain brush, the workbench and the model kiln are not hidden behind a menu in
a player's build — they are not compiled in. A player's build should not carry a way
to break a save, and it certainly should not carry code that can spend somebody's
credits. The release workflow greps the binary and refuses to publish one with the
tools in it, because a dropped build flag is otherwise silent: a release that ships
a brush looks exactly like one that does not.
