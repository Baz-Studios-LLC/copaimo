# Hoverboard image-generation prompt record

**Date:** 2026-09-01  
**Generator:** Codex built-in image generation  
**Purpose:** reproducible visual handoff for Claude; images are reference, not shippable source assets.

## Reference inputs

- `dev/art/shots/back1_zoom.png` — current in-game colour/material reference.
- `dev/art/golden/rest_quarter.png` — current Warden form/proportion reference.

## Final prompt — concept sheet

> Create a polished AAA game-development concept sheet for a compact foldable hoverboard designed for the
> exact stylized adventurer shown in the two reference images. Preserve the character's recognizable
> proportions, cream tunic, olive trousers, brown boots, warm orange-brown backpack, tied brown hair and
> semi-cel-shaded visual language. Show one consistent board design in clean orthographic top, side and
> front views, plus three-quarter deployed view, three-step accordion folding sequence, compact vertical
> backpack-mounted stowed state, and the character riding it in a readable balanced stance. Board dimensions
> 1.25 m long, 0.34 m wide, 0.10 m thick; three rigid hinged deck sections; wheel-less hover technology;
> olive and off-white upper deck, charcoal underside/chassis, restrained burnt-orange mechanical accents,
> thin cyan underside hover glow. Broad graphic planes, selective dark ink only on silhouette and important
> hinge/panel separations, visible bevels, no black line around every tiny bevel, no photoreal grime, no
> logos, no text, no UI, no extra characters, no weapons, no wheels. Neutral warm-gray studio background,
> even soft lighting, precise production-design clarity, consistent geometry in every view, high-resolution
> landscape sheet.

## Final prompt — deploy/stow storyboard

> Create an eight-panel animation key-pose storyboard for the same exact stylized Warden character and the
> same exact foldable hoverboard design from the supplied references. Landscape contact sheet, eight equal
> panels, left-to-right sequence, consistent camera at rear three-quarter/side game-readable angle, neutral
> warm-gray studio background, no text and no panel labels. Poses: 1 board folded vertically on backpack,
> relaxed on-foot; 2 right hand reaches behind to grip board; 3 folded board pulled free in right hand with
> weight shift; 4 board tossed low ahead while three deck sections unfold and cyan hover units activate;
> 5 lead foot plants on deck while trailing foot pushes off, hands balancing; 6 stable riding stance with
> soft knees and readable silhouette; 7 braking step-off and right-hand catch as the board folds toward the
> hand; 8 compact board locked back onto backpack and character returning upright. Preserve cream tunic,
> olive trousers, brown boots, orange-brown backpack, tied brown hair, semi-cel shading and selective ink.
> The board must stay 1.25 m by 0.34 m when open, use olive/off-white deck, charcoal underside, burnt-orange
> accents and restrained cyan glow. Emphasize believable centre-of-mass shifts, hand/prop contact and feet
> meeting the deck. No magical teleportation, no wheels, no duplicate boards, no extra characters, no
> scenery, no UI, no motion blur obscuring contact.

## Outputs

- `HOVERBOARD_CONCEPT_SHEET_2026-09-01.png`
- `HOVERBOARD_DEPLOY_STOW_STORYBOARD_2026-09-01.png`
