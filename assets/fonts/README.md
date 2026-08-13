# Fonts

Drop a `.ttf` here named **`ui.ttf`** and the terrain tool uses it for its whole
interface. Without one it falls back to Bevy's built-in face, which still works
but is a narrow subset.

## Why you'd bother

Bevy embeds a *subset* font covering little more than ASCII. Typographic
characters — `·`, `—`, `→`, `×` — render as empty boxes. The tool's UI is
written in plain ASCII and builds its structure out of layout (thin rule nodes,
meter bars, boxed keycaps) rather than punctuation, so it looks correct either
way. A real font mainly buys you nicer letterforms and proper proportional
spacing.

## Picking one

Anything with a permissive license, since this tool is used across projects:

* **Inter** — SIL Open Font License, designed for UI at small sizes
* **IBM Plex Sans** / **Plex Mono** — SIL OFL
* **JetBrains Mono** — SIL OFL, good if you want the numbers to line up
* **Roboto** — Apache 2.0

Avoid shipping Windows system fonts (Segoe UI, Consolas, Tahoma). They're
licensed for use on the machine, not for redistribution inside a tool.

Keep the file named `ui.ttf`; the loader checks that exact path and stays quiet
if it isn't there.
