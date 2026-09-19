//! The world, stored — a settlement's layout as a file somebody can edit.
//!
//! # Why anything is stored at all
//!
//! Nothing was. `Terrain::trees_in` says it outright: *"Nothing about a tree is
//! stored anywhere."* That is what makes the world reproducible, and it is what lets
//! a chunk plant itself on any thread in any order and get the same forest.
//!
//! What it cannot do is let anybody fix a road by hand. And the thing that decided
//! it was this: several sessions of tuning generator constants to make procedural
//! output match a painting. When you are fighting a generator to hit a specific
//! picture, that content wants to be authored.
//!
//! So one settlement is stored — the first city, the one being built to the concept
//! art — and the rest of the world still generates. If this feels good the others
//! follow one at a time; if it does not, one city's worth of work is the loss.
//!
//! # Two rules
//!
//! **Derived data is never stored.** `Layout::streets` and `Layout::nodes` are
//! derived from `ways`, and the doc comment on `streets` says so: "Derived from
//! `ways`, never built beside it." Writing them to a file would be this project's
//! recurring bug family in its most durable form — one fact with two derivations,
//! and the file's copy winning every argument, forever. The file holds `ways`, and
//! `network()` re-derives on load exactly as it always has. The walls and the stairs
//! are left out for the same reason: they follow the terrain's terraces.
//!
//! **Every row says where it came from.** Generated or authored — see `Made`. A
//! re-bake regenerates the generated rows and leaves the authored ones alone. Without
//! that, the first generator improvement after the first hand edit is a choice
//! between the two, and that choice is what kills hybrid pipelines. It is here from
//! the first version because it cannot be added later.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::town::{Layout, Place, Plot, Way};

/// Where a row came from.
///
/// The whole point of the store. A row nobody has touched is the generator's and may
/// be thrown away and made again; a row somebody edited is theirs and may not.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Made {
    /// The generator's. A re-bake replaces it.
    #[default]
    Generated,
    /// Somebody's. A re-bake keeps it, and drops anything generated that lands on
    /// top of it.
    Authored,
}

/// One row of a stored layout: the thing, and where it came from.
///
/// Flattened, so the file reads as the thing itself with one extra field rather than
/// as a wrapper somebody editing it has to see through.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Row<T> {
    /// Absent in a hand-written file means `Generated`, which is the safe default:
    /// the worst it costs is that a re-bake replaces a row somebody meant to keep,
    /// and they will notice. The other way round silently pins a stale row forever.
    #[serde(default)]
    pub from: Made,
    #[serde(flatten)]
    pub what: T,
}

impl<T> Row<T> {
    fn made(what: T) -> Self {
        Row { from: Made::Generated, what }
    }
}

/// What the file was made from, so a stale one can be spotted rather than trusted.
///
/// Not used to REJECT a file - a stale bake is exactly what somebody editing the
/// world has, and refusing to load it would make the store useless the first time a
/// constant changed. It is here so `--bake` can say what moved.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default)]
pub struct Stamp {
    pub at: Vec2,
    pub radius: f32,
    pub seed: u32,
}

/// A place where the generator is not to put anything, and what kind.
///
/// # A deletion cannot be a reference, because generated rows have no names
///
/// The obvious way to record "I deleted this building" is to name the row. It cannot
/// be done: a generated row has no durable name, and every scheme for inventing one
/// turns out to be a hash of the generator's current behaviour. Codex proposed a
/// lineage path through the growth queue; two skeptics took it apart against the
/// source, and the deepest reason is that the row we STORE is a post-planarise piece
/// rather than anything the generator emitted, so an id minted at emission still
/// needs an ordinal that renumbers when a neighbour changes.
///
/// So a deletion says WHERE rather than WHICH. It keeps working when the generator
/// moves the building three metres, and it goes quiet of its own accord when the
/// generator stops putting anything there. Nothing has to be named for it to work,
/// which is exactly the property a name was supposed to provide.
///
/// The cost is honest and worth stating: a veto is indiscriminate. Two things the
/// generator legitimately puts within `within` of each other both go.
/// Generic in what it silences, because the world has more than one kind of thing
/// a person wants gone and they all want saying the same way. A settlement vetoes
/// plots and ways; the wild vetoes trees and boulders - see `world::wild`.
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct Veto<Kind = Vetoed> {
    pub at: Vec2,
    /// How far the veto reaches, in metres.
    pub within: f32,
    /// Which kind it silences. `All` for "nothing here at all".
    #[serde(default)]
    pub kind: Kind,
}

/// What a veto silences.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Vetoed {
    #[default]
    All,
    Plots,
    Ways,
    Opens,
}

impl<Kind: PartialEq + Default + Copy> Veto<Kind> {
    /// Whether this silences `kind` at `at`. The default variant of a kind means
    /// "everything", which is why every kind has one and why it is the default.
    pub fn stops(&self, kind: Kind, at: Vec2) -> bool {
        (self.kind == Kind::default() || self.kind == kind)
            && self.at.distance(at) <= self.within
    }
}

/// A settlement's layout, as a file.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Baked {
    /// Bumped when the file's shape changes, not when its contents do.
    pub version: u32,
    pub stamp: Stamp,
    pub opens: Vec<Row<Place>>,
    pub ways: Vec<Row<Way>>,
    pub plots: Vec<Row<Plot>>,
    /// Where the generator is not to put anything. Survives every re-bake - see
    /// `Veto`, which is the whole of how a deletion is recorded.
    #[serde(default)]
    pub vetoes: Vec<Veto>,
}

/// The shape of the file as this code understands it.
pub const VERSION: u32 = 1;

/// How near an authored row a generated one has to land to be dropped, in metres.
///
/// A re-bake puts the generator's own idea of a town back, and where somebody has
/// moved a building the generator will cheerfully put the original back beside it.
/// The authored row wins and the generated one goes - which is the rule that makes
/// the whole arrangement survivable, and the only place the two kinds ever meet.
const AUTHORED_CLEARS: f32 = 6.0;

impl Baked {
    /// The generator's own layout, as a file, with nothing authored in it yet.
    pub fn of(site: &crate::world::settle::Site, seed: u32, laid: &Layout) -> Self {
        Baked {
            version: VERSION,
            stamp: Stamp { at: site.at, radius: site.radius, seed },
            opens: laid.opens.iter().cloned().map(Row::made).collect(),
            ways: laid.ways.iter().cloned().map(Row::made).collect(),
            plots: laid.plots.iter().cloned().map(Row::made).collect(),
            // A fresh bake vetoes nothing. `rebaked` carries the old file's across.
            vetoes: Vec::new(),
        }
    }

    /// This file's rows laid into a layout, with everything derived left empty.
    ///
    /// The caller finishes it - `network` for the streets and the nodes, and the
    /// terraces for the walls and the stairs - because those are derived and the
    /// file does not hold them.
    pub fn laid(&self) -> Layout {
        // VETOED ROWS ARE DROPPED HERE, not at spawn time.
        //
        // The layout this returns is consumed by the LEVELLING as well as by the
        // renderer - `settle::lay_the_town_out` turns every plot into a pad and
        // every street into a lane while the world is being planned. A deleted
        // building filtered only where it is drawn would leave its flattened pad
        // in the ground: a rectangle of level earth with nothing standing on it.
        let alive = |kind: Vetoed, at: Vec2| !self.vetoes.iter().any(|veto| veto.stops(kind, at));
        Layout {
            opens: self
                .opens
                .iter()
                .filter(|row| alive(Vetoed::Opens, row.what.at))
                .map(|row| row.what.clone())
                .collect(),
            ways: self
                .ways
                .iter()
                .filter(|row| {
                    // A way is a chain; it is vetoed only if EVERY point is.
                    // Half a street left standing is worse than either answer.
                    !row.what.points.iter().all(|at| !alive(Vetoed::Ways, *at))
                })
                .map(|row| row.what.clone())
                .collect(),
            streets: Vec::new(),
            nodes: Vec::new(),
            plots: self
                .plots
                .iter()
                .filter(|row| alive(Vetoed::Plots, row.what.at))
                .map(|row| row.what.clone())
                .collect(),
            // Derived on the way in - see `town::finish`. Never in the file.
            lamps: Vec::new(),
            walls: Vec::new(),
            stairs: Vec::new(),
        }
    }

    /// A fresh generation, with this file's authored rows carried onto it.
    ///
    /// Generated rows are the generator's to replace, so they simply come from
    /// `fresh`. Authored rows are carried across untouched, and any generated row
    /// that lands within `AUTHORED_CLEARS` of one is dropped - the person who moved
    /// the building does not want the old one back beside it.
    pub fn rebaked(&self, fresh: Baked) -> Baked {
        fn carry<T: Clone>(
            had: &[Row<T>],
            fresh: Vec<Row<T>>,
            where_is: impl Fn(&T) -> Vec2,
        ) -> Vec<Row<T>> {
            let authored: Vec<&Row<T>> =
                had.iter().filter(|row| row.from == Made::Authored).collect();
            let mut kept: Vec<Row<T>> = fresh
                .into_iter()
                .filter(|row| {
                    let at = where_is(&row.what);
                    !authored
                        .iter()
                        .any(|mine| where_is(&mine.what).distance(at) < AUTHORED_CLEARS)
                })
                .collect();
            kept.extend(authored.into_iter().cloned());
            kept
        }
        Baked {
            version: VERSION,
            stamp: fresh.stamp,
            // Carried, always. A veto that expired with the file it was written in
            // would let the deleted building back the first time anybody re-baked.
            vetoes: self.vetoes.clone(),
            opens: carry(&self.opens, fresh.opens, |place| place.at),
            plots: carry(&self.plots, fresh.plots, |plot| plot.at),
            // A way is a CHAIN, so it has no single place to compare. Its first
            // point is as good an anchor as any and better than pretending
            // otherwise: two roads starting within six metres of each other are the
            // same road as far as anybody editing is concerned.
            ways: carry(&self.ways, fresh.ways, |way| {
                way.points.first().copied().unwrap_or_default()
            }),
        }
    }

    /// How many rows somebody has taken ownership of.
    pub fn authored(&self) -> usize {
        let count = |from: &Made| usize::from(*from == Made::Authored);
        self.opens.iter().map(|row| count(&row.from)).sum::<usize>()
            + self.ways.iter().map(|row| count(&row.from)).sum::<usize>()
            + self.plots.iter().map(|row| count(&row.from)).sum::<usize>()
    }
}

/// Where a settlement's file lives.
///
/// Named by the settlement's PERMANENT NAME - see `config::SETTLEMENTS` - and found
/// through `asset_file`, so it is the same file whether the game is run from the
/// repository or from a packaged build. It was keyed by the site's index in a
/// vector and looked up relative to the working directory, and Codex found both
/// faults (P0.1, 2026-09-17): the ranch is pushed ahead of the table so the index
/// was already off by one, and a test launched elsewhere would silently exercise
/// the generated city instead of the stored one.
pub fn path_of(name: &str) -> std::path::PathBuf {
    crate::asset_file(&format!("assets/world/settlement_{name}.json"))
}

/// Every stored settlement, read once.
///
/// # Read once, and on purpose
///
/// `lay_the_site_out` is called while the world is being planned, from whatever
/// thread got there first, and more than once for the same settlement. Reading the
/// file on each call would put IO in the middle of terrain generation and make the
/// answer depend on when it was asked - which is precisely the property this file's
/// header says the generated world has and must keep.
static STORED: std::sync::OnceLock<std::collections::HashMap<String, Baked>> =
    std::sync::OnceLock::new();

/// The stored layout for a settlement, if there is one.
pub fn stored(name: &str) -> Option<&'static Baked> {
    STORED
        .get_or_init(|| {
            let mut found = std::collections::HashMap::new();
            let names = crate::config::SETTLEMENTS
                .iter()
                .map(|row| row.3)
                .chain(std::iter::once("ranch"));
            for name in names {
                let path = path_of(name);
                let Ok(text) = std::fs::read_to_string(&path) else {
                    continue;
                };
                match serde_json::from_str::<Baked>(&text) {
                    // A FILE FROM A LATER BUILD IS NOT READ.
                    //
                    // `VERSION` was written into every file and never looked at,
                    // which makes it decoration. A file whose shape this build does
                    // not know would be read with `#[serde(default)]` filling in
                    // whatever it lacked - silently, and as a town somebody would
                    // then edit. Older is fine and is the normal case; newer is not.
                    Ok(baked) if baked.version > VERSION => error!(
                        "{} is version {} and this build knows {VERSION} - generating                          instead. Re-bake with a newer build, or delete the file.",
                        path.display(),
                        baked.version
                    ),
                    Ok(baked) => {
                        info!(
                            "stored settlement {name}: {} ways, {} plots, {} authored,                              {} vetoed",
                            baked.ways.len(),
                            baked.plots.len(),
                            baked.authored(),
                            baked.vetoes.len()
                        );
                        found.insert(name.to_string(), baked);
                    }
                    // LOUD, and then generate. A file somebody is editing by hand
                    // will be malformed sometimes, and silently falling back to the
                    // generator would look like the edit simply did nothing.
                    Err(why) => error!("{} could not be read: {why}", path.display()),
                }
            }
            found
        })
        .get(name)
}

/// Writes a settlement's layout out, carrying over whatever was authored in the old
/// one.
pub fn write(name: &str, baked: Baked) -> std::io::Result<Baked> {
    let merged = match stored(name) {
        Some(had) => had.rebaked(baked),
        None => baked,
    };
    let path = path_of(name);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(&merged)
        .map_err(|why| std::io::Error::other(why.to_string()))?;
    std::fs::write(&path, text)?;
    Ok(merged)
}

/// Whether the run was asked to bake the world and stop.
pub fn asked_for() -> bool {
    std::env::args().any(|arg| arg == "--bake")
}

/// Lays every settlement out and writes it to its file.
///
/// A one-shot process, before the app and without it - the same reasoning as
/// `measure`: the generation this writes down is pure and thread-safe by design, and
/// standing a window up round it would put the thing being recorded inside something
/// far larger than itself. It is also what makes the same-process staleness of
/// `stored()` harmless here: this process reads once, writes, and ends.
///
/// The ranch is not a settlement and `world::town` skips it, so it is skipped here.
pub fn bake_everything() {
    let terrain = crate::world::terrain::Terrain::new();
    let plan = terrain.plan();
    let mut wrote = 0;
    for (key, site) in plan.sites().iter().enumerate() {
        if site.ranch {
            continue;
        }
        // FRESH, from the generator - never from the store. `lay_the_site_out` would
        // hand back the file that is already there, and a bake that starts from its
        // own last output can never import an improvement. See `generate_the_site`.
        let laid = crate::world::town::generate_the_site(plan, key, site);
        let baked = Baked::of(site, crate::world::town::seed_of(key), &laid);
        match write(site.name, baked) {
            Ok(merged) => {
                println!(
                    "baked {:<14} {:>4} ways {:>4} plots {:>3} places {:>3} authored -> {}",
                    site.name,
                    merged.ways.len(),
                    merged.plots.len(),
                    merged.opens.len(),
                    merged.authored(),
                    path_of(site.name).display()
                );
                wrote += 1;
            }
            Err(why) => eprintln!("{} could not be written: {why}", site.name),
        }
    }
    println!("{wrote} settlements written");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::town::{Building, District};

    fn plot(at: Vec2, what: Building) -> Plot {
        Plot { at, serves: None, district: District::Market, facing: 0.0, what }
    }

    /// A veto keeps a deleted thing deleted, through a re-bake and a move.
    ///
    /// # The three ways a deletion comes undone
    ///
    /// It comes back on the next re-bake, because the veto lived in the file that
    /// was replaced. It comes back when the generator moves it a metre, because the
    /// deletion named a row rather than a place. Or it half comes back: gone from
    /// the drawing and still there in the ground, because the filter ran where the
    /// layout is rendered rather than where it is returned - and the levelling
    /// consumes the same layout to flatten a pad under every plot.
    ///
    /// This holds all three shut.
    #[test]
    fn a_veto_outlives_the_file_it_was_written_in() {
        let mut had = Baked::default();
        had.plots.push(Row::made(plot(Vec2::ZERO, Building::Cottage)));
        had.vetoes.push(Veto { at: Vec2::ZERO, within: 8.0, kind: Vetoed::All });

        // Gone from the layout the moment the file is read - which is the layout
        // the levelling sees, not merely the one the renderer sees.
        assert!(had.laid().plots.is_empty(), "the vetoed plot was laid out anyway");

        // The generator puts it back, three metres over, and rebuilds the town.
        let mut fresh = Baked::default();
        fresh.plots.push(Row::made(plot(Vec2::new(3.0, 0.0), Building::Cottage)));
        fresh.plots.push(Row::made(plot(Vec2::new(90.0, 0.0), Building::Shop)));

        let merged = had.rebaked(fresh);
        assert_eq!(merged.vetoes.len(), 1, "the veto did not survive the re-bake");
        assert!(
            merged.laid().plots.iter().all(|plot| plot.what != Building::Cottage),
            "the deleted cottage came back: {:?}",
            merged.laid().plots.iter().map(|p| (p.what, p.at)).collect::<Vec<_>>()
        );
        // And it silenced only what it was pointed at.
        assert_eq!(merged.laid().plots.len(), 1, "the veto took something else with it");
    }

    /// A re-bake takes the generator's new rows and keeps the person's.
    ///
    /// # The one rule that makes the store survivable
    ///
    /// Three things have to be true at once, and each is a way the arrangement dies
    /// if it is not. A generated row the generator now makes DIFFERENTLY has to come
    /// through changed - or no improvement ever reaches a stored town, which is the
    /// fault Codex found in the first `--bake`. An authored row has to survive
    /// untouched - or the first re-bake after the first edit throws the edit away.
    /// And a generated row that lands on top of an authored one has to go - or the
    /// building somebody moved comes back beside itself.
    #[test]
    fn a_rebake_replaces_the_generated_and_keeps_the_authored() {
        let mut had = Baked::default();
        had.plots.push(Row::made(plot(Vec2::ZERO, Building::Cottage)));
        had.plots.push(Row {
            from: Made::Authored,
            what: plot(Vec2::new(50.0, 50.0), Building::Cottage),
        });

        let mut fresh = Baked::default();
        // The generator improved: the same lot is a shop now.
        fresh.plots.push(Row::made(plot(Vec2::ZERO, Building::Shop)));
        // And it put something down two metres from the authored building.
        fresh.plots.push(Row::made(plot(Vec2::new(52.0, 50.0), Building::Cottage)));

        let merged = had.rebaked(fresh);
        let kinds: Vec<(Made, Building, Vec2)> =
            merged.plots.iter().map(|row| (row.from, row.what.what, row.what.at)).collect();

        assert!(
            kinds.contains(&(Made::Generated, Building::Shop, Vec2::ZERO)),
            "the improved generated row did not come through: {kinds:?}"
        );
        assert!(
            kinds.contains(&(Made::Authored, Building::Cottage, Vec2::new(50.0, 50.0))),
            "the authored row was lost: {kinds:?}"
        );
        assert_eq!(
            merged.plots.len(),
            2,
            "a generated row landed on the authored one and was kept: {kinds:?}"
        );
        assert_eq!(merged.authored(), 1);
    }
}
