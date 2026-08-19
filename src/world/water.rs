//! The sea, and the way it moves.
//!
//! One large translucent surface at sea level. It extends well past the world's
//! borders so that when you stand on a western beach and look west, the horizon
//! is open water — which is what sells "the map ends here" without a wall.
//!
//! # It approaches and recedes
//!
//! The **tide** is the important half. Swell alone gives a textured sheet; it is
//! the slow rise and fall of the whole surface that makes water advance up a
//! beach and draw back off it. On a coast that shelves over hundreds of metres,
//! half a metre of vertical travel walks the waterline a long way horizontally,
//! so the sea visibly comes and goes without the water level ever doing anything
//! dramatic.
//!
//! `sea_height` is shared with Opificium's terrain bench, number for number — a
//! shoreline that washes differently in the tool than in the game is a shoreline
//! you cannot judge while sculpting it.

use bevy::prelude::*;

use crate::config::{SEA_LEVEL, TIDE, TIDE_PERIOD};
use crate::shade::{CloudShade, Shaded};
use crate::world::terrain::TerrainSource;

#[derive(Component)]
pub struct Water;

/// Quads along the surface's edge.
const QUADS: u32 = 160;

/// Swell: how tall, how far apart, and how fast, in meters and seconds.
///
/// **The wavelengths are long because the mesh can't hold short ones.** The
/// surface spans several times the world, so even at this many quads its
/// vertices sit well over a hundred meters apart, and a thirty-meter wave
/// written onto that grid doesn't come out as a wave — it comes out as noise,
/// sampled at random points along a curve nobody can see. Under about four
/// vertices per wavelength is a lie. What's left is long ocean swell.
const SWELL: [(f32, f32, f32); 2] = [(0.20, 1800.0, 24.0), (0.12, 900.0, 15.0)];

/// How high the sea stands at a point, at a moment.
///
/// # Gameplay's copy of the surface — the DRAWN one is on the GPU
///
/// The shader's `sea_surface_at` in `cloud_shade.wgsl` runs this same sum per
/// vertex, from these same constants handed over in a uniform, against the same
/// clock. Walking the mesh's twenty-six thousand vertices here every frame cost
/// a full re-upload of the mesh per frame to describe motion a vertex shader
/// gets for free. This stays for the questions gameplay asks — where the
/// waterline is, how deep a wade is. **Change one, change both.**
///
/// Nothing calls it TODAY — wading reads the flat `SEA_LEVEL`, which is the
/// right answer for gameplay (a wade limit that surged with the swell would
/// push the player about) — but it is the only statement in this program of
/// what the drawn surface does, and the day something wants the live waterline
/// this is it.
#[allow(dead_code)]
pub fn sea_height(at: Vec2, seconds: f32) -> f32 {
    let tide = (seconds / TIDE_PERIOD * std::f32::consts::TAU).sin() * TIDE;
    let mut swell = 0.0;
    for (i, (height, length, period)) in SWELL.iter().enumerate() {
        // Each layer runs at its own angle so they interfere rather than
        // marching in step — waves in lockstep read as corrugated iron.
        let angle = i as f32 * 2.1;
        let along = at.x * angle.cos() + at.y * angle.sin();
        let phase = along / length - seconds / period;
        swell += (phase * std::f32::consts::TAU).sin() * height;
    }
    SEA_LEVEL + tide + swell
}

pub fn spawn_water(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<Shaded>>,
    terrain: Res<TerrainSource>,
) {
    // Four times the world's longest axis: far enough that the surface's own
    // edge is always out of sight, whichever coast you're standing on.
    let size = terrain.half().max_element() * 4.0;

    // The swell's numbers, handed to the shader in the slots grass uses for its
    // movers — the sea has no movers, and a uniform has the size it has. See
    // `sea_height` for the contract.
    let mut extension = CloudShade::default();
    extension.bending = Vec4::new(2.0, 0.0, 0.0, 0.0);
    extension.movers[0] = Vec4::new(SWELL[0].0, SWELL[0].1, SWELL[0].2, 0.0);
    extension.movers[1] = Vec4::new(SWELL[1].0, SWELL[1].1, SWELL[1].2, 0.0);
    extension.movers[2] = Vec4::new(TIDE, TIDE_PERIOD, SEA_LEVEL, 0.0);

    let material = materials.add(Shaded {
        base: StandardMaterial {
            base_color: Srgba::new(0.05, 0.26, 0.40, 0.80).into(),
            perceptual_roughness: 0.08,
            reflectance: 0.45,
            alpha_mode: AlphaMode::Blend,
            // Drawn from both sides so the surface still reads correctly when the
            // camera dips below it in the shallows.
            cull_mode: None,
            ..default()
        },
        extension,
    });

    commands.spawn((
        Water,
        // A grid rather than one quad: the swell is per-vertex, in the shader.
        // The mesh itself is STATIC — it used to be rewritten on the CPU every
        // frame, which re-uploaded the whole thing sixty times a second.
        Mesh3d(meshes.add(Plane3d::default().mesh().size(size, size).subdivisions(QUADS))),
        MeshMaterial3d(material),
        Transform::from_xyz(0.0, 0.0, 0.0),
        // A translucent sheet still casts in Bevy, so without this the sea was
        // depth-rendered into all three cascades every frame — and the only
        // thing under it to shadow is the sea floor.
        bevy::pbr::NotShadowCaster,
    ));
    // Normals stay flat, as before: a broad water surface reads off its colour
    // and its silhouette against the shore rather than off its shading.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_surface_moves_and_stays_within_its_swell_and_tide() {
        // The CPU statement of the surface the GPU draws — see `sea_height`.
        // What can be pinned from here is that the constants handed to the
        // shader describe a sea that actually moves and never leaves the band
        // its own numbers promise.
        let most = TIDE + SWELL.iter().map(|(height, ..)| height).sum::<f32>();
        for step in 0..200 {
            let moment = step as f32 * 7.3;
            let at = Vec2::new(step as f32 * 31.0, -(step as f32) * 17.0);
            let lift = sea_height(at, moment) - SEA_LEVEL;
            assert!(
                lift.abs() <= most + 1.0e-4,
                "the sea stood {lift:.3} m off its level, past its own {most:.3}"
            );
        }
        assert!(
            (sea_height(Vec2::ZERO, 0.0) - sea_height(Vec2::ZERO, 13.0)).abs() > 1.0e-3,
            "the sea is standing still"
        );
    }
}
