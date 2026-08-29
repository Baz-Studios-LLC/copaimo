//! Raises the stone bridges that carry a road over water.
//!
//! # One arch, repeated
//!
//! The crossings this world needs are 668 m and 1154 m, and a fantasy world is
//! welcome to a bridge that long. Nothing models one: `dev/art/bridge.py` builds a
//! single arch with the pier under it and an abutment for each shore, and this lays
//! them end to end along the crossing. That is how a viaduct is built and the only
//! way a kilometre of bridge is affordable.
//!
//! # The deck is a surface you can walk on
//!
//! Collision in this game is walls - upright segments that refuse a step through
//! them - and there is no floor anywhere in it, because until now the floor was
//! always the terrain. A deck fourteen metres over a lake is the first surface in
//! the world that is not the ground.
//!
//! Rather than grow a second collision system for one case, the ground is asked to
//! answer differently where a bridge is: `Settlements::deck_at` reports the deck
//! under a point, and the player's footing takes the higher of that and the terrain.
//! The terrain itself is untouched, so the water still renders as water and nothing
//! grows on the deck - only what the WARDEN stands on changes, which is all a bridge
//! has to change.

use bevy::prelude::*;
use bevy::scene::SceneRoot;

use crate::world::StreamAnchor;
use crate::world::terrain::TerrainSource;

/// How long one arch is, and therefore how far apart the game lays them.
///
/// The contract with `dev/art/bridge.py`. Both numbers are written out beside the
/// models when they are built and checked against these by
/// `the_bridge_models_are_the_size_the_game_thinks_they_are`.
pub const SPAN_LONG: f32 = 18.0;

/// How far above the model's own foot its road surface sits.
///
/// `masonry::weld` stands every figure on zero, so a bridge model's origin is the
/// bottom of its pier - a long way under the water. This is what gets subtracted
/// from the crossing's deck height to place one.
pub const DECK_ABOVE_FOOT: f32 = 24.0;

/// The roadway between the parapets, in metres. What counts as being ON the bridge.
pub const ROADWAY_WIDE: f32 = 6.4;

/// How far from the player bridges are raised, in metres.
///
/// Generous, because a bridge is a landmark: seeing one across the water is most of
/// what tells you there is a way over, and a bridge that fades in at fifty metres is
/// a bridge you never set out for.
const RAISES_WITHIN: f32 = 2_400.0;

/// A piece of bridge that is standing, so it can be taken down again.
#[derive(Component)]
struct Standing;

/// The coarse cell the bridges were last raised for.
#[derive(Resource, Default)]
pub struct Raised {
    cell: Option<IVec2>,
}

/// Lays every bridge near the player, and takes down the ones left behind.
///
/// Keyed on a coarse cell rather than rebuilt per frame: a kilometre of bridge is
/// seventy-odd models, and seventy models respawned every frame is a stutter and a
/// flicker rather than a bridge.
fn raise_the_bridges(
    mut commands: Commands,
    assets: Res<AssetServer>,
    terrain: Res<TerrainSource>,
    mut raised: ResMut<Raised>,
    anchors: Query<&GlobalTransform, With<StreamAnchor>>,
    standing: Query<Entity, With<Standing>>,
) {
    let Some(anchor) = anchors.iter().next() else {
        return;
    };
    let here = Vec2::new(anchor.translation().x, anchor.translation().z);
    let cell = (here / (RAISES_WITHIN * 0.5)).floor().as_ivec2();
    if raised.cell == Some(cell) {
        return;
    }
    raised.cell = Some(cell);

    for entity in &standing {
        commands.entity(entity).despawn();
    }

    for bridge in terrain.plan().spans() {
        let run = bridge.to - bridge.from;
        let length = run.length();
        if length < 1.0 {
            continue;
        }
        // Near enough to matter, measured against the whole span rather than its
        // middle: standing at one end of a kilometre of bridge, the middle is far
        // away and the bridge is right in front of you.
        let along = ((here - bridge.from).dot(run) / (length * length)).clamp(0.0, 1.0);
        if here.distance(bridge.from + run * along) > RAISES_WITHIN {
            continue;
        }

        // The models are built running along +X, so the turn is the bearing of the
        // crossing. Bevy's Y rotation runs the other way round from the atan2 of a
        // world-space direction, hence the sign.
        let turn = -run.y.atan2(run.x);
        let foot = bridge.deck - DECK_ABOVE_FOOT;
        let ahead = run / length;

        // An abutment at each shore, then arches filling everything between them.
        let ends = SPAN_LONG * 0.5;
        for shore in [0.0_f32, 1.0] {
            let at = bridge.from + ahead * (length * shore + ends * 0.5 * (1.0 - 2.0 * shore));
            commands.spawn((
                Standing,
                SceneRoot(
                    assets.load(GltfAssetLabel::Scene(0).from_asset("models/bridge_end.glb")),
                ),
                Transform::from_xyz(at.x, foot, at.y)
                    .with_rotation(Quat::from_rotation_y(turn)),
                Visibility::default(),
            ));
        }

        let between = (length - ends).max(0.0);
        let arches = (between / SPAN_LONG).floor().max(1.0) as usize;
        // Spread across what is actually left, so the last arch meets the far
        // abutment instead of stopping short of it or running past into the shore.
        let step = between / arches as f32;
        for arch in 0..arches {
            let at = bridge.from + ahead * (ends * 0.5 + (arch as f32 + 0.5) * step);
            commands.spawn((
                Standing,
                SceneRoot(
                    assets.load(GltfAssetLabel::Scene(0).from_asset("models/bridge_span.glb")),
                ),
                Transform::from_xyz(at.x, foot, at.y)
                    .with_rotation(Quat::from_rotation_y(turn)),
                Visibility::default(),
            ));
        }
        info!(
            "raising a bridge at ({:.0}, {:.0}): {:.0} m of it, {arches} arches",
            bridge.from.x, bridge.from.y, length,
        );
    }
}

pub struct BridgePlugin;

impl Plugin for BridgePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Raised>().add_systems(
            Update,
            raise_the_bridges.run_if(crate::build::a_world_is_up),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The game and Blender agree about how big a bridge is.
    ///
    /// `dev/art/bridge.py` writes what it built beside the models. If somebody
    /// changes an arch's span there and not here, the game lays them at the wrong
    /// spacing - which shows up as gaps between arches, or as every second pier
    /// buried in its neighbour, and in neither case as an error.
    #[test]
    fn the_bridge_models_are_the_size_the_game_thinks_they_are() {
        let note = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("assets/models/bridge.txt");
        let said = std::fs::read_to_string(&note)
            .unwrap_or_else(|_| panic!("run dev/art/build.sh: {} is missing", note.display()));

        let read = |key: &str| -> f32 {
            said.lines()
                .find_map(|line| line.strip_prefix(&format!("{key} ")))
                .unwrap_or_else(|| panic!("{key} is not in {}", note.display()))
                .trim()
                .parse()
                .expect("a number")
        };

        assert!(
            (read("SPAN_LONG") - SPAN_LONG).abs() < 1.0e-3,
            "Blender builds an arch {:.2} m long and the game lays them {SPAN_LONG:.2} m apart",
            read("SPAN_LONG"),
        );
        assert!(
            (read("DECK_ABOVE_FOOT") - DECK_ABOVE_FOOT).abs() < 1.0e-3,
            "Blender puts the deck {:.2} m above the foot and the game drops it {DECK_ABOVE_FOOT:.2} m",
            read("DECK_ABOVE_FOOT"),
        );
        assert!(
            (read("ROADWAY_WIDE") - ROADWAY_WIDE).abs() < 1.0e-3,
            "the roadway is {:.2} m wide and the game walks {ROADWAY_WIDE:.2} m of it",
            read("ROADWAY_WIDE"),
        );
    }
}
