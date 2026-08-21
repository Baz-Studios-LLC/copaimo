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

/// How far one stride of the `walk` clip carries the warden, in metres.
///
/// Measured from the clip: the legs swing `STRIDE` degrees either side of the hip,
/// which at this leg length is about this far. It is the number to correct if the
/// feet appear to skate — raise it if they slip forward, lower it if they scuff.
const STRIDE_COVERS: f32 = 1.35;

/// How long a blend between standing and walking takes, in seconds.
///
/// Short. A quarter of a second reads as the warden changing their mind; anything
/// longer reads as the animation catching up with the game.
const BLEND: f32 = 0.18;

/// The clips, once they have been found and put in a graph.
#[derive(Resource)]
pub struct Motions {
    graph: Handle<AnimationGraph>,
    walk: AnimationNodeIndex,
    idle: AnimationNodeIndex,
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
    let (Some(walk), Some(idle)) = (
        file.named_animations.get("walk"),
        file.named_animations.get("idle"),
    ) else {
        warn!(
            "the body has no `walk` and `idle` — it carries {:?}, so the warden will slide",
            file.named_animations.keys().collect::<Vec<_>>()
        );
        commands.remove_resource::<Waiting>();
        return;
    };
    let mut graph = AnimationGraph::new();
    let walk = graph.add_clip(walk.clone(), 1.0, graph.root);
    let idle = graph.add_clip(idle.clone(), 1.0, graph.root);
    commands.insert_resource(Motions {
        graph: graphs.add(graph),
        walk,
        idle,
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
    let walking = pace.speed > 0.05;

    for (mut player, mut moves) in &mut players {
        let wanted = if walking { motions.walk } else { motions.idle };
        if !player.is_playing_animation(wanted) {
            moves
                .play(&mut player, wanted, std::time::Duration::from_secs_f32(BLEND))
                .repeat();
        }
        if walking {
            // Strides a second, which is what stops the feet skating: the clip is
            // one stride long, so its speed IS how many of them the warden needs.
            if let Some(active) = player.animation_mut(motions.walk) {
                active.set_speed(pace.speed / STRIDE_COVERS);
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
