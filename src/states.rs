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
            .add_systems(Startup, open_where_asked)
            .add_systems(OnEnter(AppState::Menu), apply_cursor)
            .add_systems(OnEnter(AppState::Playing), apply_cursor);
        // The terrain tool captures the cursor like the world does — and this
        // line was MISSING (its slot held a duplicate of the Playing line), so
        // entering the tool inherited the menu's free, visible pointer. That
        // white OS arrow wandering the screen while the brush aimed down the
        // view centre was the "brush circle and cursor are offset": two
        // pointers, one of which meant nothing.
        #[cfg(feature = "tools")]
        app.add_systems(OnEnter(AppState::Editing), apply_cursor);
        // Not in the terrain tool: it guards ESC itself, because leaving
        // with an afternoon's shaping unwritten should say so first.
        app.add_systems(
            Update,
            escape_to_menu.run_if(in_state(AppState::Playing)),
        );
    }
}

/// Captures the cursor for the modes that use mouse-look, releases it for the
/// menu. Driven by state transitions rather than a key, so the cursor can never
/// end up grabbed while a menu is asking to be clicked.
/// Opens straight into a state, when asked from outside.
///
/// # Why this exists rather than a line I keep editing
///
/// Testing the workbench meant reaching into this file, adding a Startup system
/// that jumps to it, building, looking, and taking the line out again. That worked
/// until the day I forgot the last step — and the game shipped booting into the
/// workbench, on main, because a temporary edit is only temporary if somebody
/// remembers.
///
/// So it is a switch from outside the source. `COPAIMO_OPEN=bench` opens the
/// bench, `=edit` the terrain tool, `=play` the world; anything else, or nothing,
/// opens the title screen as a player would see it. Nothing to undo, and nothing
/// to forget.
///
/// Behind the tools feature, so a release cannot be talked into it at all.
fn open_where_asked(mut next: ResMut<NextState<AppState>>) {
    #[cfg(feature = "tools")]
    if let Ok(asked) = std::env::var("COPAIMO_OPEN") {
        let opening = match asked.trim().to_ascii_lowercase().as_str() {
            "bench" | "workbench" => Some(AppState::Bench),
            "edit" | "editor" | "terrain" => Some(AppState::Editing),
            "play" | "playing" | "world" => Some(AppState::Playing),
            _ => None,
        };
        if let Some(opening) = opening {
            info!("COPAIMO_OPEN={asked}: opening {opening:?}");
            next.set(opening);
            return;
        }
        warn!("COPAIMO_OPEN={asked} is not a thing to open; showing the title");
    }
    let _ = &mut next;
}

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
