---
name: test-after-building
description: Always test after building — compiling and launching without a panic is not verification; only ask the user to check things that genuinely need eyes
metadata:
  type: feedback
---

Test everything after building it. Compiling clean, passing existing tests, and
launching without a panic is **not** verification — it is the absence of one kind
of failure.

**Why:** on 2026-08-18 I shipped a workbench gizmo where clicking an arrow placed
a piece instead of grabbing the handle. I had written that exact failure mode into
the gizmo's own comment and then never exercised a click. The user found it in
seconds. Earlier the same day a CI check reported success while broken for the
same reason. The user: "simple logic mistakes like that aren't great."

**How to apply:** before saying something is done, test the behaviour, not just
the build. Interaction logic (click routing, drag, mode switching) is testable
headlessly — a Bevy `App` with `MinimalPlugins`, the real systems, and simulated
`ButtonInput` runs without a window, so "it needs a window" is usually false. See
`src/bench/mod.rs` tests for the harness. Reserve "please check this" for things
that genuinely need eyes: whether a colour reads, whether a shape looks right,
whether a control FEELS right. Say plainly which of the two something is.

Related: [[ranger-monster-game-concept]], [[dry-no-code-reuse]]
