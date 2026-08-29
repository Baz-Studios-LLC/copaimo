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
            .spawn((
                Chunk(coord),
                Transform::from_xyz(origin.x, 0.0, origin.y),
                // Spawned WITH visibility, not just a transform: four kinds of
                // children hang off a chunk — trees, cover, props, river — and
                // every one of them warned (B0004) that inherited visibility was
                // undefined because the parent had none.
                Visibility::default(),
            ))
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
    dressed: Query<(), Or<(With<super::cover::Cover>, With<super::prop::Props>)>>,
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

        // The wood and the water this chunk was carrying go first, and go
        // unconditionally: sculpting re-meshes a chunk, so this runs again for
        // ground that already had both, and clearing inside the planting instead
        // meant a chunk whose grove had not loaded yet kept its old surface and
        // gained a second one on top.
        //
        // # But NOT its grass and its litter
        //
        // Those took the same trip and it made the tool flicker: dragging the
        // brush re-meshes a chunk many times a second, and each pass swept the
        // cover and the props off the ground and left it bare until a background
        // task handed new ones back a frame or three later. Grass and rocks
        // strobing under the brush, reported exactly that way.
        //
        // They are left standing on the old ground instead, and swapped for the
        // new ones when those arrive — the same bargain the chunk's own mesh
        // already makes, and for the same reason. Wrong by a few centimetres for
        // a moment beats absent.
        if let Some(standing) = standing {
            for old in standing.iter() {
                if !dressed.contains(old) {
                    commands.entity(old).despawn();
                }
            }
        }
        // The chunk forgets that it is dressed, so the ground it now has gets
        // dressed again. What is standing there in the meantime is the OLD
        // dressing, which `collect_cover` and `collect_props` clear as they put
        // the new down.
        commands
            .entity(entity)
            .remove::<(super::cover::HasCover, super::prop::HasProps)>();

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
                    Timber,
                    Mesh3d(variety.wood.clone()),
                    MeshMaterial3d(variety.bark.clone()),
                    stance,
                ))
                .with_children(|trunk| {
                    trunk.spawn((
                        Timber,
                        Mesh3d(variety.leaves.clone()),
                        MeshMaterial3d(variety.leaf.clone()),
                        Transform::IDENTITY,
                    ));
                });
        });
    }
    // Freshly planted wood casts until the gate looks at it again — see
    // `shade_far_wood`. Without this, a far chunk re-planted by the brush would
    // keep the chunk-level record while its new trees missed the component the
    // record stands for.
    commands.entity(entity).remove::<CastsNoShade>();
}

/// A tree's drawn body — the wood or the leaves — so the shadow gate can find
/// them without caring what else hangs off a chunk.
#[derive(Component)]
pub struct Timber;

/// A chunk whose wood has been told not to cast, so the gate only walks a
/// chunk's trees when it CROSSES the ring rather than every frame.
#[derive(Component)]
pub struct CastsNoShade;

/// How far out a tree still casts a shadow, in chunks from the viewer.
///
/// The cascades were measured at 16.7 ms of a 23.8 ms frame, and nearly all of it
/// was re-submitting every tree in the streamed disc to all three of them — the
/// disc holds ~254 chunks, and a ring keeps a handful.
///
/// # It has to end where the SHADOWS end, not sooner
///
/// This was two chunks, and two chunks is 256 m against a `SHADOW_DISTANCE` of
/// 400 — so a tree's shadow blinked out well inside the range where every other
/// shadow in the world was still being drawn, and walking toward a wood switched
/// its shadows on a ring at a time. Reported as trees popping, and rightly.
///
/// Three chunks is 384 m: just inside the shadow distance, where the last cascade
/// is already giving out anyway, so there is only ONE place in the world where
/// shadows stop instead of two.
const SHADOW_CHUNKS: i32 = 3;

/// Parks and wakes tree shadows as chunks cross the shadow ring.
///
/// Chunk-level bookkeeping, deliberately: the naive version asks every tree in
/// every loaded chunk every frame, which is tens of thousands of entity lookups
/// to conclude nothing changed. A chunk remembers which side of the ring it is
/// on, and its trees are only walked on the frame it crosses — with a chunk of
/// hysteresis so standing on a boundary does not flip the same ring every step.
pub fn shade_far_wood(
    mut commands: Commands,
    map: Res<ChunkMap>,
    anchors: Query<&GlobalTransform, With<StreamAnchor>>,
    gated: Query<(), With<CastsNoShade>>,
    children: Query<&Children>,
    timber: Query<(), With<Timber>>,
) {
    let Some(anchor) = anchors.iter().next() else {
        return;
    };
    let middle = chunk_at(anchor.translation());

    for (&coord, &entity) in &map.loaded {
        let away = (coord - middle).abs().max_element();
        let is_parked = gated.contains(entity);
        let wants_parking = if is_parked {
            away > SHADOW_CHUNKS
        } else {
            away > SHADOW_CHUNKS + 1
        };
        if wants_parking == is_parked {
            continue;
        }

        // The chunk's trees: wood as children, leaves as grandchildren.
        let Ok(kids) = children.get(entity) else {
            // Nothing planted yet. Leave the record unset so the walk happens
            // once the trees exist.
            continue;
        };
        for kid in kids.iter() {
            let mut toggle = |body: Entity| {
                if wants_parking {
                    commands.entity(body).insert(bevy::pbr::NotShadowCaster);
                } else {
                    commands.entity(body).remove::<bevy::pbr::NotShadowCaster>();
                }
            };
            if timber.contains(kid) {
                toggle(kid);
            }
            if let Ok(grandkids) = children.get(kid) {
                for grandkid in grandkids.iter() {
                    if timber.contains(grandkid) {
                        toggle(grandkid);
                    }
                }
            }
        }
        if wants_parking {
            commands.entity(entity).insert(CastsNoShade);
        } else {
            commands.entity(entity).remove::<CastsNoShade>();
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
    /// Wood, leaves, and the green this variety wears.
    pub trees: Vec<Variety>,

}

pub struct Variety {
    pub wood: Handle<Mesh>,
    pub leaves: Handle<Mesh>,
    pub leaf: Handle<Shaded>,
    pub bark: Handle<Shaded>,
    /// How thick this variety's trunk is at the foot, in its own units.
    ///
    /// MEASURED off the wood the tree is actually drawn with rather than taken from
    /// a number beside it — the two cannot disagree if there is only one of them,
    /// and what a warden walks into is the mesh. See `trunk_radius`.
    pub trunk: f32,
    /// Where this variety sits in the leaf range, 0 dark to 1 light.
    ///
    /// Kept because the SEASON re-derives the colour from it: the range moves with
    /// the year and each variety keeps its place in it, which is what makes an
    /// autumn wood as mottled as the summer one was. Without this the colour would
    /// be a number computed once at startup and thrown away.
    pub tint: f32,
    /// What kind of tree this variety IS.
    ///
    /// Recorded because the pool's own layout — grouped by species, four variants
    /// apiece — is `terrain_core`'s business and not something to be re-derived
    /// here. An authored shape has to land on the variety whose COLOUR belongs to
    /// the same species, and the first cut worked that out with a modulo: it put
    /// a birch's chalk-pale trunk under an oak's crown, all over the world.
    pub species: terrain_core::tree::Species,
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
/// The palest bark in the world, which is a birch.
///
/// Warmed and brought down from (0.82, 0.80, 0.74). That was very nearly white,
/// and a near-white matte surface under an open sky takes its colour from the sky:
/// it came out cool grey — concrete rather than bark — even though the numbers
/// themselves are warm. Bark is never brighter than the grass it stands in.
const BARK_PALE: Srgba = Srgba::rgb(0.74, 0.69, 0.58);

/// The green a tree wears, from where it sits in the leaf range.
fn leaf_colour(tint: f32) -> Vec4 {
    LinearRgba::from(LEAF_DARK)
        .to_vec4()
        .lerp(LinearRgba::from(LEAF_LIGHT).to_vec4(), tint)
}

/// The colour of a trunk, from where its species sits in the bark range.
///
/// # Cubed, not squared, and not straight
///
/// Straight, the ramp put every species in the MIDDLE of brown-to-chalk, and the
/// middle of brown-to-chalk is grey: a whole wood the colour of concrete. Squaring
/// fixed most of that. Cubing finishes it — the species that should be brown are
/// pushed further down the ramp, and a birch, which draws 0.86 and up, still lands
/// near the pale end where it belongs.
///
/// The numbers this has to keep apart, from `terrain_core`: spruce draws
/// 0.12–0.26, acacia 0.28–0.44, oak 0.30–0.48, pine 0.44–0.62, birch 0.86–1.0.
/// Cubed, pine's palest is 0.24 and birch's darkest is 0.64 — a gap wide enough
/// that no amount of variation blurs a pine into a birch.
///
/// Worth saying plainly, because it was misread once: the wood full of pale trunks
/// was NOT this. Authored shapes were being matched to varieties by position
/// instead of by species, so birch-coloured trunks were wearing oak crowns. The
/// palette only ever needed warming.
fn bark_colour(bark: f32) -> Vec4 {
    let pale = bark * bark * bark;
    LinearRgba::from(BARK_DARK)
        .to_vec4()
        .lerp(LinearRgba::from(BARK_PALE).to_vec4(), pale)
}

/// How wide a trunk is at the foot, measured from the wood itself.
///
/// # Why the mesh and not a constant
///
/// A collider written down beside a shape is a second description of it, and two
/// descriptions drift: change the tree and the thing a warden bumps into stays
/// where the old one was. This asks the geometry that is actually drawn.
///
/// The lowest fifth of the tree, because that is what a person walks into. Higher
/// up the question is meaningless — an oak's crown is metres across and nobody
/// collides with a canopy — and taking the whole mesh's extent would put an
/// invisible wall out at the drip line.
fn trunk_radius(wood: &terrain_core::Geometry) -> f32 {
    let floor = wood
        .places
        .iter()
        .fold(f32::MAX, |low, place| low.min(place[1]));
    let bole = wood
        .places
        .iter()
        .filter(|place| place[1] <= floor + BASE_RING)
        .map(|place| (place[0] * place[0] + place[2] * place[2]).sqrt())
        // Not the axis itself: a trunk is capped and the cap has a centre vertex
        // sitting at nought, which is not a radius.
        .filter(|radius| *radius > 0.02)
        .fold(f32::MAX, f32::min);
    if bole == f32::MAX { 0.0 } else { bole }
}

/// How thick a slice at the foot of a tree counts as its base ring, in metres.
///
/// # Four answers, and the three the measurement threw out
///
/// The wood mesh is trunk AND branches, and several species reach the ground with
/// something that is not a trunk - a spruce carries limbs down, a willow hangs
/// withes to the floor. `player::what_the_trunks_measure` prints what each attempt
/// actually read, and it refused three in a row:
///
///     WIDEST in the bottom fifth      oak 3.65 m, willow 3.80 - that is a bough,
///                                     and it stops a warden in open air three
///                                     metres from a tree he is nowhere near
///     TENTH PERCENTILE of the same    fixed the oak and the spruce; willow still
///                                     read 3.80, because nearly everything low on
///                                     a willow is hanging withe and no percentile
///                                     of it lands on the bole
///     NEAREST THE AXIS, bottom fifth  0.022 m - a fifth of a twenty-metre tree is
///                                     four metres up, where the trunk has tapered
///                                     and branches attach close in
///
/// What is true of a trunk and of nothing else is that at the FOOT it is the only
/// thing there, and everything else in the mesh grows outward from it. So: the ring
/// of vertices nearest the ground, and of those the one closest to the axis. Every
/// species in the pool then measures between 0.14 m and 0.61 m, which is a tree.
///
/// A third of a metre of slice, because the trunk is drawn in six segments and a
/// tall tree's first ring can sit two metres up - too thin a slice and there is
/// nothing in it at all, which is how the third attempt above managed to return
/// infinity for every spruce.
const BASE_RING: f32 = 0.35;

/// Grows the world's trees once, at startup.
pub fn grow_the_grove(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<Shaded>>,
) {
    let trees = (0..terrain_core::tree::VARIETIES as u32)
        .map(|seed| {
            let tree = terrain_core::tree::grow(seed);
            let green = leaf_colour(tree.tint);
            let wood_colour = bark_colour(tree.bark);
            Variety {
                species: tree.species,
                tint: tree.tint,
                trunk: trunk_radius(&tree.wood),
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

/// The same again, taking the geometry rather than borrowing it.
///
/// # Two versions, because there are two situations
///
/// A finished cover or prop mesh is owned by the system integrating it and dropped
/// the moment it has been converted, so cloning five vectors out of it and then
/// throwing the originals away doubles the memory traffic of the one part of chunk
/// streaming that happens ON THE FRAME. A welded meadow is the heaviest mesh in the
/// world by vertex count, and that copy lands in the same frame as the asset upload.
///
/// The borrowing version stays for the callers that need their geometry afterwards.
///
/// Found by Codex's audit.
pub fn into_coloured_mesh(geometry: terrain_core::Geometry) -> Mesh {
    let terrain_core::Geometry {
        places,
        normals,
        uvs,
        colours,
        indices,
    } = geometry;
    let mesh = Mesh::new(
        bevy::render::mesh::PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, places)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(bevy::render::mesh::Indices::U32(indices));
    if colours.is_empty() {
        return mesh;
    }
    mesh.with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colours)
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

#[cfg(test)]
mod palette {
    use super::*;
    use terrain_core::tree::Species;

    /// A brown-barked tree reads brown, and only a birch goes pale.
    ///
    /// The fault this guards is not a crash: it is a wood that comes out the
    /// colour of concrete, which nobody can call a bug from a screenshot without
    /// arguing about it. So the claim is stated in numbers — how much warmer than
    /// blue a trunk is, and how far apart the palest brown tree and the darkest
    /// birch sit on the ramp.
    #[test]
    fn brown_barked_trees_read_brown_and_only_a_birch_goes_pale() {
        let warmth = |bark: f32| {
            let colour = bark_colour(bark);
            colour.x - colour.z
        };
        let lightness = |bark: f32| bark_colour(bark).x;

        // The ranges each species draws from, out of `terrain_core`.
        let brown = [
            ("spruce", 0.12, 0.26),
            ("acacia", 0.28, 0.44),
            ("oak", 0.30, 0.48),
            ("pine", 0.44, 0.62),
        ];
        for (name, low, high) in brown {
            for bark in [low, (low + high) * 0.5, high] {
                assert!(
                    warmth(bark) > 0.0,
                    "{name} at {bark} is not warmer than it is blue"
                );
                assert!(
                    lightness(bark) < 0.35,
                    "{name} at {bark} reads at {:.2} — that is stone, not bark",
                    lightness(bark)
                );
            }
        }

        // A birch is the pale one, and the gap is wide.
        let palest_brown = lightness(0.62);
        let darkest_birch = lightness(0.86);
        assert!(
            darkest_birch > palest_brown * 2.0,
            "the darkest birch reads {darkest_birch:.2} and the palest brown tree \
             {palest_brown:.2} — too close to tell apart"
        );
        // And still bark rather than paper: warm, and no brighter than grass.
        assert!(warmth(1.0) > 0.05, "the palest bark has gone colourless");
        assert!(lightness(1.0) < 0.8, "the palest bark is brighter than the world");

        // Every species the world actually grows lands inside the claim above.
        for species in Species::ALL {
            let tree = terrain_core::tree::grow_as(species, 0);
            let light = lightness(tree.bark);
            assert!(
                (0.0..0.8).contains(&light),
                "{species:?} draws bark {:.2}, which paints {light:.2}",
                tree.bark
            );
        }
    }
}
