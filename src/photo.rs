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
    /// What this shot is lit for, and what it will be checked against.
    pub lighting: Lighting,
}

/// What a shot's lighting is FOR.
///
/// # A shot called `night_node`, taken at noon
///
/// The run had one hour and a shot had none, so the three viewpoints written down
/// as "the lighting evidence, at the hours it has to be judged at" were all
/// photographed at `EVIDENCE_HOUR` - midday. Nobody had opened them, and their file
/// names said they were evidence about the lamps.
///
/// The first fix gave a shot an `Option<f32>` and set it from the name: any shot
/// called `night_*` asked for 22:00. That is two improvements on nothing and still
/// two things wrong with it - a viewpoint could be added with no hour at all and
/// silently take the run's, and the NAME decided the lighting, so the label and the
/// content were still separate claims that could disagree.
///
/// This is Codex's answer and it is better. Every shot must say what it is lit for
/// or the code does not compile; the hour is derived from that rather than sitting
/// beside it; and the state that actually results is checked before the shutter -
/// see `as_promised`. An instrument that reports the wrong thing under a convincing
/// label is how a fault survives a review, so this one refuses to write the file.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Lighting {
    /// Midday. For everything the lighting is not the subject of.
    Noon,
    /// The sun on the horizon, where a fade has to complement what is left of the sky.
    Dusk,
    /// Full dark. `lamps` says whether this viewpoint stands close enough to a
    /// settlement for its lighting to be part of the evidence - an approach shot
    /// from outside the boundary is a night shot with nothing lit in it on purpose,
    /// and the checker cannot tell that from the name.
    Night { lamps: bool },
    /// A particular hour, when none of the above is the point.
    At(f32),
    /// Whatever the machine says. Only for looking at the real thing.
    Live,
}

impl Lighting {
    /// The hour this asks the world to hold, if it asks for one.
    pub fn hour(self) -> Option<f32> {
        match self {
            Lighting::Noon => Some(EVIDENCE_HOUR),
            Lighting::Dusk => Some(DUSK_HOUR),
            Lighting::Night { .. } => Some(AFTER_DARK),
            Lighting::At(hour) => Some(hour),
            Lighting::Live => None,
        }
    }

    fn called(self) -> String {
        match self {
            Lighting::Noon => "noon".into(),
            Lighting::Dusk => "dusk".into(),
            Lighting::Night { .. } => "night".into(),
            Lighting::At(hour) => format!("{hour:.1}h"),
            Lighting::Live => "live".into(),
        }
    }
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
    /// One line per shot: what it promised and what it got.
    pub report: Vec<String>,
}

/// How far the held hour may drift before the shot is not the shot it says it is.
const HOUR_SLACK: f32 = 0.05;

/// Checks a shot against the lighting it declares, and describes what it found.
///
/// # Refusing to write convincing evidence
///
/// The matrix's whole value is that two runs are comparable, which rests on a shot
/// being what its name says. The old failure was not a wrong picture - it was a
/// wrong picture NOBODY OPENED, filed under a name that said it was right.
///
/// So the state is read after the sky and the lamps have run and before the shutter,
/// and a mismatch stops the run rather than writing the file. A missing photograph
/// is an obvious problem; a daylit `night_node.png` is an invisible one.
///
/// # What it does not check
///
/// Codex also suggested comparing `ClearColor` against `sky_colour` at the actual
/// sun height. That one is left out on purpose: the clear colour is mixed with the
/// overcast, so the comparison would need to reproduce the mix - a second derivation
/// of the thing it is checking, which is the fault this whole day has been about. The
/// sun's own height and the light it casts say the same thing without a copy.
fn as_promised(
    shot: &Shot,
    clock: &crate::sky::TimeOfDay,
    weather: &crate::weather::TheWeather,
    lux: f32,
    lamps: usize,
) -> Result<String, String> {
    let height = clock.sun_height();
    let row = format!(
        "| {} | {} | {} | {:.2} | {:.2} | {:.0} | {} | {} |",
        shot.name,
        shot.lighting.called(),
        shot.lighting
            .hour()
            .map_or("-".into(), |hour| format!("{hour:.1}")),
        clock.hours,
        height,
        lux,
        lamps,
        if weather.falling == crate::weather::Falling::Nothing { "clear" } else { "falling" },
    );

    if let Some(wanted) = shot.lighting.hour() {
        // Round the clock, so 23.99 against 0.01 is a minute apart and not a day.
        let apart = (clock.hours - wanted).rem_euclid(24.0);
        let apart = apart.min(24.0 - apart);
        if apart > HOUR_SLACK {
            return Err(format!(
                "{} asked for {wanted:.2}h and the world is at {:.2}h",
                shot.name, clock.hours,
            ));
        }
        if clock.follows_clock {
            return Err(format!("{} is held evidence and the clock is still running", shot.name));
        }
        if weather.follows_clock {
            return Err(format!("{} is held evidence and the weather is still running", shot.name));
        }
    }

    match shot.lighting {
        Lighting::Night { lamps: wanted } => {
            if height >= 0.0 {
                return Err(format!("{} is a night shot and the sun is up", shot.name));
            }
            // Moonlight, not sunlight. The two are an order apart - see `sky`.
            if lux > crate::sky::MOON_LUX * 1.05 {
                return Err(format!(
                    "{} is a night shot lit at {lux:.0} lux, which is daylight",
                    shot.name,
                ));
            }
            // And the thing the shot exists to show is actually burning.
            //
            // Asked for by the shot rather than guessed from its name. The first
            // version of this check assumed any night shot wanted lamps in it and
            // stopped the run at `night_entrance`, which stands outside the boundary
            // looking in and is supposed to have none - the checker inferring intent
            // is the same fault as the file name carrying it.
            if wanted && lamps == 0 {
                return Err(format!(
                    "{} is evidence about the lamps and not one of them is lit",
                    shot.name,
                ));
            }
        }
        Lighting::Noon => {
            if height <= 0.0 {
                return Err(format!("{} is a daylight shot and the sun is down", shot.name));
            }
            if lux < crate::sky::DAY_LUX * 0.5 {
                return Err(format!(
                    "{} is a daylight shot lit at {lux:.0} lux",
                    shot.name,
                ));
            }
        }
        // Dusk is the hour where neither test means anything, which is why it is
        // worth photographing. The hour check above is the whole of its contract.
        Lighting::Dusk | Lighting::At(_) | Lighting::Live => {}
    }
    Ok(row)
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
                // Whatever `--hour` says, or the machine's own time. A single shot
                // is somebody looking at something on purpose, not evidence filed
                // under a name.
                lighting: Lighting::Live,
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
/// Where a settlement's guild hall stands, and which way its door faces.
///
/// # A viewpoint that is about a landmark has to find the landmark
///
/// The settlement shots were aimed at `site.at`, the middle of the plan, on the
/// reasoning that the guild hall takes the square so the middle is where it is. That
/// held while the hall was 18 m across and stopped the day it grew to 26: the search
/// that places it walks OUTWARD until it finds room, so it moved - and three shots
/// labelled as the middle of a village came back showing a well and some cottages
/// with the landmark out of frame entirely.
///
/// The fix is not to aim further out. It is to stop assuming: the town is laid out
/// with the same seed the game gives it, and the camera is pointed at whatever that
/// says. `raise_the_towns` keys a settlement on its index in the plan, so the two
/// agree by construction rather than by coincidence.
fn guild_hall_in(plan: &crate::world::settle::Settlements, index: usize) -> Option<(Vec2, f32)> {
    let site = plan.sites().get(index)?;
    if site.ranch {
        return None;
    }
    crate::world::town::lay_out(
        site,
        plan.approach(site.at),
        crate::config::WORLD_SEED.wrapping_add(index as u32 * 7717),
    )
    .plots
    .into_iter()
    .find(|plot| plot.what == crate::world::town::Building::GuildHall)
    // Out through its DOOR, which is where the sign and the rose are. Stood behind
    // it the shot is a roof.
    .map(|plot| (plot.at, plot.facing))
}

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
    // Every viewpoint says what it is lit for. There is no default: a new one that
    // does not decide will not compile, which is the whole point of the change.
    let mut add = |name: &str, at: Vec2, from: Vec2, height: f32, back: f32, lighting: Lighting| {
        shots.push(Shot {
            name: name.into(),
            at,
            from: from.normalize_or(Vec2::new(0.0, 1.0)),
            height,
            back,
            lighting,
        });
    };

    // The ranch, where the game starts.
    if let Some(ranch) = plan.sites().iter().find(|site| site.ranch) {
        add("ranch_gate", ranch.at, Vec2::new(0.0, 1.0), 5.0, 34.0, Lighting::Noon);
    }

    // A village and a city: their entrance, from outside the boundary looking in,
    // and their middle. `EDGE_LIES_AT` is where the wall stands, so a little past
    // that is outside it.
    for (label, city) in [("village", false), ("city", true)] {
        let Some((index, site)) = plan
            .sites()
            .iter()
            .enumerate()
            .find(|(_, site)| !site.ranch && site.city == city)
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
            Lighting::Noon
        );
        // THE HALL, not the middle. See `guild_hall_in`.
        let (heart, look) = match guild_hall_in(plan, index) {
            Some((at, facing)) => (at, Vec2::new(facing.sin(), -facing.cos())),
            None => (site.at, out),
        };
        add(
            &format!("{label}_node"),
            heart,
            look,
            if city { 10.0 } else { 8.0 },
            if city { 46.0 } else { 38.0 },
            Lighting::Noon
        );
        // AND THE STREET AT EYE LEVEL, which is where a kerb lives.
        //
        // Every settlement shot in this matrix was taken from five to ten metres up
        // and forty back, which is the right height to judge a plan and the wrong one
        // to judge a surface: a footway's kerb is 14 cm, and from up there it is less
        // than a pixel. Footways went in and the evidence could not have shown them
        // either way.
        add(
            &format!("{label}_street"),
            site.at + out * (site.radius * 0.45),
            -out,
            1.7,
            14.0,
            Lighting::Noon,
        );

        // And the country outside it, which is where the arrival ought to begin.
        add(
            &format!("{label}_approach"),
            site.at + out * site.radius * 2.4,
            out,
            5.0,
            40.0,
            Lighting::Noon
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
            add(name, here, -ahead, 2.4, 12.0, Lighting::Noon);
        }
    }

    // THE LIGHTING EVIDENCE, at the hours it has to be judged at.
    //
    // Codex's list. A still cannot show the walk-through - that wants video - but the
    // other three are stills and they are the ones that catch what an aerial cannot:
    // whether the dusk fade complements what is left of the sky, whether the node is
    // navigable at full dark, and whether light with no shadows leaks through the
    // building it should be stopped by.
    if let Some((index, site)) = plan
        .sites()
        .iter()
        .enumerate()
        .find(|(_, site)| site.city && !site.ranch)
    {
        let out = plan.approach(site.at).normalize_or(Vec2::new(0.0, 1.0));
        add(
            "night_entrance",
            site.at + out * site.radius * 1.02,
            out,
            5.0,
            44.0,
            // Outside the boundary looking in: the lit windows carry this one, and
            // the street lamps are further off than they are admitted from.
            Lighting::Night { lamps: false },
        );
        // On the hall after dark: its own lamps, its lit windows, and whether the
        // sign and the rose still read once the sun is off them.
        let (heart, look) = match guild_hall_in(plan, index) {
            Some((at, facing)) => (at, Vec2::new(facing.sin(), -facing.cos())),
            None => (site.at, out),
        };
        add("night_node", heart, look, 7.0, 34.0, Lighting::Night { lamps: true });
        // Behind a building, looking back at the lamps on the far side of it: if
        // light is passing through the wall, this is where it shows.
        add(
            "night_behind",
            site.at + out.perp() * site.radius * 0.55,
            out.perp(),
            4.0,
            26.0,
            Lighting::Night { lamps: true },
        );
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
        add("bridge_entrance", bridge.from, -along, 6.0, 46.0, Lighting::Noon);
        add(
            "bridge_middle",
            (bridge.from + bridge.to) * 0.5,
            -along,
            5.0,
            34.0,
            Lighting::Noon
        );
    }

    info!("shot matrix: {} viewpoints", shots.len());
    photo.shots = shots;
}

/// Writes down what each shot promised and what it got, beside the pictures.
///
/// The point is that a fault is visible in the FOLDER. A matrix nobody opens is how
/// three lighting shots came to be taken at midday for as long as they existed.
fn write_the_report(photo: &Photo, taking: &Taking) {
    let Some(folder) = photo.matrix.as_ref() else {
        return;
    };
    let mut page = String::from(
        "# Shot matrix\n\n         What each shot asked its world to be, and what the world was when the\n         shutter went. Written by `photo::write_the_report`.\n\n         | shot | lit for | asked | clock | sun | lux | lamps | weather |\n         |---|---|---|---|---|---|---|---|\n",
    );
    for row in &taking.report {
        page.push_str(row);
        page.push('\n');
    }
    page.push_str(&format!(
        "\n{} of {} shots taken.\n",
        taking.report.len(),
        photo.shots.len(),
    ));
    let _ = std::fs::create_dir_all(folder);
    let _ = std::fs::write(folder.join("matrix_report.md"), page);
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
    clock: Res<crate::sky::TimeOfDay>,
    weather: Res<crate::weather::TheWeather>,
    suns: Query<&DirectionalLight>,
    points: Query<&PointLight>,
    spots: Query<&SpotLight>,
) {
    let Some(shot) = photo.shot(&taking) else {
        // NOTHING YET IS NOT NOTHING LEFT.
        //
        // A matrix is filled in once the world knows where its settlements are, so
        // for the first frames the list is empty - and quitting on an empty list
        // quits before taking a single photograph. The run exited cleanly, said
        // "shot matrix: 15 viewpoints" on its way out, and wrote no files.
        //
        // Only an empty list that was never filled means there is nothing to do.
        if !photo.shots.is_empty() {
            write_the_report(&photo, &taking);
            quit.write(AppExit::Success);
        }
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
    // WHAT THE WORLD ACTUALLY IS, before the shutter rather than after the review.
    let lux = suns.iter().map(|sun| sun.illuminance).fold(0.0_f32, f32::max);
    let lamps = points.iter().filter(|light| light.intensity > 0.0).count()
        + spots.iter().filter(|light| light.intensity > 0.0).count();
    match as_promised(shot, &clock, &weather, lux, lamps) {
        Ok(row) => taking.report.push(row),
        Err(wrong) => {
            error!("the matrix will not write evidence it cannot stand behind: {wrong}");
            write_the_report(&photo, &taking);
            quit.write(AppExit::error());
            return;
        }
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

/// And the hour a shot of the lighting is taken at: full dark, with the lamps up
/// and no dusk left in the sky to flatter them.
const AFTER_DARK: f32 = 22.0;

/// The sun on the horizon, where a fade has to sit against what is left of the sky.
const DUSK_HOUR: f32 = 18.4;

/// Holds the clock and the sky still, so two photographs differ only by their subject.
///
/// Every frame, for the reason the camera override is every frame: the game's own
/// systems will happily move both back.
pub fn hold_the_world_still(
    photo: Res<Photo>,
    taking: Res<Taking>,
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
    // The command line first, because that is somebody asking; then the shot's own
    // hour; then noon.
    let wanted = photo
        .hour
        .or_else(|| photo.shots.get(taking.at).and_then(|shot| shot.lighting.hour()));
    // A shot lit LIVE is one nobody wants held. Everything else names an hour.
    let Some(wanted) = wanted else {
        return;
    };
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
