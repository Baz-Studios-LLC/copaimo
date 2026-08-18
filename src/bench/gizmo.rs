//! The arrows: taking hold of a piece and moving it.
//!
//! # Why arrows rather than more keys
//!
//! Everything else on this bench is placed by aiming and clicking, which is right
//! for putting a piece DOWN — you are choosing a cell. Moving one that is already
//! down is a different act: you have a thing, and you want it over there, or a
//! storey up. Aiming at a cell cannot express "up" at all, which is why height was
//! the one thing this tool could not do.
//!
//! Three arrows is the answer every program that has this problem arrived at, and
//! for a reason worth stating: an arrow says which way it will go BEFORE you drag
//! it. A key does not, and a free drag in three dimensions from a two-dimensional
//! mouse has to guess which of them you meant.
//!
//! # Red, green, blue
//!
//! X, Y, Z, in that order, and not because it is pretty. Every tool a maker has
//! ever used colours them this way, so the one thing they should not have to learn
//! here is which arrow is which.
//!
//! # It still snaps
//!
//! The same rule the rest of the bench keeps: an arrow proposes and the lattice
//! disposes. Dragging along an axis moves in module steps, or quarter-metres with
//! SHIFT. A gizmo that moved freely would let a maker take a wall off the lattice
//! by hand, which is the one thing the lattice exists to prevent.

use bevy::prelude::*;
use bevy::render::view::RenderLayers;

use crate::build::kit::{self, Bench, Piece};

use super::{BenchEye, Hand, OfBench, BENCH_LAYER};

/// Which piece the arrows are on, and which arrow is being dragged.
#[derive(Resource, Default)]
pub struct Holding {
    /// The piece under the arrows, by id.
    pub piece: Option<u32>,
    /// The axis being dragged, if one is.
    dragging: Option<usize>,
    /// Where along that axis the drag started, so the piece moves BY the drag
    /// rather than jumping to wherever the pointer first landed.
    grabbed: f32,
    /// Where the piece was when the drag started.
    from: Vec3,
    /// How many modules this drag has already stretched by, so the piece grows
    /// with the pointer rather than once per frame.
    stretched: i32,
    /// Which arrow the pointer is over, if any.
    ///
    /// # Why hovering has to be its own thing
    ///
    /// The arrows could not be clicked at all, and this is why. Which piece the
    /// arrows sit on was decided by what the GROUND cursor was nearest — and
    /// moving the pointer onto an arrow moves that cursor, because the cursor is
    /// where the view ray meets the floor and an arrow stands above it. So
    /// reaching for a handle slid the ground cursor away from the piece, the piece
    /// was let go of, and the arrows vanished from under the pointer on the way to
    /// them.
    ///
    /// Knowing the pointer is over an arrow fixes both halves: the selection stops
    /// being re-picked while it is, and a click there is a click on the handle
    /// rather than on the ground behind it.
    hovering: Option<usize>,
}

impl Holding {
    /// Whether the pointer is on a handle — hovering one or dragging it.
    ///
    /// What anything else acting on a click should ask. Dragging alone was not
    /// enough: on the frame a handle is first pressed nothing is being dragged
    /// yet, which is exactly the frame the click has to be kept away from the
    /// ground.
    pub fn on_a_handle(&self) -> bool {
        self.hovering.is_some() || self.dragging.is_some()
    }

    /// Puts the arrows in hand, for a test that cannot aim a mouse.
    ///
    /// The alternative is making the field public, which would let anything set
    /// it — and what it means is "a drag is in progress", which only the drag
    /// itself can honestly say.
    #[cfg(test)]
    pub fn hold_for_test(&mut self, axis: usize) {
        self.dragging = Some(axis);
    }

    /// Puts the pointer over an arrow, for a test that cannot aim a mouse.
    #[cfg(test)]
    pub fn hover_for_test(&mut self, axis: usize) {
        self.hovering = Some(axis);
    }
}

/// Marks the arrows, so they can be cleared and redrawn together.
#[derive(Component)]
pub struct Arrow;

/// How long an arrow is, how thick its shaft, and how big its head.
const REACH: f32 = 1.35;
const SHAFT: f32 = 0.045;
const HEAD: f32 = 0.17;

/// How near the pointer has to come to an arrow to take hold of it, in metres at
/// the arrow's own distance.
///
/// Generous. A thin shaft is hard to hit and the cost of missing is that the
/// click falls through to whatever is behind, which on this bench means placing a
/// piece you did not want.
const GRAB: f32 = 0.34;

/// The three axes a piece is moved along, and the colour each one wears.
///
/// Turned WITH the piece, not with the world.
///
/// A wall placed across the room has its length running along world Z, and a red
/// arrow pointing along world X would then stretch it through its own thickness.
/// The arrows are the piece's own axes, so the red one always runs along the thing
/// a maker would call its length however it has been turned.
fn axes(turn: Quat) -> [(Vec3, Color); 3] {
    [
        (turn * Vec3::X, Color::srgb(0.92, 0.30, 0.32)),
        (Vec3::Y, Color::srgb(0.42, 0.86, 0.36)),
        (turn * Vec3::Z, Color::srgb(0.32, 0.52, 0.95)),
    ]
}

/// Picks the piece the arrows sit on.
///
/// Whatever is nearest the cursor, which is the same rule everything else on this
/// bench follows for reaching an existing piece.
pub fn choose(hand: Res<Hand>, bench: Res<Bench>, mut holding: ResMut<Holding>) {
    if holding.on_a_handle() {
        // Not while the pointer is on a handle. Reaching for an arrow moves the
        // ground cursor away from the piece — the arrow stands above the floor and
        // the cursor is where the ray meets it — so re-picking here let go of the
        // very piece being reached for. And during a drag the piece moves under
        // the pointer, which would hand the arrows to whatever it passed over.
        return;
    }
    let near = bench
        .pieces()
        .iter()
        .map(|piece| (piece.middle().distance(hand.at), piece.id))
        .filter(|(away, _)| *away <= kit::MODULE * 1.5)
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, id)| id);
    if holding.piece != near {
        holding.piece = near;
    }
}

/// Where a piece is, by id.
fn piece_at(bench: &Bench, id: u32) -> Option<Piece> {
    bench.pieces().iter().find(|p| p.id == id).copied()
}

/// How near a ray passes to a line segment, and how far along the segment.
///
/// The whole of hit-testing an arrow. Both numbers are wanted: the first says
/// whether it was grabbed, the second says where along it, which is what the drag
/// is measured against.
fn ray_against_axis(from: Vec3, along: Vec3, base: Vec3, axis: Vec3) -> (f32, f32) {
    // Closest approach of two lines. If they are near enough to parallel the
    // cross product vanishes, and there is no sensible answer — so say "miles
    // away" rather than divide by nothing.
    let across = along.cross(axis);
    let denominator = across.length_squared();
    if denominator < 1.0e-6 {
        return (f32::MAX, 0.0);
    }
    let between = base - from;
    let up_the_axis = between.cross(along).dot(across) / denominator;
    let up_the_ray = between.cross(axis).dot(across) / denominator;
    // Behind the camera is not a hit.
    if up_the_ray < 0.0 {
        return (f32::MAX, 0.0);
    }
    let on_axis = base + axis * up_the_axis.clamp(0.0, REACH);
    let on_ray = from + along * up_the_ray;
    (on_axis.distance(on_ray), up_the_axis)
}

/// Taking hold of an arrow, dragging it, and letting go.
pub fn drag(
    buttons: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<BenchEye>>,
    mut holding: ResMut<Holding>,
    mut bench: ResMut<Bench>,
) {
    let (Some(window), Some((camera, eye))) = (windows.iter().next(), cameras.iter().next()) else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Ok(ray) = camera.viewport_to_world(eye, cursor) else {
        return;
    };

    if buttons.just_released(MouseButton::Left) {
        holding.dragging = None;
        return;
    }

    let Some(id) = holding.piece else {
        return;
    };
    let Some(piece) = piece_at(&bench, id) else {
        holding.piece = None;
        return;
    };
    let base = piece.middle();

    // Which arrow the pointer is on, worked out EVERY frame rather than only when
    // the button goes down. What is hovered decides whether the selection holds
    // still and whether a click belongs to the ground, and both of those have to
    // be true before the click arrives.
    let mut best: Option<(f32, usize, f32)> = None;
    for (at, (axis, _)) in axes(piece.turn()).iter().enumerate() {
        let (away, along) = ray_against_axis(ray.origin, *ray.direction, base, *axis);
        if away > GRAB {
            continue;
        }
        if best.is_none_or(|(nearest, ..)| away < nearest) {
            best = Some((away, at, along));
        }
    }
    if holding.dragging.is_none() {
        holding.hovering = best.map(|(_, axis, _)| axis);
    }

    // Taking hold.
    if buttons.just_pressed(MouseButton::Left) && holding.dragging.is_none() {
        if let Some((_, axis, along)) = best {
            holding.dragging = Some(axis);
            holding.grabbed = along;
            holding.from = piece.foot;
            holding.stretched = 0;
        }
        return;
    }

    // Dragging.
    let Some(axis_at) = holding.dragging else {
        return;
    };
    if !buttons.pressed(MouseButton::Left) {
        holding.dragging = None;
        return;
    }
    let axis = axes(piece.turn())[axis_at].0;
    let (_, along) = ray_against_axis(ray.origin, *ray.direction, base, axis);
    if along == 0.0 {
        return;
    }

    // CTRL turns the length arrow into a STRETCH handle.
    //
    // The same arrow, because it is the same direction: the red one runs along the
    // piece's own length, and what a maker wants from it is either "further along"
    // or "longer". A separate handle would mean another thing to hit and another
    // thing to explain, and the two are never wanted at once.
    if keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]) && axis_at == 0 {
        // In whole modules, measured from where the arrow was taken hold of, so a
        // slow drag does not run a wall out to its limit.
        let moved = along - holding.grabbed;
        let want = (moved / kit::MODULE).round() as i32;
        if want != holding.stretched {
            bench.stretch(id, want - holding.stretched);
            holding.stretched = want;
        }
        return;
    }

    // BY the drag, not TO the pointer. Measured from where the arrow was taken
    // hold of, so a piece does not jump the moment it is grabbed off-centre.
    let moved = along - holding.grabbed;
    let step = if keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]) {
        kit::SNAP
    } else {
        kit::MODULE
    };
    let wanted = holding.from + axis * moved;
    let put = Bench::snapped_to(wanted, step);
    // Never below the floor. A piece under the ground is a piece nobody can see
    // and nobody can select to bring back.
    let put = Vec3::new(put.x, put.y.max(0.0), put.z);
    if put != piece.foot {
        bench.move_to(id, put);
    }
}

/// Draws the arrows on whatever is held.
///
/// Rebuilt whenever the selection or the piece moves, like everything else in
/// this room: three arrows is nothing to respawn, and one code path answers "where
/// are the arrows" rather than two that can disagree.
pub fn show(
    mut commands: Commands,
    holding: Res<Holding>,
    bench: Res<Bench>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    drawn: Query<Entity, With<Arrow>>,
    mut was: Local<Option<(u32, Vec3, u8, u32)>>,
) {
    let now = holding
        .piece
        .and_then(|id| piece_at(&bench, id))
        .map(|piece| (piece.id, piece.middle(), piece.quarters, piece.spans));
    if now == *was && !drawn.is_empty() {
        return;
    }
    if now.is_none() && drawn.is_empty() {
        return;
    }
    *was = now;

    for entity in &drawn {
        commands.entity(entity).despawn();
    }
    let Some((_, base, quarters, _)) = now else {
        return;
    };
    let turn_of = Quat::from_rotation_y(quarters as f32 * std::f32::consts::FRAC_PI_2);

    let shaft = meshes.add(Cuboid::new(SHAFT, REACH - HEAD, SHAFT));
    let head = meshes.add(Cone {
        radius: HEAD * 0.5,
        height: HEAD,
    });

    for (axis, colour) in axes(turn_of) {
        // Unlit, and deliberately: a handle is a control, not a thing in the
        // room. Lit, it would go dark on the shaded side and read as part of the
        // building.
        let skin = materials.add(StandardMaterial {
            base_color: colour,
            unlit: true,
            ..default()
        });
        // The shaft is built along Y, so every other axis is that turned.
        let turn = Quat::from_rotation_arc(Vec3::Y, axis);

        commands.spawn((
            OfBench,
            Arrow,
            RenderLayers::layer(BENCH_LAYER),
            Mesh3d(shaft.clone()),
            MeshMaterial3d(skin.clone()),
            Transform::from_translation(base + axis * (REACH - HEAD) * 0.5)
                .with_rotation(turn),
        ));
        commands.spawn((
            OfBench,
            Arrow,
            RenderLayers::layer(BENCH_LAYER),
            Mesh3d(head.clone()),
            MeshMaterial3d(skin),
            Transform::from_translation(base + axis * (REACH - HEAD * 0.5)).with_rotation(turn),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reaching_for_an_arrow_does_not_let_go_of_the_piece() {
        // The reported bug, and it made the arrows unclickable entirely.
        //
        // Which piece the arrows sit on was decided by what the GROUND cursor was
        // nearest — and moving the pointer onto an arrow moves that cursor,
        // because an arrow stands above the floor and the cursor is where the view
        // ray meets it. So reaching for a handle slid the cursor off the piece,
        // the piece was let go of, and the arrows vanished on the way to them.
        let mut bench = Bench::default();
        let id = bench.add(kit::Part::Post, Vec3::ZERO, 0, 0).expect("a post");

        let mut app = App::new();
        app.insert_resource(bench)
            .insert_resource(Hand::default())
            .init_resource::<Holding>()
            .add_systems(Update, choose);

        // The cursor on the piece: it gets picked up.
        app.update();
        assert_eq!(
            app.world().resource::<Holding>().piece,
            Some(id),
            "a piece under the cursor was not picked up"
        );

        // Now the pointer reaches for an arrow, which drags the ground cursor well
        // away from the piece. The selection has to hold.
        app.world_mut().resource_mut::<Holding>().hover_for_test(1);
        app.world_mut().resource_mut::<Hand>().at = Vec3::new(40.0, 0.0, 40.0);
        app.update();
        assert_eq!(
            app.world().resource::<Holding>().piece,
            Some(id),
            "reaching for an arrow let go of the piece it belongs to"
        );

        // And with nothing hovered, a cursor that far off does let go.
        app.world_mut().resource_mut::<Holding>().hovering = None;
        app.update();
        assert_eq!(
            app.world().resource::<Holding>().piece,
            None,
            "the selection stuck to a piece the cursor had left"
        );
    }

    #[test]
    fn an_arrow_is_grabbed_by_pointing_at_it_and_not_past_it() {
        // The whole of hit-testing a handle. Too tight and a thin shaft cannot be
        // hit; too loose and every click anywhere grabs one, which on this bench
        // would mean dragging a wall when you meant to place one.
        let base = Vec3::new(0.0, 1.0, 0.0);

        // The Y arrow, seen from the SIDE and pointed at halfway up.
        //
        // Not from directly above: that puts the arrow edge-on, a dot on screen,
        // and it cannot be grabbed at all — which is correct and is what the
        // parallel guard is for. It is also why every tool that draws these fades
        // an axis out as it turns toward the viewer.
        let (away, along) = ray_against_axis(
            base + Vec3::new(0.0, REACH * 0.5, 6.0),
            Vec3::NEG_Z,
            base,
            Vec3::Y,
        );
        assert!(away < GRAB, "pointing at the Y arrow missed it by {away:.3}");
        assert!(
            (along - REACH * 0.5).abs() < 0.05,
            "grabbed the Y arrow at {along:.2}, not halfway up"
        );

        // And the X arrow, crossed halfway along.
        let (away, along) = ray_against_axis(
            Vec3::new(REACH * 0.5, 4.0, 0.0) + base,
            Vec3::NEG_Y,
            base,
            Vec3::X,
        );
        assert!(away < GRAB, "crossing the X arrow missed by {away:.3}");
        assert!(
            (along - REACH * 0.5).abs() < 0.05,
            "grabbed at {along:.2} along an arrow it crossed at {:.2}",
            REACH * 0.5
        );

        // And a ray nowhere near it.
        let (away, _) = ray_against_axis(
            Vec3::new(9.0, 4.0, 9.0),
            Vec3::NEG_Y,
            base,
            Vec3::X,
        );
        assert!(away > GRAB, "a ray nine metres away grabbed an arrow");
    }

    #[test]
    fn a_ray_behind_the_camera_never_grabs_anything() {
        // A line has two ends and a ray has one. Without this the arrows can be
        // taken hold of by pointing away from them, which reads as the tool
        // grabbing at random.
        let (away, _) = ray_against_axis(
            Vec3::new(0.0, 1.0, 5.0),
            Vec3::Z,
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::X,
        );
        assert_eq!(away, f32::MAX, "an arrow behind the camera was grabbed");
    }

    #[test]
    fn an_arrow_along_the_view_is_not_grabbed_by_dividing_by_nothing() {
        // Looking straight down an axis makes the two lines parallel, and the
        // arithmetic for "closest approach" divides by a vanishing number. The
        // answer is that it cannot be grabbed, not that it is infinitely close.
        let (away, _) = ray_against_axis(
            Vec3::new(-5.0, 1.0, 0.0),
            Vec3::X,
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::X,
        );
        assert!(away.is_finite());
        assert_eq!(away, f32::MAX, "an arrow edge-on was grabbed anyway");
    }

    #[test]
    fn the_arrows_turn_with_the_piece() {
        // A wall placed across the room has its length along world Z. A red arrow
        // pointing along world X would then stretch it through its own thickness —
        // which is the whole reason these are the piece's axes and not the world's.
        let quarter = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
        let [(length, _), (up, _), (across, _)] = axes(quarter);

        assert!(
            length.dot(Vec3::NEG_Z).abs() > 0.99,
            "a quarter-turned piece's length arrow points {length:?}"
        );
        // Up is up whichever way a thing is turned. A piece cannot be rotated onto
        // its side here, so there is no case where its own up is not the world's.
        assert_eq!(up, Vec3::Y);
        assert!(across.dot(Vec3::X).abs() > 0.99, "the third arrow is {across:?}");
    }

    #[test]
    fn the_axes_are_the_colours_every_tool_uses() {
        // The one thing a maker should not have to learn here.
        let [(x, red), (y, green), (z, blue)] = axes(Quat::IDENTITY);
        assert_eq!((x, y, z), (Vec3::X, Vec3::Y, Vec3::Z));
        let reddest = red.to_linear();
        assert!(reddest.red > reddest.green && reddest.red > reddest.blue);
        let greenest = green.to_linear();
        assert!(greenest.green > greenest.red && greenest.green > greenest.blue);
        let bluest = blue.to_linear();
        assert!(bluest.blue > bluest.red && bluest.blue > bluest.green);
    }
}
