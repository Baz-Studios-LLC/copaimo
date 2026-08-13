//! Sun, sky and lighting.

use bevy::pbr::CascadeShadowConfigBuilder;
use bevy::prelude::*;

/// Sky color, behind everything and at the horizon.
pub const SKY_COLOR: Color = Color::srgb(0.56, 0.69, 0.83);

pub struct SkyPlugin;

impl Plugin for SkyPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(SKY_COLOR))
            .insert_resource(AmbientLight {
                // Tinted toward the sky so shadowed ground reads as cool and
                // outdoor rather than flat gray.
                color: Color::srgb(0.70, 0.80, 1.0),
                brightness: 1_200.0,
                ..default()
            })
            .add_systems(Startup, spawn_sun);
    }
}

fn spawn_sun(mut commands: Commands) {
    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        // Cascades sized to the visible world: tight near the viewer where
        // shadow detail is read, stretching out toward the streaming edge where
        // it isn't. Without fog the far bound matters more — shadows simply
        // stopping mid-landscape is visible in a way it wasn't before.
        CascadeShadowConfigBuilder {
            num_cascades: 4,
            minimum_distance: 0.5,
            maximum_distance: 900.0,
            first_cascade_far_bound: 40.0,
            overlap_proportion: 0.2,
        }
        .build(),
        // Mid-morning: high enough to light the ground, low enough that
        // hillsides and mountain ranges cast shadows that reveal their shape.
        Transform::from_rotation(Quat::from_euler(EulerRot::YXZ, -0.9, -0.85, 0.0)),
    ));
}
