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
use crate::world::edit::EditGrid;
use crate::world::heightmap::HeightMap;

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
    edits: RwLock<EditGrid>,
}

impl Terrain {
    pub fn new() -> Self {
        let map = HeightMap::load();

        // The map image decides the world's proportions; WORLD_WIDTH decides
        // its scale. A 2:1 map at 8192 m across is 4096 m tall.
        let aspect = map.as_ref().map_or(FALLBACK_ASPECT, HeightMap::aspect);
        let half = Vec2::new(WORLD_WIDTH * 0.5, WORLD_WIDTH / aspect * 0.5);
        let map_carries_elevation = map.as_ref().is_some_and(HeightMap::carries_elevation);

        Self {
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
            detail: Fbm::<Perlin>::new(WORLD_SEED.wrapping_add(1))
                .set_octaves(4)
                .set_frequency(1.0),
            moisture: Fbm::<Perlin>::new(WORLD_SEED.wrapping_add(2)).set_octaves(3),
            warp_x: Perlin::new(WORLD_SEED.wrapping_add(3)),
            warp_z: Perlin::new(WORLD_SEED.wrapping_add(4)),
            continent: Fbm::<Perlin>::new(WORLD_SEED.wrapping_add(5)).set_octaves(5),
            edits: RwLock::new(EditGrid::load(half)),
        }
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
            Ok(edits) => generated + edits.sample(x, z),
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

        h += self.range_height(wx, wz, inland) * coast;

        // Fine detail, masked to the land — the sea floor is under water and
        // mostly hidden, and keeping it calm is both cheaper and smoother.
        let d = self.detail.get([wx as f64 * DETAIL_FREQ, wz as f64 * DETAIL_FREQ]) as f32;
        h + d * DETAIL_ELEVATION * coast
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

    /// Moisture at a world position, 0 (arid) to 1 (lush). Drives biome color;
    /// later it can drive which monsters live where.
    pub fn moisture(&self, x: f32, z: f32) -> f32 {
        let m = self
            .moisture
            .get([x as f64 * MOISTURE_FREQ, z as f64 * MOISTURE_FREQ]) as f32;
        (m * 0.5 + 0.5).clamp(0.0, 1.0)
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
             furthest from any coast: {deepest_inland:.0} m (INLAND_FULL is {INLAND_FULL:.0} m)\n\n{picture}",
            if terrain.has_map() { "map image" } else { "procedural fallback" },
            half.x * 2.0,
            half.y * 2.0,
            land_fraction * 100.0,
            trough,
            peak,
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
