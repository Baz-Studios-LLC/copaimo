# Designing a multi-level city

Sourced notes, for the first city's terraces. Written because the terraces were
being built from what seemed reasonable rather than from how these places are
actually made, and it showed: walls holding back open fields, roads forever changing
level, and a town that read as flat however many walls were standing in it.

## What a real hill town does

The finding that matters, and the one this project had backwards:

**Streets follow the contours.** Paths beaten by people and animals hold the maximum
grade near its lowest practical value, winding along the contours of the site rather
than running at them. Core streets are narrow — around 4–5 m — and wind along the
slope.
([Grokipedia, *Hill town*](https://grokipedia.com/page/hill_town);
[Wandering Italy, *Twisted Streets*](https://www.wanderingitaly.com/blog/article/1003/twisted-streets))

**The ways that climb are steps, not roads.** Infrastructure in hill towns emphasises
gravity and natural contours over flat-road systems, with **stepped pathways serving
as primary thoroughfares** to navigate inclines without extensive earthworks. A hill
town does not ramp its streets to get up the hill; it walks up them.
([Grokipedia, *Hill town*](https://grokipedia.com/page/hill_town))

**Buildings cluster and share walls.** Multi-storey buildings with shared walls are
usual — efficient on limited land, and mutually supporting against erosion and
instability on a slope. Terracing itself exists to mitigate erosion and make
construction possible.
([Grokipedia, *Hill town*](https://grokipedia.com/page/hill_town))

**The town grows around nodes.** Medieval Italian towns grew around two: the church
and the civic or military centre, and the street pattern radiates from those.
([Grokipedia, *Hill town*](https://grokipedia.com/page/hill_town))

So the order of operations is: the **slope** exists, the **streets** are laid along
it, the **terraces** are the ground between them, and the **walls** hold up the
streets. Not: grow streets anywhere, then cut terraces across them, then look for
somewhere to put a wall — which is what was being done here.

## What terracing is, structurally

Terraced retaining walls break a slope into level steps, each of which is a usable
platform. Individual tiers run up to about 1.8 m, with a whole system climbing on
the order of 12 m over a hillside. Terracing "guides how people move and interact",
each level serving a purpose and flowing into the next.
([Allan Block, *Terraced Retaining Walls*](https://allanblock.com/resources/articles/terraced-retaining-walls);
[Tamate, *Terraced Landscaping*](https://tamatelandscaping.com/terraced-landscaping/))

Two things follow for the game. Tiers are SHORT — a 3.6 m riser is already double a
typical garden tier and reads as civic rather than domestic, which is right for a
retaining wall carrying a street but not for one round a yard. And a terrace is a
PLACE, not a step: if a level has no use on it, it is not a terrace, it is a bank.

## What game level design wants from it

**Distinct horizontal layers, connected by vertical transitions.** Verticality is
built as layers with deliberate connections between them, and negative space — a
ravine, a drop — is what makes the separation read.

**Make the verticality readable.** A player should understand where the levels are
without stopping to work it out; if they have to hunt for what is under a ledge, the
layering has failed.

**Landmarks anchor the layers.** Every meaningful area wants a unique visual
signature — a tower, a distinctive roof, a coloured door. The "weenie", borrowed from
theme-park design, is the large visible thing that draws the eye and tells you where
you are.

**Signpost the transitions.** Steps up should be visible as steps up: lit, contrasted
or framed, so the way to the next level is found rather than searched for.

([game-development, *Vertical Level Design Techniques in 3D Games*](https://salivity.github.io/game-development/article/vertical-level-design-techniques-in-3d-games);
[IronEqual, *Practical guide on first-person level design*](https://medium.com/ironequal/practical-guide-on-first-person-level-design-e187e45c744c))

## What this means for Copaimo's first city

In order, and each one is a thing currently wrong:

1. **The hillside is the primary object.** It exists before the streets and does not
   depend on them. `settle::hill` on the `contour-streets` branch is this.
2. **Streets are laid along its contours**, with the fall taken by stepped ways. The
   growth's global goal has to include the contour direction, which is exactly where
   Parish and Müller intend terrain to enter the method.
3. **A terrace is the ground between two streets**, so it is a platform with a use on
   it, not a band cut across the plan.
4. **A wall holds up a street.** It has a street on top of it and something at its
   foot. A wall with a field on both sides is not a retaining wall.
5. **Steps where the fall is taken**, and they want to be seen: at the head of a
   street, framed, and leading somewhere visible.
6. **A landmark per level**, so the layers can be told apart from the ground and from
   outside the town.

## Status

The hillside and the contour steering are on the `contour-streets` branch and are
NOT merged: the pull costs the city a third of its buildings, because the growth's
step length and branching are tuned for straight sprouts and a curving one covers
less ground. The blocks want sizing to the contour rather than the contour bending to
fit the blocks. See that branch's commit message.
