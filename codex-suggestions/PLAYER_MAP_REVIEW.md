# Copaimo suggestions for Claude

This folder is review-only. Codex did not change the game code or assets.

## Player map review

### 1. Make Escape close only the map

`src/map.rs:80` and `src/states.rs:137` both handle Escape while the game is in
the Playing state. When the map is open, the same keypress can close the map and
also send the player back to the menu, despite the map's stated behavior.

Suggestion: prevent the global `escape_to_menu` handler from running while
`map::Open` is true. Add an integration test that opens the map, presses Escape,
and verifies that the map closes while the state remains Playing.

### 2. Treat the map as a modal screen

The map covers the game, but `move_player`, `orbit_input`, and the camera controls
remain active. A player can therefore walk without seeing the world and rotate or
zoom the camera behind the map.

Suggestion: gate gameplay input while `map::Open` is true. Camera follow may still
run if needed, but movement, orbit, zoom, fly controls, and unrelated hotkeys
should not respond through the map.

### 3. Invalidate the cached painting after world edits

`Chart::asked` is set once and never reset, and an in-flight `Painting` task can
survive leaving Playing. In the default tools build, a maker can open the map,
return to the terrain editor, change terrain or countries, and then re-enter the
game with the old map still cached.

Suggestion: associate the chart with a terrain revision, or discard the cached
image and any obsolete painting task when entering or leaving modes where the
world can change. Ensure one painting represents one consistent terrain snapshot.

### 4. Show direction as promised

The module documentation says the player map shows where the warden stands and
which way the warden faces, but `YouAreHere` is currently only a dot. The existing
terrain overview already has a proven heading needle and a "N is up" caption.

Suggestion: reuse or extract that heading behavior. Decide explicitly whether the
needle represents the warden's facing or the camera's facing, because those can
differ during orbiting.

### 5. Strengthen the map-pixel test

`the_map_shows_what_people_built` accepts any settlement-marker color at every
site and samples only the center. It therefore does not verify the correct
city/town/ranch symbol, and it cannot prove that the pale ring exists.

Suggestion: assert the expected center color for each site type, then inspect the
nearby pixel neighborhood for the ring. Road samples hidden by known marks or
bridges can be excluded explicitly instead of allowing a large fraction to be
missing.

### 6. Give the first opening a loading state

On the first M press, the UI shows nothing until the asynchronous painting is
complete. On a slower machine this can look as though M did not work.

Suggestion: raise the dimmed map shell immediately and show a brief "Drawing
map..." message until the image is ready.

