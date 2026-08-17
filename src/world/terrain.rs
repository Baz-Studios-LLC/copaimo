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

/// How the shore rises from the waterline, 0 at the water to 1 at full height.
///
/// **It has to have slope where it meets the water.** This was a plain
/// smoothstep, and a smoothstep's derivative is zero at BOTH ends — so the ground
/// was very nearly horizontal exactly at the waterline. How far a tide sweeps is
/// its height divided by that slope, and dividing by almost nothing walked the
/// sea metres up the sand for a hand's depth of tide.
///
/// So a quadratic ease-out is mixed in. It has real slope at the water and none
/// at the top, which is the shape a beach actually is — steepest where the waves
/// work it, flattening as it meets the land. Mixing rather than replacing keeps
/// the top of the ramp flat, so there is still no crease where the beach gives
/// way to the inland rise.
///
/// The other half of the reason a smoothstep was there in the first place is
/// unchanged and still matters: nothing may change height faster than the vertex
/// grid can draw, or a coastline combs into vertical slats. This is gentler than
/// that limit everywhere.
fn beach_ramp(along: f32) -> f32 {
    let t = along.clamp(0.0, 1.0);
    let eased = crate::util::smoothstep(0.0, 1.0, t);
    // 1 - (1-t)^2: slope of two at the water, nothing at the top.
    let toe = 1.0 - (1.0 - t) * (1.0 - t);
    eased * (1.0 - BEACH_TOE) + toe * BEACH_TOE
}
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
            // Where a river would be drawn, asked of the rivers alone. There are
            // no settlements yet to level anything, so this is the same question
            // `river_surface` asks with nothing standing in the way of it.
            &|at| {
                terrain.rivers.bed_at(at.x, at.y) >= RIVER_EDGE
                    && terrain.rivers.cut_at(at.x, at.y) >= CHANNEL_LEAST
            },
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
                    // On the drawn surface, not the true one. See `drawn_height`.
                    at: Vec3::new(at.x, self.drawn_height(at.x, at.y), at.y),
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
            SEA_LEVEL + COAST_HEIGHT * beach_ramp(shore / BEACH_WIDTH)
        } else {
            SEA_LEVEL - OCEAN_DEPTH * beach_ramp(-shore / SHELF_WIDTH)
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

        // Foothills reach well past the mountain itself, so it does not stand up
        // out of a plain like a boil. Everything below is shaped inside this.
        let reach = MASSIF_RADIUS * MASSIF_SKIRT;
        let away = peak.distance(Vec2::new(x, z));
        if away >= reach {
            return 0.0;
        }

        // The mass itself: a dome, and on its own that is exactly the fault —
        // a smooth radial bump with no slope anywhere steep enough to shed its
        // snow, which is why it read as one white pimple.
        let body = crate::util::smoothstep(MASSIF_RADIUS, 0.0, away).powf(1.9);

        // What makes it a mountain instead. `1 - |noise|` creases sharply where
        // the field crosses zero, and those creases run for hundreds of metres —
        // so the flanks come out as spurs with gullies between them rather than
        // as a shell. Multiplied, not added, so the gullies cut INTO the mass and
        // the ridges keep its height.
        let crease = 1.0
            - self
                .ranges
                .get([x as f64 * RANGE_FREQ * 6.0, z as f64 * RANGE_FREQ * 6.0])
                .abs() as f32;
        let spurs = 1.0 - MASSIF_RELIEF * (1.0 - crease.powf(1.5));

        // And the shoulders it sits on, reaching out past the mountain into
        // broken high ground.
        let hills = crate::util::smoothstep(reach, MASSIF_RADIUS * 0.4, away);
        let rough = self
            .rugged
            .get([x as f64 * RANGE_FREQ * 3.0, z as f64 * RANGE_FREQ * 3.0]) as f32;
        let skirt = hills * hills * MASSIF_FOOTHILLS * (0.7 + 0.5 * rough);

        MASSIF_HEIGHT * (body * spurs + skirt)
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
        let height = self.height(x, z);
        // The same depth the surface is drawn from, not a second opinion.
        //
        // This used to read the river's old held water level against the ground —
        // a different field, a different threshold, and a different answer, so a
        // place could be biome-water with no water drawn on it or drawn water on
        // ground the biome called grass. `river_depth` asks for no ground of its
        // own, which is what makes it affordable once per blade of grass.
        let water_above = self.river_depth(x, z, height);

        BiomeGround {
            height,
            slope: 1.0 - self.normal(x, z, 2.0).y,
            moisture: self.moisture(x, z),
            shore: self.shore_meters(x, z),
            levelled: self
                .settlements
                .level(Vec2::new(x, z))
                .map(|(_, weight)| weight)
                .unwrap_or(0.0),
            water_above,
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

    /// The height of the ground as it is actually DRAWN, not as it truly is.
    ///
    /// (See `beach_ramp` below for the shape of the shore itself.)
    ///
    /// A chunk mesh is a grid of quads two metres apart, so between its vertices
    /// the surface is a flat triangle — and on any ground that bulges, that
    /// triangle sits BELOW the real height. Anything placed at the real height
    /// therefore stands off the ground it is meant to be standing on, which is
    /// why the trees were floating.
    ///
    /// So anything that sits on the surface asks for the surface, and gets the
    /// same bilinear answer the renderer draws.
    pub fn drawn_height(&self, x: f32, z: f32) -> f32 {
        let step = CHUNK_SIZE / CHUNK_QUADS as f32;
        let (gx, gz) = (x / step, z / step);
        let (x0, z0) = (gx.floor(), gz.floor());
        let (tx, tz) = (gx - x0, gz - z0);

        let corner = |cx: f32, cz: f32| self.height(cx * step, cz * step);
        let near = corner(x0, z0) * (1.0 - tx) + corner(x0 + 1.0, z0) * tx;
        let far = corner(x0, z0 + 1.0) * (1.0 - tx) + corner(x0 + 1.0, z0 + 1.0) * tx;
        near * (1.0 - tz) + far * tz
    }

    /// The generated ground with the rivers cut into it, and nothing else.
    ///
    /// Between `raw_height` and `base_height`: the land as the water left it,
    /// before anybody levelled a town on it. This is what the towns are sited
    /// against, so they are placed on a map that already has its valleys.
    pub fn dry_height(&self, x: f32, z: f32) -> f32 {
        self.raw_height(x, z) - self.rivers.cut_at(x, z)
    }

    /// The still water standing in a channel here, if any.
    ///
    /// `None` on dry land. Rivers do not flow — there is no current to model and
    /// nothing that would read one — so a river is a surface at a height, the
    /// same as the sea is.
    /// How much of a river's channel is still cut into the ground here.
    ///
    /// The cut is made first and the towns are levelled on top of it, so a town
    /// standing where a river ran FILLS THE CHANNEL IN. The ground goes back up;
    /// the record of the cut does not. Anything asking `rivers.at` alone is
    /// asking what the water once did rather than what the ground is now.
    ///
    /// Levelling raises the ground by exactly `cut * pull` — the whole cut where
    /// a site is fully flat, none of it out past the skirt — so what is left of
    /// the channel is exactly what is left of the cut.
    fn open_channel(&self, x: f32, z: f32) -> f32 {
        let cut = self.rivers.cut_at(x, z);
        match self.settlements.level(Vec2::new(x, z)) {
            Some((_, pull)) => cut * (1.0 - pull),
            None => cut,
        }
    }

    /// How deep the water standing here is, and nought where there is none.
    ///
    /// **The one answer.** The surface that gets drawn and the biome that calls a
    /// place water have to be the same claim, or the world grows grass in its
    /// rivers and puts fish on its fields. They were two claims, made from
    /// different fields with different thresholds, and they disagreed everywhere
    /// the two fields did.
    ///
    /// The water fills the channel that is still cut into the ground here — see
    /// [`Self::open_channel`] — three quarters of the way up, and fades to
    /// nothing at both of that channel's own edges: at the shoulder where the bed
    /// gives way to the bank, and at the shallowest cut worth drawing. A surface
    /// that simply stopped would leave a step, and a step of water with nothing
    /// under it IS the slab on the grass.
    pub fn river_depth(&self, x: f32, z: f32, ground: f32) -> f32 {
        // Over a bed, not over a bank. A channel's cut reaches several times its
        // own width because banks do; filling to the edge of the cut would put a
        // river across its own floodplain.
        let bed = self.rivers.bed_at(x, z);
        if bed < RIVER_EDGE {
            return 0.0;
        }
        let channel = self.open_channel(x, z);
        if channel < CHANNEL_LEAST {
            return 0.0;
        }
        // Every edge of the water feathers away to nothing, and all of them the
        // same way.
        //
        // There are three, and each one had to be found the hard way, so they are
        // worth naming: the BANK, where the bed gives way to the rise beside it;
        // the SHALLOWS, where the channel runs out of depth; and the MOUTH, where
        // the river meets a sea already drawn at its own level.
        //
        // The feather is quadratic because a straight run-out leaves a LIP. The
        // surface is drawn on a mesh with vertices every couple of metres and its
        // edge is wherever the last wet one falls, so a taper still a third of
        // the way up when it runs out of vertices leaves that last one standing
        // above the ground — a rim of water round the whole river, which is a
        // slab with a river-shaped hole in it. Squared, the last stride is
        // already nearly nothing.
        let feather = |t: f32| {
            let t = t.clamp(0.0, 1.0);
            t * t
        };

        let bank = feather((bed - RIVER_EDGE) / (1.0 - RIVER_EDGE));
        // In HEIGHT above the sea, not in distance from it. A river reaching a
        // steep shore drops a metre in two, so a fade spread over any fixed
        // distance is crossed in a single stride.
        let mouth = feather(crate::util::smoothstep(
            SEA_LEVEL + RIVER_MOUTH_LOW,
            SEA_LEVEL + RIVER_MOUTH_HIGH,
            ground,
        ));
        // The shallows are left straight. This one is governed by the cut, which
        // changes over tens of metres rather than a few, so it has vertices to
        // spare — and feathering every edge would leave a river that is shallow
        // everywhere and deep nowhere.
        let shallows = (channel - CHANNEL_LEAST) * RIVER_FILL;

        shallows * bank * mouth
    }

    /// Where a river's surface sits, if there is one here.
    ///
    /// # Water fills a channel; it does not sit at a height
    ///
    /// Four attempts at this drew a surface at some height and then argued about
    /// where it was allowed to appear — a held level, a level capped, a level
    /// masked to a bed, a fixed depth above the bed. Every one of them put sheets
    /// of water on open grass, because a height and the ground are two different
    /// shapes and every disagreement between them is a slab hanging in the air.
    ///
    /// So the water is not at a height at all. It fills the channel that is
    /// actually still cut into the ground at this point, three quarters of the
    /// way up. A quarter of the channel is bank, always, so the surface is below
    /// the surrounding land by construction — not by a threshold that could be
    /// tuned wrong, and not on flat country in particular.
    ///
    /// The last fixed-depth version stood 0.7 m above the bed. Where a channel
    /// was shallower than that the difference stood proud of the field, and where
    /// a town had levelled its ground the channel was gone entirely and the water
    /// was drawn on the town square. That was 787 of the 804 slabs.
    pub fn river_surface(&self, x: f32, z: f32) -> Option<f32> {
        let ground = self.drawn_height(x, z);
        if ground < SEA_LEVEL + 0.2 {
            // The sea already draws this, and two surfaces in one place flicker.
            return None;
        }
        // A finger's depth or it is not water. Below that the sheet and the
        // ground are the same surface to a float, and two surfaces in one place
        // tear at each other as the camera moves.
        let deep = self.river_depth(x, z, ground);
        if deep < 0.02 {
            return None;
        }
        Some(ground + deep)
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
    fn the_mountain_is_a_mountain_and_not_a_dome() {
        // It came out as one smooth white pimple: a radial bump with no slope
        // anywhere steep enough to shed its snow, standing straight up off a
        // plain. Both halves of that are measurable.
        let terrain = Terrain::new();
        let Some(peak) = terrain.massif else {
            return;
        };

        // Walked round the mountain at a fixed radius, the height must VARY —
        // a dome gives the same answer all the way round.
        let ring = MASSIF_RADIUS * 0.45;
        let heights: Vec<f32> = (0..72)
            .map(|step| {
                let turn = step as f32 / 72.0 * std::f32::consts::TAU;
                let at = peak + Vec2::new(turn.cos(), turn.sin()) * ring;
                terrain.height(at.x, at.y)
            })
            .collect();
        let high = heights.iter().copied().fold(f32::MIN, f32::max);
        let low = heights.iter().copied().fold(f32::MAX, f32::min);
        assert!(
            high - low > MASSIF_HEIGHT * 0.15,
            "the flanks vary by {:.0} m, which is a dome",
            high - low
        );

        // And there is rock on it, not just snow. A dome is too gentle
        // everywhere to expose any.
        let climate = terrain.climate();
        let mut rock = 0;
        for step in 0..400 {
            let turn = step as f32 * 2.4;
            let out = (step as f32 / 400.0) * MASSIF_RADIUS;
            let at = peak + Vec2::new(turn.cos(), turn.sin()) * out;
            if Biome::of(terrain.ground_at(at.x, at.y), &climate) == Biome::Rock {
                rock += 1;
            }
        }
        assert!(rock > 10, "only {rock} of 400 samples on the mountain are rock");
    }

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
                let cut = terrain.rivers.cut_at(at.x, at.y);
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
    fn the_shore_has_a_slope_where_the_water_meets_it() {
        // The whole reason for the toe. A smoothstep is flat at BOTH ends, so the
        // ground had no slope at all exactly at the waterline — and how far a tide
        // sweeps is its height over that slope. Dividing by almost nothing walked
        // the sea metres up the sand.
        let slope_at = |along: f32| {
            let step = 1.0e-3;
            (beach_ramp(along + step) - beach_ramp(along)) / step
        };

        let at_the_water = slope_at(0.0);
        assert!(
            at_the_water > 0.4,
            "the shore is flat where the water meets it: {at_the_water:.3}"
        );

        // How far the tide actually walks, in metres of sand.
        let rise = COAST_HEIGHT / BEACH_WIDTH * at_the_water;
        let sweep = TIDE / rise;
        assert!(
            sweep < 3.0,
            "the tide walks {sweep:.1} m up the beach, which is too far"
        );
        assert!(sweep > 0.2, "and it should still be visible: {sweep:.2} m");

        // Flat at the TOP, though, or there is a crease where the beach gives way
        // to the land behind it.
        assert!(
            slope_at(0.999) < 0.15,
            "a crease at the top of the beach: {:.3}",
            slope_at(0.999)
        );

        // And still monotonic, or a coastline steps.
        let mut last = -1.0;
        for step in 0..=100 {
            let here = beach_ramp(step as f32 / 100.0);
            assert!(here >= last, "the ramp goes backwards at {step}");
            last = here;
        }
    }


    #[test]
    fn no_town_has_a_river_running_through_it() {
        let terrain = Terrain::new();

        // Everything the world was asked for is still there. The siting now
        // turns down any ground a river crosses, and a rejection that cannot be
        // satisfied is a map that quietly comes up short of towns rather than
        // saying so.
        let sites = terrain.sites();
        assert_eq!(
            sites.len(),
            1 + CITIES + TOWNS,
            "the ranch, {CITIES} cities and {TOWNS} towns"
        );

        for site in sites {
            let mut wet = 0;
            let mut dz = -site.radius;
            while dz <= site.radius {
                let mut dx = -site.radius;
                while dx <= site.radius {
                    let at = site.at + Vec2::new(dx, dz);
                    if dx * dx + dz * dz <= site.radius * site.radius
                        && terrain.river_surface(at.x, at.y).is_some()
                    {
                        wet += 1;
                    }
                    dx += 8.0;
                }
                dz += 8.0;
            }
            assert_eq!(
                wet, 0,
                "the place at {:.0}, {:.0} has {wet} samples of river in it",
                site.at.x, site.at.y
            );
        }
    }

    #[test]
    fn water_does_not_lie_on_dry_ground() {
        // Sheets of river were being drawn across whole beaches. A channel's
        // banks reach several times its own width, and asking only that
        // SOMETHING had been cut counted a five-centimetre graze a hundred and
        // fifty metres out as riverbed.
        let terrain = Terrain::new();
        let half = terrain.half();

        let mut standing = 0;
        let mut rims = 0;
        let mut worst = 0.0_f32;
        let mut worst_at = Vec2::ZERO;
        let mut over = 0;
        let mut over_town = 0;
        let mut lips: Vec<f32> = Vec::new();
        for step_z in -90..90 {
            for step_x in -180..180 {
                let at = Vec2::new(
                    step_x as f32 / 180.0 * half.x,
                    step_z as f32 / 90.0 * half.y,
                );
                let Some(water) = terrain.river_surface(at.x, at.y) else {
                    continue;
                };
                standing += 1;

                // Every drawn surface must have ground under it and sea below it,
                // or it is a sheet lying on a beach.
                let ground = terrain.drawn_height(at.x, at.y);
                assert!(
                    water > ground,
                    "water at {:.0}, {:.0} sits {:.2} m UNDER its own bed",
                    at.x,
                    at.y,
                    ground - water
                );
                assert!(
                    water > SEA_LEVEL,
                    "river water at or below the sea, which the sea already draws"
                );
                // And — the one that matters — the RIM has to meet the ground.
                //
                // Every earlier version of this compared the water with the
                // ground directly beneath it, which the definition of the water
                // already guarantees, so it could never fail; it sat there
                // passing through four separate versions that all put sheets of
                // water on open grass. Then it compared against dry ground
                // further off, which fails on any hillside for the honest reason
                // that ground below a stream is below it.
                //
                // What a slab actually IS is a surface that ENDS above the land
                // it ends on — a step at its edge with daylight under it. So find
                // the edge and measure the step. A sample with a dry neighbour a
                // mesh step away is the rim of the drawn surface.
                let step = CHUNK_SIZE / RIVER_QUADS as f32;
                let rim = [Vec2::X, Vec2::NEG_X, Vec2::Y, Vec2::NEG_Y]
                    .iter()
                    .any(|out| {
                        let edge = at + *out * step;
                        terrain.river_surface(edge.x, edge.y).is_none()
                    });
                if rim {
                    rims += 1;
                    let lip = water - ground;
                    lips.push(lip);
                    if lip > worst {
                        worst = lip;
                        worst_at = at;
                    }
                    if lip > 0.15 {
                        over += 1;
                        if terrain.settlements.level(at).is_some() {
                            over_town += 1;
                        }
                    }
                }
            }
        }
        lips.sort_by(f32::total_cmp);
        let pick = |q: f32| lips[((lips.len() as f32 - 1.0) * q) as usize];
        println!(
            "river drawn at {standing} of 64,800; {rims} rim samples, lip p50 {:.3}              p99 {:.3} max {worst:.2}; {over} over 0.15 m, {over_town} of those levelled",
            pick(0.5),
            pick(0.99)
        );

        // NONE of them on levelled ground. This is the reported fault itself:
        // a town levels its site, which fills in whatever channel ran through
        // it, and the water carried on being drawn at the depth of a channel
        // that was no longer there. Rectangles of river lying on a town's flat
        // field — 787 of the 804 slabs in the world.
        assert_eq!(
            over_town, 0,
            "{over_town} rims stand proud on ground somebody levelled"
        );

        // Almost every rim meets the ground it ends on. Half of them are within
        // four centimetres and ninety-nine in a hundred within twelve, which is
        // under the height of the grass.
        assert!(pick(0.99) <= 0.12, "the river's rim: p99 {:.2} m", pick(0.99));

        // And the handful that do not are river MOUTHS on steep shores — the
        // ground falling more than a metre in the two the mesh has to fade
        // across, so the last vertex is left carrying water over a drop. It
        // reads as a rapid rather than as a slab, and chasing it further would
        // mean shallowing every river in the world to fix one vertex at a few
        // coasts. Bounded rather than hidden: this was hundreds when the fault
        // was real.
        assert!(
            over <= 2,
            "{over} rims of {rims} stand more than 0.15 m proud, worst {worst:.2} m              at {:.0}, {:.0}",
            worst_at.x,
            worst_at.y
        );
    }

    #[test]
    fn standing_in_a_river_is_standing_in_water() {
        // What the whole classification is for. A river runs well above the sea,
        // so until the ground could say it was flooded, one read as whatever land
        // it had cut through — and anything aquatic had nowhere to live but the
        // coast.
        let terrain = Terrain::new();
        let half = terrain.half();

        let mut found = None;
        'looking: for step_z in -80..80 {
            for step_x in -160..160 {
                let at = Vec2::new(
                    step_x as f32 / 160.0 * half.x,
                    step_z as f32 / 80.0 * half.y,
                );
                // Inland, so this cannot be the sea answering.
                if terrain.shore_meters(at.x, at.y) < 300.0 {
                    continue;
                }
                if terrain.river_surface(at.x, at.y).is_some() {
                    found = Some(at);
                    break 'looking;
                }
            }
        }

        {
            // Printed always, not only on failure. Whether a world has the right
            // AMOUNT of water is a judgement nobody can make from a pass or a
            // fail, and these are the numbers the threshold was set from.
            let mut wet_by_shore: Vec<f32> = Vec::new();
            for step_z in -80..80 {
                for step_x in -160..160 {
                    let at = Vec2::new(
                        step_x as f32 / 160.0 * half.x,
                        step_z as f32 / 80.0 * half.y,
                    );
                    if terrain.river_surface(at.x, at.y).is_some() {
                        wet_by_shore.push(terrain.shore_meters(at.x, at.y));
                    }
                }
            }
            wet_by_shore.sort_by(f32::total_cmp);
            let world = half.x * 2.0 * half.y * 2.0;
            println!(
                "channels: {}   wet: {}   shore {:?}..{:?}
                 biggest catchment: {:.0} m2 = {:.4} of the world ({:.0} m2)",
                terrain.rivers.channel_cells(),
                wet_by_shore.len(),
                wet_by_shore.first(),
                wet_by_shore.last(),
                terrain.rivers.largest_catchment(),
                terrain.rivers.largest_catchment() / world,
                world,
            );
        }
        let Some(at) = found else {
            panic!("no inland river to stand in");
        };
        let ground = terrain.ground_at(at.x, at.y);
        assert!(
            ground.height > 0.0,
            "this test is worthless if the spot is below sea level: {:.1} m",
            ground.height
        );
        assert!(ground.water_above > 0.0, "the ground should know it is flooded");
        assert_eq!(
            terrain.biome(at.x, at.y),
            Biome::Water,
            "a river should be water at {:.0}, {:.0}",
            at.x,
            at.y
        );
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
