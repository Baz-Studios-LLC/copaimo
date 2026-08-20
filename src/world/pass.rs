//! The canyon country that closes the road east.
//!
//! A flat-topped massif across the whole route, from the desert's edge to the green
//! country beyond it. There is no way over it and no short way round it, so getting
//! east means going THROUGH: one winding slot canyon, floored by the plain itself,
//! walled in sheer jagged rock, open to the sky the whole way.
//!
//! # Why a canyon, and not the tunnel it used to be
//!
//! The mountain shipped with a tunnel — first hard-coded, then bored by the maker's
//! own tool — and the tunnel fought the world's one load-bearing fact for a week: a
//! heightfield has exactly one height at every (x, z). Everything under the ground
//! needed a second mesh, a second walking rule, a second camera rule, a carve, a
//! hole cut out of the terrain's own skin, and a doorframe to make the hole
//! findable — and every one of those was real work that ended in "still not right".
//!
//! A canyon is the same GATE — you cannot pass until you find the way, and the way
//! bends so you cannot see through it — built entirely out of things a heightfield
//! is good at: walls go up, the floor stays down, the sky stays overhead. No second
//! ground, no holes, no doors.
//!
//! # The shape
//!
//! * the **top is flat** — a mesa, not a ridge. It reads as a landform you walk
//!   around or through, never over.
//! * the **walls are jagged**: the rim is warped by two octaves of noise in the
//!   massif's own frame, so the silhouette is crags and buttresses rather than a
//!   drawn line.
//! * the **canyon winds**: its centreline swings two hundred metres side to side on
//!   the way through, so no straight line crosses without climbing the full wall,
//!   and no sightline reaches the far country.

use bevy::prelude::*;

/// The middle of the massif, in metres.
///
/// Placed so its WESTERN foot lands on the desert's own eastern edge, at about
/// (180, -880) — the map printed by `dump_the_world` is what that was read off. So
/// the journey east is desert, then the canyon country, then the green world, then
/// the snow, and neither flank has the wrong country on it.
pub const AT: Vec2 = Vec2::new(456.0, -997.0);

/// Which way the massif's thickness runs, in radians about Y — the direction the
/// canyon carries a traveller. Nought is due east.
///
/// Set from the region's own lean rather than picked: the country boundary runs
/// along `(TILT, 1)` in map coordinates, which is `(0.39, 0.92)` on the ground once
/// each axis is scaled by its own extent, and the way through runs across that.
pub const HEADING: f32 = -0.40;

/// How high the top stands above the ground it is raised on, in metres.
///
/// Well over the treeline (150 m), so the walls strip to bare rock and the rim
/// reads as stone from the plain below. Not so high that the flat top becomes the
/// tallest thing in the world — the true mountains keep that.
const TOP: f32 = 170.0;

/// How far the massif reaches, in metres: the length of the WALL, measured across
/// the canyon's travel, and its THICKNESS, measured along it.
///
/// **Named for the wall rather than for the way through, and that is worth the
/// extra word.** They were `ALONG` and `ACROSS` once, which read naturally and
/// meant the opposite of what `local` returns — so the massif was built long in
/// the direction you travel and thin in the direction it was supposed to block.
const WALL_LONG: f32 = 900.0;
const WALL_THICK: f32 = 520.0;

/// How far the walls take to rise from the plain to the top, in metres.
///
/// Fifty-five metres of run for a hundred and seventy of rise is a seventy-degree
/// face: sheer to look at, unclimbable to walk, and still coarse enough that the
/// two-metre vertex grid draws it without stretching artefacts.
const WALL_RUN: f32 = 55.0;

/// Half the width of the canyon floor, in metres.
///
/// Twenty metres wall to wall: room for the follow camera behind the warden and
/// for two parties to pass, tight enough to read as a slot in the rock.
const GAP_HALF: f32 = 10.0;

/// How far the canyon's walls take to reach full height, in metres.
///
/// Steeper than the outer walls on purpose — inside the slot the rock should
/// feel close overhead-tall, and a gentler flare would read as a valley.
const GAP_RUN: f32 = 34.0;

/// How far the rims wander from their drawn line, in metres: the big warp that
/// makes buttresses, and the small one that chips the edges.
const JAG_BROAD: f32 = 22.0;
const JAG_FINE: f32 = 7.0;

/// Where a point sits in the massif's own frame: along its thickness — the way the
/// canyon runs — and across its length.
fn local(at: Vec2) -> (f32, f32) {
    let away = at - AT;
    let (sin, cos) = HEADING.sin_cos();
    (away.x * cos + away.y * sin, -away.x * sin + away.y * cos)
}

/// The canyon centreline: how far ACROSS the way through sits, at this point ALONG.
///
/// Two sines, a slow full S and a quicker wiggle on top of it. The swing is what
/// gates the crossing: any straight line through the massif leaves the slot
/// somewhere and meets full-height rock, so the only way east is to follow the
/// bends — and the bends also close every sightline to the far side.
fn wander(along: f32) -> f32 {
    120.0 * (along * 0.011 + 0.7).sin() + 40.0 * (along * 0.037 + 2.1).sin()
}

/// How the massif stands over this point: the rock it ADDS above the ground, and
/// how deep inside the footprint the point is, 0 at the rims to 1 well within.
///
/// One computation feeding both [`lift`] and [`shape`], so the rock and the
/// causeway under it can never disagree about where the massif is.
fn stands(at: Vec2) -> (f32, f32) {
    let (along, across) = local(at);

    // Out past the footprint entirely: most of the world, answered cheaply.
    if along.abs() > WALL_THICK * 0.5 || across.abs() > WALL_LONG * 0.5 {
        return (0.0, 0.0);
    }

    // The rim warp, in the massif's own frame so it turns with it. One broad
    // octave for buttresses, one fine octave to chip the edge.
    let frame = Vec2::new(across, along);
    let jag = (2.0 * terrain_core::forest::field(frame / 90.0, 83) - 1.0) * JAG_BROAD
        + (2.0 * terrain_core::forest::field(frame / 24.0, 84) - 1.0) * JAG_FINE;

    // A mesa in both directions: flat inside, a sheer warped wall at the edge.
    let rim = |d: f32, half: f32| crate::util::smoothstep(half, half - WALL_RUN, d + jag);
    let mesa = rim(across.abs(), WALL_LONG * 0.5).min(rim(along.abs(), WALL_THICK * 0.5));
    if mesa <= 0.0 {
        return (0.0, 0.0);
    }

    // The slot: nothing near the centreline, full rock past its own jagged walls.
    let stray = (across - wander(along)).abs();
    let chip = (2.0 * terrain_core::forest::field(frame / 50.0, 85) - 1.0) * 8.0;
    let slot = crate::util::smoothstep(GAP_HALF, GAP_HALF + GAP_RUN, stray + chip);
    if slot <= 0.0 {
        return (0.0, mesa);
    }

    // The top, flat with a metre or three of drift so it is stone, not glass.
    let crown = 0.97 + 0.05 * terrain_core::forest::field(frame / 130.0, 86);

    (TOP * mesa * slot * crown, mesa)
}

/// What the massif adds to the ground here, in metres. Only ever ADDS.
///
/// The game itself goes through [`shape`]; this is the bare rock alone, and it is
/// the tests' own instrument.
#[cfg(test)]
fn lift(at: Vec2) -> f32 {
    stands(at).0
}

/// The ground with the whole massif on it: the rock, standing on a floor that has
/// been GRADED through the slot.
///
/// The graded floor is the one thing here that is not pure addition, and it earns
/// its exception: the natural ground under the massif dips six metres below the
/// sea partway through, and a flooded slot is not a road — the warden would wade a
/// bend of it with their feet clamped to the tide. So inside the footprint the
/// floor is raised (never cut) toward a causeway running gently down from the
/// desert side to the green side, blended in over the mouths so each end meets the
/// plain it walks out onto. Ground already above the grade keeps its own shape.
pub fn shape(at: Vec2, ground: f32) -> f32 {
    let (rock, inside) = stands(at);
    if inside <= 0.0 {
        return ground + rock;
    }
    let (along, _) = local(at);
    // Down from the western approach (~24 m) to the eastern (~13 m).
    let grade = 24.0 + (13.0 - 24.0) * (along / WALL_THICK + 0.5).clamp(0.0, 1.0);
    let floored = ground + (grade - ground).max(0.0) * inside;
    floored + rock
}

#[cfg(test)]
mod tests {
    use super::*;

    fn axes() -> (Vec2, Vec2) {
        let (sin, cos) = HEADING.sin_cos();
        (Vec2::new(cos, sin), Vec2::new(-sin, cos))
    }

    /// The top is a MESA: flat, tall, and nothing pokes through or drops out.
    #[test]
    fn the_top_is_flat_and_tall() {
        let (along, across) = axes();
        let mut low = f32::MAX;
        let mut high = f32::MIN;
        for a in -8..=8 {
            for c in -14..=14 {
                let at = AT + along * (a as f32 * 24.0) + across * (c as f32 * 24.0);
                let (l, _) = local(at);
                // Inside the body, clear of the rims and clear of the canyon.
                let (l_abs, c_local) = (l.abs(), local(at).1);
                if l_abs > WALL_THICK * 0.5 - WALL_RUN - JAG_BROAD - 10.0 {
                    continue;
                }
                if c_local.abs() > WALL_LONG * 0.5 - WALL_RUN - JAG_BROAD - 10.0 {
                    continue;
                }
                if (c_local - wander(l)).abs() < GAP_HALF + GAP_RUN + 24.0 {
                    continue;
                }
                let stands = lift(at);
                low = low.min(stands);
                high = high.max(stands);
            }
        }
        assert!(high > TOP * 0.95, "the top only reaches {high:.0} m");
        assert!(
            high - low < TOP * 0.12,
            "the top varies by {:.0} m — that is a ridge, not a mesa",
            high - low
        );
    }

    /// No straight line crosses the massif without climbing most of the wall.
    ///
    /// This is the GATE. The canyon exists and its floor is the plain — but it
    /// wanders, so a straight crossing anywhere leaves the slot and meets rock.
    #[test]
    fn no_straight_line_crosses_without_climbing() {
        let (along, across) = axes();
        let mut weakest = f32::MAX;
        let mut where_weakest = 0.0;
        for c in -84..=84 {
            let aside = c as f32 * 5.0;
            let mut barrier = 0.0_f32;
            for step in -32..=32 {
                let at = AT + across * aside + along * (step as f32 * (WALL_THICK * 1.2 / 64.0));
                barrier = barrier.max(lift(at));
            }
            if barrier < weakest {
                weakest = barrier;
                where_weakest = aside;
            }
        }
        assert!(
            weakest > TOP * 0.6,
            "the straight crossing at {where_weakest:.0} m aside only climbs {weakest:.0} m"
        );
    }

    /// The canyon goes through at ground level, and it WINDS.
    #[test]
    fn the_canyon_winds_through_at_ground_level() {
        let (along, across) = axes();
        let mut tallest = 0.0_f32;
        let mut swing = (f32::MAX, f32::MIN);
        for step in -70..=70 {
            let l = step as f32 * (WALL_THICK * 0.7 / 70.0);
            let centre = wander(l);
            swing = (swing.0.min(centre), swing.1.max(centre));
            let at = AT + along * l + across * centre;
            tallest = tallest.max(lift(at));
        }
        assert!(
            tallest < 3.0,
            "the canyon floor stands {tallest:.1} m proud of the plain"
        );
        assert!(
            swing.1 - swing.0 > 200.0,
            "the canyon only swings {:.0} m — a corridor, not a winding slot",
            swing.1 - swing.0
        );
    }

    /// The canyon's walls are sheer, and they are CRAGS rather than drawn lines.
    #[test]
    fn the_walls_are_sheer_and_jagged() {
        let (along, across) = axes();

        // Sheer: from the centreline, full height arrives within the slot's own
        // run plus the chip the noise is allowed.
        let l = 40.0;
        let centre = wander(l);
        let foot = AT + along * l + across * centre;
        let wall = AT + along * l + across * (centre + GAP_HALF + GAP_RUN + 14.0);
        assert!(lift(foot) < 3.0, "the foot of the wall is not on the floor");
        assert!(
            lift(wall) > TOP * 0.6,
            "the wall only stands {:.0} m a stone's throw from the floor",
            lift(wall)
        );

        // Jagged: where the wall stands varies along the slot. For a run of
        // stations, find how far from the centreline the rock reaches half
        // height; a drawn line would put it in the same place every time.
        let mut nearest = f32::MAX;
        let mut furthest = f32::MIN;
        for step in -8..=8 {
            let l = step as f32 * 24.0;
            let centre = wander(l);
            let mut reach = GAP_HALF + GAP_RUN + 30.0;
            for off in 0..80 {
                let stray = GAP_HALF + off as f32;
                let at = AT + along * l + across * (centre + stray);
                if lift(at) > TOP * 0.5 {
                    reach = stray;
                    break;
                }
            }
            nearest = nearest.min(reach);
            furthest = furthest.max(reach);
        }
        assert!(
            furthest - nearest > 8.0,
            "the wall stands {nearest:.0}–{furthest:.0} m out — a drawn line, not crags"
        );
    }

    /// The floor through the slot is DRY GROUND, graded gently.
    ///
    /// The natural ground under the massif dips six metres below the sea partway
    /// through, and a flooded slot is not a road — the warden would wade a bend of
    /// it with their feet clamped to the tide. `shape` grades the floor up, and
    /// only ever UP: a hill already standing in the slot keeps its own shape.
    #[test]
    fn the_floor_is_a_dry_road_even_over_drowned_ground() {
        let (along, across) = axes();
        for step in -45..=45 {
            let l = step as f32 * 4.0;
            let at = AT + along * l + across * wander(l);
            let floored = shape(at, -6.0);
            assert!(
                floored > 10.0,
                "at {l:.0} m along, a drowned floor is only raised to {floored:.1} m"
            );
        }
        let at = AT + across * wander(0.0);
        assert!(
            (shape(at, 40.0) - 40.0).abs() < 0.01,
            "a hill standing in the slot was flattened"
        );
    }

    /// And on the REAL ground: the walk through is dry the whole way, with no
    /// step in it steeper than a walk.
    #[test]
    fn the_real_walk_through_is_dry_and_gentle() {
        let terrain = crate::world::terrain::Terrain::new();
        let (along, across) = axes();
        let mut last = None;
        for step in -55..=55 {
            let l = step as f32 * 4.0;
            let at = AT + along * l + across * wander(l);
            let here = terrain.height(at.x, at.y);
            assert!(here > 2.0, "the floor at {l:.0} m along is {here:.1} m — wet feet");
            if let Some(previous) = last {
                let rise: f32 = here - previous;
                assert!(
                    rise.abs() < 2.5,
                    "a {rise:.1} m step at {l:.0} m along — a wall in the road"
                );
            }
            last = Some(here);
        }
    }

    /// Going AROUND is the long way: the wall holds most of its length.
    #[test]
    fn the_wall_cannot_be_walked_round_without_going_a_long_way() {
        let (along, across) = axes();
        for side in [-1.0_f32, 1.0] {
            let mut holds_to = 0.0;
            for c in 0..90 {
                let aside = c as f32 * 5.0 * side;
                // A barrier still stands at this offset if SOME point along the
                // thickness is high — the crossing test's own question.
                let mut barrier = 0.0_f32;
                for step in -32..=32 {
                    let at =
                        AT + across * aside + along * (step as f32 * (WALL_THICK * 1.2 / 64.0));
                    barrier = barrier.max(lift(at));
                }
                if barrier > TOP * 0.5 {
                    holds_to = aside.abs();
                }
            }
            assert!(
                holds_to > WALL_LONG * 0.42,
                "the wall gives out {holds_to:.0} m to one side — a stroll around it"
            );
        }
    }
}

#[cfg(test)]
mod country {
    use super::*;

    /// The massif has to be the JOIN between the two countries, not a wall
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
        for step in -5..=5 {
            let down = across_way * step as f32 * (WALL_LONG * 0.06);
            // Just outside each end of the canyon: where a walker actually steps
            // out, and where "which country is this" has an answer that matters.
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

#[cfg(test)]
mod probe {
    use super::*;

    #[test]
    #[ignore = "a measurement of the real ground"]
    fn what_the_canyon_measures() {
        let terrain = crate::world::terrain::Terrain::new();
        let (sin, cos) = HEADING.sin_cos();
        let along = Vec2::new(cos, sin);
        let across = Vec2::new(-sin, cos);

        let mut floor = (f32::MAX, f32::MIN);
        let mut biggest_step = 0.0_f32;
        let mut last = None;
        for step in -80..=80 {
            let l = step as f32 * (WALL_THICK * 0.5 / 80.0);
            let at = AT + along * l + across * wander(l);
            let h = terrain.height(at.x, at.y);
            floor = (floor.0.min(h), floor.1.max(h));
            if let Some(previous) = last {
                biggest_step = biggest_step.max(h - previous).max(previous - h);
            }
            last = Some(h);
        }
        println!(
            "the canyon floor runs {:.0}..{:.0} m, worst step {biggest_step:.2} m per {:.1} m",
            floor.0,
            floor.1,
            WALL_THICK * 0.75 / 80.0
        );

        let mut top = (f32::MAX, f32::MIN);
        for c in [-300.0_f32, -180.0, 180.0, 300.0] {
            for l in [-140.0_f32, 0.0, 140.0] {
                let at = AT + along * l + across * c;
                if (c - wander(l)).abs() < GAP_HALF + GAP_RUN + 30.0 {
                    continue;
                }
                let h = terrain.height(at.x, at.y);
                top = (top.0.min(h), top.1.max(h));
            }
        }
        println!("the mesa top stands {:.0}..{:.0} m absolute", top.0, top.1);
        let west = AT - along * 300.0;
        let east = AT + along * 300.0;
        println!(
            "the approaches: west {:.0} m, east {:.0} m",
            terrain.height(west.x, west.y),
            terrain.height(east.x, east.y)
        );
    }
}
