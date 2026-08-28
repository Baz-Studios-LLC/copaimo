//! Fallen leaves: what autumn leaves on the ground.
//!
//! # It goes in the cover mesh, not in a layer of its own
//!
//! A fallen leaf is ground cover. It is scattered per slot, it belongs to the chunk
//! it lies on, it is built off the main thread with the grass, and it is thrown away
//! when the viewer walks off — every one of those is something `cover` already does,
//! and a second system doing them again beside it would be the same work twice with
//! twice the places for it to go wrong.
//!
//! So litter is a few more triangles pushed into the mesh the grass is already
//! being written into, decided at the same slot, off the same hashes.
//!
//! # It lies where the leaves came from
//!
//! Not everywhere it is autumn. Leaves fall off broadleaf trees, so litter follows
//! the ground that grows them — thick under woodland, thin on open grass near it,
//! and nothing at all on sand, rock, snow or water. That is the same instinct the
//! weather follows: the country decides, rather than the effect being laid over
//! everything and hidden where it does not fit.
//!
//! # Flat, and lying at every angle
//!
//! A leaf on the ground is a scrap lying flat. These are two triangles apiece with
//! their normal pointing UP rather than along the ground's own slope — the same
//! choice the grass makes, and for the same reason: lit honestly, a scatter of
//! little flat things flickers as the camera turns.

use bevy::prelude::*;

use terrain_core::cover as sprigs;
use terrain_core::Geometry;

use crate::season::Season;
use crate::world::terrain::Biome;

/// How many leaves a slot can carry where the litter is thickest.
///
/// Three. A slot is a grass tuft's worth of ground, and the litter has to read as a
/// scatter rather than as a carpet — a carpet is a texture, and this is not one.
const MOST_PER_SLOT: usize = 3;

/// How wide a leaf is, in metres, before its own variation.
const LEAF_WIDE: f32 = 0.085;

/// How far a leaf sits above the ground it lies on.
///
/// A centimetre and a half. Flat on the surface z-fights with it; any higher and
/// the leaves hover, which reads as litter floating rather than lying.
const LEAF_LIFTS: f32 = 0.015;

/// The share of slots that carry any litter at all, at its thickest.
const THICKEST: f32 = 0.55;

/// Salts of this crate's own, well clear of the ones `terrain_core::cover` uses.
const SALT_LITTER: u32 = 91;
const SALT_LEAF_TURN: u32 = 92;
const SALT_LEAF_SIZE: u32 = 93;
const SALT_LEAF_TINT: u32 = 94;
const SALT_LEAF_OFF: u32 = 95;

/// The colours fallen leaves come in — russet through amber, and a spent brown.
///
/// Drawn from the same russets the canopy turns, because the litter is where that
/// canopy went. Browner on average, though: a leaf on the ground has been there a
/// while, and a lawn of the same amber as the tree above it reads as a spill.
const LITTER: [[f32; 3]; 4] = [
    [0.42, 0.20, 0.07],
    [0.55, 0.28, 0.08],
    [0.62, 0.38, 0.11],
    [0.33, 0.22, 0.12],
];

/// How thickly leaves lie on this ground, 0 to 1.
///
/// Woodland is where they fall, so it gets the most; open grass beside it catches
/// what blows off the wood. Everywhere else gets none — there is nothing overhead
/// on sand or rock or snow to have dropped anything.
///
/// # Across the whole season, not just its last week
///
/// `through` and not `turning`. The colour of a canopy turns in a week and holds
/// the rest of the season, so `turning` is the right clock for it; litter does the
/// opposite — it builds all through autumn, lies all winter, and rots away through
/// spring. Driven off `turning` it was flat for three quarters of every season and
/// then jumped: winter ended with the wood 45% covered and spring began with it
/// bare, so a month of leaves vanished between one night and the next morning.
///
/// The four pieces are written to MEET at the boundaries — spring starts at exactly
/// what winter ended with — which is what makes the year continuous rather than
/// four curves that happen to sit next to each other.
pub fn how_thick(biome: Biome, season: Season, through: f32) -> f32 {
    let ground = match biome {
        Biome::Forest => 1.0,
        Biome::Grass => 0.45,
        _ => 0.0,
    };
    let through = through.clamp(0.0, 1.0);
    let year = match season {
        // Bare, and staying bare: last year's leaves are gone and this year's are
        // still on the branch.
        Season::Summer => 0.0,
        // Falling. Nothing on the first morning, a full floor by the last night.
        Season::Autumn => through,
        // Lying, and slowly going into the ground.
        Season::Winter => 1.0 - WINTER_ROTS * through,
        // What is left of winter's, rotting away to nothing by summer.
        Season::Spring => (1.0 - WINTER_ROTS) * (1.0 - through),
    };
    ground * year.clamp(0.0, 1.0)
}

/// How much of winter's leaf fall has rotted down by the end of it.
const WINTER_ROTS: f32 = 0.55;

/// Scatters the leaves for one slot into the cover mesh.
///
/// Takes the slot the grass took and the ground it already asked about, so this
/// costs no extra terrain work — the expensive question has been answered by the
/// time we are called.
#[allow(clippy::too_many_arguments)]
pub fn scatter(
    into: &mut Geometry,
    slot_x: i32,
    slot_z: i32,
    at: Vec3,
    biome: Biome,
    season: Season,
    through: f32,
) {
    let thick = how_thick(biome, season, through);
    if thick <= 0.0 {
        return;
    }
    if sprigs::chance(slot_x, slot_z, SALT_LITTER) > thick * THICKEST {
        return;
    }

    let many = 1 + (sprigs::chance(slot_x, slot_z, SALT_LEAF_SIZE) * MOST_PER_SLOT as f32) as usize;
    for leaf in 0..many.min(MOST_PER_SLOT) {
        // Each leaf in a slot gets its own corner of the hash, so two leaves in one
        // slot are not the same leaf drawn twice.
        let step = leaf as i32 * 37;
        let turn = sprigs::chance(slot_x + step, slot_z, SALT_LEAF_TURN) * std::f32::consts::TAU;
        let size = LEAF_WIDE * (0.7 + 0.7 * sprigs::chance(slot_x, slot_z + step, SALT_LEAF_SIZE));
        let off = Vec3::new(
            (sprigs::chance(slot_x + step, slot_z, SALT_LEAF_OFF) - 0.5) * 0.55,
            0.0,
            (sprigs::chance(slot_x, slot_z + step, SALT_LEAF_OFF) - 0.5) * 0.55,
        );
        let shade = sprigs::chance(slot_x + step, slot_z + step, SALT_LEAF_TINT);
        let tint = LITTER[(shade * LITTER.len() as f32) as usize % LITTER.len()];

        put_a_leaf(into, at + off + Vec3::Y * LEAF_LIFTS, turn, size, tint);
    }
}

/// One leaf: two triangles, flat, lying at whatever angle it fell.
fn put_a_leaf(into: &mut Geometry, at: Vec3, turn: f32, size: f32, tint: [f32; 3]) {
    let base = into.places.len() as u32;
    let (sin, cos) = turn.sin_cos();
    // A leaf is longer than it is wide, which is what keeps it from reading as
    // confetti.
    let long = size * 1.5;
    let corners = [
        Vec3::new(-size, 0.0, -long),
        Vec3::new(size, 0.0, -long),
        Vec3::new(size, 0.0, long),
        Vec3::new(-size, 0.0, long),
    ];
    let uvs = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];

    for (corner, uv) in corners.iter().zip(uvs) {
        let spun = Vec3::new(
            corner.x * cos - corner.z * sin,
            corner.y,
            corner.x * sin + corner.z * cos,
        );
        into.places.push((at + spun).to_array());
        // UP, like the grass, and for the same reason: a scatter of little flat
        // things lit by their own normals flickers as the camera turns.
        into.normals.push([0.0, 1.0, 0.0]);
        into.uvs.push(uv);
        into.colours.push([tint[0], tint[1], tint[2], 1.0]);
    }
    // Both faces, so a leaf is not invisible from underneath a slope.
    for index in [0, 1, 2, 0, 2, 3, 0, 2, 1, 0, 3, 2] {
        into.indices.push(base + index);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leaves_lie_where_leaves_fell_and_nowhere_else() {
        // The rule the litter shares with the weather: the ground decides. Nothing
        // overhead on sand or rock or snow ever dropped a leaf on it.
        for season in Season::ALL {
            for through in [0.0, 0.5, 1.0] {
                for biome in [
                    Biome::Desert,
                    Biome::Snow,
                    Biome::Rock,
                    Biome::Water,
                    Biome::Shore,
                ] {
                    assert_eq!(
                        how_thick(biome, season, through),
                        0.0,
                        "leaves fell on {biome:?} in {}",
                        season.name()
                    );
                }
            }
        }
    }

    #[test]
    fn a_wood_is_deeper_in_leaves_than_the_grass_beside_it() {
        let wood = how_thick(Biome::Forest, Season::Autumn, 1.0);
        let field = how_thick(Biome::Grass, Season::Autumn, 1.0);
        assert!(wood > field, "a wood has {wood} and open grass {field}");
        assert!(field > 0.0, "no leaves at all blew onto the grass");
    }

    #[test]
    fn autumn_lays_it_down_winter_keeps_it_and_summer_is_clear() {
        let wood = |season, through| how_thick(Biome::Forest, season, through);

        assert_eq!(wood(Season::Summer, 0.0), 0.0, "summer is not clear");
        assert_eq!(wood(Season::Summer, 1.0), 0.0, "summer is not clear");
        assert_eq!(wood(Season::Autumn, 0.0), 0.0, "autumn began with a full floor");
        assert!(
            wood(Season::Autumn, 1.0) > 0.95,
            "autumn ended without covering the wood"
        );
        assert!(
            wood(Season::Winter, 1.0) < wood(Season::Winter, 0.0),
            "winter's litter never rots down"
        );
        assert!(
            wood(Season::Spring, 1.0) < 0.01,
            "spring ended with leaves still on the ground"
        );
    }

    #[test]
    fn it_comes_and_goes_without_a_step_in_it() {
        // Walked right round the wheel, including across every boundary. A carpet
        // of leaves that appears between one day and the next is the same fault as
        // a wood that changes colour overnight — and this test caught exactly that:
        // winter used to end at 45% and spring to begin at nothing.
        let mut worst: f32 = 0.0;
        let mut last = how_thick(Biome::Forest, Season::Spring, 0.0);
        for season in [
            Season::Spring,
            Season::Summer,
            Season::Autumn,
            Season::Winter,
            Season::Spring,
        ] {
            for step in 0..=200 {
                let now = how_thick(Biome::Forest, season, step as f32 / 200.0);
                worst = worst.max((now - last).abs());
                last = now;
            }
        }
        // A season is 28 days and this walks it in 200 steps, so a step is about
        // three hours. Nothing may move more than a hundredth in that.
        assert!(
            worst < 0.01,
            "the litter jumps {worst:.3} in about three hours — leaves appear overnight"
        );
    }

    #[test]
    fn a_leaf_is_two_triangles_lying_flat_and_lit_from_above() {
        let mut geometry = Geometry::default();
        put_a_leaf(&mut geometry, Vec3::new(1.0, 2.0, 3.0), 0.8, 0.1, [0.5, 0.3, 0.1]);

        assert_eq!(geometry.places.len(), 4, "a leaf is four corners");
        assert_eq!(geometry.indices.len(), 12, "a leaf is two triangles, both ways");
        for normal in &geometry.normals {
            assert_eq!(*normal, [0.0, 1.0, 0.0], "a leaf is lit off its own slope");
        }
        let heights: Vec<f32> = geometry.places.iter().map(|p| p[1]).collect();
        for height in &heights {
            assert!(
                (height - heights[0]).abs() < 1e-6,
                "a leaf is not lying flat: {heights:?}"
            );
        }
    }
}
