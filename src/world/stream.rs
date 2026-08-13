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

/// Attaches finished meshes to their chunk entities.
pub fn collect_chunks(
    mut commands: Commands,
    material: Option<Res<TerrainMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut pending: Query<(Entity, &mut PendingChunk)>,
) {
    // The shared material is created in Startup; on the very first frames a
    // task could finish before it exists, so hold the mesh until it does.
    let Some(material) = material else {
        return;
    };

    for (entity, mut task) in &mut pending {
        let Some(mesh) = block_on(future::poll_once(&mut task.0)) else {
            continue;
        };
        commands
            .entity(entity)
            .remove::<PendingChunk>()
            .insert((Mesh3d(meshes.add(mesh)), MeshMaterial3d(material.0.clone())));
    }
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
