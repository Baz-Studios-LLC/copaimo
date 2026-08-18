//! The main menu.
//!
//! Deliberately drawn over the live 3D world rather than a flat backdrop — the
//! world is already streaming, and a view of it behind the menu is a free look
//! at what you're about to walk into.

use bevy::app::AppExit;
use bevy::prelude::*;

use crate::states::AppState;

const PANEL: Color = Color::srgba(0.04, 0.06, 0.10, 0.82);
const IDLE: Color = Color::srgba(0.10, 0.15, 0.22, 0.92);
const HOVER: Color = Color::srgba(0.18, 0.30, 0.42, 0.95);
const PRESSED: Color = Color::srgba(0.26, 0.44, 0.58, 0.98);

#[derive(Component)]
struct MenuRoot;

#[derive(Component, Clone, Copy)]
enum MenuAction {
    Play,
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

/// Wide enough to read the subtitle. The art is 2098 x 749, so this is a little
/// under half size and the crest under the wordmark still holds together.
const TITLE_WIDE: f32 = 560.0;

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Menu), spawn_menu)
            .add_systems(OnExit(AppState::Menu), despawn_menu)
            .add_systems(Update, menu_buttons.run_if(in_state(AppState::Menu)));
    }
}

fn spawn_menu(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands
        .spawn((
            MenuRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(12.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.01, 0.03, 0.06, 0.45)),
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    padding: UiRect::axes(Val::Px(46.0), Val::Px(34.0)),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(10.0),
                    ..default()
                },
                BackgroundColor(PANEL),
            ))
            .with_children(|panel| {
                // The title ART, not the word set in whatever font happened to
                // load. The logo carries the subtitle, the crest and the whole
                // look of the thing; a game whose own name is typed out on its
                // front screen looks like a placeholder because it is one.
                //
                // Sized by WIDTH with the height left to work itself out, so the
                // logo keeps its proportions whatever the window is. Giving both
                // would squash it, and a squashed logo is worse than no logo.
                panel.spawn((
                    ImageNode::new(asset_server.load(TITLE_ART)),
                    Node {
                        width: Val::Px(TITLE_WIDE),
                        margin: UiRect::bottom(Val::Px(18.0)),
                        ..default()
                    },
                ));

                for (action, label, hint) in [
                    (MenuAction::Play, "Explore World", "walk the map as the warden"),
                    #[cfg(feature = "tools")]
                    (MenuAction::Edit, "Shape the World", "sculpt the ground you walk on"),
                    #[cfg(feature = "tools")]
                    (MenuAction::Bench, "Workbench", "build houses and fences, piece by piece"),
                    (MenuAction::Quit, "Quit", ""),
                ] {
                    spawn_button(panel, action, label, hint);
                }
            });
        });
}

fn spawn_button(parent: &mut ChildSpawnerCommands, action: MenuAction, label: &str, hint: &str) {
    parent
        .spawn((
            action,
            Button,
            Node {
                width: Val::Px(300.0),
                padding: UiRect::axes(Val::Px(18.0), Val::Px(12.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(2.0),
                ..default()
            },
            BackgroundColor(IDLE),
        ))
        .with_children(|button| {
            button.spawn((
                Text::new(label),
                TextFont {
                    font_size: 21.0,
                    ..default()
                },
                TextColor(Color::srgb(0.94, 0.97, 1.0)),
            ));
            if !hint.is_empty() {
                button.spawn((
                    Text::new(hint),
                    TextFont {
                        font_size: 13.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.55, 0.68, 0.80)),
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
        (&Interaction, &MenuAction, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
    mut next: ResMut<NextState<AppState>>,
    mut exit: EventWriter<AppExit>,
) {
    for (interaction, action, mut background) in &mut buttons {
        match interaction {
            Interaction::Pressed => {
                background.0 = PRESSED;
                match action {
                    MenuAction::Play => next.set(AppState::Playing),
                    #[cfg(feature = "tools")]
                    MenuAction::Edit => next.set(AppState::Editing),
                    #[cfg(feature = "tools")]
                    MenuAction::Bench => next.set(AppState::Bench),
                    MenuAction::Quit => {
                        exit.write(AppExit::Success);
                    }
                }
            }
            Interaction::Hovered => background.0 = HOVER,
            Interaction::None => background.0 = IDLE,
        }
    }
}
