//! The woods, as the game reads them.
//!
//! Where trees stand is decided by [`terrain_core::forest`], which this game and
//! Opificium's terrain bench **both** run — so the forest here and the forest at
//! the bench are the same forest, by construction rather than by agreement.
//!
//! What is left here is the part the crate deliberately does not do: knowing
//! where this game keeps its file, and saying so when something is wrong with
//! it. The crate reads bytes; deciding they came from `assets/world/forest.bin`,
//! and logging it, is the game's business.

use bevy::log::{info, warn};
use bevy::prelude::*;
#[cfg(feature = "tools")]
use std::io;

use crate::config::FOREST_PATH;

pub use terrain_core::forest::{chance, density, natural_density, Painted, Planted};

/// Reads the woods planted at the bench, or an empty layer if there are none.
///
/// Every way this can go wrong ends the same — the world as the ground alone
/// would have it — so the only real work is saying WHICH went wrong. A refused
/// file and an absent one look identical on screen otherwise.
pub fn load(half: Vec2) -> Painted {
    let path = crate::asset_file(FOREST_PATH);
    let path = path.as_path();
    if !path.exists() {
        // The ordinary case for a world nobody has planted. Not news.
        return terrain_core::forest::empty(half);
    }

    match std::fs::read(path) {
        Ok(bytes) => match terrain_core::forest::read(&bytes, half) {
            Ok(painted) => {
                info!("planted woods: {} cells from {}", painted.painted_cells(), path.display());
                painted
            }
            // Refused rather than stretched: woods landing in the wrong places
            // is worse than none, and nothing on screen would say why.
            Err(why) => {
                warn!("{}: {why} - taking the ground's own answer", path.display());
                terrain_core::forest::empty(half)
            }
        },
        Err(why) => {
            warn!("{}: {why} - taking the ground's own answer", path.display());
            terrain_core::forest::empty(half)
        }
    }
}

/// Writes the planted woods back.
///
/// The bench once had a writer, a passing round-trip test, and nothing calling
/// it — so an afternoon's planting went away on restart. Saving the ground and
/// saving the woods happen together, in one keystroke, for that reason.
/// Writing a layer is a TOOL's job. A player's build reads what a maker left
/// and never writes any of it back, so this is not compiled into one.
#[cfg(feature = "tools")]
pub fn save(painted: &mut Painted) -> io::Result<()> {
    let path = crate::asset_file(FOREST_PATH);
    let path = path.as_path();
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder)?;
    }
    std::fs::write(path, painted.to_bytes())?;
    // Only once the bytes have actually landed — the crate has no way to know
    // whether they did, so it doesn't guess.
    painted.mark_saved();
    Ok(())
}
