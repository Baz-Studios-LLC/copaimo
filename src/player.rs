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
use crate::states::AppState;
use crate::util::facing_quat;
use crate::world::terrain::TerrainSource;
use crate::world::WorldBounds;

/// A deliberate slow walk, in metres a second. Held on Ctrl.
///
/// # Seven was not a walk, it was a world record
///
/// This was 7.0 and the sprint 15.0, on a figure 1.7 m tall. Reported simply as
/// "movement is extremely fast", which it was by a factor of five. It also meant the
/// WALK CLIP NEVER PLAYED, because the run threshold sat BELOW the walking speed:
/// every step the warden had ever taken ran the run clip at five cycles a second, so
/// every judgement anyone had made about how the gaits looked was a judgement about
/// the run played at five times its cadence. **When a whole family of opinions is
/// wrong at once, check whether they were all formed about the same mistake.**
///
/// # And then it went too slow, for a reason worth writing down
///
/// The correction put the DEFAULT pace at a walk, which is not what the games this one
/// is measured against do. In Palworld, Skyrim, WoW and Unity's own third-person
/// template the default is a JOG and walking is a deliberate slow mode that keyboard
/// players almost never choose. "Movement is still too slow" was not asking for a
/// faster walk; it was asking to stop walking everywhere.
///
/// The physics agreed. At 2.25 m/s a 1.7 m person has a Froude number of 0.57
/// (v²/gL, leg 0.90 m), and the walk-to-run transition sits at Fr ≈ 0.5 — so a real
/// human at the old "walk" speed would already have broken into a run.
///
/// 1.9 is what the walk CLIP natively carries: 1.935 m a cycle at 115 steps a minute.
/// Played at its own rate, so nothing stretches and nothing skates.
pub const WALK_SPEED: f32 = 1.27;

/// The default pace, in metres a second. A jog.
///
/// The clip called `run` is really a jog — 2.282 m a cycle at 180 steps a minute is
/// 3.42 m/s — and this plays it at very nearly its native rate. Comparable figures:
/// Palworld's default is 3.50, Unity's third-person sprint 5.34, Epic's own authored
/// run 5.00.
pub const JOG_SPEED: f32 = 2.86;

/// A sprint, in metres a second. Held on Shift.
///
/// # Capped by the clip, and the cap is the next piece of work
///
/// There is no sprint clip yet, so this stretches the jog. The reference brief puts
/// the acceptable stretch at ±25% around a clip's native speed, and 3.42 × 1.25 =
/// 4.28 — past that the cadence leaves the believable band and the legs churn, which
/// is what 4.6 was doing at 242 steps a minute.
///
/// **A real 6 m/s sprint needs 3.50 m a cycle, and the way to get it is AIRTIME, not
/// reach.** Planted-foot travel is about one leg length in every clip, from a jog to
/// a world-class sprint — Weyand measures 0.99 ± 0.08 m across 6.2 to 11.1 m/s — and
/// stride is that contact length divided by the stance fraction. So a longer stride
/// comes from spending less of the cycle on the ground, and trying to buy it with a
/// bigger leg angle is why 42 degrees of thigh swing once read as the splits.
pub const SPRINT_SPEED: f32 = 5.14;

/// How fast the warden swivels to face the way they're heading, in radians/sec.
const TURN_RATE: f32 = 12.0;
/// Standing eye-to-toe height, used to keep the body clear of the ground.
/// The steepest rise the warden can WALK up, in metres climbed per metre
/// travelled. One-in-one is a 45° scramble and still walking; this is a little
/// past it.
///
/// Terrain is this game's only geometry, so this rule is what makes a wall a
/// WALL: the canyon country's faces rise three-to-five metres per metre, and
/// without a refusal here the warden strolls up them and the canyon gates
/// nothing. Only the step UP is refused — any slope can be walked back down —
/// so nowhere is a trap.
pub const CLIMB_LIMIT: f32 = 1.4;

/// How deep the warden may wade, in metres below sea level.
///
/// The sea is not walkable in the base game — it is for boats. This is both how
/// far they can stand into it and how far they can *walk* into it: one number,
/// so the depth they are held at and the depth they are turned back at can never
/// disagree and leave them bobbing at a line they cannot cross.
const WADE_DEPTH: f32 = 1.4;

/// Whether one step of a walk is allowed: not into deep water, not up a cliff.
///
/// The sea is for boats. Rather than an invisible wall at the waterline — which
/// reads as a bug, and stops you paddling at a beach at all — the warden wades
/// until the water is about knee-to-waist and is then turned back by it. Only the
/// step INTO deeper water is refused, so someone who somehow ends up out there
/// can always walk home. The cliff rule is the same shape: only the step UP is
/// refused, so no slope is a trap.
fn may_step(terrain: &crate::world::terrain::Terrain, from: Vec3, to: Vec3) -> bool {
    let here = terrain.height(from.x, from.z);
    let there = terrain.height(to.x, to.z);

    let depth = SEA_LEVEL - there;
    if depth > WADE_DEPTH && depth >= SEA_LEVEL - here {
        return false;
    }

    let run = Vec2::new(to.x - from.x, to.z - from.z).length();
    run <= f32::EPSILON || there - here <= run * CLIMB_LIMIT
}

#[derive(Component)]
pub struct Player;

/// How fast the warden is actually travelling, in metres a second.
///
/// The ASKED speed is not it: a step into deep water or up a cliff is refused, and
/// a warden pressed against a canyon wall is not walking however hard the key is
/// held. So this is measured from where they ended up, which is what a walk cycle
/// has to match or the feet skate.
#[derive(Component, Default)]
pub struct Striding {
    pub speed: f32,
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        // On ENTERING the game, not at startup. At startup the menu has not run,
        // so `Progress::from` is always empty — which made Continue a dead
        // letter: the warden was already standing at the ranch before the save
        // was read, the saved position was never applied, and the thirty-second
        // autosave then overwrote the real save with the ranch. New Game
        // mid-session had the mirror fault: the old warden stayed where they
        // were, and the "fresh" save inherited the position.
        app.add_systems(OnEnter(AppState::Playing), spawn_player)
            // The warden only walks in the game. In the terrain tool the same
            // keys fly the camera, and in the menu nothing should move at all.
            .init_resource::<crate::look::Look>()
            .add_systems(
                OnEnter(AppState::Playing),
                crate::motion::ask_for_the_clips,
            )
            .add_systems(
                Update,
                (
                    move_player,
                    // The clips: asked for on the way in, found when the file
                    // arrives, handed to each player the scene brings in.
                    crate::motion::find_the_clips.run_if(crate::motion::still_waiting),
                    // BEFORE anything is played: the offset a worn thing gets is
                    // taken from the skeleton's rest pose, and a pose baked in at
                    // attachment stays baked in.
                    crate::look::hang_things_on_the_head
                        .before(crate::motion::hand_the_clips_over),
                    crate::motion::hand_the_clips_over
                        .run_if(crate::motion::the_clips_are_ready),
                    crate::motion::match_the_clip_to_the_walking
                        .run_if(crate::motion::the_clips_are_ready),
                    // Every frame while the world is open: a scene arrives over
                    // several frames and a part cannot be painted before it exists.
                    crate::look::paint_the_warden,
                )
                    .run_if(in_state(AppState::Playing)),
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
    assets: Res<AssetServer>,
    look: Res<crate::look::Look>,
    terrain: Res<TerrainSource>,
    bounds: Res<WorldBounds>,
    progress: Res<crate::save::Progress>,
    mut standing: Query<&mut Transform, With<Player>>,
) {
    // Continuing: exactly where they left off, facing the way they left off
    // facing. Landing a returning player at the ranch every time would make
    // Continue a slower New Game.
    let (spawn, facing) = if let Some(save) = &progress.from {
        let ground = terrain.height(save.at.x, save.at.z);
        // The ground rather than the stored height. A save carries a Y, but the
        // world under it can be resculpted between sittings — and a warden
        // restored to last week's height stands in the air or inside a hill.
        let at = Vec3::new(save.at.x, ground, save.at.z);
        info!("continuing at {:.0}, {:.0}", at.x, at.z);
        (at, save.facing)
    } else {
        // On the ranch, which is where the game begins. `find_spawn` is kept as
        // the fallback for a world whose map does not put land there — a redrawn
        // map could leave the pinned spot at sea, and dropping the warden into
        // the water with no explanation is worse than starting them somewhere
        // arbitrary.
        let ranch = Vec2::new(RANCH_AT.0, RANCH_AT.1);
        let on_land = terrain.height(ranch.x, ranch.y) > SEA_LEVEL + 1.0;
        let spawn = if on_land {
            Vec3::new(ranch.x, terrain.height(ranch.x, ranch.y), ranch.y)
        } else {
            warn!("the ranch at {:.0}, {:.0} is under water on this map", ranch.x, ranch.y);
            find_spawn(&terrain, &bounds)
        };
        info!("warden spawning at {:.0}, {:.0}", spawn.x, spawn.z);
        (spawn, 0.0)
    };

    // A warden already standing — the second visit to Playing this session — is
    // MOVED, not doubled: the body is built once and this is where it goes now.
    if let Some(mut warden) = standing.iter_mut().next() {
        warden.translation = spawn;
        warden.rotation = Quat::from_rotation_y(facing);
        return;
    }
    raise_the_warden(&mut commands, &assets, &look, spawn, facing);
}

/// Stands the warden up, wherever they are starting from.
///
/// One body, built once. Both ways into the world need it — a new game at the
/// ranch and a continued one where the save left off — and a warden assembled in
/// two places is a warden that grows a hat in one of them.
fn raise_the_warden(
    commands: &mut Commands,
    assets: &AssetServer,
    look: &crate::look::Look,
    spawn: Vec3,
    facing: f32,
) {
    // The body, and the hair and hat as children of it.
    //
    // Three files rather than one because a hairstyle and a hat are CHOICES: the
    // body is the same model whichever hair is on it, and a hat is authored to sit
    // over a wig rather than instead of one. They are parented to the warden, so
    // they travel and turn with them without anything having to keep them in step.
    let body: Handle<Scene> =
        assets.load(GltfAssetLabel::Scene(0).from_asset(look.build.model()));

    // The body hangs on a child of its own, carrying the scale and the turn that
    // make this particular file fit the world. The warden's OWN transform stays
    // clean: it is what the walk moves, what the camera follows and what the save
    // records, and none of that wants a model's quirks baked into it.
    let fitted = Transform::from_scale(Vec3::splat(
        crate::look::TALL / look.build.authored_height(),
    ))
    .with_rotation(Quat::from_rotation_y(look.build.turn()));

    commands
        .spawn((
            Player,
            // Pushes the grass aside as they go. About the width of a person
            // plus an arm — what actually brushes past is wider than what walks.
            crate::shade::Wades { reach: 1.8 },
            // Its parts are painted as the scene brings them in — see
            // `look::paint_the_warden`. Nothing can be painted at spawn: a glTF
            // scene is instanced asynchronously and none of it exists yet.
            crate::look::Dressing,
            Striding::default(),
            Transform::from_translation(spawn).with_rotation(Quat::from_rotation_y(facing)),
            Visibility::default(),
        ))
        .with_children(|parent| {
            parent.spawn((SceneRoot(body), fitted, Visibility::default()));
            // A model that arrives dressed keeps its own hair, and a cap over
            // modelled hair is two heads of it.
            if look.build.dressed() {
                return;
            }
            if let Some(style) = look.hair.model() {
                let hair: Handle<Scene> =
                    assets.load(GltfAssetLabel::Scene(0).from_asset(style));
                parent.spawn((
                    SceneRoot(hair),
                    crate::look::WornOnTheHead,
                    Transform::default(),
                    Visibility::default(),
                ));
            }
            if let Some(worn) = look.hat.model() {
                let hat: Handle<Scene> = assets.load(GltfAssetLabel::Scene(0).from_asset(worn));
                parent.spawn((
                    SceneRoot(hat),
                    crate::look::WornOnTheHead,
                    Transform::default(),
                    Visibility::default(),
                ));
            }
        });
}

pub fn move_player(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mode: Res<CameraMode>,
    terrain: Res<TerrainSource>,
    bounds: Res<WorldBounds>,
    cameras: Query<&Transform, (With<MainCamera>, Without<Player>)>,
    mut players: Query<(&mut Transform, &mut Striding), With<Player>>,
) {
    // In free-fly the same keys drive the camera instead.
    if *mode == CameraMode::Fly {
        return;
    }
    let (Some(camera), Ok((mut transform, mut pace))) = (cameras.iter().next(), players.single_mut())
    else {
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
    if direction == Vec3::ZERO {
        pace.speed = 0.0;
    }
    if direction != Vec3::ZERO {
        // Jogging is the DEFAULT and walking is the deliberate choice, which is the
        // way round every game this one is measured against does it. Shift sprints,
        // Ctrl slows to a walk.
        let speed = if keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]) {
            SPRINT_SPEED
        } else if keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]) {
            WALK_SPEED
        } else {
            JOG_SPEED
        };
        let next = transform.translation + direction * speed * time.delta_secs();
        let next = bounds.clamp(next, 2.0);

        // The step, or the best part of it: a refused step is retried along each
        // axis alone, so brushing a canyon wall at an angle slides along it
        // rather than sticking to it.
        let from = transform.translation;
        let step = [
            next,
            Vec3::new(next.x, from.y, from.z),
            Vec3::new(from.x, from.y, next.z),
        ]
        .into_iter()
        .find(|to| *to != from && may_step(&terrain.0, from, *to));
        let before = transform.translation;
        if let Some(to) = step {
            transform.translation = to;
        }
        // From what actually happened, not from what was asked.
        let went = transform.translation.distance(before);
        pace.speed = if time.delta_secs() > 0.0 {
            went / time.delta_secs()
        } else {
            0.0
        };

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::save::{Progress, Save};
    use bevy::state::app::StatesPlugin;

    /// The way east is WALKABLE end to end, with the climb rule in force.
    ///
    /// The gate has to refuse the walls and pass the floor. The wall half is the
    /// test below; this is the other half, and it is the one that would strand a
    /// player: a canyon nobody can walk is not a gate, it is a full stop.
    #[test]
    fn the_canyon_can_be_walked_from_the_desert_to_the_green_world() {
        let terrain = crate::world::terrain::Terrain::new();
        let stand = |flat: Vec2| Vec3::new(flat.x, terrain.height(flat.x, flat.y), flat.y);

        // Along the canyon's own way, at a stride a walk actually takes.
        let mut at = crate::world::pass::way_through(-320.0);
        let mut refused = 0;
        let mut worst = 0.0_f32;
        for step in -319..=320 {
            let next = crate::world::pass::way_through(step as f32);
            if may_step(&terrain, stand(at), stand(next)) {
                at = next;
            } else {
                refused += 1;
                let rise = terrain.height(next.x, next.y) - terrain.height(at.x, at.y);
                worst = worst.max(rise);
            }
        }
        assert_eq!(
            refused, 0,
            "{refused} steps along the canyon are refused, the worst a {worst:.1} m rise"
        );
        let out = crate::world::pass::way_through(320.0);
        assert!(
            at.distance(out) < 1.0,
            "the walk stopped {:.0} m short of the eastern mouth",
            at.distance(out)
        );
    }

    /// A canyon wall is a WALL to the walk: the step up it is refused, the step
    /// back down is not, and walking along the floor is untouched.
    ///
    /// Terrain is the game's only geometry and nothing else stops a walker — so
    /// without this, the warden strolls up a seventy-degree face and the canyon
    /// gates nothing at all.
    #[test]
    fn a_canyon_wall_refuses_the_step_up_but_never_the_step_down() {
        let terrain = crate::world::terrain::Terrain::new();
        let middle = crate::world::pass::way_through(40.0);
        let ahead = crate::world::pass::way_through(43.0);
        let (sin, cos) = crate::world::pass::HEADING.sin_cos();
        // Toward negative across: the plain slot wall, clear of the junctions.
        let out = -Vec2::new(-sin, cos);
        let stand = |flat: Vec2| Vec3::new(flat.x, terrain.height(flat.x, flat.y), flat.y);

        // Walk out from the middle to where the wall starts, so the step under
        // test is the one that leaves the floor — the gap's width is a tuning
        // number and this test must not care what it currently is.
        let floor = terrain.height(middle.x, middle.y);
        let mut foot = middle;
        for step in 1..200 {
            let at = middle + out * step as f32;
            if terrain.height(at.x, at.y) > floor + 2.0 {
                break;
            }
            foot = at;
        }
        let into_wall = foot + out * 1.5;
        assert!(
            (terrain.height(foot.x, foot.y) - floor).abs() < 2.5,
            "the scan never found the canyon floor"
        );
        assert!(
            may_step(&terrain, stand(middle), stand(ahead)),
            "walking along the canyon floor is refused"
        );
        assert!(
            !may_step(&terrain, stand(foot), stand(into_wall)),
            "the wall let the warden walk up it — the canyon gates nothing"
        );
        assert!(
            may_step(&terrain, stand(into_wall), stand(foot)),
            "the way back DOWN the wall is refused — a slope became a trap"
        );
    }

    /// The whole continue-and-new-game flow, run rather than reasoned about.
    ///
    /// # The bug this pins was invisible to every other test
    ///
    /// `spawn_player` ran at `Startup`, when the menu had not run and
    /// `Progress::from` was still empty — so Continue never applied the saved
    /// position, the autosave then overwrote the real save with the ranch, and
    /// New Game mid-session left the old warden standing where they were. Every
    /// piece worked alone; the fault was WHEN they ran against each other.
    #[test]
    fn continuing_stands_the_warden_where_the_save_says_and_new_game_at_the_ranch() {
        let terrain = crate::world::terrain::Terrain::new();
        let half = terrain.half();

        let mut app = App::new();
        app.add_plugins((
            bevy::app::TaskPoolPlugin::default(),
            bevy::asset::AssetPlugin::default(),
            StatesPlugin,
        ))
        .init_state::<AppState>()
        .init_asset::<Mesh>()
        // The warden is a glTF scene now. Without `Assets<Scene>` registered,
        // allocating a handle for one panics inside the asset server rather than
        // failing to load — which reads as a mystery rather than a missing plugin.
        .init_asset::<Scene>()
        .init_asset::<crate::shade::Shaded>()
        .insert_resource(TerrainSource(std::sync::Arc::new(terrain)))
        .insert_resource(WorldBounds {
            half,
            min_chunk: IVec2::ZERO,
            max_chunk: IVec2::ZERO,
        })
        .init_resource::<Progress>()
        // The warden is a model now, and raising one reads how it should look.
        .init_resource::<crate::look::Look>()
        .add_systems(OnEnter(AppState::Playing), spawn_player);

        // A save from somewhere that is NOT the ranch, with a stale height —
        // the world may have been resculpted since it was written.
        let ranch = Vec2::new(RANCH_AT.0, RANCH_AT.1);
        let elsewhere = ranch + Vec2::new(400.0, 250.0);
        app.world_mut().resource_mut::<Progress>().from = Some(Save {
            at: Vec3::new(elsewhere.x, 9_999.0, elsewhere.y),
            facing: 1.25,
            played: 60.0,
            stamped: String::new(),
        });

        // Continue: into the world.
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Playing);
        app.update();

        let standing = |app: &mut App| {
            let mut wardens = app.world_mut().query_filtered::<&Transform, With<Player>>();
            let all: Vec<Transform> = wardens.iter(app.world()).copied().collect();
            assert_eq!(all.len(), 1, "there are {} wardens standing", all.len());
            all[0]
        };
        let warden = standing(&mut app);
        assert!(
            (warden.translation.x - elsewhere.x).abs() < 0.01
                && (warden.translation.z - elsewhere.y).abs() < 0.01,
            "Continue put the warden at {:?}, not at the save",
            warden.translation
        );
        assert!(
            warden.translation.y < 5_000.0,
            "the save's stale height was believed: {:.0}",
            warden.translation.y
        );

        // Back to the menu, then NEW GAME: the save is dropped and the SAME
        // warden — not a second one — stands at the ranch again.
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Menu);
        app.update();
        app.world_mut().resource_mut::<Progress>().from = None;
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Playing);
        app.update();

        let warden = standing(&mut app);
        assert!(
            (warden.translation.x - ranch.x).abs() < 0.01
                && (warden.translation.z - ranch.y).abs() < 0.01,
            "New Game left the warden at {:?} instead of the ranch",
            warden.translation
        );
    }
}
