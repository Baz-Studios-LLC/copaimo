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
//! It also means the bench has no terrain, no streaming, no weather and no
//! seventeen-thousand-tuft meadow. A building is a few dozen boxes, so the whole
//! room redraws from scratch every time anything changes — see `rebuild`, which is
//! the simplest thing that can possibly work and stays affordable at this size.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

pub mod kiln;
pub mod panel;
pub mod reference;

use crate::build::kit::{self, Bench, Part, TINTS};
use crate::build::pattern::{self, Pattern};
use crate::build::plan;
use crate::shade::{shaded, Shaded};
use crate::states::AppState;

/// How far the camera looks from, and at what height it pivots.
const EYE: Vec3 = Vec3::new(7.0, 5.0, 9.0);
const PIVOT: f32 = 1.2;

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
    /// How far off.
    away: f32,
}

impl Default for View {
    fn default() -> Self {
        Self {
            around: 0.0,
            away: 1.0,
        }
    }
}

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
        Transform::from_translation(EYE).looking_at(Vec3::Y * PIVOT, Vec3::Y),
    ));

    // Two lights and no sun. The bench is indoors as far as anything here is
    // concerned, and a day/night cycle over a workbench would mean building a
    // fence by moonlight because of what time it happens to be.
    commands.spawn((
        OfBench,
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
    commands.insert_resource(ClearColor(Color::srgb(0.10, 0.11, 0.14)));

    // The floor: a grid of the kit's own module, so a maker can count squares
    // instead of measuring. Drawn as thin slabs rather than lines because this
    // world has no line renderer and a checker reads the depth better anyway.
    let tile = meshes.add(Cuboid::new(kit::MODULE * 0.98, 0.02, kit::MODULE * 0.98));
    // PLAIN materials, not the world's.
    //
    // The floor was made of the material every solid thing outdoors wears, which
    // carries cloud shadows — so clouds nobody could see drifted across the
    // workbench and the grid changed colour for no reason a maker could name.
    // Nothing in this room is outdoors.
    let pale = materials.add(StandardMaterial {
        base_color: Color::srgb(0.20, 0.22, 0.26),
        perceptual_roughness: 0.95,
        ..default()
    });
    let dark = materials.add(StandardMaterial {
        base_color: Color::srgb(0.15, 0.16, 0.20),
        perceptual_roughness: 0.95,
        ..default()
    });
    for x in -FLOOR_REACH..FLOOR_REACH {
        for z in -FLOOR_REACH..FLOOR_REACH {
            let checker = (x + z).rem_euclid(2) == 0;
            commands.spawn((
                OfBench,
                Mesh3d(tile.clone()),
                MeshMaterial3d(if checker { pale.clone() } else { dark.clone() }),
                Transform::from_xyz(
                    (x as f32 + 0.5) * kit::MODULE,
                    -0.01,
                    (z as f32 + 0.5) * kit::MODULE,
                ),
            ));
        }
    }
}

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
    // R turns it, quarter by quarter. There is no free rotation on purpose — see
    // the kit's own note.
    if keys.just_pressed(KeyCode::KeyR) {
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
    let snapped = Bench::snapped(held);
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

    // A whole module with SHIFT, one snap without. Most placing is module to
    // module — a wall beside a wall — and stepping there in six presses would be
    // six times the work for the common case.
    let reach = if keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]) {
        kit::MODULE
    } else {
        kit::SNAP
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
    mut view: ResMut<View>,
    mut cameras: Query<&mut Transform, With<BenchEye>>,
) {
    let mut moved = false;
    if keys.just_pressed(KeyCode::BracketLeft) {
        view.around -= std::f32::consts::FRAC_PI_2;
        moved = true;
    }
    if keys.just_pressed(KeyCode::BracketRight) {
        view.around += std::f32::consts::FRAC_PI_2;
        moved = true;
    }
    if keys.just_pressed(KeyCode::Minus) {
        view.away = (view.away * 1.25).min(4.0);
        moved = true;
    }
    if keys.just_pressed(KeyCode::Equal) {
        view.away = (view.away / 1.25).max(0.35);
        moved = true;
    }
    if !moved {
        return;
    }

    // Quarter turns, like everything else here. A building made of axis-aligned
    // parts is looked at from its corners, and a free orbit would mostly be used
    // to get back to one of them.
    let turn = Quat::from_rotation_y(view.around);
    for mut camera in &mut cameras {
        camera.translation = turn * (EYE * view.away);
        camera.look_at(Vec3::Y * PIVOT, Vec3::Y);
    }
}

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
    mut materials: ResMut<Assets<Shaded>>,
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
    let cloth = materials.add(shaded(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.88,
        reflectance: 0.05,
        ..default()
    }));

    if !solid.is_empty() {
        commands.spawn((
            OfBench,
            Work,
            Mesh3d(meshes.add(solid.into_mesh())),
            MeshMaterial3d(cloth.clone()),
            Transform::IDENTITY,
        ));
    }
    if !glass.is_empty() {
        commands.spawn((
            OfBench,
            Work,
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
    mut materials: ResMut<Assets<Shaded>>,
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
        Mesh3d(meshes.add(solid.into_mesh())),
        MeshMaterial3d(materials.add(shaded(StandardMaterial {
            // Lit and see-through, so it reads as "about to be" rather than as
            // part of the work.
            base_color: Color::srgba_u8(r, g, b, 130),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        }))),
        Transform::IDENTITY,
    ));
}
