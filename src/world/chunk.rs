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

use crate::config::{CHUNK_QUADS, CHUNK_SIZE, RIVER_QUADS};
use crate::shade::{shaded, Shaded};
use crate::world::biome::surface_color;
use crate::world::terrain::Terrain;

/// Marks a spawned terrain chunk and records which grid cell it covers.
///
/// The coordinate is carried on the entity as well as in `ChunkMap` because
/// per-chunk world content — trees, rocks, later encounters — will be spawned
/// as children of the chunk and needs to know where it is.
#[derive(Component)]
pub struct Chunk(pub IVec2);

/// One shared material for every chunk — all the color variety comes from
/// vertex colors, so there is no reason to allocate a material per chunk.
#[derive(Resource, Deref)]
pub struct TerrainMaterial(pub Handle<Shaded>);

pub fn setup_material(mut commands: Commands, mut materials: ResMut<Assets<Shaded>>) {
    let handle = materials.add(shaded(StandardMaterial {
        // White base: the PBR shader multiplies this by the mesh's vertex
        // colors, so leaving it white lets the biome palette come through
        // exactly as authored.
        base_color: Color::WHITE,
        perceptual_roughness: 0.94,
        reflectance: 0.06,
        ..default()
    }));
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
/// The ground of one chunk, and the still water standing in its channels.
///
/// Both in one pass because the water needs the same heights the ground does,
/// and those are the expensive part — a second pass would sample the whole world
/// twice to draw a river that covers a hundredth of it.
pub fn build_chunk(terrain: &Terrain, coord: IVec2) -> (Mesh, Option<Mesh>) {
    let ground = build_mesh(terrain, coord);
    (ground, build_river(terrain, coord))
}

/// The surface of any river crossing this chunk.
///
/// A quad is drawn only where all four of its corners stand in water, so a river
/// ends at its bank rather than in a fringe of triangles poking out of the grass.
/// Rivers do not flow — there is nothing that would read a current — so this is a
/// surface at a height, exactly as the sea is.
fn build_river(terrain: &Terrain, coord: IVec2) -> Option<Mesh> {
    let quads = RIVER_QUADS as usize;
    let side = quads + 1;
    let step = CHUNK_SIZE / RIVER_QUADS as f32;
    let origin = chunk_origin(coord);

    let mut surface = vec![None; side * side];
    let mut wet = false;
    for iz in 0..side {
        for ix in 0..side {
            let world = origin + Vec2::new(ix as f32, iz as f32) * step;
            surface[iz * side + ix] = terrain.river_surface(world.x, world.y);
            wet |= surface[iz * side + ix].is_some();
        }
    }
    if !wet {
        return None;
    }

    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    for iz in 0..quads {
        for ix in 0..quads {
            let corners = [
                (ix, iz),
                (ix + 1, iz),
                (ix + 1, iz + 1),
                (ix, iz + 1),
            ];
            // All four, or none. A partly wet quad is a bank.
            let Some(heights) = corners
                .iter()
                .map(|(x, z)| surface[z * side + x])
                .collect::<Option<Vec<f32>>>()
            else {
                continue;
            };

            let base = positions.len() as u32;
            for ((x, z), height) in corners.iter().zip(&heights) {
                positions.push([*x as f32 * step, *height, *z as f32 * step]);
                normals.push([0.0, 1.0, 0.0]);
                uvs.push([*x as f32 / quads as f32, *z as f32 / quads as f32]);
            }
            indices.extend_from_slice(&[base, base + 3, base + 1, base + 1, base + 3, base + 2]);
        }
    }
    if indices.is_empty() {
        return None;
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    Some(mesh)
}

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

    // Heights on a grid one vertex wider than the chunk on every side.
    //
    // The extra ring is what lets a normal be a central difference over heights
    // already asked for, instead of four MORE terrain evaluations per vertex.
    // Each one runs the map lookup, the domain warp and half a dozen octaves of
    // noise, so the old five-samples-per-vertex made chunk building four times
    // the work it needed to be: 21,125 evaluations a chunk, against 4,489 now.
    //
    // Still seamless, which is the property that matters. A normal here is a
    // function of world position and the fixed step alone — never of which chunk
    // is asking — so two chunks meeting at a vertex compute the same one.
    let padded = side + 2;
    let mut heights = vec![0.0_f32; padded * padded];
    for pz in 0..padded {
        for px in 0..padded {
            let world = origin + Vec2::new(px as f32 - 1.0, pz as f32 - 1.0) * step;
            heights[pz * padded + px] = terrain.height(world.x, world.y);
        }
    }
    let sampled = |px: usize, pz: usize| heights[pz * padded + px];

    for iz in 0..side {
        for ix in 0..side {
            let local = Vec2::new(ix as f32 * step, iz as f32 * step);
            let world = origin + local;
            // The vertex's own place in the padded grid, one in from its edge.
            let (px, pz) = (ix + 1, iz + 1);

            let height = sampled(px, pz);
            // Over a whole grid step rather than the half-step the old analytic
            // form used. A full step is what the mesh's own facets span, so this
            // is the slope the surface actually has rather than one sampled
            // finer than anything drawn can show.
            let slope_x = sampled(px + 1, pz) - sampled(px - 1, pz);
            let slope_z = sampled(px, pz + 1) - sampled(px, pz - 1);
            let normal = Vec3::new(-slope_x, 2.0 * step, -slope_z).normalize();
            let slope = 1.0 - normal.y;
            let character = terrain.shore_character(world.x, world.y);
            let worn = terrain.worn(world.x, world.y);

            positions.push([local.x, height, local.y]);
            normals.push([normal.x, normal.y, normal.z]);
            let (country, belonging) = terrain.region(world.x, world.y);
            colors.push(surface_color(
                world,
                height,
                slope,
                character,
                worn,
                country,
                belonging,
                terrain.in_a_cutting(world.x, world.y),
            ));
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
