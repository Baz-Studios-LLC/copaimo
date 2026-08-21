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
// Lengthened from 900 when the rim's wander was gentled. The old wander pushed the
// rim out by up to 29 m in places, and the massif's ENDS were relying on it: with a
// tamer rim the last thirty metres fell into the taper, and a straight crossing
// there climbed only 101 m against a gate that wants 102. Smoothing a wall shortens
// it, so the wall gets the length back.
const WALL_LONG: f32 = 940.0;
const WALL_THICK: f32 = 520.0;

/// How far the walls take to rise from the plain to the top, in metres.
///
/// Fifty-five metres of run for a hundred and seventy of rise is a seventy-degree
/// face: sheer to look at, unclimbable to walk, and still coarse enough that the
/// two-metre vertex grid draws it without stretching artefacts.
const WALL_RUN: f32 = 55.0;

/// Half the width of the canyon floor, in metres.
///
/// **Fifty-two metres wall to wall.** This is a THOROUGHFARE, not a crack: the
/// warden, a companion at their heel, and oncoming traffic all pass each other
/// with air to spare, and the follow camera swings behind the warden without
/// clipping rock. Twenty was a crack and thirty was still a corridor — both were
/// sized by how a slot canyon looks in a photograph rather than by what has to
/// walk down it.
const GAP_HALF: f32 = 26.0;

/// How far the canyon's walls take to reach full height, in metres.
///
/// Steeper than the outer walls on purpose — inside the slot the rock should
/// feel close overhead-tall, and a gentler flare would read as a valley.
const GAP_RUN: f32 = 34.0;

/// How far the rims wander from their drawn line, in metres, and over what
/// distance each wander plays out.
///
/// # A heightfield cannot hold a jagged cliff
///
/// These were 22 m over 90 and 7 m over 24, and the walls came out with a fine
/// COMB along their top and bottom edges. The arithmetic says why, and it is not
/// subtle: moving the rim sideways by a metre moves the ground up or down by the
/// wall's own gradient, which here is about four and a half metres per metre. So a
/// rim that wanders even half a metre between two vertices — and they are two
/// metres apart — steps the ground by two. The old fine octave wandered nearly two
/// metres per metre, which is seventeen metres of step between neighbours, and the
/// two octaves together measured forty-one.
///
/// Sheer, jagged, and a heightfield: pick two. The walls stay sheer, so the rim
/// line has to be gentle — a wander of A metres over L needs L greater than about
/// twenty-six times A to keep a step under a metre. What gives the canyon its
/// shape instead is the thing that was always doing the work: the way through
/// winds two hundred metres side to side, and it forks and spurs.
const JAG_BROAD: f32 = 7.0;
const JAG_BROAD_OVER: f32 = 380.0;
const JAG_FINE: f32 = 1.5;
const JAG_FINE_OVER: f32 = 130.0;

/// The same, for the edge of the slot itself: how far it wanders, over what.
///
/// Steeper than the outer walls — 170 m of fall over `GAP_RUN` — so it needs to
/// wander even more gently for the same smoothness.
const CHIP: f32 = 3.5;
const CHIP_OVER: f32 = 150.0;

/// The fork: a second true way through, leaving the main slot and rejoining it.
///
/// Real slot canyons braid, and a braid is the honest kind of "diverging path":
/// both ways go somewhere, the island between them is real rock, and a traveller
/// picks a side without a signpost. Narrower than the main way, so the main way
/// still reads as the main way.
const FORK_FROM: f32 = -150.0;
const FORK_TO: f32 = 120.0;
const FORK_SWING: f32 = 165.0;
const FORK_HALF: f32 = 20.0;

/// The spurs: box canyons that open off the way and pinch shut.
///
/// Dead ends are what make a junction a CHOICE — take the wrong turn and walk
/// back out — and they are alcoves the world can hide things in later. Each is
/// (where it leaves, its heading in the massif's frame, how far it runs); the
/// last stretch narrows to nothing, a headwall rather than a door.
const SPURS: [(f32, f32, f32); 2] = [(-40.0, 2.35, 130.0), (150.0, 1.3, 100.0)];
const SPUR_HALF: f32 = 15.0;

/// The (across, along) noise frame turned back into (along, across), which is the
/// order every centreline here thinks in.
fn frame_along_across(frame: Vec2) -> Vec2 {
    Vec2::new(frame.y, frame.x)
}

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

/// The fork's own centreline, where it is running: it lifts off the main way at
/// [`FORK_FROM`], bulges [`FORK_SWING`] metres aside, and lands back on it at
/// [`FORK_TO`] — a sine window, so both junctions are smooth.
fn fork(along: f32) -> Option<f32> {
    if !(FORK_FROM..=FORK_TO).contains(&along) {
        return None;
    }
    let t = (along - FORK_FROM) / (FORK_TO - FORK_FROM);
    Some(wander(along) + FORK_SWING * (std::f32::consts::PI * t).sin())
}

/// A point on the canyon's way through, in world space: where the main centreline
/// crosses `along` metres into the massif.
///
/// The tests' instrument — they walk it to prove the gate passes the floor and
/// refuses the walls. Nothing in the game routes itself yet; when something does
/// (a road, an NPC crossing east), this is what it should ask rather than
/// rediscovering the centreline.
#[cfg(test)]
pub fn way_through(along: f32) -> Vec2 {
    let (sin, cos) = HEADING.sin_cos();
    AT + Vec2::new(cos, sin) * along + Vec2::new(-sin, cos) * wander(along)
}

/// How the massif stands over this point: the rock it ADDS above the ground, how
/// deep inside the footprint the point is (0 at the rims to 1 well within), and how
/// far from any way through it is (0 on a canyon floor to 1 in solid rock).
///
/// One computation feeding [`lift`], [`shape`] and [`solid`], so the rock, the
/// causeway under it and the tests' idea of where the rock is can never disagree.
fn stands(at: Vec2) -> (f32, f32, f32) {
    let (along, across) = local(at);

    // Out past the footprint entirely: most of the world, answered cheaply.
    if along.abs() > WALL_THICK * 0.5 || across.abs() > WALL_LONG * 0.5 {
        return (0.0, 0.0, 0.0);
    }

    // The rim warp, in the massif's own frame so it turns with it. One broad
    // octave for buttresses, one fine octave to chip the edge.
    let frame = Vec2::new(across, along);
    let jag = (2.0 * terrain_core::forest::field(frame / JAG_BROAD_OVER, 83) - 1.0) * JAG_BROAD
        + (2.0 * terrain_core::forest::field(frame / JAG_FINE_OVER, 84) - 1.0) * JAG_FINE;

    // A mesa in both directions: flat inside, a sheer warped wall at the edge.
    let rim = |d: f32, half: f32| crate::util::smoothstep(half, half - WALL_RUN, d + jag);
    let mesa = rim(across.abs(), WALL_LONG * 0.5).min(rim(along.abs(), WALL_THICK * 0.5));
    if mesa <= 0.0 {
        return (0.0, 0.0, 0.0);
    }

    // The ways through and into the rock: the main slot, the fork, the spurs.
    // Rock stands only where every one of them says rock, so a junction is just
    // two answers agreeing that the ground is open.
    let chip = (2.0 * terrain_core::forest::field(frame / CHIP_OVER, 85) - 1.0) * CHIP;
    let open = |stray: f32, half: f32| {
        crate::util::smoothstep(half, half + GAP_RUN, stray + chip)
    };
    let mut slot = open((across - wander(along)).abs(), GAP_HALF);
    if let Some(centre) = fork(along) {
        slot = slot.min(open((across - centre).abs(), FORK_HALF));
    }
    for (from, heading, long) in SPURS {
        let start = Vec2::new(from, wander(from));
        let way = Vec2::new(heading.cos(), heading.sin());
        let gone = (frame_along_across(frame) - start).dot(way).clamp(0.0, long);
        let near = (frame_along_across(frame) - start - way * gone).length();
        // Pinches shut over its last stretch: a headwall, not a door.
        let width = SPUR_HALF * crate::util::smoothstep(long, long - 45.0, gone);
        slot = slot.min(open(near, width));
    }
    if slot <= 0.0 {
        return (0.0, mesa, 0.0);
    }

    // The top, flat with a metre or three of drift so it is stone, not glass.
    let crown = 0.97 + 0.05 * terrain_core::forest::field(frame / 130.0, 86);

    (TOP * mesa * slot * crown, mesa, slot)
}

/// What the massif adds to the ground here, in metres. Only ever ADDS.
///
/// The game itself goes through [`shape`]; this is the bare rock alone, and it is
/// the tests' own instrument.
#[cfg(test)]
fn lift(at: Vec2) -> f32 {
    stands(at).0
}

/// Whether this point is FULL rock: inside the mesa proper and clear of every way
/// through it.
///
/// The tests' instrument for sampling the TOP. Derived from the same numbers the
/// ground itself uses, because the alternative — a test naming coordinates it
/// believes are rock — goes stale the moment the canyon is widened, and then it
/// either fails for the wrong reason or passes while sampling a canyon floor.
#[cfg(test)]
fn solid(at: Vec2) -> bool {
    let (_, mesa, slot) = stands(at);
    mesa >= 0.999 && slot >= 0.999
}

/// How firmly the canyon country claims this ground for the DESERT, nought to one.
///
/// Wall to wall and slot floor included, the massif is desert rock: the green
/// world begins on the plain past the eastern mouth, not halfway down the canyon.
/// The claim fades to nothing over a skirt outside the footprint; `region` remaps
/// both sides so each lets go AT the handover line — the lesson every painted
/// boundary in this world already learned, applied to a generated one.
pub fn claim(at: Vec2) -> f32 {
    let (along, across) = local(at);
    let hold = |d: f32, half: f32| crate::util::smoothstep(half + 80.0, half - 40.0, d);
    hold(along.abs(), WALL_THICK * 0.5).min(hold(across.abs(), WALL_LONG * 0.5))
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
    let (rock, inside, _) = stands(at);
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
    ///
    /// Sampled wherever the massif is FULL rock — `solid` asks the ground's own
    /// numbers — rather than at coordinates this test believes are rock. Named
    /// coordinates go stale the moment the canyon is widened, and a stale sample
    /// set either fails for the wrong reason or passes while measuring a floor.
    #[test]
    fn the_top_is_flat_and_tall() {
        let (along, across) = axes();
        let mut low = f32::MAX;
        let mut high = f32::MIN;
        let mut looked = 0;
        for a in -9..=9 {
            for c in -18..=18 {
                let at = AT + along * (a as f32 * 22.0) + across * (c as f32 * 22.0);
                if !solid(at) {
                    continue;
                }
                looked += 1;
                let stands = lift(at);
                low = low.min(stands);
                high = high.max(stands);
            }
        }
        assert!(looked > 120, "only {looked} places on the top are solid rock");
        assert!(high > TOP * 0.95, "the top only reaches {high:.0} m");
        assert!(
            high - low < TOP * 0.12,
            "the top varies by {:.0} m over {looked} places — a ridge, not a mesa",
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
        // The NEGATIVE side: the fork and both spurs diverge toward positive
        // across, so this wall face is the plain slot wall everywhere.
        let wall = AT + along * l + across * (centre - (GAP_HALF + GAP_RUN + 14.0));
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
                // Scanned away from the fork and the spurs, so what varies is the
                // wall itself and not a junction.
                let at = AT + along * l + across * (centre - stray);
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

    /// How rough the walls are ALONG their own line, on the terrain's own grid.
    ///
    ///     cargo test how_toothed_the_walls_are -- --ignored --nocapture
    ///
    /// Reported as jaggedness at the top and bottom of the canyon walls. Across a
    /// wall the ground legitimately climbs three metres for every two of travel —
    /// that is what a seventy-degree wall IS. Along it, it should barely change, and
    /// that is where teeth show: each column of the heightfield crosses the rim at
    /// its own distance, so a rim that wanders quickly comes out as a comb.
    #[test]
    #[ignore = "a measurement"]
    fn how_toothed_the_walls_are() {
        let terrain = crate::world::terrain::Terrain::new();
        let (along, across) = axes();
        // The terrain's own vertex spacing: teeth finer than this cannot be drawn,
        // and teeth at exactly this are what is being looked for.
        let step = crate::config::CHUNK_SIZE / crate::config::CHUNK_QUADS as f32;

        for (name, band) in [("high on the wall", 0.72), ("low on the wall", 0.22)] {
            let mut worst = 0.0_f32;
            let mut worst_at = 0.0;
            let mut total = 0.0;
            let mut counted = 0;
            for lane in -3..=3 {
                let l = lane as f32 * 60.0;
                let centre = wander(l);
                // Out from the centreline until the wall reaches this share of TOP.
                let mut stray = GAP_HALF;
                while stray < 200.0 {
                    let at = AT + along * l + across * (centre - stray);
                    if lift(at) > TOP * band {
                        break;
                    }
                    stray += 1.0;
                }
                // Then walk ALONG the wall at that distance, on the grid.
                let mut last: Option<f32> = None;
                for tick in -40..=40 {
                    let onward = l + tick as f32 * step;
                    // FOLLOWING the canyon, not a straight line beside it. The
                    // centreline itself swings up to nearly three metres for every
                    // metre travelled, so a fixed offset walks ACROSS the wall — and
                    // a wall is meant to change height when you cross it. Measured
                    // that way, this reported forty metres of step and reported it
                    // just the same after the roughness had been taken out, which is
                    // how the mistake showed.
                    let at = AT + along * onward + across * (wander(onward) - stray);
                    let here = terrain.height(at.x, at.y);
                    if let Some(before) = last {
                        let jump: f32 = (here - before).abs();
                        total += jump;
                        counted += 1;
                        if jump > worst {
                            worst = jump;
                            worst_at = l;
                        }
                    }
                    last = Some(here);
                }
            }
            println!(
                "{name}: worst step {worst:.1} m (at {worst_at:.0} m along),                  average {:.2} m over {counted} samples",
                total / counted as f32
            );
        }
    }

    /// The floor is wide enough for a party AND oncoming traffic.
    ///
    /// The width is the whole reason this shape replaced a tunnel — a passage the
    /// warden can only edge through cannot hold the NPCs that are meant to use it,
    /// and the follow camera needs room behind them. Measured as the real walkable
    /// floor at stations along the way, both sides of the centreline.
    #[test]
    fn the_floor_is_wide_enough_for_traffic() {
        let (along, across) = axes();
        let mut narrowest = f32::MAX;
        let mut wherever = 0.0;
        for step in -60..=60 {
            let l = step as f32 * 4.0;
            let centre = wander(l);
            let reach = |dir: f32| {
                let mut out = 0.0;
                for probe in 1..200 {
                    let at = AT + along * l + across * (centre + dir * probe as f32);
                    if lift(at) > 2.0 {
                        break;
                    }
                    out = probe as f32;
                }
                out
            };
            let width = reach(-1.0) + reach(1.0);
            if width < narrowest {
                narrowest = width;
                wherever = l;
            }
        }
        assert!(
            narrowest > 34.0,
            "the floor pinches to {narrowest:.0} m at {wherever:.0} m along — \
             a party and oncoming traffic do not pass there"
        );
    }

    /// The way through DIVERGES: a fork rejoins around an island of true rock.
    #[test]
    fn the_fork_is_a_second_true_way_round_an_island() {
        let (along, across) = axes();
        let mid = (FORK_FROM + FORK_TO) * 0.5;
        let main = AT + along * mid + across * wander(mid);
        let branch = AT + along * mid + across * fork(mid).expect("the fork is running");
        let island = (main + branch) * 0.5;
        assert!(lift(main) < 3.0, "the main way is blocked beside the fork");
        assert!(lift(branch) < 3.0, "the fork is not open at its own middle");
        assert!(
            lift(island) > TOP * 0.5,
            "only {:.0} m of rock between the two ways — a wide spot, not a fork",
            lift(island)
        );
    }

    /// A spur is a box canyon: open to walk into, pinched shut at its head.
    #[test]
    fn a_spur_is_a_box_canyon_not_a_second_gate() {
        let (along, across) = axes();
        let world = |p: Vec2| AT + along * p.x + across * p.y;
        for (from, heading, long) in SPURS {
            let start = Vec2::new(from, wander(from));
            let way = Vec2::new(heading.cos(), heading.sin());
            let mouth = world(start + way * 30.0);
            let head = world(start + way * (long + 55.0));
            assert!(
                lift(mouth) < 3.0,
                "the spur at {from} m along is not open at its mouth"
            );
            assert!(
                lift(head) > TOP * 0.6,
                "the spur at {from} m along breaks through — a second gate, not a dead end"
            );
        }
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

    /// The whole canyon country is DESERT: the top, the walls, and the floor of
    /// the slot itself. The handover to the green world happens on the plain past
    /// the eastern mouth, not halfway down the canyon.
    #[test]
    fn the_canyon_country_is_all_desert() {
        use terrain_core::region::Country;
        let terrain = crate::world::terrain::Terrain::new();
        for l in [-200.0_f32, -80.0, 0.0, 90.0, 200.0] {
            let at = way_through(l);
            assert_eq!(
                terrain.region(at.x, at.y).0,
                Country::Desert,
                "the canyon floor {l:.0} m along is not desert"
            );
        }
        let (sin, cos) = HEADING.sin_cos();
        let across = Vec2::new(-sin, cos);
        for c in [-320.0_f32, 320.0] {
            let top = AT + across * c;
            assert_eq!(
                terrain.region(top.x, top.y).0,
                Country::Desert,
                "the mesa top {c:.0} m across is not desert"
            );
        }
    }

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

        // Wherever the massif is full rock — asked of `solid`, not of coordinates
        // this probe believes are rock. The named-coordinate version reported a
        // 22 m "mesa top" the moment the canyon was widened under it.
        let mut top = (f32::MAX, f32::MIN);
        let mut solid_places = 0;
        for c in -18..=18 {
            for l in -9..=9 {
                let at = AT + along * (l as f32 * 22.0) + across * (c as f32 * 22.0);
                if !solid(at) {
                    continue;
                }
                solid_places += 1;
                let h = terrain.height(at.x, at.y);
                top = (top.0.min(h), top.1.max(h));
            }
        }
        println!(
            "the mesa top stands {:.0}..{:.0} m absolute over {solid_places} solid places",
            top.0, top.1
        );
        let west = AT - along * 300.0;
        let east = AT + along * 300.0;
        println!(
            "the approaches: west {:.0} m, east {:.0} m",
            terrain.height(west.x, west.y),
            terrain.height(east.x, east.y)
        );

        // How wide the floor actually is, walked at stations along the way: out
        // from the centreline each way until the ground climbs two metres.
        let mut widths = Vec::new();
        for step in -8..=8 {
            let l = step as f32 * 30.0;
            let middle = way_through(l);
            let floor = terrain.height(middle.x, middle.y);
            let reach = |dir: f32| {
                let mut out = 0.0;
                for probe in 1..140 {
                    let at = middle + across * (dir * probe as f32);
                    if terrain.height(at.x, at.y) > floor + 2.0 {
                        break;
                    }
                    out = probe as f32;
                }
                out
            };
            widths.push(reach(-1.0) + reach(1.0));
        }
        let narrow = widths.iter().copied().fold(f32::MAX, f32::min);
        let wide = widths.iter().copied().fold(f32::MIN, f32::max);
        println!("the floor is {narrow:.0}..{wide:.0} m wide along the way");

        // And the junctions, so the branching is visible as numbers.
        for (name, l) in [("fork mouth", FORK_FROM), ("fork middle", (FORK_FROM + FORK_TO) * 0.5)] {
            let main = way_through(l);
            let branch = fork(l).map(|c| AT + along * l + across * c);
            match branch {
                Some(b) => println!(
                    "{name}: main floor {:.0} m, branch floor {:.0} m, {:.0} m apart",
                    terrain.height(main.x, main.y),
                    terrain.height(b.x, b.y),
                    main.distance(b)
                ),
                None => println!("{name}: the fork is not running here"),
            }
        }
    }
}
