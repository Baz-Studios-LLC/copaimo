# Working with an AI on this

Written by the AI, about its own failures on this project, checked against published research
because the pattern turns out to be well documented rather than personal.

The short version: **on 3D and rig work, an AI's reasoning is not the bottleneck — its
observability is.** Almost every hour lost on this project was lost to acting on a belief about
geometry that a measurement would have refuted in seconds.

## What the research says

**The observability gap.** In a Blender 3D scene-generation study, agents given **only visual
output** as feedback scored **0 out of 10** on full-scene success, across every condition tried —
while demonstrably being able to write the code. Failures originate in code logic and execution
state; human evaluation happens at the rendered output. Multiple different internal bugs produce
identical visible symptoms, so output-level feedback is *symptom-correcting, not
cause-identifying*.

The agents showed **failure-mode oscillation** — swinging between complementary failures. Include
the ground plane in collision checks and objects vanish; exclude it and they overlap. Round and
round.

Then the finding that matters: adding **one architectural constraint** at the code level —
"exclude the ground plane from collision checks" — made the system **converge within 3 cycles**,
every independent run. The bottleneck was feedback observability, not programming competence.

The recommendation is not "look at more pictures". It is **intermediate observability layers**:
structured execution summaries, runtime signals, and visual debugging overlays that expose partial
execution state *alongside* the render.

**Long-horizon work degrades sharply.** Frontier models are near 100% on tasks that take a human
under four minutes and **under 10% on tasks over four hours**. On long-horizon repository
evolution benchmarks the best frontier model scores 25%. The dominant explanation is a
**context-handling gap, not a reasoning gap**.

**3D spatial reasoning is a specific weakness.** Inaccurate spatial modelling, and trouble with 3D
distance estimation, object localisation and multi-step manipulation. Models fail to capture
fine-grained spatial constraints.

## How that showed up here, exactly

Every one of these is real, from this repository's history.

### Measuring the wrong representation

The dominant failure class, by a distance. In each case the number was correct *about something*
— just not about the thing being claimed:

| Claim | What was actually measured |
|---|---|
| "the straps are 12 cm off the arm axis, under the limit" | mesh-local units compared against a world-centimetre threshold |
| "6975 of 10131 edges are boundary — the mesh is full of holes" | split topology, where every hard edge looks like a boundary. Welded: 140 of 6710 |
| "the legs are 45% of body height, which is short" | thigh+calf bone chain, not the hip-to-floor landmark. Real figure 50.1%, i.e. normal |
| "the head bob is fixed" | a head that was detaching — the neck stretched 386% |
| "the rest ankle bend is 46° out on the left" | `pose.bones` holding the previous clip's leftover pose |
| "this is the character we already fixed" | the raw delivery, not the built asset |
| "the thumb points 0.96 across the palm" | true before a subdivide, 0.41 after — a branch direction that moves with mesh density |

The user's standing instruction — *"Blender is the tool; measure and render in it rather than
reasoning about geometry"* — was earned. **Every rig fault on this project came from reasoning.**

### Tuning a threshold instead of fixing a design fault

The build re-derived mesh repairs from a classifier on every run. When it cut the sleeve cuffs, I
tuned the threshold. When it cut a trouser leg, I tuned the threshold. When it took part of a
shoulder, I tuned the threshold. Three rounds before the actual answer — *stop re-deciding, commit
the asset* — became visible. And it only became visible because the user asked why the broken
original had not been overwritten yet.

**The tell:** if the fix to a wrong decision is a different number, check whether the decision
should be being made at all.

### Failure-mode oscillation, verbatim from the research

Which digit is the thumb. I proposed four discriminators in a row, each reasoned from numbers,
each clean on one hand or one mesh density, **each wrong**:

- points across the palm → correct as delivered, broke after subdivision
- reaches least far → names the pinky (12.2 cm vs 14.0)
- knuckle nearest the wrist → names the pinky (9.5 vs 11.0)
- most opposed to the others → names the pinky (25.2° vs 23.3°)
- removal leaves the rest collinear → names the *middle finger* (14.69:1 vs 5.56:1)

A thumb is not the short digit, the splayed one, or the odd one out. **Painting the five digits
five colours and rendering it settled the question in one pass.** I should have done that before
the first argument, not after the fourth.

### Reasoning about handedness

I wrote a comment asserting that the roll convention gives both hands the same curl sign. Then
measured it: **-1 left, +1 right.** A cross product is a pseudovector; mirroring its inputs
mirrors *and* flips the result. This is not a thing to be right about at a keyboard, and the code
now measures it instead — curl the fingers, see whether the tips approach the wrist.

### Two changes in one build

Added a bone to the keyed set *and* raised a pitch amount in the same run, saw the number move,
and credited the wrong one. The bone channel was still dead.

### A guard that compared against its own input

The A-pose bake was verified by comparing its result to the shape fed into it. When the input was
wrong, the check passed and wrote a mesh in one pose bound to a skeleton in another.

### Verifying the wrong thing

"It compiles and launches without a panic" is not verification. Neither is "the render looks
right" when a texture can hide the defect — the hand tearing question was only settled by welding
coincident vertices and measuring their spread under pose (698 groups, worst split 0.000 mm).

## The protocol

Ordered by how much they would have saved.

### 1. Build the instrument before the fix

If the answer depends on geometry, write the probe first, run it, and read it. Do not open with a
change. This project's probes — `measure_covers.py`, `tear_audit.py`, `find_the_fingers.py`,
`colour_the_digits.py` — are worth more than most of the fixes, and every one of them was written
*after* a wrong guess.

**Validate the ruler first.** If a number does not move when a knob turns, the knob is
disconnected. Check that before believing the reading.

### 2. Make identity visible, not inferred

When a step decides *what something is* — which digit, which shell, which faces — render it in
colour and look, before acting on it. Inference from summary statistics is where oscillation
starts. One labelled render beats four arguments.

### 3. Guard against the specification, never against the input

Absolute targets: soles at zero, arms at 45°, sides mirrored to half a millimetre, rest pose
unmoved to a micron, weights summing to one, four influences. A guard that compares output to
input passes happily on garbage.

And compare against something the derivation was **not** given. The two hands checking each other
caught an unstable palm normal and a stray-node knuckle, both of which looked perfectly reasonable
on one hand alone.

### 4. One change per measurement

Two changes in one build means the next number is unattributable. This is slower and it is the
only thing that makes the numbers mean anything.

### 5. Give the human something to look at, every time

Not because they are the verifier of last resort, but because they catch in one glance what takes
me an hour. The reverse also holds: **a stale viewer makes every report unreliable in both
directions.** Measured once at two hours and four rounds of changes out of date. Rebuilding the
viewer is part of rebuilding the asset, and the scene now stamps what it was built from so
staleness is visible rather than silent.

I was asked for this repeatedly — *"I need to be able to see in blender too, lol for the 12th
time"* — and kept measuring in the terminal instead. That is the single clearest instruction I
failed to internalise.

### 6. Prefer explicit APIs to context-sensitive ones

`bmesh.ops` and direct data access take arguments. `bpy.ops` acts on whatever is selected in
whatever context — and has twice destroyed geometry here while reporting success. Where an operator
is unavoidable, assert the selection took, then check the totals.

### 7. Record the wrong turns next to the code

Not sentiment — the wrong answers are load-bearing. `add_finger_bones.py` documents four failed
thumb tests with their measured numbers, so nobody re-proposes them. `prepare_rig.py` documents
why welding is forbidden despite looking free. Those comments are the difference between a lesson
and a lesson learned twice.

## For the human side

Things that measurably improve what you get back:

- **Say which of two readings you mean.** "Too thin" was ambiguous once and cost a pass in the
  wrong direction. A one-line clarification is cheaper than a wrong session.
- **Point at it.** The screenshot with lines drawn on it communicated the hand's angle instantly
  after several rounds of failing to describe it in words.
- **Push back on a tradeoff that sounds like a limit.** "The best I can do is X" often means I
  stopped researching too early. Both a measured "limit" and a "we could do X but not Y" have
  turned out to be my failure to look up the standard technique. Stride warping, spring bones and
  distance matching were all sitting in public documentation while I derived worse versions.
- **Keep scoping to a section.** The long-horizon research matches the experience: quality falls
  off with task length. One cohesive area, verified, beats a sweep.
- **`CLAUDE.md` and the memory files are the lever.** They are loaded every session. A rule
  written there survives; a rule stated in conversation does not survive the conversation.

## The one-line version

Reasoning about 3D is the weak instrument. Measuring, rendering, labelling and guarding against a
specification are the strong ones. Reach for the strong ones **first**, not after four arguments.

## Sources

- [The Observability Gap: Why Output-Level Human Feedback Fails for LLM Coding Agents (arXiv)](https://arxiv.org/html/2603.26942)
- [SWE-EVO: Benchmarking Coding Agents in Long-Horizon Software Evolution Scenarios (arXiv)](https://arxiv.org/pdf/2512.18470)
- [Spatial Reasoning in LLM Game Agents (arXiv)](https://arxiv.org/html/2607.22732)
- [Large Language Model Reasoning Failures (arXiv)](https://arxiv.org/pdf/2602.06176)
- [The Verification Horizon: No Silver Bullet for Coding Agent Rewards (arXiv)](https://arxiv.org/pdf/2606.26300)
- [Less Context, Better Agents: Efficient Context Engineering for Long-Horizon Tool-Using LLM Agents (arXiv)](https://arxiv.org/pdf/2606.10209)
- [PARC: An Autonomous Self-Reflective Coding Agent for Long-Horizon Tasks (arXiv)](https://arxiv.org/pdf/2512.03549)
- [Context Engineering for AI agents — mem0](https://mem0.ai/blog/context-engineering-ai-agents-guide)
- [L3GO: Language Agents with Chain-of-3D-Thoughts (arXiv)](https://arxiv.org/pdf/2402.09052)
- [GeoGramBench: Benchmarking Geometric Program Reasoning in Modern LLMs (arXiv)](https://arxiv.org/pdf/2505.17653)
