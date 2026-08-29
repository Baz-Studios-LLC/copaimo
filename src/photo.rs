//! Flies the game to a place, waits for it to load, photographs it, and quits.
//!
//! # Why this exists
//!
//! "There has to be a way you can drive the game and actually see things yourself."
//!
//! There was not, and the cost of that shows all over this project's history. Every
//! fault in the towns — a post through every doorway, dressings built inside the
//! rooms, roads that never appeared, a landmark standing on the spawn point — was
//! found by a person looking at their screen and reported back, while every
//! measurement I could take said the thing was correct. The measurements were
//! correct. They were of the arithmetic, and the arithmetic was never what was
//! wrong.
//!
//! Blender renders told me what a MODEL looks like. They cannot tell me what the
//! GAME looks like: whether the streets drew, whether the buildings landed where the
//! layout said, whether the thing is lit, whether you can get through the door. That
//! gap is where every one of those bugs lived.
//!
//! So: `copaimo --photo x,z` starts the real game, puts the camera where it is told,
//! waits until the world around it has actually streamed in, writes a PNG and exits.
//! Same binary, same assets, same systems — a photograph of the game rather than a
//! render of its parts.
//!
//! ```text
//! copaimo --photo -4596,988                 # look at the ranch
//! copaimo --photo 1320,-85 --height 90      # a city, from up high
//! copaimo --photo 0,0 --out shots/here.png
//! ```
//!
//! It is a development tool and costs the game nothing: with no `--photo` on the
//! command line every system here is skipped by a run condition.

use std::path::PathBuf;

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};

/// What was asked for on the command line.
#[derive(Resource, Clone)]
pub struct Photo {
    /// Where to stand, in world metres.
    pub at: Vec2,
    /// How far above the ground to put the eye.
    pub height: f32,
    /// How far back from `at` the camera sits. Zero looks straight down.
    pub back: f32,
    /// Where the file goes.
    pub out: PathBuf,
    /// How many frames to let the world stream before the shutter.
    pub settle: u32,
}

/// Frames counted since the world came up.
#[derive(Resource, Default)]
pub struct Waiting(pub u32);

impl Photo {
    /// Reads `--photo` and friends from the command line, if they are there.
    ///
    /// Hand-parsed rather than with a crate: four arguments do not justify a
    /// dependency the shipped game would carry.
    pub fn asked_for() -> Option<Photo> {
        let args: Vec<String> = std::env::args().collect();
        let value = |name: &str| -> Option<String> {
            args.iter()
                .position(|a| a == name)
                .and_then(|at| args.get(at + 1))
                .cloned()
        };

        let spot = value("--photo")?;
        let (x, z) = spot.split_once(',')?;
        let at = Vec2::new(x.trim().parse().ok()?, z.trim().parse().ok()?);

        Some(Photo {
            at,
            height: value("--height").and_then(|v| v.parse().ok()).unwrap_or(28.0),
            back: value("--back").and_then(|v| v.parse().ok()).unwrap_or(46.0),
            out: value("--out")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("dev/art/shots/game.png")),
            settle: value("--settle").and_then(|v| v.parse().ok()).unwrap_or(240),
        })
    }
}

/// True when a photograph was asked for.
pub fn taking_a_photo(photo: Option<Res<Photo>>) -> bool {
    photo.is_some()
}

/// Puts the camera where it was told, every frame.
///
/// Every frame rather than once, because the game's own camera systems will happily
/// move it back — this is a viewpoint imposed on a running game, not a replacement
/// for its camera.
pub fn stand_where_told(
    photo: Res<Photo>,
    terrain: Option<Res<crate::world::terrain::TerrainSource>>,
    mut cameras: Query<&mut Transform, With<Camera3d>>,
) {
    let ground = terrain
        .map(|t| t.0.height(photo.at.x, photo.at.y))
        .unwrap_or(0.0);
    let aim = Vec3::new(photo.at.x, ground + 2.0, photo.at.y);
    let eye = aim + Vec3::new(0.0, photo.height, photo.back);
    for mut place in &mut cameras {
        place.translation = eye;
        place.look_at(aim, Vec3::Y);
    }
}

/// Puts the WARDEN at the spot too.
///
/// The camera override alone was not enough and the first photographs proved it:
/// they came out looking at open sea from the ranch, because the game's own camera
/// follows the player and runs after this did - so the view snapped back every
/// frame. Moving the player means the game's camera arrives at the right place by
/// itself, and the override below only has to choose the angle.
pub fn stand_the_warden_there(
    photo: Res<Photo>,
    terrain: Option<Res<crate::world::terrain::TerrainSource>>,
    mut wardens: Query<&mut Transform, With<crate::player::Player>>,
) {
    let ground = terrain
        .map(|t| t.0.height(photo.at.x, photo.at.y))
        .unwrap_or(0.0);
    for mut place in &mut wardens {
        place.translation = Vec3::new(photo.at.x, ground, photo.at.y);
    }
}

/// Keeps the streaming anchor at the spot, so the world loads AROUND the camera.
///
/// Without this the chunks, the cover and the settlements all load around wherever
/// the warden happens to be, and the photograph is of an empty green plain with the
/// town two kilometres behind it.
pub fn anchor_where_told(
    photo: Res<Photo>,
    terrain: Option<Res<crate::world::terrain::TerrainSource>>,
    mut anchors: Query<
        (&mut Transform, &mut GlobalTransform),
        With<crate::world::StreamAnchor>,
    >,
) {
    let ground = terrain
        .map(|t| t.0.height(photo.at.x, photo.at.y))
        .unwrap_or(0.0);
    let at = Vec3::new(photo.at.x, ground, photo.at.y);
    for (mut place, mut world) in &mut anchors {
        place.translation = at;
        *world = GlobalTransform::from(*place);
    }
}

/// Waits for the world to arrive, takes the picture, and quits.
pub fn take_the_photo(
    mut commands: Commands,
    photo: Res<Photo>,
    mut waiting: ResMut<Waiting>,
    mut done: Local<bool>,
    mut quit: EventWriter<AppExit>,
) {
    if *done {
        // A frame or two after the shutter, so the file is written before the
        // process goes away.
        waiting.0 += 1;
        if waiting.0 > photo.settle + 30 {
            quit.write(AppExit::Success);
        }
        return;
    }

    waiting.0 += 1;
    if waiting.0 < photo.settle {
        return;
    }

    if let Some(folder) = photo.out.parent() {
        let _ = std::fs::create_dir_all(folder);
    }
    let out = photo.out.clone();
    info!(
        "photographing {:.0}, {:.0} into {}",
        photo.at.x,
        photo.at.y,
        out.display()
    );
    commands.spawn(Screenshot::primary_window()).observe(save_to_disk(out));
    *done = true;
}

/// Starts the game rather than the menu, when a photograph was asked for.
///
/// Without this the whole module sits behind `in_state(Playing)` and never runs -
/// the game opens on its menu and waits for a person, which is exactly what a
/// photograph is meant to avoid needing.
fn start_playing(mut next: ResMut<NextState<crate::states::AppState>>) {
    next.set(crate::states::AppState::Playing);
}

pub struct PhotoPlugin;

impl Plugin for PhotoPlugin {
    fn build(&self, app: &mut App) {
        let Some(photo) = Photo::asked_for() else {
            return;
        };
        app.insert_resource(photo)
            .init_resource::<Waiting>()
            .add_systems(Startup, start_playing)
            .add_systems(
                Update,
                (stand_the_warden_there, anchor_where_told, take_the_photo)
                    .run_if(taking_a_photo)
                    .run_if(in_state(crate::states::AppState::Playing)),
            )
            // The camera LAST, after everything that moves cameras - otherwise the
            // game's own follow-cam wins and the photograph is of wherever the
            // warden happened to be looking.
            .add_systems(
                PostUpdate,
                stand_where_told
                    .run_if(taking_a_photo)
                    .run_if(in_state(crate::states::AppState::Playing))
                    .before(bevy::transform::TransformSystem::TransformPropagate),
            );
    }
}
