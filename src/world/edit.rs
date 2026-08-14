//! The hand-sculpted layer of the world, as the game reads it.
//!
//! Generated terrain gets a plausible landscape; it doesn't get *this mountain,
//! here*. Authored geography lives in `assets/world/edits.bin`: a grid of signed
//! height offsets in meters, laid over whatever the map and the noise produced.
//!
//! **The game only reads this file. It is written by Opificium**, the studio's
//! maker's bench, at its terrain bench — see `DESIGN.md`. The two programs share
//! no code, only the file, and its layout is written down in Opificium's
//! `FORMATS.md`. Nothing here sculpts, because the game is not where sculpting
//! happens.
//!
//! # Offsets, not heights
//!
//! Each cell holds how far the ground moved, not where the ground is. That is
//! what lets the two coexist: re-roll the noise, redraw the map, retune the
//! mountains, and a hand-placed hill stays a hill riding on the new ground. A
//! grid of absolute heights would be invalidated by the next tuning pass, and
//! nobody would sculpt anything.

use std::fs;
use std::path::Path;

use bevy::log::{info, warn};
use bevy::prelude::*;

use crate::config::{EDITS_PATH, EDIT_CELL};

/// Names the file, so a stale or unrelated one is refused rather than read as
/// garbage elevation.
const MAGIC: &[u8; 8] = b"RNGREDT1";

/// Below this, a cell counts as untouched ground.
const SCULPT_EPSILON: f32 = 0.01;

pub struct EditGrid {
    width: usize,
    height: usize,
    half: Vec2,
    /// Signed height offset in meters, row-major, north row first.
    offsets: Vec<f32>,
    sculpted: usize,
}

impl EditGrid {
    /// An empty layer: the world exactly as generated.
    pub fn empty(half: Vec2) -> Self {
        let width = (half.x * 2.0 / EDIT_CELL).ceil() as usize + 1;
        let height = (half.y * 2.0 / EDIT_CELL).ceil() as usize + 1;
        Self {
            width,
            height,
            half,
            offsets: vec![0.0; width * height],
            sculpted: 0,
        }
    }

    pub fn load(half: Vec2) -> Self {
        Self::load_from(Path::new(EDITS_PATH), half)
    }

    /// Path-explicit form, so tests can read a fixture without touching the
    /// game's own file.
    pub fn load_from(path: &Path, half: Vec2) -> Self {
        let mut empty = Self::empty(half);
        if !path.exists() {
            // The ordinary case for a world nobody has sculpted yet, not a
            // failure. Silent, because it is not news.
            return empty;
        }

        let bytes = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(err) => {
                warn!("{}: {err} - using the world as generated", path.display());
                return empty;
            }
        };

        let header = 8 + 4 * 4;
        if bytes.len() < header || &bytes[..8] != MAGIC {
            warn!("{} is not sculpted ground - ignoring it", path.display());
            return empty;
        }

        let word = |at: usize| {
            u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]) as usize
        };
        let real = |at: usize| {
            f32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
        };
        let (width, height) = (word(8), word(12));
        let saved_half = Vec2::new(real(16), real(20));

        // Refused rather than stretched. Offsets landing in the wrong places
        // would be worse than none: hills would appear in the sea and the
        // ground under a town would drop away, with nothing to say why.
        if width != empty.width || height != empty.height || saved_half.distance(half) > 1.0 {
            warn!(
                "{} was sculpted for a {:.0}x{:.0} m world but this one is {:.0}x{:.0} m \
                 - ignoring it rather than putting the ground in the wrong place. \
                 Re-sculpt it at Opificium's terrain bench.",
                path.display(),
                saved_half.x * 2.0,
                saved_half.y * 2.0,
                half.x * 2.0,
                half.y * 2.0
            );
            return empty;
        }

        if bytes.len() < header + width * height * 4 {
            warn!("{} is truncated - ignoring it", path.display());
            return empty;
        }

        empty.offsets = (0..width * height).map(|i| real(header + i * 4)).collect();
        empty.sculpted = empty
            .offsets
            .iter()
            .filter(|v| v.abs() > SCULPT_EPSILON)
            .count();

        info!(
            "sculpted ground: {} cells from {}",
            empty.sculpted,
            path.display()
        );
        empty
    }

    /// How many cells have been moved off zero.
    pub fn sculpted_cells(&self) -> usize {
        self.sculpted
    }

    /// The offset at a world position, read between cells. Off the grid reads as
    /// zero, so the open ocean past the world's edge is never lifted.
    pub fn sample(&self, x: f32, z: f32) -> f32 {
        let fx = (x + self.half.x) / EDIT_CELL;
        let fz = (z + self.half.y) / EDIT_CELL;
        if fx < 0.0 || fz < 0.0 || fx > (self.width - 1) as f32 || fz > (self.height - 1) as f32 {
            return 0.0;
        }

        let x0 = fx.floor() as usize;
        let z0 = fz.floor() as usize;
        let x1 = (x0 + 1).min(self.width - 1);
        let z1 = (z0 + 1).min(self.height - 1);
        let tx = fx - x0 as f32;
        let tz = fz - z0 as f32;

        let at = |x: usize, z: usize| self.offsets[z * self.width + x];
        let near = at(x0, z0) * (1.0 - tx) + at(x1, z0) * tx;
        let far = at(x0, z1) * (1.0 - tx) + at(x1, z1) * tx;
        near * (1.0 - tz) + far * tz
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    const HALF: Vec2 = Vec2::new(400.0, 300.0);

    /// Writes a file in the shape `FORMATS.md` describes, so the reader is
    /// tested against the contract rather than against its own writer — which
    /// now lives in another program entirely.
    fn write_fixture(path: &Path, half: Vec2, fill: impl Fn(usize, usize) -> f32) {
        let width = (half.x * 2.0 / EDIT_CELL).ceil() as usize + 1;
        let height = (half.y * 2.0 / EDIT_CELL).ceil() as usize + 1;
        let mut file = fs::File::create(path).expect("fixture should open");
        file.write_all(MAGIC).unwrap();
        file.write_all(&(width as u32).to_le_bytes()).unwrap();
        file.write_all(&(height as u32).to_le_bytes()).unwrap();
        file.write_all(&half.x.to_le_bytes()).unwrap();
        file.write_all(&half.y.to_le_bytes()).unwrap();
        for z in 0..height {
            for x in 0..width {
                file.write_all(&fill(x, z).to_le_bytes()).unwrap();
            }
        }
    }

    #[test]
    fn an_absent_file_is_an_empty_layer_not_an_error() {
        let grid = EditGrid::load_from(Path::new("no/such/edits.bin"), HALF);
        assert_eq!(grid.sculpted_cells(), 0);
        assert_eq!(grid.sample(0.0, 0.0), 0.0);
    }

    #[test]
    fn sculpted_ground_reads_back_where_it_was_put() {
        let road = std::env::temp_dir().join("ranger-edits-read.bin");
        // One raised cell, so its position can be checked and not just its value.
        let raised = (10, 8);
        write_fixture(&road, HALF, |x, z| {
            if (x, z) == raised { 12.0 } else { 0.0 }
        });

        let grid = EditGrid::load_from(&road, HALF);
        assert_eq!(grid.sculpted_cells(), 1);

        let at = Vec2::new(
            raised.0 as f32 * EDIT_CELL - HALF.x,
            raised.1 as f32 * EDIT_CELL - HALF.y,
        );
        assert!(
            (grid.sample(at.x, at.y) - 12.0).abs() < 1.0e-4,
            "the raised cell should read back at its own position"
        );
        // Bilinear, so a neighbouring cell is partly lifted and one further out
        // is not lifted at all.
        assert!(grid.sample(at.x + EDIT_CELL * 0.5, at.y) > 4.0);
        assert!(grid.sample(at.x + EDIT_CELL * 2.0, at.y).abs() < SCULPT_EPSILON);

        let _ = fs::remove_file(&road);
    }

    #[test]
    fn ground_sculpted_for_another_world_is_refused_not_stretched() {
        let road = std::env::temp_dir().join("ranger-edits-mismatch.bin");
        write_fixture(&road, HALF, |_, _| 20.0);

        // Same file, a world twice the size. Every offset would land in the
        // wrong place, so none of them are used.
        let grid = EditGrid::load_from(&road, HALF * 2.0);
        assert_eq!(grid.sculpted_cells(), 0);
        assert_eq!(grid.sample(0.0, 0.0), 0.0);

        let _ = fs::remove_file(&road);
    }

    #[test]
    fn a_file_that_is_not_ours_is_ignored() {
        let road = std::env::temp_dir().join("ranger-edits-foreign.bin");
        fs::write(&road, b"this is not sculpted ground at all").unwrap();
        assert_eq!(EditGrid::load_from(&road, HALF).sculpted_cells(), 0);
        let _ = fs::remove_file(&road);
    }
}
