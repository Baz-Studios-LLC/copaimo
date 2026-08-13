//! Small math helpers shared across plugins.
//!
//! Anything used by more than one module lives here rather than being copied —
//! terrain coloring, the player, and (later) monsters all want the same
//! smoothstep and turn-to-face math.

use bevy::prelude::*;

/// Hermite smoothstep: 0 at `edge0`, 1 at `edge1`, eased in between.
/// Used all over the biome blending so bands fade rather than banding hard.
///
/// Passing `edge0 > edge1` deliberately gives a *descending* ramp — 1 at
/// `edge1`, falling to 0 at `edge0`. The coastline fade relies on that, so the
/// only case guarded here is edges that are equal and have no ramp at all.
pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let span = edge1 - edge0;
    if span.abs() < f32::EPSILON {
        return if x < edge0 { 0.0 } else { 1.0 };
    }
    let t = ((x - edge0) / span).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Rotation that faces `dir` along the model's forward axis, keeping the model
/// upright. Returns `None` for a direction with no horizontal component, so
/// callers can hold their previous facing instead of snapping to a default.
pub fn facing_quat(dir: Vec3) -> Option<Quat> {
    let flat = Vec3::new(dir.x, 0.0, dir.z);
    if flat.length_squared() < 1.0e-6 {
        return None;
    }
    Some(Quat::from_rotation_y(flat.x.atan2(flat.z)))
}
