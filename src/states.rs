//! Top-level application states, and the cursor policy that goes with them.
//!
//! The game and the terrain tool are separate modes reached from the main menu,
//! rather than the tool being a mode you toggle inside the game. That keeps the
//! tool's input, UI and camera entirely its own — and keeps the seam clean if
//! it ever moves to its own crate for another project.

use bevy::prelude::*;
use bevy::window::{CursorGrabMode, PrimaryWindow};

#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AppState {
    #[default]
    Menu,
    /// Walking the world as the warden.
    Playing,
    /// Shaping the ground, in the game, on the live world.
    ///
    /// It lived here, moved out to Opificium, and came back — because that is
    /// how studios do it. Unreal's Landscape and Unity's terrain are runtime
    /// systems the editor wraps tooling around, one codebase, editor-only parts
    /// stripped from shipping builds. Sculpting in the game means no file
    /// round-trip and no second program to keep in step: you change the ground
    /// and you are standing on it.
    ///
    /// Opificium keeps its own bench for other projects. Both run `terrain-core`.
    #[cfg(feature = "tools")]
    Editing,
    /// The workbench: composing a building out of parts, away from the world.
    ///
    /// Separate from shaping the ground, deliberately. Those are different jobs at
    /// different scales — one moves a hillside, the other places a fence rail —
    /// and a tool that tried to be both would have two sets of controls fighting
    /// over the same mouse. What joins them is the placed sheet: the bench makes a
    /// building, and the terrain tool stands it somewhere.
    #[cfg(feature = "tools")]
    Bench,
}

impl AppState {
    /// Whether this state drives a first-person-style cursor grab.
    fn captures_cursor(self) -> bool {
        // Not the bench. It is a POINTING tool — you aim at a lattice cell and
        // click it — and a captured cursor is for looking around with the mouse.
        // A tool that grabs the pointer in order to place a fence rail is fighting
        // the one input the job actually wants.
        #[cfg(feature = "tools")]
        {
            matches!(self, AppState::Playing | AppState::Editing)
        }
        #[cfg(not(feature = "tools"))]
        {
            matches!(self, AppState::Playing)
        }
    }
}

pub struct StatesPlugin;

impl Plugin for StatesPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<AppState>()
            .add_systems(OnEnter(AppState::Menu), apply_cursor)
            .add_systems(OnEnter(AppState::Playing), apply_cursor)
            .add_systems(OnEnter(AppState::Playing), apply_cursor)
            // Not in the terrain tool: it guards ESC itself, because leaving
            // with an afternoon's shaping unwritten should say so first.
            .add_systems(
                Update,
                escape_to_menu.run_if(in_state(AppState::Playing)),
            );
    }
}

/// Captures the cursor for the modes that use mouse-look, releases it for the
/// menu. Driven by state transitions rather than a key, so the cursor can never
/// end up grabbed while a menu is asking to be clicked.
fn apply_cursor(state: Res<State<AppState>>, mut windows: Query<&mut Window, With<PrimaryWindow>>) {
    let Some(mut window) = windows.iter_mut().next() else {
        return;
    };
    let capture = state.get().captures_cursor();

    // `Confined` rather than `Locked`: it behaves consistently across platforms,
    // and mouse-look reads raw device motion so it keeps working at the edges.
    window.cursor_options.grab_mode = if capture {
        CursorGrabMode::Confined
    } else {
        CursorGrabMode::None
    };
    window.cursor_options.visible = !capture;
}

fn escape_to_menu(keys: Res<ButtonInput<KeyCode>>, mut next: ResMut<NextState<AppState>>) {
    if keys.just_pressed(KeyCode::Escape) {
        next.set(AppState::Menu);
    }
}
