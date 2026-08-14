//! Builds the mesh for a single terrain chunk.
//!
//! A chunk is a `CHUNK_SIZE` × `CHUNK_SIZE` grid of quads sampled from the
//! terrain heightfield. Vertex positions are chunk-local (the entity's
//! `Transform` places it in the world), which keeps coordinates small and
//! precise even out at the far corners of an 8 km map.
//!
//! Chunks stitch seamlessly because both the height *and* the normal at any
//! point depend only on world coordinates — two neighbors sampling their shared
//! edge get bit-identical answers, so there is no crack and no lighting seam.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};

use crate::config::{CHUNK_QUADS, CHUNK_SIZE};
use crate::world::biome::surface_color;
use crate::world::terrain::Terrain;

/// Marks a spawned terrain chunk and records which grid cell it covers.
///
/// The coordinate is carried on the entity as well as in `ChunkMap` because
/// per-chunk world content — trees, rocks, later encounters — will be spawned
/// as children of the chunk and needs to know where it is.
#[derive(Component)]
pub struct Chunk(#[allow(dead_code)] pub IVec2);

/// One shared material for every chunk — all the color variety comes from
/// vertex colors, so there is no reason to allocate a material per chunk.
#[derive(Resource, Deref)]
pub struct TerrainMaterial(pub Handle<StandardMaterial>);

pub fn setup_material(mut commands: Commands, mut materials: ResMut<Assets<StandardMaterial>>) {
    let handle = materials.add(StandardMaterial {
        // White base: the PBR shader multiplies this by the mesh's vertex
        // colors, so leaving it white lets the biome palette come through
        // exactly as authored.
        base_color: Color::WHITE,
        perceptual_roughness: 0.94,
        reflectance: 0.06,
        ..default()
    });
    commands.insert_resource(TerrainMaterial(handle));
}

/// World-space origin (north-west corner) of a chunk.
pub fn chunk_origin(coord: IVec2) -> Vec2 {
    Vec2::new(coord.x as f32, coord.y as f32) * CHUNK_SIZE
}

/// Which chunk a world position falls in.
pub fn chunk_at(position: Vec3) -> IVec2 {
    IVec2::new(
        (position.x / CHUNK_SIZE).floor() as i32,
        (position.z / CHUNK_SIZE).floor() as i32,
    )
}

/// Samples the terrain and produces the chunk's mesh. Pure and thread-safe, so
/// this runs on a background task rather than blocking the frame.
pub fn build_mesh(terrain: &Terrain, coord: IVec2) -> Mesh {
    let quads = CHUNK_QUADS as usize;
    let side = quads + 1;
    let step = CHUNK_SIZE / CHUNK_QUADS as f32;
    let origin = chunk_origin(coord);

    let count = side * side;
    let mut positions = Vec::with_capacity(count);
    let mut normals = Vec::with_capacity(count);
    let mut colors = Vec::with_capacity(count);
    let mut uvs = Vec::with_capacity(count);

    for iz in 0..side {
        for ix in 0..side {
            let local = Vec2::new(ix as f32 * step, iz as f32 * step);
            let world = origin + local;

            let height = terrain.height(world.x, world.y);
            // Half a grid cell is the right epsilon here: fine enough to catch
            // the detail the mesh can actually represent, coarse enough not to
            // amplify noise the vertices don't sample.
            let normal = terrain.normal(world.x, world.y, step * 0.5);
            let slope = 1.0 - normal.y;
            let moisture = terrain.moisture(world.x, world.y);
            let character = terrain.shore_character(world.x, world.y);

            positions.push([local.x, height, local.y]);
            normals.push([normal.x, normal.y, normal.z]);
            colors.push(surface_color(height, slope, moisture, character));
            uvs.push([ix as f32 / quads as f32, iz as f32 / quads as f32]);
        }
    }

    // Two triangles per quad, wound counter-clockwise seen from above so they
    // face +Y and survive backface culling.
    let mut indices = Vec::with_capacity(quads * quads * 6);
    for iz in 0..quads {
        for ix in 0..quads {
            let a = (iz * side + ix) as u32;
            let b = a + 1;
            let c = a + side as u32;
            let d = c + 1;
            indices.extend_from_slice(&[a, c, b, b, c, d]);
        }
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        // Chunks are drawn but never read back, and dropping the CPU copy after
        // upload keeps memory flat while streaming hundreds of them.
        RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}
