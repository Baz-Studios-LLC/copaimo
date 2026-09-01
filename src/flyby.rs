//! Flying the real game across the map, and timing every frame of it.
//!
//!     cargo run --release -- --flyby
//!     cargo run --release -- --flyby --speed 220 --over 6
//!
//! # Why this exists
//!
//! The user reported the frame rate dying while flying around in edit mode, and
//! the first thing I reached for was a photograph - which showed 161 fps over a
//! city and proved nothing at all, because a photograph is taken standing still
//! and the fault is in the MOVING. `measure.rs` cannot answer it either: it says
//! so in its own header, being honest CPU timing with no frame loop in it.
//!
//! Codex then asked, correctly, for the missing instrument: a rapid multi-town
//! fly route reporting the frame-time distribution and what the settlement pool
//! was doing while it ran. This is that. It flies the real camera - the one
//! carrying `StreamAnchor`, which is what makes flying stream towns at all -
//! along a route through real settlements, and reports what the frames cost.
//!
//! # What it reports, and why those numbers
//!
//! A mean frame time hides exactly the fault being looked for: one six-second
//! hitch inside a thousand good frames barely moves it. So this reports the
//! DISTRIBUTION - median, 95th, 99th, worst - because a stutter lives in the
//! tail, and the worst frame is the one the player actually feels.
//!
//! Alongside it, what the settlement pool did: how many raises were started, how
//! many landed, how many were called off, and the most in flight at once. That
//! is the evidence for AQ-026, which is a claim about jobs nobody wants any more.

use bevy::prelude::*;

use crate::world::StreamAnchor;

/// What was asked for on the command line.
#[derive(Resource, Clone)]
pub struct Flyby {
    /// How fast to fly, in metres a second.
    speed: f32,
    /// How many settlements to fly over.
    over: usize,
}

/// How fast the editor's own fly camera moves when nobody is holding boost.
///
/// Taken from `camera::FLY_SPEED` rather than picked, so the route is flown at a
/// speed somebody could actually fly it at. Boost multiplies this by five, which
/// is the case worth measuring - a slow flight gives the pool time it does not
/// have to have.
const FLIES_AT: f32 = 70.0 * 5.0;

/// Where the camera flies, in metres above the ground.
const FLIES_OVER: f32 = 260.0;

impl Flyby {
    /// Reads `--flyby` and friends, if they are there.
    ///
    /// Hand-parsed like `photo::asked_for`, and for the same reason.
    pub fn asked_for() -> Option<Self> {
        let args: Vec<String> = std::env::args().collect();
        if !args.iter().any(|arg| arg == "--flyby") {
            return None;
        }
        let value = |name: &str| -> Option<String> {
            args.iter()
                .position(|a| a == name)
                .and_then(|at| args.get(at + 1))
                .cloned()
        };
        Some(Self {
            speed: value("--speed").and_then(|v| v.parse().ok()).unwrap_or(FLIES_AT),
            over: value("--over").and_then(|v| v.parse().ok()).unwrap_or(6),
        })
    }
}

/// The route, and what the flight has cost so far.
#[derive(Resource, Default)]
struct Flying {
    /// The settlements to pass over, in order.
    over: Vec<Vec2>,
    /// Which leg is being flown.
    leg: usize,
    /// How far along that leg, in metres.
    along: f32,
    /// Every frame's length in seconds, and whether a settlement finished
    /// coming up on it - which is how a hitch gets attributed to a cause
    /// rather than guessed at.
    frames: Vec<(f32, bool)>,
    /// How many had landed as of last frame.
    had_landed: u32,
    /// How many entities there were last frame, so a hitch can be asked what
    /// appeared on it. A cost that spawns is a different animal from one that
    /// computes, and guessing between them is how the last two hours went.
    had_entities: usize,
    /// What appeared on each frame.
    appeared: Vec<i64>,
    /// How long this frame's main schedule took, which is everything the game
    /// does on the CPU before the renderer is handed the world.
    thinking: std::time::Duration,
    /// That, per frame.
    thought: Vec<f32>,
    /// Frames spent before the first leg, which pay for the world coming up and
    /// are not what this measures.
    settling: u32,
}

/// How long to let the world come up before the clock starts.
const SETTLES_FOR: u32 = 240;

pub struct FlybyPlugin;

impl Plugin for FlybyPlugin {
    fn build(&self, app: &mut App) {
        let Some(flyby) = Flyby::asked_for() else {
            return;
        };
        app.insert_resource(flyby)
            .init_resource::<Flying>()
            .init_resource::<Thinking>()
            .add_systems(Startup, start_flying)
            // AROUND EVERYTHING THE GAME DOES, so a hitch can be told apart
            // from a hitch: if the main schedule is quick and the frame is not,
            // the cost is the renderer's and no amount of budgeting systems
            // will touch it.
            .add_systems(First, start_thinking)
            .add_systems(Last, stop_thinking)
            .add_systems(Update, fly.run_if(crate::build::a_world_is_up));
    }
}

/// When this frame's main schedule began.
#[derive(Resource)]
struct Thinking(std::time::Instant);

impl Default for Thinking {
    fn default() -> Self {
        Self(std::time::Instant::now())
    }
}

fn start_thinking(mut thinking: ResMut<Thinking>) {
    thinking.0 = std::time::Instant::now();
}

fn stop_thinking(thinking: Res<Thinking>, mut flying: ResMut<Flying>) {
    flying.thinking = thinking.0.elapsed();
}

/// Straight into the world rather than the menu, like a photograph.
fn start_flying(mut state: ResMut<NextState<crate::states::AppState>>) {
    state.set(crate::states::AppState::Playing);
}

fn fly(
    flyby: Res<Flyby>,
    mut flying: ResMut<Flying>,
    mut mode: ResMut<crate::camera::CameraMode>,
    terrain: Option<Res<crate::world::terrain::TerrainSource>>,
    raising: Res<crate::world::town::Raising>,
    // REAL time, not virtual. `Time` is clamped to a maximum delta - a quarter
    // of a second by default - so a worse frame than that reads as exactly
    // 250.0 ms, which is what the first flight reported twice running. A ruler
    // that saturates at the interesting value measures nothing at the top end.
    time: Res<Time<Real>>,
    mut anchors: Query<&mut Transform, With<StreamAnchor>>,
    everything: Query<()>,
    mut quit: EventWriter<AppExit>,
) {
    let Some(terrain) = terrain else {
        return;
    };
    // Flying, because that is the case being measured: the anchor rides the
    // camera - see `camera::spawn_camera` - so a flight is what streams towns.
    *mode = crate::camera::CameraMode::Fly;

    // THE ROUTE: the settlements themselves, nearest to farthest from the first,
    // so the flight crosses real towns rather than a line over open meadow.
    if flying.over.is_empty() {
        let plan = terrain.0.plan();
        let mut sites: Vec<Vec2> = plan
            .sites()
            .iter()
            .filter(|site| !site.ranch)
            .map(|site| site.at)
            .collect();
        let Some(first) = sites.first().copied() else {
            return;
        };
        sites.sort_by(|a, b| {
            a.distance(first)
                .partial_cmp(&b.distance(first))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        sites.truncate(flyby.over.max(2));
        info!(
            "flying over {} settlements at {:.0} m/s",
            sites.len(),
            flyby.speed
        );
        flying.over = sites;
    }

    // The world has to be up before its frames mean anything.
    if flying.settling < SETTLES_FOR {
        flying.settling += 1;
        let start = flying.over[0];
        for mut place in &mut anchors {
            place.translation = Vec3::new(start.x, ground_at(&terrain, start), start.y);
        }
        return;
    }

    if flying.leg + 1 >= flying.over.len() {
        report(&flying, &raising);
        quit.write(AppExit::Success);
        return;
    }

    // ALONG THE LEG at the flying speed, in the frame's own time - so a frame
    // that took a long time moves the camera further, exactly as flying does.
    let (from, to) = (flying.over[flying.leg], flying.over[flying.leg + 1]);
    let leg = from.distance(to).max(1.0);
    flying.along += flyby.speed * time.delta_secs();
    if flying.along >= leg {
        flying.along = 0.0;
        flying.leg += 1;
        return;
    }
    let at = from.lerp(to, flying.along / leg);
    for mut place in &mut anchors {
        place.translation = Vec3::new(at.x, ground_at(&terrain, at), at.y);
    }
    let landed = raising.landed > flying.had_landed;
    flying.had_landed = raising.landed;
    let now = everything.iter().count();
    let appeared = now as i64 - flying.had_entities as i64;
    flying.had_entities = now;
    flying.appeared.push(appeared);
    let thought = flying.thinking.as_secs_f32() * 1000.0;
    flying.thought.push(thought);
    flying.frames.push((time.delta_secs(), landed));
}

/// The height the camera flies at over a point.
fn ground_at(terrain: &crate::world::terrain::TerrainSource, at: Vec2) -> f32 {
    terrain.0.height(at.x, at.y) + FLIES_OVER
}

/// What the flight cost, as a distribution rather than an average.
fn report(flying: &Flying, raising: &crate::world::town::Raising) {
    if flying.frames.is_empty() {
        warn!("the flight recorded no frames");
        return;
    }
    let mut frames: Vec<f32> = flying.frames.iter().map(|(ms, _)| *ms).collect();
    frames.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let at = |share: f32| frames[((frames.len() - 1) as f32 * share) as usize] * 1000.0;
    let over = |ms: f32| frames.iter().filter(|f| **f * 1000.0 > ms).count();

    // WHAT THE HITCHES WERE. A frame where a settlement finished coming up is
    // paying for the standing-up - the scenes, the footings, the mesh handed to
    // the GPU - which is main-thread work no pool can take. One where nothing
    // landed is paying for something else, and the two want different fixes.
    let hitched: Vec<&(f32, bool)> = flying
        .frames
        .iter()
        .filter(|(secs, _)| *secs * 1000.0 > 33.0)
        .collect();
    let standing_up = hitched.iter().filter(|(_, landed)| *landed).count();
    let worst_other = hitched
        .iter()
        .filter(|(_, landed)| !*landed)
        .map(|(secs, _)| *secs * 1000.0)
        .fold(0.0_f32, f32::max);

    info!("flew {} frames", frames.len());
    info!(
        "  frame time: median {:.1} ms, 95th {:.1}, 99th {:.1}, worst {:.1}",
        at(0.5),
        at(0.95),
        at(0.99),
        at(1.0),
    );
    // A frame over 33 ms is a visible hitch at 30 fps; one over 100 is a stall
    // nobody would call anything else.
    info!(
        "  hitches: {} frames over 33 ms, {} over 100 ms",
        over(33.0),
        over(100.0),
    );
    info!(
        "  settlements: {} raises started, {} landed, {} called off, {} at once at most",
        raising.started, raising.landed, raising.called_off, raising.most_at_once,
    );
    info!(
        "  still in flight when the route ended: {}",
        raising.at_work()
    );
    info!(
        "  of {} hitched frames, {} were a settlement standing up; worst of the rest {:.1} ms",
        hitched.len(),
        standing_up,
        worst_other,
    );
    // WHERE IN THE FLIGHT they fell. A fault that recurs is a cost being paid
    // over and over; one that stops is something paid once - a pipeline
    // compiled, an asset loaded - and the two want completely different fixes.
    let when: Vec<String> = flying
        .frames
        .iter()
        .enumerate()
        .filter(|(_, (secs, _))| *secs * 1000.0 > 100.0)
        .map(|(at, (secs, _))| {
            format!(
                "{at}:{:.0}ms(think {:.0}){:+}",
                secs * 1000.0,
                // The PREVIOUS frame's thinking: this frame's is recorded in
                // `Last`, after `fly` has already written the row.
                flying.thought.get(at.saturating_sub(1)).copied().unwrap_or(0.0),
                flying.appeared.get(at).copied().unwrap_or(0)
            )
        })
        .collect();
    info!("  frames over 100 ms, in order: {}", when.join(" "));
}
