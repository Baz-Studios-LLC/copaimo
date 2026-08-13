//! Visual language for the terrain tool.
//!
//! Kept in one place so the tool looks like one product rather than a pile of
//! panels, and so restyling it for another project is a single file.
//!
//! **Glyphs.** Bevy ships a subset font that covers little more than ASCII —
//! typographic characters like `·` and `—` render as empty boxes. Rather than
//! depend on a font that may not be there, the UI is built from ASCII text plus
//! real layout: rules are thin colored nodes, meters are nested nodes, and
//! keycaps are boxes with a letter in them. It reads better than punctuation
//! tricks would anyway, and it cannot break.

use std::path::Path;

use bevy::prelude::*;

use crate::world::edit::BrushOp;

/// Optional override font. Drop any `.ttf` at `assets/fonts/ui.ttf` and the
/// whole tool picks it up; without one it uses Bevy's built-in face.
const UI_FONT_PATH: &str = "fonts/ui.ttf";

// A restrained dark palette: the terrain is the subject, the tool is chrome.
pub const PANEL: Color = Color::srgba(0.055, 0.075, 0.098, 0.94);
pub const HEADER: Color = Color::srgba(0.020, 0.035, 0.055, 0.96);
pub const RULE: Color = Color::srgba(1.0, 1.0, 1.0, 0.07);
pub const ROW_ACTIVE: Color = Color::srgba(1.0, 1.0, 1.0, 0.06);
pub const KEYCAP: Color = Color::srgba(1.0, 1.0, 1.0, 0.09);
pub const METER_TRACK: Color = Color::srgba(1.0, 1.0, 1.0, 0.10);

pub const TEXT: Color = Color::srgb(0.90, 0.94, 0.97);
pub const TEXT_MUTED: Color = Color::srgb(0.55, 0.63, 0.71);
pub const TEXT_DIM: Color = Color::srgb(0.36, 0.43, 0.50);
pub const UNSAVED: Color = Color::srgb(1.00, 0.72, 0.30);

pub const PANEL_WIDTH: f32 = 316.0;

/// Each tool gets a color, used by the palette, its meters and the brush ring
/// in the world — so the ring under your crosshair always matches the
/// highlighted row without having to read anything.
pub fn tool_color(op: BrushOp) -> Color {
    match op {
        BrushOp::Raise => Color::srgb(0.42, 0.92, 0.55),
        BrushOp::Lower => Color::srgb(1.00, 0.55, 0.40),
        BrushOp::Smooth => Color::srgb(0.45, 0.72, 1.00),
        BrushOp::Flatten => Color::srgb(1.00, 0.85, 0.40),
        BrushOp::Path => Color::srgb(0.78, 0.62, 1.00),
        BrushOp::Roughen => Color::srgb(0.50, 0.92, 0.82),
    }
}

/// Handle to the UI font, if the project supplied one.
#[derive(Resource, Default)]
pub struct UiFont(Option<Handle<Font>>);

impl UiFont {
    /// A `TextFont` at the given size, using the override font when present.
    pub fn at(&self, size: f32) -> TextFont {
        TextFont {
            font: self.0.clone().unwrap_or_default(),
            font_size: size,
            ..default()
        }
    }
}

pub fn load_ui_font(mut commands: Commands, assets: Res<AssetServer>) {
    // Checked on disk rather than handed to the asset server blind, so a
    // missing font is a quiet fallback instead of a load error every run.
    let present = Path::new("assets").join(UI_FONT_PATH).exists();
    if present {
        info!("terrain tool using {UI_FONT_PATH}");
    }
    commands.insert_resource(UiFont(present.then(|| assets.load(UI_FONT_PATH))));
}

// ------------------------------------------------------------------ fragments

/// A one-pixel horizontal rule, for separating sections.
pub fn rule() -> impl Bundle {
    (
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(1.0),
            ..default()
        },
        BackgroundColor(RULE),
    )
}

/// A section heading: small, muted, wide-set.
pub fn section(font: &UiFont, label: &str) -> impl Bundle {
    (
        Text::new(label.to_string()),
        font.at(11.0),
        TextColor(TEXT_DIM),
    )
}

/// A boxed keycap, e.g. the `1` beside a tool or `S` in a shortcut.
pub fn keycap(parent: &mut ChildSpawnerCommands, font: &UiFont, key: &str) {
    parent
        .spawn((
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
