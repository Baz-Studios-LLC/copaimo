//! Tunnels a maker has bored, and the rock left standing over them.
//!
//! # Why this exists rather than another constant
//!
//! The mountain pass was written in code: a place, a heading, a thickness. Moving
//! it meant reading a screenshot and guessing which number it implied, and it went
//! wrong three times in an evening — the wall crossed the desert boundary instead
//! of following it, then it was a mesa, then it was too thin. Every one of those
//! was the same fault the countries had before they were paintable, and it has the
//! same answer: **the person who can see where a tunnel belongs should be the one
//! putting it there.**
//!
//! So a bore is data. Two points on the ground, picked in the terrain tool, and
//! the world does the rest. [`crate::world::pass`] keeps the mountain itself,
//! because a mountain is a landscape and there is already a brush for raising one.
//!
//! # The hill keeps its skin, and the tunnel is a tube inside it
//!
//! The first build of this carved the corridor down to a floor and left the rock
//! above it standing as a mesh. Geometrically sound, and wrong to look at: bore
//! through a small rise and the "rock above" is a thin shell reading as a black
//! tent pitched on the ground; bore out over water and it is a sliver hanging in
//! the air. Both were photographed within a minute of the tool existing.
//!
//! So a bore leaves the ground alone — **all** of it. What it does is build the
//! **tube**: a floor and an arched ceiling, mouth to mouth, seen from inside, under
//! ground that keeps its trees, its snow and its shading because nothing carved it.
//!
//! # Nothing is carved, and the mouths still come out flush
//!
//! The build before this carved a short cutting at each mouth so there was
//! somewhere to walk in from. The extent of that carve was decided by how thinly
//! the hill held the tunnel — which on a gentle slope is hundreds of metres, so
//! boring through a low rise gouged a valley across the whole hillside. It was
//! photographed within a minute, again.
//!
//! It was never needed. A bore's mouths are put at the ground's own height (see
//! [`Bore::aimed`]) and its floor is the straight line between them, so **at both
//! ends the floor already meets the surface** — the tube opens onto the hillside
//! flush, with nothing to cut away. Everywhere between, the floor is under the hill
//! and the hill is untouched.
//!
//! If a maker wants a mouth widened into a proper portal, the ground brushes are
//! right there and always were. A tool that gouges a valley nobody asked for to
//! save a maker two strokes of LOWER is not a saving.
//!
//! # A bore needs something to bore through
//!
//! Over flat ground or open water there is no rock overhead, so there is nothing to
//! make a tunnel in — and the tool says so rather than laying a mesh in the open.
//! See [`Bore::has_rock_over_it`].
//!
//! # The floor is remembered, not re-derived
//!
//! A bore stores the height of the ground at each of its mouths, taken once when
//! it is laid. It cannot ask the terrain, because the terrain is asking IT — the
//! carve is part of the height, and a floor worked out from the carved ground would
//! sink a little further every time the question was asked.

use bevy::prelude::*;

use terrain_core::Geometry;

/// One tunnel: two mouths, and the floor it runs between them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bore {
    pub from: Vec2,
    pub to: Vec2,
    /// The ground's height at each mouth when this was laid — see the note above
    /// about why it is remembered rather than asked for.
    pub floor_from: f32,
    pub floor_to: f32,
}

/// Half the width of the arch, and how high its crown stands over the floor.
///
/// Eleven metres across and seven high: round enough to read as a hole bored
/// through rock rather than a letterbox, and four times a person's height overhead,
/// because a tunnel you stoop in reads as a drain.
pub const WIDE: f32 = 5.5;
pub const HIGH: f32 = 7.0;

/// The shortest tunnel worth making, in metres.
///
/// Two presses in nearly the same place is a slip, not a tunnel — and it is also
/// the reach used to decide which bore a FILL IN press means, since a tunnel is
/// picked by pointing near it.
pub const SPAN: f32 = 10.0;

/// How far above a floor still counts as being IN the tunnel — see
/// [`Bores::walk_floor`].
pub const HEADROOM: f32 = HIGH * 1.6;

/// How far above the waterline ground has to be to put a mouth on it.
///
/// A mouth is somewhere a person stands, so it wants dry land rather than the
/// waterline itself — a doorway with the tide in it is a well.
const STANDABLE: f32 = 1.5;

/// How far the tube's floor stands above the ground it copies, so the two are
/// never the same surface twice where a cutting has carved down to it.
const FLOOR_LIFT: f32 = 0.12;

/// How the tube is built: points around each ring of the arch, and metres between
/// rings along the tunnel.
const RING_POINTS: usize = 14;
const RING_STEP: f32 = 4.0;

impl Bore {
    /// Where a point sits in this bore's frame: how far along, 0 at one mouth and
    /// 1 at the other, and how far to the side in metres.
    fn local(&self, at: Vec2) -> (f32, f32) {
        let run = self.to - self.from;
        let length = run.length().max(1.0e-3);
        let along = run / length;
        let away = at - self.from;
        (away.dot(along) / length, away.dot(Vec2::new(-along.y, along.x)))
    }

    /// How far this bore moves the ground at a point: never, and by nothing.
    ///
    /// A method rather than a comment because it is a CLAIM — that boring a tunnel
    /// leaves the surface exactly as it found it — and a claim is a thing a test can
    /// hold onto. Two builds in a row carved something here and both were wrong.
    pub fn cut_nothing(&self, _at: Vec2) -> f32 {
        0.0
    }

    /// How long this bore is, in metres.
    pub fn length(&self) -> f32 {
        self.from.distance(self.to)
    }

    /// The floor's height at a point along the bore.
    fn floor_at(&self, t: f32) -> f32 {
        self.floor_from + (self.floor_to - self.floor_from) * t.clamp(0.0, 1.0)
    }

    /// How much rock stands over the tunnel at a point, in metres.
    ///
    /// `ground` is the height WITHOUT any bore cut into it — what the hill actually
    /// is. Negative where the ground has dropped below the floor line, which is
    /// what happens out past a hillside.
    fn overhead(&self, at: Vec2, ground: f32) -> f32 {
        let (t, _) = self.local(at);
        ground - self.floor_at(t)
    }


    /// The tunnel floor under a point, if that point is inside the tube with real
    /// rock overhead.
    pub fn floor(&self, at: Vec2, ground: f32) -> Option<f32> {
        let (t, across) = self.local(at);
        // From the mouth INWARD, not from wherever the rock happens to be a full
        // arch deep. This wanted `HIGH` of rock overhead before the floor would
        // claim a walker, so entering meant climbing the hillside until the rock
        // was deep enough and then dropping in — a 6.8 m fall, which the walk test
        // caught by measuring the step. A tunnel takes your feet the moment you
        // are in it.
        ((0.0..=1.0).contains(&t)
            && across.abs() < WIDE
            && self.overhead(at, ground) > FLOOR_LIFT * 2.0)
            .then(|| self.floor_at(t) + FLOOR_LIFT)
    }

    /// A tunnel between two aimed points, trimmed to ground somebody can walk on.
    ///
    /// # Aim at the hill, not at the tunnel
    ///
    /// The maker says "through here to there" by pointing at two places. What they
    /// are NOT doing is surveying: an aim that overshoots a hill into open water is
    /// the normal way to use this, and the tool's job is to make the tunnel that
    /// aim describes rather than to refuse the aim.
    ///
    /// So both ends are walked in to the first ground a person could stand on, and
    /// the mouths are put there. Aim across a shoreline and the tunnel comes out at
    /// the shore. Aim clean past a hill on both sides and it comes out at the foot
    /// on both sides. The one thing this cannot fix is an aim with no hill in it at
    /// all, which [`Self::makes_sense`] answers separately.
    ///
    /// `None` when there is no standable ground along the aim anywhere — pointing
    /// out to sea and further out to sea.
    pub fn aimed(from: Vec2, to: Vec2, ground: impl Fn(Vec2) -> f32) -> Option<Self> {
        let reach = from.distance(to);
        if reach < SPAN {
            return None;
        }
        let steps = (reach / 4.0).ceil() as i32;
        let dry = |t: f32| {
            let at = from.lerp(to, t);
            ground(at) > crate::config::SEA_LEVEL + STANDABLE
        };
        let head = (0..=steps).map(|i| i as f32 / steps as f32).find(|t| dry(*t))?;
        let tail = (0..=steps)
            .rev()
            .map(|i| i as f32 / steps as f32)
            .find(|t| dry(*t))?;
        if (tail - head) * reach < SPAN {
            return None;
        }

        let mouth = from.lerp(to, head);
        let far = from.lerp(to, tail);
        Some(Self {
            from: mouth,
            to: far,
            floor_from: ground(mouth),
            floor_to: ground(far),
        })
    }

    /// Whether there is anything here to bore THROUGH.
    ///
    /// The one thing trimming the ends cannot fix: an aim with no hill in it. Over
    /// a plain there is no rock over the floor anywhere, so there is no tunnel to
    /// make and a mesh laid there would hang in the open.
    pub fn makes_sense(&self, ground: impl Fn(Vec2) -> f32) -> Result<(), &'static str> {
        let steps = 40;
        let rocked = (0..=steps).any(|step| {
            let at = self.from.lerp(self.to, step as f32 / steps as f32);
            self.overhead(at, ground(at)) > HIGH * 1.5
        });
        if rocked {
            Ok(())
        } else {
            Err("nothing to bore through - aim at a hill")
        }
    }

    /// The tunnel itself: a floor and an arched ceiling from mouth to mouth.
    ///
    /// Wound to face INWARD, because inside is the only place it is ever seen from:
    /// through a mouth its far wall shows as the dark of the tunnel and its near
    /// wall is culled. The light is baked into the vertex colours — stone at the
    /// mouths falling to near-black through the middle — because no sun reaches a
    /// tunnel and there are no lamps yet.
    pub fn tube(&self, ground: impl Fn(Vec2) -> f32) -> Geometry {
        let length = self.length();
        if length < SPAN {
            return Geometry::default();
        }
        let along_way = (self.to - self.from) / length;
        let across_way = Vec2::new(-along_way.y, along_way.x);

        // # The tube is only as long as the rock is
        //
        // Built mouth to mouth, a bore aimed generously past a hill — or out over
        // water, which is how this was found — hangs its lining in the open air at
        // whichever end ran out of ground. So the ends are FOUND: the tube spans
        // the stretch that genuinely has rock over it, and a maker can aim well
        // clear of a hill and get a tunnel exactly as long as the hill is.
        let rocked = |t: f32| {
            let at = self.from.lerp(self.to, t);
            self.overhead(at, ground(at)) > HIGH * 0.6
        };
        let fine = 200;
        let first = (0..=fine).map(|i| i as f32 / fine as f32).find(|t| rocked(*t));
        let last = (0..=fine)
            .rev()
            .map(|i| i as f32 / fine as f32)
            .find(|t| rocked(*t));
        let (Some(head), Some(tail)) = (first, last) else {
            return Geometry::default();
        };
        // A margin either way, so each end sits inside its own cutting rather than
        // flush with where the rock gave out.
        let margin = (SPAN * 1.4) / length;
        let head = (head - margin).max(0.0);
        let tail = (tail + margin).min(1.0);
        if (tail - head) * length < SPAN {
            return Geometry::default();
        }

        let steps = (((tail - head) * length / RING_STEP).ceil() as usize).max(2);
        let ring = RING_POINTS + 1 + 2;

        let mut mesh = Geometry::default();
        for step in 0..=steps {
            let run = step as f32 / steps as f32;
            let t = head + run * (tail - head);
            let middle = self.from + along_way * (t * length);
            let floor = self.floor_at(t) + FLOOR_LIFT;
            // How deep in the hill this ring is, 0 at either end of the LINING to
            // 1 in its middle: the baked darkness.
            let dark = crate::util::smoothstep(0.0, 0.35, 0.5 - (run - 0.5).abs());

            for point in 0..=RING_POINTS {
                let across = (point as f32 / RING_POINTS as f32 * 2.0 - 1.0) * WIDE;
                let up = HIGH * (1.0 - (across / WIDE).powi(2)).max(0.0).sqrt();
                let at = middle + across_way * across;
                let grain =
                    terrain_core::forest::field(Vec2::new(t * length, across * 3.0) / 9.0, 82);
                let shade = (0.72 + grain * 0.28) * (1.0 - dark * 0.88);
                mesh.places.push([at.x, floor + up, at.y]);
                mesh.normals.push([0.0, 0.0, 0.0]);
                mesh.uvs.push([point as f32 / RING_POINTS as f32, step as f32]);
                mesh.colours
                    .push([0.125 * shade, 0.118 * shade, 0.11 * shade, 1.0]);
            }
            // The floor pair under the arch's feet. The tube owns the ground a
            // walker stands on: the heightfield in here is the hill's TOP.
            for side in [-1.0_f32, 1.0] {
                let at = middle + across_way * side * WIDE;
                let grain =
                    terrain_core::forest::field(Vec2::new(t * length, side * 40.0) / 9.0, 83);
                let shade = (0.6 + grain * 0.2) * (1.0 - dark * 0.85);
                mesh.places.push([at.x, floor, at.y]);
                mesh.normals.push([0.0, 0.0, 0.0]);
                mesh.uvs.push([0.5 + side * 0.5, step as f32]);
                mesh.colours
                    .push([0.14 * shade, 0.132 * shade, 0.122 * shade, 1.0]);
            }
        }

        for step in 0..steps {
            let a = (step * ring) as u32;
            let b = ((step + 1) * ring) as u32;
            for point in 0..RING_POINTS {
                let (p0, p1) = (a + point as u32, a + point as u32 + 1);
                let (q0, q1) = (b + point as u32, b + point as u32 + 1);
                // The arch is seen from BELOW, so it is wound the opposite way to
                // a ground surface: a ring step crossed with a tunnel step points
                // up, which is into the rock.
                mesh.indices.extend_from_slice(&[p0, q0, p1, p1, q0, q1]);
            }
            let (f0, f1) = (a + ring as u32 - 2, a + ring as u32 - 1);
            let (g0, g1) = (b + ring as u32 - 2, b + ring as u32 - 1);
            // And the floor from ABOVE, like any ground.
            mesh.indices.extend_from_slice(&[f0, f1, g0, f1, g1, g0]);
        }

        settle_the_normals(&mut mesh);
        mesh
    }
}

/// Gives every vertex the sum of the faces meeting at it.
///
/// A normal built OUT OF the winding cannot contradict it. The pass was drawn
/// inside out for a whole session because its normals were written down separately
/// from its faces.
fn settle_the_normals(mesh: &mut Geometry) {
    for face in mesh.indices.chunks(3) {
        let corner = |i: usize| Vec3::from_array(mesh.places[face[i] as usize]);
        let weight = (corner(1) - corner(0)).cross(corner(2) - corner(0));
        for slot in face {
            let normal = &mut mesh.normals[*slot as usize];
            normal[0] += weight.x;
            normal[1] += weight.y;
            normal[2] += weight.z;
        }
    }
    for normal in &mut mesh.normals {
        let settled = Vec3::from_array(*normal).normalize_or_zero();
        *normal = if settled.length_squared() < 0.5 {
            [0.0, 1.0, 0.0]
        } else {
            settled.to_array()
        };
    }
}

// ------------------------------------------------------------------- the layer

/// Every bore in the world.
#[derive(Resource, Default, Debug)]
pub struct Bores {
    list: Vec<Bore>,
    /// Whether anything has changed since this was last written.
    pub unsaved: bool,
}

impl Bores {
    pub fn all(&self) -> &[Bore] {
        &self.list
    }

    pub fn len(&self) -> usize {
        self.list.len()
    }

    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    /// The height a walker's feet belong at, or `None` if no tunnel claims them.
    ///
    /// # The one place this world has two grounds
    ///
    /// Everywhere else the ground is a height per place and feet snap to it. In a
    /// tunnel there are two: the floor, and the hill's own top above it. Which one
    /// a walker is on cannot be told from where they are standing — both are
    /// directly overhead one another — so it is told from where they are standing
    /// NOW: somebody already down at floor level stays down, somebody up on the
    /// hill stays up.
    ///
    /// That is also what makes a mouth work with no door: walk in along the cutting
    /// and the drawn ground carries you down to the floor, and by the time the rock
    /// closes overhead you are already low enough to be claimed.
    pub fn walk_floor(&self, at: Vec2, ground: f32, standing: f32) -> Option<f32> {
        self.list
            .iter()
            .filter_map(|bore| bore.floor(at, ground))
            .filter(|floor| standing < floor + HEADROOM)
            .fold(None, |best: Option<f32>, floor| {
                Some(best.map_or(floor, |had| had.max(floor)))
            })
    }

    #[cfg(feature = "tools")]
    pub fn add(&mut self, bore: Bore) {
        self.list.push(bore);
        self.unsaved = true;
    }

    /// Takes out whichever bore's middle is nearest, and says whether one went.
    #[cfg(feature = "tools")]
    pub fn remove_nearest(&mut self, to: Vec2, within: f32) -> bool {
        let Some((_, index)) = self
            .list
            .iter()
            .enumerate()
            .map(|(index, bore)| (((bore.from + bore.to) * 0.5).distance(to), index))
            .filter(|(away, _)| *away <= within)
            .min_by(|a, b| a.0.total_cmp(&b.0))
        else {
            return false;
        };
        self.list.remove(index);
        self.unsaved = true;
        true
    }

    #[cfg(feature = "tools")]
    pub fn mark_saved(&mut self) {
        self.unsaved = false;
    }
}

/// Where the bores are kept.
pub fn path() -> std::path::PathBuf {
    std::path::Path::new("assets/world/bores.json").to_path_buf()
}

/// Reads the bores, or an empty world if there are none to read.
///
/// Every failure is the same answer — a world with no tunnels in it — and every one
/// says why in the log. A world that refuses to load because a tunnel is unreadable
/// has turned a lost tunnel into a lost world.
pub fn load() -> Bores {
    let Ok(text) = std::fs::read_to_string(path()) else {
        return Bores::default();
    };
    match read(&text) {
        Ok(bores) => bores,
        Err(why) => {
            warn!("{}: {why}", path().display());
            Bores::default()
        }
    }
}

pub fn read(text: &str) -> Result<Bores, String> {
    let body: serde_json::Value =
        serde_json::from_str(text).map_err(|why| format!("not readable: {why}"))?;
    let list = body
        .get("bores")
        .and_then(serde_json::Value::as_array)
        .ok_or("no bores in it")?;

    let mut bores = Bores::default();
    for one in list {
        let number = |key: &str| one.get(key).and_then(serde_json::Value::as_f64);
        let pair = |key: &str| {
            let at = one.get(key)?.as_array()?;
            Some(Vec2::new(
                at.first()?.as_f64()? as f32,
                at.get(1)?.as_f64()? as f32,
            ))
        };
        let (Some(from), Some(to)) = (pair("from"), pair("to")) else {
            return Err("a bore with no mouths".into());
        };
        let bore = Bore {
            from,
            to,
            floor_from: number("floor_from").unwrap_or(0.0) as f32,
            floor_to: number("floor_to").unwrap_or(0.0) as f32,
        };
        // Numbers, not just number-shaped — the same rule the save file and the
        // painted layers keep. A tunnel between two infinities cuts the world in
        // half and nothing on screen would say why.
        if !bore.from.is_finite()
            || !bore.to.is_finite()
            || !bore.floor_from.is_finite()
            || !bore.floor_to.is_finite()
        {
            return Err("a bore that is not a place".into());
        }
        bores.list.push(bore);
    }
    Ok(bores)
}

#[cfg(feature = "tools")]
pub fn write(bores: &Bores) -> String {
    let mut out = String::from("{\n  \"bores\": [\n");
    for (index, bore) in bores.list.iter().enumerate() {
        out.push_str(&format!(
            "    {{ \"from\": [{:.2},{:.2}], \"to\": [{:.2},{:.2}], \
             \"floor_from\": {:.2}, \"floor_to\": {:.2} }}",
            bore.from.x, bore.from.y, bore.to.x, bore.to.y, bore.floor_from, bore.floor_to
        ));
        if index + 1 < bores.list.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str("  ]\n}\n");
    out
}

#[cfg(feature = "tools")]
pub fn save(bores: &mut Bores) -> std::io::Result<()> {
    let road = path();
    if let Some(folder) = road.parent() {
        std::fs::create_dir_all(folder)?;
    }
    std::fs::write(&road, write(bores))?;
    bores.mark_saved();
    Ok(())
}

/// The rock standing over one bore, as an entity in the world.
#[derive(Component)]
pub struct Rock;

/// Builds and stands the rock over every bore, and takes down what was there.
///
/// Runs whenever the bores change, which is what makes the tool immediate: lay a
/// tunnel and the rock over it is there the same moment the ground opens.
pub fn raise_the_rock(
    mut commands: Commands,
    terrain: Res<crate::world::terrain::TerrainSource>,
    mut meshes: ResMut<Assets<Mesh>>,
    material: Option<Res<crate::world::chunk::TerrainMaterial>>,
    standing: Query<Entity, With<Rock>>,
) {
    let Some(material) = material else {
        return;
    };
    for old in &standing {
        commands.entity(old).despawn();
    }
    let Ok(bores) = terrain.0.bores().read() else {
        return;
    };
    for bore in bores.all() {
        // Against the ground WITHOUT the tunnels — the hill as it stands, which is
        // what decides where the mouths are and how dark the middle gets.
        let rock = bore.tube(|at| terrain.0.unbored(at.x, at.y));
        if rock.is_empty() {
            continue;
        }
        commands.spawn((
            Rock,
            Mesh3d(meshes.add(crate::world::stream::as_coloured_mesh(&rock))),
            MeshMaterial3d(material.0.clone()),
            Transform::IDENTITY,
            Visibility::default(),
        ));
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    /// A ridge across the x axis: 80 m at the middle, gone by 100 m out.
    fn hill(at: Vec2) -> f32 {
        20.0 + 80.0 * (1.0 - (at.x.abs() / 100.0).min(1.0))
    }

    fn flat(_: Vec2) -> f32 {
        20.0
    }

    fn through_the_hill() -> Bore {
        Bore {
            from: Vec2::new(-140.0, 0.0),
            to: Vec2::new(140.0, 0.0),
            floor_from: 20.0,
            floor_to: 20.0,
        }
    }

    #[test]
    fn boring_a_tunnel_does_not_move_one_grain_of_earth() {
        // Two faults in a row, and this is the second one's test.
        //
        // The first build carved the whole corridor and left the rock above it as a
        // mesh, so a bore through a small rise was a black tent on open ground. The
        // fix carved only short cuttings at the mouths — and the extent of THAT was
        // decided by how thinly the hill held the tunnel, which on a gentle slope
        // is hundreds of metres, so it gouged a valley across the hillside.
        //
        // Nothing is carved at all now, and the mouths still come out flush, because
        // they sit at the ground's own height and the floor is the line between
        // them. Measured across the whole bore and well past both ends: the surface
        // a bore leaves behind is the surface it found.
        let bore = through_the_hill();
        let reach = bore.length();
        for step in -60..=160 {
            let t = step as f32 / 100.0;
            for side in [-1.0_f32, 0.0, 1.0] {
                let at = bore.from.lerp(bore.to, t) + Vec2::new(0.0, side * WIDE * 1.5);
                assert_eq!(
                    bore.cut_nothing(at),
                    0.0,
                    "the ground moved {:.1} m along and {:.1} m aside",
                    t * reach,
                    side * WIDE * 1.5
                );
            }
        }

        // And the mouths ARE flush: at each end the tunnel's floor is the ground,
        // so there is nothing to cut away to walk in.
        for (mouth, floor) in [(bore.from, bore.floor_from), (bore.to, bore.floor_to)] {
            assert!(
                (hill(mouth) - floor).abs() < 0.01,
                "a mouth sits {:.2} m off the ground it opens onto",
                hill(mouth) - floor
            );
        }
    }

    #[test]
    fn a_bore_needs_something_to_bore_through() {
        // Both screenshots that killed the first build: a tunnel laid over flat
        // ground, and one laid out over open water. There is no rock overhead in
        // either, so there is no tunnel to make — and the tool refuses rather than
        // hanging a mesh in the air.
        let over_flat = Bore {
            from: Vec2::new(-80.0, 0.0),
            to: Vec2::new(80.0, 0.0),
            floor_from: 20.0,
            floor_to: 20.0,
        };
        assert_eq!(
            over_flat.makes_sense(flat),
            Err("nothing to bore through - aim at a hill"),
            "a bore across a plain was accepted"
        );

        // The other screenshot: aimed from one side of the hill clean across it and
        // out into water. That aim is fine — it is how anybody aims — so the far
        // mouth is walked back in to the shore rather than the whole thing being
        // refused. What must never happen is a mouth left standing in the sea.
        let coast = |at: Vec2| if at.x > 140.0 { -8.0 } else { hill(at) };
        let aimed = Bore::aimed(Vec2::new(-200.0, 0.0), Vec2::new(400.0, 0.0), coast)
            .expect("an aim across a hill to the sea should still make a tunnel");
        assert!(
            aimed.floor_to > 0.0 && aimed.floor_from > 0.0,
            "a mouth was left under water: floors {:.1} and {:.1}",
            aimed.floor_from,
            aimed.floor_to
        );
        assert!(
            aimed.to.x < 145.0,
            "the far mouth stands at {:.0} m, out past the shoreline",
            aimed.to.x
        );
        assert_eq!(aimed.makes_sense(coast), Ok(()), "the trimmed bore was refused");

        // And an aim with no dry land under it at all is nothing anybody can walk.
        assert!(
            Bore::aimed(Vec2::new(300.0, 0.0), Vec2::new(600.0, 0.0), coast).is_none(),
            "an aim entirely out to sea made a tunnel"
        );

        // Aimed generously PAST the hill on both sides — the normal way to use the
        // tool — the lining stops where the hill does rather than running the full
        // length.
        let generous = Bore {
            from: Vec2::new(-260.0, 0.0),
            to: Vec2::new(260.0, 0.0),
            floor_from: 20.0,
            floor_to: 20.0,
        };
        let lining = generous.tube(hill);
        assert!(!lining.is_empty(), "nothing was built through the hill");
        let reach = lining
            .places
            .iter()
            .map(|place| place[0])
            .fold(f32::MIN, f32::max);
        assert!(
            reach < 190.0,
            "the lining runs out to {reach:.0} m, well past the hill's own 100"
        );

        assert_eq!(
            through_the_hill().makes_sense(hill),
            Ok(()),
            "a bore straight through a hill was refused"
        );
    }

    #[test]
    fn the_tube_is_a_tunnel_someone_can_walk_and_see_into() {
        let bore = through_the_hill();
        let tube = bore.tube(hill);
        assert!(!tube.is_empty(), "no lining was built");

        // Headroom under the crown, the whole length.
        let ring = RING_POINTS + 1 + 2;
        let rings = tube.places.len() / ring;
        for step in 0..rings {
            let crown = tube.places[step * ring + RING_POINTS / 2][1];
            let floor = tube.places[step * ring + ring - 1][1];
            assert!(
                crown - floor > HIGH * 0.85,
                "ring {step} has {:.1} m of headroom",
                crown - floor
            );
        }

        // Every face looks at the space a walker is in. A face wound the other way
        // is invisible from inside, which is how the pass came to be drawn inside
        // out for a whole session.
        let length = bore.length();
        let along_way = (bore.to - bore.from) / length;
        let mut wrong = 0;
        for face in tube.indices.chunks(3) {
            let corner = |i: usize| Vec3::from_array(tube.places[face[i] as usize]);
            let (a, b, c) = (corner(0), corner(1), corner(2));
            let out = (b - a).cross(c - a);
            if out.length_squared() < 1.0e-8 {
                continue;
            }
            let middle = (a + b + c) / 3.0;
            let t = (Vec2::new(middle.x, middle.z) - bore.from).dot(along_way) / length;
            let axis = bore.from.lerp(bore.to, t.clamp(0.0, 1.0));
            let inside = Vec3::new(axis.x, bore.floor_at(t) + HIGH * 0.4, axis.y);
            if out.dot(inside - middle) <= 0.0 {
                wrong += 1;
            }
        }
        assert_eq!(wrong, 0, "{wrong} faces of the lining face into the rock");
    }

    #[test]
    fn the_middle_of_the_tunnel_is_dark_and_the_mouths_are_not() {
        let tube = through_the_hill().tube(hill);
        let ring = RING_POINTS + 1 + 2;
        let rings = tube.places.len() / ring;
        let lit = |step: usize| {
            let colour = tube.colours[step * ring + RING_POINTS / 2];
            colour[0] + colour[1] + colour[2]
        };
        let mouth = lit(0).max(lit(rings - 1));
        let middle = lit(rings / 2);
        assert!(
            middle < mouth * 0.35,
            "the middle ({middle:.3}) is nearly as bright as the mouths ({mouth:.3})"
        );
    }

    #[test]
    fn the_two_level_rule_knows_who_is_underground() {
        let mut bores = Bores::default();
        bores.add(through_the_hill());
        let at = Vec2::ZERO;
        let ground = hill(at);

        // Down in the corridor: the floor claims them.
        let floor = bores
            .walk_floor(at, ground, 22.0)
            .expect("the tunnel should claim somebody standing in it");
        assert!((floor - 20.0).abs() < 0.5, "the floor came out at {floor:.1}");

        // Up on the hill directly above: it must not, or they drop through rock.
        assert!(
            bores.walk_floor(at, ground, ground).is_none(),
            "somebody on the hilltop was claimed by the tunnel underneath"
        );

        // Outside the tube's walls, and past the mouths: nothing.
        assert!(
            bores.walk_floor(Vec2::new(0.0, WIDE + 3.0), ground, 22.0).is_none(),
            "the floor reaches outside the tube's own walls"
        );
        let out = Vec2::new(200.0, 0.0);
        assert!(
            bores.walk_floor(out, hill(out), 22.0).is_none(),
            "the floor runs on past the mouth"
        );
    }

    #[test]
    fn bores_survive_being_written_and_read() {
        let mut bores = Bores::default();
        bores.add(through_the_hill());
        bores.add(Bore {
            from: Vec2::new(10.0, -20.0),
            to: Vec2::new(-30.0, 40.0),
            floor_from: 5.5,
            floor_to: 9.25,
        });

        let back = read(&write(&bores)).expect("should read back");
        assert_eq!(back.len(), bores.len());
        for (was, is) in bores.all().iter().zip(back.all()) {
            assert!(was.from.abs_diff_eq(is.from, 0.01), "{was:?} came back as {is:?}");
            assert!(was.to.abs_diff_eq(is.to, 0.01));
            assert!((was.floor_from - is.floor_from).abs() < 0.01);
        }
    }

    #[test]
    fn a_bore_that_is_not_a_place_is_refused() {
        let why = read(r#"{"bores":[{"from":[0,0],"to":[1,null],"floor_from":0}]}"#).unwrap_err();
        assert!(why.contains("mouths"), "unhelpful reason: {why}");
        // JSON cannot hold an infinity, so the parser turns one back before the
        // finiteness check ever sees it. Both are refusals; what matters is that
        // neither is read as a place.
        let why = read(r#"{"bores":[{"from":[0,0],"to":[1,2],"floor_from":1e400}]}"#).unwrap_err();
        assert!(why.contains("readable"), "unhelpful reason: {why}");
        assert!(read("{}").is_err(), "a file with no bores in it was accepted");
    }
}
