//! The world: terrain generation, chunk streaming, and the sea.

pub mod authored;
pub mod biome;
pub mod bridge;
pub mod forest;
pub mod chunk;
pub mod country;
pub mod cover;
pub mod edit;
pub mod heightmap;
pub mod litter;
pub mod town;
pub mod pass;
pub mod placed;
pub mod prop;
pub mod route;
pub mod settle;
pub mod stream;
pub mod surface;
pub mod terrain;
pub mod tufts;
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
    pub(crate) fn new(half: Vec2) -> Self {
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

    /// Keeps a flat position inside the map, well in from the border.
    ///
    /// What a tunnel head is held by while it drives: a bore that ran off the edge
    /// of the world would be a hole into nothing.
    pub fn clamp_flat(&self, at: Vec2) -> Vec2 {
        let limit = (self.half - Vec2::splat(CHUNK_SIZE)).max(Vec2::ZERO);
        at.clamp(-limit, limit)
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

/// Whether the world is the thing on screen.
///
/// The workbench is a room with a floor and whatever you are building in it. The
/// landscape has no business streaming behind it.
fn a_world_is_on_screen(state: Res<State<crate::states::AppState>>) -> bool {
    use crate::states::AppState;
    #[cfg(feature = "tools")]
    {
        matches!(state.get(), AppState::Playing | AppState::Editing)
    }
    #[cfg(not(feature = "tools"))]
    {
        matches!(state.get(), AppState::Playing)
    }
}

/// Whether the sea still needs putting up.
fn no_sea_yet(seas: Query<(), With<water::Water>>) -> bool {
    seas.is_empty()
}


/// Takes it down again on the way out.
///
/// The sea is the one that matters: it is a single plane eight kilometres across
/// sitting at nought, and anything else drawn near that height ends up fighting it
/// for pixels or standing underneath it.
fn clear_the_world(
    mut commands: Commands,
    seas: Query<Entity, With<water::Water>>,
) {
    for sea in &seas {
        commands.entity(sea).despawn();
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
                ),
            )
            // The world's CONTENT arrives with the world, and leaves with it.
            //
            // The sea used to be spawned at startup and never taken away — so it
            // was still there in the workbench, an eight-kilometre plane at y=0
            // with the bench floor a centimetre under it. That is the whole of
            // what looked like a grid with colours in it: z-fighting where the two
            // nearly coincide, and pieces standing in it going blue below the
            // waterline because they were UNDERWATER.
            //
            // Growing the trees and the props costs real work too, and a maker who
            // opened the bench paid for a forest they never saw.
            // Each guarded on its own, so entering a world twice does not grow a
            // second forest or float a second sea. Run conditions rather than one
            // system calling the others: a system called as a function has to be
            // handed borrows it was never written to take.
            .add_systems(
                OnEnter(crate::states::AppState::Playing),
                (
                    water::spawn_water.run_if(no_sea_yet),
                    stream::grow_the_grove.run_if(not(resource_exists::<stream::Grove>)),
                    prop::setup_props.run_if(not(resource_exists::<prop::PropPool>)),
                    authored::ask_for_the_authored_woods
                        .run_if(not(resource_exists::<authored::AuthoredWoods>)),
                    cover::read_the_sprig_kit
                        .run_if(not(resource_exists::<cover::SprigKit>)),
                ),
            )
            .add_systems(OnExit(crate::states::AppState::Playing), clear_the_world);

        // The terrain tool streams the same world, so it needs the same content
        // — the sea, the grove, the props. It got none of them: entering the
        // tool without having played first showed a world with no trees and no
        // water, and the PLANT brush painted the woods layer invisibly because
        // there was no grove to draw from. Same guards, same teardown, so
        // whichever door you come through the world is the world.
        #[cfg(feature = "tools")]
        app.add_systems(
            OnEnter(crate::states::AppState::Editing),
            (
                water::spawn_water.run_if(no_sea_yet),
                stream::grow_the_grove.run_if(not(resource_exists::<stream::Grove>)),
                prop::setup_props.run_if(not(resource_exists::<prop::PropPool>)),
                authored::ask_for_the_authored_woods
                    .run_if(not(resource_exists::<authored::AuthoredWoods>)),
                cover::read_the_sprig_kit
                    .run_if(not(resource_exists::<cover::SprigKit>)),
            ),
        )
        .add_systems(OnExit(crate::states::AppState::Editing), clear_the_world);

        // Authored shapes drop into the grown pool as their files arrive, and this
        // stops asking once every species is settled — see `authored`.
        app.add_systems(
            Update,
            authored::take_the_authored_shapes
                .run_if(resource_exists::<authored::AuthoredWoods>)
                .run_if(resource_exists::<stream::Grove>)
                .run_if(not(authored::the_woods_are_settled)),
        );

        app.add_systems(
                Update,
                (
                    stream::queue_chunks,
                    stream::collect_chunks,
                    stream::unload_chunks,
                    stream::shade_far_wood,
                    // After the ground, because cover can only be laid on a
                    // chunk that is already loaded.
                    cover::strip_the_cover_when_the_season_turns,
                    cover::dress_chunks,
                    cover::collect_cover,
                    cover::undress_chunks,
                    prop::litter_chunks,
                    prop::collect_props,
                    prop::clear_chunks,
                )
                    // Not in the workbench. Streaming a world nobody is looking at
                    // costs a frame's work every frame and, worse, keeps a second
                    // camera and a whole landscape alive behind a room that is
                    // meant to have nothing in it but what you are building.
                    .run_if(a_world_is_on_screen)
                    .chain(),
            );
    }
}
