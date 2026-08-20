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
//! # The floor is where you were AIMING, stamped flat
//!
//! A brushful takes the height of the point under the pointer and lays the whole
//! footprint at that one height. Not the terrain's own height per cell, which is
//! what would make a dug floor lumpy — and a floor a man dug with a pick is not
//! lumpy. Aim lower as you go and the tunnel slopes; aim level and it is level.
//!
//! Where a brushful crosses ground already dug, the LOWER floor stands: you can dig
//! deeper but you cannot un-dig by painting over it. Filling in is its own stroke.
//!
//! # A shaft to nowhere
//!
//! Nothing may be dug below [`DEEPEST`]. A pointer can only aim at the surface, so
//! the floor is always some real height somewhere — but a low aim carried into a
//! hillside is how you would dig straight down and out through the bottom of the
//! world, and that is a hole nobody can climb out of.

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
pub const HALF_WIDE: f32 = 6.0;
pub const LEG: f32 = 2.6;
pub const HIGH: f32 = 6.5;

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
        Some((
            centre - Vec2::splat(radius + CELL * 2.0),
            centre + Vec2::splat(radius + CELL * 2.0),
        ))
    }

    /// The floor of the void under a point, or `None` where nothing is dug.
    ///
    /// Read BETWEEN cells, among the open ones only. Nearest-cell would step by two
    /// metres at every boundary, which is a lumpy floor — and blending with the
    /// closed cells around a void would drag its floor toward nothing at the walls.
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
    let Some((low, high)) = dug.bounds() else {
        return Geometry::default();
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

    let mut mesh = Geometry::default();
    let stone = |shade: f32| [0.108 * shade, 0.101 * shade, 0.094 * shade, 1.0];

    for slot in 0..open.len() {
        if !open[slot] {
            continue;
        }
        let (cx, cz) = (x0 + (slot % wide) as isize, z0 + (slot / wide) as isize);
        let middle = dug.middle_of(cx as usize, cz as usize);
        let Some(floor) = dug.floor_at(middle) else {
            continue;
        };
        let (a, b) = (middle - Vec2::splat(CELL * 0.5), middle + Vec2::splat(CELL * 0.5));
        // Never through the hillside above — measured at the cell's CORNERS and not
        // its middle. A cell is two metres across on a slope that can fall a metre
        // in that distance, so a roof cleared against the middle still broke the
        // surface at the downhill corner. The lowest corner is the one that decides.
        let lowest = [
            Vec2::new(a.x, a.y),
            Vec2::new(b.x, a.y),
            Vec2::new(a.x, b.y),
            Vec2::new(b.x, b.y),
        ]
        .into_iter()
        .map(&ground)
        .fold(f32::MAX, f32::min);
        let roof = (floor + vault(inward[slot])).min(lowest - LID);

        // How deep in the dark this is: a cell with sky close by is lit, and one
        // well inside the hill is not. No lamps exist yet, so the vertex colours
        // carry the light.
        let overhead = (ground(middle) - floor - HIGH).max(0.0);
        let dark = 1.0 - crate::util::smoothstep(0.0, 26.0, overhead) * 0.82;

        let quad = |mesh: &mut Geometry, corners: [Vec3; 4], up: bool, shade: f32| {
            let base = mesh.places.len() as u32;
            let normal = if up { Vec3::Y } else { -Vec3::Y };
            for corner in corners {
                mesh.places.push(corner.to_array());
                mesh.normals.push(normal.to_array());
                mesh.uvs.push([0.0, 0.0]);
                mesh.colours.push(stone(shade));
            }
            if up {
                mesh.indices
                    .extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
            } else {
                mesh.indices
                    .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
            }
        };

        // The floor, seen from above.
        let sole = floor + FLOOR_LIFT;
        quad(
            &mut mesh,
            [
                Vec3::new(a.x, sole, a.y),
                Vec3::new(b.x, sole, a.y),
                Vec3::new(b.x, sole, b.y),
                Vec3::new(a.x, sole, b.y),
            ],
            true,
            dark * 1.15,
        );
        // The vault, seen from below.
        quad(
            &mut mesh,
            [
                Vec3::new(a.x, roof, a.y),
                Vec3::new(b.x, roof, a.y),
                Vec3::new(b.x, roof, b.y),
                Vec3::new(a.x, roof, b.y),
            ],
            false,
            dark * 0.8,
        );

        // And a wall wherever the rock next door is still solid.
        for (dx, dz) in [(1_isize, 0_isize), (-1, 0), (0, 1), (0, -1)] {
            let (nx, nz) = ((slot % wide) as isize + dx, (slot / wide) as isize + dz);
            let solid = nx < 0
                || nz < 0
                || nx as usize >= wide
                || nz as usize >= deep
                || !open[nz as usize * wide + nx as usize];
            if !solid {
                continue;
            }
            // The face between this cell and that one, from floor to vault, wound
            // to look back at the cell it belongs to.
            let (p, q) = match (dx, dz) {
                (1, 0) => (Vec2::new(b.x, a.y), Vec2::new(b.x, b.y)),
                (-1, 0) => (Vec2::new(a.x, b.y), Vec2::new(a.x, a.y)),
                (0, 1) => (Vec2::new(b.x, b.y), Vec2::new(a.x, b.y)),
                _ => (Vec2::new(a.x, a.y), Vec2::new(b.x, a.y)),
            };
            let base = mesh.places.len() as u32;
            let inward_normal = Vec3::new(-dx as f32, 0.0, -dz as f32);
            for corner in [
                Vec3::new(p.x, sole, p.y),
                Vec3::new(q.x, sole, q.y),
                Vec3::new(q.x, roof, q.y),
                Vec3::new(p.x, roof, p.y),
            ] {
                mesh.places.push(corner.to_array());
                mesh.normals.push(inward_normal.to_array());
                mesh.uvs.push([0.0, 0.0]);
                mesh.colours.push(stone(dark));
            }
            mesh.indices
                .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
    }
    mesh
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
) {
    let Some(material) = material else {
        return;
    };
    for old in &standing {
        commands.entity(old).despawn();
    }
    let Ok(dug) = terrain.0.dug().read() else {
        return;
    };
    if dug.is_empty() {
        return;
    }
    let mesh = void(&dug, |at| terrain.0.height(at.x, at.y));
    if mesh.is_empty() {
        return;
    }
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
