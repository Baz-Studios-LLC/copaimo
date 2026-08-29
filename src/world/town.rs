//! How a settlement is laid out: streets, the plots between them, and what stands
//! on each one.
//!
//! # Lynch's five, which is what every game city is built on
//!
//! Konstantinos Dimopoulos, who designs cities for games for a living, and the
//! level-design reading on Breath of the Wild both come back to the same framework:
//! Kevin Lynch's five elements of a legible place. A player builds a mental map out
//! of exactly these, and a settlement missing any of them is one they get lost in:
//!
//!   * PATHS - routes that lead somewhere on purpose rather than petering out.
//!   * NODES - focal points where people gather, spaced along the paths.
//!   * LANDMARKS - distinctive things you can fix your position by. Placed BESIDE a
//!     node, so the two reinforce each other: Death Mountain beside Goron City.
//!   * DISTRICTS - areas that are internally consistent and different from their
//!     neighbours, told apart by architectural scale, material and street layout.
//!   * EDGES - boundaries that break continuity and separate one part from another.
//!
//! The other lesson from that reading is that LEGIBILITY BEATS REALISM. Breath of
//! the Wild's districts are more sharply bounded than any real geography, on
//! purpose, because a player who can read the world at a glance is worth more than
//! one who could survey it.
//!
//! And a game city is SMALLER than the thing it stands for. Novigrad is presented
//! as a trade capital of thirty thousand and is built at the size of a real small
//! town - the density of things to look at is what sells it, not the acreage.
//!
//! # Roads, then parcels, then lots
//!
//! Which is the standard pipeline, and every step exists for a reason the step
//! before cannot supply:
//!
//!   1. STREETS are laid first, because everything else is defined relative to
//!      them. A plot is not a piece of ground, it is a piece of ground *with
//!      frontage*.
//!   2. The ground between streets is a PARCEL.
//!   3. A parcel is cut into LOTS along its frontage, stopped by rules: too narrow,
//!      or no longer touching a street.
//!
//! The alternative - scattering buildings on a disc and drawing paths afterwards -
//! is what makes a procedural town read as a campsite. Buildings face the street
//! because they were placed against one, not because they were turned to.
//!
//! # The main street is the road that got here
//!
//! A town exists because a road passes through it, so its high street runs along
//! that road rather than at an angle chosen by a hash. `Settlements::approach`
//! answers which way the network arrives, and everything here is built on that
//! axis. It is the difference between a town on a road and a town beside one.
//!
//! # Nothing is stored
//!
//! A layout is worked out from the site and the seed whenever it is asked for, the
//! same way trees and props are. Two programs asking about the same town get the
//! same town, no file passes between them, and a town nobody visits costs nothing.

use bevy::prelude::*;

use crate::config::WORLD_SEED;
use crate::world::settle::Site;

/// How wide a street is, kerb to kerb.
///
/// Six metres. Wide enough for the eye to read it as a street rather than an alley,
/// narrow enough that the buildings either side are in the same picture - a street
/// you cannot see both sides of at once is a road.
pub const STREET_WIDE: f32 = 6.0;

/// How far a building stands back from the kerb.
pub const SETBACK: f32 = 1.6;

/// The narrowest street frontage worth building on, in metres.
///
/// A cottage is 6 m across and wants a little air either side, so a strip narrower
/// than this holds nothing. Measured as FRONTAGE rather than as area because that
/// is what a lot on a street is sold by, and because depth is the parcel's to give.
///
/// 13 m rather than the 8.2 it started at. At 8.2 the ranch's town came to THREE
/// HUNDRED AND ELEVEN buildings on a 130 m site - which is not a village, it is a
/// terrace wrapped four times round a square. The frontage rule is the density
/// knob: widen the strips and the same streets carry fewer, larger plots with air
/// between them.
// The smallest building plus its air, and nothing arbitrary on top. A cottage is
// 9 m and wants 4 m of room around it, so 13 m of frontage is exactly "something
// fits here" - which is the rule this constant was always trying to express.
// Raising it past that just deletes lots that a house would have stood on happily,
// and at 21 it deleted every lot in a village.
/// How many buildings a place has. Not how many FIT - how many it HAS.
///
/// # A fantasy town is small, and that is a design decision rather than a shortage
///
/// Every attempt to thin these towns went at the geometry - wider frontages, fewer
/// rings, more air - and every one of them was answering the question "how many
/// buildings can stand here" when the question is "how many should". The answer is
/// not a number the ground produces. It is a number the GENRE has:
///
///   * a Pokemon town is five to ten buildings
///   * a Zelda village is about fifteen
///   * Novigrad is presented as a capital of thirty thousand and is built at the
///     size of a real small town
///
/// This is not realism and is not trying to be. A player crossing a town should
/// meet a handful of doors they might open, not a hundred they never will, and
/// three hundred houses on a hillside reads as a housing estate however carefully
/// its streets are laid.
///
/// So the layout still works out every lot the ground offers, and then keeps this
/// many of them, spread evenly so the town fills its streets instead of crowding
/// one end. The rest of the ground stays as yards and gardens - which is also what
/// makes the ones that ARE there read as somewhere people live.
const HOUSES_IN_A_VILLAGE: usize = 11;
const HOUSES_IN_A_CITY: usize = 28;

const A_FRONTAGE_IS_AT_LEAST: f32 = 13.0;

/// How far a town reaches, as a share of the ground levelled for it.
///
/// Past 1.0 on purpose, and it is the user's call: "if you need to encroach on the
/// surrounding area from the established circles that's fine." A settlement is
/// allowed to spill over the rim of its levelled disc rather than be squeezed into
/// it, which is also what real ones do - the flat ground is why the town is there,
/// not a wall around it.
///
/// The outermost buildings then stand on the fade where the level ground turns
/// back into countryside, which is a slope. They are placed on the ground's own
/// height wherever they land, so a house on the fade sits into the hill rather than
/// floating over it.
const FILLS: f32 = 1.15;

/// A part of a town that is internally consistent and unlike its neighbours.
///
/// Lynch's fourth element, and the one my towns had none of: every building was
/// picked by distance from the middle and a dice roll, so a street looked the same
/// wherever you stood in it and there was nothing to tell one part of a town from
/// another. Districts are told apart by what is BUILT in them, which is the
/// cheapest of the three levers the reading names - architectural scale, material,
/// and street layout - and the one that shows from furthest away.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum District {
    /// Around the square: trade. Shops, and the guild hall on the square itself.
    Market,
    /// The working quarter: townhouses over workshops, tight to the street.
    Crafts,
    /// The edge, where the town thins out into cottages and gardens.
    Outskirts,
}

impl District {
    /// The district a place belongs to.
    ///
    /// By RING rather than by a smooth falloff, and that is the legibility lesson
    /// applied: a boundary you can see is worth more than a gradient that is more
    /// truthful. A player should be able to stand somewhere and know which part of
    /// the town they are in.
    /// The district a place belongs to, given the two distances that divide them.
    ///
    /// The dividing distances are the TOWN's, not a formula's. They used to be fixed
    /// multiples of the block depth, which quietly stopped meaning anything the
    /// moment the town got smaller: cut from three rings to two, every lot fell in
    /// the inner two bands and a city had two buildings in its outskirts. A district
    /// is a share of a town, so it is measured against the town that exists.
    fn of(out: f32, inner: f32, outer: f32) -> District {
        if out < inner {
            District::Market
        } else if out < outer {
            District::Crafts
        } else {
            District::Outskirts
        }
    }

    /// Where the three districts divide, from the lots a town actually has.
    ///
    /// The nearer third of them is the market, the middle the crafts quarter, the
    /// outer third the edge - so all three exist and are worth walking between
    /// however big or small the place turned out.
    pub fn divisions(out: &mut Vec<f32>) -> (f32, f32) {
        if out.is_empty() {
            return (f32::MAX, f32::MAX);
        }
        out.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let at = |share: f32| out[((out.len() as f32 - 1.0) * share).round() as usize];
        (at(0.34), at(0.68))
    }

    /// What is built here, given a roll.
    fn builds(self, roll: f32) -> Building {
        match self {
            // Trade, and a few homes over the shops.
            District::Market => {
                if roll < 0.62 {
                    Building::Shop
                } else {
                    Building::Townhouse
                }
            }
            // The workshops: mostly two-storey, some trade, few cottages.
            District::Crafts => {
                if roll < 0.55 {
                    Building::Townhouse
                } else if roll < 0.72 {
                    Building::Shop
                } else {
                    Building::Cottage
                }
            }
            // Homes and gardens.
            District::Outskirts => {
                if roll < 0.78 {
                    Building::Cottage
                } else {
                    Building::Townhouse
                }
            }
        }
    }
}

/// What stands on a plot.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Building {
    Cottage,
    Townhouse,
    Shop,
    GuildHall,
}

impl Building {
    /// The file it is drawn from, under `assets/models/`.
    pub fn model(self) -> &'static str {
        match self {
            Building::Cottage => "models/town_cottage.glb",
            Building::Townhouse => "models/town_townhouse.glb",
            Building::Shop => "models/town_shop.glb",
            Building::GuildHall => "models/town_guild_hall.glb",
        }
    }

    /// How much ground it covers, in metres, before its turn is applied.
    ///
    /// Measured off what `dev/art/town.py` builds, and it is the FOOTPRINT rather
    /// than the whole extent: a roof overhangs by 42 cm on every side and a warden
    /// walks under an overhang rather than into it.
    pub fn footprint(self) -> Vec2 {
        match self {
            // Half again as big as they started, and the reason is the CAMERA.
            // It follows the warden from three or four metres back, so a room he
            // fits in comfortably is one the view clips out of the moment he walks
            // into it. Kept in step with `dev/art/town.py` by
            // `a_building_asks_for_the_room_its_model_needs`.
            Building::Cottage => Vec2::new(9.0, 7.5),
            Building::Townhouse => Vec2::new(9.0, 9.0),
            Building::Shop => Vec2::new(12.0, 9.0),
            Building::GuildHall => Vec2::new(18.0, 13.5),
        }
    }

    /// How much room it needs on a lot, including the ground it is set into.
    fn wants(self) -> Vec2 {
        // Four metres of air around every building, not 1.6.
        //
        // At 1.6 the eaves of one house nearly touched the next and a street read as
        // a terrace with the gaps left in by accident - "see how all the buildings
        // are cramped together". A fantasy town is not a row of semi-detached
        // houses: it wants yards, gardens, and room to walk between things.
        self.footprint() + Vec2::splat(4.0)
    }
}

/// One building, placed.
#[derive(Clone, Copy, Debug)]
pub struct Plot {
    /// Where its middle stands.
    pub at: Vec2,
    /// Which way its front faces, as a yaw in radians.
    ///
    /// Every building in `town.py` has its door in its own -Y wall, so this is the
    /// turn that points that wall at the street.
    pub facing: f32,
    pub what: Building,
}

/// A street, as a line with a width.
#[derive(Clone, Copy, Debug)]
pub struct Street {
    pub from: Vec2,
    pub to: Vec2,
    pub wide: f32,
}

impl Street {
    /// How far a point is from the middle of this street, and how far along it.
    pub fn nearest(&self, at: Vec2) -> (f32, f32) {
        let run = self.to - self.from;
        let length = run.length().max(1.0e-4);
        let along = ((at - self.from).dot(run) / (length * length)).clamp(0.0, 1.0);
        let on = self.from + run * along;
        (at.distance(on), along * length)
    }
}

/// Everything laid out for one settlement.
#[derive(Clone, Debug, Default)]
pub struct Layout {
    pub streets: Vec<Street>,
    pub plots: Vec<Plot>,
}

/// A strip of buildable ground fronting one street, in world coordinates.
///
/// Oriented rather than axis-aligned, which is the change that made a real town
/// plan possible: streets radiate from a square at whatever angles the town wants,
/// and a parcel beside one has to lie along it. The first two attempts kept parcels
/// square to a town frame, which is fine for a crossroads and useless for anything
/// that is not one.
#[derive(Clone, Copy, Debug)]
struct Parcel {
    /// Its middle.
    at: Vec2,
    /// The way its buildings look, which is back across the street it fronts.
    facing: f32,
    /// How much street it fronts.
    frontage: f32,
    /// How far back from the street it reaches.
    depth: f32,
    /// Where the STREET EDGE of the parcel this was cut from lies, along the door
    /// direction. Inherited by every lot, so a lot that no longer touches the
    /// street it was cut from can be told and dropped.
    front: f32,
}

impl Parcel {
    /// Which way this parcel's buildings look.
    fn door(&self) -> Vec2 {
        Vec2::new(self.facing.sin(), -self.facing.cos())
    }

    /// Its own street edge, along the door direction.
    fn edge(&self) -> f32 {
        self.at.dot(self.door()) + self.depth * 0.5
    }

    /// Whether it still fronts the street it was cut from.
    fn has_frontage(&self) -> bool {
        (self.edge() - self.front).abs() < 0.6
    }
}

/// How wide a side lane is. Narrower than the high street, because it is one.
pub const LANE_WIDE: f32 = 4.2;

/// Lays a strip of buildable ground down each side of one street segment.
///
/// Works for a street at any angle, which is the whole point: a radial leaving a
/// market square is at whatever bearing that radial has, and the houses along it
/// have to stand square to IT rather than to a compass.
///
/// `skip_inner` leaves the side nearer the town's middle bare, which is what the
/// square's own boundary wants - there is no building inside a market square.
fn frontage_parcels(
    into: &mut Vec<Parcel>,
    middle: Vec2,
    from: Vec2,
    to: Vec2,
    wide: f32,
    depth: f32,
    skip_inner: bool,
) {
    let run = to - from;
    let length = run.length();
    if length < 6.0 {
        return;
    }
    let axis = run / length;
    let perp = axis.perp();
    let mid = (from + to) * 0.5;

    for side in [-1.0_f32, 1.0] {
        let at = mid + perp * (side * (wide * 0.5 + SETBACK + depth * 0.5));
        if skip_inner && at.distance(middle) < mid.distance(middle) {
            continue;
        }
        // The door points back across the street, which is -perp on this side.
        let door = -perp * side;
        let facing = door.x.atan2(-door.y);
        let parcel = Parcel {
            at,
            facing,
            frontage: length,
            depth,
            front: 0.0,
        };
        let front = parcel.edge();
        into.push(Parcel { front, ..parcel });
    }
}

/// Lays out one settlement.
///
/// `approach` is the direction the road network arrives from, which the high street
/// is built along. `seed` separates one town's dice from another's.
pub fn lay_out(site: &Site, approach: Vec2, seed: u32) -> Layout {
    let reach = site.radius * FILLS;
    if reach < 24.0 {
        return Layout::default();
    }

    // # A SQUARE, RADIALS, AND RINGS
    //
    // Which is what a town actually is, and neither of the two plans before this
    // was. A cross is a road junction. A spine with ribs off it is a suburb of
    // cul-de-sacs - the ribs are dead ends, they enclose nothing, and a town whose
    // streets do not join up has no blocks in it.
    //
    // Real towns organise around a MARKET SQUARE: it is the first thing set out,
    // the roads radiate from it to the edges and the gates, and concentric streets
    // connect those radials to each other. What that produces is a network with
    // CYCLES in it, and a cycle is a block - a ring of buildings with their backs
    // to each other and their faces on four different streets. The guild hall, the
    // shops and the inns take the square, because the ground with the most feet on
    // it is worth the most.
    //
    // Every one of those is a thing this plan now has and the last two did not.
    let square = (reach * 0.19).clamp(11.0, 17.0);
    // A block has to be deep enough for the DEEPEST building plus its air, or a lot
    // passes the frontage rule and then nothing will stand on it. That is what
    // happened when the air around a building went from 1.6 m to 4: a cottage needs
    // 7.5 + 4 = 11.5 m of depth, the floor here was 9, and every village in the
    // world came out with zero buildings in it while the frontage rule took the
    // blame. The floor is the requirement, not a round number.
    let depth = (reach * 0.16).clamp(14.0, 22.0);
    // One ring per band of blocks, out as far as the town reaches.
    let band = depth * 2.0 + LANE_WIDE + SETBACK * 2.0;
    // A city may have three bands of blocks; a village has one or two. A place with
    // forty houses does not need three ring roads, and giving it them is what turned
    // the ranch's town into a small city.
    // Two bands for a city and ONE for a village. Three rings of blocks is a
    // county town; the ranch's neighbour had a hundred buildings in it.
    let most = if site.city { 2 } else { 2 };
    let rings = (((reach - square) / band).floor() as usize).clamp(1, most);

    // The radials. One PAIR of them is the road that got here, carried straight
    // through the square and out the other side - a town on a road has that road
    // as its main street, and everything else is arranged around it.
    let through = approach.y.atan2(approach.x);
    let mut spokes = vec![through, through + std::f32::consts::PI];
    let want = if site.city { 6 } else { 4 };
    for extra in 0..want {
        // Irregularly spaced, because a town is not a wheel. Terrain, ownership and
        // where the last cart went are what set these in a real one, and an even
        // fan is the one thing that reads as drawn rather than grown.
        let turn = through
            + std::f32::consts::TAU * (extra as f32 + 0.5 + 0.42 * unit(seed, 60 + extra as u32))
                / want as f32;
        // Never so close to an existing spoke that the block between them is a
        // wedge too thin to build on.
        if spokes
            .iter()
            .all(|had: &f32| angle_between(*had, turn) > 0.55)
        {
            spokes.push(turn);
        }
    }
    spokes.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let mut streets = Vec::new();
    let mut parcels = Vec::new();

    // # The rings WOBBLE, and the radials do not all reach
    //
    // A perfect ring at a perfect radius crossed by evenly fanned spokes draws a
    // mandala, and the first cut of this plan drew one. The sources are blunt about
    // why that is wrong: "very rarely can we find almost perfectly geometric
    // examples of chartered cities - the initial plan is deformed by terrain, a bend
    // in the river, a steep hill, previous buildings, ownership divisions."
    //
    // So a ring's radius is different at every spoke it passes, which turns each
    // concentric street from a circle into the wandering polygon a real one is, and
    // some spokes stop short of the outermost ring - a street that peters out at the
    // edge of town is the commonest thing there is. The blocks between then come out
    // all different sizes, which is the point: a block that is the same as its
    // neighbour is a block somebody drew.
    let ring_at = |n: usize| square + band * n as f32;
    let ring_r = |spoke: usize, n: usize| -> f32 {
        if n == 0 {
            return square;
        }
        ring_at(n) * (0.86 + 0.27 * unit(seed.wrapping_add(spoke as u32 * 31), 70 + n as u32))
    };
    // How far out each radial actually goes.
    let spoke_reaches = |spoke: usize| -> usize {
        let roll = unit(seed.wrapping_add(spoke as u32 * 53), 80);
        if roll < 0.22 && rings > 1 { rings - 1 } else { rings }
    };
    for spoke in 0..spokes.len() {
        let (a, b) = (spokes[spoke], spokes[(spoke + 1) % spokes.len()]);
        let from = site.at + Vec2::from_angle(a) * square;
        let to = site.at + Vec2::from_angle(b) * square;
        streets.push(Street { from, to, wide: STREET_WIDE });
        frontage_parcels(&mut parcels, site.at, from, to, STREET_WIDE, depth, true);
    }

    // The radials, each running from the square out through every ring it crosses.
    for (index, spoke) in spokes.iter().enumerate() {
        let out = Vec2::from_angle(*spoke);
        let last = spoke_reaches(index);
        let wide = if angle_between(*spoke, through) < 0.1
            || angle_between(*spoke, through + std::f32::consts::PI) < 0.1
        {
            STREET_WIDE
        } else {
            LANE_WIDE
        };
        streets.push(Street {
            from: site.at + out * square,
            to: site.at + out * ring_r(index, last),
            wide,
        });
        // Cut at each ring it crosses, so a radial's frontage is a block's worth at
        // a time rather than one strip running the whole way out.
        for ring in 0..last {
            let (near, away) = (ring_r(index, ring), ring_r(index, ring + 1));
            if away - near < 12.0 {
                continue;
            }
            frontage_parcels(
                &mut parcels,
                site.at,
                site.at + out * (near + SETBACK),
                site.at + out * (away - SETBACK),
                wide,
                depth,
                false,
            );
        }
    }

    // The rings, drawn as chords between consecutive radials. These are what turn a
    // fan of dead ends into a network with blocks in it.
    for ring in 1..=rings {
        for spoke in 0..spokes.len() {
            let next = (spoke + 1) % spokes.len();
            // A ring only runs between two spokes that both reach it.
            if spoke_reaches(spoke) < ring || spoke_reaches(next) < ring {
                continue;
            }
            let from = site.at + Vec2::from_angle(spokes[spoke]) * ring_r(spoke, ring);
            let to = site.at + Vec2::from_angle(spokes[next]) * ring_r(next, ring);
            if from.distance(site.at) > reach || to.distance(site.at) > reach {
                continue;
            }
            streets.push(Street { from, to, wide: LANE_WIDE });
            frontage_parcels(&mut parcels, site.at, from, to, LANE_WIDE, depth, false);
        }
    }

    // THE GUILD HALL TAKES THE SQUARE, which is where a guild hall goes: the search
    // below walks the square's edge for a spot clear of every radial mouth.
    let mut civic: Option<Plot> = None;
    if site.city {
        let hall = Building::GuildHall;
        let bulk = hall.footprint().length() * 0.5;
        let stand = square + STREET_WIDE * 0.5 + SETBACK + hall.footprint().y * 0.5;
        for step in 0..48 {
            let turn = through + std::f32::consts::TAU * step as f32 / 48.0;
            let at = site.at + Vec2::from_angle(turn) * stand;
            if at.distance(site.at) > reach {
                continue;
            }
            if streets
                .iter()
                .any(|street| street.nearest(at).0 < street.wide * 0.5 + bulk * 0.6)
            {
                continue;
            }
            // Facing back at the square.
            let door = (site.at - at).normalize();
            civic = Some(Plot {
                at,
                facing: door.x.atan2(-door.y),
                what: hall,
            });
            break;
        }
    }

    // Each parcel is cut into lots along its frontage, and each lot gets a building.
    let mut lots = Vec::new();
    for (index, parcel) in parcels.iter().enumerate() {
        subdivide(*parcel, seed.wrapping_add(index as u32 * 977), 0, &mut lots);
    }
    lots.sort_by(|a, b| {
        a.at.distance_squared(site.at)
            .partial_cmp(&b.at.distance_squared(site.at))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut plots: Vec<Plot> = Vec::new();
    if let Some(hall) = civic {
        plots.push(hall);
    }
    // Where the districts divide, from the lots this town actually has.
    let (inner, outer) = {
        let mut out: Vec<f32> = lots.iter().map(|l| l.at.distance(site.at)).collect();
        District::divisions(&mut out)
    };

    for (index, lot) in lots.iter().enumerate() {
        if !lot.has_frontage() {
            continue;
        }
        let what = what_stands_here(index, lot, site.at, inner, outer, seed);
        let Some(what) = what else { continue };

        // Placed against the street rather than in the middle of its lot.
        let door = lot.door();
        let front = lot.at + door * (lot.depth * 0.5);
        let at = front - door * (what.footprint().y * 0.5 + 0.35);
        if at.distance(site.at) > reach {
            continue;
        }

        let bulk = what.footprint().length() * 0.5;
        if streets
            .iter()
            .any(|street| street.nearest(at).0 < street.wide * 0.5 + bulk * 0.55)
        {
            continue;
        }
        if plots.iter().any(|placed| {
            let want = (bulk + placed.what.footprint().length() * 0.5) * 0.62;
            at.distance(placed.at) < want
        }) {
            continue;
        }
        plots.push(Plot {
            at,
            facing: lot.facing,
            what,
        });
    }

    // Thinned to what a town of this kind HAS. See HOUSES_IN_A_VILLAGE.
    //
    // Evenly, by stride, rather than by cutting the list short - taking the first N
    // fills one quarter of the town and leaves the rest of the streets empty, which
    // reads as a place half-built rather than a small one.
    let wanted = if site.city {
        HOUSES_IN_A_CITY
    } else {
        HOUSES_IN_A_VILLAGE
    };
    if plots.len() > wanted {
        let mut kept: Vec<Plot> = Vec::with_capacity(wanted);
        // The hall is never thinned out: a city without its guild hall is not a
        // city, and it is the one building the game needs to be able to find.
        let hall = plots.iter().position(|p| p.what == Building::GuildHall);
        if let Some(at) = hall {
            kept.push(plots[at]);
        }
        // Thinned WITHIN each district, in proportion to what that district had.
        //
        // One stride across the whole list is not the same thing: the list runs ring
        // by ring, so a single stride over it kept 1 building in the outskirts of a
        // 28-building city and left the districts - the thing that makes a town
        // legible at all - as a name on two of them. Each district gives up the same
        // share of itself, so all three survive at any size.
        let (inner, outer) = {
            let mut out: Vec<f32> = plots.iter().map(|p| p.at.distance(site.at)).collect();
            District::divisions(&mut out)
        };
        let others: Vec<usize> = (0..plots.len()).filter(|i| Some(*i) != hall).collect();
        let room = wanted.saturating_sub(kept.len()).max(1);
        for district in [District::Market, District::Crafts, District::Outskirts] {
            let here: Vec<usize> = others
                .iter()
                .copied()
                .filter(|i| District::of(plots[*i].at.distance(site.at), inner, outer) == district)
                .collect();
            if here.is_empty() {
                continue;
            }
            let share = ((here.len() as f32 / others.len() as f32) * room as f32).round();
            let take = (share as usize).max(1).min(here.len());
            let stride = (here.len() as f32 / take as f32).max(1.0);
            for step in 0..take {
                let at = (step as f32 * stride).round() as usize;
                if let Some(index) = here.get(at) {
                    kept.push(plots[*index]);
                }
            }
        }
        plots = kept;
    }

    Layout { streets, plots }
}

/// The smaller angle between two bearings.
fn angle_between(one: f32, two: f32) -> f32 {
    let mut gap = (one - two).abs() % std::f32::consts::TAU;
    if gap > std::f32::consts::PI {
        gap = std::f32::consts::TAU - gap;
    }
    gap
}

/// Cuts a parcel into lots by slicing it along its shorter axis, recursively.
///
/// The rules that stop it are the ones the research names: too small, too thin. A
/// lot that fails either is kept whole rather than cut again, and a lot that is
/// still too big to be one building is cut once more.
fn subdivide(parcel: Parcel, seed: u32, depth: u32, into: &mut Vec<Parcel>) {
    if depth > 6 || parcel.frontage < A_FRONTAGE_IS_AT_LEAST * 2.0 {
        if parcel.frontage >= A_FRONTAGE_IS_AT_LEAST {
            into.push(parcel);
        }
        return;
    }

    // Cut ALONG the street only, never across the parcel's depth: cutting across
    // makes a back lot with no frontage, which is then thrown away, and half of
    // every parcel becomes nothing.
    let split = 0.42 + 0.16 * unit(seed, 3);
    let near = parcel.frontage * split;
    let far = parcel.frontage - near;
    let sideways = parcel.door().perp();

    for (share, sign) in [(near, -1.0_f32), (far, 1.0)] {
        subdivide(
            Parcel {
                at: parcel.at + sideways * (sign * (parcel.frontage - share) * 0.5),
                frontage: share,
                ..parcel
            },
            seed.wrapping_mul(7919).wrapping_add(if sign < 0.0 { 1 } else { 2 }),
            depth + 1,
            into,
        );
    }
}

/// What belongs on this lot.
///
/// # Trade at the middle, homes at the edge
///
/// Which is how a town actually sorts itself: the ground with the most passing
/// traffic is worth the most, so that is where the shops are, and the guild hall
/// takes the best lot of all. Cottages go where the town thins out. Doing this by
/// distance from the centre rather than by a dice roll is most of what makes a
/// generated town read as a place rather than as a scatter.
fn what_stands_here(
    index: usize,
    lot: &Parcel,
    middle: Vec2,
    inner: f32,
    outer: f32,
    seed: u32,
) -> Option<Building> {
    let roll = unit(seed.wrapping_add(index as u32 * 131), 11);
    let fits = |what: Building| {
        let wants = what.wants();
        lot.frontage >= wants.x && lot.depth >= wants.y
    };

    // TRADE ON THE SQUARE, WORKSHOPS BEHIND IT, HOMES AT THE EDGE. The medieval
    // rule, the obvious one, and Lynch's districts all at once: the ground with the
    // most feet on it carries the trade, and what a place is FOR is what tells one
    // part of a town from another.
    let wanted = District::of(lot.at.distance(middle), inner, outer).builds(roll);
    if fits(wanted) {
        Some(wanted)
    } else if fits(Building::Cottage) {
        Some(Building::Cottage)
    } else {
        None
    }
}

/// What the cities are called.
///
/// # Written down rather than generated
///
/// A syllable machine gives you Grondar and Velmoth forever, and every one of them
/// is a name nobody chose. These are chosen: they sound like places on the same
/// map as one another, they are easy to say out loud, and each is short enough to
/// sit under a marker without being abbreviated.
///
/// They are also assigned by POSITION rather than at random - see `name_of` - so
/// the city in the cold north gets a cold northern name and the one on the dry
/// plateau gets a dry one. A world where the ice city is called Sunmere and the
/// desert city Frosthold is a world nobody will believe.
const NORTHERN: [&str; 6] = ["Hollowfrost", "Kettleridge", "Varn", "Colderry", "Stonewake", "Ashfen"];
const MIDDLE: [&str; 8] = [
    "Marrowmede", "Greenhollow", "Oakenford", "Bellwether",
    "Thornbury", "Willowmarch", "Emberlyn", "Rookhaven",
];
const DRY: [&str; 5] = ["Sunmere", "Dustholt", "Amberrock", "Scaldpan", "Wayfarer's Rest"];
const COASTAL: [&str; 5] = ["Saltmarrow", "Harbourly", "Tidewatch", "Gullsbay", "Coldwater"];

/// The name of the settlement at this site.
///
/// Every city gets one. Towns and villages do not, on purpose: a world where every
/// hamlet of four cottages has a name is a world where no name means anything, and
/// the pitch's Warden Exams happen in CITIES.
pub fn name_of(site: &Site, country: terrain_core::region::Country, index: usize) -> Option<&'static str> {
    if !site.city {
        return None;
    }
    let list: &[&'static str] = match country {
        terrain_core::region::Country::Snow => &NORTHERN,
        terrain_core::region::Country::Desert => &DRY,
        _ if site.at.length() > 4_000.0 => &COASTAL,
        _ => &MIDDLE,
    };
    Some(list[index % list.len()])
}

fn unit(seed: u32, salt: u32) -> f32 {
    let mut x = seed
        .wrapping_mul(0x9E37_79B9)
        .wrapping_add(salt.wrapping_mul(0x85EB_CA6B));
    x ^= x >> 15;
    x = x.wrapping_mul(0x2545_F491);
    x ^= x >> 13;
    (x % 100_000) as f32 / 100_000.0
}

// ============================================================ raising them

use bevy::scene::SceneRoot;

use crate::world::StreamAnchor;
use crate::world::terrain::TerrainSource;

/// How far from the player a settlement is built.
///
/// A town is a hundred or so scenes and it wants to be standing before it comes
/// into view rather than popping up as you reach it, so this is comfortably past
/// where one is legible. It is measured to the site's MIDDLE, so a big city starts
/// building while you are still well outside it.
const RAISES_WITHIN: f32 = 900.0;

/// A building standing in the world.
#[derive(Component)]
pub struct Standing {
    pub what: Building,
}

/// The settlements that are standing, and the layout each was built from.
///
/// # Worked out once, not once a frame
///
/// `lay_out` walks a site's parcels and subdivides every one of them, and the town
/// at the ranch comes to three hundred buildings. The first cut called it from the
/// COLLISION path - which runs every frame, for every step the warden tries - so
/// the whole town was being planned sixty times a second to answer "is there a wall
/// in front of me".
///
/// It is planned when the town is built and kept until the town comes down.
#[derive(Resource, Default)]
pub struct Built {
    standing: std::collections::HashMap<u32, Layout>,
}

impl Built {
    /// How many buildings are standing, for the HUD.
    pub fn buildings(&self) -> usize {
        self.standing.values().map(|layout| layout.plots.len()).sum()
    }

    pub fn towns(&self) -> usize {
        self.standing.len()
    }

    /// Everything standing near a point that cannot be walked through.
    pub fn walls_near(&self, at: Vec2, reach: f32) -> Vec<(Vec2, Vec2, f32)> {
        let mut walls = Vec::new();
        for layout in self.standing.values() {
            for plot in &layout.plots {
                if plot.at.distance(at) > reach + plot.what.footprint().length() {
                    continue;
                }
                walls.extend(plot.walls());
            }
        }
        walls
    }
}

/// One town's worth of buildings, kept so the whole lot can be taken down together.
#[derive(Component)]
struct FromSite(u32);

/// How far above the ground a street's surface is laid, in metres.
///
/// Four centimetres. Flat on the terrain z-fights with it - two surfaces at the
/// same height flicker against each other wherever they meet - and any higher is a
/// kerb you can see the edge of from across the square.
// 9 cm, up from 4. Four cleared the ground in arithmetic and not on screen: the
// chunk mesh is a grid of flat triangles and the depth buffer has opinions, so a
// surface laid four centimetres over it flickers along every triangle edge.
const ROAD_LIES: f32 = 0.09;

/// How long a piece of road is before it takes another height sample.
///
/// The lanes flatten what they run over, so a street is nearly level along its
/// length - but only nearly, and a street laid as one long quad bridges whatever
/// is left and floats at one end.
const ROAD_STEPS_EVERY: f32 = 2.5;

/// The colour of packed earth, worn darker than the ground it is worn into.
///
/// # Why the surface is drawn and not painted
///
/// The obvious answer was to let `Biome::Settled` do it: a lane levels the ground,
/// levelled ground is settled ground, and settled ground is already bare earth. It
/// works and it is invisible, because a TOWN is levelled ground too - so the street
/// and the garden either side of it come out exactly the same colour, and the test
/// that asked whether anything beside a street looked different said no.
///
/// A road has to be a different SURFACE from the ground it crosses, so it is one.
// STONE. Asked for by name, and right for the place: a guild town's high street is
// laid, not worn. Cool grey against the warm pale earth a settlement stands on, so
// the street reads as a different material rather than as a darker patch of the
// same one.
const ROAD_STONE: [f32; 4] = [0.34, 0.34, 0.36, 1.0];

/// How much one paving stone differs from its neighbour.
///
/// A road of one flat colour is a painted stripe. Varying each quad a little is what
/// turns it into stones - it costs nothing, because the paving is already built as
/// quads and every quad already carries a colour.
const STONE_VARIES: f32 = 0.16;
const ROAD_KERB: [f32; 4] = [0.52, 0.50, 0.46, 1.0];

/// The material a street's paving wears.
///
/// # Why it cannot borrow the ground cover's
///
/// It did, and the roads came out writhing. `CoverMaterial` is the GRASS material,
/// and the grass material's whole job is to be displaced: its shader bends every
/// vertex away from whatever is standing in it, which is what makes a meadow part
/// as a warden walks through it. Laying a road in it made the road part too - the
/// paving pulled open around the player and shut behind him, which was reported,
/// exactly and correctly, as "the roads are odd and show the grass where I walk".
///
/// Nothing was growing there at all: measured, a town's ground is `Settled` out to
/// its rim with a cover density of 0.00. The grass was the road.
///
/// So the paving has its own material, with the bending switched off. It is still a
/// `Shaded`, because the cloud shadows have to fall on a street like they fall on
/// everything else.
#[derive(Resource)]
pub struct RoadSurface(pub Handle<crate::shade::Shaded>);

fn mix_the_road_surface(mut commands: Commands, mut materials: ResMut<Assets<crate::shade::Shaded>>) {
    let surface = materials.add(crate::shade::shaded(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.96,
        reflectance: 0.02,
        ..default()
    }));
    commands.insert_resource(RoadSurface(surface));
}

/// Builds one town's streets as a mesh laid on the ground.
fn pave(streets: &[Street], terrain: &crate::world::terrain::Terrain, low: Vec2) -> Mesh {
    let mut places: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut colours: Vec<[f32; 4]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    for street in streets {
        let run = street.to - street.from;
        let length = run.length();
        if length < 1.0 {
            continue;
        }
        let along = run / length;
        let side = along.perp();
        let steps = (length / ROAD_STEPS_EVERY).ceil().max(1.0) as usize;

        for step in 0..=steps {
            let on = street.from + run * (step as f32 / steps as f32);
            // Three across: kerb, middle, kerb, so the edge can be a shade paler
            // and the road has an edge at all.
            for (across, colour) in [
                (-street.wide * 0.5, ROAD_KERB),
                (0.0, ROAD_STONE),
                (street.wide * 0.5, ROAD_KERB),
            ] {
                let at = on + side * across;
                let height = terrain.drawn_height(at.x, at.y) + ROAD_LIES;
                places.push([at.x - low.x, height, at.y - low.y]);
                normals.push([0.0, 1.0, 0.0]);
                colours.push(colour);
                uvs.push([step as f32, across]);
            }
        }

        let base = (places.len() - (steps + 1) * 3) as u32;
        for step in 0..steps as u32 {
            for lane in 0..2u32 {
                let a = base + step * 3 + lane;
                let b = a + 1;
                let c = a + 3;
                let d = a + 4;
                indices.extend_from_slice(&[a, c, b, b, c, d]);
            }
        }
    }

    let mut mesh = Mesh::new(
        bevy::render::mesh::PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, places);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colours);
    mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));
    mesh
}

/// Builds the settlements near the player, and takes down the ones left behind.
///
/// # Why it is keyed on the site and not on chunks
///
/// A building is not chunk-sized - a guild hall is twelve metres across and a town
/// is two hundred - so streaming them per chunk would spawn and despawn the same
/// hall repeatedly as the player walked its boundary. A settlement is built once,
/// whole, and stands until the player is a long way from it.
pub fn raise_the_towns(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<crate::shade::Shaded>>,
    mut road_surface: Local<Option<Handle<crate::shade::Shaded>>>,
    terrain: Res<TerrainSource>,
    mut built: ResMut<Built>,
    anchors: Query<&GlobalTransform, With<StreamAnchor>>,
    standing: Query<(Entity, &FromSite)>,
) {
    let Some(anchor) = anchors.iter().next() else {
        return;
    };
    let here = Vec2::new(anchor.translation().x, anchor.translation().z);

    let plan = terrain.plan();
    for (index, site) in plan.sites().iter().enumerate() {
        let key = index as u32;
        let near = site.at.distance(here) < RAISES_WITHIN;
        if near == built.standing.contains_key(&key) {
            continue;
        }
        if !near {
            // Left behind: take the whole town down at once.
            for (entity, from) in &standing {
                if from.0 == key {
                    commands.entity(entity).despawn();
                }
            }
            built.standing.remove(&key);
            continue;
        }

        let layout = lay_out(site, plan.approach(site.at), WORLD_SEED.wrapping_add(key * 7717));
        info!(
            "raising {} at ({:.0}, {:.0}): {} buildings",
            if site.city { "a city" } else { "a town" },
            site.at.x,
            site.at.y,
            layout.plots.len()
        );
        for plot in &layout.plots {
            // On the GROUND's own height wherever it lands, not on the site's
            // levelled height: a town is allowed to spill past the rim of the
            // ground that was flattened for it, and a house out on the fade has to
            // sit into the slope rather than float over it.
            let stands = terrain.drawn_height(plot.at.x, plot.at.y);
            commands.spawn((
                Standing { what: plot.what },
                FromSite(key),
                SceneRoot(assets.load(GltfAssetLabel::Scene(0).from_asset(plot.what.model()))),
                Transform::from_xyz(plot.at.x, stands, plot.at.y)
                    .with_rotation(Quat::from_rotation_y(-plot.facing)),
                Visibility::default(),
            ));
        }
        // The streets themselves, as one mesh for the town.
        //
        // The material is made HERE, on demand, rather than looked up from a
        // resource a startup system was supposed to have filled in. That lookup was
        // an `if let Some(..)`, which means the one failure it can have is silent:
        // the buildings go up and the streets simply do not, which is precisely the
        // shape of "still no roads" reported three times against a paving mesh that
        // measured correctly every time it was asked. A road that cannot be skipped
        // cannot be skipped for a reason nobody can see.
        let surface = road_surface.get_or_insert_with(|| {
            materials.add(crate::shade::shaded(StandardMaterial {
                base_color: Color::WHITE,
                perceptual_roughness: 0.96,
                reflectance: 0.02,
                ..default()
            }))
        });
        let paving = pave(&layout.streets, &terrain.0, site.at);
        commands.spawn((
            FromSite(key),
            Mesh3d(meshes.add(paving)),
            MeshMaterial3d(surface.clone()),
            Transform::from_xyz(site.at.x, 0.0, site.at.y),
            Visibility::default(),
            bevy::pbr::NotShadowCaster,
        ));
        built.standing.insert(key, layout);
    }
}

impl Plot {
    /// This building's walls, as (middle, half-extents, turn) in world space.
    ///
    /// The front wall comes in two pieces with the doorway between them, which is
    /// what makes the building enterable. Everything else is one slab a side.
    pub fn walls(&self) -> Vec<(Vec2, Vec2, f32)> {
        let half = self.what.footprint() * 0.5;
        let (sin, cos) = self.facing.sin_cos();
        let out = |local: Vec2| {
            self.at + Vec2::new(local.x * cos - local.y * sin, local.x * sin + local.y * cos)
        };
        let thick = 0.3;
        let mut walls = Vec::new();

        // Back and both flanks: one slab each.
        walls.push((out(Vec2::new(0.0, half.y)), Vec2::new(half.x, thick), self.facing));
        for side in [-1.0_f32, 1.0] {
            walls.push((
                out(Vec2::new(side * half.x, 0.0)),
                Vec2::new(thick, half.y),
                self.facing,
            ));
        }

        // The front, in two pieces with the doorway between them.
        let door = DOOR_CLEAR * 0.5;
        let pier = (half.x - door).max(0.0);
        if pier > 0.05 {
            for side in [-1.0_f32, 1.0] {
                walls.push((
                    out(Vec2::new(side * (half.x - pier * 0.5), -half.y)),
                    Vec2::new(pier * 0.5, thick),
                    self.facing,
                ));
            }
        }
        walls
    }
}

/// How wide the gap in the front wall is, in metres.
///
/// The doorway `dev/art/town.py` builds is 1.4 m, and this is wider on purpose: a
/// collision gap exactly as wide as the opening leaves a warden 0.66 m across
/// aiming at a 1.4 m target with no tolerance, which reads as a door that
/// sometimes refuses you. The extra is invisible - the geometry either side of it
/// is wall - and it is the difference between walking in and fighting the frame.
const DOOR_CLEAR: f32 = 2.2;

pub struct TownPlugin;

impl Plugin for TownPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Built>()
            .add_systems(Update, raise_the_towns.run_if(crate::build::a_world_is_up));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a_site(city: bool, radius: f32) -> Site {
        Site {
            at: Vec2::new(120.0, -80.0),
            height: 30.0,
            radius,
            city,
            ranch: false,
        }
    }

    #[test]
    fn a_city_has_exactly_one_guild_hall_and_a_village_has_none() {
        let city = lay_out(&a_site(true, 90.0), Vec2::X, 7);
        let halls = city
            .plots
            .iter()
            .filter(|p| p.what == Building::GuildHall)
            .count();
        assert_eq!(halls, 1, "a city laid out {halls} guild halls");

        let village = lay_out(&a_site(false, 55.0), Vec2::X, 7);
        assert_eq!(
            village
                .plots
                .iter()
                .filter(|p| p.what == Building::GuildHall)
                .count(),
            0,
            "a village got a guild hall"
        );
        assert!(!village.plots.is_empty(), "a village got no buildings at all");
    }

    #[test]
    fn nothing_stands_in_the_street() {
        // The whole point of laying streets first. Every building has to be clear of
        // every carriageway, or the town is a pile of houses with roads drawn
        // through them.
        for seed in 0..40 {
            let site = a_site(seed % 2 == 0, 70.0 + (seed % 5) as f32 * 12.0);
            let layout = lay_out(&site, Vec2::new(1.0, 0.4).normalize(), seed);
            for plot in &layout.plots {
                let half = plot.what.footprint().max_element() * 0.5;
                for street in &layout.streets {
                    let (off, _) = street.nearest(plot.at);
                    assert!(
                        off >= street.wide * 0.5 + half * 0.55,
                        "seed {seed}: a {:?} stands {off:.1} m from the middle of a \
                         {:.1} m street",
                        plot.what,
                        street.wide
                    );
                }
            }
        }
    }

    #[test]
    fn no_two_buildings_stand_in_each_other() {
        for seed in 0..40 {
            let site = a_site(seed % 3 == 0, 60.0 + (seed % 7) as f32 * 10.0);
            let layout = lay_out(&site, Vec2::Y, seed);
            for (index, one) in layout.plots.iter().enumerate() {
                for other in &layout.plots[index + 1..] {
                    let want = (one.what.footprint().max_element()
                        + other.what.footprint().max_element())
                        * 0.5;
                    let gap = one.at.distance(other.at);
                    assert!(
                        gap >= want * 0.75,
                        "seed {seed}: a {:?} and a {:?} are {gap:.1} m apart and want \
                         {want:.1}",
                        one.what,
                        other.what
                    );
                }
            }
        }
    }

    #[test]
    fn a_town_keeps_inside_the_ground_that_was_levelled_for_it() {
        for seed in 0..30 {
            let site = a_site(seed % 2 == 0, 80.0);
            let layout = lay_out(&site, Vec2::X, seed);
            for plot in &layout.plots {
                let out = plot.at.distance(site.at);
                assert!(
                    // Allowed past the levelled rim - see FILLS, and the user's
                    // call that encroaching on the surrounding ground is fine.
                    out <= site.radius * 1.35,
                    "seed {seed}: a building stands {out:.0} m out on a site levelled \
                     to {:.0} m",
                    site.radius
                );
            }
        }
    }

/// Draws a city and a village to `dev/art/map/town_plan.png`.
    ///
    /// Ignored: it writes a picture, and a layout is a shape - the tests above say
    /// nothing stands in a street and nothing overlaps, and neither of them can say
    /// whether it looks like a town.
    #[test]
    #[ignore = "writes a picture to be looked at"]
    fn draw_a_town() {
        const SCALE: f32 = 3.2;
        const PAD: u32 = 20;

        let mut sheet = image::RgbImage::new(1240, 640);
        for pixel in sheet.pixels_mut() {
            *pixel = image::Rgb([26, 30, 26]);
        }

        for (panel, (city, radius, seed)) in
            [(true, 95.0_f32, 4_u32), (false, 58.0, 11)].iter().enumerate()
        {
            let site = a_site(*city, *radius);
            let approach = Vec2::new(0.82, 0.57).normalize();
            let layout = lay_out(&site, approach, *seed);
            let origin = (PAD + panel as u32 * 620 + 310, 320_u32);
            let to_px = |at: Vec2| {
                let off = (at - site.at) * SCALE;
                (
                    (origin.0 as f32 + off.x) as i32,
                    (origin.1 as f32 + off.y) as i32,
                )
            };
            let mut dot = |x: i32, y: i32, rgb: [u8; 3]| {
                if x >= 0 && y >= 0 && (x as u32) < sheet.width() && (y as u32) < sheet.height() {
                    sheet.put_pixel(x as u32, y as u32, image::Rgb(rgb));
                }
            };

            // The levelled ground.
            for step in 0..1440 {
                let turn = step as f32 / 1440.0 * std::f32::consts::TAU;
                let (px, py) = to_px(site.at + Vec2::new(turn.cos(), turn.sin()) * site.radius);
                dot(px, py, [52, 62, 50]);
            }
            // The streets, as their real width.
            for street in &layout.streets {
                let run = street.to - street.from;
                let steps = (run.length() * SCALE) as i32;
                let side = Vec2::new(-run.y, run.x).normalize();
                for step in 0..=steps {
                    let on = street.from + run * (step as f32 / steps.max(1) as f32);
                    let across = (street.wide * 0.5 * SCALE) as i32;
                    for off in -across..=across {
                        let (px, py) = to_px(on + side * (off as f32 / SCALE));
                        dot(px, py, [78, 74, 66]);
                    }
                }
            }
            // The buildings, as their footprint, turned.
            for plot in &layout.plots {
                let rgb = match plot.what {
                    Building::GuildHall => [232, 196, 92],
                    Building::Shop => [126, 178, 208],
                    Building::Townhouse => [206, 150, 116],
                    Building::Cottage => [150, 196, 140],
                };
                let half = plot.what.footprint() * 0.5;
                let (sin, cos) = plot.facing.sin_cos();
                let wide = (half.x * SCALE) as i32;
                let deep = (half.y * SCALE) as i32;
                for u in -wide..=wide {
                    for v in -deep..=deep {
                        let local = Vec2::new(u as f32 / SCALE, v as f32 / SCALE);
                        let world = plot.at
                            + Vec2::new(local.x * cos - local.y * sin, local.x * sin + local.y * cos);
                        let (px, py) = to_px(world);
                        // The door wall drawn dark, so which way it faces is visible.
                        let on_front = v == -deep;
                        dot(px, py, if on_front { [40, 30, 26] } else { rgb });
                    }
                }
            }
            println!(
                "{} : {} buildings, {} streets",
                if *city { "city   " } else { "village" },
                layout.plots.len(),
                layout.streets.len()
            );
        }
        let dir = std::path::Path::new("dev/art/map");
        std::fs::create_dir_all(dir).ok();
        sheet.save(dir.join("town_plan.png")).expect("the plan should save");
        println!("drew dev/art/map/town_plan.png");
    }

/// What the REAL world's settlements lay out to.
    #[test]
    #[ignore = "a measurement of the real world"]
    fn what_the_towns_measure() {
        let terrain = crate::world::terrain::Terrain::new();
        let plan = terrain.plan();
        let ranch = Vec2::new(crate::config::RANCH_AT.0, crate::config::RANCH_AT.1);
        println!("{} sites", plan.sites().len());
        let mut total = 0;
        for (index, site) in plan.sites().iter().enumerate() {
            let layout = lay_out(
                site,
                plan.approach(site.at),
                crate::config::WORLD_SEED.wrapping_add(index as u32 * 7717),
            );
            total += layout.plots.len();
            if index < 12 {
                println!(
                    "  {:<7} r={:5.1} at ({:7.0},{:7.0})  {:5.0} m from the ranch  -> {:3} buildings, {:2} streets",
                    if site.city { "city" } else { "town" },
                    site.radius,
                    site.at.x,
                    site.at.y,
                    site.at.distance(ranch),
                    layout.plots.len(),
                    layout.streets.len()
                );
            }
        }
        println!("{total} buildings in the world");
        let nearest = plan
            .sites()
            .iter()
            .map(|s| s.at.distance(ranch))
            .fold(f32::MAX, f32::min);
        println!("nearest settlement to the ranch: {nearest:.0} m (raised within {RAISES_WITHIN})");
    }

/// The system actually raises a town when a warden stands in one.
    ///
    /// # Why this is an app test and not a unit test
    ///
    /// Everything else here tests `lay_out`, which is a pure function and was
    /// correct while nothing appeared in the game at all. What is between the two is
    /// a Bevy system with a run condition, a resource it needs, an anchor it looks
    /// for and a schedule it sits in - four things that can each be wrong on their
    /// own, and none of which a test of the layout can see.
/// The streets are not just computed - they are SPAWNED, with a mesh on them.
    ///
    /// "Still no roads", three times, against a paving mesh that measured 1,929
    /// vertices every time it was asked. A mesh that exists in a function and never
    /// reaches the world is exactly as useful as no mesh, and nothing here was
    /// asking the world - only the arithmetic.
    #[test]
    fn a_settlement_lays_its_streets_in_the_world() {
        let (mut app, _site) = a_world_with_a_town();
        app.update();

        let mut meshes = app.world_mut().query::<(&Mesh3d, &Transform)>();
        let found: Vec<_> = meshes.iter(app.world()).collect();
        assert!(!found.is_empty(), "nothing at all was spawned");

        let store = app.world().resource::<Assets<Mesh>>();
        let paving: Vec<usize> = found
            .iter()
            .filter_map(|(mesh, _)| store.get(&mesh.0))
            .map(|mesh| mesh.count_vertices())
            .filter(|count| *count > 500)
            .collect();
        assert!(
            !paving.is_empty(),
            "{} meshes were spawned and none of them is a street - the paving is              computed and thrown away",
            found.len()
        );
        println!("spawned {} meshes; the largest is {} vertices", found.len(),
            paving.iter().copied().max().unwrap_or(0));
    }

    /// An app with the town plugin, standing in a real SETTLEMENT.
    ///
    /// Not at `sites()[0]` - that is the ranch, which is deliberately not a
    /// settlement, so a test standing there was measuring whatever happened to be
    /// in reach rather than the thing it named.
    fn a_world_with_a_town() -> (App, Site) {
        use bevy::asset::AssetPlugin;
        use bevy::state::app::StatesPlugin;

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default(), StatesPlugin));
        app.init_state::<crate::states::AppState>();
        app.insert_state(crate::states::AppState::Playing);
        app.init_asset::<Scene>();
        app.init_asset::<Mesh>();
        app.init_asset::<crate::shade::Shaded>();
        app.init_asset::<bevy::gltf::Gltf>();
        app.add_plugins(TownPlugin);

        let terrain = crate::world::terrain::Terrain::new();
        let site = *terrain
            .plan()
            .sites()
            .iter()
            .find(|s| !s.ranch)
            .expect("the world has no settlements at all");
        app.insert_resource(crate::world::terrain::TerrainSource(std::sync::Arc::new(
            terrain,
        )));
        app.world_mut().spawn((
            StreamAnchor,
            Transform::from_xyz(site.at.x, 0.0, site.at.y),
            GlobalTransform::from_xyz(site.at.x, 0.0, site.at.y),
        ));
        (app, site)
    }

    #[test]
    fn standing_in_a_settlement_raises_it() {
        let (mut app, site) = a_world_with_a_town();
        app.update();

        let standing = app
            .world_mut()
            .query::<&Standing>()
            .iter(app.world())
            .count();
        // More than a handful, not more than twenty. A village HAS eleven buildings
        // now - see HOUSES_IN_A_VILLAGE - so a threshold of twenty was asking
        // whether the town was big rather than whether it was there.
        assert!(
            standing > 5,
            "standing in a settlement raised {standing} buildings"
        );
        let built = app.world().resource::<Built>();
        // At least the one being stood in. Two is fine and correct: settlements are
        // raised within RAISES_WITHIN of the anchor, and the first non-ranch site in
        // the world happens to have a neighbour inside that. Insisting on exactly
        // one was asserting a property of the map, not of the code.
        assert!(built.towns() >= 1, "{} towns were built", built.towns());
        assert_eq!(built.buildings(), standing);

        // And its walls are there to be walked into.
        let walls = built.walls_near(site.at, 60.0);
        assert!(!walls.is_empty(), "the town it raised has no walls in it");
    }

    /// A town's streets are paved, and the paving lies on the ground.
    ///
    /// # What this catches, and what the first version asked instead
    ///
    /// The fault that shipped: streets were laid out, buildings were placed against
    /// them, and NOTHING appeared on the ground. A plan that exists only in the
    /// layout is not a road.
    ///
    /// The first attempt at this test asked whether a street reads as `Settled`
    /// while the ground beside it does not - and it failed, correctly, for a reason
    /// worth keeping: a TOWN is levelled ground, so the whole of it is already
    /// settled, and a street painted the same way is invisible against its own
    /// verge. A road has to be a different SURFACE from the ground it crosses.
    #[test]
    fn the_streets_are_paved_and_the_paving_lies_on_the_ground() {
        let terrain = crate::world::terrain::Terrain::new();
        let plan = terrain.plan();
        let site = plan.sites()[0];
        let layout = lay_out(&site, plan.approach(site.at), crate::config::WORLD_SEED);
        assert!(!layout.streets.is_empty(), "the town has no streets");

        let paving = pave(&layout.streets, &terrain, site.at);
        let count = paving.count_vertices();
        assert!(count > 200, "the paving is {count} vertices, which is nothing");

        let Some(bevy::render::mesh::VertexAttributeValues::Float32x3(places)) =
            paving.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("the paving has no positions");
        };
        let mut worst: f32 = 0.0;
        for place in places {
            let at = Vec2::new(place[0] + site.at.x, place[2] + site.at.y);
            let ground = terrain.drawn_height(at.x, at.y);
            worst = worst.max((place[1] - ground - ROAD_LIES).abs());
        }
        assert!(
            worst < 0.01,
            "the paving stands {worst:.2} m off the ground it is laid on"
        );
    }

#[test]
    #[ignore = "a measurement of the real ground"]
    fn what_the_town_ground_is() {
        use crate::world::terrain::Biome;
        let terrain = crate::world::terrain::Terrain::new();
        let plan = terrain.plan();
        let climate = terrain.climate();
        let site = plan.sites()[0];
        let layout = lay_out(&site, plan.approach(site.at), crate::config::WORLD_SEED);
        println!("site at ({:.0},{:.0}) r={:.0}, {} buildings, {} streets",
            site.at.x, site.at.y, site.radius, layout.plots.len(), layout.streets.len());

        let street = layout.streets[0];
        let mid = (street.from + street.to) * 0.5;
        for (label, at) in [
            ("on a street ", mid),
            ("5 m off it  ", mid + (street.to - street.from).perp().normalize() * 5.0),
            ("20 m off it ", mid + (street.to - street.from).perp().normalize() * 20.0),
            ("site middle ", site.at),
            ("120 m out   ", site.at + Vec2::new(120.0, 0.0)),
        ] {
            let ground = terrain.ground_at(at.x, at.y);
            let biome = Biome::of(ground, &climate);
            let thick = terrain_core::cover::density(
                biome,
                Biome::confidence(ground, &climate),
                terrain_core::cover::patch(biome, at),
            );
            println!(
                "  {label}: {biome:?}  levelled {:.2}  height {:.2}  cover {thick:.2}",
                ground.levelled, ground.height
            );
        }
    }

/// A town has districts you could tell apart standing in them.
    ///
    /// Lynch's fourth element, and the test is the point of it: a district that is
    /// not DIFFERENT from its neighbours is not a district, it is a name. So this
    /// asks what is actually built in each ring and refuses a town where the answer
    /// is the same everywhere - which is exactly what the first three layouts were,
    /// buildings picked by a dice roll and a distance.
    #[test]
    fn a_town_has_districts_and_they_do_not_look_alike() {
        let site = a_site(true, 190.0);
        let layout = lay_out(&site, Vec2::X, 9);

        let (inner, outer) = {
            let mut out: Vec<f32> = layout.plots.iter().map(|p| p.at.distance(site.at)).collect();
            District::divisions(&mut out)
        };

        let mut counts = std::collections::HashMap::new();
        for plot in &layout.plots {
            let district = District::of(plot.at.distance(site.at), inner, outer);
            *counts
                .entry((district, plot.what))
                .or_insert(0usize) += 1;
        }

        let share = |district: District, what: Building| {
            let here: usize = counts
                .iter()
                .filter(|((d, _), _)| *d == district)
                .map(|(_, n)| *n)
                .sum();
            let this = counts.get(&(district, what)).copied().unwrap_or(0);
            if here == 0 { 0.0 } else { this as f32 / here as f32 }
        };

        // Every district has to exist at all.
        for district in [District::Market, District::Crafts, District::Outskirts] {
            let here: usize = counts
                .iter()
                .filter(|((d, _), _)| *d == district)
                .map(|(_, n)| *n)
                .sum();
            assert!(here > 3, "{district:?} has {here} buildings in it");
        }

        // And they have to be DIFFERENT. Trade at the middle, homes at the edge -
        // if a shop is as likely on the outskirts as on the square then the town
        // has one district wearing three names.
        let shops_in = share(District::Market, Building::Shop);
        let shops_out = share(District::Outskirts, Building::Shop);
        let homes_in = share(District::Market, Building::Cottage);
        let homes_out = share(District::Outskirts, Building::Cottage);
        println!(
            "market: {:.0}% shops, {:.0}% cottages | outskirts: {:.0}% shops, {:.0}% cottages",
            shops_in * 100.0, homes_in * 100.0, shops_out * 100.0, homes_out * 100.0
        );
        assert!(
            shops_in > shops_out + 0.3,
            "shops are {shops_in:.2} of the market and {shops_out:.2} of the outskirts"
        );
        assert!(
            homes_out > homes_in + 0.3,
            "cottages are {homes_out:.2} of the outskirts and {homes_in:.2} of the market"
        );
    }

#[test]
    #[ignore = "a measurement of the real paving"]
    fn what_the_paving_measures() {
        let terrain = crate::world::terrain::Terrain::new();
        let plan = terrain.plan();
        let site = plan.sites()[0];
        let layout = lay_out(&site, plan.approach(site.at), crate::config::WORLD_SEED);
        println!("{} streets", layout.streets.len());
        let mesh = pave(&layout.streets, &terrain, site.at);
        use bevy::render::mesh::VertexAttributeValues;
        let places = match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
            Some(VertexAttributeValues::Float32x3(v)) => v.clone(),
            _ => Vec::new(),
        };
        println!("paving: {} vertices, {} indices",
            places.len(),
            mesh.indices().map(|i| i.len()).unwrap_or(0));
        if places.is_empty() { return; }
        let ys: Vec<f32> = places.iter().map(|p| p[1]).collect();
        let lo = ys.iter().cloned().fold(f32::MAX, f32::min);
        let hi = ys.iter().cloned().fold(f32::MIN, f32::max);
        println!("paving Y from {lo:.2} to {hi:.2}");
        println!("ground at the site middle: {:.2}", terrain.height(site.at.x, site.at.y));
        let xs: Vec<f32> = places.iter().map(|p| p[0]).collect();
        let zs: Vec<f32> = places.iter().map(|p| p[2]).collect();
        println!("paving spans x {:.0}..{:.0}, z {:.0}..{:.0} (entity sits at the site)",
            xs.iter().cloned().fold(f32::MAX, f32::min), xs.iter().cloned().fold(f32::MIN, f32::max),
            zs.iter().cloned().fold(f32::MAX, f32::min), zs.iter().cloned().fold(f32::MIN, f32::max));
    }

    #[test]
    fn every_building_faces_a_street() {
        // A door that opens onto the back of the next house is the thing that makes
        // a generated town feel like it was poured rather than laid out.
        for seed in 0..30 {
            let site = a_site(seed % 2 == 0, 85.0);
            let approach = Vec2::new(0.7, -0.7).normalize();
            let layout = lay_out(&site, approach, seed);
            for plot in &layout.plots {
                // The way the door looks, which is the building's own -Y turned by
                // its facing.
                let out = Vec2::new(plot.facing.cos(), plot.facing.sin());
                let door = Vec2::new(out.y, -out.x);
                let ahead = plot.at + door * 6.0;
                let nearer = layout
                    .streets
                    .iter()
                    .map(|s| s.nearest(ahead).0)
                    .fold(f32::MAX, f32::min);
                let here = layout
                    .streets
                    .iter()
                    .map(|s| s.nearest(plot.at).0)
                    .fold(f32::MAX, f32::min);
                assert!(
                    nearer < here,
                    "seed {seed}: a {:?} faces away from every street ({nearer:.1} vs \
                     {here:.1})",
                    plot.what
                );
            }
        }
    }

#[test]
    fn every_city_is_named_and_no_town_is() {
        use terrain_core::region::Country;
        let city = a_site(true, 90.0);
        let town = a_site(false, 50.0);
        assert!(name_of(&town, Country::Ordinary, 0).is_none(), "a town was named");
        for index in 0..24 {
            assert!(
                name_of(&city, Country::Ordinary, index).is_some(),
                "city {index} went unnamed"
            );
        }
        // And the name suits the country it stands in, which is the whole reason
        // this is not one flat list.
        assert_eq!(name_of(&city, Country::Snow, 0), Some("Hollowfrost"));
        assert_eq!(name_of(&city, Country::Desert, 0), Some("Sunmere"));
    }

#[test]
    fn a_town_actually_has_a_town_in_it() {
        // The test that was missing, and its absence let a change land that emptied
        // every settlement in the world: the others all say "no building does X",
        // which is vacuously true of a town with no buildings. Six of them passed
        // on a city containing one house.
        for seed in 0..30 {
            let city = lay_out(&a_site(true, 90.0), Vec2::new(0.8, 0.6).normalize(), seed);
            assert!(
                city.plots.len() >= 18,
                "seed {seed}: a city has {} buildings in it",
                city.plots.len()
            );
            let village = lay_out(&a_site(false, 58.0), Vec2::new(0.3, -0.95).normalize(), seed);
            assert!(
                village.plots.len() >= 6,
                "seed {seed}: a village has {} buildings in it",
                village.plots.len()
            );
        }
    }

    #[test]
    fn a_town_is_the_same_town_every_time_it_is_asked() {
        let site = a_site(true, 90.0);
        let once = lay_out(&site, Vec2::X, 21);
        let twice = lay_out(&site, Vec2::X, 21);
        assert_eq!(once.plots.len(), twice.plots.len());
        for (a, b) in once.plots.iter().zip(&twice.plots) {
            assert_eq!(a.what, b.what);
            assert!((a.at - b.at).length() < 1.0e-5);
        }
    }
}
