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

use crate::world::edit::Brushing;

/// Cinzel, which is what Opificium letters its benches with — carried across so
/// the terrain mode here and the terrain bench there are recognisably the same
/// tool. Open Font License; the licence travels beside it in `assets/fonts/`.
///
/// Falls back to Bevy's built-in face if it is missing, so a stripped checkout
/// still runs. Older note kept because it is still true: drop any `.ttf` at this
/// whole tool picks it up; without one it uses Bevy's built-in face.
const UI_FONT_PATH: &str = "fonts/Cinzel.ttf";

// A restrained dark palette: the terrain is the subject, the tool is chrome.
// Opificium's own, so the two read as one workshop rather than two programs
// that happen to edit the same ground. Near-black panels, gold on them, and bone
// for the writing — taken from `src/look.rs` in that repository.
//
// Gold is the accent for EVERYTHING here, which is why the tools below are told
// apart by their own colours rather than by it.
pub const PANEL: Color = Color::srgba(0.045, 0.050, 0.062, 0.985);
pub const HEADER: Color = Color::srgba(0.028, 0.032, 0.042, 0.99);
pub const RULE: Color = Color::srgba(0.83, 0.68, 0.34, 0.22);
pub const ROW_ACTIVE: Color = Color::srgba(0.83, 0.68, 0.34, 0.13);
pub const KEYCAP: Color = Color::srgba(0.83, 0.68, 0.34, 0.16);
pub const METER_TRACK: Color = Color::srgba(0.83, 0.68, 0.34, 0.16);

/// Bone, not white. Paper-coloured writing on near-black is what gives the bench
/// its look; pure white on it reads as a terminal.
pub const TEXT: Color = Color::srgb(0.93, 0.90, 0.83);
pub const TEXT_MUTED: Color = Color::srgb(0.72, 0.68, 0.60);
pub const TEXT_DIM: Color = Color::srgb(0.48, 0.45, 0.40);
/// The gold the panels are edged and lit with.
pub const ACCENT: Color = Color::srgb(0.83, 0.68, 0.34);
pub const UNSAVED: Color = Color::srgb(0.86, 0.56, 0.22);

/// As wide as Opificium's own panels stand, for the same reason it gives: a
/// bench with mismatched margins reads as one that was assembled rather than
/// drawn. Wider here than there, because this sidebar carries the readouts that
/// repository puts in a second panel.
pub const PANEL_WIDTH: f32 = 300.0;

/// Each tool gets a color, used by the palette, its meters and the brush ring
/// in the world — so the ring under your crosshair always matches the
/// highlighted row without having to read anything.
///
/// The bench's own colours for the nine it shares, in the same order: it names them
/// out of the open game's palette (`grass`, `cloth-rust`, `earth`, `foliage`)
/// and this names them in figures, but a maker moving between the two should
/// never have to re-learn which colour means ERODE.
pub fn tool_color(how: Brushing) -> Color {
    match how {
        Brushing::Raise => Color::srgb(0.42, 0.92, 0.55),
        Brushing::Lower => Color::srgb(1.00, 0.55, 0.40),
        Brushing::Smooth => Color::srgb(0.45, 0.72, 1.00),
        Brushing::Flatten => Color::srgb(1.00, 0.85, 0.40),
        Brushing::Path => Color::srgb(0.78, 0.62, 1.00),
        Brushing::Roughen => Color::srgb(0.50, 0.92, 0.82),
        Brushing::Erode => Color::srgb(0.74, 0.56, 0.36),
        Brushing::Ramp => Color::srgb(1.00, 0.62, 0.78),
        // Deeper than RAISE's green on purpose. They are the two greens on the
        // shelf and one moves earth while the other grows woods, so telling them
        // apart at a glance matters more than either being pretty.
        Brushing::Plant => Color::srgb(0.34, 0.68, 0.36),
        // Grey, and the only one on the shelf that is. Every other tool makes
        // something and wears a colour; this one takes making back out.
        Brushing::Revert => Color::srgb(0.72, 0.74, 0.78),
        // Sand, because the desert is the country a maker reaches for this brush
        // to draw. Which country it is actually laying is said in words on the
        // shelf — a single swatch cannot show three.
        Brushing::Country => Color::srgb(0.86, 0.74, 0.46),
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
    // Checked on disk rather than handed to the asset server blind, so a missing
    // font is a quiet fallback instead of a load error every run.
    //
    // Both roots, because the check and the load do not agree about where `assets`
    // is: this looks beside the working directory and the asset server looks beside
    // the executable. Running through cargo those are the same place and running
    // the binary directly they are not, which showed up as a font that existed and
    // an error saying it did not.
    let beside_cwd = Path::new("assets").join(UI_FONT_PATH);
    let beside_exe = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|at| at.join("assets").join(UI_FONT_PATH)));
    let present =
        beside_cwd.exists() || beside_exe.is_some_and(|road| road.exists());
    if present {
        info!("tool panels using {UI_FONT_PATH}");
    } else {
        warn!("{UI_FONT_PATH} not found beside the working directory or the binary");
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

// The keycap moved into `widget::row`, which is the only thing that ever drew one:
// a cap belongs to the row it labels, and a free function that spawns one was an
// invitation to draw keycaps that no row was under.
