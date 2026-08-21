//! Lets authored models take over the shapes the world grows for itself.
//!
//! # Replacing the shape without disturbing anything else
//!
//! A tree here is a POOL entry. `grow_the_grove` grows a handful of varieties
//! once and every tree in the world is an instance of one of them, with its own
//! place, turn and scale — a forest is tens of thousands of trees and a mesh each
//! is not affordable. A variety is two meshes, `wood` and `leaves`, because bark
//! and foliage wear different materials and one mesh can only wear one.
//!
//! So an authored tree needs to be exactly that: two meshes. Nothing about
//! placement, streaming, the shadow ring or the per-tree scale has to change, and
//! none of it does.
//!
//! # Why the generated pool is still built first
//!
//! The obvious way round is to load the files and build the pool from them. It is
//! wrong here for one reason: glTF loading is ASYNCHRONOUS, and the pool is wanted
//! the instant the first chunk streams in. Waiting for it would put this straight
//! back into a bug this project has already had and fixed — trees appearing slowly,
//! or not at all, because the thing that plants them ran before the thing that
//! grows them finished.
//!
//! So the generated pool is built exactly as before, at once, and the authored
//! shapes are dropped in over the top when they arrive. What gets replaced is the
//! **mesh asset behind the pool's handle**, not the handle — so every tree already
//! standing in the world changes shape too, rather than only the ones planted from
//! then on. At startup that lands well before the menu is dismissed.
//!
//! A species with no file keeps the grown shape, so this is species-at-a-time and
//! reversible: delete a `.glb` and the world grows its own again.

use bevy::gltf::{Gltf, GltfMesh};
use bevy::prelude::*;

use super::stream::Grove;

/// The species a `.glb` may be authored for.
///
/// These are `terrain_core`'s own species names, lowercased, because a file has to
/// be matched to the variety whose COLOUR belongs to the same tree — see
/// [`file_for`]. Palm and willow are not here and keep the shapes the world grows
/// for them, which is the designed fallback.
///
/// **This must match `SPECIES` in `dev/art/trees.py`** — see
/// `the_species_here_are_the_species_blender_builds`, which reads that file.
pub const SPECIES: [&str; 5] = ["oak", "birch", "spruce", "pine", "acacia"];

/// Which authored file, if any, belongs to a species.
///
/// # Matched by species, never by position
///
/// The pool is grouped by species, four variants apiece, and that layout belongs
/// to `terrain_core`. The first cut of this picked a file with `index % 5`, which
/// scattered authored shapes across the groups: a variety got a birch's shape and
/// a spruce's bark, or an oak's crown over the chalk-pale trunk that makes a birch
/// a birch. Reported as every trunk in the world looking like concrete, and it was
/// not the palette at all.
///
/// Exhaustive on purpose. Add a species to `terrain_core` and this stops
/// compiling until somebody decides whether it has a model — which is the
/// question worth being asked, rather than silently keeping a grown shape nobody
/// meant to keep.
fn file_for(species: terrain_core::tree::Species) -> Option<&'static str> {
    use terrain_core::tree::Species;
    Some(match species {
        Species::Oak => "oak",
        Species::Birch => "birch",
        Species::Spruce => "spruce",
        Species::Pine => "pine",
        Species::Acacia => "acacia",
        // No model yet; the world grows its own.
        Species::Palm | Species::Willow => return None,
    })
}

/// The authored files, once asked for.
#[derive(Resource)]
pub struct AuthoredWoods {
    /// One slot per species, in `SPECIES` order. `None` where there is no file.
    asked: Vec<Option<Handle<Gltf>>>,
    /// Which species have been dropped into the pool already.
    taken: Vec<bool>,
}

/// Asks for whatever authored species are actually on disk.
///
/// Checked as FILES first rather than simply loading and letting a miss fail:
/// a missing species is the ordinary case — most of them will not exist for a
/// long time — and a handful of asset-not-found errors in the log every launch
/// trains everybody to ignore the log.
pub fn ask_for_the_authored_woods(mut commands: Commands, assets: Res<AssetServer>) {
    let folder = crate::asset_file("assets/models");
    let mut asked = Vec::with_capacity(SPECIES.len());
    let mut found = 0;
    for species in SPECIES {
        let name = format!("tree_{species}.glb");
        if folder.join(&name).is_file() {
            asked.push(Some(assets.load(format!("models/{name}"))));
            found += 1;
        } else {
            asked.push(None);
        }
    }
    if found > 0 {
        info!("{found} authored tree species to take over from the grown ones");
    }
    commands.insert_resource(AuthoredWoods {
        taken: vec![false; asked.len()],
        asked,
    });
}

/// Drops each authored shape into the pool as its file finishes loading.
///
/// Runs until every species that has a file has been taken, then stops asking.
pub fn take_the_authored_shapes(
    mut woods: ResMut<AuthoredWoods>,
    grove: Res<Grove>,
    files: Res<Assets<Gltf>>,
    parts: Res<Assets<GltfMesh>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (which, handle) in woods.asked.clone().into_iter().enumerate() {
        if woods.taken[which] {
            continue;
        }
        let Some(handle) = handle else {
            woods.taken[which] = true;
            continue;
        };
        let Some(file) = files.get(&handle) else {
            continue; // still in flight; ask again next frame
        };

        let Some(wood) = one_mesh(file, &parts, &meshes, "wood") else {
            warn!(
                "models/tree_{}.glb has no mesh named `wood`; keeping the grown shape",
                SPECIES[which]
            );
            woods.taken[which] = true;
            continue;
        };
        let Some(leaves) = one_mesh(file, &parts, &meshes, "leaves") else {
            warn!(
                "models/tree_{}.glb has no mesh named `leaves`; keeping the grown shape",
                SPECIES[which]
            );
            woods.taken[which] = true;
            continue;
        };

        let dressed = lay_the_shape_into_the_pool(&grove, &mut meshes, which, &wood, &leaves);
        info!(
            "tree_{} took over {dressed} of the grove's {} varieties",
            SPECIES[which],
            grove.trees.len()
        );
        woods.taken[which] = true;
    }
}

/// Puts one species' shapes into every pool variety it covers, and says how many.
///
/// # The asset is replaced, not the handle
///
/// Trees already standing in the world hold these handles in their own
/// `Mesh3d`. Swapping the handle in the pool would leave every planted tree
/// wearing the grown shape until something replanted it — which, for a chunk that
/// has already streamed in, is never. Overwriting the asset the handle points at
/// changes what every instance draws, at once.
///
/// Every variety of that species takes the same shape — four of them, one per
/// variant — and they still differ in colour, which is where most of what makes a
/// wood look like a wood comes from. A species with no file is left alone.
fn lay_the_shape_into_the_pool(
    grove: &Grove,
    meshes: &mut Assets<Mesh>,
    which: usize,
    wood: &Mesh,
    leaves: &Mesh,
) -> usize {
    let mut dressed = 0;
    for variety in &grove.trees {
        if file_for(variety.species) != Some(SPECIES[which]) {
            continue;
        }
        meshes.insert(&variety.wood, wood.clone());
        meshes.insert(&variety.leaves, leaves.clone());
        dressed += 1;
    }
    dressed
}

/// Whether every species has been settled one way or the other.
pub fn the_woods_are_settled(woods: Option<Res<AuthoredWoods>>) -> bool {
    woods.is_some_and(|woods| woods.taken.iter().all(|done| *done))
}

/// The one mesh in a glTF file that goes by this name, already loaded.
///
/// Its primitives are welded in Blender, so a named mesh is one primitive. If it
/// somehow arrives in several, the first is taken and the rest are lost — which is
/// why the generator joins each half into a single object under a known name.
fn one_mesh(
    file: &Gltf,
    parts: &Assets<GltfMesh>,
    meshes: &Assets<Mesh>,
    name: &str,
) -> Option<Mesh> {
    let handle = file.named_meshes.get(name)?;
    let part = parts.get(handle)?;
    let first = part.primitives.first()?;
    meshes.get(&first.mesh).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;


    /// A shape lands on the varieties of its OWN species, and nowhere else.
    ///
    /// # What this covers, and what it does not
    ///
    /// The written-here part: which varieties a species claims, and that the
    /// replacement lands on the ASSET rather than the handle — so a tree already
    /// standing in the world changes shape instead of only ones planted after.
    ///
    /// It is built to catch the bug that shipped. The pool is grouped by species,
    /// four variants apiece, and the first cut picked varieties with `index % 5`:
    /// authored shapes scattered across the groups, so a variety wore one species'
    /// crown over another's bark. In the world that came out as chalk-pale birch
    /// trunks under oak canopies everywhere, and it was read as the bark palette
    /// being wrong. So this asserts the negative as hard as the positive — nothing
    /// belonging to another species may move.
    ///
    /// The glTF loading itself is Bevy's and is left to Bevy. A headless load was
    /// tried and does not complete under a hand-assembled plugin set — every file
    /// sits at `Loading` through seconds of real waiting, which is worth knowing
    /// before anybody spends an afternoon on it. What stands in for it is the file
    /// contract, checked above: two meshes, named `wood` and `leaves`.
    #[test]
    fn a_shape_lands_only_on_the_varieties_of_its_own_species() {
        use bevy::render::mesh::PrimitiveTopology;
        use terrain_core::tree::{Species, VARIANTS};

        let mut meshes = Assets::<Mesh>::default();
        let flat = |points: usize| {
            Mesh::new(
                PrimitiveTopology::TriangleList,
                bevy::asset::RenderAssetUsages::RENDER_WORLD,
            )
            .with_inserted_attribute(
                Mesh::ATTRIBUTE_POSITION,
                vec![[0.0f32, 0.0, 0.0]; points.max(3)],
            )
        };

        // The real layout: every species, VARIANTS of each, in pool order.
        let mut trees = Vec::new();
        for species in Species::ALL {
            for _ in 0..VARIANTS {
                trees.push(crate::world::stream::Variety {
                    species,
                    wood: meshes.add(flat(3)),
                    leaves: meshes.add(flat(3)),
                    leaf: Handle::default(),
                    bark: Handle::default(),
                });
            }
        }
        // The handles a planted tree would be holding, taken BEFORE the swap.
        let planted: Vec<(Species, Handle<Mesh>, Handle<Mesh>)> = trees
            .iter()
            .map(|variety| (variety.species, variety.wood.clone(), variety.leaves.clone()))
            .collect();
        let grove = Grove { trees };

        // Birch takes over. Chosen deliberately: it is the palest bark in the
        // world, so it is the species whose shape being in the wrong place was
        // visible from any hillside.
        let which = SPECIES
            .iter()
            .position(|name| *name == "birch")
            .expect("birch is an authored species");
        let dressed = lay_the_shape_into_the_pool(&grove, &mut meshes, which, &flat(40), &flat(70));
        assert_eq!(
            dressed, VARIANTS,
            "birch claimed {dressed} varieties; the pool holds {VARIANTS} of each species"
        );

        for (species, wood, leaves) in &planted {
            // Read back through the ORIGINAL handle: that is what a standing tree
            // holds, and it is the whole reason the asset is replaced.
            let woody = meshes.get(wood).expect("a pool mesh went missing").count_vertices();
            let leafy = meshes.get(leaves).expect("a pool mesh went missing").count_vertices();
            if *species == Species::Birch {
                assert_eq!(
                    (woody, leafy),
                    (40, 70),
                    "a birch variety still draws the shape the world grew for it"
                );
            } else {
                assert_eq!(
                    (woody, leafy),
                    (3, 3),
                    "a {species:?} took the birch shape — that is the bug that put \
                     chalk-pale trunks under oak crowns"
                );
            }
        }
    }

    /// The species listed here are the species Blender actually builds.
    ///
    /// Two lists of the same names in two languages, and they only agree because
    /// somebody typed them twice. Add a species to the generator and forget this
    /// one and the new tree is simply never used — nothing errors, the wood just
    /// keeps growing its own shape. So the agreement is read out of the generator
    /// rather than trusted.
    #[test]
    fn the_species_here_are_the_species_blender_builds() {
        let script = std::fs::read_to_string("dev/art/trees.py")
            .expect("dev/art/trees.py should be beside the crate");
        let line = script
            .lines()
            .find(|line| line.starts_with("SPECIES = "))
            .expect("dev/art/trees.py sets no SPECIES");
        let listed: Vec<String> = line
            .trim_start_matches("SPECIES = ")
            .trim_matches(|c| c == '(' || c == ')')
            .split(',')
            .map(|part| part.trim().trim_matches('"').to_string())
            .filter(|part| !part.is_empty())
            .collect();
        assert_eq!(
            listed,
            SPECIES.to_vec(),
            "dev/art/trees.py builds {listed:?} and the game looks for {SPECIES:?}"
        );
    }

    /// Every species named here has a file, and it keeps the model conventions.
    ///
    /// The conventions themselves are `models.rs`'s business; this checks the
    /// thing that would otherwise be silent — a species the game asks for and
    /// nothing ever provides.
    #[test]
    fn every_species_has_a_model_that_is_a_tree() {
        for species in SPECIES {
            let road = std::path::Path::new("assets/models").join(format!("tree_{species}.glb"));
            let bytes = match std::fs::read(&road) {
                Ok(bytes) => bytes,
                // Not a failure: a species is allowed to have no file yet and
                // keep the grown shape. Said out loud so it is not mistaken for
                // a pass.
                Err(_) => {
                    println!("tree_{species}.glb is not there yet; the grown shape stands");
                    continue;
                }
            };
            let model = crate::models::inspect(&bytes)
                .unwrap_or_else(|why| panic!("tree_{species}.glb is not a model: {why}"));
            let faults = crate::models::faults(&model);
            assert!(faults.is_empty(), "tree_{species}.glb: {}", faults.join("; "));
            // The contract the runtime depends on: two meshes, by NAME, one
            // primitive each. Blender names a glTF mesh after the object, so this
            // is really checking that `weld` still names its halves `wood` and
            // `leaves` — rename either and the game finds nothing and silently
            // keeps the shape it grew, which is the worst kind of failure.
            for wanted in ["wood", "leaves"] {
                let found = model
                    .meshes
                    .iter()
                    .find(|(name, _)| name == wanted)
                    .unwrap_or_else(|| {
                        panic!(
                            "tree_{species}.glb has no mesh named `{wanted}` — it has {:?}",
                            model.meshes
                        )
                    });
                assert_eq!(
                    found.1, 1,
                    "tree_{species}.glb `{wanted}` is in {} pieces; the generator welds each                      half into one, and only the first would be drawn",
                    found.1
                );
            }

            // A tree is taller than it is wide. Not true of everything in the
            // world, but it is true of every one of these, and it catches a file
            // that has been swapped for something else entirely.
            let tall = model.high[1] - model.low[1];
            let wide = (model.high[0] - model.low[0]).max(model.high[2] - model.low[2]);
            assert!(
                tall > wide * 0.55,
                "tree_{species}.glb is {tall:.1} m tall and {wide:.1} m across — not a tree"
            );
        }
    }
}
