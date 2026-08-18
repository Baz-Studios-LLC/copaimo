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

use crate::config::{CHILL_SNOWLINE, CHILL_TREELINE, SEA_LEVEL, SNOWLINE, TREELINE};
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
    /// Bare earth, for roads and yards and the worn ground round a door.
    dirt: Vec3,
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
    // Warm and mid — a cart road in daylight, not mud and not sand. Reads as
    // earth against both the dry and the lush grass it has to sit between.
    dirt: linear(0.40, 0.31, 0.20),
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
/// * `arid`, `chill` — which region this is, from [`terrain_core::region`]
///
/// # The ground and the biome have to agree
///
/// This is a second path: [`terrain_core::biome::Biome::of`] decides what a place
/// IS, and this decides what it LOOKS like, and for a long time they were told
/// different things. The classifier knew about the regions and this did not, so
/// the northern desert was classified desert and painted as dry grassland, and
/// snow country was classified snow and painted green until two hundred metres up.
///
/// They were also working off two different snow lines — 165 m for deciding, 210 m
/// for painting — which is the same fault in its purest form: one idea, two
/// numbers, and forty-five metres of world where the ground disagreed with itself
/// about what it was. There is one number now and both paths read it.
pub fn surface_color(
    height: f32,
    slope: f32,
    moisture: f32,
    character: f32,
    worn: f32,
    arid: f32,
    chill: f32,
) -> [f32; 4] {
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

    // And a dry region is SAND, not dry grass.
    //
    // Sand used to appear only in the shoreline band, so the driest ground the
    // world could paint was the khaki at the bottom of the grass ramp — which is
    // a parched meadow, not a desert. The rim of the region keeps that khaki and
    // the heart of it is sand, which is also roughly how a desert actually goes.
    let parched = vegetated.lerp(p.sand, smoothstep(0.30, 0.85, arid));

    // Altitude strips the greenery back to bare alpine ground, then to snow —
    // and a cold region brings both lines down to meet the ground, exactly as it
    // does for the classifier. That is what makes snow country white rather than
    // green with a white mountain in the middle of it.
    let treeline = TREELINE - TREELINE * CHILL_TREELINE * chill;
    let snowline = SNOWLINE - SNOWLINE * CHILL_SNOWLINE * chill;
    let above_treeline = parched.lerp(p.alpine, smoothstep(treeline - 25.0, treeline + 40.0, height));
    let capped = above_treeline.lerp(p.snow, smoothstep(snowline - 30.0, snowline + 20.0, height));

    let mut color = if height >= SEA_LEVEL { capped } else { underwater };

    // The shoreline band: how close to the waterline this is, fading out with
    // height from both sides rather than ending at a line.
    let shoreline = 1.0 - smoothstep(BEACH_FULL, BEACH_GONE, (height - SEA_LEVEL).abs());

    // What the band is *made of* is the point. Sand is not the default state of
    // a coast — it needs somewhere for sediment to settle, which means a gentle
    // shore, and it changes along the coast rather than being true of the whole
    // map. Where those don't hold, the sea meets rock instead. A world with
    // every continent outlined in sand reads as a drawing of a map, not ground.
    let gentle = 1.0 - smoothstep(0.06, 0.22, slope);
    let sandy = shoreline * character * gentle;
    let stony = shoreline * (1.0 - character * gentle);

    color = color.lerp(p.rock, stony * 0.7);
    color = color.lerp(p.sand, sandy);

    // Steep ground is bare rock no matter what biome it sits in — this is what
    // makes cliffs and mountainsides read as stone instead of vertical lawn.
    color = color.lerp(p.rock, smoothstep(0.34, 0.62, slope));

    // What somebody DECIDED the ground is, last of all — a road is a road
    // whatever the climate says should be growing there, and it is laid over
    // the biome rather than argued with it.
    //
    // Only above the waterline: a road painted into the sea would be a stripe
    // of dirt floating on the water, and refusing it here costs nothing.
    if worn > 0.0 && height >= SEA_LEVEL {
        // Not to the full colour at full bias. Ground wears to bare earth with
        // grass still coming through at the edges of the ruts, and a road laid
        // as flat paint reads as a decal on the landscape.
        color = color.lerp(p.dirt, (worn * 0.86).clamp(0.0, 1.0));
    } else if worn < 0.0 {
        // Forced green: whatever the biome had, grown back over.
        color = color.lerp(p.lush_grass, (-worn * 0.7).clamp(0.0, 1.0));
    }

    [color.x, color.y, color.z, 1.0]
}
