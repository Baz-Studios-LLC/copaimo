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

/// A rectangle of ground, axis-aligned in the town's own frame.
#[derive(Clone, Copy, Debug)]
struct Parcel {
    /// Middle, in the town's frame.
    at: Vec2,
    /// Full extents.
    span: Vec2,
    /// Which way the street it fronts lies, as a yaw in the town's frame.
    facing: f32,
    /// Where the STREET EDGE of the parcel this was cut from lies, measured along
    /// the door direction.
    ///
    /// Inherited by every lot cut out of it, and it is what makes the frontage rule
    /// possible: a lot is only worth building on if it still touches the street the
    /// parcel was laid against. Without it, cutting a parcel across its depth makes
    /// a back lot with no road access, and a building on one stands in the back
    /// garden of the building in front of it - which is what the overlap test
    /// caught.
    front: f32,
}

impl Parcel {
    /// Which way this parcel's buildings look.
    fn door(&self) -> Vec2 {
        Vec2::new(self.facing.sin(), -self.facing.cos())
    }

    /// Its own street edge, along the door direction.
    fn edge(&self) -> f32 {
        let door = self.door();
        self.at.dot(door) + (self.span.x * door.x).abs() * 0.5 + (self.span.y * door.y).abs() * 0.5
    }

    /// Whether it still fronts the street it was cut from.
    fn has_frontage(&self) -> bool {
        (self.edge() - self.front).abs() < 0.6
    }
}

/// How wide a side lane is. Narrower than the high street, because it is one.
pub const LANE_WIDE: f32 = 4.2;

/// A strip of buildable ground down each side of a street.
///
/// `at` and `axis` are in the town's frame: where the street's middle is and which
/// way it runs. Both sides get a parcel, each facing back at the street it fronts,
/// which is what makes a lane have two rows of houses rather than one.
fn frontage_parcels(
    into: &mut Vec<Parcel>,
    at: Vec2,
    axis: Vec2,
    length: f32,
    front: f32,
    depth: f32,
    _wide: f32,
) {
    let axis = if axis.length_squared() > 1.0e-6 { axis.normalize() } else { Vec2::X };
    let side_axis = axis.perp();
    let turn = axis.y.atan2(axis.x);
    for side in [-1.0_f32, 1.0] {
        let middle = at + side_axis * (side * (front + depth * 0.5));
        // A door faces back ACROSS the street, and the turn that does it is the
        // street's own - not a quarter off it.
        //
        // Derived rather than guessed, because guessing got it wrong: a building's
        // door points along its local -Y, which after a turn of `f` points
        // `(sin f, -cos f)`. The parcel on the +perp side of a street needs its door
        // pointing at -perp, which is `(axis.y, -axis.x)` - so `sin f = axis.y` and
        // `cos f = axis.x`, and f is the street's own angle. The other side is that
        // plus a half turn. A quarter turn, which is what was there, points every
        // door straight down the street it is supposed to face.
        let facing = if side > 0.0 { turn } else { turn + std::f32::consts::PI };
        let door = Vec2::new(facing.sin(), -facing.cos());
        let span = Vec2::new(
            (length * axis.x).abs() + (depth * side_axis.x).abs(),
            (length * axis.y).abs() + (depth * side_axis.y).abs(),
        );
        let edge = middle.dot(door)
            + (span.x * door.x).abs() * 0.5
            + (span.y * door.y).abs() * 0.5;
        into.push(Parcel { at: middle, span, front: edge, facing });
    }
}

/// Lays out one settlement.
///
/// `approach` is the direction the road network arrives from, which the high street
/// is built along. `seed` separates one town's dice from another's.
pub fn lay_out(site: &Site, approach: Vec2, seed: u32) -> Layout {
    let reach = site.radius * FILLS;
    if reach < 12.0 {
        return Layout::default();
    }

    // The town's own frame: X along the high street, Y across it.
    let along = if approach.length_squared() > 1.0e-6 {
        approach.normalize()
    } else {
        Vec2::X
    };
    let across = Vec2::new(-along.y, along.x);
    let turn = along.y.atan2(along.x);

    let mut streets = Vec::new();
    let mut parcels = Vec::new();

    // # Not a crossroads
    //
    // The first cut laid a high street and one cross street at right angles through
    // the middle, and a symmetric cross is the least interesting plan there is: it
    // is the shape of a road junction rather than of a place, it puts the town's
    // best ground under tarmac, and every town in the world comes out identical
    // because the only thing a hash gets to choose is which way the cross points.
    //
    // Towns do not grow that way. One route passes through - that is why the town
    // is there - and everything else hangs off it: lanes branch where somebody
    // needed to get somewhere, at whatever spacing the ground allowed, on whichever
    // side had room. A back lane appears once the frontage on the main street runs
    // out. What that produces is a town with a spine and ribs, and it reads as
    // somewhere that grew rather than somewhere that was drawn.
    let depth = (reach * 0.5).min(20.0);
    let front = STREET_WIDE * 0.5 + SETBACK;

    // The spine.
    streets.push(Street {
        from: site.at - along * reach,
        to: site.at + along * reach,
        wide: STREET_WIDE,
    });
    frontage_parcels(&mut parcels, Vec2::ZERO, Vec2::X, reach * 1.9, front, depth, STREET_WIDE);

    // The ribs. Where they leave the spine and how far they run is the town's own
    // business: irregular spacing is most of what stops a plan looking drawn.
    let ribs = if site.city { 4 } else { 2 };
    let mut last_at = -reach;
    for rib in 0..ribs {
        let want = reach * (-0.72 + 1.5 * (rib as f32 + 0.4 + 0.45 * unit(seed, 20 + rib)) / ribs as f32);
        // Never two ribs on top of each other, and never one at the very end of the
        // spine where it would have a town on one side and a field on the other.
        if want - last_at < 26.0 || want.abs() > reach * 0.82 {
            continue;
        }
        last_at = want;
        // Alternating sides, mostly - but not strictly, or the alternation itself
        // becomes the pattern the eye finds.
        let side = if unit(seed, 30 + rib) < 0.62 {
            if rib % 2 == 0 { 1.0 } else { -1.0 }
        } else if rib % 2 == 0 {
            -1.0
        } else {
            1.0
        };
        let run = (depth + front) * (1.5 + 0.9 * unit(seed, 40 + rib));
        let lane = LANE_WIDE;
        let mouth = site.at + along * want + across * (side * front * 0.4);
        streets.push(Street {
            from: mouth,
            to: mouth + across * (side * run),
            wide: lane,
        });
        // Ground either side of the rib, starting clear of the spine it leaves.
        frontage_parcels(
            &mut parcels,
            Vec2::new(want, side * (front + run * 0.5)),
            // The TOWN's frame, not the world's: the spine is +X here whatever
            // direction the road actually arrives from. Passing the world vectors
            // in was the whole of why a city came out with one building in it -
            // every parcel was computed against a diagonal and none of the lots
            // landed anywhere near the streets they were supposed to front.
            Vec2::Y * side,
            run * 0.9,
            lane * 0.5 + SETBACK,
            depth * 0.8,
            lane,
        );
    }

    // A back lane, once a city has more frontage than one street can carry.
    if site.city {
        let back = front + depth + STREET_WIDE * 0.6;
        let side = if unit(seed, 51) < 0.5 { 1.0 } else { -1.0 };
        streets.push(Street {
            from: site.at - along * reach * 0.62 + across * (side * back),
            to: site.at + along * reach * 0.62 + across * (side * back),
            wide: LANE_WIDE,
        });
        frontage_parcels(
            &mut parcels,
            Vec2::new(0.0, side * back),
            Vec2::X,
            reach * 1.2,
            LANE_WIDE * 0.5 + SETBACK,
            depth * 0.7,
            LANE_WIDE,
        );
    }

    // THE CIVIC PLOT, and why it is carved out before anything is subdivided.
    //
    // A guild hall wants 12 by 9 metres of ground. Subdivision cuts a parcel down
    // until its lots are about the size of a house, so no lot it produces is ever
    // big enough - the first cut laid out cities with nought guild halls in them
    // and the test said so. A city square is not a leftover: it is set aside first
    // and the streets are laid around it, which is also how a real one happens.
    let mut civic: Option<Plot> = None;
    if site.city {
        let hall = Building::GuildHall;
        let stand = STREET_WIDE * 0.5 + SETBACK + hall.footprint().y * 0.5 + 0.4;
        // UP THE HIGH STREET, not on the crossroads.
        //
        // Sat at the middle it straddled the cross street - the town's two streets
        // meet at the centre, so the centre is the one place a building cannot go,
        // and putting the most important building there was exactly the wrong
        // instinct. A hall at the head of the high street, looking back down it, is
        // both clear of the junction and the better composition: you see it from
        // the whole length of the street as you walk up.
        // TRIED IN SEVERAL PLACES, because the hall is placed before the lanes are
        // checked and a lane can now run straight through where it wants to stand.
        //
        // It was one fixed spot up the high street when the plan was a cross and
        // there was nothing else to hit. With ribs branching at hashed intervals
        // that spot is sometimes a road, and a guild hall in a lane is the same
        // fault as the one in the crossroads - only harder to see coming, because
        // it happens for some seeds and not others.
        let base = (STREET_WIDE * 0.5 + hall.footprint().x * 0.5 + 4.0).max(reach * 0.34);
        let bulk = hall.footprint().length() * 0.5;
        'placing: for attempt in 0..12 {
            let step = 0.16 * reach * (attempt / 2) as f32;
            let up = if attempt % 2 == 0 { base + step } else { -(base + step) };
            for side in [1.0_f32, -1.0] {
                let at_town = Vec2::new(up, side * stand);
                let at = site.at + along * at_town.x + across * at_town.y;
                if at.distance(site.at) > reach {
                    continue;
                }
                if streets
                    .iter()
                    .any(|street| street.nearest(at).0 < street.wide * 0.5 + bulk * 0.55)
                {
                    continue;
                }
                civic = Some(Plot {
                    at,
                    facing: if side > 0.0 { turn } else { turn + std::f32::consts::PI },
                    what: hall,
                });
                break 'placing;
            }
        }
    }

    // Each parcel is cut into lots, and each lot gets a building.
    let mut lots = Vec::new();
    for (index, parcel) in parcels.iter().enumerate() {
        subdivide(*parcel, seed.wrapping_add(index as u32 * 977), 0, &mut lots);
    }

    // Sorted by how central they are, so that what goes where is decided by
    // position rather than by the order the recursion happened to finish in - a
    // guild hall on the outskirts because a slice went the other way is exactly the
    // kind of thing that makes a generated town feel generated.
    lots.sort_by(|a, b| {
        a.at.length_squared()
            .partial_cmp(&b.at.length_squared())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut plots = Vec::new();
    if let Some(hall) = civic {
        plots.push(hall);
    }
    for (index, lot) in lots.iter().enumerate() {
        // Nothing may be built on ground the guild hall is standing on, or beside
        // it closer than the room it needs.
        if let Some(hall) = civic {
            let want = (hall.what.footprint().max_element() + 8.0) * 0.5;
            let at_world = site.at + along * lot.at.x + across * lot.at.y;
            if at_world.distance(hall.at) < want {
                continue;
            }
        }
        // Only lots that still touch the street they were laid against. See
        // `Parcel::has_frontage`.
        if !lot.has_frontage() {
            continue;
        }
        // Which way this lot's door looks, in the town's frame, and therefore which
        // of the lot's two extents is its FRONTAGE and which is its depth.
        let door = Vec2::new(lot.facing.sin(), -lot.facing.cos());
        let sideways = door.perp();
        let frontage = (lot.span.x * sideways.x).abs() + (lot.span.y * sideways.y).abs();
        let deep = (lot.span.x * door.x).abs() + (lot.span.y * door.y).abs();

        let what = what_stands_here(index, lot, frontage, deep, seed);
        let Some(what) = what else { continue };

        // PLACED AGAINST THE STREET, not in the middle of its lot.
        //
        // Two reasons, and the first is that it is simply what buildings do - a
        // house sits on its frontage and keeps its ground behind it. The second is
        // the bug: subdividing a parcel across its depth makes a shallow front lot
        // and a deeper back one, and a building centred on the shallow one reaches
        // out past the lot's own edge and into the carriageway.
        let front = lot.at + door * (deep * 0.5);
        let at_town = front - door * (what.footprint().y * 0.5 + 0.35);

        let at = site.at + along * at_town.x + across * at_town.y;
        if at.distance(site.at) > reach {
            continue;
        }
        // NOTHING STANDS IN A CARRIAGEWAY, checked against every street rather than
        // against the one this lot fronts.
        //
        // The parcels beside the high street run its whole length, which means they
        // run straight THROUGH the cross street where the two meet - so a lot could
        // be laid on the junction and a shop built in the middle of it. Clipping
        // the parcels at the junction would work and this is better: it holds for
        // any street the town ever grows, not just the one crossing that was
        // thought of when the parcels were laid.
        let bulk = what.footprint().length() * 0.5;
        if streets
            .iter()
            .any(|street| street.nearest(at).0 < street.wide * 0.5 + bulk * 0.55)
        {
            continue;
        }
        // AND NOT INTO A NEIGHBOUR. Lots cut from one parcel cannot overlap each
        // other, which is why this was not needed at first - but a city has parcels
        // along two streets that meet at right angles, and the lots at the inside
        // of that corner belong to different parcels and know nothing of one
        // another. Two cottages came out 4.4 m apart wanting 6.
        //
        // Checked against what is already placed rather than by trying to make the
        // parcels not touch: a town grows more streets later, and every new pair of
        // them makes another corner.
        if plots.iter().any(|placed| {
            let want = (bulk + placed.what.footprint().length() * 0.5) * 0.62;
            at.distance(placed.at) < want
        }) {
            continue;
        }
        plots.push(Plot {
            at,
            facing: turn + lot.facing,
            what,
        });
    }

    Layout { streets, plots }
}

/// Cuts a parcel into lots by slicing it along its shorter axis, recursively.
///
/// The rules that stop it are the ones the research names: too small, too thin. A
/// lot that fails either is kept whole rather than cut again, and a lot that is
/// still too big to be one building is cut once more.
fn subdivide(parcel: Parcel, seed: u32, depth: u32, into: &mut Vec<Parcel>) {
    let door = parcel.door();
    // The frontage axis: the one running ALONG the street, which is the only one
    // this is allowed to cut.
    let frontage = (parcel.span.x * door.y).abs() + (parcel.span.y * door.x).abs();

    if depth > 6 || frontage < A_FRONTAGE_IS_AT_LEAST * 2.0 {
        if frontage >= A_FRONTAGE_IS_AT_LEAST {
            into.push(parcel);
        }
        return;
    }

    // CUT ALONG THE STREET ONLY, never across the parcel's depth.
    //
    // The first cut sliced whichever axis was longer, which is the textbook OBB
    // rule - and it is the rule for a parcel with roads on all four sides. A strip
    // of ground along ONE street is not that: cutting it across its depth makes a
    // back lot with no frontage, which the frontage rule then throws away, so half
    // of every parcel became nothing and the town came out with nineteen buildings
    // scattered over ground that should hold forty.
    //
    // A street frontage is cut into narrow deep strips, which is what a row of
    // houses is and what every real high street looks like from above.
    let split = 0.42 + 0.16 * unit(seed, 3);
    let near = frontage * split;
    let far = frontage - near;
    let sideways = door.perp();

    for (share, sign) in [(near, -1.0_f32), (far, 1.0)] {
        let off = sideways * (sign * (frontage - share) * 0.5);
        let span = if parcel.span.x * sideways.x.abs() > parcel.span.y * sideways.y.abs() {
            Vec2::new(share, parcel.span.y)
        } else {
            Vec2::new(parcel.span.x, share)
        };
        subdivide(
            Parcel {
                at: parcel.at + off,
                span,
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
    frontage: f32,
    deep: f32,
    seed: u32,
) -> Option<Building> {
    let roll = unit(seed.wrapping_add(index as u32 * 131), 11);

    // A building is measured against the lot's FRONTAGE and DEPTH rather than
    // against its x and y: the lot is in the town's frame and the building is in
    // its own, and on a cross street those two are a quarter turn apart. Comparing
    // them directly put wide buildings on narrow lots whenever the street ran the
    // other way.
    let fits = |what: Building| {
        let wants = what.wants();
        frontage >= wants.x && deep >= wants.y
    };

    let out = lot.at.length();
    let wanted = if out < 26.0 && roll < 0.55 {
        Building::Shop
    } else if roll < 0.4 {
        Building::Townhouse
    } else {
        Building::Cottage
    };
    // What was wanted if it fits, and a cottage if it does not - a cottage is the
    // smallest thing there is, so a lot that cannot hold one holds nothing.
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
