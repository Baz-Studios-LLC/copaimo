//! A picture to build against.
//!
//! # What it is for
//!
//! Building something you have a picture of means reading proportions off the
//! picture and reproducing them in parts — which is exactly the kind of measuring
//! nobody is good at by eye. Standing the picture *in the room, at a stated size*
//! turns that into tracing: put a wall where the wall is.
//!
//! # It has to be honest about scale
//!
//! The one thing that would make this worse than useless is a picture at an
//! unknown size. Trace a cottage off an image scaled to whatever the loader felt
//! like and you get a cottage of no particular dimensions, discovered later when it
//! stands next to something built properly.
//!
//! So the width is in **metres**, it is stated on screen, and it is changed in
//! module-sized steps — because the thing a maker actually wants is "make this
//! picture four modules wide", after which every wall in it lands on the lattice.
//!
//! # Upright or flat
//!
//! Standing up, it is an elevation: the front of a house, traced wall by wall.
//! Lying down, it is a plan: a footprint to lay floors on. Both are worth having
//! and they are the same quad turned ninety degrees, so both are here.

use bevy::prelude::*;
use std::path::{Path, PathBuf};

use crate::build::kit::MODULE;

/// Where a maker drops pictures to build against.
pub const REFERENCE_DIR: &str = "assets/reference";

/// What is on the wall of the bench.
#[derive(Resource, Default)]
pub struct Reference {
    /// Every picture found, in a settled order.
    found: Vec<PathBuf>,
    /// Which one is up, or none.
    showing: Option<usize>,
    /// How wide it is drawn, in metres.
    wide: f32,
    /// Standing up as an elevation, or lying down as a plan.
    upright: bool,
    /// How far it is pushed back, so the work can stand in front of it.
    back: f32,
    /// How solid it is drawn.
    fade: f32,
}

/// The quad it is drawn on.
#[derive(Component)]
pub struct Sheet;

impl Reference {
    /// Finds the pictures once, when the bench opens.
    pub fn look(&mut self) {
        self.found.clear();
        let Ok(entries) = std::fs::read_dir(Path::new(REFERENCE_DIR)) else {
            // No folder is the ordinary case, not a fault: most work needs no
            // picture, and a warning every time the bench opened would be noise.
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let known = path
                .extension()
                .and_then(|kind| kind.to_str())
                .is_some_and(|kind| {
                    matches!(kind.to_ascii_lowercase().as_str(), "png" | "jpg" | "jpeg")
                });
            if known {
                self.found.push(path);
            }
        }
        // Sorted, so which picture `I` reaches first does not depend on the order
        // the filesystem happened to hand them over.
        self.found.sort();
    }

    /// What is up, for saying so on screen.
    pub fn said(&self) -> String {
        match self.showing.and_then(|at| self.found.get(at)) {
            Some(path) => format!(
                "{} at {:.1} m {}",
                path.file_name().unwrap_or_default().to_string_lossy(),
                self.wide,
                if self.upright { "upright" } else { "flat" }
            ),
            None if self.found.is_empty() => format!("none in {REFERENCE_DIR}"),
            None => format!("{} to hand", self.found.len()),
        }
    }

    /// Steps to the next picture, then to none, then round again.
    ///
    /// Through nothing deliberately: a maker who has traced what they needed wants
    /// the picture GONE, and cycling straight from the last to the first would mean
    /// pressing a key as many times as there are files to get an empty bench.
    pub fn next(&mut self) {
        if self.found.is_empty() {
            return;
        }
        self.showing = match self.showing {
            None => Some(0),
            Some(at) if at + 1 < self.found.len() => Some(at + 1),
            Some(_) => None,
        };
    }

    fn path(&self) -> Option<&PathBuf> {
        self.showing.and_then(|at| self.found.get(at))
    }

    /// The picture a firing would be made from.
    ///
    /// The same one that is up, so what a maker sends is what they can see. A key
    /// that fired "the first picture in the folder" while a different one was on
    /// the wall would spend money on the wrong image, and nobody would find out for
    /// two minutes.
    pub fn chosen(&self) -> Option<&PathBuf> {
        self.path()
    }
}

/// Sets the sheet up when the bench opens.
pub fn open(mut reference: ResMut<Reference>) {
    reference.look();
    reference.wide = MODULE * 4.0;
    reference.back = MODULE * 2.0;
    reference.upright = true;
    reference.fade = 0.55;
    reference.showing = None;
}

/// The keys, and the quad they put up.
///
/// Rebuilt from scratch whenever anything about it changes, like everything else
/// in this room — one picture is one quad, so there is nothing to be gained by
/// being clever and something to lose by having two code paths.
pub fn show(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    asset_server: Res<AssetServer>,
    mut reference: ResMut<Reference>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    sheets: Query<Entity, With<Sheet>>,
) {
    let mut moved = false;
    if keys.just_pressed(KeyCode::KeyI) {
        reference.next();
        moved = true;
    }
    if keys.just_pressed(KeyCode::KeyU) {
        reference.upright = !reference.upright;
        moved = true;
    }
    // In module steps, because "four modules wide" is the thing a maker wants and
    // it puts every wall in the picture on the lattice.
    if keys.just_pressed(KeyCode::Period) {
        reference.wide += MODULE;
        moved = true;
    }
    if keys.just_pressed(KeyCode::Comma) {
        reference.wide = (reference.wide - MODULE).max(MODULE);
        moved = true;
    }
    if keys.just_pressed(KeyCode::KeyK) {
        reference.fade = if reference.fade > 0.3 { 0.2 } else { 0.55 };
        moved = true;
    }
    if !moved && !sheets.is_empty() {
        return;
    }
    if !moved && reference.path().is_none() {
        return;
    }

    for entity in &sheets {
        commands.entity(entity).despawn();
    }
    let Some(path) = reference.path().cloned() else {
        return;
    };

    // Loaded through the asset server by a path relative to `assets`, which is
    // what it takes; the folder constant carries the prefix so a maker only ever
    // sees the folder they dropped the file in.
    let relative = path
        .strip_prefix("assets")
        .unwrap_or(&path)
        .to_string_lossy()
        .replace('\\', "/");
    let picture: Handle<Image> = asset_server.load(relative);

    // Kept in proportion is impossible before the image has loaded, and guessing
    // at four-by-three would be wrong for most things. A square is the honest
    // default: it is obviously a placeholder ratio rather than a plausible wrong
    // one, and the width — the number that matters — is exact from the first frame.
    let tall = reference.wide;
    let quad = meshes.add(Rectangle::new(reference.wide, tall));

    let skin = materials.add(StandardMaterial {
        base_color_texture: Some(picture),
        base_color: Color::srgba(1.0, 1.0, 1.0, reference.fade),
        alpha_mode: AlphaMode::Blend,
        // Unlit, and it matters: a reference is a drawing, not a surface in the
        // room. Lighting it would make it darker on one side and a maker would be
        // tracing a shadow.
        unlit: true,
        double_sided: true,
        cull_mode: None,
        ..default()
    });

    let stance = if reference.upright {
        // Standing behind the work, facing the default view.
        Transform::from_xyz(0.0, tall * 0.5, -reference.back)
    } else {
        // Lying on the floor, just above it so it does not fight the grid.
        Transform::from_xyz(0.0, 0.01, 0.0)
            .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))
    };

    commands.spawn((
        super::OfBench,
        Sheet,
        Mesh3d(quad),
        MeshMaterial3d(skin),
        stance,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stepping_through_the_pictures_passes_through_none() {
        // A maker who has traced what they needed wants the picture GONE. Cycling
        // straight from the last back to the first would mean pressing a key as
        // many times as there are files to get an empty bench.
        let mut reference = Reference {
            found: vec!["a.png".into(), "b.png".into()],
            ..Default::default()
        };
        assert!(reference.path().is_none(), "nothing is up to begin with");
        reference.next();
        assert_eq!(reference.path(), Some(&PathBuf::from("a.png")));
        reference.next();
        assert_eq!(reference.path(), Some(&PathBuf::from("b.png")));
        reference.next();
        assert!(reference.path().is_none(), "the way back to an empty bench");
        reference.next();
        assert_eq!(reference.path(), Some(&PathBuf::from("a.png")));
    }

    #[test]
    fn an_empty_folder_is_not_a_fault() {
        // Most work needs no picture. A warning every time the bench opened, or a
        // key that appeared broken, would both be worse than silence.
        let mut reference = Reference::default();
        reference.look();
        reference.next();
        assert!(reference.path().is_none());
        assert!(reference.said().contains(REFERENCE_DIR));
    }

    #[test]
    fn the_width_is_stated_in_metres_and_never_reaches_nothing() {
        // The one thing that would make a reference worse than useless is a picture
        // at an unknown size: trace a cottage off it and you get a cottage of no
        // particular dimensions, found out later beside something built properly.
        let mut reference = Reference {
            found: vec!["a.png".into()],
            wide: MODULE * 4.0,
            ..Default::default()
        };
        reference.next();
        // Four MODULES is six metres, and the readout says metres — which is the
        // whole point of it. Getting those two mixed up is exactly the mistake a
        // stated scale exists to stop, and this test made it first time round.
        assert!(reference.said().contains("6.0 m"), "{}", reference.said());

        // Shrinking it stops at one module rather than passing through zero and
        // out the other side.
        for _ in 0..20 {
            reference.wide = (reference.wide - MODULE).max(MODULE);
        }
        assert!((reference.wide - MODULE).abs() < 1.0e-6);
    }
}
