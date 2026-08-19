//! The pieces both tools' panels are built from.
//!
//! # Why this exists
//!
//! The terrain tool grew a good panel — sections, keycaps, sliders, live
//! readouts — and then the workbench arrived and got a wall of text at the top of
//! the screen instead. That was the wrong call, and the reason it happened is
//! worth writing down: the panel was *inside* the terrain tool, so building a
//! second one meant either copying it or lifting it out, and copying it is how a
//! codebase ends up with two interfaces that drift apart.
//!
//! So it is lifted out. One visual language, one set of behaviours, and a tool
//! that wants a panel gets the same panel.
//!
//! # Everything here is clickable
//!
//! The other half of what went wrong. Both tools were keyboard-only, which is
//! fine for someone who already knows them and hostile to everyone else — a
//! keybind is invisible until you have read a list of them, and a list of keys at
//! the top of the screen is not an interface.
//!
//! Keys still work, and they should: a maker who knows the tool should never have
//! to move their hand to the mouse. But every one of them has a thing on screen
//! you can press instead, and that thing shows its key so the keyboard is
//! discoverable rather than documented.

use bevy::prelude::*;
use bevy::ui::ScrollPosition;
use bevy::window::PrimaryWindow;

use super::theme::{
    UiFont, ACCENT, CARD, CARD_EDGE, KEYCAP, ROW_ACTIVE, TEXT, TEXT_DIM, TEXT_MUTED,
};

/// A panel that scrolls under the wheel.
///
/// # Bevy applies a scroll offset; nothing in Bevy ever SETS one
///
/// `Overflow::scroll_y()` clips and honours `ScrollPosition`, and that is the
/// whole of what the engine does — no built-in system reads the wheel. The bench
/// panel had the overflow set and nothing writing the position, which looks
/// exactly like a finished feature until the window is short enough to need it:
/// everything past the bottom edge was clipped and permanently unreachable.
#[derive(Component)]
pub struct Scrolls;

/// Whether the pointer is inside this node.
///
/// In LOGICAL pixels, which is the one space the two sides can honestly meet in:
/// the cursor is reported logical, while a node's size and place are physical.
/// Compared raw they agree only at 100% display scale, which is how a control
/// works on the machine that built it and misses on a laptop at 125%.
pub fn pointer_on(cursor: Vec2, node: &ComputedNode, at: &GlobalTransform) -> bool {
    let logical = node.inverse_scale_factor();
    let middle = at.translation().truncate() * logical;
    let half = node.size() * logical * 0.5;
    (cursor.x - middle.x).abs() <= half.x && (cursor.y - middle.y).abs() <= half.y
}

/// Whether the pointer is over any scrolling panel.
///
/// For the tools' OTHER wheel readers to ask before acting — a wheel that zooms
/// the room and scrolls the panel at once is answering two questions with one
/// gesture. Geometric rather than `Interaction`, because `Interaction` only
/// exists on buttons and a panel is mostly not buttons.
pub fn pointer_on_a_panel(
    windows: &Query<&Window, With<PrimaryWindow>>,
    panels: &Query<(&ComputedNode, &GlobalTransform), With<Scrolls>>,
) -> bool {
    let Some(cursor) = windows.iter().next().and_then(Window::cursor_position) else {
        return false;
    };
    panels.iter().any(|(node, at)| pointer_on(cursor, node, at))
}

/// Scrolls whichever panel the pointer is over.
pub fn scroll_panels(
    windows: Query<&Window, With<PrimaryWindow>>,
    scroll: Res<bevy::input::mouse::AccumulatedMouseScroll>,
    mut panels: Query<(&ComputedNode, &GlobalTransform, &mut ScrollPosition), With<Scrolls>>,
) {
    let notches = crate::util::wheel_notches(&scroll);
    if notches == 0.0 {
        return;
    }
    let Some(window) = windows.iter().next() else {
        return;
    };
    // Only a pointer somebody can SEE. The terrain tool confines and hides the
    // cursor while sculpting, and the hidden pointer still moves with the mouse —
    // so without this, a wheel flick mid-stroke could scroll a panel the maker
    // cannot even point at.
    if !window.cursor_options.visible {
        return;
    }
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    for (node, at, mut position) in &mut panels {
        if pointer_on(cursor, node, at) {
            // Wheel up reads earlier content. Layout clamps the offset to what
            // the content actually allows and writes the clamped figure back, so
            // no bound is kept here to disagree with it.
            position.offset_y -= notches * SCROLL_STEP;
        }
    }
}

/// Logical pixels per wheel notch. About two rows.
const SCROLL_STEP: f32 = 48.0;

/// How a row looks when the pointer is over it, and when it is being pressed.
pub const ROW_HOVER: Color = Color::srgba(0.83, 0.68, 0.34, 0.09);
pub const ROW_PRESS: Color = Color::srgba(0.83, 0.68, 0.34, 0.20);
pub const ROW_IDLE: Color = Color::NONE;

/// A row that can be pressed, carrying what pressing it means.
///
/// Generic over the tool's own idea of a choice, so the terrain tool's rows carry
/// a brush and the bench's carry a part, and neither has to know about the other.
#[derive(Component)]
pub struct Choice<T: Send + Sync + 'static>(pub T);

/// Marks the row currently in force, so it can be lit without the tool having to
/// hunt for it.
#[derive(Component)]
pub struct Chosen;

/// A heading that folds the rows under it away.
///
/// Sub-branches, and the reason for them is a shelf that keeps growing: eleven
/// terrain tools and ten parts and six colours is more than anybody wants to
/// read at once, and a maker laying a fence does not need the roof parts on
/// screen while they do it.
#[derive(Component)]
pub struct Branch {
    /// Which group this heading opens and closes.
    pub group: &'static str,
    pub open: bool,
}

/// Rows belonging to a branch, hidden and shown with it.
#[derive(Component)]
pub struct OfBranch(pub &'static str);

/// A swatch of colour that can be pressed.
#[derive(Component)]
pub struct Swatch(pub usize);

/// A group: a heading you can press, and a box holding what it opens.
///
/// # A heading is not a container
///
/// This spawned a heading and left the rows as its siblings, so nothing said where
/// one group stopped and the next began except a gap — eleven tool rows under four
/// headings read as one list of fifteen things. The box is the cheapest thing that
/// says "these belong together", and the eye takes it without being asked.
///
/// The rows go INSIDE it, which is why this takes a closure rather than being a
/// heading you spawn and then follow with rows. Getting that wrong is not
/// possible now: there is nowhere else to put them.
///
/// `open` is whether it starts unfolded. A comment in the bench claimed its
/// generators began folded shut for over a session while nothing implemented it —
/// the parameter did not exist, so the claim could not be true.
pub fn branch(
    parent: &mut ChildSpawnerCommands,
    font: &UiFont,
    group: &'static str,
    label: &str,
    open: bool,
    inside: impl FnOnce(&mut ChildSpawnerCommands),
) {
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                margin: UiRect::bottom(Val::Px(7.0)),
                ..default()
            },
        ))
        .with_children(|group_node| {
            group_node
                .spawn((
                    Branch { group, open },
                    Button,
                    Node {
                        width: Val::Percent(100.0),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(7.0),
                        padding: UiRect::new(
                            Val::Px(8.0),
                            Val::Px(8.0),
                            Val::Px(6.0),
                            Val::Px(6.0),
                        ),
                        ..default()
                    },
                    BackgroundColor(ROW_IDLE),
                ))
                .with_children(|row| {
                    // A caret rather than a plus or a triangle glyph: the font may
                    // not have either, and a caret is a character every font has.
                    // The folded glyph matches `fold_branches`, which owns it.
                    row.spawn((
                        BranchMark(group),
                        Text::new(if open { "v" } else { ">" }.to_string()),
                        font.at(9.0),
                        TextColor(ACCENT),
                    ));
                    row.spawn((
                        Text::new(label.to_string()),
                        font.at(11.0),
                        TextColor(TEXT_DIM),
                    ));
                });

            // The box itself.
            group_node
                .spawn((
                    OfBranch(group),
                    Node {
                        width: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::axes(Val::Px(5.0), Val::Px(5.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        display: if open { Display::Flex } else { Display::None },
                        ..default()
                    },
                    BorderColor(CARD_EDGE),
                    BorderRadius::all(Val::Px(6.0)),
                    BackgroundColor(CARD),
                ))
                .with_children(inside);
        });
}

/// The caret on a branch heading, so it can be turned as the branch folds.
#[derive(Component)]
pub struct BranchMark(pub &'static str);

/// Spawns a pressable row: a keycap, a label, and whatever it stands for.
pub fn row<T: Send + Sync + 'static>(
    parent: &mut ChildSpawnerCommands,
    font: &UiFont,
    group: &'static str,
    key: &str,
    label: &str,
    what: T,
) {
    parent
        .spawn((
            Choice(what),
            OfBranch(group),
            Button,
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.0),
                padding: UiRect::axes(Val::Px(6.0), Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(ROW_IDLE),
        ))
        .with_children(|row| {
            // The key is shown ON the row rather than in a list somewhere else,
            // which is what makes the keyboard discoverable instead of documented.
            if !key.is_empty() {
                row.spawn((
                    Node {
                        min_width: Val::Px(18.0),
                        justify_content: JustifyContent::Center,
                        padding: UiRect::axes(Val::Px(4.0), Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(KEYCAP),
                ))
                .with_children(|cap| {
                    cap.spawn((
                        Text::new(key.to_string()),
                        font.at(11.0),
                        TextColor(TEXT_MUTED),
                    ));
                });
            }
            row.spawn((
                RowLabel,
                Text::new(label.to_string()),
                font.at(13.0),
                TextColor(TEXT_MUTED),
            ));
        });
}

/// A row that says what a gesture does, without being pressable.
///
/// Some controls are not buttons and cannot be: you cannot click a thing to make
/// the wheel zoom. But a gesture nobody has mentioned is a feature that does not
/// exist as far as a maker is concerned, so it still gets a line — same keycap,
/// same layout, no highlight and no hand cursor, because pressing it does nothing
/// and it should not look as though it might.
pub fn note(
    parent: &mut ChildSpawnerCommands,
    font: &UiFont,
    group: &'static str,
    key: &str,
    what: &str,
) {
    parent
        .spawn((
            OfBranch(group),
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.0),
                padding: UiRect::axes(Val::Px(6.0), Val::Px(3.0)),
                ..default()
            },
        ))
        .with_children(|row| {
            row.spawn((
                Node {
                    min_width: Val::Px(18.0),
                    justify_content: JustifyContent::Center,
                    padding: UiRect::axes(Val::Px(4.0), Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(KEYCAP),
            ))
            .with_children(|cap| {
                cap.spawn((
                    Text::new(key.to_string()),
                    font.at(10.0),
                    TextColor(TEXT_DIM),
                ));
            });
            row.spawn((
                Text::new(what.to_string()),
                font.at(12.0),
                TextColor(TEXT_DIM),
            ));
        });
}

/// The text of a row, so it can be lit when the row is the one in force.
#[derive(Component)]
pub struct RowLabel;

/// A line of pressable colour swatches.
pub fn swatches(
    parent: &mut ChildSpawnerCommands,
    group: &'static str,
    colours: &[(&'static str, [u8; 3])],
) {
    parent
        .spawn((
            OfBranch(group),
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(5.0),
                padding: UiRect::axes(Val::Px(6.0), Val::Px(4.0)),
                ..default()
            },
        ))
        .with_children(|line| {
            for (at, (_, rgb)) in colours.iter().enumerate() {
                line.spawn((
                    Swatch(at),
                    Button,
                    Node {
                        width: Val::Px(26.0),
                        height: Val::Px(20.0),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BorderColor(Color::NONE),
                    BackgroundColor(Color::srgb_u8(rgb[0], rgb[1], rgb[2])),
                ));
            }
        });
}

/// Lights rows as the pointer moves over them.
///
/// Runs for every tool, on whatever their choice type is — so a tool adds a row
/// and gets the behaviour, rather than remembering to wire it.
pub fn light_rows<T: Send + Sync + 'static>(
    mut rows: Query<
        (&Interaction, &mut BackgroundColor, Option<&Chosen>),
        (Changed<Interaction>, With<Choice<T>>),
    >,
) {
    for (touch, mut colour, chosen) in &mut rows {
        colour.0 = match (touch, chosen.is_some()) {
            (Interaction::Pressed, _) => ROW_PRESS,
            (Interaction::Hovered, _) => ROW_HOVER,
            (Interaction::None, true) => ROW_ACTIVE,
            (Interaction::None, false) => ROW_IDLE,
        };
    }
}

/// Keeps the lit row and its label in step with what is actually in force.
///
/// Called by a tool with whatever it currently has selected, so the panel can
/// never disagree with the tool — which it would the moment a key changed the
/// selection and only the mouse path updated the row.
pub fn mark_chosen<T: PartialEq + Send + Sync + 'static>(
    commands: &mut Commands,
    rows: &Query<(Entity, &Choice<T>, Option<&Chosen>, &Children)>,
    labels: &mut Query<&mut TextColor, With<RowLabel>>,
    backgrounds: &mut Query<&mut BackgroundColor, With<Choice<T>>>,
    now: &T,
) {
    for (entity, choice, chosen, kids) in rows {
        let is = choice.0 == *now;
        if is && chosen.is_none() {
            commands.entity(entity).insert(Chosen);
        } else if !is && chosen.is_some() {
            commands.entity(entity).remove::<Chosen>();
        }
        if let Ok(mut colour) = backgrounds.get_mut(entity) {
            colour.0 = if is { ROW_ACTIVE } else { ROW_IDLE };
        }
        for kid in kids.iter() {
            if let Ok(mut text) = labels.get_mut(kid) {
                text.0 = if is { ACCENT } else { TEXT_MUTED };
            }
        }
    }
}

/// Folds and unfolds branches when their headings are pressed.
pub fn fold_branches(
    mut headings: Query<(&Interaction, &mut Branch, &mut BackgroundColor), Changed<Interaction>>,
    mut marks: Query<(&BranchMark, &mut Text)>,
    mut rows: Query<(&OfBranch, &mut Node)>,
) {
    let mut turned: Vec<(&'static str, bool)> = Vec::new();
    for (touch, mut branch, mut colour) in &mut headings {
        match touch {
            Interaction::Pressed => {
                branch.open = !branch.open;
                turned.push((branch.group, branch.open));
                colour.0 = ROW_PRESS;
            }
            Interaction::Hovered => colour.0 = ROW_HOVER,
            Interaction::None => colour.0 = ROW_IDLE,
        }
    }
    for (group, open) in turned {
        for (mark, mut text) in &mut marks {
            if mark.0 == group {
                **text = if open { "v".into() } else { ">".into() };
            }
        }
        for (of, mut node) in &mut rows {
            if of.0 == group {
                node.display = if open { Display::Flex } else { Display::None };
            }
        }
    }
}

/// Where along a node the pointer is, 0 at its left edge to 1 at its right.
///
/// What makes a meter a SLIDER rather than a picture of one. A bar that only
/// reports a value is a readout; a bar you can press at the two-thirds mark and
/// have it become two thirds is a control, and the difference is one function.
///
/// The node's geometry is physical pixels and the cursor is logical — see
/// [`pointer_on`] — so the node is brought into the cursor's space first. Left
/// raw, a slider read the wrong fraction on any display scale but 100%.
pub fn fraction_along(cursor: Vec2, node: &ComputedNode, at: &GlobalTransform) -> f32 {
    let logical = node.inverse_scale_factor();
    let middle = at.translation().truncate() * logical;
    let wide = (node.size().x * logical).max(1.0);
    ((cursor.x - (middle.x - wide * 0.5)) / wide).clamp(0.0, 1.0)
}

/// Lights the swatch in force and the one under the pointer.
pub fn light_swatches(
    mut swatches: Query<(&Interaction, &Swatch, &mut BorderColor)>,
    chosen: usize,
) {
    for (touch, swatch, mut border) in &mut swatches {
        border.0 = match (swatch.0 == chosen, touch) {
            (true, _) => ACCENT,
            (false, Interaction::Hovered | Interaction::Pressed) => TEXT,
            (false, Interaction::None) => Color::NONE,
        };
    }
}
