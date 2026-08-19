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
//! # What a bore actually does
//!
//! Two things, and they have to agree exactly:
//!
//! * it **cuts the ground down** to a floor running level between its two mouths,
//!   over a width wide enough to walk. A bore only ever cuts DOWN — run one across
//!   open ground and nothing happens, because there was no hill in the way.
//! * it **leaves the rock** that was above that cut standing, as a mesh, from the
//!   tunnel's arched ceiling up to where the ground used to be.
//!
//! Both are drawn from the same numbers, so the hole and the rock over it cannot
//! drift apart.
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

/// How far either side of the middle the ground is cut FLAT.
///
/// Well past the arch, so the tunnel's whole lining is the rock mesh and the
/// terrain's own ramp happens outside it. A heightfield samples every two metres
/// and cannot hold a vertical wall — left to make the walls itself it gives a stair
/// with two-metre treads, which from inside is a jagged silhouette climbing away on
/// both sides.
pub const SPAN: f32 = 12.0;

/// How far past the flat cut the ground takes to climb back to what it was.
const SHOULDER: f32 = 3.0;

/// How far the rock's underside is held clear of the ground it lies against.
///
/// Two surfaces at the same height fight for the depth buffer, and the pass showed
/// that as a black sawtooth ribbon down its whole length.
const CLEAR: f32 = 0.25;

/// How far past the mouths the rock is built, in metres.
///
/// The rock thins to nothing where the ground it is replacing has dropped to the
/// tunnel's crown, so this only has to be far enough out that the thinning finishes
/// inside the mesh rather than at its edge.
const OVERRUN: f32 = 40.0;

/// Steps along a bore and across it when its rock is built.
///
/// About two metres a step, which is the terrain's own vertex spacing — the
/// resolution of the thing it has to meet.
const ACROSS_STEPS: usize = 18;
const ALONG_STEP: f32 = 2.5;

impl Bore {
    /// Where a point sits in this bore's frame: how far along, 0 at one mouth and
    /// 1 at the other, and how far to the side in metres.
    fn local(&self, at: Vec2) -> (f32, f32) {
        let run = self.to - self.from;
        let length = run.length().max(1.0e-3);
        let along = run / length;
        let away = at - self.from;
        let forward = away.dot(along);
        (forward / length, away.dot(Vec2::new(-along.y, along.x)))
    }

    /// The floor's height at a point along the bore.
    fn floor_at(&self, t: f32) -> f32 {
        self.floor_from + (self.floor_to - self.floor_from) * t.clamp(0.0, 1.0)
    }

    /// How much of the passage a point is: 1 on the flat floor, 0 outside it.
    ///
    /// Nothing beyond the mouths. A bore is a hole through something, not a
    /// trench running off across the country at either end.
    fn share(&self, at: Vec2) -> f32 {
        let (t, across) = self.local(at);
        if !(0.0..=1.0).contains(&t) {
            return 0.0;
        }
        crate::util::smoothstep(SPAN + SHOULDER, SPAN, across.abs())
    }

    /// How far this bore cuts the ground down at a point, in metres.
    ///
    /// Never up. Run a bore over open ground and this is nought the whole way —
    /// there was no hill to get through.
    pub fn cut(&self, at: Vec2, ground: f32) -> f32 {
        let share = self.share(at);
        if share <= 0.0 {
            return 0.0;
        }
        let (t, _) = self.local(at);
        (ground - self.floor_at(t)).max(0.0) * share
    }

    /// Whether a point is inside the tunnel with rock overhead, 0 to 1.
    ///
    /// What tells the world to leave it alone: nothing grows under a mountain, and
    /// the floor is painted as the stone it is. Nought where the ground was never
    /// higher than the crown — there the bore is open sky and keeps its grass.
    pub fn under_rock(&self, at: Vec2, ground: f32) -> f32 {
        let (t, _) = self.local(at);
        let overhead = ground - self.floor_at(t) - HIGH;
        self.share(at) * crate::util::smoothstep(0.0, HIGH, overhead)
    }

    /// The underside of the rock at a point across the bore, above the floor.
    ///
    /// A half-ellipse over the arch and the bare floor outside it, so the rock
    /// beyond the arch's foot rests ON the ground rather than hanging over it.
    fn lining(&self, t: f32, across: f32) -> f32 {
        let reach = (across.abs() / WIDE).min(1.0);
        self.floor_at(t) + HIGH * (1.0 - reach * reach).max(0.0).sqrt()
    }

    /// The rock this bore leaves standing over itself.
    ///
    /// `ground` gives the height the world would have WITHOUT this bore, which is
    /// what the rock is replacing. Two sheets — the ground's own surface above and
    /// the tunnel's lining below — pinched shut wherever the ground was no higher
    /// than the crown, which is what makes the mouths mouths and means there is no
    /// end cap anywhere to get wrong.
    pub fn rock(&self, ground: impl Fn(Vec2) -> f32) -> Geometry {
        let run = self.to - self.from;
        let length = run.length();
        if length < 1.0 {
            return Geometry::default();
        }
        let along = run / length;
        let side = Vec2::new(-along.y, along.x);
        let wide = SPAN + SHOULDER;
        let steps = (((length + OVERRUN * 2.0) / ALONG_STEP).ceil() as usize).max(2);

        let mut top = Vec::with_capacity((steps + 1) * (ACROSS_STEPS + 1));
        let mut under = Vec::with_capacity(top.capacity());
        for step in 0..=steps {
            let forward = -OVERRUN + step as f32 * (length + OVERRUN * 2.0) / steps as f32;
            let t = forward / length;
            for slot in 0..=ACROSS_STEPS {
                let across = -wide + slot as f32 * (2.0 * wide / ACROSS_STEPS as f32);
                let at = self.from + along * forward + side * across;
                let was = ground(at);
                // The ground as the world now draws it, which is what the rock
                // must not sink into.
                let now = was - self.cut(at, was);
                let below = self.lining(t, across).max(now + CLEAR).min(was);
                top.push(Vec3::new(at.x, was, at.y));
                under.push(Vec3::new(at.x, below, at.y));
            }
        }

        let mut rock = Geometry::default();
        sheet(&mut rock, &top, steps, true);
        sheet(&mut rock, &under, steps, false);
        settle_the_normals(&mut rock);
        rock
    }
}

/// Adds one grid of points as a surface.
///
/// `up` decides the winding, and the shading is derived from the winding
/// afterwards — see [`settle_the_normals`]. Written down separately they can
/// disagree, and a face lit from the wrong side is invisible: the pass was drawn
/// inside out for a whole session because of exactly that.
fn sheet(mesh: &mut Geometry, grid: &[Vec3], steps: usize, up: bool) {
    let wide = ACROSS_STEPS + 1;
    let base = mesh.places.len() as u32;

    for (index, point) in grid.iter().enumerate() {
        mesh.places.push(point.to_array());
        mesh.normals.push([0.0, 0.0, 0.0]);
        mesh.uvs.push([(index % wide) as f32 / wide as f32, (index / wide) as f32]);
        mesh.colours.push(stone(*point, up));
    }
    for row in 0..steps {
        for col in 0..ACROSS_STEPS {
            let a = base + (row * wide + col) as u32;
            let (b, c, d) = (a + 1, a + wide as u32, a + wide as u32 + 1);
            if up {
                mesh.indices.extend_from_slice(&[a, b, c, b, d, c]);
            } else {
                mesh.indices.extend_from_slice(&[a, c, b, b, c, d]);
            }
        }
    }
}

/// Gives every vertex the sum of the faces meeting at it.
///
/// A normal built OUT OF the winding cannot contradict it, which is the whole
/// reason it is done this way round.
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

/// What the rock is painted, in linear RGBA. The lining darker than the outside:
/// no sunlight reaches the inside of a tunnel.
fn stone(at: Vec3, up: bool) -> [f32; 4] {
    let grain = terrain_core::forest::field(Vec2::new(at.x, at.z) / 7.0, 77);
    let shade = if up { 0.86 + grain * 0.28 } else { 0.26 + grain * 0.12 };
    [0.115 * shade, 0.108 * shade, 0.101 * shade, 1.0]
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

    /// How far every bore together cuts the ground down here.
    ///
    /// The DEEPEST of them rather than the sum: two tunnels crossing make one
    /// junction, not a hole twice as deep.
    pub fn cut(&self, at: Vec2, ground: f32) -> f32 {
        self.list
            .iter()
            .map(|bore| bore.cut(at, ground))
            .fold(0.0_f32, f32::max)
    }

    /// Whether this point is inside any tunnel with rock overhead.
    pub fn under_rock(&self, at: Vec2, ground: f32) -> f32 {
        self.list
            .iter()
            .map(|bore| bore.under_rock(at, ground))
            .fold(0.0_f32, f32::max)
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
        // Against the ground WITHOUT the tunnels: the rock IS the ground the bore
        // took away, so it has to be measured from the world that still had it.
        let rock = bore.rock(|at| terrain.0.unbored(at.x, at.y));
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

    fn hill(at: Vec2) -> f32 {
        // A ridge across the x axis: 80 m at the middle, gone by 100 m out.
        20.0 + 80.0 * (1.0 - (at.x.abs() / 100.0).min(1.0))
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
    fn a_bore_cuts_a_hill_down_and_leaves_open_ground_alone() {
        // The whole of what a bore is: a hole through something. Run one over flat
        // country and it should do nothing at all, because there was nothing in
        // the way — a tunnel is not a trench.
        let bore = through_the_hill();

        let middle = Vec2::ZERO;
        let ground = hill(middle);
        assert!(
            (ground - bore.cut(middle, ground) - 20.0).abs() < 0.5,
            "the floor came out at {:.1}, not level with the mouths",
            ground - bore.cut(middle, ground)
        );

        // Beside the bore, the hill is untouched.
        let beside = Vec2::new(0.0, SPAN + SHOULDER + 5.0);
        assert_eq!(bore.cut(beside, hill(beside)), 0.0, "the hill was cut beside the bore");

        // Past the mouths, nothing — including where the ground is lower than the
        // floor, which must not be filled IN either.
        for out in [Vec2::new(-200.0, 0.0), Vec2::new(200.0, 0.0)] {
            assert_eq!(bore.cut(out, hill(out)), 0.0, "the bore ran on past its own mouth");
        }
    }

    #[test]
    fn the_rock_over_a_bore_closes_itself_and_has_a_tunnel_in_it() {
        let bore = through_the_hill();
        let rock = bore.rock(hill);
        assert!(!rock.is_empty(), "no rock was left standing");

        // Thick over the middle of the hill, and shut at both mouths — which is
        // what makes a mouth a mouth rather than an edge somebody chose.
        let wide = ACROSS_STEPS + 1;
        let half = rock.places.len() / 2;
        let apart = |index: usize| rock.places[index][1] - rock.places[half + index][1];

        let rows = half / wide;
        let middle = (rows / 2) * wide + ACROSS_STEPS / 2;
        assert!(apart(middle) > 40.0, "the rock over the hill is {:.0} m", apart(middle));
        for row in [0usize, rows - 1] {
            for slot in 0..=ACROSS_STEPS {
                let gap = apart(row * wide + slot);
                assert!(gap < 0.5, "the rock stands {gap:.1} m open at a mouth");
            }
        }
    }

    #[test]
    fn there_is_room_to_walk_and_rock_over_your_head() {
        let bore = through_the_hill();
        let ground = hill(Vec2::ZERO);
        assert!(
            bore.lining(0.5, 0.0) - 20.0 >= 6.0,
            "only {:.1} m of headroom",
            bore.lining(0.5, 0.0) - 20.0
        );
        assert!(
            bore.under_rock(Vec2::ZERO, ground) > 0.9,
            "the middle of the tunnel does not read as being under rock"
        );
        // And at the mouth, where the hill has run out, it is open sky.
        let mouth = Vec2::new(-135.0, 0.0);
        assert!(
            bore.under_rock(mouth, hill(mouth)) < 0.1,
            "the mouth reads as being under rock"
        );
    }

    #[test]
    fn the_lining_faces_the_tunnel_and_the_ground_faces_the_sky() {
        // A face lit from the wrong side is an invisible face, and the pass was
        // drawn inside out for a whole session before anybody could say why.
        let bore = through_the_hill();
        let rock = bore.rock(hill);
        let half = rock.places.len() / 2;

        let mut sky_wrong = 0;
        let mut ceiling_wrong = 0;
        for face in rock.indices.chunks(3) {
            let corner = |i: usize| Vec3::from_array(rock.places[face[i] as usize]);
            let (a, b, c) = (corner(0), corner(1), corner(2));
            let wound = (b - a).cross(c - a);
            if wound.length_squared() < 1.0e-8 {
                continue;
            }
            let out = wound.normalize();
            let middle = (a + b + c) / 3.0;
            if (face[0] as usize) < half {
                sky_wrong += (out.y <= 0.0) as i32;
            } else {
                // Only the ceiling: the lining is one folded sheet and its walls
                // legitimately point sideways.
                let across = (middle - Vec3::new(bore.from.x, 0.0, bore.from.y)).z;
                if across.abs() < WIDE * 0.7 && middle.y > 20.0 + HIGH * 0.4 {
                    ceiling_wrong += (out.y >= 0.0) as i32;
                }
            }
        }
        assert_eq!(sky_wrong, 0, "{sky_wrong} faces of the ground face downward");
        assert_eq!(ceiling_wrong, 0, "{ceiling_wrong} faces of the ceiling face up into the rock");
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
        // The same rule the save file and the painted layers keep: a tunnel
        // between two infinities cuts the world in half and nothing on screen
        // would say why.
        let why = read(r#"{"bores":[{"from":[0,0],"to":[1,null],"floor_from":0}]}"#).unwrap_err();
        assert!(why.contains("mouths"), "unhelpful reason: {why}");
        // JSON cannot hold an infinity at all, so the parser turns one back
        // before the check below ever sees it. Both refusals are refusals; what
        // matters is that neither is silently read as a place.
        let why = read(r#"{"bores":[{"from":[0,0],"to":[1,2],"floor_from":1e400}]}"#).unwrap_err();
        assert!(why.contains("readable"), "unhelpful reason: {why}");
        assert!(read("{}").is_err(), "a file with no bores in it was accepted");
    }
}
