//! Everything somebody has put in the world by hand, and where they put it.
//!
//! # What this is for
//!
//! `assets/buildings/*.json` says what a building IS — a cottage is these boxes
//! in these colours. It says nothing about where any cottage stands, and until
//! now nothing did: the world raised one building at each town site, which is a
//! stand-in and not placement. Nothing could be put somewhere on purpose, and
//! nothing that had been put somewhere could be found again.
//!
//! This is the other half. A short list of *this thing, here, turned this way,
//! this big* — read at startup, written by the editor.
//!
//! **It is the keystone for three separate jobs**, which is why it comes before
//! any of them: a workbench needs somewhere to put what it makes; moving a thing
//! needs the world to remember where it was; and taking a boulder out needs the
//! world to have an opinion about that boulder in the first place.
//!
//! # Why JSON here when the other layers are binary
//!
//! The painted layers are dense grids — millions of cells — so they are packed
//! floats and nobody reads them. This is sparse and small: tens or hundreds of
//! entries, each one a decision somebody made. That wants to be legible,
//! diffable, and fixable in a text editor when something goes wrong, and the cost
//! of that at this size is nothing.
//!
//! # On the ground, not at a height
//!
//! `at` is x and z only. How high something sits is worked out from the ground
//! under it, plus `lift`.
//!
//! That is deliberate and it is the one decision here worth arguing about. An
//! absolute height is simpler and wrong: the ground gets sculpted, and every
//! building placed before the sculpting would be left buried or standing on air
//! with nothing to say which. Storing the offset means a house sits on its hill
//! however the hill is reshaped afterwards. Something that genuinely wants to be
//! off the ground — a bridge over a gorge — is placed on the gorge floor with a
//! lift, and survives the same way.

use bevy::log::{info, warn};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::io;
use std::path::Path;

use crate::config::PLACED_PATH;

/// The format this reads and writes. Bumped when the shape changes in a way an
/// older reader could not cope with.
const FORMAT: u32 = 1;

/// One thing standing in the world.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Placed {
    /// Its own name, for the life of the world.
    ///
    /// Not its position in the list. A maker deletes the third of five things and
    /// every index after it shifts, so anything holding one — a selection, an
    /// undo entry, a link from one thing to another — would be pointing at a
    /// different object with nothing to say it had moved. An id costs four bytes
    /// and is the difference between "move that" working and appearing to.
    pub id: u32,
    /// What it is: the name of a plan in the catalogue.
    pub kind: String,
    /// Where it stands, in world metres. X and Z only — see the module note.
    pub at: Vec2,
    /// How far above the ground it sits, in metres.
    #[serde(default)]
    pub lift: f32,
    /// Which way it faces, in radians about Y.
    ///
    /// Radians, matching `marks[].yaw` in the baked building format. Degrees
    /// would be kinder to read by hand, and two files in one folder measuring
    /// angles differently is a trap worth more than the kindness.
    #[serde(default)]
    pub turn: f32,
    #[serde(default = "one")]
    pub scale: f32,
}

fn one() -> f32 {
    1.0
}

/// Everything placed in this world.
#[derive(Resource, Default, Debug)]
pub struct Standing {
    things: Vec<Placed>,
    /// Whether anything has changed since this was last written.
    pub unsaved: bool,
}

impl Standing {
    pub fn all(&self) -> &[Placed] {
        &self.things
    }

    pub fn len(&self) -> usize {
        self.things.len()
    }

    pub fn is_empty(&self) -> bool {
        self.things.is_empty()
    }

    /// Puts something in the world and hands back its name.
    pub fn add(&mut self, kind: impl Into<String>, at: Vec2, turn: f32, scale: f32) -> u32 {
        // One past the highest in use, never the count. Deleting the last thing
        // and adding another would otherwise hand out a name that something else
        // has already been referred to by.
        let id = self.things.iter().map(|t| t.id).max().unwrap_or(0) + 1;
        self.things.push(Placed {
            id,
            kind: kind.into(),
            at,
            lift: 0.0,
            turn,
            scale,
        });
        self.unsaved = true;
        id
    }

    pub fn get(&self, id: u32) -> Option<&Placed> {
        self.things.iter().find(|t| t.id == id)
    }

    /// For moving, turning or resizing something already placed.
    ///
    /// Unused until there is a gizmo to drag. Here because it is the whole point
    /// of things having names, and a test exercises it.
    #[allow(dead_code)]
    pub fn get_mut(&mut self, id: u32) -> Option<&mut Placed> {
        self.unsaved = true;
        self.things.iter_mut().find(|t| t.id == id)
    }

    /// Takes something out. `false` if there was nothing by that name.
    pub fn remove(&mut self, id: u32) -> bool {
        let before = self.things.len();
        self.things.retain(|t| t.id != id);
        let went = self.things.len() != before;
        self.unsaved |= went;
        went
    }

    /// The nearest thing to a point, within a distance. For clicking on things.
    pub fn nearest(&self, to: Vec2, within: f32) -> Option<u32> {
        self.things
            .iter()
            .map(|t| (t.at.distance(to), t.id))
            .filter(|(away, _)| *away <= within)
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, id)| id)
    }

    pub fn mark_saved(&mut self) {
        self.unsaved = false;
    }
}

/// The file, as it sits on disk.
#[derive(Serialize, Deserialize)]
struct Sheet {
    format: u32,
    placed: Vec<Placed>,
}

/// Reads a sheet of placed things from JSON.
pub fn read(json: &str) -> Result<Standing, String> {
    let sheet: Sheet = serde_json::from_str(json).map_err(|why| why.to_string())?;
    if sheet.format != FORMAT {
        // Refused rather than guessed at. A file from a newer writer may mean
        // something different by the same field, and a building in the wrong
        // place is worse than one that is honestly missing.
        return Err(format!("format {}, and this reads {FORMAT}", sheet.format));
    }

    // Two things sharing a name is the one fault that would make every later
    // reference ambiguous, so it is caught at the door rather than found later.
    let mut seen = std::collections::HashSet::new();
    for thing in &sheet.placed {
        if !seen.insert(thing.id) {
            return Err(format!("two things both called {}", thing.id));
        }
        if !thing.scale.is_finite() || thing.scale <= 0.0 {
            return Err(format!("{} has a scale of {}", thing.id, thing.scale));
        }
        if !thing.at.is_finite() || !thing.turn.is_finite() || !thing.lift.is_finite() {
            return Err(format!("{} is placed nowhere in particular", thing.id));
        }
    }

    Ok(Standing {
        things: sheet.placed,
        unsaved: false,
    })
}

pub fn write(standing: &Standing) -> String {
    let sheet = Sheet {
        format: FORMAT,
        placed: standing.things.clone(),
    };
    // Pretty, because the whole point of this being JSON is that somebody can
    // open it.
    serde_json::to_string_pretty(&sheet).unwrap_or_else(|_| "{}".into())
}

/// Reads what a maker placed, or an empty world if there is none.
pub fn load() -> Standing {
    let path = Path::new(PLACED_PATH);
    if !path.exists() {
        // The ordinary case for a world nobody has built in yet.
        return Standing::default();
    }
    match std::fs::read_to_string(path) {
        Ok(json) => match read(&json) {
            Ok(standing) => {
                info!("placed: {} things from {}", standing.len(), path.display());
                standing
            }
            Err(why) => {
                warn!("{}: {why} - nothing raised", path.display());
                Standing::default()
            }
        },
        Err(why) => {
            warn!("{}: {why} - nothing raised", path.display());
            Standing::default()
        }
    }
}

pub fn save(standing: &mut Standing) -> io::Result<()> {
    let path = Path::new(PLACED_PATH);
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder)?;
    }
    std::fs::write(path, write(standing))?;
    standing.mark_saved();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sheet_survives_the_round_trip() {
        let mut standing = Standing::default();
        let cottage = standing.add("cottage", Vec2::new(120.0, -40.0), 0.7, 1.0);
        let barn = standing.add("barn", Vec2::new(-3000.0, 900.0), -2.1, 1.4);
        assert_ne!(cottage, barn, "two things share a name");

        let back = read(&write(&standing)).expect("its own writing");
        assert_eq!(back.len(), 2);
        let one = back.get(cottage).expect("the cottage came back");
        assert_eq!(one.kind, "cottage");
        assert_eq!(one.at, Vec2::new(120.0, -40.0));
        assert!((one.turn - 0.7).abs() < 1.0e-6);
        assert!((back.get(barn).unwrap().scale - 1.4).abs() < 1.0e-6);
        assert!(!back.unsaved, "freshly read work is not unsaved work");
    }

    #[test]
    fn a_name_outlives_a_deletion() {
        // The reason things carry an id and not their position in the list.
        // Delete the middle of three and every index after it shifts, so a
        // selection or an undo entry holding one would be pointing at a
        // different object with nothing to say it had moved.
        let mut standing = Standing::default();
        let first = standing.add("a", Vec2::ZERO, 0.0, 1.0);
        let second = standing.add("b", Vec2::splat(10.0), 0.0, 1.0);
        let third = standing.add("c", Vec2::splat(20.0), 0.0, 1.0);

        assert!(standing.remove(second));
        assert!(!standing.remove(second), "removing it twice should say so");
        assert_eq!(standing.get(first).map(|t| t.kind.as_str()), Some("a"));
        assert_eq!(standing.get(third).map(|t| t.kind.as_str()), Some("c"));

        // And a new thing never inherits a name already used, even though the
        // count has gone down.
        let fourth = standing.add("d", Vec2::splat(30.0), 0.0, 1.0);
        assert!(fourth > third, "{fourth} reuses a name at or below {third}");
    }

    #[test]
    fn a_broken_sheet_is_refused_rather_than_half_read() {
        // A building in the wrong place is worse than one honestly missing: the
        // first looks like a fault in the drawing and gets chased there, the
        // second says what is wrong in the log.
        assert!(read("not json at all").is_err());
        assert!(
            read("{\"format\":99,\"placed\":[]}").is_err(),
            "a newer format may mean something different by the same field"
        );
        assert!(
            read(
                "{\"format\":1,\"placed\":[\
                 {\"id\":1,\"kind\":\"a\",\"at\":[0,0]},\
                 {\"id\":1,\"kind\":\"b\",\"at\":[9,9]}]}"
            )
            .is_err(),
            "two things sharing a name makes every later reference ambiguous"
        );
        assert!(
            read("{\"format\":1,\"placed\":[{\"id\":1,\"kind\":\"a\",\"at\":[0,0],\"scale\":0}]}")
                .is_err(),
            "nothing is nothing big"
        );

        // And the defaults hold, so a hand-written entry needs only what it means.
        let bare = read("{\"format\":1,\"placed\":[{\"id\":4,\"kind\":\"post\",\"at\":[1.5,2.5]}]}")
            .expect("the shortest entry anyone would write");
        let post = bare.get(4).expect("the post");
        assert_eq!(post.scale, 1.0);
        assert_eq!(post.turn, 0.0);
        assert_eq!(post.lift, 0.0);
    }

    #[test]
    fn a_thing_can_be_moved_without_being_replaced() {
        // What names are FOR. Moving something has to keep it the same something,
        // or every reference to it — a selection, an undo entry, a door another
        // building points at — is quietly pointing at a hole.
        let mut standing = Standing::default();
        let id = standing.add("cottage", Vec2::ZERO, 0.0, 1.0);
        standing.mark_saved();

        let thing = standing.get_mut(id).expect("the cottage");
        thing.at = Vec2::new(80.0, -20.0);
        thing.turn = 1.2;
        thing.lift = 3.0;

        assert!(standing.unsaved, "moving something is work to be saved");
        let moved = standing.get(id).expect("still the same cottage");
        assert_eq!(moved.at, Vec2::new(80.0, -20.0));
        assert_eq!(moved.kind, "cottage");
        assert_eq!(standing.len(), 1, "moving it made a second one");

        // And it survives being written out and read back where it was put.
        let back = read(&write(&standing)).expect("its own writing");
        let same = back.get(id).expect("the cottage came back");
        assert_eq!(same.at, Vec2::new(80.0, -20.0));
        assert!((same.lift - 3.0).abs() < 1.0e-6);
    }

    #[test]
    fn the_nearest_thing_is_the_one_you_clicked() {
        let mut standing = Standing::default();
        standing.add("far", Vec2::new(500.0, 0.0), 0.0, 1.0);
        let near = standing.add("near", Vec2::new(10.0, 0.0), 0.0, 1.0);

        assert_eq!(standing.nearest(Vec2::new(12.0, 0.0), 50.0), Some(near));
        // And nothing at all rather than the least distant thing in the world, so
        // clicking empty ground does not grab a building over the horizon.
        assert_eq!(standing.nearest(Vec2::new(2_000.0, 0.0), 50.0), None);
    }
}
