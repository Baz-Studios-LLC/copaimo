//! The workbench: composing a building out of parts, away from the world.
//!
//! What a part IS and how a work becomes a file is [`crate::build::kit`]. This is
//! the room you stand in to do it: a floor to build on, a cursor that snaps, and
//! the keys.
//!
//! # Why it is a room and not a corner of the terrain tool
//!
//! Shaping a hillside and placing a fence rail are different jobs at different
//! scales, and a tool trying to be both has two sets of controls fighting over one
//! mouse. What joins them is the placed sheet: the bench makes a building, the
//! terrain tool stands it somewhere.
//!
//! # It shares nothing with the world
//!
//! No terrain, no streaming, no weather, no meadow — and, deliberately, not even
//! the world's MATERIAL. Everything the game draws wears `shade::Shaded`, which
//! carries the cloud-shadow uniforms; the bench is a room with two lamps in it and
//! has no clouds to be shadowed by, so it uses a plain standard material and owes
//! the world nothing.
//!
//! The single connection runs the other way: what is made here is BAKED into the
//! buildings folder, and the game reads it as an asset like any other. That is the
//! whole of the coupling, and it is a file rather than a dependency.
//!
//! A building is a few dozen boxes, so the whole room redraws from scratch every
//! time anything changes — see `rebuild`, the simplest thing that can possibly
//! work and affordable at this size.

use bevy::prelude::*;
use bevy::gizmos::AppGizmoBuilder;
use bevy::render::view::RenderLayers;
use bevy::window::PrimaryWindow;

pub mod gizmo;
pub mod kiln;
pub mod panel;
pub mod reference;

use crate::build::kit::{self, Bench, Part, TINTS};
use crate::build::pattern::{self, Pattern};
use crate::build::plan;
use crate::states::AppState;

/// What height the view pivots about: about the middle of a wall, so a building
/// turns about itself rather than about its feet.
///
/// Where the eye SITS is no longer a constant beside this. It was, and the two
/// drifted: the camera opened at one place while the view believed it was at
/// another, so the first orbit jumped the picture. It comes from `View::eye` now,
/// which is the only sum that answers the question.
const PIVOT: f32 = 1.2;

/// The layer the bench draws on, and nothing else does.
///
/// # Why this is a layer and not a tidy-up
///
/// The bench is meant to be a room with the work in it. It kept showing the game
/// world instead — terrain, grass, clouds, the sea — and the fix I reached for
/// first was to stop the world STREAMING while the bench was open. That does not
/// work and could not: what is already spawned goes on being drawn, so the fix
/// depended on the bench never being opened after the world had loaded.
///
/// A layer is not a cleanup, it is a rule. The bench's camera is told to draw
/// this layer and only this layer, so no amount of world left standing can appear
/// in it — there is no order of events that gets a tree into this room.
const BENCH_LAYER: usize = 1;

/// The layer the HANDLES draw on, over everything.
///
/// A handle is a control, not a thing in the room, and it must never be behind
/// anything in it. Drawn on the bench's own layer, a piece could swallow its own
/// arrows — stretch a wall to two modules and its body is 3 m long while the red
/// arrow reaches 0.95 m from the middle, so the arrow sat entirely inside the
/// wall. This layer is drawn by its own camera, after the room, onto a cleared
/// depth buffer, so no piece can ever be in front of a handle.
const HANDLE_LAYER: usize = 2;

/// How far the floor reaches, in modules either way.
const FLOOR_REACH: i32 = 8;

/// Everything the bench has spawned, so it can all be taken away again.
#[derive(Component)]
pub struct OfBench;

/// The bench's own camera.
///
/// Named, because the world keeps one too and `iter().next()` on cameras is a coin
/// toss. The cursor was being aimed down a ray cast from the world's camera, which
/// is how it ended up three kilometres from the bench.
#[derive(Component)]
pub struct BenchEye;

/// The camera that draws the handles over the room.
///
/// It rides the bench eye — same place, same way up — and must never be mistaken
/// for it: aiming a cursor ray through it would work today, because the two are
/// identical, and quietly break the day they are not.
#[derive(Component)]
pub struct HandleEye;

/// The work itself, as drawn. Cleared and rebuilt whenever a piece changes.
#[derive(Component)]
struct Work;

/// What the left button does.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Doing {
    /// Putting pieces down.
    #[default]
    Building,
    /// Changing the colour of pieces already down.
    ///
    /// A mode rather than a modifier key, because painting is something a maker
    /// does for a minute at a time — going round a roof — and holding a key for a
    /// minute is worse than pressing one twice.
    Painting,
}

/// Where the next piece would go, and what it would be.
#[derive(Resource)]
pub struct Hand {
    /// What is in hand, or nothing.
    ///
    /// # An empty cursor is the resting state
    ///
    /// The bench always held a part, so every click anywhere placed one — which
    /// made the move arrows unusable, because reaching for one meant clicking, and
    /// clicking meant building. Guard after guard was added to say where a click
    /// must NOT build, and each was a patch over the same wrong assumption.
    ///
    /// Nothing is held unless a part has been chosen, and placing it empties the
    /// hand again. So the ordinary state of the pointer is harmless: it selects,
    /// it grabs handles, it does nothing by accident. Building is the deliberate
    /// act — choose a part, place it, choose again — which is also how it reads
    /// when somebody watches you do it.
    pub part: Option<Part>,
    /// The last part CHOSEN, which is not the same as the one in hand.
    ///
    /// Placing empties the hand, so the hand alone cannot tell "the maker picked a
    /// wall again" from "the maker has switched to a wall" — and the difference is
    /// whether their chosen colour survives. See [`Self::take`].
    chose: Option<Part>,
    pub at: Vec3,
    pub quarters: u8,
    pub tint: usize,
    pub doing: Doing,
}

impl Hand {
    /// Takes up a part, in the material that part is made of.
    ///
    /// The one way a part gets into the hand — the keys and the panel both come
    /// through here, because "picking a part" has a second half now and two copies
    /// of it would drift the moment one of them gained a third.
    ///
    /// Switching parts takes the new part's own material; picking the SAME part
    /// again leaves the colour alone, so a maker laying a row of dark-wood walls
    /// keeps their choice across every placement.
    pub fn take(&mut self, part: Part) {
        if self.chose != Some(part) {
            self.tint = part.natural();
            self.chose = Some(part);
        }
        self.part = Some(part);
    }
}

impl Default for Hand {
    fn default() -> Self {
        Self {
            part: None,
            chose: None,
            at: Vec3::ZERO,
            quarters: 0,
            tint: 0,
            doing: Doing::Building,
        }
    }
}

/// What was last asked for, and with which seed.
///
/// Kept so that G asks again with a NEW seed and Shift+G asks for a different KIND
/// of thing. Pressing generate twice and getting the same house back would make the
/// feature useless: most of what anybody does with a generator is press it until
/// they like what came out.
#[derive(Resource, Default)]
pub struct Asked {
    pub what: usize,
    pub seed: u32,
}

/// How the camera is looking at the work.
#[derive(Resource)]
struct View {
    /// Radians around the work.
    around: f32,
    /// Radians above it. Clamped, so the camera never goes under the floor or
    /// over the top — both put the work behind the grid and neither is a view
    /// anybody chose.
    pitch: f32,
    /// How far off, in metres.
    away: f32,
    /// What it is looking AT.
    ///
    /// Movable, which is the whole of panning. A camera that can only orbit a
    /// fixed point can inspect one thing from every side and cannot reach the far
    /// end of a long building at all — you end up placing pieces at arm's length
    /// because that is where the orbit happens to reach.
    pivot: Vec3,
}

impl View {
    /// Where the eye sits, from the three numbers that describe the view.
    ///
    /// One place, because the camera is placed from this on open and moved from it
    /// on every orbit — and when those were two sums they disagreed.
    pub fn eye(&self) -> Vec3 {
        let out = Vec3::new(
            self.around.sin() * self.pitch.cos(),
            self.pitch.sin(),
            self.around.cos() * self.pitch.cos(),
        );
        self.pivot + out * self.away
    }
}

impl Default for View {
    fn default() -> Self {
        Self {
            around: 0.6,
            pitch: 0.55,
            away: 13.0,
            pivot: Vec3::Y * PIVOT,
        }
    }
}

/// How close and how far the camera may get.
///
/// Near enough to see a rail's thickness, far enough to hold a tower. Not
/// unbounded: a wheel that can zoom for ever ends up inside the work or in the
/// next county, and both take longer to recover from than they took to reach.
const NEAREST: f32 = 2.5;
const FURTHEST: f32 = 60.0;

/// How far the camera may tip. Just short of straight down and just above the
/// floor, because both ends are degenerate: at the pole the view spins about
/// nothing, and at the floor the work is edge-on.
const LOWEST: f32 = 0.08;
const HIGHEST: f32 = 1.45;

pub struct BenchPlugin;

impl Plugin for BenchPlugin {
    fn build(&self, app: &mut App) {
        // The outline's own gizmo group, aimed at the handle layer — see
        // `gizmo::BenchLines`. Configured here rather than where it is drawn,
        // because a group's layers are a property of the app and not of a frame.
        app.init_gizmo_group::<gizmo::BenchLines>()
            .insert_gizmo_config(
                gizmo::BenchLines,
                bevy::gizmos::config::GizmoConfig {
                    render_layers: RenderLayers::layer(HANDLE_LAYER),
                    ..default()
                },
            )
            .init_resource::<Bench>()
            .init_resource::<Hand>()
            .init_resource::<View>()
            .init_resource::<Asked>()
            .init_resource::<reference::Reference>()
            .init_resource::<gizmo::Holding>()
            .init_resource::<kiln::Firing>()
            .add_systems(
                OnEnter(AppState::Bench),
                (open, reference::open, panel::open),
            )
            .add_systems(OnExit(AppState::Bench), close)
            .add_systems(
                Update,
                (
                    // What the maker asked for.
                    (choose, generate, aim, move_hand),
                    // Then the handles, which get the click BEFORE anything places
                    // with it: they ran after, so a click on an arrow placed a
                    // piece and then took hold. Ordering is not the only guard —
                    // `place` asks whether the arrows took the click — but a
                    // frame's lag between pointing at a handle and the handle
                    // knowing is pointless to inflict.
                    (gizmo::drag, gizmo::choose, place, turn_view, walk_view),
                    // Then what is drawn from it. The handle camera moves last,
                    // after anything that could have moved the bench's eye.
                    (
                        gizmo::show,
                        gizmo::light_handles,
                        gizmo::ride_along,
                        rebuild,
                        draw_ghost,
                        reference::show,
                    ),
                    (kiln::ask, kiln::collect),
                    // The panel: what was pressed, then what everything says.
                    (
                        panel::pressed,
                        panel::pressed_swatch,
                        crate::tools::widget::fold_branches,
                        crate::tools::widget::scroll_panels,
                        crate::tools::widget::light_rows::<panel::Press>,
                        panel::refresh,
                        panel::colour_unsaved,
                        gizmo::outline,
                    ),
                )
                    .chain()
                    .run_if(in_state(AppState::Bench)),
            );
    }
}

/// Stands the room up: a light, a floor, a camera.
fn open(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut bench: ResMut<Bench>,
    mut elsewhere: Query<&mut Camera>,
) {
    // So the first frame draws whatever was already on the bench.
    bench.set_changed();

    // The world's camera is switched off rather than despawned: it belongs to the
    // world and will be wanted again the moment anybody leaves.
    for mut other in &mut elsewhere {
        other.is_active = false;
    }
    commands.spawn((
        OfBench,
        BenchEye,
        Camera3d::default(),
        RenderLayers::layer(BENCH_LAYER),
        // Where the VIEW says, not a constant beside it. The two had drifted —
        // the camera opened at (7, 5, 9) while the view believed it was somewhere
        // else entirely, so the first orbit or zoom jumped the picture.
        Transform::from_translation(View::default().eye()).looking_at(View::default().pivot, Vec3::Y),
    ));
    // And the handle camera: the same eye, drawing only the handle layer, after
    // everything, onto a cleared depth buffer — so no piece can stand in front of
    // its own arrows. The UI rides it too, being the highest-order camera, so the
    // panel stays above the handles in turn.
    commands.spawn((
        OfBench,
        HandleEye,
        Camera3d::default(),
        Camera {
            order: 1,
            clear_color: bevy::render::camera::ClearColorConfig::None,
            ..default()
        },
        RenderLayers::layer(HANDLE_LAYER),
        Transform::from_translation(View::default().eye()).looking_at(View::default().pivot, Vec3::Y),
    ));

    // Two lights and no sun. The bench is indoors as far as anything here is
    // concerned, and a day/night cycle over a workbench would mean building a
    // fence by moonlight because of what time it happens to be.
    commands.spawn((
        OfBench,
        RenderLayers::layer(BENCH_LAYER),
        DirectionalLight {
            illuminance: 6_500.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::default().looking_at(Vec3::new(-0.4, -1.0, -0.6), Vec3::Y),
    ));
    commands.insert_resource(AmbientLight {
        color: Color::srgb(0.80, 0.84, 0.92),
        brightness: 900.0,
        ..default()
    });
    // Black, and nothing behind it. A workbench is a room with the thing you are
    // making in it; a sky belongs to a world, and the one that used to show
    // through was not this room's — it was the game's, still being driven while
    // the bench was open.
    commands.insert_resource(ClearColor(Color::BLACK));

    // The floor of the ROOM: a grid to count squares against, and solid.
    //
    // It was a checker of tiles at 98% of a module, which left a real GAP between
    // every one of them — so a piece placed on a lattice point stood over a hole
    // with black underneath and read as floating. The geometry was flush the whole
    // time; what was missing was floor.
    //
    // Still a checker, because counting modules is what it is FOR. It is simply
    // continuous now: the squares are shades of the same dark surface rather than
    // separate tiles with daylight between them.
    commands.spawn((
        OfBench,
        RenderLayers::layer(BENCH_LAYER),
        Mesh3d(meshes.add(crate::world::stream::as_coloured_mesh(&boards()))),
        // A plain material, not the world's. The bench is indoors as far as
        // anything here is concerned, and cloud shadows over a workbench would be
        // weather in a room.
        MeshMaterial3d(materials.add(StandardMaterial {
            // White, so the grain mixed into the vertices comes through as mixed —
            // the same bargain the terrain and the ground cover both make.
            base_color: Color::WHITE,
            perceptual_roughness: 0.86,
            reflectance: 0.03,
            ..default()
        })),
        Transform::IDENTITY,
    ));
}

/// The room's floor: a continuous checker, welded into one mesh.
///
/// One mesh, one draw call, and no gaps — which is the whole point. Laid as
/// separate tiles it had daylight between them, and a piece standing on a lattice
/// point looked as though it hovered over a hole. It did.
fn boards() -> terrain_core::Geometry {
    let mut floor = terrain_core::Geometry::default();
    let reach = FLOOR_REACH as f32 * kit::MODULE;

    for step_x in -FLOOR_REACH..FLOOR_REACH {
        for step_z in -FLOOR_REACH..FLOOR_REACH {
            let low = Vec2::new(step_x as f32, step_z as f32) * kit::MODULE;
            let shade = if (step_x + step_z).rem_euclid(2) == 0 {
                FLOOR_PALE
            } else {
                FLOOR_DARK
            };
            // Edge to edge. Squares that meet leave a checker; squares that stop
            // short leave holes.
            quad(
                &mut floor,
                low,
                (low + Vec2::splat(kit::MODULE)).min(Vec2::splat(reach)),
                0.0,
                shade,
            );
        }
    }
    floor
}

/// One flat rectangle, face up.
fn quad(into: &mut terrain_core::Geometry, low: Vec2, high: Vec2, y: f32, colour: [f32; 4]) {
    let base = into.places.len() as u32;
    for (x, z) in [
        (low.x, low.y),
        (high.x, low.y),
        (high.x, high.y),
        (low.x, high.y),
    ] {
        into.places.push([x, y, z]);
        into.normals.push([0.0, 1.0, 0.0]);
        into.uvs.push([0.0, 0.0]);
        into.colours.push(colour);
    }
    into.indices
        .extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
}

/// The two shades the room's floor is checked in.
const FLOOR_PALE: [f32; 4] = [0.052, 0.058, 0.076, 1.0];
const FLOOR_DARK: [f32; 4] = [0.034, 0.038, 0.052, 1.0];

fn close(
    mut commands: Commands,
    mine: Query<Entity, With<OfBench>>,
    mut elsewhere: Query<&mut Camera, Without<BenchEye>>,
) {
    for entity in &mine {
        commands.entity(entity).despawn();
    }
    // And the world gets its eye back.
    for mut other in &mut elsewhere {
        other.is_active = true;
    }
}

/// Picking a part, a colour, and which way round.
fn choose(keys: Res<ButtonInput<KeyCode>>, mut hand: ResMut<Hand>) {
    // The kit's own table, which the panel prints from as well — see
    // `kit::PART_KEYS`. Ten parts is one past the digits, so a panel numbering its
    // own rows and an input holding its own keys would already disagree about the
    // last of them.
    for ((key, _), part) in kit::PART_KEYS.iter().zip(Part::ALL) {
        if keys.just_pressed(*key) {
            hand.take(part);
        }
    }
    // R turns the piece in HAND, quarter by quarter. There is no free rotation on
    // purpose — see the kit's own note.
    //
    // Shift+R turns whatever is already down under the cursor. Two keys rather
    // than one that guesses: "turn this" and "turn the next one" are different
    // intentions, and a key that picks between them by whether something happens
    // to be nearby turns the wrong thing exactly when a maker is working fast.
    if keys.just_pressed(KeyCode::KeyR)
        && !keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight])
    {
        hand.quarters = (hand.quarters + 1) % 4;
    }
    if keys.just_pressed(KeyCode::KeyC) {
        hand.tint = (hand.tint + 1) % TINTS.len();
    }
    // Building or painting. A mode rather than a held modifier, because painting
    // is something a maker does for a minute at a time — going round a roof — and
    // holding a key for a minute is worse than pressing one twice.
    if keys.just_pressed(KeyCode::KeyP) {
        hand.doing = match hand.doing {
            Doing::Building => Doing::Painting,
            Doing::Painting => Doing::Building,
        };
    }
}

/// Asking for something to be built.
///
/// `G` asks again with a new seed, the same kind of thing, because most of what
/// anybody does with a generator is press it until they like what came out.
/// `Shift+G` moves on to a different kind. What arrives is ordinary pieces, so the
/// next thing you do is take a wall out and widen the door.
fn generate(keys: Res<ButtonInput<KeyCode>>, mut asked: ResMut<Asked>, mut bench: ResMut<Bench>) {
    if !keys.just_pressed(KeyCode::KeyG) {
        return;
    }
    if keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]) {
        asked.what = (asked.what + 1) % Pattern::ALL.len();
    }
    // A different seed every time, and repeatable across runs. The clock would do
    // and cannot be read from here; the count of asks is as good and stays the
    // same between sessions, which is worth more than being unpredictable.
    asked.seed = asked.seed.wrapping_add(1);
    let what = Pattern::ALL[asked.what];
    pattern::draw(&mut bench, what, asked.seed);
    info!(
        "asked for a {} ({}), {} pieces",
        what.name(),
        asked.seed,
        bench.len()
    );
}

/// Aiming with the mouse.
///
/// # The mouse proposes; the lattice disposes
///
/// A ray through the pointer, met against the horizontal plane the cursor is
/// already on, and then **snapped**. That last word is the whole design.
///
/// It would be easy to let the mouse place freely and call it precision. It would
/// also throw away the thing that makes the kit work: every part is a multiple of
/// the snap, so pieces abut exactly and a wall meets a floor without anybody
/// measuring. Free placement gives you walls a centimetre apart and a building
/// with hairline gaps you can see through — the kind of fault that is invisible
/// while you build and obvious in the finished thing.
///
/// So the mouse says which cell, and the cell says where. The keys still work and
/// still nudge, and whichever moved last wins — a mouse that overrode the keys
/// every frame would make them useless.
fn aim(
    keys: Res<ButtonInput<KeyCode>>,
    over_panel: Query<&Interaction>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<BenchEye>>,
    mut hand: ResMut<Hand>,
    mut was: Local<Option<(Vec2, Vec3)>>,
) {
    let (Some(window), Some((camera, eye))) = (windows.iter().next(), cameras.iter().next()) else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let fine = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    // Reaching for the panel, not aiming at the floor. Letting the cursor follow
    // would slide it across the work every time somebody went for a button.
    if over_panel
        .iter()
        .any(|touch| matches!(touch, Interaction::Hovered | Interaction::Pressed))
    {
        return;
    }
    // Which lattice the thing in hand sits on: cells, or the joins between them.
    // See `Part::off_the_grid`.
    let lean = hand
        .part
        .map_or(Vec3::ZERO, |part| part.off_the_grid(hand.quarters));

    // Only when the pointer has actually moved — otherwise every key nudge would be
    // undone on the very next frame by a mouse sitting still — or when the lattice
    // itself has changed under it, which is what taking up a different part or
    // turning the one in hand does.
    if *was == Some((cursor, lean)) {
        return;
    }
    *was = Some((cursor, lean));

    let Ok(ray) = camera.viewport_to_world(eye, cursor) else {
        return;
    };
    // The plane the cursor is already at, so raising the cursor with Q and E
    // builds a storey up rather than aiming at the floor from a worse angle.
    let plane = InfinitePlane3d::new(Vec3::Y);
    let Some(along) = ray.intersect_plane(Vec3::Y * hand.at.y, plane) else {
        return;
    };
    // Within the room, and only just outside it.
    //
    // A ray aimed near the horizon meets a horizontal plane at a grazing angle and
    // strikes it a very long way off — the cursor read three kilometres from the
    // bench, which is a number a maker cannot even see, let alone build at. The
    // floor is what there is; pointing past it holds the cursor at its edge rather
    // than following the ray wherever the arithmetic says.
    let struck = ray.get_point(along);
    let edge = FLOOR_REACH as f32 * kit::MODULE;
    let held = Vec3::new(
        struck.x.clamp(-edge, edge),
        hand.at.y,
        struck.z.clamp(-edge, edge),
    );
    // To the module unless the fine key is held, for the same reason the keys step
    // that way: what a maker is nearly always doing is putting a piece beside
    // another piece. Snapped on the LEANED lattice, so a wall lands on a join.
    let snapped = if fine {
        Bench::snapped(held)
    } else {
        Bench::snapped_to(held - lean, kit::MODULE) + lean
    };
    // The height is the maker's, not the snap's. Rounding it to the module put the
    // cursor back on the ground every time the mouse moved, so raising it a
    // quarter-metre to clear a floor was undone before anything could be placed.
    let snapped = Vec3::new(snapped.x, hand.at.y, snapped.z);
    if snapped != hand.at {
        hand.at = snapped;
    }
}

/// Moving the cursor about the bench.
///
/// Keys rather than the mouse, and that is a deliberate choice rather than a
/// shortcut. A building is placed on a lattice, and a lattice is what a keyboard is
/// good at: press once, move one snap, know exactly where you are. Aiming a mouse
/// at a 25 cm cell from across a room is a fight, and every builder that offers
/// both ends up with people using the keys.
fn move_hand(keys: Res<ButtonInput<KeyCode>>, view: Res<View>, mut hand: ResMut<Hand>) {
    // Relative to how the camera is looking, so "left" is left on screen rather
    // than west on a compass nobody can see.
    let quarter = (view.around / std::f32::consts::FRAC_PI_2).round() as i32;
    let (ahead, aside) = match quarter.rem_euclid(4) {
        0 => (Vec3::NEG_Z, Vec3::X),
        1 => (Vec3::X, Vec3::Z),
        2 => (Vec3::Z, Vec3::NEG_X),
        _ => (Vec3::NEG_X, Vec3::NEG_Z),
    };

    let mut step = Vec3::ZERO;
    if keys.just_pressed(KeyCode::ArrowUp) {
        step += ahead;
    }
    if keys.just_pressed(KeyCode::ArrowDown) {
        step -= ahead;
    }
    if keys.just_pressed(KeyCode::ArrowRight) {
        step += aside;
    }
    if keys.just_pressed(KeyCode::ArrowLeft) {
        step -= aside;
    }
    if keys.just_pressed(KeyCode::KeyE) || keys.just_pressed(KeyCode::Space) {
        step += Vec3::Y;
    }
    if keys.just_pressed(KeyCode::KeyQ) {
        step -= Vec3::Y;
    }
    if step == Vec3::ZERO {
        return;
    }

    // A whole module by default, a quarter-metre with SHIFT held.
    //
    // This was the other way round, and it is most of why pieces did not snap
    // together: a wall is a module wide, so a cursor stepping a quarter of one put
    // walls a quarter-metre apart far more often than it put them touching. The
    // common case is a piece beside a piece, and the common case is what a default
    // is for. The fine step is still there for the times it is genuinely wanted —
    // a post half a module off a wall — behind a key you hold on purpose.
    let reach = if keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]) {
        kit::SNAP
    } else {
        kit::MODULE
    };
    hand.at = Bench::snapped(hand.at + step * reach);
    // Never below the floor. Nothing is built under the ground here and a piece
    // that vanished under it would look lost rather than low.
    hand.at.y = hand.at.y.max(0.0);
}

/// Putting a piece down, taking one back, and saving the work.
/// Whether a click on the mouse belongs to the WORK, or to something in front of
/// it.
///
/// # One rule, in one place
///
/// A click lands on whatever is nearest the eye and stops there. That is obvious
/// and it is exactly what a tool with layers over its world keeps getting wrong,
/// twice here already: pressing WALL in the panel also put a wall down behind it,
/// and taking hold of a move arrow dropped a piece before it grabbed. Both times
/// the fix was an `if` in one system, and both times the next layer added had to
/// remember to write its own.
///
/// So it is one function. Anything that comes to sit over the work adds itself
/// here, and everything that acts on a click asks here — rather than each caller
/// keeping its own list of what might be in the way.
fn reaches_the_work(on_panel: bool, on_a_handle: bool) -> bool {
    !on_panel && !on_a_handle
}

fn place(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    over_panel: Query<&Interaction>,
    holding: Res<gizmo::Holding>,
    mut hand: ResMut<Hand>,
    mut bench: ResMut<Bench>,
    mut next: ResMut<NextState<AppState>>,
) {
    // Whether this click is the work's at all — see `reaches_the_work`, which is
    // where that question is answered for good.
    let on_panel = over_panel.iter().any(|touch| *touch != Interaction::None);
    if !reaches_the_work(on_panel, holding.on_a_handle()) {
        return;
    }

    // The left button and ENTER do the same thing, and what that is depends on the
    // mode. The mouse is the one anybody will use; the key is there because a
    // maker nudging the cursor with W and A should not have to reach for the mouse
    // to put the piece down.
    let go = keys.just_pressed(KeyCode::Enter) || buttons.just_pressed(MouseButton::Left);
    let mut emptied = false;
    if go {
        match hand.doing {
            Doing::Building => {
                // Only with something in hand. An empty cursor is the resting
                // state and it builds nothing — see `Hand::part`.
                if let Some(part) = hand.part {
                    // Where the piece actually goes: tucked onto what holds it up
                    // and standing on what is under it — see `Bench::settling`, which
                    // the ghost in hand reads too.
                    let foot = bench.settling(part, hand.at, hand.quarters);
                    if bench.add(part, foot, hand.quarters, hand.tint).is_some() {
                        // And the hand empties. Placing is one deliberate act, not
                        // a mode you are left in afterwards.
                        emptied = true;
                    }
                }
            }
            Doing::Painting => {
                bench.paint_nearest(hand.at, kit::MODULE, hand.tint);
            }
        }
    }
    if emptied {
        hand.part = None;
    }
    // Turning something already down. Getting a wall's facing wrong is the
    // commonest mistake on a lattice, and before this the only remedy was to
    // delete it and place it again.
    if keys.just_pressed(KeyCode::KeyR)
        && keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight])
    {
        bench.turn_nearest(hand.at, kit::MODULE);
    }

    // Taking away is the same gesture in both modes. There is no such thing as
    // un-painting, and a right button that did nothing in one mode would read as
    // broken.
    if keys.just_pressed(KeyCode::Delete)
        || keys.just_pressed(KeyCode::Backspace)
        || buttons.just_pressed(MouseButton::Right)
    {
        // Whatever is nearest the cursor, within a module — the same rule the
        // terrain tool follows for taking things away.
        bench.remove_nearest(hand.at, kit::MODULE);
    }
    if keys.just_pressed(KeyCode::KeyZ) && keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight])
    {
        bench.undo();
    }
    if keys.just_pressed(KeyCode::KeyS) && keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight])
    {
        match kit::save(&mut bench) {
            Ok(path) => info!("saved the work to {}", path.display()),
            Err(why) => error!("could not save the work: {why}"),
        }
    }
    if keys.just_pressed(KeyCode::Escape) {
        // Unsaved work is kept in the resource, so leaving and coming back finds
        // it where it was. Nothing is thrown away by walking out of the room.
        next.set(AppState::Menu);
    }
}

/// Turning the view about the work, and stepping in and out.
/// Walking the view about with WASD.
///
/// # The mouse already aims; the keys should carry you
///
/// WASD nudged the CURSOR, which the mouse was already doing better — so the
/// keyboard duplicated the pointer and nothing moved the camera except a
/// modifier-and-middle-drag nobody would guess. A view you can only turn, from a
/// spot you cannot leave, is a view of one thing.
///
/// So the keys walk the camera and the mouse aims, which is the division every
/// tool of this kind settles on. The arrow keys still nudge the cursor for the
/// times a cell has to be hit exactly.
fn walk_view(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut view: ResMut<View>,
    mut cameras: Query<&mut Transform, With<BenchEye>>,
) {
    let (sin, cos) = view.around.sin_cos();
    let right = Vec3::new(cos, 0.0, -sin);
    let ahead = Vec3::new(sin, 0.0, cos);

    let mut going = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) {
        going -= ahead;
    }
    if keys.pressed(KeyCode::KeyS) {
        going += ahead;
    }
    if keys.pressed(KeyCode::KeyA) {
        going -= right;
    }
    if keys.pressed(KeyCode::KeyD) {
        going += right;
    }
    if going == Vec3::ZERO {
        return;
    }

    // Held, not tapped, and by the second rather than the frame — so crossing a
    // big building takes the same time on any machine.
    //
    // Scaled by how far off the camera is, for the same reason the pan drag is: a
    // step should cross the same share of what you can SEE whether you are close
    // in on a rail or stood back from a tower.
    let rate = view.away * WALK_RATE * time.delta_secs();
    view.pivot += going.normalize() * rate;
    for mut camera in &mut cameras {
        camera.translation = view.eye();
        camera.look_at(view.pivot, Vec3::Y);
    }
}

fn turn_view(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut moved: EventReader<bevy::input::mouse::MouseMotion>,
    scroll: Res<bevy::input::mouse::AccumulatedMouseScroll>,
    windows: Query<&Window, With<PrimaryWindow>>,
    panels: Query<(&ComputedNode, &GlobalTransform), With<crate::tools::widget::Scrolls>>,
    mut view: ResMut<View>,
    mut cameras: Query<&mut Transform, With<BenchEye>>,
) {
    // Drag the MIDDLE button to orbit.
    //
    // Not the left or the right: those place and take away, and they have to stay
    // that way — a tool where the button that builds also moves the camera is a
    // tool where every misjudged drag leaves a wall somewhere. Middle-drag to
    // orbit is what every program that does this uses, which is the whole reason
    // to use it.
    let dragging = buttons.pressed(MouseButton::Middle);
    // SHIFT with it pans instead of orbiting. The same button, because it is the
    // same gesture — take hold of the view and move it — and every program that
    // does both puts them on one button with a modifier between.
    let panning = dragging && keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    let mut swung = Vec2::ZERO;
    for motion in moved.read() {
        if dragging {
            swung += motion.delta;
        }
    }
    let mut shifted = false;
    if swung != Vec2::ZERO && panning {
        // Across the screen, not across the world: dragging right moves what you
        // are looking at to the right, whichever way the camera happens to face.
        // Panning in world axes is the thing that makes a camera feel broken.
        let (sin, cos) = view.around.sin_cos();
        let right = Vec3::new(cos, 0.0, -sin);
        let ahead = Vec3::new(sin, 0.0, cos);
        // Scaled by how far off the camera is, so a drag moves the same distance
        // ACROSS THE SCREEN whether you are close in or far out.
        let rate = view.away * PAN_RATE;
        view.pivot += (right * -swung.x + ahead * swung.y) * rate;
        // Never under the floor, for the same reason the pitch is clamped.
        view.pivot.y = view.pivot.y.max(0.0);
        shifted = true;
    } else if swung != Vec2::ZERO {
        view.around -= swung.x * ORBIT_RATE;
        // Up drags the eye UP. The other way round is defensible and it is not
        // what anybody expects when they take hold of a thing and lift it.
        view.pitch = (view.pitch + swung.y * ORBIT_RATE).clamp(LOWEST, HIGHEST);
        shifted = true;
    }

    // The wheel zooms, by a FACTOR rather than a step.
    //
    // A fixed step is wrong at both ends: it crawls when you are far out and jumps
    // straight through the work when you are close. Multiplying keeps the movement
    // the same fraction of what you can see, which is what makes a zoom feel even.
    //
    // Not over the panel, whose wheel is its scroll. One gesture answering two
    // questions — scroll the shelf AND zoom the room behind it — reads as the
    // tool fighting itself.
    if scroll.delta.y != 0.0 && !crate::tools::widget::pointer_on_a_panel(&windows, &panels) {
        view.away = (view.away * ZOOM_RATE.powf(-scroll.delta.y)).clamp(NEAREST, FURTHEST);
        shifted = true;
    }

    // And the keys still work. Quarter turns, because a building of axis-aligned
    // parts is looked at from its corners, and getting exactly back to one by
    // dragging is a fiddle.
    if keys.just_pressed(KeyCode::BracketLeft) {
        view.around = quarter_from(view.around, -1.0);
        shifted = true;
    }
    if keys.just_pressed(KeyCode::BracketRight) {
        view.around = quarter_from(view.around, 1.0);
        shifted = true;
    }
    if keys.just_pressed(KeyCode::Minus) {
        view.away = (view.away * ZOOM_RATE).min(FURTHEST);
        shifted = true;
    }
    if keys.just_pressed(KeyCode::Equal) {
        view.away = (view.away / ZOOM_RATE).max(NEAREST);
        shifted = true;
    }
    if !shifted {
        return;
    }

    for mut camera in &mut cameras {
        camera.translation = view.eye();
        camera.look_at(view.pivot, Vec3::Y);
    }
}

/// The next quarter turn round from where the camera actually is.
///
/// Rounded to the quarter first, so a camera dragged to some angle between two of
/// them lands on a corner rather than a quarter turn from wherever it happened to
/// be left. Pressing the key twice from anywhere gets you to a known view.
fn quarter_from(around: f32, way: f32) -> f32 {
    let quarter = std::f32::consts::FRAC_PI_2;
    ((around / quarter).round() + way) * quarter
}

/// How far a pixel of drag turns the view, pans it, and how much a notch zooms.
///
/// The pan is a share of the DISTANCE rather than a fixed number of metres, so a
/// drag moves the same distance across the screen close in and far out.
const ORBIT_RATE: f32 = 0.006;
const PAN_RATE: f32 = 0.0016;

/// How fast WASD walks the view, as a share of the distance each second.
const WALK_RATE: f32 = 0.9;
const ZOOM_RATE: f32 = 1.18;

/// Draws the work, from scratch, whenever it changes.
///
/// The whole thing every time rather than a diff. A building is a few dozen boxes,
/// so rebuilding costs nothing measurable — and one code path answers "what is on
/// the bench" whether a piece was added, removed, or the file was reloaded. A diff
/// that is subtly wrong leaves a rail nobody can delete.
fn rebuild(
    mut commands: Commands,
    bench: Res<Bench>,
    standing: Query<Entity, With<Work>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if !bench.is_changed() {
        return;
    }
    for entity in &standing {
        commands.entity(entity).despawn();
    }
    if bench.is_empty() {
        return;
    }

    // Through the game's own building renderer, on the game's own format. The
    // bench has no geometry of its own, which is what makes what you see here what
    // you get when it is raised in the world.
    let plan = bench.to_plan();
    let (solid, glass) = crate::build::shape::raise(&plan);
    let cloth = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.88,
        reflectance: 0.05,
        ..default()
    });

    if !solid.is_empty() {
        commands.spawn((
            OfBench,
            Work,
            RenderLayers::layer(BENCH_LAYER),
            Mesh3d(meshes.add(solid.into_mesh())),
            MeshMaterial3d(cloth.clone()),
            Transform::IDENTITY,
        ));
    }
    if !glass.is_empty() {
        commands.spawn((
            OfBench,
            Work,
            RenderLayers::layer(BENCH_LAYER),
            Mesh3d(meshes.add(glass.into_mesh())),
            MeshMaterial3d(cloth),
            Transform::IDENTITY,
        ));
    }
}

/// The piece in hand, drawn where it would land.
#[derive(Component)]
struct Ghost;

fn draw_ghost(
    mut commands: Commands,
    hand: Res<Hand>,
    bench: Res<Bench>,
    ghosts: Query<Entity, With<Ghost>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Redrawn when the WORK changes as well as when the hand does: where a piece
    // would come to rest depends on what is already standing there, so putting a
    // floor down moves the ghost of the wall about to go on it.
    if !hand.is_changed() && !bench.is_changed() && !ghosts.is_empty() {
        return;
    }
    for entity in &ghosts {
        commands.entity(entity).despawn();
    }
    // Nothing in hand, nothing to preview. An empty cursor showing a ghost would
    // be saying a click will build when it will not.
    let Some(part) = hand.part else {
        return;
    };

    // The part itself, shown as it will be rather than as a box around it — a
    // wedge that previews as a cuboid puts the roof on the wrong way round about
    // half the time.
    // WHERE IT WILL LAND, not where the cursor is. The ghost showed a wall buried
    // in the floor and the wall then stood on top of it — a preview that lies about
    // the one thing it is for.
    let foot = bench.settling(part, hand.at, hand.quarters);
    let one = plan::Plan {
        name: String::new(),
        kind: String::new(),
        half_w: 0.0,
        half_d: 0.0,
        high: 0.0,
        boxes: vec![plan::Block {
            at: foot + Vec3::Y * part.size().y * 0.5,
            size: part.size(),
            turn: Quat::from_rotation_y(hand.quarters as f32 * std::f32::consts::FRAC_PI_2),
            form: part.form(),
            colour: Color::WHITE,
            stage: String::new(),
        }],
        marks: Vec::new(),
    };
    let (solid, _) = crate::build::shape::raise(&one);
    if solid.is_empty() {
        return;
    }

    let [r, g, b] = TINTS[hand.tint.min(TINTS.len() - 1)].1;
    commands.spawn((
        OfBench,
        Ghost,
        RenderLayers::layer(BENCH_LAYER),
        Mesh3d(meshes.add(solid.into_mesh())),
        MeshMaterial3d(materials.add(StandardMaterial {
            // SOLID, in the colour it will actually be.
            //
            // It was drawn see-through on the idea that "about to be" should look
            // provisional. It reads as a fault instead: every piece in hand looks
            // like a piece that failed to load, and the one thing a maker wants
            // from a preview is to see what they are about to get.
            base_color: Color::srgb_u8(r, g, b),
            // Lit like the work, so it sits in the room rather than glowing in it.
            perceptual_roughness: 0.85,
            ..default()
        })),
        Transform::IDENTITY,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A bench with the real systems in it, and no window.
    ///
    /// # "It needs a window" was not true
    ///
    /// I had been testing the arithmetic and leaving the BEHAVIOUR to be found by
    /// whoever ran the game, on the grounds that input needs a window. It does
    /// not: `place` queries a keyboard, a mouse, some interactions and the bench,
    /// and every one of those is a resource a test can set. What needed a window
    /// was my excuse.
    ///
    /// So the click routing is exercised for real here — the systems, the actual
    /// resources, a simulated press — rather than a pure function that agrees with
    /// itself.
    fn bench_app() -> App {
        let mut app = App::new();
        app.add_plugins(bevy::state::app::StatesPlugin)
            .init_state::<AppState>()
            .init_resource::<Bench>()
            .init_resource::<Hand>()
            .init_resource::<gizmo::Holding>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .add_systems(Update, place);
        app
    }

    /// Puts a part in hand, as choosing one from the panel does.
    fn take_up(app: &mut App, part: kit::Part) {
        app.world_mut().resource_mut::<Hand>().take(part);
    }

    /// Presses the left button, runs one frame, and lets go again.
    ///
    /// Letting go matters: `clear` empties `just_pressed` but leaves the button
    /// HELD, and a press only registers on a button that was up — so without the
    /// release this works once and every click after it is silently nothing.
    fn click(app: &mut App) {
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.update();
        let mut buttons = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
        buttons.release(MouseButton::Left);
        buttons.clear();
    }

    fn pieces(app: &App) -> usize {
        app.world().resource::<Bench>().len()
    }

    #[test]
    fn clicking_the_work_places_what_is_in_hand_and_then_empties_it() {
        // The control, and the new rule in one. Without the first half, every test
        // below would pass on a bench that had quietly stopped placing anything at
        // all.
        let mut app = bench_app();
        take_up(&mut app, kit::Part::Wall);
        click(&mut app);
        assert_eq!(pieces(&app), 1, "a click with a wall in hand placed nothing");

        // Placing is one deliberate act, not a mode you are left in.
        assert_eq!(
            app.world().resource::<Hand>().part,
            None,
            "the hand still held a part after placing it"
        );

        // So the next click builds nothing.
        click(&mut app);
        assert_eq!(pieces(&app), 1, "a second click placed a piece from an empty hand");
    }

    #[test]
    fn an_empty_cursor_builds_nothing() {
        // The resting state, and the whole reason for it. The bench always held a
        // part, so every click anywhere placed one — which made the move arrows
        // unusable, because reaching for one means clicking and clicking meant
        // building. Guard after guard was added to say where a click must NOT
        // build; the answer was that it should not build by default.
        let mut app = bench_app();
        assert_eq!(app.world().resource::<Hand>().part, None, "the bench opened holding something");
        click(&mut app);
        assert_eq!(pieces(&app), 0, "an empty cursor placed a piece");
    }

    #[test]
    fn switching_parts_takes_the_new_part_s_material_and_keeps_a_chosen_one() {
        // The rule that fixes "the foundation is the colour of the wood" without
        // fighting a maker who has picked a colour on purpose.
        let mut app = bench_app();

        take_up(&mut app, kit::Part::Wall);
        assert_eq!(
            app.world().resource::<Hand>().tint,
            kit::Part::Wall.natural(),
            "a wall did not arrive in timber"
        );

        // A deliberate colour, and then the SAME part again — as happens after every
        // placement, because placing empties the hand.
        let dark = kit::TINTS.iter().position(|(name, _)| *name == "dark wood").unwrap();
        app.world_mut().resource_mut::<Hand>().tint = dark;
        take_up(&mut app, kit::Part::Wall);
        assert_eq!(
            app.world().resource::<Hand>().tint,
            dark,
            "picking the same part again threw away the colour the maker chose"
        );

        // A different part brings its own material with it.
        take_up(&mut app, kit::Part::Foundation);
        assert_eq!(
            app.world().resource::<Hand>().tint,
            kit::Part::Foundation.natural(),
            "a foundation did not arrive in stone"
        );
    }

    #[test]
    fn clicking_a_move_arrow_does_not_also_place_a_piece() {
        // The reported bug, run rather than reasoned about: every attempt to move
        // something dropped a post on it first.
        let mut app = bench_app();
        take_up(&mut app, kit::Part::Wall);
        app.world_mut().resource_mut::<gizmo::Holding>().hold_for_test(0);
        click(&mut app);
        assert_eq!(pieces(&app), 0, "taking hold of an arrow placed a piece");

        // And merely being OVER one, which is the state on the very frame the
        // button goes down — nothing is being dragged yet, and that is exactly the
        // frame the click has to be kept off the ground.
        let mut app = bench_app();
        take_up(&mut app, kit::Part::Wall);
        app.world_mut().resource_mut::<gizmo::Holding>().hover_for_test(1);
        click(&mut app);
        assert_eq!(pieces(&app), 0, "clicking on an arrow placed a piece");
    }

    #[test]
    fn clicking_the_panel_does_not_also_place_a_piece() {
        // The same fault one layer out, which this bench has also had: pressing
        // WALL in the panel put a wall down behind it.
        let mut app = bench_app();
        take_up(&mut app, kit::Part::Wall);
        app.world_mut().spawn(Interaction::Pressed);
        click(&mut app);
        assert_eq!(pieces(&app), 0, "a click on the panel reached the work");

        // And a panel the pointer is merely NEAR is not in the way.
        let mut app = bench_app();
        take_up(&mut app, kit::Part::Wall);
        app.world_mut().spawn(Interaction::None);
        click(&mut app);
        assert_eq!(pieces(&app), 1, "an untouched panel blocked the work");
    }

    #[test]
    fn a_click_stops_at_the_nearest_thing_it_lands_on() {
        // Twice now this bench has let one click reach two places. Pressing WALL in
        // the panel also put a wall down behind it, and taking hold of a move arrow
        // dropped a piece before it grabbed — the second one after I had written
        // the failure mode into the gizmo's own comment and then not guarded it.
        //
        // A click lands on whatever is nearest the eye and stops there.
        assert!(reaches_the_work(false, false), "a click on nothing should reach the work");
        assert!(!reaches_the_work(true, false), "a click on the panel reached the work");
        assert!(!reaches_the_work(false, true), "a click on an arrow reached the work");
        assert!(!reaches_the_work(true, true));
    }

    /// Where the camera ends up, which is the arithmetic `turn_view` runs.
    fn eye(view: &View) -> Vec3 {
        let out = Vec3::new(
            view.around.sin() * view.pitch.cos(),
            view.pitch.sin(),
            view.around.cos() * view.pitch.cos(),
        );
        Vec3::Y * PIVOT + out * view.away
    }

    #[test]
    fn the_camera_never_goes_under_the_floor_or_over_the_top() {
        // Both ends are degenerate rather than merely ugly: at the pole the view
        // spins about nothing, and below the floor the work is behind the grid.
        // The clamp is what makes a free orbit safe to hand somebody.
        for tipped in [-9.0_f32, -0.4, 0.0, 0.5, 1.6, 99.0] {
            let view = View {
                pitch: tipped.clamp(LOWEST, HIGHEST),
                ..Default::default()
            };
            assert!(
                view.pitch >= LOWEST && view.pitch <= HIGHEST,
                "pitch {tipped} was not brought back"
            );
            // And the eye is above the floor at every one of them.
            assert!(eye(&view).y > 0.0, "the camera got under the floor at {tipped}");
        }
    }

    #[test]
    fn zooming_is_bounded_at_both_ends() {
        // A wheel that zooms for ever ends up inside the work or in the next
        // county, and both take longer to recover from than they took to reach.
        let mut away = View::default().away;
        for _ in 0..200 {
            away = (away / ZOOM_RATE).max(NEAREST);
        }
        assert!((away - NEAREST).abs() < 1.0e-3, "zoomed in to {away}");
        for _ in 0..400 {
            away = (away * ZOOM_RATE).min(FURTHEST);
        }
        assert!((away - FURTHEST).abs() < 1.0e-3, "zoomed out to {away}");
    }

    #[test]
    fn zooming_moves_by_a_fraction_rather_than_a_step() {
        // A fixed step is wrong at both ends: it crawls when you are far out and
        // jumps through the work when you are close. What makes a zoom feel even
        // is that each notch moves the same SHARE of what you can see.
        let near = 4.0_f32;
        let far = 40.0_f32;
        let moved = |from: f32| (from * ZOOM_RATE - from) / from;
        assert!(
            (moved(near) - moved(far)).abs() < 1.0e-5,
            "a notch moves {:.3} of the view up close and {:.3} far out",
            moved(near),
            moved(far)
        );
    }

    #[test]
    fn squaring_up_lands_on_a_corner_from_anywhere() {
        // Pressing the key twice from any angle has to reach a known view. Adding
        // a quarter to wherever the camera was left would keep whatever fraction
        // of a turn the drag ended on, for ever.
        let quarter = std::f32::consts::FRAC_PI_2;
        for awkward in [0.13_f32, 0.9, -1.7, 2.4, 5.9] {
            let squared = quarter_from(awkward, 1.0);
            let off = (squared / quarter) - (squared / quarter).round();
            assert!(off.abs() < 1.0e-5, "{awkward} squared up to {squared}, off a corner");
        }
        // And it moves in the direction asked for.
        assert!(quarter_from(0.0, 1.0) > 0.0);
        assert!(quarter_from(0.0, -1.0) < 0.0);
    }

    #[test]
    fn the_view_starts_looking_down_at_the_work() {
        // A bench that opens edge-on to the floor, or from directly overhead, is a
        // bench somebody has to fix before they can start.
        let view = View::default();
        let at = eye(&view);
        assert!(at.y > 1.0, "the camera opens {:.2} m up", at.y);
        assert!(
            view.away >= NEAREST && view.away <= FURTHEST,
            "it opens outside its own zoom range"
        );
        // Looking at the work rather than past it: the pivot is between the eye
        // and the far side.
        assert!(at.length() > PIVOT, "the camera opens inside the work");
    }
}
