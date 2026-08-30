//! The lamps that light a settlement after dark.
//!
//! # Two fittings, one job
//!
//! A city stands a steel column at the kerb with an arm out over the carriageway; a
//! village hangs a lantern on a timber post at head height. Built in
//! `dev/art/lamp.py`, stood along the streets by `town::light_the_streets`, and lit
//! here.
//!
//! # Why only a handful are lit at once
//!
//! A city lays a lamp every twenty-six metres, so a large one has well over a
//! hundred of them and the world has more than a thousand. Every one of those as a
//! live point light is not a lighting model, it is a stall: clustered forward
//! shading has a hard ceiling on lights per cluster, and a street is exactly the
//! case where they all land in the same cluster.
//!
//! So the FITTINGS all stand, always, and the LIGHTS follow the warden - the nearest
//! `MOST_LIT` of them, re-chosen as he walks. Nobody can see a lamp go dark behind
//! them at fifty metres, and the ones in front are the ones that were going to be
//! looked at.

use bevy::prelude::*;
use std::collections::HashMap;
use std::sync::LazyLock;
use bevy::scene::SceneRoot;

use crate::world::StreamAnchor;
use crate::world::terrain::TerrainSource;

/// How many lamps are lit at once.
const MOST_LIT: usize = 44;

/// How far a lamp can be from the warden and still be lit, in metres.
const LIT_WITHIN: f32 = 120.0;

/// How bright each fitting burns, in lumens, and how far its light carries.
///
/// Warm, because every other light in this world is daylight or a grey overcast, and
/// the one thing a lamp has to do is feel like a different KIND of light from the
/// sky.
///
/// These were both a tenth of this and reported as "very dim", which they were:
/// Bevy's own default point light is a million lumens, so 120,000 is a candle. A
/// street lamp is a made thing on a pole meant to light a carriageway; a village
/// lantern is somebody's lamp outside their door and is meant to be the smaller of
/// the two, so they no longer share a number.
// # These are art values, not lamp output
//
// I wrote that 120,000 was "a candle". It is not, and Codex was right to correct it:
// Bevy's own table puts a 100 W incandescent at 1,600 lumens, so 120,000 is about
// seventy-five of them. Its 1,000,000 default is documented as a very large CINEMA
// light, chosen to register at Bevy's default very-overcast-day exposure.
//
// So these are not what a street lamp emits. They are what makes a lit pool read at
// THIS game's exposure against a night this bright, tuned from repeatable evidence -
// `--photo --hour 22` - and they should be re-tuned from evidence if the exposure or
// the night ambient ever moves, not reasoned about from wattage.
const STREET_BURNS: f32 = 1_600_000.0;
const POST_BURNS: f32 = 900_000.0;
const STREET_CARRIES: f32 = 55.0;
const POST_CARRIES: f32 = 30.0;
const LAMPLIGHT: Color = Color::srgb(1.0, 0.82, 0.55);

/// How high the sun has to sink before the lamps come on.
///
/// `TimeOfDay::sun` is 1 at noon and negative at night. They come up through dusk
/// rather than snapping on, which is most of what makes an evening read as an
/// evening - and they are fully up before it is properly dark, the way a lamp that
/// somebody lit would be.
const LIT_BELOW: f32 = 0.16;
const FULLY_LIT_AT: f32 = -0.04;

/// A lamp standing in the world.
#[derive(Component)]
pub struct Lamp {
    /// Where its light hangs above its foot.
    head: f32,
    /// How far out along the arm the head is.
    ///
    /// # The light was coming out of the column
    ///
    /// A city fitting reaches 1.5 m out over the carriageway and the lamp is on the
    /// END of that arm. The light was hung at `(0, head, 0)` - straight up from the
    /// foot - so the glow came off the top of the post while the lamp head beside it
    /// stayed dark, which reads as a bug rather than as a lamp. A village lantern
    /// sits on top of its post and this is nought for it.
    arm: f32,
}

/// The light hung on the lamp this is a child of.
#[derive(Component)]
struct Lit;

/// The lamp's own glass, which GLOWS rather than being lit.
///
/// # A bulb cannot be lit by itself
///
/// The fitting's glass came out black at night while the housing around it was lit
/// warm, and the reason is obvious once said: the point light sits inside the glass,
/// and nothing lights a surface from its own middle. A lamp does not reflect light,
/// it makes it - so the glass is emissive, and it is the one thing in the world that
/// is.
///
/// A separate piece rather than part of the model, because a figure is welded into
/// one object with one material and its colour lives in its vertices. This sits
/// exactly over the model's own glass and is slightly bigger, so what you see is the
/// glow.
#[derive(Component)]
struct Glass;

/// How much brighter than its colour the glass burns.
const GLASS_GLOWS: f32 = 6.0;

/// Whether the lamps are burning, from the height of the sun.
///
/// One function because three systems ask, and a fourth thing now depends on the
/// answer: a fitting streamed in at noon has to arrive with its glass already dark.
/// Written out separately in each of them, that is four copies of a threshold and
/// the certainty that one of them ends up on the wrong side of it.
fn burning(clock: &crate::sky::TimeOfDay) -> bool {
    crate::util::smoothstep(LIT_BELOW, FULLY_LIT_AT, clock.sun_height()) > 0.0
}

/// A storey of a city building with somebody still in it.
///
/// # A city with nobody home
///
/// Lamps light the street and leave the towers as black slabs against a black sky,
/// which is the opposite of what a city looks like at night: what says "people live
/// here" from a distance is not the street lighting, it is that some of the windows
/// are on and most are not.
///
/// A band of glass per lit storey, sat just proud of the facade and emissive, so it
/// reads at any distance and costs one box. Which storeys are lit comes from a hash
/// of the building's own position, so a tower's pattern is its own and does not
/// change as you walk toward it.
#[derive(Component)]
struct Awake;

/// On a building whose windows are already lit tonight.
///
/// # Asking the building instead of counting its children
///
/// Whether a tower had been done was worked out by scanning every lit pane in the
/// world looking for one whose parent was this tower - inside a loop over every
/// tower, every frame. A city of thirty-four buildings with a dozen lit storeys
/// each turns that into hundreds of comparisons a frame to re-learn something the
/// building itself could simply say.
///
/// It comes off when the panes come down, which is the only time it stops being
/// true; and if the building is despawned it goes with it.
///
/// Found by Codex's audit.
#[derive(Component)]
struct LitTonight;

/// How far away a city's windows are still worth lighting, in metres.
///
/// Much further than the lamps: a lit tower is what you SEE a city by, and the
/// whole point is that it reads from outside.
const AWAKE_WITHIN: f32 = 900.0;

/// What share of a city's storeys have somebody in at night.
const AWAKE_SHARE: f32 = 0.34;

/// A city storey, floor to floor - `FLOOR_TALL` in `dev/art/town.py`.
///
/// The old world's storey used to be here beside it and is gone: nothing in this
/// file works out where an old-world window is any more, so nothing needs to know
/// how tall its floors are. See `WINDOWS`.
const FLOOR_TALL: f32 = 3.4;

/// How far up a tower's glazing starts - it spends its ground floor on a lobby.
const LOBBY: f32 = FLOOR_TALL * 1.5;

/// How far proud of the glass a lit pane sits. Enough to win the depth test and
/// little enough that it is in the window rather than in front of it.
const PROUD: f32 = 0.02;

/// How wide and tall a cottage's window is, in metres, and how far up its storey.
/// What `dev/art/town.py` measured off the buildings it built.
///
/// Compiled in rather than loaded: it is a few kilobytes, every consumer wants it
/// before the first frame, and `include_str!` makes cargo rebuild when it changes -
/// so the table and the models cannot get out of step in a build that succeeded.
const TOWN_CONTRACT: &str = include_str!("../../assets/models/town.txt");

/// One window in a building, where the model actually has one.
pub(crate) struct Pane {
    /// Which floor it lights, so a lit room lights all of its own windows.
    pub storey: usize,
    /// Where it sits, in the building's own frame, already stood proud of the wall.
    pub at: Vec3,
    /// How big the glass is.
    pub size: Vec3,
}

/// Every figure's windows, keyed by the name it is built under.
///
/// # The game was working these out for itself
///
/// It placed lit panes from the building's LOT FOOTPRINT - two across the front at
/// 24 % of the width, one halfway down each flank, on a 0.9 x 1.15 pane at 1.7 m -
/// and not one of those numbers was true of any building in the world. The lot is
/// not the building: it is what the building keeps clear on the ground, and it is
/// bigger. `storeys` claimed a cottage had two, and a cottage has one, so half its
/// windows were lit at 5.3 m on a wall that stops at 3.6.
///
/// From a distance that reads as a village with its lights on. Close up it is
/// rectangles glowing on blank plaster and two more hanging in the air beside a
/// chimney, which is what a photograph of a village after dark showed the moment
/// anybody took one.
///
/// So the windows are measured off the glass in Blender and read here. The game no
/// longer has an opinion about where a window is; it only decides which are lit.
///
/// Found by Codex, reviewing for facts this codebase states twice.
static WINDOWS: LazyLock<HashMap<&'static str, Vec<Pane>>> = LazyLock::new(|| {
    let mut found: HashMap<&'static str, Vec<Pane>> = HashMap::new();
    for line in TOWN_CONTRACT.lines() {
        let Some(rest) = line.strip_prefix("WINDOW ") else {
            continue;
        };
        let mut word = rest.split_whitespace();
        let Some(figure) = word.next() else {
            continue;
        };
        let said: Vec<f32> = word.filter_map(|number| number.parse().ok()).collect();
        let [storey, x, y, z, wide, tall, deep] = said[..] else {
            continue;
        };
        found.entry(figure).or_default().push(Pane {
            storey: storey as usize,
            at: Vec3::new(x, y, z),
            size: Vec3::new(wide, tall, deep),
        });
    }
    found
});

/// The windows of a figure, and how many floors they are spread over.
///
/// Written down in one place because both are the same question asked twice, and
/// the answer used to be a constant that disagreed with the models.
pub(crate) fn windows_of(what: crate::world::town::Building) -> Option<(&'static [Pane], usize)> {
    let panes = WINDOWS.get(what.figure())?;
    // Sorted by storey where they were measured, so the last one has the highest.
    let storeys = panes.last().map_or(0, |pane| pane.storey + 1);
    (storeys > 0).then_some((panes.as_slice(), storeys))
}

/// What the two ages burn.
///
/// A village lantern is amber - something with a flame in it, or a filament pretending
/// to be one. A city's is a cooler warm-white: a made light with a specification,
/// still warm enough not to read as clinical. The fittings already tell the two apart
/// by day; this keeps them apart after dark.
const STREETLIGHT: Color = Color::srgb(1.0, 0.93, 0.80);

/// How wide the city fitting's cone opens.
// Sixty-six degrees. A cone puts its lumens where the fitting points instead of
// spraying them, so the pools came out tighter than the omni light's - which is
// correct, that spill was the leakage - but a street wants its pools to nearly meet.
const STREET_SPREAD: f32 = 1.15;

/// How near a lamp has to be to be ADMITTED to the lit set, as against kept in it.
/// The gap between this and `LIT_WITHIN` is the hysteresis.
const ADMIT_WITHIN: f32 = 92.0;

/// What a lit window is coloured. Warmer and paler than a street lamp: it is a room
/// with a lamp in it seen through glass, not the lamp itself.
const INDOORS: Color = Color::srgb(1.0, 0.90, 0.68);
const WINDOW_GLOWS: f32 = 2.6;

/// Stands every lamp near the warden.
///
/// Taking them down again is `raise_the_towns`' job: a lamp carries the same
/// `FromSite` its settlement's buildings do and goes when they go.
///
/// The FITTING and its glass, always. Only the handful nearest actually cast light -
/// see `light_them_at_night` - but every one of them shows a lit head, which is what
/// keeps a street's rhythm going past the radius where real lights stop and makes
/// the change in that set very hard to notice.
pub fn stand_the_lamps(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut paints: ResMut<Assets<StandardMaterial>>,
    mut bulbs: Local<Option<(Handle<Mesh>, Handle<StandardMaterial>, Handle<StandardMaterial>)>>,
    terrain: Res<TerrainSource>,
    clock: Res<crate::sky::TimeOfDay>,
    built: Res<crate::world::town::Built>,
    mut raised: Local<std::collections::HashSet<u32>>,
) {
    let (bulb, street_glass, post_glass) = bulbs
        .get_or_insert_with(|| {
            let glow = |tint: Color| StandardMaterial {
                base_color: tint,
                emissive: LinearRgba::from(tint) * GLASS_GLOWS,
                unlit: true,
                ..default()
            };
            (
                meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
                paints.add(glow(STREETLIGHT)),
                paints.add(glow(LAMPLIGHT)),
            )
        })
        .clone();

    // WHICH TOWNS HAVE THEIR LAMPS UP, remembered rather than rediscovered.
    //
    // This used to ask by scanning every standing lamp for one belonging to this
    // settlement - a query over hundreds of fittings, per settlement, every frame,
    // to re-learn something that cannot change once it is true.
    //
    // A key leaves this set exactly when it leaves `Built::standing`, and that is
    // the same moment `raise_the_towns` despawns everything carrying its `FromSite`
    // - the lamps with it. So the two can only agree.
    //
    // Found by Codex's audit.
    raised.retain(|key| built.standing.contains_key(key));
    let lit = burning(&clock);

    for (key, layout) in built.standing.iter() {
        if !raised.insert(*key) {
            continue;
        }
        for lamp in &layout.lamps {
            let street = lamp.head > crate::world::town::POST_HEAD + 0.5;
            let arm = if street { crate::world::town::STREET_ARM } else { 0.0 };
            let ground =
                crate::world::town::stands_at(&terrain.0, lamp.at, Vec2::splat(0.6), lamp.turn);
            commands
                .spawn((
                    Lamp { head: lamp.head, arm },
                    crate::world::town::FromSite(*key),
                    SceneRoot(assets.load(GltfAssetLabel::Scene(0).from_asset(if street {
                        "models/lamp_street.glb"
                    } else {
                        "models/lamp_post.glb"
                    }))),
                    Transform::from_xyz(lamp.at.x, ground, lamp.at.y)
                        .with_rotation(Quat::from_rotation_y(lamp.turn)),
                    Visibility::default(),
                ))
                .with_children(|on| {
                    // The glass, on every fitting whether or not it lights anything.
                    let (size, drop) = if street {
                        (Vec3::new(0.78, 0.24, 0.38), -0.10)
                    } else {
                        (Vec3::new(0.40, 0.50, 0.40), 0.0)
                    };
                    on.spawn((
                        Glass,
                        Mesh3d(bulb.clone()),
                        MeshMaterial3d(if street {
                            street_glass.clone()
                        } else {
                            post_glass.clone()
                        }),
                        Transform::from_xyz(arm, lamp.head + drop, 0.0).with_scale(size),
                        // LIT OR DARK AS IT ARRIVES.
                        //
                        // It used to arrive visible and wait for `open_the_glass` to
                        // notice. That was harmless while that system looked at every
                        // pane every frame; now that it only looks when the sun
                        // crosses the threshold, a lamp streamed in at noon would
                        // have burned until dusk.
                        if lit { Visibility::Inherited } else { Visibility::Hidden },
                        bevy::pbr::NotShadowCaster,
                    ));
                });
        }
    }
}

/// Shows and hides the glass with the sun, at the moment the sun crosses.
///
/// Separate from the lights because there are hundreds of these and twenty of those.
/// Hiding costs nothing and keeps the fitting's own geometry standing by day.
///
/// # Twice a day, not sixty times a second
///
/// The answer changes at dusk and at dawn and at no other time, so the scan runs
/// then. What makes that safe is that a fitting is now spawned with its glass
/// already right - see `stand_the_lamps` - because a system that only looks on the
/// threshold cannot notice anything that arrives between two of them.
pub fn open_the_glass(
    clock: Res<crate::sky::TimeOfDay>,
    mut glass: Query<&mut Visibility, With<Glass>>,
    mut showing: Local<Option<bool>>,
) {
    let lit = burning(&clock);
    if *showing == Some(lit) {
        return;
    }
    *showing = Some(lit);
    let want = if lit { Visibility::Inherited } else { Visibility::Hidden };
    for mut show in &mut glass {
        if *show != want {
            *show = want;
        }
    }
}

/// Lights the nearest lamps, and puts out the rest.
///
/// # Hiding the moment a lamp joins the set
///
/// Only `MOST_LIT` fittings cast real light, and that set follows the warden - so
/// when rank 21 becomes rank 20 one light appears and another goes. Done bluntly
/// that is a lamp switching on ahead of you for no reason, which is worse than a
/// dark lamp.
///
/// Two things hide it. The intensity FADES to nothing over the outer part of the
/// radius, so anything joining or leaving is already almost out; and the set has
/// HYSTERESIS - a lamp is admitted only well inside, and kept until it crosses the
/// outer edge - so nothing sits on the boundary flickering in and out.
pub fn light_them_at_night(
    mut commands: Commands,
    clock: Res<crate::sky::TimeOfDay>,
    anchors: Query<&GlobalTransform, With<StreamAnchor>>,
    lamps: Query<(Entity, &GlobalTransform, &Lamp)>,
    mut points: Query<(Entity, &ChildOf, &mut PointLight), With<Lit>>,
    mut spots: Query<(Entity, &ChildOf, &mut SpotLight), With<Lit>>,
) {
    let Some(anchor) = anchors.iter().next() else {
        return;
    };
    let here = anchor.translation();
    let up = crate::util::smoothstep(LIT_BELOW, FULLY_LIT_AT, clock.sun_height());

    let already: Vec<Entity> = points
        .iter()
        .map(|(_, of, _)| of.parent())
        .chain(spots.iter().map(|(_, of, _)| of.parent()))
        .collect();

    // Everything in reach, nearest first, with what is already lit given the first
    // refusal - that is the hysteresis.
    let mut near: Vec<(f32, Entity, f32, f32)> = lamps
        .iter()
        .map(|(entity, at, lamp)| {
            (at.translation().distance(here), entity, lamp.head, lamp.arm)
        })
        .filter(|(away, ..)| *away < LIT_WITHIN)
        .collect();
    near.sort_by(|a, b| {
        let keep = |e: Entity| u8::from(!already.contains(&e));
        (keep(a.1), a.0)
            .partial_cmp(&(keep(b.1), b.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    near.retain(|(away, lamp, ..)| already.contains(lamp) || *away < ADMIT_WITHIN);
    near.truncate(MOST_LIT);

    // How bright one is at its distance: full up close, nothing at the edge.
    let carries = |away: f32| crate::util::smoothstep(LIT_WITHIN, LIT_WITHIN * 0.7, away);

    for (entity, of, mut light) in &mut points {
        match near.iter().find(|(_, lamp, ..)| *lamp == of.parent()) {
            Some((away, ..)) => light.intensity = POST_BURNS * up * carries(*away),
            None => commands.entity(entity).despawn(),
        }
    }
    for (entity, of, mut light) in &mut spots {
        match near.iter().find(|(_, lamp, ..)| *lamp == of.parent()) {
            Some((away, ..)) => light.intensity = STREET_BURNS * up * carries(*away),
            None => commands.entity(entity).despawn(),
        }
    }

    if up <= 0.0 {
        return;
    }
    for (away, lamp, head, arm) in near {
        if already.contains(&lamp) {
            continue;
        }
        let street = arm > 0.0;
        commands.entity(lamp).with_children(|on| {
            if street {
                // A SPOT, because the fitting is one.
                //
                // A head on an arm aimed over the carriageway is not an
                // omnidirectional source, and modelling it as one spends most of its
                // light upward, backward, and - with shadows off - straight through
                // the building behind it. A cone pointed down gives the
                // carriageway-shaped pool the fitting is shaped to make, and stops
                // the leak without paying for shadows.
                on.spawn((
                    Lit,
                    SpotLight {
                        color: STREETLIGHT,
                        intensity: STREET_BURNS * up * carries(away),
                        range: STREET_CARRIES,
                        outer_angle: STREET_SPREAD,
                        inner_angle: STREET_SPREAD * 0.45,
                        shadows_enabled: false,
                        ..default()
                    },
                    Transform::from_xyz(arm, head, 0.0)
                        .looking_to(Vec3::NEG_Y, Vec3::X),
                ));
            } else {
                // A lantern on a post really is omnidirectional, and reads as one.
                on.spawn((
                    Lit,
                    PointLight {
                        color: LAMPLIGHT,
                        intensity: POST_BURNS * up * carries(away),
                        range: POST_CARRIES,
                        shadows_enabled: false,
                        ..default()
                    },
                    Transform::from_xyz(arm, head, 0.0),
                ));
            }
        });
    }
}

/// Turns the lights on in the towers.
pub fn light_the_windows(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut paints: ResMut<Assets<StandardMaterial>>,
    mut pane: Local<Option<(Handle<Mesh>, Handle<StandardMaterial>)>>,
    clock: Res<crate::sky::TimeOfDay>,
    year: Res<crate::season::TheYear>,
    mut tonight: Local<i64>,
    anchors: Query<&GlobalTransform, With<StreamAnchor>>,
    towers: Query<
        (Entity, &GlobalTransform, &crate::world::town::Standing),
        Without<LitTonight>,
    >,
    awake: Query<(Entity, &ChildOf), With<Awake>>,
) {
    // Panes down, and the buildings they belonged to told so.
    let put_out = |commands: &mut Commands| {
        for (pane, of) in &awake {
            commands.entity(pane).despawn();
            commands.entity(of.parent()).remove::<LitTonight>();
        }
    };
    let Some(anchor) = anchors.iter().next() else {
        return;
    };
    let here = anchor.translation();
    let up = crate::util::smoothstep(LIT_BELOW, FULLY_LIT_AT, clock.sun_height());

    // A DIFFERENT NIGHT IS A DIFFERENT PATTERN.
    //
    // The panes come down when the date turns, so the next night lights a different
    // set. Without this the pattern was a property of the building alone and a tower
    // wore the same windows for the life of the world.
    let night = crate::season::what_night_it_is(&year);
    if *tonight != night {
        *tonight = night;
        put_out(&mut commands);
        return;
    }

    // By day every window is off, and the panes come down rather than being left
    // black - a dark quad over the glass is worse than no quad at all.
    if up <= 0.0 {
        put_out(&mut commands);
        return;
    }

    let (mesh, paint) = pane
        .get_or_insert_with(|| {
            (
                meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
                paints.add(StandardMaterial {
                    base_color: INDOORS,
                    emissive: LinearRgba::from(INDOORS) * WINDOW_GLOWS,
                    unlit: true,
                    ..default()
                }),
            )
        })
        .clone();

    for (entity, at, standing) in &towers {
        // What this building's windows are. A curtain wall's are a band a storey and
        // the game lays them out; everything else has real windows in real walls, and
        // where those are is the model's business rather than this file's.
        let measured = windows_of(standing.what);
        let Some(storeys) = standing.what.facade().map(|(_, _, floors)| floors).or(
            measured.map(|(_, storeys)| storeys),
        ) else {
            continue;
        };
        if at.translation().distance(here) > AWAKE_WITHIN {
            continue;
        }
        commands.entity(entity).insert(LitTonight);

        let foot = at.translation();
        // Which storeys are in, from where the building STANDS - so a tower keeps
        // its own pattern however often it is streamed in and out.
        let seed = foot.x.to_bits() ^ foot.z.to_bits() ^ (night as u32).wrapping_mul(2_654_435_761);
        commands.entity(entity).with_children(|on| {
            for storey in 0..storeys {
                let roll = crate::world::town::unit(seed, storey as u32 * 31 + 7);
                if roll > AWAKE_SHARE {
                    continue;
                }
                // Where the light shows, which depends on what the wall is made of.
                //
                // A tower's facade is a curtain wall and a lit floor is a lit BAND
                // the width of it. A cottage has windows in a wall: lighting the
                // whole face of one would read as a building on fire, so it gets
                // panes the size of the windows that are actually there.
                let panes: Vec<(Vec3, Vec3)> = if let Some((wide, deep, _)) =
                    standing.what.facade()
                {
                    // ON THE FACADE, at the height the glazing actually is.
                    let z = LOBBY + storey as f32 * FLOOR_TALL + FLOOR_TALL * 0.67;
                    let band = FLOOR_TALL * 0.66;
                    // ONE LIGHT AT A TIME, not the whole floor.
                    //
                    // Lighting the band lit a rectangle the width of the building,
                    // which reads as a floor with its lights on rather than as a
                    // room with somebody in it. The facade is divided into squares
                    // by its own mullions - see `curtain_wall` - so the lit pane is
                    // one of those squares.
                    // Blender builds a figure Z-up and the export turns it Y-up, so
                    // the wall whose span runs along Blender X keeps its span on
                    // local X and its FACE moves to local Z. Getting that the wrong
                    // way round hangs a wall's worth of windows off the end of the
                    // building.
                    let mut panes = Vec::new();
                    for (span, face) in [(wide, true), (deep, false)] {
                        let lights = ((span * 0.94 / band).round() as usize).max(2);
                        for light in 0..lights {
                            // Which of them is in, hashed per pane.
                            let roll = crate::world::town::unit(
                                seed,
                                (storey * 97 + light * 13 + usize::from(face) * 7) as u32,
                            );
                            if roll > 0.45 {
                                continue;
                            }
                            let over = -span * 0.47
                                + span * 0.94 * (light as f32 + 0.5) / lights as f32;
                            let across = span * 0.94 / lights as f32 - 0.18;
                            if face {
                                panes.push((
                                    Vec3::new(across, band - 0.18, 0.04),
                                    Vec3::new(over, z, deep * 0.5 + PROUD),
                                ));
                            } else {
                                panes.push((
                                    Vec3::new(0.04, band - 0.18, across),
                                    Vec3::new(wide * 0.5 + PROUD, z, over),
                                ));
                            }
                        }
                    }
                    panes
                } else {
                    // THE WINDOWS THE MODEL ACTUALLY HAS, on the storey that is in.
                    let Some((panes, _)) = measured else {
                        continue;
                    };
                    panes
                        .iter()
                        .filter(|pane| pane.storey == storey)
                        .map(|pane| (pane.size, pane.at))
                        .collect()
                };
                for (size, at) in panes {
                    on.spawn((
                        Awake,
                        Mesh3d(mesh.clone()),
                        MeshMaterial3d(paint.clone()),
                        Transform::from_translation(at).with_scale(size),
                        Visibility::default(),
                        bevy::pbr::NotShadowCaster,
                    ));
                }
            }
        });
    }
}

pub struct LampPlugin;

impl Plugin for LampPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                stand_the_lamps,
                open_the_glass,
                light_them_at_night,
                light_the_windows,
            )
                .chain()
                .run_if(crate::build::a_world_is_up),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The game hangs each light where Blender put the fitting's head.
    ///
    /// Two fittings at two heights, so one guess cannot serve both - and a point
    /// light at the wrong height is a glow floating beside a lamp, which reads as a
    /// bug rather than as a lamp.
    #[test]
    fn the_lamp_models_hang_their_light_where_the_game_thinks() {
        let note =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/models/lamp.txt");
        let said = std::fs::read_to_string(&note)
            .unwrap_or_else(|_| panic!("run dev/art/build.sh: {} is missing", note.display()));
        let read = |key: &str| -> f32 {
            said.lines()
                .find_map(|line| line.strip_prefix(&format!("{key} ")))
                .unwrap_or_else(|| panic!("{key} is not in {}", note.display()))
                .trim()
                .parse()
                .expect("a number")
        };
        for (key, ours) in [
            ("STREET_HEAD", crate::world::town::STREET_HEAD),
            ("POST_HEAD", crate::world::town::POST_HEAD),
            ("STREET_ARM", crate::world::town::STREET_ARM),
        ] {
            assert!(
                (read(key) - ours).abs() < 1.0e-3,
                "Blender hangs {key} at {:.2} m and the game lights it at {ours:.2}",
                read(key),
            );
        }
    }

    /// A different night lights a different set of windows.
    ///
    /// The pattern used to be a property of the BUILDING alone, so a tower wore the
    /// same windows for the life of the world - the sort of thing nobody notices
    /// once and everybody notices eventually.
    #[test]
    fn tonight_is_not_last_night() {
        let at = Vec2::new(1_234.0, -567.0);
        let seed = |night: i64| {
            at.x.to_bits() ^ at.y.to_bits() ^ (night as u32).wrapping_mul(2_654_435_761)
        };
        // The same building over a fortnight: how many of its storeys are in.
        let awake = |night: i64| {
            (0..15)
                .filter(|storey| {
                    crate::world::town::unit(seed(night), *storey as u32 * 31 + 7) <= AWAKE_SHARE
                })
                .collect::<Vec<_>>()
        };
        let first = awake(0);
        let same = (1..14).filter(|night| awake(*night) == first).count();
        assert!(
            same < 3,
            "{same} of the next thirteen nights light exactly the same storeys as the first",
        );
        // And it is the same building each night, not a different one.
        assert_eq!(awake(7), awake(7), "a night is not even consistent with itself");
    }

    /// They are dark by day and lit by night, and they cross over at dusk.
    #[test]
    fn the_lamps_come_on_at_night() {
        let up = |sun: f32| crate::util::smoothstep(LIT_BELOW, FULLY_LIT_AT, sun);
        assert_eq!(up(1.0), 0.0, "lamps burning at noon");
        assert_eq!(up(0.5), 0.0, "lamps burning in the afternoon");
        assert_eq!(up(-0.6), 1.0, "lamps out at midnight");
        let dusk = up(0.06);
        assert!(
            dusk > 0.0 && dusk < 1.0,
            "lamps snap on rather than coming up through dusk: {dusk}",
        );
    }
}
