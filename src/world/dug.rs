//! Rock a maker has dug out, and the void it leaves.
//!
//! # Digging, not tunnel-making
//!
//! Three builds of this tried to MAKE a tunnel — two clicks and the world worked
//! out mouths, length, arch and floor for itself. Every one of them was wrong in a
//! new way, because a tunnel is a decision with a shape and the tool kept guessing
//! at the shape.
//!
//! So this one does not make tunnels. It **opens material**, a brushful at a time,
//! and what shape the tunnel ends up is the maker's business. Branch it, widen it
//! into a chamber, dog-leg it round a corner — the tool has no opinion, because it
//! is a shovel rather than an architect.
//!
//! # The ground itself is never touched
//!
//! The surface stays exactly as it was: a hill dug through keeps its trees, its
//! snow and its shading, and stays impenetrable at ground level. What digging
//! writes is a second, LOWER surface — the floor of the void — and the walking rule
//! hands a walker whichever of the two has claim on them. That is the only place
//! this world has two grounds stacked over each other.
//!
//! # The floor is set where the STROKE began, and held
//!
//! A brushful lays its whole footprint at one height, rather than following the
//! terrain's own height per cell — a floor a man dug with a pick is not lumpy. That
//! one height is the aim where the STROKE began, and it holds until the button
//! comes up.
//!
//! It used to be re-read from the aim every brushful, which sounds like the same
//! thing and is not: driving into a hillside, the aim climbs with the slope, so the
//! passage climbed with it and never got under the hill. Separate strokes still
//! choose separate levels, which is what lets one passage branch off another at a
//! different height. See `editor::dig_tunnels`.
//!
//! Where a brushful crosses ground already dug, the LOWER floor stands: you can dig
//! deeper but you cannot un-dig by painting over it. Filling in is its own stroke.
//!
//! # A shaft to nowhere
//!
//! Nothing may be dug below [`DEEPEST`]. A low aim carried into a hillside is how
//! you would dig straight down and out through the bottom of the world, and that is
//! a hole nobody can climb out of.

use bevy::prelude::*;

use terrain_core::Geometry;

use crate::config::SEA_LEVEL;

/// How wide a cell of the dug grid is, in metres.
///
/// The terrain's own vertex spacing. Finer would let a maker carve detail the ground
/// around it cannot answer for; coarser and a brushful would land as a staircase.
pub const CELL: f32 = 2.0;

/// The deepest anything may be dug, in metres.
///
/// Just above the waterline: a tunnel under the sea is a thing this world has no
/// answer for — the sea is drawn as one flat sheet and would be overhead — and a
/// shaft that keeps going is how you get a hole nobody can climb out of.
pub const DEEPEST: f32 = SEA_LEVEL + 1.0;

/// The void's shape, in metres: how far the walls stand apart at their widest, how
/// tall they are before the arch starts, and how high the crown stands over the
/// floor.
///
/// An arch with a FLAT BOTTOM, which is what a dug passage is: a floor you can
/// stand a cart on, walls that go straight up far enough to walk beside, and a
/// curved top because that is the shape that holds itself up.
///
/// # Sized for a camera and a crowd, not for one person
///
/// It was eleven metres by six and a half, which fits a walker and nothing else:
/// the follow camera sits back and above the warden, so in a passage that size it
/// spent the whole way clipped into rock, and two monsters could not pass each
/// other. Eighteen by ten gives the camera its room and leaves a road wide enough
/// for whatever ends up using it.
pub const HALF_WIDE: f32 = 9.0;
pub const LEG: f32 = 3.6;
pub const HIGH: f32 = 10.0;

/// How far above the floor the walking surface sits, so the drawn floor and the
/// ground it lies over are never the same surface twice.
const FLOOR_LIFT: f32 = 0.1;

/// The thinnest skin of rock left between a vault and the open air, in metres.
const LID: f32 = 0.3;

/// How far above a floor still counts as being IN the void.
///
/// Head height and a bit. Below this a walker belongs to the tunnel; above it they
/// are on the hill and the surface has them — which is what lets a mouth work at
/// all, since at a mouth both surfaces are at the same height.
pub const HEADROOM: f32 = HIGH * 1.4;

/// How much rock over a dug floor is too little to keep, in metres.
///
/// # The one place digging touches the surface, and why it must
///
/// "The surface is never touched" was the rule, and it made every passage
/// invisible and unenterable: the hillside's own face still stood across the
/// mouth, so a maker dug a void nobody could see into and the walk rule carried
/// them through the drawn ground like a ghost. A hole has to BE a hole somewhere.
///
/// So where less than this much ground is left over a dug floor, the surface
/// opens down to the floor — the mouth of a passage, and the honest answer for a
/// scrape dug under shallow soil, which is a cutting open to the sky. Where MORE
/// than this stands over the floor, the hill is sealed and untouched.
///
/// # The size of it decides how tall a mouth is
///
/// It is also the height of the opening: the first cell the cave is drawn in has
/// this much ground over its floor, so the vault there clears the floor by about
/// this much and that is the doorway you walk through. Below it the ground is cut
/// away; above it the hill is left alone.
///
/// This began at the full arch height with a soft band above it — eight metres of
/// cover gone — which on a mountainside is a mouth and on rolling grass is
/// everywhere: one test drag strip-mined a field into grey terraces. Then 2.8 m,
/// which made the mouths too low to read as openings at all. Four and a half is a
/// doorway a cart could take, against a vault of `HIGH` deeper in.
pub const DOORWAY: f32 = 4.5;

/// Nothing has been dug here.
///
/// A sentinel rather than an `Option` per cell: the grid is a few million cells for
/// a whole world and this keeps it four bytes each, which is also what makes the
/// file a plain block of floats.
const UNDUG: f32 = f32::MAX;

/// What a maker has dug out of the world.
#[derive(Resource)]
pub struct Dug {
    wide: usize,
    deep: usize,
    half: Vec2,
    /// The floor height of each cell, or [`UNDUG`].
    floor: Vec<f32>,
    dug: usize,
    /// Bumped by every stroke that changes anything, so the drawing knows to
    /// rebuild. The dug ground lives behind a lock rather than as ECS data, so
    /// nothing else can watch it change.
    pub generation: u64,
    /// Whether anything has been dug since this was last written.
    pub unsaved: bool,
}

impl std::fmt::Debug for Dug {
    // Its shape and how much is dug, never the grid: a world of it is millions of
    // cells and printing them helps nobody.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Dug {{ {}x{} cells of {CELL} m, {} dug }}",
            self.wide, self.deep, self.dug
        )
    }
}

impl Dug {
    /// A world nobody has dug into.
    pub fn empty(half: Vec2) -> Self {
        let wide = (half.x * 2.0 / CELL).ceil() as usize + 1;
        let deep = (half.y * 2.0 / CELL).ceil() as usize + 1;
        Self {
            wide,
            deep,
            half,
            floor: vec![UNDUG; wide * deep],
            dug: 0,
            generation: 0,
            unsaved: false,
        }
    }

    pub fn cells_dug(&self) -> usize {
        self.dug
    }

    pub fn is_empty(&self) -> bool {
        self.dug == 0
    }

    /// Which cell a world position falls in, unclamped.
    fn cell_of(&self, at: Vec2) -> (isize, isize) {
        (
            ((at.x + self.half.x) / CELL).floor() as isize,
            ((at.y + self.half.y) / CELL).floor() as isize,
        )
    }

    /// Where a cell's middle sits in the world.
    fn middle_of(&self, x: usize, z: usize) -> Vec2 {
        Vec2::new(
            (x as f32 + 0.5) * CELL - self.half.x,
            (z as f32 + 0.5) * CELL - self.half.y,
        )
    }

    fn slot(&self, x: isize, z: isize) -> Option<usize> {
        (x >= 0 && z >= 0 && (x as usize) < self.wide && (z as usize) < self.deep)
            .then(|| z as usize * self.wide + x as usize)
    }

    /// Whether a cell has been dug.
    fn open(&self, x: isize, z: isize) -> bool {
        self.slot(x, z).is_some_and(|slot| self.floor[slot] != UNDUG)
    }

    /// Opens material: everything within `radius` of `centre` becomes void, with its
    /// floor at `floor`.
    ///
    /// One height for the whole brushful — see the note on the module about why the
    /// floor is the aim rather than the ground. Where a cell is already open the
    /// LOWER floor stands.
    ///
    /// Hands back the ground it changed, for redrawing, or `None` if nothing did.
    #[cfg(feature = "tools")]
    pub fn dig(&mut self, centre: Vec2, radius: f32, floor: f32) -> Option<terrain_core::Patch> {
        let floor = floor.max(DEEPEST);
        let (low, high) = (centre - Vec2::splat(radius), centre + Vec2::splat(radius));
        let (x0, z0) = self.cell_of(low);
        let (x1, z1) = self.cell_of(high);
        let mut touched = false;

        for z in z0..=z1 {
            for x in x0..=x1 {
                let Some(slot) = self.slot(x, z) else {
                    continue;
                };
                if self.middle_of(x as usize, z as usize).distance(centre) > radius {
                    continue;
                }
                let was = self.floor[slot];
                let now = if was == UNDUG { floor } else { was.min(floor) };
                if now == was {
                    continue;
                }
                if was == UNDUG {
                    self.dug += 1;
                }
                self.floor[slot] = now;
                touched = true;
            }
        }

        if !touched {
            return None;
        }
        self.unsaved = true;
        self.generation += 1;
        // A cell past the brush either way: the mesh's arch reaches over its
        // neighbours, so the ground that has to be redrawn is wider than the stroke.
        Some((
            centre - Vec2::splat(radius + CELL * 2.0),
            centre + Vec2::splat(radius + CELL * 2.0),
        ))
    }

    /// Fills material back in — the eraser, and the only way to un-dig.
    #[cfg(feature = "tools")]
    pub fn fill(&mut self, centre: Vec2, radius: f32) -> Option<terrain_core::Patch> {
        let (low, high) = (centre - Vec2::splat(radius), centre + Vec2::splat(radius));
        let (x0, z0) = self.cell_of(low);
        let (x1, z1) = self.cell_of(high);
        let mut touched = false;

        for z in z0..=z1 {
            for x in x0..=x1 {
                let Some(slot) = self.slot(x, z) else {
                    continue;
                };
                if self.floor[slot] == UNDUG {
                    continue;
                }
                if self.middle_of(x as usize, z as usize).distance(centre) > radius {
                    continue;
                }
                self.floor[slot] = UNDUG;
                self.dug -= 1;
                touched = true;
            }
        }

        if !touched {
            return None;
        }
        self.unsaved = true;
        self.generation += 1;
        Some((
            centre - Vec2::splat(radius + CELL * 2.0),
            centre + Vec2::splat(radius + CELL * 2.0),
        ))
    }

    /// How far the surface over a dug floor is opened down, in metres.
    ///
    /// The DOORWAY: nought almost everywhere — sealed under deep rock, and already
    /// level on open ground — and the full drop to the floor where the void breaks
    /// the hill's face. Smoothed over the top of the band so the doorway's crown is
    /// a lintel rather than a one-vertex spike.
    pub fn opening(&self, at: Vec2, surface: f32) -> f32 {
        let Some(floor) = self.floor_at(at) else {
            return 0.0;
        };
        let over = surface - floor;
        if over <= 0.0 {
            return 0.0;
        }
        // # All the way down, or not at all
        //
        // This eased the carve out across `DOORWAY` with a smoothstep, and that
        // left a band where the ground was carved PART of the way to the floor and
        // the cave was not drawn either — so walking in from a mouth the ground
        // ramped back up and sealed over the passage. Measured at the time: the
        // pad ended at 20 m, the next cells stood at 21.2, and the cave began under
        // ground at 24.5. There was no hole because the hillside covered it.
        //
        // Carve and cave have to PARTITION the dug ground: every dug cell is either
        // cut down to the floor with no roof over it, or left alone with the cave
        // drawn underneath. A hard threshold does that, and the step it leaves in
        // the terrain at the boundary is the mouth's own lintel.
        if over < DOORWAY {
            over
        } else {
            0.0
        }
    }

    /// The floor of the void under a point, or `None` where nothing is dug.
    ///
    /// Read BETWEEN cells, among the open ones only. Nearest-cell would step by two
    /// metres at every boundary, which is a lumpy floor — and blending with the
    /// closed cells around a void would drag its floor toward nothing at the walls.
    /// How one cell stands to the surface: `None` for undug rock, `Some(false)`
    /// for a cell the carve opens to the sky, `Some(true)` for a cell still sealed
    /// under the ground, with the cave running beneath it.
    ///
    /// The SAME question [`void`] answers per cell when it decides what it draws,
    /// offered whole so the terrain can ask it too: a mouth's face is the seam
    /// between the two answers, and it has to fall in the same place for both
    /// meshes. The terrain's first attempt asked its own version through the
    /// bilinear [`Self::floor_at`], which reaches half a cell past the dug ground —
    /// so the trench's own side walls voted "passage" and the ground unzipped along
    /// the whole route into a crater. (`ground` may be the sealed or the carved
    /// surface; they agree here, because the carve either takes a point all the way
    /// down to the floor or leaves it alone.)
    pub fn cell_kind(&self, world: Vec2, ground: impl Fn(Vec2) -> f32) -> Option<bool> {
        let (cx, cz) = self.cell_of(world);
        if !self.open(cx, cz) {
            return None;
        }
        let middle = self.middle_of(cx as usize, cz as usize);
        let floor = self.floor_at(middle)?;
        Some(ground(middle) - floor >= DOORWAY)
    }

    pub fn floor_at(&self, at: Vec2) -> Option<f32> {
        let fx = (at.x + self.half.x) / CELL - 0.5;
        let fz = (at.y + self.half.y) / CELL - 0.5;
        let (x0, z0) = (fx.floor() as isize, fz.floor() as isize);
        let (tx, tz) = (fx - x0 as f32, fz - z0 as f32);

        let mut sum = 0.0;
        let mut weight = 0.0;
        for (dx, dz, w) in [
            (0, 0, (1.0 - tx) * (1.0 - tz)),
            (1, 0, tx * (1.0 - tz)),
            (0, 1, (1.0 - tx) * tz),
            (1, 1, tx * tz),
        ] {
            let Some(slot) = self.slot(x0 + dx, z0 + dz) else {
                continue;
            };
            let floor = self.floor[slot];
            if floor == UNDUG || w <= 0.0 {
                continue;
            }
            sum += floor * w;
            weight += w;
        }
        (weight > 0.0).then(|| sum / weight)
    }

    /// The height a walker's feet belong at, or `None` if the void has no claim.
    ///
    /// # Two grounds, and which one has you
    ///
    /// Over a dug passage there are two surfaces: the hillside above and the floor
    /// below. Standing on the hill, the hill has you; standing in the tunnel, the
    /// floor does. What decides it is where the walker ALREADY is — `standing` — so
    /// the answer is continuous: you keep the surface you are on until you walk off
    /// the end of it.
    ///
    /// At a mouth both surfaces meet, so either answer is the same answer and
    /// stepping between them is seamless.
    pub fn walk_floor(&self, at: Vec2, standing: f32) -> Option<f32> {
        let floor = self.floor_at(at)? + FLOOR_LIFT;
        (standing < floor + HEADROOM).then_some(floor)
    }

    /// The far corners of everything dug, or `None` if nothing is.
    fn bounds(&self) -> Option<(Vec2, Vec2)> {
        let mut low = Vec2::splat(f32::MAX);
        let mut high = Vec2::splat(f32::MIN);
        for (slot, floor) in self.floor.iter().enumerate() {
            if *floor == UNDUG {
                continue;
            }
            let middle = self.middle_of(slot % self.wide, slot / self.wide);
            low = low.min(middle);
            high = high.max(middle);
        }
        (low.x <= high.x).then_some((low, high))
    }
}

// ------------------------------------------------------------------ the drawing

/// How far in from the nearest wall each open cell is, in metres.
///
/// A two-pass chamfer over the open cells, which is all the arch needs: the crown
/// rises with distance from the wall, so a narrow passage gets a low vault and a
/// wide chamber a tall one, and a junction of two passages arches over itself
/// without anybody working out that it is a junction.
fn inwardness(open: &[bool], wide: usize, deep: usize) -> Vec<f32> {
    let mut away = vec![f32::MAX; open.len()];
    let step = CELL;
    let diagonal = CELL * std::f32::consts::SQRT_2;

    for slot in 0..open.len() {
        if !open[slot] {
            away[slot] = 0.0;
        }
    }
    let mut pass = |order: Vec<usize>, back: bool| {
        for slot in order {
            if !open[slot] {
                continue;
            }
            let (x, z) = (slot % wide, slot / wide);
            let mut best = away[slot];
            let neighbours: [(isize, isize, f32); 4] = if back {
                [(1, 0, step), (0, 1, step), (1, 1, diagonal), (-1, 1, diagonal)]
            } else {
                [(-1, 0, step), (0, -1, step), (-1, -1, diagonal), (1, -1, diagonal)]
            };
            for (dx, dz, cost) in neighbours {
                let (nx, nz) = (x as isize + dx, z as isize + dz);
                if nx < 0 || nz < 0 || nx as usize >= wide || nz as usize >= deep {
                    // Off the grid is solid, so the wall is right here.
                    best = best.min(cost);
                    continue;
                }
                best = best.min(away[nz as usize * wide + nx as usize] + cost);
            }
            away[slot] = best;
        }
    };
    pass((0..open.len()).collect(), false);
    pass((0..open.len()).rev().collect(), true);
    away
}

/// How high the vault stands over the floor, this far in from a wall.
///
/// A flat bottom, legs, then a curve: [`LEG`] at the wall rising to [`HIGH`] in the
/// middle of a passage as wide as one gets.
fn vault(inward: f32) -> f32 {
    let t = (inward / HALF_WIDE).clamp(0.0, 1.0);
    LEG + (HIGH - LEG) * t.sqrt()
}

/// The void, as a mesh seen from inside.
///
/// Floor, vault and walls over everything dug — one mesh for the lot, rebuilt
/// whenever the digging changes. Digging is something a maker does a few times a
/// second at most, so there is nothing to gain by being cleverer than that.
///
/// `ground` gives the surface height, which the vault is never allowed to break
/// through: dig a shallow scrape and the roof over it stays under the hillside
/// rather than standing out of it as a sliver of ceiling in the open air.
pub fn void(dug: &Dug, ground: impl Fn(Vec2) -> f32) -> Geometry {
    void_parts(dug, ground).0
}

/// The void, and how much of it is HEWN.
///
/// The vertices come in two runs: the cave's own floor, vault and walls, cut out of
/// the rock, and then the dressed stone of the doorways, which is BUILT and stands in
/// ground the carve opened on purpose. Almost every rule this file keeps — no floor
/// over carved ground, no wall without rock behind it, no two heights in one column —
/// is a rule about hewn rock, and a doorway breaks all three deliberately.
///
/// So the boundary between the two runs is returned rather than guessed at. A colour
/// was tried as the marker and went wrong at once: the lintel is a darker shade of the
/// same stone, so a test looking for one exact colour let sixty-four lintel faces
/// through and called them walls.
pub fn void_parts(dug: &Dug, ground: impl Fn(Vec2) -> f32) -> (Geometry, usize) {
    let Some((low, high)) = dug.bounds() else {
        return (Geometry::default(), 0);
    };
    // A ring of solid cells round the lot, so the walls at the outermost dug cells
    // have somewhere to stand.
    let (x0, z0) = dug.cell_of(low - Vec2::splat(CELL));
    let (x1, z1) = dug.cell_of(high + Vec2::splat(CELL));
    let wide = (x1 - x0 + 1).max(1) as usize;
    let deep = (z1 - z0 + 1).max(1) as usize;

    let open: Vec<bool> = (0..wide * deep)
        .map(|slot| dug.open(x0 + (slot % wide) as isize, z0 + (slot / wide) as isize))
        .collect();
    let inward = inwardness(&open, wide, deep);

    // Which cells get drawn at all: dug, and left sealed by the carve.
    //
    // Asked of a cell AND its four neighbours, because a cell reaches half a cell
    // into each of them — a sealed cell beside a carved-open one would put its own
    // edge down in ground the terrain has already opened, which is a rim of
    // flickering quads round every mouth.
    let drawn: Vec<bool> = (0..wide * deep)
        .map(|slot| {
            if !open[slot] {
                return false;
            }
            let middle = dug.middle_of(
                (x0 + (slot % wide) as isize) as usize,
                (z0 + (slot / wide) as isize) as usize,
            );
            // Cells with real ground over them, and nothing else. The carve takes
            // everything under `DOORWAY`, so the two TILE: a dug cell is cut down
            // to the floor with no roof, or left alone with the cave under it.
            //
            // Eroding the cave back from the carve used to be necessary because the
            // carve faded out, and it is what buried every mouth under the fading
            // band. Both now come from one number — how much ground stands over the
            // floor — read by two consumers.
            //
            // Asking `opening() <= 0` instead was nearly right and wrong at the far
            // end: out on the flat past a hill there is NO ground over the floor, so
            // there is nothing to carve, so opening is nought — and a cave got drawn
            // with its vault clamped BELOW its own floor. Nothing to carve and
            // nothing to roof are different answers.
            ground(middle) - dug.floor_at(middle).unwrap_or(f32::MAX) >= DOORWAY
        })
        .collect();

    // # One surface, not a tile each
    //
    // The floor and the vault used to be a quad per cell, each at its own cell's
    // height — and neighbouring cells have slightly different floors, so no two
    // tiles met. Every cell boundary was a hairline gap with the sky behind it,
    // which came back photographed from inside as a floor ruled into a pale grid
    // and a ceiling full of ragged holes.
    //
    // So both are built on the CORNER lattice instead, and every height is sampled
    // at the corner. Neighbouring quads then share their vertices exactly, by
    // construction rather than by luck, and there is nothing between them to see
    // through. `floor_at` is bilinear, so a corner has one answer however many
    // cells meet at it.
    let corners = (wide + 1) * (deep + 1);
    let corner_at = |cx: usize, cz: usize| {
        Vec2::new(
            (x0 + cx as isize) as f32 * CELL - dug.half.x,
            (z0 + cz as isize) as f32 * CELL - dug.half.y,
        )
    };
    // A corner belongs to the mesh if any cell touching it is drawn.
    let used: Vec<bool> = (0..corners)
        .map(|slot| {
            let (cx, cz) = (slot % (wide + 1), slot / (wide + 1));
            [(0_isize, 0_isize), (-1, 0), (0, -1), (-1, -1)]
                .into_iter()
                .any(|(dx, dz)| {
                    let (nx, nz) = (cx as isize + dx, cz as isize + dz);
                    nx >= 0
                        && nz >= 0
                        && (nx as usize) < wide
                        && (nz as usize) < deep
                        && drawn[nz as usize * wide + nx as usize]
                })
        })
        .collect();

    // The floor and the roof at each corner. Averaged over the drawn cells that
    // meet there, so the roof follows the vault's own shape without any one cell
    // deciding it alone.
    let mut sole = vec![0.0_f32; corners];
    let mut roof = vec![0.0_f32; corners];
    for slot in 0..corners {
        if !used[slot] {
            continue;
        }
        let (cx, cz) = (slot % (wide + 1), slot / (wide + 1));
        let at = corner_at(cx, cz);
        let floor = dug.floor_at(at).unwrap_or_else(|| {
            // A corner on the very rim: take the nearest drawn cell's own floor
            // rather than nothing, so the edge still closes.
            let mut best = 0.0;
            for (dx, dz) in [(0_isize, 0_isize), (-1, 0), (0, -1), (-1, -1)] {
                let (nx, nz) = (cx as isize + dx, cz as isize + dz);
                if nx < 0 || nz < 0 || nx as usize >= wide || nz as usize >= deep {
                    continue;
                }
                let middle = dug.middle_of((x0 + nx) as usize, (z0 + nz) as usize);
                if let Some(floor) = dug.floor_at(middle) {
                    best = floor;
                }
            }
            best
        });

        let mut lift = 0.0;
        let mut count = 0.0;
        for (dx, dz) in [(0_isize, 0_isize), (-1, 0), (0, -1), (-1, -1)] {
            let (nx, nz) = (cx as isize + dx, cz as isize + dz);
            if nx < 0 || nz < 0 || nx as usize >= wide || nz as usize >= deep {
                continue;
            }
            let cell = nz as usize * wide + nx as usize;
            if !drawn[cell] {
                continue;
            }
            lift += vault(inward[cell]);
            count += 1.0;
        }
        let lift = if count > 0.0 { lift / count } else { LEG };

        sole[slot] = floor + FLOOR_LIFT;
        // Never up through the ground: the surface here is sealed, and a vault that
        // broke it would be a hole in a hillside nobody dug.
        roof[slot] = (floor + lift).min(ground(at) - LID).max(sole[slot] + 0.2);
    }

    // How dark it is at each corner: rock overhead is what makes a cave dark, and
    // no lamps exist yet, so the vertex colours carry the light.
    let dark: Vec<f32> = (0..corners)
        .map(|slot| {
            if !used[slot] {
                return 1.0;
            }
            let (cx, cz) = (slot % (wide + 1), slot / (wide + 1));
            let at = corner_at(cx, cz);
            let over = (ground(at) - sole[slot]).max(0.0);
            1.0 - crate::util::smoothstep(0.0, 26.0, over) * 0.82
        })
        .collect();


    let mut mesh = Geometry::default();
    let mut slots: Vec<Option<u32>> = vec![None; corners * 2];
    let stone = |shade: f32| [0.108 * shade, 0.101 * shade, 0.094 * shade, 1.0];

    // One vertex per corner per surface, made on demand and reused by every quad
    // that touches it.
    let mut put = |mesh: &mut Geometry, slot: usize, ceiling: bool, normal: Vec3| -> u32 {
        let key = slot * 2 + ceiling as usize;
        if let Some(index) = slots[key] {
            return index;
        }
        let (cx, cz) = (slot % (wide + 1), slot / (wide + 1));
        let at = corner_at(cx, cz);
        let y = if ceiling { roof[slot] } else { sole[slot] };
        let shade = dark[slot] * if ceiling { 0.8 } else { 1.15 };
        let index = mesh.places.len() as u32;
        mesh.places.push([at.x, y, at.y]);
        mesh.normals.push(normal.to_array());
        mesh.uvs.push([cx as f32, cz as f32]);
        mesh.colours.push(stone(shade));
        slots[key] = Some(index);
        index
    };

    for (slot, sealed) in drawn.iter().enumerate() {
        if !sealed {
            continue;
        }
        let (cx, cz) = (slot % wide, slot / wide);
        let corner = |dx: usize, dz: usize| (cz + dz) * (wide + 1) + (cx + dx);
        let (a, b, c, d) = (
            corner(0, 0),
            corner(1, 0),
            corner(1, 1),
            corner(0, 1),
        );

        // The floor, seen from above.
        let floor: Vec<u32> = [a, b, c, d]
            .into_iter()
            .map(|slot| put(&mut mesh, slot, false, Vec3::Y))
            .collect();
        mesh.indices.extend_from_slice(&[
            floor[0], floor[3], floor[1], floor[1], floor[3], floor[2],
        ]);

        // The vault, seen from below.
        let vault: Vec<u32> = [a, b, c, d]
            .into_iter()
            .map(|slot| put(&mut mesh, slot, true, -Vec3::Y))
            .collect();
        mesh.indices.extend_from_slice(&[
            vault[0], vault[1], vault[3], vault[1], vault[2], vault[3],
        ]);

        // And a wall wherever there is ROCK next door. Built on the same two
        // corners the floor and the vault used, so the three meet exactly and
        // there is no seam to see through.
        //
        // # Rock, not merely "not drawn"
        //
        // This asked whether the neighbour was DRAWN, and a cell the carve has
        // opened is dug but not drawn — so every mouth got a wall built straight
        // across it. Reported as the entrances being drawn over, and that is
        // exactly what it was: the cave sealing its own doorways from the inside.
        //
        // A wall's job is to hold back rock. Where the neighbour is dug, there is
        // no rock to hold back, whether or not this cave bothers to draw it.
        for (dx, dz, near, far) in [
            (1_isize, 0_isize, b, c),
            (-1, 0, d, a),
            (0, 1, c, d),
            (0, -1, a, b),
        ] {
            let (nx, nz) = (cx as isize + dx, cz as isize + dz);
            let rock = nx < 0
                || nz < 0
                || nx as usize >= wide
                || nz as usize >= deep
                || !open[nz as usize * wide + nx as usize];
            if !rock {
                continue;
            }
            let inward_normal = Vec3::new(-dx as f32, 0.0, -dz as f32);
            let base = mesh.places.len() as u32;
            for (slot, ceiling) in [(near, false), (far, false), (far, true), (near, true)] {
                let (ccx, ccz) = (slot % (wide + 1), slot / (wide + 1));
                let at = corner_at(ccx, ccz);
                let y = if ceiling { roof[slot] } else { sole[slot] };
                mesh.places.push([at.x, y, at.y]);
                mesh.normals.push(inward_normal.to_array());
                mesh.uvs.push([0.0, 0.0]);
                mesh.colours.push(stone(dark[slot]));
            }
            mesh.indices
                .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
    }
    // Everything up to here is hewn out of the rock. The doorways are built.
    let hewn = mesh.places.len();
    frame_the_doorways(&mut mesh, dug, &open, &drawn, wide, deep, x0, z0, &ground);
    (mesh, hewn)
}


/// The stones of a doorframe, in metres: a pillar's square cross-section, the
/// thickness of the beam across them, how deep a stone is set into the ground, and
/// the air left between the top of the opening and the beam over it.
const PILLAR: f32 = 1.8;
const LINTEL: f32 = 1.6;
const FOOTING: f32 = 0.25;
const CLEARANCE: f32 = 0.6;

/// Stands one doorframe at every mouth: a pillar either side, a beam across the top.
///
/// # One frame per mouth, not a stone per cell
///
/// A mouth that was only an absence — carved ground and a gap in a hillside — could
/// not be found, four reports running. The first attempt at building one put a
/// lintel on every threshold CELL at that cell's own crown height, ten metres up —
/// and neighbouring floors differ by centimetres, so what stood was a floating
/// crenellated wall. "Not sure if this is worse."
///
/// A doorway is one THING. So the threshold cells are gathered into mouths first,
/// each mouth measures its own facing and width, and one frame is raised for the
/// lot — at the height of the OPENING, which is [`DOORWAY`] by definition, not the
/// height of the vault behind it.
#[allow(clippy::too_many_arguments)]
fn frame_the_doorways(
    mesh: &mut Geometry,
    dug: &Dug,
    open: &[bool],
    drawn: &[bool],
    wide: usize,
    deep: usize,
    x0: isize,
    z0: isize,
    ground: &impl Fn(Vec2) -> f32,
) {
    // The threshold: sealed cells with carved-open ground next door, each knowing
    // which way out is.
    let mut lip: Vec<(isize, isize, Vec2)> = Vec::new();
    for (slot, sealed) in drawn.iter().enumerate() {
        if !sealed {
            continue;
        }
        let (cx, cz) = ((slot % wide) as isize, (slot / wide) as isize);
        let mut out = Vec2::ZERO;
        for (dx, dz) in [(1_isize, 0_isize), (-1, 0), (0, 1), (0, -1)] {
            let (nx, nz) = (cx + dx, cz + dz);
            if nx < 0 || nz < 0 || nx as usize >= wide || nz as usize >= deep {
                continue;
            }
            let cell = nz as usize * wide + nx as usize;
            if open[cell] && !drawn[cell] {
                out += Vec2::new(dx as f32, dz as f32);
            }
        }
        if out != Vec2::ZERO {
            lip.push((cx, cz, out));
        }
    }

    // Gathered into mouths: threshold cells near each other are one opening,
    // however ragged a line they make. Each cluster is one doorway.
    let mut claimed = vec![false; lip.len()];
    for first in 0..lip.len() {
        if claimed[first] {
            continue;
        }
        claimed[first] = true;
        let mut mouth = vec![first];
        let mut grew = true;
        while grew {
            grew = false;
            for other in 0..lip.len() {
                if claimed[other] {
                    continue;
                }
                let near = mouth.iter().any(|&m| {
                    (lip[m].0 - lip[other].0).abs() <= 2 && (lip[m].1 - lip[other].1).abs() <= 2
                });
                if near {
                    claimed[other] = true;
                    mouth.push(other);
                    grew = true;
                }
            }
        }
        let mouth: Vec<(isize, isize, Vec2)> = mouth.into_iter().map(|m| lip[m]).collect();
        raise_a_doorframe(mesh, dug, &mouth, x0, z0, ground);
    }
}

/// One doorframe: two square pillars and the beam across them, in dressed stone.
fn raise_a_doorframe(
    mesh: &mut Geometry,
    dug: &Dug,
    mouth: &[(isize, isize, Vec2)],
    x0: isize,
    z0: isize,
    ground: &impl Fn(Vec2) -> f32,
) {
    // The ground as a walker sees it — the sealed surface with the carve taken out.
    let carved = |at: Vec2| ground(at) - dug.opening(at, ground(at));
    // Which way the doorway faces is the mouth's own consensus.
    let facing = mouth.iter().fold(Vec2::ZERO, |sum, (_, _, out)| sum + *out);
    let Some(out) = facing.try_normalize() else {
        return;
    };
    // The side axis is picked so (side, out) wind the way (x, z) do, which is the
    // handedness block() is written for.
    let side = Vec2::new(out.y, -out.x);

    // The opening, measured in the doorway's own frame: how far it runs across,
    // how far forward its outermost cell reaches, and the lowest floor in it.
    let middles: Vec<Vec2> = mouth
        .iter()
        .map(|(cx, cz, _)| dug.middle_of((x0 + cx) as usize, (z0 + cz) as usize))
        .collect();
    let heart = middles.iter().copied().sum::<Vec2>() / middles.len() as f32;
    let mut across = (f32::MAX, f32::MIN);
    let mut forward = f32::MIN;
    for at in &middles {
        let local = *at - heart;
        across = (across.0.min(local.dot(side)), across.1.max(local.dot(side)));
        forward = forward.max(local.dot(out));
    }
    let Some(floor) = middles
        .iter()
        .filter_map(|at| dug.floor_at(*at))
        .min_by(|a, b| a.total_cmp(b))
    else {
        return;
    };

    // The opening runs half a cell past the outermost middles, and the frame
    // straddles the seam itself: the beam covers the cut edge of the ground where
    // the face was skipped, and the stones' back halves sit into the hillside over
    // the vault — which is what a doorframe set into rock looks like.
    let half_across = (across.1 - across.0) * 0.5 + CELL * 0.5;
    let middle = heart
        + side * ((across.1 + across.0) * 0.5)
        + out * (forward + CELL * 0.5);
    // The head of the frame sits just over the opening — and the opening is
    // DOORWAY tall by definition, since that is where the carve stops.
    let head = floor + DOORWAY + CLEARANCE;

    let dressed = |shade: f32| [0.196 * shade, 0.186 * shade, 0.17 * shade, 1.0];
    // A pillar flush with each side of the opening, seated on the ground it actually
    // stands on. The first build footed both at the mouth's lowest floor, and on the
    // trench's sloping sides that buried one of them to the knees — "slightly too
    // low into the ground". Seating is per pillar, on the lowest of its own corners,
    // sunk just enough that no corner floats.
    for hand in [-1.0_f32, 1.0] {
        let stand = middle + side * (hand * (half_across - PILLAR * 0.5));
        let seat = [Vec2::ZERO, Vec2::ONE, -Vec2::ONE, Vec2::new(1.0, -1.0), Vec2::new(-1.0, 1.0)]
            .into_iter()
            .map(|corner| {
                carved(stand + side * (corner.x * PILLAR * 0.5) + out * (corner.y * PILLAR * 0.5))
            })
            .fold(f32::MAX, f32::min)
            .min(floor);
        block(
            mesh,
            stand,
            side,
            out,
            PILLAR * 0.5,
            PILLAR * 0.5,
            seat - FOOTING,
            head,
            dressed(1.0),
        );
    }
    // ...and one LEVEL beam across the pair of them.
    block(
        mesh,
        middle,
        side,
        out,
        half_across + 0.3,
        LINTEL * 0.5,
        head,
        head + LINTEL,
        dressed(0.86),
    );
}

/// One rectangular block of dressed stone, drawn on all six faces.
///
/// A doorway is a THING standing in the world — seen from outside as well as in, so
/// every face of it has to be there. The block is oriented: `side` and `out` are the
/// doorway's own axes, so a frame on a diagonal passage stands square to its opening
/// rather than to the world.
#[allow(clippy::too_many_arguments)]
fn block(
    mesh: &mut Geometry,
    at: Vec2,
    side: Vec2,
    out: Vec2,
    half_side: f32,
    half_out: f32,
    low: f32,
    high: f32,
    colour: [f32; 4],
) {
    if high <= low {
        return;
    }
    let corner = |su: f32, sv: f32, y: f32| {
        let flat = at + side * (half_side * su) + out * (half_out * sv);
        Vec3::new(flat.x, y, flat.y)
    };
    let faces = [
        (
            [
                corner(-1.0, -1.0, high),
                corner(1.0, -1.0, high),
                corner(1.0, 1.0, high),
                corner(-1.0, 1.0, high),
            ],
            Vec3::Y,
        ),
        (
            [
                corner(-1.0, 1.0, low),
                corner(1.0, 1.0, low),
                corner(1.0, -1.0, low),
                corner(-1.0, -1.0, low),
            ],
            -Vec3::Y,
        ),
        (
            [
                corner(1.0, -1.0, low),
                corner(1.0, 1.0, low),
                corner(1.0, 1.0, high),
                corner(1.0, -1.0, high),
            ],
            Vec3::new(side.x, 0.0, side.y),
        ),
        (
            [
                corner(-1.0, 1.0, low),
                corner(-1.0, -1.0, low),
                corner(-1.0, -1.0, high),
                corner(-1.0, 1.0, high),
            ],
            Vec3::new(-side.x, 0.0, -side.y),
        ),
        (
            [
                corner(-1.0, 1.0, low),
                corner(1.0, 1.0, low),
                corner(1.0, 1.0, high),
                corner(-1.0, 1.0, high),
            ],
            Vec3::new(out.x, 0.0, out.y),
        ),
        (
            [
                corner(1.0, -1.0, low),
                corner(-1.0, -1.0, low),
                corner(-1.0, -1.0, high),
                corner(1.0, -1.0, high),
            ],
            Vec3::new(-out.x, 0.0, -out.y),
        ),
    ];
    for (quad, normal) in faces {
        let base = mesh.places.len() as u32;
        for point in quad {
            mesh.places.push(point.to_array());
            mesh.normals.push(normal.to_array());
            mesh.uvs.push([0.0, 0.0]);
            mesh.colours.push(colour);
        }
        mesh.indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
}

// ------------------------------------------------------------------- on the disk

/// Names the file, so an unrelated one is refused rather than read as a world full
/// of holes.
const MAGIC: &[u8; 8] = b"RNGRDUG1";

pub fn path() -> std::path::PathBuf {
    std::path::Path::new("assets/world/dug.bin").to_path_buf()
}

pub fn read(bytes: &[u8], half: Vec2) -> Result<Dug, String> {
    let empty = Dug::empty(half);
    let header = 8 + 4 * 4;
    if bytes.len() < header || &bytes[..8] != MAGIC {
        return Err("not dug ground".into());
    }
    let word = |at: usize| {
        u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]) as usize
    };
    let real =
        |at: usize| f32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]);
    let (wide, deep) = (word(8), word(12));
    let kept = Vec2::new(real(16), real(20));

    // Refused rather than stretched — the same rule every other layer keeps. Holes
    // landing in the wrong places are worse than none, and nothing on screen would
    // say why.
    if wide != empty.wide || deep != empty.deep || kept.distance(half) > 1.0 {
        return Err(format!(
            "dug for a {:.0}x{:.0} m world, not this {:.0}x{:.0} m one",
            kept.x * 2.0,
            kept.y * 2.0,
            half.x * 2.0,
            half.y * 2.0
        ));
    }
    if bytes.len() < header + wide * deep * 4 {
        return Err("truncated".into());
    }

    let floor: Vec<f32> = (0..wide * deep).map(|i| real(header + i * 4)).collect();
    // Numbers, not just number-shaped: a floor of NaN reads as dug and puts a
    // walker's feet nowhere. UNDUG is the one infinity that belongs here.
    if floor.iter().any(|v| v.is_nan() || (v.is_infinite() && *v != UNDUG)) {
        return Err("dug ground holding numbers that are not numbers".into());
    }
    let dug = floor.iter().filter(|v| **v != UNDUG).count();
    Ok(Dug {
        floor,
        dug,
        ..empty
    })
}

pub fn to_bytes(dug: &Dug) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 + 16 + dug.floor.len() * 4);
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&(dug.wide as u32).to_le_bytes());
    out.extend_from_slice(&(dug.deep as u32).to_le_bytes());
    out.extend_from_slice(&dug.half.x.to_le_bytes());
    out.extend_from_slice(&dug.half.y.to_le_bytes());
    for floor in &dug.floor {
        out.extend_from_slice(&floor.to_le_bytes());
    }
    out
}

/// Reads what has been dug, or a world nobody has dug into.
pub fn load(half: Vec2) -> Dug {
    let Ok(bytes) = std::fs::read(path()) else {
        return Dug::empty(half);
    };
    match read(&bytes, half) {
        Ok(dug) => dug,
        Err(why) => {
            warn!("{}: {why}", path().display());
            Dug::empty(half)
        }
    }
}

#[cfg(feature = "tools")]
pub fn save(dug: &mut Dug) -> std::io::Result<()> {
    let road = path();
    if let Some(folder) = road.parent() {
        std::fs::create_dir_all(folder)?;
    }
    std::fs::write(&road, to_bytes(dug))?;
    dug.unsaved = false;
    Ok(())
}

/// The void, as an entity in the world.
#[derive(Component)]
pub struct Void;

/// Draws whatever has been dug, and takes down what was drawn before.
///
/// One mesh for the lot, rebuilt whenever the digging changes. Digging is something
/// a maker does a few times a second at most, so there is nothing to be gained by
/// being cleverer than that.
pub fn draw_the_void(
    mut commands: Commands,
    terrain: Res<crate::world::terrain::TerrainSource>,
    mut meshes: ResMut<Assets<Mesh>>,
    material: Option<Res<crate::world::chunk::TerrainMaterial>>,
    standing: Query<Entity, With<Void>>,
    mut seen: Local<Option<u64>>,
) {
    let Some(material) = material else {
        return;
    };
    let Ok(dug) = terrain.0.dug().read() else {
        return;
    };
    // Runs every frame and draws almost never: only when a stroke has changed the
    // digging — the ground lives behind a lock, so a generation stamp is the only
    // change detection there is — or when the world was taken down and put back up
    // with the digging still in it.
    //
    // This was an OnEnter one-shot, which is the first half of why the shovel
    // "didn't work": every stroke updated the data and nothing ever redrew it.
    let missing = standing.is_empty() && !dug.is_empty();
    if *seen == Some(dug.generation) && !missing {
        return;
    }
    *seen = Some(dug.generation);
    for old in &standing {
        commands.entity(old).despawn();
    }
    if dug.is_empty() {
        return;
    }
    let mesh = void(&dug, |at| terrain.0.height(at.x, at.y));
    if mesh.is_empty() {
        return;
    }
    info!(
        "the cave: {} cells dug, {} vertices drawn",
        dug.cells_dug(),
        mesh.places.len()
    );
    commands.spawn((
        Void,
        Mesh3d(meshes.add(crate::world::stream::as_coloured_mesh(&mesh))),
        MeshMaterial3d(material.0.clone()),
        Transform::IDENTITY,
        Visibility::default(),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    const HALF: Vec2 = Vec2::new(400.0, 300.0);

    fn hill(at: Vec2) -> f32 {
        30.0 + 60.0 * (1.0 - (at.x.abs() / 120.0).min(1.0))
    }

    #[test]
    fn a_brushful_lays_one_flat_floor_rather_than_following_the_ground() {
        // The whole of choice (a): the floor is the height under the POINTER,
        // stamped flat across the footprint. Taking the ground per cell instead is
        // what would make a dug floor lumpy, and a floor a man dug with a pick is
        // not lumpy.
        let mut dug = Dug::empty(HALF);
        let aim = 42.0;
        dug.dig(Vec2::ZERO, 14.0, aim).expect("something was dug");

        let mut lowest = f32::MAX;
        let mut highest = f32::MIN;
        for step in -6..=6 {
            for side in -6..=6 {
                let at = Vec2::new(step as f32 * 2.0, side as f32 * 2.0);
                if at.length() > 10.0 {
                    continue;
                }
                let floor = dug.floor_at(at).expect("inside the brush");
                lowest = lowest.min(floor);
                highest = highest.max(floor);
            }
        }
        assert!(
            (highest - lowest).abs() < 0.01,
            "the floor ranges {lowest:.2}..{highest:.2} across one brushful"
        );
        assert!((lowest - aim).abs() < 0.01, "the floor came out at {lowest:.2}, not {aim}");
    }

    #[test]
    fn digging_deeper_wins_and_painting_higher_does_not_undo_it() {
        // You can dig deeper, but you cannot un-dig by painting over. Filling in is
        // its own stroke, which is also the only undo this tool has.
        let mut dug = Dug::empty(HALF);
        dug.dig(Vec2::ZERO, 10.0, 40.0);
        dug.dig(Vec2::ZERO, 10.0, 34.0);
        assert!((dug.floor_at(Vec2::ZERO).unwrap() - 34.0).abs() < 0.01, "digging deeper lost");

        dug.dig(Vec2::ZERO, 10.0, 50.0);
        assert!(
            (dug.floor_at(Vec2::ZERO).unwrap() - 34.0).abs() < 0.01,
            "painting higher raised the floor"
        );

        dug.fill(Vec2::ZERO, 12.0).expect("the fill did nothing");
        assert_eq!(dug.floor_at(Vec2::ZERO), None, "filling in left a void");
        assert!(dug.is_empty(), "{dug:?} still holds cells");
    }

    #[test]
    fn nothing_can_be_dug_out_through_the_bottom_of_the_world() {
        // A pointer can only aim at the surface, so a floor is always some real
        // height — but a low aim carried into a hillside is how you dig straight
        // down and out, and that is a hole nobody climbs out of.
        let mut dug = Dug::empty(HALF);
        dug.dig(Vec2::ZERO, 8.0, -400.0);
        let floor = dug.floor_at(Vec2::ZERO).expect("dug");
        assert!(
            floor >= DEEPEST - 0.001,
            "dug to {floor:.1} m, below the world's own floor of {DEEPEST}"
        );
    }

    #[test]
    fn the_floor_takes_a_walker_only_when_they_are_down_in_it() {
        // The one place this world has two grounds. Standing on the hill, the hill
        // has you; down in the passage, the floor does.
        let mut dug = Dug::empty(HALF);
        dug.dig(Vec2::ZERO, 10.0, 30.0);

        assert!(
            dug.walk_floor(Vec2::ZERO, 31.0).is_some(),
            "the floor let go of somebody standing on it"
        );
        assert_eq!(
            dug.walk_floor(Vec2::ZERO, 30.0 + HEADROOM + 5.0),
            None,
            "the floor claimed somebody up on the hillside"
        );
        // And nowhere near anything dug, it never claims anybody.
        assert_eq!(dug.walk_floor(Vec2::new(200.0, 0.0), 30.0), None);
    }

    #[test]
    fn the_void_is_an_arch_with_a_flat_bottom_and_it_stays_under_the_hill() {
        let mut dug = Dug::empty(HALF);
        // A passage straight through the hill, dug as a maker would: a run of
        // brushfuls at one aim.
        for step in -50..=50 {
            dug.dig(Vec2::new(step as f32 * 2.0, 0.0), HALF_WIDE, 32.0);
        }

        let mesh = void(&dug, hill);
        assert!(!mesh.is_empty(), "nothing was drawn for a dug passage");

        // Flat bottom: every floor vertex at one height.
        let sole = 32.0 + FLOOR_LIFT;
        let floors: Vec<f32> = mesh
            .places
            .iter()
            .map(|p| p[1])
            .filter(|y| (y - sole).abs() < 0.001)
            .collect();
        assert!(floors.len() > 100, "only {} floor vertices", floors.len());

        // Arched: the crown over the middle of the passage stands higher than the
        // springing at its wall.
        assert!(
            vault(HALF_WIDE) > vault(0.0) + 1.0,
            "the vault is {:.1} m at the wall and {:.1} in the middle",
            vault(0.0),
            vault(HALF_WIDE)
        );
        assert!((vault(0.0) - LEG).abs() < 0.01, "the wall is not LEG tall");

        // And never out through the hillside: the roof stays under the ground
        // everywhere, or a shallow scrape shows a sliver of ceiling in the open.
        for place in &mesh.places {
            let at = Vec2::new(place[0], place[2]);
            assert!(
                place[1] <= hill(at) - LID + 0.001,
                "the void reaches {:.1} m where the hill is only {:.1}",
                place[1],
                hill(at)
            );
        }
    }

    #[test]
    fn a_junction_arches_over_itself() {
        // Branching is the whole reason the floor follows the aim, so a crossing
        // has to be a place rather than two passages fighting: the vault rises with
        // distance from the nearest wall, so where two passages meet it is taller
        // than either — and nothing had to work out that it was a junction.
        // Measured through the arch's own rule rather than the mesh, and with
        // passages narrower than the widest the vault knows about — which is the
        // only case where there is headroom left to gain. Two of those crossing put
        // a cell further from solid rock than anywhere along either, so the
        // crossing vaults higher, and nothing had to work out that it was a
        // junction.
        let passage = HALF_WIDE * 0.5;
        let crossing = (passage * passage * 2.0f32).sqrt();
        assert!(
            vault(crossing) > vault(passage) + 0.3,
            "a junction vaults {:.2} m where its passages vault {:.2}",
            vault(crossing),
            vault(passage)
        );
        // And a passage as wide as the vault knows about is already at full height,
        // so a chamber does not grow without bound.
        assert_eq!(vault(HALF_WIDE * 3.0), vault(HALF_WIDE), "the vault has no ceiling");
    }

    #[test]
    fn dug_ground_survives_being_written_and_read() {
        let mut dug = Dug::empty(HALF);
        dug.dig(Vec2::new(40.0, -20.0), 12.0, 25.5);
        dug.dig(Vec2::new(-60.0, 30.0), 8.0, 18.25);

        let back = read(&to_bytes(&dug), HALF).expect("should read back");
        assert_eq!(back.cells_dug(), dug.cells_dug());
        for at in [Vec2::new(40.0, -20.0), Vec2::new(-60.0, 30.0), Vec2::ZERO] {
            match (dug.floor_at(at), back.floor_at(at)) {
                (Some(was), Some(is)) => assert!((was - is).abs() < 0.001),
                (None, None) => {}
                (was, is) => panic!("{at:?} read back as {is:?}, not {was:?}"),
            }
        }
    }

    #[test]
    fn dug_ground_from_another_world_is_refused_with_a_reason() {
        let dug = Dug::empty(HALF);
        let why = read(&to_bytes(&dug), HALF * 2.0).unwrap_err();
        assert!(why.contains("world"), "unhelpful reason: {why}");
        assert_eq!(read(b"not mine at all", HALF).unwrap_err(), "not dug ground");

        let mut short = to_bytes(&dug);
        short.truncate(40);
        assert_eq!(read(&short, HALF).unwrap_err(), "truncated");
    }
}

#[cfg(test)]
mod one_rule {
    use super::*;

    /// Nothing may be drawn where the ground has been carved open.
    ///
    /// # Two rules for one question, photographed from inside
    ///
    /// `opening` eases the surface down over [`DOORWAY`], and the mesh asked the
    /// same question a second way — "is the roof down at the floor?" — which flips
    /// at a point rather than easing. Across the band between them BOTH answered
    /// yes: the terrain carved most of the way down to the floor, and the void laid
    /// its own floor and vault a few centimetres from it. Two surfaces a hair apart
    /// over a wide flat area is a depth-buffer fight, and it came back as a floor
    /// striped in pale bands with a ceiling that was really the underside of ground.
    ///
    /// This is the third time in this feature that one question had two answers —
    /// the biome boundary and the tunnel's own winding were the others — so it is
    /// worth a test that states the rule rather than the symptom: where the carve
    /// has opened the ground, the mesh keeps out.
    #[test]
    fn nothing_is_drawn_where_the_ground_is_carved_open() {
        // A hill with a shallow shoulder, so a route across it passes through every
        // depth of cover there is: open at the ends, the easing band, then sealed.
        let hill = |at: Vec2| 20.0 + 34.0 * (1.0 - (at.x.abs() / 150.0).min(1.0));

        let mut dug = Dug::empty(Vec2::splat(400.0));
        for step in -60..=60 {
            let at = Vec2::new(step as f32 * 2.5, 0.0);
            // Level, from the ground at one end — a lowered route.
            dug.dig(at, HALF_WIDE, hill(Vec2::new(150.0, 0.0)));
        }

        let (mesh, hewn) = void_parts(&dug, hill);
        assert!(!mesh.is_empty(), "nothing was drawn for a lowered route");

        // Every vertex drawn must be in a cell the carve left SEALED. A vertex in an
        // opened cell is a surface standing next to the terrain's own.
        // The FLOOR is the surface this is about: a second floor laid at the same
        // height as carved ground is the striping. It is a rule about CELLS, so it is
        // asked at each floor triangle's own middle rather than at its corners — a
        // corner is shared by four cells and belongs to none of them, and the ones
        // along a threshold sit exactly on the line between sealed and open.
        //
        // A doorway's dressed stone is left out: it stands in carved ground on
        // purpose, well clear of it, and that is the whole point of it.
        let mut trespass = 0;
        let mut sealed_seen = 0;
        for face in mesh.indices.chunks(3) {
            if face.iter().any(|&i| i as usize >= hewn) {
                continue;
            }
            if mesh.normals[face[0] as usize][1] < 0.5 {
                continue;
            }
            let middle = face
                .iter()
                .map(|&i| Vec3::from_array(mesh.places[i as usize]))
                .sum::<Vec3>()
                / 3.0;
            let at = Vec2::new(middle.x, middle.z);
            if dug.opening(at, hill(at)) > 0.0 {
                trespass += 1;
            } else {
                sealed_seen += 1;
            }
        }
        assert_eq!(
            trespass, 0,
            "{trespass} floor tiles are laid in ground the carve had already opened"
        );
        assert!(
            sealed_seen > 50,
            "only {sealed_seen} floor tiles under sealed ground"
        );

        // And the sealed part is a passage rather than a hall: its ceiling is the
        // vault's own height over the floor, not wherever the hilltop happens to be.
        let floor = hill(Vec2::new(150.0, 0.0));
        let tallest = mesh.places[..hewn]
            .iter()
            .map(|place| place[1] - floor)
            .fold(f32::MIN, f32::max);
        assert!(
            tallest <= HIGH + 0.2,
            "the ceiling stands {tallest:.1} m over the floor — that is a hall, not a tunnel"
        );
    }
}

// ------------------------------------------------------------------- the sketch

/// How far apart the points of a tidied route are, in metres.
///
/// Under half a cell, so consecutive brushfuls overlap heavily and the passage is
/// continuous rather than a string of beads.
const TIDY_STEP: f32 = CELL * 0.5;

/// The scale of wobble a tidied route smooths away, in metres.
///
/// A couple of brush widths. Nobody can drag a crosshair over a mountain in a
/// straight line — the hand shakes, the view swings, the ground drops away — and a
/// tunnel alignment is straights and gentle curves with no kinks in it. Below this
/// the wobble goes; above it the curve the maker meant survives.
const TIDY_REACH: f32 = 26.0;

/// Turns a hand-drawn sketch into a tunnel's alignment.
///
/// # A sketch is not a centreline
///
/// What comes off the mouse is neither smooth nor evenly spaced: the points arrive
/// one per frame of a drag, so a slow hand piles them up and a fast one leaves
/// them tens of metres apart — and a brushful at each would dig a row of beads
/// rather than a passage. The wobble is worse: a crosshair dragged over a hillside
/// shakes by metres, and a passage dug along that comes out with kinks a cart
/// could not take.
///
/// So the sketch is **resampled** at [`TIDY_STEP`], which fills every gap, and
/// then **smoothed** with its two ends pinned, which takes out the shake and
/// leaves the line the maker meant. The ends stay exactly where they were put
/// because those are the portals: the one part of a drawn route that is a
/// decision rather than an accident of the hand.
pub fn tidy(sketch: &[Vec2]) -> Vec<Vec2> {
    if sketch.len() < 2 {
        return sketch.to_vec();
    }

    // Resampled by arc length, so the spacing is even however fast the hand moved.
    let mut walked = vec![0.0_f32];
    for pair in sketch.windows(2) {
        walked.push(walked.last().copied().unwrap_or(0.0) + pair[0].distance(pair[1]));
    }
    let total = walked.last().copied().unwrap_or(0.0);
    if total < TIDY_STEP {
        return sketch.to_vec();
    }

    let steps = (total / TIDY_STEP).ceil() as usize;
    let mut route = Vec::with_capacity(steps + 1);
    let mut leg = 0;
    for step in 0..=steps {
        let want = (step as f32 / steps as f32) * total;
        while leg + 2 < sketch.len() && walked[leg + 1] < want {
            leg += 1;
        }
        let span = walked[leg + 1] - walked[leg];
        let t = if span > 1.0e-4 {
            ((want - walked[leg]) / span).clamp(0.0, 1.0)
        } else {
            0.0
        };
        route.push(sketch[leg].lerp(sketch[leg + 1], t));
    }

    // And smoothed, ends pinned. Laplacian passes rather than one wide average:
    // the same result, and each pass is a line of arithmetic that cannot get the
    // window's edges wrong.
    let passes = ((TIDY_REACH / TIDY_STEP).powi(2) * 0.5) as usize;
    for _ in 0..passes {
        let was = route.clone();
        for index in 1..route.len() - 1 {
            let middle = (was[index - 1] + was[index + 1]) * 0.5;
            route[index] = was[index].lerp(middle, 0.5);
        }
    }
    route
}

#[cfg(test)]
mod sketches {
    use super::*;

    /// A hand-drawn line, exaggerated: a straight run with a shake on it and one
    /// long gap where the view swung and the pointer jumped.
    fn scrawl() -> Vec<Vec2> {
        let mut sketch = Vec::new();
        for step in 0..=60 {
            let along = step as f32 * 10.0;
            // Skip a stretch, the way a fast drag does.
            if (25..32).contains(&step) {
                continue;
            }
            let shake = (along * 0.6).sin() * 7.0 + (along * 1.7).cos() * 3.0;
            sketch.push(Vec2::new(along, shake));
        }
        sketch
    }

    #[test]
    fn a_scrawl_becomes_an_alignment_without_moving_its_portals() {
        let sketch = scrawl();
        let route = tidy(&sketch);

        // The portals are the one part of a drawn line that is a decision.
        assert!(
            route[0].distance(sketch[0]) < 0.01,
            "the first portal moved to {:?}",
            route[0]
        );
        assert!(
            route[route.len() - 1].distance(sketch[sketch.len() - 1]) < 0.01,
            "the last portal moved"
        );

        // No gaps: every step is one brushful's overlap or less, so the passage is
        // continuous rather than a row of beads.
        let widest = route
            .windows(2)
            .map(|pair| pair[0].distance(pair[1]))
            .fold(0.0_f32, f32::max);
        assert!(
            widest <= TIDY_STEP + 0.01,
            "a {widest:.1} m gap survived — that is beads, not a tunnel"
        );

        // And the shake is gone. Measured as the worst corner in the line: a hand's
        // wobble turns a few degrees every couple of metres, an alignment does not.
        let sharpest = |line: &[Vec2]| {
            line.windows(3)
                .filter_map(|three| {
                    let a = (three[1] - three[0]).try_normalize()?;
                    let b = (three[2] - three[1]).try_normalize()?;
                    Some(a.angle_to(b).abs())
                })
                .fold(0.0_f32, f32::max)
        };
        let before = sharpest(&sketch);
        let after = sharpest(&route);
        assert!(
            after < before * 0.25,
            "the worst corner went from {:.0}° to {:.0}° — still a scrawl",
            before.to_degrees(),
            after.to_degrees()
        );
        assert!(
            after < 0.12,
            "a {:.0}° kink is left in the alignment",
            after.to_degrees()
        );
    }

    #[test]
    fn a_curve_the_maker_meant_survives_the_tidying() {
        // Smoothing must not straighten the line into something else. A deliberate
        // bend — a route swinging round a shoulder — has to still be there.
        let bend: Vec<Vec2> = (0..=60)
            .map(|step| {
                let t = step as f32 / 60.0;
                let angle = t * std::f32::consts::FRAC_PI_2;
                Vec2::new(angle.sin(), 1.0 - angle.cos()) * 400.0
            })
            .collect();
        let route = tidy(&bend);

        // How far the line departs from the straight between its ends: a quarter
        // circle of this size bows out by well over a hundred metres, and the
        // tidied one has to bow out nearly as far.
        let bow = |line: &[Vec2]| {
            let (from, to) = (line[0], line[line.len() - 1]);
            let axis = (to - from).normalize();
            line.iter()
                .map(|at| (*at - from).reject_from(axis).length())
                .fold(0.0_f32, f32::max)
        };
        let (was, is) = (bow(&bend), bow(&route));
        assert!(
            is > was * 0.9,
            "the bend was flattened from {was:.0} m to {is:.0} m of bow"
        );
    }

    #[test]
    fn a_dab_is_left_alone() {
        // Nothing to resample and nothing to smooth. It is refused upstream as too
        // short to be a route; this only has to not panic on it.
        assert_eq!(tidy(&[]).len(), 0);
        assert_eq!(tidy(&[Vec2::ZERO]).len(), 1);
        let pair = [Vec2::ZERO, Vec2::new(0.2, 0.0)];
        assert_eq!(tidy(&pair).len(), 2, "a sub-step pair should pass through");
    }
}

#[cfg(test)]
mod the_makers_own {
    use super::*;

    /// Whatever is in `assets/world/dug.bin` must come out as a cave.
    ///
    /// Not a fixture — the maker's own saved digging, read off disk. Three builds
    /// of this were argued about from screenshots while the one question nobody
    /// asked was whether the data on disk produced any interior at all.
    ///
    /// Skips itself when nothing has been dug, so it is a check on a real world
    /// rather than a test that fails on a clean checkout.
    #[test]
    fn what_is_on_disk_comes_out_as_a_cave() {
        let terrain = crate::world::terrain::Terrain::new();
        let dug = terrain.dug().read().expect("dug ground");
        if dug.is_empty() {
            return;
        }

        let mesh = void(&dug, |at| terrain.height(at.x, at.y));
        assert!(
            !mesh.is_empty(),
            "{} cells are dug and the cave came out with no geometry at all",
            dug.cells_dug()
        );

        // A cave has an inside: floor below, vault above, and enough of both to
        // stand in. Measured off the mesh rather than off the rules that made it.
        let low = mesh.places.iter().map(|p| p[1]).fold(f32::MAX, f32::min);
        let high = mesh.places.iter().map(|p| p[1]).fold(f32::MIN, f32::max);
        assert!(
            high - low > LEG,
            "the cave is {:.1} m from floor to ceiling — that is a sheet, not a space",
            high - low
        );

        // And it is UNDER something: somewhere along it there is real rock overhead,
        // or what has been dug is a trench rather than a cave.
        let mut deepest = 0.0_f32;
        for place in &mesh.places {
            let at = Vec2::new(place[0], place[2]);
            if let Some(floor) = dug.floor_at(at) {
                deepest = deepest.max(terrain.sealed_height(at.x, at.y) - floor);
            }
        }
        println!(
            "the maker's digging: {} cells, {} vertices, up to {deepest:.0} m of cover",
            dug.cells_dug(),
            mesh.places.len()
        );
        assert!(
            deepest > DOORWAY * 2.0,
            "the deepest cover anywhere is {deepest:.1} m — nothing is under a hill"
        );
    }
}

#[cfg(test)]
mod probe {
    use super::*;

    #[test]
    #[ignore = "a measurement of the maker's own world"]
    fn where_are_the_mouths() {
        let terrain = crate::world::terrain::Terrain::new();
        let dug = terrain.dug().read().expect("dug");
        if dug.is_empty() {
            println!("nothing dug");
            return;
        }
        let mut buckets = [0usize; 6];
        let mut mouths = Vec::new();
        let mut sampled = 0;
        // Walk the whole grid's dug cells by sampling their own middles.
        for (slot, floor) in dug.floor.iter().enumerate() {
            if *floor == UNDUG {
                continue;
            }
            sampled += 1;
            let at = dug.middle_of(slot % dug.wide, slot / dug.wide);
            let cover = terrain.sealed_height(at.x, at.y) - floor;
            let which = match cover {
                c if c < 0.5 => 0,
                c if c < DOORWAY => 1,
                c if c < DOORWAY * 2.0 => 2,
                c if c < 20.0 => 3,
                c if c < 80.0 => 4,
                _ => 5,
            };
            buckets[which] += 1;
            if cover < DOORWAY {
                mouths.push(at);
            }
        }
        println!("dug cells: {sampled}");
        println!("  cover < 0.5 m (open trench): {}", buckets[0]);
        println!("  cover < DOORWAY  (a MOUTH):  {}", buckets[1]);
        println!("  cover < 5.6 m   (thin roof): {}", buckets[2]);
        println!("  cover < 20 m:                {}", buckets[3]);
        println!("  cover < 80 m:                {}", buckets[4]);
        println!("  cover >= 80 m (deep):        {}", buckets[5]);
        if let (Some(first), Some(last)) = (mouths.first(), mouths.last()) {
            println!("mouth ground spans {first:?} .. {last:?}");
        } else {
            println!("NO MOUTH ANYWHERE - the cave is sealed shut");
        }
    }
}

#[cfg(test)]
mod watertight {
    use super::*;

    /// The cave's surfaces have to MEET, not merely be near each other.
    ///
    /// # A floor ruled into a pale grid
    ///
    /// The floor and vault were a quad per cell, each flat at its own cell's
    /// height — and neighbouring cells have slightly different floors, so no two
    /// tiles ever met. Every cell boundary was a hairline gap with the sky behind
    /// it. From inside, photographed: a floor drawn as a pale grid, and a ceiling
    /// full of ragged holes with daylight and trees through them.
    ///
    /// Nothing about a per-cell rule can fix that; the surfaces have to be built on
    /// shared corners so a boundary is one vertex rather than two nearly-equal ones.
    /// This checks that as a property of the mesh: every edge that ought to be
    /// interior is used by exactly two triangles, and every vertex position that
    /// appears twice appears at exactly the same height.
    #[test]
    fn the_floor_and_the_vault_have_no_seams_in_them() {
        let hill = |at: Vec2| 30.0 + 90.0 * (1.0 - (at.x.abs() / 200.0).min(1.0));
        let mut dug = Dug::empty(Vec2::splat(400.0));
        // A passage with a bend in it and a varying floor, which is what makes the
        // tiles disagree in the first place.
        for step in 0..=90 {
            let t = step as f32 / 90.0;
            let at = Vec2::new(-180.0 + t * 360.0, (t * 6.0).sin() * 30.0);
            dug.dig(at, HALF_WIDE, 34.0 + t * 9.0);
        }

        let (mesh, hewn) = void_parts(&dug, hill);
        assert!(!mesh.is_empty(), "nothing was drawn");

        // Corners shared rather than duplicated: two vertices at the same place
        // must be at the same height, or there is a step between the quads.
        use std::collections::HashMap;
        let mut heights: HashMap<(i64, i64, bool), Vec<f32>> = HashMap::new();
        let key = |place: &[f32; 3]| {
            (
                (place[0] * 16.0).round() as i64,
                (place[2] * 16.0).round() as i64,
            )
        };
        // The hewn cave only: a doorway's stones are boxes, and a box has a top and
        // a bottom in the same column as the floor and the vault it stands between.
        for (place, normal) in mesh.places[..hewn].iter().zip(&mesh.normals[..hewn]) {
            // Floors and ceilings are told apart by which way they face; walls are
            // allowed to share a column with both.
            if normal[1].abs() < 0.5 {
                continue;
            }
            let (x, z) = key(place);
            heights
                .entry((x, z, normal[1] > 0.0))
                .or_default()
                .push(place[1]);
        }
        let mut split = 0;
        for ys in heights.values() {
            let low = ys.iter().copied().fold(f32::MAX, f32::min);
            let high = ys.iter().copied().fold(f32::MIN, f32::max);
            if high - low > 0.001 {
                split += 1;
            }
        }
        assert_eq!(
            split, 0,
            "{split} places have two different heights for the same surface — \
             that is a seam, and the sky comes through it"
        );

        // And the vault is above the floor everywhere it is drawn, or the cave is
        // inside out somewhere.
        for (place, normal) in mesh.places[..hewn].iter().zip(&mesh.normals[..hewn]) {
            if normal[1] >= -0.5 {
                continue;
            }
            let at = Vec2::new(place[0], place[2]);
            if let Some(floor) = dug.floor_at(at) {
                assert!(
                    place[1] > floor,
                    "a ceiling vertex sits at {:.2} m, under its own floor at {floor:.2}",
                    place[1]
                );
            }
        }
    }
}

#[cfg(test)]
mod mouths {
    use super::*;

    /// No wall may stand where there is no rock to hold back.
    ///
    /// # The cave sealed its own doorways
    ///
    /// A wall was emitted wherever a DRAWN cell met a cell that was not drawn — and
    /// a cell the carve has opened is dug but not drawn, so every mouth got a wall
    /// built straight across it. Reported as "the entrances are still being drawn
    /// over", which is precisely what it was: from outside, a grey slab where a
    /// doorway should be.
    ///
    /// A wall holds back rock. Where the neighbour is dug there is no rock, whether
    /// or not this cave bothers to draw it, so the rule is about the DIGGING and not
    /// about the drawing.
    #[test]
    fn a_mouth_is_not_walled_shut() {
        // A hill with a plain in front of it, and a passage driven in from the
        // plain — so the route has one genuinely open end.
        let hill = |at: Vec2| 20.0 + 70.0 * crate::util::smoothstep(-40.0, 90.0, at.x);
        let mut dug = Dug::empty(Vec2::splat(400.0));
        for step in 0..=70 {
            let at = Vec2::new(-120.0 + step as f32 * 4.0, 0.0);
            dug.dig(at, HALF_WIDE, 20.0);
        }

        let (mesh, hewn) = void_parts(&dug, hill);
        assert!(!mesh.is_empty(), "nothing was drawn");

        // Every wall in the mesh must have rock on its far side. A wall's normal
        // points INTO the cave, so the side it holds back is half a cell the other
        // way.
        let mut walled = 0;
        let mut worst = None;
        for face in mesh.indices.chunks(3) {
            let corner = |i: usize| Vec3::from_array(mesh.places[face[i] as usize]);
            let normal = Vec3::from_array(mesh.normals[face[0] as usize]);
            if normal.y.abs() > 0.5 {
                continue;
            }
            // Not the doorway's own stones: a pillar is a THING standing in the
            // opening, and it has rock on some sides and air on others by design.
            if face.iter().any(|&i| i as usize >= hewn) {
                continue;
            }
            let middle = (corner(0) + corner(1) + corner(2)) / 3.0;
            // Snapped to the neighbouring CELL's own middle, which is the unit the
            // mesh decides on. `floor_at` is bilinear and reaches half a cell past
            // the dug ground, so an unsnapped probe calls the rock at the far end
            // of a passage "dug" and reports a wall that is doing its job.
            let behind = Vec2::new(middle.x, middle.z) - Vec2::new(normal.x, normal.z) * CELL * 0.6;
            let (bx, bz) = dug.cell_of(behind);
            let behind = dug.middle_of(bx.max(0) as usize, bz.max(0) as usize);
            if dug.open(bx, bz) {
                walled += 1;
                worst = worst.or(Some(behind));
            }
        }
        assert_eq!(
            walled, 0,
            "{walled} wall faces stand against dug ground — the first at {worst:?}, \
             which is a doorway with a wall across it"
        );

        // And the open end really is open: at the mouth the cave has a floor and no
        // roof over it, because the terrain has been carved down to the floor there.
        let mouth = Vec2::new(-110.0, 0.0);
        assert!(dug.floor_at(mouth).is_some(), "the mouth is not dug");
        // Open means the ground is not standing over the floor — either because the
        // carve took it down or, as here, because there was never any to take: at
        // the foot of the hill the floor IS the ground, and `opening` is nought
        // because there is nothing to carve rather than because it is sealed.
        let cover = hill(mouth) - dug.floor_at(mouth).expect("dug");
        assert!(
            cover < DOORWAY,
            "the mouth is under {cover:.1} m of ground — that is not a doorway"
        );
    }

    /// And there is room in it for a camera and for a crowd.
    #[test]
    fn the_passage_is_wide_and_tall_enough_to_follow_somebody_through() {
        // Asked for plainly: wider, and the ceiling raised, so the follow camera
        // can stay behind the warden and two monsters can pass each other.
        assert!(
            HALF_WIDE * 2.0 >= 16.0,
            "a {:.0} m wide passage is a corridor",
            HALF_WIDE * 2.0
        );
        assert!(HIGH >= 9.0, "a {HIGH} m ceiling is too low to follow anybody through");
        // The camera sits about this far above what it is watching; it has to fit
        // under the crown with something to spare.
        assert!(
            HIGH > crate::camera::LOOK_HEIGHT + 6.0,
            "only {:.1} m of clearance over the warden's shoulders",
            HIGH - crate::camera::LOOK_HEIGHT
        );
        // The vault has to be an arch across that width, not a flat lid.
        assert!(
            vault(HALF_WIDE) > vault(0.0) + 3.0,
            "the vault only rises {:.1} m from wall to crown",
            vault(HALF_WIDE) - vault(0.0)
        );
    }

    /// A doorway is ONE built thing, and it stands where somebody can see it.
    ///
    /// # "Not sure if this is worse"
    ///
    /// The first doorway was a stone per threshold cell and a porch of vault
    /// reaching out over the approach. Photographed: a floating crenellated wall
    /// over a black tent with a torn hem — each lintel at its own cell's crown, ten
    /// metres up, and the porch showing the cave's baked-dark ceiling to the sky.
    ///
    /// So the rule, stated: at a mouth the dressed stone is exactly three stones —
    /// a pillar either side and one beam — the frame spans the opening, and it
    /// stands in the open air over the carved ground, not inside the hill.
    #[test]
    fn a_doorway_frames_the_mouth() {
        let hill = |at: Vec2| 20.0 + 70.0 * crate::util::smoothstep(-40.0, 90.0, at.x);
        let mut dug = Dug::empty(Vec2::splat(400.0));
        for step in 0..=70 {
            let at = Vec2::new(-120.0 + step as f32 * 4.0, 0.0);
            dug.dig(at, HALF_WIDE, 20.0);
        }

        let (mesh, hewn) = void_parts(&dug, hill);
        let dressed = &mesh.places[hewn..];
        assert_eq!(
            dressed.len(),
            3 * 24,
            "one mouth should be three stones, not {}",
            dressed.len() / 24
        );

        // Standing in the open, over the ground as the carve leaves it. The
        // footings are SET INTO the ground, so the bar is half — a frame buried in
        // the hillside fails it, a frame standing on its feet does not.
        let proud = dressed
            .iter()
            .filter(|place| {
                let at = Vec2::new(place[0], place[2]);
                let seen = hill(at) - dug.opening(at, hill(at));
                place[1] > seen
            })
            .count();
        assert!(
            proud * 2 > dressed.len(),
            "only {proud} of {} dressed vertices stand clear of the ground",
            dressed.len()
        );

        // And the beam's top rides clear of the UNCARVED hillside around it, or
        // there is nothing to spot from the air.
        let top = dressed.iter().map(|place| place[1]).fold(f32::MIN, f32::max);
        for place in dressed.iter().filter(|place| place[1] >= top - 0.01) {
            let over = place[1] - hill(Vec2::new(place[0], place[2]));
            assert!(
                over > 1.0,
                "the beam's top clears the hillside by only {over:.1} m"
            );
        }

        // Spanning the opening, which runs across z here.
        let (near, far) = dressed
            .iter()
            .fold((f32::MAX, f32::MIN), |(lo, hi), place| {
                (lo.min(place[2]), hi.max(place[2]))
            });
        assert!(
            far - near > HALF_WIDE,
            "the frame spans {:.1} m of an opening {:.0} m wide",
            far - near,
            HALF_WIDE * 2.0
        );

        // And at the OPENING's height, not the vault's: a beam ten metres up reads
        // as a floating wall, not a doorway.
        let floor = 20.0;
        assert!(
            top - floor < DOORWAY + CLEARANCE + LINTEL + 0.1,
            "the frame tops out {:.1} m over the floor",
            top - floor
        );
    }
}

#[cfg(test)]
mod entrances {
    use super::*;

    /// A tunnel has to have a hole you can walk in through.
    ///
    /// # Reported four times as "still no entrances"
    ///
    /// The carve faded out across `DOORWAY` with a smoothstep, so between the pad it
    /// cut and the first cell the cave was drawn in there was a band carved only
    /// PART of the way down — the ground ramped back up and sealed over the passage.
    /// Measured at the time: the pad ended at 20 m, the band stood at 21.2, and the
    /// cave began under ground at 24.5. There was no hole because the hillside
    /// covered it.
    ///
    /// Carve and cave PARTITION the dug ground now, from one number: ground under
    /// `DOORWAY` is cut away to the floor with no roof; ground over it is left alone
    /// with the cave beneath. The step that leaves in the terrain is the mouth's own
    /// lintel, and the gap between the floor and the vault behind it is the doorway.
    #[test]
    fn a_tunnel_has_a_hole_you_can_walk_in_through() {
        let hill = |at: Vec2| 20.0 + 80.0 * crate::util::smoothstep(-60.0, 120.0, at.x);
        let mut dug = Dug::empty(Vec2::splat(400.0));
        for step in 0..=80 {
            dug.dig(Vec2::new(-160.0 + step as f32 * 4.0, 0.0), HALF_WIDE, 20.0);
        }

        // Walk in along the passage and find the first cell the cave is drawn in —
        // the mouth — checking the ground never rises between the pad and it.
        let mut pad_top = f32::MIN;
        let mut mouth = None;
        for step in 0..120 {
            let at = Vec2::new(-160.0 + step as f32 * 2.0, 0.0);
            let Some(floor) = dug.floor_at(at) else { continue };
            let cover = hill(at) - floor;
            if cover >= DOORWAY {
                mouth = Some((at, floor, cover));
                break;
            }
            // Still outside: the ground here must be cut ALL the way to the floor,
            // or it is a ramp burying whatever is behind it.
            let carved = hill(at) - dug.opening(at, hill(at));
            pad_top = pad_top.max(carved - floor);
        }
        let (at, floor, cover) = mouth.expect("the passage never reaches sealed ground");
        assert!(
            pad_top < 0.01,
            "the ground stands {pad_top:.2} m above the floor on the way in — a ramp              over the mouth, not a pad up to it"
        );

        // And the mouth is a hole with room in it: the vault behind the lintel
        // clears the floor by something a walker fits through.
        let headroom = (hill(at) - LID).min(floor + vault(HALF_WIDE)) - floor;
        assert!(
            headroom > 3.0,
            "the opening at the mouth is only {headroom:.1} m tall"
        );
        assert!(
            cover >= DOORWAY && cover < DOORWAY + 2.0,
            "the mouth sits under {cover:.1} m of ground, which is not where the              carve hands over"
        );
    }

    /// Nothing may be roofed where there is no ground to roof it with.
    #[test]
    fn flat_ground_gets_a_cutting_and_not_a_cave() {
        // Out past a hill the floor IS the ground: nothing to carve, and nothing to
        // put a roof on. Asking `opening() <= 0` got this wrong — there is nothing
        // to carve there, so opening is nought, so a cave was drawn with its vault
        // clamped BELOW its own floor.
        let flat = |_: Vec2| 20.0;
        let mut dug = Dug::empty(Vec2::splat(400.0));
        for step in 0..=40 {
            dug.dig(Vec2::new(-80.0 + step as f32 * 4.0, 0.0), HALF_WIDE, 20.0);
        }
        let mesh = void(&dug, flat);
        assert!(
            mesh.is_empty(),
            "{} vertices of cave were drawn on flat ground with nothing over it",
            mesh.places.len()
        );
    }

    #[test]
    #[ignore = "a measurement"]
    fn what_stands_between_the_carved_pad_and_the_cave() {
        let hill = |at: Vec2| 20.0 + 80.0 * crate::util::smoothstep(-60.0, 120.0, at.x);
        let mut dug = Dug::empty(Vec2::splat(400.0));
        for step in 0..=80 {
            dug.dig(Vec2::new(-160.0 + step as f32 * 4.0, 0.0), HALF_WIDE, 20.0);
        }
        println!(" x    ground  floor  cover  drawn-surface  headroom  what");
        for step in 0..34 {
            let at = Vec2::new(-100.0 + step as f32 * 6.0, 0.0);
            let g = hill(at);
            let Some(floor) = dug.floor_at(at) else { continue };
            let opening = dug.opening(at, g);
            // Whether void() would draw this cell: its own rule, corners and all.
            // The same rule `void` uses, so the table cannot lie about the mesh.
            let drawn = g - floor >= DOORWAY;
            let _ = opening;
            let vault_top = if drawn { (g - LID).min(floor + vault(HALF_WIDE)) } else { floor };
            println!(
                "{:6.0} {g:7.1} {floor:6.1} {:6.1} {:12.1} {:9.1}  {}",
                at.x,
                g - floor,
                g - opening,
                if drawn { vault_top - floor } else { 0.0 },
                if drawn { "cave" } else { "open cut" }
            );
        }
    }

    #[test]
    #[ignore = "a measurement of the maker's own world"]
    fn what_the_doorways_look_like() {
        let terrain = crate::world::terrain::Terrain::new();
        let dug = terrain.dug().read().expect("dug");
        if dug.is_empty() {
            println!("nothing dug");
            return;
        }
        let (mesh, hewn) = void_parts(&dug, |at| terrain.sealed_height(at.x, at.y));
        let dressed = mesh.places.len() - hewn;
        println!(
            "the cave: {} cells dug, {hewn} hewn vertices, {dressed} of dressed stone",
            dug.cells_dug()
        );
        // 24 vertices to a block, so this counts the stones that were stood up.
        println!("stones stood: {}", dressed / 24);

        // And where they are, clustered so a maker can walk to one.
        let mut clusters: Vec<(Vec2, f32, f32, usize)> = Vec::new();
        for place in &mesh.places[hewn..] {
            let at = Vec2::new(place[0], place[2]);
            match clusters
                .iter_mut()
                .find(|(middle, _, _, _)| middle.distance(at) < 60.0)
            {
                Some((_, low, high, count)) => {
                    *low = low.min(place[1]);
                    *high = high.max(place[1]);
                    *count += 1;
                }
                None => clusters.push((at, place[1], place[1], 1)),
            }
        }
        for (at, low, high, count) in &clusters {
            println!(
                "  a doorway near ({:.0}, {:.0}): {} m of stone standing, {count} vertices, \
                 ground {:.0} m",
                at.x,
                at.y,
                (high - low).round(),
                terrain.sealed_height(at.x, at.y)
            );
        }
        assert!(!clusters.is_empty(), "no doorway was built anywhere");

        // The question that matters: does the stone stand in OPEN AIR, or is it
        // buried in the hill? `height` is the surface as rendered, carve and all —
        // stone above it is stone a maker can see. Four reports of "no entrance"
        // were all this: something real, and under the ground.
        let mut in_air = 0;
        let mut buried = 0;
        let mut tallest = 0.0_f32;
        for place in &mesh.places[hewn..] {
            let surface = terrain.height(place[0], place[2]);
            if place[1] > surface + 0.05 {
                in_air += 1;
                tallest = tallest.max(place[1] - surface);
            } else {
                buried += 1;
            }
        }
        println!(
            "dressed stone: {in_air} vertices in open air, {buried} buried; \
             the highest stands {tallest:.1} m clear of the ground"
        );
        assert!(
            tallest > 3.0,
            "the tallest stone stands {tallest:.1} m out of the ground — \
             that is not something anybody will spot from the air"
        );
    }
}
