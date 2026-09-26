---
name: neon-edge-release-cadence
description: "VIOLET EDGE — batch releases; don't cut a GitHub release for a single change"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: e087e0de-1f68-4dbf-93b4-93dd3a17b6e9
  modified: 2026-08-07T15:17:52.268Z
---

**Batch VIOLET EDGE releases.** Don't push a GitHub release for one change — accumulate a
significant amount of work first, then cut a release. (Stated 2026-07-20.)

**Why:** no sense spending a release + CI run on a single tweak; the user tests desktop builds
periodically, not per-change.

**How to apply:** keep committing LOCALLY after each verified change (build + `cargo test` +
clippy green), but leave `Cargo.toml` at the last-published version and DON'T tag/push until the
user explicitly says it's time to release. This composes with the standing rule to never push
without an explicit go-ahead. See release recipe in [[neon-edge-github-repo]]; project in
[[neon-edge-bevy-port]].

**Version numbers grow SLOWLY (user, 2026-07-28): "no more huge release increments for a small
amount of additions."** Default to PATCH bumps (0.4.2 → 0.4.3) even for feature-carrying releases.
v0.4.3 shipped a large batch (Nova Shield, 12 achievements, lore system, HUD overhaul, any-screen
scaling) as a patch bump under this policy.

**The line that has actually settled (as of 2026-08-04):** PATCH for polish + fixes (v0.6.1 = menu
pass, neon-tube border, starfield); MINOR for a CONTENT DROP — new rock types + a new powerup + a
boss upgrade together (v0.6.0 = Facet/Husk/Gorge/Glutton+; v0.7.0 = Binary/Sunder/Lance/Slinger+).
Both minors were flagged to the user at cut time and neither was objected to. Still flag the choice
in one line when cutting a minor so they can redirect.

**SHIP IT UNTESTED IF IT'S READY TO BUILD (user, 2026-08-04): "Even if it's not tested I'd rather
have it in. We can adjust if necessary later."** Don't hold a release waiting for playtest, and
don't hedge about it — they're the only tester, so nothing gets tuned until it's in their hands.
Note untested areas once, plainly, then cut. (Green tests + clippy + a release build are still the
bar for cutting at all.)

**ALWAYS keep patch notes current (user, 2026-07-21):** every change goes in `CHANGELOG.md` (repo,
has an `## Unreleased` section) as you make it, and every release sets a player-facing GitHub release
**body**. Why: the [[baz-studios-launcher]] has a **"What's New" panel that renders each game's release
`body`** — so the release notes ARE how testers see what changed. Stale/empty notes = testers can't
see changes. (The launcher already shows them; no launcher change needed — just don't ship empty notes.)
