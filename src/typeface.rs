//! The face the game is set in.
//!
//! # Why this is not in `tools`
//!
//! It was, and that was a gap rather than a tidiness point: `tools` is compiled
//! out of a release, so the TITLE SCREEN — the one screen every player sees —
//! could not reach the font and fell back to whatever the default happens to be.
//! A maker's build looked like the game and a player's build looked like a
//! prototype, which is exactly backwards.
//!
//! So it lives here, always compiled, and the tools use it like anything else.

use bevy::prelude::*;
use std::path::Path;

/// Cinzel. Open Font License; the licence travels beside it in `assets/fonts/`.
///
/// Falls back to Bevy's built-in face when it is missing, so a stripped checkout
/// still runs — but that fallback is a monospace, and a monospace under a
/// chrome-and-serif wordmark reads as a terminal that has wandered onto the wrong
/// screen. It is a way to keep running, not a second design.
const UI_FONT_PATH: &str = "fonts/Cinzel.ttf";

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
    // Asked of the same root the asset server was given, so this cannot report a
    // font in use that the loader then fails to find — which is exactly what it
    // did while the two disagreed about where `assets` was.
    let present = Path::new(&crate::asset_root()).join(UI_FONT_PATH).exists();
    if present {
        info!("setting type in {UI_FONT_PATH}");
    } else {
        warn!("{UI_FONT_PATH} not found beside the working directory or the binary");
    }
    commands.insert_resource(UiFont(present.then(|| assets.load(UI_FONT_PATH))));
}

