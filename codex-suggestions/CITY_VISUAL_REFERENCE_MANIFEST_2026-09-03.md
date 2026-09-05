# Copaimo city visual-reference manifest

Purpose: preserve the approved city concepts, their meanings, and their generation records while the game
implementation continues.

These files are reference artifacts only. Claude may inspect and cite them but should not overwrite, rename
or repurpose them. If a concept needs revision, add a versioned sibling and retain the approved original.

## Preservation status

All three approved PNG files below are present in the suggestions folder and tracked by Git in commit
5966788. The copies in this directory are authoritative; the image generator's working-output directory is
not required for recovery.

| City | Artifact | Approval/use | Bytes | SHA-256 |
|---|---|---|---:|---|
| 01 | COPAIMO_CITY_CONCEPT_MARKET_TERRACES_2026-09-03.png | user-approved beauty target | 3,389,453 | EDA0D50B11A5EDD64E2F9FB70EE1EEB0E34275AD1F326FC00689691A8FC29C80 |
| 01 | COPAIMO_CITY_LAYOUT_MASTERPLAN_2026-09-03.png | build/layout interpretation of approved target | 3,640,066 | 664FD935D16A1643BF5453FE838A9EBCDB5EA1E7DFBCB8020ECC1AD99BDEC4A5 |
| 02 | COPAIMO_CITY_02_TRANSITIONAL_MODERN_CONCEPT_2026-09-03.png | user-approved historic-to-modern target | 3,345,776 | E6D50EE850EC29BB0E5EA09512EF2B4B693C1D7E6C07E895CD1E9C7120E62415 |

## Meaning of each reference

### City 01

The nearest-city visual foundation: warm, terraced, pedestrian, market-centred and visibly shared by humans
and Copaimo. Use the beauty image for atmosphere, density, silhouette variety and lived-in public space. Use
the masterplan for route hierarchy and spatial relationships. Do not trace either image as a literal map.

Authoritative written companions:

- COPAIMO_CITY_LAYOUT_IMPLEMENTATION_GUIDE_2026-09-03.md
- COPAIMO_CITY_IMAGEGEN_PROMPTS_2026-09-03.md

### City 02

The approved transition city: old plaster, timber, tile and stone fabric grows into a contemporary civic,
garden and fabrication district using precise pale stone, restrained glass, dark metal, copper, planted roofs,
covered galleries, a public lift and useful pedestrian bridges. It is more advanced than City 01 but retains
substantial visual headroom for Cities 03–07.

Authoritative written companions:

- COPAIMO_CITY_02_MODERN_TRANSITION_AND_PROGRESSION_2026-09-03.md
- COPAIMO_CITY_02_IMAGEGEN_PROMPT_2026-09-03.md

## Future City 03–07 artifact contract

Do not generate the next city's final concept until Claude has completed the requested City 01 and City 02
proofs or the user explicitly asks to proceed sooner. When continuing:

1. preserve these approved images unchanged;
2. use City 01 and City 02 as progression anchors, not edit targets;
3. give each later city a different purpose, plan, biome response, landmark and building-family set;
4. increase technological maturity outward from the ranch without reducing pedestrian life or historical
   depth;
5. show the new image to the user before adding it to Claude's reference set;
6. after approval, save the final PNG, exact prompt, implementation guide and checksum in this directory;
7. use filenames beginning COPAIMO_CITY_03 through COPAIMO_CITY_07 and a date/version suffix;
8. never overwrite an earlier approved version.

## Verification

If an image appears damaged or unexpectedly changed, compare its SHA-256 against the table above and recover
the tracked copy from commit 5966788. Do not replace it with a newly generated approximation under the same
filename.
