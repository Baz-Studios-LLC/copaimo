//! Sun, sky, clouds — and the clock they all run on.
//!
//! **The world keeps the player's own time.** Not a cycle spinning at some
//! chosen rate: it reads the clock on the machine, so nine in the morning here is
//! nine in the morning there. Playing before school is a different world from
//! playing after dinner, and neither costs the player a wait.
//!
//! That is the whole design, and everything else follows from it: the sun's
//! place, its colour, how bright the sky is, what the clouds are lit with, and —
//! because the light is what casts them — every shadow in the world.
//!
//! # Scrubbing it
//!
//! A world tied to real time is a world whose dusk you cannot look at on demand,
//! which is no way to tune one. `F6` and `F7` push the hour back and forward and
//! detach the clock; `F8` gives it back. Nothing is stored — restart and you are
//! on the player's own time again.

use bevy::pbr::{CascadeShadowConfigBuilder, NotShadowCaster, NotShadowReceiver};
use bevy::prelude::*;

use crate::config::{CLOUDS, CLOUD_CEILING, CLOUD_DRIFT, CLOUD_SCALE, CLOUD_SPREAD, STARS};
use crate::world::StreamAnchor;

/// What time the world thinks it is, and whether that is the player's time.
#[derive(Resource)]
pub struct TimeOfDay {
    /// Hours since midnight, 0 to 24.
    pub hours: f32,
    /// Hours added on top of the clock, for looking at a dusk on purpose.
    pub nudge: f32,
    /// False once somebody has scrubbed it, until they ask for it back.
    pub follows_clock: bool,
}

impl Default for TimeOfDay {
    fn default() -> Self {
        Self {
            // Mid-morning, which is what it will be for a heartbeat before the
            // first read of the real clock replaces it.
            hours: 9.0,
            nudge: 0.0,
            follows_clock: true,
        }
    }
}

impl TimeOfDay {
    /// How high the sun stands, -1 at midnight to 1 at noon.
    ///
    /// A sine through the day rather than anything astronomical. What matters is
    /// that it crosses zero at six and eighteen, because that is where every
    /// other decision here changes.
    pub fn sun_height(&self) -> f32 {
        ((self.hours - 6.0) / 12.0 * std::f32::consts::PI).sin()
    }

    /// Whether the sun is up at all.
    pub fn is_day(&self) -> bool {
        self.sun_height() > 0.0
    }

    /// The hour, as somebody would say it.
    pub fn spoken(&self) -> String {
        let hour = self.hours.floor().clamp(0.0, 23.0) as u32;
        let minute = ((self.hours - hour as f32) * 60.0).floor().clamp(0.0, 59.0) as u32;
        let tail = if self.follows_clock { "" } else { " (held)" };
        format!("{hour:02}:{minute:02}{tail}")
    }
}

/// The moon, and the field of stars behind it.
///
/// Both ride with the viewer: they are meant to be unreachably far off, and the
/// cheapest honest way to say that is to keep them centred on whoever is looking.
/// Walking a kilometre must not walk you under the moon.
#[derive(Component)]
struct Moon;

#[derive(Component)]
struct Stars;

/// Marks a cloud, so the drift can find them.
#[derive(Component)]
struct Cloud {
    /// Metres per second, its own, so a sky does not move as one sheet.
    speed: f32,
}

/// The one material every cloud wears, retinted as the light changes.
#[derive(Resource, Deref)]
struct CloudSkin(Handle<StandardMaterial>);

pub struct SkyPlugin;

impl Plugin for SkyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TimeOfDay>()
            .insert_resource(ClearColor(sky_colour(0.4)))
            .add_systems(Startup, (spawn_sun, spawn_clouds, spawn_night_sky))
            .add_systems(
                Update,
                (read_the_clock, drive_the_sky, drift_clouds, carry_the_night).chain(),
            );
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
        Transform::default(),
    ));
}

/// Reads the machine's clock, unless somebody has taken hold of it.
fn read_the_clock(keys: Res<ButtonInput<KeyCode>>, mut when: ResMut<TimeOfDay>) {
    if keys.just_pressed(KeyCode::F6) {
        when.follows_clock = false;
        when.nudge -= 1.0;
    }
    if keys.just_pressed(KeyCode::F7) {
        when.follows_clock = false;
        when.nudge += 1.0;
    }
    if keys.just_pressed(KeyCode::F8) {
        when.follows_clock = true;
        when.nudge = 0.0;
    }

    if when.follows_clock {
        when.hours = local_hours();
    } else {
        // Held where it was put, plus whatever the scrubbing has added.
        when.hours = (local_hours() + when.nudge).rem_euclid(24.0);
    }
}

/// Hours since local midnight, read from the machine.
///
/// `SystemTime` is UTC and knows nothing of where the player is, so the offset
/// comes from `chrono`, which is the one thing in this file that needs a crate.
fn local_hours() -> f32 {
    use chrono::Timelike;
    let now = chrono::Local::now();
    now.hour() as f32 + now.minute() as f32 / 60.0 + now.second() as f32 / 3600.0
}

/// Puts the sun where the hour says, and colours everything from it.
fn drive_the_sky(
    when: Res<TimeOfDay>,
    skin: Option<Res<CloudSkin>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut clear: ResMut<ClearColor>,
    mut ambient: ResMut<AmbientLight>,
    mut suns: Query<(&mut Transform, &mut DirectionalLight)>,
) {
    let height = when.sun_height();

    // Where the light comes FROM. The sun rises in the east, crosses toward the
    // south and sets in the west; at night the same arithmetic is pointed the
    // other way and it is the moon, so the light still comes from above rather
    // than up through the ground.
    let turn = (when.hours - 6.0) / 12.0 * std::f32::consts::PI;
    let mut from = Vec3::new(turn.cos(), turn.sin(), SOUTHING).normalize();
    if !when.is_day() {
        from = Vec3::new(-from.x, -from.y, -from.z);
    }

    let (colour, strength) = if when.is_day() {
        // Warm and weak at the horizon, white and full overhead. The warmth is
        // what makes a low sun read as morning rather than as a dim noon.
        let up = height.clamp(0.0, 1.0);
        (
            mix_colour(DAWN_LIGHT, NOON_LIGHT, smoothstep_up(up, 0.0, 0.35)),
            // Never quite nothing at the horizon, or sunrise arrives as a switch.
            DAY_LUX * (0.12 + 0.88 * up.powf(0.6)),
        )
    } else {
        (MOON_LIGHT, MOON_LUX)
    };

    for (mut place, mut light) in &mut suns {
        place.rotation = Transform::from_translation(from)
            .looking_at(Vec3::ZERO, Vec3::Y)
            .rotation;
        light.color = colour;
        light.illuminance = strength;
    }

    clear.0 = sky_colour(height);
    // The sky is the ambient light: bright blue-white by day, and at night a
    // little more than nothing, so a world is dark rather than invisible.
    ambient.color = mix_colour(NIGHT_AMBIENT, DAY_AMBIENT, smoothstep_up(height, -0.15, 0.25));
    ambient.brightness = NIGHT_LUX + (DAY_AMBIENT_LUX - NIGHT_LUX) * smoothstep_up(height, -0.2, 0.3);

    // Clouds take the sun's own colour, which is what turns a sky pink at dusk.
    if let Some(skin) = skin {
        if let Some(material) = materials.get_mut(&skin.0) {
            let lit = if when.is_day() {
                mix_colour(DUSK_CLOUD, DAY_CLOUD, smoothstep_up(height, 0.0, 0.3))
            } else {
                NIGHT_CLOUD
            };
            material.base_color = lit;
        }
    }
}

/// Grows the world's clouds and hangs them overhead.
fn spawn_clouds(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let shapes: Vec<Handle<Mesh>> = (0..terrain_core::cloud::VARIETIES as u32)
        .map(|seed| meshes.add(crate::world::stream::as_coloured_mesh(&terrain_core::cloud::grow(seed))))
        .collect();

    let skin = materials.add(StandardMaterial {
        base_color: DAY_CLOUD,
        // Unlit, and deliberately. A cloud's shading is baked into its vertices
        // because a directional light cannot tell the inside of one from its
        // surface — letting the light have it as well would fight that, and at
        // night the clouds would simply go black instead of grey.
        unlit: true,
        ..default()
    });
    commands.insert_resource(CloudSkin(skin.clone()));

    for index in 0..CLOUDS {
        // Spread over a square that the drift wraps them around, at a height
        // that varies a little so the ceiling is not a plane.
        let across = terrain_core::forest::chance(index as i32, 0, 21) - 0.5;
        let along = terrain_core::forest::chance(index as i32, 0, 22) - 0.5;
        let lift = terrain_core::forest::chance(index as i32, 0, 23);
        let turn = terrain_core::forest::chance(index as i32, 0, 24) * std::f32::consts::TAU;
        let size = (0.7 + terrain_core::forest::chance(index as i32, 0, 25) * 1.1) * CLOUD_SCALE;
        let speed = CLOUD_DRIFT * (0.6 + terrain_core::forest::chance(index as i32, 0, 26) * 0.8);

        commands.spawn((
            Cloud { speed },
            Mesh3d(shapes[index % shapes.len()].clone()),
            MeshMaterial3d(skin.clone()),
            Transform::from_xyz(
                across * CLOUD_SPREAD,
                CLOUD_CEILING + lift * CLOUD_CEILING * 0.35,
                along * CLOUD_SPREAD,
            )
            .with_rotation(Quat::from_rotation_y(turn))
            .with_scale(Vec3::splat(size)),
            // Neither casting nor catching. A cloud four hundred metres up would
            // need the shadow cascades stretched past anything useful to cast
            // properly, and catching one from the ground below is meaningless.
            NotShadowCaster,
            NotShadowReceiver,
        ));
    }
}

/// Hangs the moon and scatters the stars.
fn spawn_night_sky(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Unlit, both of them. A star lit by the sun is a contradiction, and the
    // moon has to stay bright when the sun is on the other side of the world.
    let moonstone = materials.add(StandardMaterial {
        base_color: Color::srgb(0.94, 0.94, 0.88),
        unlit: true,
        ..default()
    });
    commands.spawn((
        Moon,
        Mesh3d(meshes.add(Sphere::new(MOON_SIZE).mesh().ico(2).unwrap())),
        MeshMaterial3d(moonstone),
        Transform::default(),
        NotShadowCaster,
        NotShadowReceiver,
    ));

    // One mesh for the whole sky. A star apiece would be a thousand entities to
    // draw a thing nobody looks at directly.
    let mut field = Mesh::new(
        bevy::render::mesh::PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::RENDER_WORLD,
    );
    let mut places: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut colours: Vec<[f32; 4]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    for star in 0..STARS {
        // Scattered over the dome rather than the sphere: half of them under the
        // ground would be half of them wasted.
        let spin = terrain_core::forest::chance(star as i32, 0, 31) * std::f32::consts::TAU;
        let lift = terrain_core::forest::chance(star as i32, 0, 32).powf(0.6)
            * std::f32::consts::FRAC_PI_2
            * 0.98;
        // Plain spherical coordinates: `spin` around, `lift` up from the horizon.
        let at = Vec3::new(spin.cos() * lift.cos(), lift.sin(), spin.sin() * lift.cos())
            * STAR_DOME;

        // Brightness is ALPHA, not size — which is the whole fix.
        //
        // Sizing them by brightness made the bright ones big, and anything big
        // enough to see the shape of is a shape rather than a point of light.
        // These were single TRIANGLES, so the sky had little arrowheads in it.
        // Now they are barely-there squares, all much the same size, and what
        // separates a bright star from a faint one is how strongly it burns.
        let bright = terrain_core::forest::chance(star as i32, 0, 33);
        let strength = 0.25 + bright * bright * 0.75;
        let size = STAR_SIZE * (0.8 + bright * 0.5);

        // A quad facing the middle of the dome, which is where the viewer always
        // is, because the whole field is carried on them.
        let out = at.normalize_or(Vec3::Y);
        let side = out.cross(Vec3::Y).normalize_or(Vec3::X) * size;
        let up_axis = out.cross(side).normalize_or(Vec3::Z) * size;

        let base = places.len() as u32;
        for corner in [
            -side - up_axis,
            side - up_axis,
            side + up_axis,
            -side + up_axis,
        ] {
            places.push((at + corner).to_array());
            normals.push((-out).to_array());
            uvs.push([0.5, 0.5]);
            // A touch of colour, because a sky of identical white points reads as
            // noise. Most stars are near white; a few lean warm or blue.
            let warmth = terrain_core::forest::chance(star as i32, 0, 34);
            colours.push([
                1.0,
                0.94 + warmth * 0.06,
                0.86 + warmth * 0.14,
                strength,
            ]);
        }
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    field.insert_attribute(Mesh::ATTRIBUTE_POSITION, places);
    field.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    field.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    field.insert_attribute(Mesh::ATTRIBUTE_COLOR, colours);
    field.insert_indices(bevy::render::mesh::Indices::U32(indices));

    let starlight = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 1.0, 0.96, 0.0),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        double_sided: true,
        cull_mode: None,
        ..default()
    });
    commands.insert_resource(StarSkin(starlight.clone()));
    commands.spawn((
        Stars,
        Mesh3d(meshes.add(field)),
        MeshMaterial3d(starlight),
        Transform::default(),
        NotShadowCaster,
        NotShadowReceiver,
    ));
}

/// Keeps the moon and the stars over the viewer, and fades them with the sun.
fn carry_the_night(
    when: Res<TimeOfDay>,
    skin: Option<Res<StarSkin>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    anchors: Query<&GlobalTransform, (With<StreamAnchor>, Without<Moon>, Without<Stars>)>,
    mut moons: Query<&mut Transform, (With<Moon>, Without<Stars>)>,
    mut stars: Query<&mut Transform, (With<Stars>, Without<Moon>)>,
) {
    let Some(anchor) = anchors.iter().next() else {
        return;
    };
    let middle = anchor.translation();
    let height = when.sun_height();

    // Full dark to nothing across the twilight, so they come out as the sky
    // goes rather than blinking on at six.
    let night = crate::util::smoothstep(0.06, -0.22, height);

    for mut place in &mut stars {
        place.translation = middle;
    }
    if let Some(skin) = skin {
        if let Some(material) = materials.get_mut(&skin.0) {
            material.base_color = Color::srgba(1.0, 1.0, 0.96, night);
        }
    }

    // Opposite the sun, which is where a full moon is and is also where the
    // moonlight is already coming from.
    let turn = (when.hours - 6.0) / 12.0 * std::f32::consts::PI;
    let toward = Vec3::new(-turn.cos(), -turn.sin(), -SOUTHING).normalize();
    for mut place in &mut moons {
        place.translation = middle + toward * MOON_DISTANCE;
    }
}

/// Drifts the clouds, and keeps them over the viewer's head.
fn drift_clouds(
    time: Res<Time>,
    anchors: Query<&GlobalTransform, (With<StreamAnchor>, Without<Cloud>)>,
    mut clouds: Query<(&Cloud, &mut Transform)>,
) {
    let Some(anchor) = anchors.iter().next() else {
        return;
    };
    let middle = anchor.translation();
    let half = CLOUD_SPREAD * 0.5;

    for (cloud, mut place) in &mut clouds {
        place.translation.x += cloud.speed * time.delta_secs();

        // Wrapped around the viewer rather than around the world. There are
        // eighty of these and the world is eight kilometres across, so scattering
        // them over the whole of it would leave the sky empty; keeping them in a
        // box that follows means the sky is always dressed and the count stays
        // small.
        let offset = place.translation - middle;
        if offset.x > half {
            place.translation.x -= CLOUD_SPREAD;
        } else if offset.x < -half {
            place.translation.x += CLOUD_SPREAD;
        }
        if offset.z > half {
            place.translation.z -= CLOUD_SPREAD;
        } else if offset.z < -half {
            place.translation.z += CLOUD_SPREAD;
        }
    }
}

/// The material the stars wear, faded in and out with the night.
#[derive(Resource, Deref)]
struct StarSkin(Handle<StandardMaterial>);

// ------------------------------------------------------------------- the palette

/// How far off the moon hangs, and how big it is drawn.
///
/// Both arbitrary and only their ratio matters — this is the angle it subtends,
/// which is the only thing anybody can see.
const MOON_DISTANCE: f32 = 1_400.0;
const MOON_SIZE: f32 = 62.0;

/// How far off the stars sit. Inside the moon, so it never hides behind them.
const STAR_DOME: f32 = 1_150.0;
/// How big a star is drawn, at `STAR_DOME` away.
///
/// Tiny, and it has to be. At two and a half units these were five or six pixels
/// across and their shape was plainly visible — the sky had arrowheads in it. A
/// star is a point of light or it is not a star.
const STAR_SIZE: f32 = 0.75;

/// How far south the sun's arc leans, so it is not a perfect overhead sweep.
const SOUTHING: f32 = 0.35;

const DAY_LUX: f32 = 11_000.0;
/// Moonlight. Weak, and not as weak as it was: a night nobody can see the ground
/// in is not atmosphere, it is a black screen with a HUD on it. This is enough to
/// read a hillside by and far short of reading it as day.
const MOON_LUX: f32 = 900.0;
const DAY_AMBIENT_LUX: f32 = 1_200.0;
const NIGHT_LUX: f32 = 340.0;

const DAWN_LIGHT: Color = Color::srgb(1.0, 0.62, 0.34);
const NOON_LIGHT: Color = Color::srgb(1.0, 0.97, 0.92);
const MOON_LIGHT: Color = Color::srgb(0.60, 0.70, 1.0);

const DAY_AMBIENT: Color = Color::srgb(0.70, 0.80, 1.0);
const NIGHT_AMBIENT: Color = Color::srgb(0.30, 0.38, 0.62);

const DAY_CLOUD: Color = Color::srgb(1.0, 1.0, 1.0);
const DUSK_CLOUD: Color = Color::srgb(1.0, 0.72, 0.58);
const NIGHT_CLOUD: Color = Color::srgb(0.30, 0.34, 0.48);

/// Sky colour for a sun height, night through dawn to full day.
pub fn sky_colour(height: f32) -> Color {
    // Lifted off black. A night sky at pure black takes the stars with it — they
    // have nothing to sit against — and the land below goes to a silhouette.
    const NIGHT: Color = Color::srgb(0.055, 0.075, 0.17);
    const DUSK: Color = Color::srgb(0.72, 0.46, 0.36);
    const DAY: Color = Color::srgb(0.56, 0.69, 0.83);

    if height < 0.0 {
        // Below the horizon, dusk gives way to night over the twilight band.
        //
        // Wide, and it has to be. The sun drops fast either side of the horizon —
        // a quarter of its arc in the hour after six — so a band of a quarter was
        // barely an hour of dusk and seven in the evening was already black. At
        // nearly half it is closer to two hours, which is what an evening is.
        mix_colour(NIGHT, DUSK, smoothstep_up(height, -0.45, 0.0))
    } else {
        mix_colour(DUSK, DAY, smoothstep_up(height, 0.0, 0.28))
    }
}

fn mix_colour(low: Color, high: Color, t: f32) -> Color {
    let (low, high) = (LinearRgba::from(low), LinearRgba::from(high));
    LinearRgba::from_vec4(low.to_vec4().lerp(high.to_vec4(), t.clamp(0.0, 1.0))).into()
}

fn smoothstep_up(x: f32, edge0: f32, edge1: f32) -> f32 {
    crate::util::smoothstep(edge0, edge1, x)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(hours: f32) -> TimeOfDay {
        TimeOfDay {
            hours,
            nudge: 0.0,
            follows_clock: true,
        }
    }

    #[test]
    fn the_sun_is_up_between_six_and_six() {
        assert!(!at(3.0).is_day(), "three in the morning is night");
        assert!(at(9.0).is_day());
        assert!(at(12.0).is_day());
        assert!(at(17.0).is_day());
        assert!(!at(21.0).is_day(), "nine at night is night");

        // Highest at noon, and level at the two ends of the day.
        assert!((at(12.0).sun_height() - 1.0).abs() < 1.0e-4);
        assert!(at(6.0).sun_height().abs() < 1.0e-4);
        assert!(at(18.0).sun_height().abs() < 1.0e-4);
        assert!(at(0.0).sun_height() < -0.99, "midnight is the bottom of it");
    }

    #[test]
    fn the_sky_darkens_through_the_evening_without_snapping() {
        // A sky that goes black the moment the sun sets is what everybody
        // notices, so the twilight band is checked for actually being a band.
        let brightness = |hours: f32| {
            let colour = LinearRgba::from(sky_colour(at(hours).sun_height()));
            colour.red + colour.green + colour.blue
        };
        let noon = brightness(12.0);
        let setting = brightness(18.0);
        let dusk = brightness(19.0);
        let night = brightness(23.0);

        assert!(noon > setting, "noon should be brighter than sunset");
        assert!(setting > dusk, "sunset should be brighter than dusk");
        assert!(dusk > night, "dusk should be brighter than midnight");
        // And the step across the horizon must not be the biggest one.
        let across_the_horizon = (brightness(17.9) - brightness(18.1)).abs();
        assert!(
            across_the_horizon < (noon - night) * 0.25,
            "the sky snaps at sunset: {across_the_horizon:.2}"
        );
    }

    #[test]
    // These compare constants, and clippy is right that they do. They are kept
    // as a test rather than dropped because they guard a RELATIONSHIP between
    // the numbers that is easy to break while tuning one of them on its own —
    // the first night was a black screen with a HUD on it.
    #[allow(clippy::assertions_on_constants)]
    fn the_night_is_dark_and_not_blind() {
        // A night nobody can see the ground in is not atmosphere, it is a black
        // screen with a HUD on it — and the first one was exactly that. The moon
        // has to be far weaker than the sun and far stronger than nothing.
        assert!(MOON_LUX < DAY_LUX * 0.15, "moonlight should not read as day");
        assert!(MOON_LUX > DAY_LUX * 0.03, "moonlight should light a hillside");
        assert!(NIGHT_LUX < DAY_AMBIENT_LUX, "night ambient under day ambient");

        // And the sky it stands against is off black, or the stars have nothing
        // to sit on and the land below goes to a silhouette.
        let midnight = LinearRgba::from(sky_colour(-1.0));
        let sum = midnight.red + midnight.green + midnight.blue;
        assert!(sum > 0.02, "the night sky is pure black: {sum:.3}");
        assert!(sum < 0.4, "and it should still read as night: {sum:.3}");
    }

    #[test]
    fn the_hour_reads_as_a_clock() {
        assert_eq!(at(9.5).spoken(), "09:30");
        assert_eq!(at(0.0).spoken(), "00:00");
        assert_eq!(at(23.99).spoken(), "23:59");

        let held = TimeOfDay {
            hours: 14.0,
            nudge: 2.0,
            follows_clock: false,
        };
        assert!(held.spoken().contains("held"), "a scrubbed clock should say so");
    }
}
