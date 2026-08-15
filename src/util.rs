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

/// How many wheel notches were scrolled this frame, whatever the device.
///
/// `AccumulatedMouseScroll` reports LINES for a notched wheel and PIXELS for a
/// trackpad or a high-resolution one — and a single flick of the second kind is
/// a hundred or more. Read as notches, that drove `1.15^100` into the brush
/// radius and slammed it between its floor and its ceiling in one gesture, which
/// is indistinguishable from the control not working at all.
///
/// Clamped as well as converted. A device that reports a whole gesture in one
/// frame should not move a control further than a person could mean to.
pub fn wheel_notches(scroll: &bevy::input::mouse::AccumulatedMouseScroll) -> f32 {
    use bevy::input::mouse::MouseScrollUnit;

    let notches = match scroll.unit {
        MouseScrollUnit::Line => scroll.delta.y,
        // The conventional line height a wheel notch stands for. Nothing reports
        // its own, so this is the number every toolkit picks.
        MouseScrollUnit::Pixel => scroll.delta.y / 20.0,
    };
    notches.clamp(-MAX_NOTCHES, MAX_NOTCHES)
}

/// The most one frame of scrolling may count for.
const MAX_NOTCHES: f32 = 3.0;

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
