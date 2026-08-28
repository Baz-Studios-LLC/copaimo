# The drawn world

Everything here belongs to the world as it was **drawn**, up to 2026-08-27: the
map image the continents were traced from, and every layer anybody hand-edited on
top of it — sculpted ground, painted country, planted woods, laid surfaces, and
placed buildings.

None of it is read any more. From 2026-08-28 the world is **grown** from
`config::LANDMASSES`, and these layers are keyed to world coordinates that no
longer mean what they meant — the ground under them is different ground. Left in
place they do not fail, they simply land somewhere arbitrary: a hand-painted yard
and a hand-cut channel came out in the middle of Karrow's desert, several
kilometres from anything, which is how this was noticed.

Kept rather than deleted because it is authored work and git remembers where it
came from. To put the drawn world back, move these up one directory and set
`config::GROWS_ITS_OWN_WORLD` to false.
