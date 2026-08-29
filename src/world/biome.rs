//! Turns a point's height, slope and country into a surface color.
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

use crate::config::{SEA_LEVEL, SNOWLINE, TREELINE};
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
    /// What a modern city stands on. Cool and pale, so it reads as a made surface
    /// rather than as very dry ground - the two ages of the world have to be
    /// different underfoot as well as overhead.
    paving: Vec3,
    shallow: Vec3,
    sand: Vec3,
    lush_grass: Vec3,
    rock: Vec3,
    alpine: Vec3,
    snow: Vec3,
}

static PALETTE: LazyLock<Palette> = LazyLock::new(|| Palette {
    silt: linear(0.09, 0.15, 0.22),
    // Warm and mid — a cart road in daylight, not mud and not sand. Reads as
    // earth against both the dry and the lush grass it has to sit between.
    dirt: linear(0.40, 0.31, 0.20),
    paving: linear(0.52, 0.52, 0.53),
    shallow: linear(0.22, 0.38, 0.46),
    sand: linear(0.74, 0.68, 0.50),
    lush_grass: linear(0.26, 0.47, 0.22),
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
/// * `country` — which country this is, from [`terrain_core::region`]
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
    at: Vec2,
    height: f32,
    slope: f32,
    character: f32,
    worn: f32,
    country: terrain_core::region::Country,
    belonging: f32,
    // How built-up this ground is: positive for the old world's packed earth,
    // negative for a modern city's paving, zero for open country.
    settled: f32,
) -> [f32; 4] {
    let p = &*PALETTE;

    // Underwater, by **depth**: dark in the deep, lightening as it shallows.
    // Deliberately not sand — the beach is a separate band added below, and
    // running the sea floor to sand made every gradual shelf pale for hundreds
    // of meters, which is what turned the whole world into beaches.
    let depth = SEA_LEVEL - height;
    let underwater = p.silt.lerp(p.shallow, smoothstep(45.0, 3.0, depth));

    // The green world is one green.
    //
    // This was a ramp from dry grass through lush grass to forest, driven by a
    // moisture reading — and there is no moisture. A patch of the green world is
    // not drier than the next patch; it is the same country, and what varies
    // across it is what is STANDING on it, which is trees and grass and boulders
    // rather than the colour of the dirt.
    let vegetated = p.lush_grass;

    // # Colour BLENDS where the biome switches
    //
    // A country is a hard choice and it has to be — a place either grows trees or
    // it does not. But painting from a hard choice draws a LINE across the ground,
    // and dithering the choice only makes the line wiggle: a threshold on a smooth
    // field is a line however it is jittered.
    //
    // So the category stays discrete and the colour does not. How firmly somewhere
    // belongs to its region mixes the two grounds, which is what turns a boundary
    // into the band of sand-through-scrub-through-grass that a desert's edge
    // actually is. The trees still stop along a ragged line, and that line is now
    // somewhere inside the band instead of being the band.
    //
    // Over the WHOLE of the belonging, not a clipped middle of it. This ran from
    // 0.15 to 0.85, which spends the same total change over seventy per cent of
    // the band — a steeper middle for no visual gain, and the middle was already
    // the steep part with the cap fading on top of it.
    let into = smoothstep(0.0, 1.0, belonging);

    // Sand, not the khaki at the dry end of a grass ramp. Sand existed in this
    // palette all along and was reachable only from the shoreline band, so the
    // driest thing the world could paint was a parched meadow.
    //
    // And snow country is WHITE, all of it, down to the water.
    //
    // Its low ground used to paint conifer green, because that is the biome down
    // there — and the result was a ring of green around every white island, which
    // reads as snow that stops before the shoreline. It does not: a snowy forest
    // is conifers standing ON snow. The ground goes white and the trees are still
    // planted, which is both what it looks like and what was actually wanted from
    // "there should be trees in snowy areas".
    let (elsewhere, snowline) = match country {
        terrain_core::region::Country::Desert => (p.sand, SNOWLINE),
        terrain_core::region::Country::Snow => (p.snow, -1_000.0),
        terrain_core::region::Country::Ordinary => (vegetated, SNOWLINE),
    };
    let ground_colour = vegetated.lerp(elsewhere, into);

    // Altitude strips the greenery back to bare stone, then to snow.
    let above_treeline =
        ground_colour.lerp(p.alpine, smoothstep(TREELINE - 25.0, TREELINE + 40.0, height));

    // How much of the white cap this height wears: what the world's own snow
    // line says, blended toward what this COUNTRY's snow line says.
    //
    // # Blend the amount, never sweep the window
    //
    // The snow line itself used to travel with the boundary — interpolated from
    // the world's ~165 m down to snow country's minus-a-thousand as `into` rose.
    // Arithmetically tidy, and a cliff on the ground: the cap is a smoothstep
    // window only fifty metres tall, and sliding its POSITION through eleven
    // hundred metres of height means it crosses any given piece of ground in a
    // few hundredths of `into` — under a metre of walk at a painted edge. The
    // ground colour faded gently across the band while the white cap switched on
    // like a shutter in the middle of it, which is the hard join the maker
    // photographed.
    //
    // Both ends' answers are computed where they stand and the AMOUNT mixes, so
    // the cap fades in across the whole band exactly as the ground colour does.
    let cap_outside = smoothstep(SNOWLINE - 30.0, SNOWLINE + 20.0, height);
    let cap_inside = smoothstep(snowline - 30.0, snowline + 20.0, height);
    let cap = cap_outside + (cap_inside - cap_outside) * into;

    // BUT NOT ON THE CANYON MASSIF.
    //
    // Its top stands 170 m over the plain and the snow line is 165, so it cleared
    // the line by five metres and wore a white cap - a snow-capped mesa in the
    // middle of a desert, and the most prominent thing in that whole country once
    // the wall was lengthened to reach the sea.
    //
    // The snow line's own note says what is wrong with that: snow is meant to mean
    // THE mountain, the one landmark you navigate by, and nothing else in the world
    // is supposed to reach one. The canyon is a different landform with a different
    // job - it is the gate that closes the road east - and `world::pass` says in its
    // first paragraph that its walls strip to bare rock.
    //
    // Asked of the canyon itself rather than by moving the snow line, which would
    // have taken the cap off the mountain too. `pass::stands` answers most of the
    // world with one comparison, so this costs nothing away from the massif.
    let bare = (crate::world::pass::lift(at) / 40.0).clamp(0.0, 1.0);
    let cap = cap * (1.0 - bare);
    let capped = above_treeline.lerp(p.snow, cap);

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

    // A frozen shore is still a shore, and it is not sand.
    //
    // The classifier used to answer this by deleting the shore in cold country,
    // which fixed a ring of sand round a white island by removing the coastline —
    // and a coastline is a PLACE, with things living on it that live nowhere else.
    // So the shore stays everywhere and the cold is answered here, where the
    // question was: what it looks like, not whether it exists.
    let frozen = matches!(country, terrain_core::region::Country::Snow) as u8 as f32 * into;
    let beach = p.sand.lerp(p.snow, frozen);

    color = color.lerp(p.rock, stony * 0.7 * (1.0 - frozen));
    color = color.lerp(beach, sandy);

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

    // AND WHAT PEOPLE HAVE MADE OF IT.
    //
    // After the wear and before the blotching, because a town's ground is a made
    // surface and the blotching is grass growing unevenly ON a surface - which is
    // true of a village's earth and not of a city's pavement, so the fade below is
    // what keeps a paved square from sprouting.
    if settled > 0.0 && height >= SEA_LEVEL {
        color = color.lerp(p.dirt, (settled * 0.82).clamp(0.0, 1.0));
    } else if settled < 0.0 && height >= SEA_LEVEL {
        color = color.lerp(p.paving, (-settled * 0.9).clamp(0.0, 1.0));
    }

    // # Ground is never one colour, and this one was
    //
    // Standing on open grass, the whole screen was a single flat green with a
    // warden on it. Every ingredient above answers a question about the PLACE —
    // how high, how steep, which country — and a field answers all of them the
    // same way, so a field came out as paint.
    //
    // What breaks it up is not detail nobody can name: it is that grass grows
    // thicker in the hollows and thinner over the stony patches, in blotches a few
    // paces across and patches a field across. Two scales of it, laid on the
    // colour the place has already earned rather than mixed into the decisions
    // above — this changes what somewhere LOOKS like and must never change what it
    // IS, or a tree would stop growing because the ground under it went a shade
    // darker.
    //
    // Vertex colours, so it costs nothing to draw: the terrain mesh already
    // carries a colour a vertex, on a two-metre grid, and the fine scale is drawn
    // at about that size so the mesh can actually hold it.

    let mottle = mottle_at(at);
    // How much each surface takes. Snow is genuinely near-uniform and blotching it
    // reads as dirt; rock and grass are the opposite, and sand sits between.
    let takes = match country {
        terrain_core::region::Country::Snow => MOTTLE_SNOW,
        terrain_core::region::Country::Desert => MOTTLE_SAND,
        terrain_core::region::Country::Ordinary => MOTTLE,
    };
    // Nothing underwater: the sea is drawn over it, and a mottled sea floor read
    // through moving water is just noise.
    let takes = if height >= SEA_LEVEL { takes } else { 0.0 };

    // Brightness, and a lean toward the yellow of dry grass or the blue-green of
    // thick growth. Tone alone reads as lighting; it is the hue drift that reads
    // as different GROWTH on the same ground.
    color *= 1.0 + mottle * takes;
    let lean = mottle_at(at * DRIFT_SCALE + Vec2::splat(37.0)) * takes * DRIFT;
    color.x *= 1.0 + lean;
    color.z *= 1.0 - lean;

    [color.x.max(0.0), color.y.max(0.0), color.z.max(0.0), 1.0]
}

/// Two scales of blotching over the ground, about -0.5 to 0.5.
///
/// The broad one is patches a field across — where the ground is richer or
/// stonier. The fine one is the few-paces mottling within them, drawn at roughly
/// the terrain mesh's own vertex spacing, which is the finest thing vertex colours
/// can hold. Any finer and the mesh would average it away into flat paint again,
/// which is the very thing being fixed.
fn mottle_at(at: Vec2) -> f32 {
    // Each field is 0..1, so each term is a half either way and the pair of them
    // ADD rather than average: averaging two independent fields is what quietly
    // halves the spread, and the first attempt at this came out too faint to see
    // for exactly that reason. Roughly a whole either way at the extremes, a third
    // either way most of the time.
    let broad = terrain_core::forest::field(at / MOTTLE_BROAD, 73) - 0.5;
    let fine = terrain_core::forest::field(at / MOTTLE_FINE, 74) - 0.5;
    broad + fine
}

/// How far the ground strays from its own colour, by country, and the two scales
/// it strays at in metres.
///
/// Tuned by eye against a flat field at midday, which is the worst case: a slope
/// has its own shading to break it up and a field has nothing. `dump_the_ground`
/// plus `dev/ground.py` draws exactly that patch without launching anything, and
/// is how these numbers were arrived at rather than guessed.
///
/// The fine scale is deliberately several times the terrain's two-metre vertex
/// spacing. Finer than about four vertices and the mesh cannot hold the blotch: it
/// averages away into the flat paint this exists to break up.
const MOTTLE: f32 = 0.24;
const MOTTLE_SAND: f32 = 0.12;
const MOTTLE_SNOW: f32 = 0.05;
const MOTTLE_BROAD: f32 = 46.0;
const MOTTLE_FINE: f32 = 9.0;

/// How far the colour leans toward dry yellow or thick blue-green, and at what
/// scale relative to the tone.
///
/// Small. This is the difference between two patches of the same grass, not
/// between grass and something else.
const DRIFT: f32 = 0.18;
const DRIFT_SCALE: f32 = 0.55;
