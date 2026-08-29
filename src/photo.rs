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

/// One named viewpoint.
#[derive(Clone)]
pub struct Shot {
    /// What the file is called, without the extension.
    pub name: String,
    /// What the camera looks AT, in world metres.
    pub at: Vec2,
    /// Which way the camera sits FROM that point, as a unit vector on the ground.
    ///
    /// The single-shot `--photo` route leaves this at `+Z`, which is what it always
    /// did. A matrix shot needs it: "the village entrance" means standing outside
    /// the gate looking in, and which way that is depends on where the road comes
    /// from, not on which way the world's Z axis happens to run.
    pub from: Vec2,
    /// How far above the ground to put the eye.
    pub height: f32,
    /// How far back from `at` the camera sits. Zero looks straight down.
    pub back: f32,
}

/// What was asked for on the command line.
#[derive(Resource, Clone)]
pub struct Photo {
    /// The viewpoints to take, in order.
    ///
    /// One for `--photo`. For `--matrix` this starts empty and is filled from the
    /// world once the settlements exist - a named viewpoint is "the entrance to the
    /// nearest village", which nothing can work out at the time argv is read.
    pub shots: Vec<Shot>,
    /// The folder a matrix goes into, if this is one.
    pub matrix: Option<PathBuf>,
    /// Where a single photograph goes.
    pub out: PathBuf,
    /// How many frames to let the world stream before the shutter.
    pub settle: u32,
    /// Whether to pull the map up before the shutter goes.
    pub map: bool,
    /// Whether to let the world's own clock and weather run.
    ///
    /// Off by default, and that is the point. A photograph is EVIDENCE, and two
    /// photographs of the same place are only comparable if the only thing that
    /// changed between them is the thing being judged. Left to itself the game
    /// follows the real clock and rolls its own weather, so the matrix came back
    /// rainy and overcast one run and bright the next - and every difference in
    /// haze, cloud, shadow length and rain streaks reads as a change to whatever
    /// was being reviewed.
    ///
    /// `--live` puts the clock and the weather back, for the times when the weather
    /// IS the subject.
    pub live: bool,
    /// The hour to hold, when something other than noon is the subject.
    ///
    /// `--hour 22` for a night shot. Still frozen - a chosen hour is as repeatable
    /// as the default one, which is the whole point.
    pub hour: Option<f32>,
}

impl Photo {
    /// The viewpoint being taken right now.
    pub fn shot(&self, taking: &Taking) -> Option<&Shot> {
        self.shots.get(taking.at)
    }
}

/// Where the run has got to.
#[derive(Resource, Default)]
pub struct Taking {
    /// Which shot is being taken.
    pub at: usize,
    /// Frames waited on it.
    pub waited: u32,
    /// Whether its shutter has already gone.
    pub taken: bool,
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

        let settle = value("--settle").and_then(|v| v.parse().ok()).unwrap_or(240);
        let map = args.iter().any(|arg| arg == "--map");
        let live = args.iter().any(|arg| arg == "--live");
        let hour = value("--hour").and_then(|v| v.parse().ok());

        // A whole matrix, filled in from the world once it exists.
        if let Some(folder) = value("--matrix") {
            return Some(Photo {
                shots: Vec::new(),
                matrix: Some(PathBuf::from(folder)),
                out: PathBuf::new(),
                settle,
                map,
                live,
                hour,
            });
        }

        let spot = value("--photo")?;
        let (x, z) = spot.split_once(',')?;
        let at = Vec2::new(x.trim().parse().ok()?, z.trim().parse().ok()?);

        Some(Photo {
            shots: vec![Shot {
                name: "game".into(),
                at,
                // Straight back along +Z, which is what this always did.
                from: Vec2::new(0.0, 1.0),
                height: value("--height").and_then(|v| v.parse().ok()).unwrap_or(28.0),
                back: value("--back").and_then(|v| v.parse().ok()).unwrap_or(46.0),
            }],
            matrix: None,
            out: value("--out")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("dev/art/shots/game.png")),
            settle,
            map,
            live,
            hour,
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
    taking: Res<Taking>,
    terrain: Option<Res<crate::world::terrain::TerrainSource>>,
    mut cameras: Query<&mut Transform, With<Camera3d>>,
) {
    let Some(shot) = photo.shot(&taking) else {
        return;
    };
    let ground = terrain
        .map(|t| t.0.walk_height(shot.at.x, shot.at.y))
        .unwrap_or(0.0);
    let aim = Vec3::new(shot.at.x, ground + 2.0, shot.at.y);
    let eye = aim + Vec3::new(shot.from.x * shot.back, shot.height, shot.from.y * shot.back);
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
    taking: Res<Taking>,
    terrain: Option<Res<crate::world::terrain::TerrainSource>>,
    mut wardens: Query<&mut Transform, With<crate::player::Player>>,
) {
    let Some(shot) = photo.shot(&taking) else {
        return;
    };
    let ground = terrain
        .map(|t| t.0.walk_height(shot.at.x, shot.at.y))
        .unwrap_or(0.0);
    for mut place in &mut wardens {
        place.translation = Vec3::new(shot.at.x, ground, shot.at.y);
    }
}

/// Keeps the streaming anchor at the spot, so the world loads AROUND the camera.
///
/// Without this the chunks, the cover and the settlements all load around wherever
/// the warden happens to be, and the photograph is of an empty green plain with the
/// town two kilometres behind it.
pub fn anchor_where_told(
    photo: Res<Photo>,
    taking: Res<Taking>,
    terrain: Option<Res<crate::world::terrain::TerrainSource>>,
    mut anchors: Query<
        (&mut Transform, &mut GlobalTransform),
        With<crate::world::StreamAnchor>,
    >,
) {
    let Some(shot) = photo.shot(&taking) else {
        return;
    };
    let ground = terrain
        .map(|t| t.0.walk_height(shot.at.x, shot.at.y))
        .unwrap_or(0.0);
    let at = Vec3::new(shot.at.x, ground, shot.at.y);
    for (mut place, mut world) in &mut anchors {
        place.translation = at;
        *world = GlobalTransform::from(*place);
    }
}

/// Fills in the matrix, once the world knows where its settlements are.
///
/// # Why the shot list is not on the command line
///
/// A named viewpoint is a claim about the WORLD - "the entrance to the nearest
/// village", "the middle of the long bridge" - and none of those can be turned into
/// a coordinate until the settlements have been planned. Naming them rather than
/// writing coordinates down is the whole point: the same nine shots keep meaning the
/// same nine things after the map changes, which is what makes two runs comparable.
pub fn fill_the_matrix(
    mut photo: ResMut<Photo>,
    terrain: Option<Res<crate::world::terrain::TerrainSource>>,
) {
    if photo.matrix.is_none() || !photo.shots.is_empty() {
        return;
    }
    let Some(terrain) = terrain else { return };
    let plan = terrain.plan();
    if plan.sites().is_empty() {
        return;
    }

    let mut shots = Vec::new();
    let mut add = |name: &str, at: Vec2, from: Vec2, height: f32, back: f32| {
        shots.push(Shot {
            name: name.into(),
            at,
            from: from.normalize_or(Vec2::new(0.0, 1.0)),
            height,
            back,
        });
    };

    // The ranch, where the game starts.
    if let Some(ranch) = plan.sites().iter().find(|site| site.ranch) {
        add("ranch_gate", ranch.at, Vec2::new(0.0, 1.0), 5.0, 34.0);
    }

    // A village and a city: their entrance, from outside the boundary looking in,
    // and their middle. `EDGE_LIES_AT` is where the wall stands, so a little past
    // that is outside it.
    for (label, city) in [("village", false), ("city", true)] {
        let Some(site) = plan
            .sites()
            .iter()
            .find(|site| !site.ranch && site.city == city)
        else {
            continue;
        };
        let out = plan.approach(site.at).normalize_or(Vec2::new(0.0, 1.0));
        let gate = site.at + out * site.radius * 1.02;
        add(
            &format!("{label}_entrance"),
            gate,
            out,
            if city { 7.0 } else { 5.0 },
            if city { 52.0 } else { 40.0 },
        );
        add(
            &format!("{label}_node"),
            site.at,
            out,
            if city { 9.0 } else { 5.0 },
            if city { 66.0 } else { 40.0 },
        );
        // And the country outside it, which is where the arrival ought to begin.
        add(
            &format!("{label}_approach"),
            site.at + out * site.radius * 2.4,
            out,
            5.0,
            40.0,
        );
    }

    // THE CANYON, at head height and from inside it.
    //
    // A high oblique shows the geography and answers none of the questions worth
    // asking about a gate: whether the mouths read as the only way through, whether
    // the walls feel close from the follow camera, and whether the heightfield's
    // edge combs where you can see it. Three shots, both mouths and the turn
    // between them, taken from where a warden's eyes are.
    {
        use crate::world::pass::way_through;
        // ON THE FLOOR, which is not the massif's middle: the canyon winds two
        // hundred metres either side on the way through, so a shot placed at the
        // middle of the rock stands on the plain beside it and shows anything but
        // the slot. `way_through` is the centreline itself.
        let reach = 300.0;
        for (name, at, look) in [
            ("canyon_west_mouth", -reach, 1.0_f32),
            ("canyon_inside", 0.0, 1.0),
            ("canyon_east_mouth", reach, -1.0),
        ] {
            let here = way_through(at);
            // Facing along the canyon, so the camera looks down it rather than at a
            // wall: the eye sits back the way the warden came.
            let ahead = (way_through(at + 40.0 * look) - here).normalize_or(Vec2::Y);
            // Close behind. The slot is 38 m wall to wall, so a 34 m pull-back puts
            // the camera IN the rock and the lower half of the frame is the inside
            // of a cliff. A shot has to be sized to the space it is taken in.
            add(name, here, -ahead, 2.4, 12.0);
        }
    }

    // The longest bridge: its entrance and its middle, which is the shot that says
    // whether a kilometre of crossing has anything to look at along it.
    if let Some(bridge) = plan
        .spans()
        .iter()
        .max_by(|a, b| {
            a.from
                .distance(a.to)
                .total_cmp(&b.from.distance(b.to))
        })
    {
        let along = (bridge.to - bridge.from).normalize_or(Vec2::new(0.0, 1.0));
        add("bridge_entrance", bridge.from, -along, 6.0, 46.0);
        add(
            "bridge_middle",
            (bridge.from + bridge.to) * 0.5,
            -along,
            5.0,
            34.0,
        );
    }

    info!("shot matrix: {} viewpoints", shots.len());
    photo.shots = shots;
}

/// Waits for the world to arrive, takes each picture in turn, and quits.
///
/// One run, many shots. A named matrix is only useful if taking it is cheap, and
/// booting the game nine times to photograph nine places is not cheap - each boot
/// spends several hundred frames streaming a world it then throws away. The camera
/// is moved instead, and the world is given time to arrive at each new place.
pub fn take_the_photo(
    mut commands: Commands,
    photo: Res<Photo>,
    mut taking: ResMut<Taking>,
    mut quit: EventWriter<AppExit>,
) {
    let Some(shot) = photo.shot(&taking) else {
        quit.write(AppExit::Success);
        return;
    };

    taking.waited += 1;
    if taking.taken {
        // A few frames after the shutter, so the file is written before either the
        // camera moves on or the process goes away.
        if taking.waited > 30 {
            taking.at += 1;
            taking.waited = 0;
            taking.taken = false;
        }
        return;
    }

    // The first shot pays the full streaming cost. The rest have most of the world
    // already in hand, so they wait for what moving actually changes.
    let settle = if taking.at == 0 {
        photo.settle
    } else {
        photo.settle.min(180).max(90)
    };
    if taking.waited < settle {
        return;
    }

    let out = match &photo.matrix {
        Some(folder) => folder.join(format!("{}.png", shot.name)),
        None => photo.out.clone(),
    };
    if let Some(folder) = out.parent() {
        let _ = std::fs::create_dir_all(folder);
    }
    info!(
        "photographing {} at {:.0}, {:.0} into {}",
        shot.name,
        shot.at.x,
        shot.at.y,
        out.display()
    );
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(out));
    taking.taken = true;
    taking.waited = 0;
}

/// Starts the game rather than the menu, when a photograph was asked for.
///
/// Without this the whole module sits behind `in_state(Playing)` and never runs -
/// the game opens on its menu and waits for a person, which is exactly what a
/// photograph is meant to avoid needing.
fn start_playing(mut next: ResMut<NextState<crate::states::AppState>>) {
    next.set(crate::states::AppState::Playing);
}

/// The hour every photograph is taken at, unless `--live` says otherwise.
///
/// Noon rather than a pretty hour. The sun is at its highest, so shadows are short
/// and nothing is lost in them, and it is the one hour that cannot be confused with
/// any other - a shot at "about four" is a shot whose lighting nobody can reproduce.
const EVIDENCE_HOUR: f32 = 12.0;

/// Holds the clock and the sky still, so two photographs differ only by their subject.
///
/// Every frame, for the reason the camera override is every frame: the game's own
/// systems will happily move both back.
pub fn hold_the_world_still(
    photo: Res<Photo>,
    mut clock: ResMut<crate::sky::TimeOfDay>,
    mut weather: ResMut<crate::weather::TheWeather>,
) {
    if photo.live {
        return;
    }
    // Held through the clock's OWN offset rather than by writing the hour.
    //
    // `read_the_clock` recomputes `hours` from the real time plus `nudge` every
    // frame, so writing `hours` only works for whatever happens to read it before
    // the next frame - the sky went dark and the ground stayed lit at noon, because
    // the sun's own system had already run. Setting the nudge instead means the
    // clock itself reports the hour asked for, and every consumer agrees without
    // anybody having to be ordered.
    let wanted = photo.hour.unwrap_or(EVIDENCE_HOUR);
    clock.follows_clock = false;
    let real = (clock.hours - clock.nudge).rem_euclid(24.0);
    clock.nudge = (wanted - real).rem_euclid(24.0);
    clock.hours = wanted;

    weather.follows_clock = false;
    weather.falling = crate::weather::Falling::Nothing;
    weather.fall = 0.0;
    weather.overcast = 0.0;
    weather.wind = 0.15;
}

/// Pulls the map up, when the photograph is meant to be of the map.
fn open_the_map(photo: Res<Photo>, mut open: ResMut<crate::map::Open>) {
    if photo.map {
        open.0 = true;
    }
}

pub struct PhotoPlugin;

impl Plugin for PhotoPlugin {
    fn build(&self, app: &mut App) {
        let Some(photo) = Photo::asked_for() else {
            return;
        };
        app.insert_resource(photo)
            .init_resource::<Waiting>()
            .init_resource::<Taking>()
            .add_systems(Startup, start_playing)
            .add_systems(
                Update,
                (
                    // AFTER the clock reads itself. It rewrites the hour every frame
                    // from the real time of day, so a hold that runs before it is
                    // simply overwritten - which is why `--hour 22` came back with a
                    // blue sky and every lamp out.
                    hold_the_world_still.after(crate::sky::read_the_clock),
                    fill_the_matrix,
                    open_the_map,
                    stand_the_warden_there,
                    anchor_where_told,
                    take_the_photo,
                )
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
