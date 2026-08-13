//! Loads the source map image and turns it into a CPU-side elevation source.
//!
//! This is what makes the world *ours* rather than whatever the noise felt like
//! producing: the continent outline, the inland seas, the island chains — all of
//! it comes from the map image.
//!
//! The image gives us two separate things:
//!
//! * **land coverage** — a cleaned, smoothed land/sea mask. This is the
//!   authority on where the continents are.
//! * **elevation** — normalized brightness, used for relief when the source is
//!   a real grayscale heightmap.
//!
//! Coverage gets its own cleaning pass because real maps are covered in line
//! work — region borders, rivers, roads, labels. Those are dark pixels, and read
//! naively they carve deep trenches across the continents that alias into rows
//! of spikes against the terrain's vertex grid. A majority filter erases
//! anything thinner than the brush and leaves coastlines untouched.

use crate::config::{
    HEIGHTMAP_PATH, MAP_SEA_THRESHOLD, MASK_BLUR_RADIUS, MASK_CLEAN_PASSES, MASK_CLEAN_RADIUS,
};

pub struct HeightMap {
    width: usize,
    height: usize,
    /// Normalized brightness in 0..1, row-major, north row first.
    elevation: Vec<f32>,
    /// Cleaned land coverage in 0..1. 0 is solidly sea, 1 solidly land, with a
    /// soft band along the coast.
    coverage: Vec<f32>,
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

        let luma = img.to_luma8();
        let (width, height) = (luma.width() as usize, luma.height() as usize);
        if width < 2 || height < 2 {
            warn!("world map at {HEIGHTMAP_PATH} is too small ({width}x{height})");
            return None;
        }

        let mut elevation: Vec<f32> = luma.pixels().map(|p| p[0] as f32 / 255.0).collect();

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

        let coverage = build_coverage(&elevation, width, height);

        let land_fraction =
            coverage.iter().filter(|&&v| v > 0.5).count() as f32 / coverage.len() as f32;
        info!(
            "loaded world map {width}x{height} from {HEIGHTMAP_PATH} ({:.0}% land)",
            land_fraction * 100.0
        );

        Some(Self {
            width,
            height,
            elevation,
            coverage,
        })
    }

    /// Width / height of the source image. The world's north–south extent is
    /// derived from this so the terrain never stretches the map out of shape.
    pub fn aspect(&self) -> f32 {
        self.width as f32 / self.height as f32
    }

    /// Cleaned land coverage, 0 (open sea) to 1 (solidly inland).
    pub fn coverage(&self, u: f32, v: f32) -> f32 {
        self.sample(&self.coverage, u, v)
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

/// Threshold the map into land/sea, erase the line work, and soften the result
/// into a 0..1 coverage field.
fn build_coverage(elevation: &[f32], width: usize, height: usize) -> Vec<f32> {
    let mut mask: Vec<u8> = elevation
        .iter()
        .map(|&v| u8::from(v > MAP_SEA_THRESHOLD))
        .collect();

    // Majority filter: each pixel becomes whatever most of its neighbourhood is.
    // A border line or a river a few pixels wide is outvoted by the land around
    // it and disappears; a coastline has land on one side and sea on the other
    // all the way along, so it holds its position. Repeated passes widen the
    // reach without needing one enormous window.
    for _ in 0..MASK_CLEAN_PASSES {
        mask = majority_filter(&mask, width, height, MASK_CLEAN_RADIUS);
    }

    // Soften the hard 0/1 edge into a coastal ramp, so beaches shelve into the
    // water instead of dropping off a step.
    box_blur(&mask, width, height, MASK_BLUR_RADIUS)
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

fn box_blur(mask: &[u8], width: usize, height: usize, radius: usize) -> Vec<f32> {
    let integral = integral_image(mask, width, height);
    let mut out = vec![0.0f32; width * height];
    for y in 0..height {
        for x in 0..width {
            let (total, area) = box_stats(&integral, width, height, x, y, radius);
            out[y * width + x] = total as f32 / area as f32;
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
