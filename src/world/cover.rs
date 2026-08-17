//! Ground cover: the grass and flowers a chunk is dressed with.
//!
//! Where it grows and what it looks like belong to [`terrain_core::cover`], which
//! the bench runs too. What is here is the part that is this game's own: which
//! chunks are close enough to be worth dressing, building their cover off the
//! main thread, and taking it away again when the viewer walks off.
//!
//! # A detail radius, not the streaming radius
//!
//! Terrain streams out to `VIEW_CHUNKS` — about 1150 m — because that is the
//! horizon. Cover cannot: a chunk holds two thousand tufts, and dressing the
//! whole streamed world would be seven hundred thousand of them. It is also
//! pointless, since a 40 cm blade of grass is invisible past a hundred metres or
//! so. So cover has a radius of its own, a few chunks wide, and follows the
//! viewer within the ground that is already loaded.
//!
//! # Built on a thread, like the ground under it
//!
//! Dressing a chunk asks the world what kind of place each slot is, which is a
//! handful of noise evaluations apiece — several times the cost of the chunk's
//! own mesh. Doing that on the main thread is a stutter every time you cross a
//! boundary, so it goes to the task pool exactly as chunk meshing does.

use bevy::prelude::*;
use bevy::tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, Task};

use terrain_core::cover::{self as sprigs, Sprig};
use terrain_core::Geometry;

use crate::config::{CHUNK_SIZE, COVER_CHUNKS, MAX_PENDING_COVER};
use crate::shade::{shaded, Shaded};
use crate::world::chunk::{chunk_at, chunk_origin, Chunk};
use crate::world::stream::{as_coloured_mesh, ChunkMap};
use crate::world::terrain::{Biome, Terrain, TerrainSource};
use crate::world::StreamAnchor;

/// The cover standing on one chunk. A child of it, so it moves and dies with it.
#[derive(Component)]
pub struct Cover;

/// Cover being built for a chunk on a background thread.
#[derive(Component)]
pub struct PendingCover(Task<Geometry>);

/// The one material every tuft in the world wears.
#[derive(Resource, Deref)]
pub struct CoverMaterial(pub Handle<Shaded>);

pub fn setup_material(mut commands: Commands, mut materials: ResMut<Assets<Shaded>>) {
    let handle = materials.add(shaded(StandardMaterial {
        // White, so the greens the crate baked into the vertices come through
        // exactly as mixed — the same bargain the terrain makes with its biome
        // colours.
        base_color: Color::WHITE,
        perceptual_roughness: 0.93,
        reflectance: 0.02,
        // A blade is a single triangle wound both ways, so there is no back to
        // cull; turning culling off is the honest description of that.
        double_sided: true,
        cull_mode: None,
        ..default()
    }));
    commands.insert_resource(CoverMaterial(handle));
}

/// Starts building cover for loaded chunks near the viewer that have none.
pub fn dress_chunks(
    mut commands: Commands,
    terrain: Res<TerrainSource>,
    chunks: Res<ChunkMap>,
    anchors: Query<&GlobalTransform, With<StreamAnchor>>,
    dressed: Query<(), Or<(With<Cover>, With<PendingCover>)>>,
    children: Query<Option<&Children>, With<Chunk>>,
    busy: Query<(), With<PendingCover>>,
) {
    let Some(anchor) = anchors.iter().next() else {
        return;
    };
    // Capped like chunk meshing is: crossing a boundary must not queue a hundred
    // of these at once.
    let mut room = MAX_PENDING_COVER.saturating_sub(busy.iter().count());
    if room == 0 {
        return;
    }

    let middle = chunk_at(anchor.translation());
    let pool = AsyncComputeTaskPool::get();

    for step_z in -COVER_CHUNKS..=COVER_CHUNKS {
        for step_x in -COVER_CHUNKS..=COVER_CHUNKS {
            if room == 0 {
                return;
            }
            let coord = middle + IVec2::new(step_x, step_z);
            let Some(&entity) = chunks.loaded.get(&coord) else {
                continue;
            };
            // Already dressed, or already being dressed.
            let has_cover = children
                .get(entity)
                .ok()
                .flatten()
                .is_some_and(|kids| kids.iter().any(|kid| dressed.contains(kid)));
            if has_cover || dressed.contains(entity) {
                continue;
            }

            let ground = terrain.0.clone();
            let low = chunk_origin(coord);
            let task = pool.spawn(async move { dress(&ground, low) });
            commands.entity(entity).insert(PendingCover(task));
            room -= 1;
        }
    }
}

/// Attaches finished cover to its chunk.
pub fn collect_cover(
    mut commands: Commands,
    material: Option<Res<CoverMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut pending: Query<(Entity, &mut PendingCover)>,
) {
    let Some(material) = material else {
        return;
    };
    for (entity, mut task) in &mut pending {
        let Some(cover) = block_on(future::poll_once(&mut task.0)) else {
            continue;
        };
        commands.entity(entity).remove::<PendingCover>();
        if cover.is_empty() {
            // Rock, water, a levelled town — nothing grows and nothing is spawned
            // rather than an empty mesh being kept about.
            continue;
        }
        commands.entity(entity).with_children(|chunk| {
            chunk.spawn((
                Cover,
                Mesh3d(meshes.add(as_coloured_mesh(&cover))),
                MeshMaterial3d(material.0.clone()),
                // Chunk-local, like the ground's own vertices.
                Transform::IDENTITY,
                // Grass casts no shadow, and that is what pays for there being
                // this much of it.
                //
                // A meadow is by far the heaviest thing in the world by triangle
                // count, and every caster is submitted again for each of the four
                // shadow cascades — so a chunk of grass is drawn five times to
                // show a smudge under something a hand tall. It still RECEIVES:
                // grass in a tree's shadow or under a cloud goes dark with the
                // ground it stands on, which is the part anybody can see.
                bevy::pbr::NotShadowCaster,
            ));
        });
    }
}

/// Takes cover off chunks the viewer has walked away from.
pub fn undress_chunks(
    mut commands: Commands,
    chunks: Res<ChunkMap>,
    anchors: Query<&GlobalTransform, With<StreamAnchor>>,
    children: Query<&Children>,
    cover: Query<(), With<Cover>>,
) {
    let Some(anchor) = anchors.iter().next() else {
        return;
    };
    let middle = chunk_at(anchor.translation());
    // One chunk of slack past the dressing radius, so standing on a boundary
    // does not dress and undress the same ring every other frame.
    let keep = COVER_CHUNKS + 1;

    for (&coord, &entity) in &chunks.loaded {
        let away = (coord - middle).abs();
        if away.x <= keep && away.y <= keep {
            continue;
        }
        let Ok(kids) = children.get(entity) else {
            continue;
        };
        for kid in kids.iter() {
            if cover.contains(kid) {
                commands.entity(kid).despawn();
            }
        }
        // A chunk that is being dressed and has gone out of range should stop.
        commands.entity(entity).remove::<PendingCover>();
    }
}

/// Builds every tuft standing on one chunk, welded into a single mesh.
///
/// Pure and thread-safe: it asks the terrain questions and appends geometry, and
/// touches nothing else. That is what lets it run on the task pool.
fn dress(terrain: &Terrain, low: Vec2) -> Geometry {
    let high = low + CHUNK_SIZE;
    let step = sprigs::SPACING.max(0.5);
    let climate = terrain.climate();

    // A world-wide lattice rather than a per-chunk one, so a tuft does not move
    // when the chunk boundaries around it change — the same rule the woods keep.
    let first = (low / step).floor().as_ivec2();
    let last = (high / step).ceil().as_ivec2();

    let mut mesh = Geometry::default();
    for slot_z in first.y..=last.y {
        for slot_x in first.x..=last.x {
            // Jittered off the lattice, or a meadow comes out in rows.
            let jitter = Vec2::new(
                sprigs::chance(slot_x, slot_z, sprigs::SALT_JITTER_X) - 0.5,
                sprigs::chance(slot_x, slot_z, sprigs::SALT_JITTER_Z) - 0.5,
            ) * step
                * 0.9;
            let at = Vec2::new(slot_x as f32 * step, slot_z as f32 * step) + jitter;
            if at.x < low.x || at.x >= high.x || at.y < low.y || at.y >= high.y {
                continue;
            }

            let ground = terrain.ground_at(at.x, at.y);
            let biome = Biome::of(ground, &climate);
            let sureness = Biome::confidence(ground, &climate);
            // How deep into a meadow this slot is. Asked once and used twice:
            // it decides both whether anything grows here and how big it gets,
            // and a patch that is denser without being taller reads as more of
            // the same rather than as a meadow.
            let patch = sprigs::patch(biome, at);
            let thickness = sprigs::density(biome, sureness, patch);
            if thickness <= 0.0
                || sprigs::chance(slot_x, slot_z, sprigs::SALT_PRESENT) > thickness
            {
                continue;
            }

            let kind = sprigs::kind(biome, sprigs::chance(slot_x, slot_z, sprigs::SALT_KIND));
            // Scrub sits lower and wider; grass on good ground stands up.
            let scale = match kind {
                Sprig::Scrub => 0.7,
                _ => 1.0,
            } * (0.7 + 0.6 * sprigs::chance(slot_x, slot_z, sprigs::SALT_SCALE))
                * sprigs::stature(patch);

            sprigs::add(
                &mut mesh,
                kind,
                // Chunk-local, and set on the ground's own surface.
                // The drawn surface, like the trees — a tuft floating a
                // handspring off the ground is as wrong as a tree doing it.
                Vec3::new(at.x - low.x, terrain.drawn_height(at.x, at.y), at.y - low.y),
                sprigs::chance(slot_x, slot_z, sprigs::SALT_TURN) * std::f32::consts::TAU,
                scale,
                sprigs::chance(slot_x, slot_z, sprigs::SALT_SHADE),
                sprigs::chance(slot_x, slot_z, sprigs::SALT_PETAL),
                // How deep into the thicket this one is: fuller, wider and
                // darker the further in, so a patch reads as a mass.
                patch,
            );
        }
    }
    mesh
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What a dressed chunk actually costs.
    ///
    /// This is the number that decides whether ground cover is affordable at all,
    /// and it is not guessable: it depends on the spacing, on how many blades a
    /// tuft carries, and on what the biome under a particular chunk turns out to
    /// be. So it is measured on the real world, at the ranch, where the ground is
    /// levelled grassland and the answer is close to the worst case.
    #[test]
    fn dressing_a_chunk_stays_affordable() {
        let terrain = Terrain::new();
        let climate = terrain.climate();

        // A chunk of open country, FOUND rather than assumed. The ranch chunk
        // itself is levelled, so it carries only trodden grass — and guessing at
        // an offset from it landed five hundred metres out to sea, because the
        // ranch is on a coast. The cost ceiling has to be measured where cover is
        // thickest, so the test goes looking for that ground.
        let from = Vec2::new(crate::config::RANCH_AT.0, crate::config::RANCH_AT.1);
        let middle = (from / CHUNK_SIZE).floor().as_ivec2();
        let coord = (0..12)
            .flat_map(|ring| {
                (-ring..=ring).flat_map(move |step_z| {
                    (-ring..=ring).map(move |step_x| IVec2::new(step_x, step_z))
                })
            })
            .map(|step| middle + step)
            .find(|coord| {
                let at = chunk_origin(*coord) + CHUNK_SIZE * 0.5;
                let ground = terrain.ground_at(at.x, at.y);
                matches!(Biome::of(ground, &climate), Biome::Grass | Biome::Forest)
            })
            .expect("the world should hold some open country near the ranch");

        let mesh = dress(&terrain, chunk_origin(coord));

        let vertices = mesh.places.len();
        println!("cover on open country: {vertices} vertices");

        // Every vertex needs its colour or the mesh is refused by the renderer.
        assert_eq!(mesh.colours.len(), vertices);

        // The ceiling that matters. Twenty-five chunks are dressed at once, so
        // this is a twenty-fifth of the budget for the whole of it.
        //
        // It was forty thousand, and that was set when grass still cast shadows —
        // when every one of these vertices was submitted five times over, once
        // for the main pass and again for each of the four cascades. It is drawn
        // once now, so the same frame buys a great deal more of it, and the extra
        // went into making the patches thick enough to lose a monster in.
        //
        // Eighty-five thousand is a little over two million vertices of grass on
        // screen, drawn once.
        //
        // Raised twice now, and each time on a measurement rather than on the
        // arithmetic. Going from 47,000 a chunk to 73,000 — half as much grass
        // again — moved the frame from 48.3 fps to 48.7 and the main pass from
        // 6.69 ms to 6.41: nothing, twice, in opposite directions. This machine
        // is not vertex-bound at this scale, and the honest reading of the number
        // is that grass is no longer where the frame goes.
        //
        // Which is exactly why the ceiling is kept rather than dropped. It is not
        // guarding the GPU any more, it is guarding against a change that
        // multiplies the count by ten without anybody noticing — SPACING is a
        // square law, and halving it quadruples this. If it trips, the lever is
        // COVER_CHUNKS or the crate's SPACING, in that order, and MEASURE before
        // moving this line again.
        assert!(
            vertices < 85_000,
            "a dressed chunk costs {vertices} vertices, which is too many"
        );
        // And it has to actually be growing something, or the test proves nothing.
        assert!(
            vertices > 500,
            "open country should carry cover, and this chunk has {vertices} vertices"
        );
    }
}
