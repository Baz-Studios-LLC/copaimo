//! The warden: a placeholder body and a ground-following character controller.
//!
//! The body is built from primitives at correct human scale (~1.8 m tall). That
//! matters more than it looks like it should — without something of a known
//! size standing on it, there is no way to judge whether a hill is a hill or a
//! mountain, or whether the map is too big to walk across. When real art lands
//! in `assets/models/`, `spawn_player` is the one place that changes.
//!
//! Movement is camera-relative (push forward and you go where you're looking)
//! and the warden is snapped to the terrain height each frame rather than
//! simulated with physics — the world has no colliders yet, and the heightfield
//! is an exact answer for "where is the ground".

use bevy::prelude::*;

use crate::camera::{CameraMode, MainCamera};
use crate::config::{RANCH_AT, SEA_LEVEL};
use crate::states::AppState;
use crate::util::facing_quat;
use crate::world::terrain::TerrainSource;
use crate::world::WorldBounds;

// # EVERY NUMBER BELOW WAS MEASURED OFF A CHARACTER THAT NO LONGER EXISTS
//
// The ranger's mesh, rig and clips were deleted on 2026-08-24 to be replaced from new source
// files. `covers`, the frame counts, the leg-length correction and the speed tiers are all
// facts about THAT character's animation, and on a different one they are somebody else's
// stride. They are kept because they show the SHAPE of what has to be re-derived - which
// quantities matter and how they relate - not because any of them is still true.
//
// Re-measure before trusting one. The clip tests read the model file and `.expect()` it, so
// they panic rather than quietly passing; that is deliberate.
/// MEASURED 2026-08-23: the hip socket sits at 85.2 cm on a 170.2 cm figure, which is
/// 50.1% - dead in the adult human 50-52% band. The '45%' below is WRONG and it misled
/// hours of tuning, because it made every reach limit look anatomical and therefore
/// unfixable. The real limit is a constant: the bind stands at 99.7% leg extension
/// (hip-to-ankle 78.1 cm on a 78.35 cm leg), so ik_gait.STANCE_LEG_EXTENDS = 0.98 caps
/// usable reach BELOW what standing upright needs, and the crouch that follows is what
/// eats the stride. The genuine oddity is that he is 5.7 heads tall against 7.5
/// realistic - a large head, not short legs.
///
/// A deliberate slow walk, in metres a second. Held on Ctrl.
///
/// # Seven was not a walk, it was a world record
///
/// This was 7.0 and the sprint 15.0, on a figure 1.7 m tall. Reported simply as
/// "movement is extremely fast", which it was by a factor of five. It also meant the
/// WALK CLIP NEVER PLAYED, because the run threshold sat BELOW the walking speed:
/// every step the warden had ever taken ran the run clip at five cycles a second, so
/// every judgement anyone had made about how the gaits looked was a judgement about
/// the run played at five times its cadence. **When a whole family of opinions is
/// wrong at once, check whether they were all formed about the same mistake.**
///
/// # And then it went too slow, for a reason worth writing down
///
/// The correction put the DEFAULT pace at a walk, which is not what the games this one
/// is measured against do. In Palworld, Skyrim, WoW and Unity's own third-person
/// template the default is a JOG and walking is a deliberate slow mode that keyboard
/// players almost never choose. "Movement is still too slow" was not asking for a
/// faster walk; it was asking to stop walking everywhere.
///
/// The physics agreed. At 2.25 m/s a 1.7 m person has a Froude number of 0.57
/// (v²/gL, leg 0.90 m), and the walk-to-run transition sits at Fr ≈ 0.5 — so a real
/// human at the old "walk" speed would already have broken into a run.
///
/// The walk CLIP natively carries 1.07 m/s, played at 1.00x - nothing stretches.
/// Measured on the clip delivered 2026-08-24: 2.542 m of travel over 2.375 s, which is
/// the one speed at which its feet do not slide.
///
/// That is a deliberate amble rather than a brisk walk, and it is the price of
/// keeping him upright. A planted foot pins the hip to sqrt(reach^2 - ahead^2) above the
/// ankle, so every extra centimetre of stride is paid for in crouch, and the stride is
/// whatever fits under `ik_gait.HIP_DROPS_AT_MOST`.
///
/// This used to add "his legs are 45% of his height where a person's are 52%", and to quote
/// that cap as 6 cm. Both are wrong now: the cap is 0.024 model units, and the 45% was a
/// MISMEASUREMENT - a thigh-plus-calf bone chain against the hip-to-floor landmark humans are
/// quoted on. Measured on the same landmark for both, this leg is 50.1% and entirely ordinary.
/// Walking is the deliberate slow mode here anyway; the default pace is the jog.
// 1.42 for the 2026-08-26 warden: the speed his own walk is ANIMATED at, so the clip plays
// at 1.00x. Driving 1.07 under it played the clip at 0.75x, and the jog's version of the same
// mismatch was reported as "that same running through water thing" - the legs churn slower
// than a body plainly meant to move faster.
pub const WALK_SPEED: f32 = 1.42;

/// The default pace, in metres a second. A jog, and the speed the game is actually played at.
///
/// 5.90, which is what SPRINT_SPEED used to be. The sprint's pace became the default because
/// the sprint is not going to be a constantly available thing, so the speed the player spends
/// their time at should be the one that felt right to move at.
///
/// A KNOB, chosen by feel. Worth stating plainly, because this comment used to argue the
/// opposite at length - "the jog is AT its ceiling", "it cannot be faster without either
/// skating or leg-blur" - and every prop under that argument has since gone:
///
/// * the cadence band stopped gating speed. See `motion::halfway`: once selection read INTENT
///   rather than a measured velocity, each tier carries exactly one speed and a ceiling bounds
///   nothing. Pinning the driven speed just under a human cadence limit is what made the jog
///   feel slow in the first place - the speed was never a choice, it was whatever a band allowed.
/// * the clip does not cover 1.541 m. Measured properly by `dev/art/measure_covers.py` it
///   carries 2.496, and the old figure being 28% short is what read as running through water.
/// * the leg was not the constraint. "Reach on a 45%-of-height leg" was a MISMEASUREMENT: a
///   thigh-plus-calf bone chain compared against the hip-to-floor landmark human figures are
///   quoted on. On the same landmark for both, this leg is 50.1% - ordinary.
///
/// Comparable figures: Palworld's default is 3.50, Unity's third-person sprint 5.34, Epic's own
/// authored run 5.00. This sits above all three deliberately - the world is large and the
/// touchstone for movement is Genshin, not a person.
///
/// What it costs is CADENCE, and that cost is real rather than theoretical: 5.90 against a clip
/// carrying 2.496 m turns the legs over about 284 steps a minute. The answer is not this number
/// and not a longer authored stride - it is stride warping, which buys speed from stride LENGTH
/// instead of tempo. See `docs/animation.md`.
// Re-measured 2026-08-24 against the delivered run: 4.964 m per cycle over 1.0333 s is
// 4.80 m/s, and that is the speed at which the clip plays at 1.00x with nothing stretched and
// nothing sliding. It was 5.90, chosen for a clip that no longer exists, and asking this one for
// 5.90 is a 23% stride stretch that lands just under the jog cadence band.
//
// This is SLOWER than before by about a fifth. Stride warping can buy it back - `STRIDE_WARPS_TO`
// allows 1.25x, so this clip can serve up to 6.0 m/s - but that is a feel decision rather than a
// measurement, and the measured value is the honest default.
// SLOWED 2026-08-25, from 4.80. "For a jog the limbs dont need to move as quickly as the run so
// we can slow down the movements too."
//
// With the phase driven by distance, this IS the limb speed: cadence is `speed / covers`, so the
// only way to calm the legs without re-authoring the clip is to move slower. What each choice
// costs, against the clip's own native 4.43 m/s:
//
//     speed   playback   cadence   effective cycle
//     4.80      1.08x       130       23.1 frames     was
//     4.00      0.90x       108       27.7 frames     now
//     3.55      0.80x        96       31.2 frames     the floor
//     3.20      0.72x        87       34.6 frames     REFUSED, the feet skate
//
// The floor is not arbitrary: under 0.80x the legs churn slower than the ground goes by and the
// feet slide, which `neither_gait_plays_at_a_blur` refuses. It sits as high as it does because
// the clip's stride is 4.435 m a cycle - **2.6 times his own height**, where a jog is about
// 1.4 to 1.8 - so every metre of ground costs very few steps. Slowing further than this needs a
// shorter stride, which is the clip's to give, not this constant's.
// 2.90 m/s = 10.4 km/h, against an authored stride of 2.506 m a cycle.
//
// The three numbers that never agreed before now do:
//
//     stride    1.47 x his own height   (a jog is 1.4-1.8)
//     cadence   139 steps a minute      (recreational running is 150-170)
//     playback  1.16x                   (the guard allows 0.80-1.25)
//
// It was 4.00 against the DELIVERED clip's 2.6x-height stride, which forced a 1.60x playback -
// outside the guard - and 192 steps a minute. Before that 4.80, at 130 steps a minute and 14.4
// km/h, which is a sprint's speed at a walk's cadence. Neither was a jog; both were a number
// chosen by feel against a stride that could not support it.
//
// Still a KNOB - see the note below about the cadence bands not gating speed - but a knob with
// a stride behind it now.
// 4.77 for the 2026-08-26 warden, and it is not a knob any more: it is the speed the
// delivered run is ANIMATED at - JOG_COVERS 2.982 m over its 0.625 s cycle - so the clip plays
// at exactly 1.00x and distance matching agrees with the animator instead of stretching him.
// 2.90 under this clip meant 0.61x playback: "running through water", reported twice.
//
// 5.35 now, asked for directly: "increase the running speed a bit". That is 1.12x on the
// clip rather than 1.00x, so the cadence runs a little ahead of the stride the animator
// delivered. Twelve per cent is small enough to read as a quicker warden rather than as a
// sped-up video, and this is a KNOB: how the game feels is the thing being tuned, and a
// clip is an input to that rather than a limit on it. If it ever reads as skating, the
// answer is a faster clip, not a slower warden.
pub const JOG_SPEED: f32 = 5.35;

/// A sprint, in metres a second. Held on Shift.
///
/// 7.40 - a quarter again on top of the jog, and deliberately a modest gap. The sprint is
/// meant to become a limited resource rather than a speed held down forever, so its job is to
/// feel like a burst on top of a pace that is already quick. A large gap would make the jog
/// feel like the slow option again, which is the fault this whole change is undoing.
///
/// Two claims that used to live here and are no longer true:
///
/// "EXACTLY the sprint clip's native 4.46 m/s, so Shift plays it at 1.00x, nothing stretches,
/// nothing skates." The clip carries 3.283 m over 1.042 s, so its native pace is 3.15 m/s, and
/// nothing but the walk is played at 1.00x. `animate_ranger::report_the_native_speeds` prints
/// the real table on every asset build rather than it being restated in prose.
///
/// "If Shift needs to be faster one day, the clip must cover more ground first, and this
/// constant then follows the new measurement, never the other way round." That was right while
/// the only ways to buy speed were a longer clip or a faster playback rate. There is a third,
/// and it is the standard one: stride warping. Speed comes from stride LENGTH at runtime, with
/// IK behind the feet and the hips dropping to pay for the reach.
///
/// The measurement against lengthening the AUTHORED stride still stands and is still the reason
/// not to do that: SPRINT_CONTACT at 1.5x and 1.8x of the run's bought 2.736 and 2.836 m a
/// cycle, but foot slide went from 0.106 to 0.178 and 0.247 - the extra sweep is past what the
/// leg can reach, so the floor solve drags the foot to cover it. That is an argument about a
/// stride baked into a clip, not about one warped at runtime with a hip drop underneath it.
// SPRINT_SPEED was here, at 6.00 m/s, with a note that "no sprint clip was delivered, so
// nothing above this has an animation behind it". That stayed true, and the tier is gone rather
// than left carrying a speed nothing could animate. The game is walk and jog.

/// How fast the warden swivels to face the way they're heading, in radians/sec.
const TURN_RATE: f32 = 12.0;

/// How quickly the MEASURED speed settles onto what was just measured, per second.
///
/// Only the playback rate reads it, and a rate that jumps around reads as a stutter, so
/// this exists to take the spikes out without adding lag a player would feel: at 16 a
/// step change is most of the way there inside a tenth of a second. It deliberately does
/// NOT smooth what the warden actually asks for - input stays instant, which is the whole
/// point of the intent/measurement split in `Striding`.
const SPEED_SETTLES: f32 = 16.0;

/// The least of the asked speed a blocked warden's clip still plays at - see the note
/// where it is used.
const BLOCKED_STILL_RUNS: f32 = 0.2;
/// Standing eye-to-toe height, used to keep the body clear of the ground.
/// The steepest rise the warden can WALK up, in metres climbed per metre
/// travelled. One-in-one is a 45° scramble and still walking; this is a little
/// past it.
///
/// Terrain is this game's only geometry, so this rule is what makes a wall a
/// WALL: the canyon country's faces rise three-to-five metres per metre, and
/// without a refusal here the warden strolls up them and the canyon gates
/// nothing. Only the step UP is refused — any slope can be walked back down —
/// so nowhere is a trap.
pub const CLIMB_LIMIT: f32 = 1.4;

/// How high a STEP the warden can take, in metres, however sheer it is.
///
/// # A step and a slope are different things, and one rule was doing both
///
/// `CLIMB_LIMIT` is a gradient, and a gradient cannot describe a kerb: a kerb is
/// 22 cm of vertical, which is infinite metres climbed per metre travelled, so the
/// climb rule refuses it outright. The only way to get a kerb past a pure gradient
/// rule is to lean its face back until it is a ramp - and a ramp is what it looked
/// like. Reported as "not a real curb, looks more like it just rained".
///
/// So the two are separated. A gradient still governs ground you WALK up, and this
/// governs ground you STEP onto: a kerb, a doorstep, a low ledge. Every character
/// controller worth the name has both, and this one had been making its kerbs
/// climbable by flattening them.
///
/// Sized against what must still be refused rather than what should be allowed. A
/// canyon wall rises three to five metres per metre and the warden covers about a
/// tenth of a metre in a frame, so its smallest single-step rise is around 0.3 m -
/// and `a_canyon_wall_refuses_the_step_up_but_never_the_step_down` is what actually
/// decides this number, not the arithmetic in this paragraph.
pub const STEP_UP: f32 = 0.26;

/// How deep the warden may wade, in metres below sea level.
///
/// The sea is not walkable in the base game — it is for boats. This is both how
/// far they can stand into it and how far they can *walk* into it: one number,
/// so the depth they are held at and the depth they are turned back at can never
/// disagree and leave them bobbing at a line they cannot cross.
const WADE_DEPTH: f32 = 1.4;

/// How wide the warden is, in metres, for the purpose of walking into things.
///
/// A third of a metre. It is a shoulder half-width and not a hitbox: the only
/// question it answers is how close he can stand to a trunk before the trunk stops
/// him, and a body that stops a third of a metre out reads as leaning on the tree
/// rather than as clipping into it.
pub const WARDEN_IS_WIDE: f32 = 0.33;

/// How far ahead the landing at the top of a step is looked for, in metres.
///
/// # A frame is not a distance
///
/// The step allowance was written as `rise <= STEP_UP`, with the rise measured
/// across ONE FRAME'S movement. That reads as a rule about kerbs and it is really a
/// rule about frame rate: a jog covers about 9 cm in a 60 Hz frame, so a 26 cm
/// allowance admits a slope of nearly 3:1 against a `CLIMB_LIMIT` of 1.4 - and about
/// 11:1 at 240 Hz, where a frame is 2.3 cm. The canyon walls are climbable on a fast
/// machine and not on a slow one. Codex found it by reading the units; the canyon
/// test could not, because it takes a single 1.5 m sample rather than a real stride.
///
/// So the question is asked over a distance the frame cannot change. A step is a
/// rise with somewhere to STAND on the other side of it, and 0.6 m is about one
/// stride: far enough past a kerb or a doorstep to be on top of it, short enough
/// that a staircase is still measured as the slope it is.
const STEP_LANDS: f32 = 0.6;

/// How far to look for things to walk into, in metres.
///
/// A step is small and a trunk is not wide, so the box only has to cover the step
/// plus the widest thing that could reach into it. Kept tight because this runs
/// every frame: a wide box asks about far more of the wood than a step can reach.
const LOOKS_AHEAD: f32 = 4.0;

/// Whether one step of a walk is allowed: not into deep water, not up a cliff.
///
/// The sea is for boats. Rather than an invisible wall at the waterline — which
/// reads as a bug, and stops you paddling at a beach at all — the warden wades
/// until the water is about knee-to-waist and is then turned back by it. Only the
/// step INTO deeper water is refused, so someone who somehow ends up out there
/// can always walk home. The cliff rule is the same shape: only the step UP is
/// refused, so no slope is a trap.
fn may_step(
    terrain: &crate::world::terrain::Terrain,
    built: &crate::world::town::Built,
    standing: &[Trunk],
    walls: &[(Vec2, Vec2, f32)],
    from: Vec3,
    to: Vec3,
) -> bool {
    // WALKING height, not ground height. They are the same everywhere except on a
    // bridge, where the deck is the surface underfoot - and asking the ground there
    // reports the lake bed, which is under the wade limit, so every step onto a
    // bridge was refused as a step into deep water.
    // And what has been BUILT over it: a road's crown, a doorstep, the boards
    // inside. The cliff rule reads these too, so a doorstep is a ramp it allows
    // rather than a lip it refuses.
    let here = crate::world::town::stands_on(terrain, built, Vec2::new(from.x, from.z));
    let there = crate::world::town::stands_on(terrain, built, Vec2::new(to.x, to.z));

    let depth = SEA_LEVEL - there;
    if depth > WADE_DEPTH && depth >= SEA_LEVEL - here {
        return false;
    }

    if into_a_trunk(standing, from, to) || into_a_wall(walls, from, to) {
        return false;
    }

    let step = Vec2::new(to.x - from.x, to.z - from.z);
    let run = step.length();
    if run <= f32::EPSILON {
        return true;
    }

    // THE GROUND OVER A STRIDE, not over this frame. See `STEP_LANDS` - measuring
    // either rule across one frame's movement makes both of them frame-rate rules.
    let ahead = Vec2::new(from.x, from.z) + step / run * STEP_LANDS;
    let climb = crate::world::town::stands_on(terrain, built, ahead) - here;

    // Walkable as a SLOPE - the whole stride rises no faster than a warden climbs -
    // or short enough to be a STEP, which is a rise with a landing behind it and so
    // is still within `STEP_UP` a stride later. A kerb is 22 cm and then level; a
    // canyon wall is 1.8 m and then more canyon wall.
    climb <= STEP_LANDS * CLIMB_LIMIT || climb <= STEP_UP
}

/// One thing standing in the world that a warden cannot walk through.
#[derive(Clone, Copy, Debug)]
pub struct Trunk {
    pub at: Vec2,
    pub radius: f32,
}

/// Whether this step walks into something standing.
///
/// # Only the step IN is refused
///
/// The same shape as the cliff rule and the wading rule above, and for the same
/// reason: a test that refuses every position inside a trunk would trap anybody who
/// somehow ended up in one — spawned there, put there by a brush, or standing where
/// a tree was planted afterwards. Refusing only the step that makes it WORSE means
/// there is always a way out of anywhere.
fn into_a_trunk(standing: &[Trunk], from: Vec3, to: Vec3) -> bool {
    let was = Vec2::new(from.x, from.z);
    let goes = Vec2::new(to.x, to.z);
    standing.iter().any(|trunk| {
        let keep = trunk.radius + WARDEN_IS_WIDE;
        let after = goes.distance(trunk.at);
        after < keep && after < was.distance(trunk.at)
    })
}

/// Everything near enough to a step to be walked into.
///
/// Asked ONCE per frame and handed to all three candidate steps, rather than each
/// asking for itself: both queries walk a lattice and the tree one takes the
/// painted forest's lock, and doing that three times a frame for the same answer is
/// waste.
///
/// Trees and litter come from the same two functions the RENDERER uses to decide
/// where to draw them — `trees_in` and `litter_in` — so there is no second opinion
/// anywhere about where the world's furniture stands.
fn standing_near(
    terrain: &crate::world::terrain::Terrain,
    grove: Option<&crate::world::stream::Grove>,
    props: Option<&crate::world::prop::PropPool>,
    at: Vec3,
    standing: &mut Vec<Trunk>,
) {
    let here = Vec2::new(at.x, at.z);
    let low = here - Vec2::splat(LOOKS_AHEAD);
    let high = here + Vec2::splat(LOOKS_AHEAD);
    standing.clear();

    if let Some(grove) = grove {
        standing.extend(terrain.trees_in(low, high).into_iter().filter_map(|tree| {
            let variety = grove.trees.get(tree.variety)?;
            let radius = variety.trunk * tree.scale;
            // A sapling is not a wall. Below a hand's width the trunk is thinner
            // than the tolerance either side of it, and stopping a warden dead on
            // something he could snap reads as an invisible post.
            (radius > THIN_ENOUGH_TO_PASS).then_some(Trunk {
                at: Vec2::new(tree.at.x, tree.at.z),
                radius,
            })
        }));
    }

    if let Some(props) = props {
        standing.extend(
            crate::world::prop::litter_in(terrain, &props.0, low, high)
                .into_iter()
                .filter(|strewn| crate::world::prop::is_solid(strewn.kind))
                .filter_map(|strewn| {
                    // `reach` is how far the thing extends from its middle, which
                    // for a log is its LENGTH — and a log is not a circle. Taken at
                    // three quarters, so a warden brushes the ends of a long one
                    // instead of being held off at arm's length from its middle.
                    let radius = strewn.reach * PROPS_ARE_ROUNDER_THAN_THEY_REACH;
                    (radius > THIN_ENOUGH_TO_PASS).then_some(Trunk {
                        at: strewn.at,
                        radius,
                    })
                }),
        );
    }
}

/// Whether a step walks into one of a town's walls.
///
/// Rectangles rather than the circles trees and props use, and the difference is
/// the doorway: a building's front wall is given as two piers with a gap between
/// them, and a gap only exists if the thing either side of it has square ends.
///
/// Only the step IN is refused, the same as everywhere else here, so a warden who
/// finds himself inside a wall can always walk back out of it.
fn into_a_wall(walls: &[(Vec2, Vec2, f32)], from: Vec3, to: Vec3) -> bool {
    let was = Vec2::new(from.x, from.z);
    let goes = Vec2::new(to.x, to.z);
    walls.iter().any(|(at, half, turn)| {
        // Into the wall's own frame, where it is an axis-aligned box grown by how
        // wide the warden is.
        let spin = Vec2::from_angle(-turn);
        let one = spin.rotate(was - *at);
        let two = spin.rotate(goes - *at);
        let grown = *half + Vec2::splat(WARDEN_IS_WIDE);

        let inside = |p: Vec2| p.x.abs() < grown.x && p.y.abs() < grown.y;
        if inside(one) {
            // Already in it: every step is allowed, so nobody is ever sealed in.
            return false;
        }
        if inside(two) {
            return true;
        }

        // THE WHOLE STEP, not just its ends.
        //
        // Testing the endpoints alone lets a step TUNNEL: a wall is 60 cm thick and
        // a step that starts outside and finishes past it never has an end inside
        // it, so it passes straight through. A walking pace never does that - a
        // frame of it is eight centimetres - but anything faster would, and a
        // collision that only holds at walking pace is one that breaks the first
        // time something is thrown, knocked back or ridden.
        //
        // Slab method: clip the segment against each pair of faces and see whether
        // any of it is left.
        let run = two - one;
        let (mut near, mut far) = (0.0_f32, 1.0_f32);
        for axis in 0..2 {
            let (start, delta, edge) = (one[axis], run[axis], grown[axis]);
            if delta.abs() < 1.0e-6 {
                if start.abs() >= edge {
                    return false;
                }
                continue;
            }
            let (mut lo, mut hi) = ((-edge - start) / delta, (edge - start) / delta);
            if lo > hi {
                std::mem::swap(&mut lo, &mut hi);
            }
            near = near.max(lo);
            far = far.min(hi);
            if near > far {
                return false;
            }
        }
        true
    })
}

/// Below this radius, in metres, a thing is not worth being stopped by.
///
/// A hand's width. Anything thinner is thinner than the tolerance either side of
/// it, and a warden brought to a halt by something he cannot see has met a bug
/// rather than an obstacle.
const THIN_ENOUGH_TO_PASS: f32 = 0.08;

/// What share of a prop's own reach is treated as solid.
///
/// A prop reports how far it extends from its middle, which is the right number for
/// deciding what it covers and the wrong shape for walking into: a fallen log
/// reaches its own length, and a circle of that radius is a bollard round a thing
/// that is mostly thin air at the ends. Three quarters lets a warden get near the
/// ends of a long prop while still holding him off the body of a boulder, which is
/// very nearly a circle already.
const PROPS_ARE_ROUNDER_THAN_THEY_REACH: f32 = 0.75;

#[derive(Component)]
pub struct Player;

/// How fast the warden is travelling, in metres a second - both what they got and
/// what they asked for, because the two answer different questions.
///
/// `speed` is MEASURED, because the asked speed is not it: a step into deep water or
/// up a cliff is refused, and a warden pressed against a canyon wall is not walking
/// however hard the key is held. That is what a walk cycle has to match or the feet
/// skate, so it drives the playback rate.
///
/// `wants` is what was ASKED, and it exists because using the measured speed to CHOOSE
/// the clip made the warden jitter. Measured speed is noisy - it was a 3D distance, so
/// it picked up the vertical travel from being planted on the terrain every frame and
/// read high on any slope, and it moves with frame time and with clamped steps. Choosing
/// a gait from a noisy number means that whenever the number sits near a handover
/// ceiling, the choice flips back and forth every frame, and each flip restarts a blend.
/// `wants` is exactly one of three constants, so it can never sit near a boundary and
/// can never chatter.
///
/// This is also how the games this one is measured against do it. Genshin Impact drives
/// locomotion from a discrete movement STATE, not from a velocity magnitude; velocity
/// only ever scales the clip once the state has chosen it. Same split as here.
#[derive(Component, Default)]
pub struct Striding {
    pub speed: f32,
    pub wants: f32,
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        // On ENTERING the game, not at startup. At startup the menu has not run,
        // so `Progress::from` is always empty — which made Continue a dead
        // letter: the warden was already standing at the ranch before the save
        // was read, the saved position was never applied, and the thirty-second
        // autosave then overwrote the real save with the ranch. New Game
        // mid-session had the mirror fault: the old warden stayed where they
        // were, and the "fresh" save inherited the position.
        app.add_systems(OnEnter(AppState::Playing), spawn_player)
            // The warden only walks in the game. In the terrain tool the same
            // keys fly the camera, and in the menu nothing should move at all.
            .init_resource::<crate::look::Look>()
            .add_systems(
                OnEnter(AppState::Playing),
                crate::motion::ask_for_the_clips,
            )
            .add_systems(
                Update,
                (
                    // Not while the map is up. The map covers the screen, so a
                    // player walking behind it is walking blind - and walking into
                    // water they cannot see.
                    move_player.run_if(not(crate::map::is_open)),
                    // The clips: asked for on the way in, found when the file
                    // arrives, handed to each player the scene brings in.
                    crate::motion::find_the_clips.run_if(crate::motion::still_waiting),
                    // BEFORE anything is played: the offset a worn thing gets is
                    // taken from the skeleton's rest pose, and a pose baked in at
                    // attachment stays baked in.
                    crate::look::hang_things_on_the_head
                        .before(crate::motion::hand_the_clips_over),
                    crate::motion::hand_the_clips_over
                        .run_if(crate::motion::the_clips_are_ready),
                    crate::motion::match_the_clip_to_the_walking
                        .run_if(crate::motion::the_clips_are_ready),
                    // Every frame while the world is open: a scene arrives over
                    // several frames and a part cannot be painted before it exists.
                    crate::look::paint_the_warden,
                )
                    .run_if(in_state(AppState::Playing)),
            );
    }
}

/// Picks somewhere sensible to start: the first patch of gentle, dry, low-lying
/// ground found spiralling out from the middle of the map. The map's center is
/// as likely to be open sea as anything else, so this can't just be the origin.
fn find_spawn(terrain: &TerrainSource, bounds: &WorldBounds) -> Vec3 {
    const RING_STEP: f32 = 48.0;
    let max_radius = bounds.half.length();

    let mut radius = 0.0;
    while radius < max_radius {
        // More samples the further out we go, so ring coverage stays even.
        let samples = ((radius / RING_STEP).ceil() as usize * 6).max(1);
        for i in 0..samples {
            let angle = i as f32 / samples as f32 * std::f32::consts::TAU;
            let p = Vec2::new(angle.cos(), angle.sin()) * radius;
            let height = terrain.height(p.x, p.y);
            // Clear of the tide, below the treeline, and flat enough to stand on.
            if height > 4.0 && height < 70.0 && terrain.normal(p.x, p.y, 2.0).y > 0.93 {
                return Vec3::new(p.x, height, p.y);
            }
        }
        radius += RING_STEP;
    }

    warn!("no suitable spawn found — dropping the warden at the origin");
    Vec3::new(0.0, terrain.height(0.0, 0.0), 0.0)
}

fn spawn_player(
    mut commands: Commands,
    assets: Res<AssetServer>,
    look: Res<crate::look::Look>,
    terrain: Res<TerrainSource>,
    bounds: Res<WorldBounds>,
    progress: Res<crate::save::Progress>,
    mut standing: Query<&mut Transform, With<Player>>,
) {
    // Continuing: exactly where they left off, facing the way they left off
    // facing. Landing a returning player at the ranch every time would make
    // Continue a slower New Game.
    let (spawn, facing) = if let Some(save) = &progress.from {
        let ground = terrain.height(save.at.x, save.at.z);
        // The ground rather than the stored height. A save carries a Y, but the
        // world under it can be resculpted between sittings — and a warden
        // restored to last week's height stands in the air or inside a hill.
        let at = Vec3::new(save.at.x, ground, save.at.z);
        info!("continuing at {:.0}, {:.0}", at.x, at.z);
        (at, save.facing)
    } else {
        // On the ranch, which is where the game begins. `find_spawn` is kept as
        // the fallback for a world whose map does not put land there — a redrawn
        // map could leave the pinned spot at sea, and dropping the warden into
        // the water with no explanation is worse than starting them somewhere
        // arbitrary.
        let ranch = Vec2::new(RANCH_AT.0, RANCH_AT.1);
        let on_land = terrain.height(ranch.x, ranch.y) > SEA_LEVEL + 1.0;
        let spawn = if on_land {
            Vec3::new(ranch.x, terrain.height(ranch.x, ranch.y), ranch.y)
        } else {
            warn!("the ranch at {:.0}, {:.0} is under water on this map", ranch.x, ranch.y);
            find_spawn(&terrain, &bounds)
        };
        info!("warden spawning at {:.0}, {:.0}", spawn.x, spawn.z);
        (spawn, 0.0)
    };

    // A warden already standing — the second visit to Playing this session — is
    // MOVED, not doubled: the body is built once and this is where it goes now.
    if let Some(mut warden) = standing.iter_mut().next() {
        warden.translation = spawn;
        warden.rotation = Quat::from_rotation_y(facing);
        return;
    }
    raise_the_warden(&mut commands, &assets, &look, spawn, facing);
}

/// Stands the warden up, wherever they are starting from.
///
/// One body, built once. Both ways into the world need it — a new game at the
/// ranch and a continued one where the save left off — and a warden assembled in
/// two places is a warden that grows a hat in one of them.
fn raise_the_warden(
    commands: &mut Commands,
    assets: &AssetServer,
    look: &crate::look::Look,
    spawn: Vec3,
    facing: f32,
) {
    // The body, and the hair and hat as children of it.
    //
    // Three files rather than one because a hairstyle and a hat are CHOICES: the
    // body is the same model whichever hair is on it, and a hat is authored to sit
    // over a wig rather than instead of one. They are parented to the warden, so
    // they travel and turn with them without anything having to keep them in step.
    let body: Handle<Scene> =
        assets.load(GltfAssetLabel::Scene(0).from_asset(look.build.model()));

    // The body hangs on a child of its own, carrying the scale and the turn that
    // make this particular file fit the world. The warden's OWN transform stays
    // clean: it is what the walk moves, what the camera follows and what the save
    // records, and none of that wants a model's quirks baked into it.
    let fitted = Transform::from_scale(Vec3::splat(
        crate::look::TALL / look.build.authored_height(),
    ))
    .with_rotation(Quat::from_rotation_y(look.build.turn()));

    commands
        .spawn((
            Player,
            // Pushes the grass aside as they go. About the width of a person
            // plus an arm — what actually brushes past is wider than what walks.
            crate::shade::Wades { reach: 1.8 },
            // Its parts are painted as the scene brings them in — see
            // `look::paint_the_warden`. Nothing can be painted at spawn: a glTF
            // scene is instanced asynchronously and none of it exists yet.
            crate::look::Dressing,
            Striding::default(),
            Transform::from_translation(spawn).with_rotation(Quat::from_rotation_y(facing)),
            Visibility::default(),
        ))
        .with_children(|parent| {
            parent.spawn((SceneRoot(body), fitted, Visibility::default()));
            // A model that arrives dressed keeps its own hair, and a cap over
            // modelled hair is two heads of it.
            if look.build.dressed() {
                return;
            }
            if let Some(style) = look.hair.model() {
                let hair: Handle<Scene> =
                    assets.load(GltfAssetLabel::Scene(0).from_asset(style));
                parent.spawn((
                    SceneRoot(hair),
                    crate::look::WornOnTheHead,
                    Transform::default(),
                    Visibility::default(),
                ));
            }
            if let Some(worn) = look.hat.model() {
                let hat: Handle<Scene> = assets.load(GltfAssetLabel::Scene(0).from_asset(worn));
                parent.spawn((
                    SceneRoot(hat),
                    crate::look::WornOnTheHead,
                    Transform::default(),
                    Visibility::default(),
                ));
            }
        });
}

pub fn move_player(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mode: Res<CameraMode>,
    terrain: Res<TerrainSource>,
    grove: Option<Res<crate::world::stream::Grove>>,
    props: Option<Res<crate::world::prop::PropPool>>,
    towns: Res<crate::world::town::Built>,
    bounds: Res<WorldBounds>,
    cameras: Query<&Transform, (With<MainCamera>, Without<Player>)>,
    mut players: Query<(&mut Transform, &mut Striding), With<Player>>,
    // Two buffers held between frames rather than allocated in each one. What is
    // standing near the warden is asked for every frame they move, and both of these
    // were built from nothing every time - see `Plot::walls_into`.
    mut standing: Local<Vec<Trunk>>,
    mut walls: Local<Vec<(Vec2, Vec2, f32)>>,
) {
    // In free-fly the same keys drive the camera instead.
    if *mode == CameraMode::Fly {
        return;
    }
    let (Some(camera), Ok((mut transform, mut pace))) = (cameras.iter().next(), players.single_mut())
    else {
        return;
    };

    // Movement is relative to where the camera is pointing, flattened onto the
    // ground plane so looking up or down never changes how fast you travel.
    let forward = camera.forward().as_vec3();
    let forward = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
    let right = Vec3::new(-forward.z, 0.0, forward.x);

    let mut input = Vec3::ZERO;
    if keys.any_pressed([KeyCode::KeyW, KeyCode::ArrowUp]) {
        input += forward;
    }
    if keys.any_pressed([KeyCode::KeyS, KeyCode::ArrowDown]) {
        input -= forward;
    }
    if keys.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]) {
        input += right;
    }
    if keys.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]) {
        input -= right;
    }

    let direction = input.normalize_or_zero();
    if direction == Vec3::ZERO {
        pace.speed = 0.0;
        pace.wants = 0.0;
    }
    if direction != Vec3::ZERO {
        // Jogging is the DEFAULT and walking is the deliberate choice, which is the
        // way round every game this one is measured against does it. Ctrl slows to a walk.
        //
        // There is no sprint. There never was an animation for one - the clip called `run.glb`
        // measures out as a JOG, 23 frames a cycle at 130 steps a minute against a run's 12-16
        // and 180-240 - and the tier was carrying a speed no clip could serve. "We probably dont
        // even need a sprint (run) in this game."
        let speed = if keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]) {
            WALK_SPEED
        } else {
            JOG_SPEED
        };
        let next = transform.translation + direction * speed * time.delta_secs();
        let next = bounds.clamp(next, 2.0);

        // The step, or the best part of it: a refused step is retried along each
        // axis alone, so brushing a canyon wall at an angle slides along it
        // rather than sticking to it.
        let from = transform.translation;
        // What is standing here, asked once for all three candidate steps below.
        standing_near(&terrain.0, grove.as_deref(), props.as_deref(), from, &mut standing);
        // The town's walls. A building is not a post, so it cannot be a circle: a
        // house's front is six metres of wall with a doorway in it, and a disc round
        // its middle would either seal the door or leave the corners walkable. They
        // come as oriented slabs and are tested as such.
        towns.walls_near(Vec2::new(from.x, from.z), LOOKS_AHEAD, &mut walls);
        let step = [
            next,
            Vec3::new(next.x, from.y, from.z),
            Vec3::new(from.x, from.y, next.z),
        ]
        .into_iter()
        .find(|to| *to != from && may_step(&terrain.0, &towns, &standing, &walls, from, *to));
        let before = transform.translation;
        if let Some(to) = step {
            transform.translation = to;
        }
        // From what actually happened, not from what was asked - but HORIZONTALLY, and
        // settled. The 3D distance was wrong twice over: it counted the vertical travel
        // from planting the feet on the terrain, so a slope read as extra ground speed,
        // and it inherited every frame-time wobble and clamped step as a spike. Feeding
        // that to the playback rate made the clip's tempo flicker frame to frame, which
        // is half of what the jitter was.
        let went = transform.translation.xz().distance(before.xz());
        let measured = if time.delta_secs() > 0.0 {
            went / time.delta_secs()
        } else {
            0.0
        };
        let settles = (SPEED_SETTLES * time.delta_secs()).clamp(0.0, 1.0);
        pace.speed += (measured - pace.speed) * settles;
        // A warden shoved against a wall measures nearly nothing, and a clip played at
        // nearly nothing is a frozen pose. Keep enough of the asked speed to carry on
        // running in place, which is what it looks like from outside anyway.
        pace.speed = pace.speed.max(speed * BLOCKED_STILL_RUNS);
        pace.wants = speed;

        // Ease into the new facing instead of snapping, so quick direction
        // changes read as a turn rather than a teleport.
        if let Some(target) = facing_quat(direction) {
            let t = (TURN_RATE * time.delta_secs()).min(1.0);
            transform.rotation = transform.rotation.slerp(target, t);
        }
    }

    // Plant the feet on the ground every frame, including when standing still,
    // so the warden settles correctly the moment the world finishes loading.
    let ground = crate::world::town::stands_on(
        &terrain.0,
        &towns,
        Vec2::new(transform.translation.x, transform.translation.z),
    );
    transform.translation.y = ground.max(SEA_LEVEL - WADE_DEPTH);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::save::{Progress, Save};
    use bevy::state::app::StatesPlugin;

    /// The way east is WALKABLE end to end, with the climb rule in force.
    ///
    /// The gate has to refuse the walls and pass the floor. The wall half is the
    /// test below; this is the other half, and it is the one that would strand a
    /// player: a canyon nobody can walk is not a gate, it is a full stop.
/// A warden cannot walk through a trunk, can always walk out of one, and slides
    /// along it rather than sticking.
    #[test]
    fn a_trunk_stops_a_warden_and_never_traps_one() {
        let oak = [Trunk {
            at: Vec2::new(10.0, 0.0),
            radius: 0.5,
        }];
        let keep = 0.5 + WARDEN_IS_WIDE;

        // Walking at it: refused once the step would put him inside.
        let outside = Vec3::new(10.0 - keep - 0.2, 0.0, 0.0);
        let inside = Vec3::new(10.0 - keep + 0.1, 0.0, 0.0);
        assert!(
            into_a_trunk(&oak, outside, inside),
            "he walked into the tree"
        );

        // Standing in it — put there by a brush, or a tree planted around him —
        // every step that gets him OUT is allowed. This is the half that matters:
        // a rule that refused every position inside a trunk would trap him there
        // forever.
        let stuck = Vec3::new(10.0, 0.0, 0.0);
        for step in 0..16 {
            let turn = step as f32 / 16.0 * std::f32::consts::TAU;
            let out = stuck + Vec3::new(turn.cos(), 0.0, turn.sin()) * 0.3;
            assert!(
                !into_a_trunk(&oak, stuck, out),
                "standing in the trunk, the step at {turn:.2} rad was refused too"
            );
        }

        // And sliding: walking north-east into the tree's west face, the diagonal
        // is refused but the northward part of it is not, which is what
        // `move_player` retries and what makes him slide round rather than stop
        // dead.
        let beside = Vec3::new(10.0 - keep - 0.05, 0.0, 0.0);
        let diagonal = beside + Vec3::new(0.2, 0.0, 0.2);
        let sideways = beside + Vec3::new(0.0, 0.0, 0.2);
        assert!(into_a_trunk(&oak, beside, diagonal), "the diagonal went in");
        assert!(
            !into_a_trunk(&oak, beside, sideways),
            "sliding along the trunk was refused, so he sticks to it"
        );
    }

    /// The trunks the world actually grows are wide enough to bump into and narrow
    /// enough to walk between.
    ///
    /// Ignored: it grows the whole pool, which is slower than a unit test should be,
    /// and it is a measurement to READ as much as a guard.
    #[test]
    #[ignore = "a measurement of the real pool"]
    fn what_the_trunks_measure() {
        let mut narrowest = f32::MAX;
        let mut widest: f32 = 0.0;
        for seed in 0..terrain_core::tree::VARIETIES as u32 {
            let tree = terrain_core::tree::grow(seed);
            let _ = tree.height;
            let floor = tree
                .wood
                .places
                .iter()
                .fold(f32::MAX, |low, place| low.min(place[1]));
            let radius = tree
                .wood
                .places
                .iter()
                .filter(|place| place[1] <= floor + 0.35)
                .map(|place| (place[0] * place[0] + place[2] * place[2]).sqrt())
                .filter(|radius| *radius > 0.02)
                .fold(f32::MAX, f32::min);
            println!(
                "{:?} {:.2} m tall, bole {:.3} m at chest height",
                tree.species, tree.height, radius
            );
            narrowest = narrowest.min(radius);
            widest = widest.max(radius);
        }
        println!("trunks run {narrowest:.3} m to {widest:.3} m");
        // Every bole in the pool falls between 0.14 m and 0.61 m. The bounds are
        // wide either side of that: this is a runaway guard on the measure in
        // `stream::trunk_radius`, which has read a bough as a trunk before, not a
        // pin on the numbers themselves.
        assert!(
            widest < 1.0,
            "the widest bole is {widest:.2} m — that is a bough, not a trunk"
        );
        assert!(
            narrowest > 0.05,
            "the narrowest bole is {narrowest:.3} m — a warden would walk through it"
        );
    }

/// What lies about stops a warden only when it has a body.
/// What the litter actually measures, and what a warden will be held off by.
    ///
    /// Ignored: it grows the whole prop pool, and it is a measurement to READ.
    #[test]
    #[ignore = "a measurement of the real pool"]
    fn what_the_litter_measures() {
        use terrain_core::prop;
        let mut worst_solid = 0.0_f32;
        for variety in 0..prop::VARIETIES {
            let grown = prop::from_pool(variety);
            let held = grown.reach * PROPS_ARE_ROUNDER_THAN_THEY_REACH;
            println!(
                "{:?}: reaches {:.2} m, held off at {:.2} m{}",
                grown.kind,
                grown.reach,
                held,
                if crate::world::prop::is_solid(grown.kind) {
                    ""
                } else {
                    "  (walked through)"
                }
            );
            if crate::world::prop::is_solid(grown.kind) {
                worst_solid = worst_solid.max(held);
            }
        }
        println!("the widest solid thing holds a warden off at {worst_solid:.2} m");
        // A boulder is a boulder, not a building. Anything past this and the world
        // is full of obstacles bigger than the gaps between them.
        assert!(
            worst_solid < 4.0,
            "the widest solid litter holds a warden {worst_solid:.2} m off"
        );
    }

/// You can walk in the front door, and not through the wall beside it.
    ///
    /// The single thing the whole town is for. A building whose collision is one
    /// block is scenery; what makes it a PLACE is that the front wall has a gap in
    /// it you can get through and cannot get through anywhere else.
    #[test]
    fn a_warden_walks_in_the_door_and_not_through_the_wall() {
        use crate::world::town::{Building, Plot};

        for turn in [0.0_f32, 0.7, 2.4, -1.9] {
            let plot = Plot {
                at: Vec2::new(40.0, -15.0),
                facing: turn,
                what: Building::Cottage,
                district: crate::world::town::District::Market,
            };
            let walls = plot.walls();
            let half = plot.what.footprint() * 0.5;
            // Out of the front of the building, in its own frame, and back in.
            let out = |local: Vec2| {
                let (sin, cos) = turn.sin_cos();
                plot.at + Vec2::new(local.x * cos - local.y * sin, local.x * sin + local.y * cos)
            };
            let flat = |at: Vec2| Vec3::new(at.x, 0.0, at.y);

            // Straight at the doorway, from outside to inside.
            let outside = out(Vec2::new(0.0, -half.y - 2.0));
            let inside = out(Vec2::new(0.0, -half.y + 1.2));
            assert!(
                !into_a_wall(&walls, flat(outside), flat(inside)),
                "turn {turn}: the doorway is blocked"
            );

            // And at the wall beside it, which must not let him through.
            let at_wall = out(Vec2::new(half.x - 0.4, -half.y - 2.0));
            let through = out(Vec2::new(half.x - 0.4, -half.y + 1.2));
            assert!(
                into_a_wall(&walls, flat(at_wall), flat(through)),
                "turn {turn}: he walked through the front wall"
            );

            // Nor through the back of it.
            let behind = out(Vec2::new(0.0, half.y + 2.0));
            let in_back = out(Vec2::new(0.0, half.y - 1.2));
            assert!(
                into_a_wall(&walls, flat(behind), flat(in_back)),
                "turn {turn}: he walked in through the back wall"
            );
        }
    }

    /// And having got in, he can always get out again.
    #[test]
    fn nobody_is_ever_sealed_inside_a_building() {
        use crate::world::town::{Building, Plot};
        let plot = Plot {
            at: Vec2::ZERO,
            facing: 0.4,
            what: Building::Shop,
                district: crate::world::town::District::Market,
        };
        let walls = plot.walls();
        // Standing in the middle of the room, every step outward is allowed - the
        // same rule the trees and the cliffs keep, and for the same reason: a rule
        // that refuses every position inside a wall traps whoever ends up in one.
        let middle = Vec3::new(plot.at.x, 0.0, plot.at.y);
        for step in 0..24 {
            let turn = step as f32 / 24.0 * std::f32::consts::TAU;
            let out = middle + Vec3::new(turn.cos(), 0.0, turn.sin()) * 0.4;
            assert!(
                !into_a_wall(&walls, middle, out),
                "the step at {turn:.2} rad out of the middle of a shop was refused"
            );
        }
    }

    #[test]
    fn a_boulder_stops_a_warden_and_a_bed_of_scree_does_not() {
        use terrain_core::prop::Kind;

        for kind in [Kind::Boulder, Kind::Stump, Kind::Log, Kind::Snag, Kind::Cactus] {
            assert!(
                crate::world::prop::is_solid(kind),
                "{kind:?} can be walked through"
            );
        }
        // Ground cover with a shape. Stopping a warden at the rim of a bush is the
        // same fault as an invisible sapling: something he can see he could step
        // past, refusing him.
        for kind in [Kind::Scree, Kind::Bush, Kind::Brush] {
            assert!(
                !crate::world::prop::is_solid(kind),
                "{kind:?} is a wall, and it should not be"
            );
        }
    }


    /// A warden can still walk in through a front door.
    ///
    /// # The risk this change carried
    ///
    /// Making the floor answer for the ground fixes the feet sinking into the boards
    /// and introduces a way to break something far worse: a floor that appears at the
    /// footprint's edge is a lip, and the cliff rule refuses a rise steeper than
    /// `CLIMB_LIMIT` - so every house in the world could have become a house you can
    /// see into and never enter.
    ///
    /// That is why the doorstep is measured off the model and read as a ramp. This
    /// walks the approach at the stride a jog actually takes and asserts every step
    /// of it is allowed, and that the warden ends up on the boards rather than under
    /// them.
    #[test]
    fn a_warden_can_walk_in_through_a_front_door() {
        use crate::world::town::{Building, Built, District, Layout, Plot};

        let terrain = crate::world::terrain::Terrain::new();
        // ON A SLOPE, which is the only place this can go wrong.
        //
        // A building's floor is levelled at the HIGHEST of its four corners, so on
        // flat ground it stands a hand's breadth over the earth and a warden steps
        // up onto it without noticing. On a slope it can be half a metre over the
        // ground at its own door, and that is the lip the walking rule refuses.
        // Written against flat ground first, this test passed with the doorstep
        // taken out entirely - it was proving nothing.
        let half_of = |what: Building| what.footprint() * 0.5;
        let step_up = |at: Vec2, what: Building| {
            crate::world::town::stands_at(&terrain, at, what.footprint(), 0.0)
                - terrain.height(at.x, at.y - half_of(what).y)
        };
        let mut at = Vec2::new(RANCH_AT.0, RANCH_AT.1 - 60.0);
        for _ in 0..600 {
            if step_up(at, Building::Cottage) > 0.35 && terrain.height(at.x, at.y) > SEA_LEVEL + 2.0
            {
                break;
            }
            at.x += 4.0;
        }
        assert!(
            step_up(at, Building::Cottage) > 0.35,
            "no sloping ground found to stand a cottage on",
        );

        for what in [Building::Cottage, Building::Shop, Building::GuildHall] {
            // Facing +y, so the front - and the doorstep - is on the -y side.
            let plot = Plot {
                at,
                facing: 0.0,
                district: District::Market,
                what,
            };
            let half = what.footprint() * 0.5;
            let walls = plot.walls();
            let mut built = Built::default();
            built.standing.insert(
                0,
                Layout {
                    ways: Vec::new(),
                    streets: Vec::new(),
                    plots: vec![plot],
                    lamps: Vec::new(),
                },
            );

            let stand = |flat: Vec2| {
                Vec3::new(
                    flat.x,
                    crate::world::town::stands_on(&terrain, &built, flat),
                    flat.y,
                )
            };
            // In at the door, from three metres out to the middle of the room, at
            // the stride a jog covers in a frame.
            let stride = JOG_SPEED / 60.0;
            let mut walked = at - Vec2::new(0.0, half.y + 3.0);
            let mut refused = 0;
            let steps = ((half.y + 3.0) / stride).ceil() as i32;
            for _ in 0..steps {
                let next = walked + Vec2::new(0.0, stride);
                if may_step(&terrain, &built, &[], &walls, stand(walked), stand(next)) {
                    walked = next;
                } else {
                    refused += 1;
                    walked = next;
                }
            }
            assert_eq!(
                refused, 0,
                "{what:?} refused {refused} of {steps} steps of its own doorway",
            );

            // And having walked in, the warden is on the boards.
            let inside = crate::world::town::stands_on(&terrain, &built, at);
            let outside = terrain.walk_height(at.x, at.y - half.y - 3.0);
            assert!(
                inside > outside,
                "{what:?}'s floor is at {inside:.2} and the ground outside is {outside:.2}",
            );
        }
    }

    #[test]
    fn the_canyon_can_be_walked_from_the_desert_to_the_green_world() {
        let terrain = crate::world::terrain::Terrain::new();
        let stand = |flat: Vec2| Vec3::new(flat.x, terrain.height(flat.x, flat.y), flat.y);

        // Along the canyon's own way, at a stride a walk actually takes.
        let mut at = crate::world::pass::way_through(-320.0);
        let mut refused = 0;
        let mut worst = 0.0_f32;
        for step in -319..=320 {
            let next = crate::world::pass::way_through(step as f32);
            if may_step(&terrain, &crate::world::town::Built::default(), &[], &[], stand(at), stand(next)) {
                at = next;
            } else {
                refused += 1;
                let rise = terrain.height(next.x, next.y) - terrain.height(at.x, at.y);
                worst = worst.max(rise);
            }
        }
        assert_eq!(
            refused, 0,
            "{refused} steps along the canyon are refused, the worst a {worst:.1} m rise"
        );
        let out = crate::world::pass::way_through(320.0);
        assert!(
            at.distance(out) < 1.0,
            "the walk stopped {:.0} m short of the eastern mouth",
            at.distance(out)
        );
    }

    /// A canyon wall is a WALL to the walk: the step up it is refused, the step
    /// back down is not, and walking along the floor is untouched.
    ///
    /// Terrain is the game's only geometry and nothing else stops a walker — so
    /// without this, the warden strolls up a seventy-degree face and the canyon
    /// gates nothing at all.
    /// Walking into a city is not stopped by anything invisible.
    ///
    /// Reported as invisible walls on the way in. A wall in this game is a refused
    /// STEP, so the way to find one is to walk the approach in the strides the game
    /// takes and ask `may_step` at every one - which is what the player's own
    /// movement does, and what no test had ever done along a real approach road.
    #[test]
    fn walking_into_a_city_is_not_stopped_by_anything_invisible() {
        let terrain = crate::world::terrain::Terrain::new();
        let plan = terrain.plan();
        let built = crate::world::town::Built::default();
        let mut refused: Vec<String> = Vec::new();
        for site in plan.sites().iter().filter(|s| s.city && !s.ranch) {
            // In along the road the town's high street is built along, from well
            // outside its ground to its middle.
            let out = plan.approach(site.at).normalize_or(bevy::math::Vec2::Y);
            let stride = 0.12;
            let mut along = site.radius * 1.6;
            while along > 0.0 {
                let from = site.at + out * along;
                let to = site.at + out * (along - stride);
                let seat = |p: bevy::math::Vec2| {
                    let y = crate::world::town::stands_on(&terrain, &built, p);
                    Vec3::new(p.x, y, p.y)
                };
                if !may_step(&terrain, &built, &[], &[], seat(from), seat(to)) {
                    refused.push(format!(
                        "{:.0} m out from a city at {:.0},{:.0}: {:.2} m up in a {:.2} m stride",
                        along,
                        site.at.x,
                        site.at.y,
                        crate::world::town::stands_on(&terrain, &built, to)
                            - crate::world::town::stands_on(&terrain, &built, from),
                        stride,
                    ));
                }
                along -= stride;
            }
        }
        assert!(
            refused.is_empty(),
            "{} steps refused on the way into a city:
  {}",
            refused.len(),
            refused
                .iter()
                .take(6)
                .cloned()
                .collect::<Vec<_>>()
                .join("
  "),
        );
    }

    /// What a warden may climb does not depend on how fast the game is drawing.
    ///
    /// # The rule that was really a frame-rate rule
    ///
    /// The step allowance was measured across one frame's movement, so it granted a
    /// slope of 3:1 at 60 Hz and 11:1 at 240 Hz: the canyon walls gated the world on
    /// a slow machine and not on a fast one, and no test could see it because every
    /// test took one sample of a comfortable size. Codex found it by reading the
    /// units rather than the behaviour.
    ///
    /// So this walks each fixture at the stride each speed actually takes at each of
    /// four frame rates - sixteen strides an approach, the shortest 6 mm - and the
    /// answer has to come out the same every time. See `STEP_LANDS`.
    #[test]
    fn what_may_be_climbed_does_not_change_with_the_frame_rate() {
        let terrain = crate::world::terrain::Terrain::new();
        let built = crate::world::town::Built::default();
        let stand = |flat: Vec2| Vec3::new(flat.x, terrain.height(flat.x, flat.y), flat.y);

        // The canyon wall, found the same way the step-up test finds it.
        let middle = crate::world::pass::way_through(40.0);
        let (sin, cos) = crate::world::pass::HEADING.sin_cos();
        let out = -Vec2::new(-sin, cos);
        let floor = terrain.height(middle.x, middle.y);
        let mut foot = middle;
        for step in 1..200 {
            let at = middle + out * step as f32;
            if terrain.height(at.x, at.y) > floor + 2.0 {
                break;
            }
            foot = at;
        }

        // A REAL KERB, on a real city street, found by walking out from the crown
        // until the ground lifts. A closure shaped like a kerb would only prove the
        // rule agrees with itself; this is the thing the warden actually walks on.
        let (built, crown, aside) = a_city_street(&terrain);
        let on = |at: Vec2| crate::world::town::stands_on(&terrain, &built, at);
        let road = on(crown);
        let mut kerb = crown;
        for out in 1..80 {
            let at = crown + aside * out as f32 * 0.1;
            if on(at) > road + 0.05 {
                kerb = at - aside * 0.1;
                break;
            }
        }
        assert!(
            on(kerb + aside * 1.0) > road + 0.1,
            "no kerb found beside the city street to test with"
        );

        // How far UP the wall each frame rate gets the warden. Walking, not sampling:
        // the old rule's fault was invisible to any single sample of a comfortable
        // size, and only a real stride taken over and over can show it.
        let mut climbed = Vec::new();
        for &hertz in &[30.0_f32, 60.0, 120.0, 240.0] {
            for &speed in &[WALK_SPEED, JOG_SPEED] {
                let stride = speed / hertz;
                let mut at = foot;
                // Bounded by DISTANCE, so every rate is given the same chance: five
                // metres of wall is more than enough to be up it.
                while at.distance(foot) < 5.0 {
                    let to = at + out * stride;
                    if !may_step(&terrain, &built, &[], &[], stand(at), stand(to)) {
                        break;
                    }
                    at = to;
                }
                climbed.push((hertz, speed, terrain.height(at.x, at.y) - floor));

                // And the kerb, stepped up onto at this rate: a real step must
                // still be climbable however short the stride is.
                let up = kerb + aside * stride;
                assert!(
                    may_step(
                        &terrain,
                        &built,
                        &[],
                        &[],
                        Vec3::new(kerb.x, on(kerb), kerb.y),
                        Vec3::new(up.x, on(up), up.y),
                    ),
                    "the kerb is refused at {hertz} Hz with a {stride:.3} m stride"
                );
            }
        }

        let least = climbed.iter().map(|got| got.2).fold(f32::MAX, f32::min);
        let most = climbed.iter().map(|got| got.2).fold(f32::MIN, f32::max);
        assert!(
            most - least < 0.5,
            "the canyon wall gives way at some frame rates and not others: {climbed:?}"
        );
        // And it is a WALL at all of them. The wall rises past twenty metres; a
        // warden who is still in the first metre and a half of it has been stopped.
        assert!(
            most < 1.5,
            "the wall was climbed {most:.1} m — the canyon gates nothing: {climbed:?}"
        );
    }

    /// One real city street, as somewhere to stand on a crown and a kerb.
    fn a_city_street(
        terrain: &crate::world::terrain::Terrain,
    ) -> (crate::world::town::Built, Vec2, Vec2) {
        let (built, from, to) = crate::world::town::a_paved_street(terrain);
        let along = (to - from).normalize_or_zero();
        ((built), (from + to) * 0.5, Vec2::new(-along.y, along.x))
    }

    #[test]
    fn a_canyon_wall_refuses_the_step_up_but_never_the_step_down() {
        let terrain = crate::world::terrain::Terrain::new();
        let middle = crate::world::pass::way_through(40.0);
        let ahead = crate::world::pass::way_through(43.0);
        let (sin, cos) = crate::world::pass::HEADING.sin_cos();
        // Toward negative across: the plain slot wall, clear of the junctions.
        let out = -Vec2::new(-sin, cos);
        let stand = |flat: Vec2| Vec3::new(flat.x, terrain.height(flat.x, flat.y), flat.y);

        // Walk out from the middle to where the wall starts, so the step under
        // test is the one that leaves the floor — the gap's width is a tuning
        // number and this test must not care what it currently is.
        let floor = terrain.height(middle.x, middle.y);
        let mut foot = middle;
        for step in 1..200 {
            let at = middle + out * step as f32;
            if terrain.height(at.x, at.y) > floor + 2.0 {
                break;
            }
            foot = at;
        }
        // ON the wall, not at its toe. A canyon wall starts gently - the first 60 cm
        // out of `foot` rise at 1.24 m/m, which is under `CLIMB_LIMIT` and genuinely
        // walkable - and steepens fast. The step under test is one taken where the
        // wall is a wall; how far the warden gets up the gentle part before being
        // stopped is `what_may_be_climbed_does_not_change_with_the_frame_rate`.
        let on_wall = foot + out * 1.5;
        let into_wall = foot + out * 3.0;
        assert!(
            (terrain.height(foot.x, foot.y) - floor).abs() < 2.5,
            "the scan never found the canyon floor"
        );
        assert!(
            may_step(&terrain, &crate::world::town::Built::default(), &[], &[], stand(middle), stand(ahead)),
            "walking along the canyon floor is refused"
        );
        assert!(
            !may_step(&terrain, &crate::world::town::Built::default(), &[], &[], stand(on_wall), stand(into_wall)),
            "the wall let the warden walk up it — the canyon gates nothing"
        );
        assert!(
            may_step(&terrain, &crate::world::town::Built::default(), &[], &[], stand(into_wall), stand(on_wall)),
            "the way back DOWN the wall is refused — a slope became a trap"
        );
    }

    /// The whole continue-and-new-game flow, run rather than reasoned about.
    ///
    /// # The bug this pins was invisible to every other test
    ///
    /// `spawn_player` ran at `Startup`, when the menu had not run and
    /// `Progress::from` was still empty — so Continue never applied the saved
    /// position, the autosave then overwrote the real save with the ranch, and
    /// New Game mid-session left the old warden standing where they were. Every
    /// piece worked alone; the fault was WHEN they ran against each other.
    #[test]
    fn continuing_stands_the_warden_where_the_save_says_and_new_game_at_the_ranch() {
        let terrain = crate::world::terrain::Terrain::new();
        let half = terrain.half();

        let mut app = App::new();
        app.add_plugins((
            bevy::app::TaskPoolPlugin::default(),
            bevy::asset::AssetPlugin::default(),
            StatesPlugin,
        ))
        .init_state::<AppState>()
        .init_asset::<Mesh>()
        // The warden is a glTF scene now. Without `Assets<Scene>` registered,
        // allocating a handle for one panics inside the asset server rather than
        // failing to load — which reads as a mystery rather than a missing plugin.
        .init_asset::<Scene>()
        .init_asset::<crate::shade::Shaded>()
        .insert_resource(TerrainSource(std::sync::Arc::new(terrain)))
        .insert_resource(WorldBounds {
            half,
            min_chunk: IVec2::ZERO,
            max_chunk: IVec2::ZERO,
        })
        .init_resource::<Progress>()
        // The warden is a model now, and raising one reads how it should look.
        .init_resource::<crate::look::Look>()
        .add_systems(OnEnter(AppState::Playing), spawn_player);

        // A save from somewhere that is NOT the ranch, with a stale height —
        // the world may have been resculpted since it was written.
        let ranch = Vec2::new(RANCH_AT.0, RANCH_AT.1);
        let elsewhere = ranch + Vec2::new(400.0, 250.0);
        app.world_mut().resource_mut::<Progress>().from = Some(Save {
            at: Vec3::new(elsewhere.x, 9_999.0, elsewhere.y),
            facing: 1.25,
            played: 60.0,
            stamped: String::new(),
        });

        // Continue: into the world.
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Playing);
        app.update();

        let standing = |app: &mut App| {
            let mut wardens = app.world_mut().query_filtered::<&Transform, With<Player>>();
            let all: Vec<Transform> = wardens.iter(app.world()).copied().collect();
            assert_eq!(all.len(), 1, "there are {} wardens standing", all.len());
            all[0]
        };
        let warden = standing(&mut app);
        assert!(
            (warden.translation.x - elsewhere.x).abs() < 0.01
                && (warden.translation.z - elsewhere.y).abs() < 0.01,
            "Continue put the warden at {:?}, not at the save",
            warden.translation
        );
        assert!(
            warden.translation.y < 5_000.0,
            "the save's stale height was believed: {:.0}",
            warden.translation.y
        );

        // Back to the menu, then NEW GAME: the save is dropped and the SAME
        // warden — not a second one — stands at the ranch again.
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Menu);
        app.update();
        app.world_mut().resource_mut::<Progress>().from = None;
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Playing);
        app.update();

        let warden = standing(&mut app);
        assert!(
            (warden.translation.x - ranch.x).abs() < 0.01
                && (warden.translation.z - ranch.y).abs() < 0.01,
            "New Game left the warden at {:?} instead of the ranch",
            warden.translation
        );
    }
}
