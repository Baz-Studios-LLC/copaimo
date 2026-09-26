---
name: neon-drift-shadowblur-perf
description: "NEON DRIFT perf gotcha — canvas shadowBlur is the bottleneck; batch strokes + use gradients for bloom, never hundreds of blurred strokes/frame"
metadata: 
  node_type: memory
  type: reference
  originSessionId: e087e0de-1f68-4dbf-93b4-93dd3a17b6e9
---

In [[neon-drift-project]] the neon look comes from `ctx.shadowBlur`, which is **very expensive** in canvas 2D. A warp-portal redesign that drew ~190 per-segment `shadowBlur` strokes + two large-radius (`52–66 * GLOW` ≈ 120–150px) blurred fills **per frame** dragged the framerate into a hang (user: "causes the game to hang").

**How to keep the glow cheap:**
- Render soft bloom as a **radial GRADIENT fill** (`createRadialGradient`), NOT a huge-`shadowBlur` fill.
- **Batch** many segments/dashes into ONE `beginPath` + one `stroke()` so each layer is a single blurred op, not N.
- Keep the count of blurred strokes/fills per frame in the **tens, not hundreds**; keep blur radii modest.
- `render.js` `neonStroke` does a 3-pass (halo+glow+core) for EVERY wireframe object — that's the baseline per-frame cost; the global `GLOW` dial (config, 2.3) tunes it, drop it first if weak hardware chugs.

**Verify perf** by timing N renders with `performance.now()` in `preview_eval` (e.g., 200 `drawBlackHoles` renders → want << 16.7ms/frame budget; the fixed portal is ~0.05ms/render). Note: screenshots have been frozen all session, so this timing + pixel sampling is the main verification path.
