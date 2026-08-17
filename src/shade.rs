//! The material the whole world wears, and the cloud shadows on it.
//!
//! Bevy's standard shading with one thing added: how much sky a point can see.
//! It lives in one place because a cloud shadow that stops at the edge of the
//! grass is worse than no cloud shadow at all — the ground, the tufts on it, the
//! trees standing in it, the water, the walls and the ranger are all the same
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
/// The ranger, and later whatever monsters are abroad nearby. Fixed because a
/// uniform has to have a size, and eight because past that nobody could tell.
pub const MOST_MOVERS: usize = 8;

/// Something that pushes the grass aside as it goes through.
///
/// On the ranger now, and on monsters when there are any — which is the reason it
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
    shadows: Res<CloudShadows>,
    mut materials: ResMut<Assets<Shaded>>,
    mut told: Local<Option<(f32, usize)>>,
) {
    let height = when.sun_height();
    let now = (height, materials.len());

    if let Some((was, count)) = *told {
        if (was - height).abs() < SUN_STEP && count == now.1 && !shadows.is_changed() {
            return;
        }
    }
    *told = Some(now);

    // Full when the sun is high, nothing when it is low, and the fade is the
    // point. A cloud two hundred metres up with the sun near the horizon throws
    // its shadow a kilometre and a half sideways — arithmetically true, and it
    // would put the whole sky's shade on the wrong county. It is also the hour
    // when the light is too flat to read a shadow on the ground by.
    let strength = CLOUD_SHADE * crate::util::smoothstep(CLOUD_SHADE_FROM, CLOUD_SHADE_TO, height);

    let mut discs = [Vec4::ZERO; MOST_CLOUDS];
    let mut count = 0;

    if strength > 0.0 {
        // How far the sun's line through a cloud runs sideways before it reaches
        // the ground: straight down at noon, further with every hour either side.
        let turn = (when.hours - 6.0) / 12.0 * std::f32::consts::PI;
        let sun = Vec3::new(turn.cos(), turn.sin(), crate::sky::SOUTHING).normalize();
        let slant = Vec2::new(sun.x, sun.z) / sun.y.max(0.05);

        for caster in shadows.0.iter().take(MOST_CLOUDS) {
            let at = caster.origin - slant * caster.height;
            discs[count] = Vec4::new(at.x, at.y, caster.radius, caster.speed);
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
    cover: Option<Res<crate::world::cover::CoverMaterial>>,
    mut materials: ResMut<Assets<Shaded>>,
    waders: Query<(&GlobalTransform, &Wades)>,
) {
    let Some(cover) = cover else {
        return;
    };
    let Some(material) = materials.get_mut(&cover.0) else {
        return;
    };

    let mut movers = [Vec4::ZERO; MOST_MOVERS];
    let mut count = 0;
    for (place, wades) in &waders {
        if count == MOST_MOVERS {
            break;
        }
        let at = place.translation();
        movers[count] = Vec4::new(at.x, at.y, at.z, wades.reach);
        count += 1;
    }

    material.extension.bending = Vec4::new(1.0, GRASS_SWING, count as f32, 0.0);
    material.extension.movers = movers;
}

/// How far the grass leans away from something standing in it, in metres.
///
/// Measured at the tip, and the foot does not move at all — a blade bends from
/// its root rather than sliding along the ground. Half a metre is enough to open
/// a path you can see behind you and not so much that the grass lies flat.
const GRASS_SWING: f32 = 0.5;

/// How far the sun must climb before the shadows are told about it.
///
/// The same bargain the light itself makes, for the same reason: a change nobody
/// can see is not worth the work of sending. At a hundredth of the sun's arc this
/// fires about once a minute, and moves a shadow a few metres under a rim that is
/// tens of metres wide.
const SUN_STEP: f32 = 0.01;

#[cfg(test)]
mod tests {
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
