//! The terrain heightfield — the single source of truth for "how high is the
//! ground at (x, z)".
//!
//! Everything that needs to know about the shape of the world asks this: chunk
//! meshing, the player's foot placement, the camera's ground clearance, the
//! spawn-point search. There is deliberately only one implementation, so the
//! ground the player walks on can never disagree with the ground that's drawn.
//!
//! Height is built in layers:
//!   1. **macro elevation** — the map image (or fallback noise): continents,
//!      seas, mountain ranges. Decides land vs. water.
//!   2. **ridged mountains** — sharp ranges, masked to only appear on high land.
//!   3. **fine detail** — small undulations so close-up ground isn't smooth putty.
//!
//! A domain warp is applied before the map lookup so coastlines wander instead
//! of revealing the source image's pixel grid.

use std::sync::{Arc, RwLock};

use bevy::prelude::*;
use noise::{Fbm, MultiFractal, NoiseFn, Perlin};

use crate::config::*;
use crate::world::edit::Sculpt;
pub use terrain_core::biome::{Biome, Climate, Ground as BiomeGround};
use crate::world::heightmap::HeightMap;
use crate::world::settle::{Settlements, Site};

/// Shared handle to the terrain. `Arc` because chunk meshes are built on
/// background threads and each task needs a cheap clone of the generator.
#[derive(Resource, Clone, Deref)]
pub struct TerrainSource(pub Arc<Terrain>);

pub struct Terrain {
    /// Source map. `None` means we're running on procedural fallback.
    map: Option<HeightMap>,
    /// Whether the map's brightness is real relief rather than fill colors.
    map_carries_elevation: bool,
    /// Half-extents of the world in meters (X = east/west, Y here = north/south).
    half: Vec2,
    /// Ridge lines. Where this field crosses zero becomes a mountain crest.
    ranges: Fbm<Perlin>,
    /// Much broader field deciding which regions are mountainous at all.
    presence: Fbm<Perlin>,
    /// Which stretches of coast are sand and which are rock.
    shores: Fbm<Perlin>,
    /// Where the one great mountain stands, if this world has one.
    massif: Option<Vec2>,
    /// Which country is rugged and which is level.
    rugged: Fbm<Perlin>,
    /// Ground leveled for towns, and the roads graded between them.
    settlements: Settlements,
    /// Where the water runs, and how far it cut to get there.
    rivers: terrain_core::river::Rivers,
    detail: Fbm<Perlin>,
    moisture: Fbm<Perlin>,
    warp_x: Perlin,
    warp_z: Perlin,
    /// Only used when there's no map image.
    continent: Fbm<Perlin>,
    /// Hand-sculpted offsets layered on top of everything above.
    ///
    /// Behind a lock because chunk meshes are built on background threads while
    /// the brush is writing on the main thread. Reads are short and uncontended
    /// in the common case; writes only happen on the frames you're sculpting.
    edits: RwLock<Sculpt>,
    /// Woods planted here or at the bench. Behind a lock for the same reason the
    /// ground is: chunks read it on background threads while the Plant brush
    /// writes on the main one.
    forest: RwLock<crate::world::forest::Painted>,
    /// What the ground is made of where somebody said so. Same bargain as the
    /// woods: read on background threads, written by the brush on the main one.
    surface: RwLock<crate::world::surface::Painted>,
}

impl Terrain {
    pub fn new() -> Self {
        let map = HeightMap::load();

        // The map image decides the world's proportions; WORLD_WIDTH decides
        // its scale. A 2:1 map at 8192 m across is 4096 m tall.
        let aspect = map.as_ref().map_or(FALLBACK_ASPECT, HeightMap::aspect);
        let half = Vec2::new(WORLD_WIDTH * 0.5, WORLD_WIDTH / aspect * 0.5);
        let map_carries_elevation = map.as_ref().is_some_and(HeightMap::carries_elevation);

        let mut terrain = Self {
            map,
            map_carries_elevation,
            half,
            // Plain fBm, and only two octaves. Ridged multifractal noise is what
            // produced the spike-forest: it creases sharply at every zero
            // crossing, and squaring the result narrowed those creases into
            // isolated teeth. Rounded low-octave noise gives broad masses that
            // read as ranges instead.
            ranges: Fbm::<Perlin>::new(WORLD_SEED)
                .set_octaves(2)
                .set_frequency(1.0)
                .set_persistence(0.45),
            presence: Fbm::<Perlin>::new(WORLD_SEED.wrapping_add(7))
                .set_octaves(2)
                .set_frequency(1.0)
                .set_persistence(0.5),
            shores: Fbm::<Perlin>::new(WORLD_SEED.wrapping_add(9))
                .set_octaves(2)
                .set_frequency(1.0)
                .set_persistence(0.5),
            rugged: Fbm::<Perlin>::new(WORLD_SEED.wrapping_add(13))
                .set_octaves(2)
                .set_frequency(1.0)
                .set_persistence(0.5),
            settlements: Settlements::nowhere(),
            rivers: terrain_core::river::Rivers::none(half),
            massif: None,
            detail: Fbm::<Perlin>::new(WORLD_SEED.wrapping_add(1))
                .set_octaves(4)
                .set_frequency(1.0),
            moisture: Fbm::<Perlin>::new(WORLD_SEED.wrapping_add(2)).set_octaves(3),
            warp_x: Perlin::new(WORLD_SEED.wrapping_add(3)),
            warp_z: Perlin::new(WORLD_SEED.wrapping_add(4)),
            continent: Fbm::<Perlin>::new(WORLD_SEED.wrapping_add(5)).set_octaves(5),
            edits: RwLock::new(crate::world::edit::load(half)),
            forest: RwLock::new(crate::world::forest::load(half)),
            surface: RwLock::new(crate::world::surface::load(half)),
        };

        // The great mountain goes in the heartland — the point furthest from any
        // sea. Placed before the towns, so their ground is judged against a
        // world that already has it and none of them ends up levelled onto its
        // flank.
        terrain.massif = terrain.map.as_ref().and_then(|map| {
            (MASSIF_HEIGHT > 0.0).then(|| {
                let (u, v) = map.deepest_inland();
                let at = Vec2::new((u - 0.5) * half.x * 2.0, (v - 0.5) * half.y * 2.0);
                info!("the great mountain stands at {:.0}, {:.0}", at.x, at.y);
                at
            })
        });

        // The water, before anything is built. Rivers are read from `raw_height`,
        // which knows nothing of them, so this never consults its own output —
        // the same rule the towns follow below.
        //
        // And BEFORE the towns, so that siting one asks about ground the rivers
        // have already cut. A town planned on ground that has no valley in it yet
        // is a town with a river through the middle of it.
        terrain.rivers = terrain_core::river::Rivers::carve(
            half,
            RIVER_SPACING,
            SEA_LEVEL,
            &|at| terrain.raw_height(at.x, at.y),
        );
        info!(
            "the water cut {} cells of channel",
            terrain.rivers.channel_cells()
        );

        // Planned after the rest of the world exists, because choosing where a
        // town goes means asking how high and how steep the ground is there —
        // and answered with `raw_height`, which knows nothing of settlements, so
        // this never reads back its own output.
        terrain.settlements = Settlements::plan(
            half,
            &|at| terrain.dry_height(at.x, at.y),
            &|at| terrain.shore_meters(at.x, at.y),
        );
        info!(
            "planned {} places and {} roads between them",
            terrain.settlements.sites().len(),
            terrain.settlements.roads_len()
        );
        terrain
    }

    /// How many cells of woods are painted. Zero means either nothing has been
    /// planted or `forest.bin` was refused — the startup log says which.
    pub fn planted_cells(&self) -> usize {
        self.forest.read().map_or(0, |woods| woods.painted_cells())
    }

    /// The hand-sculpted ground, for the mode that shapes it.
    ///
    /// Handed out as the lock rather than its contents: the brush holds a write
    /// lock across a whole stroke, and everything else takes a read lock for the
    /// length of one height query.
    pub fn edits(&self) -> &RwLock<Sculpt> {
        &self.edits
    }

    /// The painted woods, for the Plant brush. Same bargain as [`Self::edits`].
    pub fn woods(&self) -> &RwLock<crate::world::forest::Painted> {
        &self.forest
    }

    /// What the ground is made of, for the Path brush.
    pub fn surface(&self) -> &RwLock<crate::world::surface::Painted> {
        &self.surface
    }

    /// How worn to bare earth the ground is at a point, -1 to 1.
    ///
    /// Zero is the biome's own answer, and that is almost the whole world.
    pub fn worn(&self, x: f32, z: f32) -> f32 {
        self.surface.read().map_or(0.0, |worn| worn.at(x, z))
    }

    /// How many cells of surface a maker has laid.
    pub fn worn_cells(&self) -> usize {
        self.surface.read().map_or(0, |worn| worn.painted_cells())
    }

    /// Every tree standing in a patch of ground.
    ///
    /// Worked out from the ground and the painted layer rather than looked up in
    /// a list, so a chunk plants on its own, on any thread, in any order — and
    /// Opificium planting the same patch gets the same trees. Nothing about a
    /// tree is stored anywhere.
    ///
    /// The scatter itself lives in `terrain-core`, which Opificium's terrain
    /// bench runs too — so the forest here and the forest there are the same
    /// forest by construction. What is left here is asking THIS world's ground
    /// the questions the crate needs answered.
    pub fn trees_in(&self, low: Vec2, high: Vec2) -> Vec<crate::world::forest::Planted> {
        use crate::world::forest;

        let step = TREE_SPACING.max(1.0);
        // A world-wide lattice, not a per-chunk one, so a tree doesn't move when
        // the chunk boundaries around it change.
        let first = (low / step).floor().as_ivec2();
        let last = (high / step).ceil().as_ivec2();

        // Taken once for the whole patch rather than per slot. A chunk asks
        // about thousands of them, and a poisoned lock is no reason to stop
        // drawing trees — the ground's own answer stands, exactly as it would
        // for a world nobody has planted.
        let painted = self.forest.read().ok();

        let mut standing = Vec::new();
        for slot_z in first.y..=last.y {
            for slot_x in first.x..=last.x {
                // Jittered off the lattice, or the wood comes out in rows.
                let jitter = Vec2::new(
                    forest::chance(slot_x, slot_z, 1) - 0.5,
                    forest::chance(slot_x, slot_z, 2) - 0.5,
                ) * step
                    * 0.85;
                let at = Vec2::new(slot_x as f32 * step, slot_z as f32 * step) + jitter;
                if at.x < low.x || at.x >= high.x || at.y < low.y || at.y >= high.y {
                    continue;
                }

                // One gathering of the ground rather than five separate
                // questions of it, and the same one the biome is decided from —
                // so the species planted here belongs to the place the rest of
                // the game says this is.
                let ground = self.ground_at(at.x, at.y);
                if ground.shore < 25.0 {
                    continue;
                }

                let natural = forest::natural_density(
                    ground.moisture,
                    ground.height,
                    ground.slope,
                    ground.shore,
                    ground.levelled,
                    TREELINE,
                );
                let bias = painted.as_ref().map_or(0.0, |woods| woods.at(at.x, at.y));
                let density = forest::density(natural, bias);
                if density <= 0.0 || forest::chance(slot_x, slot_z, 3) > density {
                    continue;
                }

                // WHICH tree, decided by where. Nothing at all in some places:
                // open water grows none and a town's trees are somebody's
                // business rather than the wild's.
                let biome = Biome::of(ground, &self.climate());
                let Some(variety) = terrain_core::tree::pick(
                    biome,
                    forest::chance(slot_x, slot_z, 4),
                    forest::chance(slot_x, slot_z, 7),
                ) else {
                    continue;
                };

                standing.push(forest::Planted {
                    at: Vec3::new(at.x, ground.height, at.y),
                    variety,
                    turn: forest::chance(slot_x, slot_z, 5) * std::f32::consts::TAU,
                    scale: TREE_SCALE_LOW
                        + (TREE_SCALE_HIGH - TREE_SCALE_LOW) * forest::chance(slot_x, slot_z, 6),
                });
            }
        }
        standing
    }

    /// Where the towns are: level ground waiting for a settlement.
    pub fn sites(&self) -> &[Site] {
        self.settlements.sites()
    }

    /// How many cells of hand-sculpted ground loaded. Zero means either that
    /// nothing has been sculpted yet or that `edits.bin` was refused — the
    /// startup log says which.
    pub fn sculpted_cells(&self) -> usize {
        self.edits.read().map_or(0, |edits| edits.sculpted_cells())
    }

    /// Half-extents of the world in meters. X is east/west, Y is north/south.
    pub fn half(&self) -> Vec2 {
        self.half
    }

    /// True if the continent shape came from a map image rather than fallback
    /// noise. Surfaced on the debug HUD so it's obvious which one you're seeing.
    pub fn has_map(&self) -> bool {
        self.map.is_some()
    }

    /// Ground height in meters at a world position, hand-sculpted edits
    /// included. Below `SEA_LEVEL` is sea floor.
    ///
    /// This is the answer for anything that cares where the ground actually is.
    pub fn height(&self, x: f32, z: f32) -> f32 {
        let generated = self.base_height(x, z);
        match self.edits.read() {
            Ok(edits) => generated + edits.at(x, z),
            // A poisoned lock means a sculpting operation panicked. The
            // generated world is still perfectly valid, so keep drawing it
            // rather than taking the game down with it.
            Err(_) => generated,
        }
    }

    /// Ground height from the generator alone, with the edit layer excluded.
    ///
    /// The brush needs this: Smooth and Flatten decide what offset to write
    /// based on what the ground was doing underneath, and they run while
    /// holding the edit lock — so they must not read back through it.
    pub fn base_height(&self, x: f32, z: f32) -> f32 {
        let h = self.dry_height(x, z);
        // Towns stand on level ground and roads are graded between them, so the
        // last word on generated height belongs to whatever has been leveled.
        match self.settlements.level(Vec2::new(x, z)) {
            Some((target, pull)) => h + (target - h) * pull,
            None => h,
        }
    }

    /// The generated ground before any of it is leveled for people.
    ///
    /// Separate because planning where a town goes has to ask how high and how
    /// steep the ground is there, and asking `base_height` would be reading back
    /// the leveling it is in the middle of deciding.
    fn raw_height(&self, x: f32, z: f32) -> f32 {
        // Nudge the lookup position by a low-frequency noise field. Without
        // this, a coastline traced from the image reads as a straight run of
        // pixel edges; with it, the shore wanders the way a real one does.
        let (wx, wz) = self.warp(x, z);

        // Distance to the coast, positive inland and negative out to sea. The
        // whole landscape is built on this one number.
        let shore = self.shore_meters(wx, wz);

        // The coast shelves BOTH ways, each at its own rate: the land climbs a
        // beach's width to reach the shoreline height, and the floor falls a
        // shelf's width to reach the depths. They meet at zero, the waterline.
        //
        // Anything faster than this cannot be drawn. The whole drop used to
        // happen across the width of the mask's blur — a few meters — and
        // neighboring vertices landed on opposite sides of it, so every
        // coastline came out as a fence of vertical slats.
        let mut h = if shore >= 0.0 {
            SEA_LEVEL + COAST_HEIGHT * crate::util::smoothstep(0.0, BEACH_WIDTH, shore)
        } else {
            SEA_LEVEL - OCEAN_DEPTH * crate::util::smoothstep(0.0, SHELF_WIDTH, -shore)
        };

        // Shape-checking mode stops here: a beach, a shelf, and nothing else to
        // look at but the outline of the continents. Hand edits are still added
        // on top, so it's a canvas rather than a lock.
        if FLAT_WORLD {
            return h;
        }

        // 0 at the waterline, 1 once properly ashore. Everything generated is
        // masked by it, so nothing pokes out of the sea beside a beach.
        let coast = crate::util::smoothstep(0.0, BEACH_WIDTH, shore);
        if coast <= 0.0 {
            return h;
        }

        // How far inland, as 0 at the shore to 1 in the deep interior. This is
        // what makes the geography read as geography: plains by the water,
        // uplands behind them, mountains in the middle.
        let inland = (shore / INLAND_FULL).clamp(0.0, 1.0);

        // The land climbs away from the coast.
        h += crate::util::smoothstep(0.0, 0.85, inland) * INLAND_RISE * coast;

        // A true grayscale heightmap carries real relief; a political map has
        // none to give, and its brightness would only be region fill colors.
        if self.map_carries_elevation {
            h += self.macro_elevation(wx, wz) * BASE_ELEVATION * coast;
        }

        // How rugged this country is, 0 plain to 1 mountainous. Mountains and
        // fine detail are both scaled by it, so most of the world is level
        // enough to walk, farm and put a forest on, and the rough ground is
        // somewhere in particular rather than everywhere at once.
        let rugged = self.ruggedness(wx, wz);

        // The one great mountain stands whatever the ruggedness field says: it
        // is the exception the rest of the world is gentle in order to make.
        h += self.massif_height(wx, wz) * coast;
        h += self.range_height(wx, wz, inland) * coast * rugged;

        // Fine detail, masked to the land — the sea floor is under water and
        // mostly hidden, and keeping it calm is both cheaper and smoother.
        let d = self.detail.get([wx as f64 * DETAIL_FREQ, wz as f64 * DETAIL_FREQ]) as f32;
        h + d * DETAIL_ELEVATION * coast * (PLAINS_RELIEF + (1.0 - PLAINS_RELIEF) * rugged)
    }

    /// How rugged the country is here: 0 level plain, 1 full relief.
    fn ruggedness(&self, x: f32, z: f32) -> f32 {
        let n = self
            .rugged
            .get([x as f64 * RUGGED_FREQ, z as f64 * RUGGED_FREQ]) as f32
            * 0.5
            + 0.5;
        crate::util::smoothstep(RUGGED_LOW, RUGGED_HIGH, n)
    }

    /// Height contributed by the one great mountain.
    ///
    /// A broad shoulder easing up to a peak, not a cone: the falloff is raised
    /// to a power so the foot spreads and the summit is the small part, which is
    /// how a massif reads from a distance. The ridge field warps it so the
    /// flanks have spurs and gullies rather than being a smooth dome, and that
    /// warp is scaled by height, so the foot stays walkable while the top breaks
    /// up.
    fn massif_height(&self, x: f32, z: f32) -> f32 {
        let Some(peak) = self.massif else {
            return 0.0;
        };
        if MASSIF_HEIGHT <= 0.0 {
            return 0.0;
        }

        let away = peak.distance(Vec2::new(x, z));
        if away >= MASSIF_RADIUS {
            return 0.0;
        }

        let rise = crate::util::smoothstep(MASSIF_RADIUS, 0.0, away).powf(1.9);
        let ridge = self
            .ranges
            .get([x as f64 * RANGE_FREQ * 3.0, z as f64 * RANGE_FREQ * 3.0]) as f32;
        rise * MASSIF_HEIGHT * (1.0 + ridge * 0.22 * rise)
    }

    /// Height contributed by mountain ranges at a point.
    ///
    /// Three factors decide it, and all three have to agree:
    ///
    /// * **presence** — a very low-frequency field, thresholded hard, so ranges
    ///   occupy a few regions of the map instead of being its texture.
    /// * **inland** — mountains are not allowed near the coast. Beaches and
    ///   plains belong there, and a range rising straight out of the sea reads
    ///   as a mistake.
    /// * **ridge** — `1 - |noise|`. The crease where the noise crosses zero
    ///   becomes a crest, and at this frequency that crest runs for kilometers.
    ///
    /// The crest is raised to a modest power to narrow it, and that is *all*.
    /// It is deliberately not squared, and the noise is deliberately only two
    /// octaves: stacking octaves onto a ridged field and squaring the result is
    /// exactly what turned the first attempt at mountains into a map-wide
    /// forest of spikes.
    fn range_height(&self, x: f32, z: f32, inland: f32) -> f32 {
        let allowed = crate::util::smoothstep(RANGE_INLAND_START, RANGE_INLAND_FULL, inland);
        if allowed <= 0.0 {
            return 0.0;
        }

        let presence = self
            .presence
            .get([x as f64 * RANGE_PRESENCE_FREQ, z as f64 * RANGE_PRESENCE_FREQ])
            as f32
            * 0.5
            + 0.5;
        let presence = crate::util::smoothstep(RANGE_PRESENCE_CUTOFF, 1.0, presence);
        if presence <= 0.0 {
            return 0.0;
        }

        let n = self.ranges.get([x as f64 * RANGE_FREQ, z as f64 * RANGE_FREQ]) as f32;
        let crest = (1.0 - n.abs()).clamp(0.0, 1.0).powf(1.7);

        crest * presence * allowed * RANGE_ELEVATION
    }

    /// Distance to the coast in meters: **positive inland, negative out to sea**.
    ///
    /// The one number the whole landscape is built on. It crosses zero exactly
    /// at the shoreline and changes smoothly through it, which is what lets the
    /// land rise and the sea floor fall at their own separate rates instead of
    /// meeting at a cliff.
    ///
    /// Public because it's also the most useful number for tuning geography:
    /// `INLAND_FULL` and the mountain thresholds are all fractions of it, and if
    /// no land on the map ever gets far enough from the sea, ranges silently
    /// never appear.
    pub fn shore_meters(&self, x: f32, z: f32) -> f32 {
        let Some(map) = &self.map else {
            // No map: open sea everywhere, and the fallback noise supplies the
            // land instead — see `fallback_elevation`.
            return self.fallback_shore(x, z);
        };
        let (u, v) = self.to_map_uv(x, z);
        let meters_per_pixel = self.half.x * 2.0 / map.width() as f32;
        let shore = (map.inland_pixels(u, v) - map.offshore_pixels(u, v)) * meters_per_pixel;

        // The world ends in water whatever the image shows at its own margins —
        // a screenshot's UI chrome lives exactly there. Carried out to sea
        // rather than merely lowered, so the border is ocean and not a shelf.
        let fade = self.border_fade(x, z);
        shore * fade - (1.0 - fade) * SHELF_WIDTH
    }

    /// A stand-in shore distance for the no-map fallback, from its noise field.
    fn fallback_shore(&self, x: f32, z: f32) -> f32 {
        let e = self.fallback_elevation(x, z);
        // Rescaled around the waterline so it crosses zero at the same place the
        // threshold does, and reaches full depth and full inland either side.
        let t = (e - MAP_SEA_THRESHOLD) / MAP_SEA_THRESHOLD.max(1.0e-4);
        (t * INLAND_FULL).clamp(-SHELF_WIDTH, INLAND_FULL) * self.border_fade(x, z)
    }

    /// What KIND of coast this stretch is: 0 rock, 1 sand.
    ///
    /// Sand is not the default state of a shoreline. A coast is beach where the
    /// sea has somewhere to put sediment and rock where it has not, and which it
    /// is changes *along* the coast rather than being true of the whole map. A
    /// world with every continent outlined in sand reads as a drawing of a map
    /// rather than as ground.
    ///
    /// Low frequency, so a beach runs the better part of a kilometer and then
    /// gives way, instead of speckling.
    pub fn shore_character(&self, x: f32, z: f32) -> f32 {
        let n = self
            .shores
            .get([x as f64 * SHORE_FREQ, z as f64 * SHORE_FREQ]) as f32
            * 0.5
            + 0.5;
        crate::util::smoothstep(0.40, 0.62, n)
    }

    /// Moisture at a world position, 0 (arid) to 1 (lush). Drives biome color;
    /// later it can drive which monsters live where.
    pub fn moisture(&self, x: f32, z: f32) -> f32 {
        let m = self
            .moisture
            .get([x as f64 * MOISTURE_FREQ, z as f64 * MOISTURE_FREQ]) as f32;
        (m * 0.5 + 0.5).clamp(0.0, 1.0)
    }

    /// The thresholds that decide what sort of world this is.
    ///
    /// Built from `config.rs` rather than taken from the crate's defaults, so the
    /// numbers a maker tunes live with every other number that shapes the ground —
    /// and travel to the bench in `world.json` with them.
    pub fn climate(&self) -> Climate {
        Climate {
            shore_within: SHORE_WITHIN,
            treeline: TREELINE,
            snowline: SNOWLINE,
            rock_above: ROCK_SLOPE,
            desert_below: DESERT_MOISTURE,
            forest_above: FOREST_MOISTURE,
            settled_above: SETTLED_LEVELLING,
        }
    }

    /// Everything about a point that decides what kind of place it is.
    ///
    /// Gathered once because it is five separate questions of the terrain and
    /// three different callers want the answer to all of them — the biome, the
    /// species of tree, and later what lives here.
    pub fn ground_at(&self, x: f32, z: f32) -> BiomeGround {
        BiomeGround {
            height: self.height(x, z),
            slope: 1.0 - self.normal(x, z, 2.0).y,
            moisture: self.moisture(x, z),
            shore: self.shore_meters(x, z),
            levelled: self
                .settlements
                .level(Vec2::new(x, z))
                .map(|(_, weight)| weight)
                .unwrap_or(0.0),
        }
    }

    /// What kind of place this is.
    pub fn biome(&self, x: f32, z: f32) -> Biome {
        Biome::of(self.ground_at(x, z), &self.climate())
    }

    /// How strongly this ground reads as its own kind, 0 at a boundary to 1 well
    /// inside one. For anything that should fade rather than switch.
    pub fn biome_confidence(&self, x: f32, z: f32) -> f32 {
        Biome::confidence(self.ground_at(x, z), &self.climate())
    }

    /// Surface normal from central differences on the heightfield.
    ///
    /// Computed analytically rather than from mesh triangles on purpose: it
    /// depends only on world coordinates, so two neighboring chunks derive
    /// *identical* normals along their shared edge and stitch together with no
    /// visible lighting seam.
    pub fn normal(&self, x: f32, z: f32, epsilon: f32) -> Vec3 {
        let dx = self.height(x + epsilon, z) - self.height(x - epsilon, z);
        let dz = self.height(x, z + epsilon) - self.height(x, z - epsilon);
        Vec3::new(-dx, 2.0 * epsilon, -dz).normalize()
    }

    /// The generated ground with the rivers cut into it, and nothing else.
    ///
    /// Between `raw_height` and `base_height`: the land as the water left it,
    /// before anybody levelled a town on it. This is what the towns are sited
    /// against, so they are placed on a map that already has its valleys.
    pub fn dry_height(&self, x: f32, z: f32) -> f32 {
        self.raw_height(x, z) - self.rivers.at(x, z).0
    }

    /// The still water standing in a channel here, if any.
    ///
    /// `None` on dry land. Rivers do not flow — there is no current to model and
    /// nothing that would read one — so a river is a surface at a height, the
    /// same as the sea is.
    pub fn river_surface(&self, x: f32, z: f32) -> Option<f32> {
        let (cut, water) = self.rivers.at(x, z);
        // Cut but not filled is a dry bank; only where the water stands above the
        // ground it cut is there a river to see.
        (cut > 0.05 && water > self.base_height(x, z)).then_some(water)
    }

    /// Applies the coastline domain warp.
    fn warp(&self, x: f32, z: f32) -> (f32, f32) {
        let (u, v) = (x as f64 * WARP_FREQ, z as f64 * WARP_FREQ);
        (
            x + self.warp_x.get([u, v]) as f32 * WARP_STRENGTH,
            z + self.warp_z.get([u, v]) as f32 * WARP_STRENGTH,
        )
    }

    /// World position to image space. Image coordinates run 0..1 from the west
    /// edge and from the north edge (−Z).
    fn to_map_uv(&self, x: f32, z: f32) -> (f32, f32) {
        (
            (x + self.half.x) / (self.half.x * 2.0),
            (z + self.half.y) / (self.half.y * 2.0),
        )
    }

    /// Pulls land under water at the very edge of the world.
    ///
    /// "The world ends in water, not a wall" is an invariant, and it has to
    /// hold whatever the source image happens to show at its own margins — a
    /// screenshot's toolbar and scale bar live exactly there. Kept tight to the
    /// border so it trims furniture rather than real coastline.
    fn border_fade(&self, x: f32, z: f32) -> f32 {
        let d = (x.abs() / self.half.x).max(z.abs() / self.half.y);
        crate::util::smoothstep(1.0, COAST_FADE_START, d)
    }

    /// Broad elevation in 0..1, where 0 is the deepest ocean and 1 the highest
    /// ground. Sourced from the map image when we have one.
    fn macro_elevation(&self, x: f32, z: f32) -> f32 {
        match &self.map {
            Some(map) => {
                let (u, v) = self.to_map_uv(x, z);
                map.elevation(u, v)
            }
            None => self.fallback_elevation(x, z),
        }
    }

    /// Procedural stand-in used only when no map image is present: one blobby
    /// continent that fades into ocean before it reaches the world border, so
    /// the fallback world is finite in the same way the real one is.
    fn fallback_elevation(&self, x: f32, z: f32) -> f32 {
        let c = self
            .continent
            .get([x as f64 * CONTINENT_FREQ, z as f64 * CONTINENT_FREQ]) as f32;
        let base = (c * 0.5 + 0.5).clamp(0.0, 1.0);

        // Distance to the nearest world edge, 0 at the center and 1 at the
        // border, taken on whichever axis is closer to running out.
        let d = (x.abs() / self.half.x).max(z.abs() / self.half.y);
        let fade = crate::util::smoothstep(1.0, COAST_FADE_START, d);

        base * fade
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Renders the generated world as ASCII and checks it's actually a world:
    /// both land and sea, mountains that top out somewhere believable, and a
    /// border that's underwater on every side.
    ///
    /// Run with `cargo test -- --nocapture` to see the map. This is the fastest
    /// way to tell whether a new map image or a tuning change did what you
    /// expected, without waiting on a window.
    #[test]
    fn the_water_finds_its_way_to_the_sea() {
        // Rivers are FOUND, so the only way to know the finding worked on this
        // world - rather than on a test valley - is to look at this world.
        let terrain = Terrain::new();
        assert!(
            terrain.rivers.channel_cells() > 0,
            "a continent this size should carry rivers"
        );

        // And a channel has to be a channel: somewhere the ground was taken down
        // and water stands in it, above the sea and below the land around it.
        let half = terrain.half();
        let mut wet = 0;
        let mut deepest = 0.0_f32;
        for step_z in -60..60 {
            for step_x in -120..120 {
                let at = Vec2::new(
                    step_x as f32 / 120.0 * half.x,
                    step_z as f32 / 60.0 * half.y,
                );
                let (cut, _) = terrain.rivers.at(at.x, at.y);
                deepest = deepest.max(cut);
                if terrain.river_surface(at.x, at.y).is_some() {
                    wet += 1;
                }
            }
        }
        assert!(deepest > 0.5, "no channel was cut anywhere: {deepest:.2} m");
        assert!(wet > 0, "every channel came out dry");
    }

    #[test]
    fn generates_a_plausible_world() {
        const COLUMNS: usize = 110;

        let terrain = Terrain::new();
        let half = terrain.half();
        // Character cells are about twice as tall as they are wide, so halve
        // the row count to keep the printed map in proportion.
        let rows = (COLUMNS as f32 * half.y / half.x * 0.5).round() as usize;

        let mut land = 0usize;
        let mut total = 0usize;
        let mut peak = f32::MIN;
        let mut trough = f32::MAX;
        let mut deepest_inland = 0.0f32;
        let mut picture = String::new();

        // Each character covers tens of meters of ground, so a single sample
        // per cell shows aliasing — lone holes and specks that aren't in the
        // terrain at all. Averaging a small grid per character makes the
        // printed map an honest summary of what's actually there.
        const SUPERSAMPLE: usize = 3;
        let cell = Vec2::new(
            half.x * 2.0 / COLUMNS as f32,
            half.y * 2.0 / rows as f32,
        );

        for row in 0..rows {
            for column in 0..COLUMNS {
                let x = (column as f32 / (COLUMNS - 1) as f32 * 2.0 - 1.0) * half.x;
                let z = (row as f32 / (rows - 1) as f32 * 2.0 - 1.0) * half.y;

                deepest_inland = deepest_inland.max(terrain.shore_meters(x, z));

                let mut h = 0.0;
                for sz in 0..SUPERSAMPLE {
                    for sx in 0..SUPERSAMPLE {
                        let offset = (Vec2::new(sx as f32, sz as f32)
                            / (SUPERSAMPLE - 1) as f32
                            - 0.5)
                            * cell;
                        h += terrain.height(x + offset.x, z + offset.y);
                    }
                }
                let h = h / (SUPERSAMPLE * SUPERSAMPLE) as f32;

                peak = peak.max(h);
                trough = trough.min(h);
                total += 1;
                if h > SEA_LEVEL {
                    land += 1;
                }

                // In flat mode every land sample falls in the same band, which
                // is what makes the continent outline legible at a glance.
                picture.push(match h {
                    h if h < -25.0 => ' ',
                    h if h <= SEA_LEVEL => '.',
                    h if h < 30.0 => '-',
                    h if h < 90.0 => 'n',
                    h if h < 170.0 => 'M',
                    _ => 'A',
                });
            }
            picture.push('\n');
        }

        let land_fraction = land as f32 / total as f32;
        println!(
            "\nsource: {}\nworld: {:.0} x {:.0} m\nland: {:.0}%   low {:.0} m   peak {:.0} m\n\
             furthest from any coast: {deepest_inland:.0} m (INLAND_FULL is {INLAND_FULL:.0} m)\n\
             places: {} cities, {} towns, {} roads\n\n{picture}",
            if terrain.has_map() { "map image" } else { "procedural fallback" },
            half.x * 2.0,
            half.y * 2.0,
            land_fraction * 100.0,
            trough,
            peak,
            terrain.sites().iter().filter(|s| s.city).count(),
            terrain.sites().iter().filter(|s| !s.city).count(),
            terrain.settlements.roads_len(),
        );

        // The ranch is pinned by hand and does not come out of either quota, so
        // a full map is every city, every town, and the ranch besides.
        assert!(
            terrain.sites().len() == CITIES + TOWNS + 1,
            "every city and town should have found ground beside the ranch: \
             wanted {}, placed {}",
            CITIES + TOWNS + 1,
            terrain.sites().len()
        );
        assert!(
            terrain
                .sites()
                .iter()
                .any(|site| site.at.distance(Vec2::new(RANCH_AT.0, RANCH_AT.1)) < 1.0),
            "the ranch should be among the levelled places"
        );

        assert!(
            (0.10..0.80).contains(&land_fraction),
            "expected a mix of land and sea, got {:.0}% land",
            land_fraction * 100.0
        );

        if FLAT_WORLD {
            // Shape-checking mode: the whole point is that land is featureless,
            // so the check is that it's genuinely flat rather than merely calm.
            assert!(
                (peak - COAST_HEIGHT).abs() < 0.5,
                "flat mode should top out at exactly the plateau height, got {peak:.1} m"
            );
        } else {
            assert!(
                peak > 60.0,
                "world has no high ground at all (peak {peak:.0} m)"
            );
        }

        // Every corner must be open water, or the map doesn't end in ocean.
        for (x, z) in [
            (-half.x, -half.y),
            (half.x, -half.y),
            (-half.x, half.y),
            (half.x, half.y),
        ] {
            let h = terrain.height(x, z);
            assert!(h < SEA_LEVEL, "corner ({x:.0}, {z:.0}) is land at {h:.0} m");
        }
    }
}

#[cfg(test)]
mod ranch_tests {
    use super::*;

    #[test]
    fn the_ranch_stands_on_land_at_the_height_the_bench_reported() {
        // The spot was chosen by eye at Opificium's terrain bench, which read
        // 22.9 m. If the game disagrees, the two programs are not building the
        // same ground — which is the one failure the whole world.json contract
        // exists to prevent, and it would show up as a farm sunk into a hill.
        let terrain = Terrain::new();
        let (x, z) = RANCH_AT;
        let height = terrain.height(x, z);

        assert!(
            height > SEA_LEVEL + 1.0,
            "the ranch is under water at {height:.1} m"
        );
        assert!(
            (height - 22.9).abs() < 1.5,
            "the bench read 22.9 m here and the game reads {height:.1} m - \
             the two are not building the same ground"
        );
    }
}

#[cfg(test)]
mod biomes {
    use super::*;

    /// What the world is actually made of, and whether every kind of place is
    /// somewhere a monster could be found.
    ///
    /// `cargo test the_world_holds_every_biome -- --nocapture` prints the shares.
    #[test]
    fn the_world_holds_every_biome() {
        let terrain = Terrain::new();
        let half = terrain.half();
        let sea = terrain.climate();

        let mut tally = std::collections::BTreeMap::new();
        const STEPS: i32 = 160;
        for iz in 0..STEPS {
            for ix in 0..STEPS {
                let at = Vec2::new(
                    (ix as f32 / (STEPS - 1) as f32 * 2.0 - 1.0) * half.x,
                    (iz as f32 / (STEPS - 1) as f32 * 2.0 - 1.0) * half.y,
                );
                let kind = Biome::of(terrain.ground_at(at.x, at.y), &sea);
                *tally.entry(kind.name()).or_insert(0usize) += 1;
            }
        }

        let total = (STEPS * STEPS) as f32;
        for (name, count) in &tally {
            println!("{name:>10}: {:>5.1}%", *count as f32 / total * 100.0);
        }

        // Land kinds a monster is meant to live in must actually exist on the
        // map. A habitat with no ground in it is a species with nowhere to be.
        for wanted in [
            Biome::Water.name(),
            Biome::Shore.name(),
            Biome::Grass.name(),
            Biome::Forest.name(),
        ] {
            assert!(tally.contains_key(wanted), "no {wanted} anywhere in the world");
        }
    }
}
