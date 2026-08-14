//! Turns a point's height, slope and moisture into a surface color.
//!
//! Color is baked into the terrain mesh as vertex colors rather than painted
//! with textures. That's deliberate for this stage: it reads clearly, costs
//! nothing to author, and gives the world real biome variety long before there
//! are any art assets. When textures arrive, this same classification is what
//! chooses which one to blend.
//!
//! Every transition is a smoothstep, so biomes fade into one another instead of
//! drawing hard contour lines across the landscape.

use std::sync::LazyLock;

use bevy::color::LinearRgba;
use bevy::prelude::*;

use crate::config::{SEA_LEVEL, SNOW_LINE};
use crate::util::smoothstep;

/// Palette in *linear* space, which is what mesh vertex colors are interpreted
/// as. Converting from sRGB involves a pow per channel, so it's done once for
/// the whole process rather than per vertex.
/// How far above and below the waterline sand reaches, in **meters of height**.
///
/// Full sand within the first, gone by the second. Keyed to height rather than
/// to distance along the ground on purpose: keyed to distance, a beach widens
/// with its own gradient, so making the coast shelve gently turned every
/// shoreline on the map into a kilometer of sand.
const BEACH_FULL: f32 = 1.0;
const BEACH_GONE: f32 = 6.0;

struct Palette {
    silt: Vec3,
    shallow: Vec3,
    sand: Vec3,
    dry_grass: Vec3,
    lush_grass: Vec3,
    forest: Vec3,
    rock: Vec3,
    alpine: Vec3,
    snow: Vec3,
}

static PALETTE: LazyLock<Palette> = LazyLock::new(|| Palette {
    silt: linear(0.09, 0.15, 0.22),
    shallow: linear(0.22, 0.38, 0.46),
    sand: linear(0.74, 0.68, 0.50),
    dry_grass: linear(0.54, 0.55, 0.30),
    lush_grass: linear(0.26, 0.47, 0.22),
    forest: linear(0.15, 0.31, 0.17),
    rock: linear(0.36, 0.34, 0.32),
    alpine: linear(0.47, 0.45, 0.42),
    snow: linear(0.93, 0.94, 0.97),
});

fn linear(r: f32, g: f32, b: f32) -> Vec3 {
    let c = LinearRgba::from(Srgba::rgb(r, g, b));
    Vec3::new(c.red, c.green, c.blue)
}

/// Surface color as linear RGBA, ready for `Mesh::ATTRIBUTE_COLOR`.
///
/// * `height` — meters relative to sea level
/// * `slope`  — 0 for dead flat, approaching 1 for a vertical face
/// * `moisture` — 0 arid, 1 lush
pub fn surface_color(height: f32, slope: f32, moisture: f32) -> [f32; 4] {
    let p = &*PALETTE;

    // Underwater, by **depth**: dark in the deep, lightening as it shallows.
    // Deliberately not sand — the beach is a separate band added below, and
    // running the sea floor to sand made every gradual shelf pale for hundreds
    // of meters, which is what turned the whole world into beaches.
    let depth = SEA_LEVEL - height;
    let underwater = p.silt.lerp(p.shallow, smoothstep(45.0, 3.0, depth));

    // Vegetation: moisture picks dry plains → grassland, then tips into forest
    // once it's wet enough.
    let grass = p.dry_grass.lerp(p.lush_grass, smoothstep(0.25, 0.60, moisture));
    let vegetated = grass.lerp(p.forest, smoothstep(0.58, 0.88, moisture));

    // Altitude strips the greenery back to bare alpine ground, then to snow.
    let above_treeline = vegetated.lerp(p.alpine, smoothstep(125.0, 190.0, height));
    let capped = above_treeline.lerp(p.snow, smoothstep(SNOW_LINE - 30.0, SNOW_LINE + 20.0, height));

    let mut color = if height >= SEA_LEVEL { capped } else { underwater };

    // The beach, hugging the waterline from both sides and fading out with
    // height rather than ending at a line — a hard edge here traces every coast
    // on the map and reads as ink rather than as sand.
    let beach = 1.0 - smoothstep(BEACH_FULL, BEACH_GONE, (height - SEA_LEVEL).abs());
    color = color.lerp(p.sand, beach);

    // Steep ground is bare rock no matter what biome it sits in — this is what
    // makes cliffs and mountainsides read as stone instead of vertical lawn.
    color = color.lerp(p.rock, smoothstep(0.34, 0.62, slope));

    [color.x, color.y, color.z, 1.0]
}
