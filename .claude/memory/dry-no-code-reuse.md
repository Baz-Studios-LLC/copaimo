---
name: dry-no-code-reuse
description: Standing instruction — never copy-paste/duplicate code; factor shared logic into functions/helpers
metadata: 
  node_type: memory
  type: feedback
  originSessionId: e087e0de-1f68-4dbf-93b4-93dd3a17b6e9
---

The user insists on DRY code across their projects (esp. [[neon-drift-project]]): **"always make sure we're not reusing code"** — i.e. never leave copy-pasted/duplicated blocks; pull shared logic into a named function/helper.

**Why:** clean, maintainable code is a hard requirement for them, not a nice-to-have.

**How to apply:** when writing or reviewing, scan for repeated blocks and extract them (e.g. NEON DRIFT: `edgePoint`/`aimInto` in arena.js, `polygonPath` in render.js, reuse `clamp`). Prefer an existing helper over re-implementing. Also flags reused ASSETS/behavior that should be distinct — e.g. the vortex shot must NOT reuse the normal `playShoot` sound.
