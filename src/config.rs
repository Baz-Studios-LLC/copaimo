//! Every knob that controls the size and shape of the world lives here.
//!
//! The world is *finite by construction*: its landmass comes from a source map
//! image (`assets/world/heightmap.png`). Anything outside that image is open
//! ocean, so the map ends in water rather than at an invisible wall.
//!
//! Two things are separate on purpose:
//!   * the map image decides the **shape** (where land is, how high, aspect ratio)
//!   * `WORLD_WIDTH` decides the **scale** (how many meters that shape covers)
//!
//! So re-rolling the map in the generator swaps the continent without touching
//! code, and resizing the game world is a single number.

// ---------------------------------------------------------------- world scale

/// How wide the world is, in meters, east to west. This is *the* scale knob.
///
/// The north–south size is derived from the map image's aspect ratio, so a
/// 2:1 map at 8192 m wide is 4096 m deep. At the player's 7 m/s jog that's
/// about 20 minutes corner to corner — long enough for towns to sit between
/// cities and still feel like a journey.
pub const WORLD_WIDTH: f32 = 8192.0;

/// Aspect ratio (width / depth) assumed if no map image is present and we fall
/// back to pure procedural generation.
pub const FALLBACK_ASPECT: f32 = 2.0;

/// Path to the source elevation map, relative to the crate root. Grayscale or
/// color both work — brightness is read as elevation.
pub const HEIGHTMAP_PATH: &str = "assets/world/heightmap.png";

/// Where hand-sculpted terrain edits are saved, relative to the crate root.
pub const EDITS_PATH: &str = "assets/world/edits.bin";

/// Resolution of the hand-edit layer, in meters per cell. Fine enough to shape
/// an individual hill, coarse enough that the whole world's edit layer is a few
/// megabytes and can be copied cheaply.
pub const EDIT_CELL: f32 = 4.0;

/// Seed for the detail noise layers. Change it to reshuffle local terrain
/// without changing the continent outline (which comes from the image).
pub const WORLD_SEED: u32 = 20_260_813;

// ---------------------------------------------------------- chunk / streaming

/// A terrain chunk covers `CHUNK_SIZE` × `CHUNK_SIZE` meters.
pub const CHUNK_SIZE: f32 = 128.0;

/// Quads along one chunk edge. Vertex spacing = `CHUNK_SIZE / CHUNK_QUADS`,
/// so 64 quads over 128 m gives a 2 m grid — fine enough to read hills and
/// cliffs, coarse enough that a chunk builds in a couple of milliseconds.
pub const CHUNK_QUADS: u32 = 64;

/// Chunks stay loaded within this many chunks of the viewer (a radius, in
/// chunks). 9 × 128 m ≈ 1150 m of visible ground.
///
/// There is no distance fog, so this radius *is* the horizon — terrain stops
/// at the edge of it rather than dissolving into haze. Raising it pushes that
/// edge back but costs chunks by the square of the radius, so it's the first
/// thing to reconsider if the frame rate drops. Distance-based mesh LOD is the
/// real answer to seeing further; this is the cheap one.
pub const VIEW_CHUNKS: i32 = 9;

/// How many chunk meshes may be building on background threads at once. Caps
/// the work spike on first load and when the player moves fast.
pub const MAX_PENDING_CHUNKS: usize = 24;

// ------------------------------------------------------------------ elevation

/// Water surface height. Everything is measured relative to this, so "y < 0 is
/// underwater" holds everywhere in the codebase.
pub const SEA_LEVEL: f32 = 0.0;

/// Brightness in the source map that counts as the waterline, 0..1. Pixels
/// darker than this are sea floor, brighter are land.
///
/// **This depends on what kind of map you export.** A true grayscale heightmap
/// puts sea level low, around 0.20. A *colored political map* is effectively
/// two-tone — mid-brightness ocean, bright land fills — so the threshold has to
/// sit in the gap between them, around 0.50. Set wrong, whole oceans read as
/// land. `cargo test -- --nocapture` prints the map so you can see which way
/// it went.
pub const MAP_SEA_THRESHOLD: f32 = 0.74;

/// Cleanup applied to the land/sea mask before it's used, in map pixels.
///
/// Real maps are covered in line work — region borders, rivers, roads, labels.
/// Those are dark pixels, and taken at face value they carve trenches across
/// the continents. A majority filter outvotes anything thinner than its radius
/// while leaving coastlines (land on one side, sea on the other) exactly where
/// they are. Raise the radius or the pass count if thicker lines survive.
pub const MASK_CLEAN_RADIUS: usize = 3;
pub const MASK_CLEAN_PASSES: usize = 2;
/// Softens the cleaned mask's hard edge into a coastal ramp.
pub const MASK_BLUR_RADIUS: usize = 2;

/// When true, land is one flat plateau and sea one flat shelf — no generated
/// relief at all.
///
/// This is the shape-checking mode, and the natural companion to the sculpting
/// tool: the map gives you the continents, and every hill and mountain on them
/// is one you put there. Hand edits still apply on top, so a flat world is a
/// canvas rather than a locked one.
pub const FLAT_WORLD: bool = true;

/// Height of the plateau in flat mode. Low enough to read as lowland, high
/// enough to sit clearly above the waterline.
pub const FLAT_LAND_HEIGHT: f32 = 18.0;

/// Height of land where the map is at maximum brightness, before ranges.
/// Unused while `FLAT_WORLD` is on.
pub const BASE_ELEVATION: f32 = 110.0;

/// Extra height the broad mountain-range layer can stack on high ground.
pub const RANGE_ELEVATION: f32 = 190.0;

/// Amplitude of the fine surface detail layer (bumps, small undulations). This
/// is what keeps a low-resolution source map from looking like smooth putty
/// when you're standing on it.
pub const DETAIL_ELEVATION: f32 = 8.0;

/// How deep the sea floor sinks where the map is fully black.
pub const OCEAN_DEPTH: f32 = 60.0;

/// Height at which snow has fully taken over. Surface coloring only.
pub const SNOW_LINE: f32 = 210.0;

// ---------------------------------------------------------------- noise shape

/// Frequency of the mountain-range layer (controls how far apart ranges sit).
/// Low, so ranges are broad masses you walk over the shoulder of — not peaks
/// you walk between.
pub const RANGE_FREQ: f64 = 0.000_45;
/// Frequency of the fine detail layer.
pub const DETAIL_FREQ: f64 = 0.009;
/// Frequency of the moisture field that decides dry plains vs. lush forest.
pub const MOISTURE_FREQ: f64 = 0.000_9;
/// Frequency of the warp applied to map lookups. Nudges the sampled position
/// slightly so coastlines wiggle naturally instead of showing the map image's
/// straight pixel edges.
pub const WARP_FREQ: f64 = 0.004;
/// How far, in meters, that warp can displace a lookup.
pub const WARP_STRENGTH: f32 = 26.0;

// ---------------------------------------------- procedural fallback (no image)

/// Frequency of the broad landmass mask used only when there's no map image.
pub const CONTINENT_FREQ: f64 = 0.000_35;
/// Where the fallback coastline fade begins, as a fraction of the distance from
/// the world center to its border.
pub const COAST_FADE_START: f32 = 0.78;
