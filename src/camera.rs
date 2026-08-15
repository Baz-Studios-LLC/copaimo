//! The camera: a third-person orbit rig that follows the ranger, plus a free-fly
//! mode for looking at the world itself.
//!
//! Free-fly exists because this stage of the project is about the *map*. Being
//! able to lift off and read a coastline or a mountain range from above is the
//! difference between tuning the world and guessing at it. Press `F` to detach.
//!
//! The camera also carries the `StreamAnchor`, so terrain loads around wherever
//! the viewer actually is — which keeps free-fly honest, showing real streamed
//! ground rather than a special case.

use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll};
use bevy::prelude::*;

use crate::player::{move_player, Player};
use crate::states::AppState;
use crate::world::terrain::TerrainSource;
use crate::world::StreamAnchor;

const MOUSE_SENSITIVITY: f32 = 0.0035;
/// Stop just short of straight up/down so the view never flips over.
const PITCH_LIMIT: f32 = 1.45;
const MIN_DISTANCE: f32 = 3.0;
const MAX_DISTANCE: f32 = 45.0;
const ZOOM_SPEED: f32 = 2.5;
/// Height on the ranger the camera aims at — roughly the shoulders, so the
/// horizon sits where you'd expect rather than at their feet.
const LOOK_HEIGHT: f32 = 1.5;
/// How quickly the camera catches up to its ideal position. Higher is tighter.
const FOLLOW_STIFFNESS: f32 = 14.0;
/// Minimum clearance the camera keeps above the ground, so backing into a hill
/// pushes it up instead of burying it in the terrain.
const GROUND_CLEARANCE: f32 = 2.0;

const FLY_SPEED: f32 = 70.0;
const FLY_BOOST: f32 = 5.0;

#[derive(Component)]
pub struct MainCamera;

#[derive(Resource, Default, Clone, Copy, PartialEq, Eq)]
pub enum CameraMode {
    #[default]
    Follow,
    Fly,
}

/// Where the camera is looking and how far back it sits.
#[derive(Resource)]
pub struct Orbit {
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
}

impl Default for Orbit {
    fn default() -> Self {
        Self {
            yaw: 0.0,
            // Angled down a little: enough to see the ground ahead without
            // losing the horizon.
            pitch: -0.32,
            distance: 12.0,
        }
    }
}

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraMode>()
            .init_resource::<Orbit>()
            .add_systems(Startup, spawn_camera)
            .add_systems(
                Update,
                (
                    toggle_modes,
                    orbit_input,
                    // Runs after the ranger has moved this frame, so the camera
                    // never trails a frame behind them.
                    drive_camera.after(move_player),
                )
                    // Frozen in the menu: the cursor is released there, and
                    // moving the mouse toward a button must not swing the view.
                    .run_if(not(in_state(AppState::Menu))),
            );
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        MainCamera,
        StreamAnchor,
        Camera3d::default(),
        Camera {
            hdr: true,
            ..default()
        },
        Msaa::Sample4,
        // No distance fog. It used to hide the streaming boundary, but haze
        // over the whole view is the wrong trade when the point is reading the
        // shape of the land — see the note on `VIEW_CHUNKS` in config.rs.
        Transform::from_xyz(0.0, 40.0, 30.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn toggle_modes(keys: Res<ButtonInput<KeyCode>>, mut mode: ResMut<CameraMode>) {
    if keys.just_pressed(KeyCode::KeyF) {
        *mode = match *mode {
            CameraMode::Follow => CameraMode::Fly,
            CameraMode::Fly => CameraMode::Follow,
        };
    }
}

fn orbit_input(
    motion: Res<AccumulatedMouseMotion>,
    scroll: Res<AccumulatedMouseScroll>,
    mut orbit: ResMut<Orbit>,
) {
    if motion.delta != Vec2::ZERO {
        orbit.yaw -= motion.delta.x * MOUSE_SENSITIVITY;
        orbit.pitch = (orbit.pitch - motion.delta.y * MOUSE_SENSITIVITY)
            .clamp(-PITCH_LIMIT, PITCH_LIMIT);
    }

    // Notches rather than raw delta: a trackpad reports a whole flick in pixels,
    // which as a raw number throws the camera to one end of its range and back.
    let notches = crate::util::wheel_notches(&scroll);
    if notches != 0.0 {
        orbit.distance = (orbit.distance - notches * ZOOM_SPEED).clamp(MIN_DISTANCE, MAX_DISTANCE);
    }
}

fn drive_camera(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mode: Res<CameraMode>,
    orbit: Res<Orbit>,
    terrain: Res<TerrainSource>,
    players: Query<&Transform, (With<Player>, Without<MainCamera>)>,
    mut cameras: Query<&mut Transform, With<MainCamera>>,
) {
    let Some(mut camera) = cameras.iter_mut().next() else {
        return;
    };

    let rotation = Quat::from_euler(EulerRot::YXZ, orbit.yaw, orbit.pitch, 0.0);
    // The direction the rig points; the camera sits back along it.
    let back = rotation * Vec3::Z;

    match *mode {
        CameraMode::Follow => {
            let Some(player) = players.iter().next() else {
                return;
            };
            let focus = player.translation + Vec3::Y * LOOK_HEIGHT;
            let mut desired = focus + back * orbit.distance;

            // Never let the camera sink into a hillside behind the player.
            let ground = terrain.height(desired.x, desired.z) + GROUND_CLEARANCE;
            desired.y = desired.y.max(ground);

            // Frame-rate independent exponential smoothing: the camera covers
            // the same fraction of the remaining gap per second regardless of
            // how often we tick.
            let t = 1.0 - (-FOLLOW_STIFFNESS * time.delta_secs()).exp();
            camera.translation = camera.translation.lerp(desired, t);
            camera.look_at(focus, Vec3::Y);
        }
        CameraMode::Fly => {
            let forward = -back;
            let right = rotation * Vec3::X;

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
            if keys.pressed(KeyCode::Space) {
                input += Vec3::Y;
            }
            if keys.pressed(KeyCode::ControlLeft) {
                input -= Vec3::Y;
            }

            let boost = if keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]) {
                FLY_BOOST
            } else {
                1.0
            };
            camera.translation += input.normalize_or_zero() * FLY_SPEED * boost * time.delta_secs();
            camera.rotation = rotation;
        }
    }
}
