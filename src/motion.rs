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
const BREAKS_INTO_A_RUN: f32 = 3.0;

/// How far one cycle of each clip carries the warden, in metres.
///
/// # Measured off the clips, not worked out from angles
///
/// These were estimated as `2 * leg * sin(stride angle)`, giving 1.35 and 1.14, and
/// both were wrong. `dev/art/stride_measure.py` poses the real rig over the real
/// clip and measures how far a foot travels front-to-back relative to the hips —
/// which is the ground the character actually covers.
///
/// A foot swings 0.451 units in the walk and 0.478 in the run, on a model authored
/// one unit tall and scaled to 1.7 m. A CYCLE is both feet taking one step, so the
/// body advances by twice that: 1.519 m walking, 1.610 m running.
///
/// Getting the factor of two wrong is the difference between a believable cadence
/// and a blur, and the cadence test below is what caught it.
const STRIDE_COVERS: f32 = 1.519;
const RUN_COVERS: f32 = 1.610;

/// How long one gait eases into another, in seconds.
///
/// Short: a warden who starts walking should look like they started walking, not
/// like they faded into it. Long enough that the switch is not a snap.
const BLEND: f32 = 0.18;

/// The clips, once they have been found and put in a graph.
#[derive(Resource)]
pub struct Motions {
    graph: Handle<AnimationGraph>,
    walk: AnimationNodeIndex,
    idle: AnimationNodeIndex,
    /// A run, if the body has one.
    run: Option<AnimationNodeIndex>,
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
    let (Some(walk), Some(idle)) = (named("walk"), named("idle")) else {
        warn!(
            "the body has no walk and idle — it carries {:?}, so the warden will slide",
            file.named_animations.keys().collect::<Vec<_>>()
        );
        commands.remove_resource::<Waiting>();
        return;
    };
    let mut graph = AnimationGraph::new();
    let walking = graph.add_clip(walk.clone(), 1.0, graph.root);
    let standing = graph.add_clip(idle.clone(), 1.0, graph.root);
    // A run is optional: a body with only a walk sprints by walking faster, which
    // is wrong but is not broken, and is better than refusing to animate at all.
    let running = named("run").map(|run| graph.add_clip(run.clone(), 1.0, graph.root));
    if running.is_none() {
        info!("the body has no run clip; sprinting will play the walk quicker");
    }
    commands.insert_resource(Motions {
        graph: graphs.add(graph),
        walk: walking,
        idle: standing,
        run: running,
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
    let running = moving && pace.speed > BREAKS_INTO_A_RUN && motions.run.is_some();

    for (mut player, mut moves) in &mut players {
        let (wanted, covers) = if running {
            (motions.run.expect("checked"), RUN_COVERS)
        } else if moving {
            (motions.walk, STRIDE_COVERS)
        } else {
            (motions.idle, 0.0)
        };
        if !player.is_playing_animation(wanted) {
            moves
                .play(&mut player, wanted, std::time::Duration::from_secs_f32(BLEND))
                .repeat();
        }
        if moving {
            // Strides a second, which is what stops the feet skating: a clip is one
            // stride long, so its speed IS how many of them the warden needs. Each
            // gait carries a different distance, so each is divided by its own.
            if let Some(active) = player.animation_mut(wanted) {
                active.set_speed(pace.speed / covers);
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
    /// A clip is one stride, so its playback rate is strides a second. A person
    /// walks at roughly one stride a second and runs at about one and a half; much
    /// past two and the legs are a blur whatever the clip contains.
    #[test]
    fn neither_gait_plays_at_a_blur() {
        let walking = crate::player::WALK_SPEED / STRIDE_COVERS;
        let running = crate::player::SPRINT_SPEED / RUN_COVERS;
        // A person walks at about one cycle a second and runs at about one and a
        // half. Past two and a half the legs are a blur whatever the clip holds.
        assert!(
            (0.6..1.6).contains(&walking),
            "the walk plays at {walking:.2} cycles a second"
        );
        assert!(
            (1.0..2.5).contains(&running),
            "the run plays at {running:.2} cycles a second"
        );
    }
}
