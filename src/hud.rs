//! The maker's overlay: frame rate, position, what the world thinks is here.
//!
//! # Off unless asked for, and not in a release at all
//!
//! It used to open showing, which meant the game began behind a wall of numbers —
//! chunk counts and sculpted-cell tallies are things a maker wants on demand and a
//! player should never see at all. `F3` brings it up.
//!
//! And the whole module is compiled out of a release, like the terrain tool and
//! the workbench. A player's build has no debug overlay to leave switched on by
//! accident.
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
        // OFF to begin with. A tool that opens over the game is a tool that has
        // decided the numbers matter more than the thing they describe.
        app.insert_resource(HudEnabled(false))
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
    enabled: Res<HudEnabled>,
    diagnostics: Res<DiagnosticsStore>,
    mode: Res<CameraMode>,
    terrain: Res<TerrainSource>,
    bounds: Res<WorldBounds>,
    when: Res<crate::sky::TimeOfDay>,
    year: Res<crate::season::TheYear>,
    chunks: Res<ChunkMap>,
    pending: Query<&PendingChunk>,
    players: Query<&Transform, With<Player>>,
    mut hud: Query<&mut Text, With<HudText>>,
) {
    // Hidden is hidden: with the overlay off this was still sampling the
    // terrain six ways, formatting strings and re-laying-out glyphs every
    // frame, for text nobody could see.
    if !enabled.0 {
        return;
    }
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

    // Where you are ON THE MAP, and which region has claimed it.
    //
    // The regions in `terrain_core::region` are written in normalised map
    // coordinates — 0,0 north-west, 1,1 south-east — because they are read off a
    // picture of the world with the areas drawn on it. Without this, working out
    // why somewhere came out green means guessing at a marker's position on the
    // overview, which is a guess per attempt and a round trip per guess.
    //
    // With it, standing on the spot reads the answer straight off: `map 0.71,
    // 0.14` says which ellipse to move, and `desert (0.20 of it)` says whether
    // the trouble is the placement or the falloff — a zone whose rim reaches
    // somewhere still leaves it a half-hearted version of that country, which has
    // been the cause every single time so far.
    let (u, v) = terrain.map_uv(position.x, position.z);
    let (country, belonging) = terrain.region(position.x, position.z);

    **text = format!(
        "{fps:.0} fps   camera: {mode}\n\
         world: {:.0} x {:.0} m   source: {source}\n\
         position: {:.0}, {:.0}\n\
         altitude: {height:.1} m   slope: {slope:.2}\n\
         here: {} ({sure:.2} sure)   time: {}\n\
         season: {}\n\
         map: {u:.3}, {v:.3}   country: {} ({belonging:.2} of it)\n\
         chunks: {} loaded, {} building   sculpted: {sculpted}\n\
         nearest: {nearest}\n\
         \n\
         WASD move · Ctrl walk · mouse look · wheel zoom\n\
         F free-fly (Q/E down-up, -/= speed) · F3 hide\n\
         F6/F7 hour back-forward · F8 back to real time\n\
         F9/F10 season back-forward · F11 back to the real date\n\
         Esc back to menu",
        WORLD_WIDTH,
        bounds.half.y * 2.0,
        position.x,
        position.z,
        biome.name(),
        when.spoken(),
        year.spoken(),
        country.name(),
        chunks.loaded.len(),
        pending.iter().count(),
    );
}
