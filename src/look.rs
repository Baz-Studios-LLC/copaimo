//! How a warden looks, and how that gets painted onto the model.
//!
//! # The model is five meshes, and the game tints them apart
//!
//! `dev/art/people.py` builds a body as `skin`, `clothes`, `sclera`, `eyes` and
//! `pupil`, plus a wig named `hair` and a hat named `hat` in their own files. Each
//! carries a grey shading ramp in its vertices — light down a form, baked in,
//! because nothing here is textured — and the game multiplies that by whatever
//! colour this record says. So one body serves every skin tone and one wig every
//! hair colour.
//!
//! # Why the paint is a system and not part of spawning
//!
//! A glTF scene is instanced ASYNCHRONOUSLY, and its meshes do not exist on the
//! frame the warden is spawned. So nothing can be painted at spawn time; a system
//! has to find the parts as they arrive and dress them. It keeps asking until every
//! part it can name has been done, which is a frame or two after the world opens.

// Every choice here is a surface waiting for its consumer: the creator screen will
// construct these, and until it exists only the default warden's `Male`, `Crop` and
// `Cap` are ever named. They are not dead — `every_choice_names_a_model_that_exists`
// walks all of them and checks each names a file that is really there, which is the
// thing worth guarding. So the warning is silenced with a reason rather than the
// variants being deleted and re-added later.
#![allow(dead_code)]

use bevy::prelude::*;

use crate::shade::{shaded, Shaded};

/// How tall a warden stands in the world, in metres.
///
/// The terrain, the camera distance and the walking speed are all tuned against
/// this, so a model authored at some other height is scaled to it rather than the
/// world being re-tuned around whatever a file happens to be.
pub const TALL: f32 = 1.70;

/// Which body a warden wears.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Build {
    /// The made character: textured, dressed, and rigged as a full biped.
    #[default]
    Ranger,
    /// The scripted bodies. Kept because they are the ones whose PARTS can be
    /// tinted apart — see `colour_of` — which is what a character creator needs and
    /// what a single-material model cannot give.
    Male,
    Female,
}

impl Build {
    /// The file this build's body lives in.
    pub fn model(self) -> &'static str {
        match self {
            Build::Ranger => "models/person_ranger.glb",
            Build::Male => "models/person_male.glb",
            Build::Female => "models/person_female.glb",
        }
    }

    /// How tall the figure in the file stands, in its own units.
    ///
    /// Not every model is authored at human scale — one that comes in at a unit
    /// high is perfectly normal — so the game scales it to [`TALL`] rather than
    /// insisting the file be rebuilt.
    pub fn authored_height(self) -> f32 {
        match self {
            // The 2026-08-26 warden ships exactly as delivered - "literally go back to the
            // original" - and the file measures 1.70 m on its own.
            Build::Ranger => 1.70,
            Build::Male | Build::Female => 1.70,
        }
    }

    /// Which way to turn the model so its front is the game's forward, in radians.
    ///
    /// # Every model faces somewhere different
    ///
    /// The game's forward is -Z. The scripted bodies are built to it. The made
    /// character faces +X, which is neither wrong nor unusual — it is simply what
    /// the tool that produced it chose — so it is turned a quarter circle here. A
    /// yaw of +90 degrees carries +X onto -Z.
    ///
    /// Kept as a table beside the file name because that is what it is: a fact
    /// about an asset, not a rule about the world. Rebuilding somebody else's file
    /// to suit ours would be worse — the correction would have to be redone every
    /// time they sent a new one.
    pub fn turn(self) -> f32 {
        match self {
            // Nought for the delivered warden: he is authored facing Blender +Y, which the
            // glTF export carries to -Z - and -Z is already the game's forward. The quarter turn
            // belonged to the previous asset, which the build used to rotate onto +X first.
            Build::Ranger => 0.0,
            Build::Male | Build::Female => 0.0,
        }
    }

    /// Whether the model already has its own hair and clothes.
    ///
    /// A made character arrives dressed, with hair modelled and textured in. Putting
    /// a wig and a cap on top of that would be two heads of hair.
    pub fn dressed(self) -> bool {
        matches!(self, Build::Ranger)
    }
}

/// A hairstyle. The names are the files in `assets/models/part_hair_*.glb`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Hair {
    /// No wig at all. The default while the cap is a staple of the outfit — hair
    /// under a hat is geometry nobody sees, and the creator can offer it later.
    #[default]
    None,
    Crop,
    Bob,
    Tail,
    Braids,
    Curls,
}

impl Hair {
    /// The file this style lives in, or `None` for a bare head.
    pub fn model(self) -> Option<&'static str> {
        Some(match self {
            Hair::None => return None,
            Hair::Crop => "models/part_hair_crop.glb",
            Hair::Bob => "models/part_hair_bob.glb",
            Hair::Tail => "models/part_hair_tail.glb",
            Hair::Braids => "models/part_hair_braids.glb",
            Hair::Curls => "models/part_hair_curls.glb",
        })
    }
}

/// Something worn on the head, over the hair.
///
/// Its own choice rather than a hairstyle: somebody in a cap still has hair under
/// it, and the hat is authored to sit over a wig rather than instead of one.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Hat {
    #[default]
    None,
    Cap,
}

impl Hat {
    pub fn model(self) -> Option<&'static str> {
        match self {
            Hat::None => None,
            Hat::Cap => Some("models/part_hat_cap.glb"),
        }
    }
}

/// Everything the creator will decide about a warden.
///
/// A resource rather than a component: it is read when the warden is raised and by
/// the creator's own preview, and there is exactly one warden.
#[derive(Resource, Clone, Debug)]
pub struct Look {
    pub build: Build,
    pub hair: Hair,
    pub hat: Hat,
    /// Linear RGB, because that is what a material wants and what the vertex ramp
    /// is multiplied in.
    pub skin: Color,
    pub hair_colour: Color,
    pub eyes: Color,
    pub clothes: Color,
    pub hat_colour: Color,
}

impl Default for Look {
    /// A warden nobody has chosen yet.
    ///
    /// The creator has not been built, so this is what everybody gets — and it is
    /// deliberately an ordinary-looking one rather than a striking one, because the
    /// point of the default is to be unremarkable enough to test movement against.
    fn default() -> Self {
        Self {
            build: Build::Ranger,
            hair: Hair::None,
            hat: Hat::Cap,
            skin: Srgba::rgb(0.86, 0.68, 0.55).into(),
            hair_colour: Srgba::rgb(0.24, 0.16, 0.11).into(),
            eyes: Srgba::rgb(0.30, 0.48, 0.62).into(),
            clothes: Srgba::rgb(0.24, 0.33, 0.27).into(),
            // Guild green, the same family as the coat.
            hat_colour: Srgba::rgb(0.20, 0.38, 0.24).into(),
        }
    }
}

impl Look {
    /// What colour a named part of the model should be painted.
    ///
    /// `None` for a name this does not know, so an unexpected mesh is left alone
    /// rather than painted white.
    pub fn colour_of(&self, part: &str) -> Option<Color> {
        Some(match part {
            "skin" => self.skin,
            "clothes" => self.clothes,
            // The white of an eye, which is never quite white.
            "sclera" => Srgba::rgb(0.94, 0.94, 0.92).into(),
            "eyes" => self.eyes,
            // A pupil is a pupil.
            "pupil" => Srgba::rgb(0.06, 0.05, 0.06).into(),
            "hair" => self.hair_colour,
            "hat" => self.hat_colour,
            _ => return None,
        })
    }
}

/// Marks a warden whose parts still want painting.
#[derive(Component)]
pub struct Dressing;

/// A part that has been painted, so it is not looked at again.
///
/// Public only because it appears in a system signature, which makes it part of
/// that system's type — nothing outside this file has any use for it.
#[derive(Component)]
pub struct Painted;

/// Paints each part of the warden as the scene brings it in.
///
/// Runs every frame while there is anything left to dress. A glTF scene arrives
/// over more than one frame and its materials may not have loaded even once its
/// entities exist, so this cannot be keyed on `Added` — it asks, and asks again.
pub fn paint_the_warden(
    mut commands: Commands,
    look: Res<Look>,
    fresh: Query<(Entity, &Name), (With<Mesh3d>, Without<Painted>)>,
    ancestors: Query<&ChildOf>,
    dressing: Query<(), With<Dressing>>,
    mut ours: ResMut<Assets<Shaded>>,
) {
    for (entity, name) in &fresh {
        // Only parts of a warden: the world is full of other named meshes.
        let mut at = entity;
        let mut mine = dressing.contains(at);
        while !mine {
            match ancestors.get(at) {
                Ok(parent) => {
                    at = parent.parent();
                    mine = dressing.contains(at);
                }
                Err(_) => break,
            }
        }
        if !mine {
            continue;
        }
        let Some(colour) = look.colour_of(name.as_str()) else {
            // Not a part this knows. Marked anyway, or it is asked about forever.
            commands.entity(entity).insert(Painted);
            continue;
        };
        // The mesh carries its own grey ramp — light down a form, baked in — and a
        // material's base colour multiplies it. That is how one wig is every hair
        // colour and one body every skin tone.
        let coat = ours.add(shaded(StandardMaterial {
            base_color: colour,
            perceptual_roughness: 0.84,
            reflectance: 0.03,
            ..default()
        }));
        commands
            .entity(entity)
            .remove::<MeshMaterial3d<StandardMaterial>>()
            .insert((MeshMaterial3d(coat), Painted));
    }
}

/// Marks a hat or a wig, which belongs on the HEAD rather than on the warden.
#[derive(Component)]
pub struct WornOnTheHead;

/// Already hung on the head bone, so it is not looked at again.
#[derive(Component)]
pub struct Hung;

/// Hangs hats and wigs on the head bone, once the skeleton has arrived.
///
/// # A hat parented to the warden slides about
///
/// Hair and hats were children of the warden's own entity, which means they follow
/// the warden and NOT the head — so the moment the walk started bobbing and turning
/// the head, the hat stayed where the body was and appeared to slide across it.
///
/// A worn thing belongs to the head BONE. The body's skeleton arrives as entities
/// named after their bones, so this finds the one called `head` and re-parents to
/// it.
///
/// The offset is worked out rather than written down: a wig is authored in the
/// BODY's coordinates, so becoming a child of a bone means its transform has to be
/// whatever maps the body's space into that bone's. Taken from the two global
/// transforms at the moment of attachment — which is why this runs before anything
/// is played, while the skeleton is still in its rest pose. Attach it mid-stride and
/// the pose of that instant is baked into the offset for good.
pub fn hang_things_on_the_head(
    mut commands: Commands,
    worn: Query<Entity, (With<WornOnTheHead>, Without<Hung>)>,
    named: Query<(Entity, &Name, &GlobalTransform)>,
    wardens: Query<(Entity, &GlobalTransform), With<Dressing>>,
    ancestors: Query<&ChildOf>,
) {
    let Ok((warden, standing)) = wardens.single() else {
        return;
    };
    // The head bone, which has to be one of this warden's own: another figure's
    // skeleton would put the hat on somebody else.
    let head = named.iter().find(|(entity, name, _)| {
        if name.as_str() != "head" {
            return false;
        }
        let mut at = *entity;
        loop {
            if at == warden {
                return true;
            }
            match ancestors.get(at) {
                Ok(parent) => at = parent.parent(),
                Err(_) => return false,
            }
        }
    });
    let Some((bone, _, resting)) = head else {
        return;
    };

    // Body space into bone space. The wig's vertices are in body space, so this is
    // exactly the transform that leaves them where they were.
    let into_bone = resting.affine().inverse() * standing.affine();
    for thing in &worn {
        commands
            .entity(thing)
            .insert((ChildOf(bone), Transform::from_matrix(into_bone.into()), Hung));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every choice a warden can make names a file that is really there.
    ///
    /// # A missing model is an invisible warden
    ///
    /// The body, the hair and the hat are loaded by name, and a name that does not
    /// resolve is not an error the player sees — Bevy logs it and the warden simply
    /// arrives without that part, or without any body at all. So every variant of
    /// every choice is walked here and its file checked, which also means adding a
    /// hairstyle to the enum and forgetting to build it fails loudly.
    #[test]
    fn every_choice_names_a_model_that_exists() {
        let folder = std::path::Path::new("assets/models");
        let mut looked = 0;
        let mut check = |road: &str| {
            // The enums name assets as Bevy paths, which are relative to `assets/`.
            let file = folder.join(road.trim_start_matches("models/"));
            let bytes = std::fs::read(&file)
                .unwrap_or_else(|why| panic!("{road} is named but not there: {why}"));
            let model = crate::models::inspect(&bytes)
                .unwrap_or_else(|why| panic!("{road} is not a model: {why}"));
            // Hair and hats are PARTS: they sit at head height and have no business
            // standing on the floor, which the gate already knows from the name.
            let part = file
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("part_"));
            let faults = crate::models::faults_of(&model, part);
            assert!(faults.is_empty(), "{road}: {}", faults.join("; "));
            looked += 1;
            model
        };

        for build in [Build::Male, Build::Female] {
            let model = check(build.model());
            // A body is rigged, and every mesh in it is bound to the skeleton.
            assert!(
                model.joints >= 17,
                "{:?} has {} joints — the rig is missing or partial",
                build,
                model.joints
            );
            // And it carries the parts this file knows how to paint.
            let named: Vec<&str> = model.meshes.iter().map(|(name, _)| name.as_str()).collect();
            for part in ["skin", "clothes", "sclera", "eyes", "pupil"] {
                assert!(
                    named.contains(&part),
                    "{:?} has no mesh called `{part}` — nothing would paint it",
                    build
                );
            }
        }

        for hair in [
            Hair::None,
            Hair::Crop,
            Hair::Bob,
            Hair::Tail,
            Hair::Braids,
            Hair::Curls,
        ] {
            // A bare head names no file, which is a real choice and not a gap.
            let Some(road) = hair.model() else {
                continue;
            };
            let model = check(road);
            assert!(
                model.meshes.iter().any(|(name, _)| name == "hair"),
                "{hair:?} has no mesh called `hair`"
            );
        }

        for hat in [Hat::None, Hat::Cap] {
            if let Some(road) = hat.model() {
                let model = check(road);
                assert!(
                    model.meshes.iter().any(|(name, _)| name == "hat"),
                    "{hat:?} has no mesh called `hat`"
                );
            }
        }
        assert!(looked >= 8, "only {looked} models were checked");
    }

    /// Every build's authored height matches the file it names.
    ///
    /// # The number that silently resizes the warden
    ///
    /// A model is scaled from `authored_height` up to `TALL`, so if that number is
    /// wrong the warden is the wrong size — and nothing else complains. A figure
    /// authored a unit high and assumed to be 1.7 m comes out at three metres, and
    /// the only symptom is that the world suddenly looks small.
    ///
    /// The file knows its own height, so the claim is checked against it rather than
    /// trusted. This is what catches a replaced asset: somebody sends a new export
    /// at a different scale and this fails instead of the game looking odd.
    #[test]
    fn every_build_knows_how_tall_its_file_is() {
        let folder = std::path::Path::new("assets/models");
        for build in [Build::Ranger, Build::Male, Build::Female] {
            let file = folder.join(build.model().trim_start_matches("models/"));
            let Ok(bytes) = std::fs::read(&file) else {
                panic!("{build:?} names {} and it is not there", build.model());
            };
            let model = crate::models::inspect(&bytes)
                .unwrap_or_else(|why| panic!("{build:?}: {why}"));
            let stands = model.high[1] - model.low[1];
            let said = build.authored_height();
            assert!(
                (stands - said).abs() < 0.05,
                "{build:?} says its file is {said:.2} m and the file is {stands:.2} m —                  the warden would come out {:.2} m instead of {TALL:.2}",
                TALL * stands / said
            );
            // And it is rigged, because the warden walks.
            assert!(
                model.joints >= 17,
                "{build:?} has {} joints — it cannot be animated",
                model.joints
            );
        }
    }

    /// A dressed build brings its own hair, and an undressed one can be tinted.
    ///
    /// Two facts that have to agree with the file. A model with ONE material cannot
    /// have its skin and eyes coloured separately — there is nothing to colour apart
    /// — so it must be the sort that arrives dressed. And a model whose parts ARE
    /// separable must not be, or the creator has nothing to work with.
    #[test]
    fn a_dressed_build_is_one_piece_and_an_undressed_one_is_not() {
        let folder = std::path::Path::new("assets/models");
        for build in [Build::Ranger, Build::Male, Build::Female] {
            let file = folder.join(build.model().trim_start_matches("models/"));
            let Ok(bytes) = std::fs::read(&file) else {
                continue;
            };
            let model = crate::models::inspect(&bytes).expect("a body should read");
            let paintable = model
                .meshes
                .iter()
                .filter(|(name, _)| Look::default().colour_of(name).is_some())
                .count();
            if build.dressed() {
                assert_eq!(
                    paintable, 0,
                    "{build:?} says it arrives dressed but has {paintable} parts the                      painter would colour — one of the two is wrong"
                );
            } else {
                assert!(
                    paintable >= 4,
                    "{build:?} is meant to be tintable and only {paintable} of its                      meshes have a colour"
                );
            }
        }
    }

    /// A warden faces the way the game means by forward.
    ///
    /// # It walked backwards
    ///
    /// Everything in `dev/art/people.py` is modelled toward -Y, because that is
    /// where Blender's own front view looks from. The glTF Y-up conversion turns
    /// Blender -Y into +Z, and the game's forward is -Z — so every figure came out
    /// walking backwards, and nothing said so until somebody watched one walk.
    ///
    /// The eyes are the instrument: they are the one part of a body that is only ever
    /// on the front. If their middle is not at negative Z, the figure is back to
    /// front however plausible it looks standing still.
    #[test]
    fn a_warden_faces_forward() {
        let folder = std::path::Path::new("assets/models");
        for build in [Build::Male, Build::Female] {
            let file = folder.join(build.model().trim_start_matches("models/"));
            let Ok(bytes) = std::fs::read(&file) else {
                continue;
            };
            let model = crate::models::inspect(&bytes).expect("a body should read");
            let eyes = model
                .part("eyes")
                .unwrap_or_else(|| panic!("{build:?} has no eyes to tell its front by"));
            let middle = (eyes.0[2] + eyes.1[2]) * 0.5;
            assert!(
                middle < -0.02,
                "{build:?} has its eyes at z {middle:+.3}, so it is facing +Z —                  the game's forward is -Z and it would walk backwards"
            );
        }
    }

    /// And so does anything worn on the head, or the peak points behind them.
    #[test]
    fn a_hat_faces_the_same_way_as_the_face_under_it() {
        let folder = std::path::Path::new("assets/models");
        for hat in [Hat::Cap] {
            let Some(road) = hat.model() else {
                continue;
            };
            let file = folder.join(road.trim_start_matches("models/"));
            let Ok(bytes) = std::fs::read(&file) else {
                continue;
            };
            let model = crate::models::inspect(&bytes).expect("a hat should read");
            let worn = model.part("hat").expect("a hat has a mesh called `hat`");
            // A cap reaches further forward than back: that is its peak.
            let forward = -worn.0[2];
            let behind = worn.1[2];
            assert!(
                forward > behind,
                "{hat:?} reaches {behind:.3} m back and {forward:.3} m forward —                  its peak is pointing the wrong way"
            );
        }
    }

    /// Every part of the model has a colour, and nothing else does.
    ///
    /// The painter leaves a mesh alone when it does not recognise the name, which is
    /// right — the world is full of named meshes — but it means a RENAMED part would
    /// silently keep whatever the file shipped with. This is the other half of the
    /// check above: the names agree in both directions.
    #[test]
    fn the_painter_knows_every_part_and_no_others() {
        let look = Look::default();
        for part in ["skin", "clothes", "sclera", "eyes", "pupil", "hair", "hat"] {
            assert!(
                look.colour_of(part).is_some(),
                "the painter has no colour for `{part}`"
            );
        }
        for stranger in ["boot", "peak", "Cube", "", "torso"] {
            assert!(
                look.colour_of(stranger).is_none(),
                "the painter would paint `{stranger}`, which is not a part it owns"
            );
        }
    }
}
