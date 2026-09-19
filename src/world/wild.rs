//! The wild, and what a person has said about it.
//!
//! # Why the trees are not in a file
//!
//! Every tree and every boulder in the world stands on a world-wide lattice and is a
//! pure function of its slot: `trees_in` and `prop::litter_in` both work the same
//! way, hashing `(slot_x, slot_z)` for a jitter, a species and a size. Nothing about
//! a tree is stored anywhere, and that is what lets a chunk plant itself on any
//! thread in any order and get the same wood.
//!
//! Baking them WOULD work - measured, the world carries about 228,000 trees, which is
//! some four megabytes - and it buys nothing anybody asked for. A stored tree is
//! editable, and so is a generated one the moment there is somewhere to say "not that
//! one": adding is already covered by `placed.json` and its stable ids, and the whole
//! gap was taking something away. So the store holds what a person SAID, not what the
//! generator made, which is a few lines rather than a quarter of a million.
//!
//! It is the same shape as a settlement's vetoes - see `world::bake::Veto`, which
//! this shares - and for the same reason: a generated thing has no durable name, so a
//! deletion says WHERE rather than WHICH.
//!
//! # Why a place and not a slot
//!
//! A slot is the more stable name - change the jitter and a tree moves while its slot
//! does not - and it is unusable by hand, because nobody can read a lattice index off
//! the screen. A veto is written by somebody looking at a tree they want gone, so it
//! takes the coordinate they can see. `TREE_SPACING` jitters by under half a step, so
//! any veto wide enough to be worth writing is wider than the tree can wander.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::bake::Veto;

/// What a veto silences out in the country.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Wilding {
    #[default]
    All,
    /// The woods - `Terrain::trees_in`.
    Trees,
    /// Boulders, logs, stumps, brush - `prop::litter_in`.
    Props,
}

/// What a person has said about the wild.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Wild {
    /// Bumped when the file's shape changes, not when its contents do.
    pub version: u32,
    /// Places the generator may not plant or strew.
    #[serde(default)]
    pub vetoes: Vec<Veto<Wilding>>,
}

/// The shape of the file as this code understands it.
pub const VERSION: u32 = 1;

/// Where the file lives.
pub fn path() -> std::path::PathBuf {
    std::path::Path::new("assets").join("world").join("wild.json")
}

/// Read once.
///
/// `trees_in` runs on every streaming thread for every chunk and asks about thousands
/// of slots each time; re-reading a file there would put IO in the middle of the
/// world being drawn and make the answer depend on when it was asked.
static SAID: std::sync::OnceLock<Wild> = std::sync::OnceLock::new();

fn said() -> &'static Wild {
    SAID.get_or_init(|| {
        let path = path();
        let Ok(text) = std::fs::read_to_string(&path) else {
            return Wild::default();
        };
        match serde_json::from_str::<Wild>(&text) {
            Ok(wild) if wild.version > VERSION => {
                error!(
                    "{} is version {} and this build knows {VERSION} - ignoring it",
                    path.display(),
                    wild.version
                );
                Wild::default()
            }
            Ok(wild) => {
                if !wild.vetoes.is_empty() {
                    info!("the wild: {} vetoes", wild.vetoes.len());
                }
                *LIVE.write().expect("the wild is not poisoned") = wild.vetoes.clone();
                wild
            }
            // LOUD, and then plant anyway. A file somebody is editing by hand will be
            // malformed sometimes, and silently ignoring it looks like the edit did
            // nothing at all.
            Err(why) => {
                error!("{} could not be read: {why}", path.display());
                Wild::default()
            }
        }
    })
}

/// Says the generator may not put `kind` within `within` of `at`, and writes it down.
///
/// Returns false if the file could not be written, which the caller should say out
/// loud - a veto that was not saved looks exactly like one that was until the game is
/// next started.
///
/// The read is a `OnceLock` and cannot be refilled, so the new veto is pushed into
/// the live copy as well. That is the whole of the cache invalidation here, and it is
/// sound because the file only ever grows from this one door.
pub fn forbid(kind: Wilding, at: Vec2, within: f32) -> bool {
    // SAFETY of a sort: `said()` fills the lock before this takes a lock of its own,
    // so the initialiser cannot run while the write lock is held.
    let _ = said();
    let mut live = LIVE.write().expect("the wild is not poisoned");
    live.push(Veto { at, within, kind });
    let wild = Wild { version: VERSION, vetoes: live.clone() };
    let Ok(text) = serde_json::to_string_pretty(&wild) else {
        return false;
    };
    let path = path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(&path, text).is_ok()
}

/// Whether the generator may put a thing of this kind here.
///
/// The common case is a world with nothing said about it, and that costs one slice
/// length - worth caring about, because this is asked once per lattice slot per chunk
/// and a chunk holds thousands.
pub fn may_stand(kind: Wilding, at: Vec2) -> bool {
    // THE FILE FIRST, or the first question is answered before it is read. After the
    // first call this is a load and nothing else.
    let _ = said();
    let live = LIVE.read().expect("the wild is not poisoned");
    if live.is_empty() {
        return true;
    }
    !live.iter().any(|veto| veto.stops(kind, at))
}

/// The vetoes as they stand, file plus anything said since.
///
/// A `OnceLock` cannot be refilled, and the editor adds vetoes while the game is
/// running - so the lock holds what was READ and this holds what is TRUE. `said()`
/// fills both, once.
static LIVE: std::sync::RwLock<Vec<Veto<Wilding>>> = std::sync::RwLock::new(Vec::new());

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::bake::Veto;

    /// A veto silences what it names, where it names it, and nothing else.
    ///
    /// # The three ways a veto goes wrong
    ///
    /// It silences nothing, because the kind never matches. It silences the wrong
    /// kind, because `All` was read as a kind rather than as "everything". Or it
    /// silences the whole county, because the radius is compared the wrong way
    /// round - and a world with no trees in it looks like a world that failed to
    /// load, not like a world somebody edited.
    #[test]
    fn a_veto_silences_its_own_kind_in_its_own_circle() {
        let here = Vec2::new(100.0, 40.0);
        let trees = Veto { at: here, within: 12.0, kind: Wilding::Trees };
        let all = Veto { at: here, within: 12.0, kind: Wilding::All };

        assert!(trees.stops(Wilding::Trees, here + Vec2::new(11.0, 0.0)));
        assert!(!trees.stops(Wilding::Trees, here + Vec2::new(13.0, 0.0)));
        // Named a kind, so it leaves the others alone.
        assert!(!trees.stops(Wilding::Props, here));
        // `All` takes everything inside it, and still nothing outside.
        assert!(all.stops(Wilding::Props, here) && all.stops(Wilding::Trees, here));
        assert!(!all.stops(Wilding::Trees, here + Vec2::new(0.0, 40.0)));
    }

    /// A world nobody has edited plants exactly as it always did.
    ///
    /// The cheap path matters: this is asked once per lattice slot per chunk, and a
    /// chunk holds thousands. It is also the only path almost every world takes.
    #[test]
    fn an_unedited_world_is_never_vetoed() {
        assert!(may_stand(Wilding::Trees, Vec2::new(1234.0, -567.0)));
        assert!(may_stand(Wilding::Props, Vec2::ZERO));
    }

    /// A veto said in the editor is remembered before the file is next read.
    ///
    /// # The cache that cannot be refilled
    ///
    /// The file is read into a `OnceLock`, because `trees_in` runs on every
    /// streaming thread and must not do IO. A `OnceLock` cannot be refilled - so a
    /// veto written while the game is running would be in the file and NOT in the
    /// answer, and the tree a maker just deleted would stay standing until restart.
    /// `LIVE` is what is true; the lock is only what was read.
    #[test]
    fn a_veto_said_now_is_obeyed_now() {
        let here = Vec2::new(-77_000.0, 41_000.0);
        assert!(may_stand(Wilding::Trees, here), "nothing should be vetoed out here");
        LIVE.write()
            .expect("the wild is not poisoned")
            .push(Veto { at: here, within: 20.0, kind: Wilding::Trees });
        assert!(!may_stand(Wilding::Trees, here), "the veto was not obeyed");
        assert!(
            may_stand(Wilding::Props, here),
            "it silenced a kind it did not name"
        );
        LIVE.write().expect("the wild is not poisoned").clear();
    }
}
