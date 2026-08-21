//! Copaimo — a monster-companion adventure game.
//!
//! Stage one is the world itself: a large, finite, walkable landmass generated
//! from a source map image. Ranching, monsters, cities and guild exams all sit
//! on top of this later, so the priority here is a world with real geography —
//! coastlines, mountain ranges, biomes — at a scale that feels like a journey.
//!
//! The main menu leads to two separate modes: walking the world as the warden,
//! and the terrain tool for sculpting its shape.
//!
//! Each concern is a Bevy plugin in its own module:
//!   * `states` — app states (menu / playing / editing) and cursor policy
//!   * `world`  — terrain generation, chunk streaming, the sea
//!   * `player` — the warden and their character controller
//!   * `camera` — third-person orbit rig, plus free-fly
//!   * `menu`   — the title screen
//!   * `save`   — what a player has done, kept between sittings
//!
//!   * `editor` — the terrain tool, driving `terrain-core`'s brush
//!   * `bench`  — the workbench: buildings and fences, piece by piece
//!   * `build`  — buildings baked at Opificium's builder, stood on the ground
//!   * `sky`    — sun, ambient light, fog
//!   * `shade`  — the material everything solid is made of, and cloud shadows
//!   * `hud`    — the F3 debug overlay
//!
//! The world generation, the brush and the trees all live in `terrain-core`,
//! which Opificium's terrain bench links too — so ground shaped in either place
//! is shaped identically. See `DESIGN.md`.

// Two clippy lints fire on the shape of Bevy itself rather than on anything
// wrong here, and contorting the code to quiet them would cost more than they
// are worth. A system's parameters ARE its dependencies, so a system that
// touches eight things takes eight arguments; and a filtered `Query` is a type
// by construction, so naming an alias for each one buries what it selects.
#![allow(clippy::too_many_arguments, clippy::type_complexity)]

#[cfg(feature = "tools")]
mod bench;
mod build;
mod camera;
mod config;
#[cfg(feature = "tools")]
mod editor;
#[cfg(feature = "tools")]
mod tools;
#[cfg(feature = "tools")]
mod hud;
/// How a warden looks, and painting it onto the model.
mod look;
mod menu;
// Reads model files: the gate that keeps a badly exported one out of the game, and
// the reader that turns a GLB into geometry the world can weld into a chunk.
//
// Trees do NOT come through here — they are drawn as objects and use Bevy's own
// asynchronous loader (see `world::authored`). Rocks and ground cover do, because a
// chunk's worth of them is welded into one mesh on a background thread with no
// access to Bevy's assets, the moment the chunk streams in.
mod models;
mod player;
mod save;
mod typeface;
mod shade;
mod sky;
mod states;
mod util;
mod world;

use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::prelude::*;

/// Puts the crest on the window, the task bar and the dock.
///
/// # Why this is not just the packaging's job
///
/// The `.ico` compiled into the executable is what Explorer draws for the FILE.
/// What a running window shows is a separate thing entirely, set through the
/// window system — so a build with only the compiled icon has a proper icon on
/// disk and the default one while it is open, which is the state anybody actually
/// looks at for hours at a time.
///
/// Read straight off the disk rather than through the asset server: the window
/// wants its icon before the first frame, and an asset is not loaded until after
/// one. A missing file leaves the default icon and says so once, because a game
/// that will not start over an icon is worse than one with a plain icon.
fn wear_the_icon(windows: NonSend<bevy::winit::WinitWindows>) {
    const ICON: &str = "assets/Title/icon.png";

    let found = std::fs::read(ICON).or_else(|_| {
        // Beside the binary as well as beside the working directory — a packaged
        // build is launched from anywhere, and macOS launches one from `/`.
        std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(|at| at.join(ICON)))
            .map_or_else(|| Err(std::io::Error::other("no exe folder")), std::fs::read)
    });
    let Ok(bytes) = found else {
        warn!("{ICON} not found; the window keeps the default icon");
        return;
    };
    let Ok(picture) = image::load_from_memory(&bytes) else {
        warn!("{ICON} is not a picture the window can wear");
        return;
    };
    let picture = picture.into_rgba8();
    let (wide, tall) = picture.dimensions();
    let Ok(icon) = winit::window::Icon::from_rgba(picture.into_raw(), wide, tall) else {
        warn!("{ICON} could not be turned into a window icon");
        return;
    };
    for window in windows.windows.values() {
        window.set_window_icon(Some(icon.clone()));
    }
}

/// Where this build's `assets/` folder is.
///
/// Beside the working directory when there is one there — which is how it is run
/// from the repository — and otherwise beside the binary, which is how every
/// packaged build is laid out. Falls back to the plain name so a missing folder
/// is Bevy's ordinary "not found" rather than a path built out of nothing.
pub fn asset_root() -> String {
    // ABSOLUTE, and that is the whole trick. Bevy joins this onto a base
    // directory of its own — the executable's folder — so a relative "assets"
    // resolves under `target/debug/` however plainly it names the right folder.
    // An absolute path replaces the base instead of being appended to it.
    if let Ok(here) = std::path::Path::new("assets").canonicalize() {
        if here.is_dir() {
            // `canonicalize` on Windows returns a `\?\` extended path, which
            // Bevy's own path handling does not take. Trimmed back to a plain one.
            let said = here.to_string_lossy().to_string();
            return said.strip_prefix(r"\?\").unwrap_or(&said).to_string();
        }
    }
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|at| at.join("assets")))
        .filter(|road| road.is_dir())
        .map(|road| road.to_string_lossy().to_string())
        .unwrap_or_else(|| "assets".to_string())
}

/// One of this build's asset FILES, wherever the assets folder turned out to be.
///
/// # A packaged mac build opened a different world and looked fine doing it
///
/// The world's layers — the map, the sculpting, the woods, the surfacing, the
/// countries, what is placed — are read and written with plain `std::fs`, so a
/// bare `"assets/world/edits.bin"` resolves against the **working directory**.
/// That is the repository root when the game is run from source, and `/` when
/// macOS launches a `.app` bundle, which is how the launcher starts it. Nothing
/// errors: the heightmap falls back to a procedural world and every painted layer
/// loads empty, so the shipped mac build drew a world nobody had made.
///
/// [`asset_root`] already carries this rule for Bevy's own asset server and
/// `wear_the_icon` carries it for the window icon. This is the one for everything
/// read by hand, so all three now answer the same question the same way.
pub fn asset_file(relative: &str) -> std::path::PathBuf {
    which_asset_file(
        relative,
        std::path::Path::new("assets").is_dir(),
        std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(std::path::Path::to_path_buf))
            .as_deref(),
    )
}

/// The decision alone, with the world's answers passed in.
///
/// Split out because the inputs are process-wide — the working directory and the
/// running executable — and a test cannot change either without reaching into
/// every other test running beside it. This way both cases are actually tested
/// rather than argued about.
fn which_asset_file(
    relative: &str,
    cwd_has_assets: bool,
    exe_folder: Option<&std::path::Path>,
) -> std::path::PathBuf {
    if cwd_has_assets {
        return std::path::PathBuf::from(relative);
    }
    match exe_folder {
        Some(folder) => folder.join(relative),
        // Nothing better to say than the plain name: a missing folder should be
        // an ordinary "not found" rather than a path built out of nothing.
        None => std::path::PathBuf::from(relative),
    }
}

#[cfg(test)]
mod assets {
    use super::*;
    use std::path::Path;

    #[test]
    fn an_asset_is_found_beside_the_binary_when_the_working_directory_has_none() {
        // Run from the repository: the working directory is the answer, because
        // that is where a maker's own sculpting lives.
        assert_eq!(
            which_asset_file("assets/world/edits.bin", true, Some(Path::new("/somewhere/else"))),
            Path::new("assets/world/edits.bin")
        );

        // Launched from a bundle: `/` has no assets folder, so the one beside the
        // binary is the world this build shipped with. Asserted as "under the
        // binary's own folder" rather than against a literal — a POSIX-looking
        // path is not absolute on Windows, and the first draft of this test failed
        // on that rather than on anything about the rule.
        let folder = std::env::temp_dir().join("Copaimo.app/Contents/MacOS");
        let beside = which_asset_file("assets/world/edits.bin", false, Some(&folder));
        assert_eq!(beside, folder.join("assets/world/edits.bin"));
        assert!(
            beside.starts_with(&folder),
            "a bundled path has to sit under the binary's own folder"
        );

        // And with nothing to go on, the plain name — an ordinary "not found".
        assert_eq!(
            which_asset_file("assets/world/edits.bin", false, None),
            Path::new("assets/world/edits.bin")
        );
    }
}

fn main() {
    let mut app = App::new();
    app
        .add_plugins(DefaultPlugins.set(AssetPlugin {
            // Where `assets/` actually is, worked out once and told to Bevy —
            // rather than left to a default that is right in some launches and
            // wrong in others.
            //
            // The default resolves beside the EXECUTABLE, which is correct for a
            // packaged build and wrong for a binary run out of `target/`. That
            // mismatch is not merely inconvenient: the font check looked beside
            // the working directory, found the file, and logged that the font was
            // in use — while the asset server looked beside the binary, found
            // nothing, and everything drew in the fallback face. A check that
            // passes while the thing it checks fails is worse than no check.
            file_path: asset_root(),
            ..default()
        })
        .set(WindowPlugin {
            primary_window: Some(Window {
                title: "Copaimo — The Wardens Guild".into(),
                resolution: (1600.0_f32, 900.0_f32).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .add_systems(Startup, wear_the_icon)
        // The font, before anything that sets type in it.
        //
        // Registered HERE and not by a tool, which is the whole point of moving it
        // out of `tools`: a release has no tools, so a loader living there left the
        // title screen — the one screen every player sees — asking for a resource
        // that nothing had inserted.
        .add_systems(Startup, typeface::load_ui_font)
        .init_resource::<typeface::UiFont>()
        .add_plugins((
            states::StatesPlugin,
            // Before anything that builds a material, because it is what
            // registers the material the whole world is made of.
            shade::ShadePlugin,
            // World first so the terrain resource exists before anything that
            // needs to ask how high the ground is.
            world::WorldPlugin,
            sky::SkyPlugin,
            // After the world: buildings stand on ground it decides the height
            // of, at sites it decides the places of.
            build::BuildingPlugin,
            player::PlayerPlugin,
            camera::CameraPlugin,
            menu::MenuPlugin,
            save::SavePlugin,

        ));

    // The maker's tools, and only in a maker's build.
    //
    // Stripped whole from a release rather than hidden behind a menu nobody
    // clicks: a shipped terrain brush is a way to break a save, and a shipped
    // kiln is code that can spend somebody's credits. Neither belongs in a
    // player's hands, and the surest way for them not to be there is for them not
    // to be compiled.
    #[cfg(feature = "tools")]
    app.add_plugins((editor::EditorPlugin, bench::BenchPlugin, hud::HudPlugin));

    app.run();
}
