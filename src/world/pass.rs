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
/// On the desert's eastern edge, on the long unbroken corridor of land at
/// z ≈ -880 — the map printed by `dump_the_world` is what this was read off. The
/// journey east is desert, then this, then the green country, then the snow.
pub const AT: Vec2 = Vec2::new(200.0, -880.0);

/// Which way the tunnel runs, in radians about Y. Nought is due east.
///
/// A number rather than a hard-coded axis so the whole pass can be turned to meet
/// the country's own lean without anything else being touched.
pub const HEADING: f32 = 0.0;

/// How high the mountain stands above the ground it is raised on, in metres.
///
/// Enough to read as a wall from a distance and to be over the treeline at its
/// top, which is what makes it bare rock and snow rather than a green hill.
const RIDGE_HIGH: f32 = 165.0;

/// How far the mountain reaches, in metres: the length of the WALL, measured
/// across the tunnel, and its THICKNESS, measured along the tunnel.
///
/// The wall is long and the bore is short, which is the whole shape of a pass: a
/// barrier you cannot walk round and a way through you can walk in a minute.
///
/// **Named for the wall rather than for the tunnel, and that is worth the extra
/// word.** They were `ALONG` and `ACROSS`, which read naturally and meant the
/// opposite of what `local` returns — so the mountain was built long in the
/// direction you travel and thin in the direction it was supposed to block. The
/// tests said so at once: the wall gave out 143 m to the side, and the plug was
/// still 156 m thick at its own edge.
const WALL_LONG: f32 = 760.0;
const WALL_THICK: f32 = 150.0;

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

/// Half the width of the walkable floor, and how far past that the walls take to
/// close, both in metres.
///
/// Fourteen metres of floor is a road, not a corridor: wide enough for whatever
/// ends up walking through it and for a branch to open off it later. The walls
/// come back in over four, which on a hundred and sixty metres of mountain is as
/// near vertical as a heightfield can say.
const BORE_WIDE: f32 = 7.0;
const BORE_WALL: f32 = 4.0;

/// Headroom from the floor to the crown of the arch, in metres.
///
/// Nine, which is generous for a person and is meant to be: this is a way through
/// a mountain, and a tunnel you have to stoop in reads as a drain.
const BORE_HIGH: f32 = 9.0;

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
    crate::util::smoothstep(BORE_WIDE + BORE_WALL, BORE_WIDE, across.abs())
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
    let wide = (BORE_WIDE + BORE_WALL) * OVERRUN;

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
            // The rock's underside: the arch where it is over the passage, and the
            // carved wall where it is not.
            let below = ceiling(ground, across).max(carved(ground, at));
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
    rock
}

/// Adds one grid of points to the mesh as a surface.
///
/// `up` says which way it faces, which is the only difference between the two: the
/// mountain's skin is seen from outside and the tunnel's ceiling from underneath.
fn sheet(mesh: &mut Geometry, grid: &[Vec3], up: bool) {
    let wide = STEPS_ACROSS + 1;
    let base = mesh.places.len() as u32;

    for (index, point) in grid.iter().enumerate() {
        let (row, col) = (index / wide, index % wide);
        // From the grid itself rather than from the formulae, so a normal is the
        // normal of the thing actually drawn — including wherever it has been
        // pinched flat against the other sheet.
        let step = |a: usize, b: usize| grid[b.min(grid.len() - 1)] - grid[a.min(grid.len() - 1)];
        let along = if row + 1 < grid.len() / wide {
            step(index, index + wide)
        } else {
            step(index - wide, index)
        };
        let across = if col + 1 < wide {
            step(index, index + 1)
        } else {
            step(index - 1, index)
        };
        let mut normal = across.cross(along).normalize_or_zero();
        if normal.length_squared() < 0.5 {
            normal = Vec3::Y;
        }
        if (normal.y > 0.0) != up {
            normal = -normal;
        }

        mesh.places.push(point.to_array());
        mesh.normals.push(normal.to_array());
        mesh.uvs.push([col as f32 / wide as f32, row as f32]);
        mesh.colours.push(stone(*point, up));
    }

    for row in 0..STEPS_ALONG {
        for col in 0..STEPS_ACROSS {
            let a = base + (row * wide + col) as u32;
            let (b, c, d) = (a + 1, a + wide as u32, a + wide as u32 + 1);
            if up {
                mesh.indices.extend_from_slice(&[a, c, b, b, c, d]);
            } else {
                mesh.indices.extend_from_slice(&[a, b, c, b, d, c]);
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
                let aside = (BORE_WIDE + BORE_WALL + 5.0) + out as f32 * 5.0;
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
        let beside = AT + across * (BORE_WIDE + BORE_WALL + 5.0);
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
