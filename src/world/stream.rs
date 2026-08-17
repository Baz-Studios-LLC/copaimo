//! Streams terrain chunks in and out around the viewer.
//!
//! The world is far too large to mesh all at once — an 8 km × 4 km map is over
//! two thousand chunks — so only a disc of them around the camera exists at any
//! moment. Meshes are built on background threads and handed back when ready,
//! which keeps generation off the frame budget entirely: you get a chunk
//! appearing a frame or two late rather than a stutter.

use std::collections::HashMap;

use bevy::prelude::*;
use bevy::tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, Task};

use crate::config::{MAX_PENDING_CHUNKS, VIEW_CHUNKS};
use crate::config::CHUNK_SIZE;
use crate::shade::{shaded, Shaded};
use crate::world::chunk::{build_chunk, chunk_at, chunk_origin, Chunk, TerrainMaterial};
use crate::world::terrain::{Terrain, TerrainSource};
use crate::world::{StreamAnchor, WorldBounds};

/// Which chunk coordinates currently have an entity, loaded or still building.
#[derive(Resource, Default)]
pub struct ChunkMap {
    pub loaded: HashMap<IVec2, Entity>,
}

/// A chunk whose mesh is still being built on a worker thread. Dropping this
/// component (by despawning the chunk) cancels the task, so walking away from
/// an area that hasn't finished generating costs nothing.
#[derive(Component)]
pub struct PendingChunk(Task<(Mesh, Option<Mesh>)>);

/// Spawns generation tasks for chunks that are in range but don't exist yet,
/// nearest first so the ground closest to the viewer fills in soonest.
pub fn queue_chunks(
    mut commands: Commands,
    terrain: Res<TerrainSource>,
    bounds: Res<WorldBounds>,
    mut map: ResMut<ChunkMap>,
    pending: Query<&PendingChunk>,
    anchors: Query<&GlobalTransform, With<StreamAnchor>>,
) {
    let Some(anchor) = anchors.iter().next() else {
        return;
    };

    let mut in_flight = pending.iter().count();
    if in_flight >= MAX_PENDING_CHUNKS {
        return;
    }

    let center = chunk_at(anchor.translation());
    let radius_sq = VIEW_CHUNKS * VIEW_CHUNKS;

    // Gather every missing chunk in range with its squared distance, then sort
    // so the nearest gaps are filled first. Using a disc rather than a square
    // means the loaded region is the same depth in every direction.
    let mut wanted: Vec<(i32, IVec2)> = Vec::new();
    for dz in -VIEW_CHUNKS..=VIEW_CHUNKS {
        for dx in -VIEW_CHUNKS..=VIEW_CHUNKS {
            let dist_sq = dx * dx + dz * dz;
            if dist_sq > radius_sq {
                continue;
            }
            let coord = center + IVec2::new(dx, dz);
            if !bounds.contains_chunk(coord) || map.loaded.contains_key(&coord) {
                continue;
            }
            wanted.push((dist_sq, coord));
        }
    }
    wanted.sort_unstable_by_key(|(dist_sq, _)| *dist_sq);

    for (_, coord) in wanted {
        if in_flight >= MAX_PENDING_CHUNKS {
            break;
        }

        // The entity exists from the moment work starts, with its final
        // transform — only the mesh arrives later. Recording it in the map now
        // is what stops the same chunk being queued again next frame.
        let origin = chunk_origin(coord);
        let entity = commands
            .spawn((Chunk(coord), Transform::from_xyz(origin.x, 0.0, origin.y)))
            .id();
        spawn_chunk_mesh(&mut commands, entity, &terrain, coord);

        map.loaded.insert(coord, entity);
        in_flight += 1;
    }
}

/// Queues a background build of one chunk's mesh onto an existing entity.
///
/// Shared by streaming and by the sculpting tool: a chunk coming into view and
/// a chunk invalidated by the brush are the same operation, and in both cases
/// `collect_chunks` attaches the finished mesh. Rebuilding this way leaves the
/// current mesh on screen until the new one lands, so painting never makes the
/// ground blink out.
pub fn spawn_chunk_mesh(
    commands: &mut Commands,
    entity: Entity,
    terrain: &TerrainSource,
    coord: IVec2,
) {
    let generator = terrain.0.clone();
    let task = AsyncComputeTaskPool::get().spawn(async move { build_chunk(&generator, coord) });
    commands.entity(entity).insert(PendingChunk(task));
}

/// Attaches finished meshes to their chunk entities, and plants their trees.
pub fn collect_chunks(
    mut commands: Commands,
    material: Option<Res<TerrainMaterial>>,
    river_skin: Option<Res<RiverMaterial>>,
    grove: Option<Res<Grove>>,
    terrain: Res<TerrainSource>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut pending: Query<(Entity, &mut PendingChunk, &Chunk, Option<&Children>)>,
) {
    // The shared material is created in Startup; on the very first frames a
    // task could finish before it exists, so hold the mesh until it does.
    let Some(material) = material else {
        return;
    };

    for (entity, mut task, chunk, standing) in &mut pending {
        let Some((mesh, river)) = block_on(future::poll_once(&mut task.0)) else {
            continue;
        };
        commands
            .entity(entity)
            .remove::<PendingChunk>()
            .insert((Mesh3d(meshes.add(mesh)), MeshMaterial3d(material.0.clone())));

        // Everything the chunk was carrying goes first — its wood AND its water.
        //
        // Before either is put back, and unconditionally. Sculpting re-meshes a
        // chunk, so this runs again for ground that already had both; clearing
        // inside the planting instead meant a chunk whose grove had not loaded
        // yet kept its old surface and gained a second one on top. That is the
        // same doubling the trees had, and it was one `continue` away from
        // happening again.
        if let Some(standing) = standing {
            for old in standing.iter() {
                commands.entity(old).despawn();
            }
        }

        // Still water in whatever channels cross this chunk. A child, so it
        // streams and dies with the ground it stands in.
        if let (Some(river), Some(water)) = (river, &river_skin) {
            commands.entity(entity).with_children(|chunk| {
                chunk.spawn((
                    RiverSurface,
                    Mesh3d(meshes.add(river)),
                    MeshMaterial3d(water.0.clone()),
                    Transform::IDENTITY,
                ));
            });
        }

        let Some(grove) = &grove else {
            continue;
        };
        plant_chunk(&mut commands, entity, chunk.0, &terrain, grove);
    }
}

/// Clears whatever wood a chunk carries and grows what stands there now.
///
/// Trees are children of the chunk they stand on, so they stream in with that
/// ground and go away with it — no separate bookkeeping, and no wood left
/// standing over a hole where a chunk used to be.
///
/// Cleared before replanting, because a chunk is no longer planted only once:
/// the sculpting mode re-cuts the ground under the brush, and every pass would
/// otherwise leave the old wood behind — doubling the trees on each stroke, with
/// the earlier ones hanging at the height the hill used to be.
///
/// Separate from meshing on purpose. **Planting does not move the ground**, so
/// rebuilding a chunk's mesh to show a new tree is a hundred thousand wasted
/// terrain samples — and worse, it goes through the same one-rebuild-at-a-time
/// throttle the brush uses, so a wide planting stroke found most of its chunks
/// busy and dropped them. Trees appeared slowly, or not at all.
pub fn plant_chunk(
    commands: &mut Commands,
    entity: Entity,
    coord: IVec2,
    terrain: &Terrain,
    grove: &Grove,
) {
    let low = chunk_origin(coord);
    let high = low + CHUNK_SIZE;

    for tree in terrain.trees_in(low, high) {
        let Some(variety) = grove.trees.get(tree.variety) else {
            continue;
        };
        // Placed relative to the chunk, whose own transform already carries it
        // out to where it stands in the world.
        let stance = Transform::from_xyz(tree.at.x - low.x, tree.at.y, tree.at.z - low.y)
            .with_rotation(Quat::from_rotation_y(tree.turn))
            .with_scale(Vec3::splat(tree.scale));

        commands.entity(entity).with_children(|chunk| {
            chunk
                .spawn((
                    Mesh3d(variety.wood.clone()),
                    MeshMaterial3d(variety.bark.clone()),
                    stance,
                ))
                .with_children(|trunk| {
                    trunk.spawn((
                        Mesh3d(variety.leaves.clone()),
                        MeshMaterial3d(variety.leaf.clone()),
                        Transform::IDENTITY,
                    ));
                });
        });
    }
}

/// The grown trees, and what they are painted with.
///
/// One set for the whole world: a forest plants these many times over rather
/// than growing a mesh apiece. A forest is tens of thousands of trees and a mesh
/// each is not affordable — the memory is the least of it and the draw calls are
/// the rest.
#[derive(Resource)]
pub struct Grove {
    /// Wood, leaves, and the green this variety wears.
    pub trees: Vec<Variety>,

}

pub struct Variety {
    pub wood: Handle<Mesh>,
    pub leaves: Handle<Mesh>,
    pub leaf: Handle<Shaded>,
    pub bark: Handle<Shaded>,
}

/// The range a leaf can be, dark to light.
///
/// Every tree in the world used to share ONE material, so a wood of twenty
/// different shapes was twenty shapes in a single flat green — which flattened
/// it more than any amount of shaping could make up for. The tree draws where it
/// sits in this range for itself, so the bench and the game colour it alike.
const LEAF_DARK: Srgba = Srgba::rgb(0.13, 0.28, 0.13);
const LEAF_LIGHT: Srgba = Srgba::rgb(0.38, 0.55, 0.24);

/// And the range a trunk can be, from a spruce's near-black to a birch's chalk.
///
/// One bark material for the whole world made a birch unrecognisable: a pale
/// trunk is most of what tells one from an oak at any distance, and it was the
/// same brown as everything else.
const BARK_DARK: Srgba = Srgba::rgb(0.19, 0.13, 0.09);
const BARK_PALE: Srgba = Srgba::rgb(0.82, 0.80, 0.74);

/// Grows the world's trees once, at startup.
pub fn grow_the_grove(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<Shaded>>,
) {
    let trees = (0..terrain_core::tree::VARIETIES as u32)
        .map(|seed| {
            let tree = terrain_core::tree::grow(seed);
            let green = LinearRgba::from(LEAF_DARK)
                .to_vec4()
                .lerp(LinearRgba::from(LEAF_LIGHT).to_vec4(), tree.tint);
            // Squared, so only a birch gets anywhere near the pale end.
            //
            // Straight, the ramp put every species in the middle of it — and the
            // middle of brown-to-chalk is grey, so a whole wood came out the
            // colour of concrete. Squaring holds oak, spruce, pine and the rest
            // down in the browns where they belong and lets birch, which draws
            // 0.86 and up, still reach the chalk that makes it a birch.
            let pale = tree.bark * tree.bark;
            let wood_colour = LinearRgba::from(BARK_DARK)
                .to_vec4()
                .lerp(LinearRgba::from(BARK_PALE).to_vec4(), pale);
            Variety {
                wood: meshes.add(as_mesh(&tree.wood)),
                leaves: meshes.add(as_mesh(&tree.leaves)),
                // A material apiece. Twenty of them is nothing — the meshes were
                // always twenty, and this is what makes them look like twenty.
                leaf: materials.add(shaded(StandardMaterial {
                    base_color: LinearRgba::from_vec4(green).into(),
                    perceptual_roughness: 0.9,
                    reflectance: 0.04,
                    // Lit from both sides: a canopy left dark underneath reads as
                    // a rock rather than as foliage.
                    double_sided: true,
                    cull_mode: None,
                    ..default()
                })),
                bark: materials.add(shaded(StandardMaterial {
                    base_color: LinearRgba::from_vec4(wood_colour).into(),
                    perceptual_roughness: 0.95,
                    reflectance: 0.03,
                    ..default()
                })),
            }
        })
        .collect();

    commands.insert_resource(Grove {
        trees,
    });
}

/// Despawns chunks the viewer has left behind.
pub fn unload_chunks(
    mut commands: Commands,
    mut map: ResMut<ChunkMap>,
    anchors: Query<&GlobalTransform, With<StreamAnchor>>,
) {
    let Some(anchor) = anchors.iter().next() else {
        return;
    };

    let center = chunk_at(anchor.translation());
    // One chunk of hysteresis beyond the load radius: without it, standing on a
    // chunk boundary would load and unload the same ring every other frame.
    let keep = VIEW_CHUNKS + 1;
    let keep_sq = keep * keep;

    map.loaded.retain(|coord, entity| {
        let delta = *coord - center;
        if delta.x * delta.x + delta.y * delta.y <= keep_sq {
            return true;
        }
        commands.entity(*entity).despawn();
        false
    });
}

/// Turns the shared crate's plain vertex arrays into a Bevy mesh.
///
/// The one engine-shaped seam in the arrangement. `terrain-core` names no engine
/// — that is what lets this game on Bevy 0.16 and Opificium on 0.19 run the same
/// world — so somebody has to do this, and it is a dozen lines on each side.
/// Turns the crate's plain vertex arrays into a Bevy mesh.
///
/// The one engine-shaped seam in the whole arrangement, and the only place that
/// knows both vocabularies.
pub fn as_mesh(geometry: &terrain_core::Geometry) -> Mesh {
    Mesh::new(
        bevy::render::mesh::PrimitiveTopology::TriangleList,
        // Drawn, never read back.
        bevy::asset::RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, geometry.places.clone())
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, geometry.normals.clone())
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, geometry.uvs.clone())
    .with_inserted_indices(bevy::render::mesh::Indices::U32(geometry.indices.clone()))
}

/// The same, plus per-vertex colour when the geometry carries any.
///
/// Ground cover does and trees do not: a welded meadow is one mesh wearing one
/// material, so its many greens have to be in its vertices, where a tree is
/// tinted by the material its variety wears.
pub fn as_coloured_mesh(geometry: &terrain_core::Geometry) -> Mesh {
    let mesh = as_mesh(geometry);
    if geometry.colours.is_empty() {
        return mesh;
    }
    mesh.with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, geometry.colours.clone())
}

/// Marks a river's surface, so a re-meshed chunk can clear its old one.
#[derive(Component)]
pub struct RiverSurface;

/// The one material every river wears.
#[derive(Resource, Deref)]
pub struct RiverMaterial(pub Handle<Shaded>);

pub fn setup_river_material(mut commands: Commands, mut materials: ResMut<Assets<Shaded>>) {
    let handle = materials.add(shaded(StandardMaterial {
        // Darker and greener than the sea, which is what inland water looks
        // like: it is shallow, it is over mud rather than over depth, and it
        // carries what it has washed off the land.
        base_color: Color::srgba(0.16, 0.34, 0.38, 0.82),
        perceptual_roughness: 0.12,
        reflectance: 0.42,
        alpha_mode: AlphaMode::Blend,
        // Seen from below through its own surface at a bank, and a river is thin
        // enough that the far side shows through.
        double_sided: true,
        cull_mode: None,
        ..default()
    }));
    commands.insert_resource(RiverMaterial(handle));
}
