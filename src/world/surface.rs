//! What the ground is made of, where somebody decided it.
//!
//! The heightfield says where the ground *is*; the biome says what it looks like
//! from its height, slope and moisture. Neither can say "there is a road here",
//! because a road is not a shape and not a climate — it is a decision, and the
//! only place a decision can live is a layer somebody painted.
//!
//! Signed bias in [`terrain_core::painted`]'s grid, where **zero leaves the
//! biome's own answer alone**: positive wears the ground down to bare earth,
//! negative forces green back over it. Four-metre cells, four times finer than
//! the woods, because a cart road is about six metres wide and a sixteen-metre
//! cell draws a field.
//!
//! What is left here is the part the crate deliberately does not do: knowing
//! where this game keeps its file, and saying so when something is wrong with
//! it.

use bevy::log::{info, warn};
use bevy::prelude::*;
#[cfg(feature = "tools")]
use std::io;
use std::path::Path;

use crate::config::SURFACE_PATH;

pub use terrain_core::painted::{Kind, Painted};

/// An empty layer: the ground exactly as the biome would paint it.
pub fn empty(half: Vec2) -> Painted {
    Painted::empty(Kind::Surface, half)
}

/// Reads the surface a maker laid, or an empty layer if there is none.
pub fn load(half: Vec2) -> Painted {
    let path = Path::new(SURFACE_PATH);
    if !path.exists() {
        // The ordinary case for a world with no roads worn into it yet.
        return empty(half);
    }

    match std::fs::read(path) {
        Ok(bytes) => match Painted::read(&bytes, Kind::Surface, half) {
            Ok(painted) => {
                info!(
                    "worn surface: {} cells from {}",
                    painted.painted_cells(),
                    path.display()
                );
                painted
            }
            // Refused rather than stretched. Dirt in the wrong places is worse
            // than none: roads across hillsides and green through a town square.
            Err(why) => {
                warn!("{}: {why} - taking the biome's own answer", path.display());
                empty(half)
            }
        },
        Err(why) => {
            warn!("{}: {why} - taking the biome's own answer", path.display());
            empty(half)
        }
    }
}

/// Writing a layer is a TOOL's job. A player's build reads what a maker left
/// and never writes any of it back, so this is not compiled into one.
#[cfg(feature = "tools")]
pub fn save(painted: &mut Painted) -> io::Result<()> {
    let path = Path::new(SURFACE_PATH);
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder)?;
    }
    std::fs::write(path, painted.to_bytes())?;
    painted.mark_saved();
    Ok(())
}
