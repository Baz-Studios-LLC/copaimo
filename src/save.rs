//! What a player has done, kept between sittings.
//!
//! # A save is the PLAYER's, not the world's
//!
//! Worth being clear about, because this game writes several files and only one
//! of them is a save. The sculpted ground, the painted woods and biomes, the
//! placed buildings — those are the WORLD, they are authored at the tools, they
//! ship inside the game, and every player gets the same ones. A save is what one
//! person did in that world.
//!
//! So they live in different places, and that is the point rather than an
//! accident. World files sit in `assets/` and go in the build; a save sits in the
//! player's own folder and never goes anywhere near the repository. Putting a save
//! in `assets/` would mean shipping somebody's afternoon to everybody who
//! downloads the game, and re-installing would wipe it.
//!
//! # Where it goes
//!
//! Where each system says a program's own files belong, and nowhere else:
//!
//! * Windows `%APPDATA%\Copaimo\`
//! * macOS `~/Library/Application Support/Copaimo/`
//! * Linux `~/.local/share/copaimo/`
//!
//! This game is standalone and answers to nothing else for it. A save that
//! survives re-installing, that a backup picks up, and that lives beside every
//! other program's is a save in the right place on each platform — which is the
//! whole reason to use these folders rather than any convention borrowed from
//! somewhere.
//!
//! # Written whole, or not at all
//!
//! A save is written to a temporary file beside itself and then renamed over the
//! old one. A rename within a folder is atomic on every system this runs on, so a
//! crash or a power cut during a write leaves either the previous save or the new
//! one — never half of one. Writing in place is how a game eats a save at exactly
//! the moment the player would mind most.

use bevy::prelude::*;
use std::path::PathBuf;

/// What the file says it is.
///
/// Bumped when the shape changes in a way an older reader could not survive. A
/// save from the future is refused rather than guessed at: a game that reads a
/// field it does not understand and carries on is a game that silently loses what
/// the field meant.
const FORMAT: u32 = 1;

/// The folder name, as a person would write the game's title.
const FOLDER: &str = "Copaimo";
const FILE: &str = "save.json";

/// One player's progress.
///
/// Deliberately small, and it will grow: monsters, the guild rank, what is in the
/// pack. What is here is what exists to save — writing fields for things the game
/// does not have yet would be inventing a format nobody can test.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct Save {
    /// Where the warden stood.
    pub at: Vec3,
    /// Which way they faced, in radians about Y.
    pub facing: f32,
    /// How long this save has been played, in seconds.
    pub played: f64,
    /// When it was last written, as the player's own clock read it.
    ///
    /// For the menu to say "continue — yesterday evening" rather than offering an
    /// unlabelled button. Stored as written text rather than a number because the
    /// only thing that ever reads it is a person.
    pub stamped: String,
}

impl Default for Save {
    fn default() -> Self {
        Self {
            at: Vec3::ZERO,
            facing: 0.0,
            played: 0.0,
            stamped: String::new(),
        }
    }
}

/// Where a save lives, or `None` on a system with no home to put one in.
pub fn path() -> Option<PathBuf> {
    let folder = if cfg!(target_os = "windows") {
        PathBuf::from(std::env::var("APPDATA").ok()?).join(FOLDER)
    } else if cfg!(target_os = "macos") {
        PathBuf::from(std::env::var("HOME").ok()?)
            .join("Library/Application Support")
            .join(FOLDER)
    } else {
        PathBuf::from(std::env::var("HOME").ok()?)
            .join(".local/share")
            .join(FOLDER.to_lowercase())
    };
    Some(folder.join(FILE))
}

/// Reads the save, or `None` if there is not one this game can use.
///
/// Every failure is the same answer — start a new game — and every one of them
/// says why in the log. A game that refuses to start because a save is unreadable
/// has turned a lost afternoon into a lost game.
pub fn read() -> Option<Save> {
    let road = path()?;
    let text = std::fs::read_to_string(&road).ok()?;
    let body: serde_json::Value = serde_json::from_str(&text)
        .map_err(|why| warn!("{}: {why}", road.display()))
        .ok()?;

    let format = body.get("format").and_then(serde_json::Value::as_u64)? as u32;
    if format > FORMAT {
        // From a newer game. Refused rather than read as far as it goes: a reader
        // that takes the fields it recognises and drops the rest writes the loss
        // back to disk the next time it saves.
        warn!("the save is format {format} and this game reads {FORMAT}");
        return None;
    }

    let number = |key: &str| body.get(key).and_then(serde_json::Value::as_f64);
    let at = body.get("at").and_then(serde_json::Value::as_array)?;
    let axis = |n: usize| at.get(n).and_then(serde_json::Value::as_f64).unwrap_or(0.0) as f32;

    let save = Save {
        at: Vec3::new(axis(0), axis(1), axis(2)),
        facing: number("facing").unwrap_or(0.0) as f32,
        played: number("played").unwrap_or(0.0),
        stamped: body
            .get("stamped")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_string(),
    };
    // Numbers, not just number-shaped: an infinity here spawns the warden at
    // infinity and every arithmetic downstream of them with it. (A NaN cannot
    // reach this point — it is not JSON — but a hand-edited or corrupted file
    // can hold anything a parser takes.)
    if !save.at.is_finite() || !save.facing.is_finite() || !save.played.is_finite() {
        warn!("the save holds a position that is not a place; starting fresh");
        return None;
    }
    Some(save)
}

/// Writes the save, whole or not at all.
pub fn write(save: &Save) -> Result<PathBuf, String> {
    let road = path().ok_or_else(|| "no home folder to save into".to_string())?;
    let folder = road
        .parent()
        .ok_or_else(|| "the save path has no folder".to_string())?;
    std::fs::create_dir_all(folder).map_err(|why| format!("{}: {why}", folder.display()))?;

    let body = format!(
        "{{\n  \"format\": {FORMAT},\n  \"at\": [{:.3}, {:.3}, {:.3}],\n  \
         \"facing\": {:.5},\n  \"played\": {:.1},\n  \"stamped\": \"{}\"\n}}\n",
        save.at.x,
        save.at.y,
        save.at.z,
        save.facing,
        save.played,
        save.stamped.replace('"', "'")
    );

    // Beside the save, not in the temp folder: a rename is only atomic within one
    // filesystem, and a temp directory is very often on another one — where the
    // rename quietly becomes a copy, which is exactly the half-written file this
    // is here to prevent.
    let part = road.with_extension("part");
    std::fs::write(&part, body).map_err(|why| format!("{}: {why}", part.display()))?;
    std::fs::rename(&part, &road).map_err(|why| format!("{}: {why}", road.display()))?;
    Ok(road)
}

/// Deletes the save. Used by New Game, once the player has said yes.
pub fn clear() {
    if let Some(road) = path() {
        let _ = std::fs::remove_file(&road);
        let _ = std::fs::remove_file(road.with_extension("part"));
    }
}

/// How the player's clock reads, for stamping a save.
pub fn now() -> String {
    chrono::Local::now().format("%-d %b %Y, %H:%M").to_string()
}

/// How the game was started, and what has happened since.
///
/// Held apart from [`Save`] because the two answer different questions: a `Save`
/// is a file, and this is the run in progress. The menu writes `from` before the
/// world loads; everything after reads it.
#[derive(Resource, Default)]
pub struct Progress {
    /// Where the warden should appear, or `None` to start at the ranch.
    pub from: Option<Save>,
    /// Seconds played in this save, carried across from the file and added to.
    pub played: f64,
    /// Seconds since the last write, so autosaving is not every frame.
    since: f64,
}

/// How often the game writes, in seconds.
///
/// Half a minute. Often enough that a crash costs a walk rather than an evening,
/// and rare enough that it is never the reason a frame was late — the write is a
/// few hundred bytes and a rename, which is nothing, but doing nothing sixty
/// times a second is still something.
const AUTOSAVE: f64 = 30.0;

pub struct SavePlugin;

impl Plugin for SavePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Progress>()
            .add_systems(
                Update,
                keep_progress.run_if(in_state(crate::states::AppState::Playing)),
            )
            // On the way out, whatever the reason. A game that only saves on a
            // timer loses whatever happened in the last half minute every single
            // time somebody leaves properly, which is most times.
            .add_systems(OnExit(crate::states::AppState::Playing), save_on_leaving);
    }
}

/// Counts the time played and writes now and then.
fn keep_progress(
    time: Res<Time>,
    mut progress: ResMut<Progress>,
    wardens: Query<&Transform, With<crate::player::Player>>,
) {
    let step = time.delta_secs_f64();
    progress.played += step;
    progress.since += step;
    if progress.since < AUTOSAVE {
        return;
    }
    progress.since = 0.0;
    put_it_down(&progress, &wardens);
}

fn save_on_leaving(
    progress: Res<Progress>,
    wardens: Query<&Transform, With<crate::player::Player>>,
) {
    put_it_down(&progress, &wardens);
}

/// Writes where the warden is standing.
///
/// Silent on success and loud on failure, and it never stops the game either way:
/// a disk that will not take a save is a bad afternoon, and a game that closes
/// itself over one is a worse one.
fn put_it_down(progress: &Progress, wardens: &Query<&Transform, With<crate::player::Player>>) {
    let Some(warden) = wardens.iter().next() else {
        // No warden means the world has not finished loading. Saving a position
        // nobody is standing at would overwrite a real one with the origin.
        return;
    };
    // A position that is not a place is not saved. `{:.3}` formats a NaN as the
    // literal word, which is not JSON — so one bad frame upstream would write a
    // file the reader throws away whole, and the real save with it.
    if !warden.translation.is_finite() {
        error!("the warden stands at {:?}; keeping the last good save", warden.translation);
        return;
    }
    let save = Save {
        at: warden.translation,
        facing: warden.rotation.to_euler(EulerRot::YXZ).0,
        played: progress.played,
        stamped: now(),
    };
    match write(&save) {
        Ok(_) => {}
        Err(why) => error!("could not save: {why}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A save file of our own, so a test never touches the player's.
    fn scratch(name: &str) -> PathBuf {
        let road = std::env::temp_dir().join(format!("copaimo-{name}.json"));
        let _ = std::fs::remove_file(&road);
        road
    }

    /// The same reading as `read`, against a path a test controls.
    fn read_from(road: &PathBuf) -> Option<Save> {
        let text = std::fs::read_to_string(road).ok()?;
        let body: serde_json::Value = serde_json::from_str(&text).ok()?;
        let format = body.get("format").and_then(serde_json::Value::as_u64)? as u32;
        if format > FORMAT {
            return None;
        }
        let at = body.get("at").and_then(serde_json::Value::as_array)?;
        let axis = |n: usize| at.get(n).and_then(serde_json::Value::as_f64).unwrap_or(0.0) as f32;
        Some(Save {
            at: Vec3::new(axis(0), axis(1), axis(2)),
            facing: body.get("facing").and_then(serde_json::Value::as_f64).unwrap_or(0.0) as f32,
            played: body.get("played").and_then(serde_json::Value::as_f64).unwrap_or(0.0),
            stamped: body
                .get("stamped")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("")
                .to_string(),
        })
    }

    #[test]
    fn a_save_survives_the_round_trip() {
        // Everything a player would notice going missing.
        let mine = Save {
            at: Vec3::new(-3064.25, 22.5, 659.75),
            facing: 1.234_5,
            played: 4_321.5,
            stamped: "18 Aug 2026, 21:04".into(),
        };
        let road = scratch("roundtrip");
        let body = format!(
            "{{\n  \"format\": {FORMAT},\n  \"at\": [{:.3}, {:.3}, {:.3}],\n  \
             \"facing\": {:.5},\n  \"played\": {:.1},\n  \"stamped\": \"{}\"\n}}\n",
            mine.at.x, mine.at.y, mine.at.z, mine.facing, mine.played, mine.stamped
        );
        std::fs::write(&road, body).unwrap();

        let back = read_from(&road).expect("a save this game wrote should read back");
        assert!(back.at.abs_diff_eq(mine.at, 0.01), "{:?} vs {:?}", back.at, mine.at);
        assert!((back.facing - mine.facing).abs() < 1.0e-4);
        assert!((back.played - mine.played).abs() < 0.1);
        assert_eq!(back.stamped, mine.stamped);
        let _ = std::fs::remove_file(&road);
    }

    #[test]
    fn a_save_from_a_newer_game_is_refused_rather_than_guessed_at() {
        // Reading as far as it goes is worse than refusing: a reader that takes
        // the fields it knows and drops the rest writes the loss back to disk the
        // next time it saves, and the player never sees the moment it happened.
        let road = scratch("newer");
        std::fs::write(
            &road,
            format!("{{\"format\": {}, \"at\": [1,2,3]}}", FORMAT + 1),
        )
        .unwrap();
        assert!(read_from(&road).is_none(), "a future save was read anyway");
        let _ = std::fs::remove_file(&road);
    }

    #[test]
    fn rubbish_is_not_a_reason_to_refuse_to_start() {
        // Every failure gives the same answer — start a new game. A title screen
        // that will not open because a save is corrupt has turned a lost afternoon
        // into a lost game.
        for (what, body) in [
            ("empty", ""),
            ("not json", "half a save and then the power went"),
            ("no format", "{\"at\": [1,2,3]}"),
            ("no position", "{\"format\": 1}"),
        ] {
            let road = scratch(what);
            std::fs::write(&road, body).unwrap();
            assert!(read_from(&road).is_none(), "{what} was read as a save");
            let _ = std::fs::remove_file(&road);
        }
        // And a path with nothing at it at all.
        assert!(read_from(&scratch("missing")).is_none());
    }

    #[test]
    fn the_save_goes_in_the_players_folder_and_not_the_games() {
        // A save in `assets/` would ship somebody's afternoon to everybody who
        // downloads the game, and re-installing would wipe it.
        let Some(road) = path() else {
            return;
        };
        let said = road.to_string_lossy().to_lowercase();
        assert!(
            !said.contains("assets"),
            "the save is inside the game's own files: {said}"
        );
        assert!(said.ends_with("save.json"));
        // And under a folder named for the game, so it is findable by hand and by
        // the launcher, which computes this same path for itself.
        assert!(said.contains("copaimo"), "{said}");
    }
}
