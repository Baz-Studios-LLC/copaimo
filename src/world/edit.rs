//! The hand-edited layer of the world.
//!
//! Generated terrain gets you a plausible landscape; it doesn't get you *this
//! mountain, here*. This layer is where authored geography lives: a grid of
//! signed height offsets in meters, added on top of whatever the generator
//! produced, sculpted in-game with a brush and saved to disk.
//!
//! Storing offsets rather than absolute heights is what makes the two coexist.
//! Re-roll the noise or swap the map image and hand-placed hills stay where you
//! put them, riding on top of the new ground instead of being overwritten by it.
//!
//! This module is deliberately free of any dependency on the rest of the game —
//! it knows about a grid and a brush, nothing about rangers or monsters. The
//! sculpting tool in `src/editor/` is the same: together they're the piece that
//! could move to its own crate when a second project wants it.

use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

use bevy::prelude::*;
use noise::{NoiseFn, Perlin};

use crate::config::{EDITS_PATH, EDIT_CELL, WORLD_SEED};
use crate::util::smoothstep;

/// File format marker, so a stale or unrelated file is rejected rather than
/// read as garbage elevation.
const MAGIC: &[u8; 8] = b"RNGREDT1";

/// Below this, an offset is treated as untouched ground.
const SCULPT_EPSILON: f32 = 0.01;

/// How many strokes can be undone. Each stores only the cells it touched, so
/// this bounds memory by area painted rather than by world size.
const UNDO_DEPTH: usize = 64;

/// Spatial frequency of the Roughen brush's noise, in cycles per meter.
const ROUGHEN_FREQ: f64 = 0.05;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BrushOp {
    Raise,
    Lower,
    /// Blend toward the average height nearby.
    Smooth,
    /// Level to a fixed height, with a soft dish profile.
    Flatten,
    /// Level to a fixed height with a flat bed and short shoulders — roads,
    /// trails, terraces, and pads for buildings to sit on.
    Path,
    /// Add fractal detail, for breaking up ground that's been sculpted smooth.
    Roughen,
}

impl BrushOp {
    pub const ALL: [BrushOp; 6] = [
        BrushOp::Raise,
        BrushOp::Lower,
        BrushOp::Smooth,
        BrushOp::Flatten,
        BrushOp::Path,
        BrushOp::Roughen,
    ];

    pub fn name(self) -> &'static str {
        match self {
            BrushOp::Raise => "Raise",
            BrushOp::Lower => "Lower",
            BrushOp::Smooth => "Smooth",
            BrushOp::Flatten => "Flatten",
            BrushOp::Path => "Path",
            BrushOp::Roughen => "Roughen",
        }
    }

    /// Short enough to sit on one line in the tool palette without wrapping.
    pub fn hint(self) -> &'static str {
        match self {
            BrushOp::Raise => "push ground up",
            BrushOp::Lower => "pull ground down",
            BrushOp::Smooth => "average out bumps",
            BrushOp::Flatten => "level to click height",
            BrushOp::Path => "flat-bottomed road cut",
            BrushOp::Roughen => "add natural detail",
        }
    }

    /// Whether the tool pushes at a speed (Raise, Lower, Roughen) or converges
    /// on a target (Smooth, Flatten, Path). The two want different `amount`
    /// scaling, and the editor uses this to pick.
    pub fn is_directional(self) -> bool {
        matches!(self, BrushOp::Raise | BrushOp::Lower | BrushOp::Roughen)
    }

    /// Falloff from the brush center to its rim.
    fn falloff(self, distance: f32, radius: f32) -> f32 {
        match self {
            // A flat bed out to 70% of the radius, then quick shoulders — the
            // difference between a road cut and a soft dish.
            BrushOp::Path => smoothstep(radius, radius * 0.7, distance),
            _ => smoothstep(radius, 0.0, distance),
        }
    }
}

/// One tick of a brush stroke.
pub struct Stamp<'a> {
    pub center: Vec2,
    pub radius: f32,
    pub op: BrushOp,
    /// Meters per tick for directional tools, blend fraction for the rest.
    pub amount: f32,
    /// Height that Flatten and Path level toward.
    pub target: f32,
    /// The *generated* height at a point, with edits excluded.
    ///
    /// Smooth, Flatten and Path need it because they work on the finished
    /// surface — the offset they want to write depends on what the ground was
    /// doing underneath. It must not consult the edit layer, or it would
    /// deadlock against the lock the caller is already holding.
    pub base: &'a dyn Fn(Vec2) -> f32,
}

/// Cells changed by one stroke, with the values they held beforehand.
struct Stroke {
    before: HashMap<usize, f32>,
}

pub struct EditGrid {
    width: usize,
    height: usize,
    half: Vec2,
    /// Signed height offset in meters, row-major, north row first.
    offsets: Vec<f32>,
    /// Running count of cells sculpted away from zero. Maintained as cells are
    /// written rather than recounted on demand — the HUD asks for it every
    /// frame, and the grid is over two million cells.
    sculpted: usize,
    /// Whether there are changes not yet written to disk.
    pub unsaved: bool,

    /// Cells modified since the current stroke began, and their prior values.
    /// `None` between strokes.
    recording: Option<HashMap<usize, f32>>,
    undo_stack: Vec<Stroke>,
    redo_stack: Vec<Stroke>,

    noise: Perlin,
}

impl EditGrid {
    pub fn new(half: Vec2) -> Self {
        let width = (half.x * 2.0 / EDIT_CELL).ceil() as usize + 1;
        let height = (half.y * 2.0 / EDIT_CELL).ceil() as usize + 1;
        Self {
            width,
            height,
            half,
            offsets: vec![0.0; width * height],
            sculpted: 0,
            unsaved: false,
            recording: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            noise: Perlin::new(WORLD_SEED.wrapping_add(11)),
        }
    }

    // ------------------------------------------------------------- persistence

    pub fn load(half: Vec2) -> Self {
        Self::load_from(Path::new(EDITS_PATH), half)
    }

    /// Path-explicit form, so tests can round-trip without touching the real
    /// save file.
    pub fn load_from(path: &Path, half: Vec2) -> Self {
        let mut empty = Self::new(half);
        if !path.exists() {
            return empty;
        }

        let bytes = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(err) => {
                warn!(
                    "could not read {} ({err}) — starting with no edits",
                    path.display()
                );
                return empty;
            }
        };

        let header = 8 + 4 * 4;
        if bytes.len() < header || &bytes[..8] != MAGIC {
            warn!("{} is not a terrain edit file — ignoring it", path.display());
            return empty;
        }

        let read_u32 = |at: usize| {
            u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]) as usize
        };
        let read_f32 = |at: usize| {
            f32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
        };
        let (width, height) = (read_u32(8), read_u32(12));
        let saved_half = Vec2::new(read_f32(16), read_f32(20));

        if width != empty.width || height != empty.height || saved_half.distance(half) > 1.0 {
            warn!(
                "{} was saved for a {:.0}x{:.0} m world but this one is {:.0}x{:.0} m \
                 — ignoring it rather than putting the edits in the wrong place",
                path.display(),
                saved_half.x * 2.0,
                saved_half.y * 2.0,
                half.x * 2.0,
                half.y * 2.0
            );
            return empty;
        }

        if bytes.len() < header + width * height * 4 {
            warn!("{} is truncated — ignoring it", path.display());
            return empty;
        }

        empty.offsets = (0..width * height)
            .map(|i| read_f32(header + i * 4))
            .collect();
        // Counted once here; from now on `write` keeps it current.
        empty.sculpted = empty
            .offsets
            .iter()
            .filter(|v| v.abs() > SCULPT_EPSILON)
            .count();

        info!(
            "loaded terrain edits from {} ({} cells sculpted)",
            path.display(),
            empty.sculpted
        );
        empty
    }

    pub fn save(&mut self) -> io::Result<()> {
        self.save_to(Path::new(EDITS_PATH))
    }

    pub fn save_to(&mut self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut file = fs::File::create(path)?;
        file.write_all(MAGIC)?;
        file.write_all(&(self.width as u32).to_le_bytes())?;
        file.write_all(&(self.height as u32).to_le_bytes())?;
        file.write_all(&self.half.x.to_le_bytes())?;
        file.write_all(&self.half.y.to_le_bytes())?;
        for offset in &self.offsets {
            file.write_all(&offset.to_le_bytes())?;
        }
        file.flush()?;

        self.unsaved = false;
        Ok(())
    }

    // ------------------------------------------------------------------ queries

    /// How many cells have been sculpted away from zero.
    pub fn sculpted_cells(&self) -> usize {
        self.sculpted
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Bilinear offset lookup at a world position. Off-grid reads as zero, so
    /// the open ocean past the world border is never accidentally sculpted.
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
        let top = at(x0, z0) * (1.0 - tx) + at(x1, z0) * tx;
        let bottom = at(x0, z1) * (1.0 - tx) + at(x1, z1) * tx;
        top * (1.0 - tz) + bottom * tz
    }

    // -------------------------------------------------------------- undo / redo

    /// Called when a stroke begins. Everything written until `end_stroke` is
    /// recorded as one undoable unit, so a long drag undoes in one step rather
    /// than one step per frame.
    pub fn begin_stroke(&mut self) {
        self.recording = Some(HashMap::new());
        // A new edit invalidates any redo history, the same as every editor.
        self.redo_stack.clear();
    }

    pub fn end_stroke(&mut self) {
        let Some(before) = self.recording.take() else {
            return;
        };
        if before.is_empty() {
            return;
        }
        self.undo_stack.push(Stroke { before });
        if self.undo_stack.len() > UNDO_DEPTH {
            self.undo_stack.remove(0);
        }
    }

    /// Reverts the last stroke. Returns the world-space area that changed, so
    /// the caller knows which chunks to re-mesh.
    pub fn undo(&mut self) -> Option<Rect> {
        let stroke = self.undo_stack.pop()?;
        let inverse = self.restore(&stroke.before);
        let area = self.area_of(&stroke.before);
        self.redo_stack.push(Stroke { before: inverse });
        Some(area)
    }

    pub fn redo(&mut self) -> Option<Rect> {
        let stroke = self.redo_stack.pop()?;
        let inverse = self.restore(&stroke.before);
        let area = self.area_of(&stroke.before);
        self.undo_stack.push(Stroke { before: inverse });
        Some(area)
    }

    /// Writes a set of saved cell values back, returning what was there before
    /// so the operation can be reversed again.
    fn restore(&mut self, values: &HashMap<usize, f32>) -> HashMap<usize, f32> {
        let mut inverse = HashMap::with_capacity(values.len());
        for (&index, &value) in values {
            inverse.insert(index, self.offsets[index]);
            self.write_raw(index, value);
        }
        self.unsaved = true;
        inverse
    }

    /// World-space bounds of a set of cells, padded by one cell because
    /// sampling is bilinear and reaches a cell beyond what was written.
    fn area_of(&self, values: &HashMap<usize, f32>) -> Rect {
        let mut min = Vec2::splat(f32::MAX);
        let mut max = Vec2::splat(f32::MIN);
        for &index in values.keys() {
            let position = self.cell_position(index % self.width, index / self.width);
            min = min.min(position);
            max = max.max(position);
        }
        Rect::from_corners(min - EDIT_CELL, max + EDIT_CELL)
    }

    // -------------------------------------------------------------- application

    /// Applies one brush stroke tick. Returns the world-space area affected.
    pub fn apply(&mut self, stamp: &Stamp) -> Rect {
        let (x0, x1, z0, z1) = self.cells_in_range(stamp.center, stamp.radius);

        // Smoothing reads neighbours while writing, so it is computed into a
        // scratch buffer first and applied in a second pass. Otherwise cells
        // would smooth against values already smoothed this tick, which drags
        // the whole stroke in whatever order the loop happens to run.
        let mut deferred: Vec<(usize, f32)> = Vec::new();

        for z in z0..=z1 {
            for x in x0..=x1 {
                let position = self.cell_position(x, z);
                let distance = position.distance(stamp.center);
                if distance > stamp.radius {
                    continue;
                }

                let falloff = stamp.op.falloff(distance, stamp.radius);
                if falloff <= 0.0 {
                    continue;
                }

                let index = z * self.width + x;
                let current = self.offsets[index];

                match stamp.op {
                    BrushOp::Raise => self.write(index, current + stamp.amount * falloff),
                    BrushOp::Lower => self.write(index, current - stamp.amount * falloff),
                    BrushOp::Roughen => {
                        let n = self.noise.get([
                            position.x as f64 * ROUGHEN_FREQ,
                            position.y as f64 * ROUGHEN_FREQ,
                        ]) as f32;
                        self.write(index, current + n * stamp.amount * falloff);
                    }
                    BrushOp::Flatten | BrushOp::Path => {
                        let wanted = stamp.target - (stamp.base)(position);
                        let t = (stamp.amount * falloff).clamp(0.0, 1.0);
                        self.write(index, current + (wanted - current) * t);
                    }
                    BrushOp::Smooth => {
                        let average = self.neighbourhood_height(x, z, stamp.base);
                        let wanted = average - (stamp.base)(position);
                        let t = (stamp.amount * falloff).clamp(0.0, 1.0);
                        deferred.push((index, current + (wanted - current) * t));
                    }
                }
            }
        }

        for (index, value) in deferred {
            self.write(index, value);
        }

        self.unsaved = true;
        Rect::from_corners(
            stamp.center - (stamp.radius + EDIT_CELL),
            stamp.center + (stamp.radius + EDIT_CELL),
        )
    }

    /// Writes one cell, recording its prior value for undo and keeping the
    /// sculpted-cell count in step.
    fn write(&mut self, index: usize, value: f32) {
        if let Some(recording) = &mut self.recording {
            // Only the value from before the stroke started, so replaying a
            // long drag backwards lands on the right ground.
            recording.entry(index).or_insert(self.offsets[index]);
        }
        self.write_raw(index, value);
    }

    /// Writes without touching undo history — used by undo itself, which is
    /// already managing the history around the call.
    fn write_raw(&mut self, index: usize, value: f32) {
        let was = self.offsets[index].abs() > SCULPT_EPSILON;
        let is = value.abs() > SCULPT_EPSILON;
        match (was, is) {
            (false, true) => self.sculpted += 1,
            (true, false) => self.sculpted -= 1,
            _ => {}
        }
        self.offsets[index] = value;
    }

    /// World position of a grid cell.
    fn cell_position(&self, x: usize, z: usize) -> Vec2 {
        Vec2::new(
            x as f32 * EDIT_CELL - self.half.x,
            z as f32 * EDIT_CELL - self.half.y,
        )
    }

    /// Grid cells touched by a brush, clamped to the grid.
    fn cells_in_range(&self, center: Vec2, radius: f32) -> (usize, usize, usize, usize) {
        let to_cell = |v: f32, half: f32, count: usize| {
            (((v + half) / EDIT_CELL).floor() as isize).clamp(0, count as isize - 1) as usize
        };
        (
            to_cell(center.x - radius, self.half.x, self.width),
            to_cell(center.x + radius + EDIT_CELL, self.half.x, self.width),
            to_cell(center.y - radius, self.half.y, self.height),
            to_cell(center.y + radius + EDIT_CELL, self.half.y, self.height),
        )
    }

    /// Average finished height in the cells immediately around one cell.
    fn neighbourhood_height(&self, x: usize, z: usize, base: &dyn Fn(Vec2) -> f32) -> f32 {
        let mut total = 0.0;
        let mut count = 0.0;
        for dz in -1isize..=1 {
            for dx in -1isize..=1 {
                let nx = (x as isize + dx).clamp(0, self.width as isize - 1) as usize;
                let nz = (z as isize + dz).clamp(0, self.height as isize - 1) as usize;
                let position = self.cell_position(nx, nz);
                total += base(position) + self.offsets[nz * self.width + nx];
                count += 1.0;
            }
        }
        total / count
    }
}

use bevy::log::{info, warn};

#[cfg(test)]
mod tests {
    use super::*;

    const HALF: Vec2 = Vec2::new(400.0, 300.0);

    /// Flat ground, so the numbers under test are the brush's and nothing else's.
    fn flat(_: Vec2) -> f32 {
        0.0
    }

    fn stamp(center: Vec2, radius: f32, op: BrushOp, amount: f32, target: f32) -> Stamp<'static> {
        Stamp {
            center,
            radius,
            op,
            amount,
            target,
            base: &flat,
        }
    }

    #[test]
    fn raise_lifts_the_center_and_fades_to_nothing_at_the_rim() {
        let mut grid = EditGrid::new(HALF);
        let center = Vec2::new(40.0, -20.0);
        let radius = 60.0;

        grid.apply(&stamp(center, radius, BrushOp::Raise, 10.0, 0.0));

        let at_center = grid.sample(center.x, center.y);
        let midway = grid.sample(center.x + radius * 0.5, center.y);
        let outside = grid.sample(center.x + radius * 1.5, center.y);

        assert!(
            (at_center - 10.0).abs() < 0.5,
            "center should rise by the full amount, got {at_center:.2}"
        );
        assert!(
            midway > 0.5 && midway < at_center,
            "midway should be partly raised, got {midway:.2}"
        );
        assert!(
            outside.abs() < SCULPT_EPSILON,
            "ground outside the brush must be untouched, got {outside:.2}"
        );
    }

    #[test]
    fn lower_is_the_exact_inverse_of_raise() {
        let mut grid = EditGrid::new(HALF);
        grid.apply(&stamp(Vec2::ZERO, 50.0, BrushOp::Raise, 7.0, 0.0));
        grid.apply(&stamp(Vec2::ZERO, 50.0, BrushOp::Lower, 7.0, 0.0));

        assert!(grid.sample(0.0, 0.0).abs() < SCULPT_EPSILON);
        assert_eq!(
            grid.sculpted_cells(),
            0,
            "undoing a stroke should leave no cells counted as sculpted"
        );
    }

    #[test]
    fn flatten_pulls_the_surface_toward_its_target() {
        let mut grid = EditGrid::new(HALF);
        grid.apply(&stamp(Vec2::ZERO, 80.0, BrushOp::Raise, 60.0, 0.0));

        for _ in 0..200 {
            grid.apply(&stamp(Vec2::ZERO, 80.0, BrushOp::Flatten, 0.1, 25.0));
        }

        let height = grid.sample(0.0, 0.0);
        assert!(
            (height - 25.0).abs() < 1.0,
            "flatten should converge on its target, got {height:.2}"
        );
    }

    #[test]
    fn path_cuts_a_flat_bed_where_flatten_leaves_a_dish() {
        let radius = 60.0;
        let probe = radius * 0.5;

        // Start both from uniformly raised ground — a radius far larger than
        // the area probed, so the starting height is effectively identical at
        // the center and at the probe and can't skew the comparison.
        let mut with_path = EditGrid::new(HALF);
        let mut with_flatten = EditGrid::new(HALF);
        for grid in [&mut with_path, &mut with_flatten] {
            grid.apply(&stamp(Vec2::ZERO, 2000.0, BrushOp::Raise, 40.0, 0.0));
        }

        // Exactly one stamp at half strength. Run to convergence both profiles
        // reach the same target everywhere the falloff is non-zero — it's the
        // *shape* of a single application that distinguishes them.
        with_path.apply(&stamp(Vec2::ZERO, radius, BrushOp::Path, 0.5, 0.0));
        with_flatten.apply(&stamp(Vec2::ZERO, radius, BrushOp::Flatten, 0.5, 0.0));

        let path_center = with_path.sample(0.0, 0.0);
        let path_edge = with_path.sample(probe, 0.0);
        let flatten_edge = with_flatten.sample(probe, 0.0);

        assert!(
            (path_edge - path_center).abs() < 1.0,
            "path bed should be flat across its width: {path_center:.1} vs {path_edge:.1}"
        );
        assert!(
            flatten_edge > path_edge + 2.0,
            "flatten should still be dished at the same distance: \
             {flatten_edge:.1} vs {path_edge:.1}"
        );
    }

    #[test]
    fn roughen_adds_variation_without_moving_the_average() {
        let mut grid = EditGrid::new(HALF);
        grid.apply(&stamp(Vec2::ZERO, 100.0, BrushOp::Roughen, 6.0, 0.0));

        let samples: Vec<f32> = (-40..40)
            .map(|i| grid.sample(i as f32 * 2.0, 0.0))
            .collect();
        let mean = samples.iter().sum::<f32>() / samples.len() as f32;
        let spread = samples.iter().map(|v| (v - mean).abs()).fold(0.0, f32::max);

        assert!(spread > 0.5, "roughen should add visible variation");
        assert!(
            mean.abs() < 2.0,
            "roughen should not systematically raise or lower ground, mean {mean:.2}"
        );
    }

    #[test]
    fn smooth_takes_the_edge_off_a_spike() {
        let mut grid = EditGrid::new(HALF);
        grid.apply(&stamp(Vec2::ZERO, EDIT_CELL * 0.6, BrushOp::Raise, 100.0, 0.0));
        let before = grid.sample(0.0, 0.0);

        for _ in 0..40 {
            grid.apply(&stamp(Vec2::ZERO, 30.0, BrushOp::Smooth, 0.5, 0.0));
        }

        let after = grid.sample(0.0, 0.0);
        assert!(
            after < before * 0.6,
            "smoothing should pull a spike down toward its surroundings: {before:.1} -> {after:.1}"
        );
    }

    #[test]
    fn undo_reverts_a_whole_stroke_and_redo_puts_it_back() {
        let mut grid = EditGrid::new(HALF);

        grid.begin_stroke();
        // A drag is many ticks, and must undo as one.
        for i in 0..25 {
            grid.apply(&stamp(
                Vec2::new(i as f32 * 4.0, 0.0),
                40.0,
                BrushOp::Raise,
                2.0,
                0.0,
            ));
        }
        grid.end_stroke();

        let sculpted = grid.sample(40.0, 0.0);
        assert!(sculpted > 1.0, "the stroke should have raised ground");
        assert!(grid.can_undo());

        let area = grid.undo().expect("undo should report an area");
        assert!(
            grid.sample(40.0, 0.0).abs() < SCULPT_EPSILON,
            "undo should return the ground to exactly where it was"
        );
        assert_eq!(grid.sculpted_cells(), 0);
        assert!(area.width() > 0.0 && area.height() > 0.0);

        grid.redo().expect("redo should report an area");
        assert!(
            (grid.sample(40.0, 0.0) - sculpted).abs() < 1.0e-4,
            "redo should restore the stroke exactly"
        );
    }

    #[test]
    fn a_new_stroke_clears_the_redo_history() {
        let mut grid = EditGrid::new(HALF);

        grid.begin_stroke();
        grid.apply(&stamp(Vec2::ZERO, 40.0, BrushOp::Raise, 5.0, 0.0));
        grid.end_stroke();
        grid.undo();
        assert!(grid.can_redo());

        grid.begin_stroke();
        grid.apply(&stamp(Vec2::new(200.0, 0.0), 40.0, BrushOp::Raise, 5.0, 0.0));
        grid.end_stroke();

        assert!(
            !grid.can_redo(),
            "editing after an undo should discard the redo branch"
        );
    }

    #[test]
    fn edits_survive_a_save_and_reload() {
        let path = std::env::temp_dir().join("ranger-game-edit-roundtrip.bin");
        let _ = fs::remove_file(&path);

        let mut saved = EditGrid::new(HALF);
        saved.apply(&stamp(
            Vec2::new(-100.0, 50.0),
            70.0,
            BrushOp::Raise,
            18.0,
            0.0,
        ));
        saved.save_to(&path).expect("save should succeed");
        assert!(!saved.unsaved, "saving should clear the unsaved flag");

        let loaded = EditGrid::load_from(&path, HALF);
        assert_eq!(loaded.sculpted_cells(), saved.sculpted_cells());
        for probe in [
            Vec2::new(-100.0, 50.0),
            Vec2::new(-70.0, 50.0),
            Vec2::new(200.0, 200.0),
        ] {
            let expected = saved.sample(probe.x, probe.y);
            let actual = loaded.sample(probe.x, probe.y);
            assert!(
                (actual - expected).abs() < 1.0e-5,
                "at {probe:?}: expected {expected:.4}, loaded {actual:.4}"
            );
        }

        // A file saved for a different world size must be refused, not stretched.
        let mismatched = EditGrid::load_from(&path, HALF * 2.0);
        assert_eq!(mismatched.sculpted_cells(), 0);

        let _ = fs::remove_file(&path);
    }
}
