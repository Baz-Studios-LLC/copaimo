//! Which country the ground belongs to, where somebody decided it.
//!
//! # Why a maker paints this rather than a programmer placing it
//!
//! The regions were placed in code — a band across the east, an oval in the
//! middle — and moving one meant reading a marker's position off a screenshot,
//! guessing which number that implied, and nudging it. That went wrong five times
//! in a row over a single evening, and not through carelessness: the person who
//! can SEE where the desert belongs and the person who can edit the constant were
//! not the same person, and a picture is not a coordinate.
//!
//! A brush closes that loop. The maker paints the country they mean, exactly
//! where they mean it, and nobody has to translate.
//!
//! **Where nothing is painted the code still answers**, so a fresh world still
//! has a world in it — [`terrain_core::region`] keeps its bands and its desert as
//! the default rather than as the truth.
//!
//! What is left here is the part the crate deliberately does not do: knowing
//! where this game keeps its file, and saying so when something is wrong with it.

use bevy::log::{info, warn};
use bevy::prelude::*;
use std::io;
use std::path::Path;

use crate::config::COUNTRY_PATH;

pub use terrain_core::painted::{Kind, Painted};

/// An empty layer: the countries exactly as the world would lay them out.
pub fn empty(half: Vec2) -> Painted {
    Painted::empty(Kind::Country, half)
}

/// Reads the countries a maker painted, or an empty layer if there are none.
pub fn load(half: Vec2) -> Painted {
    let path = Path::new(COUNTRY_PATH);
    if !path.exists() {
        // The ordinary case for a world nobody has painted yet.
        return empty(half);
    }

    match std::fs::read(path) {
        Ok(bytes) => match Painted::read(&bytes, Kind::Country, half) {
            Ok(painted) => {
                info!(
                    "painted country: {} cells from {}",
                    painted.painted_cells(),
                    path.display()
                );
                painted
            }
            // Refused rather than stretched. A misread country layer is a desert
            // in the starting meadow and snow on the ranch — worse by a long way
            // than the generated regions it would be replacing.
            Err(why) => {
                warn!("{}: {why} - taking the world's own regions", path.display());
                empty(half)
            }
        },
        Err(why) => {
            warn!("{}: {why} - taking the world's own regions", path.display());
            empty(half)
        }
    }
}

pub fn save(painted: &mut Painted) -> io::Result<()> {
    let path = Path::new(COUNTRY_PATH);
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder)?;
    }
    std::fs::write(path, painted.to_bytes())?;
    painted.mark_saved();
    Ok(())
}
