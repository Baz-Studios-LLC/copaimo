//! The turning year: which season it is, and what the world wears because of it.
//!
//! # A season is twenty-eight real days
//!
//! The game already runs on the machine's clock — nine in the morning here is nine
//! in the morning in the world, and `sky` reads it directly rather than spinning a
//! sun at a chosen rate. The year works the same way and for the same reason: it
//! reads the real DATE. A season turns every twenty-eight days, so a season is
//! about a real month and a year is about four of them.
//!
//! That means nobody ever watches a season change. They come back after a fortnight
//! away and the wood has turned, which is the right way round for a game somebody
//! plays for years: the world moves while you are not looking at it.
//!
//! # Biomes do not move. The look does.
//!
//! A forest does not become a desert in winter. Nothing here touches `Biome`,
//! `Country`, the heightfield, or where anything grows — a season changes what the
//! world WEARS, and this module can only change colours. That is a deliberate
//! ceiling: the moment a season could move a biome, every test that pins the world
//! down would depend on the day it was run.
//!
//! # What changes, and what pointedly does not
//!
//! Only broadleaf trees turn. Spruce and pine are conifers and hold their needles
//! all year; palm and acacia are dry-country trees and do not do an autumn, so a
//! desert does not go orange in October. Oak and birch are the ones that turn, and
//! they are the woods the player walks through.
//!
//! The recolouring is nearly free, which is what makes it worth doing at all. Every
//! variety in the grove owns its own leaf MATERIAL, so a season is twenty writes to
//! twenty assets — and every tree already standing in the world changes with them,
//! because what is rewritten is the asset behind the handle and never the handle.
//! It is the same trick authored tree shapes use to replace a grown mesh under a
//! wood that has already grown.

use bevy::prelude::*;
use chrono::Datelike;

use crate::shade::Shaded;
use crate::world::stream::Grove;

/// How many days a season lasts.
///
/// Twenty-eight, as asked: about a real month, so a year is about four.
pub const SEASON_DAYS: i64 = 28;

/// The day the wheel starts, and it starts on spring.
///
/// Chosen rather than derived, and worth saying why it is not the real calendar's:
/// a twenty-eight day season cannot line up with a year that is twelve months long,
/// so this wheel drifts against the real one on purpose. Anchoring it here puts
/// August 2026 — when it was built — in high summer, which is the least surprising
/// thing for the world to be doing on the day somebody first sees it.
pub const EPOCH: (i32, u32, u32) = (2026, 7, 15);

/// How much of a season is spent turning into the next one.
///
/// The last quarter, so seven of the twenty-eight days. Without it a wood would be
/// green one morning and gold the next; with it the turn takes a week, which is
/// how long it takes.
const TURNS_OVER: f32 = 0.25;

/// One quarter of the year.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum Season {
    #[default]
    Spring,
    Summer,
    Autumn,
    Winter,
}

impl Season {
    /// The four, in the order the wheel turns.
    pub const ALL: [Season; 4] = [
        Season::Spring,
        Season::Summer,
        Season::Autumn,
        Season::Winter,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Season::Spring => "spring",
            Season::Summer => "summer",
            Season::Autumn => "autumn",
            Season::Winter => "winter",
        }
    }

    /// Where it sits on the wheel, 0 to 3.
    pub fn place(self) -> usize {
        match self {
            Season::Spring => 0,
            Season::Summer => 1,
            Season::Autumn => 2,
            Season::Winter => 3,
        }
    }

    /// The one after it. Winter comes round to spring.
    pub fn next(self) -> Season {
        Season::ALL[(self.place() + 1) % 4]
    }

    /// The nth season on the wheel, counted from spring, wrapping either way.
    pub fn nth(place: i64) -> Season {
        Season::ALL[place.rem_euclid(4) as usize]
    }
}

/// Which season it is, and how far through it we are.
#[derive(Resource, Clone, Copy, Debug)]
pub struct TheYear {
    pub season: Season,
    /// How far through this season, 0 at its first morning and 1 at its last night.
    pub through: f32,
    /// Seasons added on top of the calendar, for looking at an autumn on purpose.
    pub nudge: i64,
    /// False once somebody has scrubbed it, until they ask for it back.
    pub follows_calendar: bool,
}

impl Default for TheYear {
    fn default() -> Self {
        // Summer, which is what it will be for a heartbeat before the first read of
        // the real calendar replaces it.
        Self {
            season: Season::Summer,
            through: 0.0,
            nudge: 0,
            follows_calendar: true,
        }
    }
}

impl TheYear {
    /// How the HUD says it.
    pub fn spoken(&self) -> String {
        let day = (self.through * SEASON_DAYS as f32).floor() as i64 + 1;
        let tail = if self.follows_calendar { "" } else { " (held)" };
        format!(
            "{} day {day} of {SEASON_DAYS}{tail}",
            self.season.name()
        )
    }

    /// How far the world has turned towards the NEXT season, 0 to 1.
    ///
    /// Nought for the first three quarters of a season and then a smooth climb, so
    /// a wood turns over about a week rather than overnight.
    pub fn turning(&self) -> f32 {
        let into = ((self.through - (1.0 - TURNS_OVER)) / TURNS_OVER).clamp(0.0, 1.0);
        into * into * (3.0 - 2.0 * into)
    }
}

/// Which season a given day falls in, and how far through it.
///
/// Split out from the system so it can be asked about any day at all, which is what
/// the tests do — they walk a whole wheel of them rather than trusting whatever
/// today happens to be.
pub fn the_season_on(day: i64, nudge: i64) -> (Season, f32) {
    let epoch = chrono::NaiveDate::from_ymd_opt(EPOCH.0, EPOCH.1, EPOCH.2)
        .expect("the season epoch is a real date")
        .num_days_from_ce() as i64;
    let since = day - epoch;
    let place = since.div_euclid(SEASON_DAYS) + nudge;
    let into = since.rem_euclid(SEASON_DAYS) as f32 / SEASON_DAYS as f32;
    (Season::nth(place), into)
}

/// Reads the machine's calendar, unless somebody has taken hold of it.
fn read_the_calendar(keys: Res<ButtonInput<KeyCode>>, mut year: ResMut<TheYear>) {
    if keys.just_pressed(KeyCode::F9) {
        year.follows_calendar = false;
        year.nudge -= 1;
    }
    if keys.just_pressed(KeyCode::F10) {
        year.follows_calendar = false;
        year.nudge += 1;
    }
    if keys.just_pressed(KeyCode::F11) {
        year.follows_calendar = true;
        year.nudge = 0;
    }

    let today = chrono::Local::now().date_naive().num_days_from_ce() as i64;
    let (season, through) = the_season_on(today, year.nudge);
    year.season = season;
    year.through = through;
}

/// The greens and golds a broadleaf wears, dark end and light end.
///
/// A PAIR, not a colour, because the grove's twenty varieties each sit somewhere in
/// the range and that spread is what makes a wood read as a wood instead of a
/// stencil. The season moves the whole range and leaves every tree's place in it
/// alone, so the wood that was mottled in summer is mottled in autumn too.
fn broadleaf_range(season: Season) -> (Srgba, Srgba) {
    match season {
        // New growth: yellower and brighter than the summer it becomes.
        Season::Spring => (Srgba::rgb(0.16, 0.34, 0.13), Srgba::rgb(0.47, 0.64, 0.25)),
        // The range the world was built and balanced on.
        Season::Summer => (Srgba::rgb(0.13, 0.28, 0.13), Srgba::rgb(0.38, 0.55, 0.24)),
        // Russet through amber. The one season anybody will notice from a hill.
        Season::Autumn => (Srgba::rgb(0.34, 0.16, 0.06), Srgba::rgb(0.74, 0.46, 0.11)),
        // What is left on the branch: dull brown, and darker than anything else on
        // the wheel. The leaves coming OFF is a separate job and not this one.
        Season::Winter => (Srgba::rgb(0.24, 0.17, 0.11), Srgba::rgb(0.42, 0.32, 0.20)),
    }
}

/// What a conifer or a dry-country tree wears, which is very nearly summer all year.
///
/// Not the same as "unchanged". A spruce in winter is darker and greyer than a
/// spruce in July, and left at exactly its summer green it reads as plastic in a
/// world where everything around it has turned.
fn evergreen_range(season: Season) -> (Srgba, Srgba) {
    let (dark, light) = broadleaf_range(Season::Summer);
    let cool = match season {
        Season::Spring => 0.06,
        Season::Summer => 0.0,
        Season::Autumn => 0.10,
        Season::Winter => 0.22,
    };
    let sink = |c: Srgba| {
        Srgba::rgb(
            c.red * (1.0 - cool * 0.55),
            c.green * (1.0 - cool * 0.75),
            c.blue * (1.0 - cool * 0.20),
        )
    };
    (sink(dark), sink(light))
}

/// Whether a species turns with the year.
///
/// Oak and birch do. Spruce and pine hold their needles; palm and acacia are dry
/// country and have no autumn to do — which is also what keeps a desert from going
/// gold, and the design asks for weather to be logical for the ground it falls on.
pub fn turns_with_the_year(species: terrain_core::tree::Species) -> bool {
    use terrain_core::tree::Species;
    matches!(species, Species::Oak | Species::Birch)
}

/// The colour one variety wears today.
pub fn leaf_colour_now(
    tint: f32,
    species: terrain_core::tree::Species,
    year: &TheYear,
) -> LinearRgba {
    let range = if turns_with_the_year(species) {
        broadleaf_range
    } else {
        evergreen_range
    };
    let (now_dark, now_light) = range(year.season);
    let (next_dark, next_light) = range(year.season.next());
    let turning = year.turning();

    let blend = |a: Srgba, b: Srgba| {
        LinearRgba::from(a)
            .to_vec4()
            .lerp(LinearRgba::from(b).to_vec4(), turning)
    };
    let dark = blend(now_dark, next_dark);
    let light = blend(now_light, next_light);
    LinearRgba::from_vec4(dark.lerp(light, tint))
}

/// Whether a species stands bare at this point in the year.
///
/// Only the trees that turn ever drop, and they drop for winter alone — the whole
/// of it, and none of the rest of the year. A conifer never does; nor does a palm
/// or an acacia, which is the same rule that keeps the desert from doing an autumn.
///
/// # It happens on the boundary, and that is on purpose
///
/// There is no gradual thinning here. The colour spends the last week of autumn
/// turning, arrives at winter's brown on winter's first morning, and the leaves go
/// on the same morning — so the last thing the wood does before it stands bare is
/// finish going brown. A season boundary is a real month from the one before it, so
/// nobody is standing under the tree when it happens; the alternative is shrinking
/// every leaf clump toward a centre the mesh does not record, which would cost real
/// geometry work to animate something no player is present for.
pub fn stands_bare(species: terrain_core::tree::Species, season: Season) -> bool {
    season == Season::Winter && turns_with_the_year(species)
}

/// A pristine copy of every variety's leaves.
///
/// Winter is applied by rewriting the MESH behind each variety's handle, the same
/// way the colour rewrites the material — that is what reaches the tens of
/// thousands of trees already standing in the world without touching one of them.
/// Rewriting is destructive, though, so spring needs something to put back, and
/// this is it: the canopy as it was grown, held aside once at startup.
#[derive(Resource)]
pub struct TheCanopy {
    full: Vec<Mesh>,
}

/// Strips a leaf mesh without throwing it away.
///
/// Keeps every vertex and attribute and empties the INDEX buffer, so the mesh draws
/// no triangles at all while staying a valid mesh of exactly the layout the render
/// pipeline was built for. Collapsing the vertices to a point would have worked too
/// and would have left the GPU chewing on geometry that covers no pixels.
fn stripped(full: &Mesh) -> Mesh {
    let mut bare = full.clone();
    bare.insert_indices(bevy::render::mesh::Indices::U32(Vec::new()));
    bare
}

/// Holds the grown canopy aside, once, so winter can be undone.
fn remember_the_canopy(
    mut commands: Commands,
    canopy: Option<Res<TheCanopy>>,
    grove: Option<Res<Grove>>,
    meshes: Res<Assets<Mesh>>,
) {
    if canopy.is_some() {
        return;
    }
    let Some(grove) = grove else {
        return;
    };
    let mut full = Vec::with_capacity(grove.trees.len());
    for variety in &grove.trees {
        let Some(mesh) = meshes.get(&variety.leaves) else {
            // The pool is still arriving. Try again next frame rather than
            // remembering half a canopy and calling it the whole one.
            return;
        };
        full.push(mesh.clone());
    }
    commands.insert_resource(TheCanopy { full });
}

/// Puts the leaves on or takes them off, for the season.
fn dress_the_branches(
    year: Res<TheYear>,
    grove: Option<Res<Grove>>,
    canopy: Option<Res<TheCanopy>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut wearing: Local<Option<Season>>,
) {
    let (Some(grove), Some(canopy)) = (grove, canopy) else {
        return;
    };
    // Only when the season actually turns. A mesh rewrite is cheap once a month and
    // wasteful every frame, and `is_changed` on the year is true every frame because
    // the calendar system writes it whether or not the answer moved.
    if *wearing == Some(year.season) {
        return;
    }
    *wearing = Some(year.season);

    for (variety, full) in grove.trees.iter().zip(&canopy.full) {
        let Some(slot) = meshes.get_mut(&variety.leaves) else {
            continue;
        };
        *slot = if stands_bare(variety.species, year.season) {
            stripped(full)
        } else {
            full.clone()
        };
    }
}

/// Dresses every variety in the grove for the season.
///
/// Runs when the year changes and not every frame — a season lasts twenty-eight
/// days and the blend within one moves by a thousandth an hour, so there is nothing
/// to see between one day and the next. `is_changed` covers both the day rolling
/// over and somebody scrubbing with F9.
fn dress_the_grove(
    year: Res<TheYear>,
    grove: Option<Res<Grove>>,
    mut materials: ResMut<Assets<Shaded>>,
    mut wearing: Local<Option<(Season, i32)>>,
) {
    let Some(grove) = grove else {
        return;
    };
    // What the trees are actually WEARING, not whether the resource was written.
    //
    // `is_changed` is no use here: the calendar system writes the year every frame
    // whether or not the answer moved, so keying off it repainted twenty-eight
    // materials sixty times a second to the same colours they already were.
    //
    // The turn is quantised to a two-hundredth because it is the only part that
    // moves continuously, and it moves by about four thousandths an HOUR - so a
    // step of 0.005 repaints roughly once an hour during the one week a year the
    // wood is turning, and never otherwise.
    let now = (year.season, (year.turning() * 200.0) as i32);
    if *wearing == Some(now) && !grove.is_changed() {
        return;
    }
    *wearing = Some(now);
    for variety in &grove.trees {
        let Some(material) = materials.get_mut(&variety.leaf) else {
            continue;
        };
        material.base.base_color = leaf_colour_now(variety.tint, variety.species, &year).into();
    }
}

pub struct SeasonPlugin;

impl Plugin for SeasonPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TheYear>().add_systems(
            Update,
            (
                read_the_calendar,
                remember_the_canopy,
                dress_the_grove,
                dress_the_branches,
            )
                .chain(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The epoch as a day number, so the tests can count from it.
    fn epoch_day() -> i64 {
        chrono::NaiveDate::from_ymd_opt(EPOCH.0, EPOCH.1, EPOCH.2)
            .unwrap()
            .num_days_from_ce() as i64
    }


    /// Draws the grove's whole palette, season by season, to
    /// `dev/art/map/seasons.png`.
    ///
    /// Ignored, like the map: it writes a file and is here to be LOOKED at. A
    /// season is a thing you see or it is nothing, and every number in this module
    /// could be right while the wood still came out the colour of a traffic cone.
    ///
    /// One row per season, one column per variety in the real pool — the same
    /// twenty `terrain_core::tree::grow` hands the game, in the same order, each
    /// asked for its own colour through `leaf_colour_now`. So the strip is not a
    /// picture of the palette; it IS the palette, drawn by the code the trees use.
    #[test]
    #[ignore = "writes a picture to be looked at"]
    fn draw_the_seasons() {
        const WIDE: u32 = 96;
        const HIGH: u32 = 132;
        const LABEL: u32 = 26;

        let pool: Vec<_> = (0..terrain_core::tree::VARIETIES as u32)
            .map(terrain_core::tree::grow)
            .collect();
        let mut sheet = image::RgbImage::new(
            WIDE * pool.len() as u32,
            (HIGH + LABEL) * Season::ALL.len() as u32,
        );

        for (row, season) in Season::ALL.iter().enumerate() {
            let year = TheYear {
                season: *season,
                through: 0.0,
                nudge: 0,
                follows_calendar: true,
            };
            for (col, tree) in pool.iter().enumerate() {
                let colour = leaf_colour_now(tree.tint, tree.species, &year);
                let srgb = Srgba::from(colour);
                let rgb = image::Rgb([
                    (srgb.red.clamp(0.0, 1.0) * 255.0) as u8,
                    (srgb.green.clamp(0.0, 1.0) * 255.0) as u8,
                    (srgb.blue.clamp(0.0, 1.0) * 255.0) as u8,
                ]);
                // A band of the season's colour, and under it a darker band for the
                // conifers so a glance tells which columns are allowed to stay green.
                let evergreen = !turns_with_the_year(tree.species);
                for y in 0..HIGH + LABEL {
                    for x in 0..WIDE {
                        let px = col as u32 * WIDE + x;
                        let py = row as u32 * (HIGH + LABEL) + y;
                        let mark = if y >= HIGH {
                            if evergreen {
                                image::Rgb([28, 34, 28])
                            } else {
                                image::Rgb([80, 62, 30])
                            }
                        } else {
                            rgb
                        };
                        sheet.put_pixel(px, py, mark);
                    }
                }
            }
        }

        let dir = std::path::Path::new("dev/art/map");
        std::fs::create_dir_all(dir).expect("somewhere to put it");
        sheet
            .save(dir.join("seasons.png"))
            .expect("the swatch should save");

        for season in Season::ALL {
            let year = TheYear {
                season,
                through: 0.0,
                nudge: 0,
                follows_calendar: true,
            };
            let broad = leaf_colour_now(0.5, terrain_core::tree::Species::Oak, &year);
            let ever = leaf_colour_now(0.5, terrain_core::tree::Species::Spruce, &year);
            println!(
                "{:<7} oak ({:.2}, {:.2}, {:.2})   spruce ({:.2}, {:.2}, {:.2})",
                season.name(),
                broad.red,
                broad.green,
                broad.blue,
                ever.red,
                ever.green,
                ever.blue
            );
        }
        println!(
            "drew {} varieties x {} seasons to dev/art/map/seasons.png",
            pool.len(),
            Season::ALL.len()
        );
    }

#[test]
    fn the_broadleaves_stand_bare_in_winter_and_nothing_else_ever_does() {
        use terrain_core::tree::Species;
        for season in Season::ALL {
            for species in [Species::Spruce, Species::Pine, Species::Palm, Species::Acacia] {
                assert!(
                    !stands_bare(species, season),
                    "a {species:?} dropped its leaves in {}",
                    season.name()
                );
            }
            for species in [Species::Oak, Species::Birch] {
                assert_eq!(
                    stands_bare(species, season),
                    season == Season::Winter,
                    "a {species:?} in {} is the wrong way round",
                    season.name()
                );
            }
        }
    }

    #[test]
    fn stripping_a_canopy_leaves_a_mesh_that_draws_nothing_and_is_still_a_mesh() {
        // The point of emptying the INDICES rather than the vertices: what comes
        // back has to be something the render pipeline still accepts, or a winter
        // wood is a screenful of errors instead of bare branches.
        let mut full = Mesh::new(
            bevy::render::mesh::PrimitiveTopology::TriangleList,
            bevy::asset::RenderAssetUsages::default(),
        );
        full.insert_attribute(
            Mesh::ATTRIBUTE_POSITION,
            vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        );
        full.insert_attribute(
            Mesh::ATTRIBUTE_NORMAL,
            vec![[0.0, 1.0, 0.0], [0.0, 1.0, 0.0], [0.0, 1.0, 0.0]],
        );
        full.insert_indices(bevy::render::mesh::Indices::U32(vec![0, 1, 2]));

        let bare = stripped(&full);
        assert_eq!(
            bare.indices().map(|i| i.len()),
            Some(0),
            "a stripped canopy still draws triangles"
        );
        assert_eq!(
            bare.count_vertices(),
            full.count_vertices(),
            "stripping threw the vertices away, so spring has nothing to put back"
        );
        assert!(
            bare.attribute(Mesh::ATTRIBUTE_NORMAL).is_some(),
            "a stripped canopy lost an attribute the pipeline needs"
        );
    }

    #[test]
    fn a_season_lasts_twenty_eight_days_and_the_wheel_turns_in_four_of_them() {
        let start = epoch_day();
        // Every day of a whole wheel, so this cannot pass by sampling the middles.
        for day in 0..(SEASON_DAYS * 4) {
            let (season, _) = the_season_on(start + day, 0);
            let want = Season::nth(day / SEASON_DAYS);
            assert_eq!(
                season, want,
                "day {day} of the wheel came out {} and should be {}",
                season.name(),
                want.name()
            );
        }
        // And it comes round rather than running off the end.
        let (after, _) = the_season_on(start + SEASON_DAYS * 4, 0);
        assert_eq!(after, Season::Spring, "the wheel did not come round");
    }

    #[test]
    fn every_season_happens_and_none_of_them_is_skipped() {
        let start = epoch_day();
        let mut seen = std::collections::HashSet::new();
        for day in 0..(SEASON_DAYS * 4) {
            seen.insert(the_season_on(start + day, 0).0);
        }
        assert_eq!(seen.len(), 4, "only {} of the four seasons happen", seen.len());
    }

    #[test]
    fn the_year_runs_forwards_and_backwards_from_any_day() {
        // Well before the epoch, where `since` is negative and a plain `/` would
        // round towards zero and hand back the wrong season for half the wheel.
        let long_ago = epoch_day() - SEASON_DAYS * 9 - 5;
        let (season, through) = the_season_on(long_ago, 0);
        assert!(
            (0.0..1.0).contains(&through),
            "a day before the epoch is {through} through its season"
        );
        assert_eq!(
            season,
            Season::nth(-10),
            "counting back nine and a bit seasons landed on {}",
            season.name()
        );
    }

    #[test]
    fn autumn_is_the_one_that_turns_and_a_conifer_does_not_go_with_it() {
        use terrain_core::tree::Species;
        let at = |season, species| {
            let year = TheYear {
                season,
                through: 0.0,
                nudge: 0,
                follows_calendar: true,
            };
            leaf_colour_now(0.5, species, &year)
        };

        let summer_oak = at(Season::Summer, Species::Oak);
        let autumn_oak = at(Season::Autumn, Species::Oak);
        // Green in summer, and warmer than it is green in autumn. Asked as a
        // COMPARISON rather than against absolute numbers, because the palette is
        // meant to be tuned and a test that pins hexadecimal would just have to be
        // rewritten every time somebody warms it a shade.
        assert!(
            summer_oak.green > summer_oak.red,
            "a summer oak is not green: {summer_oak:?}"
        );
        assert!(
            autumn_oak.red > autumn_oak.green,
            "an autumn oak has not turned: {autumn_oak:?}"
        );

        let summer_spruce = at(Season::Summer, Species::Spruce);
        let autumn_spruce = at(Season::Autumn, Species::Spruce);
        assert!(
            autumn_spruce.green > autumn_spruce.red,
            "a spruce turned with the broadleaves: {autumn_spruce:?}"
        );
        // It darkens, which is not the same as turning.
        assert!(
            autumn_spruce.green < summer_spruce.green,
            "a spruce in autumn is no darker than one in July"
        );
    }

    #[test]
    fn the_turn_is_a_week_and_nothing_jumps_at_the_boundary() {
        use terrain_core::tree::Species;
        let colour_at = |through: f32| {
            let year = TheYear {
                season: Season::Summer,
                through,
                nudge: 0,
                follows_calendar: true,
            };
            leaf_colour_now(0.5, Species::Oak, &year)
        };

        // Nothing moves for the first three quarters.
        let early = colour_at(0.1);
        let three_quarters = colour_at(0.74);
        assert!(
            (early.red - three_quarters.red).abs() < 1e-6,
            "the wood started turning in the middle of summer"
        );

        // By the last night of summer it has arrived at autumn.
        let last_night = colour_at(0.999);
        let autumn = {
            let year = TheYear {
                season: Season::Autumn,
                through: 0.0,
                nudge: 0,
                follows_calendar: true,
            };
            leaf_colour_now(0.5, Species::Oak, &year)
        };
        assert!(
            (last_night.red - autumn.red).abs() < 0.02
                && (last_night.green - autumn.green).abs() < 0.02,
            "summer's last night is {last_night:?} and autumn's first morning is \
             {autumn:?} — the wood changes colour overnight"
        );

        // And it gets there smoothly rather than in one step.
        let mut worst: f32 = 0.0;
        let mut last = colour_at(0.75);
        for step in 1..=100 {
            let now = colour_at(0.75 + 0.25 * step as f32 / 100.0);
            worst = worst.max((now.red - last.red).abs());
            last = now;
        }
        assert!(
            worst < 0.02,
            "the turn moves {worst:.3} in a hundredth of a season — that is a step, \
             not a turn"
        );
    }
}
