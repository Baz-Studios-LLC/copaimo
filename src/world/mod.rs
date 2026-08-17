//! The world: terrain generation, chunk streaming, and the sea.

pub mod biome;
pub mod forest;
pub mod chunk;
pub mod cover;
pub mod edit;
pub mod heightmap;
pub mod settle;
pub mod stream;
pub mod surface;
pub mod terrain;
pub mod water;

use std::sync::Arc;

use bevy::prelude::*;

use crate::config::CHUNK_SIZE;
use crate::world::terrain::{Terrain, TerrainSource};

/// Marks the entity that terrain streaming centers on. The camera carries it,
/// because the camera is always where the viewer actually is — in follow mode
/// and in free-fly alike.
#[derive(Component)]
pub struct StreamAnchor;

/// The finite extent of the world, derived from the map image's proportions and
/// `WORLD_WIDTH`.
///
/// This is the shared authority on "where does the world end" — the player, the
/// spawn search, and (later) monsters and NPCs all clamp against the same
/// numbers rather than each carrying their own copy.
#[derive(Resource, Clone, Copy)]
pub struct WorldBounds {
    /// Half-extents in meters: X east/west, Y north/south.
    pub half: Vec2,
    pub min_chunk: IVec2,
    pub max_chunk: IVec2,
}

impl WorldBounds {
    fn new(half: Vec2) -> Self {
        Self {
            half,
            min_chunk: IVec2::new(
                (-half.x / CHUNK_SIZE).floor() as i32,
                (-half.y / CHUNK_SIZE).floor() as i32,
            ),
            max_chunk: IVec2::new(
                (half.x / CHUNK_SIZE).ceil() as i32 - 1,
                (half.y / CHUNK_SIZE).ceil() as i32 - 1,
            ),
        }
    }

    pub fn contains_chunk(&self, coord: IVec2) -> bool {
        coord.x >= self.min_chunk.x
            && coord.x <= self.max_chunk.x
            && coord.y >= self.min_chunk.y
            && coord.y <= self.max_chunk.y
    }

    /// Keeps a world position inside the map. `margin` holds the subject that
    /// far in from the border so it can't stand half-off the edge of the world.
    pub fn clamp(&self, position: Vec3, margin: f32) -> Vec3 {
        let limit = (self.half - Vec2::splat(margin)).max(Vec2::ZERO);
        Vec3::new(
            position.x.clamp(-limit.x, limit.x),
            position.y,
            position.z.clamp(-limit.y, limit.y),
        )
    }
}

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        // Built at plugin-build time rather than in a startup system: the
        // player's spawn search and the first chunk tasks both need to query
        // terrain height, and this guarantees it exists before any of them run.
        let terrain = Terrain::new();
        let bounds = WorldBounds::new(terrain.half());

        app.insert_resource(TerrainSource(Arc::new(terrain)))
            .insert_resource(bounds)
            .init_resource::<stream::ChunkMap>()
            .add_systems(
                Startup,
                (
                    chunk::setup_material,
                    cover::setup_material,
                    stream::setup_river_material,
                    water::spawn_water,
                    stream::grow_the_grove,
                ),
            )
            .add_systems(
                Update,
                (
                    water::move_water,
                    stream::queue_chunks,
                    stream::collect_chunks,
                    stream::unload_chunks,
                    // After the ground, because cover can only be laid on a
                    // chunk that is already loaded.
                    cover::dress_chunks,
                    cover::collect_cover,
                    cover::undress_chunks,
                )
                    .chain(),
            );
    }
}
