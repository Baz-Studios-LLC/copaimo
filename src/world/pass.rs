//! The mountain that closes the road east.
//!
//! A wall of rock across the whole route, from the desert's edge to the green
//! country beyond it. There is no way over and no short way round, so getting east
//! means going through — and the way through is a tunnel the MAKER bores, with
//! [`crate::world::bores`] and the terrain tool's own BORE row.
//!
//! # It used to carry its own tunnel, and that was the wrong shape for the job
//!
//! The pass shipped with a hard-coded tunnel: a place, a heading, a thickness. Every
//! one of those was a number somebody had to guess at from a screenshot, and it went
//! wrong four times in an evening — the wall crossed the desert boundary instead of
//! following it, then it was a mesa, then it was too thin, then the tunnel rendered
//! as a stripe shaved over the mountain's shoulder. Exactly the fault the countries
//! had before they were paintable.
//!
//! So the tunnel is gone from here and the mountain stays. A mountain is landscape,
//! which the generator is good at; a doorway through it is a decision, which wants
//! eyes on the place. The bore tool is those eyes.
//!
//! # A mountain, not a very tall smooth hill
//!
//! Three things break up what would otherwise be a berm at any size, and each only
//! ever cuts DOWN, so anything bored through this reads the same mountain the ground
//! draws:
//!
//! * the **crest is serrated**, so the skyline is peaks and saddles rather than a
//!   ruler line — and never low enough anywhere to be walked over.
//! * the **flanks are creased** with gullies stretched down the fall line, because
//!   water runs downhill and isotropic folds came out as round pockets that read as
//!   hammered metal.
//! * the creases live **mid-flank only**: at the crest they would notch the skyline
//!   below the saddles, at the foot they would trench the plain.

use bevy::prelude::*;

/// The middle of the mountain, in metres.
///
/// Placed so its WESTERN foot lands on the desert's own eastern edge, at about
/// (180, -880) — the map printed by `dump_the_world` is what that was read off. So
/// the journey east is desert, then the west foot, then the mountain, then the
/// green country, then the snow, and neither flank has the wrong country on it.
pub const AT: Vec2 = Vec2::new(456.0, -997.0);

/// Which way the mountain's thickness runs, in radians about Y — the direction a
/// road through it would take. Nought is due east.
///
/// # It leans, because the country does
///
/// This was due east, and the wall it makes therefore ran due north-south — across
/// a desert boundary that runs on a diagonal, because `region`'s own axis is
/// tilted and the world is half as deep as it is wide. So the wall crossed the
/// boundary at an angle and one end of it had desert on the side that was supposed
/// to be green.
///
/// Set from the region's own lean rather than picked: the boundary runs along
/// `(TILT, 1)` in map coordinates, which is `(0.39, 0.92)` on the ground once each
/// axis is scaled by its own extent, and the tunnel runs across that.
pub const HEADING: f32 = -0.40;

/// How high the mountain stands above the ground it is raised on, in metres.
///
/// Well over the treeline, and by a margin: the trees give out at 150 m, so a
/// crest brushing 165 left all but the last few metres forested and the whole
/// thing read as a very long hill. At 235 the upper flanks strip to alpine rock
/// and the crest carries snow, which is what "mountain" looks like from below.
const RIDGE_HIGH: f32 = 235.0;

/// How far the mountain reaches, in metres: the length of the WALL, measured
/// across the tunnel, and its THICKNESS, measured along the tunnel.
///
/// The wall is long and the bore is short, which is the whole shape of a pass: a
/// barrier you cannot walk round and a way through you can walk in a couple of
/// minutes.
///
/// **Named for the wall rather than for the tunnel, and that is worth the extra
/// word.** They were `ALONG` and `ACROSS`, which read naturally and meant the
/// opposite of what `local` returns — so the mountain was built long in the
/// direction you travel and thin in the direction it was supposed to block. The
/// tests said so at once: the wall gave out 143 m to the side, and the plug was
/// still 156 m thick at its own edge.
const WALL_LONG: f32 = 900.0;
const WALL_THICK: f32 = 520.0;

/// How much of the wall's LENGTH is its shoulders rather than its body.
///
/// Only the length. Along its length a ridge really is flat-crested — that is what
/// makes it a barrier rather than a hill — and it eases down into the plain at
/// each end. Across its thickness it is not: a shoulder in both directions gives a
/// flat-topped table, which is what this first came out as and what the note above
/// this constant was already warning about. Across, the mountain simply peaks, so
/// there is a crest line running the length of the wall and the bore goes through
/// the tallest part of it.
const SHOULDER: f32 = 0.72;

/// How much of the mountain this point stands under, 0 to 1.
///
/// # A wall of rock, not a smooth earthwork
///
/// The analytic profile alone — two eased falloffs — is a berm at any size:
/// perfectly smooth flanks and a crest like a ruler. Three things break it up,
/// and each is scaled so the tests about the PASS still hold:
///
/// * the **crest is serrated**: the whole profile scales with a slow noise along
///   the wall, so the skyline is peaks and saddles rather than a line. It never
///   drops far enough to be walked over — the saddles are still most of the wall.
/// * the **flanks are creased**: two octaves of `1 - |noise|` gullies, cut into
///   the slope. Strongest mid-flank and faded at the crest and the foot, so the
///   silhouette stays a wall and the plain stays a plain.
/// * nothing is added ON TOP — creases only ever cut DOWN — so the bore's roof
///   arithmetic and every walk-through test read the same mountain this draws.
pub fn ridge(at: Vec2) -> f32 {
    let (along, across) = local(at);
    let reach = |d: f32, full: f32, flat: f32| {
        crate::util::smoothstep(full, full * flat, d.abs())
    };
    // Thin along the tunnel and long across it: rock to bore through, and a wall
    // reaching away on both sides of the mouth. Peaked in the first and
    // flat-crested in the second — see `SHOULDER`.
    let body = reach(along, WALL_THICK, 0.0) * reach(across, WALL_LONG, SHOULDER);
    if body <= 0.0 {
        return 0.0;
    }

    // The serration, in the wall's own frame so it survives being turned.
    let crest = 1.0 - SERRATION
        + SERRATION * 2.0 * terrain_core::forest::field(Vec2::new(across, along) / TOOTH, 78);

    // The creases, in the wall's own frame and STRETCHED down the fall line —
    // a gully is water's work and water runs downhill, so the folds are long in
    // the direction of the slope and narrow across it. Sampled isotropically
    // they came out as round pockets, and a hillside of round pockets reads as
    // hammered metal rather than as spurs.
    let fold = |narrow: f32, salt: u32| {
        let stretched = Vec2::new(across / narrow, along / (narrow * 3.2));
        1.0 - (2.0 * terrain_core::forest::field(stretched, salt) - 1.0).abs()
    };
    // Plus one broad UNstretched octave, or the combing is too even: every spur
    // the same width the whole length of a mountainside is a texture, not ground.
    let broad = 1.0 - (2.0 * terrain_core::forest::field(at / 150.0, 81) - 1.0).abs();
    let crease = 0.45 * fold(64.0, 79) + 0.3 * fold(27.0, 80) + 0.25 * broad;
    // Mid-flank only: at the crest a gully would notch the skyline below the
    // saddles, and at the foot it would trench the plain.
    let flank = (body * (1.0 - body) * 4.0).clamp(0.0, 1.0);
    let cut = RELIEF * flank * (1.0 - crease.powf(1.4));

    RIDGE_HIGH * body * crest * (1.0 - cut)
}

/// How deep the serration and the gullies go, as shares of the local height.
///
/// The serration swings the crest a fifth either way; the gullies take up to
/// two fifths out of the mid-flank. Between them the wall's LOWEST crossing
/// stays above half its nominal height, which the walk-over test measures.
const SERRATION: f32 = 0.2;
const RELIEF: f32 = 0.42;

/// Metres between teeth along the crest.
const TOOTH: f32 = 110.0;

/// Where a point sits in the mountain's own frame: along its thickness — the way a
/// road through it would run — and across its length.
fn local(at: Vec2) -> (f32, f32) {
    let away = at - AT;
    let (sin, cos) = HEADING.sin_cos();
    (away.x * cos + away.y * sin, -away.x * sin + away.y * cos)
}

/// What the mountain adds to the ground here, in metres.
///
/// The mountain, whole. Nothing is carved out of it here at all: a tunnel through it
/// is a bore, and a bore carves its own two mouths — see [`crate::world::bores`].
pub fn lift(at: Vec2) -> f32 {
    ridge(at)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mountain_blocks_every_way_over_it() {
        // What blocks a walker is the highest ground on their PATH, so each
        // candidate crossing is measured by the most it makes them climb — not by
        // every sample on a flank being high, which the gullies would fail.
        let (sin, cos) = HEADING.sin_cos();
        let along = Vec2::new(cos, sin);
        let across = Vec2::new(-sin, cos);

        let mut weakest = f32::MAX;
        let mut where_weakest = Vec2::ZERO;
        for side in [-1.0_f32, 1.0] {
            for out in 0..=((WALL_LONG * 0.7) as i32 / 10) {
                let aside = out as f32 * 10.0;
                let mut barrier = 0.0_f32;
                for step in -30..=30 {
                    let at = AT
                        + across * aside * side
                        + along * step as f32 * (WALL_THICK * 1.2 / 30.0);
                    barrier = barrier.max(lift(at));
                }
                if barrier < weakest {
                    weakest = barrier;
                    where_weakest = across * aside * side;
                }
            }
        }
        assert!(
            weakest > RIDGE_HIGH * 0.5,
            "the crossing at {where_weakest:?} only climbs {weakest:.0} m"
        );
    }

    #[test]
    fn the_wall_cannot_be_walked_round_without_going_a_long_way() {
        let (sin, cos) = HEADING.sin_cos();
        let across = Vec2::new(-sin, cos);
        for side in [-1.0_f32, 1.0] {
            let mut round = 0.0;
            for step in 0..1_400 {
                let out = AT + across * side * step as f32;
                if ridge(out) < 8.0 {
                    round = step as f32;
                    break;
                }
            }
            assert!(
                round > WALL_LONG * 0.9,
                "the wall gives out {round:.0} m along, which is a stroll around it"
            );
        }
    }

    #[test]
    fn the_mountain_is_broken_ground_and_not_a_smooth_berm() {
        // A crest like a ruler and flanks like a lawn is a berm at any size. Both
        // halves measured: the skyline varies along the wall, and the flanks are
        // creased across it.
        let (sin, cos) = HEADING.sin_cos();
        let along = Vec2::new(cos, sin);
        let across = Vec2::new(-sin, cos);

        // Walked along the crest, the height must vary.
        let crest: Vec<f32> = (-40..=40)
            .map(|step| ridge(AT + across * step as f32 * 12.0))
            .collect();
        let high = crest.iter().copied().fold(f32::MIN, f32::max);
        let low = crest.iter().copied().fold(f32::MAX, f32::min);
        assert!(
            high - low > RIDGE_HIGH * 0.1,
            "the crest varies by {:.0} m — a ruler, not a skyline",
            high - low
        );

        // And across the flank, gullies: neighbouring lines down the slope differ.
        let flank = |offset: f32| {
            (10..30)
                .map(|step| ridge(AT + along * step as f32 * 12.0 + across * offset))
                .collect::<Vec<f32>>()
        };
        let (a, b) = (flank(0.0), flank(45.0));
        let apart = a
            .iter()
            .zip(&b)
            .map(|(x, y)| (x - y).abs())
            .fold(0.0_f32, f32::max);
        assert!(
            apart > 12.0,
            "two lines down the flank differ by only {apart:.0} m — no gullies"
        );
    }
}

#[cfg(test)]
mod country {
    use super::*;

    /// The mountain has to be the JOIN between the two countries, not a wall
    /// standing across one of them.
    #[test]
    fn the_desert_meets_its_western_foot_and_the_green_world_its_eastern() {
        use terrain_core::region::Country;
        let terrain = crate::world::terrain::Terrain::new();
        let (sin, cos) = HEADING.sin_cos();
        let along_way = Vec2::new(cos, sin);
        let across_way = Vec2::new(-sin, cos);

        let mut desert = 0;
        let mut green = 0;
        let mut looked = 0;
        // Near the pass, and along the wall rather than out to its ends: the
        // wall is half a kilometre thick now, so probing its feet at the far
        // ends of a nine-hundred-metre wall lands in the sea and measures
        // nothing. What the claim is about is the ground either side of the
        // tunnel.
        for step in -5..=5 {
            let down = across_way * step as f32 * (WALL_LONG * 0.06);
            // Just outside each MOUTH rather than out past the mountain's feet:
            // where a walker actually steps out of the tunnel, and where the
            // question "which country is this" has an answer that matters. Out at
            // the feet of a half-kilometre wall the probes land in the sea.
            let west = AT + down - along_way * WALL_THICK * 0.9;
            let east = AT + down + along_way * WALL_THICK * 0.9;
            if terrain.height(west.x, west.y) < 1.0 || terrain.height(east.x, east.y) < 1.0 {
                continue;
            }
            looked += 1;
            desert += (terrain.region(west.x, west.y).0 == Country::Desert) as i32;
            green += (terrain.region(east.x, east.y).0 == Country::Ordinary) as i32;
        }

        assert!(looked >= 4, "only {looked} places along the wall have land both sides");
        assert!(
            desert * 3 >= looked * 2,
            "the western foot is desert at only {desert} of {looked} places along the wall"
        );
        assert!(
            green * 3 >= looked * 2,
            "the eastern foot is the green world at only {green} of {looked} places"
        );
    }
}
