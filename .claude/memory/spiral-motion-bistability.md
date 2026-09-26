---
name: spiral-motion-bistability
description: Gamedev lesson — rotating a rigid spiral pattern reads as spinning either way (bistable); animate material ALONG the path for unambiguous vortex/drain direction
metadata: 
  node_type: memory
  type: reference
  originSessionId: e087e0de-1f68-4dbf-93b4-93dd3a17b6e9
---

Rotating a **rigid spiral pattern** is perceptually bistable — viewers lock onto either direction, and it often reads *backwards/outward* (NEON DRIFT's warp portal took 5 attempts; flipping spin signs never fixed it). Also true: if the thin tapered end LEADS the motion, it reads as outward.

**How to make a vortex/whirlpool read correctly:**
- Animate particles/streams **traveling along the spiral trajectory** (radius falls AND angle sweeps together per frame) — real motion, not pattern rotation.
- **Thick bright head leads inward; thin faint tail trails outward/behind.**
- Add hard motion markers (e.g. short revolving arcs whose endpoints are visible).
- Canvas y points DOWN: increasing atan2 angle = clockwise on screen; decreasing = counterclockwise. Derive the sign, don't guess.

Verify without eyes (screenshots often frozen): assert head radius strictly falls AND angle strictly falls/rises as intended over frames. See [[neon-drift-shadowblur-perf]] for keeping such effects cheap (batched strokes, gradient bloom).
