//! Buildings: what the bench draws, standing on the ground the game generates.
//!
//! Opificium's builder is where a house is *made* — parts on a sixteenth-metre
//! lattice, painted from this game's own palette ramps — and `cargo test
//! bake_the_works -- --ignored` bakes it down to plain boxes in
//! `assets/buildings/<name>.json`. Nothing here draws a building; this reads
//! what was drawn, welds it into a mesh, and stands it somewhere.
//!
//! Signs and bridges are the same thing to the bench and the same thing here: a
//! bridge is boxes, a sign is boxes. Only `kind` tells them apart, and only when
//! something cares.
//!
//! # Trees are not buildings
//!
//! They are grown in `terrain-core` from a hash of position, twenty varieties,
//! no files at all. Drawing them at the bench would make them heavier, fewer and
//! all alike. See `world/forest.rs`.
//!
//! # Where they stand, for now
//!
//! One per town site, at its middle. That is not a village — laying out a street
//! is its own job and is not started — but the sites already exist as levelled
//! ground with a size and a centre, so it puts real buildings on real ground the
//! moment a drawing is baked, rather than waiting on the layout to be designed.

mod plan;
mod shape;

use std::path::Path;

use bevy::prelude::*;

pub use plan::Plan;

use crate::config::BUILDINGS_DIR;
use crate::states::AppState;
use crate::world::terrain::TerrainSource;

/// Every building the game found on disk, in the order their names sort.
#[derive(Resource, Default)]
pub struct Catalogue(pub Vec<Plan>);

/// A building standing in the world.
#[derive(Component)]
pub struct Raised {
    /// What the village raised it as. Nothing asks yet — a tavern and a
    /// storehouse are the same boxes to the renderer — but what a building is
    /// FOR is the first thing anything built on top of this will want, and it
    /// is a fact the drawing already carries.
    #[allow(dead_code)]
    pub kind: String,
}

/// A door, a sign — whatever the drawing said the place is FOR, in world space.
///
/// Carried as an entity rather than looked up later so that whatever comes to
/// use it — a ranger walking in, a shopkeeper standing at a counter — asks the
/// world where the door is instead of re-reading the file.
#[derive(Component)]
pub struct Purpose {
    /// `door`, and whatever else a drawing marks. Not read yet: there is nobody
    /// to walk through a door. Spawned regardless, because the alternative is
    /// re-reading every file the day there is.
    #[allow(dead_code)]
    pub mark: String,
}

pub struct BuildingPlugin;

impl Plugin for BuildingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Catalogue>()
            .add_systems(Startup, read_the_catalogue)
            // After the world, so there is ground to stand on and sites to
            // stand at. Runs once on entering play rather than at startup for
            // the same reason.
            .add_systems(OnEnter(AppState::Playing), raise_the_sites);
    }
}

/// Reads every baked building in `assets/buildings/`.
///
/// A missing folder is the ordinary case for a game whose buildings have not
/// been drawn yet, and says nothing. A file that will not read says so and is
/// skipped: one bad drawing should not cost a town its other buildings.
fn read_the_catalogue(mut catalogue: ResMut<Catalogue>) {
    let folder = Path::new(BUILDINGS_DIR);
    let Ok(entries) = std::fs::read_dir(folder) else {
        return;
    };

    let mut found: Vec<(String, Plan)> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|kind| kind != "json") {
            continue;
        }
        match std::fs::read_to_string(&path).map_err(|why| why.to_string()) {
            Ok(json) => match Plan::read(&json) {
                Ok(plan) => found.push((path.display().to_string(), plan)),
                Err(why) => warn!("{}: {why} - not raised", path.display()),
            },
            Err(why) => warn!("{}: {why} - not raised", path.display()),
        }
    }

    // Sorted, so which building lands on which site does not depend on the order
    // the filesystem happened to hand them over.
    found.sort_by(|a, b| a.1.name.cmp(&b.1.name));
    catalogue.0 = found.into_iter().map(|(_, plan)| plan).collect();

    if catalogue.0.is_empty() {
        return;
    }
    info!(
        "buildings: {} read from {}",
        catalogue.0.len(),
        folder.display()
    );
}

/// Stands one building at each town site.
fn raise_the_sites(
    mut commands: Commands,
    catalogue: Res<Catalogue>,
    terrain: Res<TerrainSource>,
    standing: Query<Entity, With<Raised>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if catalogue.0.is_empty() {
        return;
    }
    // Leaving on foot and coming back should not raise a second town on top of
    // the first.
    if !standing.is_empty() {
        return;
    }

    let cloth = materials.add(building_cloth(false));
    let glazing = materials.add(building_cloth(true));

    let mut overhanging = 0;
    for (index, site) in terrain.sites().iter().enumerate() {
        let plan = &catalogue.0[index % catalogue.0.len()];
        // The site's own levelled height, not the ground under the middle — a
        // building set by a sample would sit askew if the level had any slack
        // in it, and the site is the flat it was levelled to.
        let stance = Transform::from_xyz(site.at.x, site.height, site.at.y);

        // Standing at the middle, a building reaches its furthest corner. Past
        // the levelled ground it would have one end on a hillside, which reads
        // as a broken building rather than as a site too small for it.
        let (low, high) = plan.reach();
        let corner = Vec2::new(low.x.abs().max(high.x), low.z.abs().max(high.z)).length();
        if corner > site.radius {
            overhanging += 1;
        }

        raise(
            &mut commands,
            plan,
            stance,
            &mut meshes,
            (cloth.clone(), glazing.clone()),
        );
    }

    // Counted and said once. Twenty identical complaints for twenty sites
    // raising the same drawing is noise that hides the next real warning.
    if overhanging > 0 {
        warn!("{overhanging} building(s) reach past their site's levelled ground");
    }
}

/// Stands one building, with its marks as children.
pub fn raise(
    commands: &mut Commands,
    plan: &Plan,
    stance: Transform,
    meshes: &mut Assets<Mesh>,
    cloth: (Handle<StandardMaterial>, Handle<StandardMaterial>),
) -> Entity {
    let (solid, glass) = shape::raise(plan);
    let (opaque_cloth, glazing) = cloth;

    let building = commands
        .spawn((
            Raised {
                kind: plan.kind.clone(),
            },
            Name::new(plan.name.clone()),
            stance,
            Visibility::default(),
        ))
        .id();

    // The walls and the glass are two meshes because glass has to be drawn after
    // what is behind it, and one mesh can only be one or the other. Children, so
    // moving the building moves both.
    if !solid.is_empty() {
        commands.entity(building).with_children(|parts| {
            parts.spawn((Mesh3d(meshes.add(solid.into_mesh())), MeshMaterial3d(opaque_cloth)));
        });
    }
    if !glass.is_empty() {
        commands.entity(building).with_children(|parts| {
            parts.spawn((Mesh3d(meshes.add(glass.into_mesh())), MeshMaterial3d(glazing)));
        });
    }

    for mark in &plan.marks {
        commands.entity(building).with_children(|parts| {
            parts.spawn((
                Purpose {
                    mark: mark.mark.clone(),
                },
                Transform::from_translation(mark.at).with_rotation(Quat::from_rotation_y(mark.yaw)),
            ));
        });
    }

    building
}

/// The material every building wears.
///
/// White, because the shader multiplies it by the mesh's vertex colours and the
/// bench already looked every colour up from this game's palette — so leaving it
/// white paints exactly what the drawing said. The same bargain the terrain
/// makes with its biome colours.
fn building_cloth(lets_light_through: bool) -> StandardMaterial {
    StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: if lets_light_through { 0.25 } else { 0.88 },
        reflectance: if lets_light_through { 0.5 } else { 0.05 },
        alpha_mode: if lets_light_through {
            AlphaMode::Blend
        } else {
            AlphaMode::Opaque
        },
        ..default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reads what is actually in `assets/buildings/`, not a fixture.
    ///
    /// The forms are the part of this that cannot be checked by looking: a
    /// winding or a run out by a sign is a shape that is wrong on screen and
    /// silent everywhere else. So the cottage that ships uses all four, and this
    /// walks the whole path a running game walks — file, parse, weld — and
    /// judges the result by its measurements.
    #[test]
    fn the_cottage_on_disk_reads_and_welds() {
        let path = Path::new(BUILDINGS_DIR).join("house-cottage.json");
        let json = std::fs::read_to_string(&path)
            .unwrap_or_else(|why| panic!("{}: {why}", path.display()));
        let plan = Plan::read(&json).expect("the shipped cottage should read");

        assert_eq!(plan.kind, "house");
        assert!(
            plan.marks.iter().any(|mark| mark.mark == "door"),
            "a house a ranger cannot walk into is not finished"
        );

        // All four shapes, so the one thing this file is for is actually done.
        let forms: Vec<_> = plan.boxes.iter().map(|block| block.form).collect();
        for wanted in [
            plan::Form::Box,
            plan::Form::Wedge,
            plan::Form::Ridge,
            plan::Form::Cut { low: 0.25, high: -0.25 },
            plan::Form::Hip { across: 0.5, along: 0.5 },
        ] {
            assert!(forms.contains(&wanted), "the cottage should use {wanted:?}");
        }

        let (solid, glass) = shape::raise(&plan);
        assert!(!solid.is_empty(), "walls and roof should weld");
        assert!(!glass.is_empty(), "its windows should weld separately");
    }

    /// The stated footprint has to be the real one, or a village clears the
    /// wrong plot and buildings overlap the day anything lays out a street.
    #[test]
    fn the_cottage_stands_within_the_footprint_it_claims() {
        let json =
            std::fs::read_to_string(Path::new(BUILDINGS_DIR).join("house-cottage.json")).unwrap();
        let plan = Plan::read(&json).unwrap();
        let (low, high) = plan.reach();

        assert!(low.y > -1.0e-4, "it should stand ON the ground, not in it");
        assert!(
            high.y <= plan.high + 1.0e-3,
            "it reaches {:.2} m and claims {:.2}",
            high.y,
            plan.high
        );
        assert!(
            low.x >= -plan.half_w - 1.0e-3 && high.x <= plan.half_w + 1.0e-3,
            "wider than it claims: {:.2}..{:.2} against {:.2}",
            low.x,
            high.x,
            plan.half_w
        );
        assert!(
            low.z >= -plan.half_d - 1.0e-3 && high.z <= plan.half_d + 1.0e-3,
            "deeper than it claims: {:.2}..{:.2} against {:.2}",
            low.z,
            high.z,
            plan.half_d
        );
    }
}
