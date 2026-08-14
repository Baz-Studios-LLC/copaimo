//! Where people would put a town, and the roads between them.
//!
//! Not buildings — **ground**. A settlement needs somewhere level to stand, and
//! a road needs a graded run between one and the next. Both are cut into the
//! terrain here, so that by the time anything is built the ground is already
//! willing to hold it.
//!
//! # Why it is generated rather than authored
//!
//! Because the ground has to agree with itself in two programs. The bench and
//! the game both work this out from the seed and the map, identically, so a town
//! site is in the same place and at the same height in both without a file
//! passing between them. A maker who wants a site somewhere else sculpts it
//! there — the hand-edit layer sits on top of all of this and always wins.
//!
//! # How a site is chosen
//!
//! Candidates are proposed from a hash of the seed, then rejected unless they
//! are inland of the beach, below the treeline, on ground that is not already
//! steep, and far enough from every site already placed. Cities are placed first
//! and given the room they need; towns fill in between them afterwards.

use bevy::prelude::*;

use crate::config::*;
use crate::util::smoothstep;

/// A place levelled for people to build on.
#[derive(Clone, Copy)]
pub struct Site {
    pub at: Vec2,
    /// The height its ground was levelled to.
    pub height: f32,
    /// How far the level ground reaches.
    pub radius: f32,
    /// Whether this is one of the larger places.
    pub city: bool,
}

/// A graded run of ground between two sites.
#[derive(Clone, Copy)]
struct Road {
    from: Vec2,
    to: Vec2,
    from_height: f32,
    to_height: f32,
}

/// Everything levelled, with a coarse grid over it so a height lookup only ever
/// tests the handful of features near it rather than all of them.
///
/// This matters more than it looks: the height field is asked millions of times
/// to mesh a world, and a linear scan over every site and road at each sample
/// would dominate the whole generator.
pub struct Settlements {
    sites: Vec<Site>,
    roads: Vec<Road>,
    /// One list of feature indices per cell. Sites are stored as `Some(i)`,
    /// roads as the index offset past the end of `sites`.
    cells: Vec<Vec<u16>>,
    cells_across: i32,
    cells_down: i32,
    half: Vec2,
}

/// How wide a grid cell is, in metres. Comfortably larger than the biggest
/// feature's reach, so a lookup touches one cell.
const CELL: f32 = 512.0;

impl Settlements {
    /// An empty plan, for a world that has not worked out its towns yet.
    pub fn nowhere() -> Self {
        Settlements {
            sites: Vec::new(),
            roads: Vec::new(),
            cells: Vec::new(),
            cells_across: 0,
            cells_down: 0,
            half: Vec2::ONE,
        }
    }

    pub fn sites(&self) -> &[Site] {
        &self.sites
    }

    pub fn roads_len(&self) -> usize {
        self.roads.len()
    }

    /// Works out where the towns go and grades the roads between them.
    ///
    /// `ground` answers the height BEFORE any of this is applied, and `shore`
    /// the distance to the coast. Both must be free of settlements or this would
    /// be reading its own output.
    pub fn plan(half: Vec2, ground: &dyn Fn(Vec2) -> f32, shore: &dyn Fn(Vec2) -> f32) -> Self {
        let mut sites: Vec<Site> = Vec::new();

        // Cities first, with the room they need; towns afterwards, filling in
        // whatever is left. Placing them the other way round would let a town
        // sit where a city needed to be.
        for (wanted, city) in [(CITIES, true), (CITIES + TOWNS, false)] {
            let radius = if city { CITY_RADIUS } else { TOWN_RADIUS };
            let apart = if city { CITY_SPACING } else { TOWN_SPACING };

            let mut tries = 0u32;
            while sites.len() < wanted && tries < 20_000 {
                let n = tries;
                tries += 1;
                let at = Vec2::new(
                    (unit(WORLD_SEED, n * 2) * 2.0 - 1.0) * half.x * 0.94,
                    (unit(WORLD_SEED, n * 2 + 1) * 2.0 - 1.0) * half.y * 0.94,
                );

                // Inland of the beach, so leveling a site never eats a shore.
                if shore(at) < SITE_MIN_INLAND {
                    continue;
                }
                let height = ground(at);
                // Not up a mountain. People build where the living is.
                if height > SITE_MAX_HEIGHT {
                    continue;
                }
                // Not on ground that is already a hillside: leveling that would
                // leave a scar you could see from orbit.
                if steepness(ground, at) > SITE_MAX_SLOPE {
                    continue;
                }
                if sites
                    .iter()
                    .any(|other| other.at.distance(at) < apart.max(other.radius + radius))
                {
                    continue;
                }

                sites.push(Site {
                    at,
                    height,
                    radius,
                    city,
                });
            }
        }

        let roads = link(&sites);
        let mut settlements = Settlements {
            sites,
            roads,
            cells: Vec::new(),
            cells_across: 0,
            cells_down: 0,
            half,
        };
        settlements.index();
        settlements
    }

    /// Files every feature into the cells its reach touches.
    fn index(&mut self) {
        self.cells_across = (self.half.x * 2.0 / CELL).ceil() as i32 + 1;
        self.cells_down = (self.half.y * 2.0 / CELL).ceil() as i32 + 1;
        self.cells = vec![Vec::new(); (self.cells_across * self.cells_down) as usize];

        // Worked out in full before any of it is filed, because filing borrows
        // the grid mutably and reading the features borrows it immutably.
        let mut filings: Vec<(u16, Vec2, Vec2)> = Vec::new();
        for (i, site) in self.sites.iter().enumerate() {
            let reach = site.radius + SITE_SKIRT;
            filings.push((i as u16, site.at - reach, site.at + reach));
        }
        let road_reach = ROAD_WIDTH + ROAD_SKIRT;
        let offset = self.sites.len() as u16;
        for (i, road) in self.roads.iter().enumerate() {
            let low = road.from.min(road.to) - road_reach;
            let high = road.from.max(road.to) + road_reach;
            filings.push((offset + i as u16, low, high));
        }
        for (what, low, high) in filings {
            self.file(what, low, high);
        }
    }

    fn file(&mut self, what: u16, low: Vec2, high: Vec2) {
        let (x0, y0) = self.cell_of(low);
        let (x1, y1) = self.cell_of(high);
        for y in y0..=y1 {
            for x in x0..=x1 {
                let at = (y * self.cells_across + x) as usize;
                if let Some(cell) = self.cells.get_mut(at) {
                    cell.push(what);
                }
            }
        }
    }

    fn cell_of(&self, at: Vec2) -> (i32, i32) {
        (
            (((at.x + self.half.x) / CELL) as i32).clamp(0, self.cells_across - 1),
            (((at.y + self.half.y) / CELL) as i32).clamp(0, self.cells_down - 1),
        )
    }

    /// What the ground here wants to be levelled to, and how strongly.
    ///
    /// `None` where nothing is near, which is nearly everywhere — this is on the
    /// hot path and its job is mostly to say no quickly.
    pub fn level(&self, at: Vec2) -> Option<(f32, f32)> {
        let (x, y) = self.cell_of(at);
        let cell = self.cells.get((y * self.cells_across + x) as usize)?;
        if cell.is_empty() {
            return None;
        }

        let mut target = 0.0;
        let mut weight = 0.0f32;
        let sites = self.sites.len() as u16;

        for &what in cell {
            let (height, pull) = if what < sites {
                let site = &self.sites[what as usize];
                let away = site.at.distance(at);
                // Flat out to the radius, then easing back to the land over the
                // skirt, so a town sits in the ground rather than on a plinth.
                (
                    site.height,
                    smoothstep(site.radius + SITE_SKIRT, site.radius, away),
                )
            } else {
                let road = &self.roads[(what - sites) as usize];
                let (away, along) = road.nearest(at);
                // A road climbs steadily from one end to the other, so it can be
                // walked and a cart can use it.
                let height = road.from_height + (road.to_height - road.from_height) * along;
                (height, smoothstep(ROAD_WIDTH + ROAD_SKIRT, ROAD_WIDTH, away))
            };
            if pull <= weight {
                continue;
            }
            // The strongest claim wins outright rather than averaging: a road
            // meeting a town should join its level, not split the difference and
            // leave a lip where they meet.
            target = height;
            weight = pull;
        }

        (weight > 0.0).then_some((target, weight))
    }
}

impl Road {
    /// Distance to this road, and how far along it that was, 0 to 1.
    fn nearest(&self, at: Vec2) -> (f32, f32) {
        let run = self.to - self.from;
        let length = run.length_squared();
        if length < 1.0 {
            return (self.from.distance(at), 0.0);
        }
        let along = ((at - self.from).dot(run) / length).clamp(0.0, 1.0);
        (self.from.lerp(self.to, along).distance(at), along)
    }
}

/// Connects every site into one network, each joining whichever placed site is
/// nearest.
///
/// A minimum spanning tree, grown one site at a time. It is the smallest set of
/// roads that still leaves every town reachable from every other, which is what
/// a road network is for — and it produces no crossings and no orphans.
fn link(sites: &[Site]) -> Vec<Road> {
    let mut roads = Vec::new();
    if sites.len() < 2 {
        return roads;
    }
    let mut joined = vec![false; sites.len()];
    joined[0] = true;

    for _ in 1..sites.len() {
        let mut best: Option<(f32, usize, usize)> = None;
        for (i, site) in sites.iter().enumerate() {
            if !joined[i] {
                continue;
            }
            for (j, other) in sites.iter().enumerate() {
                if joined[j] {
                    continue;
                }
                let away = site.at.distance(other.at);
                if best.is_none_or(|(shortest, _, _)| away < shortest) {
                    best = Some((away, i, j));
                }
            }
        }
        let Some((_, from, to)) = best else { break };
        joined[to] = true;
        roads.push(Road {
            from: sites[from].at,
            to: sites[to].at,
            from_height: sites[from].height,
            to_height: sites[to].height,
        });
    }
    roads
}

/// How steep the ground is at a point, sampled wide enough to catch a hillside
/// rather than a bump.
fn steepness(ground: &dyn Fn(Vec2) -> f32, at: Vec2) -> f32 {
    const STEP: f32 = 24.0;
    let dx = ground(at + Vec2::X * STEP) - ground(at - Vec2::X * STEP);
    let dz = ground(at + Vec2::Y * STEP) - ground(at - Vec2::Y * STEP);
    (dx * dx + dz * dz).sqrt() / (2.0 * STEP)
}

/// A repeatable 0..1 from a seed and a counter.
///
/// Hashed rather than drawn from a generator so that the same seed gives the
/// same towns in both programs, whatever order anything else asks for numbers.
fn unit(seed: u32, n: u32) -> f32 {
    let mut h = seed ^ n.wrapping_mul(0x9E37_79B9);
    h ^= h >> 16;
    h = h.wrapping_mul(0x7feb_352d);
    h ^= h >> 15;
    h = h.wrapping_mul(0x846c_a68b);
    h ^= h >> 16;
    h as f32 / u32::MAX as f32
}
