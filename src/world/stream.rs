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
use crate::world::chunk::{build_mesh, chunk_at, chunk_origin, Chunk, TerrainMaterial};
use crate::world::terrain::TerrainSource;
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
pub struct PendingChunk(Task<Mesh>);

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
    let task = AsyncComputeTaskPool::get().spawn(async move { build_mesh(&generator, coord) });
    commands.entity(entity).insert(PendingChunk(task));
}

/// Attaches finished meshes to their chunk entities, and plants their trees.
pub fn collect_chunks(
    mut commands: Commands,
    material: Option<Res<TerrainMaterial>>,
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
        let Some(mesh) = block_on(future::poll_once(&mut task.0)) else {
            continue;
        };
        commands
            .entity(entity)
            .remove::<PendingChunk>()
            .insert((Mesh3d(meshes.add(mesh)), MeshMaterial3d(material.0.clone())));

        // Trees are children of the chunk they stand on, so they stream in with
        // that ground and go away with it — no separate bookkeeping, and no wood
        // left standing over a hole where a chunk used to be.
        //
        // Cleared before replanting, because a chunk is no longer meshed only
        // once: the sculpting mode re-cuts the ground under the brush, and every
        // pass through here would otherwise leave the old wood behind — doubling
        // the trees on each stroke, with the earlier ones hanging at the height
        // the hill used to be.
        if let Some(standing) = standing {
            for tree in standing.iter() {
                commands.entity(tree).despawn();
            }
        }

        let Some(grove) = &grove else {
            continue;
        };
        let low = chunk_origin(chunk.0);
        let high = low + CHUNK_SIZE;

        for tree in terrain.trees_in(low, high) {
            let Some((wood, leaves)) = grove.trees.get(tree.variety) else {
                continue;
            };
            // Placed relative to the chunk, whose own transform already carries
            // it out to where it stands in the world.
            let stance = Transform::from_xyz(tree.at.x - low.x, tree.at.y, tree.at.z - low.y)
                .with_rotation(Quat::from_rotation_y(tree.turn))
                .with_scale(Vec3::splat(tree.scale));

            commands.entity(entity).with_children(|chunk| {
                chunk
                    .spawn((
                        Mesh3d(wood.clone()),
                        MeshMaterial3d(grove.bark.clone()),
                        stance,
                    ))
                    .with_children(|trunk| {
                        trunk.spawn((
                            Mesh3d(leaves.clone()),
                            MeshMaterial3d(grove.leaf.clone()),
                            Transform::IDENTITY,
                        ));
                    });
            });
        }
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
    pub trees: Vec<(Handle<Mesh>, Handle<Mesh>)>,
    pub bark: Handle<StandardMaterial>,
    pub leaf: Handle<StandardMaterial>,
}

/// Grows the world's trees once, at startup.
pub fn grow_the_grove(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let trees = (0..terrain_core::tree::VARIETIES as u32)
        .map(|seed| {
            let tree = terrain_core::tree::grow(seed);
            (meshes.add(as_mesh(&tree.wood)), meshes.add(as_mesh(&tree.leaves)))
        })
        .collect();

    commands.insert_resource(Grove {
        trees,
        bark: materials.add(StandardMaterial {
            base_color: Srgba::rgb(0.29, 0.21, 0.15).into(),
            perceptual_roughness: 0.95,
            reflectance: 0.03,
            ..default()
        }),
        leaf: materials.add(StandardMaterial {
            base_color: Srgba::rgb(0.20, 0.38, 0.19).into(),
            perceptual_roughness: 0.9,
            reflectance: 0.04,
            // Lit from both sides: a canopy left dark underneath reads as a rock
            // rather than as foliage.
            double_sided: true,
            cull_mode: None,
            ..default()
        }),
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
fn as_mesh(geometry: &terrain_core::Geometry) -> Mesh {
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
