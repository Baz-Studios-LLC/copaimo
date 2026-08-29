//! The material the whole world wears, and the cloud shadows on it.
//!
//! Bevy's standard shading with one thing added: how much sky a point can see.
//! It lives in one place because a cloud shadow that stops at the edge of the
//! grass is worse than no cloud shadow at all — the ground, the tufts on it, the
//! trees standing in it, the water, the walls and the warden are all the same
//! surface as far as a cloud overhead is concerned, so they all wear this.
//!
//! The shading itself is in `assets/shaders/cloud_shade.wgsl`, which is where
//! the reasoning about the shadows themselves is written down. This side gathers
//! the facts and hands them over.

use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderRef};

use crate::config::{CLOUD_SHADE, CLOUD_SHADE_FROM, CLOUD_SHADE_SOFT, CLOUD_SHADE_TO, CLOUD_SPREAD};
use crate::sky::TimeOfDay;

/// What every solid thing in the world is made of.
pub type Shaded = ExtendedMaterial<StandardMaterial, CloudShade>;

/// How many clouds can shade the ground at once.
///
/// The shader carries a fixed array, because a uniform has to have a size. The
/// sky is welcome to hold fewer; the assertion below keeps it from holding more.
pub const MOST_CLOUDS: usize = 32;

/// Refused at compile time rather than caught at run time.
///
/// A sky bigger than the shader's array would quietly stop casting the clouds
/// past the end of it — no crash, no warning, just some clouds with no shadow
/// and a number in a WGSL file that nobody would think to look at. Raising
/// `CLOUDS` past this now fails the build instead, which says so at the moment
/// somebody does it.
const _: () = assert!(
    crate::config::CLOUDS <= MOST_CLOUDS,
    "more clouds than the shader has shadow discs for"
);

/// A standard material, dressed for this world.
///
/// Every material in the game goes through here rather than being built by hand
/// at each spawn, so there is exactly one answer to "does this catch cloud
/// shadow" and it is yes.
pub fn shaded(base: StandardMaterial) -> Shaded {
    Shaded {
        base,
        extension: CloudShade::default(),
    }
}

/// The extension itself: two uniforms the shader reads.
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub struct CloudShade {
    /// Strength, edge softness, how far the sky tiles, how many discs are real.
    #[uniform(100)]
    pub weather: Vec4,
    /// Where each shadow began, how wide it is, and how fast it slides.
    #[uniform(101)]
    pub discs: [Vec4; MOST_CLOUDS],
    /// Whether this material is pushed aside, how far, and by how many.
    #[uniform(102)]
    pub bending: Vec4,
    /// Where each mover stands, and how far out it parts what it stands in.
    #[uniform(103)]
    pub movers: [Vec4; MOST_MOVERS],
}

/// How many things can be pushing through the grass at once.
///
/// The warden, and later whatever monsters are abroad nearby. Fixed because a
/// uniform has to have a size, and eight because past that nobody could tell.
pub const MOST_MOVERS: usize = 8;

/// Something that pushes the grass aside as it goes through.
///
/// On the warden now, and on monsters when there are any — which is the reason it
/// is a component rather than a query for the player. Grass that parts for you
/// and stands still for whatever is stalking you would be worse than grass that
/// never moved.
#[derive(Component)]
pub struct Wades {
    /// How far out it parts what it is standing in, in metres.
    pub reach: f32,
}

impl Default for CloudShade {
    fn default() -> Self {
        Self {
            // No shadow at all until the sky has been read. A material that
            // arrives mid-game — a building raised on arrival at a town — is
            // plain rather than wrong until the next sweep picks it up.
            weather: Vec4::ZERO,
            discs: [Vec4::ZERO; MOST_CLOUDS],
            // Standing still, which is what everything but grass does.
            bending: Vec4::ZERO,
            movers: [Vec4::ZERO; MOST_MOVERS],
        }
    }
}

impl MaterialExtension for CloudShade {
    fn fragment_shader() -> ShaderRef {
        "shaders/cloud_shade.wgsl".into()
    }

    /// Overridden because grass has to BEND, and a vertex is the only place a
    /// thing can be moved. Everything else takes the same path it always did —
    /// see the shader, which is Bevy's own vertex stage with one call inserted.
    fn vertex_shader() -> ShaderRef {
        "shaders/cloud_shade.wgsl".into()
    }
}

/// The radius of the circle that shades as much ground as a cloud does.
///
/// A raft of puffs is an ellipse seen from below, so the circle covering the same
/// area has the geometric mean of its two half-widths. Taking the long axis
/// instead would shade half again as much ground as there is sky above it, which
/// is the one thing about all this that somebody standing in a shadow could
/// catch by looking up.
pub fn footprint(shape: &terrain_core::Geometry) -> f32 {
    let reach = shape.places.iter().fold(Vec2::ZERO, |most, place| {
        most.max(Vec2::new(place[0].abs(), place[2].abs()))
    });
    (reach.x * reach.y).sqrt()
}

/// One cloud, as the ground below cares about it.
#[derive(Clone, Copy, Debug)]
pub struct Caster {
    /// Where it stood when the world began.
    pub origin: Vec2,
    /// How far above the ground it floats.
    pub height: f32,
    /// The radius of the circle that covers as much ground as it does.
    pub radius: f32,
    /// Metres a second, eastward.
    pub speed: f32,
}

/// The sky's cloud list, as the ground sees it. Written once, when they spawn.
#[derive(Resource, Default)]
pub struct CloudShadows(pub Vec<Caster>);

pub struct ShadePlugin;

impl Plugin for ShadePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<Shaded>::default())
            .init_resource::<CloudShadows>()
            .add_systems(Update, (carry_the_shade, part_the_grass));
    }
}

/// Keeps every material's idea of the sky current.
///
/// # Why this hardly ever does anything
///
/// The clouds' own drift is in the shader — a speed times the clock — so nothing
/// here has to be resent as they move. What does change is the sun: as it climbs,
/// the slant that carries a cloud's shadow off to one side shortens, and the
/// shadows slide in under their clouds.
///
/// That is slow. Rewriting it every frame would mean rebuilding every material in
/// the game sixty times a second to describe a change of a few centimetres, so it
/// is written when the sun has actually moved — a few times a minute — and when a
/// material turns up that has never been told.
fn carry_the_shade(
    when: Res<TimeOfDay>,
    weather: Res<crate::weather::TheWeather>,
    shadows: Res<CloudShadows>,
    mut materials: ResMut<Assets<Shaded>>,
    mut told: Local<Option<(f32, usize, f32)>>,
) {
    let height = when.sun_height();
    // The overcast is part of the key, or the shade would keep the value it had
    // when the sun last moved far enough to notice and the sky would darken
    // overhead while the ground stayed bright.
    let closed = (weather.overcast * 40.0).round() / 40.0;
    let now = (height, materials.len(), closed);

    if let Some((was, count, was_closed)) = *told {
        if (was - height).abs() < SUN_STEP
            && count == now.1
            && (was_closed - closed).abs() < f32::EPSILON
            && !shadows.is_changed()
        {
            return;
        }
    }
    *told = Some(now);

    // Full when the sun is high, nothing when it is low, and the fade is the
    // point. A cloud two hundred metres up with the sun near the horizon throws
    // its shadow a kilometre and a half sideways — arithmetically true, and it
    // would put the whole sky's shade on the wrong county. It is also the hour
    // when the light is too flat to read a shadow on the ground by.
    let mut strength =
        CLOUD_SHADE * crate::util::smoothstep(CLOUD_SHADE_FROM, CLOUD_SHADE_TO, height);
    // Deeper under a closed sky, because that is what a closed sky does. Capped
    // short of black: under full overcast the shadows should read as one heavy
    // grey over everything rather than as thirty separate dark discs.
    strength *= 1.0 + weather.overcast * 0.8;

    let mut discs = [Vec4::ZERO; MOST_CLOUDS];
    let mut count = 0;

    if strength > 0.0 {
        // How far the sun's line through a cloud runs sideways before it reaches
        // the ground: straight down at noon, further with every hour either side.
        let turn = (when.hours - 6.0) / 12.0 * std::f32::consts::PI;
        let sun = Vec3::new(turn.cos(), turn.sin(), crate::sky::SOUTHING).normalize();
        let slant = Vec2::new(sun.x, sun.z) / sun.y.max(0.05);
        let swell = 1.0 + weather.overcast * crate::sky::CLOUD_SWELLS_BY;

        for caster in shadows.0.iter().take(MOST_CLOUDS) {
            let at = caster.origin - slant * caster.height;
            // The disc grows with the cloud that throws it — see `sky`, which
            // swells the clouds by the same fraction. Two things that have to agree
            // about how big a cloud is should not be two separate numbers.
            discs[count] = Vec4::new(at.x, at.y, caster.radius * swell, caster.speed);
            count += 1;
        }
    }

    let weather = Vec4::new(strength, CLOUD_SHADE_SOFT, CLOUD_SPREAD, count as f32);

    for (_, material) in materials.iter_mut() {
        material.extension.weather = weather;
        material.extension.discs = discs;
    }
}

/// Tells the grass what is walking through it.
///
/// # Why this one runs every frame when the shadows do not
///
/// A cloud's shadow moves at walking pace across a landscape and can be sent a
/// few times a minute. Something wading through grass has to be sent NOW, or the
/// grass parts where you were rather than where you are.
///
/// It is affordable because it writes to exactly one material — the cover's — and
/// there is one of those for the whole world. Nothing else in the game bends, so
/// nothing else needs telling.
fn part_the_grass(
    time: Res<Time>,
    cover: Option<Res<crate::world::cover::CoverMaterial>>,
    mut materials: ResMut<Assets<Shaded>>,
    waders: Query<(Entity, &GlobalTransform, &Wades)>,
    mut trailing: Local<std::collections::HashMap<Entity, Vec3>>,
) {
    let Some(cover) = cover else {
        return;
    };
    let Some(material) = materials.get_mut(&cover.0) else {
        return;
    };

    // Grass does not part instantly and it does not stand back up instantly.
    //
    // The disturbance follows whoever is making it rather than being pinned to
    // them, which is the whole of the springiness: walk forward and the grass
    // ahead has not given way yet, stop and the parting catches up around you,
    // walk on and it closes behind at its own pace. It is one lerp on the CPU and
    // it does what per-blade state would have done.
    let caught_up = 1.0 - (-time.delta_secs() / GRASS_SETTLE).exp();

    let mut movers = [Vec4::ZERO; MOST_MOVERS];
    let mut count = 0;
    let mut here = std::collections::HashMap::new();

    for (who, place, wades) in &waders {
        if count == MOST_MOVERS {
            break;
        }
        let at = place.translation();
        let trailed = trailing.get(&who).map_or(at, |was| was.lerp(at, caught_up));
        here.insert(who, trailed);
        movers[count] = Vec4::new(trailed.x, trailed.y, trailed.z, wades.reach);
        count += 1;
    }
    // Only what is still about, so a despawned monster's dent does not linger in
    // the map forever.
    *trailing = here;

    material.extension.bending = Vec4::new(1.0, grass_swing(), count as f32, 0.0);
    material.extension.movers = movers;
}

/// How far the grass leans away from something standing in it, in metres.
///
/// Measured at the tip; the foot does not move at all, because a blade bends from
/// its root rather than sliding along the ground.
///
/// # It has to be read against how tall the grass IS
///
/// This was 0.9 m, set when a patch core stood 1.66 m — a bit over half the blade,
/// which is a bend. The same 0.9 against grass that now stands about a metre would
/// be more than the blade is long: not a bend but a blade lying flat and reaching
/// past its own root, which is a shear.
///
/// So it is written as a SHARE of the tallest grass the world grows, and it follows
/// `GRASS_STANDS` on its own. Two thirds leans a blade well over without laying it
/// down, and it is the same fraction that read correctly at the old height.
const GRASS_SWING: f32 = 0.66;

/// The lean in metres, which is what the shader wants.
fn grass_swing() -> f32 {
    terrain_core::cover::tallest() * crate::config::GRASS_STANDS * GRASS_SWING
}

/// How long the grass takes to give way, and to close again, in seconds.
///
/// The time constant of the lag, so most of the movement is done in about twice
/// this. Short enough that the parting keeps up with a walk and long enough that
/// it is visibly a bend rather than a switch.
const GRASS_SETTLE: f32 = 0.16;

/// How far the sun must climb before the shadows are told about it.
///
/// The same bargain the light itself makes, for the same reason: a change nobody
/// can see is not worth the work of sending. At a hundredth of the sun's arc this
/// fires about once a minute, and moves a shadow a few metres under a rim that is
/// tens of metres wide.
const SUN_STEP: f32 = 0.01;

#[cfg(test)]
mod tests {
/// Grass a warden can see over, and a bend that is a bend.
    ///
    /// Both halves of one fault: the grass stood 1.66 m against a 1.70 m warden, so
    /// walking into it he vanished, and the parting that has worked all along could
    /// not be seen happening because it happened over his head. Numbers this test
    /// would have caught before anybody had to look.
    #[test]
    fn the_grass_comes_up_to_a_warden_and_bends_rather_than_shearing() {
        let tallest = terrain_core::cover::tallest() * crate::config::GRASS_STANDS;
        let warden = crate::look::TALL;

        assert!(
            tallest < warden * 0.72,
            "the tallest grass stands {tallest:.2} m against a {warden:.2} m warden —              he wades into it and disappears"
        );
        assert!(
            tallest > warden * 0.30,
            "the tallest grass is {tallest:.2} m, which is a lawn and not long grass"
        );

        // The lean has to stay inside the blade. Past its own length it is not a
        // bend at all: the tip reaches beyond the root and the blade shears over.
        let swing = super::grass_swing();
        assert!(
            swing < tallest,
            "the grass leans {swing:.2} m on a blade {tallest:.2} m long"
        );
        assert!(
            swing > tallest * 0.35,
            "the grass leans only {swing:.2} m on a {tallest:.2} m blade — nobody              will see that happen"
        );
    }

    /// The warden's position reaches the grass, and settles rather than snapping.
    ///
    /// Every piece of this existed and none of it was guarded, which is the same shape as
    /// half a dozen faults found in the gait this week: a chain of five links - `Wades` on
    /// the warden, `part_the_grass` in Update, `CoverMaterial` as the one handle, the tufts
    /// wearing that handle, the shader branching on `bending.x` - where breaking any one of
    /// them leaves grass that simply stands still. Nothing panics and nothing looks wrong
    /// anywhere except in the game.
    ///
    /// This holds the CPU end of it: that a mover's position and reach arrive in the
    /// uniform, that `count` is reported so the shader knows how many to read, and that the
    /// disturbance TRAILS the mover instead of being pinned to it, which is the springiness.
    #[test]
    fn the_grass_is_told_where_the_warden_is() {
        let mut app = App::new();
        app.add_plugins(bevy::asset::AssetPlugin::default())
            .init_asset::<Shaded>()
            .init_asset::<Image>()
            .init_resource::<Time>()
            .add_systems(Update, part_the_grass);

        let handle = app
            .world_mut()
            .resource_mut::<Assets<Shaded>>()
            .add(shaded(StandardMaterial::default()));
        app.insert_resource(crate::world::cover::CoverMaterial(handle.clone()));

        let stood_at = Vec3::new(12.0, 3.0, -7.0);
        app.world_mut().spawn((
            GlobalTransform::from_translation(stood_at),
            Wades { reach: 1.8 },
        ));

        // A long first step, so the lerp lands essentially on the mover and the arrival can
        // be checked without the settle muddying it.
        {
            let mut time = app.world_mut().resource_mut::<Time>();
            time.advance_by(std::time::Duration::from_secs_f32(2.0));
        }
        app.update();

        let material = app.world().resource::<Assets<Shaded>>().get(&handle).unwrap();
        let bending = material.extension.bending;
        assert_eq!(bending.x, 1.0, "the cover has to be marked as a thing that bends");
        assert_eq!(bending.z, 1.0, "one mover was spawned, so one should be reported");
        assert!(bending.y > 0.0, "grass that leans nowhere is grass that does not part");

        let told = material.extension.movers[0];
        assert!(
            told.truncate().distance(stood_at) < 0.05,
            "the grass was told {told:?}, and the warden is at {stood_at:?}"
        );
        assert!(
            (told.w - 1.8).abs() < 1.0e-6,
            "the reach has to survive the trip, or the parting has no size"
        );

        // And the TRAIL: a short step from a standing start must fall short of the mover,
        // because the disturbance follows rather than teleporting. Without this the lerp
        // could be replaced by a plain copy and nothing would notice.
        let mut app = App::new();
        app.add_plugins(bevy::asset::AssetPlugin::default())
            .init_asset::<Shaded>()
            .init_asset::<Image>()
            .init_resource::<Time>()
            .add_systems(Update, part_the_grass);
        let handle = app
            .world_mut()
            .resource_mut::<Assets<Shaded>>()
            .add(shaded(StandardMaterial::default()));
        app.insert_resource(crate::world::cover::CoverMaterial(handle.clone()));
        app.world_mut().spawn((
            GlobalTransform::from_translation(Vec3::ZERO),
            Wades { reach: 1.8 },
        ));
        {
            let mut time = app.world_mut().resource_mut::<Time>();
            time.advance_by(std::time::Duration::from_secs_f32(1.0 / 60.0));
        }
        app.update();
        // First frame from empty: the trail starts AT the mover, so this only proves the
        // second frame lags. Move the mover and step again.
        let moved = Vec3::new(4.0, 0.0, 0.0);
        let who = app
            .world_mut()
            .query_filtered::<Entity, With<Wades>>()
            .single(app.world())
            .unwrap();
        app.world_mut()
            .entity_mut(who)
            .insert(GlobalTransform::from_translation(moved));
        app.update();

        let lagged = app
            .world()
            .resource::<Assets<Shaded>>()
            .get(&handle)
            .unwrap()
            .extension
            .movers[0]
            .truncate();
        assert!(
            lagged.distance(moved) > 0.5,
            "the parting landed at {lagged:?} for a mover that jumped to {moved:?} - it is \
             being pinned to them rather than trailing, so the grass has no give"
        );
    }

    use super::*;
    use crate::config::{CLOUDS, CLOUD_SCALE};

    #[test]
    fn the_shadows_read_as_weather_rather_than_as_overcast() {
        // Measured off the real sky, not a fixture: the shapes the crate
        // actually grows, at the sizes this game actually draws them.
        let shapes: Vec<f32> = (0..terrain_core::cloud::VARIETIES as u32)
            .map(|seed| footprint(&terrain_core::cloud::grow(seed)))
            .collect();

        let mut shaded = 0.0;
        let mut widest: f32 = 0.0;
        for index in 0..CLOUDS {
            // The same draw `spawn_clouds` makes, from the same salt.
            let size = (0.7 + terrain_core::forest::chance(index as i32, 0, 25) * 1.1) * CLOUD_SCALE;
            let radius = shapes[index % shapes.len()] * size;
            widest = widest.max(radius);
            shaded += std::f32::consts::PI * radius * radius;
        }
        let share = shaded / (CLOUD_SPREAD * CLOUD_SPREAD);

        // A sixth of the ground, which is a clear day with weather crossing it.
        // The soft rim takes the felt amount to about half that again, so most
        // of the world is in open sun most of the time and a shadow arriving is
        // something that happens rather than the state of things.
        //
        // The upper bound is the one that matters. Cloud count, cloud scale and
        // the ceiling have all been tuned by eye for the SKY, and each of them
        // moves the ground too — the day the sky reads right and the land is
        // permanently grey is the day somebody should be told here.
        assert!(
            (0.06..0.30).contains(&share),
            "clouds shade {:.0}% of the ground",
            share * 100.0
        );

        // And a shadow has to be big enough to stand in and notice. These are
        // read from the ground by somebody a metre and a half tall: a patch tens
        // of metres across passes as a flicker, where a couple of hundred is the
        // light going off the hillside you are walking over.
        assert!(
            (60.0..400.0).contains(&widest),
            "the widest cloud shadow is {widest:.0} m across"
        );
    }

    #[test]
    fn shadows_come_out_at_midday_and_not_at_night() {
        let shade = |height: f32| {
            CLOUD_SHADE * crate::util::smoothstep(CLOUD_SHADE_FROM, CLOUD_SHADE_TO, height)
        };

        // Nothing under a sun below the horizon, and nothing as it goes down —
        // the slant is what makes a low sun useless here, and it is already
        // hopeless well before the sun actually sets.
        assert_eq!(shade(-0.5), 0.0, "the night should have no cloud shadows");
        assert_eq!(shade(0.0), 0.0, "nor should the horizon");
        assert!(shade(0.25) > 0.0, "mid-morning should have started");
        assert!(
            (shade(1.0) - CLOUD_SHADE).abs() < 1.0e-6,
            "noon should be the full strength"
        );

        // And the full strength has to leave the ground readable. A cloud
        // shadow that reads as night is a bug report about the day/night cycle.
        assert!(
            (0.15..0.6).contains(&CLOUD_SHADE),
            "cloud shade of {CLOUD_SHADE} is not a cloud"
        );
    }
}
