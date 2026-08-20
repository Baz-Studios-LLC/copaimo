//! World overview for the terrain tool.
//!
//! An 8 km world is far too big to navigate by flying and hoping. This renders
//! the whole map top-down, marks where the camera is, and refreshes itself as
//! you sculpt — so you can see the coastline you're shaping in context instead
//! of only ever seeing the few hundred meters in front of you.
//!
//! It's built on a background thread, the same as chunk meshes. Sampling the
//! heightfield tens of thousands of times is far too slow for a frame, and a
//! tool that hitches every time it updates is a tool people stop trusting.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, Task};

use crate::camera::MainCamera;
use crate::tools::theme::{self, UiFont, TEXT_DIM, TEXT_MUTED};
use crate::states::AppState;
use crate::world::biome::surface_color;
use crate::world::terrain::{Terrain, TerrainSource};

/// Width of the rendered overview in pixels. The height follows the world's
/// aspect ratio.
const WIDTH: u32 = 256;

/// How long the edit layer must sit unchanged before the overview redraws.
/// Without this it would queue a rebuild on every frame of a drag.
const QUIET_PERIOD: f32 = 1.2;

/// How far across the heading needle's wrapper is, in pixels. The bar itself is
/// half of it, standing up from the middle.
const HEADING_SPAN: f32 = 26.0;

#[derive(Resource, Default)]
struct Minimap {
    /// How many cells any layer had painted at the last redraw, added together.
    ///
    /// The ground alone once, which meant painting a country changed the world
    /// and left the overview showing the old one — and the overview is the ONLY
    /// place a maker can see a whole region at once, so it is the one view that
    /// must not go stale while they are drawing a region.
    last_cells: usize,
    /// Seconds since the edit layer last changed.
    quiet: f32,
    building: bool,
}

#[derive(Component)]
struct MinimapImage;

#[derive(Component)]
struct MinimapMarker;

/// The needle that says which way the camera is looking.
///
/// A dot says where you are and nothing about where you are pointing, which on a
/// map with no landmarks yet is half the information missing: you can see that you
/// are on the north coast and not whether you are looking at it or away from it.
///
/// Rotated about the marker rather than about itself, which is why it is a wrapper
/// with a bar inside. A bar rotated about its own middle swings around a point
/// halfway up itself and reads as an axis through you rather than a direction from
/// you.
#[derive(Component)]
struct MinimapHeading;

#[derive(Component)]
struct MinimapRoot;

/// The box the world is drawn in, so a click on it can be turned into a place.
#[derive(Component)]
struct MinimapFrame;

/// A finished overview: raw RGBA and the dimensions it was rendered at.
#[derive(Component)]
struct MinimapTask(Task<(UVec2, Vec<u8>)>);

pub struct MinimapPlugin;

impl Plugin for MinimapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Minimap>()
            .add_systems(OnEnter(AppState::Editing), (spawn_panel, request_redraw))
            .add_systems(OnExit(AppState::Editing), despawn_panel)
            .add_systems(
                Update,
                (track_edits, collect_redraw, place_marker, fly_to_click)
                    .run_if(in_state(AppState::Editing)),
            );
    }
}

/// Pixel dimensions of the overview for a world of the given half-extents.
fn dimensions(half: Vec2) -> UVec2 {
    let height = (WIDTH as f32 * half.y / half.x).round().max(1.0) as u32;
    UVec2::new(WIDTH, height)
}

fn spawn_panel(mut commands: Commands, font: Res<UiFont>, terrain: Res<TerrainSource>) {
    let size = dimensions(terrain.half());

    commands
        .spawn((
            MinimapRoot,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(16.0),
                top: Val::Px(16.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(10.0)),
                row_gap: Val::Px(7.0),
                ..default()
            },
            BackgroundColor(theme::PANEL),
        ))
        .with_children(|panel| {
            panel
                .spawn(Node {
                    // Sized to the map below rather than 100%: the panel's own
                    // width is driven by its content, so a percentage width
                    // here would collapse to the text and leave the title and
                    // the scale label touching.
                    width: Val::Px(size.x as f32),
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                })
                .with_children(|bar| {
                    bar.spawn((
                        Text::new("WORLD OVERVIEW"),
                        font.at(11.0),
                        TextColor(TEXT_DIM),
                    ));
                    bar.spawn((
                        Text::new(format!("{:.0} km", terrain.half().x * 2.0 / 1000.0)),
                        font.at(11.0),
                        TextColor(TEXT_MUTED),
                    ));
                });

            // The image and the marker share a parent so the marker can be
            // positioned as a straight percentage of the map's own box.
            panel
                .spawn((
                    MinimapFrame,
                    Button,
                    Node {
                        width: Val::Px(size.x as f32),
                        height: Val::Px(size.y as f32),
                        ..default()
                    },
                ))
                .with_children(|frame| {
                    frame.spawn((
                        MinimapImage,
                        ImageNode::new(Handle::default()),
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                    ));
                    frame.spawn((
                        MinimapHeading,
                        Node {
                            position_type: PositionType::Absolute,
                            // Square and centred on the marker, so rotating it
                            // rotates about where the camera actually is.
                            width: Val::Px(HEADING_SPAN),
                            height: Val::Px(HEADING_SPAN),
                            margin: UiRect {
                                left: Val::Px(-HEADING_SPAN * 0.5),
                                top: Val::Px(-HEADING_SPAN * 0.5),
                                ..default()
                            },
                            ..default()
                        },
                    ))
                    .with_children(|wrap| {
                        // The bar itself, standing up from the middle. Pointing
                        // north at rest, which is up on a map drawn north-up.
                        wrap.spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Percent(50.0),
                                top: Val::Px(0.0),
                                width: Val::Px(2.0),
                                height: Val::Px(HEADING_SPAN * 0.5),
                                margin: UiRect {
                                    left: Val::Px(-1.0),
                                    ..default()
                                },
                                ..default()
                            },
                            BackgroundColor(Color::srgba(1.0, 0.30, 0.35, 0.85)),
                        ));
                    });
                    frame.spawn((
                        MinimapMarker,
                        Node {
                            position_type: PositionType::Absolute,
                            width: Val::Px(7.0),
                            height: Val::Px(7.0),
                            margin: UiRect {
                                left: Val::Px(-3.5),
                                top: Val::Px(-3.5),
                                ..default()
                            },
                            ..default()
                        },
                        BackgroundColor(Color::srgb(1.0, 0.30, 0.35)),
                    ));
                });

            panel.spawn((
                Text::new("N is up  -  hold ALT, click to fly there"),
                font.at(10.0),
                TextColor(TEXT_DIM),
            ));
        });
}

/// Flies the camera to wherever the overview was clicked.
///
/// An 8 km world crossed by pointing the nose and holding W is the single worst
/// thing about shaping one — a minute of flying to reach a coastline, and
/// another to get back. The map already knows where everything is; this makes it
/// answer.
///
/// Only while ALT holds the pointer free, which is also when the brush is not
/// painting, so a click can mean one thing or the other and never both.
fn fly_to_click(
    buttons: Res<ButtonInput<MouseButton>>,
    free: Res<crate::editor::CursorFree>,
    terrain: Res<TerrainSource>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    frames: Query<(&ComputedNode, &GlobalTransform), With<MinimapFrame>>,
    mut cameras: Query<&mut Transform, With<MainCamera>>,
    mut toast: ResMut<crate::editor::ui::Toast>,
) {
    // Pointing rather than looking — the tool's resting state now, so this is only
    // asking that somebody is not mid-turn of the view.
    if !free.0 || !buttons.just_pressed(MouseButton::Left) {
        return;
    }
    let (Some(window), Some((frame, at)), Some(mut camera)) = (
        windows.iter().next(),
        frames.iter().next(),
        cameras.iter_mut().next(),
    ) else {
        return;
    };
    let Some(pointer) = window.cursor_position() else {
        return;
    };

    // The node's own box, brought into the LOGICAL pixels the cursor is given
    // in — its size and place are physical, and compared raw the two agree only
    // at 100% display scale. On a laptop at 125% every click landed short and
    // left of where it pointed, by exactly the scale factor.
    let logical = frame.inverse_scale_factor();
    let size = frame.size() * logical;
    let corner = at.translation().truncate() * logical - size * 0.5;
    let within = (pointer - corner) / size;
    if within.x < 0.0 || within.x > 1.0 || within.y < 0.0 || within.y > 1.0 {
        return;
    }

    let half = terrain.half();
    let world = (within * 2.0 - Vec2::ONE) * half;
    // Held well clear of whatever is underneath, so arriving inside a mountain
    // is not a thing that can happen.
    let above = terrain.height(world.x, world.y).max(0.0) + ARRIVAL_HEIGHT;
    camera.translation = Vec3::new(world.x, above, world.y);

    toast.show(format!("Flew to {:.0}, {:.0}", world.x, world.y));
}

/// How high above the ground a jump leaves the camera.
const ARRIVAL_HEIGHT: f32 = 180.0;

fn despawn_panel(mut commands: Commands, roots: Query<Entity, With<MinimapRoot>>) {
    for entity in &roots {
        commands.entity(entity).despawn();
    }
}

/// Queues a background render of the whole world.
fn request_redraw(mut commands: Commands, terrain: Res<TerrainSource>, mut state: ResMut<Minimap>) {
    if state.building {
        return;
    }
    state.building = true;

    let generator = terrain.0.clone();
    let size = dimensions(terrain.half());
    let task = AsyncComputeTaskPool::get().spawn(async move { (size, render(&generator, size)) });
    commands.spawn(MinimapTask(task));
}

/// Redraws once the edit layer has been quiet for a moment.
fn track_edits(
    commands: Commands,
    time: Res<Time>,
    terrain: Res<TerrainSource>,
    mut state: ResMut<Minimap>,
) {
    let ground = terrain.edits().read().map(|edits| edits.sculpted_cells());
    let marked = terrain.countries().read().map(|them| them.painted_cells());
    let cells = match (ground, marked) {
        (Ok(ground), Ok(marked)) => ground + marked,
        // A locked layer is a frame to wait, not a reason to redraw.
        _ => state.last_cells,
    };

    if cells != state.last_cells {
        state.last_cells = cells;
        state.quiet = 0.0;
        return;
    }

    // Only counts once there's something to redraw for.
    if state.quiet >= QUIET_PERIOD {
        return;
    }
    state.quiet += time.delta_secs();
    if state.quiet >= QUIET_PERIOD {
        request_redraw(commands, terrain, state);
    }
}

fn collect_redraw(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut state: ResMut<Minimap>,
    mut tasks: Query<(Entity, &mut MinimapTask)>,
    mut targets: Query<&mut ImageNode, With<MinimapImage>>,
) {
    for (entity, mut task) in &mut tasks {
        let Some((size, pixels)) = block_on(future::poll_once(&mut task.0)) else {
            continue;
        };
        commands.entity(entity).despawn();
        state.building = false;

        let image = images.add(Image::new(
            Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            pixels,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD,
        ));

        // Swapping the handle rather than mutating in place: the old image is
        // released as soon as nothing points at it, and there's no window where
        // the texture is half-written.
        for mut node in &mut targets {
            node.image = image.clone();
        }
    }
}

fn place_marker(
    terrain: Res<TerrainSource>,
    cameras: Query<&GlobalTransform, With<MainCamera>>,
    mut markers: Query<&mut Node, (With<MinimapMarker>, Without<MinimapHeading>)>,
    mut headings: Query<(&mut Node, &mut Transform), With<MinimapHeading>>,
) {
    let (Some(camera), Some(mut marker)) = (cameras.iter().next(), markers.iter_mut().next())
    else {
        return;
    };

    let half = terrain.half();
    let position = camera.translation();
    let u = ((position.x + half.x) / (half.x * 2.0)).clamp(0.0, 1.0);
    let v = ((position.z + half.y) / (half.y * 2.0)).clamp(0.0, 1.0);

    marker.left = Val::Percent(u * 100.0);
    marker.top = Val::Percent(v * 100.0);

    // And which way it is looking.
    //
    // The bearing is measured clockwise from north, because that is what a map
    // means by a direction: `+x` is east and `+z` is south in this world, so north
    // is `-z` and the needle at rest points up.
    //
    // Rotating about Z in UI space turns +X toward +Y, and UI's +Y is DOWN the
    // screen — so a positive angle reads clockwise, which is the same sense as the
    // bearing and needs no minus sign.
    let facing = camera.forward();
    let bearing = facing.x.atan2(-facing.z);
    for (mut node, mut turn) in &mut headings {
        node.left = Val::Percent(u * 100.0);
        node.top = Val::Percent(v * 100.0);
        turn.rotation = Quat::from_rotation_z(bearing);
    }
}

/// Samples the world into RGBA pixels. Pure and thread-safe.
fn render(terrain: &Terrain, size: UVec2) -> Vec<u8> {
    let half = terrain.half();
    let mut pixels = Vec::with_capacity((size.x * size.y * 4) as usize);

    // One pixel is tens of meters, so the normal is taken over that same
    // distance — a 1 m epsilon would report slopes the map can't show.
    let epsilon = (half.x * 2.0 / size.x as f32) * 0.5;

    for py in 0..size.y {
        for px in 0..size.x {
            let x = (px as f32 / (size.x - 1) as f32 * 2.0 - 1.0) * half.x;
            let z = (py as f32 / (size.y - 1) as f32 * 2.0 - 1.0) * half.y;

            let height = terrain.height(x, z);
            let slope = 1.0 - terrain.normal(x, z, epsilon).y;
            // The same classification the terrain itself uses, so the overview
            // reads as the world rather than as a separate diagram.
            let color = surface_color(
                Vec2::new(x, z),
                height,
                slope,
                terrain.shore_character(x, z),
                terrain.worn(x, z),
                terrain.region(x, z).0,
                terrain.region(x, z).1,
            );

            // `surface_color` returns linear; the texture is sRGB.
            let encode = |linear: f32| {
                let srgba = LinearRgba::rgb(linear, linear, linear);
                (Srgba::from(srgba).red.clamp(0.0, 1.0) * 255.0) as u8
            };
            pixels.extend_from_slice(&[
                encode(color[0]),
                encode(color[1]),
                encode(color[2]),
                255,
            ]);
        }
    }

    pixels
}
