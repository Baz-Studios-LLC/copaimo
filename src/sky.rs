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
use crate::shade::{Caster, CloudShadows};
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
    /// Where it stood when the world began, before any drift.
    origin: Vec2,
    /// Metres per second, its own, so a sky does not move as one sheet.
    speed: f32,
    /// How big it is drawn when it is not fading — see `drift_clouds`.
    size: f32,
    /// The ceiling it was hung at, before the weather lowers it.
    height: f32,
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
                (read_the_clock, drive_the_sky, drift_clouds, carry_the_night)
                    .chain()
                    // Not on the workbench. The sky writes `ClearColor` every
                    // frame, so the bench setting its own background at open was
                    // overwritten before the first frame was drawn — which is why
                    // a room with no sky in it had a blue one.
                    .run_if(away_from_the_bench),
            );
    }
}

/// Everywhere but the workbench — which only exists in a tools build.
///
/// A `cfg` predicate rather than `not(in_state(Bench))`, because naming the
/// `Bench` variant in an always-compiled file is exactly how the release build
/// stopped compiling: the variant is tools-only, and no test caught it because
/// tests build with default features. `cargo check --no-default-features` is the
/// check that would have.
fn away_from_the_bench(state: Res<State<crate::states::AppState>>) -> bool {
    #[cfg(feature = "tools")]
    {
        *state.get() != crate::states::AppState::Bench
    }
    #[cfg(not(feature = "tools"))]
    {
        let _ = state;
        true
    }
}

fn spawn_sun(mut commands: Commands) {
    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            // # What the two biases actually are
            //
            // An earlier comment here said clip space, and it was wrong — checked
            // against Bevy 0.16's own shadow shader: the DEPTH bias is metres of
            // world space along the direction to the light, and the NORMAL bias
            // is scaled by the size of a shadow-map texel, so it grows with
            // `SHADOW_DISTANCE` and with cascade coarseness. That scaling is why
            // cutting the distance from nine hundred metres to four hundred
            // changed what the numbers were worth and the world came out streaked
            // with self-shadowing.
            //
            // Raised from the defaults, because acne is worse than the thing too
            // much bias causes. The cost of overdoing it is peter-panning — a
            // shadow creeping out from under the thing casting it — so if trees
            // start looking like they are hovering, this is the number, and it is
            // the depth one first.
            //
            // The NORMAL bias set here only holds for a HIGH light: what a given
            // bias must cover grows as the light drops toward the horizon (with
            // the cotangent of its elevation), so `drive_the_sky` raises it at
            // grazing angles and parks shadows entirely when neither sun nor moon
            // stands high enough — see the note there. These streaks were the
            // "random lines all over the map".
            shadow_depth_bias: 0.055,
            shadow_normal_bias: 2.6,
            ..default()
        },
        // Cascades sized to the visible world: tight near the viewer where
        // shadow detail is read, stretching out toward the streaming edge where
        // it isn't. Without fog the far bound matters more — shadows simply
        // stopping mid-landscape is visible in a way it wasn't before.
        CascadeShadowConfigBuilder {
            num_cascades: SHADOW_CASCADES,
            minimum_distance: 0.5,
            maximum_distance: SHADOW_DISTANCE,
            first_cascade_far_bound: 26.0,
            overlap_proportion: 0.2,
        }
        .build(),
        Transform::default(),
    ));
}

/// Reads the machine's clock, unless somebody has taken hold of it.
/// Public so a photograph can be held at a chosen hour AFTER this has run.
///
/// It rewrites `hours` from the real clock every frame - even when held, where it
/// recomputes from `nudge` - so a system that sets the hour and happens to run first
/// has its answer thrown away. `photo::hold_the_world_still` orders itself behind it.
pub fn read_the_clock(keys: Res<ButtonInput<KeyCode>>, mut when: ResMut<TimeOfDay>) {
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

    // ONE derivation of the hour, WRITTEN only when it moves.
    //
    // `local_hours` reads whole seconds, so its answer is the same for sixty frames
    // out of sixty-one - and assigning through a `ResMut` marks the resource changed
    // whether or not the value moved, so `TimeOfDay` looked new every frame and
    // nothing downstream could ever run on `Changed`.
    //
    // Codex's audit suggested resyncing on a timer and integrating between syncs. I
    // objected that this adds a second source of truth for the hour - the `nudge`
    // mechanism in `photo.rs` exists because writing `hours` directly already went
    // wrong that way once - and they agreed and dropped that half. At fifteen degrees
    // an hour, one second of sun is four thousandths of a degree; there is nothing to
    // interpolate. Compare and write is the whole change.
    let wanted = if when.follows_clock {
        local_hours()
    } else {
        // Held where it was put, plus whatever the scrubbing has added.
        (local_hours() + when.nudge).rem_euclid(24.0)
    };
    if when.hours != wanted {
        when.hours = wanted;
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

/// The grey a sky turns as it closes over, and the grey snow comes out of.
///
/// Rain cloud is darker and bluer than snow cloud, which is pale and flat — a
/// snowy sky is bright and featureless where a rainy one is heavy and leaden. Both
/// are much darker than the white a fair-weather cloud wears, because the underside
/// of a cloud you can see rain falling from is the part the sun does not reach.
const RAIN_CLOUD: Color = Color::srgb(0.30, 0.32, 0.36);
const SNOW_CLOUD: Color = Color::srgb(0.62, 0.63, 0.68);

/// What the sky itself washes toward when it is overcast.
const OVERCAST_SKY: Color = Color::srgb(0.52, 0.55, 0.60);

/// How much of the cloud's white is taken away at full overcast.
const CLOUD_DARKENS_BY: f32 = 0.88;

/// How much bigger and lower the ceiling gets when the sky closes over.
///
/// A rain sky is not a fair sky in grey: the clouds are LOWER and they join up.
/// Growing them is what closes the gaps between them, and dropping them is what
/// makes the ceiling feel like a lid.
pub const CLOUD_SWELLS_BY: f32 = 0.55;
const CLOUD_SINKS_BY: f32 = 0.20;

/// Puts the sun where the hour says, and colours everything from it.
fn drive_the_sky(
    when: Res<TimeOfDay>,
    weather: Res<crate::weather::TheWeather>,
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
        // Faded toward the horizon exactly as the sun is, and for one more
        // reason besides looks: shadows park when the light stands too low —
        // see below — and a moon that kept its full nine hundred lux to the
        // horizon would make that parking visible. Dimmed, the shadows are
        // gone before there is enough light to miss them by.
        let up = (-height).clamp(0.0, 1.0);
        (MOON_LIGHT, MOON_LUX * (0.25 + 0.75 * up.powf(0.6)))
    };

    // How high whichever light is on duty stands: the sun's height by day, and
    // the same figure the other way up for the moon, matching the direction flip
    // above.
    let standing = if when.is_day() { height } else { -height };

    for (mut place, mut light) in &mut suns {
        let facing = Transform::from_translation(from)
            .looking_at(Vec3::ZERO, Vec3::Y)
            .rotation;
        // Only when it has actually moved. The clock runs in real seconds, so
        // left alone this writes a very slightly different rotation EVERY frame —
        // and a directional light that never holds still never lets its shadow
        // cascades settle, so every edge in the world crawls and flickers. Held
        // still between steps, the cascades are stable and the sun still crosses
        // the sky.
        if place.rotation.angle_between(facing) > SUN_STEP {
            place.rotation = facing;
        }
        light.color = colour;
        light.illuminance = strength;

        // # A grazing light cannot be biased into honesty
        //
        // What a shadow bias must cover grows with the cotangent of the light's
        // elevation, so a fixed bias that is generous at noon runs out near the
        // horizon — and the ground self-shadows in shadow-map texel rows: long
        // thin parallel streaks along the light's azimuth, all over the map.
        // Those were the "random lines", and they showed at night because the
        // moon spends hours at angles the sun crosses in minutes.
        //
        // So the normal bias grows as the light drops, and below a floor the
        // shadows park entirely: no bias covers a light nearly level with the
        // ground, and by then the light is too dim for their absence to read.
        // Parking them at night is also most of a night frame's shadow cost
        // given back.
        let low = standing.max(SHADOW_FLOOR);
        light.shadows_enabled = standing > SHADOW_FLOOR;
        light.shadow_normal_bias = 2.6_f32.max(GRAZING_COVER / low);
    }

    // Washed toward flat grey as the sky closes, and the ambient light with it —
    // a bright blue sky behind a black cloud is the thing that makes weather in a
    // game read as a decal hung in front of the weather it actually has.
    clear.0 = mix_colour(
        sky_colour(height),
        OVERCAST_SKY,
        weather.overcast * if when.is_day() { 0.75 } else { 0.35 },
    );
    // The sky is the ambient light: bright blue-white by day, and at night a
    // little more than nothing, so a world is dark rather than invisible.
    ambient.color = mix_colour(NIGHT_AMBIENT, DAY_AMBIENT, smoothstep_up(height, -0.15, 0.25));
    ambient.brightness = (NIGHT_LUX + (DAY_AMBIENT_LUX - NIGHT_LUX) * smoothstep_up(height, -0.2, 0.3))
        * (1.0 - weather.overcast * 0.35);

    // Clouds take the sun's own colour, which is what turns a sky pink at dusk.
    if let Some(skin) = skin {
        let fair = if when.is_day() {
            mix_colour(DUSK_CLOUD, DAY_CLOUD, smoothstep_up(height, 0.0, 0.3))
        } else {
            NIGHT_CLOUD
        };
        // And then the weather has them. A rain cloud IS a rain cloud: the same
        // cloud carrying water is darker, and which grey it turns says which of
        // rain or snow is coming out of it before the first of it arrives.
        let laden = match weather.falling {
            crate::weather::Falling::Snow => SNOW_CLOUD,
            _ => RAIN_CLOUD,
        };
        let lit = mix_colour(fair, laden, weather.overcast * CLOUD_DARKENS_BY);
        // Only when it differs — see the star skin in `carry_the_night` for why
        // an unconditional `get_mut` is a re-upload per frame for nothing.
        if materials.get(&skin.0).is_some_and(|was| was.base_color != lit) {
            if let Some(material) = materials.get_mut(&skin.0) {
                material.base_color = lit;
            }
        }
    }
}

/// Grows the world's clouds and hangs them overhead.
fn spawn_clouds(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Each shape, and how much ground it shades. See `shade::footprint`.
    let shapes: Vec<(Handle<Mesh>, f32)> = (0..terrain_core::cloud::VARIETIES as u32)
        .map(|seed| {
            let shape = terrain_core::cloud::grow(seed);
            let reach = crate::shade::footprint(&shape);
            (
                meshes.add(crate::world::stream::as_coloured_mesh(&shape)),
                reach,
            )
        })
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

    let mut casters = Vec::with_capacity(CLOUDS);

    for index in 0..CLOUDS {
        // Spread over a square that the drift wraps them around, at a height
        // that varies a little so the ceiling is not a plane.
        let across = terrain_core::forest::chance(index as i32, 0, 21) - 0.5;
        let along = terrain_core::forest::chance(index as i32, 0, 22) - 0.5;
        let lift = terrain_core::forest::chance(index as i32, 0, 23);
        let turn = terrain_core::forest::chance(index as i32, 0, 24) * std::f32::consts::TAU;
        let size = (0.7 + terrain_core::forest::chance(index as i32, 0, 25) * 1.1) * CLOUD_SCALE;
        let speed = CLOUD_DRIFT * (0.6 + terrain_core::forest::chance(index as i32, 0, 26) * 0.8);

        let (shape, reach) = &shapes[index % shapes.len()];
        let origin = Vec2::new(across * CLOUD_SPREAD, along * CLOUD_SPREAD);
        let height = CLOUD_CEILING + lift * CLOUD_CEILING * 0.35;

        casters.push(Caster {
            origin,
            height,
            radius: reach * size,
            speed,
        });

        commands.spawn((
            Cloud {
                origin,
                speed,
                size,
                height,
            },
            Mesh3d(shape.clone()),
            MeshMaterial3d(skin.clone()),
            Transform::from_xyz(origin.x, height, origin.y)
                .with_rotation(Quat::from_rotation_y(turn))
                .with_scale(Vec3::splat(size)),
            // Neither casting nor catching, and it is not that they cast no
            // shadow — see `shade`, which lays their shadows on the ground
            // directly. It is that the engine's own shadow pass cannot do it: a
            // caster two hundred metres up needs the cascades stretched past
            // anything useful for the world underneath.
            NotShadowCaster,
            NotShadowReceiver,
        ));
    }

    // What the ground needs to know about the sky. Written once — the drift is
    // arithmetic the shader can do for itself.
    commands.insert_resource(CloudShadows(casters));
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
        // Only touched when the answer changes. `get_mut` marks the material
        // modified whether or not anything differs, and a modified material
        // re-prepares its GPU state — so an unconditional write here re-uploaded
        // the star skin every frame of the whole DAY to keep saying alpha nought.
        let wanted = Color::srgba(1.0, 1.0, 0.96, night);
        if materials.get(&skin.0).is_some_and(|was| was.base_color != wanted) {
            if let Some(material) = materials.get_mut(&skin.0) {
                material.base_color = wanted;
            }
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
    weather: Res<crate::weather::TheWeather>,
    anchors: Query<&GlobalTransform, (With<StreamAnchor>, Without<Cloud>)>,
    mut clouds: Query<(&Cloud, &mut Transform)>,
) {
    let Some(anchor) = anchors.iter().next() else {
        return;
    };
    let here = Vec2::new(anchor.translation().x, anchor.translation().z);
    let elapsed = time.elapsed_secs();

    for (cloud, mut place) in &mut clouds {
        // Worked out from the clock rather than nudged along each frame.
        //
        // The shadow this cloud lays on the ground is drawn from exactly this
        // arithmetic, inside the shader — see `shade`. Two things that have to
        // agree about where a cloud is should not be two different sums, and an
        // accumulating one drifts away from a computed one over an afternoon.
        let drifted = cloud.origin + Vec2::new(cloud.speed * elapsed, 0.0);

        // Wrapped around the viewer rather than around the world. Thirty clouds
        // over eight kilometres of world would leave the sky empty; a box that
        // follows you keeps it dressed wherever you stand, and the count small.
        //
        // Which makes the sky a tile repeated in every direction — and the copy
        // you can see is whichever one you are nearest to.
        let wrapped = drifted - CLOUD_SPREAD * ((drifted - here) / CLOUD_SPREAD).round();
        place.translation.x = wrapped.x;
        place.translation.z = wrapped.y;

        // Bigger and lower as the sky closes over, so the gaps between clouds shut
        // and the ceiling comes down. Scale rather than count: spawning more clouds
        // would mean more shadow discs than the shader's fixed array can hold, and
        // a cloud that grows closes a gap just as well as a new one would fill it.
        let swell = 1.0 + weather.overcast * CLOUD_SWELLS_BY;
        place.scale = Vec3::splat(cloud.size * swell);
        place.translation.y = cloud.height * (1.0 - weather.overcast * CLOUD_SINKS_BY);

        // # A cloud that teleports where you can see it is a cloud that pops
        //
        // The wrap above is what keeps thirty clouds dressing an eight-kilometre
        // world, and it puts the seam half a tile away — a kilometre, which is
        // well inside sight. So a cloud reaching the edge of its tile vanished
        // from one side of the sky and reappeared on the other, in view, and did
        // it again every time the viewer walked far enough.
        //
        // It is faded instead: a cloud shrinks away as it nears the seam and grows
        // back out of nothing on the far side. Which is roughly what a cloud does
        // at that distance anyway, so the honest fix and the cheap one agree.
        //
        // The shadow it lays is NOT faded with it. The shader tiles the sky around
        // each patch of GROUND rather than around the viewer, so its seam is
        // somewhere else entirely — and a shadow a kilometre off, under a cloud
        // that is fading out, is not something anybody can catch.
        let edge = (wrapped - here).abs().max_element() / (CLOUD_SPREAD * 0.5);
        let fade = crate::util::smoothstep(1.0, CLOUD_FADE_FROM, edge);
        place.scale = Vec3::splat(cloud.size * fade);
    }
}

/// The material the stars wear, faded in and out with the night.
#[derive(Resource, Deref)]
struct StarSkin(Handle<StandardMaterial>);

// ------------------------------------------------------------------- the palette

/// How far across its own tile a cloud is before it begins fading out.
///
/// Not quite three quarters of the way to the seam, so the fade has a good stretch
/// of sky to happen over rather than being a dissolve nobody believes.
const CLOUD_FADE_FROM: f32 = 0.72;

/// How far off the moon hangs, and how big it is drawn.
///
/// Both arbitrary and only their ratio matters — this is the angle it subtends,
/// which is the only thing anybody can see. Which is exactly why they moved: at
/// 1,400 m the moon hung outside what the camera can see at all, so the night sky
/// had no moon in it and nothing to compare a missing moon against. The ratio is
/// unchanged, so it is the same moon at the same size in the sky.
const MOON_DISTANCE: f32 = 1_050.0;
const MOON_SIZE: f32 = 46.5;

/// How far off the stars sit. Inside the moon, so it never hides behind them —
/// and inside what the camera can see, which the moon's own note explains.
const STAR_DOME: f32 = 940.0;
/// How big a star is drawn, at `STAR_DOME` away.
///
/// Tiny, and it has to be. At two and a half units these were five or six pixels
/// across and their shape was plainly visible — the sky had arrowheads in it. A
/// star is a point of light or it is not a star.
const STAR_SIZE: f32 = 0.75;

/// How far shadows are cast, and in how many slices.
///
/// **This was the frame rate.** At nine hundred metres over four cascades, every
/// cascade redrew the whole visible world — four passes over fifty-four million
/// vertices apiece, on top of the one pass that actually draws anything. Thirty
/// of a forty-millisecond frame was spent drawing shadow maps, most of it for
/// trees far enough away that their shadows are a few pixels of grey.
///
/// A shadow's job is to sit an object on the ground it is standing on, and that
/// is read within a hundred metres or so. Past that it is texture on a hillside,
/// which the terrain's own shading already gives.
/// **Changing this changes what the NORMAL bias is worth.** That bias is scaled
/// by the size of a shadow-map texel, which grows with this distance — see the
/// light above, where both numbers are set together and why.
const SHADOW_DISTANCE: f32 = 400.0;
const SHADOW_CASCADES: usize = 3;

/// Below this sine of elevation, shadows park; above it, the normal bias grows
/// as `GRAZING_COVER / elevation` until the resting 2.6 covers it.
///
/// The floor is low on purpose: by the time a light stands at four hundredths,
/// the bias has already grown so large that its shadows have all but dissolved
/// into peter-panning, so switching them off there is a step nobody sees — where
/// switching at a healthy elevation would visibly pop every shadow in the world.
const SHADOW_FLOOR: f32 = 0.04;
const GRAZING_COVER: f32 = 1.1;

/// How far the sun must have moved before it is moved, in radians.
///
/// About a fifth of a degree — a few seconds of an hour. Small enough that
/// nobody sees it step, large enough that the light holds still for hundreds of
/// frames at a time, which is what the shadow cascades need to stop crawling.
const SUN_STEP: f32 = 0.0035;

/// How far south the sun's arc leans, so it is not a perfect overhead sweep.
///
/// Public because the cloud shadows have to lean the same way — a shadow thrown
/// along a different line from the light that throws it is worse than none.
pub const SOUTHING: f32 = 0.35;

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

#[cfg(test)]
mod weather_look {
    use super::*;

    /// A helper matching what `drive_the_sky` does, so the test measures the same
    /// arithmetic rather than a copy of it that can drift away from it.
    fn cloud_at(overcast: f32, falling: crate::weather::Falling) -> Srgba {
        let laden = match falling {
            crate::weather::Falling::Snow => SNOW_CLOUD,
            _ => RAIN_CLOUD,
        };
        Srgba::from(mix_colour(DAY_CLOUD, laden, overcast * CLOUD_DARKENS_BY))
    }

    fn brightness(c: Srgba) -> f32 {
        0.2126 * c.red + 0.7152 * c.green + 0.0722 * c.blue
    }

    #[test]
    fn a_rain_cloud_is_a_rain_cloud() {
        use crate::weather::Falling;

        let fair = cloud_at(0.0, Falling::Nothing);
        let gathering = cloud_at(0.5, Falling::Rain);
        let heavy = cloud_at(1.0, Falling::Rain);
        let snowy = cloud_at(1.0, Falling::Snow);

        // It darkens as the sky closes, and it does it all the way down.
        assert!(
            brightness(fair) > brightness(gathering),
            "a gathering sky is no darker than a fair one"
        );
        assert!(
            brightness(gathering) > brightness(heavy),
            "a closed sky is no darker than a half-closed one"
        );
        assert!(
            brightness(heavy) < brightness(fair) * 0.55,
            "a full rain cloud is {:.2} against fair weather's {:.2} — that is a \
             slightly grubby white, not a rain cloud",
            brightness(heavy),
            brightness(fair)
        );

        // And snow comes out of a different sky: pale and flat, not leaden.
        assert!(
            brightness(snowy) > brightness(heavy) * 1.4,
            "a snow sky ({:.2}) is as dark as a rain sky ({:.2})",
            brightness(snowy),
            brightness(heavy)
        );

        // The rain cloud is the bluer, colder grey of the two.
        let rain = Srgba::from(RAIN_CLOUD);
        assert!(
            rain.blue > rain.red,
            "the rain cloud is a warm grey: {rain:?}"
        );
    }
}
