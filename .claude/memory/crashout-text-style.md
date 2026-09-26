---
name: crashout-text-style
description: CRASHOUT in-game text should avoid random British slang — keep it plain/American-neutral office-comedy
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 0e338b91-d93f-4e1b-ae02-1da4bb0cf769
---

The user wants CRASHOUT's in-game text (toasts, coworker dialogue, floaters, event messages) kept clear of **random British slang**. They flagged "The place is a tip" (changed to "The place is trashed — mess everywhere!").

**Why:** It reads as out-of-place; the game's voice is plain American office-comedy.

**Toasts must be ACCURATE** — a toast must reflect what actually happened. The user flagged "an angry client stormed in" firing when no client spawned (the `escalation`/`doubleBooked` events spawn a client only `if !client && sinceClient>8` but used to toast unconditionally). When writing an event that announces a thing, only toast that thing when it actually occurs; otherwise toast what DID happen.

**How to apply:** When writing or editing any user-facing string (GOSSIP/CHAT_TPL/MEETING lines, toasts, floaters, event text), avoid Briticisms (tip, knackered, gutted, chuffed, naff, dodgy, quid, fortnight, etc.). "bin" is APPROVED by the user (keep it) — used in the trash-disposal floaters ("to the bin!", "binned!"); it's part of the core trash vocabulary, do not flag/change it. Relates to [[crashout-design-log]].
