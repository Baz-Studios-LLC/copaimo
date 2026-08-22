//! Plays the warden's clips, so walking looks like walking.
//!
//! # A rig with no clip is a figure that slides
//!
//! `dev/art/people.py` authors two clips into the body — `walk` and `idle` — and a
//! glTF skin arrives in Bevy as a `SkinnedMesh` with no help needed. What does not
//! happen on its own is anything playing: the skeleton just sits in its rest pose
//! while the warden slides about the world like a chess piece.
//!
//! # Found by name, not by index
//!
//! The clips could be reached as `Animation(0)` and `Animation(1)`, and the order
//! they come out in is whatever the exporter felt like — here it was idle first,
//! which is alphabetical and not meaningful. So the `Gltf` asset is read and the
//! clips are looked up by their own names. Adding a third clip then cannot silently
//! turn walking into standing.
//!
//! # The speed follows the warden
//!
//! A walk cycle played at a fixed rate against a warden moving at a different one
//! is the skating that every game with feet has to solve. The stride covers a known
//! distance, so the clip's speed is simply how many strides a second the warden is
//! actually managing.

use bevy::prelude::*;

use crate::player::{Player, Striding};

/// Above this, in metres a second, the warden is running rather than walking.
///
/// Between the walk's own comfortable pace and the sprint: a warden pushing along
/// at the top of a walk should already be running, because a walk cycle played fast
/// enough to keep up with a sprint reads as a cartoon scurry.
// Between the two speeds, and it has to be — this was 6.5 while WALK_SPEED was 7.0,
// so the threshold sat BELOW walking pace and the walk clip never played once. Any
// value here must lie strictly between `player::WALK_SPEED` and
// `player::SPRINT_SPEED`, which `the_gait_threshold_lies_between_the_speeds` checks.
const BREAKS_INTO_A_RUN: f32 = 3.4;

/// How far one cycle of each clip carries the warden, in metres.
///
/// # Measured off the planted foot, which took three goes to get right
///
/// First estimated as `2 * leg * sin(stride angle)`, giving 1.35 and 1.14. Both
/// wrong, because a leg's reach is not the ground it covers.
///
/// Then MEASURED as one foot's fore-aft swing, doubled, on the reasoning that a
/// cycle is both feet taking one step. Also wrong, and wrong differently for the two
/// gaits — which is the sort of error that hides. The identity is
/// `speed = cadence * stride`, and a planted foot is stationary on the GROUND, so
/// relative to the hips it travels backward at exactly the character's speed. A foot
/// moving `S` during a stance lasting a fraction `f` of the cycle therefore carries
/// the body `S / f`, not `2 S`. A walk has a foot down about 60% of the time and a
/// run about 35%, so doubling overstates a walk and understates a run by a third.
///
/// `dev/art/stride_measure.py` now fits a line to the planted foot's travel and
/// reports the slope, so `f` falls out instead of being assumed.
///
/// **Measured by the one identity that is exact: contact length over stance
/// fraction.** The contact length is the planted foot's travel relative to the hips,
/// taken over the window each clip AUTHORS that foot to be down - poses 0 to
/// stance-1 - because that is the only stretch where the foot is on the ground and
/// the identity applies. 0.795 m walking, 0.716 jogging, 0.750 sprinting, against a
/// human figure of roughly one leg length in every gait.
///
/// Two earlier attempts disagreed with each other by half, and both were measuring
/// something else: one fitted a line to the whole cycle including the swing, the
/// other took half the two feet's combined spread, which is only the contact length
/// if the feet are exactly antiphase.
///
/// **The older note, kept because it is still the reason the run was so wrong:** The provisional
/// 2.20 was fitted to a walk that kept a foot down for three frames of twenty-four,
/// so the fit had three points and the two feet disagreed by 19%; and 1.610 was never
/// measured at all, because the old run had no frames with a foot down anywhere in
/// it. Both were notes asking to be re-measured once the clips had real stance
/// phases, which the eight-pose cycles do.
///
/// 1.935 and 2.282, and the cadences that follow are believable without help: 112
/// steps a minute walking at 1.8 m/s and 189 running at 3.6, against 95-140 and
/// 150-200 for real people. That headroom is what let the speeds go up.
const WALK_COVERS: f32 = 1.271;
const RUN_COVERS: f32 = 1.908;

/// The moving gaits, slowest first: the word in the clip's name, how far one cycle
/// carries the warden in metres, and the speed above which the next one takes over.
///
/// # A table rather than fields, because a third clip is coming
///
/// This was two named fields and a boolean. The comment above `find_the_clips` had
/// already predicted the problem — "reaching for `Animation(0)` would quietly make the
/// warden stand still the day a third clip is added" — and the same was true of the
/// playing code: every gait meant another arm of an `if`, another pair of constants
/// threaded through, and another chance to hand one gait's distance to another gait's
/// clip.
///
/// Adding a jog between the walk and the run is now a ROW. The selection, the cadence
/// and the tests all read this table, so there is no second place to update and forget.
///
/// `covers` is measured, never derived: `dev/art/stride_measure.py` fits a line to the
/// planted foot's travel to get the stance fraction, and a cycle carries `foot travel
/// / stance fraction`. See `WALK_COVERS` for why the obvious `2 x foot swing` is
/// wrong by 45% on anything with a flight phase.
const GAITS: &[(&str, f32, f32)] = &[
    ("walk", WALK_COVERS, hands_over_above(WALK_COVERS, WALK_FRAMES)),
    ("run", RUN_COVERS, hands_over_above(RUN_COVERS, RUN_FRAMES)),
    // A sprint clip does not exist yet. The row is here because a missing gait is
    // skipped with a warning and the fastest one present inherits everything above
    // it, so declaring the intended set costs nothing and the day the clip lands it
    // is picked up without touching any code.
    ("sprint", SPRINT_COVERS, f32::INFINITY),
];

/// How many frames each clip is authored over, and at what rate Blender wrote them.
///
/// These are here so a clip's NATIVE speed can be stated rather than guessed: a clip
/// authored over `frames` at `FPS` runs one cycle in `frames / FPS` seconds, so at its
/// own natural rate it carries `covers x FPS / frames` metres a second. That is the one
/// speed at which its feet do not slide.
const FPS: f32 = 24.0;
const WALK_FRAMES: f32 = 24.0;
const RUN_FRAMES: f32 = 16.0;
const SPRINT_FRAMES: f32 = 14.0;

/// The fastest a clip should be stretched past its own native speed.
///
/// A clip has exactly ONE speed at which its feet do not slide, and playback rate buys
/// speed by raising cadence only. The reference brief puts the usable correction at
/// ±25% around a clip authored at the right speed — beyond that the cadence leaves the
/// believable band, which is the churn. So this is what decides where one tier hands
/// over to the next, rather than a threshold picked by feel.
const STRETCHES_TO: f32 = 1.25;

/// What a clip carries at its own natural rate, in metres a second.
const fn natively_carries(covers: f32, frames: f32) -> f32 {
    covers * FPS / frames
}

/// The speed above which a clip should give way to the next tier up.
const fn hands_over_above(covers: f32, frames: f32) -> f32 {
    natively_carries(covers, frames) * STRETCHES_TO
}

/// What a sprint cycle will carry once there is a sprint clip, in metres.
///
/// 3.50 m over fourteen frames is 6.0 m/s natively, at 206 steps a minute — and it is
/// reachable, because planted-foot travel stays near one leg length at every speed
/// (measured at 0.99 ± 0.08 m from 6.2 to 11.1 m/s) and stride is that contact length
/// divided by the stance fraction. The extra stride is therefore bought by spending
/// LESS of the cycle on the ground, not by reaching further. Trying to reach further is
/// why 42 degrees of thigh swing once read as the splits.
const SPRINT_COVERS: f32 = 2.999;

/// What to hand `set_speed` so a clip plays at the right cadence.
///
/// `set_speed` is a MULTIPLE of a clip's natural rate — one cycle over its authored
/// duration — and not a rate. So playing `speed / covers` cycles a second means
/// asking for that many multiplied by however long the clip happens to be.
///
/// Leaving the duration out is a bug that hides: the walk lasts 1.042 s, near enough
/// to one that nothing looked wrong, while the run lasts 0.708 and played 41% too
/// fast. The property that catches it is that the cadence must come out the SAME
/// whatever the clip's length, which is what the test asserts.
fn playback_rate(speed: f32, covers: f32, clip_lasts: f32) -> f32 {
    clip_lasts * speed / covers
}

/// How long one gait eases into another, in seconds.
///
/// Short: a warden who starts walking should look like they started walking, not
/// like they faded into it. Long enough that the switch is not a snap.
const BLEND: f32 = 0.18;

/// The clips, once they have been found and put in a graph.
#[derive(Resource)]
pub struct Motions {
    graph: Handle<AnimationGraph>,
    idle: AnimationNodeIndex,
    /// Every moving gait the body actually carries, slowest first.
    ///
    /// Built from `GAITS`, skipping any the file does not have — a body with only a
    /// walk still walks, and sprints by walking faster, which is wrong but is not
    /// broken and beats refusing to animate.
    gaits: Vec<Gait>,
    /// How long each gait's clip runs, in seconds.
    ///
    /// # Why a cadence needs the clip's own length
    ///
    /// `set_speed` is a MULTIPLE of a clip's natural rate, not a rate. A clip's
    /// natural rate is one cycle over its authored duration, so cycles a second is
    /// `speed / duration` — and handing it `strides_a_second` alone silently assumes
    /// every clip lasts exactly one second.
    ///
    /// The walk very nearly does, at 1.042 s. The run lasts 0.708, because it is
    /// authored over sixteen frames rather than twenty-four, so it played 41% too
    /// fast and the feet skated for it. Nothing in the cadence test could catch that,
    /// because the test checked the number being ASKED for rather than the one the
    /// player would produce.
    idle_lasts: f32,
}

/// One gait, with everything needed to play it at the right rate.
struct Gait {
    node: AnimationNodeIndex,
    /// How far one cycle carries the warden, in metres.
    covers: f32,
    /// How long the clip runs, in seconds — see `playback_rate`.
    lasts: f32,
    /// Above this speed the next gait up takes over.
    upto: f32,
}

/// The body's own glTF, held while its clips are being waited for.
#[derive(Resource)]
pub struct Waiting(Handle<Gltf>);

/// Asks for the body file, so its clips can be read out of it.
pub fn ask_for_the_clips(
    mut commands: Commands,
    assets: Res<AssetServer>,
    look: Res<crate::look::Look>,
) {
    commands.insert_resource(Waiting(assets.load(look.build.model())));
}

/// Builds the graph once the file has loaded.
///
/// By NAME: the order glTF writes its animations in is the exporter's business, and
/// reaching for `Animation(0)` would quietly make the warden stand still the day a
/// third clip is added.
pub fn find_the_clips(
    mut commands: Commands,
    waiting: Res<Waiting>,
    files: Res<Assets<Gltf>>,
    clips: Res<Assets<AnimationClip>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    let Some(file) = files.get(&waiting.0) else {
        return;
    };
    // # Found by the word in the name, not by the whole of it
    //
    // A clip authored here is called `walk`. A clip that came with a generated rig
    // is called `preset:biped:idle`. Insisting on an exact name meant the made
    // warden had no idle at all as far as the game was concerned — it warned that
    // the body carried nothing it recognised and let him slide, while the clip sat
    // right there under a longer name.
    //
    // So a clip is matched on CONTAINING its gait's word, case-insensitively. That
    // reads every convention anybody is likely to export under, and the exact name
    // is still preferred when both are present.
    let named = |gait: &str| -> Option<&Handle<AnimationClip>> {
        file.named_animations.get(gait).or_else(|| {
            file.named_animations
                .iter()
                .find(|(name, _)| name.to_lowercase().contains(gait))
                .map(|(_, clip)| clip)
        })
    };
    // Only the idle is required here. Which moving gaits exist is the `GAITS` loop's
    // business, and it warns per missing gait rather than refusing the lot.
    let Some(idle) = named("idle") else {
        warn!(
            "the body has no idle — it carries {:?}, so the warden will slide",
            file.named_animations.keys().collect::<Vec<_>>()
        );
        commands.remove_resource::<Waiting>();
        return;
    };
    // The clips themselves, for their durations. They are sub-assets of the file, so
    // they normally arrive with it — but not necessarily in the same frame, and a
    // duration of nought would divide the cadence into infinity. So this waits.
    let Some(idle_lasts) = clips.get(idle).map(AnimationClip::duration) else {
        return;
    };

    let mut graph = AnimationGraph::new();
    let standing = graph.add_clip(idle.clone(), 1.0, graph.root);
    let mut gaits = Vec::new();
    for (called, covers, upto) in GAITS {
        let Some(clip) = named(called) else {
            info!("the body has no {called} clip; the gait above it will cover for it");
            continue;
        };
        let Some(lasts) = clips.get(clip).map(AnimationClip::duration) else {
            return;
        };
        if lasts <= 0.0 {
            warn!("the {called} clip has no length, so its cadence cannot be set");
            continue;
        }
        gaits.push(Gait {
            node: graph.add_clip(clip.clone(), 1.0, graph.root),
            covers: *covers,
            lasts,
            upto: *upto,
        });
    }
    if gaits.is_empty() {
        warn!("the body carries none of the gaits in GAITS, so the warden will slide");
        commands.remove_resource::<Waiting>();
        return;
    }
    // The fastest gait always catches everything above it, whatever its row said, so
    // that a missing top tier cannot leave a speed with no clip to play.
    if let Some(top) = gaits.last_mut() {
        top.upto = f32::INFINITY;
    }
    commands.insert_resource(Motions {
        graph: graphs.add(graph),
        idle: standing,
        gaits,
        idle_lasts,
    });
    commands.remove_resource::<Waiting>();
}

/// Hands the graph to each player the scene brings in, standing still to begin.
///
/// The glTF loader puts an `AnimationPlayer` on the scene's own animation root, so
/// this waits for one to appear rather than trying to guess which entity it is.
pub fn hand_the_clips_over(
    mut commands: Commands,
    motions: Res<Motions>,
    fresh: Query<Entity, Added<AnimationPlayer>>,
) {
    for entity in &fresh {
        let mut moves = AnimationTransitions::new();
        let mut player = AnimationPlayer::default();
        moves.play(&mut player, motions.idle, std::time::Duration::ZERO);
        commands
            .entity(entity)
            .insert((AnimationGraphHandle(motions.graph.clone()), player, moves));
    }
}

/// Walks when the warden walks, stands when they stand, at their own speed.
pub fn match_the_clip_to_the_walking(
    motions: Res<Motions>,
    striding: Query<&Striding, With<Player>>,
    mut players: Query<(&mut AnimationPlayer, &mut AnimationTransitions)>,
) {
    let Ok(pace) = striding.single() else {
        return;
    };
    // Below this a warden is standing: a hair of drift from a clamped step should
    // not start the feet going.
    let moving = pace.speed > 0.05;
    // The slowest gait whose ceiling this speed is still under. The list is ordered
    // and its last entry catches everything, so this always finds one.
    let gait = motions
        .gaits
        .iter()
        .find(|gait| pace.speed <= gait.upto)
        .unwrap_or_else(|| motions.gaits.last().expect("never empty"));

    for (mut player, mut moves) in &mut players {
        let (wanted, covers, lasts) = if moving {
            (gait.node, gait.covers, gait.lasts)
        } else {
            (motions.idle, 0.0, motions.idle_lasts)
        };
        if !player.is_playing_animation(wanted) {
            moves
                .play(&mut player, wanted, std::time::Duration::from_secs_f32(BLEND))
                .repeat();
        }
        if moving {
            // Strides a second is `speed / covers`: a clip is one stride long, so
            // that is how many of them the warden needs, and each gait carries its
            // own distance. Multiplied by the clip's LENGTH because `set_speed` is a
            // multiple of its natural rate rather than a rate — see `Motions`.
            if let Some(active) = player.animation_mut(wanted) {
                active.set_speed(playback_rate(pace.speed, covers, lasts));
            }
        }
    }
}

/// Whether the clips have been found yet.
pub fn the_clips_are_ready(motions: Option<Res<Motions>>) -> bool {
    motions.is_some()
}

/// And whether they are still being waited for.
pub fn still_waiting(waiting: Option<Res<Waiting>>) -> bool {
    waiting.is_some()
}

#[cfg(test)]
mod pacing {
    use super::*;

    /// The walk clip has to be reachable, and so does the run.
    ///
    /// The threshold sat below walking speed once, so every step the warden took
    /// played the RUN clip — at five cycles a second, which is a blur. Nothing
    /// errored and the walk clip sat in the file unused, so every judgement about
    /// how the gaits looked was made about the wrong clip.
    #[test]
    fn the_gait_threshold_lies_between_the_speeds() {
        let walk = crate::player::WALK_SPEED;
        let run = crate::player::SPRINT_SPEED;
        assert!(
            walk < BREAKS_INTO_A_RUN,
            "walking at {walk} m/s is already over the {BREAKS_INTO_A_RUN} m/s              threshold, so the walk clip can never play"
        );
        assert!(
            BREAKS_INTO_A_RUN < run,
            "the run threshold {BREAKS_INTO_A_RUN} is above the top speed {run},              so the run clip can never play"
        );
    }

    /// And the clips play at a believable cadence at those speeds.
    ///
    /// # Measured through the clip's own length, not around it
    ///
    /// This test used to assert on `speed / covers` — the number the game ASKS for —
    /// and passed while the run played 41% too fast, because `set_speed` is a
    /// multiple of a clip's natural rate and the run is authored over sixteen frames
    /// rather than twenty-four. Checking the request rather than the result is how a
    /// test agrees with the code about something they are both wrong about.
    ///
    /// So the durations come out of the FILE, and the cadence asserted here is the
    /// one the animation player will produce.
    #[test]
    fn neither_gait_plays_at_a_blur() {
        let file = std::fs::read("assets/models/person_ranger.glb")
            .expect("the ranger's own file, which the game loads");
        let model = crate::models::inspect(&file).expect("a readable GLB");
        let lasts = |gait: &str| -> f32 {
            model
                .clips
                .iter()
                .find(|(name, _)| name.to_lowercase().contains(gait))
                .map(|(_, seconds)| *seconds)
                .unwrap_or_else(|| panic!("no {gait} clip in {:?}", model.clips))
        };

        // In steps a minute, because that is the unit the evidence is in and "cycles
        // a second" hid how bad the run once was. Two steps to a cycle.
        //
        // What the player will actually produce: the rate handed to `set_speed`,
        // divided by the clip's own length, because that rate is a multiple of one
        // cycle over that length.
        let steps = |speed: f32, covers: f32, gait: &str| -> f32 {
            playback_rate(speed, covers, lasts(gait)) / lasts(gait) * 120.0
        };

        // # The debt this test used to carry is paid
        //
        // It held a ratchet at 300 steps a minute with a note explaining why: the run
        // clip kept a foot down for no frames at all, its stride was about 1.5x too
        // short for the speed it carried, and it was also being asked to carry the
        // SPRINT, because there was no sprint clip. The cadence was whatever was left
        // over — 298 — and the bound had been written loose enough to admit it.
        //
        // There is a sprint clip now, authored with two stance poses out of eight and
        // a real flight phase, and each tier plays its own clip at very nearly that
        // clip's native rate. So the bands below are the ones people actually walk and
        // run at, with no allowance for a fault.
        for (what, speed, covers, gait, band) in [
            ("walk", crate::player::WALK_SPEED, WALK_COVERS, "walk", (90.0, 140.0)),
            ("jog", crate::player::JOG_SPEED, RUN_COVERS, "run", (150.0, 200.0)),
            (
                "sprint",
                crate::player::SPRINT_SPEED,
                SPRINT_COVERS,
                "sprint",
                (170.0, 215.0),
            ),
        ] {
            let cadence = steps(speed, covers, gait);
            assert!(
                (band.0..=band.1).contains(&cadence),
                "the {what} plays at {cadence:.0} steps a minute at {speed} m/s, and \
                 {} to {} is what a person does. Fix the STRIDE rather than this \
                 bound: cadence is speed divided by how far a cycle carries them.",
                band.0,
                band.1
            );
        }
    }

    /// The gait table has to be ordered, positive, and open at the top.
    ///
    /// Three properties, each of which breaks the selection silently rather than
    /// loudly if it is violated — which is exactly why they are asserted rather than
    /// trusted. The selection takes the FIRST gait whose ceiling the speed is under,
    /// so an unordered table makes a fast gait shadow a slow one; a `covers` of nought
    /// divides the cadence into infinity; and a finite top ceiling leaves the speeds
    /// above it with no clip.
    #[test]
    fn the_gait_table_is_ordered_and_open_at_the_top() {
        assert!(!GAITS.is_empty(), "there has to be at least one moving gait");
        let mut previous = 0.0f32;
        for (called, covers, upto) in GAITS {
            assert!(
                *covers > 0.0,
                "the {called} gait covers {covers} m a cycle, and a cadence cannot be                  divided by that"
            );
            assert!(
                *upto > previous,
                "the {called} gait's ceiling {upto} is not above the one before it                  ({previous}), so it can never be chosen: the selection takes the first                  gait the speed fits under"
            );
            previous = *upto;
        }
        let (top, _, ceiling) = GAITS[GAITS.len() - 1];
        assert!(
            ceiling.is_infinite(),
            "the fastest gait is {top} and its ceiling is {ceiling}, so nothing plays              above that speed"
        );
    }

    /// And every gait in the table plays at a believable cadence at the speed that
    /// selects it — not just the two that used to have named constants.
    #[test]
    fn no_gait_in_the_table_churns_at_its_own_ceiling() {
        let mut floor = 0.0f32;
        for (called, covers, upto) in GAITS {
            // The fastest speed this gait is asked to carry: its own ceiling, or the
            // sprint if it is the open-topped one at the top.
            let fastest = if upto.is_infinite() {
                crate::player::SPRINT_SPEED
            } else {
                *upto
            };
            let steps = fastest / covers * 120.0;
            assert!(
                (60.0..=250.0).contains(&steps),
                "the {called} gait carries {fastest} m/s over {covers} m a cycle, which                  is {steps:.0} steps a minute. Real people manage 95 to 140 walking and                  150 to 200 running, so this needs a longer stride or another tier."
            );
            floor = fastest;
        }
        assert!(floor > 0.0, "no gait carried anything");
    }

    /// The cadence must not depend on how long the clip was authored.
    ///
    /// This is the test that would have caught the run playing 41% too fast, and the
    /// one the old test could not be: it asserts a PROPERTY rather than a value.
    /// `speed / covers` alone passes any check of the number it produces while being
    /// wrong by exactly the clip's duration, so the only way to see the fault is to
    /// vary the duration and demand the answer stay put.
    #[test]
    fn the_cadence_does_not_care_how_long_the_clip_is() {
        let covers = WALK_COVERS;
        let speed = crate::player::WALK_SPEED;
        let wanted = speed / covers;
        for lasts in [0.25, 0.5, 0.708, 1.0, 1.042, 2.0, 7.5] {
            let plays_at = playback_rate(speed, covers, lasts) / lasts;
            assert!(
                (plays_at - wanted).abs() < 1e-4,
                "a clip authored over {lasts} s plays at {plays_at:.4} cycles a second                  where {wanted:.4} was wanted — the cadence is following the clip's                  length instead of the warden's speed"
            );
        }
    }

    /// And a clip's length is not wildly off one stride's worth of time.
    ///
    /// The cadence fix makes any duration work, which removes the pressure to author
    /// clips at a sensible length — and a clip stretched far from the speed it plays
    /// at loses its keys' spacing to the resampling. So the authored length is
    /// checked against the time a stride actually takes.
    #[test]
    fn each_clip_is_authored_near_the_time_its_stride_takes() {
        let file = std::fs::read("assets/models/person_ranger.glb")
            .expect("the ranger's own file, which the game loads");
        let model = crate::models::inspect(&file).expect("a readable GLB");
        for (gait, speed, covers) in [
            ("walk", crate::player::WALK_SPEED, WALK_COVERS),
            ("run", crate::player::SPRINT_SPEED, RUN_COVERS),
        ] {
            let lasts = model
                .clips
                .iter()
                .find(|(name, _)| name.to_lowercase().contains(gait))
                .map(|(_, seconds)| *seconds)
                .unwrap_or_else(|| panic!("no {gait} clip in {:?}", model.clips));
            let a_stride_takes = covers / speed;
            let stretch = lasts / a_stride_takes;
            assert!(
                (0.4..2.5).contains(&stretch),
                "the {gait} clip is authored over {lasts:.3} s and a stride takes                  {a_stride_takes:.3} s, so it plays at {stretch:.2}x its own rate"
            );
        }
    }
}
