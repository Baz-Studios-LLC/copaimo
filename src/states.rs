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
    /// Walking the world as the ranger.
    ///
    /// The only mode there is. There was a third — the terrain sculpting tool —
    /// and it moved out to Opificium, the studio's maker's bench. The game reads
    /// the ground that writes; it does not shape it. See `DESIGN.md`.
    Playing,
}

impl AppState {
    /// Whether this state drives a first-person-style cursor grab.
    fn captures_cursor(self) -> bool {
        matches!(self, AppState::Playing)
    }
}

pub struct StatesPlugin;

impl Plugin for StatesPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<AppState>()
            .add_systems(OnEnter(AppState::Menu), apply_cursor)
            .add_systems(OnEnter(AppState::Playing), apply_cursor)
            .add_systems(Update, escape_to_menu.run_if(not(in_state(AppState::Menu))));
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
