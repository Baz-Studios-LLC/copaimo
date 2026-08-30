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
    /// The player's ranch, which is a site so that nothing else takes its ground -
    /// and is NOT a settlement. `world::town` skips it.
    ///
    /// It was not marked, and the town layout builds on every site it is given, so
    /// a hundred houses went up on top of the ranch and the ranch was reported as
    /// having "moved to a different location". It had not moved an inch; it was
    /// underneath a town.
    pub ranch: bool,
}

/// A street inside a town, as a claim on the ground it runs over.
///
/// # Why a street is a levelling claim and not a thing that is drawn
///
/// The town streets were laid out, the buildings were placed against them, and
/// nothing whatever appeared on the ground - the plan existed only in the layout.
/// Drawing them would mean a mesh, a material and a decision about where the mesh
/// sits relative to terrain that moves under it.
///
/// They are ground, so they are told to the GROUND. A lane levels what it runs
/// over, exactly as a road between towns does, and levelled ground is what
/// `Biome::Settled` already means - bare packed earth. So a street flattens itself,
/// paints itself, stops grass and cover growing on itself, and keeps props and
/// trees off itself, all through machinery that was already there.
///
/// Narrow and strong, where a site is wide and gentle: a site's claim fades over
/// its whole radius, which is why the middle of a town is grass. A lane has to be
/// unmistakably a lane at its edge and nothing a metre beyond it.
#[derive(Clone, Copy)]
pub struct Lane {
    pub from: Vec2,
    pub to: Vec2,
    /// The height it holds, which is its town's.
    pub height: f32,
    /// Kerb to kerb.
    pub wide: f32,
}

impl Lane {
    /// How far a point is from the middle of this lane.
    fn off(&self, at: Vec2) -> f32 {
        let run = self.to - self.from;
        let length2 = run.length_squared().max(1.0e-4);
        let along = ((at - self.from).dot(run) / length2).clamp(0.0, 1.0);
        at.distance(self.from + run * along)
    }
}

/// How far out a settlement's own ground reaches, as a share of its radius.
///
/// Past the buildings, because a place's ground is what it stands on AND what it
/// has trodden flat around itself. A boundary wall at 1.06 with the ground stopping
/// at 1.0 would put a stripe of meadow between the last yard and the wall.
const SETTLED_REACHES: f32 = 1.12;

/// How much of that is solid before it starts giving way to country.
const SETTLED_SOLID: f32 = 0.72;

/// How far past its kerb a lane keeps levelling, in metres.
///
/// Short. A street with a wide skirt is a clearing with a path in it.
const LANE_SKIRT: f32 = 2.4;

/// A graded run of ground between two sites.
///
/// Public because `Settlements::ways` hands them out to a test probe; nothing
/// outside this file reads their fields.
#[derive(Clone)]
pub struct Road {
    pub from: Vec2,
    pub to: Vec2,
    /// The height the road holds, sampled along its length and then graded.
    ///
    /// **Not a straight line between the two ends.** That is what it was, and it
    /// is what cut gorges: two towns a couple of kilometres apart with a hill
    /// between them got a road at the straight-line height all the way, so the
    /// hill was carved through to it — tens of metres deep, with a skirt of
    /// twenty-six to blend the walls. A road follows the land and cuts only what
    /// it must to stay walkable.
    profile: Vec<f32>,
    /// How deep the road cuts at each of those same stations: the ground it was
    /// graded through, minus the height it settled on.
    ///
    /// Kept rather than worked out on demand, and that is the point. How far a
    /// cutting's sides have to reach depends on how deep it is — but if "how
    /// deep" is measured at the point being ASKED about, the pull varies across
    /// the section and the surface comes out scalloped. Measured along the road
    /// instead, a section is one clean ramp.
    cuts: Vec<f32>,
}

/// Everything levelled, with a coarse grid over it so a height lookup only ever
/// tests the handful of features near it rather than all of them.
///
/// This matters more than it looks: the height field is asked millions of times
/// to mesh a world, and a linear scan over every site and road at each sample
/// would dominate the whole generator.
/// A flat pad of ground under one building.
///
/// # A building is seated on its highest corner, so the ground has to be level
///
/// `world::town::stands_at` puts a building on the HIGHEST of its footprint corners,
/// because one seated on the average is one you walk into the roof of at the high
/// end. The cost lands at the other end: on any slope the low corner hangs, and the
/// wider the footprint the further. A guild hall is 26 m across and hung visibly.
///
/// It was filled with a footing - a stone skirt from the floor down to the ground -
/// which is what a real building on a slope has and which is still there for the
/// last centimetres. What it could not do is stop the ground being uneven in the
/// first place, and a building that needs half a metre of masonry to meet its own
/// site is a building standing on the wrong ground.
///
/// So the ground is levelled under each one, the same way it is already levelled
/// under a town and along its streets. Nothing new was needed to work out where the
/// buildings are: `lay_the_streets` has been calling `town::lay_out` all along, with
/// the seed the game itself uses, and walking `layout.streets` while `layout.plots`
/// sat beside it untouched.
pub struct Pad {
    at: Vec2,
    half: Vec2,
    facing: f32,
    height: f32,
}

impl Pad {
    /// How far outside the pad a point lies, nought anywhere on it.
    ///
    /// The box distance, in the pad's own frame - a building is a rectangle and a
    /// circle round it would level a disc, which on a street of them reads as a row
    /// of saucers.
    fn off(&self, at: Vec2) -> f32 {
        let away = at - self.at;
        let (sin, cos) = self.facing.sin_cos();
        // Into the pad's frame: the inverse of the turn `Plot::walls` builds with.
        let local = Vec2::new(away.x * cos + away.y * sin, -away.x * sin + away.y * cos);
        let out = local.abs() - self.half;
        out.max(Vec2::ZERO).length() + out.x.max(out.y).min(0.0)
    }
}

/// How far a building's pad eases back into the ground around it, in metres.
///
/// Short. A pad is a terrace cut for one building, not a second town square: at ten
/// metres the pads of neighbouring houses merge into one plateau and the street
/// between them stops reading as ground at all.
const PAD_SKIRT: f32 = 2.5;

/// How much wider a pad's skirt grows with the building on it.
///
/// # A terrace has to be walkable off as well as onto
///
/// A pad levels to the ground at the building's own middle, so the drop at its rim
/// is whatever the land falls across the footprint - and the wider the footprint the
/// more that is. Resolved over a fixed 2.5 m it became a step: `walking_into_a_city
/// _is_not_stopped_by_anything_invisible` found the warden refused at 0.28 m of rise
/// in a 0.12 m stride, two centimetres past what `player::STEP_UP` allows, at the rim
/// of a guild hall's pad. An invisible wall round the middle of six cities.
///
/// So the skirt grows with the building, which is the same shape `road_skirt` uses
/// for a cutting: what has to be resolved is proportional to the thing making it, so
/// the distance to resolve it over must be too.
const PAD_SPREADS: f32 = 0.4;

/// How far past the footprint the pad stays perfectly flat, in metres.
///
/// # The ground is sampled more coarsely than the pad is drawn
///
/// `drawn_height` reads the terrain at grid vertices two metres apart and
/// interpolates between them, so the height AT a building's corner is a blend of
/// vertices that may be up to two metres outside the footprint. With the pad ending
/// exactly at the footprint, those outer vertices were on the slope, and the corner
/// inherited a share of it: measured at 0.39 m of fall on a shop, down from 2.01 but
/// not down to nothing.
///
/// So the flat reaches a grid step and a half past the building. Wider than needed,
/// because the cost is a metre of extra terrace nobody will notice and the failure is
/// a building on a tilt.
const PAD_HOLDS: f32 = 2.5;

pub struct Settlements {
    sites: Vec<Site>,
    roads: Vec<Road>,
    /// Every water crossing the network needs, as structures to be built.
    bridges: Vec<Bridge>,
    lanes: Vec<Lane>,
    /// Level ground under every building. See `Pad`.
    pads: Vec<Pad>,
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
            bridges: Vec::new(),
            lanes: Vec::new(),
            pads: Vec::new(),
            cells: Vec::new(),
            cells_across: 0,
            cells_down: 0,
            half: Vec2::ONE,
        }
    }

    /// The roads themselves.
    ///
    /// No longer test-only: `world::town` draws them. They were graded into the
    /// terrain and never rendered, so the only sign a road existed was a
    /// suspiciously level line of grass.
    /// Every bridge the network needs.
    pub fn spans(&self) -> &[Bridge] {
        &self.bridges
    }

    /// How built-up the ground here is, and of which age of the world.
    ///
    /// # A city standing on a lawn
    ///
    /// A settlement LEVELS its ground and never changed its surface, so a
    /// photograph from the middle of a city showed skyscrapers and a guild hall
    /// standing on unbroken meadow, with a market square that was a circle of grass.
    /// Nothing was wrong with any of it: the levelling worked, the buildings stood
    /// where they should, and the ground underneath was still open country because
    /// nobody had ever told it otherwise.
    ///
    /// Signed, the same trick `worn` uses for its own second meaning. POSITIVE is
    /// packed earth - the old world's yards and thoroughfares. NEGATIVE is paving,
    /// which is what the modern cities stand on. Zero is country.
    ///
    /// Fades over the outer part of the site rather than stopping at a line, so a
    /// town's ground gives way to grass instead of ending in a disc you can see from
    /// the air.
    pub fn ground_at(&self, at: Vec2) -> f32 {
        let mut most = 0.0f32;
        for site in &self.sites {
            if site.ranch {
                continue;
            }
            let away = site.at.distance(at);
            let reach = site.radius * SETTLED_REACHES;
            if away > reach {
                continue;
            }
            // Solid across the built part, easing out over the last of it.
            let share = crate::util::smoothstep(reach, site.radius * SETTLED_SOLID, away);
            let signed = if site.city { -share } else { share };
            if signed.abs() > most.abs() {
                most = signed;
            }
        }
        most
    }

    /// The height of a bridge deck over this point, if a bridge carries it.
    ///
    /// This is what makes a bridge walkable. Collision in this game is upright
    /// walls and there is no floor in it anywhere, because until bridges the floor
    /// was always the terrain - so rather than grow a second collision system for
    /// one case, the GROUND answers differently where a bridge is. The terrain is
    /// not touched: the water still renders as water and nothing grows on the deck.
    /// Only what the warden stands on changes, which is all a bridge has to change.
    pub fn deck_at(&self, at: Vec2) -> Option<f32> {
        self.bridges.iter().find_map(|bridge| {
            let run = bridge.to - bridge.from;
            let length2 = run.length_squared().max(1.0e-4);
            let along = ((at - bridge.from).dot(run) / length2).clamp(0.0, 1.0);
            let on = bridge.from + run * along;
            (at.distance(on) < crate::world::bridge::ROADWAY_WIDE * 0.5).then_some(bridge.deck)
        })
    }

    pub fn ways(&self) -> &[Road] {
        &self.roads
    }

    pub fn sites(&self) -> &[Site] {
        &self.sites
    }

    /// Which way the road network arrives at this site, as a flat unit vector.
    ///
    /// A town's high street runs along the road that got there - that is why the
    /// town is there at all - so the layout in `world::town` is built on this axis
    /// rather than on an angle from a hash. The difference is a town ON a road
    /// versus a town BESIDE one.
    ///
    /// Averaged over every road that meets the site, taken as an UNSIGNED axis
    /// rather than a direction: two roads leaving opposite sides of a town are one
    /// through route, and averaging them as directions would cancel them to nothing
    /// and hand back a hash's guess for the busiest street in the world.
    pub fn approach(&self, at: Vec2) -> Vec2 {
        let mut axis = Vec2::ZERO;
        for road in &self.roads {
            for (end, other) in [(road.from, road.to), (road.to, road.from)] {
                if end.distance(at) > CELL {
                    continue;
                }
                let run = other - end;
                if run.length_squared() < 1.0 {
                    continue;
                }
                let run = run.normalize();
                // Folded onto a half-turn, so a road leaving north and one leaving
                // south agree instead of cancelling.
                let folded = if run.x < 0.0 { -run } else { run };
                axis += folded;
            }
        }
        if axis.length_squared() > 1.0e-6 {
            axis.normalize()
        } else {
            Vec2::X
        }
    }

    pub fn roads_len(&self) -> usize {
        self.roads.len()
    }

    /// Works out where the towns go and grades the roads between them.
    ///
    /// `ground` answers the height BEFORE any of this is applied, `shore` the
    /// distance to the coast, and `wet` whether a river would be drawn at a
    /// point. All three must be free of settlements or this would be reading its
    /// own output.
    pub fn plan(
        half: Vec2,
        ground: &dyn Fn(Vec2) -> f32,
        shore: &dyn Fn(Vec2) -> f32,
        wet: &dyn Fn(Vec2) -> bool,
        // What a road pays for crossing a place, on top of the ground itself.
        avoid: &dyn Fn(Vec2) -> f32,
    ) -> Self {
        let mut sites: Vec<Site> = Vec::new();

        // The ranch first, before anything that could take its ground.
        //
        // It is the one place on the map chosen by hand rather than found: the
        // game starts here, so it is picked by eye and pinned. Everything below
        // keeps its distance from whatever is already placed, so putting it in
        // first is the whole of what protects it.
        let ranch = Vec2::new(RANCH_AT.0, RANCH_AT.1);
        sites.push(Site {
            at: ranch,
            height: ground(ranch),
            radius: RANCH_RADIUS,
            city: false,
            ranch: true,
        });

        // Then the thirteen that are actually on the map, exactly where they were
        // put. No rejection sampling and no quotas: `SETTLEMENTS` is the list.
        for (x, z, city) in SETTLEMENTS {
            let at = Vec2::new(x, z);
            sites.push(Site {
                at,
                height: ground(at),
                radius: if city { CITY_RADIUS } else { TOWN_RADIUS },
                city,
                ranch: false,
            });
        }

        // Only if the world is still laying its own. Kept as a switch rather than
        // torn out: the grading is tested and a maker may want a linked network
        // again, but a hand-laid road beats a generated one every time because
        // somebody is looking at the country while they lay it.
        let (roads, bridges) = if LINK_TOWNS_WITH_ROADS {
            link(&sites, ground, half, avoid)
        } else {
            (Vec::new(), Vec::new())
        };
        let mut settlements = Settlements {
            sites,
            roads,
            bridges,
            lanes: Vec::new(),
            pads: Vec::new(),
            cells: Vec::new(),
            cells_across: 0,
            cells_down: 0,
            half,
        };
        settlements.index();
        // The streets inside each town, once there are sites and roads for the
        // layout to be built from. Filed as claims like everything else, so from
        // here on the ground itself knows where a street is.
        let (lanes, pads) = settlements.lay_the_town_out();
        settlements.lanes = lanes;
        settlements.pads = pads;
        settlements.index();
        settlements
    }

    /// Every town's streets and every building's pad, as claims on the ground.
    ///
    /// One walk of the layouts for both, because they come from the same one: this
    /// used to take the streets and leave `layout.plots` sitting beside them, so the
    /// ground was levelled along every road in a town and left uneven under every
    /// house on it.
    fn lay_the_town_out(&self) -> (Vec<Lane>, Vec<Pad>) {
        let mut lanes = Vec::new();
        let mut pads = Vec::new();
        for (index, site) in self.sites.iter().enumerate() {
            let layout = crate::world::town::lay_out(
                site,
                self.approach(site.at),
                WORLD_SEED.wrapping_add(index as u32 * 7717),
            );
            for street in &layout.streets {
                lanes.push(Lane {
                    from: street.from,
                    to: street.to,
                    height: site.height,
                    wide: street.wide,
                });
            }
            for plot in &layout.plots {
                pads.push(Pad {
                    at: plot.at,
                    // The footprint exactly. It is what the building keeps clear on
                    // the ground and what `stands_at` reads its corners from, so
                    // levelling anything else would level the wrong rectangle.
                    half: plot.what.footprint() * 0.5,
                    facing: plot.facing,
                    // The town's own level, which is what the streets outside the
                    // door are already at.
                    height: site.height,
                });
            }
        }
        (lanes, pads)
    }

    /// The pad a point stands on, as its middle and how firmly it holds.
    ///
    /// # Why this is asked separately, and last
    ///
    /// `level` is applied to the GENERATED ground, and `Terrain::height` adds the
    /// sculpted edit layer on top of the result. That order is right for a town and
    /// a road - somebody brushing the terrain is entitled to reshape a hillside a
    /// road crosses - and it is wrong for the ground directly under a building,
    /// which cannot be uneven whatever anybody has brushed onto it.
    ///
    /// Measured before it was believed: with pads in `level`, a guild hall in the
    /// world still stood on ground falling 2.01 m across its own footprint, because
    /// it sits on sculpted terrain and the sculpting lands afterwards.
    ///
    /// This returns the pad's MIDDLE rather than a height, so the caller can level to
    /// whatever the ground at that middle actually is - edits and all - instead of to
    /// a generated height the sculpting may have moved a long way from.
    /// `level_at` is asked for the ground at a pad's middle, edits and all.
    pub fn pad_under(&self, at: Vec2, level_at: impl Fn(Vec2) -> f32) -> Option<(f32, f32)> {
        let (x, y) = self.cell_of(at);
        let cell = self.cells.get((y * self.cells_across + x) as usize)?;
        let first = self.sites.len() as u16 + self.roads.len() as u16 + self.lanes.len() as u16;

        // THE STRONGEST CLAIM GOVERNS THE PULL AND EVERY CLAIM SHARES THE TARGET.
        //
        // This kept only the strongest pad and levelled to ITS middle, which is a
        // seam waiting for two pads to overlap: where the two pulls cross, the
        // winner changes in one sample and the target jumps from one building's
        // ground to the other's. `Settlements::level` documents the same trap and
        // avoids it the same way, and Codex spotted that the pads had not been given
        // the treatment their own neighbours already had.
        //
        // Pads overlap by construction now - the skirt grows with the footprint, so
        // a guild hall's reaches 8 m past its walls - and a compact village puts
        // three of them over one doorstep.
        let mut governs = 0.0_f32;
        let mut target = 0.0_f32;
        let mut shares = 0.0_f32;
        for &what in cell {
            if what < first {
                continue;
            }
            let pad = &self.pads[(what - first) as usize];
            let skirt = PAD_SKIRT + pad.half.length() * PAD_SPREADS;
            let pull = smoothstep(PAD_HOLDS + skirt, PAD_HOLDS, pad.off(at));
            if pull <= 0.0 {
                continue;
            }
            governs = governs.max(pull);
            // WEIGHTED SO A PAD AT FULL PULL IS THE ONLY CLAIM THAT COUNTS.
            //
            // Sharing by `pull` alone is continuous and flattens nothing: inside a
            // shop's own footprint its neighbour still gets a vote, and the ground
            // under the shop came out falling 38 cm - measured, by the guard that
            // exists for exactly that. Sharing by the strongest alone is flat and
            // jumps where the winner changes, which is the seam Codex found.
            //
            // So the weight rises to infinity as a pad saturates, the way inverse
            // distance weighting does. A point on a building's own pad has that
            // pad's pull at 1 and every other claim vanishes beside it; a point out
            // in the skirt has no saturated claim and they all share smoothly; and
            // the approach from one to the other is continuous, which is the whole
            // reason not to just pick a winner.
            let share = pull / (1.0 - pull).max(1.0e-4);
            target += level_at(pad.at) * share;
            shares += share;
        }
        (shares > 0.0).then(|| (target / shares, governs))
    }

    pub fn lanes(&self) -> &[Lane] {
        &self.lanes
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
        // The widest a road can ever pull, not the narrowest. Filing them by the
        // unbattered skirt is what put a wall down each side of every one.
        // COUNTRY ROADS ARE NOT FILED, because they no longer move any earth.
        //
        // A road's profile is the ground it crosses, so levelling against it is an
        // identity - except that it is sampled at stations and interpolated straight
        // between them, which on curved ground flattens a little, and the edge of
        // that little flattening is a step. Measured: 1.64 m over a quarter-metre.
        //
        // They are a SURFACE now, drawn by `world::town`, so the terrain does not
        // need to hear about them at all. `roads` is still kept and still shapes
        // `approach` - which way a town's high street runs - it simply does not
        // reach the levelling.
        let offset = self.sites.len() as u16;
        let lane_offset = offset + self.roads.len() as u16;
        for (i, lane) in self.lanes.iter().enumerate() {
            let reach = lane.wide * 0.5 + LANE_SKIRT;
            let low = lane.from.min(lane.to) - reach;
            let high = lane.from.max(lane.to) + reach;
            filings.push((lane_offset + i as u16, low, high));
        }
        let pad_offset = lane_offset + self.lanes.len() as u16;
        for (i, pad) in self.pads.iter().enumerate() {
            // The half-diagonal, because a pad may be turned any way and the box it
            // needs filing under is the one that contains it however it lies.
            let reach = pad.half.length() * (1.0 + PAD_SPREADS) + PAD_HOLDS + PAD_SKIRT;
            filings.push((pad_offset + i as u16, pad.at - reach, pad.at + reach));
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
    /// The height this place has been levelled to, and how strongly.
    pub fn level(&self, at: Vec2) -> Option<(f32, f32)> {
        let (x, y) = self.cell_of(at);
        let cell = self.cells.get((y * self.cells_across + x) as usize)?;
        if cell.is_empty() {
            return None;
        }

        // # The strongest claim decides HOW MUCH, and all of them decide WHAT
        //
        // This took the strongest claim outright, target and all — and that leaves
        // a step wherever two claims cross. At a crossover the two pulls are equal
        // and the two targets are not, so the winner switches from one height to
        // the other in the space of one vertex while the pull carries on smoothly.
        // Reported as a raised section that could not be smoothed out with the
        // brush; measured at 8.6 m of step between neighbours two metres apart,
        // which is a pull of 0.47 times about eighteen metres of disagreement.
        //
        // It is the third time this shape has come up here. The biome boundary did
        // it (the category flipped at the threshold while the strength carried on)
        // and so did the painted country. The answer is the same each time: a thing
        // that flips cannot be the thing that varies.
        //
        // So the HEIGHT is now blended across every claim, weighted by the cube of
        // each pull, and only the STRENGTH is the strongest claim. Cubed, because
        // the original note is right that a road meeting a town should join the
        // town's level rather than splitting the difference — at any real distance
        // the dominant claim is overwhelming, and the blend only shows in the narrow
        // band where two pulls are genuinely comparable. Which is precisely where a
        // step must not be.
        let mut wanted = 0.0;
        let mut wanting = 0.0f32;
        let mut weight = 0.0f32;
        let sites = self.sites.len() as u16;

        let roads = sites + self.roads.len() as u16;
        let pads = roads + self.lanes.len() as u16;

        for &what in cell {
            let (height, pull) = if what >= pads {
                // NOT HERE. A building's pad has the last word instead - see
                // `pad_under` and `Terrain::height` - because this levelling happens
                // BEFORE the sculpted edit layer, and a pad that flattens the
                // generated ground and is then brushed into a slope has done nothing.
                continue;
            } else if what >= roads {
                // A town's own street. Narrow and firm: flat across its width and
                // done a couple of metres past the kerb, where a site's claim
                // fades over a hundred metres and a road's over its whole batter.
                // That is what makes a street read as a street through the grass
                // rather than as one more patch of levelled ground.
                let lane = &self.lanes[(what - roads) as usize];
                let away = lane.off(at);
                (
                    lane.height,
                    smoothstep(lane.wide * 0.5 + LANE_SKIRT, lane.wide * 0.5, away),
                )
            } else if what < sites {
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
                let height = road.height_at(along);
                // And its sides are battered in proportion to the cut. A fixed
                // skirt had to resolve a thirty-metre cut over forty-four metres,
                // which is a gorge; widening with the depth turns the same cut
                // into a slope you can walk up.
                // From the road's own recorded cut at this point ALONG it, not
                // from the ground at the point being asked about — that made the
                // pull vary across a section and scalloped the sides.
                let skirt = road_skirt(road.cut_at(along));
                (height, smoothstep(ROAD_WIDTH + skirt, ROAD_WIDTH, away))
            };
            if pull <= 0.0 {
                continue;
            }
            let say = pull * pull * pull;
            wanted += height * say;
            wanting += say;
            weight = weight.max(pull);
        }

        (weight > 0.0 && wanting > 0.0).then(|| (wanted / wanting, weight))
    }
}

/// How far a cutting's sides reach, for a cut of a given depth.
///
/// The fix for roads that came out as gorges. A fixed skirt has to resolve
/// however deep a cut is over the same distance, so the deeper the cut the
/// steeper its walls — thirty metres over forty-four is thirty-four degrees, and
/// the eye reads that as blasted rock rather than a road.
fn road_skirt(depth: f32) -> f32 {
    (ROAD_SKIRT + depth.abs() * ROAD_BATTER).min(ROAD_MAX_SKIRT)
}

/// Reads a value sampled along a road, between its stations.
///
/// One reader for the height and the cut both, because they are sampled at the
/// same stations and a second copy of this arithmetic is a second chance for the
/// two to be read out of step with each other.
fn read_along(sampled: &[f32], along: f32) -> f32 {
    if sampled.len() < 2 {
        return sampled.first().copied().unwrap_or(0.0);
    }
    let last = sampled.len() - 1;
    let step = (along.clamp(0.0, 1.0) * last as f32).min(last as f32 - 1.0e-4);
    let low = step.floor() as usize;
    let t = step - low as f32;
    sampled[low] * (1.0 - t) + sampled[low + 1] * t
}

impl Road {
    /// The graded height a fraction of the way along, read between samples.
    fn height_at(&self, along: f32) -> f32 {
        read_along(&self.profile, along)
    }

    /// How deep the road cuts at a point along it.
    fn cut_at(&self, along: f32) -> f32 {
        read_along(&self.cuts, along)
    }

    /// Distance to this road, and how far along it was, 0 to 1.
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
/// Connects every site into one network, and grades the run between each pair
/// against the land it actually crosses.
/// A bridge: a structure carried over water, with a shore at each end.
///
/// It is NOT a raising of the seabed. Filling water in until a road can drive over
/// it is a causeway, and a causeway would quietly redraw the coastline, move the
/// biome that follows the coastline, and put a road where the map says sea.
#[derive(Clone, Copy, Debug)]
pub struct Bridge {
    /// The shore it leaves.
    pub from: Vec2,
    /// The shore it lands on.
    pub to: Vec2,
    /// The height its deck holds, which is one height for the whole span.
    pub deck: f32,
}

/// Turns a walk over the ground into road segments.
///
/// The walk comes out of the survey as cell middles, so it steps in 64 m squares
/// and has the staircase you would expect. Two passes of averaging pull it into
/// curves - and every moved point is checked back against the ground, because a
/// smoothing that cuts a corner across a headland puts the road in the sea, which
/// is the whole thing this module exists to prevent.
fn lay(roads: &mut Vec<Road>, ground: &dyn Fn(Vec2) -> f32, walk: &[Vec2]) {
    if walk.len() < 2 {
        return;
    }
    let mut points = walk.to_vec();
    for _ in 0..2 {
        let was = points.clone();
        for at in 1..was.len() - 1 {
            let eased = (was[at - 1] + was[at] * 2.0 + was[at + 1]) * 0.25;
            // Only if it is still on land. A corner cut across a bay is not a curve.
            if ground(eased) > SEA_LEVEL + 1.0 {
                points[at] = eased;
            }
        }
    }

    for pair in points.windows(2) {
        let (foot, head) = (pair[0], pair[1]);
        if foot.distance(head) < 0.5 {
            continue;
        }
        // NO EARTHWORKS. The profile IS the ground - a road between towns is a
        // surface laid on the country, so it cannot leave a step to fall off or
        // disturb a biome.
        let steps = ((foot.distance(head) / ROAD_STEP).ceil() as usize).clamp(1, 512);
        let profile: Vec<f32> = (0..=steps)
            .map(|i| ground(foot.lerp(head, i as f32 / steps as f32)))
            .collect();
        let cuts = vec![0.0; profile.len()];
        roads.push(Road { profile, cuts, from: foot, to: head });
    }
}

/// Grows the road network outward over land, joining whatever it can reach.
///
/// Prim's, with the walk's own cost for its edges. A pair with no dry walk between
/// them has no edge at all, so the tree grows exactly as far as the land goes.
fn reach_out(
    sites: &[Site],
    land: &crate::world::route::Land,
    reach: &[Option<crate::world::route::Reach>],
    joined: &mut [bool],
    roads: &mut Vec<Road>,
    ground: &dyn Fn(Vec2) -> f32,
) {
    loop {
        let mut best: Option<(f32, usize, usize)> = None;
        for (i, from) in reach.iter().enumerate() {
            if !joined[i] {
                continue;
            }
            let Some(from) = from else { continue };
            for (j, site) in sites.iter().enumerate() {
                if joined[j] {
                    continue;
                }
                let Some(cost) = from.cost_to(land, site.at) else {
                    continue;
                };
                if best.is_none_or(|(was, _, _)| cost < was) {
                    best = Some((cost, i, j));
                }
            }
        }
        let Some((_, from, to)) = best else { break };
        joined[to] = true;
        if let Some(walk) = reach[from]
            .as_ref()
            .and_then(|r| r.route_to(land, sites[to].at))
        {
            lay(roads, ground, &walk);
        }
    }
}

/// Joins the settlements up: roads over land, bridges over water.
///
/// # Not the shortest path
///
/// Both halves of this used to be decided by straight-line distance - which pair of
/// settlements to join, and where the road between them went. Neither question can
/// be answered by a straight line on a world with water in it, and the result was
/// roads that ran into a lake and out the far side.
///
/// Now the ground answers both. `route::Land` surveys what is walkable once, and
/// every cost here is the cost of an actual walk over dry ground: a settlement on
/// the far side of a bay is FAR, whatever the map ruler says, and the road that
/// goes there goes round. Long is by design.
///
/// Settlements no walk can reach are on another landmass. Those get a bridge at the
/// narrowest crossing between the two shores, and a road out to it at each end.
fn link(
    sites: &[Site],
    ground: &dyn Fn(Vec2) -> f32,
    half: Vec2,
    avoid: &dyn Fn(Vec2) -> f32,
) -> (Vec<Road>, Vec<Bridge>) {
    let mut roads = Vec::new();
    let mut bridges = Vec::new();
    if sites.len() < 2 {
        return (roads, bridges);
    }

    let land = crate::world::route::Land::survey(half, ground, avoid);
    let reach: Vec<Option<crate::world::route::Reach>> =
        sites.iter().map(|site| land.walk_from(site.at)).collect();
    let islands: Vec<Option<u16>> = sites.iter().map(|site| land.island_at(site.at)).collect();

    let mut joined = vec![false; sites.len()];
    joined[0] = true;
    reach_out(sites, &land, &reach, &mut joined, &mut roads, ground);

    // Whatever the tree could not reach is across water. Each such landmass is
    // brought in by ONE bridge at the narrowest place between it and a shore that is
    // already on the network, with a road at each end to the nearest settlement on
    // that side. Then the walk above carries on over the new ground.
    loop {
        let Some(orphan) = (0..sites.len()).find(|i| !joined[*i]) else {
            break;
        };
        let Some(want) = islands[orphan] else {
            joined[orphan] = true;
            continue;
        };

        let mut best: Option<crate::world::route::Crossing> = None;
        let mut asked: Vec<u16> = Vec::new();
        for (i, island) in islands.iter().enumerate() {
            if !joined[i] {
                continue;
            }
            let Some(have) = island else { continue };
            if *have == want || asked.contains(have) {
                continue;
            }
            asked.push(*have);
            if let Some(crossing) = land.crossing(*have, want) {
                if best.is_none_or(|had| crossing.span < had.span) {
                    best = Some(crossing);
                }
            }
        }

        let Some(crossing) = best else {
            // Nothing near enough to bridge. Marked done so the loop ends: a
            // landmass this far out is simply not on the road network, and
            // pretending otherwise would mean a road across open sea.
            for (i, island) in islands.iter().enumerate() {
                if *island == Some(want) {
                    joined[i] = true;
                }
            }
            continue;
        };

        // One height for the whole deck, taken from the higher shore so neither end
        // steps down onto the water.
        bridges.push(Bridge {
            from: crossing.from,
            to: crossing.to,
            deck: ground(crossing.from).max(ground(crossing.to)),
        });

        for head in [crossing.from, crossing.to] {
            let mut nearest: Option<(f32, usize)> = None;
            for (i, from) in reach.iter().enumerate() {
                let Some(from) = from else { continue };
                let Some(cost) = from.cost_to(&land, head) else {
                    continue;
                };
                if nearest.is_none_or(|(was, _)| cost < was) {
                    nearest = Some((cost, i));
                }
            }
            if let Some((_, i)) = nearest {
                if let Some(walk) = reach[i].as_ref().and_then(|r| r.route_to(&land, head)) {
                    lay(&mut roads, ground, &walk);
                }
            }
        }

        joined[orphan] = true;
        reach_out(sites, &land, &reach, &mut joined, &mut roads, ground);
    }

    (roads, bridges)
}

/// Works out the height a road holds along its length.
///
/// Samples the land it crosses, then walks the profile back and forth moving
/// height between neighbours until no step is steeper than a cart can manage.
/// Material moves BOTH ways — cut off the rises, filled into the dips — so what
/// comes out follows the country instead of ignoring it.
///
/// The ends are pinned to their towns after every pass. A road that grades itself
/// beautifully and then does not meet the town it leads to is no use.
fn grade(
    ground: &dyn Fn(Vec2) -> f32,
    from: Vec2,
    to: Vec2,
    from_height: f32,
    to_height: f32,
) -> Vec<f32> {
    let length = from.distance(to);
    let steps = ((length / ROAD_STEP).ceil() as usize).clamp(1, 512);
    let step = length / steps as f32;

    let mut profile: Vec<f32> = (0..=steps)
        .map(|i| {
            let along = i as f32 / steps as f32;
            ground(from.lerp(to, along))
        })
        .collect();

    let most = ROAD_GRADE * step;
    for _ in 0..GRADE_PASSES {
        for i in 1..profile.len() {
            settle_pair(&mut profile, i - 1, i, most);
        }
        for i in (1..profile.len()).rev() {
            settle_pair(&mut profile, i - 1, i, most);
        }
        // Pinned last, so the towns always win.
        *profile.first_mut().unwrap() = from_height;
        *profile.last_mut().unwrap() = to_height;
    }
    profile
}

/// Moves height between two neighbouring samples until the step between them is
/// something a cart could take, giving half the correction to each.
fn settle_pair(profile: &mut [f32], low: usize, high: usize, most: f32) {
    let drop = profile[high] - profile[low];
    if drop.abs() <= most {
        return;
    }
    let excess = (drop.abs() - most) * 0.5 * drop.signum();
    profile[high] -= excess;
    profile[low] += excess;
}

/// How steep the ground is at a point, sampled wide enough to catch a hillside
/// rather than a bump.
/// Whether any river would be drawn inside a site of this size.
///
/// Stepped at half the river grid's own spacing. The narrowest channel is seven
/// metres across and the grid it is recorded on is twenty, so anything coarser
/// than this could step straight over one and call the ground dry.
fn crosses_water(wet: &dyn Fn(Vec2) -> bool, at: Vec2, radius: f32) -> bool {
    let step = RIVER_SPACING * 0.5;
    let mut dz = -radius;
    while dz <= radius {
        let mut dx = -radius;
        while dx <= radius {
            if dx * dx + dz * dz <= radius * radius && wet(at + Vec2::new(dx, dz)) {
                return true;
            }
            dx += step;
        }
        dz += step;
    }
    false
}

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

#[cfg(test)]
mod roads {
    use super::*;

    /// The steepest the side of a cutting is allowed to be, as a rise over run.
    /// About twenty-five degrees — steeper than that and it reads as a wall.
    const WALKABLE: f32 = 0.47;

    /// The slope of a cutting's wall, for a cut of a given depth.
    ///
    /// The cut resolves from the edge of the bed out to the end of the skirt, so
    /// its average wall slope is the depth over that reach.
    fn wall_slope(depth: f32) -> f32 {
        depth / road_skirt(depth)
    }

    #[test]
    fn a_cutting_is_battered_rather_than_cut_square() {
        // What the complaint was: roads between towns arriving as gorges. A road
        // holding its grade across a ridge cuts through, which is what a road
        // does; the fault was that the cut blended back into the land over a
        // FIXED forty-four metres however deep it was.
        for depth in [2.0_f32, 5.0, 10.0, 20.0, 30.0, 45.0, 60.0] {
            let slope = wall_slope(depth);
            assert!(
                slope <= WALKABLE,
                "a {depth:.0} m cut has {slope:.2} walls, over {:.0} m of skirt",
                road_skirt(depth)
            );
        }
    }

    #[test]
    fn the_fixed_skirt_really_was_the_gorge() {
        // Kept as the reason the constant exists, so nobody has to take the claim
        // on trust: with no batter at all, a deep cut is a trench.
        let square = 30.0 / ROAD_SKIRT;
        assert!(
            square > WALKABLE * 1.4,
            "the old fixed skirt should be demonstrably too steep: {square:.2}"
        );
        assert!(
            wall_slope(30.0) < square * 0.5,
            "battering should at least halve it"
        );
    }

    /// A steep hill between two towns, both down at ten metres.
    fn hill(at: Vec2) -> f32 {
        let across = (at.x / 220.0).clamp(-1.0, 1.0);
        10.0 + 70.0 * (1.0 - across * across).max(0.0)
    }

    #[test]
    fn a_road_climbs_a_hill_instead_of_cutting_through_it() {
        // The gorges. A road graded as a straight line between two towns at the
        // same height holds that height under everything between them, so a
        // seventy-metre hill was carved out to a seventy-metre trench with a
        // skirt of twenty-six to blend its walls.
        let (from, to) = (Vec2::new(-700.0, 0.0), Vec2::new(700.0, 0.0));
        let profile = grade(&hill, from, to, 10.0, 10.0);
        let steps = profile.len() - 1;
        let step = from.distance(to) / steps as f32;

        let mut deepest = 0.0_f32;
        for (i, &height) in profile.iter().enumerate() {
            let along = i as f32 / steps as f32;
            deepest = deepest.max(hill(from.lerp(to, along)) - height);
        }
        // A straight line cuts the full seventy. Following the land, what is
        // left is a shallow notch over the crown rather than a canyon.
        assert!(
            deepest < 18.0,
            "still cutting {deepest:.0} m out of the hill"
        );

        // And it is still a road: nothing steeper than a cart could take.
        for (i, pair) in profile.windows(2).enumerate() {
            let grade = (pair[1] - pair[0]).abs() / step;
            assert!(
                grade <= ROAD_GRADE * 1.4,
                "step {i} climbs at {grade:.2}, steeper than {ROAD_GRADE}"
            );
        }

        // A road that grades itself beautifully and misses the town it leads to
        // is no use.
        assert!((profile[0] - 10.0).abs() < 1.0e-3, "the near end must meet its town");
        assert!((profile[steps] - 10.0).abs() < 1.0e-3, "and so must the far end");
    }

    #[test]
    fn ground_a_cart_could_already_take_is_left_alone() {
        // A gentle rise inside the grade needs no earthworks at all, and a road
        // that levels it anyway is the same fault in miniature.
        let slope = |at: Vec2| 10.0 + (at.x + 700.0) * 0.05;
        let (from, to) = (Vec2::new(-700.0, 0.0), Vec2::new(700.0, 0.0));
        let profile = grade(&slope, from, to, 10.0, 80.0);
        let steps = profile.len() - 1;

        for (i, &height) in profile.iter().enumerate() {
            let along = i as f32 / steps as f32;
            let natural = slope(from.lerp(to, along));
            assert!(
                (height - natural).abs() < 1.5,
                "sample {i} moved {:.1} m for no reason",
                height - natural
            );
        }
    }
}

#[cfg(test)]
mod levelling {
    use super::*;

    /// Levelling never puts a step in the ground.
    ///
    /// # What went wrong, and why nothing caught it
    ///
    /// `level` returned the strongest claim's TARGET as well as its strength. Two
    /// claims that cross — a road running into a town, two roads meeting — have
    /// equal pull and unequal target at the crossover, so the height snapped from
    /// one to the other between two vertices while the pull carried on smoothly.
    /// Found as a raised section a maker could not smooth out with the brush, and
    /// they could not: the sculpt layer is four-metre cells and cannot express the
    /// inverse of a step that sharp, and the generator was re-applying it underneath
    /// regardless. Measured at 8.6 m of step between neighbours two metres apart.
    ///
    /// Every road and town test passed throughout. They ask about gradients ALONG a
    /// road and about the width of its cutting — real questions, none of which walks
    /// across the seam BETWEEN two features, which is the only place this showed.
    ///
    /// # It measures the levelling, not the ground
    ///
    /// The first cut of this test bounded the total height step near a town, and it
    /// failed on a mountainside 240 m away where no settlement had any claim at all
    /// — ordinary terrain is allowed to be a cliff. What must not have a step in it
    /// is what LEVELLING ADDS, so that is what is measured: `(target - dry) * pull`,
    /// which is zero where nothing claims the ground and cannot be confounded by
    /// whatever the ground was doing already.
    #[test]
    fn levelling_never_puts_a_step_in_the_ground() {
        let terrain = crate::world::terrain::Terrain::new();
        let plan = terrain.plan();
        // MUCH finer than the terrain's own two-metre vertices, and that is the
        // instrument. At two metres a step and a steep ramp look alike — a road
        // cut into a hillside legitimately grades a couple of metres over two. At a
        // quarter of a metre a ramp shrinks in proportion and a discontinuity does
        // not, so what is left above the bound can only be a jump.
        let step = crate::config::CHUNK_SIZE / crate::config::CHUNK_QUADS as f32 / 8.0;

        // What levelling does to this point, and nothing else.
        let moved = |at: Vec2| {
            plan.level(at).map_or(0.0, |(target, pull)| {
                (target - terrain.dry_height(at.x, at.y)) * pull
            })
        };

        let mut worst = 0.0_f32;
        let mut worst_at = Vec2::ZERO;
        let mut looked = 0;
        for site in terrain.sites() {
            let reach = site.radius + SITE_SKIRT + 40.0;
            let ticks = (reach * 2.0 / step) as i32;
            for lane in -4..=4 {
                let offset = lane as f32 * reach / 4.0;
                for tick in 0..ticks {
                    let along = -reach + tick as f32 * step;
                    for at in [
                        site.at + Vec2::new(along, offset),
                        site.at + Vec2::new(offset, along),
                    ] {
                        let jump = (moved(at + Vec2::new(step, 0.0)) - moved(at)).abs();
                        looked += 1;
                        if jump > worst {
                            worst = jump;
                            worst_at = at;
                        }
                    }
                }
            }
        }

        assert!(looked > 1000, "only {looked} places were looked at");
        // Levelling is allowed to grade the ground steeply — a road cut into a
        // hillside is a cut. What it may not do is JUMP. Two metres of travel may
        // move the ground a metre and not eight.
        assert!(
            worst < 0.6,
            "levelling steps the ground {worst:.2} m over a quarter-metre at \
             {:.0}, {:.0} — that is a lip, and no brush can take it out",
            worst_at.x,
            worst_at.y
        );
        println!("worst levelling step {worst:.2} m over {looked} places");
    }

    /// Prints every claim on a place, either side of a step.
    ///
    ///     cargo test what_claims -- --ignored --nocapture
    #[test]
    #[ignore = "a measurement"]
    fn what_claims() {
        let terrain = crate::world::terrain::Terrain::new();
        let plan = terrain.plan();
        for spot in [Vec2::new(2288.0, -352.0), Vec2::new(2290.0, -352.0), Vec2::new(2292.0, -352.0)] {
            let dry = terrain.dry_height(spot.x, spot.y);
            let level = plan.level(spot);
            println!(
                "at {:.0}, {:.0}: dry {dry:.2}, level {:?}, drawn {:.2}",
                spot.x, spot.y, level, terrain.base_height(spot.x, spot.y)
            );
            for (which, site) in plan.sites().iter().enumerate() {
                let away = site.at.distance(spot);
                if away < site.radius + SITE_SKIRT + 40.0 {
                    println!(
                        "    site {which}: away {away:.1}, radius {:.1}, height {:.2}, pull {:.3}",
                        site.radius,
                        site.height,
                        crate::util::smoothstep(site.radius + SITE_SKIRT, site.radius, away)
                    );
                }
            }
            for (which, road) in plan.ways().iter().enumerate() {
                let (away, along) = road.nearest(spot);
                if away < ROAD_WIDTH + ROAD_MAX_SKIRT + 40.0 {
                    let skirt = road_skirt(road.cut_at(along));
                    println!(
                        "    road {which}: away {away:.1}, along {along:.3}, height {:.2},                          cut {:.2}, skirt {skirt:.1}, pull {:.3}",
                        road.height_at(along),
                        road.cut_at(along),
                        crate::util::smoothstep(ROAD_WIDTH + skirt, ROAD_WIDTH, away)
                    );
                }
            }
        }
    }

    /// A road running into a town still arrives at the town's own level.
    ///
    /// The guard on the fix. Blending the targets is what removes the step, and
    /// blending them EVENLY would undo what winner-takes-all was for: a road would
    /// approach a town and stop short of its height, leaving a ramp nobody asked
    /// for. Cubing the weights keeps the dominant claim dominant.
    #[test]
    fn a_road_arriving_at_a_town_takes_the_towns_level() {
        let terrain = crate::world::terrain::Terrain::new();
        let mut checked = 0;
        for site in terrain.sites() {
            // Well inside the town, where nothing else has any real claim.
            let inside = terrain.base_height(site.at.x, site.at.y);
            assert!(
                (inside - site.height).abs() < 1.0,
                "the middle of a town sits at {inside:.1} m and it was graded to {:.1}",
                site.height
            );
            checked += 1;
        }
        assert!(checked > 0, "the world has no towns to check");
    }
}

