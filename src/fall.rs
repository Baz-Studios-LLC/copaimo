//! The rain and the snow you can actually see coming down.
//!
//! # A box that follows you, not weather over a world
//!
//! Filling eight kilometres of world with raindrops is not affordable and is not
//! necessary: rain is only ever visible for the few dozen metres around whoever is
//! looking at it. So there is one box of drops, it is centred on the camera, and a
//! drop that leaves it comes back in on the other side. That is the same trick the
//! clouds use to dress a whole sky with thirty of them, for the same reason.
//!
//! # Worked out, never accumulated
//!
//! Every drop's position is arithmetic on the clock — start, speed, and how long it
//! has been going — rather than a step added each frame. Accumulating would tie the
//! rain's speed to the frame rate and let it drift out of the box over an evening,
//! and it is the same rule `sky` follows for the clouds, whose shadows are computed
//! from the identical sum inside the shader.
//!
//! # One pool, two kinds of weather
//!
//! Rain and snow are the same eight hundred entities wearing different numbers:
//! rain is a long thin streak falling hard and nearly straight, snow is a small
//! flake falling slowly and wandering as it goes. Swapping numbers costs nothing;
//! keeping two pools would cost eight hundred entities that are hidden most of the
//! year.
//!
//! How many are SHOWN follows how hard it is falling, so a shower is thin and a
//! downpour is not, and in clear weather every one of them is hidden and this
//! module costs a single early return.

use bevy::pbr::{NotShadowCaster, NotShadowReceiver};
use bevy::prelude::*;

use crate::weather::{Falling, TheWeather};

/// How many drops there are to draw with.
///
/// Eight hundred is a downpour at the size of box below. They are one mesh and one
/// material, so the renderer batches them, and the cost that matters is the
/// transform written per drop per frame rather than the draw.
const DROPS: usize = 800;

/// How far the box reaches around the camera, in metres.
///
/// Twenty-five. Big enough that the far side is past where a drop reads as a drop,
/// small enough that eight hundred of them is dense rather than scattered.
const REACH: f32 = 25.0;

/// How tall the box is. A drop leaving the bottom reappears at the top.
const TALL: f32 = 30.0;

/// How fast rain falls, in metres a second, and how fast snow does.
///
/// Rain is terminal velocity for a small drop, near enough. Snow is more than ten
/// times slower, which is most of what makes snow read as snow before you can see
/// the shape of a flake.
const RAIN_FALLS: f32 = 17.0;
const SNOW_FALLS: f32 = 1.3;

/// How far the wind carries what is falling, as a share of its fall speed.
///
/// Rain comes down nearly straight and slants in a gale; a flake is barely falling
/// at all by comparison and goes wherever the air goes, so the same wind moves it
/// much further sideways.
const RAIN_SLANTS: f32 = 0.30;
const SNOW_SLANTS: f32 = 1.60;

/// How far a flake wanders across the wind, and how fast it swings.
///
/// Snow only. Rain does not wander — a raindrop that wobbles reads as an insect.
const SNOW_WANDERS: f32 = 0.9;
const SNOW_SWINGS: f32 = 0.7;

/// The size of one drop and one flake, in metres.
const RAIN_SIZE: Vec2 = Vec2::new(0.018, 0.75);
const SNOW_SIZE: Vec2 = Vec2::new(0.055, 0.055);

/// One drop, and where it started.
#[derive(Component)]
struct Drop {
    /// Where in the box it sits, each 0 to 1. Fixed for the life of the drop.
    place: Vec3,
    /// How far ahead of the others it is, 0 to 1, so they do not fall in ranks.
    phase: f32,
    /// A little faster or slower than its neighbours.
    hurry: f32,
}

/// The one mesh and the two materials everything falling wears.
#[derive(Resource)]
struct FallSkin {
    rain: Handle<StandardMaterial>,
    snow: Handle<StandardMaterial>,
}

/// Where one drop is now.
///
/// Pure arithmetic, split out so it can be tested without a window: given the box,
/// the clock and the wind, this is the whole of the motion.
fn where_a_drop_is(drop: &Drop, seconds: f32, falling: Falling, wind: Vec2, gust: f32) -> Vec3 {
    let (speed, slant) = match falling {
        Falling::Snow => (SNOW_FALLS, SNOW_SLANTS),
        _ => (RAIN_FALLS, RAIN_SLANTS),
    };
    let speed = speed * drop.hurry;

    // How far it has fallen, wrapped into the height of the box. `rem_euclid` and
    // not `%`, so a negative clock — which a scrubbed one can be — still lands
    // inside the box instead of above it.
    let fallen = (seconds * speed + drop.phase * TALL).rem_euclid(TALL);
    let y = TALL * 0.5 - fallen;

    // How long this drop has been falling, which is what the wind has had to work
    // with. Taken from the same wrap, so the slant resets with the drop rather than
    // marching it off to the horizon.
    let aloft = fallen / speed;
    let mut across = wind * (speed * slant * gust * aloft);

    if matches!(falling, Falling::Snow) {
        // A flake wanders as it comes down, on the WORLD's axes and not the wind's.
        //
        // The first cut swung it across the wind, which is wrong twice: a flake in
        // still air still wanders - that is most of what makes snow look like snow -
        // and hanging the motion on the wind vector meant that when the wind
        // dropped to nothing the axis became a zero vector and every flake fell
        // dead straight, like shot. Two rates, so it traces a slow figure rather
        // than a circle.
        let swing = (seconds + drop.phase * 20.0) * SNOW_SWINGS;
        across += Vec2::new(
            swing.sin() * SNOW_WANDERS,
            (swing * 0.63).cos() * SNOW_WANDERS * 0.7,
        );
    }

    Vec3::new(
        (drop.place.x - 0.5) * REACH * 2.0 + across.x,
        y,
        (drop.place.z - 0.5) * REACH * 2.0 + across.y,
    )
}

/// Aims one drop: along the way it is falling, and broadside to the eye.
///
/// # A streak has to be pitched AND yawed, and only one of them is the wind
///
/// The quad's long axis has to lie along the fall or the drop is a placard rather
/// than a streak — that much is the wind's business and it is the same for every
/// drop in the shower. But a flat quad also has a FACE, and a face turned edge-on
/// is a drop that vanishes: with only the pitch applied, half the shower disappears
/// depending on where you are looking, which is exactly what you cannot have from
/// something whose whole job is to be seen.
///
/// So the quad is spun about the fall axis until its face is as square to the eye
/// as that axis allows. `from_eye` is the drop's own offset from the camera, since
/// the box is centred there — the drops are all around the eye and each one needs
/// its own answer, not the shower's.
fn turned_to(from_eye: Vec3, along: Vec3) -> Quat {
    // The quad is built in XY with its length on Y, so local +Y has to end up
    // pointing back UP the fall.
    let up = -along;
    let to_eye = -from_eye;
    // Whatever is left of the direction to the eye once the fall axis is taken out
    // of it. Nothing is left when a drop is directly above or below the camera, and
    // then any spin will do: it is edge-on from there whatever we choose.
    let mut face = to_eye - up * to_eye.dot(up);
    if face.length_squared() < 1e-6 {
        face = if up.z.abs() < 0.9 { Vec3::Z } else { Vec3::X };
        face -= up * face.dot(up);
    }
    let face = face.normalize();
    let right = up.cross(face).normalize();
    Quat::from_mat3(&Mat3::from_cols(right, up, face))
}

/// Brings the pool into being, hidden.
fn stock_the_sky(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // One unit quad, scaled per kind. A drop is a streak and a flake is a speck,
    // and those are two numbers rather than two meshes.
    let quad = meshes.add(Rectangle::new(1.0, 1.0));

    let skin = FallSkin {
        // Unlit, both. A raindrop is lit by the whole sky rather than by the sun,
        // and letting the directional light have it would turn the near side of a
        // shower bright and the far side black.
        rain: materials.add(StandardMaterial {
            base_color: Color::srgba(0.72, 0.78, 0.88, 0.40),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            double_sided: true,
            cull_mode: None,
            ..default()
        }),
        snow: materials.add(StandardMaterial {
            base_color: Color::srgba(1.0, 1.0, 1.0, 0.88),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            double_sided: true,
            cull_mode: None,
            ..default()
        }),
    };

    for index in 0..DROPS {
        // Scattered with the same hash the world uses for everything else it
        // scatters, so a drop's place is fixed and repeatable rather than rolled.
        let at = index as i32;
        let place = Vec3::new(
            terrain_core::forest::chance(at, 0, 71),
            0.0,
            terrain_core::forest::chance(at, 0, 72),
        );
        commands.spawn((
            Drop {
                place,
                phase: terrain_core::forest::chance(at, 0, 73),
                hurry: 0.85 + terrain_core::forest::chance(at, 0, 74) * 0.30,
            },
            Mesh3d(quad.clone()),
            MeshMaterial3d(skin.rain.clone()),
            Transform::default(),
            Visibility::Hidden,
            NotShadowCaster,
            NotShadowReceiver,
        ));
    }

    commands.insert_resource(skin);
}

/// Moves what is falling, and decides how much of it there is.
fn let_it_fall(
    time: Res<Time>,
    weather: Res<TheWeather>,
    skin: Option<Res<FallSkin>>,
    cameras: Query<&GlobalTransform, (With<Camera3d>, Without<Drop>)>,
    mut drops: Query<(
        &Drop,
        &mut Transform,
        &mut Visibility,
        &mut MeshMaterial3d<StandardMaterial>,
    )>,
    mut resting: Local<bool>,
) {
    let Some(skin) = skin else {
        return;
    };
    let Some(eye) = cameras.iter().next() else {
        return;
    };

    // Clear weather costs one early return and eight hundred entities sitting
    // hidden, which is what the pool is for.
    //
    // # It cost rather more than that
    //
    // It hid all eight hundred EVERY FRAME. Assigning through a `Mut` marks the
    // component changed whether or not the value moved, so a clear day wrote 800
    // change ticks a frame - 48,000 a second - to say nothing had happened, and
    // every system that watches visibility had to look at them.
    //
    // So the pool is put down once and then left alone. `resting` is the whole of
    // it: hidden already, nothing to do. What makes that safe is that the drops are
    // spawned hidden in `stock_the_sky`, so the state this remembers is true from
    // the first frame rather than only after the first clear one - and the compare
    // before the assignment below means an entity that was already hidden is not
    // touched even on the transition.
    //
    // Found by Codex's audit.
    if weather.falling == Falling::Nothing || weather.fall <= 0.0 {
        if *resting {
            return;
        }
        for (_, _, mut seen, _) in &mut drops {
            if *seen != Visibility::Hidden {
                *seen = Visibility::Hidden;
            }
        }
        *resting = true;
        return;
    }
    *resting = false;

    let at = eye.translation();
    let seconds = time.elapsed_secs();
    let snowing = weather.falling == Falling::Snow;
    let want = skin_for(&skin, snowing);
    let size = if snowing { SNOW_SIZE } else { RAIN_SIZE };

    // How many are shown, from how hard it is falling. The rest stay hidden rather
    // than being made transparent: a drop nobody can see should not be drawn.
    let showing = (DROPS as f32 * weather.fall).round() as usize;

    // Which way the streaks lean, and how far. A drop's own quad is turned to face
    // along the way it is travelling, so a slanting shower slants.
    let wind = weather.wind_way;
    let gust = weather.wind;
    let lean = (wind * gust * if snowing { SNOW_SLANTS } else { RAIN_SLANTS }).clamp_length_max(0.9);
    let along = Vec3::new(lean.x, -1.0, lean.y).normalize();

    for (index, (drop, mut place, mut seen, mut wears)) in drops.iter_mut().enumerate() {
        // Above the count, and below it: written only when the answer changes. The
        // boundary moves as the rain hardens, so most frames touch neither end.
        if index >= showing {
            if *seen != Visibility::Hidden {
                *seen = Visibility::Hidden;
            }
            continue;
        }
        if *seen != Visibility::Visible {
            *seen = Visibility::Visible;
        }
        if wears.0 != want {
            wears.0 = want.clone();
        }

        let offset = where_a_drop_is(drop, seconds, weather.falling, wind, gust);
        // Wrapped around the CAMERA rather than around the world, so the box is
        // always where the eye is however far it has walked.
        let wrapped = Vec2::new(offset.x, offset.z)
            - REACH * 2.0 * ((Vec2::new(offset.x, offset.z)) / (REACH * 2.0)).round();

        let stands = at + Vec3::new(wrapped.x, offset.y, wrapped.y);
        place.translation = stands;
        place.rotation = turned_to(stands - at, along);
        place.scale = Vec3::new(size.x, size.y, 1.0);
    }
}

fn skin_for(skin: &FallSkin, snowing: bool) -> Handle<StandardMaterial> {
    if snowing {
        skin.snow.clone()
    } else {
        skin.rain.clone()
    }
}

pub struct FallPlugin;

impl Plugin for FallPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, stock_the_sky)
            .add_systems(Update, let_it_fall);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a_drop(place_x: f32, phase: f32) -> Drop {
        Drop {
            place: Vec3::new(place_x, 0.0, 0.5),
            phase,
            hurry: 1.0,
        }
    }


    /// A clear sky stops touching the drops, and starts again when it rains.
    ///
    /// # Why this runs the real system
    ///
    /// The change it guards is an early return, and an early return is exactly the
    /// kind of thing that is correct in the steady state and wrong on the edges: a
    /// pool left visible when the rain stops, or left hidden when it starts. So this
    /// drives `let_it_fall` itself through both transitions and counts what it
    /// actually writes, rather than testing a copy of its condition.
    ///
    /// Bevy's own change detection is the instrument. `Ref::is_changed` is true for
    /// a component written since the counting system last ran, which is precisely
    /// "did the drop get touched this frame".
    #[test]
    fn a_settled_sky_is_left_alone() {
        #[derive(Resource, Default)]
        struct Touched(usize);

        fn count_touched(mut touched: ResMut<Touched>, drops: Query<Ref<Visibility>, With<Drop>>) {
            touched.0 = drops.iter().filter(|seen| seen.is_changed()).count();
        }

        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Touched>()
            .init_resource::<TheWeather>()
            .insert_resource(FallSkin {
                rain: Handle::default(),
                snow: Handle::default(),
            })
            .add_systems(Update, (let_it_fall, count_touched).chain());
        app.world_mut().spawn((Camera3d::default(), GlobalTransform::default()));
        // Spawned SHOWING, so a pass that does nothing is visibly different from a
        // pass that hides them.
        for step in 0..8 {
            app.world_mut().spawn((
                a_drop(step as f32, step as f32 * 0.1),
                Transform::default(),
                Visibility::Visible,
                MeshMaterial3d::<StandardMaterial>(Handle::default()),
            ));
        }

        let touched = |app: &App| app.world().resource::<Touched>().0;

        // The first pass hides them; spawning marks everything changed anyway, so
        // nothing is claimed about it.
        app.update();
        app.update();
        assert_eq!(touched(&app), 0, "a clear sky went on writing to the drops");

        {
            let mut sky = app.world_mut().resource_mut::<TheWeather>();
            sky.falling = Falling::Rain;
            sky.fall = 1.0;
        }
        app.update();
        assert_eq!(touched(&app), 8, "the rain did not bring the drops back");
        // And having brought them back it leaves their visibility alone. This is the
        // path that runs every frame it rains, so the comparison before the
        // assignment matters more here than on either transition.
        app.update();
        assert_eq!(touched(&app), 0, "falling rain rewrites every drop's visibility");

        {
            let mut sky = app.world_mut().resource_mut::<TheWeather>();
            sky.falling = Falling::Nothing;
            sky.fall = 0.0;
        }
        app.update();
        assert_eq!(touched(&app), 8, "the drops were left hanging when the rain stopped");
        app.update();
        assert_eq!(touched(&app), 0, "the cleared sky went back to writing every frame");

        // AND IT IS NOT LOOKING EITHER.
        //
        // Counting writes cannot tell a system that skipped its loop from one that
        // ran it and found nothing to change - taking the gate out entirely left
        // this test green, which is a hole in the test and not a virtue of the
        // change. So: show one drop from outside while the sky is clear. A system
        // that is still iterating would put it straight back down; a settled one
        // does not look at all.
        //
        // That also writes down the contract the gate depends on. The pool belongs
        // to `let_it_fall`. Nothing else may set a drop visible, and if anything ever
        // needs to, it has to clear the resting state with it.
        let shown = app
            .world_mut()
            .query_filtered::<Entity, With<Drop>>()
            .iter(app.world())
            .next()
            .expect("a drop to show");
        *app.world_mut().get_mut::<Visibility>(shown).expect("its visibility") =
            Visibility::Visible;
        app.update();
        assert_eq!(
            *app.world().get::<Visibility>(shown).expect("its visibility"),
            Visibility::Visible,
            "a settled sky is still walking its whole pool every frame",
        );
    }

    #[test]
    fn a_drop_is_never_seen_edge_on() {
        // The fault this catches is a shower that half disappears depending on
        // which way you look, which is what a pitched-but-unyawed quad does.
        let along = Vec3::new(0.2, -1.0, 0.1).normalize();
        for ring in 0..64 {
            let turn = ring as f32 / 64.0 * std::f32::consts::TAU;
            // All round the camera, and at three heights, since a drop overhead is
            // the awkward case.
            for lift in [-8.0, 0.5, 9.0] {
                let from_eye = Vec3::new(turn.cos() * 6.0, lift, turn.sin() * 6.0);
                let spin = turned_to(from_eye, along);
                let face = spin * Vec3::Z;
                let to_eye = (-from_eye).normalize();
                let squareness = face.dot(to_eye).abs();

                // Its long axis must still lie along the fall, whatever the yaw did.
                let length_axis = spin * Vec3::Y;
                assert!(
                    length_axis.dot(-along) > 0.999,
                    "the streak stopped pointing along the fall: {length_axis:?}"
                );
                // And it must not be edge-on. Straight overhead it cannot help it,
                // so that one is allowed to be poor - but nowhere else.
                let overhead = from_eye.normalize().dot(-along).abs();
                if overhead < 0.9 {
                    assert!(
                        squareness > 0.35,
                        "at {turn:.2} rad, lift {lift}, the drop is {squareness:.2} \
                         square to the eye — that is nearly edge-on"
                    );
                }
            }
        }
    }

    #[test]
    fn every_drop_stays_inside_the_box_it_falls_in() {
        // Over an hour of clock, which is what catches a wrap that only works for
        // the first pass and then walks the rain out of the world.
        for step in 0..3600 {
            let seconds = step as f32;
            for falling in [Falling::Rain, Falling::Snow] {
                let drop = a_drop(0.3, 0.7);
                let at = where_a_drop_is(&drop, seconds, falling, Vec2::X, 1.0);
                assert!(
                    at.y >= -TALL * 0.5 - 1e-3 && at.y <= TALL * 0.5 + 1e-3,
                    "at {seconds}s a {falling:?} drop is {} above the middle of a \
                     box {TALL} tall",
                    at.y
                );
            }
        }
    }

    #[test]
    fn rain_falls_far_faster_than_snow_and_snow_is_carried_much_further() {
        let drop = a_drop(0.5, 0.0);
        // A tenth of a second in: how far each has fallen from the top.
        let rain = where_a_drop_is(&drop, 0.1, Falling::Rain, Vec2::X, 0.0);
        let snow = where_a_drop_is(&drop, 0.1, Falling::Snow, Vec2::X, 0.0);
        let rain_fell = TALL * 0.5 - rain.y;
        let snow_fell = TALL * 0.5 - snow.y;
        assert!(
            rain_fell > snow_fell * 10.0,
            "rain fell {rain_fell:.2} m and snow {snow_fell:.2} m — snow is not slow"
        );

        // And in the same wind the slower thing is taken much further sideways BY
        // THE TIME IT HAS COME DOWN THE SAME DISTANCE, which is the comparison that
        // means anything. Asked at the same instant instead, rain wins easily and
        // says nothing: in one second rain has fallen seventeen metres and snow has
        // fallen one, so of course it has been blown further.
        let after = |falling, metres: f32| {
            let speed = match falling {
                Falling::Snow => SNOW_FALLS,
                _ => RAIN_FALLS,
            };
            where_a_drop_is(&drop, metres / speed, falling, Vec2::X, 1.0).x
        };
        let far_rain = after(Falling::Rain, 10.0);
        let far_snow = after(Falling::Snow, 10.0);
        assert!(
            far_snow.abs() > far_rain.abs() * 3.0,
            "over ten metres of fall the wind carried rain {far_rain:.2} m and snow \
             {far_snow:.2} m — a flake is not being blown about"
        );
    }

    #[test]
    fn rain_does_not_wobble_and_snow_does() {
        // A raindrop that wanders reads as an insect. A flake that does not reads
        // as a pellet.
        let drop = a_drop(0.5, 0.2);
        let straight: Vec<f32> = (0..40)
            .map(|s| where_a_drop_is(&drop, s as f32 * 0.05, Falling::Rain, Vec2::ZERO, 0.0).x)
            .collect();
        let wandering: Vec<f32> = (0..40)
            .map(|s| where_a_drop_is(&drop, s as f32 * 0.05, Falling::Snow, Vec2::ZERO, 0.0).x)
            .collect();

        let spread = |v: &Vec<f32>| {
            let lo = v.iter().cloned().fold(f32::MAX, f32::min);
            let hi = v.iter().cloned().fold(f32::MIN, f32::max);
            hi - lo
        };
        assert!(
            spread(&straight) < 1e-4,
            "rain wandered {:.3} m across with no wind at all",
            spread(&straight)
        );
        assert!(
            spread(&wandering) > 0.2,
            "snow only wandered {:.3} m — it is falling like shot",
            spread(&wandering)
        );
    }

    #[test]
    fn drops_do_not_fall_in_ranks() {
        // Different phases have to put drops at different heights at the same
        // instant, or the rain arrives as a sheet coming down in steps.
        let heights: Vec<f32> = (0..20)
            .map(|i| {
                let drop = a_drop(0.5, i as f32 / 20.0);
                where_a_drop_is(&drop, 3.0, Falling::Rain, Vec2::ZERO, 0.0).y
            })
            .collect();
        let lo = heights.iter().cloned().fold(f32::MAX, f32::min);
        let hi = heights.iter().cloned().fold(f32::MIN, f32::max);
        assert!(
            hi - lo > TALL * 0.7,
            "twenty drops span only {:.1} m of a {TALL} m box",
            hi - lo
        );
    }
}
