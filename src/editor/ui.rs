//! The terrain tool's interface: a sidebar, live readouts, and transient
//! confirmations.
//!
//! Kept separate from the game's F3 debug overlay on purpose. This is the
//! tool's UI, it appears only in the tool, and it moves with the tool if the
//! module is ever lifted into another project.

use bevy::prelude::*;
use bevy::text::LineBreak;

use crate::tools::theme::{
    self, rule, section, tool_color, UiFont, ACCENT, PANEL, PANEL_WIDTH, RULE,
    TEXT,
    TEXT_DIM, TEXT_MUTED, UNSAVED,
};
use crate::editor::{Brush, MAX_RADIUS, MAX_STRENGTH, MIN_RADIUS, MIN_STRENGTH};
use crate::tools::widget::{self, Choice, RowLabel};
use crate::states::AppState;
use crate::world::edit::Brushing;
use crate::world::terrain::TerrainSource;

/// How long a confirmation stays on screen, and how long it takes to fade.
const TOAST_HOLD: f32 = 1.8;
const TOAST_FADE: f32 = 0.6;

/// A transient confirmation — saves, undo, redo.
///
/// Silent success is the wrong default for a tool: pressing Ctrl+S and seeing
/// nothing happen is indistinguishable from the shortcut not working.
#[derive(Resource, Default)]
pub struct Toast {
    message: String,
    remaining: f32,
}

impl Toast {
    pub fn show(&mut self, message: impl Into<String>) {
        self.message = message.into();
        self.remaining = TOAST_HOLD + TOAST_FADE;
    }
}

#[derive(Component)]
struct EditorUiRoot;

/// The caption under the palette, saying what the tool in hand does.
#[derive(Component)]
struct ToolSaying;

/// A live value in the sidebar. One component and one system rather than a
/// marker type per field.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum Readout {
    Radius,
    Strength,
    Position,
    Ground,
    Place,
    Edited,
    Planted,
    Marked,
    Surfaced,
    History,
}

/// The filled portion of a meter bar.
#[derive(Component, Clone, Copy)]
pub enum Meter {
    Radius,
    Strength,
}

#[derive(Component)]
struct UnsavedMark;

#[derive(Component)]
struct ToastPanel;

#[derive(Component)]
struct ToastLabel;

pub struct EditorUiPlugin;

impl Plugin for EditorUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Toast>()
            .add_systems(OnEnter(AppState::Editing), (spawn_sidebar, spawn_toast))
            .add_systems(OnExit(AppState::Editing), despawn_ui)
            .add_systems(
                Update,
                (
                    // What was pressed, before what everything says — so a row
                    // pressed this frame is lit this frame rather than next.
                    pressed_tool,
                    dragged_meter,
                    crate::tools::widget::fold_branches,
                    crate::tools::widget::light_rows::<Brushing>,
                    refresh_tools,
                    refresh_readouts,
                    refresh_meters,
                    drive_toast,
                )
                    .chain()
                    .run_if(in_state(AppState::Editing)),
            );
    }
}

// ----------------------------------------------------------------- construction

fn spawn_sidebar(mut commands: Commands, font: Res<UiFont>) {
    // Crosshair, dead center: the brush aims straight down the view ray.
    commands.spawn((
        EditorUiRoot,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(50.0),
            top: Val::Percent(50.0),
            width: Val::Px(5.0),
            height: Val::Px(5.0),
            margin: UiRect {
                left: Val::Px(-2.5),
                top: Val::Px(-2.5),
                ..default()
            },
            ..default()
        },
        BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.9)),
    ));

    commands
        .spawn((
            EditorUiRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(16.0),
                top: Val::Px(16.0),
                width: Val::Px(PANEL_WIDTH),
                flex_direction: FlexDirection::Column,
                // Nothing leaves the panel, whatever is put in it. Text that
                // overruns is a layout mistake to fix at the source — but a
                // shelf whose writing runs out over the world does not look
                // like a tool, and no future addition should be able to do it
                // by accident.
                overflow: Overflow::clip(),
                // A hairline edge. Without one the panel is a dark rectangle
                // that fades into whatever is behind it, and where a tool stops
                // and the world starts is exactly what should never be vague.
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(PANEL),
            BorderColor(RULE),
        ))
        .with_children(|panel| {
            header(panel, &font);
            body(panel, &font);
        });
}

fn header(panel: &mut ChildSpawnerCommands, font: &UiFont) {
    panel
        .spawn((
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::axes(Val::Px(14.0), Val::Px(11.0)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
            BackgroundColor(theme::HEADER),
        ))
        .with_children(|bar| {
            // Gold on near-black, lettered in Cinzel: the bench's own header,
            // so a maker crossing between the two programs sees one workshop.
            bar.spawn((Text::new("TERRAIN TOOL"), font.at(14.0), TextColor(ACCENT)));
            // A dot rather than the word "unsaved": it reads at a glance and
            // never changes the layout as it appears and disappears.
            bar.spawn((
                UnsavedMark,
                Node {
                    width: Val::Px(7.0),
                    height: Val::Px(7.0),
                    ..default()
                },
                BackgroundColor(Color::NONE),
            ));
        });
}

fn body(panel: &mut ChildSpawnerCommands, font: &UiFont) {
    panel
        .spawn(Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            padding: UiRect::axes(Val::Px(14.0), Val::Px(12.0)),
            row_gap: Val::Px(9.0),
            ..default()
        })
        .with_children(|body| {
            // Grouped, and each group folds.
            //
            // Eleven tools in one list is a wall, and the groups are not
            // decoration: shaping the ground, laying something over it, growing
            // things on it and taking work back out are four different jobs, and a
            // maker doing one of them does not need the other three on screen.
            for (group, label, tools) in [
                (
                    "shape",
                    "SHAPE THE GROUND",
                    &[
                        Brushing::Raise,
                        Brushing::Lower,
                        Brushing::Smooth,
                        Brushing::Flatten,
                        Brushing::Roughen,
                        Brushing::Erode,
                        Brushing::Ramp,
                    ][..],
                ),
                ("lay", "LAY OVER IT", &[Brushing::Path][..]),
                ("grow", "GROW AND MARK", &[Brushing::Plant, Brushing::Country][..]),
                ("back", "TAKE IT BACK", &[Brushing::Revert][..]),
            ] {
                widget::branch(body, font, group, label);
                for how in tools {
                    widget::row(
                        body,
                        font,
                        group,
                        crate::editor::key_for(*how),
                        how.name(),
                        *how,
                    );
                }
            }
            tool_saying(body, font);

            body.spawn(rule());
            body.spawn(section(font, "BRUSH"));
            meter_row(body, font, "Radius", Meter::Radius, Readout::Radius);
            meter_row(body, font, "Strength", Meter::Strength, Readout::Strength);

            body.spawn(rule());
            body.spawn(section(font, "CURSOR"));
            value_row(body, font, "Position", Readout::Position);
            value_row(body, font, "Ground", Readout::Ground);
            value_row(body, font, "Place", Readout::Place);

            body.spawn(rule());
            body.spawn(section(font, "EDITS"));
            value_row(body, font, "Sculpted", Readout::Edited);
            value_row(body, font, "Planted", Readout::Planted);
            value_row(body, font, "Biome", Readout::Marked);
            value_row(body, font, "Surfaced", Readout::Surfaced);
            value_row(body, font, "History", Readout::History);

            body.spawn(rule());
            for (keys, action) in [
                ("LMB", "apply brush"),
                ("RMB", "invert brush"),
                ("Wheel", "brush radius"),
                ("[ ]", "brush strength"),
                ("Alt", "free pointer, click map"),
                ("Q E", "fly down / up"),
                ("- =", "fly speed"),
                ("Ctrl Z", "undo / Ctrl Y redo"),
                ("Ctrl S", "save all three layers"),
                ("Esc", "back to menu"),
            ] {
                shortcut_row(body, font, keys, action);
            }
        });
}

fn tool_saying(parent: &mut ChildSpawnerCommands, font: &UiFont) {
    parent
        .spawn(Node {
            width: Val::Percent(100.0),
            padding: UiRect::new(Val::Px(6.0), Val::Px(6.0), Val::Px(6.0), Val::Px(2.0)),
            ..default()
        })
        .with_children(|line| {
            line.spawn((
                ToolSaying,
                Text::new(""),
                font.at(12.0),
                TextColor(TEXT_DIM),
                // Wrapping, unlike the names: a caption is prose and the panel
                // is the width it is.
                Node {
                    width: Val::Percent(100.0),
                    ..default()
                },
            ));
        });
}

fn meter_row(
    parent: &mut ChildSpawnerCommands,
    font: &UiFont,
    label: &str,
    meter: Meter,
    readout: Readout,
) {
    parent
        .spawn(Node {
            width: Val::Percent(100.0),
            align_items: AlignItems::Center,
            column_gap: Val::Px(8.0),
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                Text::new(label.to_string()),
                font.at(12.0),
                TextColor(TEXT_MUTED),
                Node {
                    width: Val::Px(62.0),
                    ..default()
                },
            ));
            // Track and fill. A bar shows where you are within the usable range
            // at a glance, which a number alone never does.
            row.spawn((
                // Pressable, which is what makes it a control rather than a
                // picture of one. Taller than the bar it draws so there is
                // something to hit: four pixels is a readout, twelve is a target.
                MeterTrack(meter),
                Button,
                Node {
                    width: Val::Px(112.0),
                    height: Val::Px(12.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::NONE),
            ))
            .with_children(|hit| {
                hit.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(4.0),
                        ..default()
                    },
                    BackgroundColor(theme::METER_TRACK),
                ))
                .with_children(|track| {
                    track.spawn((
                        meter,
                        Node {
                            width: Val::Percent(0.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        BackgroundColor(TEXT_MUTED),
                    ));
                });
            });
            row.spawn((
                readout,
                Text::new(""),
                font.at(12.0),
                TextColor(TEXT),
                TextLayout {
                    linebreak: LineBreak::NoWrap,
                    ..default()
                },
            ));
        });
}

fn value_row(parent: &mut ChildSpawnerCommands, font: &UiFont, label: &str, readout: Readout) {
    parent
        .spawn(Node {
            width: Val::Percent(100.0),
            column_gap: Val::Px(8.0),
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                Text::new(label.to_string()),
                font.at(12.0),
                TextColor(TEXT_MUTED),
                Node {
                    width: Val::Px(62.0),
                    ..default()
                },
            ));
            row.spawn((
                readout,
                Text::new(""),
                font.at(12.0),
                TextColor(TEXT),
                TextLayout {
                    linebreak: LineBreak::NoWrap,
                    ..default()
                },
            ));
        });
}

fn shortcut_row(parent: &mut ChildSpawnerCommands, font: &UiFont, keys: &str, action: &str) {
    parent
        .spawn(Node {
            width: Val::Percent(100.0),
            align_items: AlignItems::Center,
            column_gap: Val::Px(8.0),
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                Node {
                    width: Val::Px(56.0),
                    justify_content: JustifyContent::FlexEnd,
                    ..default()
                },
                children![(
                    Text::new(keys.to_string()),
                    font.at(11.0),
                    TextColor(TEXT_MUTED),
                    TextLayout {
                        linebreak: LineBreak::NoWrap,
                        ..default()
                    },
                )],
            ));
            row.spawn((
                Text::new(action.to_string()),
                font.at(11.0),
                TextColor(TEXT_DIM),
                TextLayout {
                    linebreak: LineBreak::NoWrap,
                    ..default()
                },
            ));
        });
}

fn spawn_toast(mut commands: Commands, font: Res<UiFont>) {
    commands.spawn((
        EditorUiRoot,
        ToastPanel,
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(48.0),
            left: Val::Percent(50.0),
            margin: UiRect::left(Val::Px(-110.0)),
            width: Val::Px(220.0),
            justify_content: JustifyContent::Center,
            padding: UiRect::axes(Val::Px(16.0), Val::Px(9.0)),
            ..default()
        },
        BackgroundColor(Color::NONE),
        children![(
            ToastLabel,
            Text::new(""),
            font.at(13.0),
            TextColor(Color::NONE),
            TextLayout {
                linebreak: LineBreak::NoWrap,
                ..default()
            },
        )],
    ));
}

fn despawn_ui(mut commands: Commands, roots: Query<Entity, With<EditorUiRoot>>) {
    for entity in &roots {
        commands.entity(entity).despawn();
    }
}

// -------------------------------------------------------------------- refresh

/// Selecting a tool by pressing its row.
///
/// The same field the keys write, so the panel and the keyboard can never mean
/// different things. Selecting a tool mid-drag also drops a half-laid ramp, for
/// the same reason pressing its key does.
pub fn pressed_tool(
    rows: Query<(&Interaction, &Choice<Brushing>), Changed<Interaction>>,
    mut brush: ResMut<Brush>,
) {
    for (touch, choice) in &rows {
        if *touch != Interaction::Pressed {
            continue;
        }
        // Pressing the biome row again cycles which country it lays, exactly as
        // pressing B again does.
        if choice.0.is_countrying() && brush.how == choice.0 {
            let all = terrain_core::region::Country::ALL;
            let next = all
                .iter()
                .position(|c| *c == brush.laying)
                .map_or(0, |at| (at + 1) % all.len());
            brush.laying = all[next];
        }
        brush.how = choice.0;
        brush.ramp_from = None;
    }
}

/// Dragging a meter to set the value it shows.
///
/// A bar that only reports is a readout; a bar you can press at the two-thirds
/// mark and have become two thirds is a control. The keys and the wheel still work
/// and are still better for a small change — this is for a big one.
pub fn dragged_meter(
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    tracks: Query<(&Interaction, &MeterTrack, &ComputedNode, &GlobalTransform)>,
    mut brush: ResMut<Brush>,
) {
    let Some(cursor) = windows.iter().next().and_then(|w| w.cursor_position()) else {
        return;
    };
    for (touch, track, node, at) in &tracks {
        // Held, not just clicked, so a maker can sweep the brush up to size in one
        // gesture rather than pressing along the bar.
        if *touch != Interaction::Pressed {
            continue;
        }
        let along = crate::tools::widget::fraction_along(cursor, node, at);
        match track.0 {
            Meter::Radius => {
                brush.radius = MIN_RADIUS + (MAX_RADIUS - MIN_RADIUS) * along;
            }
            Meter::Strength => {
                brush.strength = MIN_STRENGTH + (MAX_STRENGTH - MIN_STRENGTH) * along;
            }
        }
    }
}

/// The pressable part of a meter.
#[derive(Component)]
pub struct MeterTrack(pub Meter);

fn refresh_tools(
    mut commands: Commands,
    brush: Res<Brush>,
    choices: Query<(Entity, &Choice<Brushing>, Option<&crate::tools::widget::Chosen>, &Children)>,
    mut row_labels: Query<&mut TextColor, With<RowLabel>>,
    mut row_backs: Query<&mut BackgroundColor, With<Choice<Brushing>>>,
    mut saying: Query<(&mut Text, &mut TextColor), (With<ToolSaying>, Without<RowLabel>)>,
) {
    // The lit row follows whatever holds the tool — a key press, a click, or the
    // ramp being cancelled — rather than only the path that set it.
    //
    // This used to light rows through its own `ToolRow` query as well, which is how
    // it came to hold two mutable borrows of `BackgroundColor` at once and panicked
    // the moment the tool opened. There is one way a row is lit now.
    crate::tools::widget::mark_chosen(
        &mut commands,
        &choices,
        &mut row_labels,
        &mut row_backs,
        &brush.how,
    );

    for (mut text, mut colour) in &mut saying {
        // The biome brush says WHICH country it is laying, because that is the
        // half of its state a maker cannot see anywhere else. One brush lays three
        // things and a single row cannot show all three, so the caption carries it.
        let said = if brush.how.is_countrying() {
            format!(
                "Painting {} - press the row again to change, right button clears",
                brush.laying.name()
            )
        } else {
            brush.how.said().to_string()
        };
        if **text != said {
            **text = said;
        }
        // Tinted with the tool, so the caption, the lit row and the ring on the
        // ground are all one colour and none of them has to be read.
        colour.0 = tool_color(brush.how).with_alpha(0.75);
    }
}

fn refresh_meters(brush: Res<Brush>, mut meters: Query<(&Meter, &mut Node, &mut BackgroundColor)>) {
    for (meter, mut node, mut color) in &mut meters {
        let fraction = match meter {
            Meter::Radius => {
                // Logarithmic, matching how the wheel scales it — otherwise the
                // bar sits pinned near zero across most of the useful range.
                let t = (brush.radius / crate::editor::MIN_RADIUS).ln()
                    / (crate::editor::MAX_RADIUS / crate::editor::MIN_RADIUS).ln();
                t.clamp(0.0, 1.0)
            }
            Meter::Strength => {
                let t = (brush.strength / crate::editor::MIN_STRENGTH).ln()
                    / (crate::editor::MAX_STRENGTH / crate::editor::MIN_STRENGTH).ln();
                t.clamp(0.0, 1.0)
            }
        };
        node.width = Val::Percent(fraction * 100.0);
        color.0 = tool_color(brush.how);
    }
}

fn refresh_readouts(
    brush: Res<Brush>,
    terrain: Res<TerrainSource>,
    mut readouts: Query<(&Readout, &mut Text)>,
    mut unsaved: Query<&mut BackgroundColor, With<UnsavedMark>>,
) {
    let (cells, is_unsaved, ground_undo, ground_redo) =
        terrain
            .edits()
            .read()
            .map_or((0, false, false, false), |edits| {
                (
                    edits.sculpted_cells(),
                    edits.unsaved,
                    edits.can_undo(),
                    edits.can_redo(),
                )
            });
    // Both layers, because the key reaches into either — a readout that only
    // watched the ground would say "nothing to undo" with a wood still standing.
    let (woods_undo, woods_redo, woods_unsaved) =
        terrain.woods().read().map_or((false, false, false), |woods| {
            (woods.can_undo(), woods.can_redo(), woods.unsaved)
        });
    let (worn_undo, worn_redo, worn_unsaved) =
        terrain
            .surface()
            .read()
            .map_or((false, false, false), |worn| {
                (worn.can_undo(), worn.can_redo(), worn.unsaved)
            });
    let undo_depth = ground_undo || woods_undo || worn_undo;
    let redo_depth = ground_redo || woods_redo || worn_redo;
    // ANY layer. The dot watched the ground alone, so an afternoon's planting
    // could sit unwritten under a panel saying everything was saved.
    let is_unsaved = is_unsaved || woods_unsaved || worn_unsaved;

    for mut background in &mut unsaved {
        background.0 = if is_unsaved { UNSAVED } else { Color::NONE };
    }

    for (readout, mut text) in &mut readouts {
        **text = match readout {
            Readout::Radius => format!("{:.0} m", brush.radius),
            // No unit, because there isn't one: strength is metres per second to
            // the tools that push, a blend fraction to the ones that level, and
            // a settling pace to erosion. `Brushing::rate` is the single place
            // that decides which, and naming a unit here would be a second copy
            // of that decision, free to drift from it.
            Readout::Strength => format!("{:.0}", brush.strength),
            Readout::Position => match brush.hit {
                Some(hit) => format!("{:.0}, {:.0}", hit.x, hit.z),
                None => "off world".to_string(),
            },
            Readout::Ground => match brush.hit {
                Some(hit) => {
                    let slope = 1.0 - terrain.normal(hit.x, hit.z, 1.0).y;
                    format!("{:.1} m   slope {slope:.2}", hit.y)
                }
                None => "-".to_string(),
            },
            // What kind of place the brush is over, and how sure. Tuning a
            // climate means standing in it and reading this: "Desert 0.08" says
            // the boundary is right here.
            Readout::Place => match brush.hit {
                Some(hit) => format!(
                    "{} {:.2}",
                    terrain.biome(hit.x, hit.z).name(),
                    terrain.biome_confidence(hit.x, hit.z)
                ),
                None => "-".to_string(),
            },
            Readout::Edited => format!("{cells} cells"),
            Readout::Planted => format!("{} cells", terrain.planted_cells()),
            Readout::Marked => format!("{} cells", terrain.marked_cells()),
            Readout::Surfaced => format!("{} cells", terrain.worn_cells()),
            Readout::History => match (undo_depth, redo_depth) {
                (false, _) => "nothing to undo".to_string(),
                (true, true) => "undo and redo ready".to_string(),
                (true, false) => "undo ready".to_string(),
            },
        };
    }
}

fn drive_toast(
    time: Res<Time>,
    mut toast: ResMut<Toast>,
    mut panels: Query<&mut BackgroundColor, With<ToastPanel>>,
    mut labels: Query<(&mut Text, &mut TextColor), With<ToastLabel>>,
) {
    if toast.remaining > 0.0 {
        toast.remaining = (toast.remaining - time.delta_secs()).max(0.0);
    }

    // Hold at full opacity, then fade — so a confirmation is readable rather
    // than a flicker, and doesn't linger over the work either.
    let alpha = (toast.remaining / TOAST_FADE).clamp(0.0, 1.0);

    for mut background in &mut panels {
        background.0 = theme::HEADER.with_alpha(alpha * 0.92);
    }
    for (mut text, mut color) in &mut labels {
        if **text != toast.message {
            **text = toast.message.clone();
        }
        color.0 = TEXT.with_alpha(alpha);
    }
}
