//! How a settlement is laid out: streets, the plots between them, and what stands
//! on each one.
//!
//! # Roads, then parcels, then lots
//!
//! This is the standard pipeline and it is worth naming because every step of it
//! exists for a reason the step before cannot supply:
//!
//!   1. STREETS are laid first, because everything else is defined relative to
//!      them. A plot is not a piece of ground, it is a piece of ground *with
//!      frontage*.
//!   2. The ground between streets is a PARCEL.
//!   3. A parcel is cut into LOTS by recursive subdivision, sliced along its
//!      shorter axis and stopped by rules: too small, too thin, or no longer
//!      touching a street.
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
const A_FRONTAGE_IS_AT_LEAST: f32 = 8.2;

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

/// What stands on a plot.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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
            Building::Cottage => Vec2::new(6.0, 4.5),
            Building::Townhouse => Vec2::new(6.0, 6.0),
            Building::Shop => Vec2::new(7.5, 6.0),
            Building::GuildHall => Vec2::new(10.5, 7.5),
        }
    }

    /// How much room it needs on a lot, including the ground it is set into.
    fn wants(self) -> Vec2 {
        self.footprint() + Vec2::splat(1.6)
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
    let depth = (reach * 0.16).clamp(9.0, 15.0);
    // One ring per band of blocks, out as far as the town reaches.
    let band = depth * 2.0 + LANE_WIDE + SETBACK * 2.0;
    let rings = (((reach - square) / band).floor() as usize).clamp(1, 3);

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

    let mut plots = Vec::new();
    if let Some(hall) = civic {
        plots.push(hall);
    }
    for (index, lot) in lots.iter().enumerate() {
        if !lot.has_frontage() {
            continue;
        }
        let what = what_stands_here(index, lot, site.at, seed);
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
fn what_stands_here(index: usize, lot: &Parcel, middle: Vec2, seed: u32) -> Option<Building> {
    let roll = unit(seed.wrapping_add(index as u32 * 131), 11);
    let fits = |what: Building| {
        let wants = what.wants();
        lot.frontage >= wants.x && lot.depth >= wants.y
    };

    // TRADE ON THE SQUARE, HOMES ON THE EDGE. The medieval rule and the obvious one:
    // the ground with the most feet on it carries the shops and the inns, and the
    // houses are further out. Sorted by distance from the middle rather than rolled,
    // so a town has a centre rather than a scatter.
    let out = lot.at.distance(middle);
    let wanted = if out < 34.0 && roll < 0.62 {
        Building::Shop
    } else if out < 58.0 && roll < 0.55 {
        Building::Townhouse
    } else {
        Building::Cottage
    };
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

#[cfg(test)]
mod tests {
    use super::*;

    fn a_site(city: bool, radius: f32) -> Site {
        Site {
            at: Vec2::new(120.0, -80.0),
            height: 30.0,
            radius,
            city,
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
