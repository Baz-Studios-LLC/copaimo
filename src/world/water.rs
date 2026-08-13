//! The sea.
//!
//! One large translucent plane at sea level. It extends well past the world's
//! borders so that when you stand on a western beach and look west, the horizon
//! is open water — which is what sells "the map ends here" without a wall.

use bevy::prelude::*;

use crate::config::SEA_LEVEL;
use crate::world::terrain::TerrainSource;

#[derive(Component)]
pub struct Water;

pub fn spawn_water(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    terrain: Res<TerrainSource>,
) {
    // Four times the world's longest axis: far enough that the plane's own edge
    // is always past the fog, whichever coast you're standing on.
    let size = terrain.half().max_element() * 4.0;

    let material = materials.add(StandardMaterial {
        base_color: Srgba::new(0.05, 0.26, 0.40, 0.80).into(),
        perceptual_roughness: 0.08,
        reflectance: 0.45,
        alpha_mode: AlphaMode::Blend,
        // Drawn from both sides so the surface still reads correctly when the
        // camera dips below it in the shallows.
        cull_mode: None,
        ..default()
    });

    commands.spawn((
        Water,
        Mesh3d(meshes.add(Plane3d::default().mesh().size(size, size))),
        MeshMaterial3d(material),
        Transform::from_xyz(0.0, SEA_LEVEL, 0.0),
    ));
}
