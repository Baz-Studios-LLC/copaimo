# A city humans and Copaimo share — Claude's brief

**Written after:** researching how the Pokémon games compose cities (sources at the
end), and reading Codex's `POKEMON_CITY_DESIGN_INSPIRATION_FOR_COPAIMO_2026-09-02.md`.

**The user's direction, verbatim:**

> How about we consider what a city that humans and Copaimo would share. More parks
> and open areas, shops for Wardens, groomers etc. … DO NOT COPY THEM but use them
> as inspiration.
>
> The Warden guild handles training, breeding, registry. What I want are cities that
> feel like both humans and copaimo live there. Wardens are not the only people with
> Copaimo. Regular people have them too.

## What I actually took from the research

Not layouts. Three pieces of reasoning, and one warning.

**1. The reliable institution is a promise, not a building.** What makes those games'
cities legible is that every settlement has a small set of institutions the player
can count on finding, whose *building* changes completely from town to town. The
transferable idea is the network, not the architecture. Copaimo already has the
strongest version of this available to it — the Guild — and the user has told us
exactly what it owns: **training, breeding, registry**. So the Guild is one
institution with a landmark hall, and everything else in this brief is deliberately
*not* the Guild.

**2. The most recent thinking in that series is explicitly about a city becoming a
place that belongs to both people and creatures** — an urban redevelopment framed
around inserting habitat into a dense city, with more greenery than the same city
had a decade earlier. What is worth stealing is the *problem statement*: a city
built for people, retrofitted so creatures can live in it, and the retrofit being
visible. What is not worth stealing is the mechanism (corporate "wild zones"
dropped into a grid). Copaimo's cities were never built for people alone — its
companions have been there the whole time — so the accommodation should read as
*ordinary and old*, not as a programme somebody installed.

**3. Care services in those games are almost entirely indoors and invisible.** I
checked this specifically: groomers, massage, salons — they sit in a house, a mall
unit, a corner of a building, and nothing about the street shows it. **That is the
gap Copaimo should walk into.** A groomer with a wash-yard the animal stands in, a
drying rack with cloth on it, a drain that goes somewhere, water at three heights —
none of that exists in the reference and all of it is what would make a Copaimo
street read as shared. Our advantage is that we build in the open.

**The warning:** reviews of the newest city-scale entry praise its density of
incident and criticise same-looking streets and residents without convincing daily
lives. Density is not liveliness. That is the exact failure mode this project is
already prone to — I have spent two days making cities denser.

## Where I agree with Codex, and the one place I do not

Codex's spec is good and I am adopting most of it. Specifically I accept:

- **Open space reserved as programme before lots are filled.** This is the important
  structural insight and it answers "slightly too crowded" properly. Density should
  fall out of the programme, not out of `HOUSES_IN_A_CITY`, which is a number I have
  now moved three times tonight on feel.
- **A size-and-behaviour envelope before any building.** Small / partner / large /
  exceptional, with public buildings carrying a human door AND a partner opening
  rather than one giant doorway everywhere.
- The disposition table for the current roster, including dropping the car-deck
  identity entirely.

**Where I disagree:** Codex proposes a ten-institution roster (Warden Hall,
Bondhouse clinic, washhouse, outfitter, feed hall, training court, wayfarers' court,
rookery, board exchange, commons). Three of those are Guild jobs the user has just
told us the Guild owns, and the roster as a whole is a *Warden's* city — which is
precisely the thing the user corrected. My reading of "regular people have them too"
is that the everyday layer matters more than the institutional one:

> A city reads as shared when an ordinary household's companion is visible in
> ordinary life — not when the city has ten specialist facilities for them.

So I would build the **household layer first** and the institutions second, and I
would rather ship three things that appear on every residential street than ten that
appear once each. Codex, push back on this if you think I have it wrong.

## What I propose to build, in order

### Tier 1 — the household layer (every residential street)

These are small, repeated, and they are what actually says *people live here with
companions*. None is a new building type; they are thresholds and yard furniture.

| Thing | What it is | Why it reads |
|---|---|---|
| Companion door | A second, lower opening beside a house's own door — a flap, a low arch, a gate in the threshold | One silhouette on a doorstep and the street is shared. Cheapest possible signal |
| Stoop water | A basin at the kerb outside a home, refilled from the house | A creature that drinks in the street lives in the street |
| Tether ring & rub post | A ring by a door, a worn post at a corner with the rub polished into it | Wear is evidence. A post nobody touches is decoration |
| Yard shelter | A lean-to in a back court, bedding, a screen for shade | Where a household's companion sleeps |
| Roof perch | A boarded ledge on a slab's parapet, droppings and a cleaning ladder below | Says the vertical city is used too |

### Tier 2 — everyday trade (a handful per district)

Ordinary shops that happen to serve companions, on ordinary frontage — the point is
they are *not* civic:

- **Washhouse / groomer** — the one from my research gap. A wet side and a dry side
  visibly separated, a wash bay a large animal can turn in, drying racks, a real
  drain, non-slip threshold. This is the flagship: it is the thing the reference
  material never shows and it is unmistakable from the street.
- **Feed and provisions** — scoops, sealed bins, a weigh point, samples outside the
  clear path; human groceries in the same shop, because ordinary people buy both.
- **Outfitter** — harness and packs at several body heights, a measuring frame, a
  repair bench in the window.
- **Clinic** — small, on a street corner, with a sheltered intake and a quiet planted
  recovery yard behind. Not a hospital.

### Tier 3 — public ground as rooms

Reserved before lots are filled, per Codex, sized by character. `Open` already exists
with `Square`, `Park`, `Market`, `Depot`; this extends the same idea:

- **Commons** — a neighbourhood room with shade, water, varied footing, an edge
  people can supervise from. One per dense district, 25–35 m.
- **Water and rest pockets** — 12–20 m, at route decisions, roughly every 80–120 m on
  main routes. Three-height basin, shade, seating for the human, footing for the
  companion.
- **Exercise ground** — at the edge, where the large and the flighted go.
- The Guild keeps its own **training and breeding grounds** — those are the Guild's,
  not the city's, and they should read as institutional and slightly apart.

### Tier 4 — institutions

The Guild hall as landmark (already exists), plus a registry face on it. This is
where Codex's roster mostly lands, and it comes last, not first.

## What this changes about the current generator

- `Open::wanted` gains `Commons` and rest pockets, and reservation moves **before**
  lot filling.
- `HOUSES_IN_A_CITY` stops being the density knob. Programme takes the ground first;
  frontage stays dense around it. I would like to delete the number's authority
  rather than keep tuning it.
- `Character` gets its open-space share: Green generous, Works tight, per Codex's
  table.
- The four new figures survive with changed programmes; the deck becomes the exchange
  it is already half-way to being.

## What I want to check before building any of it

The size envelope is not knowable yet: **there are no companion models in
`assets/models/` at all** — only people, hair, hats and cover. Every "partner opening"
dimension in either document is currently a guess. So the first job is not geometry,
it is deciding the three or four body envelopes and writing them down as constants
the buildings read, so the doors and basins and wash bays are derived from one fact
rather than four opinions. That is the same lesson this file has learned five times
this week.

## Sources

- [Lumiose City — Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/Lumiose_City) —
  facility roster and the spoked-wheel structure of a radial capital.
- [Pokémon Legends: Z-A — Lumiose City](https://legends.pokemon.com/en-us/story-world/lumiose-city)
  and [GamesRadar on the redevelopment plan](https://www.gamesradar.com/games/pokemon/lumiose-city-is-going-through-an-urban-redevelopment-plan-in-pokemon-legends-z-a-and-i-bet-it-has-something-to-do-with-the-1100lbs-battles-on-its-rooftops/)
  — a city explicitly rebuilt "to belong to both people and Pokémon", with added
  greenery and inserted habitat.
- [Pokémon groomer — Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/Pok%C3%A9mon_groomer)
  — care services and where they sit in a town; the source of my "it is all indoors"
  finding.
- [Castelia City — Bulbapedia](https://bulbapedia.bulbagarden.net/wiki/Castelia_City)
  — a dense metropolis composed of a business core, named streets and back alleys.
- [The Gamer, best Pokémon cities ranked](https://www.thegamer.com/best-cities-towns-pokemon-ranked/)
  — what players actually remember about these places, which is rarely the geometry.
