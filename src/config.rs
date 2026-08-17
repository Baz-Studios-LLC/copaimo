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

/// Woods planted or cleared here or at Opificium's terrain bench.
pub const FOREST_PATH: &str = "assets/world/forest.bin";

/// What the ground is made of where somebody laid it: roads, yards, worn earth.
pub const SURFACE_PATH: &str = "assets/world/surface.bin";

/// Buildings baked at Opificium's builder — houses, signs, bridges, all boxes.
/// Read at startup; an absent folder is a game whose buildings are not drawn yet.
pub const BUILDINGS_DIR: &str = "assets/buildings";

/// Resolution of the hand-edit layer, in meters per cell. Fine enough to shape
/// an individual hill, coarse enough that the whole world's edit layer is a few
/// megabytes and can be copied cheaply.
///
/// Taken from the shared crate rather than written down again: it is the shape
/// of a file two programs both read and write, so a second copy of the number
/// is a second chance to disagree.
pub const EDIT_CELL: f32 = terrain_core::sculpt::CELL;

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

/// How far ground cover reaches from the viewer, in chunks.
///
/// Far short of `VIEW_CHUNKS`, and deliberately. Terrain streams to the horizon
/// because that is what a horizon is; grass cannot follow it — a chunk holds two
/// thousand tufts, so dressing the streamed world would be seven hundred
/// thousand of them. It would also be wasted, since a forty-centimetre blade is
/// invisible past a hundred metres. Two chunks is about 320 m of dressed ground,
/// which is further than anyone can tell.
pub const COVER_CHUNKS: i32 = 2;

/// How many chunks may be having their cover built at once.
pub const MAX_PENDING_COVER: usize = 6;

/// Whether the world carves rivers into itself at all.
///
/// **Off.** Kept as a switch rather than torn out, the same bargain the roads
/// between towns get: the machinery is written, tested and shared with the
/// bench, and none of it is in the way while it is not running.
///
/// What killed them was WIDTH, and it is worth writing down because it is the
/// thing to fix if they come back. A channel's cut spreads over three times its
/// own width, because banks do — so water filled to any useful depth spreads
/// about that far as well, and a river drawn eighteen metres wide arrives on
/// screen at sixty. Across the whole network that came to a fifth of the land
/// under water: not rivers through a landscape, a landscape with a lake on it.
///
/// The levers, in the order they matter: `BANKS` in the crate, which is what
/// makes the cut spread; `RIVER_EDGE` below, which is where the waterline sits
/// on that spread and cannot simply be tightened without the surface's edge
/// going ragged again; and `NARROWEST`, which sets the smallest channel and was
/// raised to eighteen metres because anything less could not be drawn at all.
/// Any two of those pull against the third, which is the actual problem and is
/// not a tuning pass.
pub const RIVERS: bool = false;

/// Metres between samples when the rivers are worked out.
///
/// The whole map is sampled at this spacing once at load, so it is a straight
/// trade: finer finds smaller creeks and costs more startup. Twenty metres over
/// an eight-kilometre world is about eighty thousand samples, which is under a
/// second, and is fine enough that a seven-metre channel still lands on several
/// cells rather than falling between two.
pub const RIVER_SPACING: f32 = 20.0;

/// Quads along a chunk edge when the river surface is drawn.
///
/// The same as the ground's, and it has to be. This was a quarter of it, on the
/// reasoning that still water is flat and a flat thing needs no vertices — but
/// the surface is not flat any more. It follows the bed beneath it, so it needs
/// every vertex the bed has or it cannot follow anything.
///
/// It is also what draws the water's EDGE. At eight metres a river's bank came
/// out as a flight of eight-metre stairs, which is not what a riverbank looks
/// like from any distance at all.
///
/// The cost is nothing. Rivers cover a hundredth of the world and a quad is only
/// emitted where there is water under it, so most chunks build no river mesh at
/// all and the ones that do build a ribbon.
pub const RIVER_QUADS: u32 = 64;

/// How deep the cut must be before water is drawn standing in it, in metres.
///
/// A river's banks reach several times its own width, so most of what a channel
/// touches is bank rather than bed. Drawing water wherever anything was cut at
/// all put sheets of it across whole beaches.
pub const CHANNEL_LEAST: f32 = 0.45;

/// How far up its own channel a river fills, 0 empty to 1 brimming.
///
/// A FRACTION, not a depth, and that is the whole of what keeps water off the
/// grass. The surface is drawn this far up the channel that is actually still
/// cut into the ground at that point — so a quarter of the channel is always
/// bank, whatever the channel happens to be, and the water cannot be above the
/// land beside it however flat or steep the country is.
///
/// A fixed depth in metres could not do that. It was 0.7 m, and where a channel
/// was shallower than 0.7 m the remainder stood proud of the field. Worse, a
/// town levelling its ground FILLED a channel in and the water carried on being
/// drawn at the old depth: 787 of the 804 slabs on open grass were sitting on a
/// town's flat.
///
/// It also gives a big river deeper water than a creek, which a fixed depth
/// never did.
pub const RIVER_FILL: f32 = 0.75;

/// How far down a channel's profile the water's edge sits, 0 bank to 1 floor.
///
/// Where the drawn surface ENDS. The cut spreads out over the banks — several
/// times the channel's own width — so drawing to the edge of it would put a
/// river across its own floodplain. Just over half way down the profile is the
/// shoulder of the channel proper.
///
/// The surface fades to nothing as it reaches this rather than stopping at it.
/// Stopping is what leaves a step of water standing in the air, which is what a
/// slab of river on dry grass is.
pub const RIVER_EDGE: f32 = 0.55;

/// The heights above the sea across which a river fades into it, in metres.
///
/// The third of a river's three edges, and the one that gets forgotten. A
/// surface that follows its bed rises with the bed — so a river running down to
/// the coast arrives at the waterline still carrying its own depth, and ends in
/// a step nearly two metres above the sea it is running into.
///
/// Measured in HEIGHT rather than in distance, because that is what the problem
/// is made of. A river reaching a steep shore drops a metre in two, so a fade
/// spread over any fixed distance is crossed in one stride; a fade spread over
/// height is crossed at the same point on every shore, gentle or sheer.
///
/// Nothing below the first of these. Down there you are looking at the sea, and
/// the ground between is the flat a river leaves at its own mouth.
pub const RIVER_MOUTH_LOW: f32 = 1.0;
pub const RIVER_MOUTH_HIGH: f32 = 6.0;

// ---------------------------------------------------------------------- sky

/// How many clouds hang over the world at once.
///
/// They are wrapped around the VIEWER rather than scattered over the map: the
/// world is eight kilometres across and eighty clouds spread over that would
/// leave the sky empty, where eighty in a box that follows you is a dressed sky
/// wherever you stand.
/// Eighty of these filled the sky when they were specks four hundred metres up.
/// Brought down to a hundred and sixty-five and drawn four times bigger, each one
/// covers many times the sky it did — and eighty of THOSE is an overcast day
/// every day. Thirty reads as weather.
pub const CLOUDS: usize = 30;

/// How high the cloud base sits, in metres, and how far the box reaches.
///
/// Far lower than it was. At four hundred metres a cloud subtends almost nothing
/// — it read as a speck of litter rather than as weather, however large the mesh
/// actually was. Height and size are one problem: what matters is how much sky a
/// cloud covers, and the cheapest way to cover more of it is to come down.
pub const CLOUD_CEILING: f32 = 165.0;
pub const CLOUD_SPREAD: f32 = 2_000.0;

/// How much bigger than its grown size a cloud is drawn.
///
/// The mesh comes out a few tens of metres across, which is the size of a real
/// cloud's puff and nothing like the size of a cloud. Scaled up here rather than
/// grown bigger so the shape stays the shape.
pub const CLOUD_SCALE: f32 = 4.5;

/// How fast they drift, in metres a second. Slow enough to be weather rather
/// than traffic — a cloud should cross the sky in minutes, not seconds.
pub const CLOUD_DRIFT: f32 = 3.5;

/// How much light a cloud's shadow takes off the ground under it.
///
/// A third, and it is meant to be read as weather passing rather than as dusk
/// arriving. The shade lands on everything a surface receives — sun and sky
/// both, which is what a cloud actually blocks — so it goes further than a third
/// of the sunlight would on its own.
pub const CLOUD_SHADE: f32 = 0.45;

/// Where a shadow's soft rim begins, as a share of its radius.
///
/// A cloud does not have an edge; it has a place where it runs out, and two
/// hundred metres of air blurs whatever is left of that. Anything crisper reads
/// as a painted circle on the grass.
///
/// Raised from a third, because at a third a small cloud was ALL rim — it never
/// reached full strength anywhere, so half the sky cast shadows you could barely
/// see. At a half there is a solid middle to every one of them.
pub const CLOUD_SHADE_SOFT: f32 = 0.5;

/// The sun heights the shadows fade in and out across.
///
/// Cloud shadows are a midday thing here, and that is honest rather than a
/// dodge. A cloud a couple of hundred metres up with the sun near the horizon
/// casts its shadow more than a kilometre sideways — so the shade over your head
/// belongs to a cloud you cannot see, and the cloud you CAN see is shading
/// somewhere off past the hills. It is also the hour when the light is flattest
/// and a shadow on the ground reads least.
pub const CLOUD_SHADE_FROM: f32 = 0.08;
pub const CLOUD_SHADE_TO: f32 = 0.30;

/// How many stars fill the night sky.
///
/// One mesh holds all of them — a star apiece would be a thousand entities to
/// draw something nobody looks at directly.
pub const STARS: usize = 900;

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

/// How much bluer than red a pixel must be to count as water on a *colored*
/// map, in 0-255 units.
///
/// This is what a political map is actually classified by, because brightness
/// cannot tell open water from a black place name, a road, or a dashed border —
/// they're all dark, and a brightness threshold cuts every label on the map
/// into the terrain as a lake. Water is the one thing that's distinctly blue.
/// Measured on the supplied map: ocean sits at 48-80, every land fill at 32 or
/// below, so 40 separates them with room on both sides.
pub const MAP_SEA_BLUE_MARGIN: i16 = 40;

/// Cleanup applied to the land/sea mask before it's used, in map pixels.
///
/// Real maps are covered in line work — region borders, rivers, roads, labels.
/// Those are dark pixels, and taken at face value they carve trenches across
/// the continents. A majority filter outvotes anything thinner than its radius
/// while leaving coastlines (land on one side, sea on the other) exactly where
/// they are. Raise the radius or the pass count if thicker lines survive.
pub const MASK_CLEAN_RADIUS: usize = 3;
pub const MASK_CLEAN_PASSES: usize = 2;
/// How far inland, in meters, the land takes to rise from the waterline to
/// `COAST_HEIGHT`, and how far out to sea the floor takes to fall to
/// `OCEAN_DEPTH`.
///
/// **These exist because a coast has to shelve.** Without them the whole drop
/// from land to sea floor happened across a few meters, and no vertex grid can
/// draw that: neighboring vertices land on opposite sides of it and every
/// coastline comes out as a fence of vertical slats. Spread over a beach and a
/// shelf, the change per cell is small and a coast reads as a coast.
pub const BEACH_WIDTH: f32 = 90.0;

/// How much of the shore's rise is a quadratic toe rather than a smoothstep.
///
/// The fix for a sea that walked up the sand. A smoothstep is flat at both ends,
/// so the ground had no slope at all exactly where the water meets it — and how
/// far a tide sweeps is its height over that slope. Mixing in a curve that has
/// slope at the bottom and none at the top gives the shore about six degrees at
/// the waterline, which is a beach, and leaves the top of the ramp flat so there
/// is no crease where it meets the land.
pub const BEACH_TOE: f32 = 0.3;
pub const SHELF_WIDTH: f32 = 600.0;

/// Smallest land blob kept, in map pixels. Anything smaller is deleted as
/// furniture rather than geography — screenshots carry buttons, scale bars and
/// legends, none of which are water-colored, and all of which would otherwise
/// become tiny rectangular islands. Real islands are far larger.
pub const MIN_ISLAND_PIXELS: usize = 900;

/// When true, land is one flat plateau and sea one flat shelf — no generated
/// relief at all.
///
/// The shape-checking mode: turn it on to see nothing but the outline of the
/// continents. Hand edits still apply on top, so it's a canvas, not a lock.
pub const FLAT_WORLD: bool = false;

/// Height of land at the shoreline. Low enough to read as coastal plain, high
/// enough to sit clearly above the waterline. Also the plateau height in flat
/// mode.
pub const COAST_HEIGHT: f32 = 16.0;

/// How much the land rises from the coast to the deep interior. Gives coastal
/// plains that climb into uplands, which is where ranges then sit.
pub const INLAND_RISE: f32 = 28.0;

/// Distance from the coast, in meters, at which land counts as fully inland.
/// Everything geographic — the rise above, where mountains are allowed — is
/// measured against this, so it sets how far you walk before the country
/// starts to feel like an interior.
///
/// **Must be checked against the map.** `cargo test -- --nocapture` prints how
/// far the furthest point on the current map gets from any coast. Set this
/// above that number and nothing ever counts as inland, so the mountains
/// silently never appear — which is exactly what happened at 1100 m on a map
/// whose deepest interior is 820 m.
pub const INLAND_FULL: f32 = 620.0;

/// Height of land where the map is at maximum brightness. Only used when the
/// source is a true grayscale heightmap carrying real elevation.
pub const BASE_ELEVATION: f32 = 110.0;

// ------------------------------------------------------------------ mountains

/// Peak height a range can add on top of the inland rise.
pub const RANGE_ELEVATION: f32 = 52.0;

// ------------------------------------------------------------------- the woods

/// Meters between the slots a tree may stand in.
///
/// The single knob for how thick a forest is. Trees go up as the **square** of
/// it, so 14 m is a quarter the trees of 7 m — reach for this first.
///
/// **Must match Opificium's `tree_spacing`.** Both programs work the forest out
/// from scratch and never exchange a list of trees, so a difference here gives
/// the bench one forest and the game another, silently.
pub const TREE_SPACING: f32 = 14.0;

/// Height at which trees give out. Below `MASSIF_HEIGHT`, so the great mountain
/// stands bare above its own treeline.
pub const TREELINE: f32 = 150.0;

/// Height at which the tops turn to snow.
///
/// Between `TREELINE` and `MASSIF_HEIGHT`, so the great mountain wears a cap and
/// nothing else in the world reaches one. A range at `RANGE_ELEVATION` never gets
/// close, which is the point — snow should mean THE mountain.
///
/// Lowered from 250. At that height only the last quarter of the mountain was
/// white, and now that its flanks are cut into spurs and gullies rather than a
/// smooth shell, the snow had almost nothing to sit on — it read as a dusting on
/// a summit rather than as high country. At 165 it comes well down the spurs and
/// onto the shoulders, and the treeline at 150 still sits below it, so there is a
/// band of bare rock between the last trees and the first snow. That band is what
/// makes high ground look high.
pub const SNOWLINE: f32 = 165.0;

/// Slope past which ground is bare stone rather than anything growing.
pub const ROCK_SLOPE: f32 = 0.62;

/// Moisture below which land is desert, and above which it closes into forest.
///
/// These two decide what sort of continent this is. Widening the gap makes a
/// world of open grassland; closing it makes everywhere either wood or sand.
pub const DESERT_MOISTURE: f32 = 0.38;
pub const FOREST_MOISTURE: f32 = 0.58;

/// Metres from the coast within which ground counts as beach.
///
/// The same number the forest already refuses to grow inside, named once so the
/// treeline of a beach and the edge of the shore biome cannot drift apart.
pub const SHORE_WITHIN: f32 = 25.0;

/// How much levelling makes ground somebody's rather than nobody's.
pub const SETTLED_LEVELLING: f32 = 0.62;

/// How much bigger or smaller than grown a planted tree may be, so a stand has
/// young trees and old ones in it.
pub const TREE_SCALE_LOW: f32 = 0.75;
pub const TREE_SCALE_HIGH: f32 = 1.35;

// -------------------------------------------------------------------- the ranch

/// Where the player's farm stands, in world metres, and how far its level ground
/// reaches.
///
/// **Chosen by hand, not found.** Every other place on the map is worked out
/// from the ground — towns go wherever the land allows — but the ranch is where
/// the game begins and ends up, so it is picked by eye and pinned. This spot was
/// chosen at Opificium's terrain bench: a gentle shelf at about 23 m, inland of
/// the coast, on the western landmass.
///
/// It is levelled first, before any town is planned, so nothing else can be
/// placed on top of it. The ranger starts standing on it.
///
/// Exported in `world.json`, because the bench must level the same ground or a
/// farm sculpted there would sit at the wrong height in the game.
pub const RANCH_AT: (f32, f32) = (-3064.0, 659.0);
pub const RANCH_RADIUS: f32 = 130.0;

/// The one great mountain: how high it stands above the ground it sits on, and
/// how far out its foot reaches.
///
/// The world is otherwise deliberately gentle — plains and hills you walk over
/// rather than around. That makes ONE massif worth more than a map full of
/// them: it's visible from most of the continent, it's what you navigate by,
/// and it's somewhere you decide to go. `RANGE_ELEVATION` stays low so this
/// reads as the exception it is.
///
/// It stands at the point furthest from any sea — the heart of the largest
/// landmass. Found rather than chosen, so redrawing the map moves the mountain
/// to the new map's interior instead of stranding it in a bay.
///
/// Set `MASSIF_HEIGHT` to 0 for a world with no such landmark.
pub const MASSIF_HEIGHT: f32 = 340.0;
pub const MASSIF_RADIUS: f32 = 950.0;

/// How far the foothills reach, as a multiple of the mountain's own radius.
///
/// A mountain that stops at its own edge stands up out of a plain like a boil,
/// which is what this one did. Real high ground has broken country around it.
pub const MASSIF_SKIRT: f32 = 2.1;

/// How deeply the gullies cut into the mass, 0 to 1.
///
/// The dome was modulated by a fifth, which is a bulge rather than a mountain.
/// Cutting nearly half of it away along the creases gives spurs with real
/// valleys between them — and, just as importantly, faces steep enough to count
/// as rock, so the whole thing is no longer uniformly under snow.
pub const MASSIF_RELIEF: f32 = 0.45;

/// How high the foothills stand, as a fraction of the mountain.
pub const MASSIF_FOOTHILLS: f32 = 0.16;

/// Frequency of the ridge lines. Low, so a crest runs for kilometers — this is
/// the number that decides whether you get mountain *ranges* or a rash of
/// bumps.
pub const RANGE_FREQ: f64 = 0.000_42;

/// Frequency of the field deciding *where* ranges exist at all. Low, so
/// mountains occupy a few regions of the map rather than being its texture —
/// but not so low that the whole world gets one verdict. Around three cycles
/// across the map gives a handful of distinct mountainous regions.
pub const RANGE_PRESENCE_FREQ: f64 = 0.000_35;

/// How much of the presence field becomes mountainous. Higher leaves more of
/// the world as open country.
pub const RANGE_PRESENCE_CUTOFF: f32 = 0.45;

/// Fraction of `INLAND_FULL` before mountains may start, and where they reach
/// full height. Keeps ranges off the coast, where plains and beaches belong.
pub const RANGE_INLAND_START: f32 = 0.25;
pub const RANGE_INLAND_FULL: f32 = 0.70;

/// Amplitude of the fine surface detail layer (bumps, small undulations). This
/// is what keeps a low-resolution source map from looking like smooth putty
/// when you're standing on it.
pub const DETAIL_ELEVATION: f32 = 8.0;

/// How deep the sea floor sinks where the map is fully black.
pub const OCEAN_DEPTH: f32 = 60.0;

/// Height at which snow has fully taken over. Surface coloring only.
pub const SNOW_LINE: f32 = 210.0;

// ---------------------------------------------------------------- noise shape

/// Frequency of the fine detail layer.
pub const DETAIL_FREQ: f64 = 0.009;
/// Frequency of the moisture field that decides dry plains vs. lush forest.
pub const MOISTURE_FREQ: f64 = 0.000_9;
/// How often the character of the coast changes — beach here, rock there. Low,
/// so a beach runs the better part of a kilometer before giving way.
///
/// Not exported in `world.json`: this decides how the ground is *colored*, and
/// color has no bearing on the offsets sculpted at the bench. Opificium keeps
/// the same number so the two look alike, but nothing breaks if they drift.
pub const SHORE_FREQ: f64 = 0.000_6;
/// Frequency of the warp applied to map lookups. Nudges the sampled position
/// slightly so coastlines wiggle naturally instead of showing the map image's
/// straight pixel edges.
pub const WARP_FREQ: f64 = 0.004;
/// How far, in meters, that warp can displace a lookup.
pub const WARP_STRENGTH: f32 = 26.0;

// ---------------------------------------------- procedural fallback (no image)

/// Frequency of the broad landmass mask used only when there's no map image.
pub const CONTINENT_FREQ: f64 = 0.000_35;
/// Where the coastline fade begins, as a fraction of the distance from the
/// world center to its border.
///
/// Applied to the map as well as the procedural fallback, because "the world
/// ends in water, not a wall" has to hold whatever the source image says at its
/// own edges — a screenshot's UI chrome lives exactly there. Kept very close to
/// the border so it trims furniture without eating real coastline.
pub const COAST_FADE_START: f32 = 0.95;

// ----------------------------------------------------------- level ground

/// How much of the world is rugged, and how much is level.
///
/// A very low-frequency field, thresholded: below `RUGGED_LOW` the ground is
/// plain — flat enough for forest, farmland and walking — and only above
/// `RUGGED_HIGH` does it get the full detail and the mountains.
///
/// Without this every square meter of the map was equally lumpy, which leaves
/// nowhere for anything to happen. A world needs somewhere level to put things.
pub const RUGGED_FREQ: f64 = 0.000_25;
pub const RUGGED_LOW: f32 = 0.38;
pub const RUGGED_HIGH: f32 = 0.72;
/// Relief left in the flattest country, so plains still read as ground rather
/// than as a table.
pub const PLAINS_RELIEF: f32 = 0.12;

// ------------------------------------------------------------- settlements

/// How many of each kind of place gets ground leveled for it. Cities hold the
/// Ranger Guild exams; towns are the smaller places between them.
pub const CITIES: usize = 6;
pub const TOWNS: usize = 14;

/// How far the level ground reaches at each, in meters.
pub const CITY_RADIUS: f32 = 190.0;
pub const TOWN_RADIUS: f32 = 95.0;

/// How far apart they must stand.
pub const CITY_SPACING: f32 = 1_100.0;
pub const TOWN_SPACING: f32 = 420.0;

/// How far the leveling eases back into the surrounding land, so a town sits
/// *in* the ground rather than on a plinth.
pub const SITE_SKIRT: f32 = 140.0;

/// A site must be at least this far inland, below this height, and on ground no
/// steeper than this — people build where the living is, and leveling a
/// hillside would leave a scar visible from orbit.
pub const SITE_MIN_INLAND: f32 = 70.0;
pub const SITE_MAX_HEIGHT: f32 = 130.0;
pub const SITE_MAX_SLOPE: f32 = 0.13;

/// Half-width of the graded road between sites, and its shoulders.
pub const ROAD_WIDTH: f32 = 9.0;
/// How far a road eases back into the land either side of its bed.
///
/// Wider than it was. A road that follows the country cuts far less than one
/// graded on a straight line, but where it still cuts, twenty-six metres was
/// not enough to blend the wall — the eye reads that as a trench side.
pub const ROAD_SKIRT: f32 = 44.0;

/// How far a cutting's sides are battered, per metre of its depth.
///
/// The fixed skirt above is what turned roads into gorges. A road that has to
/// cross a ridge holds its grade and cuts through, which is what a road does —
/// but the cut has to resolve into the land over a FIXED forty-four metres
/// however deep it is, so a thirty-metre cut came out with thirty-four-degree
/// walls. That reads as a canyon somebody blasted, not a road somebody built.
///
/// Real cuttings are battered in proportion to their depth, because soil has an
/// angle it will hold. Three metres of skirt per metre of depth is about
/// eighteen degrees — a slope you can walk up, and a cut you can see is a cut.
pub const ROAD_BATTER: f32 = 3.0;

/// Whether the world lays its own roads between towns.
///
/// **Off, because roads are authored now.** The generator's roads were the source
/// of the gorges: a graded run holds its grade across a ridge and cuts through,
/// and however carefully the sides are battered the result is a machine's answer
/// to a question that wants a person's. The PATH brush lays a road that follows
/// the country because somebody is looking at the country while they lay it.
///
/// The machinery below is kept, tested, and one word from returning — this is a
/// switch rather than a deletion, so turning towns back into a linked network is
/// changing `false` to `true`.
pub const LINK_TOWNS_WITH_ROADS: bool = false;

/// The widest a cutting's skirt may reach, in metres.
///
/// A cap is not tidiness, it is correctness. Roads are filed in a coarse grid so
/// a height lookup tests only the features near it, and a road is filed by the
/// ground it can possibly reach. Battering the sides without raising that reach
/// meant a road stopped being FOUND past the old fifty-three metres while its
/// skirt still wanted to pull — so the pull fell to nothing in one step and left
/// a wall parallel to every road. Which is more gorges, not fewer.
///
/// So the reach is this, the skirt is clamped to it, and the two cannot disagree.
/// A cut deeper than about forty metres gets a slightly steeper wall than the
/// batter asks for, which is a fair trade for a bounded lookup.
pub const ROAD_MAX_SKIRT: f32 = 160.0;

/// Metres between height samples along a road when it is graded.
pub const ROAD_STEP: f32 = 22.0;

/// The steepest a road is allowed to climb, as a rise over its run.
///
/// About one in eight. Steep enough to cross real country without levelling
/// half of it, gentle enough that a cart could take it.
pub const ROAD_GRADE: f32 = 0.13;

/// How many times the grading walks the profile. Each pass moves height between
/// neighbours; a handful converges and more buys nothing.
pub const GRADE_PASSES: usize = 24;

// ------------------------------------------------------------------- water

/// How far the tide carries the waterline up and down, in meters, and how long
/// a full cycle takes.
///
/// **Smaller than it looks like it needs to be, and here is why.**
///
/// How far a tide sweeps is its height divided by the beach's SLOPE — and the
/// beach is raised by a smoothstep, whose derivative is zero at both ends. So the
/// ground is very nearly horizontal exactly at the waterline, which is exactly
/// where the tide is. Dividing by almost nothing is what made a hand's depth of
/// tide walk the sea metres up the sand.
///
/// The beach has a slope at the waterline now (see `BEACH_TOE`), so this no
/// longer divides by almost nothing: at about six degrees, this height of tide
/// walks the waterline roughly a metre. That reads as a tide. It was cut to a
/// third while the shore was still flat at the water, and can afford to come
/// back up now that it is not.
///
/// The coast shelves over hundreds of meters, so the water's
/// horizontal travel is its vertical travel divided by a gradient of about a
/// tenth — every centimeter of tide is ten centimeters of beach. At half a meter
/// the sea drew back a good fifteen meters and stranded the shallows, which
/// reads as a lake emptying rather than as a shore.
pub const TIDE: f32 = 0.12;
pub const TIDE_PERIOD: f32 = 20.0;

// ------------------------------------------------- handing this to the bench

/// Writes `assets/world/world.json`, the recipe Opificium's terrain bench reads.
///
/// **Run this whenever a number above changes:**
///
/// ```text
/// cargo test export_world_for_opificium -- --ignored --nocapture
/// ```
///
/// The bench and the game must agree about the generated ground EXACTLY. A
/// maker sculpts *offsets* — how far the ground moved — and the game adds those
/// to ground it generates itself. If the two disagree about what was underneath
/// by so much as a meter, every hill placed at the bench sits at the wrong
/// height in the game, and nothing on screen says why.
///
/// So the numbers travel as data rather than being written down twice, exactly
/// as a palette does. Ignored by default because it writes a file, and a plain
/// `cargo test` should not.
#[cfg(test)]
mod handing_over {
    use super::*;

    #[test]
    #[ignore = "writes assets/world/world.json"]
    fn export_world_for_opificium() {
        // Hand-written rather than derived, so this file needs no serde
        // dependency and the shape stays visible beside the numbers it carries.
        // Field names are Opificium's `terrain::ground::Recipe`.
        // Unpacked, because a tuple field cannot be named inside a format string.
        let (ranch_x, ranch_z) = RANCH_AT;
        let json = format!(
            "{{\n  \
             \"width\": {WORLD_WIDTH:?},\n  \
             \"seed\": {WORLD_SEED},\n  \
             \"sea_blue_margin\": {MAP_SEA_BLUE_MARGIN},\n  \
             \"sea_threshold\": {MAP_SEA_THRESHOLD:?},\n  \
             \"clean_radius\": {MASK_CLEAN_RADIUS},\n  \
             \"clean_passes\": {MASK_CLEAN_PASSES},\n  \
             \"min_island_pixels\": {MIN_ISLAND_PIXELS},\n  \
             \"coast_fade_start\": {COAST_FADE_START:?},\n  \
             \"coast_height\": {COAST_HEIGHT:?},\n  \
             \"inland_rise\": {INLAND_RISE:?},\n  \
             \"beach_width\": {BEACH_WIDTH:?},\n  \
             \"shelf_width\": {SHELF_WIDTH:?},\n  \
             \"inland_full\": {INLAND_FULL:?},\n  \
             \"ocean_depth\": {OCEAN_DEPTH:?},\n  \
             \"base_elevation\": {BASE_ELEVATION:?},\n  \
             \"range_elevation\": {RANGE_ELEVATION:?},\n  \
             \"ranch_x\": {ranch_x:?},\n  \
             \"ranch_z\": {ranch_z:?},\n  \
             \"ranch_radius\": {RANCH_RADIUS:?},\n  \
             \"massif_height\": {MASSIF_HEIGHT:?},\n  \
             \"massif_radius\": {MASSIF_RADIUS:?},\n  \
             \"range_freq\": {RANGE_FREQ:?},\n  \
             \"range_presence_freq\": {RANGE_PRESENCE_FREQ:?},\n  \
             \"range_presence_cutoff\": {RANGE_PRESENCE_CUTOFF:?},\n  \
             \"range_inland_start\": {RANGE_INLAND_START:?},\n  \
             \"range_inland_full\": {RANGE_INLAND_FULL:?},\n  \
             \"detail_elevation\": {DETAIL_ELEVATION:?},\n  \
             \"detail_freq\": {DETAIL_FREQ:?},\n  \
             \"warp_strength\": {WARP_STRENGTH:?},\n  \
             \"warp_freq\": {WARP_FREQ:?},\n  \
             \"rugged_freq\": {RUGGED_FREQ:?},\n  \
             \"rugged_low\": {RUGGED_LOW:?},\n  \
             \"rugged_high\": {RUGGED_HIGH:?},\n  \
             \"plains_relief\": {PLAINS_RELIEF:?},\n  \
             \"cities\": {CITIES},\n  \
             \"towns\": {TOWNS},\n  \
             \"city_radius\": {CITY_RADIUS:?},\n  \
             \"town_radius\": {TOWN_RADIUS:?},\n  \
             \"city_spacing\": {CITY_SPACING:?},\n  \
             \"town_spacing\": {TOWN_SPACING:?},\n  \
             \"site_skirt\": {SITE_SKIRT:?},\n  \
             \"site_min_inland\": {SITE_MIN_INLAND:?},\n  \
             \"site_max_height\": {SITE_MAX_HEIGHT:?},\n  \
             \"site_max_slope\": {SITE_MAX_SLOPE:?},\n  \
             \"road_width\": {ROAD_WIDTH:?},\n  \
             \"road_skirt\": {ROAD_SKIRT:?},\n  \
             \"road_step\": {ROAD_STEP:?},\n  \
             \"road_grade\": {ROAD_GRADE:?},\n  \
             \"shore_within\": {SHORE_WITHIN:?},\n  \
             \"snowline\": {SNOWLINE:?},\n  \
             \"rock_slope\": {ROCK_SLOPE:?},\n  \
             \"desert_moisture\": {DESERT_MOISTURE:?},\n  \
             \"forest_moisture\": {FOREST_MOISTURE:?},\n  \
             \"settled_levelling\": {SETTLED_LEVELLING:?},\n  \
             \"flat\": {FLAT_WORLD}\n\
             }}\n"
        );

        let road = std::path::Path::new(HEIGHTMAP_PATH)
            .parent()
            .expect("the map lives in a folder")
            .join("world.json");
        std::fs::write(&road, json).expect("the world folder should be writable");
        println!("wrote {}", road.display());
    }
}
