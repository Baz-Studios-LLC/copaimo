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
///
/// # Forward is -Z, because that is what forward means here
///
/// This was `atan2(x, z)`, which aims a model's **+Z** along the way it is
/// travelling — and Bevy's forward, the one `Transform::forward()` returns and
/// the one the camera already uses, is **-Z**. So anything with a front was
/// rotated a half-turn from where it should be.
///
/// Nothing caught it because the only thing this rotated was the blocky
/// placeholder warden, which is symmetric front to back: a box body under a
/// round hat looks identical either way, so a warden walking backwards looked
/// exactly like a warden walking forwards. The first model with a face on it
/// would have walked backwards across the whole world.
///
/// Kept in Bevy's own terms rather than the model's, because the alternative —
/// authoring every model back to front to suit this one function — makes every
/// future monster wrong by convention.
pub fn facing_quat(dir: Vec3) -> Option<Quat> {
    let flat = Vec3::new(dir.x, 0.0, dir.z);
    if flat.length_squared() < 1.0e-6 {
        return None;
    }
    // Solve Ry(t) * -Z = dir: -sin t = x and -cos t = z.
    Some(Quat::from_rotation_y((-flat.x).atan2(-flat.z)))
}

#[cfg(test)]
mod facing {
    use super::*;

    /// A model faces the way it walks, in Bevy's own sense of forward.
    ///
    /// The rotation this hands back is applied to a model whose front is -Z, so
    /// the check is on where -Z ends up: it has to land on the direction of
    /// travel. Written as the property rather than as an angle, because an angle
    /// is exactly what was wrong before and an expected-angle test would have
    /// been written to match it.
    #[test]
    fn the_rotation_points_a_models_front_along_the_way_it_is_going() {
        for (name, dir) in [
            ("north", Vec3::new(0.0, 0.0, -1.0)),
            ("south", Vec3::new(0.0, 0.0, 1.0)),
            ("east", Vec3::new(1.0, 0.0, 0.0)),
            ("west", Vec3::new(-1.0, 0.0, 0.0)),
            ("north-east", Vec3::new(0.7, 0.0, -0.7).normalize()),
            ("down-hill", Vec3::new(0.5, -0.8, 0.5).normalize()),
        ] {
            let turn = facing_quat(dir).unwrap_or_else(|| panic!("no facing for {name}"));
            // Where the model's own front ends up.
            let front = turn * Vec3::NEG_Z;
            let want = Vec3::new(dir.x, 0.0, dir.z).normalize();
            assert!(
                front.distance(want) < 1.0e-5,
                "heading {name}: the model's front points at {front:?}, not {want:?}"
            );
            // And still standing up: no roll, no pitch.
            assert!(
                (turn * Vec3::Y).distance(Vec3::Y) < 1.0e-5,
                "heading {name} tipped the model over"
            );
        }
    }

    /// Standing still holds the old facing instead of snapping to a default.
    #[test]
    fn no_horizontal_direction_is_no_answer_rather_than_a_default() {
        assert!(facing_quat(Vec3::ZERO).is_none());
        assert!(facing_quat(Vec3::Y).is_none(), "straight up is not a heading");
        assert!(facing_quat(Vec3::NEG_Y).is_none());
    }
}
