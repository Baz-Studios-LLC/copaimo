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
/// **Both are now measured off clips that have a stance to measure.** The provisional
/// 2.20 was fitted to a walk that kept a foot down for three frames of twenty-four,
/// so the fit had three points and the two feet disagreed by 19%; and 1.610 was never
/// measured at all, because the old run had no frames with a foot down anywhere in
/// it. Both were notes asking to be re-measured once the clips had real stance
/// phases, which the eight-pose cycles do.
///
/// 1.935 and 2.282, and the cadences that follow are believable without help: 112
/// steps a minute walking at 1.8 m/s and 189 running at 3.6, against 95-140 and
/// 150-200 for real people. That headroom is what let the speeds go up.
const STRIDE_COVERS: f32 = 1.935;
const RUN_COVERS: f32 = 2.282;

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
/// / stance fraction`. See `STRIDE_COVERS` for why the obvious `2 x foot swing` is
/// wrong by 45% on anything with a flight phase.
const GAITS: &[(&str, f32, f32)] = &[
    ("walk", STRIDE_COVERS, BREAKS_INTO_A_RUN),
    ("run", RUN_COVERS, f32::INFINITY),
];

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

        // What the player will actually produce: the rate handed to `set_speed`,
        // divided by the clip's own length, because that rate is a multiple of one
        // cycle over that length.
        let plays_at = |speed: f32, covers: f32, gait: &str| -> f32 {
            playback_rate(speed, covers, lasts(gait)) / lasts(gait)
        };
        // In steps a minute, because that is the unit the evidence is in and
        // "cycles a second" hid how bad the run was. Two steps to a cycle.
        let steps = |cycles: f32| cycles * 2.0 * 60.0;
        let walking = steps(plays_at(crate::player::WALK_SPEED, STRIDE_COVERS, "walk"));
        let running = steps(plays_at(crate::player::SPRINT_SPEED, RUN_COVERS, "run"));

        // Walking sits at 100 to 115 steps a minute, and 140 is itself the
        // walk-to-run transition — past it a walk cycle is being asked to do a run's
        // job.
        assert!(
            (90.0..140.0).contains(&walking),
            "the walk plays at {walking:.0} steps a minute, and 90 to 140 is a walk"
        );

        // # A ratchet, not a blessing
        //
        // Recreational running is 150 to 180 steps a minute and elites are 180 plus.
        // The run plays at 298, and this test used to PASS it, because the bound was
        // written as "under 2.5 cycles a second" — which is 300 steps a minute, above
        // anything a person does. A bound loose enough to admit the fault is not a
        // test of it, and stating the unit wrong is what let it look reasonable.
        //
        // The cause is not this number. The run clip keeps a foot down for no frames
        // at all, and its stride is about 1.5x too short for the speed it carries, so
        // the cadence is whatever is left over. Lowering `SPRINT_SPEED` until this
        // passed honestly would mean 2.7 m/s — slower than the sprint has ever been,
        // to hide a fault in the clip.
        //
        // So the ceiling sits where the value stands, and its job is to stop it
        // getting WORSE. It comes down when the run is re-authored with a real stance
        // and a flight phase, and this comment is the record of that debt.
        const RUN_CHURNS_AT: f32 = 300.0;
        assert!(
            running <= RUN_CHURNS_AT,
            "the run plays at {running:.0} steps a minute, past the {RUN_CHURNS_AT:.0}              it is allowed while its stride is too short. A person runs at 150 to 180.              Give the run a longer stride rather than raising this."
        );
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
        let covers = STRIDE_COVERS;
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
            ("walk", crate::player::WALK_SPEED, STRIDE_COVERS),
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
