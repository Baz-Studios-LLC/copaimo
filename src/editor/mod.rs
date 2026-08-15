//! The terrain sculpting tool — a separate mode, entered from the main menu.
//!
//! A brush follows the crosshair across the ground and raises, lowers, smooths,
//! flattens, cuts paths through or roughens it. Edits go to the hand-edit layer
//! (`world/edit.rs`) as offsets on top of generated terrain, so sculpted
//! geography survives re-rolling the noise or swapping the map image.
//!
//! Affected chunks re-mesh live as you paint, reusing the same background task
//! machinery that streaming uses — the old mesh stays on screen until the new
//! one is ready, so the ground never blinks out from under you.
//!
//! **Where the brush itself lives.** Not here — in `terrain-core`, which
//! Opificium's terrain bench runs too. This module is the *mode*: aiming, the
//! gestures, the panel, and telling chunks to mesh again. That split is why the
//! tool could come back into the game at all, and it is how the studios do it:
//! the editor is built on top of the runtime, and the shaping code exists once.
//!
//! **Reuse.** This module plus `world/edit.rs` is the tool. Everything it needs
//! from the host project is narrow and listed here, so pointing it at another
//! world is a matter of supplying these rather than untangling it:
//!
//! * a heightfield to raycast and read — `Terrain::height` / `base_height`
//! * an offset grid to write — `Terrain::edits`
//! * a painted forest layer — `Terrain::woods`
//! * a way to invalidate meshes over a rectangle — `invalidate_area` below
//! * a camera to aim from — any entity with a `GlobalTransform`

mod minimap;
mod theme;
mod ui;

use bevy::prelude::*;

use crate::camera::{CameraMode, MainCamera};
use crate::config::{CHUNK_SIZE, EDIT_CELL};
use crate::states::AppState;
use crate::world::edit::{Brushing, Patch, Stamp};
use crate::world::stream::{spawn_chunk_mesh, ChunkMap, PendingChunk};
use crate::world::terrain::{Terrain, TerrainSource};

/// How far the brush can reach from the camera, in meters.
const REACH: f32 = 800.0;
/// Ray march step. Small enough not to tunnel through a ridge, large enough
/// that a full-reach miss is a few hundred samples rather than thousands.
const MARCH_STEP: f32 = 1.5;

pub const MIN_RADIUS: f32 = 4.0;
pub const MAX_RADIUS: f32 = 500.0;
/// Radius changes by a proportion per wheel notch rather than a fixed number of
/// meters, so it feels the same whether you're shaping a mound or a range.
const RADIUS_STEP: f32 = 1.15;

pub const MIN_STRENGTH: f32 = 2.0;
pub const MAX_STRENGTH: f32 = 150.0;
const STRENGTH_STEP: f32 = 1.25;

#[derive(Resource)]
pub struct Brush {
    pub radius: f32,
    /// What strength means depends on the tool — meters per second to the ones
    /// that push, a blend fraction to the ones that level, a settling pace to
    /// erosion. `Brushing::rate` is the one place that decides which.
    pub strength: f32,
    pub how: Brushing,
    /// Where the brush is currently pointed, if it's on the ground at all.
    pub hit: Option<Vec3>,
    /// Height captured when a levelling stroke began, so the whole stroke
    /// levels to one plane instead of chasing the ground as it moves.
    flatten_target: f32,
    /// Whether a stroke is currently open, for undo grouping.
    stroking: bool,
    /// Which layer the open stroke was started on. The tool can change
    /// mid-drag, so closing goes by this rather than by what is selected now.
    strokes: Layer,
    /// The first end of a ramp, once it has been clicked. A ramp is laid
    /// between two points rather than dragged, so it needs somewhere to
    /// remember the first one.
    pub ramp_from: Option<Vec3>,
    /// Which layer each stroke went to, oldest first.
    ///
    /// The ground and the woods keep their own histories, and neither can know
    /// about the other. Undo means "take back the last thing I did", so
    /// something has to remember what that was — otherwise pressing it after
    /// planting silently reaches past the wood and takes back a hillside.
    order: Vec<Layer>,
}

/// Which of the two painted layers a stroke touched.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Layer {
    Ground,
    Woods,
}

/// As deep as either layer's own history. Beyond this the layer has forgotten
/// the stroke anyway, so remembering that it happened would only mislead.
const ORDER_DEPTH: usize = 64;

impl Default for Brush {
    fn default() -> Self {
        Self {
            radius: 40.0,
            strength: 25.0,
            how: Brushing::Raise,
            hit: None,
            flatten_target: 0.0,
            stroking: false,
            strokes: Layer::Ground,
            ramp_from: None,
            order: Vec::new(),
        }
    }
}

impl Brush {
    /// Remembers that a stroke landed, so undo knows where to look.
    fn stroked(&mut self, layer: Layer) {
        self.order.push(layer);
        if self.order.len() > ORDER_DEPTH {
            self.order.remove(0);
        }
    }
}

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Brush>()
            .add_plugins((ui::EditorUiPlugin, minimap::MinimapPlugin))
            .add_systems(OnEnter(AppState::Editing), enter_editor)
            .add_systems(
                Update,
                (
                    aim_brush,
                    adjust_brush,
                    paint,
                    lay_ramp,
                    history,
                    save_edits,
                    draw_brush,
                )
                    .chain()
                    .run_if(in_state(AppState::Editing)),
            );
    }
}

fn enter_editor(mut camera: ResMut<CameraMode>) {
    // Sculpting from the follow camera means aiming past your own ranger at
    // whatever happens to be in front of them. Free-fly is what the tool wants.
    *camera = CameraMode::Fly;
}

/// Marches the camera's view ray until it goes under the ground, then binary
/// searches the last step for the crossing. Cheaper and simpler than colliding
/// against chunk meshes, and it works on terrain that hasn't been meshed yet.
fn raycast_terrain(terrain: &Terrain, origin: Vec3, direction: Vec3) -> Option<Vec3> {
    let mut previous = origin;
    let mut travelled = 0.0;

    while travelled < REACH {
        travelled += MARCH_STEP;
        let point = origin + direction * travelled;

        if point.y <= terrain.height(point.x, point.z) {
            // `low` is always above ground and `high` always below, so the
            // surface is bracketed and this converges on it.
            let (mut low, mut high) = (previous, point);
            for _ in 0..16 {
                let middle = (low + high) * 0.5;
                if middle.y <= terrain.height(middle.x, middle.z) {
                    high = middle;
                } else {
                    low = middle;
                }
            }
            return Some(high);
        }
        previous = point;
    }

    None
}

fn aim_brush(
    terrain: Res<TerrainSource>,
    cameras: Query<&GlobalTransform, With<MainCamera>>,
    mut brush: ResMut<Brush>,
) {
    let Some(camera) = cameras.iter().next() else {
        return;
    };
    brush.hit = raycast_terrain(&terrain.0, camera.translation(), camera.forward().as_vec3());
}

fn adjust_brush(
    keys: Res<ButtonInput<KeyCode>>,
    scroll: Res<bevy::input::mouse::AccumulatedMouseScroll>,
    mut brush: ResMut<Brush>,
) {
    if scroll.delta.y != 0.0 {
        let factor = RADIUS_STEP.powf(scroll.delta.y);
        brush.radius = (brush.radius * factor).clamp(MIN_RADIUS, MAX_RADIUS);
    }

    if keys.just_pressed(KeyCode::BracketRight) {
        brush.strength = (brush.strength * STRENGTH_STEP).min(MAX_STRENGTH);
    }
    if keys.just_pressed(KeyCode::BracketLeft) {
        brush.strength = (brush.strength / STRENGTH_STEP).max(MIN_STRENGTH);
    }

    const TOOL_KEYS: [KeyCode; 9] = [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
        KeyCode::Digit7,
        KeyCode::Digit8,
        KeyCode::Digit9,
    ];
    for (key, how) in TOOL_KEYS.iter().zip(Brushing::ALL) {
        if keys.just_pressed(*key) {
            brush.how = how;
            // Half a ramp with a different tool selected would lay itself from
            // wherever it was left the next time the tool came back around.
            brush.ramp_from = None;
        }
    }
}

fn paint(
    mut commands: Commands,
    time: Res<Time>,
    buttons: Res<ButtonInput<MouseButton>>,
    terrain: Res<TerrainSource>,
    chunks: Res<ChunkMap>,
    busy: Query<(), With<PendingChunk>>,
    mut brush: ResMut<Brush>,
) {
    // Laid between two clicked points, not dragged. `lay_ramp` has it.
    if brush.how.is_two_point() {
        return;
    }

    // Right button inverts the stroke, so raising and lowering — and planting
    // and clearing — are one gesture rather than a mode switch.
    let inverted = buttons.pressed(MouseButton::Right);
    let painting = buttons.pressed(MouseButton::Left) || inverted;

    // Open and close the undo group around the whole drag, so a stroke lasting
    // two hundred frames undoes in one step rather than two hundred. Whichever
    // layer the tool writes to keeps its own, and `brush.order` remembers which
    // it was so undo can find it again.
    let layer = if brush.how.is_planting() {
        Layer::Woods
    } else {
        Layer::Ground
    };

    if painting && !brush.stroking {
        match layer {
            Layer::Ground => {
                if let Ok(mut edits) = terrain.edits().write() {
                    edits.begin_stroke();
                }
            }
            Layer::Woods => {
                if let Ok(mut woods) = terrain.woods().write() {
                    woods.begin_stroke();
                }
            }
        }
        brush.stroking = true;
        brush.strokes = layer;
        brush.flatten_target = brush.hit.map_or(0.0, |hit| hit.y);
    } else if !painting && brush.stroking {
        // Closed against the layer the stroke OPENED on, not the one selected
        // now — the tool can be changed mid-drag, and closing the wrong layer
        // would leave a group open forever and lose the drag.
        match brush.strokes {
            Layer::Ground => {
                if let Ok(mut edits) = terrain.edits().write() {
                    edits.end_stroke();
                }
            }
            Layer::Woods => {
                if let Ok(mut woods) = terrain.woods().write() {
                    woods.end_stroke();
                }
            }
        }
        let closed = brush.strokes;
        brush.stroked(closed);
        brush.stroking = false;
    }

    let Some(hit) = brush.hit.filter(|_| painting) else {
        return;
    };

    let how = match (brush.how, inverted) {
        (Brushing::Raise, true) => Brushing::Lower,
        (Brushing::Lower, true) => Brushing::Raise,
        (how, _) => how,
    };
    let amount = how.rate(brush.strength, time.delta_secs());
    let at = Vec2::new(hit.x, hit.z);

    // Planting touches the woods and never the ground. A brush that moved earth
    // as well would dig a hole every time somebody grew a stand of trees.
    let patch = if how.is_planting() {
        let Ok(mut woods) = terrain.woods().write() else {
            return;
        };
        // Negative bias clears, and zero leaves the ground's own answer alone —
        // which is why the right button thins a wood rather than paving it.
        woods.paint(at, brush.radius, if inverted { -amount } else { amount })
    } else {
        let Ok(mut edits) = terrain.edits().write() else {
            return;
        };
        // Reads the generator directly, never back through the edit layer —
        // that would deadlock against the write lock held right here.
        let under = |p: Vec2| terrain.base_height(p.x, p.y);
        edits.apply(&Stamp {
            centre: at,
            radius: brush.radius,
            how,
            amount,
            target: brush.flatten_target,
            under: &under,
        })
    };

    invalidate_area(&mut commands, &terrain, &chunks, &busy, patch);
}

/// The ramp: click once for the foot, once for the head, and a graded run is
/// laid between them in a single stroke.
///
/// A gesture of its own because the levelling brushes pull toward ONE height,
/// which is right for a town square and useless for a road up a hillside. The
/// right button cancels a half-placed one.
fn lay_ramp(
    mut commands: Commands,
    buttons: Res<ButtonInput<MouseButton>>,
    terrain: Res<TerrainSource>,
    chunks: Res<ChunkMap>,
    busy: Query<(), With<PendingChunk>>,
    mut brush: ResMut<Brush>,
    mut toast: ResMut<ui::Toast>,
) {
    if !brush.how.is_two_point() {
        return;
    }

    if buttons.just_pressed(MouseButton::Right) && brush.ramp_from.take().is_some() {
        toast.show("Ramp cancelled");
        return;
    }
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }
    let Some(hit) = brush.hit else {
        return;
    };

    let Some(from) = brush.ramp_from.take() else {
        brush.ramp_from = Some(hit);
        toast.show("Ramp: now click the far end");
        return;
    };

    let patch = {
        let Ok(mut edits) = terrain.edits().write() else {
            return;
        };
        let under = |p: Vec2| terrain.base_height(p.x, p.y);
        // Laid inside a stroke of its own so one press takes the whole run back,
        // the same as a drag with any other tool.
        edits.begin_stroke();
        let patch = edits.ramp(from, hit, brush.radius, &under);
        edits.end_stroke();
        patch
    };

    brush.stroked(Layer::Ground);
    toast.show(format!("Ramp laid, {:.0} m", from.distance(hit)));
    invalidate_area(&mut commands, &terrain, &chunks, &busy, patch);
}

fn history(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    terrain: Res<TerrainSource>,
    chunks: Res<ChunkMap>,
    busy: Query<(), With<PendingChunk>>,
    mut brush: ResMut<Brush>,
    mut toast: ResMut<ui::Toast>,
) {
    let control = keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    if !control {
        return;
    }
    let shift = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);

    // Ctrl+Z undoes; Ctrl+Y and Ctrl+Shift+Z both redo, since both conventions
    // are in common use and neither costs anything to support.
    let redo = keys.just_pressed(KeyCode::KeyY) || (shift && keys.just_pressed(KeyCode::KeyZ));
    let undo = !shift && keys.just_pressed(KeyCode::KeyZ);
    if !undo && !redo {
        return;
    }

    // Which layer to reach into. Undoing takes the last stroke off the end;
    // redoing puts one back, so it goes to whichever layer would be next.
    let layer = if undo {
        brush.order.pop()
    } else {
        let ground = terrain.edits().read().is_ok_and(|edits| edits.can_redo());
        let woods = terrain.woods().read().is_ok_and(|woods| woods.can_redo());
        // Only one can be ambiguous, and only after undoing across both. The
        // ground wins that tie because it is what most of the work is.
        match (ground, woods) {
            (true, _) => Some(Layer::Ground),
            (false, true) => Some(Layer::Woods),
            (false, false) => None,
        }
    };

    let patch = match layer {
        Some(Layer::Ground) => terrain.edits().write().ok().and_then(|mut edits| {
            if undo {
                edits.undo()
            } else {
                edits.redo()
            }
        }),
        Some(Layer::Woods) => terrain.woods().write().ok().and_then(|mut woods| {
            if undo {
                woods.undo()
            } else {
                woods.redo()
            }
        }),
        None => None,
    };

    match patch {
        Some(patch) => {
            if !undo {
                if let Some(layer) = layer {
                    brush.stroked(layer);
                }
            }
            toast.show(match (undo, layer) {
                // Named, because taking back a hillside and taking back a wood
                // look nothing alike and the wood may be behind you.
                (true, Some(Layer::Woods)) => "Planting undone",
                (true, _) => "Undone",
                (false, Some(Layer::Woods)) => "Planting redone",
                (false, _) => "Redone",
            });
            invalidate_area(&mut commands, &terrain, &chunks, &busy, patch);
        }
        // Say so rather than doing nothing — a dead shortcut and an empty
        // history look identical otherwise.
        None => toast.show(if undo {
            "Nothing to undo"
        } else {
            "Nothing to redo"
        }),
    }
}

/// Queues a re-mesh of every loaded chunk overlapping a world-space rectangle.
///
/// The edit layer is sampled bilinearly, so a change influences one cell beyond
/// its own bounds — hence the margin, without which chunk seams would drift
/// apart along the edge of a stroke.
fn invalidate_area(
    commands: &mut Commands,
    terrain: &TerrainSource,
    chunks: &ChunkMap,
    busy: &Query<(), With<PendingChunk>>,
    patch: Patch,
) {
    let (min, max) = patch;
    let low = ((min - EDIT_CELL) / CHUNK_SIZE).floor().as_ivec2();
    let high = ((max + EDIT_CELL) / CHUNK_SIZE).floor().as_ivec2();

    for z in low.y..=high.y {
        for x in low.x..=high.x {
            let coord = IVec2::new(x, z);
            let Some(&entity) = chunks.loaded.get(&coord) else {
                continue;
            };
            // Already rebuilding: skip rather than queue a second task. This is
            // what throttles painting — a chunk rebuilds as fast as it can and
            // no faster, however many frames the stroke lasts.
            if busy.contains(entity) {
                continue;
            }
            spawn_chunk_mesh(commands, entity, terrain, coord);
        }
    }
}

fn draw_brush(mut gizmos: Gizmos, terrain: Res<TerrainSource>, brush: Res<Brush>) {
    let Some(hit) = brush.hit else {
        return;
    };

    // Drawn as a ring of short segments sampled at ground height rather than a
    // flat circle, so on a slope it wraps the terrain and you can see exactly
    // what the stroke will cover.
    const SEGMENTS: usize = 72;
    let color = theme::tool_color(brush.how);

    let point_at = |index: usize, radius: f32| {
        let angle = index as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
        let x = hit.x + angle.cos() * radius;
        let z = hit.z + angle.sin() * radius;
        Vec3::new(x, terrain.height(x, z) + 0.4, z)
    };

    let mut ring = |radius: f32, color: Color| {
        let mut previous = point_at(0, radius);
        for index in 1..=SEGMENTS {
            let next = point_at(index, radius);
            gizmos.line(previous, next, color);
            previous = next;
        }
    };

    ring(brush.radius, color);
    // Path has a flat bed out to 70% of its radius; showing that inner edge is
    // the difference between placing a road accurately and guessing.
    if brush.how == Brushing::Path {
        ring(brush.radius * 0.7, color.with_alpha(0.45));
    }

    // A short mast at the center, so the brush is findable when the ring falls
    // out of view behind a rise.
    gizmos.line(hit, hit + Vec3::Y * 3.0, color);

    // Half a ramp is invisible otherwise: the first click lands somewhere behind
    // you and there is nothing on screen saying a run is waiting on its far end.
    if let Some(from) = brush.ramp_from {
        gizmos.line(from, from + Vec3::Y * 6.0, color);
        gizmos.line(from, hit, color.with_alpha(0.7));
    }
}

fn save_edits(
    keys: Res<ButtonInput<KeyCode>>,
    terrain: Res<TerrainSource>,
    mut toast: ResMut<ui::Toast>,
) {
    let pressed_save = keys.just_pressed(KeyCode::KeyS)
        && keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    if !pressed_save {
        return;
    }

    // Ground and woods together, in one keystroke. They were separate once and
    // planting quietly failed to survive a restart; a maker who has just spent
    // an afternoon on a hillside should not have to know there are two files.
    let ground = {
        let Ok(mut edits) = terrain.edits().write() else {
            return;
        };
        crate::world::edit::save(&mut edits).map(|()| edits.sculpted_cells())
    };
    let woods = {
        let Ok(painted) = terrain.woods().read() else {
            return;
        };
        crate::world::forest::save(&painted).map(|()| painted.painted_cells())
    };

    match (ground, woods) {
        (Ok(cells), Ok(planted)) => {
            info!("saved {cells} sculpted cells and {planted} planted");
            toast.show(format!("Saved {cells} sculpted, {planted} planted"));
        }
        // Said separately, because which one failed decides what was lost.
        (Err(why), _) => {
            error!("could not save the sculpted ground: {why}");
            toast.show("Ground not saved - see log");
        }
        (_, Err(why)) => {
            error!("could not save the planted woods: {why}");
            toast.show("Woods not saved - see log");
        }
    }
}
