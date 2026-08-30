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
use std::sync::LazyLock;

use crate::config::WORLD_SEED;
use crate::world::settle::Site;

/// How wide a street is, kerb to kerb.
///
/// Six metres. Wide enough for the eye to read it as a street rather than an alley,
/// narrow enough that the buildings either side are in the same picture - a street
/// you cannot see both sides of at once is a road.
pub const STREET_WIDE: f32 = 6.0;

/// And what a CITY's are.
///
/// A village lane is a worn track the width of the carts that made it. A city street
/// is a made thing with a footway down each side, and the footways are extra: giving
/// a 6 m street two pavements would leave a metre of carriageway, which is an alley.
/// So the carriageway keeps the width it already had and the street grows by exactly
/// the two footways it gained: a 10 m high street is the old 6 m of road with 2 m of
/// pavement each side, and an 8 m lane is the old 4.2 m rounded to 4.
///
/// Sized by what a city can afford as much as by what a street wants. At 12 m and 9 m
/// - a 2.4 m footway - `a_town_actually_has_a_town_in_it` dropped a city to fifteen
/// buildings against a floor of eighteen, because every metre of street is a metre of
/// lot nobody builds on. The guard is right and the first numbers were greedy.
///
/// Buildings follow automatically: `clear_of_streets` measures from the ribbon's own
/// half-width, so a wider street sets its frontages back by exactly the footway it
/// gained rather than by a second number that could disagree.
pub const CITY_STREET_WIDE: f32 = 10.0;
pub const CITY_LANE_WIDE: f32 = 8.0;

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
// 16, up from 11. Photographed, eleven across a village's rings left long empty
// stretches of street - "they feel kinda sparse". Still a village and still nowhere
// near the three hundred it started at.
const HOUSES_IN_A_VILLAGE: usize = 16;
const HOUSES_IN_A_CITY: usize = 34;

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
// 0.94, down from 1.15.
//
// # A town has to fit on the ground that was levelled for it
//
// This built out to 1.15 times the site's radius on the reasoning that a town may
// spill past its levelled rim onto the fade. It may - a cottage on a gentle slope is
// fine - but a STREET may not, because `settle` flattens every lane to the site's
// own height: a lane laid past the levelled ground cuts a five-metre lip into the
// hillside it crosses, which is a wall you cannot walk up.
//
// The levelled radii went up to match, so a town is the same size it was; it is the
// GROUND that grew, not the plan. See CITY_RADIUS and TOWN_RADIUS.
const FILLS: f32 = 0.94;

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

/// How near a building has to be for a yard to be ITS yard, in metres.
///
/// About two lots. Nearer than this and the two read as one property - a house and
/// its garden, a shop and its work yard - so the yard takes its programme from the
/// building. Further and the yard belongs to the street instead, and the district
/// says what it is.
const BELONGS_WITHIN: f32 = 26.0;

/// The height a thing of this size has to stand at so no part of it sinks.
///
/// # A footprint is not a point
///
/// Everything was placed at the height of its own MIDDLE. On level ground that is
/// right and on anything else it is not: a nine-metre yard on a one-in-twenty slope
/// has its far corner nearly a quarter of a metre under the ground, and what you see
/// is a fence with its bottom rail buried and a bench sunk to the seat.
///
/// The highest corner decides. A thing then rests ON the ground at its high side and
/// stands slightly proud at its low side, which is what a thing standing on a slope
/// does - and is the error worth having, because the other one hides geometry.
pub fn stands_at(
    terrain: &crate::world::terrain::Terrain,
    at: Vec2,
    footprint: Vec2,
    facing: f32,
) -> f32 {
    under(terrain, at, footprint, facing).1
}

/// The lowest and highest ground under a building's footprint.
///
/// Two answers from one walk of the corners, because a building needs both: it is
/// SEATED on the highest, so it is never sunk into a rise, and it is FOOTED down to
/// the lowest, so the far end does not hang in the air. Taking only the highest is
/// what made a 26 m guild hall float.
pub fn under(
    terrain: &crate::world::terrain::Terrain,
    at: Vec2,
    footprint: Vec2,
    facing: f32,
) -> (f32, f32) {
    let half = footprint * 0.5;
    let (sin, cos) = facing.sin_cos();
    let middle = terrain.drawn_height(at.x, at.y);
    let (mut lowest, mut highest) = (middle, middle);
    for sx in [-1.0_f32, 1.0] {
        for sy in [-1.0_f32, 1.0] {
            let local = Vec2::new(sx * half.x, sy * half.y);
            let corner = at + Vec2::new(local.x * cos - local.y * sin, local.x * sin + local.y * cos);
            let ground = terrain.drawn_height(corner.x, corner.y);
            lowest = lowest.min(ground);
            highest = highest.max(ground);
        }
    }
    (lowest, highest)
}

/// How far the ground has to fall under a building before it is given a footing.
///
/// Below this the model's own plinth covers it and a second slab would only
/// z-fight with the first.
const FOOTING_SHOWS: f32 = 0.06;

/// Which way to turn a model so its door lands where the doorway is.
///
/// # The door was on the back of every building in the world
///
/// A figure is built in Blender with its doorway on -Y, and the glTF export turns
/// Blender's Z-up into Y-up: `(x, y, z)` becomes `(x, z, -y)`. So the door that was
/// built facing -Y arrives in the game facing +Z.
///
/// The spawn turned the model by `-facing`, which sends its local +Z to
/// `(-sin, cos)`. `Plot::walls` puts the doorway gap at `(sin, -cos)` - the way the
/// lot's own frontage looks. Those are exactly opposite, so the wall with the door
/// in it faced away from the street while the gap you could actually walk through
/// was in the blank wall on the street side.
///
/// Every measurement said this was fine, because every measurement asked the LOT:
/// doors sat 3.5 m from a kerb, and `every_building_faces_a_street` passed on all
/// thirty seeds. Nothing compared the model against the collision. Photographed from
/// above, the cottage's doorstep is on the far side of it from the road.
///
/// `PI - facing` sends local +Z to `(sin, -cos)`, which is the doorway.
fn model_turn(facing: f32) -> f32 {
    std::f32::consts::PI - facing
}

impl District {
    /// How much of its frontage this district occupies, as yards per building.
    ///
    /// The hierarchy the research asks for, and the thing a single global share
    /// cannot say: a market street should be nearly solid, a crafts quarter busy but
    /// broken by work yards, and the outskirts should give way to gardens and open
    /// ground. Below one means more buildings than yards.
    pub fn occupies(self) -> f32 {
        match self {
            District::Market => 1.0,
            District::Crafts => 0.7,
            District::Outskirts => 0.45,
        }
    }

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
    fn builds(self, roll: f32, city: bool) -> Building {
        if city {
            // The modern city. Height falls off from the middle, which is what a
            // skyline IS - a city whose every building is the same height reads as
            // a housing scheme however tall they all are.
            return match self {
                District::Market => {
                    if roll < 0.55 {
                        Building::CityTower
                    } else {
                        Building::CityBlock
                    }
                }
                District::Crafts => {
                    if roll < 0.30 {
                        Building::CityTower
                    } else {
                        Building::CityBlock
                    }
                }
                District::Outskirts => Building::CityBlock,
            };
        }
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
    // # Two ages of the world
    //
    // A village is old-school fantasy - half-timbered, thatch and slate. A city is
    // modern: curtain wall, concrete, a paved street. That is not two art styles
    // stapled together, it is the setting's own history showing on the ground, and
    // it is the strongest district tool there is: you know what kind of place you
    // are standing in from the silhouette, before you can read a single sign.
    Cottage,
    Townhouse,
    Shop,
    GuildHall,
    /// A city's ordinary building: five floors of curtain wall over a lobby.
    CityBlock,
    /// Nine floors with a stepped crown - where a skyline starts.
    CityTower,
    /// Fifteen floors and a mast. THE tall thing, and the reason a city has a
    /// middle you can see from outside it. See `Building::is_landmark`.
    CitySpire,

    // ---------------------------------------------------------------- the yards
    //
    // What stands on a lot that gets no building. Each is one PROGRAMME - a purpose,
    // with its parts arranged to imply a relationship - rather than a scatter of
    // props: a garden has beds and a path from the gate to the door, a work yard has
    // a bench under a lean-to with its material stacked beside it. See `dev/art/yard.py`.
    Garden,
    WorkYard,
    Pen,
    StoreYard,
    Stall,

    // The same purposes in the OTHER age's vocabulary.
    //
    // One kit for both ages put a post-and-rail fence and a stack of crates in the
    // middle of a modern city, which reads as a farmyard somebody left between two
    // office towers. A crafts quarter has a work yard either way; it is a lean-to
    // and stacked timber in a village and a service bay with a skip and pallets in
    // a city.
    CityGreen,
    CityService,
    CityKiosk,
    CityForecourt,
    /// A stepped stone cross on a village square.
    MarketCross,
    /// A roofed well, for a village junction.
    Well,
    /// A city's junction landmark: a plinth under a leaning steel spike.
    Monument,
}

impl Building {
    /// The file it is drawn from, under `assets/models/`.
    pub fn model(self) -> &'static str {
        match self {
            Building::Cottage => "models/town_cottage.glb",
            Building::Townhouse => "models/town_townhouse.glb",
            Building::CityBlock => "models/town_city_block.glb",
            Building::CityTower => "models/town_city_tower.glb",
            Building::CitySpire => "models/town_city_spire.glb",
            Building::Garden => "models/yard_garden.glb",
            Building::WorkYard => "models/yard_work.glb",
            Building::Pen => "models/yard_pen.glb",
            Building::StoreYard => "models/yard_store.glb",
            Building::Stall => "models/yard_stall.glb",
            Building::CityGreen => "models/yard_city_green.glb",
            Building::CityService => "models/yard_city_service.glb",
            Building::CityKiosk => "models/yard_city_kiosk.glb",
            Building::CityForecourt => "models/yard_city_forecourt.glb",
            Building::MarketCross => "models/town_market_cross.glb",
            Building::Well => "models/town_well.glb",
            Building::Monument => "models/town_monument.glb",
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
            // A HALL YOU SPEND TIME IN.
            //
            // 18 x 13.5 was a branch office, sized off the concept sheet's own
            // elevations. The guild is where a warden registers a companion, takes
            // transfers, reads the board and takes work - minutes at a time, not
            // seconds - and a room that size is a corridor with furniture in it once
            // a counter, a table and benches are in there. Half as much building
            // again in each direction is nearly twice the floor.
            Building::GuildHall => Vec2::new(26.0, 18.0),
            // Measured off the exported models, same as the rest.
            Building::CityBlock => Vec2::new(11.3, 10.8),
            Building::CityTower => Vec2::new(10.8, 11.3),
            Building::CitySpire => Vec2::new(11.8, 12.8),
            // A lot's worth of ground, which is a cottage's - a yard stands on
            // exactly the lots a house would have taken. Measured off the exported
            // models like the rest.
            Building::Garden
            | Building::WorkYard
            | Building::Pen
            | Building::StoreYard
            | Building::CityGreen
            | Building::CityService
            | Building::CityForecourt => Vec2::new(9.0, 7.5),
            Building::CityKiosk => Vec2::new(7.5, 4.6),
            // A stall belongs to the street rather than to a plot, so it is smaller
            // and has no fence to put a wall across a square.
            Building::Stall => Vec2::new(7.2, 4.2),
            // A landmark stands in the open, so its footprint is what it occupies
            // rather than what it needs around it.
            Building::MarketCross => Vec2::new(3.4, 3.4),
            Building::Well => Vec2::new(2.4, 2.2),
            Building::Monument => Vec2::new(5.0, 5.0),
        }
    }

/// Whether this is a LANDMARK rather than a building somebody lives in.
    ///
    /// Scott Rogers' hub-town rules, from the Disneyland model, name two things my
    /// towns did not have. One is a "weenie": something tall enough to see from
    /// outside the place, that pulls you toward its middle. The other is that a
    /// landmark has to be a DIFFERENT KIND OF THING from what surrounds it, not a
    /// bigger one - a tall house is a house, and my guild hall was exactly that.
    ///
    /// A landmark takes no lot and keeps no frontage: it stands in the open where
    /// people gather, which is what makes a node a node.
    /// How many kinds there are.
    const KINDS: usize = 19;

    /// Where each kind sits in `ALL`.
    ///
    /// # Making the list actually exhaustive
    ///
    /// `ALL` on its own is a list somebody has to remember to extend, and Rust is
    /// perfectly happy to compile a twentieth variant while the array stays at
    /// nineteen - so "a future variant cannot evade it" was a stronger claim than the
    /// code backed, which Codex was right to pick up.
    ///
    /// This match is EXHAUSTIVE, so the compiler will not accept a new variant until
    /// somebody gives it a place. `the_list_of_kinds_is_every_kind` then checks that
    /// every place from nought to `KINDS` is filled exactly once, which fails until
    /// `ALL` and `KINDS` have been extended too. Neither half is enough alone.
    fn place(self) -> usize {
        match self {
            Building::Cottage => 0,
            Building::Townhouse => 1,
            Building::Shop => 2,
            Building::GuildHall => 3,
            Building::CityBlock => 4,
            Building::CityTower => 5,
            Building::CitySpire => 6,
            Building::MarketCross => 7,
            Building::Well => 8,
            Building::Monument => 9,
            Building::Garden => 10,
            Building::WorkYard => 11,
            Building::Pen => 12,
            Building::StoreYard => 13,
            Building::Stall => 14,
            Building::CityGreen => 15,
            Building::CityService => 16,
            Building::CityKiosk => 17,
            Building::CityForecourt => 18,
        }
    }

    /// Every kind there is.
    ///
    /// Written once so a test cannot miss one. `every_building_has_a_model_on_disk`
    /// used to list the variants by hand, and five yards were added to the enum
    /// without being added to it - so the one guard that proves a `Building` names a
    /// file that exists stopped covering a third of them, silently, which is the
    /// only way that guard can fail.
    pub const ALL: [Building; Self::KINDS] = [
        Building::Cottage,
        Building::Townhouse,
        Building::Shop,
        Building::GuildHall,
        Building::CityBlock,
        Building::CityTower,
        Building::CitySpire,
        Building::MarketCross,
        Building::Well,
        Building::Monument,
        Building::Garden,
        Building::WorkYard,
        Building::Pen,
        Building::StoreYard,
        Building::Stall,
        Building::CityGreen,
        Building::CityService,
        Building::CityKiosk,
        Building::CityForecourt,
    ];

    /// Whether a yard is enclosed, and how wide the way in is.
    ///
    /// # A fence you can walk through is scenery
    ///
    /// Yards started with no collision at all, on the grounds that you walk INTO a
    /// garden. True of the ground and false of the fence around it: a 1.9 m mesh
    /// screen you stroll through reads as a hologram, and the pen, the work yard and
    /// the service bay are all defined by being enclosed.
    ///
    /// So the fenced programmes get their fence, with the gap at the front left
    /// open - the same gap the model has, because that is where the gate is. The
    /// open programmes - a stall, a kiosk, a planted square, a paved forecourt -
    /// have nothing to walk into and get nothing.
    ///
    /// # A gate is not the only way a front can be open
    ///
    /// This returned a gate width and nothing else, so every fenced yard was assumed
    /// to have four runs. The city's SERVICE BAY has three: `city_service` in
    /// `dev/art/yard.py` builds both flanks and the back and no front at all, because
    /// a loading bay is a thing you drive into. The game put collision stubs across
    /// that open frontage anyway - invisible walls over most of a bay you can see
    /// straight through.
    ///
    /// Naming the two cases makes the service bay impossible to state wrongly: a
    /// programme has to say which it is rather than leave it to be inferred from a
    /// number.
    ///
    /// Found by Codex. The old-world gates are still two copies of one fact - 3.06 is
    /// `wide * 0.34` in `yard.py` - and they currently agree; closing that loop wants
    /// the fence runs measured and written down the way the windows now are.
    pub fn fenced(self) -> Option<Fenced> {
        let sides = FENCES.get(self.figure())?;
        // A side whose largest hole is most of the side has no run in it.
        let open = |side: usize| sides[side].0 > sides[side].1 * OPEN_SIDE;
        if open(1) && open(2) && open(3) {
            return None;
        }
        if open(0) {
            return Some(Fenced::OpenFronted);
        }
        Some(Fenced::Gated(sides[0].0))
    }

    /// The wall a lit window hangs on: how wide, how deep, and how many storeys of
    /// it are glass.
    ///
    /// # A footprint is not a facade
    ///
    /// The lit panes were placed against `footprint`, which is what a building keeps
    /// clear on the GROUND and is deliberately bigger than the building itself. So
    /// they floated a metre off the glass, hung past the corners, and lined up with
    /// none of the windows behind them - reported as lights floating in front of the
    /// buildings, which is exactly what they were.
    ///
    /// These are the numbers the figure was built with, checked against what Blender
    /// writes out by `the_facades_are_the_size_the_game_thinks_they_are`. A tower
    /// spends its ground floor on a lobby, so the glazing starts a storey and a half
    /// up and there is one fewer of it than the building has floors.
    /// How wide the gap in this building's front wall is, in metres.
    ///
    /// The opening the model was actually built with, plus `DOOR_GIVE`. A tower gets
    /// its lobby rather than a cottage's door - one constant for both is what left
    /// the city with an entrance it could only be walked through the middle of.
    pub fn walk_in(self) -> f32 {
        let opening = if self.facade().is_some() { LOBBY_DOORWAY } else { DOORWAY };
        opening + DOOR_GIVE
    }

    pub fn facade(self) -> Option<(f32, f32, usize)> {
        match self {
            Building::CityBlock => Some((10.5, 9.0, 4)),
            Building::CityTower => Some((10.0, 9.5, 8)),
            Building::CitySpire => Some((11.0, 11.0, 13)),
            _ => None,
        }
    }

    /// How many floors this has, and how tall one is.
    ///
    /// Measured off the exported models, like the footprints, and kept in step with
    /// `FLOOR_TALL` in `dev/art/town.py`. Only the city knows: the old world's
    /// buildings have windows placed one at a time rather than a band a storey.
    /// How many GLAZED storeys a curtain-walled figure has.
    ///
    /// # It used to answer for the old world too, and it was wrong
    ///
    /// It said a cottage had two. A cottage has one - `shell` is called with one
    /// storey - so the lamps lit a second floor's worth of windows at 5.3 m on a wall
    /// that stops at 3.6, out in the air above the eaves. The shop and the guild hall
    /// were wrong as well, in both directions.
    ///
    /// Nothing asks now: an old-world building's windows are measured off the model
    /// and read from `town.txt` - see `world::lamp::WINDOWS` - and the number of
    /// floors comes from the windows themselves. This delegates to `facade` rather
    /// than repeating its third field, so the one number left cannot drift either.
    pub fn storeys(self) -> Option<usize> {
        self.facade().map(|(_, _, floors)| floors)
    }

    /// The name `dev/art/town.py` builds this under, which is how the measured
    /// contract in `assets/models/town.txt` is keyed.
    ///
    /// Taken off `model` rather than written out again.
    pub fn figure(self) -> &'static str {
        self.model()
            .trim_start_matches("models/")
            .trim_end_matches(".glb")
    }

    /// Whether its windows come as a band of glass a storey or as separate panes.
    ///
    /// A tower's facade is a curtain wall and a lit floor is a lit BAND. A cottage
    /// has windows in a wall, and lighting the whole wall of one would read as a
    /// building on fire.
    pub fn glazed_in_bands(self) -> bool {
        matches!(
            self,
            Building::CityBlock | Building::CityTower | Building::CitySpire
        )
    }

    /// Whether this is a yard rather than a building.
    ///
    /// A yard is ground with things standing on it - beds, a bench, a stack of
    /// timber, a fence a metre high. You walk into a garden; there is nothing to
    /// walk into the side of. So a yard has no walls, which also means it costs the
    /// collision path nothing at all.
    pub fn is_yard(self) -> bool {
        matches!(
            self,
            Building::Garden
                | Building::WorkYard
                | Building::Pen
                | Building::StoreYard
                | Building::Stall
                | Building::CityGreen
                | Building::CityService
                | Building::CityKiosk
                | Building::CityForecourt
        )
    }

    pub fn is_landmark(self) -> bool {
        matches!(
            self,
            Building::MarketCross | Building::Well | Building::Monument
        )
    }

    /// The tall thing a settlement of this kind is known by, seen from the road in.
    pub fn weenie(city: bool) -> Building {
        if city {
            Building::CitySpire
        } else {
            Building::GuildHall
        }
    }

    /// The landmark that stands on the square, and the one at a lesser junction.
    /// What a lot with no building on it is FOR, by where it stands.
    ///
    /// District-led, because that is what districts are: a market street trades, a
    /// crafts quarter works, and the outskirts grow things and keep animals. Two
    /// programmes per district rather than one, so a run of lots does not repeat -
    /// and only two, because the point is that a garden next to a garden still reads
    /// as a neighbourhood while five unrelated props read as litter.
    /// What a lot with no building on it is FOR.
    ///
    /// `roll` is a hash of the settlement's seed and the LOT's own identity, not its
    /// position in a list. Taken from enumeration order, inserting or removing one
    /// eligible lot earlier in the ring flipped the programme of every lot after it,
    /// so a change anywhere rewrote the whole town.
    pub fn yard_for(
        district: District,
        city: bool,
        beside: Option<Building>,
        roll: u32,
    ) -> Building {
        // WHAT IT BELONGS TO decides what it is.
        //
        // The programme used to come from a hash of the lot, which put a work yard
        // beside a cottage and a kitchen garden behind a shop as readily as the other
        // way round. That is the difference between props that are placed and props
        // that are scattered: a yard is somebody's, and whose it is should be
        // obvious from standing between the two.
        //
        // A house has a garden. A shop has the working half of its trade behind it. A
        // guild hall has the market that gathers at it. The district still decides
        // when there is no building near enough to belong to.
        if let Some(neighbour) = beside {
            return match (neighbour, city) {
                // Trade draws trade.
                (Building::Shop, false) => Building::Stall,
                (Building::GuildHall, false) => Building::Stall,
                (Building::Shop | Building::GuildHall, true) => Building::CityKiosk,

                // A house has ground it grows things on, or keeps a beast on out at
                // the edge where there is room for one.
                (Building::Cottage, false) if district == District::Outskirts => {
                    Building::Pen
                }
                (Building::Cottage | Building::Townhouse, false) => Building::Garden,

                // An office block's back is where its bins and pallets live; its
                // front is where the paving and the benches are.
                (Building::CityBlock, true) => Building::CityService,
                (Building::CityTower | Building::CitySpire, true) => {
                    Building::CityForecourt
                }
                (Building::Cottage | Building::Townhouse, true) => Building::CityGreen,

                // A landmark gets room and an audience, never a work yard.
                (what, false) if what.is_landmark() => Building::Stall,
                (what, true) if what.is_landmark() => Building::CityForecourt,

                _ => Self::yard_by_district(district, city, roll),
            };
        }
        Self::yard_by_district(district, city, roll)
    }

    /// What a lot is for when nothing stands near enough to own it.
    fn yard_by_district(district: District, city: bool, roll: u32) -> Building {
        let other = roll % 2 == 1;
        match (district, city) {
            // Trade, either way: a canvas stall on a village square, a steel and
            // glass kiosk on a city's.
            (District::Market, false) if !other => Building::Stall,
            (District::Market, false) => Building::StoreYard,
            (District::Market, true) if !other => Building::CityKiosk,
            (District::Market, true) => Building::CityForecourt,

            // Work: a lean-to with timber stacked beside it, or a bay with a skip
            // and pallets behind a mesh fence.
            (District::Crafts, false) if !other => Building::WorkYard,
            (District::Crafts, false) => Building::StoreYard,
            (District::Crafts, true) if !other => Building::CityService,
            (District::Crafts, true) => Building::CityForecourt,

            // Growing things. A village grows food and keeps a beast; a city plants
            // a square and clips it flat.
            (District::Outskirts, false) if !other => Building::Garden,
            (District::Outskirts, false) => Building::Pen,
            (District::Outskirts, true) if !other => Building::CityGreen,
            (District::Outskirts, true) => Building::CityService,
        }
    }

    pub fn landmarks(city: bool) -> (Building, Building) {
        if city {
            (Building::Monument, Building::Monument)
        } else {
            (Building::MarketCross, Building::Well)
        }
    }

    /// How much room it needs on a lot, including the ground it is set into.
    fn wants(self) -> Vec2 {
        // AIR SUITED TO THE AGE.
        //
        // A cottage wants a garden and a city block wants a pavement, and giving
        // everything the cottage's four metres had a consequence nobody would guess:
        // a city's market lots came out too small for the towers its own district
        // rule asks for, `what_stands_here` fell back to blocks, and the market read
        // 29% towers where the rule says 55. The districts were being decided by fit
        // rather than by design.
        //
        // The four metres itself was right and stays: at 1.6 the eaves of one house
        // nearly touched the next and a street read as a terrace with the gaps left
        // in by accident.
        let air = match self {
            Building::CityBlock | Building::CityTower | Building::CitySpire => 2.2,
            _ => 4.0,
        };
        self.footprint() + Vec2::splat(air)
    }
}

/// One building, placed.
#[derive(Clone, Copy, Debug)]
pub struct Plot {
    /// Where its middle stands.
    pub at: Vec2,
    /// Which part of the town it belongs to.
    ///
    /// RECORDED, not re-derived. `District::divisions` splits a town at the
    /// percentiles of whatever population it is handed, so working the district out
    /// again later from a different list - the thinned plots rather than the lots -
    /// gives a different answer, and buildings end up filed under districts they
    /// were not built for. Towers appeared in the outskirts of a city whose rule
    /// says the outskirts have none.
    pub district: District,
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
    /// The point on this street's middle line closest to `at`.
    pub fn nearest_point(&self, at: Vec2) -> Vec2 {
        let run = self.to - self.from;
        let length2 = run.length_squared().max(1.0e-8);
        self.from + run * ((at - self.from).dot(run) / length2).clamp(0.0, 1.0)
    }

    /// How far a point is from the middle of this street, and how far along it.
    pub fn nearest(&self, at: Vec2) -> (f32, f32) {
        let on = self.nearest_point(at);
        (at.distance(on), on.distance(self.from))
    }
}

/// How much bare ground is kept between a kerb and the nearest wall, in metres.
const KERB_CLEAR: f32 = 0.8;

/// How far in front of a door we look for the street it is supposed to open onto.
const DOOR_LOOKS: f32 = 3.0;

/// How far this building reaches from its middle in one direction.
///
/// # Why buildings kept standing in the road
///
/// The clearance test used `footprint().length() * 0.55` - a little over half the
/// footprint's DIAGONAL - as a stand-in for "how much building is in the way". It is
/// not one. A cottage's diagonal half is 5.86 m, so the test reserved 3.22 m while
/// the building's own corner reaches 5.86: a cottage cleared to sit 6.22 m from a
/// centreline put its corner 0.36 m from it, which is inside a road that is 3 m to
/// the kerb. Against the street a lot was CUT from that never showed, because the
/// door face is the shallow side; against a street crossing behind or beside it, it
/// showed every time.
///
/// This is the box's exact support function instead: project the two half-extents
/// onto the direction being asked about. Against its own street it returns the half
/// DEPTH, which is what is actually pointing that way, so a properly set-back
/// building still passes; against a street off its flank it returns the half WIDTH,
/// which is what that street is really up against.
fn reach_toward(what: Building, facing: f32, toward: Vec2) -> f32 {
    let half = what.footprint() * 0.5;
    let (sin, cos) = facing.sin_cos();
    // The axes `Plot::walls` builds on: across the frontage, and out through the door.
    let across = Vec2::new(cos, sin);
    let door = Vec2::new(sin, -cos);
    toward.dot(across).abs() * half.x + toward.dot(door).abs() * half.y
}

/// Whether a building of this size, standing here and facing this way, is off every
/// street - measured against the part of it that actually faces each one.
fn clear_of_streets(streets: &[Street], at: Vec2, facing: f32, what: Building) -> bool {
    streets.iter().all(|street| {
        let on = street.nearest_point(at);
        let away = at.distance(on);
        if away < 1.0e-3 {
            return false;
        }
        away > street.wide * 0.5 + reach_toward(what, facing, (on - at) / away) + KERB_CLEAR
    })
}

/// How much air is left between two buildings, in metres.
///
/// Not nought. A footprint is what a building keeps clear on the ground and its
/// ROOF is bigger - eaves overhang by a third of a metre and a porch further - so
/// two buildings whose footprints merely touch have their gutters through each
/// other.
const ELBOW: f32 = 1.0;

/// Whether a building standing here would stand in one already standing.
///
/// # Nothing checked this. At all.
///
/// Every placement in this file tested a building against the ROADS - twice over,
/// carefully, with an exact support function - and nothing ever asked whether the
/// spot was already occupied. Lots rarely collide because the subdivision hands out
/// disjoint ones, so it held up by construction and never by rule.
///
/// The guild hall broke it because the guild hall is placed OFF the lot grid: it is
/// walked round the square looking for a gap between the radials, and a gap between
/// two roads is not the same thing as an empty one. It landed on a townhouse - the
/// two of them interpenetrating, one roof through the other's wall.
///
/// This is the separating axis theorem on the only axes a pair of rectangles can be
/// separated along - the four face normals - reusing `reach_toward` as the support
/// function, which is the same one the road test measures with.
fn clear_of_buildings(plots: &[Plot], at: Vec2, facing: f32, what: Building) -> bool {
    let (sin, cos) = facing.sin_cos();
    plots.iter().all(|plot| {
        let between = plot.at - at;
        let (theirs, theirc) = plot.facing.sin_cos();
        [
            Vec2::new(cos, sin),
            Vec2::new(sin, -cos),
            Vec2::new(theirc, theirs),
            Vec2::new(theirs, -theirc),
        ]
        .iter()
        .any(|axis| {
            between.dot(*axis).abs()
                > reach_toward(what, facing, *axis)
                    + reach_toward(plot.what, plot.facing, *axis)
                    + ELBOW
        })
    })
}

/// Whether the door on this spot opens onto a street rather than away from one.
///
/// A lot inherits the facing of the street it was cut from, so in principle every
/// door already addresses one. In practice a lot can be cut against one street and
/// end up nearer another - the ring it fronts curves away, a radial crosses behind
/// it - and then the door is the far side of the building from the road anybody
/// walks up. Asked directly rather than assumed: step out of the door, step out of
/// the back wall, and the door had better be the end that finds a street first.
fn door_faces_a_street(streets: &[Street], at: Vec2, facing: f32, what: Building) -> bool {
    if streets.is_empty() {
        return true;
    }
    let door = Vec2::new(facing.sin(), -facing.cos());
    let out = what.footprint().y * 0.5 + DOOR_LOOKS;
    let nearest =
        |p: Vec2| streets.iter().map(|s| s.nearest(p).0).fold(f32::MAX, f32::min);
    nearest(at + door * out) < nearest(at - door * out)
}

/// One ROAD, as the line it actually runs along.
///
/// # Why a road is a chain and not a bag of segments
///
/// A ring road was built as a few dozen short straight pieces and drawn as a few
/// dozen separate rectangles, each square across its own direction. On a curve a
/// rectangle's outer edge is shorter than the arc it stands for and its inner edge
/// is longer, so consecutive pieces gap on the outside and overlap on the inside -
/// a sawtooth of triangular bites out of the kerb the whole way round.
///
/// It cannot be fixed while drawing, because at that point the pieces have already
/// forgotten they were one road: the only way to find a piece's neighbour is to
/// search for another piece that happens to share an endpoint, and where a ring
/// meets a radial that search finds two. I tried it and got a starburst of spikes.
///
/// So the chain is what the layout holds and the segments are DERIVED from it.
/// Everything that wants segments - frontage, clearance, junctions, lamps - still
/// gets them, and the one thing that needs to know where a road bends now does.
#[derive(Clone, Debug)]
pub struct Way {
    pub points: Vec<Vec2>,
    pub wide: f32,
    /// The width this road converges to where it becomes a city street.
    ///
    /// A city's own ways join themselves and never change. A country road joins the
    /// high street it arrives on - see `RoadSection`, and the note there about why a
    /// road cannot simply divide its existing width into footways.
    pub joins: f32,
}

impl Way {
    /// The straight pieces this road is made of.
    pub fn segments(&self) -> impl Iterator<Item = Street> + '_ {
        self.points.windows(2).map(|pair| Street {
            from: pair[0],
            to: pair[1],
            wide: self.wide,
        })
    }

    /// The way the ribbon lies across the road at each of its points, and how much
    /// the cross-section has to stretch there to keep the road's width.
    ///
    /// At a bend both pieces use ONE cross-section, bisecting the turn - which is
    /// what makes their quads share an edge exactly instead of gapping. The stretch
    /// is `1 / cos(half the turn)`, capped: on a hairpin that factor runs away and
    /// throws the kerb into the next county, which is exactly how the first attempt
    /// at this produced spikes.
    fn across(&self) -> Vec<(Vec2, f32)> {
        let ways: Vec<Vec2> = self
            .points
            .windows(2)
            .map(|pair| (pair[1] - pair[0]).normalize_or(Vec2::X))
            .collect();
        if ways.is_empty() {
            return Vec::new();
        }
        (0..self.points.len())
            .map(|at| {
                let before = (at > 0).then(|| ways[at - 1]);
                let after = ways.get(at).copied();
                let (bisect, square) = match (before, after) {
                    (Some(a), Some(b)) => ((a.perp() + b.perp()).normalize_or(a.perp()), b.perp()),
                    (Some(a), None) => (a.perp(), a.perp()),
                    (None, Some(b)) => (b.perp(), b.perp()),
                    (None, None) => (Vec2::Y, Vec2::Y),
                };
                let lean = bisect.dot(square).abs();
                (bisect, if lean > 0.45 { 1.0 / lean } else { 1.0 / 0.45 })
            })
            .collect()
    }
}

/// A lamp standing at a kerb.
#[derive(Clone, Copy, Debug)]
pub struct Lamp {
    pub at: Vec2,
    /// Which way it is turned. A city lamp's arm reaches out over the carriageway,
    /// so this points at the road; a village post is symmetrical and does not care.
    pub turn: f32,
    /// How high its light hangs - the two fittings are different, and a point light
    /// guessed at the wrong height reads as a glow beside the lamp.
    pub head: f32,
}

/// Everything laid out for one settlement.
#[derive(Clone, Debug, Default)]
pub struct Layout {
    /// The roads as they run. What gets DRAWN.
    pub ways: Vec<Way>,
    /// The same roads cut into straight pieces, which is what every geometric
    /// question about them wants. Derived from `ways`, never built beside it.
    pub streets: Vec<Street>,
    pub plots: Vec<Plot>,
    pub lamps: Vec<Lamp>,
}

/// How far apart lamps stand along a street, in metres.
///
/// Close enough that the pools of light nearly meet, which is what makes a lit
/// street read as a street rather than as a row of separate lamps. A city lights
/// more tightly than a village: a village lamp is somebody's lantern outside their
/// own door and there are gaps between them on purpose.
const LAMPS_EVERY_IN_A_CITY: f32 = 26.0;
const LAMPS_EVERY_IN_A_VILLAGE: f32 = 38.0;

/// How far out from the kerb a lamp stands, in metres.
const LAMPS_OFF_THE_KERB: f32 = 1.1;

/// Where the light hangs on each fitting, in metres. The contract with
/// `dev/art/lamp.py` - see `the_lamp_models_hang_their_light_where_the_game_thinks`.
pub const STREET_HEAD: f32 = 5.6;
pub const POST_HEAD: f32 = 3.1;
/// How far a city fitting's arm reaches out over the carriageway, in metres. The
/// head - and so the light - is on the end of it, not over the column.
pub const STREET_ARM: f32 = 1.5;

/// Stands lamps along a settlement's streets.
///
/// Alternating sides, so a street is lit from both without being lined twice, and
/// stepped along from the street's own start so two streets meeting at a junction do
/// not both put a lamp in the same corner.
fn light_the_streets(streets: &[Street], plots: &[Plot], city: bool) -> Vec<Lamp> {
    let every = if city {
        LAMPS_EVERY_IN_A_CITY
    } else {
        LAMPS_EVERY_IN_A_VILLAGE
    };
    let head = if city { STREET_HEAD } else { POST_HEAD };
    let mut lamps: Vec<Lamp> = Vec::new();

    for street in streets {
        let run = street.to - street.from;
        let length = run.length();
        if length < every * 0.6 {
            continue;
        }
        let along = run / length;
        let side = along.perp();
        let steps = (length / every).floor().max(1.0) as usize;
        for step in 0..=steps {
            // Inset from both ends, so nothing stands in a junction.
            let at_along = (step as f32 + 0.5) * (length / (steps + 1) as f32);
            if at_along > length - 2.0 {
                continue;
            }
            let hand = if step % 2 == 0 { 1.0 } else { -1.0 };
            let at = street.from
                + along * at_along
                + side * hand * (street.wide * 0.5 + LAMPS_OFF_THE_KERB);

            // Not in a building, and not on top of another lamp.
            if plots
                .iter()
                .any(|plot| plot.at.distance(at) < plot.what.footprint().max_element() * 0.5 + 1.0)
            {
                continue;
            }
            if lamps.iter().any(|other| other.at.distance(at) < every * 0.5) {
                continue;
            }
            // The arm reaches over the road, which is back the way we stepped out.
            let toward = -side * hand;
            lamps.push(Lamp {
                at,
                turn: (-toward.y).atan2(toward.x),
                head,
            });
        }
    }
    lamps
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

/// How long a piece of a curved street is before it takes another bearing.
///
/// A ring is drawn as a chain of short straight pieces, and this is how short. Six
/// metres is under the width of the street itself, so the corner between two pieces
/// is shallower than the road is wide and the eye reads a curve rather than a bend.
const A_CURVE_STEPS_EVERY: f32 = 6.0;

/// Lays a ring segment as an ARC rather than as the chord across it.
///
/// # A ring of six straight pieces is a hexagon
///
/// Which is what these were: each stretch of ring road ran straight from one radial
/// to the next, so a town's ring roads were polygons and every junction was a
/// corner. Reported as wanting "curves instead of straight edges", and from above it
/// is unmistakable.
///
/// The arc is still straight pieces - everything downstream wants segments, and the
/// paving is built from them - but enough of them, short enough, that the corner
/// between any two is far shallower than the road is wide.
fn arc_streets(
    ways: &mut Vec<Way>,
    parcels: &mut Vec<Parcel>,
    middle: Vec2,
    from: Vec2,
    to: Vec2,
    wide: f32,
    depth: f32,
    ring: bool,
) {
    let (a, b) = (from - middle, to - middle);
    let (start, end) = (a.to_angle(), b.to_angle());
    // The short way round, always: the long way would draw the rest of the ring.
    let mut sweep = end - start;
    while sweep > std::f32::consts::PI {
        sweep -= std::f32::consts::TAU;
    }
    while sweep < -std::f32::consts::PI {
        sweep += std::f32::consts::TAU;
    }

    let radius = (a.length() + b.length()) * 0.5;
    let along = (sweep.abs() * radius).max(1.0);
    let steps = (along / A_CURVE_STEPS_EVERY).ceil().max(1.0) as usize;

    // The FRONTAGE follows the arc too, in stretches of a few pieces.
    //
    // It did not, at first: the parcels were still laid against the chord while the
    // road bowed away from it, so a building addressed a straight line that was no
    // longer there and `every_building_faces_a_street` went red immediately. On a
    // 17 m square ring that gap is 2.3 m - wider than the pavement - so the houses
    // were standing in the road.
    //
    // Grouped rather than one parcel per piece: a six-metre piece has no frontage
    // worth the name, and three of them is eighteen, which fits a house.
    const PIECES_TO_A_PARCEL: usize = 3;
    let mut parcel_from = from;

    let mut last = from;
    // ONE road, kept as the line it runs along - see `Way`.
    let mut line = vec![from];
    for step in 1..=steps {
        let part = step as f32 / steps as f32;
        // The radius eases between the two ends, so a ring that wobbles from one
        // spoke to the next still wobbles - it just does it along a curve.
        let here = a.length() + (b.length() - a.length()) * part;
        let turn = start + sweep * part;
        let next = if step == steps {
            to
        } else {
            middle + Vec2::from_angle(turn) * here
        };
        line.push(next);
        if step % PIECES_TO_A_PARCEL == 0 || step == steps {
            frontage_parcels(parcels, middle, parcel_from, next, wide, depth, ring);
            parcel_from = next;
        }
        last = next;
    }
    ways.push(Way { points: line, wide, joins: wide });
}

/// Where a town's streets actually MEET, whatever plan drew them.
///
/// # Placement has to read the network, not the plan that made it
///
/// The hall and the landmarks were placed by walking the radial plan's own geometry
/// - the square's edge, the spokes, "lanes as opposed to the high street". None of
/// those words mean anything on a grid, where every street runs through, or on a
/// street village, which has no square at all. Every attempt to add a second plan
/// therefore broke placement in four different ways at once.
///
/// These three helpers ask the STREETS instead. Any plan that produces streets gets
/// a hall, gets its landmarks, and keeps them out of the road.
fn junctions_of(streets: &[Street]) -> Vec<(Vec2, f32)> {
    let mut found: Vec<(Vec2, f32, usize)> = Vec::new();
    for street in streets {
        if (street.to - street.from).length() < 1.0 {
            continue;
        }
        for end in [street.from, street.to] {
            match found.iter_mut().find(|(at, _, _)| at.distance(end) < 1.2) {
                Some((_, wide, count)) => {
                    *wide = wide.max(street.wide);
                    *count += 1;
                }
                None => found.push((end, street.wide, 1)),
            }
        }
    }
    // Three or more ends is a junction. Two is a bend in one road, and one is where
    // a lane stops - neither is a place anybody gathers.
    found
        .into_iter()
        .filter(|(_, _, count)| *count >= 3)
        .map(|(at, wide, _)| (at, wide))
        .collect()
}

/// The nearest spot to `about` with room for something `half` wide, off every street
/// and clear of everything already placed. `None` if the town is too full.
fn open_ground(
    streets: &[Street],
    plots: &[Plot],
    about: Vec2,
    what: Building,
    search: f32,
) -> Option<Vec2> {
    let clear = |at: Vec2| {
        // THE ROADS, MEASURED THE SAME WAY AS THE BUILDINGS.
        //
        // This kept its own circle - `max_element * 0.5` - after the building check
        // beside it became exact, which Codex caught: a rectangle's corner reaches
        // further than half its longer side, so at an oblique angle the circle can
        // clear a spot whose corner is in the carriageway. Every landmark that goes
        // through here is square or nearly so, which is why nothing has shown it, but
        // there is no reason for this one call to keep the approximation when the
        // exact test is one line away and is what every other placement uses.
        clear_of_streets(streets, at, 0.0, what)
            // A THIRD CIRCLE ROUND A RECTANGLE, now gone the same way as the other
            // two. This one measured the standing building at `max_element * 0.5`,
            // which for the guild hall is 13 m - and the hall's own corner reaches
            // 15.8. So a monument could be cleared at 17 m from the middle of a hall
            // whose corner was 15.8 m out, and stand inside it. That is exactly what
            // the sweep test caught, in a village nobody had photographed.
            //
            // A landmark is near enough square that which way it faces does not
            // change what it takes up, so it is asked about at nought.
            && clear_of_buildings(plots, at, 0.0, what)
    };
    if clear(about) {
        return Some(about);
    }
    // Outward in rings, so the answer is always the nearest one there is.
    for step in 1..=8 {
        let out = search * step as f32 / 8.0;
        let around = 8 + step * 2;
        for turn in 0..around {
            let at = about
                + Vec2::from_angle(turn as f32 / around as f32 * std::f32::consts::TAU) * out;
            if clear(at) {
                return Some(at);
            }
        }
    }
    None
}

/// The lot nearest `about` that can hold something of this size, if there is one.
///
/// Returns the INDEX, so the caller can take the lot's own facing with it - which is
/// the whole point: a building put on a lot inherits the frontage that lot was cut
/// against, and therefore faces the street it was cut from.
fn lot_that_fits(plots: &[Plot], about: Vec2, what: Building) -> Option<usize> {
    let wants = what.wants();
    let mut best: Option<(usize, f32)> = None;
    for (index, plot) in plots.iter().enumerate() {
        if plot.what.is_landmark() {
            continue;
        }
        // The lot it stands on has to be big enough, or the new building overhangs
        // the street its facing was measured against.
        let room = plot.what.footprint();
        if room.x + 5.0 < wants.x || room.y + 5.0 < wants.y {
            continue;
        }
        let out = plot.at.distance(about);
        if best.is_none_or(|(_, was)| out < was) {
            best = Some((index, out));
        }
    }
    best.map(|(index, _)| index)
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

    // HOW WIDE THIS PLACE'S STREETS ARE.
    //
    // A city's are wider than a village's by exactly the two footways they carry -
    // see `CITY_STREET_WIDE`. Decided once, here, and handed to everything that lays
    // a road, so the width a street is drawn at, the width the warden walks, and the
    // width the buildings are set back from are one number.
    let (high_street, lane) = if site.city {
        (CITY_STREET_WIDE, CITY_LANE_WIDE)
    } else {
        (STREET_WIDE, LANE_WIDE)
    };

    // The roads as CHAINS. `streets` is derived from these once they are all laid.
    let mut ways: Vec<Way> = Vec::new();
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
        arc_streets(&mut ways, &mut parcels, site.at, from, to, high_street, depth, true);
    }

    // The radials, each running from the square out through every ring it crosses.
    for (index, spoke) in spokes.iter().enumerate() {
        let out = Vec2::from_angle(*spoke);
        let last = spoke_reaches(index);
        let wide = if angle_between(*spoke, through) < 0.1
            || angle_between(*spoke, through + std::f32::consts::PI) < 0.1
        {
            high_street
        } else {
            lane
        };
        // A radial is a chain of two points, which is what a straight road is.
        ways.push(Way {
            points: vec![
                site.at + out * square,
                site.at + out * ring_r(index, last),
            ],
            wide,
            joins: wide,
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
            arc_streets(&mut ways, &mut parcels, site.at, from, to, lane, depth, false);
        }
    }

    // The roads are all laid. Everything from here asks geometric questions of them
    // - where a lot fronts, what a building clears, where roads meet, where a lamp
    // stands - and every one of those wants straight pieces, so the pieces are cut
    // from the chains ONCE, here, rather than being built alongside them.
    let streets: Vec<Street> = ways.iter().flat_map(|way| way.segments()).collect();

    // THE GUILD HALL TAKES THE SQUARE, which is where a guild hall goes: the search
    // below walks the square's edge for a spot clear of every radial mouth.
    //
    // # Every settlement has one
    //
    // This was `site.city` and had been for the life of the feature, so the guild
    // whose name the game carries had a hall in the four cities and nowhere else -
    // nine villages with no branch to register a companion at, and nothing in them
    // saying whose world you were walking through. Nobody noticed because a village
    // still looked like a village: what was missing was a building nobody had drawn
    // yet, and the placement was written to match what existed rather than what the
    // world needed.
    //
    // The ranch is not a settlement and is skipped everywhere else too - see `Site`.
    let mut civic: Option<Plot> = None;
    if !site.ranch {
        let hall = Building::GuildHall;
        let stand = square + STREET_WIDE * 0.5 + SETBACK + hall.footprint().y * 0.5;
        // Walked outward as well as around.
        //
        // A guild hall is 18 m across, so standing clear of a radial's kerb wants
        // nearly 13 m from its middle line - more than half the gap between two
        // radials where they leave a square, which is exactly where the old search
        // looked and only there. It found nothing and the city got no guild hall.
        // One ring further out the same gap is wider, because the radials diverge.
        'find: for out in 0..6 {
            let stand = stand + out as f32 * hall.footprint().x * 0.5;
        for step in 0..48 {
            let turn = through + std::f32::consts::TAU * step as f32 / 48.0;
            let at = site.at + Vec2::from_angle(turn) * stand;
            if at.distance(site.at) > reach {
                continue;
            }
            // FACING THE NEAREST STREET, not the square.
            //
            // It used to face back at the square unconditionally, which reads well
            // only while it is ON the square. Pushed a ring outward to find room, a
            // hall facing inward addresses an empty green with its back to the high
            // street - "entrances need to face a road". So the street it stands
            // nearest chooses which way it looks, and the square keeps it only when
            // the square is what it fronts.
            let Some(on) = streets
                .iter()
                .map(|street| street.nearest_point(at))
                .min_by(|a, b| {
                    a.distance(at)
                        .partial_cmp(&b.distance(at))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
            else {
                continue;
            };
            let door = (on - at).normalize_or_zero();
            if door == Vec2::ZERO {
                continue;
            }
            let facing = door.x.atan2(-door.y);
            if !clear_of_streets(&streets, at, facing, hall)
                || !door_faces_a_street(&streets, at, facing, hall)
            {
                continue;
            }
            civic = Some(Plot {
                at,
                facing,
                what: hall,
                // On the square, whatever the percentiles would have said.
                district: District::Market,
            });
            break 'find;
        }
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
        let what = what_stands_here(index, lot, site.at, inner, outer, site.city, seed);
        let Some(what) = what else { continue };

        // Placed against the street rather than in the middle of its lot.
        let door = lot.door();
        let front = lot.at + door * (lot.depth * 0.5);
        let at = front - door * (what.footprint().y * 0.5 + 0.35);
        if at.distance(site.at) > reach {
            continue;
        }

        // OFF EVERY ROAD, and OPENING ONTO ONE.
        //
        // Slid along its own frontage rather than dropped. A lot cut near the end of
        // a parcel sits close to whatever street crosses there, so testing properly
        // and giving up thinned a city from thirty-odd buildings to seventeen - the
        // rule was right and the response to it was wrong. Sliding keeps the
        // building on the frontage it was cut from, keeps the facing that frontage
        // gave it, and just moves it clear of the crossing: which is what a surveyor
        // does with a corner plot.
        let (sin, cos) = lot.facing.sin_cos();
        let across = Vec2::new(cos, sin);
        let room = (lot.frontage - what.footprint().x) * 0.5;
        let mut stood = None;
        for step in 0..=8 {
            for side in [1.0_f32, -1.0] {
                let shift = side * room * step as f32 / 8.0;
                let try_at = at + across * shift;
                if clear_of_streets(&streets, try_at, lot.facing, what)
                    && door_faces_a_street(&streets, try_at, lot.facing, what)
                    && clear_of_buildings(&plots, try_at, lot.facing, what)
                {
                    stood = Some(try_at);
                    break;
                }
                if step == 0 {
                    break;
                }
            }
            if stood.is_some() {
                break;
            }
        }
        let Some(at) = stood else { continue };
        // THE HALF-DIAGONAL CIRCLE, AGAIN.
        //
        // This was `(bulk + theirs) * 0.62` on half-diagonals - two circles drawn
        // round two rectangles and then scaled down by a fudge until towns stopped
        // looking thin. It is the same approximation, with the same fudge, that put
        // buildings in roads until `reach_toward` replaced it there; the roads got
        // the exact answer and the buildings kept the guess.
        //
        // A circle round a rectangle is wrong in both directions at once: too big
        // along the axes, so it thins a street that would have fit, and too small at
        // the corners even before a 0.62 is applied to it.
        if !clear_of_buildings(&plots, at, lot.facing, what) {
            continue;
        }
        plots.push(Plot {
            at,
            facing: lot.facing,
            what,
            district: District::of(at.distance(site.at), inner, outer),
        });
    }

    // LANDMARKS, before the thinning, because they are not houses and must not be
    // thinned away.
    //
    // Rogers' hub-town rules, applied: a landmark stands ON a node - the square, and
    // the junctions where the ring roads meet the radials - and it is a different
    // KIND of thing from the buildings around it, so it reads as somewhere to gather
    // rather than as a bigger house. They take no lot and keep no frontage.
    let (on_the_square, at_a_junction) = Building::landmarks(site.city);

    // ON the middle, but never in the ROAD.
    //
    // Placed at `site.at` outright before, which is safe only because the radial
    // plan keeps its middle open - that is what a market square IS. Asked of the
    // network instead, so a plan that runs a street through its middle gets its
    // landmark beside that street rather than under it.
    if let Some(at) = open_ground(&streets, &plots, site.at, on_the_square, square * 1.2) {
        plots.push(Plot {
            at,
            facing: approach.y.atan2(approach.x),
            what: on_the_square,
            district: District::Market,
        });
    }

    // And one at each of the town's real JUNCTIONS - the places three or more
    // streets meet, which is Rogers' node whatever plan drew them. Spread out, or
    // they stop being landmarks and become street furniture.
    let junction_half = at_a_junction.footprint().max_element() * 0.5;
    let mut placed: Vec<Vec2> = plots
        .iter()
        .filter(|p| p.what.is_landmark())
        .map(|p| p.at)
        .collect();
    let most_marks = if site.city { 5 } else { 3 };

    let mut meeting = junctions_of(&streets);
    // Furthest from the middle first, so a town's landmarks reach its edges rather
    // than crowding the one junction nearest the square.
    meeting.sort_by(|a, b| {
        b.0.distance(site.at)
            .partial_cmp(&a.0.distance(site.at))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    for (at, wide) in meeting {
        if placed.len() > most_marks {
            break;
        }
        // Far enough out to be a junction rather than the square itself. The square
        // RING's own junctions sit at exactly `square`, and excluding anything
        // inside 1.1 of that threw away every main node a village has - which is
        // how a village came to have one landmark and nothing to navigate by.
        if at.distance(site.at) < square * 0.7 {
            continue;
        }
        if placed.iter().any(|other| other.distance(at) < square * 0.9) {
            continue;
        }
        let Some(spot) = open_ground(
            &streets,
            &plots,
            at + (at - site.at).normalize_or_zero() * (wide * 0.5 + junction_half + 1.2),
            at_a_junction,
            wide * 2.0 + 6.0,
        ) else {
            continue;
        };
        if placed.iter().any(|other| other.distance(spot) < square * 0.9) {
            continue;
        }
        placed.push(spot);
        let facing = (spot - site.at).normalize_or_zero();
        plots.push(Plot {
            at: spot,
            facing: facing.y.atan2(facing.x),
            what: at_a_junction,
            district: District::Crafts,
        });
    }

    // THE WEENIE, AND IT IS THE GUILD HALL.
    //
    // Rogers' first hub rule: a place needs one thing tall enough to see from
    // OUTSIDE it that pulls you toward the centre. A city had one - but it was the
    // office spire, put nearest the middle by exactly this rule, and the guild hall
    // was left to find a lot like any other building.
    //
    // Photographed from a city entrance that read as a row of near-identical slabs,
    // and the numbers said why: blocks 19.7 m, towers 37.6 m, spire 57.1 m, guild
    // hall 14.1 m. The shortest thing on the street was the one building the whole
    // game is named after, so the hall was rebuilt as an 80.5 m campanile and took
    // the middle.
    //
    // # And then it stopped being tall, and this text did not
    //
    // The hall is built to a concept sheet now and it is 12.7 m - a town branch, not
    // a cathedral. Everything below still described it as the 80.5 m thing you see a
    // city by, which is how a comment outlives the decision it records.
    //
    // So the two jobs are separated, because they were only ever conflated by the
    // hall happening to do both. The SKYLINE landmark is the spire again, which is
    // what `Building::weenie(true)` has said all along. What the hall keeps is its
    // SQUARE: `KEEPS_CLEAR` around it is negative space at street level, so the
    // building a warden is looking for is read against sky from the approach road
    // rather than against the flank of a tower. That is public-space composition and
    // it is worth keeping at 12.7 m; it is not skyline protection and must not be
    // read as any.
    // And the fallback, for a settlement whose square had nowhere the hall would
    // stand: it takes an ordinary lot instead of going without.
    if !site.ranch && !plots.iter().any(|p| p.what == Building::GuildHall) {
        if let Some(index) = lot_that_fits(&plots, site.at, Building::GuildHall) {
            plots[index].what = Building::GuildHall;
        }
    }

    if site.city {
        // THE SPIRE IS THE THING YOU SEE THE CITY BY. Across the middle rather than
        // at it, and to one side of the way in, so the hall on the square and the
        // spire on the skyline are two separate sightings rather than one behind the
        // other from the entrance road.
        let aside = site.at + Vec2::new(-approach.y, approach.x) * reach * 0.72;
        if let Some(index) = lot_that_fits(&plots, aside, Building::CitySpire) {
            if plots[index].at.distance(site.at) > square + KEEPS_CLEAR * 0.5 {
                plots[index].what = Building::CitySpire;
            }
        }

        // THE HALL'S SQUARE. Rogers' other half, and the half a height contest
        // misses: a landmark needs room around it. At 80.5 m that meant nothing
        // should out-top it; at 12.7 m it means nothing should stand over it at the
        // moment you arrive, which is the same rule doing an honest job at a
        // believable size. Anything tall too close is built lower.
        if let Some(hall) = plots
            .iter()
            .position(|plot| plot.what == Building::GuildHall)
        {
            let seat = plots[hall].at;
            for plot in plots.iter_mut() {
                let tall = matches!(plot.what, Building::CityTower | Building::CitySpire);
                if tall && plot.at.distance(seat) < KEEPS_CLEAR {
                    plot.what = Building::CityBlock;
                }
            }
        }
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
        // The hall and every landmark survive whatever the size: a city without its
        // guild hall is not a city, and a node without its landmark is a junction.
        let keep_always: Vec<usize> = (0..plots.len())
            .filter(|i| {
                plots[*i].what == Building::GuildHall
                    || plots[*i].what.is_landmark()
                    || plots[*i].what == Building::CitySpire
            })
            .collect();
        for at in &keep_always {
            kept.push(plots[*at]);
        }
        let others: Vec<usize> = (0..plots.len())
            .filter(|i| !keep_always.contains(i))
            .collect();
        // Which lots a building took, so the rest can be given a use below.
        let mut taken: Vec<usize> = Vec::new();
        let room = wanted.saturating_sub(kept.len()).max(1);
        for district in [District::Market, District::Crafts, District::Outskirts] {
            let here: Vec<usize> = others
                .iter()
                .copied()
                .filter(|i| plots[*i].district == district)
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
                    taken.push(*index);
                }
            }
        }

        // AND THE LOTS THAT DID NOT GET A BUILDING BECOME YARDS.
        //
        // # A town is dense when its frontage is occupied
        //
        // These used to be dropped. That was right while a town stood on meadow and
        // wrong the moment its ground became packed earth: photographed from the
        // middle of a village, half the frontage was bare dirt, and a city could hold
        // thirty-four buildings and still read as empty because each stood alone in a
        // tan field.
        //
        // The answer is not more houses - the counts are what a place of this kind
        // HAS. A fence, a row of beans, a lean-to and a stack of timber occupy a
        // street edge as surely as a wall does, at a fraction of the geometry, and
        // they say the thing a wall does not: somebody lives here and does something
        // all day.
        //
        // TO A BUDGET, and the budget is the buildings.
        //
        // The first cut turned about seven in ten of every discarded lot into a yard.
        // That makes the size of a settlement depend on how many provisional lots the
        // street generator happened to produce rather than on what the place IS: a
        // sixteen-house village came out with forty-eight yards and a thirty-four
        // building city with ninety-four, and a change upstream that yielded more
        // candidate lots would have multiplied both without anything saying so.
        //
        // How much frontage a district occupies is a property OF the district, which
        // one global share cannot express - a market street is meant to be nearly
        // solid and an outskirt is meant to break into gardens and air. So each
        // district gets a ratio against its own retained buildings, and takes that
        // many by stride around the ring rather than a clump off the front.
        let built: Vec<Plot> = kept.clone();
        for district in [District::Market, District::Crafts, District::Outskirts] {
            let free: Vec<usize> = (0..plots.len())
                .filter(|i| {
                    !taken.contains(i)
                        && !keep_always.contains(i)
                        && plots[*i].district == district
                })
                .collect();
            if free.is_empty() {
                continue;
            }
            let houses = built
                .iter()
                .filter(|plot| plot.district == district && !plot.what.is_yard())
                .count();
            let want = ((houses as f32 * district.occupies()).round() as usize).min(free.len());
            if want == 0 {
                continue;
            }
            let stride = (free.len() as f32 / want as f32).max(1.0);
            for step in 0..want {
                let at = (step as f32 * stride).round() as usize;
                let Some(index) = free.get(at) else { continue };
                let mut yard = plots[*index];
                // Hashed from where the lot IS, so a change to one lot cannot move
                // the programme of any other.
                let roll = (unit(
                    seed.wrapping_add(yard.at.x.to_bits() ^ yard.at.y.to_bits()),
                    97,
                ) * 1_000.0) as u32;
                // Whose yard it is: the nearest building, if one is near enough to
                // own it. `BELONGS_WITHIN` is about two lots - beyond that a yard is
                // its own thing standing on the street rather than somebody's back
                // garden, and the district decides.
                let beside = built
                    .iter()
                    .filter(|plot| !plot.what.is_yard())
                    .map(|plot| (plot.at.distance(yard.at), plot.what))
                    .filter(|(away, _)| *away < BELONGS_WITHIN)
                    .min_by(|a, b| a.0.total_cmp(&b.0))
                    .map(|(_, what)| what);
                yard.what = Building::yard_for(yard.district, site.city, beside, roll);
                kept.push(yard);
            }
        }
        plots = kept;
    }

    let lamps = light_the_streets(&streets, &plots, site.city);
    Layout {
        ways,
        streets,
        plots,
        lamps,
    }
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
    city: bool,
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
    let wanted = District::of(lot.at.distance(middle), inner, outer).builds(roll, city);
    if fits(wanted) {
        Some(wanted)
    } else if fits(if city { Building::CityBlock } else { Building::Cottage }) {
        Some(if city { Building::CityBlock } else { Building::Cottage })
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

pub fn unit(seed: u32, salt: u32) -> f32 {
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
    pub standing: std::collections::HashMap<u32, Layout>,
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
    pub fn walls_near(&self, at: Vec2, reach: f32, walls: &mut Vec<(Vec2, Vec2, f32)>) {
        walls.clear();
        for layout in self.standing.values() {
            for plot in &layout.plots {
                // Squared, because this only asks whether the plot is in reach and
                // the answer never needs the distance itself.
                let far = reach + plot.what.footprint().length();
                if plot.at.distance_squared(at) > far * far {
                    continue;
                }
                plot.walls_into(walls);
            }
        }
    }
}

/// What `dev/art/town.py` measured off the buildings it built.
///
/// Compiled in: a few kilobytes, wanted before the first frame, and `include_str!`
/// makes cargo rebuild when it changes.
pub(crate) const TOWN_CONTRACT: &str = include_str!("../../assets/models/town.txt");

/// The floor inside a building, and the step up to it.
pub struct Floor {
    /// How high the boards are above the ground the building stands on.
    pub top: f32,
    /// How far the step out front reaches, and how wide it is.
    pub reach: f32,
    pub wide: f32,
}

/// Every figure's floor, keyed by the name it is built under.
///
/// # The game thought every floor was the ground
///
/// A building's floor is laid on its plinth, and the ground it stands on is the
/// HIGHEST of its four corners - so on any slope the boards are well clear of the
/// earth beside them. The warden stood at terrain height regardless and sank into
/// them, which on a hillside is most of a shin.
///
/// Measured rather than assumed, for the same reason the windows are: taking the
/// highest interior slab reported a townhouse's floor at 3.7 m, which as a walking
/// surface would have put the warden on its roof.
pub static FLOORS: std::sync::LazyLock<std::collections::HashMap<&'static str, Floor>> =
    std::sync::LazyLock::new(|| {
        let mut found = std::collections::HashMap::new();
        for line in TOWN_CONTRACT.lines() {
            let Some(rest) = line.strip_prefix("FLOOR ") else {
                continue;
            };
            let mut word = rest.split_whitespace();
            let Some(figure) = word.next() else {
                continue;
            };
            let said: Vec<f32> = word.filter_map(|number| number.parse().ok()).collect();
            let [top, reach, wide] = said[..] else {
                continue;
            };
            found.insert(figure, Floor { top, reach, wide });
        }
        found
    });

/// What `dev/art/yard.py` measured off the yards it built: the largest hole in each
/// side and how long that side is, front, back, left, right.
const YARD_CONTRACT: &str = include_str!("../../assets/models/yard.txt");

/// How much of a side has to be missing before there is no run there at all.
const OPEN_SIDE: f32 = 0.7;

/// Every yard's fence, keyed by the name it is built under.
///
/// # The game had four runs and the model had three
///
/// `fenced` answered with a gate width and nothing else, so every fenced yard was
/// taken to be closed on all four sides. The city's service bay is closed on three -
/// it is a loading bay, and you drive into it - and the game fenced its open mouth
/// anyway, leaving the player walking into nothing across a frontage they could see
/// straight through.
///
/// The old-world gate widths were wrong too, by the width of a post: 3.06 is the
/// spacing of the gateposts and 2.92 is the hole between them, which is the number a
/// warden has to fit through.
///
/// Found by Codex, who also said not to make the two copies agree without deciding
/// which was intended. The open bay is the user's decision; this is what stops there
/// being two copies to disagree.
static FENCES: std::sync::LazyLock<std::collections::HashMap<&'static str, [(f32, f32); 4]>> =
    std::sync::LazyLock::new(|| {
        let mut found = std::collections::HashMap::new();
        for line in YARD_CONTRACT.lines() {
            let Some(rest) = line.strip_prefix("FENCE ") else {
                continue;
            };
            let mut word = rest.split_whitespace();
            let Some(figure) = word.next() else {
                continue;
            };
            let said: Vec<f32> = word.filter_map(|number| number.parse().ok()).collect();
            let [fg, fs, bg, bs, lg, ls, rg, rs] = said[..] else {
                continue;
            };
            found.insert(figure, [(fg, fs), (bg, bs), (lg, ls), (rg, rs)]);
        }
        found
    });

/// How a yard is closed in, when it is closed in at all.
///
/// See `Building::fenced`.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Fenced {
    /// Both flanks and the back, and nothing across the front: a loading bay you
    /// walk or drive straight into.
    OpenFronted,
    /// All four runs, with a gateway this wide in the front one.
    Gated(f32),
}

/// How high a warden stands here: the ground, or whatever has been laid over it.
///
/// # Everything you walk on is drawn above the ground
///
/// The terrain is the floor of this game and almost nothing you actually walk on IS
/// the terrain. A road is laid a crown's height over it so it does not z-fight with
/// the ground it follows; a building's floor sits on a plinth over the highest of
/// its four corners. The warden stood at terrain height through all of it, so the
/// feet sank into every path and most of a shin into every floor.
///
/// `Terrain::walk_height` already told this story for bridges - the deck answers
/// instead of the lake bed - and this is the rest of it. It stays out of `Terrain`
/// because what is BUILT is not the terrain's business: the streets and the plots
/// live in `Built`, which is raised and taken down as the player moves.
///
/// Reported as feet clipping into the path, and into the floor indoors.
pub fn stands_on(
    terrain: &crate::world::terrain::Terrain,
    built: &Built,
    at: Vec2,
) -> f32 {
    // ONE GROUND, for the whole function.
    //
    // This started from `walk_height` - which is built on `Terrain::height` - and then
    // measured every road from `drawn_height`, which is the height the MESH is built
    // at. The two are the same on a settlement's levelled ground and they are not the
    // same in the wild: at the canyon's foot they differ by 53 cm.
    //
    // That did not matter while only town streets were in here, because towns stand on
    // flat ground by construction. It mattered the moment the country roads came in:
    // a stretch of road beside a canyon wall raised the floor half a metre, and half a
    // metre off a two-and-a-half metre climb is the difference between a wall that
    // gates and a wall you stroll up. `a_canyon_wall_refuses_the_step_up` caught it in
    // one run.
    //
    // So a road adds its own LIFT to the ground the rest of the rule is judged on,
    // rather than substituting a different ground underneath it.
    let mut on = terrain.walk_height(at.x, at.y);
    let ground = on;
    for layout in built.standing.values() {
        for street in &layout.streets {
            // ASKED ON THE MIDDLE LINE, which is where a road's section is decided.
            let centre = street.nearest_point(at);
            let across = at.distance(centre);
            if across > street.wide * 0.5 + SHOULDER_WIDE {
                continue;
            }
            let paved = paved_here(terrain.plan(), centre);
            let cut =
                RoadSection::new(street.wide, street.wide, paved, wander_at(centre, paved));
            if across <= cut.shoulder {
                on = on.max(ground + cut.lift(across));
            }
        }
        for plot in &layout.plots {
            if let Some(floor) = plot.floor_at(terrain, at) {
                on = on.max(floor);
            }
        }
    }

    // AND THE ROADS BETWEEN TOWNS, which belong to no layout.
    //
    // `Built` holds what a settlement raised. The country roads are streamed as their
    // own mesh straight from the settlement plan, so they were never in this loop -
    // and for as long as it has existed the warden has walked the whole road network
    // between towns at terrain height, under a crown nine centimetres over their
    // head. It did not show because nine centimetres is a shoe.
    //
    // It shows now: the last thirty metres of every approach raises a footway with a
    // kerb on it, and feet in the middle of that is not a shoe. Found by Codex while
    // the footways were going in.
    let plan = terrain.plan();
    for road in plan.ways() {
        // The cheap reject first - there are hundreds of these and this runs several
        // times a frame.
        let middle = (road.from + road.to) * 0.5;
        let span = road.from.distance(road.to) * 0.5 + ROAD_REACHES;
        if middle.distance_squared(at) > span * span {
            continue;
        }
        let run = road.to - road.from;
        let along = run.length_squared();
        let part = if along > 1.0e-6 {
            ((at - road.from).dot(run) / along).clamp(0.0, 1.0)
        } else {
            0.0
        };
        // NOT IF IT HAS NO SURFACE HERE. A dirt track is not laid across desert or
        // snow, so until this asked, the warden was lifted by the crown of a road that
        // is not drawn - an invisible ramp over every sand and snow crossing in the
        // network. One predicate, shared with the drawing: `has_a_surface`.
        let Some(made) = has_a_surface(plan, terrain, road) else {
            continue;
        };
        let centre = road.from + run * part;
        let paved = made.max(paved_here(plan, centre));
        let cut = RoadSection::new(
            crate::config::ROAD_WIDE,
            CITY_STREET_WIDE,
            paved,
            wander_at(centre, paved),
        );
        let across = at.distance(centre);
        if across <= cut.shoulder {
            on = on.max(ground + cut.lift(across));
        }
    }
    on
}

/// How far from a country road's middle line its made surface can possibly reach.
///
/// The widest it ever gets is the city street it joins, plus its shoulder. Used only
/// to throw away the roads that are nowhere near before measuring the ones that are.
const ROAD_REACHES: f32 = CITY_STREET_WIDE * 0.5 + SHOULDER_WIDE;

/// One town's worth of buildings, kept so the whole lot can be taken down together.
///
/// Public because `world::lamp` stands its lamps against the same key: the
/// settlements own the lifetime, and a second idea of what is standing is a second
/// thing to get out of step.
#[derive(Component)]
pub struct FromSite(pub u32);

/// How high a road's surface stands over the ground it is laid on.
///
/// `out` is how far across the ribbon the point is, nought down the middle and one
/// at the shoulder. Full lift at the crown falling to almost nothing at the edge, so
/// the ribbon meets the ground at its sides and there is no step to see.
///
/// One function because two things need the answer: the mesh that draws the road,
/// and `stands_on`, which is what stops the warden's feet sinking into it. Written
/// out twice, those drift, and the second one is only ever noticed by somebody
/// looking at their own boots.
pub fn road_lift(out: f32) -> f32 {
    let out = out.clamp(0.0, 1.0);
    ROAD_LIES * (1.0 - out * out) + ROAD_HEM
}

/// How wide a footway is, in metres, and how high its kerb stands.
///
/// A kerb is 100-150 mm in the world and there is no reason to exaggerate it: what
/// makes a footway read is the LINE down each side of the carriageway and the change
/// of surface across it, not the height of the step.
pub const FOOTWAY_WIDE: f32 = 2.0;

/// The narrowest the outer band of a road is ever drawn, in metres.
///
/// # Zero-width bands are triangles that cost and draw nothing
///
/// The footway's stations were placed at `half - FOOTWAY_WIDE * paved`, so on a
/// country lane - where `paved` is nought - six of the thirteen stations landed
/// exactly on the road's edge with the ones beside them. `the_paving_faces_the_sky`
/// caught it immediately and correctly: 3,156 of a village's 6,982 paving triangles
/// had no area, so no normal, so nothing to say they faced up.
///
/// A road's station count cannot vary along its length - `paved` is a gradient and
/// the strip below has to weave a constant number of vertices - so the answer is for
/// the band never to close completely. At nought it is a hand's width of ordinary
/// road surface at the edge, which is invisible; as the paving arrives it opens out
/// into the footway.
const VERGE_LEAST: f32 = 0.35;

/// How far apart the two stations at the top of a kerb sit, in metres.
///
/// There are two because the kerb's colour has to STOP there and the footway's
/// start, and one station carries one colour - so the change from stone to flag is
/// a hard line rather than a two-metre fade across the pavement. Put at exactly the
/// same place they were a zero-area quad the whole length of every road, which is
/// the second thing `the_paving_faces_the_sky` caught. Two centimetres apart the
/// line still reads as hard and the triangles have area.
const SEAM: f32 = 0.02;
/// # A kerb has to be a STEP, not a change of colour
///
/// This was 0.14 - a real kerb - and the note beside it said there was no reason to
/// exaggerate one, because what makes a footway read is the line and the change of
/// surface. That was wrong, and how it was wrong is worth keeping: a 14 cm rise, with
/// the batter derived from the climb rule, leaves a face 13 cm wide on the ground -
/// which at a third-person camera's height, under this game's near-flat shading,
/// carries almost no value difference. So the footway read as paint, and was reported
/// exactly that way: "these are no true curbs and just read as different colors".
///
/// 22 cm is taller than a kerb in the world, and this is not the world. It is the
/// height at which the face becomes a face: a 20 cm band of its own colour, turned
/// away from the sky enough to shade differently from both surfaces it divides. The
/// batter follows it, so it stays a step the warden can take.
const KERB_RISE: f32 = 0.22;

/// How far the kerb's face leans back, in metres. A CHAMFER, not a ramp.
///
/// This was `KERB_RISE / CLIMB_LIMIT`, derived so the climb rule would accept it -
/// which made the face 20 cm wide for a 22 cm rise, a 35 degree slope. From a
/// third-person camera that is not a kerb, it is a damp patch at the edge of the
/// road, and it was reported as one.
///
/// The climb rule no longer has to accept it: `player::STEP_UP` does, because a step
/// and a slope are different things. So the face is nearly vertical with the small
/// bevel a cut stone actually has, and `a_kerb_is_a_step_and_not_a_wall` checks it
/// against the rule that now governs it.
const KERB_RUN: f32 = 0.05;

/// A road's whole cross-section at one point along it.
///
/// # Add the urban right-of-way; do not subtract it from the country road
///
/// The first cut of the footways took them out of the width a road already had. A
/// country lane is 4.6 m, so at full paving it gave up 2 m to each side and clamped
/// what was left - leaving a 1.38 m carriageway, narrower than one cart, which then
/// snapped to the 10 m city street it was joining. Codex's research put the rule
/// plainly: a road transition is not a material fade, it is a gradual change in the
/// whole right-of-way, and the carriageway has to stay usable the whole way through.
///
/// So the TOTAL width eases from what the road is to what it joins, and the footways
/// are added around a carriageway that keeps its own size. At the end of a country
/// approach the section is exactly the high street's: 6 m of road between two 2 m
/// pavements, which is what it is about to become.
///
/// Every consumer asks this one function - the mesh's stations, their colours, and
/// what the warden's feet stand on. That is the same contract the windows and the
/// fences already have: one fact, calculated once.
#[derive(Clone, Copy)]
pub struct RoadSection {
    /// How much of a city street this is, nought to one.
    pub paved: f32,
    /// Half the whole right-of-way, kerb to kerb to verge.
    pub half: f32,
    /// Half the carriageway - where the carts go.
    pub carriage: f32,
    /// How high the kerb stands, and how far it leans back.
    pub kerb: f32,
    pub batter: f32,
    /// Where the made surface gives out into the ground.
    pub shoulder: f32,
}

impl RoadSection {
    /// The section of a road `wide` metres across that joins a `joins` metre street.
    /// `wander` is how much wider or narrower this stretch happens to be - see
    /// `wander_at`. It scales the WHOLE section, not only its outer edge: the mesh
    /// used to wander `half` and leave the kerb's batter at nominal size, so on the
    /// narrow side of a wander the kerb stood where the analytical surface did not
    /// put it. Codex found that, with the rest of this family.
    pub fn new(wide: f32, joins: f32, paved: f32, wander: f32) -> Self {
        let paved = paved.clamp(0.0, 1.0);
        let half = (wide + (joins - wide) * paved) * 0.5 * wander;
        let footway = (VERGE_LEAST + (FOOTWAY_WIDE - VERGE_LEAST) * paved) * wander;
        Self {
            paved,
            half,
            carriage: (half - footway).max(0.8),
            kerb: KERB_RISE * paved,
            batter: (VERGE_LEAST * 0.3 + KERB_RUN * paved) * wander,
            shoulder: half + SHOULDER_WIDE * wander,
        }
    }

    /// How high its surface stands at `across` metres from the middle.
    pub fn lift(&self, across: f32) -> f32 {
        let across = across.abs();
        let shoulder = self.shoulder.max(0.01);
        if self.kerb <= 0.0 {
            // A COUNTRY ROAD IS UNCHANGED. Written as an early return rather than as
            // a profile that happens to collapse, so a village lane cannot drift by a
            // millimetre when the footway's numbers are tuned.
            return road_lift(across / shoulder);
        }
        if across <= self.carriage {
            road_lift(across / shoulder)
        } else if across <= self.carriage + self.batter {
            let up = (across - self.carriage) / self.batter.max(0.001);
            road_lift(self.carriage / shoulder) + self.kerb * up
        } else if across <= self.half {
            road_lift(self.carriage / shoulder) + self.kerb
        } else {
            let out = ((across - self.half) / SHOULDER_WIDE).clamp(0.0, 1.0);
            let top = road_lift(self.carriage / shoulder) + self.kerb;
            top + (ROAD_HEM - top) * out
        }
    }
}

/// How much wider or narrower a road is at this point on its middle line.
///
/// # One field, sampled in one place
///
/// A walked track is wider where the ground is easy, so the ribbon is modulated by a
/// slow field. The mesh applied it and `stands_on` did not, so the drawn road and the
/// walkable road disagreed by up to a sixth of a width. Worse, `stands_on` asked
/// `paved_here` at the PLAYER's position rather than on the road's middle line, so
/// stepping sideways across one cross-section could change which section the game
/// thought it was standing on. A road's section is a property of a point on the ROAD,
/// not of where somebody stands beside it.
///
/// A dirt track wanders and a city street does not: a kerb is a made edge, and a
/// straight one.
fn wander_at(on: Vec2, paved: f32) -> f32 {
    1.0 + (terrain_core::forest::field(on / ROAD_WANDERS_OVER, 733) - 0.5)
        * ROAD_WANDERS_BY
        * (1.0 - paved)
}

/// How high a street's surface stands at a given distance from its middle.
///
/// # One cross-section, drawn and walked
///
/// `pave` builds the road's vertices and `stands_on` decides what the warden's feet
/// rest on, and both of them need to agree about the shape of a street to the
/// centimetre. They used to share `road_lift`, which was fine while a road was a
/// crown and nothing else. A kerb is the moment that stops being enough: put a step
/// in the mesh and not in the walk surface and the player wades through the footway;
/// put it in the walk surface and not the mesh and they walk on air beside the road.
///
/// So the profile is one function and they both ask it. `across` is the distance from
/// the centreline, `half` the road's own half-width, `shoulder` where it gives out
/// into the ground, and `paved` how much of a city street this is - nought in a
/// village, where there is no footway and this returns exactly what `road_lift` did.

/// How far above the ground a street's surface is laid, in metres.
///
/// Four centimetres. Flat on the terrain z-fights with it - two surfaces at the
/// same height flicker against each other wherever they meet - and any higher is a
/// kerb you can see the edge of from across the square.
// 9 cm, up from 4. Four cleared the ground in arithmetic and not on screen: the
// chunk mesh is a grid of flat triangles and the depth buffer has opinions, so a
// surface laid four centimetres over it flickers along every triangle edge.
const ROAD_LIES: f32 = 0.09;

/// How much a walked path's width wanders, and over what distance.
///
/// A third either way is a lot on paper and reads as very little on the ground -
/// what the eye picks up is that the two edges are not parallel, which is the whole
/// difference between a track and a band. Over twenty-two metres, so it is a slow
/// change along the path rather than a ripple.
const ROAD_WANDERS_BY: f32 = 0.34;
const ROAD_WANDERS_OVER: f32 = 22.0;

/// How far the road's own edge still stands off the ground, in metres.
///
/// Not nought. Two surfaces at exactly the same height flicker against each other
/// wherever they meet, so the edge keeps just enough to win the depth test and not
/// enough for anybody to see a step.
const ROAD_HEM: f32 = 0.015;

/// How much a road's colour varies with wear, and over what distance.
///
/// Enough to break a flat slab into something that looks used, not so much that a
/// road stops being one colour. The distance is in metres, and it is large: what
/// wears a track is where the carts go, which changes over tens of metres rather
/// than every step.
// Raised from 0.17. At that the variation was there in the mesh and invisible on
// screen, so a path was still two flat tones sitting next to each other - the shape
// read as walked and the surface still read as painted on.
const ROAD_WEARS: f32 = 0.34;
const ROAD_WEARS_OVER: f32 = 26.0;

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
// Mid grey, not dark. These are read UNDER the near-cel banding, which pulls every
// surface toward the nearest of four steps - so a colour chosen by eye off a swatch
// lands a whole band darker than intended once it is in the world. Photographed at
// 0.34 a paved street came out charcoal; a road is a light surface with dark things
// standing on it, and it has to stay lighter than the grass beside it.
// A CITY STREET IS COBBLED.
//
// It was a flat grey slab - one value over the whole carriageway, which reads as
// poured concrete and is the one surface a stone-built city should not have. Warmer
// and darker than the slab was, so the stones have somewhere to vary to.
//
// Also darkened from 0.56 for the same reason the paving was: it was chosen while
// every road faced the wrong way and took ambient light only, so it had to be pale
// to read at all. Taking the sun, a city street came out white.
/// A colour written the way a person picks one, turned into the light a shader wants.
///
/// # The trap this exists to close
///
/// A vertex colour reaches the shader as LINEAR light. Every road constant here was
/// written as though it were sRGB - the value you would type into a colour picker -
/// and linear 0.31 is sRGB 0.58, so every road in the world shipped about twice as
/// bright as its number said. That is two "perfectly good browns" that photographed
/// pale, a city street that came out white however far the constant was pushed down,
/// and three darkenings that each did less than they should have.
///
/// Hand-converting the four of them fixed those four. This closes the trap: a colour
/// is now WRITTEN in the space it was chosen in and converted on the way out, so the
/// next one cannot be wrong.
///
/// Blender's side never had this problem - `masonry.paint` has always run
/// `to_linear` on its palette. The mistake was only ever possible on the Rust side,
/// where a colour is a bare array with nothing to say which space it is in.
fn srgb(r: f32, g: f32, b: f32) -> [f32; 4] {
    let up = |c: f32| {
        if c <= 0.040_45 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    [up(r), up(g), up(b), 1.0]
}

static ROAD_STONE: LazyLock<[f32; 4]> = LazyLock::new(|| srgb(0.42, 0.41, 0.40));

/// The footway's own stone, and how big a flag is.
///
/// Paler and warmer than the carriageway, because a footway is laid rather than
/// driven on: what tells the two apart at a glance is that one is made of small dark
/// stones and the other of big pale ones. The size does most of that work - a flag is
/// twice a cobble, so the two surfaces have visibly different grain even where the
/// colours are close.
static ROAD_FLAG: LazyLock<[f32; 4]> = LazyLock::new(|| srgb(0.60, 0.585, 0.55));
const FLAG_IS: f32 = 1.15;

/// How big a cobble is, in metres, and how much one differs from the next.
///
/// Small enough to be a stone rather than a slab, big enough to survive the road
/// being drawn at a metre a vertex - what carries at distance is that the surface is
/// BROKEN, not that any one stone is legible.
const COBBLE_IS: f32 = 0.55;
const COBBLES_VARY: f32 = 0.30;

/// What a VILLAGE's lanes are made of: packed earth and cobble, warm and rough.
///
/// A village is old-school fantasy and a city is modern, and the ground underfoot is
/// half of that difference - asphalt through a thatched village would undo the
/// silhouette work above it before you looked up.
// Warmer and more saturated than it was. At (0.66, 0.54, 0.38) a village lane was a
// pale tan, and the near-cel banding pulls saturation out of everything it steps -
// so on screen it read as the same grey as a city street and every road in the world
// looked paved. Dirt has to be unmistakably BROWN before the banding gets it.
// # Why a brown road kept photographing grey
//
// Twice this was set to a perfectly good brown - a mid (0.56, 0.40, 0.24) and a
// light (0.82, 0.63, 0.40) - and twice a photograph of a village lane came back
// neutral grey, indistinguishable from a city's paving.
//
// It is not the banding and it is not the material. A road is a flat, upward-facing
// surface, so almost all the light landing on it is SKY light, and the sky is blue.
// Dividing an observed road pixel by the colour that produced it puts this world's
// road light at about (0.22, 0.30, 0.50) - blue arrives 2.2x stronger than red.
// Any colour whose blue channel is more than about a 2.2th of its red comes out the
// other side neutral, however brown it looked in the constant.
//
// So the blue is crushed rather than the red raised. R:B here is about 5.6:1, which
// lands on screen at roughly 2.5:1 - brown, and legibly not paving.
// # These are LINEAR, and for a long time they were not
//
// A vertex colour reaches the shader as LINEAR light, and every one of these was
// written as though it were sRGB - the value you would type into a colour picker.
// Linear 0.31 is sRGB 0.58, so every road in the world shipped about twice as bright
// as the number said, which is most of the history above: two "perfectly good
// browns" that photographed pale, a city street that came out white however far the
// constant was pushed down, and three separate darkenings that each moved it less
// than expected.
//
// Blender's side was never wrong - `masonry.paint` runs `to_linear` on the palette.
// It was only ever these. Each is now the linear value of the sRGB colour named in
// its comment.

static ROAD_EARTH: LazyLock<[f32; 4]> = LazyLock::new(|| srgb(0.62, 0.42, 0.24));
static ROAD_COBBLE: LazyLock<[f32; 4]> = LazyLock::new(|| srgb(0.46, 0.43, 0.39));

/// How much one paving stone differs from its neighbour.
///
/// A road of one flat colour is a painted stripe. Varying each quad a little is what
/// turns it into stones - it costs nothing, because the paving is already built as
/// quads and every quad already carries a colour.
const STONE_VARIES: f32 = 0.16;
// The kerb of a PAVED street. A dirt track has no kerb - see `pave`, which uses the
// surface colour at its edges when there is no city to put a kerb on.
// DARKER than either surface it divides - the carriageway is 0.42 and the footway
// 0.60 - so it reads as a line between them at distance and as a shaded face close
// up. A kerb the colour of the road is a road with a step in it.
static ROAD_KERB: LazyLock<[f32; 4]> = LazyLock::new(|| srgb(0.30, 0.29, 0.28));

/// How wide the margin is where a road gives out into the ground, in metres.
///
/// Not a kerb and not a verge anybody walks on - it is the distance over which the
/// surface stops being road and starts being whatever is around it. Wide enough to
/// read as a blend at walking distance, narrow enough that the road keeps its width.
// Widened from 1.7. The margin is where a path stops being a path, and a long one
// reads as ground that has been walked less rather than as an edge.
//
// 5.4 now. At walking distance 2.5 m read as a blend, and from any height above the
// roofs it did not: the ribbon's own fade is a couple of metres while the settled
// GROUND under it browns off over tens, so a crisp shape sat inside a soft halo of
// the same colour and the eye read the mismatch as a hard edge. Reported as "the
// ground blends oddly".
//
// The fade's far lane already carries the terrain's own colour - `hem` asks
// `ground_colour` for it - so widening this is widening a gradient that already ends
// in exactly the ground beside it. The road keeps its width; only the dissolve gets
// longer.
const SHOULDER_WIDE: f32 = 5.4;

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

/// How much room the guild hall is given, in metres.
///
/// Nothing taller than a city block stands inside this of the hall, so its tower is
/// seen against sky rather than against a neighbour - a landmark read against
/// another building is not read at all.
///
/// A PLAZA, not half the city. At 54 m this cleared towers out of most of the market
/// district, which broke the thing districts are for: `a_town_has_districts_and_they
/// _do_not_look_alike` reported the middle of a city holding no more tall buildings
/// than its outskirts, and it was right. Thirty-four keeps the hall's own square
/// clear and leaves the business district standing around it, which is the shape a
/// cathedral square has anyway.
const KEEPS_CLEAR: f32 = 34.0;

// # THE RING WALL IS GONE
//
// A settlement used to be enclosed by a low timber or concrete ring - Lynch's edge,
// the one of his five elements that does not fall out of laying ground - broken by a
// gateway wherever a street crossed it.
//
// It went because it was a CIRCLE. Every settlement wore the same perfect ring at
// the same fraction of its radius, and a perfect circle is the one shape that says
// "generated" from any angle: a real place is bounded by what happens to be there,
// and no real place is bounded by a compass. Reviewed independently as "another
// perfect circle" and called by the user, looking at it in game, something to just
// remove.
//
// What it was FOR still stands and is now carried by the ground instead. A
// settlement's earth or paving reaches past its buildings and fades out over the
// last of itself - see `Settlements::ground_at` - so arriving still has a moment,
// and the moment is a change underfoot rather than a fence with a gap in it.


/// Joins one unbroken stretch of boundary into triangles.
fn stitch(indices: &mut Vec<u32>, run: &[usize]) {
    for pair in run.windows(2) {
        let (a, b) = (pair[0] as u32, pair[1] as u32);
        for rung in 0..5u32 {
            let (p, q) = (a + rung, a + rung + 1);
            let (r, t) = (b + rung, b + rung + 1);
            indices.extend_from_slice(&[p, r, q, q, r, t]);
        }
    }
}

/// Builds one town's streets as a mesh laid on the ground.
/// Mixes two road colours.
fn mix(a: [f32; 4], b: [f32; 4], part: f32) -> [f32; 4] {
    let part = part.clamp(0.0, 1.0);
    [
        a[0] + (b[0] - a[0]) * part,
        a[1] + (b[1] - a[1]) * part,
        a[2] + (b[2] - a[2]) * part,
        1.0,
    ]
}

/// How paved the ground under a point is, nought to one.
///
/// One where it is well inside a city and nought out in the country, easing over
/// `PAVING_ARRIVES` at the edge - so a dirt road coming in becomes a street over the
/// last stretch of its approach instead of at a line.
fn paved_here(plan: &crate::world::settle::Settlements, at: Vec2) -> f32 {
    plan.sites()
        .iter()
        .filter(|site| site.city && !site.ranch)
        .map(|site| {
            crate::util::smoothstep(
                site.radius + PAVING_ARRIVES,
                site.radius,
                site.at.distance(at),
            )
        })
        .fold(0.0_f32, f32::max)
}

/// Over what distance a country road turns into a city street, in metres.
const PAVING_ARRIVES: f32 = 34.0;

/// Where roads actually MEET, as (place, the widest road meeting there).
///
/// # A bend is not a junction
///
/// This used to answer "every endpoint of every segment", which was right when a
/// road was a row of independent rectangles: consecutive pieces were square to
/// different bearings, so every joint of a curved ring left a notch and every notch
/// wanted a patch.
///
/// `Way` fixed that. A chain mitres its own bends - both pieces take one
/// cross-section from `Way::across`, so they share an edge exactly and there is
/// nothing left to fill. The discs went on being emitted anyway, at every
/// subdivision of every curve, and once the cross-section grew a raised footway they
/// stopped being harmless: a flat carriageway-coloured patch at each of them, over
/// the kerb, all the way round every ring. Codex's research is blunt about the
/// distinction and this is it, in code: cap where roads MEET, and leave a bend alone.
///
/// # And a road ends ON another road, not on one of its corners
///
/// The first version of this clustered shared VERTICES, which found the square where
/// several radials start and missed almost everything else: a ring is drawn as a
/// chain of arc samples, and a radial meets it wherever it happens to arrive - which
/// is between two of those samples, not on one. So the caps vanished from most of the
/// junctions in every settlement and the notch came back. Reported as roads not
/// connecting properly to most cities and some towns, which is what it was.
///
/// An END of one road, tested against the LINE of every other. A way's interior
/// bends are still left alone - those are mitred and were never the problem.
fn junctions_in(ways: &[Way]) -> Vec<Meeting> {
    // How near a road has to pass for an end to be ON it.
    const TOUCHING: f32 = 0.6;

    let near_way = |way: &Way, at: Vec2| {
        way.points.windows(2).any(|pair| {
            let run = pair[1] - pair[0];
            let along = run.length_squared();
            let part = if along > 1.0e-6 {
                ((at - pair[0]).dot(run) / along).clamp(0.0, 1.0)
            } else {
                0.0
            };
            at.distance(pair[0] + run * part) < TOUCHING
        })
    };

    let mut met: Vec<Meeting> = Vec::new();
    for (index, way) in ways.iter().enumerate() {
        let (Some(first), Some(last)) = (way.points.first(), way.points.last()) else {
            continue;
        };
        for end in [*first, *last] {
            // WHO ELSE IS HERE. Every other road whose LINE passes through this end,
            // not merely those with a vertex on it.
            let mut arms = vec![(way.wide, way.joins)];
            let mut whose = vec![index];
            for (other, road) in ways.iter().enumerate() {
                if other != index && near_way(road, end) {
                    arms.push((road.wide, road.joins));
                    whose.push(other);
                }
            }
            if arms.len() < 2 {
                continue;
            }
            match met.iter_mut().find(|node| node.at.distance(end) < TOUCHING) {
                Some(node) => {
                    for (arm, who) in arms.into_iter().zip(whose) {
                        if !node.whose.contains(&who) {
                            node.whose.push(who);
                            node.arms.push(arm);
                        }
                    }
                }
                None => met.push(Meeting { at: end, arms, whose }),
            }
        }
    }
    met
}

/// A place where roads meet, and what meets there.
///
/// # The arms, not their widest
///
/// This kept only `max(wide)` and the patch was drawn at that road's carriageway. At
/// a 10 m high street meeting an 8 m lane the patch came out 3 m across while the
/// lane's carriageway is 2 m, so it paved a metre into the lane's footway - and the
/// test written to prevent exactly that passed, because it only ever measured a 10 m
/// patch against a 10 m road. Codex caught both the fault and the hole in its guard.
///
/// Each arm also carries what it JOINS. A country road arriving at a gateway is 4.6 m
/// widening to 10, and a node that only knew 4.6 would resolve a section with the
/// footways carved back out of it - the pinch `RoadSection` exists to prevent.
pub struct Meeting {
    pub at: Vec2,
    /// Every road that meets here, as (its width, the width it joins).
    pub arms: Vec<(f32, f32)>,
    whose: Vec<usize>,
}

impl Meeting {
    /// How far the patch may reach: the NARROWEST carriageway that meets here.
    ///
    /// A patch is there to fill the notch between carriageways. Reaching past the
    /// tightest of them is paving somebody's pavement.
    pub fn fills(&self, paved: f32) -> f32 {
        self.arms
            .iter()
            .map(|(wide, joins)| RoadSection::new(*wide, *joins, paved, 1.0).carriage)
            .fold(f32::MAX, f32::min)
    }
}

fn pave(
    ways: &[Way],
    terrain: &crate::world::terrain::Terrain,
    low: Vec2,
    city: f32,
) -> Mesh {
    let at_plan = terrain.plan();
    let mut places: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut colours: Vec<[f32; 4]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    // ------------------------------------------------------------------ MITRED
    //
    // # Drawn from the chain, so a bend is a bend and not a break
    //
    // Each piece of a ring used to be laid as its own rectangle, square across its
    // own direction. On a curve a rectangle's outer edge is shorter than the arc it
    // stands for and its inner edge is longer, so consecutive pieces gapped on the
    // outside and overlapped on the inside - a sawtooth of triangular bites out of
    // the kerb the whole way round.
    //
    // The first attempt at fixing it worked on the pieces and made things far worse:
    // to mitre you need a piece's neighbour, and once a road has been cut up the only
    // way to find one is to look for another piece sharing an endpoint - which at the
    // mouth of a radial finds two. Spikes everywhere.
    //
    // A `Way` knows. Both pieces at a bend take ONE cross-section from
    // `Way::across`, bisecting the turn, so their quads share an edge exactly and
    // there is nothing left to gap. Where a road ENDS the cross-section is square,
    // and where roads meet the junction disc covers the joint - which is the right
    // division of labour, because a junction is a place and a bend is not.
    for way in ways {
        if way.points.len() < 2 {
            continue;
        }
        let across = way.across();
        for (piece, pair) in way.points.windows(2).enumerate() {
            let (from, to) = (pair[0], pair[1]);
            let run = to - from;
            let length = run.length();
            if length < 0.05 {
                continue;
            }
            let steps = (length / ROAD_STEPS_EVERY).ceil().max(1.0) as usize;
            let (side_from, stretch_from) = across[piece];
            let (side_to, stretch_to) = across[piece + 1];

        for step in 0..=steps {
            let part = step as f32 / steps as f32;
            let on = from + run * part;
            // The cross-section turns through the piece from one end's to the
            // other's, so a bend eases rather than shearing at one station - and at
            // each end it is exactly what the neighbouring piece will use.
            let side = side_from.lerp(side_to, part).normalize_or(side_from)
                * (stretch_from + (stretch_to - stretch_from) * part);
            // Three across: kerb, middle, kerb, so the edge can be a shade paler
            // and the road has an edge at all.
            // FIVE across, not three.
            //
            // With three - kerb, middle, kerb - the carriageway's own colour exists
            // only on the centre LINE, and every other pixel of the road is a blend
            // toward the grey of a kerb. Photographed, a village's packed-earth lane
            // came out the same cold grey as a city street, and the two ages of the
            // world stopped being two ages at the one place they touch the ground.
            //
            // Kerbs at the very edge and the surface held flat across the middle.
            // HOW PAVED IS IT HERE, rather than is this a paved road.
            //
            // # A dirt track that stops dead against a kerb
            //
            // A country road was drawn as dirt or as paving by a single flag for the
            // whole mesh, decided by whether the leg's MIDDLE stood on a city's
            // ground. So the surface changed material at a leg boundary, in one
            // step, in the middle of open country - reported as a path ending
            // abruptly at the city path, which is exactly what a boolean looks like
            // when what it describes is a gradient.
            //
            // A road does not become a street at a line; it becomes one over the
            // last thirty metres of the approach. `paved` is that, and every colour
            // below is mixed by it.
            let paved = city.max(paved_here(at_plan, on));
            let surface = mix(*ROAD_EARTH, *ROAD_STONE, paved);

            // A WALKED PATH WANDERS IN WIDTH.
            //
            // A band of exactly constant width with two ruler-straight edges is a
            // thing somebody laid down. What makes a track read as walked is that it
            // is wider where the ground is easy and narrower where it is not, in
            // long slow changes rather than a wobble - so the width is modulated by
            // a field over tens of metres, in the world's own coordinates, and two
            // roads crossing agree about it.
            //
            // A city's paving does NOT do this: a kerb is a made edge and a straight
            // one, and wandering it would read as a mistake rather than as wear.
            // A kerb is a made edge and a straight one, so the wander fades out as
            // the paving comes in rather than stopping with it.
            let wander = wander_at(on, paved);
            let cut = RoadSection::new(way.wide, way.joins, paved, wander);
            let half = cut.half;
            // A kerb only where there is paving to kerb. A cart track's edge is
            // where the dirt stops and the grass starts, and putting a stone kerb
            // down each side of one is most of why they all read as paved.
            // A kerb only where there is paving to kerb, and it arrives with the
            // paving rather than all at once.
            let edge = mix(surface, *ROAD_KERB, paved);

            // AND A SHOULDER EITHER SIDE.
            //
            // # Two separate colours
            //
            // The ribbon used to stop dead at its own edge, so a dirt road was a
            // brown band with a razor line down each side of it against whatever the
            // ground was - reported as reading like two colours laid next to each
            // other rather than as a track worn into the earth. A real road has no
            // edge; it has a margin where the surface gives out and the ground takes
            // over.
            //
            // The shoulder vertices carry the GROUND's own colour, asked of the
            // terrain at that exact spot, so the ribbon fades into whatever is
            // actually there - grass, a town's packed earth, sand - and keeps fading
            // into the right thing when the road crosses from one into another.
            let shoulder = half + SHOULDER_WIDE * wander;
            let hem = |out: f32| terrain.ground_colour((on + side * out).x, (on + side * out).y);

            // A FOOTWAY DOWN EACH SIDE, where the street is a city's.
            //
            // The carriageway gives up `FOOTWAY_WIDE` to each side and the kerb
            // between them is a real step - see `road_surface`, which is also what
            // the warden's feet stand on.
            //
            // Thirteen stations across rather than seven, and the extra six are what
            // a kerb costs: the top and the foot of each face, and a repeat of the
            // top carrying the footway's colour instead of the kerb's, so the line
            // between stone and flag is a hard edge rather than a two-metre fade.
            //
            // In a village the outer bands narrow to `VERGE_LEAST` and carry the
            // road's own colour flat, so they cost a few vertices and show nothing.
            // They do not close: the station COUNT has to be the same at every point
            // along a road, because `paved` is a gradient - a lane becomes a street
            // over the last thirty metres of its approach - so there is no line
            // anywhere to change the count at, and a band that shuts completely is a
            // triangle with no area and no normal. Half a village's paving was that
            // for one build.
            let walk = cut.carriage;
            let batter = cut.batter;
            let flag = mix(surface, *ROAD_FLAG, paved);
            for (across, colour, grain) in [
                (-shoulder, hem(-shoulder), 0.0),
                (-half, flag, FLAG_IS),
                (-(walk + batter + SEAM), flag, FLAG_IS),
                (-(walk + batter), edge, 0.0),
                (-walk, edge, 0.0),
                (-walk * 0.62, surface, COBBLE_IS),
                (0.0, surface, COBBLE_IS),
                (walk * 0.62, surface, COBBLE_IS),
                (walk, edge, 0.0),
                (walk + batter, edge, 0.0),
                (walk + batter + SEAM, flag, FLAG_IS),
                (half, flag, FLAG_IS),
                (shoulder, hem(shoulder), 0.0),
            ] {
                let at = on + side * across;

                // CROWNED, and tucked in at the edges.
                //
                // # A road that reads as a plank laid on a field
                //
                // The whole ribbon sat `ROAD_LIES` above the ground - nine
                // centimetres, everywhere - so its outer edge hung in the air over
                // the ground it was supposed to be part of, with a step and a
                // shadow all the way along it. Reported as the road and the ground
                // reading as "two separate objects", which is exactly what a
                // surface floating over another surface is.
                //
                // The lift now falls off across the width: full down the middle,
                // almost nothing at the shoulder, so the ribbon meets the ground at
                // its edge and there is no step to see. What is left is a CROWN -
                // higher down the centre than at the sides - which is how a road is
                // actually built, and which reads as worn in rather than put down.
                let lift = cut.lift(across);

                // AND BRUSHED, not painted.
                //
                // One flat colour over the whole surface is the other half of why it
                // read as an object rather than as ground. Packed earth is worn in
                // patches - a wheel rut here, a dry spot there - so the colour is
                // multiplied by a slow field and a faster one, drawn in the world's
                // own coordinates so the variation crosses a junction rather than
                // stopping at the edge of whichever piece drew it.
                let broad = terrain_core::forest::field(at / ROAD_WEARS_OVER, 517);
                let fine = terrain_core::forest::field(at / (ROAD_WEARS_OVER * 0.21), 518);
                // Three scales, because wear has three: where the carts go, where
                // the puddles sit, and the scuff of the ground itself.
                let close = terrain_core::forest::field(at / (ROAD_WEARS_OVER * 0.06), 519);
                let mut worn = 1.0
                    + (broad - 0.5) * ROAD_WEARS
                    + (fine - 0.5) * ROAD_WEARS * 0.5
                    + (close - 0.5) * ROAD_WEARS * 0.22;

                // AND THE STONES THEMSELVES, on a city street.
                //
                // Every cobble takes its own value from a field sampled at the size
                // of a stone, so what you see is a surface made of pieces rather
                // than a poured one. Only where there is paving to cobble: a dirt
                // track has no stones in it, and giving it some would read as
                // gravel.
                // Each surface at the size of its own stone - `grain` - so the
                // carriageway is cobbled and the footway is flagged, and the two read
                // as different materials rather than as one in two colours.
                if paved > 0.0 && grain > 0.0 {
                    let stone = terrain_core::forest::field(at / grain, 941);
                    worn *= 1.0 + (stone - 0.5) * COBBLES_VARY * paved;
                }
                let colour = [
                    colour[0] * worn,
                    colour[1] * worn,
                    colour[2] * worn,
                    colour[3],
                ];

                let height = terrain.drawn_height(at.x, at.y) + lift;
                places.push([at.x - low.x, height, at.y - low.y]);
                normals.push([0.0, 1.0, 0.0]);
                colours.push(colour);
                uvs.push([step as f32, across]);
            }
        }

        const LANES: usize = 13;
        let base = (places.len() - (steps + 1) * LANES) as u32;
        for step in 0..steps as u32 {
            for lane in 0..(LANES as u32 - 1) {
                let a = base + step * LANES as u32 + lane;
                let b = a + 1;
                let c = a + LANES as u32;
                let d = c + 1;
                // WOUND FACE UP.
                //
                // # Why the lamps did not light the road
                //
                // Every one of these was wound the other way, so the paving faced
                // DOWN while the normals it carried said up. That was known about -
                // it is why the material has `cull_mode: None`, which was the fix
                // for the road being invisible - but disabling culling only makes a
                // back face DRAW. It does not make it face the right way.
                //
                // The consequence was invisible by day and obvious by night. Ambient
                // sky light does not care which way a surface points, so a road lit
                // only by ambient looked flat but fine; a point light cares about
                // nothing else, so every lamp in every town lit the ground beside
                // the road and left the road itself black. Measured: 400 of 400
                // triangles faced down.
                indices.extend_from_slice(&[a, b, c, b, d, c]);
            }
        }
        }
    }

    // # THE JOINTS, and the notches they left
    //
    // Every street is laid as its own strip of quads, square across its own bearing.
    // Where two meet at an angle - which is every joint of a curved ring and every
    // junction in the town - the two strips are square to DIFFERENT bearings, so
    // their corners do not line up and a wedge of bare ground shows between them.
    // Around a ring built from six-metre arc pieces that is a notch at every joint,
    // and from above it reads as a cog rather than a circle.
    //
    // The fix is the one a road builder uses: pave the junction itself. Every place
    // a street ends gets a disc of road, wide enough to swallow the notch from any
    // pair of bearings, laid at the same height as the rest. It costs a fan of eight
    // triangles per joint and it is what makes a junction look like a junction
    // rather than like two roads that happen to touch.
    let ends = junctions_in(ways);

    const AROUND_A_JOINT: usize = 10;
    for node in ends {
        // ONLY AS BIG AS THE NARROWEST CARRIAGEWAY MEETING HERE.
        //
        // The disc had the radius of the whole right-of-way, which was right when a
        // road was one flat band and is destructive now: it painted carriageway colour
        // over both footways and cut through the kerb between them. Then it had the
        // WIDEST arm's carriageway, which is wrong wherever two sizes of road meet.
        // What a patch fills is the notch between carriageways - see `Meeting::fills`.
        let at = node.at;
        let paved = city.max(paved_here(at_plan, at));
        let reach = node.fills(paved);
        // The crown to lay it at is the widest arm's, so the patch meets the road it
        // is filling rather than sitting under it.
        let crown = node
            .arms
            .iter()
            .map(|(wide, joins)| RoadSection::new(*wide, *joins, paved, 1.0))
            .max_by(|a, b| a.half.partial_cmp(&b.half).unwrap_or(std::cmp::Ordering::Equal))
            .map(|cut| (cut.lift(0.0), cut.lift(reach)))
            .unwrap_or((ROAD_LIES, ROAD_LIES));
        let middle = places.len() as u32;
        let height = terrain.drawn_height(at.x, at.y) + crown.0;
        places.push([at.x - low.x, height, at.y - low.y]);
        normals.push([0.0, 1.0, 0.0]);
        colours.push(mix(*ROAD_EARTH, *ROAD_STONE, paved));
        uvs.push([0.0, 0.0]);

        for step in 0..=AROUND_A_JOINT {
            let turn = step as f32 / AROUND_A_JOINT as f32 * std::f32::consts::TAU;
            let rim = at + Vec2::from_angle(turn) * reach;
            // At the carriageway's own height where it meets it, so the patch is a
            // crowned cone continuous with the road rather than a flat lid over it.
            let height = terrain.drawn_height(rim.x, rim.y) + crown.1;
            places.push([rim.x - low.x, height, rim.y - low.y]);
            normals.push([0.0, 1.0, 0.0]);
            // The SURFACE colour, not the kerb. A kerb around every joint beads the
            // whole ring with visible cobbled discs - which is what a ring built
            // from six-metre arc pieces looks like when each joint wears a rim. The
            // disc is there to fill a notch, and a patch that fills a hole should
            // not announce itself.
            colours.push(mix(*ROAD_EARTH, *ROAD_STONE, city.max(paved_here(at_plan, rim))));
            uvs.push([turn, 1.0]);
        }
        for step in 0..AROUND_A_JOINT as u32 {
            // Face up, like the ribbon - see the note on the ribbon's winding. The
            // discs were the 670 triangles still facing down after that was fixed,
            // and they showed as a ring of dark patches at every joint the moment
            // the lamps started lighting the road properly.
            indices.extend_from_slice(&[middle, middle + 2 + step, middle + 1 + step]);
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

/// The country roads near the player, drawn as dirt.
///
/// # A graded road nobody can see is a graded road
///
/// `settle` has always cut these into the terrain - it flattens a strip and eases
/// the sides back into the land - but nothing ever DREW them, so the only sign a
/// road existed was a suspiciously level line of grass. A road is a surface.
///
/// Built with the same ribbon the town's streets use, at the same height above the
/// ground and with the same junction discs, so a country road meeting a town's
/// high street looks like one road rather than two that happen to touch.
fn dirt_roads_near(
    plan: &crate::world::settle::Settlements,
    terrain: &crate::world::terrain::Terrain,
    at: Vec2,
) -> Vec<Way> {
    country_roads_near(plan, terrain, at, false)
}

/// The same roads where they run through a CITY, which are paved.
///
/// # A cart track across a plaza
///
/// Every country road was drawn as dirt for its whole length, the part inside a
/// city included - so a modern city with paved streets and a paved square had a
/// brown earth track driving across the middle of it and out the far side. The road
/// BETWEEN towns is a dirt road; the same road, once it is inside a city, is that
/// city's street, and it is surfaced like one.
fn paved_roads_near(
    plan: &crate::world::settle::Settlements,
    terrain: &crate::world::terrain::Terrain,
    at: Vec2,
) -> Vec<Way> {
    country_roads_near(plan, terrain, at, true)
}

/// Whether a country road has a made surface here, and if so how paved it is.
///
/// # What decides that a road is DRAWN has to decide that it is THERE
///
/// A dirt track is not laid across desert or snow - sand and snow do not hold the
/// mark feet make - so `country_roads_near` filters those legs out of the mesh. When
/// the country roads came into `stands_on` they arrived without that filter, and the
/// warden was lifted by roads with no surface. One predicate, both consumers.
///
/// `None` means there is nothing here to stand on.
fn has_a_surface(
    plan: &crate::world::settle::Settlements,
    terrain: &crate::world::terrain::Terrain,
    road: &crate::world::settle::Road,
) -> Option<f32> {
    let mid = (road.from + road.to) * 0.5;
    if plan
        .sites()
        .iter()
        .any(|site| site.city && !site.ranch && site.at.distance(mid) < site.radius)
    {
        return Some(1.0);
    }
    let bare = matches!(
        terrain.region(mid.x, mid.y).0,
        terrain_core::region::Country::Desert | terrain_core::region::Country::Snow
    );
    (!bare).then_some(0.0)
}

/// The country roads near the player, of one surface or the other.
fn country_roads_near(
    plan: &crate::world::settle::Settlements,
    terrain: &crate::world::terrain::Terrain,
    at: Vec2,
    paved: bool,
) -> Vec<Way> {
    plan.ways()
        .iter()
        .filter(|road| {
            let mid = (road.from + road.to) * 0.5;
            mid.distance(at) < RAISES_WITHIN * 1.6
        })
        // WHETHER THIS LEG HAS A SURFACE, and which of the two it is. Both questions
        // belong to `has_a_surface`, which `stands_on` asks too - a road the player is
        // lifted by and cannot see is worse than either alone.
        .filter(|road| has_a_surface(plan, terrain, road) == Some(f32::from(u8::from(paved))))
        // Each leg as a chain of its own. The legs of one route DO join end to
        // end, and mitring across them would be better still - but a route is
        // smoothed into a gentle curve before it is ever laid, so its bends are
        // shallow and its joints do not saw. A town's rings are the sharp case.
        .map(|road| Way {
            points: vec![road.from, road.to],
            wide: crate::config::ROAD_WIDE,
            // What it is about to become. The approach eases its whole width to this
            // over `PAVING_ARRIVES` so the section it hands over is the section it
            // hands over to - see `RoadSection`.
            joins: CITY_STREET_WIDE,
        })
        .collect()
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
    mut footing: Local<Option<(Handle<Mesh>, Handle<crate::shade::Shaded>)>>,
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
        // THE RANCH IS NOT A SETTLEMENT. It is a `Site` only so nothing else can
        // take its ground, and the player SPAWNS on it - a market cross stood on the
        // spawn point with the warden wedged inside it, unable to move.
        //
        // This skip was written once and lost to a later edit of the same block, and
        // the guard meant to catch that walked the OTHER settlements measuring how
        // far their buildings were from the ranch. It never asked what comes up when
        // you stand HERE. `standing_at_the_ranch_raises_nothing` asks that now.
        if site.ranch {
            continue;
        }
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
        // What this settlement actually cost, per settlement.
        //
        // Codex's invariant: adding more provisional lot candidates must not silently
        // multiply the shipped scene. It cannot be read off "N buildings" - that was
        // the number that quietly went from sixteen to sixty-four - so the split is
        // printed and the budget is visible from the log.
        let yards = layout.plots.iter().filter(|p| p.what.is_yard()).count();
        info!(
            "raising {} at ({:.0}, {:.0}): {} buildings, {yards} yards, {} scenes",
            if site.city { "a city" } else { "a town" },
            site.at.x,
            site.at.y,
            layout.plots.len() - yards,
            layout.plots.len(),
        );
        for plot in &layout.plots {
            // On the GROUND's own height wherever it lands, not on the site's
            // levelled height: a town is allowed to spill past the rim of the
            // ground that was flattened for it, and a house out on the fade has to
            // sit into the slope rather than float over it.
            let (sits, stands) = under(&terrain.0, plot.at, plot.what.footprint(), plot.facing);

            // A FOOTING WHERE THE GROUND FALLS AWAY.
            //
            // A building is seated on the HIGHEST of its corners, because one sunk
            // into a rise is one you walk into the roof of. The cost is the other
            // end: on any slope the low corner hangs, and the wider the footprint the
            // further it hangs. A cottage's 9 m span hid it. The guild hall's 26 m
            // did not, and it was reported as floating - then as every building
            // floating, which is the same fault at every size at once.
            //
            // The gap is filled rather than argued with, because that is what a
            // building on a slope actually has: a footing, holding the floor level
            // while the ground drops away under it. Sized to the fall, so on level
            // ground none is built.
            let drop = stands - sits;
            if drop > FOOTING_SHOWS {
                let footing = footing.get_or_insert_with(|| {
                    (
                        meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
                        materials.add(crate::shade::shaded(StandardMaterial {
                            // Darker than the stone above it: a footing is the part
                            // in the building's own shadow, and a pale slab under a
                            // house reads as the house sitting on a plate.
                            base_color: Color::srgb(0.28, 0.27, 0.25),
                            perceptual_roughness: 0.95,
                            reflectance: 0.02,
                            ..default()
                        })),
                    )
                });
                let span = plot.what.footprint();
                commands.spawn((
                    FromSite(key),
                    Mesh3d(footing.0.clone()),
                    MeshMaterial3d(footing.1.clone()),
                    // Down from the floor to below the lowest corner, so it meets the
                    // ground rather than stopping just above it.
                    // TURNED THE WAY THE FOOTPRINT IS, which is NEGATIVE facing.
                    //
                    // `Plot::walls` lays its box out with `(x cos - y sin, x sin +
                    // y cos)`, and a Bevy turn about +Y maps a local point the other
                    // way round - so `facing` reflects the box instead of rotating
                    // it, and the footing came out skewed across the front of the
                    // building like a spilled plinth. Derived rather than guessed:
                    // matching the two expressions gives theta = -facing.
                    Transform::from_xyz(plot.at.x, stands - drop * 0.5 - 0.05, plot.at.y)
                        .with_rotation(Quat::from_rotation_y(-plot.facing))
                        // Just inside the walls, so a footing is something the
                        // building stands ON rather than a ledge around it.
                        .with_scale(Vec3::new(span.x * 0.97, drop + 0.1, span.y * 0.97)),
                    Visibility::default(),
                ));
            }

            commands.spawn((
                Standing { what: plot.what },
                FromSite(key),
                SceneRoot(assets.load(GltfAssetLabel::Scene(0).from_asset(plot.what.model()))),
                Transform::from_xyz(plot.at.x, stands, plot.at.y)
                    .with_rotation(Quat::from_rotation_y(model_turn(plot.facing))),
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
                // Both sides. A road is a single-sided ribbon, and a ribbon wound the
                // wrong way is not dim or dark - it is INVISIBLE, which is what
                // "still no roads" looked like through three rounds of measuring a
                // mesh that was entirely correct.
                double_sided: true,
                cull_mode: None,
                ..default()
            }))
        });
        let paving = pave(&layout.ways, &terrain.0, site.at, f32::from(u8::from(site.city)));
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
    /// This plot's floor at a point, if its floor is what you would be standing on.
    ///
    /// Inside the walls, the boards. Just outside the front, the STEP - a ramp from
    /// the ground up to the threshold, because a floor that appears at the footprint
    /// edge is a lip the walking rule would refuse and a doorway you cannot enter.
    /// The step's own reach and width are measured off the model, so the ramp is the
    /// treads that are actually there.
    pub fn floor_at(&self, terrain: &crate::world::terrain::Terrain, at: Vec2) -> Option<f32> {
        if self.what.is_yard() || self.what.is_landmark() {
            return None;
        }
        let floor = FLOORS.get(self.what.figure())?;
        let half = self.what.footprint() * 0.5;
        // Into the building's own frame - the inverse of the turn `walls_into` uses.
        let (sin, cos) = self.facing.sin_cos();
        let away = at - self.at;
        let local = Vec2::new(away.x * cos + away.y * sin, -away.x * sin + away.y * cos);
        if local.x.abs() > half.x + floor.reach || local.y.abs() > half.y + floor.reach {
            return None;
        }
        let base = stands_at(terrain, self.at, self.what.footprint(), self.facing);
        if local.x.abs() <= half.x && local.y.abs() <= half.y {
            return Some(base + floor.top);
        }
        // The step, in front of the front wall and no wider than the treads.
        let out = -local.y - half.y;
        if out <= 0.0 || out > floor.reach || local.x.abs() > floor.wide * 0.5 {
            return None;
        }
        let ground = terrain.walk_height(at.x, at.y);
        let top = base + floor.top;
        Some(ground + (top - ground) * (1.0 - out / floor.reach.max(0.01)))
    }

    pub fn walls(&self) -> Vec<(Vec2, Vec2, f32)> {
        let mut walls = Vec::new();
        self.walls_into(&mut walls);
        walls
    }

    /// The same, added to a buffer somebody else owns.
    ///
    /// The movement path gathers what is standing near the warden every frame it
    /// moves, and every plot in reach used to hand back a freshly allocated `Vec` of
    /// five slabs to be copied into another one and dropped. It refills one buffer
    /// now. `walls` above is kept for the callers that just want the list.
    ///
    /// Found by Codex's audit.
    pub fn walls_into(&self, walls: &mut Vec<(Vec2, Vec2, f32)>) {
        // A yard's FENCE, if it has one, with the gateway left open.
        if self.what.is_yard() {
            let Some(fence) = self.what.fenced() else {
                return;
            };
            let half = self.what.footprint() * 0.5;
            let (sin, cos) = self.facing.sin_cos();
            let out = |local: Vec2| {
                self.at + Vec2::new(local.x * cos - local.y * sin, local.x * sin + local.y * cos)
            };
            let thick = 0.18;
            // The back run and both flanks.
            walls.push((out(Vec2::new(0.0, half.y)), Vec2::new(half.x, thick), self.facing));
            walls.push((out(Vec2::new(-half.x, 0.0)), Vec2::new(thick, half.y), self.facing));
            walls.push((out(Vec2::new(half.x, 0.0)), Vec2::new(thick, half.y), self.facing));
            // And the front, in two pieces with the gateway between them - unless
            // there is no front run to put a gateway in.
            let Fenced::Gated(gate) = fence else {
                return;
            };
            let stub = (half.x - gate * 0.5).max(0.0);
            if stub > 0.05 {
                for side in [-1.0_f32, 1.0] {
                    walls.push((
                        out(Vec2::new(side * (half.x - stub * 0.5), -half.y)),
                        Vec2::new(stub * 0.5, thick),
                        self.facing,
                    ));
                }
            }
            return;
        }
        let half = self.what.footprint() * 0.5;
        let (sin, cos) = self.facing.sin_cos();
        let out = |local: Vec2| {
            self.at + Vec2::new(local.x * cos - local.y * sin, local.x * sin + local.y * cos)
        };
        let thick = 0.3;

        // Back and both flanks: one slab each.
        walls.push((out(Vec2::new(0.0, half.y)), Vec2::new(half.x, thick), self.facing));
        for side in [-1.0_f32, 1.0] {
            walls.push((
                out(Vec2::new(side * half.x, 0.0)),
                Vec2::new(thick, half.y),
                self.facing,
            ));
        }

        // A LANDMARK IS SOLID. It has no door, so it gets no gap - a monument you
        // can walk into is a monument with a hole in it.
        if self.what.is_landmark() {
            walls.push((
                out(Vec2::new(0.0, -half.y)),
                Vec2::new(half.x, thick),
                self.facing,
            ));
            return;
        }

        // The front, in two pieces with the doorway between them.
        let door = self.what.walk_in() * 0.5;
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
    }
}

/// The clear opening `dev/art/town.py` builds, in metres.
///
/// There are two, because there are two kinds of way in: a doorway, and a tower's
/// LOBBY, which that file deliberately builds at 1.6 times a door because a city
/// block's entrance is a pair of piers and not a cottage door. Both are measured off
/// the built mesh into `assets/models/town.txt` and checked against these by
/// `the_doorway_you_can_see_is_the_one_you_can_walk_through`, so neither can drift
/// from the model without going red.
const DOORWAY: f32 = 1.9;
const LOBBY_DOORWAY: f32 = 3.04;

/// How much wider than the opening the collision gap is, in metres.
///
/// Wider on purpose: a gap exactly as wide as the opening leaves a warden aiming at
/// it with no tolerance, which reads as a door that sometimes refuses you. The extra
/// is invisible - the geometry either side of it is wall - and it is the difference
/// between walking in and fighting the frame.
///
/// # It has to actually contain the doorway
///
/// There used to be one number here, 2.2, described in a comment as a 1.4 m doorway
/// plus give. The doorway was 1.195 m, it was not centred - the bay grid put it at
/// +0.75 while this gap has always been centred on nought - and a tower's was 3.04.
/// So a quarter of a cottage's visible doorway was solid to the player, 1.25 m of
/// blank plaster beside it was not, and a city block had 42 cm of invisible wall
/// inside each edge of its own lobby.
const DOOR_GIVE: f32 = 0.3;

/// Lays the country roads around the player, and takes them up behind.
///
/// Keyed on a coarse cell rather than rebuilt every frame: the mesh is the same for
/// as long as the player is anywhere near the same place, and a road that is rebuilt
/// per frame is a road that flickers.
#[derive(Resource, Default)]
pub struct DirtLaid {
    cell: Option<IVec2>,
}

#[derive(Component)]
struct CountryRoad;

fn lay_the_country_roads(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<crate::shade::Shaded>>,
    mut surface: Local<Option<Handle<crate::shade::Shaded>>>,
    terrain: Res<TerrainSource>,
    mut laid: ResMut<DirtLaid>,
    anchors: Query<&GlobalTransform, With<StreamAnchor>>,
    standing: Query<Entity, With<CountryRoad>>,
) {
    let Some(anchor) = anchors.iter().next() else {
        return;
    };
    let here = Vec2::new(anchor.translation().x, anchor.translation().z);
    let cell = (here / (RAISES_WITHIN * 0.5)).floor().as_ivec2();
    if laid.cell == Some(cell) {
        return;
    }
    laid.cell = Some(cell);

    for entity in &standing {
        commands.entity(entity).despawn();
    }

    let roads = dirt_roads_near(terrain.plan(), &terrain.0, here);
    if roads.is_empty() {
        return;
    }
    let material = surface
        .get_or_insert_with(|| {
            materials.add(crate::shade::shaded(StandardMaterial {
                base_color: Color::WHITE,
                perceptual_roughness: 0.97,
                reflectance: 0.02,
                double_sided: true,
                cull_mode: None,
                ..default()
            }))
        })
        .clone();

    // ONE mesh, and the surface decides itself.
    //
    // This used to be two - a dirt run and a paved run, split by whether a leg's
    // middle stood on a city's ground - which put a hard material change at a leg
    // boundary out in open country. `pave` asks how paved each POINT is now, so the
    // same road becomes a street over the last stretch of its approach.
    if !roads.is_empty() {
        let mesh = pave(&roads, &terrain.0, here, 0.0);
        commands.spawn((
            CountryRoad,
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(material.clone()),
            Transform::from_xyz(here.x, 0.0, here.y),
            Visibility::default(),
            bevy::pbr::NotShadowCaster,
        ));
    }
}

pub struct TownPlugin;

impl Plugin for TownPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Built>()
            .init_resource::<DirtLaid>()
            .add_systems(
                Update,
                lay_the_country_roads.run_if(crate::build::a_world_is_up),
            )
            .add_systems(Update, raise_the_towns.run_if(crate::build::a_world_is_up));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(super) fn a_site(city: bool, radius: f32) -> Site {
        Site {
            at: Vec2::new(120.0, -80.0),
            height: 30.0,
            radius,
            city,
            ranch: false,
        }
    }

    /// No building stands in another building.
    ///
    /// # Checked with a different instrument than the one that places them
    ///
    /// `clear_of_buildings` decides this with the separating axis theorem, so a test
    /// that called it would be asking the placement to mark its own work - and the
    /// fault it exists to catch was precisely a rule everybody assumed was there.
    ///
    /// So this walks the CORNERS: every corner of every building, tested for being
    /// inside another building's rectangle. Different maths, same question. It misses
    /// only the case where two rectangles cross without any corner going inside -
    /// a perfect plus sign - which needs one building much longer than the other is
    /// wide, and the town has nothing that shape.
    #[test]
    fn no_building_stands_in_another_building() {
        // MANY LAYOUTS, not one. The first version of this checked a single village
        // and a single city and passed with the fix TAKEN OUT - the subdivision hands
        // out disjoint lots, so most layouts have no collision in them at all and a
        // test of one proves nothing. The fault needs a settlement whose square
        // happens to put the guild hall where a lot was going to be.
        for seed in 0..40u32 {
        for (city, radius) in [(true, 120.0_f32), (false, 70.0)] {
            let laid = lay_out(&a_site(city, radius), Vec2::new(0.6, -0.8).normalize(), seed);
            let solid: Vec<&Plot> = laid.plots.iter().filter(|p| !p.what.is_yard()).collect();

            let corners = |plot: &Plot| {
                let half = plot.what.footprint() * 0.5;
                let (sin, cos) = plot.facing.sin_cos();
                let across = Vec2::new(cos, sin);
                let door = Vec2::new(sin, -cos);
                [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)].map(
                    |(x, y): (f32, f32)| plot.at + across * (x * half.x) + door * (y * half.y),
                )
            };
            let inside = |plot: &Plot, point: Vec2| {
                let half = plot.what.footprint() * 0.5;
                let (sin, cos) = plot.facing.sin_cos();
                let away = point - plot.at;
                away.dot(Vec2::new(cos, sin)).abs() < half.x - 0.01
                    && away.dot(Vec2::new(sin, -cos)).abs() < half.y - 0.01
            };

            for (index, one) in solid.iter().enumerate() {
                for other in solid.iter().skip(index + 1) {
                    for corner in corners(one) {
                        assert!(
                            !inside(other, corner),
                            "a {:?} at {:?} has a corner inside a {:?} at {:?}",
                            one.what, one.at, other.what, other.at,
                        );
                    }
                    for corner in corners(other) {
                        assert!(
                            !inside(one, corner),
                            "a {:?} at {:?} has a corner inside a {:?} at {:?}",
                            other.what, other.at, one.what, one.at,
                        );
                    }
                }
            }
            assert!(solid.len() > 6, "only {} buildings were laid out", solid.len());
        }
        }
    }

    /// Every settlement the game actually ships has a guild hall.
    ///
    /// # A synthetic site is not the world
    ///
    /// The test below this one lays out a made-up village and a made-up city and
    /// checks each gets a hall. It passed while the hall was enlarged from 18 x 13.5
    /// to 26 x 18 - and the enlarged hall then failed to fit in a real village, which
    /// a photograph showed and the test did not. A circle of radius 55 at the origin
    /// has room the actual thirteen sites do not: their roads arrive from particular
    /// directions, their squares are the size the genre gives them, and their ground
    /// is levelled to a shape.
    ///
    /// So this asks the WORLD. It builds the real terrain, walks the real settlement
    /// plan with each site's real approach and the seed the game gives it, and names
    /// any that come out without a hall.
    /// A kerb is a step you can take, not a wall.
    ///
    /// # The rule that has to accept the geometry is the rule the geometry is checked
    /// against
    ///
    /// `KERB_RUN` is derived from `CLIMB_LIMIT` so that a 14 cm kerb leans back far
    /// enough to be walkable. Deriving it is not the same as checking it: the profile
    /// has four pieces and a hand-derived constant only governs one of them. This
    /// walks the whole section in two-centimetre steps and asks the same question
    /// `may_step` asks - because a kerb that refuses the player is an invisible wall
    /// down both sides of every street in every city, and nothing else would fail.
    /// A bend gets no junction patch; a crossing does.
    ///
    /// # The mesh test that could not have caught this
    ///
    /// `the_paving_faces_the_sky` checks every triangle points up, and both the
    /// footway and the patch laid over it point up - so a disc painting carriageway
    /// colour across a raised pavement is invisible to it, which is exactly Codex's
    /// point. What separates the two cases is not a normal, it is whether more than
    /// one road is there at all, so that is what is asserted.
    #[test]
    fn a_bend_is_not_a_junction_and_a_crossing_is() {
        // One road with two bends in it. `Way` mitres its own corners, so there is
        // nothing to fill and nothing should be emitted.
        let bent = Way {
            points: vec![
                Vec2::new(0.0, 0.0),
                Vec2::new(20.0, 4.0),
                Vec2::new(38.0, 16.0),
                Vec2::new(50.0, 34.0),
            ],
            wide: CITY_STREET_WIDE,
            joins: CITY_STREET_WIDE,
        };
        let capped = junctions_in(std::slice::from_ref(&bent));
        assert!(
            capped.is_empty(),
            "a single winding road was given {} junction patches, one at every bend",
            capped.len(),
        );

        // A second road ending on the first one's middle. That IS a junction.
        let joining = Way {
            points: vec![Vec2::new(38.0, 16.0), Vec2::new(38.0, -20.0)],
            wide: CITY_LANE_WIDE,
            joins: CITY_LANE_WIDE,
        };
        let met = junctions_in(&[bent, joining]);
        assert_eq!(met.len(), 1, "a crossroads got {} patches", met.len());
        assert!(
            met[0].at.distance(Vec2::new(38.0, 16.0)) < 0.6,
            "the patch landed at {:?} rather than where the roads meet",
            met[0].at,
        );
        // BOTH arms are kept, because a patch has to fit the narrower of them - see
        // `a_junction_patch_does_not_pave_any_arm_s_footway`.
        assert_eq!(met[0].arms.len(), 2, "the node forgot one of its roads");
    }

    /// A junction patch stays inside EVERY arm's carriageway, not just the widest.
    ///
    /// # The version of this test that passed while the fault was there
    ///
    /// It measured a 10 m patch against a 10 m road and an 8 m patch against an 8 m
    /// road, and both fitted. A junction is where roads of DIFFERENT sizes meet, and
    /// the patch was drawn at the widest arm's carriageway - so a high street meeting
    /// a lane put 3 m of carriageway into a road whose own is 2 m, a metre out across
    /// its pavement. Codex found the fault and the hole in the guard together, which
    /// is the more useful half: a test that only ever asks the easy case is a test
    /// that reports the answer you hoped for.
    #[test]
    fn a_junction_patch_does_not_pave_any_arm_s_footway() {
        let street = |wide: f32| Way {
            points: vec![Vec2::ZERO, Vec2::new(0.0, 40.0)],
            wide,
            joins: wide,
        };
        // A high street meeting a lane, which is the case that was wrong.
        let mixed = Meeting {
            at: Vec2::ZERO,
            arms: vec![
                (CITY_STREET_WIDE, CITY_STREET_WIDE),
                (CITY_LANE_WIDE, CITY_LANE_WIDE),
            ],
            whose: vec![0, 1],
        };
        for paved in [0.0_f32, 0.5, 1.0] {
            let reach = mixed.fills(paved);
            for (wide, joins) in &mixed.arms {
                let arm = RoadSection::new(*wide, *joins, paved, 1.0);
                assert!(
                    reach <= arm.carriage + 1.0e-4,
                    "at {paved} paved a patch of {reach:.2} m reaches past a {wide} m arm's \
                     {:.2} m carriageway and out onto its footway",
                    arm.carriage,
                );
            }
        }
        drop(street(CITY_STREET_WIDE));
    }

    /// A gateway junction resolves each arm by what it JOINS, not by what it is.
    ///
    /// A country road arriving at a city is 4.6 m widening to 10. A node that knew
    /// only the 4.6 would carve the footways back out of it - the pinched section
    /// `RoadSection` exists to prevent, reintroduced at the one place the two road
    /// kinds touch.
    #[test]
    fn a_gateway_junction_uses_what_each_arm_becomes() {
        let gateway = Meeting {
            at: Vec2::ZERO,
            arms: vec![
                (crate::config::ROAD_WIDE, CITY_STREET_WIDE),
                (CITY_STREET_WIDE, CITY_STREET_WIDE),
            ],
            whose: vec![0, 1],
        };
        // Fully paved, the country arm has BECOME the high street, so the patch is
        // the high street's carriageway and nothing is pinched.
        let full = gateway.fills(1.0);
        let street = RoadSection::new(CITY_STREET_WIDE, CITY_STREET_WIDE, 1.0, 1.0);
        assert!(
            (full - street.carriage).abs() < 1.0e-4,
            "at the gateway the patch is {full:.2} m and the street's carriageway is {:.2} m",
            street.carriage,
        );
        // And halfway in it is between the two, never narrower than the country road
        // it started as.
        let half = gateway.fills(0.5);
        let country = RoadSection::new(crate::config::ROAD_WIDE, CITY_STREET_WIDE, 0.0, 1.0);
        assert!(
            half >= country.carriage - 1.0e-4 && half <= full + 1.0e-4,
            "halfway through the gateway the patch is {half:.2} m, outside {:.2}..{full:.2}",
            country.carriage,
        );
    }

    #[test]
    fn a_kerb_is_a_step_and_not_a_wall() {
        for wide in [CITY_STREET_WIDE, CITY_LANE_WIDE] {
            let cut = RoadSection::new(wide, wide, 1.0, 1.0);
            assert!(cut.kerb > 0.05, "a {wide} m city street has no kerb at all");
            // THE WHOLE RISE OF THE KERB, against the rule that governs a step.
            //
            // This used to walk the profile in 2 cm steps and check each against
            // `CLIMB_LIMIT`, which is a gradient - and the only way a kerb passes a
            // gradient test is by not being a kerb. What has to be true is that the
            // warden can get UP it, and `may_step` allows that when the whole rise is
            // within `STEP_UP`.
            assert!(
                cut.kerb <= crate::player::STEP_UP,
                "a {wide} m street's kerb is {:.2} m and the warden can only step {:.2} - \
                 it is a wall down both sides of the road",
                cut.kerb,
                crate::player::STEP_UP,
            );
            // And nothing ELSE in the section is a climb: the crown and the shoulder
            // are ground, and they still have to be walkable as slopes.
            let step = 0.02;
            let mut across = cut.carriage + cut.batter + 0.01;
            while across < cut.shoulder {
                let rise = cut.lift(across + step) - cut.lift(across);
                assert!(
                    rise <= step * crate::player::CLIMB_LIMIT + 1.0e-4,
                    "a {wide} m street climbs {rise:.3} m in {step} m at {across:.2} out from \
                     its middle, past the kerb, where it should be flat footway",
                );
                across += step;
            }
        }
    }

    /// A country road grows into the street it joins, and its carriageway survives.
    ///
    /// The first footways were SUBTRACTED from the country road's existing 4.6 m, so
    /// at full paving the approach had two 2 m pavements around a 1.38 m carriageway -
    /// narrower than one cart - and then snapped to a 10 m street. Codex's research
    /// named the rule: add the urban right-of-way, do not carve it out of the rural
    /// one. This is that rule, as an assertion.
    #[test]
    fn a_country_road_widens_into_the_street_it_joins() {
        let mut widest = 0.0_f32;
        for tenth in 0..=10 {
            let paved = tenth as f32 / 10.0;
            let cut = RoadSection::new(crate::config::ROAD_WIDE, CITY_STREET_WIDE, paved, 1.0);
            assert!(
                cut.half >= widest - 1.0e-4,
                "the road NARROWS as it is paved: {:.2} m at {paved}",
                cut.half * 2.0,
            );
            widest = cut.half;
            assert!(
                cut.carriage * 2.0 >= 3.4,
                "at {paved} paved the carriageway pinches to {:.2} m, which is under one cart",
                cut.carriage * 2.0,
            );
        }
        // And it arrives as exactly the section it is joining.
        let arriving = RoadSection::new(crate::config::ROAD_WIDE, CITY_STREET_WIDE, 1.0, 1.0);
        let street = RoadSection::new(CITY_STREET_WIDE, CITY_STREET_WIDE, 1.0, 1.0);
        assert!(
            (arriving.half - street.half).abs() < 1.0e-4
                && (arriving.carriage - street.carriage).abs() < 1.0e-4,
            "an approach ends {:.2} m wide and the street it joins is {:.2} m",
            arriving.half * 2.0,
            street.half * 2.0,
        );
    }

    #[test]
    fn every_settlement_in_the_world_has_a_guild_hall() {
        let terrain = crate::world::terrain::Terrain::new();
        let plan = terrain.plan();
        let mut missing = Vec::new();
        let mut halls = 0;
        let mut counts: Vec<(bool, usize)> = Vec::new();
        for (key, site) in plan.sites().iter().enumerate() {
            if site.ranch {
                continue;
            }
            let laid = lay_out(
                site,
                plan.approach(site.at),
                crate::config::WORLD_SEED.wrapping_add(key as u32 * 7717),
            );
            let here = laid
                .plots
                .iter()
                .filter(|p| p.what == Building::GuildHall)
                .count();
            halls += here;
            counts.push((site.city, laid.plots.iter().filter(|p| !p.what.is_yard()).count()));
            if here != 1 {
                missing.push(format!(
                    "{} at {:.0},{:.0} radius {:.0} laid {here} halls",
                    if site.city { "city" } else { "village" },
                    site.at.x,
                    site.at.y,
                    site.radius,
                ));
            }
        }
        assert!(
            missing.is_empty(),
            "{} of the world's settlements have no guild hall:\n  {}",
            missing.len(),
            missing.join("\n  "),
        );
        assert!(halls > 8, "only {halls} guild halls in the whole world");

        // AND NONE OF THEM IS SPARSE.
        //
        // Widening a city's streets for footways takes ground from its lots, and the
        // only honest way to know whether that thinned the world is to count the
        // world. It did not: cities come out at 34 and 35 against a budget of 34,
        // villages at 15 and 16 against 16 - every settlement is still filling its
        // programme, because a real city is 232 m across and has the room.
        //
        // Floors rather than exact numbers, so a seed's roll can move a building
        // without anybody having to edit a test.
        for (city, count) in counts {
            let (kind, least) = if city { ("city", 30) } else { ("village", 13) };
            assert!(
                count >= least,
                "a {kind} came out with {count} buildings, which is sparse -                  something has taken its ground away",
            );
        }
    }

    #[test]
    fn every_settlement_has_exactly_one_guild_hall() {
        // This test used to be called `a_city_has_exactly_one_guild_hall_and_a_village_
        // has_none`, and it passed. The placement was `site.city`, so the guild whose
        // name the game carries had a branch in the four cities and in none of the
        // nine villages - and the test recorded that as the intent rather than as the
        // gap it was. A test can hold a decision in place long after anybody would
        // make it again; the name is the tell, because nobody would write that one
        // down as a design goal.
        for (city, radius) in [(true, 90.0_f32), (false, 55.0)] {
            let laid = lay_out(&a_site(city, radius), Vec2::X, 7);
            let halls = laid
                .plots
                .iter()
                .filter(|p| p.what == Building::GuildHall)
                .count();
            let kind = if city { "city" } else { "village" };
            assert_eq!(halls, 1, "a {kind} laid out {halls} guild halls");
            assert!(!laid.plots.is_empty(), "a {kind} got no buildings at all");
        }

        // And the ranch gets no hall. Only that: `lay_out` will lay a settlement out
        // on any site it is handed, and it is `raise_the_towns` that never hands it
        // the ranch - so asserting the ranch has no BUILDINGS here tests a promise
        // this function does not make, which is how the first version of this failed.
        let mut home = a_site(false, 55.0);
        home.ranch = true;
        assert!(
            !lay_out(&home, Vec2::X, 7)
                .plots
                .iter()
                .any(|p| p.what == Building::GuildHall),
            "the ranch was given a guild hall"
        );
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
                    // The city, in cooler greys - the plan should read as two ages
                    // at a glance, exactly as the world does.
                    Building::CitySpire => [236, 240, 246],
                    Building::CityTower => [186, 198, 212],
                    Building::CityBlock => [150, 164, 180],
                    // Landmarks, in the one colour nothing else wears.
                    Building::MarketCross | Building::Well | Building::Monument => {
                        [240, 122, 96]
                    }
                    // The yards, in a muted green so a plan reads at a glance as
                    // built ground against used ground.
                    Building::Garden | Building::Pen | Building::CityGreen => {
                        [122, 158, 104]
                    }
                    Building::WorkYard | Building::StoreYard | Building::CityService => {
                        [138, 132, 106]
                    }
                    Building::Stall | Building::CityKiosk => [190, 168, 112],
                    Building::CityForecourt => [176, 178, 180],
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
        let mut walls = Vec::new();
        built.walls_near(site.at, 60.0, &mut walls);
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

        let paving = pave(&layout.ways, &terrain, site.at, f32::from(u8::from(site.city)));
        let count = paving.count_vertices();
        assert!(count > 200, "the paving is {count} vertices, which is nothing");

        let Some(bevy::render::mesh::VertexAttributeValues::Float32x3(places)) =
            paving.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("the paving has no positions");
        };
        // IN A BAND, not at one height.
        //
        // This used to demand every vertex sit exactly `ROAD_LIES` over the ground,
        // which is what a flat ribbon does - and a flat ribbon is what made a road
        // read as a plank laid on a field, because its outer edge hung nine
        // centimetres in the air with a step and a shadow the length of it. The road
        // is crowned now: full lift down the middle, almost none at the shoulder.
        //
        // What still has to be true is that it never sinks into the ground and never
        // floats over it, and that is what this asks.
        let mut highest: f32 = 0.0;
        let mut lowest = f32::MAX;
        for place in places {
            let at = Vec2::new(place[0] + site.at.x, place[2] + site.at.y);
            let over = place[1] - terrain.drawn_height(at.x, at.y);
            highest = highest.max(over);
            lowest = lowest.min(over);
        }
        assert!(
            lowest > 0.0,
            "the paving sinks {:.3} m INTO the ground, which is a road you cannot see",
            -lowest,
        );
        assert!(
            highest < ROAD_LIES + ROAD_HEM + 0.01,
            "the paving stands {highest:.2} m off the ground it is laid on",
        );

        // And it really is crowned - the middle stands higher than the edge, which is
        // the whole of what stops the edge reading as a step.
        assert!(
            highest - lowest > ROAD_LIES * 0.5,
            "the paving is flat across its width: {lowest:.3} m at its lowest and {highest:.3} at its highest",
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
            if plot.what.is_landmark() {
                continue;
            }
            let district = plot.district;
            *counts
                .entry((district, plot.what))
                .or_insert(0usize) += 1;
        }

        // Of the BUILDINGS in a district, not of everything standing in it.
        //
        // Yards went into `plots` when the empty lots were given a use, and they
        // went straight into this denominator with them - so a market district whose
        // towers were unchanged reported its share of them falling from a third to a
        // ninth. Nothing about the districts had moved; the question had.
        let share = |district: District, what: Building| {
            let here: usize = counts
                .iter()
                .filter(|((d, w), _)| *d == district && !w.is_yard())
                .map(|(_, n)| *n)
                .sum();
            let this = counts.get(&(district, what)).copied().unwrap_or(0);
            if here == 0 { 0.0 } else { this as f32 / here as f32 }
        };

        // Every district has to exist at all.
        for district in [District::Market, District::Crafts, District::Outskirts] {
            // Buildings, matching the assertion's own words. Counting yards too
            // would let a district exist on gardens alone, which is not a district
            // with buildings in it - and the message would still say it was.
            let here: usize = counts
                .iter()
                .filter(|((d, w), _)| *d == district && !w.is_yard())
                .map(|(_, n)| *n)
                .sum();
            assert!(here > 3, "{district:?} has {here} buildings in it");
        }

        // And they have to be DIFFERENT. Trade at the middle, homes at the edge -
        // if a shop is as likely on the outskirts as on the square then the town
        // has one district wearing three names.
        // A CITY is modern, so its districts are told apart by HEIGHT rather than by
        // trade: towers at the middle, blocks at the rim. That is what a skyline is,
        // and a city whose every building is the same height reads as a housing
        // scheme however tall they all are. A village is still trade at the middle
        // and homes at the edge.
        let (tall, low) = if site.city {
            (Building::CityTower, Building::CityBlock)
        } else {
            (Building::Shop, Building::Cottage)
        };
        let shops_in = share(District::Market, tall);
        let shops_out = share(District::Outskirts, tall);
        let homes_in = share(District::Market, low);
        let homes_out = share(District::Outskirts, low);
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
        let mesh = pave(&layout.ways, &terrain, site.at, f32::from(u8::from(site.city)));
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

/// The ranch has the ranch on it and nothing else.
    ///
    /// "The ranch should just have the ranch no other buildings." It is a `Site` so
    /// that no settlement can take its ground, and the layout skips it - but a
    /// skipped site is invisible in every measurement unless something asks, and
    /// what went wrong the first time was that nothing did. This walks the whole
    /// world's settlements and refuses any building standing on the ranch.
    #[test]
    fn nothing_is_built_on_the_ranch() {
        let terrain = crate::world::terrain::Terrain::new();
        let plan = terrain.plan();
        let ranch = Vec2::new(crate::config::RANCH_AT.0, crate::config::RANCH_AT.1);

        let mut nearest = f32::MAX;
        for (index, site) in plan.sites().iter().enumerate() {
            if site.ranch {
                continue;
            }
            let layout = lay_out(
                site,
                plan.approach(site.at),
                crate::config::WORLD_SEED.wrapping_add(index as u32 * 7717),
            );
            for plot in &layout.plots {
                nearest = nearest.min(plot.at.distance(ranch));
            }
        }
        assert!(
            nearest > crate::config::RANCH_RADIUS,
            "a town building stands {nearest:.0} m from the ranch, which is levelled              out to {:.0} m - the ranch is not a settlement and nothing else may              stand on it",
            crate::config::RANCH_RADIUS
        );
        println!("the nearest town building is {nearest:.0} m from the ranch");
    }

#[test]
    #[ignore = "a measurement of the landmark filter"]
    fn why_the_junction_landmarks_vanish() {
        for city in [true, false] {
            let site = a_site(city, if city { 190.0 } else { 95.0 });
            let layout = lay_out(&site, Vec2::X, 5);
            let marks = layout.plots.iter().filter(|p| p.what.is_landmark()).count();
            let radials = layout
                .streets
                .iter()
                .filter(|s| s.wide < STREET_WIDE - 0.01)
                .count();
            let square = (site.radius * FILLS * 0.19).clamp(11.0, 17.0);
            let mut far = 0;
            for street in &layout.streets {
                if street.wide >= STREET_WIDE - 0.01 {
                    continue;
                }
                if street.from.distance(site.at) >= square * 1.4 {
                    far += 1;
                }
            }
            println!(
                "{}: {radials} lanes, {far} of them start further than {:.0} m out,                  {marks} landmarks placed",
                if city { "city   " } else { "village" },
                square * 1.4
            );
        }
    }

/// A town has landmarks, they are spread about, and a city has its weenie.
    ///
    /// Rogers' hub rules, as a guard rather than as a comment. All three parts of
    /// this have been broken at some point: landmarks that were built and never
    /// placed, a filter so tight that six of seven eligible junctions were rejected,
    /// and a spire that was modelled, exported, and never once stood up.
    #[test]
    fn a_town_has_landmarks_and_a_city_has_something_tall() {
        for seed in 0..12 {
            for city in [true, false] {
                let site = a_site(city, if city { 190.0 } else { 95.0 });
                let layout = lay_out(&site, Vec2::new(0.6, -0.8).normalize(), seed);

                let marks: Vec<&Plot> =
                    layout.plots.iter().filter(|p| p.what.is_landmark()).collect();
                assert!(
                    marks.len() >= 2,
                    "seed {seed}, city {city}: {} landmarks - a place with one node                      has nothing to navigate BY",
                    marks.len()
                );

                // Spread out, or they are street furniture rather than nodes.
                for (at, one) in marks.iter().enumerate() {
                    for other in marks.iter().skip(at + 1) {
                        let apart = one.at.distance(other.at);
                        assert!(
                            apart > 12.0,
                            "seed {seed}: two landmarks stand {apart:.1} m apart"
                        );
                    }
                }

                // AND THE THING YOU SEE FROM OUTSIDE, WHICH IS NOT THE HALL.
                //
                // This asked only that a city have a guild hall with room around it,
                // on the grounds that the hall was 80.5 m and therefore the thing you
                // navigate by. The hall is 12.7 m now and the test kept passing,
                // because what it actually checks - a hall exists, no tower stands
                // near it - is true of a hall of any height. A test can go on being
                // green long after the sentence it was written to defend stopped
                // being true.
                //
                // Both are asked for now, separately: something TALL for the skyline,
                // which is the spire, and the hall's own square at street level.
                if city {
                    let Some(hall) = layout
                        .plots
                        .iter()
                        .find(|p| p.what == Building::GuildHall)
                    else {
                        panic!("seed {seed}: a city with no guild hall - nothing to see it by from the road in");
                    };
                    let crowding = layout
                        .plots
                        .iter()
                        .filter(|p| {
                            matches!(p.what, Building::CityTower | Building::CitySpire)
                                && p.at.distance(hall.at) < KEEPS_CLEAR
                        })
                        .count();
                    assert_eq!(
                        crowding, 0,
                        "seed {seed}: {crowding} tower(s) stand inside the guild hall's square, so it is read against a building instead of against sky",
                    );
                    assert!(
                        layout
                            .plots
                            .iter()
                            .any(|p| p.what == Building::CitySpire),
                        "seed {seed}: a city with nothing tall in it - there is no spire to see it by from outside",
                    );
                }
            }
        }
    }

/// Every model a building names is actually on disk.
    ///
    /// The cheapest possible version of the lesson this module keeps relearning:
    /// ask the ARTEFACT. A `Building` that names a file nobody exported is a
    /// building that silently does not appear, and everything upstream of it -
    /// the layout, the plots, the collision - measures perfectly correct while the
    /// town comes out empty.
    #[test]
    fn every_building_has_a_model_on_disk() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
        // EVERY kind, from the one list - see `Building::ALL`. Written out by hand
        // here once, and five yards were added to the enum without being added to
        // it, so the guard that proves a `Building` names a file that exists quietly
        // stopped covering a third of them.
        for what in Building::ALL {
            let path = root.join(what.model());
            assert!(
                path.exists(),
                "{what:?} names {} and nothing is there - it would not appear in the                  world, and every measurement upstream of it would still be right",
                what.model()
            );
        }
    }

/// Standing AT THE RANCH raises nothing at all.
    ///
    /// The previous guard walked the settlements and measured the distance from the
    /// ranch to their buildings - and never asked the only question that matters,
    /// which is whether the ranch's OWN site gets built on. It passed while a market
    /// cross stood on the spawn point with the player wedged inside it.
    ///
    /// This stands an app at the ranch and counts what came up.
    #[test]
    fn standing_at_the_ranch_raises_nothing() {
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
        let ranch = Vec2::new(crate::config::RANCH_AT.0, crate::config::RANCH_AT.1);
        app.insert_resource(crate::world::terrain::TerrainSource(std::sync::Arc::new(
            terrain,
        )));
        app.world_mut().spawn((
            StreamAnchor,
            Transform::from_xyz(ranch.x, 0.0, ranch.y),
            GlobalTransform::from_xyz(ranch.x, 0.0, ranch.y),
        ));
        app.update();

        let mut standing = app.world_mut().query::<(&Standing, &Transform)>();
        let near: Vec<f32> = standing
            .iter(app.world())
            .map(|(_, at)| Vec2::new(at.translation.x, at.translation.z).distance(ranch))
            .collect();
        let closest = near.iter().cloned().fold(f32::MAX, f32::min);
        assert!(
            near.is_empty() || closest > crate::config::RANCH_RADIUS,
            "{} buildings were raised standing at the ranch, the nearest {closest:.0} m              away - the ranch is not a settlement and the player SPAWNS here",
            near.len()
        );
        println!("standing at the ranch raised {} buildings", near.len());
    }

    #[test]
    fn no_building_stands_in_a_road() {
        // The other half of the same complaint, and the one nothing tested: a door
        // can face a street perfectly while the building's far corner sits in a
        // different street. That is what `footprint().length() * 0.55` allowed - it
        // reserved a little over half the footprint's DIAGONAL against a road the
        // building met side-on with its full half WIDTH.
        //
        // Asked of the CARRIAGEWAY rather than of the placement rule: walk each
        // street between its kerbs and check that no point of road is inside any
        // building. A guard that reruns the rule it is guarding cannot fail.
        for seed in 0..30 {
            let site = a_site(seed % 2 == 0, 85.0);
            let layout = lay_out(&site, Vec2::new(0.7, -0.7).normalize(), seed);
            for plot in &layout.plots {
                let half = plot.what.footprint() * 0.5;
                let (sin, cos) = plot.facing.sin_cos();
                let across = Vec2::new(cos, sin);
                let door = Vec2::new(sin, -cos);
                for street in &layout.streets {
                    let run = street.to - street.from;
                    let steps = (run.length() / 1.0).ceil().max(1.0) as usize;
                    let side = run.normalize_or_zero().perp();
                    for step in 0..=steps {
                        let on = street.from + run * (step as f32 / steps as f32);
                        for kerb in [-1.0_f32, 0.0, 1.0] {
                            let point = on + side * (kerb * street.wide * 0.5);
                            let local = point - plot.at;
                            let inside = local.dot(across).abs() < half.x
                                && local.dot(door).abs() < half.y;
                            assert!(
                                !inside,
                                "seed {seed}: a {:?} stands in a road - the         carriageway runs {:.1} m inside its walls",
                                plot.what,
                                (half.x - local.dot(across).abs())
                                    .min(half.y - local.dot(door).abs()),
                            );
                        }
                    }
                }
            }
        }
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
                // A landmark stands in the open ON a node - it has no frontage and
                // faces nothing, which is exactly what makes it a landmark rather
                // than a bigger house.
                if plot.what.is_landmark() {
                    continue;
                }
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
        // AT THE SIZE THE GAME'S OWN SETTLEMENTS ARE.
        //
        // This laid a city out at radius 90 and a village at 58. The world's are 232
        // and 116 - so for the life of this test the "city" it defended was a quarter
        // the area of any city the game has, and its floor of eighteen buildings was
        // calibrated against that. Widening the streets for footways pushed the toy
        // city under the floor while every real one was still comfortably over it,
        // which is a test failing for a shape of city nobody will ever walk through.
        for seed in 0..30 {
            let city = lay_out(
                &a_site(true, crate::config::CITY_RADIUS),
                Vec2::new(0.8, 0.6).normalize(),
                seed,
            );
            // BUILDINGS, not everything standing. Yards went into `plots` and
            // straight into this count, so a settlement whose houses had collapsed
            // toward zero could still sail through on the strength of its gardens -
            // which is exactly the vacuous pass this test exists to prevent.
            let houses = |layout: &Layout| {
                layout.plots.iter().filter(|p| !p.what.is_yard()).count()
            };
            assert!(
                houses(&city) >= 18,
                "seed {seed}: a city has {} buildings in it",
                houses(&city)
            );
            let village = lay_out(
                &a_site(false, crate::config::TOWN_RADIUS),
                Vec2::new(0.3, -0.95).normalize(),
                seed,
            );
            assert!(
                houses(&village) >= 6,
                "seed {seed}: a village has {} buildings in it",
                houses(&village)
            );

            // And the yards are BOUNDED by them, which is the whole point of the
            // budget: the size of a settlement is a property of the settlement, not
            // of how many provisional lots the street generator happened to make.
            for (what, layout) in [("city", &city), ("village", &village)] {
                let yards = layout.plots.iter().filter(|p| p.what.is_yard()).count();
                let built = houses(layout);
                assert!(
                    yards <= built,
                    "seed {seed}: a {what} has {yards} yards to {built} buildings - the yards are running away with it",
                );
            }
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

#[cfg(test)]
mod doorstep {
    use super::*;

    /// `ALL` really is every kind.
    ///
    /// `Building::place` is an exhaustive match, so a new variant cannot be added
    /// without being given a place; this then fails until it has been added to `ALL`
    /// and `KINDS` as well. The two together are what make the artefact guard
    /// exhaustive - `ALL` on its own is a list somebody has to remember.
    #[test]
    fn the_list_of_kinds_is_every_kind() {
        let mut seen = vec![None; Building::KINDS];
        for what in Building::ALL {
            let place = what.place();
            assert!(
                place < Building::KINDS,
                "{what:?} sits at {place}, past the end of a list of {}",
                Building::KINDS,
            );
            assert!(
                seen[place].is_none(),
                "{what:?} and {:?} both claim place {place}",
                seen[place].unwrap(),
            );
            seen[place] = Some(what);
        }
        for (place, what) in seen.iter().enumerate() {
            assert!(
                what.is_some(),
                "nothing in ALL sits at place {place} - a kind has a place and is not on the list",
            );
        }
    }

    /// A colour written in sRGB arrives as the light it should be.
    ///
    /// The whole of the road-colour trouble was that a bare `[f32; 4]` says nothing
    /// about which space it is in, and every one of them was written in the wrong
    /// one. `srgb` is the fix; this is the check that `srgb` is itself right, against
    /// values anybody can verify by hand.
    #[test]
    fn a_colour_written_in_srgb_arrives_linear() {
        // Black and white are the same in both spaces, and mid grey is famously not.
        assert!(srgb(0.0, 0.0, 0.0)[0].abs() < 1.0e-6);
        assert!((srgb(1.0, 1.0, 1.0)[0] - 1.0).abs() < 1.0e-6);
        let mid = srgb(0.5, 0.5, 0.5)[0];
        assert!(
            (mid - 0.2140).abs() < 1.0e-3,
            "sRGB 0.5 is linear 0.214 and this makes it {mid:.4}",
        );
        // The one that started it: the street is much darker than its number reads.
        let street = srgb(0.42, 0.41, 0.40);
        assert!(
            street[0] < 0.42 * 0.5,
            "sRGB 0.42 should arrive well under half of itself, not {:.3}",
            street[0],
        );
        // And the alpha is opaque, because a road is.
        assert_eq!(srgb(0.3, 0.3, 0.3)[3], 1.0);
    }

    /// Blender and the game agree about the wall a lit window hangs on.
    ///
    /// The panes used to be placed against the collision footprint, which is bigger
    /// than the building on purpose, so they floated off the glass and past the
    /// corners. Numbers this specific drift the moment somebody widens a figure, so
    /// Blender writes down what it built and this reads it back.
    #[test]
    fn the_facades_are_the_size_the_game_thinks_they_are() {
        let note = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("assets/models/town.txt");
        let said = std::fs::read_to_string(&note)
            .unwrap_or_else(|_| panic!("run dev/art/build.sh: {} is missing", note.display()));
        for (name, what) in [
            ("city_block", Building::CityBlock),
            ("city_tower", Building::CityTower),
            ("city_spire", Building::CitySpire),
        ] {
            let line = said
                .lines()
                .find_map(|line| line.strip_prefix(&format!("FACADE {name} ")))
                .unwrap_or_else(|| panic!("{name} has no facade in {}", note.display()));
            let said: Vec<f32> = line
                .split_whitespace()
                .map(|n| n.parse().expect("a number"))
                .collect();
            let (wide, deep, storeys) = what.facade().expect("a city figure has a facade");
            assert!(
                (said[0] - wide).abs() < 1.0e-3 && (said[1] - deep).abs() < 1.0e-3,
                "{name} is built {} x {} and the game hangs windows on {wide} x {deep}",
                said[0],
                said[1],
            );
            assert_eq!(
                said[2] as usize, storeys,
                "{name} has {} glazed storeys and the game lights {storeys}",
                said[2],
            );
            // `storeys` used to be a second list of these numbers and is now the
            // same one, so there is nothing left here to disagree.
        }
    }

    /// A model's door lands where the doorway is.
    ///
    /// # The one thing nothing compared
    ///
    /// Doors were 3.5 m from a kerb and `every_building_faces_a_street` passed on
    /// every seed, and the door was still on the back of the building - because both
    /// of those ask the LOT which way its frontage looks, and neither asks the MODEL
    /// which way its door was built. Photographed from above, the cottage's doorstep
    /// was on the far side of it from the road.
    ///
    /// This closes the loop: Blender writes down which way it built the door, and
    /// the turn the game applies has to bring it round to the gap `Plot::walls`
    /// leaves. Nothing here is a restatement of the placement rule.
    #[test]
    fn a_models_door_lands_where_the_doorway_is() {
        let note = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("assets/models/town.txt");
        let said = std::fs::read_to_string(&note)
            .unwrap_or_else(|_| panic!("run dev/art/build.sh: {} is missing", note.display()));
        let built: f32 = said
            .lines()
            .find_map(|line| line.strip_prefix("DOOR_ON_BLENDER_Y "))
            .expect("which way Blender built the door")
            .trim()
            .parse()
            .expect("a number");

        // Blender Z-up to glTF Y-up: (x, y, z) becomes (x, z, -y). So a door built
        // facing Blender -Y arrives facing +Z.
        let in_model = Vec3::new(0.0, 0.0, -built);

        for facing in [0.0_f32, 0.7, 1.9, 3.0, -2.2, -0.4] {
            let turned = Quat::from_rotation_y(model_turn(facing)) * in_model;
            let shows = Vec2::new(turned.x, turned.z);
            // Where `Plot::walls` leaves the gap.
            let gap = Vec2::new(facing.sin(), -facing.cos());
            assert!(
                shows.distance(gap) < 1.0e-4,
                "facing {facing:.2}: the model's door points {shows:?} and the doorway is at {gap:?} - the door is on the wrong wall",
            );
        }
    }

    // ------------------------------------------------------------ THE COTTAGE'S PLAN

    /// What `dev/art/town.py` measured off the cottage it actually built.
    ///
    /// # Why the plan is a file and not a constant
    ///
    /// The game cannot see inside a `.glb`, and the checks below are about
    /// RELATIONSHIPS - does the flue come down on the fire, is the bed out of the way
    /// in - which no amount of looking at the outside of a model answers. Blender
    /// measures the mesh it built and writes the answer down; this reads it back and
    /// checks it against what the game itself does.
    ///
    /// The two halves of the contract are deliberately on opposite sides of the
    /// build: Blender proves the geometry matches its own plan, and this proves the
    /// plan matches the game. A guard that compares a number against the thing that
    /// produced it proves nothing, which this project has already learnt the hard way.
    struct Plan {
        doors: Vec<(String, f32, f32)>,
        rects: Vec<(String, [f32; 4])>,
        spots: Vec<(String, Vec2)>,
    }

    impl Plan {
        fn read() -> Self {
            let note = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("assets/models/town.txt");
            let said = std::fs::read_to_string(&note)
                .unwrap_or_else(|_| panic!("run dev/art/build.sh: {} is missing", note.display()));
            let number = |line: &str, tag: &str| -> Option<f32> {
                line.strip_prefix(tag)?.trim().parse().ok()
            };
            let mut plan = Plan { doors: Vec::new(), rects: Vec::new(), spots: Vec::new() };
            for line in said.lines() {
                if let Some(rest) = line.strip_prefix("DOORWAY ") {
                    let mut word = rest.split_whitespace();
                    let figure = word.next().expect("a figure").to_string();
                    let said: Vec<f32> = word.filter_map(|n| n.parse().ok()).collect();
                    assert_eq!(said.len(), 2, "DOORWAY {figure} wants a middle and a width");
                    plan.doors.push((figure, said[0], said[1]));
                } else if let Some(rest) = line.strip_prefix("COTTAGE ") {
                    let mut word = rest.split_whitespace();
                    let name = word.next().expect("a name").to_string();
                    let said: Vec<f32> = word.filter_map(|n| n.parse().ok()).collect();
                    match said.len() {
                        4 => plan.rects.push((name, [said[0], said[1], said[2], said[3]])),
                        2 => plan.spots.push((name, Vec2::new(said[0], said[1]))),
                        _ => {}
                    }
                }
            }
            assert!(!plan.doors.is_empty(), "town.txt has no measured doorway - run dev/art/build.sh");
            plan
        }

        /// Which building kind a Blender figure is built for.
        fn kind(figure: &str) -> Building {
            match figure {
                "cottage" => Building::Cottage,
                "townhouse" => Building::Townhouse,
                "shop" => Building::Shop,
                "guild_hall" => Building::GuildHall,
                "city_block" => Building::CityBlock,
                "city_tower" => Building::CityTower,
                "city_spire" => Building::CitySpire,
                other => panic!("town.txt names a figure the game has no kind for: {other}"),
            }
        }

        /// The cottage's own doorway, which its plan is laid out around.
        fn cottage_door(&self) -> (f32, f32) {
            self.doors
                .iter()
                .find(|(figure, ..)| figure == "cottage")
                .map(|(_, middle, clear)| (*middle, *clear))
                .expect("town.txt has no cottage doorway")
        }

        fn rect(&self, name: &str) -> [f32; 4] {
            self.rects
                .iter()
                .find(|(had, _)| had == name)
                .unwrap_or_else(|| panic!("the cottage plan has no {name}"))
                .1
        }

        fn every(&self, name: &str) -> Vec<Vec2> {
            self.spots.iter().filter(|(had, _)| had == name).map(|(_, at)| *at).collect()
        }

        fn spot(&self, name: &str) -> Vec2 {
            let found = self.every(name);
            assert_eq!(found.len(), 1, "the cottage plan has {} {name}s", found.len());
            found[0]
        }
    }

    /// Do two footprints share any ground?
    fn overlap(a: [f32; 4], b: [f32; 4]) -> bool {
        a[0] < b[2] && a[2] > b[0] && a[1] < b[3] && a[3] > b[1]
    }

    /// Is a point inside a footprint?
    fn holds(rect: [f32; 4], at: Vec2) -> bool {
        at.x >= rect[0] && at.x <= rect[2] && at.y >= rect[1] && at.y <= rect[3]
    }

    /// The doorway you can see is the doorway you can walk through. Every one of them.
    ///
    /// # The one the player would have felt
    ///
    /// Everything else here is a fault you can see. This is one you could only feel:
    /// a cottage's visible opening ran from +0.16 to +1.35 and the gap `Plot::walls`
    /// leaves runs from -1.10 to +1.10, so a quarter of the doorway was solid and
    /// there was over a metre of walk-through plaster beside it.
    ///
    /// It survived because the two are described in different languages by different
    /// tools - a bay index in a Python split grammar, and a symmetric pair of boxes
    /// in Rust - and nothing had ever put the two numbers side by side.
    ///
    /// EVERY figure, because the fault was in the grammar rather than in one house:
    /// a check that only looked at the cottage would have gone green with a guild
    /// hall still refusing the player at its own front door.
    ///
    /// Blender's +X arrives as the plot's -x once the model is turned to face its
    /// street; a doorway is symmetric about the middle of its wall, so only the
    /// distance matters, but that is a reason and not an accident.
    #[test]
    fn the_doorway_you_can_see_is_the_one_you_can_walk_through() {
        let plan = Plan::read();
        assert!(!plan.doors.is_empty(), "town.txt lists no doorways at all");
        for (figure, middle, clear) in &plan.doors {
            let what = Plan::kind(figure);
            let gap = what.walk_in();
            let reach = middle.abs() + clear * 0.5;
            assert!(
                reach <= gap * 0.5,
                "{figure}'s built doorway runs to {reach:.3} m from the middle of its wall \
                 and the collision gap only reaches {:.3} - part of the way in that the \
                 player can see is solid to them",
                gap * 0.5,
            );
            // And the gap is not so much wider than the opening that you walk
            // through wall to get to it.
            assert!(
                gap - clear < 0.7,
                "{figure}'s collision gap is {:.2} m wider than its {clear:.2} m opening - \
                 that much of the wall either side is not there",
                gap - clear,
            );
        }
    }

    /// The flue comes down onto its own fire.
    ///
    /// The cottage's stack stood 2.5 m from its fireplace and the townhouse's stood at
    /// the opposite corner of the house, because each was one expression and the fire
    /// was a different one. See `fireside` in `dev/art/town.py`.
    #[test]
    fn the_chimney_comes_down_onto_its_own_fire() {
        let plan = Plan::read();
        let away = plan.spot("HEARTH").distance(plan.spot("CHIMNEY"));
        assert!(
            away < 0.4,
            "the chimney stands {away:.2} m from the fireplace it is supposed to carry",
        );
    }

    /// The front windows light the room people sit in.
    ///
    /// A window is only worth cutting if it lights somewhere somebody is. These have
    /// to reach the COMMON room - the interior less the sleeping alcove - which is
    /// what putting the alcove at the back of the plan buys and what a plan that moved
    /// it forward would immediately lose.
    #[test]
    fn the_front_windows_light_the_room_people_sit_in() {
        let plan = Plan::read();
        let (inner, alcove) = (plan.rect("INNER"), plan.rect("ALCOVE"));
        let windows = plan.every("FRONT_WINDOW");
        assert!(!windows.is_empty(), "the cottage has no front windows");
        for at in windows {
            // Just inside the glass, which is where the light lands.
            let lands = Vec2::new(at.x, inner[1] + 0.3);
            assert!(
                holds(inner, lands) && !holds(alcove, lands),
                "the front window at {at:?} looks into {lands:?}, which is not the common room",
            );
        }
    }

    /// And nobody sleeps in a cupboard.
    #[test]
    fn the_alcove_has_a_window_of_its_own() {
        let plan = Plan::read();
        let alcove = plan.rect("ALCOVE");
        let lit = plan.every("ALCOVE_WINDOW");
        assert!(!lit.is_empty(), "the sleeping alcove has no window");
        for at in lit {
            assert!(
                at.x >= alcove[0] && at.x <= alcove[2],
                "the window at {at:?} was supposed to light the alcove and is not on it",
            );
        }
    }

    /// The bed is not in the way in, and neither is anything else.
    ///
    /// # Circulation is reserved before rooms, not threaded through them afterwards
    ///
    /// The route from the door to the fire is decided first and nothing is allowed to
    /// stand in it; so is the standing room in front of the fire, which is a room's
    /// second anchor. Furniture is placed last, around both. Done the other way round
    /// - put the furniture down, then hope there is a way past it - is how a cottage
    /// ends up with a bed in its hall, and the old one had its bed 1.1 m inside the
    /// front door.
    #[test]
    fn the_way_in_and_the_fireside_are_left_clear() {
        let plan = Plan::read();
        let route = plan.rect("ROUTE");
        let apron = plan.rect("APRON");
        for name in ["BED_RECT", "TABLE_RECT"] {
            let stands = plan.rect(name);
            assert!(!overlap(stands, route), "{name} stands in the way in from the door");
            assert!(!overlap(stands, apron), "{name} stands in the fire's own floor");
        }
        assert!(!overlap(route, apron), "the way in and the fireside are the same floor");
        // The way in is at least as wide as the door that opens onto it.
        let (_, clear) = plan.cottage_door();
        assert!(
            route[2] - route[0] >= clear - 0.01,
            "the way in is {:.2} m across and the door is {clear:.2} m wide",
            route[2] - route[0],
        );
    }


    /// The windows the game lights are the windows the model has.
    ///
    /// # Two measurements of one thing, taken apart
    ///
    /// `dev/art/town.py` works out where the cottage's windows go from its bay grid,
    /// writes that into the plan, and separately MEASURES the glass it ended up
    /// building and writes that too. Those are independent: the first is what the
    /// plan intended and the second is what came out of the mesh.
    ///
    /// Comparing them is the only check here that is not a number against itself.
    /// The game cannot verify a window position on its own - it has no idea where
    /// the glass is, which is precisely why it used to invent one and light the
    /// plaster instead.
    #[test]
    fn the_lit_panes_are_where_the_glass_is() {
        let plan = Plan::read();
        let (panes, _) = crate::world::lamp::windows_of(Building::Cottage)
            .expect("the cottage's windows to be measured - run dev/art/build.sh");

        // Blender builds Z-up and the export turns it Y-up: a window at Blender
        // (x, y) arrives at game (x, _, -y). It is also stood a little proud of the
        // wall on the way out, which is the only difference allowed.
        for (name, wall) in [("FRONT_WINDOW", -1.0_f32), ("ALCOVE_WINDOW", 1.0)] {
            for want in plan.every(name) {
                let landed = panes
                    .iter()
                    .find(|pane| (pane.at.x - want.x).abs() < 1.0e-3
                        && (pane.at.z + want.y).abs() < 0.2)
                    .unwrap_or_else(|| {
                        panic!(
                            "the plan puts a {name} at {want:?} and no glass was built there - \
                             the panes are at {:?}",
                            panes.iter().map(|pane| pane.at).collect::<Vec<_>>(),
                        )
                    });
                let proud = (landed.at.z.abs() - want.y.abs()) * wall.signum();
                assert!(
                    (0.0..0.1).contains(&proud.abs()),
                    "the {name} pane stands {proud:.3} m off its own wall",
                );
            }
        }
    }

    /// And none of them hangs off the end of the building.
    ///
    /// # What this is allowed to claim
    ///
    /// It is not a strong check and it must not pretend to be: a pane can be on the
    /// right wall and still in the wrong place along it, which is exactly what the old
    /// code did. The conversion between Blender's frame and the game's is guarded by
    /// `the_lit_panes_are_where_the_glass_is` above, which has two measurements to
    /// compare. This has one, and the only thing it can say for certain is that a
    /// window is not somewhere the building is not.
    ///
    /// It cannot even say a window is ON a wall. It tried, and the guild hall failed
    /// it: its tower is set back well inside the hall's footprint and carries its own
    /// windows fifteen metres up. That is a building with inner walls, which
    /// `footprint` knows nothing about.
    /// How far past its ground footprint a wall is allowed to carry a window.
    ///
    /// Generous, and it has to be: the townhouse's upper storey is JETTIED - it
    /// oversails the floor below by 28 cm, which is most of what makes it read as a
    /// town house rather than a two-storey box - so its first-floor windows sit
    /// genuinely outside the ground the building stands on. This caught that on its
    /// first run, which is the right answer to the wrong question.
    ///
    const OVERSAILS: f32 = 0.45;

    #[test]
    fn every_lit_pane_stands_on_a_wall_of_its_own_building() {
        for what in Building::ALL {
            let Some((panes, storeys)) = crate::world::lamp::windows_of(what) else {
                continue;
            };
            let half = what.footprint() * 0.5;
            for pane in panes {
                assert!(
                    pane.storey < storeys,
                    "{what:?} has a window on storey {} of {storeys}",
                    pane.storey,
                );
                // The glass sits in a wall, so one of the two horizontal distances
                // has to be the wall's own face, and neither may be past it.
                let out = Vec2::new(pane.at.x.abs() - half.x, pane.at.z.abs() - half.y);
                assert!(
                    out.x < OVERSAILS && out.y < OVERSAILS,
                    "{what:?} has a window at {:?}, off the end of a {half:?} footprint",
                    pane.at,
                );
            }
        }
    }


    /// A yard's collision is the fence the model has, and nothing more.
    ///
    /// # Invisible walls across an open mouth
    ///
    /// `fenced` used to answer with a gate width alone, so every fenced yard was
    /// taken to have four runs. The city's service bay has three - `city_service`
    /// builds both flanks and the back and no front - and the game fenced its open
    /// frontage anyway, leaving the player walking into nothing across most of a bay
    /// they can see straight through.
    ///
    /// # What this can and cannot say
    ///
    /// It checks the shape of what comes out against what the programme DECLARES: an
    /// open-fronted yard must produce no wall across its front, a gated one must
    /// produce some. Declaring the service bay gated again passes it, because then
    /// the walls are correct for the declaration.
    ///
    /// So this cannot catch a programme declared wrongly - only the model knows, and
    /// nothing has measured the fence runs the way the windows are now measured. What
    /// it does buy is that the two cases have to be NAMED, so a yard can no longer
    /// have four runs assumed of it because it stated a gate width. Closing the rest
    /// of the loop is a `yard.txt` and its own piece of work.
    #[test]
    fn an_open_fronted_yard_has_nothing_across_its_mouth() {
        for what in Building::ALL {
            let Some(fence) = what.fenced() else {
                continue;
            };
            let half = what.footprint() * 0.5;
            let plot = Plot {
                at: Vec2::ZERO,
                district: District::Outskirts,
                // Facing +y, so the yard's own front is the world's -y and the
                // arithmetic below reads as it does in `walls_into`.
                facing: 0.0,
                what,
            };
            let across_the_front = plot
                .walls()
                .iter()
                .filter(|(at, _, _)| (at.y + half.y).abs() < 0.1)
                .count();
            match fence {
                Fenced::OpenFronted => assert_eq!(
                    across_the_front, 0,
                    "{what:?} is open fronted and the game fences it with \
                     {across_the_front} slabs",
                ),
                Fenced::Gated(gate) => assert!(
                    across_the_front > 0 || gate >= half.x * 2.0,
                    "{what:?} has a {gate} m gateway and no front fence either side of it",
                ),
            }
        }
    }

    /// A rear opening, if the cottage ever gets one, has to reach the yard.
    ///
    /// It has none today, and that is a decision rather than an oversight: a second
    /// doorway needs a matching gap in `Plot::walls` and a yard proven reachable
    /// behind it, which is its own piece of work. The check is written now so that
    /// adding one cannot quietly skip either - a door onto the back of the collision
    /// box is a door the player can see and never use, which is the same fault this
    /// build has just finished paying for at the front.
    #[test]
    fn a_rear_opening_would_reach_the_yard() {
        let plan = Plan::read();
        let Some(rear) = plan.every("REAR").first().copied() else {
            return;
        };
        let (inner, alcove) = (plan.rect("INNER"), plan.rect("ALCOVE"));
        let lands = Vec2::new(rear.x, inner[3] - 0.3);
        assert!(
            holds(inner, lands) && !holds(alcove, lands),
            "the rear door opens out of the sleeping alcove rather than the common room",
        );
        // And then the half nobody would remember on their own. `Plot::walls` builds
        // the back of every building as ONE SOLID SLAB - see the `walls` above - so a
        // rear door drawn in Blender today is a door the player can see and can never
        // open. That is the fault this whole build has just finished paying for at the
        // front of the house, and it costs nothing to refuse it in advance.
        panic!(
            "the cottage plan declares a rear opening at {rear:?}, and `Plot::walls` still \
             builds the back of a building as one solid slab. Split it the way the front \
             is split before this ships.",
        );
    }
}

#[cfg(test)]
mod facing {
    use super::tests::a_site;
    use super::*;

    /// Every triangle of paving faces the sky.
    ///
    /// # A surface facing the wrong way is lit by nothing
    ///
    /// The whole ribbon was wound face DOWN while carrying normals that said up.
    /// That was known about - it is why the material disables culling, which was the
    /// fix for the road being invisible - but drawing a back face does not make it
    /// face the right way.
    ///
    /// It hid for months because ambient sky light does not care which way a surface
    /// points: a road lit only by ambient looks flat but fine. A point light cares
    /// about nothing else, so the night the lamps went in, every one of them lit the
    /// ground beside the road and left the road itself black.
    ///
    /// Measured, not argued: this takes the cross product of each triangle's own
    /// edges. It has caught two separate windings - the ribbon and the junction
    /// discs, which were 670 triangles still facing down after the ribbon was fixed.
    #[test]
    fn the_paving_faces_the_sky() {
        use bevy::render::mesh::{Indices, VertexAttributeValues};
        let terrain = crate::world::terrain::Terrain::new();
        for city in [false, true] {
            let site = a_site(city, if city { 120.0 } else { 70.0 });
            let layout = lay_out(&site, Vec2::new(0.7, -0.7).normalize(), 3);
            let mesh = pave(&layout.ways, &terrain, site.at, f32::from(u8::from(city)));
            let Some(VertexAttributeValues::Float32x3(places)) =
                mesh.attribute(Mesh::ATTRIBUTE_POSITION)
            else {
                panic!("the paving has no positions");
            };
            let Some(Indices::U32(index)) = mesh.indices() else {
                panic!("the paving has no indices");
            };
            let down = index
                .chunks(3)
                .filter(|tri| {
                    let p = |i: u32| Vec3::from(places[i as usize]);
                    let (a, b, c) = (p(tri[0]), p(tri[1]), p(tri[2]));
                    (b - a).cross(c - a).y <= 0.0
                })
                .count();
            assert_eq!(
                down,
                0,
                "{} of {} paving triangles in a {} face DOWN - nothing but ambient will ever light them",
                down,
                index.len() / 3,
                if city { "city" } else { "village" },
            );
        }
    }
}
