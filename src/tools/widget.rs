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

use super::theme::{UiFont, ACCENT, KEYCAP, ROW_ACTIVE, TEXT, TEXT_DIM, TEXT_MUTED};

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
/// terrain tools and seven parts and six colours is more than anybody wants to
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

/// Spawns a heading that folds what is under it.
pub fn branch(parent: &mut ChildSpawnerCommands, font: &UiFont, group: &'static str, label: &str) {
    parent
        .spawn((
            Branch { group, open: true },
            Button,
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(6.0),
                padding: UiRect::axes(Val::Px(2.0), Val::Px(5.0)),
                ..default()
            },
            BackgroundColor(ROW_IDLE),
        ))
        .with_children(|row| {
            // A caret rather than a plus or a triangle glyph: the font may not
            // have either, and a caret is two characters that every font has.
            row.spawn((
                BranchMark(group),
                Text::new("v".to_string()),
                font.at(10.0),
                TextColor(TEXT_DIM),
            ));
            row.spawn((
                Text::new(label.to_string()),
                font.at(11.0),
                TextColor(TEXT_DIM),
            ));
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
pub fn fraction_along(cursor: Vec2, node: &ComputedNode, at: &GlobalTransform) -> f32 {
    let middle = at.translation().truncate();
    let wide = node.size().x.max(1.0);
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
