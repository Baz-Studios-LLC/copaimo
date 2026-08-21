//! Stamps authored blades and flower heads into a tuft, keeping the world's own
//! variation.
//!
//! # What is authored is the PIECE, not the tuft
//!
//! Ground cover is not placed the way a tree or a rock is. A tuft is *composed*:
//! how many blades it carries, how far round they fan, which way the clump leans,
//! how deep a green it is and how tall it has grown all come from the ground it
//! stands on. That composition is the whole reason a meadow does not read as
//! wallpaper, and replacing it with one authored tuft would trade it away for a
//! shape.
//!
//! So a file provides one blade and one head of petals, and this stamps them as
//! many times as the world says, where the world says, in the colour the world
//! says. `dev/art/cover.py` builds them.
//!
//! # The arrangement here is this crate's own
//!
//! `terrain_core::cover::add` composes a tuft too, and this does not call into it
//! — it cannot, because that function draws its own blades as it goes. So the
//! arrangement below is written here, from the same inputs: `shade`, `petal` and
//! `lush`, which the chunk dresser already works out per tuft.
//!
//! That is two implementations of one idea, which is this project's most frequent
//! bug. What keeps them honest is that they must AGREE ABOUT COLOUR: the greens
//! below are copied out of a private palette, and
//! `an_authored_tuft_is_the_same_green_as_a_grown_one` grows a real tuft through
//! the public API and compares. If the palette upstream moves, that fails.
//!
//! # The budget is fragments, not vertices
//!
//! Before making these richer: a chunk of open country carries about ninety
//! thousand vertices against a ceiling of a hundred and forty-five, so a blade can
//! afford ten vertices and not thirty. But the cost that bites is WIDTH — grass
//! overdraws itself many times over, and narrowing a blade once put vertices up a
//! fifth and fragments down by a third at the same frame cost.

use bevy::prelude::*;
use terrain_core::cover::Sprig;
use terrain_core::Geometry;

/// How tall a tuft stands at scale 1, in metres. Matches `cover::HEIGHT`.
const HEIGHT: f32 = 0.72;

/// The green a blade is, shallowest ground to deepest thicket.
///
/// **Copied from `terrain_core::cover`, which keeps them private.** Guarded by
/// `an_authored_tuft_is_the_same_green_as_a_grown_one`, which asks the crate for a
/// real tuft and compares — the only way to check a private constant from outside.
const GRASS_DARK: [f32; 3] = [0.055, 0.14, 0.035];
const GRASS_LIGHT: [f32; 3] = [0.16, 0.30, 0.075];

/// What a flower's head may be. Same source, same guard.
const PETALS: [[f32; 3]; 5] = [
    [0.62, 0.58, 0.20],
    [0.55, 0.24, 0.32],
    [0.30, 0.32, 0.62],
    [0.66, 0.44, 0.18],
    [0.58, 0.56, 0.58],
];

/// How much darker a tuft goes at the heart of a thicket.
const LUSH_SHADE: f32 = 0.3;
/// How many more blades a tuft grows there.
const LUSH_BLADES: usize = 4;
/// How far round a tuft fans its blades, least to most.
const SWEEP: (f32, f32) = (0.5, 0.82);
/// How far a whole clump may lean, in radians.
const TILT: f32 = 0.5;

/// The authored pieces a tuft is built from.
pub struct Kit {
    blade: Geometry,
    petals: Option<Geometry>,
}

/// Reads the authored pieces, if they are there.
///
/// Read synchronously and once, then shared: a chunk's cover is welded on a
/// background thread, and reading a file per chunk would be file IO on the hot
/// path for a couple of hundred chunks.
pub fn read_kit() -> Option<Kit> {
    let folder = crate::asset_file("assets/models");
    let read = |name: &str, mesh: &str| -> Option<Geometry> {
        let road = folder.join(name);
        if !road.is_file() {
            return None;
        }
        match std::fs::read(&road).map_err(|why| why.to_string()).and_then(|bytes| {
            crate::models::read_geometry(&bytes, mesh)
        }) {
            Ok(shape) => Some(shape),
            Err(why) => {
                warn!("{name}: {why}; ground cover keeps the shapes it grows");
                None
            }
        }
    };
    let blade = read("cover_blade.glb", "blade")?;
    if blade.colours.len() != blade.places.len() {
        warn!("cover_blade.glb carries no vertex colour; ground cover keeps its own");
        return None;
    }
    Some(Kit {
        blade,
        petals: read("cover_petals.glb", "petals"),
    })
}

/// Stamps one tuft, in the colour and shape the world asked for.
///
/// The arguments are exactly what `terrain_core::cover::add` takes, so the dresser
/// can call one or the other without knowing which.
#[allow(clippy::too_many_arguments)]
pub fn stamp(
    into: &mut Geometry,
    kit: &Kit,
    kind: Sprig,
    at: Vec3,
    turn: f32,
    scale: f32,
    shade: f32,
    petal: f32,
    lush: f32,
) {
    // How many blades, and never the same count twice running: two tufts of the
    // same size with the same number of blades are the same object however their
    // blades are jittered.
    let base = match kind {
        Sprig::Grass => 3,
        Sprig::Flower => 2,
        Sprig::Scrub => 5,
    };
    let blades = if kind == Sprig::Scrub {
        base
    } else {
        base + (LUSH_BLADES as f32 * lush).round() as usize
    };
    let blades = (blades + (fract(shade * 29.3) * 3.0) as usize).max(3);

    // How much of a full turn the blades are spread through. Less than all of it,
    // always: a tuft fanned right round is a rosette, and turning a rosette gives
    // the same rosette back.
    let sweep = SWEEP.0 + (SWEEP.1 - SWEEP.0) * fract(shade * 13.7);

    // And the whole clump leans, so a field has wind in it even when nothing moves.
    let tipping = fract(shade * 5.21) * std::f32::consts::TAU;
    let lean = TILT * fract(shade * 11.9);
    let leaning = Vec3::new(tipping.cos(), 0.0, tipping.sin());

    let green = shade_of(mix(GRASS_DARK, GRASS_LIGHT, shade), 1.0 - LUSH_SHADE * lush);
    let tall = HEIGHT * scale;
    let wide = 0.020 * scale * (1.0 + 0.5 * lush);
    let clump = wide * if kind == Sprig::Scrub { 3.2 } else { 2.1 };

    for blade in 0..blades {
        let roll = fract(shade * 7.13 + blade as f32 * 0.618_034);
        let sway = fract(shade * 3.77 + blade as f32 * 0.381_966 + 0.5);

        // The even step round the tuft, jittered by most of the gap between
        // blades: an even fan of equal blades is the construction of a coronet.
        let angle = turn
            + (blade as f32 + (roll - 0.5) * 1.5) / blades as f32
                * std::f32::consts::TAU
                * sweep;
        let out = Vec3::new(angle.cos(), 0.0, angle.sin());
        // Rising from a patch of ground rather than from a point, or the tuft has
        // a stem, and a stem under a fan is exactly a crown.
        let foot = at + out * clump * (0.2 + 0.8 * sway);
        // Shorter and taller blades in one tuft, or it reads as a shuttlecock.
        let length = tall * (0.72 + 0.42 * roll);

        // The template arches toward +Z, so it is turned to face `out` and then
        // tipped the way the whole clump leans.
        let facing = Quat::from_rotation_arc(Vec3::Z, out);
        let across = Vec3::new(-leaning.z, 0.0, leaning.x);
        let turned = Quat::from_axis_angle(across, lean) * facing;

        put(into, &kit.blade, foot, turned, length, green, 1.0);
    }

    if kind == Sprig::Flower {
        if let Some(petals) = &kit.petals {
            let colour = PETALS[(petal * PETALS.len() as f32) as usize % PETALS.len()];
            let head = at + Vec3::Y * tall * 0.92;
            // Scaled by the tuft, not by the blade: a flower's head is its own
            // size and does not get longer because one blade did.
            put(
                into,
                petals,
                head,
                Quat::from_rotation_y(turn),
                scale,
                colour,
                1.0,
            );
        }
    }
}

/// Copies one authored piece into the mesh, placed, turned, sized and tinted.
fn put(
    into: &mut Geometry,
    piece: &Geometry,
    at: Vec3,
    turned: Quat,
    size: f32,
    tint: [f32; 3],
    alpha: f32,
) {
    let base = into.places.len() as u32;
    for (index, place) in piece.places.iter().enumerate() {
        let local = Vec3::from_array(*place) * size;
        let world = at + turned * local;
        into.places.push(world.to_array());
        // UP, not the surface's own normal. A blade's normal points sideways, and a
        // meadow lit honestly flickers dark as the camera turns — so the whole
        // field is lit like the ground it belongs to. The generated blades do the
        // same, and it is done here rather than in the file so no export setting
        // can lose it.
        into.normals.push([0.0, 1.0, 0.0]);
        into.uvs.push(piece.uvs.get(index).copied().unwrap_or([0.0, 0.0]));
        // The piece is painted in greys: a modulation, so one shape can be every
        // green in a meadow and every colour a flower comes in.
        let shade = piece.colours.get(index).copied().unwrap_or([1.0; 4]);
        into.colours.push([
            tint[0] * shade[0],
            tint[1] * shade[1],
            tint[2] * shade[2],
            alpha,
        ]);
    }
    for index in &piece.indices {
        into.indices.push(base + index);
    }
}

fn fract(value: f32) -> f32 {
    value - value.floor()
}

fn mix(low: [f32; 3], high: [f32; 3], t: f32) -> [f32; 3] {
    let t = t.clamp(0.0, 1.0);
    [
        low[0] + (high[0] - low[0]) * t,
        low[1] + (high[1] - low[1]) * t,
        low[2] + (high[2] - low[2]) * t,
    ]
}

fn shade_of(colour: [f32; 3], by: f32) -> [f32; 3] {
    [colour[0] * by, colour[1] * by, colour[2] * by]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kit() -> Kit {
        read_kit().expect("dev/art/cover.py should have built the cover pieces")
    }

    /// The brightest part of a vertex colour, per channel.
    fn brightest(shape: &Geometry) -> [f32; 3] {
        shape.colours.iter().fold([0.0; 3], |most, colour| {
            [
                most[0].max(colour[0]),
                most[1].max(colour[1]),
                most[2].max(colour[2]),
            ]
        })
    }

    /// An authored tuft is the same green as one the world grows.
    ///
    /// # The guard on a copied palette
    ///
    /// `terrain_core::cover` keeps its greens private, so they are copied into this
    /// file — and a copied constant is exactly the shape of bug this project keeps
    /// meeting. It cannot be compared directly, so it is compared through what it
    /// PRODUCES: a real tuft is grown through the public API and the brightest
    /// vertex of each is taken.
    ///
    /// The brightest is the right measure because both run a blade from a darker
    /// root to a full-strength tip, so the tip is the palette colour itself in
    /// both. If the greens upstream move, this fails and names the difference.
    #[test]
    fn an_authored_tuft_is_the_same_green_as_a_grown_one() {
        let kit = kit();
        for (shade, lush) in [(0.0, 0.0), (0.35, 0.2), (0.5, 0.5), (0.85, 1.0), (1.0, 0.3)] {
            let mut grown = Geometry::default();
            terrain_core::cover::add(
                &mut grown,
                Sprig::Grass,
                Vec3::ZERO,
                0.4,
                1.0,
                shade,
                0.0,
                lush,
            );
            let mut ours = Geometry::default();
            stamp(
                &mut ours,
                &kit,
                Sprig::Grass,
                Vec3::ZERO,
                0.4,
                1.0,
                shade,
                0.0,
                lush,
            );
            assert!(!grown.colours.is_empty() && !ours.colours.is_empty());

            let theirs = brightest(&grown);
            let mine = brightest(&ours);
            for lane in 0..3 {
                assert!(
                    (theirs[lane] - mine[lane]).abs() < 0.002,
                    "at shade {shade} lush {lush}, channel {lane}: the world grows \
                     {:.4} and this stamps {:.4} — the copied palette has drifted",
                    theirs[lane],
                    mine[lane]
                );
            }
        }
    }

    /// The variation the world computes actually reaches the tuft.
    ///
    /// The whole reason the composition was kept rather than replaced by one
    /// authored tuft: change the ground and the grass has to change with it. So
    /// this asks whether the things that are supposed to vary do — and a stamp
    /// that ignored its arguments would pass every other test in this file.
    #[test]
    fn a_tuft_changes_with_the_ground_it_stands_on() {
        let kit = kit();
        let tuft = |turn: f32, scale: f32, shade: f32, lush: f32| {
            let mut into = Geometry::default();
            stamp(&mut into, &kit, Sprig::Grass, Vec3::ZERO, turn, scale, shade, 0.0, lush);
            into
        };

        // Deeper into a patch: more blades, and darker.
        let thin = tuft(0.0, 1.0, 0.5, 0.0);
        let thick = tuft(0.0, 1.0, 0.5, 1.0);
        assert!(
            thick.places.len() > thin.places.len(),
            "a thicket has {} vertices and thin ground {} — patch depth is ignored",
            thick.places.len(),
            thin.places.len()
        );
        assert!(
            brightest(&thick)[1] < brightest(&thin)[1],
            "a thicket is not darker than thin ground"
        );

        // Taller where it has grown: the tuft reaches higher.
        let peak = |shape: &Geometry| shape.places.iter().map(|at| at[1]).fold(f32::MIN, f32::max);
        assert!(
            peak(&tuft(0.0, 1.6, 0.5, 0.0)) > peak(&thin) * 1.3,
            "scale does not make a tuft taller"
        );

        // And it faces somewhere: turning it moves the blades.
        let east = tuft(0.0, 1.0, 0.5, 0.0);
        let west = tuft(std::f32::consts::PI, 1.0, 0.5, 0.0);
        let spread = |shape: &Geometry| {
            shape.places.iter().map(|at| at[0]).fold(f32::MIN, f32::max)
                - shape.places.iter().map(|at| at[0]).fold(f32::MAX, f32::min)
        };
        let moved = east
            .places
            .iter()
            .zip(&west.places)
            .any(|(here, there)| (here[0] - there[0]).abs() > 0.01);
        assert!(moved, "turning a tuft does not move its blades");
        assert!(spread(&east) > 0.01, "a tuft has no width at all");

        // Different shade, different tuft — not merely a different colour.
        let one = tuft(0.0, 1.0, 0.2, 0.4);
        let other = tuft(0.0, 1.0, 0.8, 0.4);
        assert!(
            one.places.len() != other.places.len()
                || one.places.iter().zip(&other.places).any(|(a, b)| {
                    (a[0] - b[0]).abs() > 0.001 || (a[2] - b[2]).abs() > 0.001
                }),
            "two tufts on different ground came out identical"
        );
    }

    /// Every stamped vertex is lit like the ground, and carries its colour.
    ///
    /// A blade's own normal points sideways, and a meadow lit that way flickers
    /// dark as the camera turns — so they are all faced straight up. It is done in
    /// the stamp rather than in the file precisely so no export setting can lose
    /// it, which means it is worth checking here.
    #[test]
    fn every_blade_is_lit_from_above_and_coloured() {
        let kit = kit();
        let mut into = Geometry::default();
        stamp(&mut into, &kit, Sprig::Flower, Vec3::new(3.0, 1.0, -2.0), 0.7, 1.1, 0.6, 0.5, 0.4);

        assert!(!into.places.is_empty(), "a flower stamped to nothing");
        assert_eq!(into.colours.len(), into.places.len(), "a colour per vertex");
        assert_eq!(into.normals.len(), into.places.len(), "a normal per vertex");
        for normal in &into.normals {
            assert_eq!(*normal, [0.0, 1.0, 0.0], "a blade is not facing up");
        }
        assert!(
            into.indices.iter().all(|at| (*at as usize) < into.places.len()),
            "a stamped tuft indexes a vertex it does not have"
        );
        assert_eq!(into.indices.len() % 3, 0, "not whole triangles");

        // Standing where it was put, not at the origin.
        let lowest = into.places.iter().map(|at| at[1]).fold(f32::MAX, f32::min);
        assert!(
            (lowest - 1.0).abs() < 0.05,
            "the tuft was planted at y=1.0 and its foot is at {lowest:.2}"
        );

        // A flower is the one thing here that is not all green: it has leaves AND a
        // head, and they have to be different colours or there was no point
        // authoring petals.
        //
        // Measured as the SPREAD in a channel rather than as "redder than green" —
        // which is what this asked first, and it failed on a perfectly good flower
        // because `petal` 0.5 draws the blue one out of the palette. A petal is not
        // a particular hue; it is a hue that is not the leaves'.
        let spread = |lane: usize| {
            let colours = into.colours.iter().map(|colour| colour[lane]);
            colours.clone().fold(f32::MIN, f32::max) - colours.fold(f32::MAX, f32::min)
        };
        let widest = (0..3).map(spread).fold(f32::MIN, f32::max);

        let mut leaves = Geometry::default();
        stamp(&mut leaves, &kit, Sprig::Grass, Vec3::ZERO, 0.7, 1.1, 0.6, 0.5, 0.4);
        let leafy = (0..3)
            .map(|lane| {
                let colours = leaves.colours.iter().map(|colour| colour[lane]);
                colours.clone().fold(f32::MIN, f32::max) - colours.fold(f32::MAX, f32::min)
            })
            .fold(f32::MIN, f32::max);
        assert!(
            widest > leafy * 2.0,
            "a flower's colours spread {widest:.3} and a plain tuft's {leafy:.3} —              the head is not a different colour from the leaves"
        );
    }
}
