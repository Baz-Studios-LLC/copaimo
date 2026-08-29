//! The lamps that light a settlement after dark.
//!
//! # Two fittings, one job
//!
//! A city stands a steel column at the kerb with an arm out over the carriageway; a
//! village hangs a lantern on a timber post at head height. Built in
//! `dev/art/lamp.py`, stood along the streets by `town::light_the_streets`, and lit
//! here.
//!
//! # Why only a handful are lit at once
//!
//! A city lays a lamp every twenty-six metres, so a large one has well over a
//! hundred of them and the world has more than a thousand. Every one of those as a
//! live point light is not a lighting model, it is a stall: clustered forward
//! shading has a hard ceiling on lights per cluster, and a street is exactly the
//! case where they all land in the same cluster.
//!
//! So the FITTINGS all stand, always, and the LIGHTS follow the warden - the nearest
//! `MOST_LIT` of them, re-chosen as he walks. Nobody can see a lamp go dark behind
//! them at fifty metres, and the ones in front are the ones that were going to be
//! looked at.

use bevy::prelude::*;
use bevy::scene::SceneRoot;

use crate::world::StreamAnchor;
use crate::world::terrain::TerrainSource;

/// How many lamps are lit at once.
const MOST_LIT: usize = 20;

/// How far a lamp can be from the warden and still be lit, in metres.
const LIT_WITHIN: f32 = 85.0;

/// How bright each fitting burns, in lumens, and how far its light carries.
///
/// Warm, because every other light in this world is daylight or a grey overcast, and
/// the one thing a lamp has to do is feel like a different KIND of light from the
/// sky.
///
/// These were both a tenth of this and reported as "very dim", which they were:
/// Bevy's own default point light is a million lumens, so 120,000 is a candle. A
/// street lamp is a made thing on a pole meant to light a carriageway; a village
/// lantern is somebody's lamp outside their door and is meant to be the smaller of
/// the two, so they no longer share a number.
// Halved once the road was wound the right way up. These were tuned while the
// paving faced DOWN and took no light at all, so they were set bright enough to
// make the GROUND read - and the moment the road started taking light too, a city
// square came out blown white.
const STREET_BURNS: f32 = 420_000.0;
const POST_BURNS: f32 = 190_000.0;
const STREET_CARRIES: f32 = 34.0;
const POST_CARRIES: f32 = 20.0;
const LAMPLIGHT: Color = Color::srgb(1.0, 0.82, 0.55);

/// How high the sun has to sink before the lamps come on.
///
/// `TimeOfDay::sun` is 1 at noon and negative at night. They come up through dusk
/// rather than snapping on, which is most of what makes an evening read as an
/// evening - and they are fully up before it is properly dark, the way a lamp that
/// somebody lit would be.
const LIT_BELOW: f32 = 0.16;
const FULLY_LIT_AT: f32 = -0.04;

/// A lamp standing in the world.
#[derive(Component)]
pub struct Lamp {
    /// Where its light hangs above its foot.
    head: f32,
    /// How far out along the arm the head is.
    ///
    /// # The light was coming out of the column
    ///
    /// A city fitting reaches 1.5 m out over the carriageway and the lamp is on the
    /// END of that arm. The light was hung at `(0, head, 0)` - straight up from the
    /// foot - so the glow came off the top of the post while the lamp head beside it
    /// stayed dark, which reads as a bug rather than as a lamp. A village lantern
    /// sits on top of its post and this is nought for it.
    arm: f32,
}

/// The light hung on the lamp this is a child of.
#[derive(Component)]
struct Lit;

/// The lamp's own glass, which GLOWS rather than being lit.
///
/// # A bulb cannot be lit by itself
///
/// The fitting's glass came out black at night while the housing around it was lit
/// warm, and the reason is obvious once said: the point light sits inside the glass,
/// and nothing lights a surface from its own middle. A lamp does not reflect light,
/// it makes it - so the glass is emissive, and it is the one thing in the world that
/// is.
///
/// A separate piece rather than part of the model, because a figure is welded into
/// one object with one material and its colour lives in its vertices. This sits
/// exactly over the model's own glass and is slightly bigger, so what you see is the
/// glow.
#[derive(Component)]
struct Glass;

/// How much brighter than its colour the glass burns.
const GLASS_GLOWS: f32 = 6.0;

/// A storey of a city building with somebody still in it.
///
/// # A city with nobody home
///
/// Lamps light the street and leave the towers as black slabs against a black sky,
/// which is the opposite of what a city looks like at night: what says "people live
/// here" from a distance is not the street lighting, it is that some of the windows
/// are on and most are not.
///
/// A band of glass per lit storey, sat just proud of the facade and emissive, so it
/// reads at any distance and costs one box. Which storeys are lit comes from a hash
/// of the building's own position, so a tower's pattern is its own and does not
/// change as you walk toward it.
#[derive(Component)]
struct Awake;

/// How far away a city's windows are still worth lighting, in metres.
///
/// Much further than the lamps: a lit tower is what you SEE a city by, and the
/// whole point is that it reads from outside.
const AWAKE_WITHIN: f32 = 900.0;

/// What share of a city's storeys have somebody in at night.
const AWAKE_SHARE: f32 = 0.34;

/// A storey, floor to floor, in each age of the world - `FLOOR_TALL` and `STOREY`
/// in `dev/art/town.py`.
const FLOOR_TALL: f32 = 3.4;
const STOREY: f32 = 3.6;

/// How wide and tall a cottage's window is, in metres, and how far up its storey.
const PANE: Vec2 = Vec2::new(0.9, 1.15);
const PANE_UP: f32 = 1.7;

/// What a lit window is coloured. Warmer and paler than a street lamp: it is a room
/// with a lamp in it seen through glass, not the lamp itself.
const INDOORS: Color = Color::srgb(1.0, 0.90, 0.68);
const WINDOW_GLOWS: f32 = 2.6;

/// Stands every lamp near the warden, and takes down the ones left behind.
pub fn stand_the_lamps(
    mut commands: Commands,
    assets: Res<AssetServer>,
    terrain: Res<TerrainSource>,
    built: Res<crate::world::town::Built>,
    standing: Query<(Entity, &crate::world::town::FromSite), With<Lamp>>,
) {
    // One lamp entity per lamp in every settlement that is currently raised, and
    // none for one that is not - the settlements own the lifetime, so this follows
    // them rather than keeping a second idea of what is standing.
    for (key, layout) in built.standing.iter() {
        if standing.iter().any(|(_, from)| from.0 == *key) {
            continue;
        }
        for lamp in &layout.lamps {
            // A lamp's base is small but it still has one, and a post half-buried
            // at the kerb reads worse than a post standing a centimetre proud.
            let ground = crate::world::town::stands_at(
                &terrain.0,
                lamp.at,
                Vec2::splat(0.6),
                lamp.turn,
            );
            let street = lamp.head > crate::world::town::POST_HEAD + 0.5;
            commands.spawn((
                Lamp {
                    head: lamp.head,
                    arm: if street { crate::world::town::STREET_ARM } else { 0.0 },
                },
                crate::world::town::FromSite(*key),
                SceneRoot(assets.load(GltfAssetLabel::Scene(0).from_asset(if street {
                    "models/lamp_street.glb"
                } else {
                    "models/lamp_post.glb"
                }))),
                Transform::from_xyz(lamp.at.x, ground, lamp.at.y)
                    .with_rotation(Quat::from_rotation_y(lamp.turn)),
                Visibility::default(),
            ));
        }
    }
}

/// Lights the nearest lamps, and puts out the rest.
pub fn light_them_at_night(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut paints: ResMut<Assets<StandardMaterial>>,
    mut glow: Local<Option<Handle<StandardMaterial>>>,
    mut bulb: Local<Option<Handle<Mesh>>>,
    clock: Res<crate::sky::TimeOfDay>,
    anchors: Query<&GlobalTransform, With<StreamAnchor>>,
    lamps: Query<(Entity, &GlobalTransform, &Lamp)>,
    mut lit: Query<(Entity, &ChildOf, &mut PointLight), With<Lit>>,
    glass: Query<(Entity, &ChildOf), With<Glass>>,
) {
    let Some(anchor) = anchors.iter().next() else {
        return;
    };
    let here = anchor.translation();

    // How far up the lamps are, nought by day and one after dusk.
    let up = crate::util::smoothstep(LIT_BELOW, FULLY_LIT_AT, clock.sun_height());

    // The nearest few, which are the ones anybody is going to look at.
    let mut near: Vec<(f32, Entity, f32, f32)> = lamps
        .iter()
        .map(|(entity, at, lamp)| {
            (at.translation().distance(here), entity, lamp.head, lamp.arm)
        })
        .filter(|(away, ..)| *away < LIT_WITHIN)
        .collect();
    near.sort_by(|a, b| a.0.total_cmp(&b.0));
    near.truncate(MOST_LIT);

    // Put out anything that is no longer in that list, and dim everything by day
    // rather than tearing the lights down and building them again at dusk.
    for (entity, of, mut light) in &mut lit {
        if let Some((_, _, _, arm)) = near.iter().find(|(_, lamp, ..)| *lamp == of.parent()) {
            light.intensity = if *arm > 0.0 { STREET_BURNS } else { POST_BURNS } * up;
        } else {
            commands.entity(entity).despawn();
        }
    }
    // The glass belongs to the light: when one goes, so does the other.
    for (entity, of) in &glass {
        if !near.iter().any(|(_, lamp, ..)| *lamp == of.parent()) {
            commands.entity(entity).despawn();
        }
    }

    if up <= 0.0 {
        return;
    }
    let glow = glow
        .get_or_insert_with(|| {
            paints.add(StandardMaterial {
                base_color: LAMPLIGHT,
                // Unlit AND emissive: unlit so the sun cannot shade it, emissive so
                // it reads as a source rather than as a pale box.
                emissive: LinearRgba::from(LAMPLIGHT) * GLASS_GLOWS,
                unlit: true,
                ..default()
            })
        })
        .clone();
    let bulb = bulb
        .get_or_insert_with(|| meshes.add(Cuboid::new(1.0, 1.0, 1.0)))
        .clone();

    for (_, lamp, head, arm) in near {
        if lit.iter().any(|(_, of, _)| of.parent() == lamp) {
            continue;
        }
        let street = arm > 0.0;
        commands.entity(lamp).with_children(|on| {
            on.spawn((
                Lit,
                PointLight {
                    color: LAMPLIGHT,
                    intensity: if street { STREET_BURNS } else { POST_BURNS } * up,
                    range: if street { STREET_CARRIES } else { POST_CARRIES },
                    // A hundred and change of these and shadows are the whole frame
                    // budget. The fitting casts none; what it lights does.
                    shadows_enabled: false,
                    ..default()
                },
                // OUT ON THE ARM, which is where the head is.
                Transform::from_xyz(arm, head, 0.0),
            ));
            // And the glass it comes out of, sat over the model's own.
            let (size, drop) = if street {
                (Vec3::new(0.78, 0.24, 0.38), -0.10)
            } else {
                (Vec3::new(0.40, 0.50, 0.40), 0.0)
            };
            on.spawn((
                Glass,
                Mesh3d(bulb.clone()),
                MeshMaterial3d(glow.clone()),
                Transform::from_xyz(arm, head + drop, 0.0).with_scale(size),
                Visibility::default(),
                bevy::pbr::NotShadowCaster,
            ));
        });
    }
}

/// Turns the lights on in the towers.
pub fn light_the_windows(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut paints: ResMut<Assets<StandardMaterial>>,
    mut pane: Local<Option<(Handle<Mesh>, Handle<StandardMaterial>)>>,
    clock: Res<crate::sky::TimeOfDay>,
    year: Res<crate::season::TheYear>,
    mut tonight: Local<i64>,
    anchors: Query<&GlobalTransform, With<StreamAnchor>>,
    towers: Query<(Entity, &GlobalTransform, &crate::world::town::Standing)>,
    awake: Query<(Entity, &ChildOf), With<Awake>>,
) {
    let Some(anchor) = anchors.iter().next() else {
        return;
    };
    let here = anchor.translation();
    let up = crate::util::smoothstep(LIT_BELOW, FULLY_LIT_AT, clock.sun_height());

    // A DIFFERENT NIGHT IS A DIFFERENT PATTERN.
    //
    // The panes come down when the date turns, so the next night lights a different
    // set. Without this the pattern was a property of the building alone and a tower
    // wore the same windows for the life of the world.
    let night = crate::season::what_night_it_is(&year);
    if *tonight != night {
        *tonight = night;
        for (entity, _) in &awake {
            commands.entity(entity).despawn();
        }
        return;
    }

    // By day every window is off, and the panes come down rather than being left
    // black - a dark quad over the glass is worse than no quad at all.
    if up <= 0.0 {
        for (entity, _) in &awake {
            commands.entity(entity).despawn();
        }
        return;
    }

    let (mesh, paint) = pane
        .get_or_insert_with(|| {
            (
                meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
                paints.add(StandardMaterial {
                    base_color: INDOORS,
                    emissive: LinearRgba::from(INDOORS) * WINDOW_GLOWS,
                    unlit: true,
                    ..default()
                }),
            )
        })
        .clone();

    for (entity, at, standing) in &towers {
        let Some(storeys) = standing.what.storeys() else {
            continue;
        };
        if at.translation().distance(here) > AWAKE_WITHIN {
            continue;
        }
        if awake.iter().any(|(_, of)| of.parent() == entity) {
            continue;
        }

        let foot = at.translation();
        let footprint = standing.what.footprint();
        // Which storeys are in, from where the building STANDS - so a tower keeps
        // its own pattern however often it is streamed in and out.
        let seed = foot.x.to_bits() ^ foot.z.to_bits() ^ (night as u32).wrapping_mul(2_654_435_761);
        commands.entity(entity).with_children(|on| {
            for storey in 0..storeys {
                let roll = crate::world::town::unit(seed, storey as u32 * 31 + 7);
                if roll > AWAKE_SHARE {
                    continue;
                }
                // Where the light shows, which depends on what the wall is made of.
                //
                // A tower's facade is a curtain wall and a lit floor is a lit BAND
                // the width of it. A cottage has windows in a wall: lighting the
                // whole face of one would read as a building on fire, so it gets
                // panes the size of the windows that are actually there.
                let panes: Vec<(Vec3, Vec3)> = if standing.what.glazed_in_bands() {
                    let z = storey as f32 * FLOOR_TALL + FLOOR_TALL * 0.67;
                    let band = FLOOR_TALL * 0.66;
                    // ONE LIGHT AT A TIME, not the whole floor.
                    //
                    // Lighting the band lit a rectangle the width of the building,
                    // which reads as a floor with its lights on rather than as a
                    // room with somebody in it. The facade is divided into squares
                    // by its own mullions - see `curtain_wall` - so the lit pane is
                    // one of those squares.
                    let mut panes = Vec::new();
                    for (span, face) in [(footprint.x, true), (footprint.y, false)] {
                        let lights = ((span * 0.94 / band).round() as usize).max(2);
                        for light in 0..lights {
                            // Which of them is in, hashed per pane.
                            let roll = crate::world::town::unit(
                                seed,
                                (storey * 97 + light * 13 + usize::from(face) * 7) as u32,
                            );
                            if roll > 0.45 {
                                continue;
                            }
                            let over = -span * 0.47
                                + span * 0.94 * (light as f32 + 0.5) / lights as f32;
                            let wide = span * 0.94 / lights as f32 - 0.16;
                            if face {
                                panes.push((
                                    Vec3::new(wide, band - 0.16, 0.06),
                                    Vec3::new(over, z, footprint.y * 0.5 + 0.02),
                                ));
                            } else {
                                panes.push((
                                    Vec3::new(0.06, band - 0.16, wide),
                                    Vec3::new(footprint.x * 0.5 + 0.02, z, over),
                                ));
                            }
                        }
                    }
                    panes
                } else {
                    let z = storey as f32 * STOREY + PANE_UP;
                    // Two panes across the front and one down each flank - a lit
                    // room shows at whichever window somebody is standing near.
                    let mut panes = Vec::new();
                    for side in [-1.0_f32, 1.0] {
                        panes.push((
                            Vec3::new(PANE.x, PANE.y, 0.06),
                            Vec3::new(side * footprint.x * 0.24, z, -footprint.y * 0.5 - 0.04),
                        ));
                        panes.push((
                            Vec3::new(0.06, PANE.y, PANE.x),
                            Vec3::new(side * (footprint.x * 0.5 + 0.04), z, 0.0),
                        ));
                    }
                    panes
                };
                for (size, at) in panes {
                    on.spawn((
                        Awake,
                        Mesh3d(mesh.clone()),
                        MeshMaterial3d(paint.clone()),
                        Transform::from_translation(at).with_scale(size),
                        Visibility::default(),
                        bevy::pbr::NotShadowCaster,
                    ));
                }
            }
        });
    }
}

pub struct LampPlugin;

impl Plugin for LampPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (stand_the_lamps, light_them_at_night, light_the_windows)
                .chain()
                .run_if(crate::build::a_world_is_up),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The game hangs each light where Blender put the fitting's head.
    ///
    /// Two fittings at two heights, so one guess cannot serve both - and a point
    /// light at the wrong height is a glow floating beside a lamp, which reads as a
    /// bug rather than as a lamp.
    #[test]
    fn the_lamp_models_hang_their_light_where_the_game_thinks() {
        let note =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/models/lamp.txt");
        let said = std::fs::read_to_string(&note)
            .unwrap_or_else(|_| panic!("run dev/art/build.sh: {} is missing", note.display()));
        let read = |key: &str| -> f32 {
            said.lines()
                .find_map(|line| line.strip_prefix(&format!("{key} ")))
                .unwrap_or_else(|| panic!("{key} is not in {}", note.display()))
                .trim()
                .parse()
                .expect("a number")
        };
        for (key, ours) in [
            ("STREET_HEAD", crate::world::town::STREET_HEAD),
            ("POST_HEAD", crate::world::town::POST_HEAD),
            ("STREET_ARM", crate::world::town::STREET_ARM),
        ] {
            assert!(
                (read(key) - ours).abs() < 1.0e-3,
                "Blender hangs {key} at {:.2} m and the game lights it at {ours:.2}",
                read(key),
            );
        }
    }

    /// A different night lights a different set of windows.
    ///
    /// The pattern used to be a property of the BUILDING alone, so a tower wore the
    /// same windows for the life of the world - the sort of thing nobody notices
    /// once and everybody notices eventually.
    #[test]
    fn tonight_is_not_last_night() {
        let at = Vec2::new(1_234.0, -567.0);
        let seed = |night: i64| {
            at.x.to_bits() ^ at.y.to_bits() ^ (night as u32).wrapping_mul(2_654_435_761)
        };
        // The same building over a fortnight: how many of its storeys are in.
        let awake = |night: i64| {
            (0..15)
                .filter(|storey| {
                    crate::world::town::unit(seed(night), *storey as u32 * 31 + 7) <= AWAKE_SHARE
                })
                .collect::<Vec<_>>()
        };
        let first = awake(0);
        let same = (1..14).filter(|night| awake(*night) == first).count();
        assert!(
            same < 3,
            "{same} of the next thirteen nights light exactly the same storeys as the first",
        );
        // And it is the same building each night, not a different one.
        assert_eq!(awake(7), awake(7), "a night is not even consistent with itself");
    }

    /// They are dark by day and lit by night, and they cross over at dusk.
    #[test]
    fn the_lamps_come_on_at_night() {
        let up = |sun: f32| crate::util::smoothstep(LIT_BELOW, FULLY_LIT_AT, sun);
        assert_eq!(up(1.0), 0.0, "lamps burning at noon");
        assert_eq!(up(0.5), 0.0, "lamps burning in the afternoon");
        assert_eq!(up(-0.6), 1.0, "lamps out at midnight");
        let dusk = up(0.06);
        assert!(
            dusk > 0.0 && dusk < 1.0,
            "lamps snap on rather than coming up through dusk: {dusk}",
        );
    }
}
