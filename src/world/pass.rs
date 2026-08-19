//! The mountain pass: a wall of rock across the road east, and a bore through it.
//!
//! # A heightfield has one surface, and a tunnel needs two
//!
//! Everything else in this world is a height per place — one number per (x, z) —
//! and the warden's feet are snapped to it. That is what makes the ground cheap to
//! generate, cheap to stream and impossible to walk under. A hole through a
//! mountain is the one shape it cannot express.
//!
//! So the job is split between the two things that CAN each do half of it:
//!
//! * the **floor and the walls** are the heightfield, carved. Where the passage
//!   runs, the mountain simply is not applied — so the bore's floor is the ordinary
//!   ground the mountain was raised on, at the height it always had, and the walls
//!   are the mountain coming back in over a few metres either side.
//! * the **rock above** is a mesh, built here, filling the slot the carving left
//!   between the tunnel's ceiling and the mountain's own outer skin.
//!
//! Neither half knows anything the other does not: both are drawn from the same
//! [`ridge`] and [`bore`] over the same ground, so the mesh cannot drift off the
//! terrain it is plugging. Sculpt the hillside and the plug follows it.
//!
//! # It ends by running out of mountain
//!
//! Nothing decides where the mouths are. The plug's thickness is the mountain's
//! height above the tunnel's crown, and where the mountain is lower than the crown
//! that is nought — so the roof thins as the ground falls toward the plain, opens
//! into a cutting, and the cutting opens onto the ground. A railway looks exactly
//! like this, for the same reason.

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
/// Enough to read as a wall from a distance and to be over the treeline at its
/// top, which is what makes it bare rock and snow rather than a green hill.
const RIDGE_HIGH: f32 = 165.0;

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
const WALL_LONG: f32 = 760.0;
const WALL_THICK: f32 = 300.0;

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
/// Two eased falloffs, one along the wall and one across it, so the mountain has a
/// body at full height and shoulders that ease into the ground rather than a rim
/// it stands up behind.
pub fn ridge(at: Vec2) -> f32 {
    let (along, across) = local(at);
    let reach = |d: f32, full: f32, flat: f32| {
        crate::util::smoothstep(full, full * flat, d.abs())
    };
    // Thin along the tunnel and long across it: rock to bore through, and a wall
    // reaching away on both sides of the mouth. Peaked in the first and
    // flat-crested in the second — see `SHOULDER`.
    RIDGE_HIGH * reach(along, WALL_THICK, 0.0) * reach(across, WALL_LONG, SHOULDER)
}

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
/// The one number the terrain asks for. The mountain, except where the passage
/// goes — so the floor of the bore is the ground as it was before any of this,
/// which is why the mouths need no blending and why walking in is level.
pub fn lift(at: Vec2) -> f32 {
    ridge(at) * (1.0 - bore(at))
}

/// Whether this is the floor of the bore, 0 to 1.
///
/// What tells the rest of the world to leave it alone: nothing grows on it and it
/// is painted as the stone it is. A tunnel with a meadow in it is not a tunnel.
pub fn underground(at: Vec2) -> f32 {
    // Under rock, rather than merely on the floor: out at the mouths the mountain
    // above has run out, and there the passage is a cutting in the open air with
    // whatever grows in a cutting.
    let roof = ridge(at) - BORE_HIGH;
    bore(at) * crate::util::smoothstep(0.0, BORE_HIGH, roof)
}

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

/// The mountain's height at a point if the bore had never been cut.
fn uncarved(natural: f32, at: Vec2) -> f32 {
    natural + ridge(at)
}

/// The ground as the terrain actually draws it.
fn carved(natural: f32, at: Vec2) -> f32 {
    natural + lift(at)
}

/// The underside of the tunnel, in metres above the sea.
///
/// A half-ellipse over the floor: full headroom down the middle, meeting the foot
/// of the walls at the edge of the bore. Outside the bore it is the floor level,
/// which is below the wall — so the rock's underside there is the wall itself.
fn ceiling(natural: f32, across: f32) -> f32 {
    let t = (across.abs() / BORE_WIDE).min(1.0);
    natural + BORE_HIGH * (1.0 - t * t).max(0.0).sqrt()
}

/// How far the rock's underside is held clear of the ground it lies against.
///
/// Where the walls climb, the plug's underside IS the carved terrain — the same
/// surface, computed twice — and two coincident surfaces fight for the depth
/// buffer. Seen from the air that came out as a black ribbon with a sawtooth edge
/// down the whole pass, which is what the mountain looked like before this.
///
/// A quarter of a metre lifts the rock clear without opening a gap anybody can see
/// into: both surfaces are stone and one is standing just inside the other.
const CLEAR_OF_THE_GROUND: f32 = 0.25;

/// How many steps the plug is built in, along the tunnel and across it.
///
/// The bore is a couple of hundred metres long and twenty-odd wide, so these are
/// about two metres a step in both directions — the terrain's own vertex spacing,
/// which is the resolution the thing it has to meet is drawn at.
const STEPS_ALONG: usize = 150;
const STEPS_ACROSS: usize = 20;

/// How far past the mountain the plug is built, as a share of its reach.
///
/// Past where the rock runs out, so the roof is thinning to nothing well inside
/// the mesh's own edge and the mesh's edge is never a thing you can see.
const OVERRUN: f32 = 1.25;

/// The rock standing over the bore.
///
/// `natural` is the ground WITHOUT the pass — the height the world would have if
/// none of this were here. It is a closure because the answer includes whatever a
/// maker has sculpted, so it cannot be worked out from the constants above.
///
/// Two sheets: the mountain's outer skin over the slot, and the tunnel's ceiling
/// under it. Where the mountain is no taller than the tunnel's crown the two meet
/// and the volume closes itself — which is what makes the mouths mouths, and means
/// there is no end cap anywhere to get wrong.
pub fn rock_over_the_bore(natural: impl Fn(Vec2) -> f32) -> Geometry {
    let (sin, cos) = HEADING.sin_cos();
    let along_way = Vec2::new(cos, sin);
    let across_way = Vec2::new(-sin, cos);
    let long = WALL_THICK * OVERRUN;
    let wide = (BORE_SPAN + BORE_WALL) * OVERRUN;

    // Both sheets over the same grid, so a vertex on one has its opposite number
    // on the other and the two can be stitched without searching for anything.
    let mut top = Vec::with_capacity((STEPS_ALONG + 1) * (STEPS_ACROSS + 1));
    let mut under = Vec::with_capacity(top.capacity());
    for step in 0..=STEPS_ALONG {
        let along = -long + step as f32 * (2.0 * long / STEPS_ALONG as f32);
        for slot in 0..=STEPS_ACROSS {
            let across = -wide + slot as f32 * (2.0 * wide / STEPS_ACROSS as f32);
            let at = AT + along_way * along + across_way * across;
            let ground = natural(at);

            let skin = uncarved(ground, at);
            // The rock's underside: the arch where it is over the passage, and
            // the carved wall where it is not — held just clear of that wall so
            // the two are never the same surface twice.
            let below =
                ceiling(ground, across).max(carved(ground, at) + CLEAR_OF_THE_GROUND);
            // Pinched shut wherever the mountain is no taller than the tunnel.
            let below = below.min(skin);

            let place = |y: f32| Vec3::new(at.x, y, at.y);
            top.push(place(skin));
            under.push(place(below));
        }
    }

    let mut rock = Geometry::default();
    sheet(&mut rock, &top, true);
    sheet(&mut rock, &under, false);
    settle_the_normals(&mut rock);
    rock
}

/// Gives every vertex the average of the faces that meet at it.
///
/// The ordinary way to normal a welded mesh, and here it is also the guarantee:
/// a normal built OUT OF the winding cannot contradict it. `up` decides which way
/// a sheet is wound and the shading follows, rather than the two being written
/// down separately and drifting.
fn settle_the_normals(mesh: &mut Geometry) {
    for face in mesh.indices.chunks(3) {
        let corner = |i: usize| Vec3::from_array(mesh.places[face[i] as usize]);
        let (a, b, c) = (corner(0), corner(1), corner(2));
        // Not normalised: a big triangle should count for more than a sliver, and
        // a face pinched to nothing should count for nothing at all.
        let weight = (b - a).cross(c - a);
        for slot in face {
            let normal = &mut mesh.normals[*slot as usize];
            normal[0] += weight.x;
            normal[1] += weight.y;
            normal[2] += weight.z;
        }
    }
    for (normal, place) in mesh.normals.iter_mut().zip(&mesh.places) {
        let settled = Vec3::from_array(*normal).normalize_or_zero();
        // A vertex with no face of any area at it — the very rim of the pinch.
        // Pointed at the sky, which is what the ground it is lying on does.
        *normal = if settled.length_squared() < 0.5 {
            let _ = place;
            [0.0, 1.0, 0.0]
        } else {
            settled.to_array()
        };
    }
}

/// Adds one grid of points to the mesh as a surface.
///
/// `up` says which way it faces, which is the only difference between the two: the
/// mountain's skin is seen from outside and the tunnel's ceiling from underneath.
fn sheet(mesh: &mut Geometry, grid: &[Vec3], up: bool) {
    let wide = STEPS_ACROSS + 1;
    let base = mesh.places.len() as u32;

    for (index, point) in grid.iter().enumerate() {
        let col = index % wide;
        mesh.places.push(point.to_array());
        // Filled in from the faces once they exist — see `settle_the_normals`.
        // Worked out from neighbouring grid points instead, a normal flips
        // wherever the sheet is pinched flat against its opposite number, and a
        // normal that disagrees with its own winding is a face lit from the wrong
        // side. Deriving one from the other is the only arrangement where they
        // cannot differ.
        mesh.normals.push([0.0, 0.0, 0.0]);
        mesh.uvs.push([col as f32 / wide as f32, (index / wide) as f32]);
        mesh.colours.push(stone(*point, up));
    }

    for row in 0..STEPS_ALONG {
        for col in 0..STEPS_ACROSS {
            let a = base + (row * wide + col) as u32;
            let (b, c, d) = (a + 1, a + wide as u32, a + wide as u32 + 1);
            // Wound so the face points the way its own normals do.
            //
            // These were the other way round, and both sheets were therefore
            // backface-culled: from above you looked straight THROUGH the
            // mountain's skin into the slot, which is why a tunnel read as a
            // ridge with a cut in it, and from inside there was no ceiling at
            // all. `every_face_is_wound_the_way_it_faces` is the check that
            // will not let it happen again.
            if up {
                mesh.indices.extend_from_slice(&[a, b, c, b, d, c]);
            } else {
                mesh.indices.extend_from_slice(&[a, c, b, b, c, d]);
            }
        }
    }
}

/// What the rock is painted, in linear RGBA.
///
/// Stone, and the ceiling darker than the skin: no sunlight reaches the inside of
/// a tunnel, and the shadow map alone will not say so at the mouths where it does.
fn stone(at: Vec3, up: bool) -> [f32; 4] {
    let grain = terrain_core::forest::field(Vec2::new(at.x, at.z) / 7.0, 77);
    let shade = if up { 0.86 + grain * 0.28 } else { 0.30 + grain * 0.12 };
    [0.115 * shade, 0.108 * shade, 0.101 * shade, 1.0]
}

/// The rock over the bore, as an entity in the world.
#[derive(Component)]
pub struct Roof;

/// Builds and stands the rock over the bore.
///
/// Once, with the world, rather than per chunk: it is a couple of hundred metres
/// of mesh in one fixed place, and streaming it would mean cutting it at chunk
/// boundaries for no gain. It goes away with the world, so the workbench does not
/// keep a mountain behind it.
pub fn raise_the_roof(
    mut commands: Commands,
    terrain: Res<crate::world::terrain::TerrainSource>,
    mut meshes: ResMut<Assets<Mesh>>,
    material: Option<Res<crate::world::chunk::TerrainMaterial>>,
) {
    let Some(material) = material else {
        return;
    };
    // Against the ground WITHOUT the pass — the plug's thickness is the
    // difference between the two, so it has to be measured from the world the
    // mountain was raised on.
    let rock = rock_over_the_bore(|at| terrain.0.without_the_pass(at.x, at.y));
    if rock.is_empty() {
        return;
    }
    commands.spawn((
        Roof,
        Mesh3d(meshes.add(crate::world::stream::as_coloured_mesh(&rock))),
        MeshMaterial3d(material.0.clone()),
        Transform::IDENTITY,
        Visibility::default(),
    ));
}

/// Takes it down with the rest of the world.
pub fn drop_the_roof(mut commands: Commands, roofs: Query<Entity, With<Roof>>) {
    for roof in &roofs {
        commands.entity(roof).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A flat world at ten metres, so the pass can be measured on its own.
    fn plain(_: Vec2) -> f32 {
        10.0
    }

    #[test]
    fn the_mountain_is_a_wall_and_the_bore_goes_through_it() {
        // The whole point of the thing: you cannot walk over it, and you can walk
        // through it. Measured as a walk rather than asserted about constants.
        let (sin, cos) = HEADING.sin_cos();
        let along = Vec2::new(cos, sin);
        let across = Vec2::new(-sin, cos);

        // Straight through the middle: the ground never rises.
        let mut highest: f32 = 0.0;
        for step in -200..=200 {
            let at = AT + along * step as f32 * 2.0;
            highest = highest.max(lift(at));
        }
        assert!(
            highest < 0.5,
            "the bore climbs {highest:.1} m — it is not a way through"
        );

        // And beside it, everywhere a walker might try to get over instead: within
        // the mountain's own thickness, every step out along the wall is rock.
        //
        // Along the WALL, not along the tunnel. Walking along the tunnel leaves the
        // mountain behind after its thickness and finds open ground, which is
        // correct and is what the first version of this managed to fail on.
        let mut lowest = f32::MAX;
        let mut where_lowest = Vec2::ZERO;
        for side in [-1.0_f32, 1.0] {
            for out in 0..=((WALL_LONG * 0.7) as i32 / 5) {
                let aside = (BORE_SPAN + BORE_WALL + 5.0) + out as f32 * 5.0;
                for step in -12..=12 {
                    let at = AT
                        + across * aside * side
                        + along * step as f32 * (WALL_THICK * 0.5 / 12.0);
                    if lift(at) < lowest {
                        lowest = lift(at);
                        where_lowest = at - AT;
                    }
                }
            }
        }
        assert!(
            lowest > RIDGE_HIGH * 0.4,
            "the wall is only {lowest:.0} m at {where_lowest:?} — it can be walked over"
        );
    }

    #[test]
    fn the_wall_cannot_be_walked_round_without_going_a_long_way() {
        // A barrier with a short way round it is scenery. The mountain has to
        // reach far enough along its own line that going around is a journey.
        let (sin, cos) = HEADING.sin_cos();
        let across = Vec2::new(-sin, cos);
        for side in [-1.0_f32, 1.0] {
            let mut round = 0.0;
            for step in 0..1_200 {
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
    fn the_bore_has_headroom_and_a_roof_over_it() {
        // A hole you can walk through has rock above it. At the middle, the plug's
        // underside must clear a walker's head and its top must be the mountain.
        let ground = plain(AT);
        let skin = uncarved(ground, AT);
        let below = ceiling(ground, 0.0).max(carved(ground, AT)).min(skin);

        assert!(
            below - ground >= 2.5,
            "only {:.1} m of headroom at the middle of the bore",
            below - ground
        );
        assert!(
            skin - below > 50.0,
            "the roof is {:.0} m thick — that is a lid, not a mountain",
            skin - below
        );
    }

    #[test]
    fn the_roof_thins_to_nothing_before_the_mesh_ends() {
        // What makes a mouth a mouth: nothing decides where it is. The rock runs
        // out because the mountain does, and it has to run out INSIDE the built
        // grid or the plug would end in mid-air with a visible edge.
        let (sin, cos) = HEADING.sin_cos();
        let along = Vec2::new(cos, sin);
        let long = WALL_THICK * OVERRUN;

        for side in [-1.0_f32, 1.0] {
            let edge = AT + along * side * long;
            let ground = plain(edge);
            let skin = uncarved(ground, edge);
            let below = ceiling(ground, 0.0).max(carved(ground, edge)).min(skin);
            assert!(
                skin - below < 0.01,
                "the plug is still {:.2} m thick at its own edge",
                skin - below
            );
        }
    }

    #[test]
    fn nothing_grows_under_the_mountain_and_everything_does_outside_it() {
        // A tunnel with a meadow in it is not a tunnel — and a cutting in the open
        // air is not a tunnel either, so the mouths keep their grass.
        assert!(
            underground(AT) > 0.9,
            "the middle of the bore is only {:.2} underground",
            underground(AT)
        );

        let (sin, cos) = HEADING.sin_cos();
        let across = Vec2::new(-sin, cos);
        let beside = AT + across * (BORE_SPAN + BORE_WALL + 5.0);
        assert!(
            underground(beside) < 0.05,
            "the rock beside the bore reads as passage"
        );

        let along = Vec2::new(cos, sin);
        let outside = AT + along * WALL_THICK * 1.4;
        assert!(
            underground(outside) < 0.05,
            "open ground past the mountain reads as underground"
        );
    }

    #[test]
    fn the_rock_closes_itself_at_every_edge() {
        // Built rather than reasoned about: the two sheets must meet all the way
        // round, or the plug is an open shell and you can see into the mountain.
        let rock = rock_over_the_bore(plain);
        assert!(!rock.is_empty(), "no rock was built at all");

        let wide = STEPS_ACROSS + 1;
        let half = rock.places.len() / 2;
        let apart = |index: usize| {
            let top = rock.places[index];
            let under = rock.places[half + index];
            (top[1] - under[1]).abs()
        };

        // Every edge of the grid: both ends of the tunnel and both sides of it.
        for slot in 0..=STEPS_ACROSS {
            for row in [0usize, STEPS_ALONG] {
                let gap = apart(row * wide + slot);
                assert!(gap < 0.01, "the plug stands {gap:.2} m open at its end");
            }
        }
        for row in 0..=STEPS_ALONG {
            for col in [0usize, STEPS_ACROSS] {
                let gap = apart(row * wide + col);
                assert!(gap < 0.01, "the plug stands {gap:.2} m open at its side");
            }
        }

        // And it is genuinely thick somewhere in the middle, or it closed by being
        // nothing at all.
        let middle = (STEPS_ALONG / 2) * wide + STEPS_ACROSS / 2;
        assert!(apart(middle) > 50.0, "the plug has no mountain in it");
    }
}

#[cfg(test)]
mod section {
    use super::*;

    /// Prints a slice through the pass, for looking at.
    ///
    /// `cargo test dump_the_pass -- --ignored --nocapture`. Rock is `#`, the air
    /// inside the bore is a space, and the floor is `_`. Across the tunnel first,
    /// then along it — the two slices between them say whether this is a hole
    /// through a mountain or a trench with a lid on it.
    #[test]
    #[ignore = "a picture, not a check"]
    fn dump_the_pass() {
        let ground = 10.0;
        let (sin, cos) = HEADING.sin_cos();
        let along_way = Vec2::new(cos, sin);
        let across_way = Vec2::new(-sin, cos);

        let solid = |at: Vec2, across: f32, y: f32| {
            let carved = carved(ground, at);
            let top = uncarved(ground, at);
            let bottom = ceiling(ground, across).max(carved).min(top);
            y < carved || (y >= bottom && y <= top)
        };

        for (title, across_slice) in [("ACROSS the tunnel", true), ("ALONG the tunnel", false)] {
            println!("--- {title}");
            for row in (0..46).rev() {
                let y = ground - 4.0 + row as f32 * 4.0;
                let line: String = (0..96)
                    .map(|col| {
                        let d = -48.0 + col as f32;
                        let (at, across) = if across_slice {
                            (AT + across_way * d, d)
                        } else {
                            (AT + along_way * (d * 4.0), 0.0)
                        };
                        if solid(at, across, y) {
                            '#'
                        } else if y < ground {
                            '_'
                        } else {
                            ' '
                        }
                    })
                    .collect();
                println!("{y:6.0} |{line}|");
            }
        }
    }
}

#[cfg(test)]
mod against_the_world {
    use super::*;

    #[test]
    #[ignore = "a probe"]
    fn what_the_roof_actually_covers() {
        let terrain = crate::world::terrain::Terrain::new();
        let natural = |at: Vec2| terrain.height(at.x, at.y) - lift(at);

        println!("ground at the bore: {:.1}", natural(AT));
        println!("ridge there: {:.1}", ridge(AT));
        println!("carved (drawn) there: {:.1}", terrain.height(AT.x, AT.y));
        let (sin, cos) = HEADING.sin_cos();
        let across_way = Vec2::new(-sin, cos);
        for step in 0..10 {
            let d = step as f32 * 4.0;
            let at = AT + across_way * d;
            println!(
                "  across {d:5.1}: bore {:.2} lift {:6.1} drawn {:6.1}",
                bore(at),
                lift(at),
                terrain.height(at.x, at.y)
            );
        }

        let rock = rock_over_the_bore(natural);
        let mut low = f32::MAX;
        let mut high = f32::MIN;
        for place in &rock.places {
            low = low.min(place[1]);
            high = high.max(place[1]);
        }
        println!(
            "roof: {} verts, {} tris, y {:.1}..{:.1}",
            rock.places.len(),
            rock.indices.len() / 3,
            low,
            high
        );
    }
}

#[cfg(test)]
mod winding {
    use super::*;

    /// Every face has to be seen from the side it is meant to be seen from.
    ///
    /// # The whole mountain was inside out
    ///
    /// Both sheets were wound backwards, so both were culled: from above you
    /// looked straight through the mountain's skin into the slot — reported as
    /// "this is just a tall ridge with a cut" — and from inside the bore there was
    /// no ceiling overhead either.
    ///
    /// The first version of this test compared each face against its own vertices'
    /// normals, which is the check terrain-core makes on a tree and is the wrong
    /// check here: the bore's lining is one folded sheet that turns through more
    /// than a right angle on its way from the wall to the crown, so a face and the
    /// average at its corners genuinely disagree at the crease. What matters is
    /// not agreement — it is which side each surface is looked at from, and there
    /// are exactly two answers: the mountain's skin is seen from the sky, and the
    /// lining is seen from the space you walk in.
    #[test]
    fn the_skin_faces_the_sky_and_the_lining_faces_the_tunnel() {
        let ground = 20.0;
        let rock = rock_over_the_bore(|_| ground);
        assert!(rock.indices.len() > 3_000, "too little rock to be a test");

        // The two sheets are pushed in order, so the first half is the skin.
        let half = rock.places.len() / 2;
        let (sin, cos) = HEADING.sin_cos();
        let across_way = Vec3::new(-sin, 0.0, cos);

        let mut sky_wrong = 0;
        let mut lining_wrong = 0;
        let mut pinched = 0;
        for face in rock.indices.chunks(3) {
            let corner = |i: usize| Vec3::from_array(rock.places[face[i] as usize]);
            let (a, b, c) = (corner(0), corner(1), corner(2));
            let wound = (b - a).cross(c - a);
            if wound.length_squared() < 1.0e-8 {
                // Pinched shut against the other sheet — that is how the mouths
                // close, and a face with no area is drawn as nothing.
                pinched += 1;
                continue;
            }
            let out = wound.normalize();
            let middle = (a + b + c) / 3.0;

            if (face[0] as usize) < half {
                // The mountain's skin. It is a height over the ground and never
                // folds, so every one of its faces looks at the sky.
                if out.y <= 0.0 {
                    sky_wrong += 1;
                }
            } else {
                // The bore's lining, where it is the CEILING — the part a warden
                // walks under and looks up at. It has to face down.
                //
                // Only there. The lining is one folded sheet: it turns through
                // more than a right angle from the arch's crown down the wall to
                // where it lies flat against the mountainside, so a face out on
                // the wall points sideways and a face at the pinch points up, and
                // both are correct. What can be stated plainly is the ceiling.
                let across = (middle - Vec3::new(AT.x, 0.0, AT.y)).dot(across_way);
                if across.abs() < BORE_WIDE * 0.7 && middle.y > ground + BORE_HIGH * 0.4 {
                    if out.y >= 0.0 {
                        lining_wrong += 1;
                    }
                }
            }
        }

        assert_eq!(sky_wrong, 0, "{sky_wrong} faces of the mountain's skin face the ground");
        assert_eq!(
            lining_wrong, 0,
            "{lining_wrong} faces of the tunnel's ceiling face up into the rock"
        );
        assert!(
            pinched < rock.indices.len() / 6,
            "{pinched} faces have no area — the plug is mostly pinched shut"
        );
    }
}

#[cfg(test)]
mod country {
    use super::*;

    /// The mountain has to be the JOIN between the two countries, not a wall
    /// standing across one of them.
    ///
    /// Reported from the game as "this side of the mountain should be desert not
    /// grass": the wall ran due north-south across a boundary that runs on a
    /// diagonal, so it crossed rather than followed it and one end had the wrong
    /// country on the wrong flank.
    #[test]
    fn the_desert_meets_its_western_foot_and_the_green_world_its_eastern() {
        use terrain_core::region::Country;
        let terrain = crate::world::terrain::Terrain::new();
        let (sin, cos) = HEADING.sin_cos();
        let along_way = Vec2::new(cos, sin);
        let across_way = Vec2::new(-sin, cos);

        // Along the whole length of the wall, not at one point on it — crossing
        // the boundary at an angle is exactly what this is here to catch.
        let mut desert = 0;
        let mut green = 0;
        let mut looked = 0;
        for step in -6..=6 {
            let down = across_way * step as f32 * (WALL_LONG * 0.1);
            let west = AT + down - along_way * WALL_THICK * 1.1;
            let east = AT + down + along_way * WALL_THICK * 1.1;
            // Only where there is land on both sides to have an opinion about.
            if terrain.height(west.x, west.y) < 1.0 || terrain.height(east.x, east.y) < 1.0 {
                continue;
            }
            looked += 1;
            desert += (terrain.region(west.x, west.y).0 == Country::Desert) as i32;
            green += (terrain.region(east.x, east.y).0 == Country::Ordinary) as i32;
        }

        assert!(looked >= 5, "only {looked} places along the wall have land both sides");
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
