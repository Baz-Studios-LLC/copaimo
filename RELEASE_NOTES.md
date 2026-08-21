## Copaimo: The Wardens Guild

Still a world to walk and nothing yet to do in it — no monsters, no battles, no
guild exams. This release is about **what the world is made of**: the trees, the
rocks, the grass and the flowers are now authored shapes rather than shapes the
code guessed at, and three things that were quietly wrong with the ground are not
any more.

### The world is modelled now

Everything the game drew used to be built from primitives in code. There is a way
in for real models now — a Blender pipeline with the conventions set once in a
script, and a gate on both sides of it that refuses a model which breaks them.

* **Five kinds of tree.** Oak, birch, spruce, pine and acacia, each built for its
  silhouette, because a silhouette is all you see of a tree at the distance the
  game draws it. Broad and round, slim and pale, tall and layered, and the acacia
  flat-topped and wider than it is tall — which is what says dry country from a
  long way off. Palm and willow still grow their own shapes.
* **Eight kinds of litter.** Boulders, scree, bushes, stumps, fallen logs, standing
  dead snags, cacti and dead brush.
* **Grass and flowers.** The blade and the flower head are authored; the *tuft* is
  still composed by the world — how many blades it carries, how far round they fan,
  which way the clump leans, how deep a green it is. That composition is the whole
  reason a meadow does not read as wallpaper, so it stayed exactly where it was.

Trees and rocks are opposites, and it decided the whole design. A tree is planted
as an object and tinted by the material its variety wears, so twenty trees can be
twenty greens. Litter is welded into ONE mesh per chunk — fifty separate little
objects would be fifty draw calls, paid for again in every shadow cascade — and one
mesh wears one material, so every colour a rock has lives in its vertices.

Detail follows size rather than being one number. An oak's crown fills the screen
when you walk under it and its outline wants to be round; the three little balls
that make a desert bush never read as anything but a bush, and paying four times
the triangles for them buys nothing.

### The ground: three things that were wrong

**A raised shelf that would not smooth away.** Towns stand on level ground and
roads are graded between them, and where two of those claims crossed, the ground
*stepped* — 8.6 m between vertices two metres apart. No brush could have taken it
out: the sculpting layer works in four-metre cells and cannot express the inverse
of a step that sharp, and the generator was re-applying it underneath anyway. Now
the strongest claim decides how much the ground moves and all of them decide where
it moves to, so a road running into a town still arrives at the town's level with
no lip where they meet.

**Debris standing out of cliffs.** Steep ground reads as rock, which carries more
litter than anywhere else and the right kinds for a mountainside — and a canyon
wall is seventy degrees of exactly that, so the walls came out studded with boulders
and dead sticks poking sideways. Litter now stops where a *walker* stops.

**A comb along the canyon rims.** The tops and bottoms of the walls were a row of
teeth. It is arithmetic rather than a bug: moving a rim sideways by a metre moves
the ground by the wall's own gradient, about 4.6 m per metre on a seventy-degree
wall — so a rim wandering half a metre between two vertices steps the ground by
two, and vertices are two metres apart. The rims wander gently now, and the
canyon's shape comes from the way through winding two hundred metres side to side,
which was always doing that work.

### And the trees are the right colour

A wood of chalk-pale trunks was two faults wearing each other's clothes. Authored
shapes were being matched to the tree pool by position rather than by species, so a
variety wore one species' crown over another's bark — and a birch's trunk is chalk
pale by design. The palette needed work too: the ramp from bark-brown to birch-white
ended so near white that even a fifth of the way along read washed out, and pine was
painting the colour of concrete. Spruce, oak, pine and acacia read brown now, and
only a birch goes pale.

### Anything with a front now faces the way it walks

A model's forward was aimed a half-turn from where it should be. Nothing caught it
because the only thing being turned was the blocky placeholder warden, which is
symmetric front to back — a box under a round hat looks the same either way, so a
warden walking backwards looked exactly like one walking forwards. The first model
with a face on it would have walked backwards across the whole world.

### None of it ships

The terrain brush, the workbench and the model kiln are not hidden behind a menu in
a player's build — they are not compiled in. A player's build should not carry a way
to break a save, and it certainly should not carry code that can spend somebody's
credits. The release workflow greps the binary and refuses to publish one with the
tools in it, because a dropped build flag is otherwise silent: a release that ships
a brush looks exactly like one that does not.
