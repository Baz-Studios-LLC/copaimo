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

/// How bright a lamp burns, and how far its light carries.
///
/// Warm, because every other light in this world is daylight or a grey overcast, and
/// the one thing a lamp has to do is feel like a different KIND of light from the
/// sky. Not so bright that a street at night stops being night.
const BURNS: f32 = 120_000.0;
const CARRIES: f32 = 22.0;
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
}

/// The light hung on the lamp this is a child of.
#[derive(Component)]
struct Lit;

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
            let ground = terrain.0.drawn_height(lamp.at.x, lamp.at.y);
            commands.spawn((
                Lamp { head: lamp.head },
                crate::world::town::FromSite(*key),
                SceneRoot(assets.load(GltfAssetLabel::Scene(0).from_asset(
                    if lamp.head > crate::world::town::POST_HEAD + 0.5 {
                        "models/lamp_street.glb"
                    } else {
                        "models/lamp_post.glb"
                    },
                ))),
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
    clock: Res<crate::sky::TimeOfDay>,
    anchors: Query<&GlobalTransform, With<StreamAnchor>>,
    lamps: Query<(Entity, &GlobalTransform, &Lamp)>,
    mut lit: Query<(Entity, &ChildOf, &mut PointLight), With<Lit>>,
) {
    let Some(anchor) = anchors.iter().next() else {
        return;
    };
    let here = anchor.translation();

    // How far up the lamps are, nought by day and one after dusk.
    let up = crate::util::smoothstep(LIT_BELOW, FULLY_LIT_AT, clock.sun_height());

    // The nearest few, which are the ones anybody is going to look at.
    let mut near: Vec<(f32, Entity, f32)> = lamps
        .iter()
        .map(|(entity, at, lamp)| (at.translation().distance(here), entity, lamp.head))
        .filter(|(away, _, _)| *away < LIT_WITHIN)
        .collect();
    near.sort_by(|a, b| a.0.total_cmp(&b.0));
    near.truncate(MOST_LIT);

    // Put out anything that is no longer in that list, and dim everything by day
    // rather than tearing the lights down and building them again at dusk.
    for (entity, of, mut light) in &mut lit {
        if near.iter().any(|(_, lamp, _)| *lamp == of.parent()) {
            light.intensity = BURNS * up;
        } else {
            commands.entity(entity).despawn();
        }
    }

    if up <= 0.0 {
        return;
    }
    for (_, lamp, head) in near {
        if lit.iter().any(|(_, of, _)| of.parent() == lamp) {
            continue;
        }
        commands.entity(lamp).with_children(|on| {
            on.spawn((
                Lit,
                PointLight {
                    color: LAMPLIGHT,
                    intensity: BURNS * up,
                    range: CARRIES,
                    // A hundred and change of these and shadows are the whole frame
                    // budget. The fitting casts none; what it lights does.
                    shadows_enabled: false,
                    ..default()
                },
                Transform::from_xyz(0.0, head, 0.0),
            ));
        });
    }
}

pub struct LampPlugin;

impl Plugin for LampPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (stand_the_lamps, light_them_at_night)
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
        ] {
            assert!(
                (read(key) - ours).abs() < 1.0e-3,
                "Blender hangs {key} at {:.2} m and the game lights it at {ours:.2}",
                read(key),
            );
        }
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
