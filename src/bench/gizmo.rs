//! The handles: taking hold of a piece to move it, or to make it longer.
//!
//! # Why handles rather than more keys
//!
//! Everything else on this bench is placed by aiming and clicking, which is right
//! for putting a piece DOWN — you are choosing a cell. Changing one that is already
//! down is a different act: you have a thing, and you want it over there, or a
//! storey up, or a module longer. Aiming at a cell cannot express "up" at all,
//! which is why height was the one thing this tool could not do, and it cannot
//! express "longer" either.
//!
//! # The drawn handle IS the handle
//!
//! This is the whole of what was wrong before, and it took five attempts to see.
//! The arrows were DRAWN as entities and HIT-TESTED as arithmetic — three imagined
//! lines recomputed from the piece every frame. Two sources of truth for one
//! question, and every fix moved one of them.
//!
//! Now the hit test walks the handle entities themselves and reads each one's
//! `GlobalTransform`. Whatever is on screen is what the pointer is tested against,
//! because it is the same object. There is no arrangement of code that can make
//! them disagree, which is a different thing from making them agree today.
//!
//! # And they are drawn over everything
//!
//! A handle is a CONTROL. It is not in the room, and it must never be behind
//! anything in it. They used to draw on the bench's own layer, so a piece could
//! swallow them — stretch a wall from one module to two and its own body is 3 m
//! long, while the red arrow reaches 0.95 m from the middle, so the arrow ends up
//! entirely inside the wall. That is the "worked once, then disappeared" that had
//! no explanation: it worked, it made the wall longer, and the wall ate it.
//!
//! So they have their own layer and their own camera, which rides the bench's eye
//! and draws after it onto a cleared depth buffer. A handle cannot be hidden by the
//! thing it is attached to, whatever size that thing is dragged to.
//!
//! # Red, green, blue — and amber
//!
//! X, Y, Z, in that order, and not because it is pretty: every tool a maker has
//! ever used colours them this way, so the one thing they should not have to learn
//! here is which arrow is which. The amber blocks at the ends are the other job —
//! pulling a piece LONGER — and they are a different colour and a different shape
//! and stood off to one side, because a handle that does something else should not
//! look like the ones that don't.
//!
//! # It still snaps
//!
//! The same rule the rest of the bench keeps: a handle proposes and the lattice
//! disposes. Sliding moves in module steps, or quarter-metres with SHIFT; pulling
//! grows in whole modules. A handle that moved freely would let a maker take a wall
//! off the lattice by hand, which is the one thing the lattice exists to prevent.

use bevy::prelude::*;
use bevy::render::view::RenderLayers;

use crate::build::kit::{self, Bench, Piece};

use super::{BenchEye, Hand, HandleEye, OfBench, HANDLE_LAYER};

/// What taking hold of a handle does.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Grip {
    /// Carry the whole piece along the handle's direction.
    Slide,
    /// Make it longer, in whole modules.
    ///
    /// `back` is the end that grows BACKWARD. A piece grows forward from its foot,
    /// so pulling its near end has to walk the foot back by as much as it grew —
    /// otherwise the far end, which the maker is not touching, slides away from
    /// whatever it was lined up against.
    Stretch { back: bool },
    /// Make it WIDER, in whole modules — a floor, and only a floor.
    ///
    /// The same gesture as `Stretch` across the other axis, and a separate arm of
    /// the match rather than an axis field on one: what a pull does to the foot is
    /// worked out along the piece's own length in one case and across it in the
    /// other, and the parts that have a length are not the parts that have a
    /// width. See `Part::widens`.
    Widen { back: bool },
}

/// One handle: which slot it is, which way it pulls, and what pulling it does.
///
/// Spawned as an entity and hit-tested as one. `dir` is a WORLD direction — the
/// rig it hangs on is never rotated, so a handle's own axis is the same vector the
/// drag measures along.
#[derive(Component, Clone, Copy, Debug)]
pub struct Handle {
    /// Which of the piece's handles this is. Stable for a given piece, so the
    /// hover and the drag can name one without holding an `Entity` that a rebuild
    /// would invalidate.
    at: usize,
    dir: Vec3,
    grip: Grip,
    /// How far along `dir`, from this handle's own origin, may be taken hold of.
    ///
    /// Its actual extent. An arrow is a shaft running one way; a block is a block.
    span: (f32, f32),
    tint: Color,
}

/// A pull in progress.
///
/// Everything it needs is copied at the moment the handle is taken hold of, and
/// nothing is read from the handle entity afterwards. That is deliberate: pulling a
/// piece longer changes its size, which rebuilds the handles, and a drag holding an
/// `Entity` would be holding a despawned one from the second module onward.
#[derive(Clone, Copy)]
struct Drag {
    at: usize,
    dir: Vec3,
    grip: Grip,
    /// The ruler, fixed where the drag began.
    ///
    /// # The jitter
    ///
    /// A drag used to measure itself against the handles where they are NOW. But
    /// the handles sit on the piece, and the piece is what the drag is moving — so
    /// every step moved the very line the next step was measured from. Push the
    /// piece a module along and the ruler goes with it, the pointer is suddenly
    /// somewhere else along that ruler, and the piece jumps back. It oscillated
    /// rather than slid.
    ///
    /// So the line is remembered from where it started and the whole drag is
    /// measured against that. The ruler holds still while the thing being measured
    /// moves, which is the only arrangement that ever works.
    anchor: Vec3,
    /// How far along that ruler the pull began, so the piece moves BY the drag
    /// rather than jumping to wherever the pointer first landed.
    t0: f32,
    /// Where the piece's foot was when the handle was taken hold of.
    from: Vec3,
    /// How many modules of pull have already been paid out, so a slow drag does not
    /// run a wall to its limit and a clamped one lets go again immediately.
    stepped: i32,
}

/// Which piece wears the handles, and which handle is in hand.
#[derive(Resource, Default)]
pub struct Holding {
    /// The piece under the handles, by id.
    pub piece: Option<u32>,
    dragging: Option<Drag>,
    /// Which handle the pointer is over, if any.
    ///
    /// # Why hovering has to be its own thing
    ///
    /// The handles could not be clicked at all, and this is why. Which piece they
    /// sit on was decided by what the GROUND cursor was nearest — and moving the
    /// pointer onto a handle moves that cursor, because the cursor is where the
    /// view ray meets the floor and a handle stands above it. So reaching for one
    /// slid the ground cursor away from the piece, the piece was let go of, and the
    /// handles vanished from under the pointer on the way to them.
    ///
    /// Knowing the pointer is over a handle fixes both halves: the selection stops
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

    /// Puts a handle in hand, for a test that cannot aim a mouse.
    ///
    /// The alternative is making the field public, which would let anything set
    /// it — and what it means is "a drag is in progress", which only the drag
    /// itself can honestly say.
    #[cfg(test)]
    pub fn hold_for_test(&mut self, at: usize) {
        self.dragging = Some(Drag {
            at,
            dir: Vec3::X,
            grip: Grip::Slide,
            anchor: Vec3::ZERO,
            t0: 0.0,
            from: Vec3::ZERO,
            stepped: 0,
        });
    }

    /// Puts the pointer over a handle, for a test that cannot aim a mouse.
    #[cfg(test)]
    pub fn hover_for_test(&mut self, at: usize) {
        self.hovering = Some(at);
    }
}

/// The rig every handle hangs on, standing at the piece's middle.
///
/// One parent, so a piece that has merely MOVED slides its handles by having the
/// rig moved rather than by building them again. A handle respawned under the
/// pointer every frame is a handle whose world position is always a frame behind
/// the thing it is attached to.
#[derive(Component)]
pub struct Rig;

/// How far a move arrow reaches, how thick its shaft, and how big its head.
const ARM: f32 = 0.95;
const SHAFT: f32 = 0.05;
const HEAD: f32 = 0.18;

/// The stretch handle: its block, the stalk that ties it to the piece's end, how
/// far past that end it stands, and how far to one side.
///
/// ASIDE is the one that matters. A stretch handle on the same line as the red
/// arrow is a handle that takes clicks meant for the arrow and vice versa —
/// Opificium hit exactly this with its roof handles and answered it the same way.
/// Off to one side, the two can never be confused by a pointer or by an eye.
const KNOB: f32 = 0.18;
const STUB: f32 = 0.34;
const STAND: f32 = 0.45;
const ASIDE: f32 = 0.62;

/// How near the pointer must come to a handle, as a share of how far off it is.
///
/// An ANGLE, not a distance. A fixed number of metres is right at one zoom and
/// wrong at every other: a handle sixty metres off is a few pixels wide, and a
/// tolerance that does not shrink with it grabs handles the pointer is nowhere
/// near — while one that suits that range cannot hit anything up close. This is
/// about a degree either way, which is the same forgiving target at every zoom.
const GRAB: f32 = 0.019;

/// And bounded at both ends, so the arithmetic cannot produce a tolerance the size
/// of the room or one no mouse can hit.
const GRAB_LEAST: f32 = 0.10;
const GRAB_MOST: f32 = 0.45;

/// How much brighter a handle goes when the pointer is on it, and when it is held.
///
/// # Working and dead looked exactly the same
///
/// This is why the handles read as broken long after the arithmetic was right.
/// Nothing changed when the pointer was on one, nothing changed when it was taken
/// hold of, and a drag shorter than a module moves the piece nowhere — because it
/// snaps. So the whole gesture could be performed correctly and produce no visible
/// answer at all, which is indistinguishable from a dead control.
///
/// A handle has to say three things: I can be grabbed, I am grabbed, and here is
/// what I did. The third was always there. These are the other two.
const HOVERED: f32 = 1.9;
const HELD: f32 = 3.2;

/// The colours. X, Y, Z as every tool draws them, and a fourth for the other job.
const RED: Color = Color::srgb(0.92, 0.30, 0.32);
const GREEN: Color = Color::srgb(0.42, 0.86, 0.36);
const BLUE: Color = Color::srgb(0.32, 0.52, 0.95);
const AMBER: Color = Color::srgb(0.98, 0.76, 0.26);

/// Where a piece's handles stand, relative to its middle, and what each one is.
///
/// The single answer to "what handles does this piece have and where are they".
/// `show` spawns from it and nothing else computes it, which is the arrangement
/// that stops the drawn thing and the tested thing drifting apart.
///
/// The move arrows are turned WITH the piece rather than with the world: a wall
/// placed across the room has its length running along world Z, and a red arrow
/// pointing along world X would then stretch it through its own thickness.
fn handles_for(piece: Piece) -> Vec<(Vec3, Handle)> {
    let turn = piece.turn();
    let arm = (-0.12, ARM + 0.12);
    let mut all = vec![
        (
            Vec3::ZERO,
            Handle { at: 0, dir: turn * Vec3::X, grip: Grip::Slide, span: arm, tint: RED },
        ),
        (
            Vec3::ZERO,
            Handle { at: 1, dir: Vec3::Y, grip: Grip::Slide, span: arm, tint: GREEN },
        ),
        (
            Vec3::ZERO,
            Handle { at: 2, dir: turn * Vec3::Z, grip: Grip::Slide, span: arm, tint: BLUE },
        ),
    ];

    // And the pairs that make it bigger, at its own ends — but only where the part
    // HAS the dimension being pulled. A post is a quarter-metre square upright: a
    // stretched one would be a beam wearing a post's name, so it gets no handle to
    // do it with, and only a floor has a width worth growing.
    //
    // # A pinwheel, because four handles in two quadrants crowd each other
    //
    // Each pair stands off to one side of its own axis (ASIDE) so it can never be
    // confused with the arrow on that axis. With both pairs offset the SAME way,
    // the length handle and the width handle at the near corner ended up under a
    // metre apart — inside the widest grab there is. Offset in rotation instead,
    // each end to a different side, and the four sit at equal distance with no two
    // in the same corner.
    let size = piece.size();
    let mut grip_pair = |at: usize, length: bool, back: bool, way: f32, lean: f32| {
        let (out_axis, side_axis) = if length {
            (turn * Vec3::X, turn * Vec3::Z)
        } else {
            (turn * Vec3::Z, turn * Vec3::X)
        };
        let half = if length { size.x } else { size.z } * 0.5;
        let dir = out_axis * way;
        all.push((
            dir * (half + STAND) + side_axis * (ASIDE * lean),
            Handle {
                at,
                dir,
                grip: if length {
                    Grip::Stretch { back }
                } else {
                    Grip::Widen { back }
                },
                // A block on a stalk: grabbable from the stalk's root out to the
                // far side of the block.
                span: (-(STUB + 0.06), KNOB * 0.75),
                tint: AMBER,
            },
        ));
    };
    if piece.part.stretches() {
        grip_pair(3, true, false, 1.0, 1.0);
        grip_pair(4, true, true, -1.0, -1.0);
    }
    if piece.part.widens() {
        grip_pair(5, false, false, 1.0, -1.0);
        grip_pair(6, false, true, -1.0, 1.0);
    }
    all
}

/// How far along `axis` (through `origin`) the closest approach to the ray is.
///
/// The lines are nearly parallel when the axis points at the eye, and there is no
/// sensible answer then — so say so rather than divide by nothing. It is also why
/// every tool that draws these fades an axis out as it turns toward the viewer.
fn along_axis(ray: Ray3d, origin: Vec3, axis: Vec3) -> Option<f32> {
    let toward = *ray.direction;
    let b = axis.dot(toward);
    let denominator = 1.0 - b * b;
    if denominator.abs() < 1.0e-4 {
        return None;
    }
    let w = ray.origin - origin;
    Some((w.dot(axis) - b * w.dot(toward)) / denominator)
}

/// Which handle the pointer is on, given where each one actually IS.
///
/// The origins come from the drawn entities' own `GlobalTransform`, which is the
/// entire point: the thing on screen is the thing being tested.
fn nearest_handle<'a>(
    ray: Ray3d,
    handles: impl Iterator<Item = (Vec3, &'a Handle)>,
) -> Option<(Handle, Vec3)> {
    let mut best: Option<(f32, Handle, Vec3)> = None;
    for (origin, handle) in handles {
        let Some(t) = along_axis(ray, origin, handle.dir) else {
            continue;
        };
        // Only where the handle actually is. Without this a handle can be taken
        // hold of by pointing anywhere along the infinite line it happens to lie on.
        if t < handle.span.0 || t > handle.span.1 {
            continue;
        }
        let on_axis = origin + handle.dir * t;
        // A line has two ends and a ray has one. Without this the handles can be
        // grabbed by pointing AWAY from them, which reads as the tool grabbing at
        // random.
        let reach = (on_axis - ray.origin).dot(*ray.direction);
        if reach <= 0.0 {
            continue;
        }
        let miss = (ray.origin + *ray.direction * reach - on_axis).length();
        if miss > (GRAB * reach).clamp(GRAB_LEAST, GRAB_MOST) {
            continue;
        }
        // The nearest, not the last found. Two handles can both be in range when
        // the view is far enough out that they are a few pixels apart, and the
        // answer then is whichever the pointer is genuinely closer to.
        if best.is_none_or(|(nearest, ..)| miss < nearest) {
            best = Some((miss, *handle, origin));
        }
    }
    best.map(|(_, handle, origin)| (handle, origin))
}

/// Picks the piece the handles sit on.
///
/// # What the pointer is ON, then what the cursor is NEAR
///
/// Two rules, in that order, because the bench has two notions of "where you are".
/// The lattice cursor is where the view ray meets the plane you are building on,
/// and it is the right answer for placing a piece. It is the wrong answer for
/// picking one up: point at the top of a wall and the cursor is on the floor
/// several metres behind it, because that is where the ray goes on past.
///
/// So a piece the ray actually strikes wins, nearest first. Only when the ray
/// strikes nothing does the cursor's own neighbourhood decide, which keeps the
/// old behaviour for pointing at the floor beside a piece — and that rule now
/// measures to the piece's box, so a stretched floor is reachable along its whole
/// length rather than only near its middle.
pub fn choose(
    hand: Res<Hand>,
    bench: Res<Bench>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<BenchEye>>,
    mut holding: ResMut<Holding>,
) {
    // A piece stays chosen until another one is.
    //
    // # The deadlock this replaces
    //
    // The selection used to be let go of whenever the ground cursor wandered out
    // of range, and that made the handles impossible to click. Two frames:
    //
    // 1. The pointer is on the piece. It gets picked. Nothing is hovered.
    // 2. The pointer moves ONTO a handle — which throws the ground cursor far away,
    //    because a handle stands above the floor and the cursor is where the view
    //    ray meets it. The selection is dropped for being out of range, and the
    //    hovering test never runs, because there is no longer a piece to test the
    //    handles of.
    //
    // So the handles vanished the instant they were pointed at, and no amount of
    // fixing the hit test could have helped: nothing was ever hit-tested. Each
    // system waited on the other.
    //
    // Not while the pointer is on a handle. Holding on to the selection is what
    // stops it being dropped when the cursor wanders; this is what stops it being
    // handed to a DIFFERENT piece that the wandering cursor happened to land on,
    // which would take the handles out from under the pointer just as surely.
    if holding.on_a_handle() {
        return;
    }

    // What the pointer is actually on, if the pointer is on anything.
    let struck = windows
        .iter()
        .next()
        .and_then(Window::cursor_position)
        .zip(cameras.iter().next())
        .and_then(|(cursor, (camera, eye))| camera.viewport_to_world(eye, cursor).ok())
        .and_then(|ray| {
            bench
                .pieces()
                .iter()
                .filter_map(|piece| {
                    piece
                        .struck_by(ray.origin, *ray.direction)
                        .map(|along| (along, piece.id))
                })
                .min_by(|a, b| a.0.total_cmp(&b.0))
                .map(|(_, id)| id)
        });

    // Failing that, whatever the lattice cursor is standing in or beside.
    let near = struck.or_else(|| {
        bench
            .pieces()
            .iter()
            .map(|piece| (piece.away_from(hand.at), piece.id))
            .filter(|(away, _)| *away <= REACH)
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, id)| id)
    });

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

/// How far from a piece the lattice cursor may stand and still reach it, in metres.
///
/// A module. It is measured to the piece's BOX, not to its middle, so this is
/// "beside it" rather than "somewhere within a piece and a half of its centre" —
/// which is what the old number meant and why a long piece slipped out of it.
const REACH: f32 = kit::MODULE;

/// Where a piece is, by id.
fn piece_at(bench: &Bench, id: u32) -> Option<Piece> {
    bench.pieces().iter().find(|p| p.id == id).copied()
}

/// Taking hold of a handle, pulling it, and letting go.
pub fn drag(
    buttons: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<BenchEye>>,
    handles: Query<(&Handle, &GlobalTransform)>,
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
    }

    // Which handle the pointer is on, worked out EVERY frame rather than only when
    // the button goes down. What is hovered decides whether the selection holds
    // still and whether a click belongs to the ground, and both of those have to be
    // true before the click arrives.
    let on = nearest_handle(ray, handles.iter().map(|(handle, at)| (at.translation(), handle)));
    if holding.dragging.is_none() {
        let now = on.map(|(handle, _)| handle.at);
        if holding.hovering != now {
            holding.hovering = now;
        }
    }

    let Some(id) = holding.piece else {
        return;
    };
    let Some(piece) = piece_at(&bench, id) else {
        holding.piece = None;
        return;
    };

    // Taking hold.
    if buttons.just_pressed(MouseButton::Left) && holding.dragging.is_none() {
        if let Some((handle, origin)) = on {
            // Slid pieces are measured from the piece; pulled ends are measured
            // from the handle's own place, which is where the maker's hand is.
            let anchor = match handle.grip {
                Grip::Slide => piece.middle(),
                Grip::Stretch { .. } | Grip::Widen { .. } => origin,
            };
            if let Some(t0) = along_axis(ray, anchor, handle.dir) {
                holding.dragging = Some(Drag {
                    at: handle.at,
                    dir: handle.dir,
                    grip: handle.grip,
                    anchor,
                    t0,
                    from: piece.foot,
                    stepped: 0,
                });
            }
        }
        return;
    }

    // Pulling.
    let Some(now) = holding.dragging else {
        return;
    };
    if !buttons.pressed(MouseButton::Left) {
        holding.dragging = None;
        return;
    }
    // Against the ruler as it was when the drag began — see `Drag::anchor`.
    let Some(t) = along_axis(ray, now.anchor, now.dir) else {
        return;
    };
    let moved = t - now.t0;

    match now.grip {
        Grip::Slide => {
            // BY the drag, not TO the pointer, so a piece does not jump the moment
            // it is grabbed off-centre.
            let step = if keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]) {
                kit::SNAP
            } else {
                kit::MODULE
            };
            let put = Bench::snapped_to(now.from + now.dir * moved, step);
            // Never below the floor. A piece under the ground is a piece nobody can
            // see and nobody can select to bring back.
            let put = Vec3::new(put.x, put.y.max(0.0), put.z);
            if put != piece.foot {
                bench.move_to(id, put);
            }
        }
        // Longer, and wider: one gesture measured along whichever axis was
        // grabbed. Which measurement of the piece grows, and which way the foot
        // has to walk when the near end is the one held, are the only differences.
        Grip::Stretch { back } | Grip::Widen { back } => {
            let lengthwise = matches!(now.grip, Grip::Stretch { .. });
            // In whole modules, measured from where the handle was taken hold of.
            let want = (moved / kit::MODULE).round() as i32;
            if want == now.stepped {
                return;
            }
            let measure = |p: &kit::Piece| if lengthwise { p.spans } else { p.across };
            let before = measure(&piece);
            if lengthwise {
                bench.stretch(id, want - now.stepped);
            } else {
                bench.widen(id, want - now.stepped);
            }
            let after = piece_at(&bench, id).map(|p| measure(&p)).unwrap_or(before);
            let applied = after as i32 - before as i32;
            // Clamped at one module or at the limit: the step is NOT paid out, so
            // dragging back the other way lets go again at once rather than having
            // to undo a pull that never happened.
            if applied == 0 {
                return;
            }
            if back {
                // The end the maker is NOT holding stands still. A piece grows
                // forward from its foot, so growing it from the near end means
                // walking the foot back by exactly as much as it grew — along its
                // own length, or across it.
                let axis = if lengthwise { Vec3::X } else { Vec3::Z };
                let along = piece.turn() * axis * (kit::MODULE * applied as f32);
                bench.move_to(id, Bench::snapped(piece.foot - along));
            }
            if let Some(held) = holding.dragging.as_mut() {
                held.stepped += applied;
            }
        }
    }
}

/// Draws the handles on whatever is held.
///
/// A piece that has merely MOVED slides its handles by having the rig moved. Only
/// a change of piece, of facing, or of length builds them again — and that last one
/// is why a drag never holds on to a handle `Entity`: pulling a piece longer
/// despawns the very handle being pulled.
pub fn show(
    mut commands: Commands,
    holding: Res<Holding>,
    bench: Res<Bench>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    rigs: Query<Entity, With<Rig>>,
    // Every measurement the handles' own positions are worked out from. The width
    // had to join it: a floor widened while its handles were up moved its far edge
    // and the handles stayed where the narrower floor had put them.
    mut was: Local<Option<(u32, u8, u32, u32)>>,
) {
    let piece = holding.piece.and_then(|id| piece_at(&bench, id));
    let now = piece.map(|p| (p.id, p.quarters, p.spans, p.across));

    // The empty query matters: leaving the bench takes every handle with it, and a
    // `Local` that still remembers the last selection would otherwise never build
    // them again on the way back in.
    let stale = now != *was || (now.is_some() && rigs.is_empty());
    if !stale {
        if let Some(piece) = piece {
            for rig in &rigs {
                commands
                    .entity(rig)
                    .insert(Transform::from_translation(piece.middle()));
            }
        }
        return;
    }
    *was = now;
    for rig in &rigs {
        commands.entity(rig).despawn();
    }
    let Some(piece) = piece else {
        return;
    };

    let rig = commands
        .spawn((
            OfBench,
            Rig,
            Transform::from_translation(piece.middle()),
            Visibility::default(),
        ))
        .id();

    let shaft = meshes.add(Cuboid::new(SHAFT, ARM - HEAD, SHAFT));
    let head = meshes.add(Cone { radius: HEAD * 0.5, height: HEAD });
    let stalk = meshes.add(Cuboid::new(SHAFT, STUB, SHAFT));
    let knob = meshes.add(Cuboid::new(KNOB, KNOB, KNOB));

    for (offset, handle) in handles_for(piece) {
        // Unlit, and deliberately: a handle is a control, not a thing in the room.
        // Lit, it would go dark on the shaded side and read as part of the building.
        let skin = materials.add(StandardMaterial {
            base_color: handle.tint,
            unlit: true,
            ..default()
        });
        // The bodies are built along Y, so every other direction is that turned.
        let turn = Quat::from_rotation_arc(Vec3::Y, handle.dir);
        let hung = commands
            .spawn((
                handle,
                Transform::from_translation(offset),
                Visibility::default(),
                ChildOf(rig),
            ))
            .id();

        let (near, far): (Handle3d, Handle3d) = match handle.grip {
            Grip::Slide => (
                (shaft.clone(), handle.dir * (ARM - HEAD) * 0.5, turn),
                (head.clone(), handle.dir * (ARM - HEAD * 0.5), turn),
            ),
            // Both pulls are the same gesture on different axes, so they are the
            // same thing to look at: a block on a stalk.
            Grip::Stretch { .. } | Grip::Widen { .. } => (
                (stalk.clone(), -handle.dir * STUB * 0.5, turn),
                (knob.clone(), Vec3::ZERO, Quat::IDENTITY),
            ),
        };
        for (mesh, at, spin) in [near, far] {
            commands.spawn((
                Mesh3d(mesh),
                MeshMaterial3d(skin.clone()),
                Transform::from_translation(at).with_rotation(spin),
                RenderLayers::layer(HANDLE_LAYER),
                ChildOf(hung),
            ));
        }
    }
}

/// One drawn body of a handle: what shape, where, and which way round.
type Handle3d = (bevy::asset::Handle<Mesh>, Vec3, Quat);

/// Lights the handle under the pointer, and the one being pulled.
pub fn light_handles(
    holding: Res<Holding>,
    handles: Query<(&Handle, &Children)>,
    skins: Query<&MeshMaterial3d<StandardMaterial>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let held = holding.dragging.map(|drag| drag.at);
    for (handle, kids) in &handles {
        // Held is brightest, hovered is brighter, the rest are themselves. Nothing
        // else lights up while one is held: the maker is doing a thing, and the
        // handles they are merely passing over are not part of it.
        let lift = if held == Some(handle.at) {
            HELD
        } else if held.is_none() && holding.hovering == Some(handle.at) {
            HOVERED
        } else {
            1.0
        };
        let plain = handle.tint.to_linear();
        let wanted = Color::linear_rgb(
            (plain.red * lift).min(1.0),
            (plain.green * lift).min(1.0),
            (plain.blue * lift).min(1.0),
        );
        for kid in kids {
            let Ok(skin) = skins.get(*kid) else {
                continue;
            };
            let Some(material) = materials.get_mut(&skin.0) else {
                continue;
            };
            // Only when it actually differs. Writing the same colour every frame
            // marks the asset changed every frame, which is a re-upload for nothing.
            if material.base_color != wanted {
                material.base_color = wanted;
            }
        }
    }
}

/// Keeps the handle camera on the bench camera's eye.
///
/// Two cameras looking from one place: the bench draws the room, and this draws the
/// handles over it with the depth buffer cleared between. That is the whole of why
/// a handle can never be swallowed by the piece it is attached to.
pub fn ride_along(
    bench: Query<&Transform, (With<BenchEye>, Without<HandleEye>)>,
    mut overlay: Query<&mut Transform, With<HandleEye>>,
) {
    let Some(eye) = bench.iter().next() else {
        return;
    };
    for mut camera in &mut overlay {
        if *camera != *eye {
            *camera = *eye;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build::kit::Part;

    /// The bench's own opening view, which is where a maker is looking from.
    fn opening_eye() -> Vec3 {
        let (around, pitch, away) = (0.6_f32, 0.55_f32, 13.0_f32);
        Vec3::Y * 1.2
            + Vec3::new(
                around.sin() * pitch.cos(),
                pitch.sin(),
                around.cos() * pitch.cos(),
            ) * away
    }

    /// A ray from an eye THROUGH a point, which is what a ray cast through the
    /// pixel that point is drawn on IS.
    fn aimed_at(eye: Vec3, target: Vec3) -> Ray3d {
        Ray3d {
            origin: eye,
            direction: Dir3::new(target - eye).expect("an eye somewhere other than the target"),
        }
    }

    /// A bench with the real drawing system in it, and no window.
    ///
    /// # This is the test that four rounds of reasoning could not replace
    ///
    /// Every previous attempt at these handles checked the ARITHMETIC and left the
    /// behaviour to be found by whoever ran the game. The arithmetic was right most
    /// of those times. What was wrong was that the numbers described handles in one
    /// place and the game drew them in another, and no test of a formula against
    /// itself can see that.
    ///
    /// So this stands the real system up, lets it spawn real entities, lets the
    /// real transform propagation place them, and then asks the real hit test about
    /// the positions those entities actually ended up at.
    fn handle_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            bevy::app::TaskPoolPlugin::default(),
            bevy::asset::AssetPlugin::default(),
            bevy::transform::TransformPlugin,
        ))
        .init_asset::<Mesh>()
        .init_asset::<StandardMaterial>()
        .init_resource::<Bench>()
        .init_resource::<Holding>()
        .add_systems(Update, show);
        app
    }

    /// A bench with the SELECTION system running, and the lattice cursor as the
    /// only way to point.
    ///
    /// No window and no camera, deliberately: `choose` then cannot cast a ray and
    /// falls through to the cursor's own rule, which is the half that was broken.
    /// Testing the branch that was wrong beats testing the one that was added.
    fn picking_app() -> App {
        let mut app = App::new();
        app.init_resource::<Bench>()
            .init_resource::<Holding>()
            .init_resource::<Hand>()
            .add_systems(Update, choose);
        app
    }

    /// Points the lattice cursor somewhere and runs a frame.
    fn point_at(app: &mut App, at: Vec3) {
        app.world_mut().resource_mut::<Hand>().at = at;
        app.update();
    }

    #[test]
    fn a_stretched_floor_is_selectable_along_its_whole_length() {
        // Reported from the bench: "once an object is placed it seems like I cannot
        // select it again". The selection measured from the lattice cursor to the
        // piece's MIDDLE against a radius of a piece and a half — which a stretched
        // piece is longer than, so its ends fell outside and the handles never
        // appeared for them.
        let mut app = picking_app();
        let id = {
            let mut bench = app.world_mut().resource_mut::<Bench>();
            let id = bench.add(Part::Floor, Vec3::ZERO, 0, 0).expect("a floor");
            bench.stretch(id, 3);
            bench.widen(id, 1);
            id
        };
        let piece = piece_at(&app.world().resource::<Bench>(), id).expect("the floor");
        let half = piece.size() * 0.5;

        // Every module along it, and across it: the cursor stands on the floor.
        for down in 0..=2 {
            for across in 0..=4 {
                let on = piece.middle()
                    + Vec3::new(
                        -half.x + piece.size().x * across as f32 / 4.0,
                        -half.y,
                        -half.z + piece.size().z * down as f32 / 2.0,
                    );
                app.world_mut().resource_mut::<Holding>().piece = None;
                point_at(&mut app, on);
                assert_eq!(
                    app.world().resource::<Holding>().piece,
                    Some(id),
                    "pointing at {on:?} on a four-by-two floor selected nothing"
                );
            }
        }

        // Well off the end of it, nothing new is picked — a reach that reached
        // everywhere would be no reach at all.
        app.world_mut().resource_mut::<Holding>().piece = None;
        point_at(&mut app, piece.middle() + Vec3::new(half.x + kit::MODULE * 3.0, -half.y, 0.0));
        assert_eq!(
            app.world().resource::<Holding>().piece,
            None,
            "a floor was selected from three modules off its end"
        );
    }

    #[test]
    fn pointing_between_two_pieces_takes_the_nearer_one() {
        // The reach is generous now, so which piece wins has to be the near one
        // rather than whichever the list happens to hold first.
        let mut app = picking_app();
        let (near, far) = {
            let mut bench = app.world_mut().resource_mut::<Bench>();
            let near = bench.add(Part::Post, Vec3::ZERO, 0, 0).expect("a post");
            let far = bench
                .add(Part::Post, Vec3::new(kit::MODULE * 2.0, 0.0, 0.0), 0, 0)
                .expect("another post");
            (near, far)
        };
        point_at(&mut app, Vec3::new(kit::MODULE * 0.5, 0.0, 0.0));
        assert_eq!(app.world().resource::<Holding>().piece, Some(near));
        point_at(&mut app, Vec3::new(kit::MODULE * 1.5, 0.0, 0.0));
        assert_eq!(app.world().resource::<Holding>().piece, Some(far));
    }

    /// Puts a piece on the bench, selects it, and runs until its handles stand.
    fn standing(app: &mut App, part: Part, spans: u32) -> Piece {
        let id = {
            let mut bench = app.world_mut().resource_mut::<Bench>();
            let id = bench.add(part, Vec3::ZERO, 0, 0).expect("a piece");
            if spans > 1 {
                bench.stretch(id, spans as i32 - 1);
            }
            id
        };
        app.world_mut().resource_mut::<Holding>().piece = Some(id);
        // Twice: once to spawn, once for the transforms to have been propagated
        // through the rig by the time anything asks where a handle is.
        app.update();
        app.update();
        piece_at(app.world().resource::<Bench>(), id).expect("the piece")
    }

    /// Every handle standing, with the world position it actually ended up at.
    fn as_drawn(app: &mut App) -> Vec<(Vec3, Handle)> {
        app.world_mut()
            .query::<(&Handle, &GlobalTransform)>()
            .iter(app.world())
            .map(|(handle, at)| (at.translation(), *handle))
            .collect()
    }

    #[test]
    fn pointing_at_a_handle_takes_hold_of_that_handle() {
        // The whole bug, in the only form that could have caught it: the handles
        // are DRAWN, their real positions are read back out of the world, and the
        // hit test is asked about those. If what is drawn and what is tested ever
        // drift apart again, this fails.
        for (part, spans) in [
            (Part::Post, 1),
            (Part::Wall, 1),
            (Part::Wall, 3),
            (Part::Floor, 2),
            (Part::Bed, 1),
            (Part::Stairs, 2),
        ] {
            let mut app = handle_app();
            let piece = standing(&mut app, part, spans);
            let drawn = as_drawn(&mut app);

            let wanted = handles_for(piece).len();
            assert_eq!(
                drawn.len(),
                wanted,
                "{} of {spans} drew {} handles, not {wanted}",
                part.name(),
                drawn.len()
            );

            let eye = opening_eye();
            for (at, handle) in &drawn {
                // Aimed at the handle's own body rather than at its origin: a
                // stretch handle's origin is its block, an arrow's is its foot, so
                // the fair target is a little way along what is drawn.
                let target = match handle.grip {
                    Grip::Slide => *at + handle.dir * (ARM * 0.55),
                    Grip::Stretch { .. } | Grip::Widen { .. } => *at,
                };
                let got = nearest_handle(aimed_at(eye, target), drawn.iter().map(|(at, h)| (*at, h)));
                let got = got.map(|(handle, _)| handle.at);
                assert_eq!(
                    got,
                    Some(handle.at),
                    "pointing straight at handle {} of a {} ({spans} module) took hold of {got:?}",
                    handle.at,
                    part.name()
                );
            }
        }
    }

    #[test]
    fn a_stretched_wall_swallows_its_own_move_arrow() {
        // Why the handles have a camera of their own, and the answer to "the red
        // arrow worked once and then disappeared". It worked, it made the wall
        // longer, and the wall ate it: a two-module wall is 3 m long, so its own
        // body reaches 1.5 m from its middle while the red arrow reaches 0.95.
        //
        // Drawn on the bench's layer that is simply invisible. Nothing about the
        // hit test was ever wrong, which is why five goes at the hit test did not
        // help.
        let mut app = handle_app();
        let piece = standing(&mut app, Part::Wall, 2);
        let half = piece.size() * 0.5;

        let tip = handles_for(piece)
            .into_iter()
            .find(|(_, handle)| handle.at == 0)
            .map(|(offset, handle)| offset + handle.dir * ARM)
            .expect("a red arrow");
        let inside = (tip.x.abs() <= half.x) && (tip.y.abs() <= half.y) && (tip.z.abs() <= half.z);
        assert!(
            inside,
            "the wall no longer swallows its arrow, so this test proves nothing: tip at {tip:?} against a half-size of {half:?}"
        );

        // Which is fine, because every drawn body of every handle is on the layer
        // the overlay camera draws after everything else.
        let bodies: Vec<RenderLayers> = app
            .world_mut()
            .query::<(&RenderLayers, &Mesh3d)>()
            .iter(app.world())
            .map(|(layers, _)| layers.clone())
            .collect();
        assert!(!bodies.is_empty(), "no handle drew a body at all");
        for layers in bodies {
            assert!(
                layers.intersects(&RenderLayers::layer(HANDLE_LAYER)),
                "a handle drew on {layers:?}, where the room can hide it"
            );
        }
    }

    #[test]
    fn the_two_kinds_of_handle_never_share_a_line() {
        // A stretch handle on the same line as the red arrow takes the clicks meant
        // for the arrow and gives its own away. Opificium hit exactly this with its
        // roof handles; standing them off to one side is its answer and this is the
        // measurement that says ours really are.
        for spans in 1..=4 {
            let mut bench = Bench::default();
            let id = bench.add(Part::Wall, Vec3::ZERO, 0, 0).expect("a wall");
            bench.stretch(id, spans - 1);
            let piece = piece_at(&bench, id).expect("the wall");
            let all = handles_for(piece);

            let arms: Vec<&(Vec3, Handle)> = all
                .iter()
                .filter(|(_, h)| h.grip == Grip::Slide)
                .collect();
            let pulls: Vec<&(Vec3, Handle)> = all
                .iter()
                .filter(|(_, h)| matches!(h.grip, Grip::Stretch { .. }))
                .collect();
            assert_eq!(pulls.len(), 2, "a wall should have two ends to pull");

            for (pull_at, _) in &pulls {
                for (arm_at, arm) in &arms {
                    // How far the handle stands off the arm's own line.
                    let along = (*pull_at - *arm_at).dot(arm.dir);
                    let off = (*pull_at - *arm_at - arm.dir * along).length();
                    assert!(
                        off > GRAB_MOST,
                        "at {spans} modules a stretch handle sits {off:.3} m off the {:?} arm, inside the widest grab there is",
                        arm.dir
                    );
                }
            }
        }
    }

    #[test]
    fn pulling_the_near_end_leaves_the_far_end_where_it_was() {
        // Which is what a maker means by pulling one end of something. A piece
        // grows forward from its foot, so growing it from the near end has to walk
        // the foot back by as much as it grew — otherwise the end they are NOT
        // holding slides off whatever it was lined up against.
        let mut bench = Bench::default();
        let id = bench.add(Part::Wall, Vec3::ZERO, 0, 0).expect("a wall");
        let far_end = |bench: &Bench| {
            let piece = piece_at(bench, id).expect("the wall");
            piece.middle().x + piece.size().x * 0.5
        };
        let near_end = |bench: &Bench| {
            let piece = piece_at(bench, id).expect("the wall");
            piece.middle().x - piece.size().x * 0.5
        };
        let was = far_end(&bench);

        // The drag's arithmetic for the back handle, run for real.
        for _ in 0..3 {
            let piece = piece_at(&bench, id).expect("the wall");
            let before = piece.spans;
            bench.stretch(id, 1);
            let applied = piece_at(&bench, id).expect("the wall").spans as i32 - before as i32;
            let along = piece.turn() * Vec3::X * (kit::MODULE * applied as f32);
            bench.move_to(id, Bench::snapped(piece.foot - along));
        }

        assert_eq!(piece_at(&bench, id).expect("the wall").spans, 4);
        assert!(
            (far_end(&bench) - was).abs() < 1.0e-4,
            "the end nobody was holding moved from {was:.4} to {:.4}",
            far_end(&bench)
        );
        assert!(
            (near_end(&bench) - (was - 4.0 * kit::MODULE)).abs() < 1.0e-3,
            "the wall grew the wrong way: its near end is at {:.4}",
            near_end(&bench)
        );
    }

    #[test]
    fn a_post_has_nothing_to_pull() {
        // Everything else has a size because of what it IS. A stretched post would
        // be a beam wearing a post's name, and a handle offering to make one is a
        // handle that lies about what the kit will do.
        let mut bench = Bench::default();
        let id = bench.add(Part::Post, Vec3::ZERO, 0, 0).expect("a post");
        let piece = piece_at(&bench, id).expect("the post");
        let all = handles_for(piece);
        assert_eq!(all.len(), 3, "a post wears {} handles", all.len());
        assert!(all.iter().all(|(_, h)| h.grip == Grip::Slide));
    }

    #[test]
    fn a_handle_behind_the_eye_is_never_grabbed() {
        // A line has two ends and a ray has one. Without this the handles can be
        // taken hold of by pointing away from them, which reads as the tool
        // grabbing at random.
        let handle = Handle {
            at: 0,
            dir: Vec3::X,
            grip: Grip::Slide,
            span: (-0.12, ARM + 0.12),
            tint: RED,
        };
        let behind = Ray3d {
            origin: Vec3::new(0.0, 1.0, 5.0),
            direction: Dir3::Z,
        };
        assert!(
            nearest_handle(behind, [(Vec3::new(0.0, 1.0, 0.0), &handle)].into_iter()).is_none(),
            "a handle behind the eye was grabbed"
        );
        // And the same handle from the same place, looked AT, is grabbed.
        let toward = Ray3d {
            origin: Vec3::new(0.4, 1.0, 5.0),
            direction: Dir3::NEG_Z,
        };
        assert!(
            nearest_handle(toward, [(Vec3::new(0.0, 1.0, 0.0), &handle)].into_iter()).is_some(),
            "a handle straight ahead was missed"
        );
    }

    #[test]
    fn a_handle_pointing_at_the_eye_is_not_grabbed_by_dividing_by_nothing() {
        // Looking straight down an axis makes the two lines parallel, and the
        // arithmetic for closest approach divides by a vanishing number. The answer
        // is that it cannot be grabbed, not that it is infinitely close.
        let handle = Handle {
            at: 0,
            dir: Vec3::X,
            grip: Grip::Slide,
            span: (-0.12, ARM + 0.12),
            tint: RED,
        };
        let down_the_axis = Ray3d {
            origin: Vec3::new(-5.0, 1.0, 0.0),
            direction: Dir3::X,
        };
        assert!(along_axis(down_the_axis, Vec3::new(0.0, 1.0, 0.0), Vec3::X).is_none());
        assert!(
            nearest_handle(
                down_the_axis,
                [(Vec3::new(0.0, 1.0, 0.0), &handle)].into_iter()
            )
            .is_none(),
            "an arrow edge-on was grabbed anyway"
        );
    }

    #[test]
    fn pointing_past_the_end_of_an_arrow_grabs_nothing() {
        // Without a span, a handle can be taken hold of anywhere along the infinite
        // line it happens to lie on — which on this bench means grabbing a wall by
        // pointing at the floor ten metres past it.
        let handle = Handle {
            at: 0,
            dir: Vec3::X,
            grip: Grip::Slide,
            span: (-0.12, ARM + 0.12),
            tint: RED,
        };
        let base = Vec3::new(0.0, 1.0, 0.0);
        let eye = base + Vec3::new(0.0, 0.0, 6.0);
        for (along, want) in [(0.5_f32, true), (0.9, true), (2.5, false), (-3.0, false)] {
            let got = nearest_handle(
                aimed_at(eye + Vec3::X * along, base + Vec3::X * along),
                [(base, &handle)].into_iter(),
            );
            assert_eq!(
                got.is_some(),
                want,
                "pointing {along} m along a {ARM} m arrow was {}",
                if got.is_some() { "a grab" } else { "a miss" }
            );
        }
    }

    #[test]
    fn the_arrows_turn_with_the_piece() {
        // A wall placed across the room has its length along world Z. A red arrow
        // pointing along world X would then stretch it through its own thickness —
        // which is the whole reason these are the piece's axes and not the world's.
        let mut bench = Bench::default();
        let id = bench.add(Part::Wall, Vec3::ZERO, 1, 0).expect("a turned wall");
        let piece = piece_at(&bench, id).expect("the wall");
        let all = handles_for(piece);

        let arm = |at: usize| all.iter().find(|(_, h)| h.at == at).expect("an arm").1.dir;
        assert!(
            arm(0).dot(Vec3::NEG_Z).abs() > 0.99,
            "a quarter-turned wall's length arrow points {:?}",
            arm(0)
        );
        // Up is up whichever way a thing is turned. A piece cannot be rotated onto
        // its side here, so there is no case where its own up is not the world's.
        assert_eq!(arm(1), Vec3::Y);
        assert!(arm(2).dot(Vec3::X).abs() > 0.99, "the third arm is {:?}", arm(2));

        // And the stretch handles follow the length, not the world.
        let pull = all
            .iter()
            .find(|(_, h)| h.at == 3)
            .expect("a stretch handle")
            .1
            .dir;
        assert!(
            pull.dot(arm(0)) > 0.99,
            "the stretch handle pulls {pull:?} while the piece runs {:?}",
            arm(0)
        );
    }

    #[test]
    fn the_axes_are_the_colours_every_tool_uses() {
        // The one thing a maker should not have to learn here.
        let mut bench = Bench::default();
        let id = bench.add(Part::Wall, Vec3::ZERO, 0, 0).expect("a wall");
        let piece = piece_at(&bench, id).expect("the wall");
        let all = handles_for(piece);
        let tint = |at: usize| all.iter().find(|(_, h)| h.at == at).expect("a handle").1.tint;

        let reddest = tint(0).to_linear();
        assert!(reddest.red > reddest.green && reddest.red > reddest.blue);
        let greenest = tint(1).to_linear();
        assert!(greenest.green > greenest.red && greenest.green > greenest.blue);
        let bluest = tint(2).to_linear();
        assert!(bluest.blue > bluest.red && bluest.blue > bluest.green);
        // And the other job wears another colour entirely.
        assert_ne!(tint(3), tint(0), "the stretch handle wears the length's own red");
        assert_eq!(tint(3), tint(4), "the two ends of one piece disagree");
    }

    #[test]
    fn the_selection_survives_the_cursor_leaving_the_piece() {
        // The exact sequence that made the handles unclickable, run frame by frame.
        //
        // Pointing at a handle throws the ground cursor away from the piece — a
        // handle stands above the floor, and the cursor is where the view ray meets
        // it. The selection was dropped for being out of range, and the hovering
        // test never ran, because there was no longer a piece to test the handles
        // of. Each system waited on the other.
        let mut bench = Bench::default();
        let post = bench.add(Part::Post, Vec3::ZERO, 0, 0).expect("a post");
        let far = bench
            .add(Part::Post, Vec3::new(30.0, 0.0, 30.0), 0, 0)
            .expect("another post, well away");

        let mut app = App::new();
        app.insert_resource(bench)
            .insert_resource(Hand::default())
            .init_resource::<Holding>()
            .add_systems(Update, choose);

        // Frame one: the cursor is on the post, and it is chosen.
        app.update();
        assert_eq!(app.world().resource::<Holding>().piece, Some(post));

        // Frame two: the pointer reaches for a handle, which throws the ground
        // cursor metres away. NOTHING is hovered yet — that is the whole point,
        // since hovering cannot be discovered until the handles survive this frame.
        app.world_mut().resource_mut::<Hand>().at = Vec3::new(9.0, 0.0, 9.0);
        app.update();
        assert_eq!(
            app.world().resource::<Holding>().piece,
            Some(post),
            "the piece was let go of the moment its own handle was reached for"
        );

        // Pointing at a different piece does change the selection — holding on must
        // not mean getting stuck.
        app.world_mut().resource_mut::<Hand>().at = Vec3::new(30.0, 0.0, 30.0);
        app.update();
        assert_eq!(app.world().resource::<Holding>().piece, Some(far));

        // And a piece taken off the bench is let go of rather than held for ever.
        app.world_mut()
            .resource_mut::<Bench>()
            .remove_nearest(Vec3::new(30.0, 0.0, 30.0), kit::MODULE);
        app.world_mut().resource_mut::<Hand>().at = Vec3::new(60.0, 0.0, 60.0);
        app.update();
        assert_eq!(
            app.world().resource::<Holding>().piece,
            None,
            "the handles stayed on a piece that is no longer there"
        );
    }

    #[test]
    fn reaching_for_a_handle_does_not_hand_it_to_something_else() {
        // The other half of the deadlock. Holding on to the selection stops it
        // being dropped when the cursor wanders; this stops it being handed to a
        // DIFFERENT piece the wandering cursor landed on, which would take the
        // handles out from under the pointer just as surely.
        let mut bench = Bench::default();
        let id = bench.add(Part::Post, Vec3::ZERO, 0, 0).expect("a post");

        let mut app = App::new();
        app.insert_resource(bench)
            .insert_resource(Hand::default())
            .init_resource::<Holding>()
            .add_systems(Update, choose);

        app.update();
        assert_eq!(app.world().resource::<Holding>().piece, Some(id));

        let other = app
            .world_mut()
            .resource_mut::<Bench>()
            .add(Part::Post, Vec3::new(40.0, 0.0, 40.0), 0, 0)
            .expect("another post");

        app.world_mut().resource_mut::<Holding>().hover_for_test(1);
        app.world_mut().resource_mut::<Hand>().at = Vec3::new(40.0, 0.0, 40.0);
        app.update();
        assert_eq!(
            app.world().resource::<Holding>().piece,
            Some(id),
            "reaching for a handle handed it to whatever the cursor landed on"
        );

        // With nothing hovered, that same cursor does choose the second piece.
        app.world_mut().resource_mut::<Holding>().hovering = None;
        app.update();
        assert_eq!(app.world().resource::<Holding>().piece, Some(other));
    }

    #[test]
    fn a_drag_is_measured_from_where_it_started_not_from_where_it_has_got_to() {
        // The jitter. A drag used to measure itself against the handles where they
        // are NOW — but the handles sit on the piece, and the piece is what the drag
        // is moving. Every step moved the ruler the next step was measured from, so
        // the piece oscillated instead of sliding.
        let eye = Vec3::new(7.0, 5.0, 9.0);
        let start = Vec3::new(0.0, 0.625, 0.0);
        let ray = aimed_at(eye, Vec3::new(0.7, 0.625, 0.0));

        let from_start = along_axis(ray, start, Vec3::X).expect("a reading");
        // The piece has since been dragged a module along, taking its handles with
        // it. Read against THAT line the same pointer says something different, and
        // the difference is the jump.
        let from_moved =
            along_axis(ray, start + Vec3::X * kit::MODULE, Vec3::X).expect("a reading");
        assert!(
            (from_start - from_moved).abs() > 0.5,
            "a moved ruler read the same, so this test proves nothing: {from_start:.3} against {from_moved:.3}"
        );

        // Which is why the drag keeps the ruler it started with. Reading twice from
        // the same start gives the same answer, whatever the piece has done.
        let again = along_axis(ray, start, Vec3::X).expect("a reading");
        assert!(
            (from_start - again).abs() < 1.0e-6,
            "the same pointer against the same ruler gave two answers"
        );
    }
}
