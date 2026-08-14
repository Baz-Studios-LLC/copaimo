//! Loads the source map image and turns it into a CPU-side elevation source.
//!
//! This is what makes the world *ours* rather than whatever the noise felt like
//! producing: the continent outline, the inland seas, the island chains — all of
//! it comes from the map image.
//!
//! The image gives us three things:
//!
//! * **land coverage** — a cleaned land/sea mask. The authority on where the
//!   continents are.
//! * **inland distance** — how far each point is from the nearest coast, which
//!   is what lets mountains be placed in the interior rather than at random.
//! * **elevation** — normalized brightness, used for relief when the source is
//!   a real grayscale heightmap.

use std::collections::VecDeque;

use crate::config::{
    HEIGHTMAP_PATH, MAP_SEA_BLUE_MARGIN, MAP_SEA_THRESHOLD, MASK_CLEAN_PASSES, MASK_CLEAN_RADIUS,
    MIN_ISLAND_PIXELS,
};

/// Colour difference above which an image counts as colored rather than
/// grayscale, and the share of pixels that must clear it.
const COLOR_EVIDENCE: i16 = 20;
const COLOR_FRACTION: f32 = 0.02;

pub struct HeightMap {
    width: usize,
    height: usize,
    /// Normalized brightness in 0..1, row-major, north row first.
    elevation: Vec<f32>,
    /// Distance to the nearest sea pixel, in map pixels. 0 at sea, rising inland.
    inland: Vec<f32>,
    /// Distance to the nearest land pixel, in map pixels. 0 on land, rising out
    /// to sea. Subtracted from `inland` this is a signed distance to the coast.
    offshore: Vec<f32>,
    /// Whether `elevation` is meaningful relief rather than region fill colors.
    carries_elevation: bool,
}

impl HeightMap {
    /// Reads the map image from disk. Returns `None` (and the world falls back
    /// to pure procedural generation) if the file is missing or unreadable —
    /// the game should still run before any map has been dropped in.
    pub fn load() -> Option<Self> {
        let img = match image::open(HEIGHTMAP_PATH) {
            Ok(img) => img,
            Err(err) => {
                warn!("no world map at {HEIGHTMAP_PATH} ({err}) — using procedural fallback");
                return None;
            }
        };

        let rgb = img.to_rgb8();
        let (width, height) = (rgb.width() as usize, rgb.height() as usize);
        if width < 2 || height < 2 {
            warn!("world map at {HEIGHTMAP_PATH} is too small ({width}x{height})");
            return None;
        }

        let pixels: Vec<[u8; 3]> = rgb.pixels().map(|p| p.0).collect();
        let mut elevation: Vec<f32> = pixels.iter().map(|p| luma(*p)).collect();

        // Stretch the observed brightness range to a full 0..1, so a map that
        // only uses part of the range still produces full relief.
        //
        // Deliberately clipped at the 0.5th/99.5th percentile rather than the
        // true min and max: map exports carry outliers that aren't terrain —
        // black label text, a white scale bar, UI chrome caught in a screenshot.
        // A single black pixel would otherwise anchor the low end and squash
        // everything real into the top of the range.
        let (lo, hi) = percentile_range(&elevation, 0.005);
        if hi - lo > 1.0e-4 {
            let scale = 1.0 / (hi - lo);
            for v in &mut elevation {
                *v = ((*v - lo) * scale).clamp(0.0, 1.0);
            }
        }

        let (sea, carries_elevation) = classify_sea(&pixels, &elevation);
        let land = clean_mask(&sea, width, height);
        let inland = shore_distance(&land, width, height, false);
        let offshore = shore_distance(&land, width, height, true);

        let land_fraction = land.iter().filter(|&&v| v == 1).count() as f32 / land.len() as f32;
        info!(
            "loaded world map {width}x{height} from {HEIGHTMAP_PATH} ({:.0}% land)",
            land_fraction * 100.0
        );

        Some(Self {
            width,
            height,
            elevation,
            inland,
            offshore,
            carries_elevation,
        })
    }

    /// Whether the source carries real relief. False for a political map, whose
    /// brightness is region fill colors and means nothing as elevation.
    pub fn carries_elevation(&self) -> bool {
        self.carries_elevation
    }

    /// Width / height of the source image. The world's north–south extent is
    /// derived from this so the terrain never stretches the map out of shape.
    pub fn aspect(&self) -> f32 {
        self.width as f32 / self.height as f32
    }

    /// Width of the source image in pixels, for converting inland distance into
    /// meters against the world's scale.
    pub fn width(&self) -> usize {
        self.width
    }

    /// Distance from the nearest coast in map pixels, 0 at sea.
    pub fn inland_pixels(&self, u: f32, v: f32) -> f32 {
        self.sample(&self.inland, u, v)
    }

    /// Distance from the nearest land in map pixels, 0 on land.
    pub fn offshore_pixels(&self, u: f32, v: f32) -> f32 {
        self.sample(&self.offshore, u, v)
    }

    /// Where the map is furthest from any sea, in image space.
    ///
    /// The heart of the largest landmass, and so where a massif belongs: a
    /// mountain wants the most land around it, and the deepest interior is by
    /// definition the point with the most. Found rather than chosen, so
    /// redrawing the map moves the mountain to the new map's heartland instead
    /// of stranding it in a bay.
    pub fn deepest_inland(&self) -> (f32, f32) {
        let mut best = 0.0;
        let mut at = (0.5, 0.5);
        for (i, &away) in self.inland.iter().enumerate() {
            if away > best {
                best = away;
                at = (
                    (i % self.width) as f32 / (self.width - 1) as f32,
                    (i / self.width) as f32 / (self.height - 1) as f32,
                );
            }
        }
        at
    }

    /// Normalized source brightness, for maps that carry real elevation.
    pub fn elevation(&self, u: f32, v: f32) -> f32 {
        self.sample(&self.elevation, u, v)
    }

    /// Bilinear lookup into one of the fields. `u` runs west→east and `v`
    /// north→south, both in 0..1. Anything outside the image reads as 0, which
    /// is what makes the world finite: sail far enough and there is only sea.
    fn sample(&self, field: &[f32], u: f32, v: f32) -> f32 {
        if !(0.0..=1.0).contains(&u) || !(0.0..=1.0).contains(&v) {
            return 0.0;
        }

        // Sample at pixel centers so the edges of the image don't read as a
        // half-pixel of stretched color.
        let fx = (u * self.width as f32 - 0.5).clamp(0.0, self.width as f32 - 1.0);
        let fy = (v * self.height as f32 - 0.5).clamp(0.0, self.height as f32 - 1.0);

        let x0 = fx.floor() as usize;
        let y0 = fy.floor() as usize;
        let x1 = (x0 + 1).min(self.width - 1);
        let y1 = (y0 + 1).min(self.height - 1);
        let tx = fx - x0 as f32;
        let ty = fy - y0 as f32;

        let at = |x: usize, y: usize| field[y * self.width + x];
        let top = at(x0, y0) * (1.0 - tx) + at(x1, y0) * tx;
        let bottom = at(x0, y1) * (1.0 - tx) + at(x1, y1) * tx;
        top * (1.0 - ty) + bottom * ty
    }
}

fn luma(p: [u8; 3]) -> f32 {
    (0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32) / 255.0
}

/// Decides which pixels are sea. Returns 1 for sea, 0 for land.
///
/// **Colored maps are classified by hue, not brightness.** Brightness cannot
/// tell open water from a black label, a road, or a dashed border — all of them
/// are dark — so a brightness threshold cuts every place name on the map into
/// the terrain as a lake. Water is the one thing on a political map that is
/// distinctly *blue*, so that's what gets tested: blue meaningfully greater
/// than red. Labels and borders are neutral or warm and stay land.
///
/// A genuine grayscale heightmap has no hue to test, so it's detected and
/// thresholded on brightness as before.
/// Returns the sea mask and whether the image carries real elevation.
fn classify_sea(pixels: &[[u8; 3]], elevation: &[f32]) -> (Vec<u8>, bool) {
    let colored = pixels
        .iter()
        .filter(|p| (p[2] as i16 - p[0] as i16).abs() > COLOR_EVIDENCE)
        .count() as f32
        / pixels.len() as f32;

    if colored < COLOR_FRACTION {
        info!("map reads as grayscale - brightness is the waterline and the relief");
        let sea = elevation
            .iter()
            .map(|&v| u8::from(v <= MAP_SEA_THRESHOLD))
            .collect();
        return (sea, true);
    }

    info!("map reads as colored - blue dominance is the waterline, brightness ignored");
    let sea = pixels
        .iter()
        .map(|p| u8::from(p[2] as i16 - p[0] as i16 > MAP_SEA_BLUE_MARGIN))
        .collect();
    (sea, false)
}

/// Erase the line work and return a land mask (1 = land).
///
/// A majority filter makes each pixel whatever most of its neighbourhood is. A
/// river or a border a few pixels wide is outvoted by the land around it and
/// disappears; a coastline has land on one side and sea on the other all the
/// way along, so it holds its position. Repeated passes widen the reach without
/// needing one enormous window.
fn clean_mask(sea: &[u8], width: usize, height: usize) -> Vec<u8> {
    let mut land: Vec<u8> = sea.iter().map(|&s| 1 - s).collect();
    for _ in 0..MASK_CLEAN_PASSES {
        land = majority_filter(&land, width, height, MASK_CLEAN_RADIUS);
    }
    drop_small_islands(&mut land, width, height);
    land
}

/// Deletes land blobs smaller than `MIN_ISLAND_PIXELS`.
///
/// Map images are rarely just maps. A screenshot carries the tool's own
/// furniture — buttons, a scale bar, a legend — and none of it is water-colored,
/// so it survives every test above and turns into little rectangular islands
/// out at sea. Real islands are far larger than any of it, so size alone
/// separates them. Cropping the source is still the cleaner fix; this makes an
/// uncropped one usable.
fn drop_small_islands(land: &mut [u8], width: usize, height: usize) {
    let mut visited = vec![false; land.len()];
    let mut component = Vec::new();
    let mut queue = VecDeque::new();

    for start in 0..land.len() {
        if land[start] == 0 || visited[start] {
            continue;
        }

        component.clear();
        queue.push_back(start);
        visited[start] = true;

        while let Some(index) = queue.pop_front() {
            component.push(index);
            let (x, y) = (index % width, index / width);

            let mut visit = |nx: usize, ny: usize, queue: &mut VecDeque<usize>| {
                let neighbour = ny * width + nx;
                if land[neighbour] == 1 && !visited[neighbour] {
                    visited[neighbour] = true;
                    queue.push_back(neighbour);
                }
            };

            if x > 0 {
                visit(x - 1, y, &mut queue);
            }
            if x + 1 < width {
                visit(x + 1, y, &mut queue);
            }
            if y > 0 {
                visit(x, y - 1, &mut queue);
            }
            if y + 1 < height {
                visit(x, y + 1, &mut queue);
            }
        }

        if component.len() < MIN_ISLAND_PIXELS {
            for &index in &component {
                land[index] = 0;
            }
        }
    }
}

/// Distance from the coast, in pixels, measured one way.
///
/// `offshore` false gives each land pixel its distance from the sea — which is
/// what makes mountain placement geographic rather than arbitrary, since ranges
/// belong in the interior with plains between them and the coast. `offshore`
/// true gives each sea pixel its distance from the land, which is what lets the
/// sea floor fall away gradually instead of dropping off a step at the shore.
///
/// Together they form a signed distance to the coast, and that is what the
/// terrain is actually built on. One sweep each, once, at load.
fn shore_distance(land: &[u8], width: usize, height: usize, offshore: bool) -> Vec<f32> {
    let mut distance = vec![f32::MAX; land.len()];
    let mut queue = VecDeque::new();

    // Measuring starts from the far side: sea when measuring inland, land when
    // measuring out to sea.
    let source = u8::from(offshore);
    for y in 0..height {
        for x in 0..width {
            let index = y * width + x;
            // Land running off the image border isn't infinitely inland, so the
            // border seeds too — but only when measuring inland, or open sea at
            // the map's edge would read as touching land.
            let edge = !offshore && (x == 0 || y == 0 || x == width - 1 || y == height - 1);
            if land[index] == source || edge {
                distance[index] = 0.0;
                queue.push_back(index);
            }
        }
    }

    while let Some(index) = queue.pop_front() {
        let (x, y) = (index % width, index / width);
        let next = distance[index] + 1.0;

        let mut visit = |nx: usize, ny: usize| {
            let neighbour = ny * width + nx;
            if distance[neighbour] > next {
                distance[neighbour] = next;
                queue.push_back(neighbour);
            }
        };

        if x > 0 {
            visit(x - 1, y);
        }
        if x + 1 < width {
            visit(x + 1, y);
        }
        if y > 0 {
            visit(x, y - 1);
        }
        if y + 1 < height {
            visit(x, y + 1);
        }
    }

    distance
}

/// Summed-area table, so every box query below is four lookups regardless of
/// the window size. Sized one larger in each axis to avoid edge special-casing.
fn integral_image(mask: &[u8], width: usize, height: usize) -> Vec<u32> {
    let stride = width + 1;
    let mut integral = vec![0u32; stride * (height + 1)];
    for y in 0..height {
        let mut row_total = 0u32;
        for x in 0..width {
            row_total += mask[y * width + x] as u32;
            integral[(y + 1) * stride + x + 1] = integral[y * stride + x + 1] + row_total;
        }
    }
    integral
}

/// Total and area of the clamped box around a pixel.
fn box_stats(
    integral: &[u32],
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    radius: usize,
) -> (u32, u32) {
    let stride = width + 1;
    let x0 = x.saturating_sub(radius);
    let y0 = y.saturating_sub(radius);
    let x1 = (x + radius).min(width - 1);
    let y1 = (y + radius).min(height - 1);

    let total = integral[(y1 + 1) * stride + x1 + 1] + integral[y0 * stride + x0]
        - integral[y0 * stride + x1 + 1]
        - integral[(y1 + 1) * stride + x0];
    let area = ((x1 - x0 + 1) * (y1 - y0 + 1)) as u32;
    (total, area)
}

fn majority_filter(mask: &[u8], width: usize, height: usize, radius: usize) -> Vec<u8> {
    let integral = integral_image(mask, width, height);
    let mut out = vec![0u8; width * height];
    for y in 0..height {
        for x in 0..width {
            let (total, area) = box_stats(&integral, width, height, x, y, radius);
            out[y * width + x] = u8::from(total * 2 > area);
        }
    }
    out
}

/// Brightness values with `tail` of the population excluded from each end.
/// Uses a 256-bin histogram rather than sorting three million samples.
fn percentile_range(samples: &[f32], tail: f32) -> (f32, f32) {
    const BINS: usize = 256;

    let mut histogram = [0usize; BINS];
    for &v in samples {
        let bin = ((v * (BINS - 1) as f32).round() as usize).min(BINS - 1);
        histogram[bin] += 1;
    }

    let cutoff = (samples.len() as f32 * tail) as usize;
    let bin_value = |bin: usize| bin as f32 / (BINS - 1) as f32;

    let mut running = 0;
    let mut low = 0.0;
    for (bin, count) in histogram.iter().enumerate() {
        running += count;
        if running > cutoff {
            low = bin_value(bin);
            break;
        }
    }

    let mut running = 0;
    let mut high = 1.0;
    for (bin, count) in histogram.iter().enumerate().rev() {
        running += count;
        if running > cutoff {
            high = bin_value(bin);
            break;
        }
    }

    (low, high)
}

// `warn!`/`info!` come from Bevy's logging, which is just `tracing` underneath.
use bevy::log::{info, warn};
