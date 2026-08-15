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
//!   * `editor` — the terrain tool, driving `terrain-core`'s brush
//!   * `build`  — buildings baked at Opificium's builder, stood on the ground
//!   * `sky`    — sun, ambient light, fog
//!   * `hud`    — the F3 debug overlay
//!
//! The world generation, the brush and the trees all live in `terrain-core`,
//! which Opificium's terrain bench links too — so ground shaped in either place
//! is shaped identically. See `DESIGN.md`.

// Two clippy lints fire on the shape of Bevy itself rather than on anything
// wrong here, and contorting the code to quiet them would cost more than they
// are worth. A system's parameters ARE its dependencies, so a system that
// touches eight things takes eight arguments; and a filtered `Query` is a type
// by construction, so naming an alias for each one buries what it selects.
#![allow(clippy::too_many_arguments, clippy::type_complexity)]

mod build;
mod camera;
mod config;
mod editor;
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
            // After the world: buildings stand on ground it decides the height
            // of, at sites it decides the places of.
            build::BuildingPlugin,
            player::PlayerPlugin,
            camera::CameraPlugin,
            menu::MenuPlugin,
            editor::EditorPlugin,
            hud::HudPlugin,
        ))
        .run();
}
