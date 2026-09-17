# Request: the world becomes stored, starting with city 1

**From:** Claude · 2026-09-17
**Status:** open — two asks below, neither of them edits

## The decision

The world is generated and nothing is stored. `Terrain::trees_in` says it outright:
*"Nothing about a tree is stored anywhere."* That is what makes the world
reproducible and lets chunks stream on any thread in any order.

We are changing that, deliberately and in one place first. The user wants to be able
to edit the world by hand — fix a road, move a building, add or replace a prop — and
the signal that pushed it over was this: the last several sessions have been me
tuning generator constants to make procedural output match a concept painting. When
you are fighting a generator to hit a specific picture, that content wants to be
authored.

**Scope: the first city only** (`site.first`, currently at `-2553, 2251`). The rest
of the world keeps generating. If authored-plus-provenance feels good, other cities
follow one at a time; if it does not, we have lost one city's worth of work.

## The shape

`lay_the_site_out` returns a `Layout`. The bake is that `Layout`, written as
hand-editable JSON (`serde` and `serde_json` are already dependencies — no new
crate), loaded in place of generating.

Two rules I want held to:

**Derived data is never baked.** `Layout::streets` and `Layout::nodes` are derived
from `ways` — the doc comment on `streets` says "Derived from `ways`, never built
beside it." Baking them would be this project's recurring bug family written into a
file: one fact with two derivations, and the file's copy winning. The bake stores
`ways`, and `network()` re-derives on load. Same for the lanes and pads the levelling
files as claims.

**Every row carries provenance.** Generated (with the seed and a generator version)
or authored. A re-bake regenerates only generator-owned rows and leaves authored ones
alone; where a regenerated row collides with an authored one, the authored row wins
and the generated one is dropped. Without this, the first generator improvement after
the first hand edit forces a choice between them, and that choice is what kills
hybrid pipelines. It has to be there from the first version.

## Ask 1 — audit what assumes generation

This is the one I most want your eyes on, because it is a search over the whole
codebase and I will miss things.

Find everything that would be wrong, or silently disagree, if a settlement's layout
came from a file instead of a function:

- Every caller of `lay_the_site_out` / `lay_out`, and which of them assume the call
  is cheap, pure, or repeatable. Several guards regenerate the whole world in a test;
  `lay_the_streets` calls `town::lay_out` with the seed the game itself uses, which
  is how the levelling and the buildings agree at all.
- **Anything currently DERIVED that a careless bake would freeze.** `streets` and
  `nodes` are the two I know. I want the rest: `Settlements::lanes` and `pads`, the
  `Floor` table, the paving, `Built::docks`, anything keyed by index into a
  regenerated list.
- Threading and streaming: the world is meshed from many threads in any order and
  `trees_in` is explicit that it must not depend on load order. Where does a loaded
  layout have to be available, and what currently guarantees it?
- Index stability: is anything addressed as "the Nth plot of settlement K"? Those
  break the moment a bake is edited.

A list with file and line is exactly what I want. No fixes.

## Ask 2 — research the re-bake merge

You can browse. I want proven practice, not generalities:

How do shipped games handle a procedural-to-authored hybrid where the generator keeps
improving after content is authored? Specifically the merge: what identity scheme
lets a regenerated row be matched to its previous self so an edit can be carried
forward, and what do teams do when the generator stops producing a thing an edit was
attached to? I have my own answer (provenance plus a quantised-position identity),
and I would rather know what people who have shipped this actually do.

Cite sources. If the honest answer is "everyone bakes once and never re-bakes", that
is a useful finding and I would rather hear it than a tidy pattern nobody uses.

## What I am doing meanwhile

Writing the bake and load path for city 1. I will not touch the other settlements.
