//! Ranger — a monster-companion adventure game.
//!
//! Stage one is the world itself: a large, finite, walkable landmass generated
//! from a source map image. Ranching, monsters, cities and guild exams all sit
//! on top of this later, so the priority here is a world with real geography —
//! coastlines, mountain ranges, biomes — at a scale that feels like a journey.
//!
//! The main menu leads to two separate modes: walking the world as the ranger,
//! and the terrain tool for sculpting its shape.
//!
//! Each concern is a Bevy plugin in its own module:
//!   * `states` — app states (menu / playing / editing) and cursor policy
//!   * `world`  — terrain generation, chunk streaming, the sea
//!   * `player` — the ranger and their character controller
//!   * `camera` — third-person orbit rig, plus free-fly
//!   * `menu`   — the main menu
//!
//! The terrain is *sculpted* in Opificium, the studio's maker's bench, not here
//! — the game only reads what that writes. See `DESIGN.md`.
//!   * `sky`    — sun, ambient light, fog
//!   * `hud`    — the F3 debug overlay

mod camera;
mod config;
mod hud;
mod menu;
mod player;
mod sky;
mod states;
mod util;
mod world;

use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Ranger — World Prototype".into(),
                resolution: (1600.0_f32, 900.0_f32).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .add_plugins((
            states::StatesPlugin,
            // World first so the terrain resource exists before anything that
            // needs to ask how high the ground is.
            world::WorldPlugin,
            sky::SkyPlugin,
            player::PlayerPlugin,
            camera::CameraPlugin,
            menu::MenuPlugin,
            hud::HudPlugin,
        ))
        .run();
}
