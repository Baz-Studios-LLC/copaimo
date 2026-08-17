//! Debug overlay (F3) for the game.
//!
//! Purely a development tool, but an important one right now: tuning a world
//! this size means being able to say "the coast at −2400, 900 is a cliff, not a
//! beach" instead of "it looks a bit steep". The terrain tool has its own panel;
//! this one is for walking the world.

use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;

use crate::camera::CameraMode;
use crate::config::WORLD_WIDTH;
use crate::player::Player;
use crate::states::AppState;
use crate::world::stream::{ChunkMap, PendingChunk};
use crate::world::terrain::TerrainSource;
use crate::world::WorldBounds;

#[derive(Component)]
struct HudText;

/// Whether the player has the overlay switched on. Kept separate from whether
/// it's currently drawn, which also depends on being in the game at all.
#[derive(Resource)]
struct HudEnabled(bool);

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(HudEnabled(true))
            .add_systems(Startup, spawn_hud)
            .add_systems(
                Update,
                (
                    // Visibility tracks the state as well as the toggle, so the
                    // overlay doesn't linger over the menu or the tool.
                    sync_visibility,
                    (toggle_hud, update_hud).run_if(in_state(AppState::Playing)),
                ),
            );
    }
}

fn spawn_hud(mut commands: Commands) {
    commands.spawn((
        HudText,
        Text::new(""),
        TextFont {
            font_size: 15.0,
            ..default()
        },
        TextColor(Color::srgb(0.95, 0.97, 1.0)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));
}

fn toggle_hud(keys: Res<ButtonInput<KeyCode>>, mut enabled: ResMut<HudEnabled>) {
    if keys.just_pressed(KeyCode::F3) {
        enabled.0 = !enabled.0;
    }
}

fn sync_visibility(
    enabled: Res<HudEnabled>,
    state: Res<State<AppState>>,
    mut hud: Query<&mut Visibility, With<HudText>>,
) {
    let show = enabled.0 && *state.get() == AppState::Playing;
    for mut visibility in &mut hud {
        *visibility = if show {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

fn update_hud(
    diagnostics: Res<DiagnosticsStore>,
    mode: Res<CameraMode>,
    terrain: Res<TerrainSource>,
    bounds: Res<WorldBounds>,
    chunks: Res<ChunkMap>,
    pending: Query<&PendingChunk>,
    players: Query<&Transform, With<Player>>,
    mut hud: Query<&mut Text, With<HudText>>,
) {
    let Some(mut text) = hud.iter_mut().next() else {
        return;
    };

    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .unwrap_or_default();

    let position = players
        .iter()
        .next()
        .map(|t| t.translation)
        .unwrap_or_default();
    let height = terrain.height(position.x, position.z);
    let moisture = terrain.moisture(position.x, position.z);
    let slope = 1.0 - terrain.normal(position.x, position.z, 1.0).y;

    let source = if terrain.has_map() {
        "map image"
    } else {
        "procedural fallback"
    };
    // Whether the ground sculpted at Opificium's terrain bench actually loaded.
    // Worth a line: a mismatched or missing edits.bin is refused rather than
    // applied, and without this the only sign is one line in the startup log.
    // The nearest place with level ground waiting for it. Navigation aid for
    // now, and the thing to walk toward once settlements are actually built.
    let here = Vec2::new(position.x, position.z);
    let nearest = terrain
        .sites()
        .iter()
        .map(|site| (site.at.distance(here), site))
        .min_by(|a, b| a.0.total_cmp(&b.0));
    let nearest = match nearest {
        Some((away, site)) => format!(
            "{} {:.0} m",
            if site.city { "city" } else { "town" },
            away
        ),
        None => "none planned".to_string(),
    };

    let sculpted = match terrain.sculpted_cells() {
        0 => "none".to_string(),
        cells => format!("{cells} cells"),
    };
    let mode = match *mode {
        CameraMode::Follow => "follow",
        CameraMode::Fly => "free-fly (F)",
    };

    // What kind of place this is, and how sure. The confidence is the useful
    // half while tuning a climate: standing somewhere that says "Desert 0.08"
    // means the boundary is right there, which is what you want to know when
    // deciding whether the deserts are big enough to travel to.
    let biome = terrain.biome(position.x, position.z);
    let sure = terrain.biome_confidence(position.x, position.z);

    **text = format!(
        "{fps:.0} fps   camera: {mode}\n\
         world: {:.0} x {:.0} m   source: {source}\n\
         position: {:.0}, {:.0}\n\
         altitude: {height:.1} m   slope: {slope:.2}   moisture: {moisture:.2}\n\
         here: {} ({sure:.2} sure)\n\
         chunks: {} loaded, {} building   sculpted: {sculpted}\n\
         nearest: {nearest}\n\
         \n\
         WASD move · Shift sprint · mouse look · wheel zoom\n\
         F free-fly (Q/E down-up, -/= speed) · F3 hide\n\
         Esc back to menu",
        WORLD_WIDTH,
        bounds.half.y * 2.0,
        position.x,
        position.z,
        biome.name(),
        chunks.loaded.len(),
        pending.iter().count(),
    );
}
