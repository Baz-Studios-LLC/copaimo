//! The camera: a third-person orbit rig that follows the warden, plus a free-fly
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

use crate::config::SEA_LEVEL;
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
/// Height on the warden the camera aims at — roughly the shoulders, so the
/// horizon sits where you'd expect rather than at their feet.
const LOOK_HEIGHT: f32 = 1.5;
/// How quickly the camera catches up to its ideal position. Higher is tighter.
const FOLLOW_STIFFNESS: f32 = 14.0;
/// Minimum clearance the camera keeps above the ground, so backing into a hill
/// pushes it up instead of burying it in the terrain.
const GROUND_CLEARANCE: f32 = 2.0;

const FLY_SPEED: f32 = 70.0;
const FLY_BOOST: f32 = 5.0;
/// How far the fly speed can be wound either side of its default.
const MIN_FLY_SCALE: f32 = 0.15;
const MAX_FLY_SCALE: f32 = 12.0;
const FLY_SCALE_STEP: f32 = 1.3;

/// How far above the ground the free-fly camera is held, in metres.
///
/// Enough to clear the grass and the litter standing in it, so skimming a
/// hillside does not put the view inside a boulder. Not so much that you cannot
/// get down among the trees to look at them.
///
/// Two metres, because the grass reaches 1.66 — which I had guessed at 1.6 and
/// the test caught. Tall grass grew twice in one evening and will grow again, so
/// the guard reads the crate's own answer rather than a number written here.
const FLY_CLEARANCE: f32 = 2.0;

/// A standing multiplier on how fast free-fly moves.
///
/// An 8 km world is crossed at two speeds and neither is one number: picking
/// over a coastline wants metres a second, getting to the far continent wants
/// hundreds. Shift boosts for a moment; this is the setting you leave alone.
#[derive(Resource, Deref)]
pub struct FlySpeed(pub f32);

impl Default for FlySpeed {
    fn default() -> Self {
        Self(1.0)
    }
}

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
            .init_resource::<FlySpeed>()
            .init_resource::<Orbit>()
            .add_systems(Startup, spawn_camera)
            // On foot, every time play begins.
            //
            // The terrain tool sets free-fly when it opens and nothing put it
            // back, so a maker who had opened it once was flying for the rest of
            // the session — every New Game and every Continue after that started
            // in the air with the warden left behind, which reads as the tool
            // never having closed.
            //
            // A state that changes a global setting has to answer for it. This is
            // the answer: whatever the last mode was, playing starts on foot, and
            // F is still there for anybody who wants to leave the ground.
            .add_systems(OnEnter(AppState::Playing), start_on_foot)
            .add_systems(
                Update,
                (
                    set_fly_speed,
                    orbit_input,
                    // Runs after the warden has moved this frame, so the camera
                    // never trails a frame behind them.
                    drive_camera.after(move_player),
                )
                    // Frozen wherever the world is not what you are looking at:
                    // the menu, and the workbench.
                    //
                    // The bench had this camera running behind its own, which is
                    // most of why it was unusable — W drove the world camera three
                    // kilometres away while the bench thought it was nudging a
                    // cursor, and the ray that aims the cursor was cast from
                    // whichever of the two cameras came back first.
                    .run_if(in_world),
            );
        // The maker's flight toggle, only in a maker's build — see `toggle_modes`.
        #[cfg(feature = "tools")]
        app.add_systems(Update, toggle_modes.run_if(in_world));
    }
}

/// Whether the world is the thing on screen.
fn in_world(state: Res<State<AppState>>) -> bool {
    #[cfg(feature = "tools")]
    {
        matches!(state.get(), AppState::Playing | AppState::Editing)
    }
    #[cfg(not(feature = "tools"))]
    {
        matches!(state.get(), AppState::Playing)
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

fn start_on_foot(mut mode: ResMut<CameraMode>) {
    *mode = CameraMode::Follow;
}

/// F swaps between following the warden and flying free.
///
/// A MAKER'S control, compiled out of releases with the rest of the tools: it
/// exists for reading the map from above, and a player who can fly across the
/// world at seventy metres a second with the warden left standing is a player
/// the game's whole geography stops meaning anything to. The terrain tool still
/// gets Fly by construction — `enter_editor` sets the mode itself.
#[cfg(feature = "tools")]
fn toggle_modes(keys: Res<ButtonInput<KeyCode>>, mut mode: ResMut<CameraMode>) {
    if keys.just_pressed(KeyCode::KeyF) {
        *mode = match *mode {
            CameraMode::Follow => CameraMode::Fly,
            CameraMode::Fly => CameraMode::Follow,
        };
    }
}

fn set_fly_speed(keys: Res<ButtonInput<KeyCode>>, mut speed: ResMut<FlySpeed>) {
    // Proportional, like the brush radius: winding it up feels the same whether
    // you are at a crawl or crossing an ocean.
    if keys.just_pressed(KeyCode::Equal) {
        speed.0 = (speed.0 * FLY_SCALE_STEP).min(MAX_FLY_SCALE);
    }
    if keys.just_pressed(KeyCode::Minus) {
        speed.0 = (speed.0 / FLY_SCALE_STEP).max(MIN_FLY_SCALE);
    }
}

fn orbit_input(
    motion: Res<AccumulatedMouseMotion>,
    scroll: Res<AccumulatedMouseScroll>,
    #[cfg(feature = "tools")] free: Option<Res<crate::editor::CursorFree>>,
    mut orbit: ResMut<Orbit>,
) {
    // The pointer has been let go to reach a panel. Moving it there must not
    // swing the view, exactly as in the menu.
    //
    // Only the tools ever let it go, so in a player's build there is no panel to
    // reach for and nothing to ask.
    #[cfg(feature = "tools")]
    if free.is_some_and(|free| free.0) {
        return;
    }
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
    speed: Res<FlySpeed>,
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
            if keys.any_pressed([KeyCode::Space, KeyCode::KeyE]) {
                input += Vec3::Y;
            }
            // Q rather than Ctrl. Ctrl is half the terrain tool's shortcuts —
            // Ctrl+S, Ctrl+Z, Ctrl+Y — so descending on it meant the camera
            // dropped a little every time the ground was saved.
            if keys.pressed(KeyCode::KeyQ) {
                input -= Vec3::Y;
            }

            let boost = if keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]) {
                FLY_BOOST
            } else {
                1.0
            };
            camera.translation +=
                input.normalize_or_zero() * FLY_SPEED * speed.0 * boost * time.delta_secs();

            // And never below the ground.
            //
            // Under the map is not a place. Everything down there is drawn from
            // the wrong side — the world is a single surface with no underside, so
            // a camera beneath it sees the backs of hills, the sea from inside,
            // and chunks that stream in and out for no visible reason. It is the
            // easiest way to make the world look broken and the hardest to
            // realise you have done it, because nothing about the view says
            // "you are underneath".
            //
            // Held above the DRAWN height rather than the true one, so the floor
            // is the surface actually on screen. The tide moves, so the sea is
            // taken at its own level: skimming the water is fine, being inside it
            // looking up is not.
            let floor = terrain
                .drawn_height(camera.translation.x, camera.translation.z)
                .max(SEA_LEVEL)
                + FLY_CLEARANCE;
            camera.translation.y = camera.translation.y.max(floor);
            camera.rotation = rotation;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // The second half compares two constants and clippy is right that it does.
    // It is kept because it guards a RELATIONSHIP that is easy to break while
    // tuning one of them — the same reason the night's light levels are checked
    // against each other.
    #[allow(clippy::assertions_on_constants)]
    fn the_fly_floor_clears_what_stands_on_the_ground() {
        // Under the map is not a place: the world is a single surface with no
        // underside, so a camera beneath it sees the backs of hills and the sea
        // from inside. What makes that worth a guard rather than a note is that
        // nothing about the view says "you are underneath" — it just looks broken.
        //
        // The clearance has to beat the tallest thing a camera can skim, or
        // hugging a hillside puts the view inside a boulder.
        let tallest_tuft = terrain_core::cover::tallest();
        assert!(
            FLY_CLEARANCE > tallest_tuft,
            "flying at {FLY_CLEARANCE} m sits inside grass {tallest_tuft:.2} m tall"
        );
        // And low enough to get down among the trees to look at them.
        assert!(FLY_CLEARANCE < 4.0, "the floor is too high to see anything from");
    }
}
