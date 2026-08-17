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
use bevy::render::mesh::VertexAttributeValues;

use crate::config::{SEA_LEVEL, TIDE, TIDE_PERIOD};
use crate::shade::{shaded, Shaded};
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

    let material = materials.add(shaded(StandardMaterial {
        base_color: Srgba::new(0.05, 0.26, 0.40, 0.80).into(),
        perceptual_roughness: 0.08,
        reflectance: 0.45,
        alpha_mode: AlphaMode::Blend,
        // Drawn from both sides so the surface still reads correctly when the
        // camera dips below it in the shallows.
        cull_mode: None,
        ..default()
    }));

    commands.spawn((
        Water,
        // A grid rather than one quad, because its vertices are walked every
        // frame to carry the swell.
        Mesh3d(meshes.add(Plane3d::default().mesh().size(size, size).subdivisions(QUADS))),
        MeshMaterial3d(material),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));
}

/// Walks the surface's vertices, so the sea moves and the waterline travels.
pub fn move_water(
    time: Res<Time>,
    mut meshes: ResMut<Assets<Mesh>>,
    water: Query<&Mesh3d, With<Water>>,
) {
    let seconds = time.elapsed_secs();
    for handle in &water {
        let Some(mesh) = meshes.get_mut(&handle.0) else {
            continue;
        };
        let Some(VertexAttributeValues::Float32x3(places)) =
            mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
        else {
            continue;
        };
        for place in places.iter_mut() {
            place[1] = sea_height(Vec2::new(place[0], place[2]), seconds);
        }
        // Normals are left flat on purpose. Recomputing them across a grid this
        // size every frame costs more than the lighting gains, and a broad water
        // surface reads off its color and its silhouette against the shore
        // rather than off its shading.
    }
}
