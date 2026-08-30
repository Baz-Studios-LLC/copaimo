//! A bot that plays the game, so the assembled world gets tested and not just its
//! formulas.
//!
//! # Why a driver and not another test
//!
//! Everything in `src/` is covered by tests that call one function with numbers and
//! check the number that comes back, and almost every fault of the last week got past
//! all of them: a road drawn five metres wider than it could be walked, a step rule
//! that was really a frame-rate rule, a kerb that rendered and refused the
//! controller, a building placed correctly and floating. Those only exist once the
//! systems meet, and the only thing that meets them is playing.
//!
//! So this plays. Codex proposed it and set the constraint that makes it worth
//! having: it must drive the REAL warden, through the real input, the real movement,
//! the real collision and the real grounding. It presses W. It does not write the
//! warden's transform, it does not call a private collision helper, and it does not
//! path around anything - a clever navigator finds a way past broken geometry, which
//! is precisely the thing that must not happen here. It aims at the seam and reports
//! what the game does.
//!
//! # What it is not, yet
//!
//! Not a pathfinder and not an explorer. Every route is authored, its outcome
//! declared in advance, and a route that expects to be BLOCKED is as important as one
//! that expects to arrive - a driver that only checks successful travel will happily
//! approve a warden who walks through walls. Roaming comes later, separately, and
//! feeds candidates to this rather than replacing it.

use bevy::prelude::*;
use std::io::Write;
use std::path::PathBuf;

/// Where the driver leaves its evidence.
///
/// Ignored by git. A run writes a report and, later, its captures, and neither is
/// source: `dev/art/shots` is carrying 182 MB of pictures that arrived one visual
/// pass at a time.
const EVIDENCE: &str = "dev/evidence";

/// How long the warden may make no progress before the route is called stuck.
///
/// Not "stopped this frame" - a kerb, a turn and a wall-slide all cost most of a
/// frame's movement legitimately, and at 240 Hz a frame is 2 cm. Progress is measured
/// toward the checkpoint over this window instead.
const STUCK_AFTER: f32 = 0.75;

/// How close to the destination counts as arriving, in metres.
///
/// Loose enough that steering by eight-way keyboard input can settle, tight enough
/// that arriving on the wrong side of a wall is not an arrival.
const ARRIVED_WITHIN: f32 = 1.2;

/// What a route is meant to prove.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Expect {
    /// The warden gets there. A failure is a passage the game claims to offer and
    /// does not.
    Arrives,
    /// The warden does not. A failure is a barrier the game claims to have and does
    /// not - the canyon walls, which gate a third of the world.
    Blocked,
}

/// One attempt: somewhere to start, somewhere to aim, and what must happen.
#[derive(Clone, Debug)]
pub struct Route {
    pub name: String,
    pub from: Vec2,
    pub to: Vec2,
    /// Ctrl held. Walking is the slower of the two and so takes the smaller stride,
    /// which is the half of the frame-rate question that hides the longest.
    pub walking: bool,
    /// The fixed update rate this route is driven at.
    pub hertz: f32,
    pub expect: Expect,
    /// Game seconds before the route times out.
    pub within: f32,
    /// How far off the straight line the warden may stray before the route is a
    /// wander rather than an attempt.
    pub corridor: f32,
}

/// What one route did.
#[derive(Clone, Debug)]
pub struct Ran {
    pub route: Route,
    pub passed: bool,
    pub why: String,
    /// Where it ended, and how far that is from the aim.
    pub ended: Vec2,
    pub left: f32,
    pub seconds: f32,
    pub went: f32,
    /// The largest single-update change in the ground under the warden's feet. A
    /// kerb is 22 cm; anything much past that is a snap the player feels.
    pub worst_snap: f32,
    pub updates: u32,
}

/// The run in progress.
#[derive(Resource)]
pub struct Driving {
    pub routes: Vec<Route>,
    pub at: usize,
    pub phase: Phase,
    pub done: Vec<Ran>,
    /// Filled once the world exists, because the routes are resolved from it.
    pub resolved: bool,
    pub report: PathBuf,
    /// Live telemetry for the route under way.
    pub since_progress: f32,
    pub best: f32,
    pub seconds: f32,
    pub went: f32,
    pub was: Vec2,
    pub ground: f32,
    pub worst_snap: f32,
    pub updates: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Phase {
    /// Letting the world stream in around the start. Real time, no telemetry.
    Settling(u32),
    /// Under way, at the route's own fixed update rate.
    Running,
    /// Every route attempted.
    Over,
}

impl Driving {
    /// Whether the driver was asked for, and where to put its report.
    pub fn asked_for() -> Option<Self> {
        let args: Vec<String> = std::env::args().collect();
        if !args.iter().any(|arg| arg == "--drive") {
            return None;
        }
        Some(Self {
            routes: Vec::new(),
            at: 0,
            phase: Phase::Settling(SETTLES),
            done: Vec::new(),
            resolved: false,
            report: PathBuf::from(EVIDENCE).join("playtest.md"),
            since_progress: 0.0,
            best: f32::MAX,
            seconds: 0.0,
            went: 0.0,
            was: Vec2::ZERO,
            ground: 0.0,
            worst_snap: 0.0,
            updates: 0,
        })
    }

    fn now(&self) -> Option<&Route> {
        self.routes.get(self.at)
    }

    /// Forgets the last route's telemetry and starts the next one's.
    fn begin(&mut self, from: Vec2, ground: f32) {
        self.since_progress = 0.0;
        self.best = f32::MAX;
        self.seconds = 0.0;
        self.went = 0.0;
        self.was = from;
        self.ground = ground;
        self.worst_snap = 0.0;
        self.updates = 0;
    }
}

/// How many frames the world is given to stream in before a route starts.
const SETTLES: u32 = 240;

pub fn driving(driving: Option<Res<Driving>>) -> bool {
    driving.is_some()
}

/// The routes, resolved from the world the game actually generated.
///
/// Named places rather than numbers wherever the world can name them: "the crown of
/// the widest street in the first city" survives a change to the terrain, and
/// `(-2619, 1747)` becomes a route through an empty field. The resolved coordinates
/// go in the report, which is where a number belongs - as evidence, not as an anchor.
fn plan_the_routes(terrain: &crate::world::terrain::Terrain) -> Vec<Route> {
    let mut routes = Vec::new();

    // ------------------------------------------------ 1. THE KERB, AT EVERY RATE
    //
    // Carriageway to footway, which is the thing that renders correctly and can
    // still refuse the controller. Three approach angles, because a step crossed
    // dead-on is the one line a fault can hide behind.
    if let Some((crown, aside)) = a_city_kerb(terrain) {
        for (angle, called) in [(0.0_f32, "square"), (0.6, "diagonal"), (1.2, "shallow")] {
            for &hertz in &[30.0_f32, 60.0, 120.0, 240.0] {
                for walking in [false, true] {
                    let (sin, cos) = angle.sin_cos();
                    let along = Vec2::new(-aside.y, aside.x);
                    let way = aside * cos + along * sin;
                    routes.push(Route {
                        name: format!(
                            "kerb {called} at {hertz:.0} Hz, {}",
                            if walking { "walking" } else { "jogging" }
                        ),
                        from: crown - aside * 1.5,
                        to: crown - aside * 1.5 + way * 6.0,
                        walking,
                        hertz,
                        expect: Expect::Arrives,
                        within: 12.0,
                        corridor: 4.0,
                    });
                }
            }
        }
    }

    // ------------------------------------------------------ 2. THE CANYON GATE
    //
    // The negative route. The canyon walls are what make the desert a place you
    // arrive at rather than wander into, and for as long as `may_step` measured a
    // step across one frame they gated nothing at all above 60 Hz.
    let middle = crate::world::pass::way_through(40.0);
    let (sin, cos) = crate::world::pass::HEADING.sin_cos();
    let out = -Vec2::new(-sin, cos);
    let floor = terrain.walk_height(middle.x, middle.y);
    let mut foot = middle;
    for step in 1..200 {
        let at = middle + out * step as f32;
        if terrain.walk_height(at.x, at.y) > floor + 2.0 {
            break;
        }
        foot = at;
    }
    for &hertz in &[30.0_f32, 60.0, 120.0, 240.0] {
        for walking in [false, true] {
            routes.push(Route {
                name: format!(
                    "canyon wall at {hertz:.0} Hz, {}",
                    if walking { "walking" } else { "jogging" }
                ),
                from: foot,
                // Twenty metres INTO the wall. Arriving is the failure here.
                to: foot + out * 20.0,
                walking,
                hertz,
                expect: Expect::Blocked,
                within: 14.0,
                corridor: 6.0,
            });
        }
    }
    // And along the floor, which must still work: a gate that stops everybody is a
    // full stop, not a gate.
    routes.push(Route {
        name: "canyon floor".into(),
        from: middle,
        to: crate::world::pass::way_through(70.0),
        walking: false,
        hertz: 60.0,
        expect: Expect::Arrives,
        within: 20.0,
        corridor: 8.0,
    });

    routes
}

/// The crown of a real city street, and the way across it to the footway.
fn a_city_kerb(terrain: &crate::world::terrain::Terrain) -> Option<(Vec2, Vec2)> {
    let plan = terrain.plan();
    for (key, site) in plan.sites().iter().enumerate() {
        if !site.city {
            continue;
        }
        let layout = crate::world::town::lay_out(
            site,
            plan.approach(site.at),
            crate::config::WORLD_SEED.wrapping_add(key as u32 * 7717),
        );
        let street = layout
            .streets
            .iter()
            .max_by(|a, b| a.wide.total_cmp(&b.wide))?;
        let crown = (street.from + street.to) * 0.5;
        let along = (street.to - street.from).normalize_or_zero();
        return Some((crown, Vec2::new(-along.y, along.x)));
    }
    None
}

/// Puts the warden at the start of the next route, and nothing else ever moves them.
///
/// The one teleport the brief allows: placing a warden at the beginning of an
/// isolated attempt. From the first driven update onward the ordinary movement,
/// grounding, collision and camera run, because a driver that writes the transform is
/// testing arithmetic it could have unit-tested instead.
pub fn start_the_route(
    mut driving: ResMut<Driving>,
    terrain: Option<Res<crate::world::terrain::TerrainSource>>,
    towns: Res<crate::world::town::Built>,
    mut wardens: Query<&mut Transform, With<crate::player::Player>>,
    mut orbit: ResMut<crate::camera::Orbit>,
    mut clock: ResMut<crate::sky::TimeOfDay>,
    mut weather: ResMut<crate::weather::TheWeather>,
    mut clocking: ResMut<Time<Virtual>>,
    mut strategy: ResMut<bevy::time::TimeUpdateStrategy>,
) {
    let Some(terrain) = terrain else {
        return;
    };
    // Pinned, every frame: a run whose light and weather drift is a story.
    crate::photo::hold_the_world_at(12.0, &mut clock, &mut weather);
    let _ = &mut clocking;

    if !driving.resolved {
        driving.routes = plan_the_routes(&terrain.0);
        driving.resolved = true;
        info!("driving {} routes", driving.routes.len());
    }

    let Phase::Settling(left) = driving.phase else {
        return;
    };

    // Real time while the world streams in. The fixed step belongs to the run.
    *strategy = bevy::time::TimeUpdateStrategy::Automatic;

    let Some(route) = driving.now().cloned() else {
        driving.phase = Phase::Over;
        return;
    };
    let ground = crate::world::town::stands_on(&terrain.0, &towns, route.from);
    for mut place in &mut wardens {
        place.translation = Vec3::new(route.from.x, ground, route.from.y);
    }
    // The camera turned to face the way the route goes, which is what a mouse does.
    // Steering still reads the camera's ACTUAL forward below, so a wrong sign here
    // costs accuracy and never correctness.
    let want = (route.to - route.from).normalize_or_zero();
    orbit.yaw = (-want.x).atan2(-want.y);

    if left > 0 {
        driving.phase = Phase::Settling(left - 1);
        return;
    }

    driving.begin(route.from, ground);
    driving.phase = Phase::Running;
    *strategy = bevy::time::TimeUpdateStrategy::ManualDuration(std::time::Duration::from_secs_f32(
        1.0 / route.hertz,
    ));
}

/// Presses the keys.
///
/// The whole point of the driver is here: this is the only thing it does to the
/// warden. Everything after it is the game.
pub fn steer(
    driving: Res<Driving>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    cameras: Query<&Transform, With<crate::camera::MainCamera>>,
    wardens: Query<&Transform, With<crate::player::Player>>,
) {
    for key in [
        KeyCode::KeyW,
        KeyCode::KeyA,
        KeyCode::KeyS,
        KeyCode::KeyD,
        KeyCode::ControlLeft,
    ] {
        keys.release(key);
    }
    if driving.phase != Phase::Running {
        return;
    }
    let (Some(route), Some(camera), Ok(warden)) =
        (driving.now(), cameras.iter().next(), wardens.single())
    else {
        return;
    };

    let here = warden.translation.xz();
    let want = (route.to - here).normalize_or_zero();
    if want == Vec2::ZERO {
        return;
    }

    // Movement is camera-relative, so the keys are chosen against the camera's own
    // forward rather than against the world. This is the same eight directions a
    // keyboard offers and no more: a driver with finer steering than a player would
    // pass through gaps a player cannot.
    let forward = camera.forward().as_vec3();
    let forward = Vec2::new(forward.x, forward.z).normalize_or_zero();
    let right = Vec2::new(-forward.y, forward.x);
    let ahead = want.dot(forward);
    let across = want.dot(right);
    // Two thirds of a right angle: the boundary between one key and two.
    const LEANS: f32 = 0.383;
    if ahead > LEANS {
        keys.press(KeyCode::KeyW);
    }
    if ahead < -LEANS {
        keys.press(KeyCode::KeyS);
    }
    if across > LEANS {
        keys.press(KeyCode::KeyD);
    }
    if across < -LEANS {
        keys.press(KeyCode::KeyA);
    }
    if route.walking {
        keys.press(KeyCode::ControlLeft);
    }
}

/// Watches what the game did with it, and calls the route.
pub fn watch(
    time: Res<Time>,
    mut driving: ResMut<Driving>,
    terrain: Option<Res<crate::world::terrain::TerrainSource>>,
    towns: Res<crate::world::town::Built>,
    wardens: Query<&Transform, With<crate::player::Player>>,
    mut quit: EventWriter<AppExit>,
) {
    if driving.phase != Phase::Running {
        if driving.phase == Phase::Over {
            quit.write(AppExit::Success);
        }
        return;
    }
    let (Some(terrain), Ok(warden)) = (terrain, wardens.single()) else {
        return;
    };
    let Some(route) = driving.now().cloned() else {
        return;
    };

    let here = warden.translation.xz();
    let step = here.distance(driving.was);
    let ground = crate::world::town::stands_on(&terrain.0, &towns, here);
    let snap = (ground - driving.ground).abs();

    driving.went += step;
    driving.seconds += time.delta_secs();
    driving.updates += 1;
    driving.was = here;
    // The first update after a teleport is not a snap.
    if driving.updates > 1 {
        driving.worst_snap = driving.worst_snap.max(snap);
    }
    driving.ground = ground;

    // PROGRESS, not speed. A kerb, a turn and a wall-slide all cost most of one
    // update legitimately, and at 240 Hz an update is two centimetres.
    let left = here.distance(route.to);
    if left < driving.best - 0.05 {
        driving.best = left;
        driving.since_progress = 0.0;
    } else {
        driving.since_progress += time.delta_secs();
    }

    // How far from the straight line between the two ends - a route that arrives by
    // going round is not the route that was asked for.
    let line = route.to - route.from;
    let along = line.normalize_or_zero();
    let strayed = (here - route.from).perp_dot(along).abs();

    let verdict = if left <= ARRIVED_WITHIN {
        Some((route.expect == Expect::Arrives, format!("arrived, {left:.2} m short")))
    } else if strayed > route.corridor {
        Some((false, format!("left the corridor by {strayed:.1} m")))
    } else if driving.since_progress > STUCK_AFTER {
        Some((
            route.expect == Expect::Blocked,
            format!(
                "stopped {:.2} m short after {:.1} s without progress",
                driving.best, driving.since_progress
            ),
        ))
    } else if driving.seconds > route.within {
        Some((
            route.expect == Expect::Blocked,
            format!("timed out {:.2} m short", driving.best),
        ))
    } else {
        None
    };

    let Some((passed, why)) = verdict else {
        return;
    };

    let ran = Ran {
        route: route.clone(),
        passed,
        why,
        ended: here,
        left,
        seconds: driving.seconds,
        went: driving.went,
        worst_snap: driving.worst_snap,
        updates: driving.updates,
    };
    info!(
        "{} {}: {}",
        if passed { "PASS" } else { "FAIL" },
        ran.route.name,
        ran.why
    );
    driving.done.push(ran);
    // Written after every route, so a crash still leaves everything up to it.
    write_the_report(&driving);

    driving.at += 1;
    driving.phase = if driving.at < driving.routes.len() {
        Phase::Settling(SETTLES / 8)
    } else {
        Phase::Over
    };
}

/// The report, rewritten after every route.
fn write_the_report(driving: &Driving) {
    let _ = std::fs::create_dir_all(EVIDENCE);
    let Ok(mut file) = std::fs::File::create(&driving.report) else {
        return;
    };
    let failed = driving.done.iter().filter(|ran| !ran.passed).count();
    let _ = writeln!(
        file,
        "# Playtest\n\n\
         The bot drives the real warden with the real keys through the real movement \
         system. A route that expects to be BLOCKED is as much a pass as one that \
         arrives.\n\n\
         Seed {}. {} of {} routes run, {failed} failed.\n",
        crate::config::WORLD_SEED,
        driving.done.len(),
        driving.routes.len(),
    );
    let _ = writeln!(
        file,
        "| route | expected | got | end | worst snap | went | s | updates |\n\
         |---|---|---|---|---|---|---|---|"
    );
    for ran in &driving.done {
        let _ = writeln!(
            file,
            "| {} | {:?} | {} {} | ({:.0}, {:.0}) | {:.2} m | {:.1} m | {:.1} | {} |",
            ran.route.name,
            ran.route.expect,
            if ran.passed { "PASS" } else { "**FAIL**" },
            ran.why,
            ran.ended.x,
            ran.ended.y,
            ran.worst_snap,
            ran.went,
            ran.seconds,
            ran.updates,
        );
    }
}

pub struct DrivePlugin;

impl Plugin for DrivePlugin {
    fn build(&self, app: &mut App) {
        let Some(asked) = Driving::asked_for() else {
            return;
        };
        app.insert_resource(asked)
            .add_systems(
                Startup,
                |mut next: ResMut<NextState<crate::states::AppState>>| {
                    next.set(crate::states::AppState::Playing);
                },
            )
            .add_systems(
                Update,
                (
                    start_the_route.after(crate::sky::read_the_clock),
                    // BEFORE the warden moves, because pressing a key after the
                    // movement system has run is pressing it for next frame.
                    steer.before(crate::player::move_player),
                    watch.after(crate::player::move_player),
                )
                    .chain()
                    .run_if(driving)
                    .run_if(in_state(crate::states::AppState::Playing)),
            );
    }
}
