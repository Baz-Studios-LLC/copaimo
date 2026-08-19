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
    /// Where the axis line WAS when the drag started.
    ///
    /// # The jitter
    ///
    /// A drag used to measure itself against the arrows where they are NOW. But
    /// the arrows sit on the piece, and the piece is what the drag is moving — so
    /// every step moved the very line the next step was measured from. Push the
    /// piece a module along and the ruler goes with it, the pointer is suddenly
    /// somewhere else along that ruler, and the piece jumps back. It oscillated
    /// rather than slid.
    ///
    /// So the line is remembered from where it started, and the whole drag is
    /// measured against that. The ruler holds still while the thing being measured
    /// moves, which is the only arrangement that ever works.
    line: Vec3,
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

/// Marks an arrow, and which axis it is, so it can be lit when the pointer is on
/// it.
#[derive(Component)]
pub struct Arrow(pub usize);

/// Lights the arrow under the pointer, and the one being dragged.
///
/// # Working and dead looked exactly the same
///
/// This is why the arrows read as broken long after they worked. Nothing changed
/// when the pointer was on one, nothing changed when it was taken hold of, and a
/// drag shorter than a module moves the piece nowhere — because it snaps. So the
/// whole gesture could be performed correctly and produce no visible answer at
/// all, which is indistinguishable from a dead control.
///
/// A handle has to say three things: I can be grabbed, I am grabbed, and here is
/// what I did. The third was already there. These are the other two.
pub fn light_arrows(
    holding: Res<Holding>,
    arrows: Query<(&Arrow, &MeshMaterial3d<StandardMaterial>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if !holding.is_changed() {
        return;
    }
    for (arrow, skin) in &arrows {
        let Some(material) = materials.get_mut(&skin.0) else {
            continue;
        };
        let plain = axes(Quat::IDENTITY)[arrow.0.min(2)].1.to_linear();
        // Held is brightest, hovered is brighter, the rest are themselves.
        let lift = if holding.dragging == Some(arrow.0) {
            HELD
        } else if holding.hovering == Some(arrow.0) {
            HOVERED
        } else {
            1.0
        };
        material.base_color = Color::linear_rgb(
            (plain.red * lift).min(1.0),
            (plain.green * lift).min(1.0),
            (plain.blue * lift).min(1.0),
        );
    }
}

/// How much brighter an arrow goes when the pointer is on it, and when it is held.
const HOVERED: f32 = 1.9;
const HELD: f32 = 3.2;

/// How long an arrow is, how thick its shaft, and how big its head.
const REACH: f32 = 1.35;
const SHAFT: f32 = 0.045;
const HEAD: f32 = 0.17;

/// How far along the length arrow counts as its far END, where taking hold
/// stretches rather than moves.
const FAR_END: f32 = 0.6;

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
    // A piece stays chosen until another one is.
    //
    // # The deadlock this replaces
    //
    // The selection used to be let go of whenever the ground cursor wandered out
    // of range, and that made the arrows impossible to click. Two frames:
    //
    // 1. The pointer is on the piece. It gets picked. Nothing is hovered.
    // 2. The pointer moves ONTO an arrow — which throws the ground cursor far
    //    away, because an arrow stands above the floor and the cursor is where the
    //    view ray meets it. The selection is dropped for being out of range, and
    //    the arrow-hovering test never runs, because there is no longer a piece to
    //    test the arrows of.
    //
    // So the arrows vanished the instant they were pointed at, and no amount of
    // fixing the hit test could have helped: nothing was ever hit-tested. Adding a
    // "do not re-pick while hovering" rule did not help either, because hovering
    // could never be discovered in the first place — the two systems each waited
    // on the other.
    //
    // Holding on until something else is chosen breaks it, and it is the better
    // behaviour anyway: a selection that evaporates when the pointer drifts is one
    // nobody can act on.
    // Not while the pointer is on a handle. Holding on to the selection is what
    // stops it being dropped when the cursor wanders; this is what stops it being
    // handed to a DIFFERENT piece that the wandering cursor happened to land on,
    // which would take the arrows out from under the pointer just as surely.
    if holding.on_a_handle() {
        return;
    }

    // Whatever is under the cursor now, if anything is.
    let near = bench
        .pieces()
        .iter()
        .map(|piece| (piece.middle().distance(hand.at), piece.id))
        .filter(|(away, _)| *away <= kit::MODULE * 1.5)
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, id)| id);

    if let Some(near) = near {
        holding.piece = Some(near);
        return;
    }

    // Nothing under the cursor: keep what was chosen, unless it has since been
    // taken off the bench.
    if let Some(held) = holding.piece {
        if !bench.pieces().iter().any(|piece| piece.id == held) {
            holding.piece = None;
            holding.hovering = None;
        }
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
            holding.line = base;
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
    // Against the line as it was when the drag began — see `Holding::line`.
    let (_, along) = ray_against_axis(ray.origin, *ray.direction, holding.line, axis);
    if along == 0.0 {
        return;
    }

    // The far END of the length arrow stretches; the rest of it moves.
    //
    // Where you take hold of a thing is what you meant to do with it — grab a
    // plank in the middle and you are carrying it, grab the end and you are
    // pulling it longer. That is a rule a maker can find by trying, which CTRL
    // was not: a modifier is invisible, and the one person who knew about it was
    // the one who wrote it down.
    //
    // Only the length arrow. Height is a storey and thickness is what a wall is;
    // neither is a thing to drag out.
    let stretching = axis_at == 0
        && piece.part.stretches()
        && (holding.grabbed > REACH * FAR_END
            || keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]));
    if stretching {
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

    for (at, (axis, colour)) in axes(turn_of).into_iter().enumerate() {
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
            Arrow(at),
            RenderLayers::layer(BENCH_LAYER),
            Mesh3d(shaft.clone()),
            MeshMaterial3d(skin.clone()),
            Transform::from_translation(base + axis * (REACH - HEAD) * 0.5)
                .with_rotation(turn),
        ));
        commands.spawn((
            OfBench,
            Arrow(at),
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
    fn scratch_real_camera() {
        // The bench's own opening view, and a piece at the origin.
        let (around, pitch, away) = (0.6_f32, 0.55_f32, 13.0_f32);
        let pivot = Vec3::Y * 1.2;
        let out = Vec3::new(
            around.sin() * pitch.cos(),
            pitch.sin(),
            around.cos() * pitch.cos(),
        );
        let eye = pivot + out * away;

        // A post at the origin: its middle is where the arrows sit.
        let base = Vec3::new(0.0, kit::Part::Post.size().y * 0.5, 0.0);

        for (name, axis) in [("X", Vec3::X), ("Y", Vec3::Y), ("Z", Vec3::Z)] {
            // A ray from the eye straight at the middle of that arrow — which is
            // exactly what a ray through the pixel the arrow is drawn on IS.
            let target = base + axis * (REACH * 0.5);
            let toward = (target - eye).normalize();
            let (away_from, along) = ray_against_axis(eye, toward, base, axis);
            println!(
                "SCRATCH {name}: away={away_from:.4} along={along:.4} (GRAB={GRAB})"
            );
        }
    }

    #[test]
    fn the_selection_survives_the_cursor_leaving_the_piece() {
        // The exact sequence that made the arrows unclickable, run frame by frame.
        //
        // Pointing at an arrow throws the ground cursor away from the piece — an
        // arrow stands above the floor, and the cursor is where the view ray meets
        // it. The selection was dropped for being out of range, and the
        // arrow-hovering test never ran, because there was no longer a piece to
        // test the arrows of. Each system waited on the other.
        let mut bench = Bench::default();
        let post = bench.add(kit::Part::Post, Vec3::ZERO, 0, 0).expect("a post");
        let far = bench
            .add(kit::Part::Post, Vec3::new(30.0, 0.0, 30.0), 0, 0)
            .expect("another post, well away");

        let mut app = App::new();
        app.insert_resource(bench)
            .insert_resource(Hand::default())
            .init_resource::<Holding>()
            .add_systems(Update, choose);

        // Frame one: the cursor is on the post, and it is chosen.
        app.update();
        assert_eq!(app.world().resource::<Holding>().piece, Some(post));

        // Frame two: the pointer reaches for an arrow, which throws the ground
        // cursor metres away. NOTHING is hovered yet — that is the whole point,
        // since hovering cannot be discovered until the arrows survive this frame.
        app.world_mut().resource_mut::<Hand>().at = Vec3::new(9.0, 0.0, 9.0);
        app.update();
        assert_eq!(
            app.world().resource::<Holding>().piece,
            Some(post),
            "the piece was let go of the moment its own arrow was reached for"
        );

        // Pointing at a different piece does change the selection — holding on
        // must not mean getting stuck.
        app.world_mut().resource_mut::<Hand>().at = Vec3::new(30.0, 0.0, 30.0);
        app.update();
        assert_eq!(app.world().resource::<Holding>().piece, Some(far));

        // And a piece taken off the bench is let go of rather than held for ever.
        app.world_mut().resource_mut::<Bench>().remove_nearest(
            Vec3::new(30.0, 0.0, 30.0),
            kit::MODULE,
        );
        app.world_mut().resource_mut::<Hand>().at = Vec3::new(60.0, 0.0, 60.0);
        app.update();
        assert_eq!(
            app.world().resource::<Holding>().piece,
            None,
            "the arrows stayed on a piece that is no longer there"
        );
    }

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

        // Another piece, which the ground cursor is about to wander over.
        let other = app
            .world_mut()
            .resource_mut::<Bench>()
            .add(kit::Part::Post, Vec3::new(40.0, 0.0, 40.0), 0, 0)
            .expect("another post");

        // Now the pointer is on an ARROW of the first piece, which throws the
        // ground cursor across the room and onto the second. The arrows must not
        // be handed over: they are being reached for.
        app.world_mut().resource_mut::<Holding>().hover_for_test(1);
        app.world_mut().resource_mut::<Hand>().at = Vec3::new(40.0, 0.0, 40.0);
        app.update();
        assert_eq!(
            app.world().resource::<Holding>().piece,
            Some(id),
            "reaching for an arrow handed it to whatever the cursor landed on"
        );

        // With nothing hovered, that same cursor does choose the second piece —
        // holding on must not mean getting stuck.
        app.world_mut().resource_mut::<Holding>().hovering = None;
        app.update();
        assert_eq!(app.world().resource::<Holding>().piece, Some(other));
    }

    #[test]
    fn a_drag_is_measured_from_where_it_started_not_from_where_it_has_got_to() {
        // The jitter. A drag used to measure itself against the arrows where they
        // are NOW — but the arrows sit on the piece, and the piece is what the
        // drag is moving. Every step moved the ruler the next step was measured
        // from, so the piece oscillated instead of sliding.
        //
        // Here is the same ray read against a line that has moved with the piece,
        // and against one that stayed where the drag began.
        let eye = Vec3::new(7.0, 5.0, 9.0);
        let start = Vec3::new(0.0, 0.625, 0.0);
        let toward = (Vec3::new(0.7, 0.625, 0.0) - eye).normalize();

        // What the pointer says, against the line where the drag began.
        let (_, from_start) = ray_against_axis(eye, toward, start, Vec3::X);

        // The piece has since been dragged a module along, taking its arrows with
        // it. Read against THAT line, the same pointer says something different —
        // and the difference is the jump.
        let moved_line = start + Vec3::X * kit::MODULE;
        let (_, from_moved) = ray_against_axis(eye, toward, moved_line, Vec3::X);

        assert!(
            (from_start - from_moved).abs() > 0.5,
            "a moved ruler read the same, so this test proves nothing: {from_start:.3} against {from_moved:.3}"
        );

        // Which is why the drag keeps the line it started with. Reading twice from
        // the same start gives the same answer, whatever the piece has done.
        let (_, again) = ray_against_axis(eye, toward, start, Vec3::X);
        assert!(
            (from_start - again).abs() < 1.0e-6,
            "the same pointer against the same line gave two answers"
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
