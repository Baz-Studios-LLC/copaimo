//! The natural objects standing on a chunk: boulders, bushes, logs, the rest.
//!
//! What they look like and where they belong is [`terrain_core::prop`], which
//! the bench runs too. What is here is this game's own part: which chunks are
//! close enough to be worth littering, building them off the main thread, and
//! taking them away when the viewer walks off.
//!
//! This is deliberately the same shape as [`crate::world::cover`], down to the
//! system names, because it is the same problem — a per-chunk detail layer with
//! a radius of its own — and two solutions to one problem is one too many.
//!
//! # Welded, not planted
//!
//! Trees are spawned one entity apiece, because a tree wears a material for its
//! bark and another for its leaves and one mesh can only wear one. A prop
//! carries its colour in its vertices instead, which means a chunk's worth of
//! them can be stamped into a SINGLE mesh — fifty boulders, bushes and fallen
//! logs in one draw call rather than fifty.
//!
//! That matters more than it sounds. The frame is already spent on shadows, and
//! every caster is submitted again for every cascade; a hundred separate little
//! objects per chunk would be paid for four times over.
//!
//! # Its own radius, and a short one
//!
//! Terrain reaches the horizon at `VIEW_CHUNKS`. Props cannot and should not: a
//! metre-wide boulder is sub-pixel well before the horizon is, so drawing it out
//! there is work with nothing to show for it.

use std::sync::Arc;

use bevy::prelude::*;
use bevy::tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, Task};

use terrain_core::prop::{self, Prop};
use terrain_core::Geometry;

use crate::config::{CHUNK_SIZE, MAX_PENDING_PROPS, PROP_CHUNKS, PROP_SCALE, PROP_SPACING};
use crate::shade::{shaded, Shaded};
use crate::world::chunk::{chunk_at, chunk_origin, Chunk};
use crate::world::stream::{as_coloured_mesh, ChunkMap};
use crate::world::terrain::{Biome, Terrain, TerrainSource};
use crate::world::StreamAnchor;

/// The litter standing on one chunk. A child of it, so it moves and dies with it.
#[derive(Component)]
pub struct Props;

/// Litter being built for a chunk on a background thread.
#[derive(Component)]
pub struct PendingProps(Task<Geometry>);

/// The chunk's litter question has been ANSWERED — including "nothing stands
/// here". Same record, same reason as [`crate::world::cover::HasCover`]: a
/// barren chunk that records nothing is asked again every frame forever.
#[derive(Component)]
pub struct HasProps;

/// The grown pool, shared with every background thread that stamps from it.
///
/// Grown once. Two dozen objects is nothing to grow, and growing one per boulder
/// would be growing a boulder ten thousand times.
#[derive(Resource, Deref)]
pub struct PropPool(Arc<Vec<Prop>>);

/// The one material everything in the pool wears.
#[derive(Resource, Deref)]
pub struct PropMaterial(pub Handle<Shaded>);

pub fn setup_props(mut commands: Commands, mut materials: ResMut<Assets<Shaded>>) {
    commands.insert_resource(PropPool(Arc::new(
        (0..prop::VARIETIES).map(prop::from_pool).collect(),
    )));

    commands.insert_resource(PropMaterial(materials.add(shaded(StandardMaterial {
        // White, so the stone, bark, leaf and dead grass the crate baked into
        // the vertices come through exactly as mixed — the same bargain the
        // terrain and the ground cover both make.
        base_color: Color::WHITE,
        perceptual_roughness: 0.92,
        reflectance: 0.03,
        ..default()
    }))));
}

/// Starts building litter for loaded chunks near the viewer that have none.
pub fn litter_chunks(
    mut commands: Commands,
    terrain: Res<TerrainSource>,
    pool: Option<Res<PropPool>>,
    chunks: Res<ChunkMap>,
    anchors: Query<&GlobalTransform, With<StreamAnchor>>,
    answered: Query<(), Or<(With<HasProps>, With<PendingProps>)>>,
    busy: Query<(), With<PendingProps>>,
) {
    let (Some(anchor), Some(pool)) = (anchors.iter().next(), pool) else {
        return;
    };
    // Capped like chunk meshing and ground cover are: crossing a boundary must
    // not queue a hundred of these at once.
    let mut room = MAX_PENDING_PROPS.saturating_sub(busy.iter().count());
    if room == 0 {
        return;
    }

    let middle = chunk_at(anchor.translation());
    let threads = AsyncComputeTaskPool::get();

    for step_z in -PROP_CHUNKS..=PROP_CHUNKS {
        for step_x in -PROP_CHUNKS..=PROP_CHUNKS {
            if room == 0 {
                return;
            }
            let coord = middle + IVec2::new(step_x, step_z);
            let Some(&entity) = chunks.loaded.get(&coord) else {
                continue;
            };
            // Already answered, or already being asked — one component test on
            // the chunk, where this used to walk its children every frame.
            if answered.contains(entity) {
                continue;
            }

            let ground = terrain.0.clone();
            let grown = pool.0.clone();
            let low = chunk_origin(coord);
            let task = threads.spawn(async move { litter(&ground, &grown, low) });
            commands.entity(entity).insert(PendingProps(task));
            room -= 1;
        }
    }
}

/// Attaches finished litter to its chunk.
pub fn collect_props(
    mut commands: Commands,
    material: Option<Res<PropMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut pending: Query<(Entity, &mut PendingProps, Option<&Children>)>,
    standing: Query<(), With<Props>>,
) {
    let Some(material) = material else {
        return;
    };
    for (entity, mut task, littered) in &mut pending {
        let Some(litter) = block_on(future::poll_once(&mut task.0)) else {
            continue;
        };
        // The old litter goes as the new is put down — see `collect_cover`.
        if let Some(littered) = littered {
            for old in littered.iter() {
                if standing.contains(old) {
                    commands.entity(old).despawn();
                }
            }
        }
        // Recorded even when the answer is "nothing stands here" — see
        // `HasProps`.
        commands.entity(entity).remove::<PendingProps>().insert(HasProps);
        if litter.is_empty() {
            // Open water, or a town's levelled ground. Nothing is spawned rather
            // than an empty mesh being kept about.
            continue;
        }
        commands.entity(entity).with_children(|chunk| {
            chunk.spawn((
                Props,
                Mesh3d(meshes.add(as_coloured_mesh(&litter))),
                MeshMaterial3d(material.0.clone()),
                // Chunk-local, like the ground's own vertices.
                Transform::IDENTITY,
            ));
        });
    }
}

/// Takes litter off chunks the viewer has walked away from.
///
/// Iterates the LITTERED, not the loaded — the same inversion as
/// [`crate::world::cover::undress_chunks`], for the same reason: the dressed
/// ring is a few dozen entities where the loaded disc is a couple of hundred
/// chunks of children.
pub fn clear_chunks(
    mut commands: Commands,
    anchors: Query<&GlobalTransform, With<StreamAnchor>>,
    props: Query<(Entity, &ChildOf), With<Props>>,
    coords: Query<&Chunk>,
    answered: Query<(Entity, &Chunk), Or<(With<HasProps>, With<PendingProps>)>>,
) {
    let Some(anchor) = anchors.iter().next() else {
        return;
    };
    let middle = chunk_at(anchor.translation());
    // One chunk of slack past the littering radius, so standing on a boundary
    // does not build and throw away the same ring every other frame.
    let keep = PROP_CHUNKS + 1;
    let out = |coord: bevy::math::IVec2| {
        let away = (coord - middle).abs();
        away.x > keep || away.y > keep
    };

    for (entity, of) in &props {
        if coords.get(of.parent()).is_ok_and(|chunk| out(chunk.0)) {
            commands.entity(entity).despawn();
        }
    }
    for (entity, chunk) in &answered {
        if out(chunk.0) {
            commands.entity(entity).remove::<(HasProps, PendingProps)>();
        }
    }
}

/// Everything standing on one chunk, welded into a single mesh.
///
/// Pure and thread-safe: it asks the terrain questions and stamps geometry, and
/// touches nothing else. That is what lets it run on the task pool.
pub fn litter(terrain: &Terrain, pool: &[Prop], low: Vec2) -> Geometry {
    let high = low + CHUNK_SIZE;
    let step = PROP_SPACING.max(1.0);

    // A world-wide lattice rather than a per-chunk one, so a boulder does not
    // move when the chunk boundaries around it change — the same rule the woods
    // and the ground cover both keep.
    let first = (low / step).floor().as_ivec2();
    let last = (high / step).ceil().as_ivec2();

    let mut mesh = Geometry::default();
    for slot_z in first.y..=last.y {
        for slot_x in first.x..=last.x {
            // Jittered off the lattice, or a hillside comes out in rows.
            let jitter = Vec2::new(
                terrain_core::forest::chance(slot_x, slot_z, 41) - 0.5,
                terrain_core::forest::chance(slot_x, slot_z, 42) - 0.5,
            ) * step
                * 0.9;
            let at = Vec2::new(slot_x as f32 * step, slot_z as f32 * step) + jitter;
            if at.x < low.x || at.x >= high.x || at.y < low.y || at.y >= high.y {
                continue;
            }

            // Nothing stands under a mountain either.
            if crate::world::pass::underground(at) > 0.5
                || terrain
                    .bores()
                    .read()
                    .is_ok_and(|bores| bores.under_rock(at, terrain.unbored(at.x, at.y)) > 0.5)
            {
                continue;
            }
            let ground = terrain.ground_at(at.x, at.y);
            let biome = Biome::of(ground, &terrain.climate());

            let thickness = prop::density(biome);
            if thickness <= 0.0 || terrain_core::forest::chance(slot_x, slot_z, 43) > thickness {
                continue;
            }
            let Some(variety) = prop::pick(
                biome,
                terrain_core::forest::chance(slot_x, slot_z, 44),
                terrain_core::forest::chance(slot_x, slot_z, 45),
            ) else {
                continue;
            };
            let Some(grown) = pool.get(variety) else {
                continue;
            };

            let turn = terrain_core::forest::chance(slot_x, slot_z, 46) * std::f32::consts::TAU;
            let scale = PROP_SCALE.0
                + (PROP_SCALE.1 - PROP_SCALE.0) * terrain_core::forest::chance(slot_x, slot_z, 47);

            mesh.stamp(
                &grown.mesh,
                Vec3::new(
                    at.x - low.x,
                    bedded(terrain, at, grown.reach * scale),
                    at.y - low.y,
                ),
                turn,
                scale,
            );
        }
    }
    mesh
}

/// The height to stand something of a given size at, so it sits IN the ground
/// rather than on one point of it.
///
/// # A boulder on a dune was floating
///
/// On the DRAWN surface, like the trees and the grass — the chunk mesh is a grid
/// of flat triangles and on bulging ground it sits below the true height, so
/// anything stood at the true height stands off the ground it is meant to be
/// sitting on. That was the first half of it and it was already fixed.
///
/// The other half is that a prop is RIGID and the ground is not flat. Stood at
/// the height under its middle, a four-metre boulder on a dune crest has its whole
/// rim hanging over ground that falls away underneath — which is exactly what a
/// crest does. The picture was unmistakable: a rock with daylight under both
/// sides of it and its shadow on the sand below.
///
/// So the ground is asked around the thing's own footprint and the LOWEST answer
/// wins, then it is pressed a little further in. A boulder is half-buried and a
/// bush grows out of the soil; nothing in a landscape balances on the one point
/// directly beneath its middle.
fn bedded(terrain: &Terrain, at: Vec2, reach: f32) -> f32 {
    let mut lowest = terrain.drawn_height(at.x, at.y);
    for step in 0..AROUND {
        let turn = step as f32 / AROUND as f32 * std::f32::consts::TAU;
        // Most of the way out rather than all of it: the very rim of a prop's
        // reach is its widest lump at its widest point, which is not what rests on
        // the ground.
        let round = at + Vec2::new(turn.cos(), turn.sin()) * reach * 0.75;
        lowest = lowest.min(terrain.drawn_height(round.x, round.y));
    }
    lowest - reach * BEDDED
}

/// How many places around a prop the ground is asked about, and how far into it
/// the thing is then pressed as a share of its own reach.
///
/// Eight is enough to catch a crest or a lip from any direction, and it is eight
/// height samples against the four and a half thousand a chunk's mesh already
/// costs — which is to say, nothing.
const AROUND: usize = 8;
const BEDDED: f32 = 0.06;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_prop_on_a_crest_beds_in_rather_than_perching() {
        // The reported fault: a boulder on a dune with daylight under both sides
        // of it. A rigid thing stood at the height under its own middle hangs over
        // whatever the ground does on the way out to its rim, and on a crest the
        // ground goes DOWN in every direction — which is what a crest is.
        //
        // Measured against the ground it actually stands on rather than against a
        // fixture: somewhere on this world is a crest, and the rule has to hold
        // wherever it is.
        let terrain = Terrain::new();
        let reach = 3.0;

        let mut found = 0;
        let mut worst_hang = 0.0_f32;
        for step in 0..4_000 {
            // A scatter over the map rather than a grid, so this is not sampling
            // one hillside.
            let at = Vec2::new(
                terrain_core::forest::chance(step, 0, 91) - 0.5,
                terrain_core::forest::chance(step, 0, 92) - 0.5,
            ) * terrain.half()
                * 1.9;
            let middle = terrain.drawn_height(at.x, at.y);
            if middle < 2.0 {
                continue;
            }
            // How far the ground falls away under the thing's own footprint.
            let mut lowest = middle;
            for turn in 0..8 {
                let angle = turn as f32 / 8.0 * std::f32::consts::TAU;
                let round = at + Vec2::new(angle.cos(), angle.sin()) * reach * 0.75;
                lowest = lowest.min(terrain.drawn_height(round.x, round.y));
            }
            if middle - lowest < 0.35 {
                // Flat enough that there was nothing to hang over.
                continue;
            }
            found += 1;
            let stood = bedded(&terrain, at, reach);
            worst_hang = worst_hang.max(stood - lowest);
            assert!(
                stood <= lowest + 1.0e-4,
                "stood at {stood:.2} m with ground down to {lowest:.2} under it",
            );
        }

        assert!(found > 50, "only {found} crests to test against");
        assert!(
            worst_hang <= 0.0,
            "something still hangs {worst_hang:.2} m over its own ground"
        );
    }
}

#[cfg(test)]
mod affordable {
    use super::*;

    #[test]
    fn littering_a_chunk_stays_affordable() {
        // The frame is already spent on shadows, and every caster is submitted
        // again for every cascade — so a chunk's litter has to stay in the same
        // order as the chunk's own mesh rather than dwarfing it.
        let terrain = Terrain::new();
        let pool: Vec<Prop> = (0..prop::VARIETIES).map(prop::from_pool).collect();

        // A wooded chunk, which is the thickest litter there is. Found rather
        // than assumed: a fixture would measure the ranch, which is levelled and
        // has none.
        let mut worst = 0;
        let mut looked = 0;
        for step in 0..24 {
            let low = Vec2::new(step as f32 * 331.0 - 3000.0, step as f32 * 197.0 - 1400.0);
            let mesh = litter(&terrain, &pool, low);
            if !mesh.is_empty() {
                looked += 1;
                worst = worst.max(mesh.vertices());
            }
        }

        assert!(looked > 4, "only {looked} of 24 chunks had anything on them");
        // A terrain chunk is 65 x 65 vertices, about 4,200. Litter may be of
        // that order and must not be of a different one.
        assert!(
            worst < 20_000,
            "the busiest chunk carries {worst} vertices of litter"
        );
    }

    #[test]
    fn nothing_is_littered_on_a_town() {
        // A town's ground is somebody's, and boulders through the market square
        // are the same fault the rivers had.
        let terrain = Terrain::new();
        let pool: Vec<Prop> = (0..prop::VARIETIES).map(prop::from_pool).collect();

        for site in terrain.sites() {
            // The chunk the site's middle falls in, which is levelled ground
            // right the way across.
            let low = chunk_origin(chunk_at(Vec3::new(site.at.x, 0.0, site.at.y)));
            let mesh = litter(&terrain, &pool, low);
            // Not zero outright — a big site's chunk can reach past its skirt —
            // but a levelled middle must be clear, so the count has to be far
            // short of what open country carries.
            assert!(
                mesh.vertices() < 4_000,
                "the place at {:.0}, {:.0} has {} vertices of litter on it",
                site.at.x,
                site.at.y,
                mesh.vertices()
            );
        }
    }
}
