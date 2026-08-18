//! The warden: a placeholder body and a ground-following character controller.
//!
//! The body is built from primitives at correct human scale (~1.8 m tall). That
//! matters more than it looks like it should — without something of a known
//! size standing on it, there is no way to judge whether a hill is a hill or a
//! mountain, or whether the map is too big to walk across. When real art lands
//! in `assets/models/`, `spawn_player` is the one place that changes.
//!
//! Movement is camera-relative (push forward and you go where you're looking)
//! and the warden is snapped to the terrain height each frame rather than
//! simulated with physics — the world has no colliders yet, and the heightfield
//! is an exact answer for "where is the ground".

use bevy::prelude::*;

use crate::camera::{CameraMode, MainCamera};
use crate::config::{RANCH_AT, SEA_LEVEL};
use crate::shade::{shaded, Shaded};
use crate::states::AppState;
use crate::util::facing_quat;
use crate::world::terrain::TerrainSource;
use crate::world::WorldBounds;

/// Jogging speed in m/s. A brisk-but-believable pace, so the time it takes to
/// cross the map is an honest signal about whether the map is the right size.
const WALK_SPEED: f32 = 7.0;
const SPRINT_SPEED: f32 = 15.0;
/// How fast the warden swivels to face the way they're heading, in radians/sec.
const TURN_RATE: f32 = 12.0;
/// Standing eye-to-toe height, used to keep the body clear of the ground.
const LEG_HEIGHT: f32 = 0.9;
/// How deep the warden may wade, in metres below sea level.
///
/// The sea is not walkable in the base game — it is for boats. This is both how
/// far they can stand into it and how far they can *walk* into it: one number,
/// so the depth they are held at and the depth they are turned back at can never
/// disagree and leave them bobbing at a line they cannot cross.
const WADE_DEPTH: f32 = 1.4;

#[derive(Component)]
pub struct Player;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_player)
            // The warden only walks in the game. In the terrain tool the same
            // keys fly the camera, and in the menu nothing should move at all.
            .add_systems(
                Update,
                move_player.run_if(in_state(AppState::Playing)),
            );
    }
}

/// Picks somewhere sensible to start: the first patch of gentle, dry, low-lying
/// ground found spiralling out from the middle of the map. The map's center is
/// as likely to be open sea as anything else, so this can't just be the origin.
fn find_spawn(terrain: &TerrainSource, bounds: &WorldBounds) -> Vec3 {
    const RING_STEP: f32 = 48.0;
    let max_radius = bounds.half.length();

    let mut radius = 0.0;
    while radius < max_radius {
        // More samples the further out we go, so ring coverage stays even.
        let samples = ((radius / RING_STEP).ceil() as usize * 6).max(1);
        for i in 0..samples {
            let angle = i as f32 / samples as f32 * std::f32::consts::TAU;
            let p = Vec2::new(angle.cos(), angle.sin()) * radius;
            let height = terrain.height(p.x, p.y);
            // Clear of the tide, below the treeline, and flat enough to stand on.
            if height > 4.0 && height < 70.0 && terrain.normal(p.x, p.y, 2.0).y > 0.93 {
                return Vec3::new(p.x, height, p.y);
            }
        }
        radius += RING_STEP;
    }

    warn!("no suitable spawn found — dropping the warden at the origin");
    Vec3::new(0.0, terrain.height(0.0, 0.0), 0.0)
}

fn spawn_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<Shaded>>,
    terrain: Res<TerrainSource>,
    bounds: Res<WorldBounds>,
) {
    // On the ranch, which is where the game begins. `find_spawn` is kept as the
    // fallback for a world whose map does not put land there — a redrawn map
    // could leave the pinned spot at sea, and dropping the warden into the water
    // with no explanation is worse than starting them somewhere arbitrary.
    let ranch = Vec2::new(RANCH_AT.0, RANCH_AT.1);
    let on_land = terrain.height(ranch.x, ranch.y) > SEA_LEVEL + 1.0;
    let spawn = if on_land {
        Vec3::new(ranch.x, terrain.height(ranch.x, ranch.y), ranch.y)
    } else {
        warn!("the ranch at {:.0}, {:.0} is under water on this map", ranch.x, ranch.y);
        find_spawn(&terrain, &bounds)
    };
    info!("warden spawning at {:.0}, {:.0}", spawn.x, spawn.z);

    let mut solid = |r: f32, g: f32, b: f32| {
        materials.add(shaded(StandardMaterial {
            base_color: Srgba::rgb(r, g, b).into(),
            perceptual_roughness: 0.8,
            ..default()
        }))
    };
    let coat = solid(0.22, 0.34, 0.24);
    let skin = solid(0.80, 0.62, 0.48);
    let hat = solid(0.18, 0.42, 0.22);

    // Parent holds the warden's world position with its origin at the feet;
    // the body parts hang off it at fixed local heights.
    commands
        .spawn((
            Player,
            // Pushes the grass aside as they go. About the width of a person
            // plus an arm — what actually brushes past is wider than what walks.
            crate::shade::Wades { reach: 1.8 },
            Transform::from_translation(spawn),
            Visibility::default(),
        ))
        .with_children(|parent| {
            // Legs
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.46, LEG_HEIGHT, 0.30))),
                MeshMaterial3d(coat.clone()),
                Transform::from_xyz(0.0, LEG_HEIGHT * 0.5, 0.0),
            ));
            // Torso
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.56, 0.62, 0.34))),
                MeshMaterial3d(coat),
                Transform::from_xyz(0.0, 1.21, 0.0),
            ));
            // Head
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.30, 0.30, 0.28))),
                MeshMaterial3d(skin),
                Transform::from_xyz(0.0, 1.67, 0.0),
            ));
            // Hat crown and brim — the warden's silhouette, and a clear read on
            // which way they're facing from any camera angle.
            parent.spawn((
                Mesh3d(meshes.add(Cylinder::new(0.18, 0.16))),
                MeshMaterial3d(hat.clone()),
                Transform::from_xyz(0.0, 1.90, 0.0),
            ));
            parent.spawn((
                Mesh3d(meshes.add(Cylinder::new(0.34, 0.03))),
                MeshMaterial3d(hat),
                Transform::from_xyz(0.0, 1.83, 0.0),
            ));
        });
}

pub fn move_player(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mode: Res<CameraMode>,
    terrain: Res<TerrainSource>,
    bounds: Res<WorldBounds>,
    cameras: Query<&Transform, (With<MainCamera>, Without<Player>)>,
    mut players: Query<&mut Transform, With<Player>>,
) {
    // In free-fly the same keys drive the camera instead.
    if *mode == CameraMode::Fly {
        return;
    }
    let (Some(camera), Ok(mut transform)) = (cameras.iter().next(), players.single_mut()) else {
        return;
    };

    // Movement is relative to where the camera is pointing, flattened onto the
    // ground plane so looking up or down never changes how fast you travel.
    let forward = camera.forward().as_vec3();
    let forward = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
    let right = Vec3::new(-forward.z, 0.0, forward.x);

    let mut input = Vec3::ZERO;
    if keys.any_pressed([KeyCode::KeyW, KeyCode::ArrowUp]) {
        input += forward;
    }
    if keys.any_pressed([KeyCode::KeyS, KeyCode::ArrowDown]) {
        input -= forward;
    }
    if keys.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]) {
        input += right;
    }
    if keys.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]) {
        input -= right;
    }

    let direction = input.normalize_or_zero();
    if direction != Vec3::ZERO {
        let speed = if keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]) {
            SPRINT_SPEED
        } else {
            WALK_SPEED
        };
        let next = transform.translation + direction * speed * time.delta_secs();
        let next = bounds.clamp(next, 2.0);

        // The sea is for boats. Rather than an invisible wall at the waterline —
        // which reads as a bug, and stops you paddling at a beach at all — the
        // warden wades until the water is about knee-to-waist and is then turned
        // back by it. Only the step INTO deep water is refused, so someone who
        // somehow ends up out there can always walk home.
        let depth = SEA_LEVEL - terrain.height(next.x, next.z);
        let here = SEA_LEVEL - terrain.height(transform.translation.x, transform.translation.z);
        if depth <= WADE_DEPTH || depth < here {
            transform.translation = next;
        }

        // Ease into the new facing instead of snapping, so quick direction
        // changes read as a turn rather than a teleport.
        if let Some(target) = facing_quat(direction) {
            let t = (TURN_RATE * time.delta_secs()).min(1.0);
            transform.rotation = transform.rotation.slerp(target, t);
        }
    }

    // Plant the feet on the ground every frame, including when standing still,
    // so the warden settles correctly the moment the world finishes loading.
    let ground = terrain.height(transform.translation.x, transform.translation.z);
    transform.translation.y = ground.max(SEA_LEVEL - WADE_DEPTH);
}
