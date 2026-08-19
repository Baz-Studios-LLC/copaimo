//! The workbench's interface.
//!
//! # This replaced a wall of text at the top of the screen
//!
//! Worth saying plainly, because the reason it happened is a trap worth naming.
//! The bench was built keyboard-first — a lattice suits keys, and keys were faster
//! to write — and the state was reported by printing it into a corner. Every one
//! of those decisions was locally reasonable and together they made a tool nobody
//! could pick up: no way to see what the parts are without reading a list of keys,
//! no way to press anything, and a readout that is text where an interface should
//! be.
//!
//! The terrain tool already had a panel. The standard was set in this codebase and
//! the bench did not meet it.
//!
//! # Both hands
//!
//! Every choice is a row you can press, and every row shows its key. Keys still
//! work — somebody who knows the tool should never have to reach for the mouse —
//! but nothing is *only* a key, because a key is invisible until somebody has read
//! a list of them.

use bevy::prelude::*;

use crate::build::kit::{Bench, Part, TINTS};
use crate::build::pattern::Pattern;
use crate::states::AppState;
use crate::tools::theme::{rule, UiFont, ACCENT, PANEL, PANEL_WIDTH, TEXT, TEXT_DIM, TEXT_MUTED, UNSAVED};
use crate::tools::widget::{self, Choice, RowLabel, Swatch};

use super::{Doing, Hand};

#[derive(Component)]
pub struct BenchPanel;

/// A live value in the panel, so one system keeps them all current.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum Readout {
    Cursor,
    Pieces,
    Picture,
    Kiln,
}

/// What a pressable row in this panel can mean.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Press {
    Part(Part),
    Mode(Doing),
    Ask(Pattern),
    Turn,
    /// Turn something already placed, rather than the piece in hand.
    TurnPlaced,
    Save,
    Undo,
    Fire,
    NextPicture,
    FlipPicture,
    Leave,
}

pub fn open(mut commands: Commands, font: Res<UiFont>) {
    commands
        .spawn((
            BenchPanel,
            super::OfBench,
            // The marker that makes the overflow below mean something: Bevy
            // APPLIES a scroll offset but nothing in it ever sets one, so a
            // panel with scroll_y and no writer is simply clipped.
            widget::Scrolls,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                width: Val::Px(PANEL_WIDTH),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(14.0)),
                row_gap: Val::Px(6.0),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            BackgroundColor(PANEL),
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new("WORKBENCH".to_string()),
                font.at(15.0),
                TextColor(ACCENT),
            ));
            panel.spawn((
                Text::new("build a thing out of parts".to_string()),
                font.at(11.0),
                TextColor(TEXT_DIM),
            ));
            panel.spawn(rule());

            // What the left button does. First, because everything below it means
            // something different depending on this.
            widget::branch(panel, &font, "mode", "MODE", true, |rows| {
                widget::row(rows, &font, "mode", "P", "BUILD", Press::Mode(Doing::Building));
                widget::row(rows, &font, "mode", "P", "PAINT", Press::Mode(Doing::Painting));
            });

            widget::branch(panel, &font, "parts", "PARTS", true, |rows| {
                for part in Part::ALL {
                    // The key from the kit's table rather than from this row's
                    // place in the list. Counting rows was right while there were
                    // seven parts and nine digits; the tenth part sits on nought,
                    // and a panel that counted would print "10" on a key that does
                    // not exist.
                    widget::row(rows, &font, "parts", part.cap(), part.name(), Press::Part(part));
                }
                widget::row(rows, &font, "parts", "R", "TURN A QUARTER", Press::Turn);
                widget::row(
                    rows,
                    &font,
                    "parts",
                    "SH-R",
                    "TURN WHAT IS UNDER IT",
                    Press::TurnPlaced,
                );
            });

            // The camera, said out loud. Orbiting by dragging a button nobody
            // mentioned is a feature that does not exist as far as a maker is
            // concerned, which is what this whole panel is for.
            widget::branch(panel, &font, "view", "VIEW", true, |rows| {
                widget::note(rows, &font, "view", "WASD", "walk the view");
                widget::note(rows, &font, "view", "MID", "drag to orbit");
                widget::note(rows, &font, "view", "SH-MID", "drag to pan");
                widget::note(rows, &font, "view", "WHL", "zoom");
                widget::note(rows, &font, "view", "[ ]", "square up a quarter");
                widget::note(rows, &font, "view", "- =", "zoom a step");
            });

            // The arrows. Worth a group of its own, because a handle nobody has
            // mentioned is one nobody grabs — and moving a piece by dragging it is
            // not something a maker will try unguessed on a tool where every other
            // gesture places something.
            widget::branch(panel, &font, "move", "MOVE A PIECE", true, |rows| {
                widget::note(rows, &font, "move", "AIM", "handles show on the nearest");
                widget::note(rows, &font, "move", "EMPTY", "cursor: clicks grab, not build");
                widget::note(rows, &font, "move", "DRAG", "an arrow to move it");
                widget::note(rows, &font, "move", "SH", "hold for quarter-metres");
                widget::note(rows, &font, "move", "R-G-B", "is X-Y-Z");
                widget::note(rows, &font, "move", "AMBER", "end blocks: pull to lengthen");
                widget::note(rows, &font, "move", "SIDE", "blocks: pull a floor wider");
                widget::note(rows, &font, "move", "ARROWS", "nudge the cursor a cell");
            });

            widget::branch(panel, &font, "colour", "COLOUR", true, |rows| {
                widget::swatches(rows, "colour", &TINTS);
            });

            // Folded shut to begin with. A maker laying a fence does not need four
            // generators on screen while they do it, and a shelf that keeps growing
            // is the whole reason branches exist.
            widget::branch(panel, &font, "ask", "ASK FOR ONE", false, |rows| {
                for what in Pattern::ALL {
                    widget::row(rows, &font, "ask", "G", what.name(), Press::Ask(what));
                }
            });

            widget::branch(panel, &font, "picture", "PICTURE", false, |rows| {
                value_row(rows, &font, "showing", Readout::Picture);
                widget::row(rows, &font, "picture", "I", "NEXT PICTURE", Press::NextPicture);
                widget::row(rows, &font, "picture", "U", "UPRIGHT / FLAT", Press::FlipPicture);
                widget::row(rows, &font, "picture", "F5", "MAKE A MODEL", Press::Fire);
                value_row(rows, &font, "kiln", Readout::Kiln);
            });

            widget::branch(panel, &font, "work", "WORK", true, |rows| {
                value_row(rows, &font, "cursor", Readout::Cursor);
                value_row(rows, &font, "pieces", Readout::Pieces);
                widget::row(rows, &font, "work", "Ctrl+Z", "UNDO", Press::Undo);
                widget::row(rows, &font, "work", "Ctrl+S", "SAVE", Press::Save);
                widget::row(rows, &font, "work", "ESC", "BACK TO THE MENU", Press::Leave);
            });

            // The gestures that are not rows, kept short: the boxes above say
            // what each group does, and repeating all of it here was a wall of
            // text under a set of menus built to replace one.
            for said in [
                "mouse aims and snaps",
                "LEFT place  ·  RIGHT take away",
                "WASD/QE nudge  ·  SHIFT a module",
            ] {
                panel.spawn((
                    Text::new(said.to_string()),
                    font.at(10.0),
                    TextColor(TEXT_DIM),
                    Node {
                        margin: UiRect::new(Val::Px(8.0), Val::Px(8.0), Val::Px(1.0), Val::Px(1.0)),
                        ..default()
                    },
                ));
            }
        });
}

/// A labelled live value.
fn value_row(parent: &mut ChildSpawnerCommands, font: &UiFont, label: &str, which: Readout) {
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                justify_content: JustifyContent::SpaceBetween,
                column_gap: Val::Px(8.0),
                padding: UiRect::axes(Val::Px(6.0), Val::Px(3.0)),
                ..default()
            },
        ))
        .with_children(|row| {
            row.spawn((
                Text::new(label.to_string()),
                font.at(11.0),
                TextColor(TEXT_DIM),
            ));
            row.spawn((which, Text::new("-".to_string()), font.at(11.0), TextColor(TEXT)));
        });
}

/// Acts on a pressed row.
///
/// One place where a press becomes a change, so the mouse and the keys can never
/// mean different things — which is exactly what happens when each grows its own
/// handler.
pub fn pressed(
    mut rows: Query<(&Interaction, &Choice<Press>), Changed<Interaction>>,
    mut hand: ResMut<Hand>,
    mut bench: ResMut<Bench>,
    mut asked: ResMut<super::Asked>,
    mut reference: ResMut<super::reference::Reference>,
    mut firing: ResMut<super::kiln::Firing>,
    mut next: ResMut<NextState<AppState>>,
) {
    for (touch, choice) in &mut rows {
        if *touch != Interaction::Pressed {
            continue;
        }
        match choice.0 {
            Press::Part(part) => hand.take(part),
            Press::Mode(doing) => hand.doing = doing,
            Press::Turn => hand.quarters = (hand.quarters + 1) % 4,
            Press::TurnPlaced => {
                bench.turn_nearest(hand.at, crate::build::kit::MODULE);
            }
            Press::Ask(what) => {
                asked.what = Pattern::ALL.iter().position(|p| *p == what).unwrap_or(0);
                asked.seed = asked.seed.wrapping_add(1);
                crate::build::pattern::draw(&mut bench, what, asked.seed);
            }
            Press::Undo => {
                bench.undo();
            }
            Press::Save => match crate::build::kit::save(&mut bench) {
                Ok(path) => info!("saved the work to {}", path.display()),
                Err(why) => error!("could not save the work: {why}"),
            },
            Press::NextPicture => reference.next(),
            Press::FlipPicture => reference.flip(),
            Press::Fire => super::kiln::start(&reference, &mut firing),
            Press::Leave => next.set(AppState::Menu),
        }
    }
}

/// Picking a colour off the shelf.
pub fn pressed_swatch(
    swatches: Query<(&Interaction, &Swatch), Changed<Interaction>>,
    mut hand: ResMut<Hand>,
) {
    for (touch, swatch) in &swatches {
        if *touch == Interaction::Pressed {
            hand.tint = swatch.0.min(TINTS.len() - 1);
        }
    }
}

/// Keeps the panel saying what is actually true.
pub fn refresh(
    mut commands: Commands,
    hand: Res<Hand>,
    bench: Res<Bench>,
    reference: Res<super::reference::Reference>,
    firing: Res<super::kiln::Firing>,
    rows: Query<(Entity, &Choice<Press>, Option<&widget::Chosen>, &Children)>,
    mut labels: Query<&mut TextColor, With<RowLabel>>,
    mut backgrounds: Query<&mut BackgroundColor, With<Choice<Press>>>,
    mut values: Query<(&Readout, &mut Text)>,
    swatches: Query<(&Interaction, &Swatch, &mut BorderColor)>,
) {
    // The part in hand and the mode, both lit from what the tool actually holds —
    // so a key press and a click can never leave the panel disagreeing with it.
    // Nothing in hand lights nothing, which is the point: an empty cursor should
    // LOOK empty, or a maker cannot tell whether the next click will build.
    widget::mark_chosen(
        &mut commands,
        &rows,
        &mut labels,
        &mut backgrounds,
        &hand.part.map_or(Press::Leave, Press::Part),
    );
    for (entity, choice, _, kids) in &rows {
        if choice.0 == Press::Mode(hand.doing) {
            if let Ok(mut colour) = backgrounds.get_mut(entity) {
                colour.0 = widget::ROW_PRESS;
            }
            for kid in kids.iter() {
                if let Ok(mut text) = labels.get_mut(kid) {
                    text.0 = ACCENT;
                }
            }
        }
    }
    widget::light_swatches(swatches, hand.tint);

    for (which, mut text) in &mut values {
        let said = match which {
            // What is in hand comes FIRST, because it decides what the next click
            // does. An empty cursor that looks the same as a loaded one is a maker
            // guessing whether they are about to build.
            Readout::Cursor => format!(
                "{}  ·  {:.2}, {:.2}, {:.2}  q{}",
                match hand.part {
                    Some(part) => part.name(),
                    None => "EMPTY - pick a part",
                },
                hand.at.x,
                hand.at.y,
                hand.at.z,
                hand.quarters
            ),
            Readout::Pieces => format!(
                "{}{}",
                bench.len(),
                if bench.unsaved { "  UNSAVED" } else { "" }
            ),
            Readout::Picture => reference.said(),
            Readout::Kiln => {
                if firing.said.is_empty() {
                    "idle".to_string()
                } else {
                    firing.said.clone()
                }
            }
        };
        if **text != said {
            **text = said;
        }
    }
}

/// Colours the unsaved mark, so it reads as a warning rather than as text.
pub fn colour_unsaved(bench: Res<Bench>, mut values: Query<(&Readout, &mut TextColor)>) {
    for (which, mut colour) in &mut values {
        if *which == Readout::Pieces {
            colour.0 = if bench.unsaved { UNSAVED } else { TEXT_MUTED };
        }
    }
}
