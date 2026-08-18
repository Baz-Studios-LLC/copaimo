//! The title screen.
//!
//! Drawn over the live 3D world rather than a flat backdrop — the world is
//! already streaming, and a view of it behind the title is a free look at what
//! you are about to walk into.
//!
//! # It fills the screen
//!
//! It used to be a small panel floating in the middle of a blue void, which is
//! what a debug menu looks like. A title screen is the first thing anybody sees
//! and the only part of a game everybody sees, so it covers the window: the art
//! large and high, the choices under it, and a wash over the world behind so the
//! type stays readable whatever the warden happens to be standing in front of.

use bevy::app::AppExit;
use bevy::prelude::*;

use crate::states::AppState;
use crate::typeface::UiFont;

// Taken off the logo rather than picked to taste. It is deep navy going to
// violet, with everything raised on it in silver — so the choices under it are
// the same navy, the same silver, and the same violet where they light up. A
// title screen whose buttons do not belong to its own logo reads as two designs.
const IDLE: Color = Color::srgb(0.075, 0.105, 0.185);
const HOVER: Color = Color::srgb(0.150, 0.170, 0.330);
const PRESSED: Color = Color::srgb(0.230, 0.230, 0.450);

/// The silver the wordmark is cut from, and the dimmer one under it.
const SILVER: Color = Color::srgb(0.90, 0.93, 0.97);
const SILVER_DIM: Color = Color::srgb(0.62, 0.69, 0.82);

/// The edge on a choice. Thin and bright, like the bevel round every letter.
const EDGE: Color = Color::srgba(0.62, 0.70, 0.88, 0.45);
const EDGE_LIT: Color = Color::srgb(0.78, 0.82, 0.98);

#[derive(Component)]
struct MenuRoot;

#[derive(Component, Clone, Copy, PartialEq)]
enum MenuAction {
    /// Start again from the ranch, throwing away whatever was saved.
    NewGame,
    /// Pick up where the save left off. Only offered when there is one.
    Continue,
    /// The maker's tools. Gone entirely from a release, rather than greyed out —
    /// a button that says "not for you" is worse than no button.
    #[cfg(feature = "tools")]
    Edit,
    #[cfg(feature = "tools")]
    Bench,
    Quit,
}

/// The title art, and how wide it is drawn.
///
/// A path rather than a `Handle`, because the menu is spawned and despawned every
/// time it opens and the asset server hands back the same handle each time — there
/// is nothing to cache and nothing to keep alive between visits.
const TITLE_ART: &str = "Title/Copaimo.png";

/// How much of the window's width the art takes.
///
/// A share rather than a number of pixels, so the title is the same size relative
/// to the screen on a laptop and on a monitor. Fixed at 560 px it was a postage
/// stamp on one and most of the width on the other.
const TITLE_SHARE: f32 = 46.0;

/// The field the title stands on.
///
/// OPAQUE, and that is the fix for a fault rather than a preference. This was a
/// wash over the live world on the idea that a look at the world is a free
/// backdrop — but the world does not stream until the game starts, so what showed
/// through was the empty sky with a single cloud drifting across it. A stray
/// object floating on a flat blue field is the sort of thing that reads as a bug
/// because it IS one.
///
/// A title screen should be composed rather than whatever happens to be behind
/// it. When there is real art to put back there it goes here deliberately.
const FIELD: Color = Color::srgb(0.055, 0.075, 0.135);

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Menu), spawn_menu)
            .add_systems(OnExit(AppState::Menu), despawn_menu)
            .add_systems(Update, menu_buttons.run_if(in_state(AppState::Menu)));
    }
}

fn spawn_menu(mut commands: Commands, asset_server: Res<AssetServer>, font: Res<UiFont>) {
    // Asked here rather than held in a resource: a player can delete a save from
    // under a running game, and a Continue that then does nothing is worse than
    // no Continue at all.
    let saved = crate::save::read();

    commands
        .spawn((
            MenuRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                // The art high and the choices under it, rather than everything
                // stacked dead centre. A title screen is read top to bottom.
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(6.0),
                padding: UiRect::vertical(Val::Percent(4.0)),
                ..default()
            },
            BackgroundColor(FIELD),
        ))
        .with_children(|root| {
            // The title ART, not the word set in whatever font happened to load.
            // The logo carries the subtitle, the crest and the whole look of the
            // thing; a game whose own name is typed out on its front screen looks
            // like a placeholder because it is one.
            //
            // Sized by WIDTH with the height left to work itself out, so the logo
            // keeps its proportions whatever the window is. Giving both would
            // squash it, and a squashed logo is worse than no logo.
            root.spawn((
                ImageNode::new(asset_server.load(TITLE_ART)),
                Node {
                    width: Val::Percent(TITLE_SHARE),
                    margin: UiRect::bottom(Val::Vh(4.0)),
                    ..default()
                },
            ));

            root.spawn((
                Node {
                    width: Val::Px(360.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Stretch,
                    row_gap: Val::Px(8.0),
                    ..default()
                },
                BackgroundColor(Color::NONE),
            ))
            .with_children(|choices| {
                // Continue FIRST, and only when there is one.
                //
                // First because it is what somebody returning wants, which is most
                // openings after the first — and putting New Game above it is how a
                // player throws away an evening by pressing the top button out of
                // habit.
                if let Some(save) = &saved {
                    let when = if save.stamped.is_empty() {
                        "carry on where you left off".to_string()
                    } else {
                        format!("{} - {:.0} min played", save.stamped, save.played / 60.0)
                    };
                    spawn_button(choices, &font, MenuAction::Continue, "Continue", &when);
                }
                spawn_button(
                    choices,
                    &font,
                    MenuAction::NewGame,
                    "New Game",
                    if saved.is_some() {
                        "start again - this replaces the save"
                    } else {
                        "begin at the ranch"
                    },
                );

                #[cfg(feature = "tools")]
                {
                    // The maker's tools, set apart from the game's own choices so
                    // the two are not read as one list. Gone entirely from a
                    // release rather than greyed out.
                    choices.spawn((
                        Node {
                            height: Val::Px(1.0),
                            margin: UiRect::vertical(Val::Px(10.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.45, 0.55, 0.70, 0.25)),
                    ));
                    spawn_button(
                        choices,
                        &font,
                        MenuAction::Edit,
                        "Shape the World",
                        "sculpt the ground you walk on",
                    );
                    spawn_button(
                        choices,
                        &font,
                        MenuAction::Bench,
                        "Workbench",
                        "build houses and fences, piece by piece",
                    );
                }

                spawn_button(choices, &font, MenuAction::Quit, "Exit", "");
            });
        });
}

fn spawn_button(
    parent: &mut ChildSpawnerCommands,
    font: &UiFont,
    action: MenuAction,
    label: &str,
    hint: &str,
) {
    parent
        .spawn((
            action,
            Button,
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::axes(Val::Px(20.0), Val::Px(13.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(3.0),
                // The bevel every letter in the wordmark carries, at the size a
                // button can wear one. Without it the choices are flat slabs
                // under a logo made entirely of raised edges.
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BorderColor(EDGE),
            BackgroundColor(IDLE),
        ))
        .with_children(|button| {
            button.spawn((
                Text::new(label),
                // The game's own face, at the size the logo would set it. This was
                // the default — which is a monospace, and a monospace under a
                // chrome-and-serif wordmark reads as a terminal that has wandered
                // onto the wrong screen.
                font.at(22.0),
                TextColor(SILVER),
            ));
            if !hint.is_empty() {
                button.spawn((
                    Text::new(hint),
                    font.at(13.0),
                    TextColor(SILVER_DIM),
                ));
            }
        });
}

fn despawn_menu(mut commands: Commands, roots: Query<Entity, With<MenuRoot>>) {
    for entity in &roots {
        commands.entity(entity).despawn();
    }
}

fn menu_buttons(
    mut buttons: Query<
        (&Interaction, &MenuAction, &mut BackgroundColor, &mut BorderColor),
        (Changed<Interaction>, With<Button>),
    >,
    mut next: ResMut<NextState<AppState>>,
    mut exit: EventWriter<AppExit>,
    mut progress: ResMut<crate::save::Progress>,
) {
    for (interaction, action, mut background, mut edge) in &mut buttons {
        match interaction {
            Interaction::Pressed => {
                background.0 = PRESSED;
                edge.0 = EDGE_LIT;
                match action {
                    // Continuing hands the save to whatever spawns the warden,
                    // BEFORE the state changes — the world loads on entering
                    // Playing, and a save arriving after that is a save arriving
                    // too late to have been used.
                    MenuAction::Continue => {
                        let save = crate::save::read();
                        progress.played = save.as_ref().map_or(0.0, |s| s.played);
                        progress.from = save;
                        next.set(AppState::Playing);
                    }
                    MenuAction::NewGame => {
                        // The old save goes when the new game STARTS, not when it
                        // is first written — otherwise quitting from the ranch
                        // without moving leaves the previous save standing, and
                        // "New Game" turns out to have done nothing.
                        crate::save::clear();
                        progress.from = None;
                        progress.played = 0.0;
                        next.set(AppState::Playing);
                    }
                    #[cfg(feature = "tools")]
                    MenuAction::Edit => next.set(AppState::Editing),
                    #[cfg(feature = "tools")]
                    MenuAction::Bench => next.set(AppState::Bench),
                    MenuAction::Quit => {
                        exit.write(AppExit::Success);
                    }
                }
            }
            Interaction::Hovered => {
                background.0 = HOVER;
                edge.0 = EDGE_LIT;
            }
            Interaction::None => {
                background.0 = IDLE;
                edge.0 = EDGE;
            }
        }
    }
}
