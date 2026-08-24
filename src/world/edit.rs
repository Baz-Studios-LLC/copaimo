//! The hand-sculpted layer of the world.
//!
//! Generated terrain gets a plausible landscape; it doesn't get *this mountain,
//! here*. Authored geography lives in `assets/world/edits.bin`: a grid of signed
//! height offsets in meters, laid over whatever the map and the noise produced.
//!
//! # Offsets, not heights
//!
//! Each cell holds how far the ground moved, not where the ground is. That is
//! what lets the two coexist: re-roll the noise, redraw the map, retune the
//! mountains, and a hand-placed hill stays a hill riding on the new ground. A
//! grid of absolute heights would be invalidated by the next tuning pass, and
//! nobody would sculpt anything.
//!
//! # Where the brush lives
//!
//! In [`terrain_core::sculpt`], which this game and Opificium's terrain bench
//! **both** run — so ground shaped here and ground shaped there is shaped the
//! same way, by construction rather than by agreement. `src/editor/` is the mode
//! that drives it.
//!
//! What is left here is the part the crate deliberately does not do: knowing
//! where this game keeps its file, and saying so when something is wrong with
//! it. The crate reads and writes bytes; deciding they belong at
//! `assets/world/edits.bin` is the game's business.

use std::fs;
#[cfg(feature = "tools")]
use std::io;
use std::path::Path;

use bevy::log::{info, warn};
use bevy::prelude::*;

use crate::config::{EDITS_PATH, WORLD_SEED};

// The brush's own vocabulary is re-exported for the tools; a player's build
// still needs `Sculpt`, because ground somebody shaped is part of the world and
// is read at startup whether or not there is anything to shape it with.
#[cfg(feature = "tools")]
pub use terrain_core::sculpt::{Brushing, Patch, Stamp};
pub use terrain_core::sculpt::Sculpt;

/// Reads the sculpted ground, or an empty layer if there is none.
///
/// Every way this can go wrong ends the same — the world exactly as generated —
/// so the only real work is saying WHICH went wrong. A refused file and an
/// absent one look identical on screen otherwise.
pub fn load(half: Vec2) -> Sculpt {
    load_from(&crate::asset_file(EDITS_PATH), half)
}

/// Path-explicit form, so a test can read a fixture without touching the game's
/// own file.
pub fn load_from(path: &Path, half: Vec2) -> Sculpt {
    if !path.exists() {
        // The ordinary case for a world nobody has sculpted yet. Not news.
        return Sculpt::empty(half, WORLD_SEED);
    }

    match fs::read(path) {
        Ok(bytes) => match Sculpt::read(&bytes, half, WORLD_SEED) {
            Ok(sculpt) => {
                info!(
                    "sculpted ground: {} cells from {}",
                    sculpt.sculpted_cells(),
                    path.display()
                );
                sculpt
            }
            // Refused rather than stretched: offsets landing in the wrong places
            // would put hills in the sea and drop the ground out from under a
            // town, with nothing on screen to say why.
            Err(why) => {
                warn!("{}: {why} - using the world as generated", path.display());
                Sculpt::empty(half, WORLD_SEED)
            }
        },
        Err(why) => {
            warn!("{}: {why} - using the world as generated", path.display());
            Sculpt::empty(half, WORLD_SEED)
        }
    }
}

/// Writes the sculpted ground back to the game's own folder.
/// Writing a layer is a TOOL's job. A player's build reads what a maker left
/// and never writes any of it back, so this is not compiled into one.
#[cfg(feature = "tools")]
pub fn save(sculpt: &mut Sculpt) -> io::Result<()> {
    save_to(&crate::asset_file(EDITS_PATH), sculpt)
}

#[cfg(feature = "tools")]
pub fn save_to(path: &Path, sculpt: &mut Sculpt) -> io::Result<()> {
    if let Some(folder) = path.parent() {
        fs::create_dir_all(folder)?;
    }
    fs::write(path, sculpt.to_bytes())?;
    // Only once the bytes have actually landed — the crate has no way to know
    // whether they did, so it doesn't guess.
    sculpt.mark_saved();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const HALF: Vec2 = Vec2::new(400.0, 300.0);

    fn flat(_: Vec2) -> f32 {
        0.0
    }

    #[test]
    fn an_absent_file_is_an_empty_layer_not_an_error() {
        let sculpt = load_from(Path::new("no/such/edits.bin"), HALF);
        assert_eq!(sculpt.sculpted_cells(), 0);
        assert_eq!(sculpt.at(0.0, 0.0), 0.0);
    }

    #[test]
    fn a_file_that_is_not_ours_is_ignored_rather_than_read_as_elevation() {
        let road = std::env::temp_dir().join("copaimo-edits-foreign.bin");
        fs::write(&road, b"this is not sculpted ground at all").unwrap();
        assert_eq!(load_from(&road, HALF).sculpted_cells(), 0);
        let _ = fs::remove_file(&road);
    }

    // Gated with the tools, like the painting tests in world/terrain.rs.
    #[cfg(feature = "tools")]
    #[test]
    fn sculpting_survives_the_game_being_shut() {
        // The bench once had a writer, a passing round-trip test, and nothing
        // calling it — so an afternoon's planting vanished on restart. This is
        // that path end to end: sculpt, save through the game's own writer, and
        // read it back through the game's own reader.
        let road = std::env::temp_dir().join("copaimo-edits-restart.bin");
        let _ = fs::remove_file(&road);

        let mut sculpt = Sculpt::empty(HALF, WORLD_SEED);
        sculpt.apply(&Stamp {
            centre: Vec2::new(-100.0, 50.0),
            radius: 70.0,
            how: Brushing::Raise,
            amount: 18.0,
            target: 0.0,
            under: &flat,
        });
        save_to(&road, &mut sculpt).expect("it should write");
        assert!(!sculpt.unsaved, "saving clears the unsaved mark");

        let read = load_from(&road, HALF);
        assert_eq!(read.sculpted_cells(), sculpt.sculpted_cells());
        assert!(
            (read.at(-100.0, 50.0) - sculpt.at(-100.0, 50.0)).abs() < 1.0e-5,
            "the hill should come back where it was put"
        );

        let _ = fs::remove_file(&road);
    }
}
