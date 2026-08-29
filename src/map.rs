//! The map a player pulls up, with M.
//!
//! # Why the game needs one at all
//!
//! The world is twelve kilometres across with thirteen settlements on it and two
//! bridges, and until now the only way to see any of that at once was the terrain
//! tool's overview - which is a maker's window, not a player's. An adventure whose
//! roads deliberately go the long way round is an adventure you cannot navigate by
//! looking at the horizon.
//!
//! # It paints the same world the tool does
//!
//! The painting lives in `world::chart` and both maps ask for it, because two maps
//! drawn by two pieces of code are two maps that disagree the first time one of them
//! is changed. What this adds on top is a player's business rather than a maker's:
//! where the warden is standing, which way the view is pointed, and the NAMES of the
//! places
//! - a mark on a map you cannot name is a mark you cannot ask anybody about.
//!
//! # Painted once, on a background thread
//!
//! Sampling the world tens of thousands of times is far too slow for a frame. It is
//! painted the first time the map is opened and kept, so opening it again is free.
//! The world does not change underfoot in play, so there is nothing to redraw.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, Task};

use crate::states::AppState;
use crate::typeface::UiFont;
use crate::world::chart::{dimensions, paint, WIDTH};
use crate::world::terrain::TerrainSource;

/// How much of the window's shorter side the map fills.
const FILLS: f32 = 0.86;

/// The dim behind the map, so the world does not compete with it.
const BEHIND: Color = Color::srgba(0.02, 0.03, 0.05, 0.72);
const INK: Color = Color::srgb(0.93, 0.92, 0.88);
const FAINT: Color = Color::srgb(0.66, 0.65, 0.62);
/// The panel shown while the world is still being painted.
const WAITING: Color = Color::srgba(0.08, 0.10, 0.14, 0.92);
/// What a place name is written on, so it reads over snow as well as over grass.
const LABEL_BEHIND: Color = Color::srgba(0.05, 0.06, 0.09, 0.66);
/// The warden's own mark. Nothing else on the map is this colour.
const YOU: Color = Color::srgb(0.98, 0.36, 0.30);

/// The painted world, once it exists.
#[derive(Resource, Default)]
struct Chart {
    image: Option<Handle<Image>>,
    size: UVec2,
    /// Whether a painting is in flight, so two are never started.
    painting: bool,
    /// The state of the world the image in hand was painted from.
    ///
    /// Not a bool. It was one - painted once and kept forever - and in a maker's
    /// build that is wrong: open the map, go into the terrain tool, move a
    /// coastline, come back, and the map still shows the world as it was. One
    /// painting has to mean one state of the world.
    painted_at: Option<usize>,
    /// The state it is being painted from right now.
    painting_at: usize,
}

/// How much of the world has been shaped, as a number that changes when it does.
///
/// The same two counters the terrain tool's overview watches to know when to redraw
/// itself, and for the same reason.
#[cfg(feature = "tools")]
fn how_the_world_stands(terrain: &TerrainSource) -> usize {
    let ground = terrain.edits().read().map(|edits| edits.sculpted_cells());
    let marked = terrain.countries().read().map(|them| them.painted_cells());
    match (ground, marked) {
        (Ok(ground), Ok(marked)) => ground + marked,
        // A locked layer is not a new world. Saying so would repaint on a frame
        // that happened to catch the lock.
        _ => usize::MAX,
    }
}

/// In a shipped game the world cannot be reshaped, so it always stands the same way
/// and the first painting is good for the whole session.
///
/// The sculpt and paint counters this reads in a maker's build are themselves behind
/// `tools` - they are the terrain editor's layers - so this is not a shortcut but the
/// only answer available.
#[cfg(not(feature = "tools"))]
fn how_the_world_stands(_terrain: &TerrainSource) -> usize {
    0
}

/// Whether the map is up.
///
/// Public so `--photo --map` can raise it: the standing rule on this project is to
/// load the game and LOOK at a change, and a map that only a keypress can open is a
/// map I can only reason about.
#[derive(Resource, Default)]
pub struct Open(pub bool);

/// Whether the map is up, as a run condition.
///
/// `Option`, so a build or a test that never adds `MapPlugin` still answers "no"
/// rather than panicking on a resource that was never inserted.
pub fn is_open(open: Option<Res<Open>>) -> bool {
    open.is_some_and(|open| open.0)
}

/// The map on screen, and whether the painting had arrived when it was raised.
///
/// Kept on the marker so the shell can go up before the painting is ready and be
/// rebuilt the moment it lands.
#[derive(Component)]
struct MapRoot {
    painted: bool,
}

#[derive(Component)]
struct MapImage;

#[derive(Component)]
struct YouAreHere;

/// The needle that says which way the view is pointed.
///
/// # Whose facing, explicitly
///
/// The CAMERA'S, not the warden's. The two differ while you orbit, and this is the
/// one worth drawing: walking is camera-relative, so the direction the view is
/// pointed is the direction "forward" will take you. A needle showing the warden's
/// own facing would swing about while the player stood still turning the camera to
/// get their bearings, which is exactly when a map is being read.
#[derive(Component)]
struct Heading;

#[derive(Component)]
struct Painting(Task<(UVec2, Vec<u8>)>);

/// M opens and closes it; Escape closes it.
///
/// Escape only CLOSES. It is the pause key everywhere else in the game, and a key
/// that opens the map from the pause menu and pauses from the map is a key nobody
/// can predict.
fn toggle(keys: Res<ButtonInput<KeyCode>>, mut open: ResMut<Open>) {
    if keys.just_pressed(KeyCode::KeyM) {
        open.0 = !open.0;
    } else if open.0 && keys.just_pressed(KeyCode::Escape) {
        open.0 = false;
    }
}

/// Starts painting the world, the first time anybody asks to see it.
fn start_painting(
    mut commands: Commands,
    open: Res<Open>,
    terrain: Res<TerrainSource>,
    mut chart: ResMut<Chart>,
) {
    if !open.0 || chart.painting {
        return;
    }
    let now = how_the_world_stands(&terrain);
    if now == usize::MAX || chart.painted_at == Some(now) {
        return;
    }
    chart.painting = true;
    chart.painting_at = now;
    let size = dimensions(terrain.half());
    let generator = terrain.0.clone();
    commands.spawn(Painting(
        AsyncComputeTaskPool::get().spawn(async move { (size, paint(&generator, size)) }),
    ));
}

fn collect_painting(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut chart: ResMut<Chart>,
    mut painting: Query<(Entity, &mut Painting)>,
) {
    for (entity, mut task) in &mut painting {
        let Some((size, pixels)) = block_on(future::poll_once(&mut task.0)) else {
            continue;
        };
        commands.entity(entity).despawn();
        chart.painting = false;
        chart.painted_at = Some(chart.painting_at);
        chart.size = size;
        chart.image = Some(images.add(Image::new(
            Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            pixels,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
        )));
    }
}

/// Puts the map on screen, and takes it off again.
/// Puts the map on screen, and takes it off again.
///
/// The shell goes up the moment M is pressed, whether or not the world has been
/// painted yet - the first press starts a background painting, and on a slow machine
/// a map that shows nothing until it finishes looks like a key that did not work. It
/// is rebuilt once when the painting lands, which is what `MapRoot::painted` tracks.
fn show_or_hide(
    mut commands: Commands,
    open: Res<Open>,
    chart: Res<Chart>,
    font: Res<UiFont>,
    terrain: Res<TerrainSource>,
    windows: Query<&Window>,
    standing: Query<(Entity, &MapRoot)>,
) {
    let up = standing.iter().next();
    let painted = chart.image.is_some();

    if !open.0 {
        if let Some((entity, _)) = up {
            commands.entity(entity).despawn();
        }
        return;
    }
    match up {
        // Already showing the right thing.
        Some((_, root)) if root.painted == painted => return,
        // Showing the "drawing" shell, and the painting has arrived.
        Some((entity, _)) => commands.entity(entity).despawn(),
        None => {}
    }

    // Sized to the window's shorter side, keeping the world's own proportions - a
    // map stretched to the screen is a map that lies about which way is far.
    let window = windows.iter().next();
    let (vw, vh) = window.map_or((1280.0, 720.0), |w| (w.width(), w.height()));
    let size = if painted {
        chart.size
    } else {
        dimensions(terrain.half())
    };
    let aspect = size.x as f32 / size.y.max(1) as f32;
    let mut high = vh * FILLS;
    let mut wide = high * aspect;
    if wide > vw * FILLS {
        wide = vw * FILLS;
        high = wide / aspect;
    }

    let half = terrain.half();
    let sites: Vec<_> = terrain.sites().to_vec();
    let image = chart.image.clone();
    commands
        .spawn((
            MapRoot { painted },
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(10.0),
                ..default()
            },
            BackgroundColor(BEHIND),
            GlobalZIndex(40),
        ))
        .with_children(|screen| {
            screen.spawn((Text::new("THE WARDENS GUILD"), font.at(13.0), TextColor(FAINT)));
            screen
                .spawn((
                    MapImage,
                    Node {
                        width: Val::Px(wide),
                        height: Val::Px(high),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    // Something to look at while the painting is on its way.
                    BackgroundColor(if painted { Color::NONE } else { WAITING }),
                ))
                .with_children(|frame| {
                    let Some(image) = image else {
                        frame.spawn((
                            Text::new("Drawing map..."),
                            font.at(12.0),
                            TextColor(FAINT),
                        ));
                        return;
                    };
                    frame.spawn((
                        ImageNode::new(image),
                        Node {
                            position_type: PositionType::Absolute,
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                    ));

                    // The places, named. Only the ones that HAVE names: a label on
                    // every hamlet is a map you cannot read.
                    for (index, site) in sites.iter().enumerate() {
                        if site.ranch {
                            continue;
                        }
                        let country = terrain.region(site.at.x, site.at.y).0;
                        let Some(name) = crate::world::town::name_of(site, country, index) else {
                            continue;
                        };
                        let u = (site.at.x + half.x) / (half.x * 2.0);
                        let v = (site.at.y + half.y) / (half.y * 2.0);
                        // A name sits to the RIGHT of its mark, except near the
                        // right-hand edge, where it would run off the map -
                        // photographed, "Marrowmede" came out as "Marrowmed" with
                        // the rest over the edge.
                        //
                        // And clear of the mark by more than the mark's own radius:
                        // at seven pixels the disc sat on the first letter, so every
                        // name lost its initial - "Bellwether" read as "ellwether".
                        let near_the_edge = u > 0.70;
                        frame.spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                left: if near_the_edge {
                                    Val::Auto
                                } else {
                                    Val::Percent(u * 100.0)
                                },
                                right: if near_the_edge {
                                    Val::Percent((1.0 - u) * 100.0)
                                } else {
                                    Val::Auto
                                },
                                top: Val::Percent(v * 100.0),
                                margin: UiRect {
                                    left: Val::Px(if near_the_edge { 0.0 } else { 11.0 }),
                                    right: Val::Px(if near_the_edge { 11.0 } else { 0.0 }),
                                    top: Val::Px(-8.0),
                                    ..default()
                                },
                                padding: UiRect::axes(Val::Px(3.0), Val::Px(1.0)),
                                ..default()
                            },
                            Text::new(name),
                            font.at(if site.city { 12.0 } else { 10.0 }),
                            TextColor(if site.city { INK } else { FAINT }),
                            // ON A BACKING, because the map is not one colour.
                            //
                            // Pale text is legible over green, sea and sand and
                            // INVISIBLE over snow - photographed, "Marrowmede" read
                            // as "Marrowme" and "Colderry" as "Colde", the rest of
                            // each name lost in the icefield behind it.
                            BackgroundColor(LABEL_BEHIND),
                            BorderRadius::all(Val::Px(2.0)),
                        ));
                    }

                    // And the warden, last, so he is over everything. The needle
                    // first, so the mark sits on top of it.
                    crate::world::chart::needle(frame, Heading, YOU);
                    frame.spawn((
                        YouAreHere,
                        Node {
                            position_type: PositionType::Absolute,
                            width: Val::Px(9.0),
                            height: Val::Px(9.0),
                            margin: UiRect {
                                left: Val::Px(-4.5),
                                top: Val::Px(-4.5),
                                ..default()
                            },
                            border: UiRect::all(Val::Px(1.5)),
                            ..default()
                        },
                        BorderColor(Color::BLACK),
                        BorderRadius::all(Val::Percent(50.0)),
                        BackgroundColor(YOU),
                    ));
                });
            screen.spawn((
                Text::new("M or ESC to close     N is up"),
                font.at(10.0),
                TextColor(FAINT),
            ));
        });
}

/// Keeps the warden's mark where the warden is.
fn move_the_mark(
    terrain: Res<TerrainSource>,
    anchors: Query<&GlobalTransform, With<crate::world::StreamAnchor>>,
    cameras: Query<&GlobalTransform, With<crate::camera::MainCamera>>,
    mut marks: Query<&mut Node, (With<YouAreHere>, Without<Heading>)>,
    mut headings: Query<(&mut Node, &mut Transform), With<Heading>>,
) {
    let (Some(anchor), Some(mut mark)) = (anchors.iter().next(), marks.iter_mut().next()) else {
        return;
    };
    let half = terrain.half();
    let at = anchor.translation();
    let u = ((at.x + half.x) / (half.x * 2.0)).clamp(0.0, 1.0);
    let v = ((at.z + half.y) / (half.y * 2.0)).clamp(0.0, 1.0);
    mark.left = Val::Percent(u * 100.0);
    mark.top = Val::Percent(v * 100.0);

    // And which way the view is pointed - see `Heading` for why it is the camera's
    // facing and not the warden's.
    let Some(camera) = cameras.iter().next() else {
        return;
    };
    let bearing = crate::world::chart::bearing_of(camera.forward().into());
    for (mut node, mut turn) in &mut headings {
        node.left = Val::Percent(u * 100.0);
        node.top = Val::Percent(v * 100.0);
        turn.rotation = Quat::from_rotation_z(bearing);
    }
}

/// Takes the map down on the way out of play, so it is not still up in the menu.
///
/// And drops any painting still in flight. A task started in play can land after the
/// world has been reshaped in the terrain tool, and what it hands back is a picture
/// of a world that no longer exists.
fn put_it_away(
    mut commands: Commands,
    mut open: ResMut<Open>,
    mut chart: ResMut<Chart>,
    standing: Query<Entity, With<MapRoot>>,
    painting: Query<Entity, With<Painting>>,
) {
    open.0 = false;
    for entity in &standing {
        commands.entity(entity).despawn();
    }
    for entity in &painting {
        commands.entity(entity).despawn();
    }
    chart.painting = false;
}

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Chart>()
            .init_resource::<Open>()
            .add_systems(
                Update,
                (
                    toggle,
                    start_painting,
                    collect_painting,
                    show_or_hide,
                    move_the_mark,
                )
                    .chain()
                    .run_if(in_state(AppState::Playing)),
            )
            .add_systems(OnExit(AppState::Playing), put_it_away);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The map is painted at the world's own proportions.
    ///
    /// A map stretched to the window lies about which way is far, and this world is
    /// noticeably taller than it is wide - so a square map would put two settlements
    /// side by side that are a long walk apart.
    #[test]
    fn the_map_keeps_the_worlds_proportions() {
        let half = Vec2::new(6_144.0, 7_662.0);
        let size = dimensions(half);
        let world = half.x / half.y;
        let drawn = size.x as f32 / size.y as f32;
        assert!(
            (world - drawn).abs() < 0.02,
            "the world is {world:.3} wide for its height and the map is {drawn:.3}",
        );
        assert_eq!(size.x, WIDTH);
    }
}
