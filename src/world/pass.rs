//! The mountain pass: a mountain across the road east, and a tunnel through it.
//!
//! # The mountain is TERRAIN, and the tunnel is a TUBE inside it
//!
//! The first build carved a slot down through the heightfield and lidded it with a
//! mesh. Geometrically sound, and wrong to look at from every angle that matters:
//! the slot's own carve killed the trees and painted a stone stripe over the whole
//! corridor, the lid was a grey band where mountainside should be, and from the
//! air the "tunnel" read as a road. A mountain with a stripe shaved over its
//! shoulder is not a mountain with a tunnel in it — it is a mountain with a scar.
//!
//! So the ground keeps its skin. The heightfield over the tunnel is the mountain,
//! whole: its trees grow, its snow lies, its creases shade, and nothing anywhere
//! says a tunnel exists — except at the two MOUTHS, where the ground is carved
//! down to walking level in a short cutting, and the dark of the tube shows.
//!
//! Inside, the tunnel is its own mesh: a floor and an arched ceiling running from
//! mouth to mouth, wound to face inward, lit darker toward its middle. The
//! heightfield above it is the mountain's top, which the warden must NOT stand on
//! while underground — so the walking rule becomes two-level exactly there:
//! [`walk_floor`] hands back the tunnel's floor whenever someone is inside the
//! corridor and below the rock, and the ordinary snap owns everywhere else.
//!
//! # The mouths still place themselves
//!
//! The cuttings are carved wherever the corridor crosses ground the mountain holds
//! only thinly — so the mouths sit exactly where mountain becomes plain, and
//! moving or reshaping the mountain moves its own doorways.

use bevy::prelude::*;

use terrain_core::Geometry;

/// Where the middle of the bore stands, in metres.
///
/// Placed so the mountain's WESTERN foot lands on the desert's own eastern edge,
/// which is at about (180, -880) — the map printed by `dump_the_world` is what
/// that was read off. So the journey east is desert, then the west foot, then the
/// mountain, then the green country, then the snow, and neither flank has the
/// wrong country on it.
pub const AT: Vec2 = Vec2::new(456.0, -997.0);

/// Which way the tunnel runs, in radians about Y. Nought is due east.
///
/// # It leans, because the country does
///
/// This was due east, and the wall it makes therefore ran due north-south — across
/// a desert boundary that runs on a diagonal, because `region`'s own axis is
/// tilted and the world is half as deep as it is wide. So the wall crossed the
/// boundary at an angle and one end of it had desert on the side that was supposed
/// to be green.
///
/// Set from the region's own lean rather than picked: the boundary runs along
/// `(TILT, 1)` in map coordinates, which is `(0.39, 0.92)` on the ground once each
/// axis is scaled by its own extent, and the tunnel runs across that.
pub const HEADING: f32 = -0.40;

/// How high the mountain stands above the ground it is raised on, in metres.
///
/// Well over the treeline, and by a margin: the trees give out at 150 m, so a
/// crest brushing 165 left all but the last few metres forested and the whole
/// thing read as a very long hill. At 235 the upper flanks strip to alpine rock
/// and the crest carries snow, which is what "mountain" looks like from below.
const RIDGE_HIGH: f32 = 235.0;

/// How far the mountain reaches, in metres: the length of the WALL, measured
/// across the tunnel, and its THICKNESS, measured along the tunnel.
///
/// The wall is long and the bore is short, which is the whole shape of a pass: a
/// barrier you cannot walk round and a way through you can walk in a couple of
/// minutes.
///
/// **Named for the wall rather than for the tunnel, and that is worth the extra
/// word.** They were `ALONG` and `ACROSS`, which read naturally and meant the
/// opposite of what `local` returns — so the mountain was built long in the
/// direction you travel and thin in the direction it was supposed to block. The
/// tests said so at once: the wall gave out 143 m to the side, and the plug was
/// still 156 m thick at its own edge.
const WALL_LONG: f32 = 900.0;
const WALL_THICK: f32 = 520.0;

/// How much of the wall's LENGTH is its shoulders rather than its body.
///
/// Only the length. Along its length a ridge really is flat-crested — that is what
/// makes it a barrier rather than a hill — and it eases down into the plain at
/// each end. Across its thickness it is not: a shoulder in both directions gives a
/// flat-topped table, which is what this first came out as and what the note above
/// this constant was already warning about. Across, the mountain simply peaks, so
/// there is a crest line running the length of the wall and the bore goes through
/// the tallest part of it.
const SHOULDER: f32 = 0.72;

/// Half the width of the bore, and how far past it the rock takes to close back
/// up, both in metres.
///
/// Eleven metres across the floor: wide enough for whatever ends up walking
/// through and for a branch to open off it later, narrow enough to read as a hole
/// bored through rock rather than a road cutting. The walls come back over three,
/// which against a hundred and sixty metres of mountain is as near vertical as a
/// heightfield can say.
const BORE_WIDE: f32 = 5.5;
const BORE_WALL: f32 = 3.0;

/// How far either side of the middle the ground is carved FLAT, in metres.
///
/// # The walls have to be rock, not terrain
///
/// The floor used to be carved just wide enough for the arch, so the terrain
/// itself climbed the walls — and a heightfield cannot make a vertical wall. It
/// samples every two metres, so a hundred and sixty metres of rise over three
/// metres of ground comes out as a stair with two-metre treads, and from inside
/// the bore that is a jagged silhouette climbing away on both sides.
///
/// So the flat floor now runs well past the arch, and the rock above lies on that
/// floor rather than on a slope: the tunnel's whole lining is mesh, cut to a smooth
/// curve, and the terrain's own ramp happens out beyond it where the rock hides it.
const BORE_SPAN: f32 = 12.0;

/// Height from the floor to the crown, in metres.
///
/// Close to the bore's own half-width, so the section is round rather than a
/// letterbox — a tunnel is a hole bored through rock and it should read as one.
/// Seven metres still gives a person four times their own height overhead, which
/// is the other thing this has to be: a tunnel you stoop in reads as a drain.
const BORE_HIGH: f32 = 7.0;

/// How much of the mountain this point stands under, 0 to 1.
///
/// # A wall of rock, not a smooth earthwork
///
/// The analytic profile alone — two eased falloffs — is a berm at any size:
/// perfectly smooth flanks and a crest like a ruler. Three things break it up,
/// and each is scaled so the tests about the PASS still hold:
///
/// * the **crest is serrated**: the whole profile scales with a slow noise along
///   the wall, so the skyline is peaks and saddles rather than a line. It never
///   drops far enough to be walked over — the saddles are still most of the wall.
/// * the **flanks are creased**: two octaves of `1 - |noise|` gullies, cut into
///   the slope. Strongest mid-flank and faded at the crest and the foot, so the
///   silhouette stays a wall and the plain stays a plain.
/// * nothing is added ON TOP — creases only ever cut DOWN — so the bore's roof
///   arithmetic and every walk-through test read the same mountain this draws.
pub fn ridge(at: Vec2) -> f32 {
    let (along, across) = local(at);
    let reach = |d: f32, full: f32, flat: f32| {
        crate::util::smoothstep(full, full * flat, d.abs())
    };
    // Thin along the tunnel and long across it: rock to bore through, and a wall
    // reaching away on both sides of the mouth. Peaked in the first and
    // flat-crested in the second — see `SHOULDER`.
    let body = reach(along, WALL_THICK, 0.0) * reach(across, WALL_LONG, SHOULDER);
    if body <= 0.0 {
        return 0.0;
    }

    // The serration, in the wall's own frame so it survives being turned.
    let crest = 1.0 - SERRATION
        + SERRATION * 2.0 * terrain_core::forest::field(Vec2::new(across, along) / TOOTH, 78);

    // The creases, in the wall's own frame and STRETCHED down the fall line —
    // a gully is water's work and water runs downhill, so the folds are long in
    // the direction of the slope and narrow across it. Sampled isotropically
    // they came out as round pockets, and a hillside of round pockets reads as
    // hammered metal rather than as spurs.
    let fold = |narrow: f32, salt: u32| {
        let stretched = Vec2::new(across / narrow, along / (narrow * 3.2));
        1.0 - (2.0 * terrain_core::forest::field(stretched, salt) - 1.0).abs()
    };
    // Plus one broad UNstretched octave, or the combing is too even: every spur
    // the same width the whole length of a mountainside is a texture, not ground.
    let broad = 1.0 - (2.0 * terrain_core::forest::field(at / 150.0, 81) - 1.0).abs();
    let crease = 0.45 * fold(64.0, 79) + 0.3 * fold(27.0, 80) + 0.25 * broad;
    // Mid-flank only: at the crest a gully would notch the skyline below the
    // saddles, and at the foot it would trench the plain.
    let flank = (body * (1.0 - body) * 4.0).clamp(0.0, 1.0);
    let cut = RELIEF * flank * (1.0 - crease.powf(1.4));

    RIDGE_HIGH * body * crest * (1.0 - cut)
}

/// How deep the serration and the gullies go, as shares of the local height.
///
/// The serration swings the crest a fifth either way; the gullies take up to
/// two fifths out of the mid-flank. Between them the wall's LOWEST crossing
/// stays above half its nominal height, which the walk-over test measures.
const SERRATION: f32 = 0.2;
const RELIEF: f32 = 0.42;

/// Metres between teeth along the crest.
const TOOTH: f32 = 110.0;

/// How much of the passage this point is, 1 on the floor and 0 in the rock.
///
/// Only across the bore. Along it there is no falloff at all and there must not
/// be: a bore that faded out along its own length would be a mountain with a dip
/// in it, and the tunnel would be closed at both ends by the very rock it is
/// supposed to be getting through.
pub fn bore(at: Vec2) -> f32 {
    let (_, across) = local(at);
    crate::util::smoothstep(BORE_SPAN + BORE_WALL, BORE_SPAN, across.abs())
}

/// What the pass adds to the ground here, in metres.
///
/// The mountain, whole — except in the CUTTINGS at the tunnel's two mouths,
/// where the ground is carved to walking level so the plain can hand over to the
/// tube. Everywhere the mountain stands tall, the skin is untouched: the tunnel
/// runs under real terrain, not through a slot in it.
pub fn lift(at: Vec2) -> f32 {
    let mountain = ridge(at);
    mountain * (1.0 - bore(at) * open_to_the_sky(mountain))
}

/// How much of a cutting the ground here is, by how thinly the mountain holds it.
///
/// 1 where the mountain is lower than a door, 0 once it stands more than a few
/// storeys — which is what puts the mouths exactly where mountain becomes plain,
/// whatever shape the mountain is.
fn open_to_the_sky(mountain: f32) -> f32 {
    crate::util::smoothstep(MOUTH_TALL, BORE_HIGH * 0.5, mountain)
}

/// The most rock a cutting will slice through before the tube takes over.
///
/// A couple of storeys above the crown: enough that each mouth is a short walled
/// cutting a walker reads as an entrance, not so much that the carve becomes the
/// old scar down the whole corridor.
const MOUTH_TALL: f32 = BORE_HIGH * 2.4;

/// Whether this is the floor of the bore, 0 to 1.
///
/// What tells the rest of the world to leave it alone: nothing grows on it and it
/// is painted as the stone it is. A tunnel with a meadow in it is not a tunnel.
pub fn underground(at: Vec2) -> f32 {
    // The CUTTINGS, now — the only ground the pass still carves. The corridor
    // under the mountain is not carved at all any more, so the skin above the
    // tunnel keeps its trees and its snow like any other mountainside, and only
    // the walled approaches at the mouths are stripped and painted stone.
    let mountain = ridge(at);
    bore(at)
        * open_to_the_sky(mountain)
        * crate::util::smoothstep(BORE_HIGH * 0.2, BORE_HIGH * 0.9, mountain)
}

/// The tunnel floor under a point, if that point is inside the corridor.
///
/// `natural` is the ground WITHOUT the pass — [`Terrain::without_the_pass`] —
/// which is what the floor is: the land the mountain was raised over, continuous
/// with the plain at both mouths because out there the pass adds nothing.
///
/// This is one half of the two-level walking rule; the caller decides whether the
/// walker is LOW enough for it to apply, because only the caller knows where the
/// walker is standing now.
pub fn floor(at: Vec2, natural: f32) -> Option<f32> {
    let (along, across) = local(at);
    (across.abs() < BORE_WIDE && along.abs() < WALL_THICK * TUBE_REACH && ridge(at) > BORE_HIGH)
        .then_some(natural)
}

/// How far above a tunnel's floor still counts as being IN the tunnel.
///
/// A little over the crown, so a walker who has stepped in through a cutting is
/// claimed by the floor before the rock closes over them, and one on the
/// mountainside hundreds of metres up never is.
pub const HEADROOM: f32 = BORE_HIGH * 1.6;

/// How far past the mountain's thickness the tube runs, as a share of it.
///
/// Into the cuttings at both ends, so the tube's open ends stand inside carved
/// ground and the seam between mesh wall and terrain wall is a join between two
/// stone surfaces rather than a hole.
const TUBE_REACH: f32 = 1.08;

/// How wide the floor a warden can actually walk on is, in metres either side.
///
/// The arch, not the carve: the ground is flat out to [`BORE_SPAN`] but the rock
/// rests on it beyond the arch's foot, so the space is the tube's own width.
pub fn walkable() -> f32 {
    BORE_WIDE
}

/// Where a point sits in the pass's own frame: along the tunnel, and across it.
fn local(at: Vec2) -> (f32, f32) {
    let away = at - AT;
    let (sin, cos) = HEADING.sin_cos();
    // Along is the tunnel's own direction; across is the ridge's.
    (away.x * cos + away.y * sin, -away.x * sin + away.y * cos)
}

/// How the tube is built: points around each ring of the arch, and metres
/// between rings along the tunnel.
///
/// The rings are fine because the ceiling is a curve somebody walks under and
/// looks along; the steps are coarse because the tunnel is straight.
const RING_POINTS: usize = 14;
const RING_STEP: f32 = 4.0;

/// The tunnel itself: a floor and an arched ceiling from mouth to mouth.
///
/// `natural` is the ground WITHOUT the pass, which is the floor's own height —
/// see [`floor`]. Wound to face INWARD, because the only place this mesh is ever
/// seen from is inside it: from outside, its far wall shows through each mouth as
/// the dark of the tunnel, and its near wall is culled.
///
/// # It carries its own light, baked
///
/// No sun reaches the middle of a tunnel and no lamp exists yet, so the vertex
/// colours do the work: stone at the mouths, falling to near-black through the
/// middle. The gradient is the thing a walker actually reads a tunnel by.
pub fn tube(natural: impl Fn(Vec2) -> f32) -> Geometry {
    let (sin, cos) = HEADING.sin_cos();
    let along_way = Vec2::new(cos, sin);
    let across_way = Vec2::new(-sin, cos);
    let reach = WALL_THICK * TUBE_REACH;
    let steps = ((reach * 2.0 / RING_STEP).ceil() as usize).max(2);

    let mut mesh = Geometry::default();
    for step in 0..=steps {
        let along = -reach + step as f32 * (reach * 2.0) / steps as f32;
        let middle = AT + along_way * along;
        let ground = natural(middle);
        // How deep in the mountain this ring is, 0 at either mouth to 1 in the
        // middle — the baked darkness.
        let deep = 1.0 - (along.abs() / reach).clamp(0.0, 1.0);
        let dark = crate::util::smoothstep(0.0, 0.55, deep);

        for point in 0..=RING_POINTS {
            // The ring runs floor-edge to floor-edge over the arch: t in -1..1.
            let t = point as f32 / RING_POINTS as f32 * 2.0 - 1.0;
            let across = t * BORE_WIDE;
            let up = BORE_HIGH * (1.0 - t * t).max(0.0).sqrt();
            let at = middle + across_way * across;
            let grain = terrain_core::forest::field(Vec2::new(along, across * 3.0) / 9.0, 82);
            let shade = (0.72 + grain * 0.28) * (1.0 - dark * 0.88);
            mesh.places.push([at.x, ground + up, at.y]);
            mesh.normals.push([0.0, 0.0, 0.0]);
            mesh.uvs.push([point as f32 / RING_POINTS as f32, step as f32]);
            mesh.colours
                .push([0.125 * shade, 0.118 * shade, 0.11 * shade, 1.0]);
        }
        // And the floor pair under the arch's feet, so the tube owns the ground
        // a walker actually stands on — the heightfield inside is the mountain's
        // TOP, which is no use to anybody down here.
        for side in [-1.0_f32, 1.0] {
            let at = middle + across_way * side * BORE_WIDE;
            let grain = terrain_core::forest::field(Vec2::new(along, side * 40.0) / 9.0, 83);
            let shade = (0.6 + grain * 0.2) * (1.0 - dark * 0.85);
            mesh.places.push([at.x, ground + FLOOR_LIFT, at.y]);
            mesh.normals.push([0.0, 0.0, 0.0]);
            mesh.uvs.push([0.5 + side * 0.5, step as f32]);
            mesh.colours
                .push([0.14 * shade, 0.132 * shade, 0.122 * shade, 1.0]);
        }
    }

    // Stitch ring to ring: the arch, then the floor strip between the two floor
    // points. Wound so every face looks at the tunnel's own axis.
    let ring = RING_POINTS + 1 + 2;
    for step in 0..steps {
        let a = (step * ring) as u32;
        let b = ((step + 1) * ring) as u32;
        for point in 0..RING_POINTS {
            let (p0, p1) = (a + point as u32, a + point as u32 + 1);
            let (q0, q1) = (b + point as u32, b + point as u32 + 1);
            // Inward: the arch is seen from BELOW, so it is wound the opposite
            // way to a ground surface. Worked out from the vectors rather than
            // guessed — a ring step is +across and a tunnel step is +along, and
            // across × along points UP, which is into the rock.
            mesh.indices.extend_from_slice(&[p0, q0, p1, p1, q0, q1]);
        }
        let (f0, f1) = (a + ring as u32 - 2, a + ring as u32 - 1);
        let (g0, g1) = (b + ring as u32 - 2, b + ring as u32 - 1);
        // And the floor is seen from ABOVE, like any ground.
        mesh.indices.extend_from_slice(&[f0, f1, g0, f1, g1, g0]);
    }

    settle_the_normals(&mut mesh);
    mesh
}

/// How far the tube's floor stands above the ground it copies.
///
/// The mouths are the reason: there the terrain itself is carved to the same
/// height, and two surfaces at one height fight for the depth buffer — the same
/// sawtooth the old plug taught.
const FLOOR_LIFT: f32 = 0.12;

/// Gives every vertex the sum of the faces meeting at it.
///
/// A normal built OUT OF the winding cannot contradict it — the old plug was
/// drawn inside out for a whole session because its normals were written down
/// separately from its faces.
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

/// The tunnel's lining, as an entity in the world.
#[derive(Component)]
pub struct Roof;

/// Builds and stands the tunnel's lining.
///
/// Once, with the world: it is a few hundred metres of mesh in one fixed place,
/// and it goes away with the world so the workbench does not keep a tunnel
/// behind it.
pub fn raise_the_roof(
    mut commands: Commands,
    terrain: Res<crate::world::terrain::TerrainSource>,
    mut meshes: ResMut<Assets<Mesh>>,
    material: Option<Res<crate::world::chunk::TerrainMaterial>>,
) {
    let Some(material) = material else {
        return;
    };
    let lining = tube(|at| terrain.0.without_the_pass(at.x, at.y));
    if lining.is_empty() {
        return;
    }
    info!(
        "the pass tunnel: {} verts of lining under the mountain at {:.0}, {:.0}",
        lining.places.len(),
        AT.x,
        AT.y
    );
    commands.spawn((
        Roof,
        Mesh3d(meshes.add(crate::world::stream::as_coloured_mesh(&lining))),
        MeshMaterial3d(material.0.clone()),
        Transform::IDENTITY,
        Visibility::default(),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A flat world at ten metres, so the pass can be measured on its own.
    fn plain(_: Vec2) -> f32 {
        10.0
    }

    /// What somebody walking the corridor actually stands on: the tunnel floor
    /// where the two-level rule applies, the drawn ground everywhere else.
    fn walking(at: Vec2) -> f32 {
        floor(at, plain(at)).unwrap_or_else(|| plain(at) + lift(at))
    }

    #[test]
    fn the_skin_over_the_tunnel_is_the_mountain_itself() {
        // The whole point of the rework: from above and from every flank, the
        // ground over the tunnel is ordinary mountainside — trees, snow, creases,
        // all of it — because it IS the heightfield, uncarved. The first build
        // carved a slot and lidded it, and the lid read as a grey road over the
        // mountain's shoulder from the air.
        let deep = lift(AT);
        assert!(
            (deep - ridge(AT)).abs() < 0.01,
            "the ground over the tunnel is carved by {:.1} m",
            ridge(AT) - deep
        );
        assert!(
            deep > 150.0,
            "the mountain only holds {deep:.0} m over the tunnel"
        );
        // And nothing up there is told to stop growing.
        assert!(
            underground(AT) < 0.01,
            "the skin over the tunnel reads as underground: {:.2}",
            underground(AT)
        );
    }

    #[test]
    fn the_corridor_is_walkable_from_plain_to_plain() {
        // Walked, not asserted about: enter from open ground on one side, out the
        // other, and the ground underfoot never jumps and never climbs the
        // mountain. The two-level rule hands over from drawn ground to tunnel
        // floor somewhere inside each cutting, and a jump there is a wall.
        let (sin, cos) = HEADING.sin_cos();
        let along = Vec2::new(cos, sin);

        let reach = WALL_THICK * 1.3;
        let mut was = walking(AT - along * reach);
        let mut highest = was;
        let mut biggest_step = 0.0_f32;
        let mut step_at = 0.0;
        for step in 1..=520 {
            let t = -reach + step as f32 * (reach * 2.0 / 520.0);
            let now = walking(AT + along * t);
            if (now - was).abs() > biggest_step {
                biggest_step = (now - was).abs();
                step_at = t;
            }
            highest = highest.max(now);
            was = now;
        }
        assert!(
            biggest_step < 1.2,
            "the walk steps {biggest_step:.2} m at {step_at:.0} m along — a wall in the corridor"
        );
        assert!(
            highest < plain(AT) + BORE_HIGH,
            "the walk climbs to {highest:.1} m — over the mountain, not through it"
        );
    }

    #[test]
    fn the_mountain_blocks_every_other_way_over() {
        // What blocks a walker is the highest ground on their PATH: each candidate
        // crossing beside the tunnel is measured by the most it makes them climb,
        // and the weakest crossing along most of the wall is still a mountain.
        let (sin, cos) = HEADING.sin_cos();
        let along = Vec2::new(cos, sin);
        let across = Vec2::new(-sin, cos);

        let mut weakest = f32::MAX;
        for side in [-1.0_f32, 1.0] {
            for out in 0..=((WALL_LONG * 0.7) as i32 / 10) {
                let aside = (BORE_WIDE + 30.0) + out as f32 * 10.0;
                let mut barrier = 0.0_f32;
                for step in -30..=30 {
                    let at = AT
                        + across * aside * side
                        + along * step as f32 * (WALL_THICK * 1.2 / 30.0);
                    barrier = barrier.max(lift(at));
                }
                weakest = weakest.min(barrier);
            }
        }
        assert!(
            weakest > RIDGE_HIGH * 0.5,
            "somewhere beside the tunnel the crossing only climbs {weakest:.0} m"
        );
    }

    #[test]
    fn the_wall_cannot_be_walked_round_without_going_a_long_way() {
        let (sin, cos) = HEADING.sin_cos();
        let across = Vec2::new(-sin, cos);
        for side in [-1.0_f32, 1.0] {
            let mut round = 0.0;
            for step in 0..1_400 {
                let out = AT + across * side * step as f32;
                if ridge(out) < 8.0 {
                    round = step as f32;
                    break;
                }
            }
            assert!(
                round > WALL_LONG * 0.9,
                "the wall gives out {round:.0} m along, which is a stroll around it"
            );
        }
    }

    #[test]
    fn the_mouths_carve_themselves_where_the_mountain_thins() {
        // Nothing decides where a mouth is: the cutting appears wherever the
        // corridor crosses ground the mountain holds only thinly, so reshaping
        // the mountain moves its own doorways. Inside, no carve at all; at the
        // mouth, carved to the floor; outside, nothing to carve.
        let (sin, cos) = HEADING.sin_cos();
        let along = Vec2::new(cos, sin);

        // Somewhere along the corridor the cutting must be OPEN (underground says
        // cutting) and somewhere deeper it must be roofed (underground says no,
        // because the skin above is whole).
        let mut saw_cutting = false;
        let mut saw_roof = false;
        for step in 0..=200 {
            let t = step as f32 * (WALL_THICK * 1.3 / 200.0);
            let at = AT + along * t;
            let here = underground(at);
            if here > 0.6 {
                saw_cutting = true;
            }
            if ridge(at) > MOUTH_TALL * 2.0 {
                assert!(
                    here < 0.05,
                    "{:.0} m along, under {:.0} m of rock, still reads as a cutting",
                    t,
                    ridge(at)
                );
                saw_roof = true;
            }
        }
        assert!(saw_cutting, "no cutting anywhere along the corridor");
        assert!(saw_roof, "the corridor never passes under the mountain at all");
    }

    #[test]
    fn the_two_level_rule_knows_who_is_underground() {
        // The floor answers only inside the corridor and only under real rock —
        // out in the cuttings and beyond the mouths, the drawn ground owns the
        // walker, or someone strolling the plain past the mouth would snap into
        // a tunnel nobody can see.
        assert!(
            floor(AT, plain(AT)).is_some(),
            "the middle of the tunnel has no floor"
        );

        let (sin, cos) = HEADING.sin_cos();
        let along = Vec2::new(cos, sin);
        let across = Vec2::new(-sin, cos);
        assert!(
            floor(AT + across * (BORE_WIDE + 2.0), plain(AT)).is_none(),
            "the floor reaches outside the tube's own walls"
        );
        assert!(
            floor(AT + along * WALL_THICK * 1.4, plain(AT)).is_none(),
            "the floor runs on past the mountain"
        );
    }
}

#[cfg(test)]
mod lining {
    use super::*;

    fn plain(_: Vec2) -> f32 {
        10.0
    }

    #[test]
    fn the_tube_is_a_tunnel_someone_can_walk_and_see_into() {
        let tube = tube(plain);
        assert!(!tube.is_empty(), "no lining was built");

        // Headroom under the crown, the full length.
        let ring = RING_POINTS + 1 + 2;
        let rings = tube.places.len() / ring;
        for step in 0..rings {
            let crown = tube.places[step * ring + RING_POINTS / 2][1];
            let floor = tube.places[step * ring + ring - 1][1];
            assert!(
                crown - floor > BORE_HIGH * 0.85,
                "ring {step} has {:.1} m of headroom",
                crown - floor
            );
        }

        // Every arch face looks INWARD — at the tunnel's axis — and every floor
        // face looks up. A face wound the other way is invisible from inside,
        // and the first build of the pass was invisible from every angle that
        // mattered for exactly this class of mistake.
        let axis_y = 10.0 + BORE_HIGH * 0.4;
        let (sin, cos) = HEADING.sin_cos();
        let along_way = Vec2::new(cos, sin);
        let mut wrong = 0;
        for face in tube.indices.chunks(3) {
            let corner = |i: usize| Vec3::from_array(tube.places[face[i] as usize]);
            let (a, b, c) = (corner(0), corner(1), corner(2));
            let out = (b - a).cross(c - a);
            if out.length_squared() < 1.0e-8 {
                continue;
            }
            let middle = (a + b + c) / 3.0;
            let t = (Vec2::new(middle.x, middle.z) - AT).dot(along_way);
            let axis = AT + along_way * t;
            let toward = Vec3::new(axis.x, axis_y, axis.y) - middle;
            if out.dot(toward) <= 0.0 {
                wrong += 1;
            }
        }
        assert_eq!(wrong, 0, "{wrong} faces of the lining face into the rock");
    }

    #[test]
    fn the_middle_of_the_tunnel_is_dark_and_the_mouths_are_not() {
        // No sun reaches the middle of a tunnel and no lamp exists yet, so the
        // vertex colours carry the light. This is what makes it read as depth
        // rather than as a grey pipe.
        let tube = tube(plain);
        let ring = RING_POINTS + 1 + 2;
        let rings = tube.places.len() / ring;
        let brightness = |step: usize| {
            let colour = tube.colours[step * ring + RING_POINTS / 2];
            colour[0] + colour[1] + colour[2]
        };
        let mouth = brightness(0).max(brightness(rings - 1));
        let middle = brightness(rings / 2);
        assert!(
            middle < mouth * 0.35,
            "the middle ({middle:.3}) is nearly as bright as the mouths ({mouth:.3})"
        );
    }
}

#[cfg(test)]
mod country {
    use super::*;

    /// The mountain has to be the JOIN between the two countries, not a wall
    /// standing across one of them.
    #[test]
    fn the_desert_meets_its_western_foot_and_the_green_world_its_eastern() {
        use terrain_core::region::Country;
        let terrain = crate::world::terrain::Terrain::new();
        let (sin, cos) = HEADING.sin_cos();
        let along_way = Vec2::new(cos, sin);
        let across_way = Vec2::new(-sin, cos);

        let mut desert = 0;
        let mut green = 0;
        let mut looked = 0;
        // Near the pass, and along the wall rather than out to its ends: the
        // wall is half a kilometre thick now, so probing its feet at the far
        // ends of a nine-hundred-metre wall lands in the sea and measures
        // nothing. What the claim is about is the ground either side of the
        // tunnel.
        for step in -5..=5 {
            let down = across_way * step as f32 * (WALL_LONG * 0.06);
            // Just outside each MOUTH rather than out past the mountain's feet:
            // where a walker actually steps out of the tunnel, and where the
            // question "which country is this" has an answer that matters. Out at
            // the feet of a half-kilometre wall the probes land in the sea.
            let west = AT + down - along_way * WALL_THICK * 0.9;
            let east = AT + down + along_way * WALL_THICK * 0.9;
            if terrain.height(west.x, west.y) < 1.0 || terrain.height(east.x, east.y) < 1.0 {
                continue;
            }
            looked += 1;
            desert += (terrain.region(west.x, west.y).0 == Country::Desert) as i32;
            green += (terrain.region(east.x, east.y).0 == Country::Ordinary) as i32;
        }

        assert!(looked >= 4, "only {looked} places along the wall have land both sides");
        assert!(
            desert * 3 >= looked * 2,
            "the western foot is desert at only {desert} of {looked} places along the wall"
        );
        assert!(
            green * 3 >= looked * 2,
            "the eastern foot is the green world at only {green} of {looked} places"
        );
    }
}
