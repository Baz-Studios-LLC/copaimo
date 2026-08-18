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
use bevy::render::view::RenderLayers;
use bevy::window::PrimaryWindow;

pub mod kiln;
pub mod panel;
pub mod reference;

use crate::build::kit::{self, Bench, Part, TINTS};
use crate::build::pattern::{self, Pattern};
use crate::build::plan;
use crate::states::AppState;

/// How far the camera looks from, and at what height it pivots.
const EYE: Vec3 = Vec3::new(7.0, 5.0, 9.0);
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
struct BenchEye;

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
    pub part: Part,
    pub at: Vec3,
    pub quarters: u8,
    pub tint: usize,
    pub doing: Doing,
}

impl Default for Hand {
    fn default() -> Self {
        Self {
            part: Part::Post,
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
}

impl Default for View {
    fn default() -> Self {
        Self {
            around: 0.6,
            pitch: 0.55,
            away: 13.0,
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
        app.init_resource::<Bench>()
            .init_resource::<Hand>()
            .init_resource::<View>()
            .init_resource::<Asked>()
            .init_resource::<reference::Reference>()
            .init_resource::<kiln::Firing>()
            .add_systems(
                OnEnter(AppState::Bench),
                (open, reference::open, panel::open),
            )
            .add_systems(OnExit(AppState::Bench), close)
            .add_systems(
                Update,
                (
                    choose,
                    generate,
                    aim,
                    move_hand,
                    place,
                    turn_view,
                    rebuild,
                    draw_ghost,
                    reference::show,
                    kiln::ask,
                    kiln::collect,
                    // The panel: what was pressed, then what everything says.
                    panel::pressed,
                    panel::pressed_swatch,
                    crate::tools::widget::fold_branches,
                    crate::tools::widget::light_rows::<panel::Press>,
                    panel::refresh,
                    panel::colour_unsaved,
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
        Transform::from_translation(EYE).looking_at(Vec3::Y * PIVOT, Vec3::Y),
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
    const KEYS: [KeyCode; 7] = [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
        KeyCode::Digit7,
    ];
    for (key, part) in KEYS.iter().zip(Part::ALL) {
        if keys.just_pressed(*key) {
            hand.part = part;
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
    mut was: Local<Option<Vec2>>,
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
    // Only when the pointer has actually moved. Otherwise every key nudge would be
    // undone on the very next frame by a mouse sitting still.
    if *was == Some(cursor) {
        return;
    }
    *was = Some(cursor);

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
    // another piece.
    let snapped = if fine {
        Bench::snapped(held)
    } else {
        Bench::snapped_to(held, kit::MODULE)
    };
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
    if keys.just_pressed(KeyCode::KeyW) || keys.just_pressed(KeyCode::ArrowUp) {
        step += ahead;
    }
    if keys.just_pressed(KeyCode::KeyS) || keys.just_pressed(KeyCode::ArrowDown) {
        step -= ahead;
    }
    if keys.just_pressed(KeyCode::KeyD) || keys.just_pressed(KeyCode::ArrowRight) {
        step += aside;
    }
    if keys.just_pressed(KeyCode::KeyA) || keys.just_pressed(KeyCode::ArrowLeft) {
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
fn place(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    over_panel: Query<&Interaction>,
    hand: Res<Hand>,
    mut bench: ResMut<Bench>,
    mut next: ResMut<NextState<AppState>>,
) {
    // The pointer is on the panel, not on the work.
    //
    // Without this, pressing WALL in the panel also puts a wall down wherever the
    // cursor happened to be — the same click reaching two places, which is the
    // oldest fault in any tool that has both a world and an interface over it.
    let reaching = over_panel
        .iter()
        .any(|touch| matches!(touch, Interaction::Hovered | Interaction::Pressed));
    // The left button and ENTER do the same thing, and what that is depends on the
    // mode. The mouse is the one anybody will use; the key is there because a
    // maker nudging the cursor with W and A should not have to reach for the mouse
    // to put the piece down.
    let go = keys.just_pressed(KeyCode::Enter)
        || (!reaching && buttons.just_pressed(MouseButton::Left));
    if go {
        match hand.doing {
            Doing::Building => {
                bench.add(hand.part, hand.at, hand.quarters, hand.tint);
            }
            Doing::Painting => {
                bench.paint_nearest(hand.at, kit::MODULE, hand.tint);
            }
        }
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
        || (!reaching && buttons.just_pressed(MouseButton::Right))
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
fn turn_view(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut moved: EventReader<bevy::input::mouse::MouseMotion>,
    scroll: Res<bevy::input::mouse::AccumulatedMouseScroll>,
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
    let mut swung = Vec2::ZERO;
    for motion in moved.read() {
        if dragging {
            swung += motion.delta;
        }
    }
    let mut shifted = false;
    if swung != Vec2::ZERO {
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
    if scroll.delta.y != 0.0 {
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

    let pivot = Vec3::Y * PIVOT;
    let out = Vec3::new(
        view.around.sin() * view.pitch.cos(),
        view.pitch.sin(),
        view.around.cos() * view.pitch.cos(),
    );
    for mut camera in &mut cameras {
        camera.translation = pivot + out * view.away;
        camera.look_at(pivot, Vec3::Y);
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

/// How far a pixel of drag turns the view, and how much one wheel notch zooms.
const ORBIT_RATE: f32 = 0.006;
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
    ghosts: Query<Entity, With<Ghost>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if !hand.is_changed() && !ghosts.is_empty() {
        return;
    }
    for entity in &ghosts {
        commands.entity(entity).despawn();
    }

    // The part itself, shown as it will be rather than as a box around it — a
    // wedge that previews as a cuboid puts the roof on the wrong way round about
    // half the time.
    let one = plan::Plan {
        name: String::new(),
        kind: String::new(),
        half_w: 0.0,
        half_d: 0.0,
        high: 0.0,
        boxes: vec![plan::Block {
            at: hand.at + Vec3::Y * hand.part.size().y * 0.5,
            size: hand.part.size(),
            turn: Quat::from_rotation_y(hand.quarters as f32 * std::f32::consts::FRAC_PI_2),
            form: hand.part.form(),
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
