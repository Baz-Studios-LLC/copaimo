# Game design

Shorter than the technical files, deliberately. Copaimo's design decisions live in `DESIGN.md`;
this is the outside reference behind them.

## The creature-collection loop

**STANDARD.** The genre is monster-taming / creature-collector: acquire creatures, train them, use
them in battle and elsewhere. The two ancestors do different things, and Copaimo is explicitly
both:

**Pokémon's loop** — walk into tall grass, something appears, throw a ball, catch it. On top of
that: type matchups for strategy, a team of six forcing roster decisions, and a Pokédex giving
completion its own satisfaction. The loop is **encounter → capture → roster**.

**Monster Rancher's loop** — you are a breeder, not a fighter. Capture, **train**, battle.
Customisation through limited points spent on moves and abilities, which improve as monsters gain
experience through exploration and battle. The loop is **raise → improve → test**.

The genre's foundational mechanics, generalised: **recruitment, customisation, party
integration** — creature companionship as the core loop rather than a side system.

> **→ For Copaimo.** The project's framing is Pokémon + Monster Rancher with monsters as *allies*
> rather than tools, which sits closer to the Rancher half. Worth noticing what that costs: the
> Rancher loop needs a **time structure** to raise things in — weeks, training sessions,
> scheduling — where the Pokémon loop needs a **world to explore**. Copaimo is building the world
> first. Those are not in conflict, but the raising loop is the one with no scaffolding yet, and it
> is the one the "allies not tools" framing leans on hardest.

## Open-world scale

**STANDARD, and it contradicts the intuition.** Perceived scale does not track map area. A 1 km²
area can feel very large when it is dense in content and visual variety. "Open world" is a style
of play, not a map size.

**STANDARD.** Fast travel *reduces* perceived scale — it makes a sprawling world feel compressed.
Genshin allows unlimited teleport to any unlocked waypoint, and pairs that with a world where any
point you can see is reachable: no peak too high, no chasm too deep. The freedom to *go* is doing
the work that size does not.

**MEASURED (Copaimo).** The reported complaint was "it takes a long time just to move beyond the
fence", and the eventual cause was **not** world size — it was `covers` understated by up to 36%,
so the clip played too fast for the ground covered and the character churned. Two candidate
explanations were tried and rejected first: a larger world, then a smaller one.

> **→ For Copaimo.** The lesson generalises. "The world feels too big" and "movement feels slow"
> are different complaints with different fixes, and the second one masquerades as the first.
> Before changing world scale, check the optical-flow side: speed, FOV behaviour under speed, and
> whether the legs agree with the ground. FOV widening with speed (`SPEED_WIDENS`) is doing real
> work here and is the standard trick.

## Companion AI

**STANDARD.** What companion systems are judged on is **readability** — whether the player can
tell what the companion is about to do:

- follow, with a sane leash and no blocking of the player
- engage threats without being told, but retreat when hurt
- respond to the environment rather than only to combat
- a small, explicit command set — heel, guard, attack-my-target — rather than a deep one

Far Cry Primal's tamed animals are the often-cited example: they react to danger, engage without
prompting, and retreat when wounded. Guild Wars' animal companions add an explicit interface for
heel / guard / lock-target on top of minion-like default behaviour.

> **→ For Copaimo. OPEN.** Nothing built yet. The thing to design first is not the AI, it is the
> **legibility contract**: what the player can rely on the monster doing without being told. That
> decides the animation set (a companion that "retreats when hurt" needs a retreat, a hurt state
> and a readable transition) and so it should be settled before monster rigs are built, not after.

## Feel

Covered in [animation.md](animation.md#game-feel); the short version:

- **Input buffering** with per-action windows — 6–8 frames for attacks, 3–4 for dodges. Uniform
  windows make dodges sticky.
- **Coyote time** on jumps.
- **Animation cancelling** reportedly buys more feel than buffering, for less work.
- **Anticipation is the animation principle that fights games.** Windups on player-initiated
  actions read as sluggish. Put anticipation on what the world initiates.

## Difficulty and balance

Nothing researched here that beats the project's existing position, which is recorded in
`DESIGN.md`. Noted only so this file does not look like it has an opinion it has not earned.

## Sources

- [Monster-taming game — Wikipedia](https://en.wikipedia.org/wiki/Monster-taming_game)
- [Awesome creature collecting games that aren't Pokemon — GameRant](https://gamerant.com/pokemon-monster-collecting-games-recommendations/)
- [The Guide to Open World Environment Design — 80 Level](https://80.lv/articles/skyrim-designer-on-building-virtual-worlds)
- [Fast travel — Wikipedia](https://en.wikipedia.org/wiki/Fast_travel)
- ['Genshin Impact': Crafting an Anime Style Open World — GDC Vault](https://www.gdcvault.com/play/1027538/-Genshin-Impact-Crafting-an)
- [Open-world games with the most realistic AI companions — GameRant](https://gamerant.com/best-open-world-games-ai-companions/)
- [Animal companion — Guild Wars Wiki](https://wiki.guildwars.com/wiki/Animal_companion)
- [Input buffering, coyote time — devlog](https://tooster.itch.io/chapter-01/devlog/560384/input-buffering-coyote-time)
