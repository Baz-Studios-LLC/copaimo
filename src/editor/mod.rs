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
//! **Reuse.** This module plus `world/edit.rs` is the tool. Everything it needs
//! from the host project is narrow and listed here, so pointing it at another
//! world is a matter of supplying these rather than untangling it:
//!
//! * a heightfield to raycast and read — `Terrain::height` / `base_height`
//! * an offset grid to write — `Terrain::edits`
//! * a way to invalidate meshes over a rectangle — `invalidate_area` below
//! * a camera to aim from — any entity with a `GlobalTransform`

mod minimap;
mod theme;
mod ui;

use bevy::prelude::*;

use crate::camera::{CameraMode, MainCamera};
use crate::config::{CHUNK_SIZE, EDIT_CELL};
use crate::states::AppState;
use crate::world::edit::{BrushOp, Stamp};
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

/// Blend rate per second for the tools that converge on a target rather than
/// pushing at a fixed speed.
const BLEND_RATE: f32 = 4.0;

#[derive(Resource)]
pub struct Brush {
    pub radius: f32,
    /// Vertical speed in meters per second for the directional tools.
    pub strength: f32,
    pub op: BrushOp,
    /// Where the brush is currently pointed, if it's on the ground at all.
    pub hit: Option<Vec3>,
    /// Height captured when a levelling stroke began, so the whole stroke
    /// levels to one plane instead of chasing the ground as it moves.
    flatten_target: f32,
    /// Whether a stroke is currently open, for undo grouping.
    stroking: bool,
}

impl Default for Brush {
    fn default() -> Self {
        Self {
            radius: 40.0,
            strength: 25.0,
            op: BrushOp::Raise,
            hit: None,
            flatten_target: 0.0,
            stroking: false,
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
                (aim_brush, adjust_brush, paint, history, save_edits, draw_brush)
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

    const TOOL_KEYS: [KeyCode; 6] = [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
    ];
    for (key, op) in TOOL_KEYS.iter().zip(BrushOp::ALL) {
        if keys.just_pressed(*key) {
            brush.op = op;
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
    // Right button inverts the stroke, so raising and lowering are the same
    // gesture rather than a mode switch.
    let inverted = buttons.pressed(MouseButton::Right);
    let painting = buttons.pressed(MouseButton::Left) || inverted;

    // Open and close the undo group around the whole drag, so a stroke lasting
    // two hundred frames undoes in one step rather than two hundred.
    if painting && !brush.stroking {
        if let Ok(mut edits) = terrain.edits().write() {
            edits.begin_stroke();
        }
        brush.stroking = true;
        brush.flatten_target = brush.hit.map_or(0.0, |hit| hit.y);
    } else if !painting && brush.stroking {
        if let Ok(mut edits) = terrain.edits().write() {
            edits.end_stroke();
        }
        brush.stroking = false;
    }

    let Some(hit) = brush.hit.filter(|_| painting) else {
        return;
    };

    let op = match (brush.op, inverted) {
        (BrushOp::Raise, true) => BrushOp::Lower,
        (BrushOp::Lower, true) => BrushOp::Raise,
        (op, _) => op,
    };

    let amount = if op.is_directional() {
        brush.strength * time.delta_secs()
    } else {
        BLEND_RATE * time.delta_secs()
    };

    let area = {
        let Ok(mut edits) = terrain.edits().write() else {
            return;
        };
        // Reads the generator directly, never back through the edit layer —
        // that would deadlock against the write lock held right here.
        let base = |p: Vec2| terrain.base_height(p.x, p.y);
        edits.apply(&Stamp {
            center: Vec2::new(hit.x, hit.z),
            radius: brush.radius,
            op,
            amount,
            target: brush.flatten_target,
            base: &base,
        })
    };

    invalidate_area(&mut commands, &terrain, &chunks, &busy, area);
}

fn history(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    terrain: Res<TerrainSource>,
    chunks: Res<ChunkMap>,
    busy: Query<(), With<PendingChunk>>,
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

    let area = {
        let Ok(mut edits) = terrain.edits().write() else {
            return;
        };
        if undo {
            edits.undo()
        } else {
            edits.redo()
        }
    };

    match area {
        Some(area) => {
            toast.show(if undo { "Undone" } else { "Redone" });
            invalidate_area(&mut commands, &terrain, &chunks, &busy, area);
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
    area: Rect,
) {
    let low = ((area.min - EDIT_CELL) / CHUNK_SIZE).floor().as_ivec2();
    let high = ((area.max + EDIT_CELL) / CHUNK_SIZE).floor().as_ivec2();

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
    let color = theme::tool_color(brush.op);

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
    if brush.op == BrushOp::Path {
        ring(brush.radius * 0.7, color.with_alpha(0.45));
    }

    // A short mast at the center, so the brush is findable when the ring falls
    // out of view behind a rise.
    gizmos.line(hit, hit + Vec3::Y * 3.0, color);
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

    let Ok(mut edits) = terrain.edits().write() else {
        return;
    };
    match edits.save() {
        Ok(()) => {
            let cells = edits.sculpted_cells();
            info!("saved terrain edits ({cells} cells)");
            toast.show(format!("Saved {cells} cells"));
        }
        Err(err) => {
            error!("could not save terrain edits: {err}");
            toast.show("Save failed - see log");
        }
    }
}
