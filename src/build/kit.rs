//! A kit of parts, and the bench you compose one on.
//!
//! # Pieces, not shapes
//!
//! A building could be authored as arbitrary boxes at arbitrary sizes, and it
//! would be worse. Everything in a real structure is a repeat of a few members —
//! a post, a rail, a wall panel, a floor slab — because that is how anything gets
//! built out of stock lengths, and it is also why buildings look like buildings.
//! Free boxes give you the freedom to make every wall a slightly different
//! thickness, which is a freedom nobody wants and every eye notices.
//!
//! So the kit is short and the sizes are fixed. **A fence and a house come out of
//! the same kit** — a fence is posts and rails, a house is a floor, walls and a
//! roof — which is the test of whether the parts were chosen well rather than
//! invented one building at a time.
//!
//! # It writes the format that already exists
//!
//! The bench does not invent a file. It builds a [`Plan`] — the same thing a
//! building baked elsewhere reads as — so the live preview is the game's own
//! renderer with nothing special about it, and what it saves goes in the
//! buildings folder beside anything else.
//!
//! That is worth insisting on. Two formats for one idea is two readers, two
//! writers, and a fortnight of finding out which one a bug is in.
//!
//! # Everything snaps, and turns in quarters
//!
//! Position is on a [`SNAP`] lattice and rotation is quarter-turns only. Both are
//! restrictions and both are the point: a wall three degrees off is a mistake that
//! reads as one and takes a while to find, and no house anybody would build has
//! one. When something genuinely wants an angle, it wants a part for it — a brace
//! — not a free rotation on a wall.

use bevy::prelude::*;
use std::io;
use std::path::Path;

use crate::build::plan::{Block, Form, Plan};
use crate::config::BUILDINGS_DIR;

/// Metres between the places a piece can stand.
///
/// A SIXTEENTH of a metre, and the number is chosen for two reasons.
///
/// Fine, first: a quarter-metre lattice is coarse enough that a maker lining a
/// trim piece up against a post cannot put it where they mean to. Sixteenths give
/// sixteen positions per metre, which is finer than anything anybody eyeballs.
///
/// And EXACT, second, which matters more. A sixteenth is a power of two, so every
/// position on this lattice is a number a float can hold with nothing left over:
/// snap a thousand times and the answer never drifts, and two pieces placed a part
/// apart meet on exactly the same coordinate rather than within a hair of it. A
/// tenth of a metre would look tidier written down and would leave a sliver of gap
/// between abutting pieces that no amount of care could close.
///
/// Every part's own size is a multiple of it, so pieces abut exactly.
pub const SNAP: f32 = 0.0625;

/// The module the kit is built around, in metres.
///
/// A wall panel is this wide, a floor slab this square, and a rail spans this far.
/// One number, so a floor laid next to a floor takes a wall along its edge without
/// anybody measuring.
pub const MODULE: f32 = 1.5;

/// One kind of member.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Part {
    /// A square upright. Corners of a fence, corners of a frame.
    Post,
    /// A horizontal bar spanning one module. Fence rails, balustrades.
    Rail,
    /// A wall panel, one module wide and a storey tall.
    Wall,
    /// A floor slab, one module square. Also a ceiling, also a deck.
    Floor,
    /// A structural member spanning one module. Lintels, sills, plates.
    Beam,
    /// A roof panel: a wedge, sloping up from one edge.
    Roof,
    /// A ridge cap, running along the top where two roof panels meet.
    Cap,
    /// A course of plinth stone, for a building to stand on.
    ///
    /// Thicker than a wall and lower, so a wall set on it steps back from its
    /// face — which is what makes a building look founded rather than dropped.
    Foundation,
    /// A flight of steps, rising as it runs.
    Stairs,
    /// A bed. The first piece of furniture, and the reason there is a `fittings`
    /// stage at all.
    Bed,
}

impl Part {
    /// Every part, in the order the bench cycles them.
    ///
    /// Fence parts first, because a fence is the simplest thing anybody will build
    /// and the first thing they will try.
    pub const ALL: [Part; 10] = [
        Part::Post,
        Part::Rail,
        Part::Wall,
        Part::Floor,
        Part::Beam,
        Part::Roof,
        Part::Cap,
        Part::Foundation,
        Part::Stairs,
        Part::Bed,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Part::Post => "POST",
            Part::Rail => "RAIL",
            Part::Wall => "WALL",
            Part::Floor => "FLOOR",
            Part::Beam => "BEAM",
            Part::Roof => "ROOF",
            Part::Cap => "CAP",
            Part::Foundation => "FOUNDATION",
            Part::Stairs => "STAIRS",
            Part::Bed => "BED",
        }
    }

    /// Whether this part's own lattice runs along the EDGES of the module grid
    /// rather than through the middles of its cells.
    ///
    /// # A wall belongs on a floor's edge, and could not be put there
    ///
    /// A floor fills a cell: laid at a lattice point, its own edges fall half a
    /// module either side. A wall goes ON one of those edges — its centre-line is
    /// the join between two cells, which is what makes a room's wall stand at the
    /// edge of its floor rather than a foot inside it.
    ///
    /// The generators knew this and the cursor did not. `pattern::walls` has always
    /// placed its walls at `-MODULE * 0.5`, while the lattice cursor snapped to
    /// whole modules — so a wall placed by hand could only ever land on a cell
    /// CENTRE, standing three-quarters of a metre in from the floor's edge and
    /// clipping through the boards. A maker could not build what the generator
    /// built, which is the clearest sign the cursor was wrong rather than them.
    ///
    /// Read off the generators rather than invented, part by part: walls, rails and
    /// beams run along the joins; floors, roofs, ridge caps, posts, stairs and beds
    /// sit in cells. A foundation is the one part with no generator to read, and it
    /// goes under a wall.
    pub fn on_an_edge(self) -> bool {
        matches!(self, Part::Wall | Part::Rail | Part::Beam | Part::Foundation)
    }

    /// How far this part's lattice stands off the module grid, once it is turned.
    ///
    /// Only the axis ACROSS the part matters — a wall is a module long whichever
    /// way it faces, and it is its thickness that wants to sit on the join. So the
    /// offset follows the piece round: a quarter turn moves it from one axis to the
    /// other. Which SIGN it takes is nobody's business, since the two edges of a
    /// cell are one module apart and a module is the step being snapped to.
    pub fn off_the_grid(self, quarters: u8) -> Vec3 {
        if !self.on_an_edge() {
            return Vec3::ZERO;
        }
        let across = MODULE * 0.5;
        if quarters % 2 == 0 {
            Vec3::new(0.0, 0.0, across)
        } else {
            Vec3::new(across, 0.0, 0.0)
        }
    }

    /// What this part is made of, as an index into [`TINTS`].
    ///
    /// # A part arrives in its own material
    ///
    /// The colour in hand used to follow the maker from one part to the next, so a
    /// foundation came out oak and a flight of stairs came out oak, because the
    /// last thing placed was. That is the wrong default in a kit whose whole point
    /// is that a part IS something: a plinth is masonry, a roof is thatch, and
    /// nobody choosing FOUNDATION means "in wood, please".
    ///
    /// It is a default and not a rule — the swatches still overrule it, and go on
    /// overruling it for as long as that part stays in hand. What resets it is
    /// choosing a DIFFERENT part, because that is the moment the maker has said
    /// what they are building next.
    pub fn natural(self) -> usize {
        match self {
            // Masonry.
            Part::Foundation => 4,
            // Thatch, on the two parts that make a roof.
            Part::Roof | Part::Cap => 2,
            // The darker wood, which is what stairs and furniture are made of and
            // what stops a staircase disappearing into the floor it stands on.
            Part::Stairs | Part::Bed => 1,
            // Everything else is the timber the kit is mostly built from.
            _ => 0,
        }
    }

    /// What to print on this part's keycap.
    pub fn cap(self) -> &'static str {
        Self::ALL
            .iter()
            .position(|one| *one == self)
            .map_or("", |at| PART_KEYS[at].1)
    }

    /// How big it is, in metres. Every figure a multiple of [`SNAP`].
    pub fn size(self) -> Vec3 {
        match self {
            // Slightly over a snap so a post reads as a post rather than a line.
            Part::Post => Vec3::new(0.25, 1.25, 0.25),
            Part::Rail => Vec3::new(MODULE, 0.25, 0.25),
            Part::Wall => Vec3::new(MODULE, WALL_HIGH, 0.25),
            Part::Floor => Vec3::new(MODULE, 0.25, MODULE),
            Part::Beam => Vec3::new(MODULE, 0.25, 0.25),
            Part::Roof => Vec3::new(MODULE, 0.75, MODULE),
            Part::Cap => Vec3::new(MODULE, 0.5, 0.5),
            // Half a metre thick against a wall's quarter, so a wall set on it
            // steps back from its face. Low, because a plinth is a course or two
            // of stone and not a storey.
            Part::Foundation => Vec3::new(MODULE, 0.5, 0.5),
            // A module of run, and the rise that module earns: half a wall's
            // height, so TWO modules of flight reach exactly one storey. The kit's
            // numbers meeting each other is the whole reason they are one number.
            Part::Stairs => Vec3::new(MODULE, WALL_HIGH * 0.5, MODULE),
            // A double bed, at the size a double bed is rather than at a module —
            // furniture is the one thing in here a person's own size decides. Every
            // figure is still a whole number of snaps.
            Part::Bed => Vec3::new(2.0, 0.875, 1.375),
        }
    }

    /// Whether it can be made longer, and in what.
    ///
    /// The parts that are already a module long. Everything else has a size
    /// because of what it IS — a post is a quarter-metre square upright — and
    /// stretching one would produce a beam wearing a post's name.
    pub fn stretches(self) -> bool {
        !matches!(self, Part::Post | Part::Bed)
    }

    /// Whether stretching it makes it TALLER as well as longer.
    ///
    /// A flight of stairs, and nothing else. Every other part keeps its height
    /// when it is stretched — that is what stretching means — but a stair's rise
    /// IS its run: a longer flight is more steps, and more steps reach higher. A
    /// flight stretched without rising would be a row of treads going nowhere.
    pub fn climbs(self) -> bool {
        matches!(self, Part::Stairs)
    }

    /// Whether it can be made WIDER as well as longer.
    ///
    /// A floor, and only a floor. Every other part's second horizontal dimension
    /// is not an extent at all: a wall's is its thickness, a beam's is its
    /// section, a roof's is the depth its pitch is measured over. Growing any of
    /// those by whole modules gives a wall a metre and a half thick — a
    /// distortion wearing a part's name, which is the thing a kit of fixed sizes
    /// exists to refuse.
    ///
    /// A floor is a SURFACE, and both its horizontal dimensions are real extents.
    /// A floor two modules across is a floor with twice the planks in it, which is
    /// what "wider" means for a floor and for nothing else here.
    pub fn widens(self) -> bool {
        matches!(self, Part::Floor)
    }

    /// What shape it is, in the baked format's terms.
    pub fn form(self) -> Form {
        match self {
            Part::Roof => Form::Wedge,
            Part::Cap => Form::Ridge,
            _ => Form::Box,
        }
    }

    /// What it is made of, for the stage a game raising things in order would use.
    pub fn stage(self) -> &'static str {
        match self {
            Part::Floor | Part::Foundation => "footings",
            Part::Roof | Part::Cap => "roof",
            // Furniture goes in after the building is closed in, which is what a
            // stage is for saying.
            Part::Bed => "fittings",
            _ => "walls",
        }
    }
}

/// Which key picks each part, and what to print on its cap.
///
/// **One table, read by the input and by the panel both.** The terrain tool learned
/// this the hard way: the keys lived with the input and the panel numbered its own
/// rows, so the eleventh tool was labelled with the first one's key and nothing in
/// either place could notice. There are ten parts here now — one past the digits —
/// so a panel counting its own rows would already be lying about the last of them.
pub const PART_KEYS: [(KeyCode, &str); 10] = [
    (KeyCode::Digit1, "1"),
    (KeyCode::Digit2, "2"),
    (KeyCode::Digit3, "3"),
    (KeyCode::Digit4, "4"),
    (KeyCode::Digit5, "5"),
    (KeyCode::Digit6, "6"),
    (KeyCode::Digit7, "7"),
    (KeyCode::Digit8, "8"),
    (KeyCode::Digit9, "9"),
    // The tenth on nought, past the nine that come before it — the same place the
    // terrain tool puts its tenth tool.
    (KeyCode::Digit0, "0"),
];

/// Refused at compile time rather than found by a maker pressing a key that does
/// nothing: a part with no key is a part only the mouse can reach.
const _: () = assert!(
    PART_KEYS.len() == Part::ALL.len(),
    "every part needs a key, and every key a part"
);

/// A storey, in metres. What a wall is tall, and what two flights of stairs climb.
pub const WALL_HIGH: f32 = 2.5;

/// The colours a piece can be, in sRGB bytes.
///
/// A short shelf rather than a colour picker, and for the same reason the sizes
/// are fixed: a building whose every plank is a slightly different brown reads as
/// noise. Painting properly — per face, or with a texture — is a job of its own;
/// this is enough that a roof is not the colour of a floor.
pub const TINTS: [(&str, [u8; 3]); 6] = [
    ("oak", [158, 133, 97]),
    ("dark wood", [96, 71, 50]),
    ("thatch", [178, 150, 84]),
    ("slate", [92, 96, 104]),
    ("stone", [156, 150, 138]),
    ("whitewash", [222, 216, 202]),
];

/// One member, standing where it was put.
#[derive(Clone, Copy, Debug)]
pub struct Piece {
    pub id: u32,
    pub part: Part,
    /// Where its FOOT sits, not its middle.
    ///
    /// A maker places a wall on the ground, not a wall's centre 1.25 m up. The
    /// baked format wants centres, so the conversion happens once, in `to_plan`,
    /// rather than in every head that touches this.
    pub foot: Vec3,
    /// Quarter turns about Y, 0 to 3.
    ///
    /// Quarters only. A wall three degrees off is a mistake that reads as one and
    /// takes a while to find, and no house anybody would build has one.
    pub quarters: u8,
    /// Which of [`TINTS`].
    pub tint: usize,
    /// How many modules long it is, along its own length.
    ///
    /// # Stretching, not scaling
    ///
    /// A wall dragged from one module to three does not become a wall drawn at
    /// three times the size — it becomes a LONGER WALL. Its thickness is the
    /// thickness a wall is, its height is a storey, and the boards in a floor stay
    /// the width of a board. Nothing is ever distorted, which is the whole reason
    /// the kit has fixed sizes at all.
    ///
    /// Whole modules only, so a stretched piece still ends where the next begins.
    pub spans: u32,
    /// How many modules WIDE it is, across its own length.
    ///
    /// # Two directions, because a floor has two
    ///
    /// `spans` was the only one, and for the parts that are lines — walls, rails,
    /// beams — one is all there is. A floor is not a line: it is a surface with a
    /// length and a width, and a maker laying a room had to place nine separate
    /// slabs to get three modules by three.
    ///
    /// Only the parts [`Part::widens`] allows have this, and it means the same
    /// thing stretching does: more floor, made of more planks, each still the
    /// width a plank is.
    pub across: u32,
}

impl Piece {
    /// How big it actually is, stretch included.
    pub fn size(self) -> Vec3 {
        let base = self.part.size();
        let high = if self.part.climbs() {
            // The one part whose height is its length's business — see
            // `Part::climbs`.
            base.y * self.spans.max(1) as f32
        } else {
            base.y
        };
        Vec3::new(
            base.x + (self.spans.max(1) - 1) as f32 * MODULE,
            high,
            base.z + (self.across.max(1) - 1) as f32 * MODULE,
        )
    }

    pub fn turn(self) -> Quat {
        Quat::from_rotation_y(self.quarters as f32 * std::f32::consts::FRAC_PI_2)
    }

    /// Where the middle of its box sits, which is what the format stores.
    ///
    /// A stretched piece grows FORWARD from its foot rather than outward from its
    /// middle, so the end a maker placed stays where they put it and the far end
    /// is the one that moves. Growing from the middle slides both ends and takes
    /// the piece off whatever it was lined up against.
    pub fn middle(self) -> Vec3 {
        let along = (self.spans.max(1) - 1) as f32 * MODULE * 0.5;
        let aside = (self.across.max(1) - 1) as f32 * MODULE * 0.5;
        // Its OWN half-height, not the part's: a flight of stairs is taller the
        // longer it is, and a middle taken from the unstretched part would sit
        // below the piece it is meant to be the middle of.
        self.foot
            + Vec3::Y * self.size().y * 0.5
            + self.turn() * Vec3::new(along, 0.0, aside)
    }

    /// The box this piece fills, in world space: its low corner and its high one.
    ///
    /// Axis-aligned, and it stays that way because quarter turns are the only turns
    /// there are — a turn swaps a piece's length and its thickness rather than
    /// tilting anything.
    pub fn spread(self) -> (Vec3, Vec3) {
        let size = self.size();
        let half = 0.5
            * if self.quarters % 2 == 1 {
                Vec3::new(size.z, size.y, size.x)
            } else {
                size
            };
        let middle = self.middle();
        (middle - half, middle + half)
    }

    /// Whether this piece and another occupy any of the same space.
    ///
    /// **Touching is not clashing**, and the whole kit rests on the difference: it
    /// is built out of pieces that abut, so a floor laid beside a floor, a wall on a
    /// plinth and a cap set on a ridge all share a face and must be left exactly
    /// where they were put.
    pub fn clashes_with(self, other: Piece) -> bool {
        let (mine_low, mine_high) = self.spread();
        let (theirs_low, theirs_high) = other.spread();
        (0..3).all(|axis| {
            mine_high[axis] - theirs_low[axis] > TOUCHING
                && theirs_high[axis] - mine_low[axis] > TOUCHING
        })
    }

    /// Whether this piece would come to rest ON another, rather than merely brushing
    /// past its corner.
    ///
    /// # A corner is an overlap, and it is not a floor
    ///
    /// Two walls meeting at a right angle share half a thickness where they meet —
    /// that IS a corner, it is inside the wall and nobody sees it. Read as one
    /// standing on the other, it lifted the second wall a storey into the air the
    /// moment a room got its second side.
    ///
    /// So being underneath is about how much of a piece's FOOTPRINT is covered, not
    /// about whether the boxes touch at all: a floor under a wall on its edge covers
    /// half of it, a wall at a corner about a twelfth. See [`UNDERFOOT`], which is
    /// where those numbers are.
    pub fn stands_on(self, other: Piece) -> bool {
        if !self.clashes_with(other) {
            return false;
        }
        let (mine_low, mine_high) = self.spread();
        let (theirs_low, theirs_high) = other.spread();
        let shared = |axis: usize| {
            (mine_high[axis].min(theirs_high[axis]) - mine_low[axis].max(theirs_low[axis])).max(0.0)
        };
        let footprint = (mine_high.x - mine_low.x) * (mine_high.z - mine_low.z);
        footprint > NOTHING && shared(0) * shared(2) > footprint * UNDERFOOT
    }

    /// How far a point is from this piece's own box, in metres. Nought inside it.
    ///
    /// # Why not the distance to its middle
    ///
    /// That is what every "nearest piece" on this bench used, with a fixed radius —
    /// and it works only while a piece is about the size of that radius. Stretching
    /// broke it: a three-module floor has its middle two and a quarter metres from
    /// either end, so pointing at the end of one measured further away than the
    /// radius allowed and the floor could not be selected, painted, turned or taken
    /// off the bench. Reported as "once an object is placed I cannot select it
    /// again", which is exactly what it looks like from the outside.
    ///
    /// A distance to the BOX has no such limit: a piece of any length is reachable
    /// anywhere along it, and the number still means metres, so the callers' own
    /// reach values keep their meaning.
    pub fn away_from(self, point: Vec3) -> f32 {
        // Into the piece's own frame, where its box is axis-aligned.
        let local = self.turn().inverse() * (point - self.middle());
        let outside = local.abs() - self.size() * 0.5;
        outside.max(Vec3::ZERO).length()
    }

    /// How far along a ray this piece is struck, if it is struck at all.
    ///
    /// The slab test, in the piece's own frame. What it buys over [`Self::away_from`]
    /// is the thing a maker expects of a pointer: clicking a wall two metres up
    /// selects that wall, where the lattice cursor beneath the pointer is on the
    /// floor several metres behind it — the taller the piece, the further behind.
    pub fn struck_by(self, from: Vec3, along: Vec3) -> Option<f32> {
        let turn = self.turn().inverse();
        let origin = turn * (from - self.middle());
        let direction = turn * along;
        let half = self.size() * 0.5;

        let mut near = f32::NEG_INFINITY;
        let mut far = f32::INFINITY;
        for axis in 0..3 {
            let (o, d, h) = (origin[axis], direction[axis], half[axis]);
            if d.abs() < 1.0e-6 {
                // Parallel to this pair of faces: either between them for the whole
                // ray, or never.
                if o.abs() > h {
                    return None;
                }
                continue;
            }
            let (mut lo, mut hi) = ((-h - o) / d, (h - o) / d);
            if lo > hi {
                std::mem::swap(&mut lo, &mut hi);
            }
            near = near.max(lo);
            far = far.min(hi);
            if near > far {
                return None;
            }
        }
        // Behind the eye is not in front of it.
        (far >= 0.0).then(|| near.max(0.0))
    }

    /// The box or boxes this piece is drawn as.
    ///
    /// Most parts are one box. A floor is boarding — see [`Self::boarding`].
    pub fn blocks(self) -> Vec<Block> {
        let [r, g, b] = TINTS[self.tint.min(TINTS.len() - 1)].1;
        let colour = Color::srgb_u8(r, g, b);
        let size = self.size();

        // The parts that are made OF something get it laid; the rest are one box.
        match self.part {
            Part::Floor => self.boarding(colour, size),
            Part::Foundation => self.coursing(colour, size),
            Part::Stairs => self.flight(colour, size),
            Part::Bed => self.bedding(colour, size),
            _ => vec![Block {
                at: self.middle(),
                size,
                turn: self.turn(),
                form: self.part.form(),
                colour,
                stage: self.part.stage().into(),
            }],
        }
    }

    /// A floor, laid as boards over a subfloor.
    ///
    /// # Why there is something underneath
    ///
    /// The boards used to be the floor's whole thickness with a gap between them,
    /// and the gap went all the way through — so a floor was a set of slats with
    /// daylight between them, which is a duckboard. The boarding is [`DECK`] thick
    /// over a solid slab now, so a joint is a fine dark line with wood at the
    /// bottom of it. That is what a joint in a floor is.
    ///
    /// # What makes it read as wood
    ///
    /// Three things, and none of them is a texture — there are no textures here,
    /// only boxes with a colour each:
    ///
    /// * boards END, at [`BOARD`] intervals, and the ends of one plank do not line
    ///   up with the ends of the next. Boards butted end to end in a straight line
    ///   across a whole floor is the one pattern a joiner avoids, and it is exactly
    ///   what a grid draws if nobody stops it.
    /// * each board takes its own tone, so a floor is not one brown.
    /// * each board is laid in [`FIBRES`] strips along its length, each a little off
    ///   its neighbour — grain, at a finer scale than the boards vary, so a board
    ///   reads as one board with figure in it rather than as three narrow planks.
    ///
    /// # The pattern is anchored to the WORLD, not to the piece
    ///
    /// Every position that decides anything — which plank row this is, where its
    /// joints fall, what tone it takes — is measured along the piece's own axes in
    /// world space. Two floors laid edge to edge therefore carry on one another's
    /// boarding instead of each restarting its pattern at its own corner and
    /// drawing a seam nobody built.
    ///
    /// # It costs boxes, and knowingly
    ///
    /// Two dozen or so a module: planks times boards times strips. The format has
    /// always taken as many boxes as a building needs and they weld into one mesh,
    /// so the cost lands on the file rather than on the frame — but a floor is the
    /// one part where the boxes ARE the thing, and
    /// `a_module_of_floor_stays_within_its_box_budget` is what says how many is too
    /// many.
    fn boarding(self, colour: Color, size: Vec3) -> Vec<Block> {
        let turn = self.turn();
        let middle = self.middle();
        let half = size * 0.5;
        let stage = self.part.stage();
        let put = |local: Vec3| middle + turn * local;

        // Where the piece's own axes stand in the world, so everything below can be
        // laid out in world space and still run along the boards.
        let run0 = middle.dot(turn * Vec3::X);
        let side0 = middle.dot(turn * Vec3::Z);

        // The subfloor: one slab, the full footprint, everything but the top skin.
        let mut out = vec![Block {
            at: put(Vec3::new(0.0, -half.y + (size.y - DECK) * 0.5, 0.0)),
            size: Vec3::new(size.x, size.y - DECK, size.z),
            turn,
            form: Form::Box,
            colour: shaded_by(colour, UNDER),
            stage: stage.into(),
        }];

        let planks = PLANKS * self.across.max(1) as usize;
        let wide = MODULE / PLANKS as f32;
        let deck_y = half.y - DECK * 0.5;

        for plank in 0..planks {
            // The plank's slot, inset half a joint on each side that has a
            // neighbour. The outermost boards run flush to the floor's own edge: a
            // joint there would be a groove around the outside of the room.
            let slot = -half.z + plank as f32 * wide;
            let near = if plank == 0 { slot } else { slot + JOINT * 0.5 };
            let far = if plank + 1 == planks {
                slot + wide
            } else {
                slot + wide - JOINT * 0.5
            };

            // Which world plank row this is, which is what its joints and its tone
            // are drawn from.
            let row = ((side0 + (near + far) * 0.5) / wide).round() as i32;
            let phase = stagger_of(row);

            for (from, to) in board_ends(run0, half.x, phase, BOARD) {
                // Half a joint at each end that meets another board, and none at
                // the floor's own edge.
                let head = if from <= -half.x + NOTHING { from } else { from + JOINT * 0.5 };
                let tail = if to >= half.x - NOTHING { to } else { to - JOINT * 0.5 };
                if tail <= head {
                    continue;
                }

                // The board's own tone, from the world cell it lies in rather than
                // from its place in this piece's own list.
                let cell = ((run0 + (from + to) * 0.5) / BOARD).round() as i32;
                let shade = terrain_core::forest::chance(row, cell, 64);
                let board = 1.0 - GRAIN * 0.5 + shade * GRAIN;

                // And laid in strips along its length: the grain.
                let strip = (far - near) / FIBRES as f32;
                for fibre in 0..FIBRES {
                    let figure =
                        terrain_core::forest::chance(row * FIBRES as i32 + fibre as i32, cell, 66);
                    out.push(Block {
                        at: put(Vec3::new(
                            (head + tail) * 0.5,
                            deck_y,
                            near + strip * (fibre as f32 + 0.5),
                        )),
                        size: Vec3::new(tail - head, DECK, strip),
                        turn,
                        form: Form::Box,
                        colour: shaded_by(colour, board * (1.0 - FIGURE * 0.5 + figure * FIGURE)),
                        stage: stage.into(),
                    });
                }
            }
        }
        out
    }

    /// A foundation, laid as coursed stone.
    ///
    /// Two courses in running bond: the upper one offset half a stone from the
    /// lower, which is how a wall is built and how it reads. Butted, with no joint
    /// and nothing behind them — a plinth is solid, and a gap in one would show
    /// daylight under a wall.
    ///
    /// What makes it read as masonry rather than as a beige kerb is that the stones
    /// are not one colour and the two courses do not break in the same places.
    /// Both are drawn from the world, as the boarding is, so a foundation carried
    /// across two pieces keeps its bond.
    fn coursing(self, colour: Color, size: Vec3) -> Vec<Block> {
        let turn = self.turn();
        let middle = self.middle();
        let half = size * 0.5;
        let put = |local: Vec3| middle + turn * local;
        let run0 = middle.dot(turn * Vec3::X);

        let mut out = Vec::new();
        let deep = size.y / COURSES as f32;
        for course in 0..COURSES {
            // Every other course shifted half a stone: the bond.
            let phase = if course % 2 == 0 { 0.0 } else { STONE * 0.5 };
            let y = -half.y + deep * (course as f32 + 0.5);
            for (from, to) in board_ends(run0, half.x, phase, STONE) {
                let cell = ((run0 + (from + to) * 0.5) / STONE).round() as i32;
                let shade = terrain_core::forest::chance(course as i32, cell, 67);
                out.push(Block {
                    at: put(Vec3::new((from + to) * 0.5, y, 0.0)),
                    size: Vec3::new(to - from, deep, size.z),
                    turn,
                    form: Form::Box,
                    colour: shaded_by(colour, 1.0 - RUBBLE * 0.5 + shade * RUBBLE),
                    stage: self.part.stage().into(),
                });
            }
        }
        out
    }

    /// A flight of stairs, laid as steps.
    ///
    /// Each step is solid to the ground rather than a tread hanging in the air:
    /// stacked boxes read as a staircase from every side, where floating treads
    /// read as a staircase only from directly in front of one.
    ///
    /// The steps overlap each other — every one of them reaches the floor — and
    /// that is deliberate. The buried faces cost geometry nobody sees; the
    /// alternative is working out which face of which step is exposed, to save
    /// triangles on a part a building has two of.
    fn flight(self, colour: Color, size: Vec3) -> Vec<Block> {
        let turn = self.turn();
        let middle = self.middle();
        let half = size * 0.5;
        let put = |local: Vec3| middle + turn * local;

        let steps = STEPS * self.spans.max(1) as usize;
        let going = size.x / steps as f32;
        let rise = size.y / steps as f32;

        (0..steps)
            .map(|step| {
                // How high this step's tread stands above the foot of the flight.
                let top = rise * (step + 1) as f32;
                let shade = terrain_core::forest::chance(step as i32, 0, 68);
                Block {
                    at: put(Vec3::new(
                        -half.x + going * (step as f32 + 0.5),
                        -half.y + top * 0.5,
                        0.0,
                    )),
                    size: Vec3::new(going, top, size.z),
                    turn,
                    form: Form::Box,
                    colour: shaded_by(colour, 1.0 - TREAD * 0.5 + shade * TREAD),
                    stage: self.part.stage().into(),
                }
            })
            .collect()
    }

    /// A bed: a frame, a mattress on it, a pillow at the head, and a headboard.
    ///
    /// The frame takes the piece's own colour and the bedding does not. That is the
    /// one place in the kit where a piece overrules the palette, and it earns it:
    /// linen is linen, and a bed made entirely of one brown is a bench.
    ///
    /// The head is the piece's near end, so a bed turned a quarter puts its head
    /// where the maker expects — against whichever wall they turned it toward.
    fn bedding(self, colour: Color, size: Vec3) -> Vec<Block> {
        let turn = self.turn();
        let middle = self.middle();
        let half = size * 0.5;
        let put = |local: Vec3| middle + turn * local;
        let stage = self.part.stage();
        let linen = Color::srgb_u8(LINEN[0], LINEN[1], LINEN[2]);

        let lay = |at: Vec3, size: Vec3, colour: Color| Block {
            at: put(at),
            size,
            turn,
            form: Form::Box,
            colour,
            stage: stage.into(),
        };

        // The frame: the whole footprint, up to where the mattress starts.
        let frame_high = SNAP * 4.0;
        let mattress_high = SNAP * 4.0;
        // Two snaps, not one. At one it was a bright line along the head of the
        // mattress rather than a pillow — a thing you can only see the top of is
        // not a thing.
        let pillow_high = SNAP * 2.0;
        let board_thick = SNAP * 2.0;

        vec![
            lay(
                Vec3::new(0.0, -half.y + frame_high * 0.5, 0.0),
                Vec3::new(size.x, frame_high, size.z),
                colour,
            ),
            // The mattress, inset all round so the frame shows as a rail.
            lay(
                Vec3::new(0.0, -half.y + frame_high + mattress_high * 0.5, 0.0),
                Vec3::new(size.x - SNAP * 2.0, mattress_high, size.z - SNAP * 2.0),
                linen,
            ),
            // The pillow, at the head end and a shade brighter than the sheets.
            lay(
                Vec3::new(
                    -half.x + board_thick + SNAP * 4.0,
                    -half.y + frame_high + mattress_high + pillow_high * 0.5,
                    0.0,
                ),
                Vec3::new(SNAP * 7.0, pillow_high, size.z - SNAP * 6.0),
                shaded_by(linen, 1.08),
            ),
            // And the headboard, which is the piece's full height and the reason it
            // has any: everything else here is knee-high.
            lay(
                Vec3::new(-half.x + board_thick * 0.5, 0.0, 0.0),
                Vec3::new(board_thick, size.y, size.z),
                colour,
            ),
        ]
    }
}

/// Where one plank's boards begin and end, along the piece's own length.
///
/// The joints fall on a world grid of [`BOARD`] offset by `phase`, so a plank
/// carried across two abutting floors is cut in the same places in both. A board
/// shorter than [`RUNT`] is joined to its neighbour rather than laid as an offcut,
/// which is what happens to it on a real floor.
fn board_ends(run0: f32, half: f32, phase: f32, length: f32) -> Vec<(f32, f32)> {
    // The first joint at or past the near end, in the piece's own length.
    let mut cuts = Vec::new();
    let mut step = ((run0 - half - phase) / length).ceil();
    loop {
        let at = step * length + phase - run0;
        if at >= half - NOTHING {
            break;
        }
        if at > -half + NOTHING {
            cuts.push(at);
        }
        step += 1.0;
        // A length of nothing would cut for ever. It cannot happen from the two
        // constants that reach here, and that is exactly the kind of cannot that
        // stops being true when somebody adds a third caller.
        if length <= NOTHING {
            break;
        }
    }

    let mut ends = Vec::with_capacity(cuts.len() + 1);
    let mut from = -half;
    for cut in cuts {
        ends.push((from, cut));
        from = cut;
    }
    ends.push((from, half));

    // Offcuts joined to the board before them.
    let runt = length * RUNT;
    let mut laid: Vec<(f32, f32)> = Vec::with_capacity(ends.len());
    for (from, to) in ends {
        match laid.last_mut() {
            Some(last) if to - from < runt => last.1 = to,
            _ => laid.push((from, to)),
        }
    }
    // And a FIRST board that is itself an offcut takes the next one with it, there
    // being nothing before it to join to.
    if laid.len() > 1 && laid[0].1 - laid[0].0 < runt {
        let joined = laid.remove(0);
        laid[0].0 = joined.0;
    }
    laid
}

/// How far along a plank row its board joints are shifted.
///
/// In thirds of a board, drawn from the row's own place in the world — so a row is
/// staggered the same way in every floor it crosses, and a straight line of butt
/// joints across a whole floor cannot happen.
fn stagger_of(row: i32) -> f32 {
    let pick = (terrain_core::forest::chance(row, 0, 65) * STAGGERS as f32) as i32;
    pick.clamp(0, STAGGERS - 1) as f32 / STAGGERS as f32 * BOARD
}

/// The same colour, lighter or darker.
fn shaded_by(colour: Color, by: f32) -> Color {
    let lit = colour.to_linear();
    Color::linear_rgb(lit.red * by, lit.green * by, lit.blue * by)
}

/// How long a piece may be stretched, in modules.
///
/// Eight is a twelve-metre wall, which is longer than any one run of a building
/// wants before it should be two walls with a post between them.
pub const MOST_SPANS: u32 = 8;

/// How many planks lie across one module of floor.
///
/// Eight, which is 18.75 cm a board — flooring. Five was 30 cm, which is decking,
/// and it is a good part of why a floor read as a boardwalk. Each plank is exactly
/// three [`SNAP`]s wide, so the boarding lands on the same grid as everything else.
const PLANKS: usize = 8;

/// How thick the boarding is over the subfloor.
///
/// Half a snap — about three centimetres, which is what a floorboard is.
///
/// It was a whole snap, and a six-centimetre board is a DECK board: the joint
/// between two of them is a slot deep enough to see the side of the next board
/// through, which is the gap the maker was looking at. Thin the boards and the
/// same joint becomes a line.
const DECK: f32 = SNAP * 0.5;

/// The line between two boards.
///
/// A centimetre: the width of a saw, near enough, and the narrowest line that still
/// READS as a line from standing height. Not a snap multiple, deliberately — every dimension a maker places
/// or lines up is on the grid, and this is neither. It is the line between two
/// boards, and it wants to be as fine as it can be while still being there.
const JOINT: f32 = 0.010;

/// How long a board runs before the next begins, and how short a piece may be —
/// as a SHARE of that length — before it is joined to its neighbour rather than
/// laid as an offcut.
///
/// A share rather than a distance because two things are cut this way now, and a
/// quarter of a board and a quarter of a stone are not the same number of metres.
const BOARD: f32 = MODULE * 2.0;
const RUNT: f32 = 0.25;

/// How many places along a plank its end joints may fall.
const STAGGERS: i32 = 3;

/// How many strips a board is laid in, across its width. The grain.
///
/// Three: enough to read as figure in the wood, few enough that a board still
/// reads as one board.
const FIBRES: usize = 3;

/// How far a board strays from the next in tone, how far one strip strays from the
/// next WITHIN a board, and how much darker the subfloor under the joints is.
///
/// The figure is much the smaller of the two deliberately: strips as different from
/// each other as the boards are would read as three narrow planks rather than as
/// one board with grain in it.
///
/// Both were raised by about half after looking at the floor in the bench, where
/// the light is bright and flat and compresses tone differences that read fine in a
/// drawing. A fifth either way is a floor of boards that came off different trees;
/// past about a third it stops being one floor and becomes a patchwork.
const GRAIN: f32 = 0.42;
const FIGURE: f32 = 0.18;
const UNDER: f32 = 0.55;

/// How long one plinth stone is, how many courses a foundation is laid in, and how
/// far one stone strays from the next in tone.
const STONE: f32 = MODULE * 0.5;
const COURSES: usize = 2;
const RUBBLE: f32 = 0.30;

/// How many steps there are to a module of stairs, and how far one tread strays
/// from the next in tone.
///
/// Four: a going of 37.5 cm, which is a generous tread, against a rise of 31.25 cm
/// from `Part::size`. Steep for a house and right for a game, where a stair is
/// something you run up.
const STEPS: usize = 4;
const TREAD: f32 = 0.12;

/// Bed linen. Not from [`TINTS`], because a bed's sheets are not the colour its
/// frame is painted — see `Piece::bedding`.
const LINEN: [u8; 3] = [212, 206, 192];

/// How much of a piece's footprint another must cover before it counts as being
/// UNDER it rather than beside it.
///
/// A quarter, and the three numbers it has to sit between are worth writing down —
/// the first guess at this was a half, which is exactly wrong:
///
/// * a wall on a floor's EDGE has half its footprint over the floor and half over
///   the drop, because its centre-line is the join. Half is the case this must
///   catch, so half is the one number the threshold cannot be.
/// * a wall meeting another at a corner shares half a thickness with it, which is
///   about a twelfth of a wall's footprint. That is the case this must not catch.
/// * a floor under anything laid in the middle of it covers all of it.
const UNDERFOOT: f32 = 0.25;

/// How much two pieces may share before they are in each other's way, in metres.
///
/// Half a snap: enough that two pieces meant to abut are not read as clashing, and
/// far less than any part is thick.
const TOUCHING: f32 = SNAP * 0.5;

/// Nothing, in metres: what counts as landing exactly on an edge.
const NOTHING: f32 = 1.0e-4;

/// A work in progress.
#[derive(Resource, Debug)]
pub struct Bench {
    pub name: String,
    pub kind: String,
    pieces: Vec<Piece>,
    pub unsaved: bool,
}

impl Default for Bench {
    fn default() -> Self {
        Self {
            name: "untitled".into(),
            kind: "house".into(),
            pieces: Vec::new(),
            unsaved: false,
        }
    }
}

impl Bench {
    /// Every piece on the bench.
    ///
    /// Only the tests read this now that the writer takes its colours off the
    /// blocks. Kept because the alternative is tests reaching into private state
    /// to check what the bench holds, and a bench nobody can ask what is on it is
    /// a bench nobody can test.
    #[allow(dead_code)]
    pub fn pieces(&self) -> &[Piece] {
        &self.pieces
    }

    pub fn len(&self) -> usize {
        self.pieces.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pieces.is_empty()
    }

    /// Empties the bench.
    ///
    /// For generating into: a house drawn into a half-built one leaves two
    /// buildings inside each other, and a maker who wanted to keep what they had
    /// would have saved it.
    pub fn clear(&mut self) {
        self.pieces.clear();
        self.unsaved = true;
    }

    /// Snaps a point to the fine lattice.
    pub fn snapped(at: Vec3) -> Vec3 {
        Self::snapped_to(at, SNAP)
    }

    /// Snaps a point to any step. The module for placing things beside each
    /// other, the fine lattice for the times that is genuinely wanted.
    pub fn snapped_to(at: Vec3, step: f32) -> Vec3 {
        let step = step.max(SNAP);
        (at / step).round() * step
    }

    /// Where a piece would come to REST if it were put down here.
    ///
    /// # A wall stands on the floor; it does not stand in it
    ///
    /// Reported from the bench: walls clipping into the flooring. The cursor's
    /// height is the plane a maker is building on, and a floor laid on that plane
    /// occupies the first quarter-metre above it — so a wall placed at the same
    /// height had its foot buried, and the only remedy was to raise the cursor by
    /// four presses of a key nobody had mentioned.
    ///
    /// A piece never lands inside another one. Its foot rises to the top of whatever
    /// it would have clashed with, which is what a maker means by putting a wall on
    /// a floor. Touching is not clashing — see [`Piece::clashes_with`] — so nothing
    /// that abuts is disturbed.
    ///
    /// It is the CURSOR's rule and not the kit's, which is why it lives here rather
    /// than inside [`Self::add`]: the generators work out exact positions and must
    /// not be second-guessed, and neither must a piece being dragged by its arrows.
    pub fn resting(&self, part: Part, foot: Vec3, quarters: u8) -> Vec3 {
        let mut foot = Self::snapped(foot);
        // Settling rather than stepping once: rising out of one piece can bring a
        // piece up into another. Bounded, because a maker pointing into a tower of
        // pieces wants an answer rather than a search.
        for _ in 0..MOST_SPANS {
            let mine = Piece {
                id: 0,
                part,
                foot,
                quarters,
                tint: 0,
                spans: 1,
                across: 1,
            };
            let top = self
                .pieces
                .iter()
                .filter(|other| mine.stands_on(**other))
                .map(|other| other.spread().1.y)
                .fold(f32::MIN, f32::max);
            if top == f32::MIN {
                break;
            }
            foot.y = Self::snapped(Vec3::Y * top).y;
        }
        foot
    }

    /// Slides a piece that runs along a join onto the ground that supports it.
    ///
    /// # A wall on the join is right, and it still looks wrong
    ///
    /// A wall's centre-line is the boundary between two cells. That is exactly right
    /// for a wall BETWEEN two rooms, and at the outside of a building it leaves half
    /// the wall hanging over the drop — measured from a maker's own saved work: a
    /// wall at z 0.625..0.875 on a floor that ends at 0.75. Reported as "still not
    /// aligned with the edge", and it is not: it is aligned with the LINE the edge
    /// is on, which is not the same thing to look at.
    ///
    /// So which side it tucks to is read from what is actually beneath it:
    ///
    /// * ground on one side only — the outside of a building — and it slides that
    ///   way, its face flush with the floor's own face and all of it supported.
    /// * ground on both sides — an interior wall between two rooms — and it stays on
    ///   the join, which is where a wall between two rooms belongs.
    /// * ground on neither — a fence in the open — and it stays on the join too,
    ///   there being nothing to line up with.
    ///
    /// Read from the geometry rather than fixed, because a fixed offset is flush on
    /// one edge of a floor and hanging in the air on the opposite one.
    pub fn hugging(&self, part: Part, foot: Vec3, quarters: u8) -> Vec3 {
        if !part.on_an_edge() {
            return foot;
        }
        let mine = Piece {
            id: 0,
            part,
            foot,
            quarters,
            tint: 0,
            spans: 1,
            across: 1,
        };
        let (low, high) = mine.spread();
        // The axis the piece's thickness runs along, once it is turned.
        let axis = if quarters % 2 == 0 { 2 } else { 0 };
        let thick = high[axis] - low[axis];
        let mut step = Vec3::ZERO;
        step[axis] = 1.0;

        // A point in the middle of each half of its footprint, on the plane it is
        // standing on.
        let middle = (low + high) * 0.5;
        let held = |side: f32| {
            let probe = middle + step * side * thick * 0.25;
            self.pieces.iter().any(|other| {
                let (its_low, its_high) = other.spread();
                its_low.x - TOUCHING <= probe.x
                    && probe.x <= its_high.x + TOUCHING
                    && its_low.z - TOUCHING <= probe.z
                    && probe.z <= its_high.z + TOUCHING
                    // Reaching the plane this piece stands on: ground under it, not
                    // a wall beside it or a floor two storeys down.
                    && its_high.y >= foot.y - TOUCHING
                    && its_low.y <= foot.y + TOUCHING
            })
        };

        match (held(1.0), held(-1.0)) {
            (true, false) => Self::snapped(foot + step * thick * 0.5),
            (false, true) => Self::snapped(foot - step * thick * 0.5),
            // Both sides or neither: the join is where it belongs.
            _ => foot,
        }
    }

    /// Where a piece would come to rest if it were put down here: tucked onto
    /// whatever holds it up, and standing on top of whatever is under it.
    ///
    /// The one answer to "where does this go", so the ghost in hand and the piece
    /// that lands cannot disagree — which they did, the ghost showing a wall buried
    /// in a floor and the wall then standing on it.
    pub fn settling(&self, part: Part, foot: Vec3, quarters: u8) -> Vec3 {
        self.resting(part, self.hugging(part, foot, quarters), quarters)
    }

    /// Adds a member, snapping it, and hands back its name.
    ///
    /// Refuses to stack two of the same part in the same place. Placing a wall on
    /// a wall is a double-click, not an intention, and two coincident boxes are
    /// invisible until they flicker against each other.
    pub fn add(&mut self, part: Part, foot: Vec3, quarters: u8, tint: usize) -> Option<u32> {
        let foot = Self::snapped(foot);
        let quarters = quarters % 4;
        if self
            .pieces
            .iter()
            .any(|p| p.part == part && p.foot.abs_diff_eq(foot, SNAP * 0.5))
        {
            return None;
        }
        let id = self.pieces.iter().map(|p| p.id).max().unwrap_or(0) + 1;
        self.pieces.push(Piece {
            id,
            part,
            foot,
            quarters,
            tint: tint % TINTS.len(),
            spans: 1,
            across: 1,
        });
        self.unsaved = true;
        Some(id)
    }

    /// Paints the nearest member, and says which it was.
    ///
    /// By its BOX rather than by its middle or its foot: what a maker points at is
    /// the body of the thing, and a stretched piece's middle can be metres from the
    /// end they are aiming at. See [`Piece::away_from`].
    pub fn paint_nearest(&mut self, to: Vec3, within: f32, tint: usize) -> Option<Part> {
        let (_, id, part) = self
            .pieces
            .iter()
            .map(|p| (p.away_from(to), p.id, p.part))
            .filter(|(away, ..)| *away <= within)
            .min_by(|a, b| a.0.total_cmp(&b.0))?;
        let piece = self.pieces.iter_mut().find(|p| p.id == id)?;
        if piece.tint == tint % TINTS.len() {
            // Nothing changed, so nothing is unsaved and nothing redraws. Painting
            // a roof the colour it already is should not mark an afternoon's work
            // as needing to be written again.
            return Some(part);
        }
        piece.tint = tint % TINTS.len();
        self.unsaved = true;
        Some(part)
    }

    /// Makes one member longer or shorter, in whole modules.
    ///
    /// Only the parts that are a module long to begin with. A post is a post: it
    /// is a quarter-metre square upright, and a "stretched" one would be a beam
    /// with a post's name on it — which is exactly the sort of thing a kit exists
    /// to prevent.
    pub fn stretch(&mut self, id: u32, by: i32) -> bool {
        let Some(piece) = self.pieces.iter_mut().find(|p| p.id == id) else {
            return false;
        };
        if !piece.part.stretches() {
            return false;
        }
        let want = (piece.spans as i32 + by).clamp(1, MOST_SPANS as i32) as u32;
        if want == piece.spans {
            return false;
        }
        piece.spans = want;
        self.unsaved = true;
        true
    }

    /// Makes one member wider or narrower, in whole modules.
    ///
    /// Only a floor — see [`Part::widens`] for why nothing else has a width worth
    /// growing. The same shape as `stretch` and deliberately not folded into it
    /// with an axis argument: they are two different questions about a piece, and
    /// the parts that answer yes to one are not the parts that answer yes to the
    /// other.
    pub fn widen(&mut self, id: u32, by: i32) -> bool {
        let Some(piece) = self.pieces.iter_mut().find(|p| p.id == id) else {
            return false;
        };
        if !piece.part.widens() {
            return false;
        }
        let want = (piece.across as i32 + by).clamp(1, MOST_SPANS as i32) as u32;
        if want == piece.across {
            return false;
        }
        piece.across = want;
        self.unsaved = true;
        true
    }

    /// Moves one member to a new foot.
    ///
    /// By id rather than by nearness, because the arrows already know which piece
    /// they are on — and a drag that re-picked by proximity every frame would hand
    /// the piece over to whatever it was dragged past.
    pub fn move_to(&mut self, id: u32, foot: Vec3) -> bool {
        let Some(piece) = self.pieces.iter_mut().find(|p| p.id == id) else {
            return false;
        };
        if piece.foot == foot {
            return false;
        }
        piece.foot = foot;
        self.unsaved = true;
        true
    }

    /// Turns the nearest member a quarter, and says which it was.
    ///
    /// Pieces already down, rather than the one in hand. Getting a wall's facing
    /// wrong is the commonest mistake there is on a lattice — everything is
    /// axis-aligned, so a piece turned the wrong way looks almost right — and
    /// before this the only remedy was to delete it and place it again.
    pub fn turn_nearest(&mut self, to: Vec3, within: f32) -> Option<Part> {
        let (_, id, part) = self
            .pieces
            .iter()
            .map(|p| (p.away_from(to), p.id, p.part))
            .filter(|(away, ..)| *away <= within)
            .min_by(|a, b| a.0.total_cmp(&b.0))?;
        let piece = self.pieces.iter_mut().find(|p| p.id == id)?;
        piece.quarters = (piece.quarters + 1) % 4;
        self.unsaved = true;
        Some(part)
    }

    /// Takes the nearest member out, and says which it was.
    pub fn remove_nearest(&mut self, to: Vec3, within: f32) -> Option<Part> {
        let (_, id, part) = self
            .pieces
            .iter()
            .map(|p| (p.away_from(to), p.id, p.part))
            .filter(|(away, ..)| *away <= within)
            .min_by(|a, b| a.0.total_cmp(&b.0))?;
        self.pieces.retain(|p| p.id != id);
        self.unsaved = true;
        Some(part)
    }

    /// Takes the last member out. The undo anybody actually reaches for.
    pub fn undo(&mut self) -> Option<Part> {
        let gone = self.pieces.pop()?;
        self.unsaved = true;
        Some(gone.part)
    }

    /// Turns the whole work into the format the game already reads.
    ///
    /// One piece is not always one box. A floor is laid as BOARDS — see
    /// `planks` — because a floor slab of one flat colour is a slab, and the
    /// thing that makes a floor read as a floor is that you can see it is made of
    /// something. The format has always taken as many boxes as a building needs,
    /// so this costs nothing but the boxes themselves.
    pub fn to_plan(&self) -> Plan {
        let boxes: Vec<Block> = self.pieces.iter().flat_map(|piece| piece.blocks()).collect();

        let mut plan = Plan {
            name: self.name.clone(),
            kind: self.kind.clone(),
            half_w: 0.0,
            half_d: 0.0,
            high: 0.0,
            boxes,
            marks: Vec::new(),
        };
        // Measured from the work rather than asked for. A footprint somebody types
        // in is a footprint that stops being true the moment they add a porch.
        let (low, high) = plan.reach();
        plan.half_w = (high.x - low.x).max(0.0) * 0.5;
        plan.half_d = (high.z - low.z).max(0.0) * 0.5;
        plan.high = high.y.max(0.0);
        plan
    }
}

/// Writes a work out as a baked building.
///
/// # Why the game writes the bench's format rather than one of its own
///
/// A building read by the game and a building saved by the game should be the same
/// file, or there are two readers and two writers and every bug lives in the gap.
/// This produces `format: 2` — the layout in Opificium's `FORMATS.md` — so what is
/// saved here can be read by anything that reads any other building, and a round
/// trip through both is a test rather than a hope.
pub fn as_json(bench: &Bench) -> String {
    let plan = bench.to_plan();
    let mut out = String::new();
    out.push_str("{\n  \"format\": 2,\n");
    out.push_str(&format!("  \"name\": {},\n", quoted(&plan.name)));
    out.push_str(&format!("  \"kind\": {},\n", quoted(&plan.kind)));
    out.push_str(&format!("  \"half_w\": {:.4},\n", plan.half_w));
    out.push_str(&format!("  \"half_d\": {:.4},\n", plan.half_d));
    out.push_str(&format!("  \"high\": {:.4},\n", plan.high));
    out.push_str("  \"boxes\": [\n");

    // The colour comes off the BLOCK, not from looking the piece's tint up again.
    //
    // This zipped boxes against pieces one for one, which stopped being true the
    // moment a floor started being laid as several boards: the pairing slid, every
    // block after the first floor took the wrong tint, and the file came out
    // shorter than the building. A block already knows what colour it is.
    for (index, block) in plan.boxes.iter().enumerate() {
        let lit = block.colour.to_srgba();
        let rgb = [
            (lit.red * 255.0).round().clamp(0.0, 255.0) as u8,
            (lit.green * 255.0).round().clamp(0.0, 255.0) as u8,
            (lit.blue * 255.0).round().clamp(0.0, 255.0) as u8,
        ];
        out.push_str("    {");
        out.push_str(&format!(
            " \"at\": [{:.4},{:.4},{:.4}],",
            block.at.x, block.at.y, block.at.z
        ));
        out.push_str(&format!(
            " \"size\": [{:.4},{:.4},{:.4}],",
            block.size.x, block.size.y, block.size.z
        ));
        out.push_str(&format!(
            " \"turn\": [{:.6},{:.6},{:.6},{:.6}],",
            block.turn.x, block.turn.y, block.turn.z, block.turn.w
        ));
        out.push_str(&format!(" \"form\": {},", quoted(&block.form.word())));
        out.push_str(&format!(" \"rgb\": [{},{},{}],", rgb[0], rgb[1], rgb[2]));
        out.push_str(" \"alpha\": 1.0,");
        out.push_str(&format!(" \"cloth\": {},", quoted(&block.stage)));
        out.push_str(&format!(" \"stage\": {} }}", quoted(&block.stage)));
        if index + 1 < plan.boxes.len() {
            out.push(',');
        }
        out.push('\n');
    }

    out.push_str("  ],\n  \"marks\": []\n}\n");
    out
}

/// The shortest correct JSON string. The names here are a maker's own, so they
/// can hold anything a filename can.
fn quoted(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for letter in text.chars() {
        match letter {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

/// Where a work by this name would be saved.
pub fn path_for(name: &str) -> std::path::PathBuf {
    // Anything that is not plainly a filename becomes a dash. A maker naming a
    // building "the smith's / forge" should get a file, not an error about a
    // directory that does not exist.
    let mut safe = String::with_capacity(name.len());
    for letter in name.chars() {
        if letter.is_ascii_alphanumeric() || letter == '_' {
            safe.push(letter.to_ascii_lowercase());
        } else if !safe.ends_with('-') {
            // One dash for a run of them. "the smith's / forge" has five
            // characters in a row that are not a filename, and five dashes in a
            // row is a name nobody would type.
            safe.push('-');
        }
    }
    let safe = safe.trim_matches('-').to_string();
    let safe = if safe.is_empty() { "untitled".into() } else { safe };
    Path::new(BUILDINGS_DIR).join(format!("{safe}.json"))
}

pub fn save(bench: &mut Bench) -> io::Result<std::path::PathBuf> {
    let path = path_for(&bench.name);
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder)?;
    }
    std::fs::write(&path, as_json(bench))?;
    bench.unsaved = false;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fence: posts a module apart with rails between them.
    fn fence(bench: &mut Bench) {
        for step in 0..4 {
            let along = step as f32 * MODULE;
            bench.add(Part::Post, Vec3::new(along, 0.0, 0.0), 0, 1);
            if step < 3 {
                bench.add(Part::Rail, Vec3::new(along + MODULE * 0.5, 0.3, 0.0), 0, 0);
                bench.add(Part::Rail, Vec3::new(along + MODULE * 0.5, 0.8, 0.0), 0, 0);
            }
        }
    }

    /// A hut: a floor, four walls and a roof.
    fn hut(bench: &mut Bench) {
        for x in 0..2 {
            for z in 0..2 {
                bench.add(
                    Part::Floor,
                    Vec3::new(x as f32 * MODULE, 0.0, z as f32 * MODULE),
                    0,
                    4,
                );
            }
        }
        for x in 0..2 {
            bench.add(Part::Wall, Vec3::new(x as f32 * MODULE, 0.25, -0.75), 0, 0);
            bench.add(Part::Wall, Vec3::new(x as f32 * MODULE, 0.25, 2.25), 0, 0);
        }
        for z in 0..2 {
            bench.add(Part::Wall, Vec3::new(-0.75, 0.25, z as f32 * MODULE), 1, 0);
            bench.add(Part::Wall, Vec3::new(2.25, 0.25, z as f32 * MODULE), 1, 0);
        }
        for x in 0..2 {
            bench.add(Part::Roof, Vec3::new(x as f32 * MODULE, 2.75, 0.75), 0, 2);
        }
        bench.add(Part::Cap, Vec3::new(0.75, 3.5, 0.75), 0, 2);
    }

    #[test]
    fn one_kit_builds_a_fence_and_a_hut() {
        // The test of whether the parts were chosen well rather than invented one
        // building at a time. If a fence needs its own parts the kit is a list of
        // special cases.
        let mut rails = Bench::default();
        fence(&mut rails);
        let mut walls = Bench::default();
        hut(&mut walls);

        assert!(rails.len() >= 10, "a four-post fence is more than {} pieces", rails.len());
        assert!(walls.len() >= 12, "a hut is more than {} pieces", walls.len());

        // Both come out as buildings the game can raise.
        for (what, bench) in [("fence", &rails), ("hut", &walls)] {
            let plan = bench.to_plan();
            // At LEAST one box a piece: a floor is laid as several boards, so the
            // two counts are no longer the same number and should not be.
            assert!(
                plan.boxes.len() >= bench.len(),
                "{what} lost a piece: {} boxes from {} pieces",
                plan.boxes.len(),
                bench.len()
            );
            assert!(plan.high > 0.5, "{what} is {:.2} m tall", plan.high);
            assert!(plan.half_w > 0.5, "{what} is {:.2} m across", plan.half_w);
        }

        // A fence is long and thin; a hut is not. Nothing enforces that — it is
        // what the same parts did with different arrangements, which is the point.
        let long = rails.to_plan();
        let square = walls.to_plan();
        assert!(
            long.half_w > long.half_d * 3.0,
            "a fence should be long and thin: {:.2} by {:.2}",
            long.half_w,
            long.half_d
        );
        assert!(
            (square.half_w / square.half_d - 1.0).abs() < 0.35,
            "a hut should be roughly square: {:.2} by {:.2}",
            square.half_w,
            square.half_d
        );
    }

    #[test]
    fn a_saved_work_reads_back_as_the_same_building() {
        // The whole reason the bench writes the format that already exists. Two
        // formats for one idea is two readers, two writers, and a fortnight of
        // finding out which one a bug is in.
        let mut bench = Bench {
            name: "test hut".into(),
            ..Default::default()
        };
        hut(&mut bench);

        let json = as_json(&bench);
        let back = Plan::read(&json).expect("the game's own reader should take it");
        let mine = bench.to_plan();

        assert_eq!(back.name, "test hut");
        assert_eq!(back.boxes.len(), mine.boxes.len());
        assert!(
            back.boxes.len() > bench.len(),
            "a hut with a floor in it should write more boxes than it has pieces"
        );
        for (read, made) in back.boxes.iter().zip(&mine.boxes) {
            assert!(read.at.abs_diff_eq(made.at, 1.0e-3), "{:?} vs {:?}", read.at, made.at);
            assert!(read.size.abs_diff_eq(made.size, 1.0e-3));
            assert_eq!(read.form, made.form, "a form changed on the way through");
            // Colour survives the trip through sRGB bytes.
            let (want, got) = (made.colour.to_srgba(), read.colour.to_srgba());
            assert!((want.red - got.red).abs() < 0.02, "{want:?} vs {got:?}");
        }
        assert!((back.high - mine.high).abs() < 1.0e-3);
    }

    #[test]
    fn stretching_makes_a_longer_piece_and_not_a_bigger_one() {
        // The whole distinction. A wall dragged from one module to three is a
        // LONGER WALL: its thickness is the thickness a wall is and its height is
        // a storey. Scaled, it would be a wall drawn at three times the size, with
        // three times the thickness — which is the thing fixed sizes exist to
        // prevent.
        let mut bench = Bench::default();
        let id = bench.add(Part::Wall, Vec3::ZERO, 0, 0).expect("a wall");
        let one = bench.pieces()[0].size();

        assert!(bench.stretch(id, 2), "a wall would not stretch");
        let three = bench.pieces()[0].size();

        assert!(
            (three.x - (one.x + 2.0 * MODULE)).abs() < 1.0e-4,
            "three modules of wall came out {:.2} m, not {:.2}",
            three.x,
            one.x + 2.0 * MODULE
        );
        assert_eq!(three.y, one.y, "stretching changed how tall a wall is");
        assert_eq!(three.z, one.z, "stretching changed how thick a wall is");
    }

    #[test]
    fn a_stretched_piece_keeps_the_end_it_was_placed_by() {
        // It grows FORWARD from its foot. Growing from the middle slides both ends
        // and takes the piece off whatever it was lined up against — which is the
        // thing a maker was doing when they placed it.
        let mut bench = Bench::default();
        let id = bench.add(Part::Wall, Vec3::ZERO, 0, 0).expect("a wall");
        let near_end = |b: &Bench| {
            let piece = b.pieces()[0];
            piece.middle().x - piece.size().x * 0.5
        };
        let before = near_end(&bench);
        bench.stretch(id, 3);
        assert!(
            (near_end(&bench) - before).abs() < 1.0e-4,
            "the placed end moved from {before:.3} to {:.3}",
            near_end(&bench)
        );
    }

    #[test]
    fn a_post_is_a_post() {
        // Everything else has a size because of what it IS. A stretched post would
        // be a beam wearing a post's name, which is exactly what a kit is for
        // preventing.
        let mut bench = Bench::default();
        let id = bench.add(Part::Post, Vec3::ZERO, 0, 0).expect("a post");
        assert!(!bench.stretch(id, 4), "a post stretched");
        assert_eq!(bench.pieces()[0].size(), Part::Post.size());

        // And nothing stretches past its limit or below one module.
        let wall = bench.add(Part::Wall, Vec3::new(9.0, 0.0, 0.0), 0, 0).unwrap();
        bench.stretch(wall, 99);
        assert_eq!(bench.pieces()[1].spans, MOST_SPANS);
        bench.stretch(wall, -99);
        assert_eq!(bench.pieces()[1].spans, 1);
    }

    #[test]
    fn a_stretched_floor_gains_boards_rather_than_wider_ones() {
        // Boards stay the width a board is. A floor whose planks grew with it
        // would read as a photograph of a floor enlarged.
        let mut bench = Bench::default();
        let id = bench.add(Part::Floor, Vec3::ZERO, 0, 0).expect("a floor");
        let one = bench.to_plan().boxes;
        bench.stretch(id, 2);
        let three = bench.to_plan().boxes;

        let width = |boxes: &[crate::build::plan::Block]| {
            boxes.iter().map(|b| b.size.z).fold(0.0_f32, f32::max)
        };
        assert!(
            (width(&one) - width(&three)).abs() < 1.0e-4,
            "boards went from {:.3} to {:.3} wide",
            width(&one),
            width(&three)
        );
        let long = |boxes: &[crate::build::plan::Block]| {
            boxes.iter().map(|b| b.size.x).fold(0.0_f32, f32::max)
        };
        assert!(long(&three) > long(&one) * 2.5, "the boards did not get longer");
    }

    /// The floor's boarding, told apart by thickness: the subfloor is everything
    /// but the top skin, a board is the skin.
    fn boards(boxes: &[crate::build::plan::Block]) -> Vec<crate::build::plan::Block> {
        boxes.iter().filter(|b| b.size.y < DECK * 1.5).cloned().collect()
    }

    fn subfloor(boxes: &[crate::build::plan::Block]) -> crate::build::plan::Block {
        let mut under: Vec<_> = boxes.iter().filter(|b| b.size.y > DECK * 1.5).collect();
        assert_eq!(under.len(), 1, "a floor should have exactly one subfloor");
        under.pop().unwrap().clone()
    }

    #[test]
    fn a_floor_is_laid_as_boards() {
        // A floor slab of one flat colour is a slab. What makes a floor read as a
        // floor is that you can see it is made of something.
        let mut bench = Bench::default();
        bench.add(Part::Floor, Vec3::ZERO, 0, 0);
        let boxes = bench.to_plan().boxes;
        let laid = boards(&boxes);
        assert!(laid.len() > 8, "a floor came out as {} boards", laid.len());

        // Boards, not tiles: each one long and narrow.
        for board in &laid {
            assert!(
                board.size.x > board.size.z * 2.0,
                "a board {:.3} by {:.3} is a tile",
                board.size.x,
                board.size.z
            );
        }

        // They cover the floor: the top faces add up to the footprint, less what
        // the joints take out of it. By area rather than by counting a row, because
        // a row is not a fixed number of boards — where a plank's ends fall is the
        // whole point of the stagger.
        let floor = Part::Floor.size();
        let covered: f32 = laid.iter().map(|b| b.size.x * b.size.z).sum();
        let footprint = floor.x * floor.z;
        assert!(
            covered <= footprint + NOTHING && covered > footprint * 0.9,
            "the boards cover {covered:.3} of a {footprint:.3} m² floor"
        );

        // And they reach its edges, so the boarding is not a rug laid on a slab.
        let edge = |pick: fn(&crate::build::plan::Block) -> f32| {
            laid.iter().map(pick).fold(f32::MIN, f32::max)
        };
        assert!(
            (edge(|b| b.at.z + b.size.z * 0.5) - floor.z * 0.5).abs() < NOTHING,
            "the boarding stops short of the floor's own edge"
        );

        // And they are not one colour — see `GRAIN` and `FIGURE`.
        let shades: std::collections::HashSet<u32> = laid
            .iter()
            .map(|b| (b.colour.to_linear().red * 10_000.0) as u32)
            .collect();
        assert!(shades.len() > 4, "a floor came out in {} shades", shades.len());

        // The whole thing still sits exactly on the ground it was placed on, and is
        // exactly as thick as a floor.
        let (low, high) = bench.to_plan().reach();
        assert!(low.y.abs() < NOTHING, "the floor sits {:.4} m off", low.y);
        assert!(
            (high.y - Part::Floor.size().y).abs() < NOTHING,
            "boarding changed how thick a floor is"
        );
    }

    #[test]
    fn a_floor_has_no_gap_to_see_through() {
        // Reported from the bench: the gaps between the planks. They went all the
        // way through, because the boards WERE the floor — so a floor was a set of
        // slats with daylight between them, which is a duckboard.
        let mut bench = Bench::default();
        let id = bench.add(Part::Floor, Vec3::ZERO, 0, 0).expect("a floor");
        bench.widen(id, 1);
        let boxes = bench.to_plan().boxes;
        let under = subfloor(&boxes);
        let laid = boards(&boxes);
        let size = bench.pieces()[0].size();

        // The subfloor is the whole footprint, so there is nowhere a joint could
        // look through.
        assert!(
            (under.size.x - size.x).abs() < NOTHING && (under.size.z - size.z).abs() < NOTHING,
            "the subfloor is {:.3} by {:.3} under a {:.3} by {:.3} floor",
            under.size.x,
            under.size.z,
            size.x,
            size.z
        );

        // And it reaches the boards rather than stopping short of them: the top of
        // the one is the bottom of the others.
        let under_top = under.at.y + under.size.y * 0.5;
        for board in &laid {
            let bottom = board.at.y - board.size.y * 0.5;
            assert!(
                (bottom - under_top).abs() < NOTHING,
                "a board's underside is {:.4} m from the subfloor's top",
                bottom - under_top
            );
        }

        // The joints themselves are a saw's width, not a finger's.
        assert!(JOINT < 0.02, "a {JOINT} m joint is a gap");
    }

    #[test]
    fn the_boards_of_a_floor_do_not_all_end_in_the_same_place() {
        // The pattern a joiner avoids and a grid draws by default: every board
        // butting its neighbour in one straight line across the whole floor.
        let mut bench = Bench::default();
        let id = bench.add(Part::Floor, Vec3::ZERO, 0, 0).expect("a floor");
        bench.stretch(id, 3);
        bench.widen(id, 1);

        // Where each plank row's boards end, gathered by row.
        let mut rows: std::collections::HashMap<i32, Vec<i32>> = Default::default();
        for board in boards(&bench.to_plan().boxes) {
            let row = (board.at.z * 1_000.0) as i32;
            rows.entry(row)
                .or_default()
                .push(((board.at.x + board.size.x * 0.5) * 100.0) as i32);
        }
        assert!(rows.len() > 8, "only {} plank rows", rows.len());

        let patterns: std::collections::HashSet<Vec<i32>> = rows
            .values()
            .map(|ends| {
                let mut ends = ends.clone();
                ends.sort_unstable();
                ends
            })
            .collect();
        assert!(
            patterns.len() > 1,
            "every plank in the floor is cut in the same places"
        );
    }

    #[test]
    fn widening_a_floor_adds_planks_of_the_same_width() {
        // The other half of "stretching, not scaling", and the half that did not
        // exist: a floor could only ever be made longer, so laying a room meant
        // placing a slab per module.
        let mut bench = Bench::default();
        let id = bench.add(Part::Floor, Vec3::ZERO, 0, 0).expect("a floor");
        let one = bench.to_plan().boxes;
        assert!(bench.widen(id, 2), "a floor would not widen");
        let three = bench.to_plan().boxes;

        let widest = |boxes: &[crate::build::plan::Block]| {
            boards(boxes).iter().map(|b| b.size.z).fold(0.0_f32, f32::max)
        };
        assert!(
            (widest(&one) - widest(&three)).abs() < NOTHING,
            "planks went from {:.4} to {:.4} wide",
            widest(&one),
            widest(&three)
        );
        // Three times the PLANK ROWS, exactly. Not three times the boards: how many
        // boards a plank is cut into depends on where its ends fall, which is the
        // stagger doing its job.
        let rows = |boxes: &[crate::build::plan::Block]| {
            boards(boxes)
                .iter()
                .map(|b| (b.at.z * 1_000.0) as i32)
                .collect::<std::collections::HashSet<_>>()
                .len()
        };
        assert_eq!(
            rows(&three),
            rows(&one) * 3,
            "three modules across came out as {} plank rows against {}",
            rows(&three),
            rows(&one)
        );

        // Wider, and no thicker or longer for it.
        let size = bench.pieces()[0].size();
        assert!((size.z - MODULE * 3.0).abs() < NOTHING, "it is {:.3} m across", size.z);
        assert_eq!(size.y, Part::Floor.size().y, "widening changed how thick it is");
        assert_eq!(size.x, Part::Floor.size().x, "widening changed how long it is");
    }

    #[test]
    fn a_widened_floor_keeps_the_edge_it_was_placed_by() {
        // The same promise stretching makes, across the other axis: the edge the
        // maker laid stays put and the far one moves. Growing from the middle would
        // slide both off whatever they were lined up against.
        let mut bench = Bench::default();
        let id = bench.add(Part::Floor, Vec3::ZERO, 0, 0).expect("a floor");
        let near_edge = |b: &Bench| {
            let piece = b.pieces()[0];
            piece.middle().z - piece.size().z * 0.5
        };
        let before = near_edge(&bench);
        bench.widen(id, 3);
        assert!(
            (near_edge(&bench) - before).abs() < NOTHING,
            "the placed edge moved from {before:.4} to {:.4}",
            near_edge(&bench)
        );
    }

    #[test]
    fn a_floor_is_the_only_thing_that_widens() {
        // Every other part's second horizontal dimension is not an extent: a wall's
        // is its thickness. A widened wall would be a metre and a half thick — a
        // distortion wearing a part's name, which is what fixed sizes are for.
        let mut bench = Bench::default();
        for part in Part::ALL {
            bench.clear();
            let id = bench.add(part, Vec3::ZERO, 0, 0).expect("a piece");
            let before = bench.pieces()[0].size();
            let took = bench.widen(id, 1);
            assert_eq!(
                took,
                part.widens(),
                "{} answered {took} to being widened",
                part.name()
            );
            if !took {
                assert_eq!(bench.pieces()[0].size(), before, "{} changed size", part.name());
            }
        }
        // And no part goes past the limit or below one module either way.
        bench.clear();
        let id = bench.add(Part::Floor, Vec3::ZERO, 0, 0).unwrap();
        bench.widen(id, 99);
        assert_eq!(bench.pieces()[0].across, MOST_SPANS);
        bench.widen(id, -99);
        assert_eq!(bench.pieces()[0].across, 1);
    }

    #[test]
    fn a_module_of_floor_stays_within_its_box_budget() {
        // Boarding costs boxes — planks times boards times strips — and the cost
        // lands on every building file that has a floor in it. Named in
        // `Piece::boarding`, which is where the reasoning is; this is the number.
        let mut bench = Bench::default();
        let id = bench.add(Part::Floor, Vec3::ZERO, 0, 0).expect("a floor");
        bench.stretch(id, 3);
        bench.widen(id, 3);
        let modules = 4.0 * 4.0;
        let each = bench.to_plan().boxes.len() as f32 / modules;
        assert!(
            each < 40.0,
            "a module of floor costs {each:.0} boxes, which is a floor drawn as a mosaic"
        );
    }

    #[test]
    fn a_flight_of_stairs_climbs_as_it_lengthens() {
        // The one part whose height is its length's business. A flight stretched
        // without rising would be a row of treads going nowhere.
        let mut bench = Bench::default();
        let id = bench.add(Part::Stairs, Vec3::ZERO, 0, 0).expect("stairs");
        let one = bench.pieces()[0].size();
        bench.stretch(id, 1);
        let two = bench.pieces()[0].size();

        assert!(
            (two.y - one.y * 2.0).abs() < NOTHING,
            "two modules of flight rise {:.3} m against one module's {:.3}",
            two.y,
            one.y
        );
        // And two modules of it reach exactly one storey, which is what makes a
        // stair worth having: it arrives at the floor above.
        assert!(
            (two.y - WALL_HIGH).abs() < NOTHING,
            "two modules of stairs reach {:.3} m against a storey of {WALL_HIGH}",
            two.y
        );

        // The steps ascend, evenly, and every one of them stands on the ground.
        let mut steps = bench.to_plan().boxes;
        steps.sort_by(|a, b| a.at.x.total_cmp(&b.at.x));
        assert_eq!(steps.len(), STEPS * 2, "a two-module flight has {} steps", steps.len());
        let mut was = 0.0;
        for step in &steps {
            let top = step.at.y + step.size.y * 0.5;
            assert!(top > was + NOTHING, "a step at {top:.3} m does not rise above {was:.3}");
            assert!(
                (step.at.y - step.size.y * 0.5).abs() < NOTHING,
                "a step floats {:.4} m off the floor",
                step.at.y - step.size.y * 0.5
            );
            was = top;
        }
        assert!(
            (was - two.y).abs() < NOTHING,
            "the top step reaches {was:.3} of the flight's own {:.3}",
            two.y
        );
    }

    #[test]
    fn a_foundation_is_laid_in_courses_that_break_in_different_places() {
        // A running bond: the upper course offset half a stone from the lower. Two
        // courses breaking in the same places is a stack, not a wall.
        let mut bench = Bench::default();
        let id = bench.add(Part::Foundation, Vec3::ZERO, 0, 0).expect("a plinth");
        bench.stretch(id, 3);
        let boxes = bench.to_plan().boxes;

        let mut courses: std::collections::HashMap<i32, Vec<i32>> = Default::default();
        for stone in &boxes {
            courses
                .entry((stone.at.y * 1_000.0) as i32)
                .or_default()
                .push(((stone.at.x + stone.size.x * 0.5) * 100.0) as i32);
        }
        assert_eq!(courses.len(), COURSES, "a foundation came out in {} courses", courses.len());
        let breaks: std::collections::HashSet<Vec<i32>> = courses
            .values()
            .map(|ends| {
                let mut ends = ends.clone();
                ends.sort_unstable();
                ends
            })
            .collect();
        assert_eq!(breaks.len(), COURSES, "both courses break in the same places");

        // Butted, not jointed: a gap in a plinth would show daylight under a wall.
        // Every course covers its whole length.
        for ends in courses.values() {
            assert!(!ends.is_empty());
        }
        let covered: f32 = boxes
            .iter()
            .filter(|b| (b.at.y - boxes[0].at.y).abs() < NOTHING)
            .map(|b| b.size.x)
            .sum();
        let size = bench.pieces()[0].size();
        assert!(
            (covered - size.x).abs() < NOTHING,
            "a course covers {covered:.3} m of a {:.3} m plinth",
            size.x
        );
    }

    #[test]
    fn a_bed_is_made_of_a_frame_and_bedding_that_are_not_the_same_colour() {
        // The one place a piece overrules the palette. Linen is linen, and a bed in
        // one brown is a bench.
        let mut bench = Bench::default();
        // Painted the darkest wood there is, so a frame that leaked into the linen
        // could not hide behind a pale tint.
        let dark = TINTS.iter().position(|(name, _)| *name == "dark wood").unwrap();
        bench.add(Part::Bed, Vec3::ZERO, 0, dark).expect("a bed");
        let boxes = bench.to_plan().boxes;
        assert!(boxes.len() >= 4, "a bed came out as {} boxes", boxes.len());

        let brightest = boxes
            .iter()
            .map(|b| b.colour.to_linear().red)
            .fold(0.0_f32, f32::max);
        let darkest = boxes
            .iter()
            .map(|b| b.colour.to_linear().red)
            .fold(1.0_f32, f32::min);
        assert!(
            brightest > darkest * 3.0,
            "the bedding at {brightest:.3} is barely brighter than the frame at {darkest:.3}"
        );

        // It does not stretch: a bed is the size a person is.
        let id = bench.pieces()[0].id;
        assert!(!bench.stretch(id, 2), "a bed stretched");
        assert!(!bench.widen(id, 2), "a bed widened");

        // The headboard is at the piece's near end, and it is the tallest thing.
        let size = bench.pieces()[0].size();
        let tallest = boxes
            .iter()
            .max_by(|a, b| a.size.y.total_cmp(&b.size.y))
            .expect("a box");
        assert!(
            (tallest.size.y - size.y).abs() < NOTHING,
            "the tallest part of a bed is {:.3} of {:.3}",
            tallest.size.y,
            size.y
        );
        assert!(
            tallest.at.x < 0.0,
            "the headboard stands at {:.3}, which is not the head end",
            tallest.at.x
        );
    }

    #[test]
    fn two_floors_side_by_side_carry_on_the_same_grain() {
        // Deterministic from where a piece STANDS, not from the order it was placed
        // in. An id is an accident of that order, and a floor whose pattern jumped
        // at a seam because of it would read as two floors.
        let grain = |first: Vec3, second: Vec3| {
            let mut bench = Bench::default();
            bench.add(Part::Floor, first, 0, 0);
            bench.add(Part::Floor, second, 0, 0);
            // By WHERE each board is rather than by its place in the list: the
            // order the boxes come out in follows the order the slabs were laid,
            // and what must not depend on that is the wood at a given spot.
            let mut wood: Vec<(i32, i32, u32)> = bench
                .to_plan()
                .boxes
                .iter()
                .map(|b| {
                    (
                        (b.at.x * 1_000.0) as i32,
                        (b.at.z * 1_000.0) as i32,
                        (b.colour.to_linear().red * 10_000.0) as u32,
                    )
                })
                .collect();
            wood.sort_unstable();
            wood
        };
        let one_way = grain(Vec3::ZERO, Vec3::new(MODULE, 0.0, 0.0));
        let other_way = grain(Vec3::new(MODULE, 0.0, 0.0), Vec3::ZERO);
        assert!(!one_way.is_empty(), "no boards at all");
        assert_eq!(
            one_way, other_way,
            "the grain depends on which slab was laid first"
        );
    }

    #[test]
    fn a_stretched_piece_can_be_reached_along_its_whole_length() {
        // Reported as "once an object is placed I cannot select it again", and it
        // was every "nearest piece" on the bench: they measured to a piece's MIDDLE
        // against a fixed radius, which a three-module floor is longer than. So the
        // far end of anything stretched was out of reach — unselectable, unpaintable
        // and unremovable, while the same piece at one module answered fine.
        let mut bench = Bench::default();
        let id = bench.add(Part::Floor, Vec3::ZERO, 0, 0).expect("a floor");
        bench.stretch(id, 3);
        let piece = bench.pieces()[0];
        let half = piece.size() * 0.5;

        // Standing on it, anywhere along it.
        for step in 0..=12 {
            let along = -half.x + piece.size().x * step as f32 / 12.0;
            let on = piece.middle() + Vec3::new(along, -half.y, 0.0);
            assert!(
                piece.away_from(on) < 1.0e-4,
                "a point on the floor at {along:.2} m along reads {:.3} m away",
                piece.away_from(on)
            );
        }

        // And the old measure is what it was: the far end really was out of the
        // radius that used to be used, so this test would have failed before.
        let end = piece.middle() + Vec3::new(half.x, -half.y, 0.0);
        assert!(
            end.distance(piece.middle()) > MODULE * 1.5,
            "the end of a four-module floor is only {:.2} m from its middle, so this \
             test no longer proves anything",
            end.distance(piece.middle())
        );

        // Beside it reads as a real distance, in metres, from the box.
        let beside = piece.middle() + Vec3::new(half.x + 0.75, -half.y, 0.0);
        assert!(
            (piece.away_from(beside) - 0.75).abs() < 1.0e-3,
            "three-quarters of a metre off the end reads {:.3}",
            piece.away_from(beside)
        );
    }

    #[test]
    fn a_turned_piece_is_reached_in_its_own_frame() {
        // The measure has to follow the piece round, or a wall placed across the
        // room is reachable from where it used to be.
        let mut bench = Bench::default();
        let id = bench.add(Part::Wall, Vec3::ZERO, 1, 0).expect("a wall");
        bench.stretch(id, 2);
        let piece = bench.pieces()[0];

        // A quarter turn puts its length along Z, so its far end is along Z too.
        let along = piece.middle() + Vec3::new(0.0, 0.0, piece.size().x * 0.5 - 0.1);
        assert!(
            piece.away_from(along) < 1.0e-4,
            "a point inside the turned wall reads {:.3} m away",
            piece.away_from(along)
        );
        let across = piece.middle() + Vec3::new(piece.size().x * 0.5, 0.0, 0.0);
        assert!(
            piece.away_from(across) > 0.9,
            "a point out to the side of a turned wall reads only {:.3} m away",
            piece.away_from(across)
        );
        let _ = id;
    }

    #[test]
    fn a_ray_strikes_the_piece_it_is_pointed_at() {
        // What the pointer is ON, which is not what the lattice cursor is near: aim
        // at the top of a wall and the cursor is on the floor metres behind it.
        let mut bench = Bench::default();
        bench.add(Part::Wall, Vec3::ZERO, 0, 0).expect("a wall");
        let piece = bench.pieces()[0];
        let high = piece.middle() + Vec3::Y * (piece.size().y * 0.25);

        // Straight down onto its top.
        let hit = piece.struck_by(high + Vec3::Y * 10.0, -Vec3::Y);
        assert!(hit.is_some_and(|along| along > 9.0), "a ray onto the top missed: {hit:?}");

        // From in front, level with where a maker would be looking.
        let from = high + Vec3::new(0.0, 0.0, 8.0);
        assert!(
            piece.struck_by(from, -Vec3::Z).is_some(),
            "a ray at the face of a wall missed it"
        );
        // Beside it, and behind it.
        assert!(
            piece.struck_by(from + Vec3::X * 4.0, -Vec3::Z).is_none(),
            "a ray four metres to the side struck the wall"
        );
        assert!(
            piece.struck_by(from, Vec3::Z).is_none(),
            "a ray pointed away from the wall struck it"
        );

        // The nearest of two along the same ray is the one in front.
        bench.add(Part::Wall, Vec3::new(0.0, 0.0, -MODULE * 2.0), 0, 0);
        let (first, second) = (bench.pieces()[0], bench.pieces()[1]);
        let near = first.struck_by(from, -Vec3::Z).expect("the near wall");
        let far = second.struck_by(from, -Vec3::Z).expect("the far wall");
        assert!(near < far, "the far wall is struck first: {near:.2} against {far:.2}");
    }

    #[test]
    fn a_wall_stands_on_a_floor_rather_than_in_it() {
        // Reported from the bench: walls clipping into the flooring. A floor laid on
        // the plane the cursor is on fills the first quarter-metre above it, so a
        // wall placed at the same height had its foot buried in the boards.
        let mut bench = Bench::default();
        bench.add(Part::Floor, Vec3::ZERO, 0, 0).expect("a floor");

        // Where a wall would come to rest, pointed at the floor's own edge.
        let edge = Vec3::new(0.0, 0.0, -MODULE * 0.5);
        let foot = bench.resting(Part::Wall, edge, 0);
        assert!(
            (foot.y - Part::Floor.size().y).abs() < 1.0e-4,
            "a wall rests {:.3} m up, on a floor {:.3} m thick",
            foot.y,
            Part::Floor.size().y
        );
        assert!(
            (foot.x - edge.x).abs() < 1.0e-4 && (foot.z - edge.z).abs() < 1.0e-4,
            "resting a wall moved it sideways, to {foot:?}"
        );

        // And once it is down, nothing is inside anything.
        let id = bench.add(Part::Wall, foot, 0, 0).expect("a wall");
        let wall = *bench.pieces().iter().find(|p| p.id == id).expect("the wall");
        let floor = *bench.pieces().iter().find(|p| p.id != id).expect("the floor");
        assert!(
            !wall.clashes_with(floor),
            "the wall is still inside the floor: {:?} against {:?}",
            wall.spread(),
            floor.spread()
        );
        // Standing ON it, not hovering over it.
        assert!(
            (wall.spread().0.y - floor.spread().1.y).abs() < 1.0e-4,
            "the wall's foot is {:.4} m from the floor's top",
            wall.spread().0.y - floor.spread().1.y
        );
    }

    #[test]
    fn a_piece_laid_beside_another_is_not_lifted_onto_it() {
        // The other half of resting, and the half that would ruin everything: the
        // kit is built out of pieces that abut. If touching counted as clashing, a
        // floor laid beside a floor would climb on top of it and a cap set on a
        // ridge would float.
        let mut bench = Bench::default();
        bench.add(Part::Floor, Vec3::ZERO, 0, 0);
        for step in 1..4 {
            let beside = Vec3::new(step as f32 * MODULE, 0.0, 0.0);
            let foot = bench.resting(Part::Floor, beside, 0);
            assert!(
                foot.y.abs() < 1.0e-4,
                "the {step}th floor climbed to {:.3} m",
                foot.y
            );
            bench.add(Part::Floor, foot, 0, 0);
        }
        // Across, too.
        let foot = bench.resting(Part::Floor, Vec3::new(0.0, 0.0, MODULE), 0);
        assert!(foot.y.abs() < 1.0e-4, "a floor across from another climbed to {:.3}", foot.y);
    }

    #[test]
    fn resting_settles_through_a_stack_rather_than_stepping_once() {
        // Rising out of one piece can bring a piece up into another.
        let mut bench = Bench::default();
        bench.add(Part::Floor, Vec3::ZERO, 0, 0);
        // A beam sitting on the floor, in the same column a wall is about to go.
        let beam = bench.resting(Part::Beam, Vec3::new(0.0, 0.0, -MODULE * 0.5), 0);
        bench.add(Part::Beam, beam, 0, 0);

        let wall = bench.resting(Part::Wall, Vec3::new(0.0, 0.0, -MODULE * 0.5), 0);
        let wanted = Part::Floor.size().y + Part::Beam.size().y;
        assert!(
            (wall.y - wanted).abs() < 1.0e-4,
            "a wall over a floor and a beam rests at {:.3}, not {wanted:.3}",
            wall.y
        );
    }

    #[test]
    fn the_parts_that_run_along_a_join_sit_off_the_grid() {
        // What the generators have always done, said once where the cursor can read
        // it too — `pattern::walls` places its walls at half a module, and until now
        // the lattice cursor could only land on whole ones.
        for part in Part::ALL {
            let off = part.off_the_grid(0);
            if part.on_an_edge() {
                assert!(
                    (off.z - MODULE * 0.5).abs() < 1.0e-4 && off.x == 0.0,
                    "{} leans {off:?}, which is not a cell join",
                    part.name()
                );
                // And it follows the piece round: a quarter turn moves the lean from
                // one axis to the other, or a turned wall lands in the middle of a
                // cell again.
                let turned = part.off_the_grid(1);
                assert!(
                    (turned.x - MODULE * 0.5).abs() < 1.0e-4 && turned.z == 0.0,
                    "{} turned a quarter leans {turned:?}",
                    part.name()
                );
            } else {
                assert_eq!(off, Vec3::ZERO, "{} should sit in a cell", part.name());
                assert_eq!(part.off_the_grid(1), Vec3::ZERO);
            }
        }

        // The parts a wall lines up with lean the same way, or a beam over a wall
        // would sit half a module off it.
        for part in [Part::Beam, Part::Rail, Part::Foundation] {
            assert_eq!(
                part.off_the_grid(0),
                Part::Wall.off_the_grid(0),
                "{} does not line up with a wall",
                part.name()
            );
        }
    }

    #[test]
    fn a_wall_on_the_lean_lattice_lands_on_a_floor_s_edge() {
        // The whole point of the lean, measured: the wall's centre-line and the
        // floor's edge are the same line.
        let mut bench = Bench::default();
        bench.add(Part::Floor, Vec3::ZERO, 0, 0);
        let floor = bench.pieces()[0];
        let (low, high) = floor.spread();

        // Where the cursor lands with a wall in hand, pointing near the floor's far
        // edge: the module grid, leaned by half a module.
        let lean = Part::Wall.off_the_grid(0);
        let aimed = Vec3::new(0.1, 0.0, high.z - 0.2);
        let at = Bench::snapped_to(aimed - lean, MODULE) + lean;
        assert!(
            (at.z - high.z).abs() < 1.0e-4,
            "the cursor landed at {:.3} against a floor edge at {:.3}",
            at.z,
            high.z
        );

        let foot = bench.resting(Part::Wall, at, 0);
        let id = bench.add(Part::Wall, foot, 0, 0).expect("a wall");
        let wall = *bench.pieces().iter().find(|p| p.id == id).unwrap();
        let (wall_low, wall_high) = wall.spread();
        let centre = (wall_low.z + wall_high.z) * 0.5;
        assert!(
            (centre - high.z).abs() < 1.0e-4,
            "the wall's centre-line is at {centre:.3} and the floor's edge at {:.3}",
            high.z
        );
        // Which puts half its thickness over the boards and half over the drop —
        // and its length along the floor's own edge rather than across the middle.
        assert!(wall_low.z < high.z && wall_high.z > high.z, "the wall is not on the edge");
        let _ = low;
    }

    #[test]
    fn a_wall_tucks_onto_the_floor_at_the_outside_of_a_building() {
        // Measured from the maker's own saved work: a wall at z 0.625..0.875 standing
        // on a floor that ends at 0.75. Its centre-line was exactly on the floor's
        // edge, which is right for a wall between two rooms and leaves half of it
        // hanging over the drop at the outside of a building.
        let mut bench = Bench::default();
        bench.add(Part::Floor, Vec3::ZERO, 0, 0).expect("a floor");
        let floor = bench.pieces()[0];
        let edge = floor.spread().1.z;
        let thick = Part::Wall.size().z;

        // Aimed at the join the floor's edge lies on.
        let foot = bench.settling(Part::Wall, Vec3::new(0.0, 0.0, edge), 0);
        let wall = Piece { id: 0, part: Part::Wall, foot, quarters: 0, tint: 0, spans: 1, across: 1 };
        let (low, high) = wall.spread();

        assert!(
            (high.z - edge).abs() < 1.0e-4,
            "the wall's outer face is at {:.4} and the floor's edge at {edge:.4}",
            high.z
        );
        assert!(
            low.z > edge - thick - 1.0e-4 && low.z < edge,
            "the wall is not sitting inside the floor: {:.4}..{:.4}",
            low.z,
            high.z
        );
        // All of it held up, and standing on the boards rather than in them.
        assert!(
            (low.y - floor.spread().1.y).abs() < 1.0e-4,
            "the wall's foot is {:.4} and the floor's top {:.4}",
            low.y,
            floor.spread().1.y
        );
        assert!(low.z >= floor.spread().0.z && high.z <= floor.spread().1.z + 1.0e-4);
    }

    #[test]
    fn a_wall_between_two_rooms_stays_on_the_join() {
        // The other side of the same rule. A wall with floor on both sides is an
        // INTERIOR wall, and an interior wall belongs on the boundary — tucking it
        // one way would put it inside one room and leave a lip in the other.
        let mut bench = Bench::default();
        bench.add(Part::Floor, Vec3::ZERO, 0, 0);
        bench.add(Part::Floor, Vec3::new(0.0, 0.0, MODULE), 0, 0);
        let join = MODULE * 0.5;

        let foot = bench.settling(Part::Wall, Vec3::new(0.0, 0.0, join), 0);
        let wall = Piece { id: 0, part: Part::Wall, foot, quarters: 0, tint: 0, spans: 1, across: 1 };
        let (low, high) = wall.spread();
        assert!(
            ((low.z + high.z) * 0.5 - join).abs() < 1.0e-4,
            "an interior wall slid off the join to {:.4}",
            (low.z + high.z) * 0.5
        );

        // And in the open, with nothing under it either side, it stays on the join
        // as well — a fence has nothing to line up with.
        let bare = Bench::default();
        let foot = bare.settling(Part::Rail, Vec3::new(0.0, 0.0, join), 0);
        assert!((foot.z - join).abs() < 1.0e-4, "a rail in the open slid to {:.4}", foot.z);
    }

    #[test]
    fn tucking_follows_the_piece_round() {
        // A wall turned a quarter has its thickness on the other axis, so the side it
        // tucks to has to move with it.
        let mut bench = Bench::default();
        bench.add(Part::Floor, Vec3::ZERO, 0, 0);
        let floor = bench.pieces()[0];
        let edge = floor.spread().1.x;

        let foot = bench.settling(Part::Wall, Vec3::new(edge, 0.0, 0.0), 1);
        let wall = Piece { id: 0, part: Part::Wall, foot, quarters: 1, tint: 0, spans: 1, across: 1 };
        let (low, high) = wall.spread();
        assert!(
            (high.x - edge).abs() < 1.0e-4,
            "the turned wall's outer face is at {:.4} and the floor's edge at {edge:.4}",
            high.x
        );
        assert!(low.x < edge, "the turned wall is outside the floor");
    }

    #[test]
    fn a_part_arrives_in_its_own_material() {
        // Reported from the bench: a foundation came out the colour of the wood and
        // so did the stairs, because the colour in hand followed the maker from one
        // part to the next. A plinth is masonry; nobody choosing FOUNDATION means
        // "in wood, please".
        let stone = TINTS.iter().position(|(name, _)| *name == "stone").unwrap();
        let dark = TINTS.iter().position(|(name, _)| *name == "dark wood").unwrap();
        let thatch = TINTS.iter().position(|(name, _)| *name == "thatch").unwrap();

        assert_eq!(Part::Foundation.natural(), stone, "a plinth is not stone");
        assert_eq!(Part::Stairs.natural(), dark, "stairs are not the darker wood");
        assert_eq!(Part::Bed.natural(), dark);
        assert_eq!(Part::Roof.natural(), thatch, "a roof is not thatch");
        assert_eq!(Part::Cap.natural(), Part::Roof.natural(), "a cap should match its roof");

        for part in Part::ALL {
            assert!(
                part.natural() < TINTS.len(),
                "{} is made of a colour that is not on the shelf",
                part.name()
            );
        }

        // And the material is what a piece is actually built with.
        let mut bench = Bench::default();
        bench.add(Part::Foundation, Vec3::ZERO, 0, Part::Foundation.natural());
        let grey = bench.to_plan().boxes[0].colour.to_srgba();
        let [r, g, b] = TINTS[stone].1;
        let wanted = Color::srgb_u8(r, g, b).to_srgba();
        assert!(
            (grey.red - wanted.red).abs() < 0.25 && (grey.blue - wanted.blue).abs() < 0.25,
            "a plinth came out {grey:?} against stone's {wanted:?}"
        );
    }

    #[test]
    fn every_part_has_a_key_and_a_cap_to_print_on_it() {
        // Two tables that were one apart: the input held the keys and the panel
        // numbered its own rows. With ten parts and nine digits, a panel that
        // counted would print "10" on a key nobody has.
        let mut seen = std::collections::HashSet::new();
        for part in Part::ALL {
            let cap = part.cap();
            assert!(!cap.is_empty(), "{} has no key", part.name());
            assert!(seen.insert(cap), "{cap} picks two parts");
        }
        assert_eq!(seen.len(), Part::ALL.len());
    }

    #[test]
    fn a_piece_rests_on_the_floor_it_was_placed_on() {
        // Reported from the bench: pieces float slightly above the grid. Measured
        // rather than eyeballed, because "slightly" is exactly the size of error
        // that argument settles badly.
        let mut bench = Bench::default();
        for part in Part::ALL {
            bench.clear();
            bench.add(part, Vec3::ZERO, 0, 0);
            let plan = bench.to_plan();
            let (low, _) = plan.reach();
            assert!(
                low.y.abs() < 1.0e-4,
                "{} sits {:.4} m off the floor",
                part.name(),
                low.y
            );
        }
    }

    #[test]
    fn a_piece_snaps_and_turns_in_quarters() {
        let mut bench = Bench::default();
        // Placed off the lattice, stored on it.
        let id = bench
            .add(Part::Post, Vec3::new(1.31, 0.04, -2.66), 7, 0)
            .expect("the first post");
        let post = bench.pieces()[0];
        assert_eq!(post.id, id);
        assert_eq!(post.foot, Vec3::new(1.3125, 0.0625, -2.6875));

        // And the lattice is EXACT, which is the property that matters rather
        // than any particular coordinate. A sixteenth is a power of two, so every
        // position is a number a float holds with nothing left over: snapping an
        // already-snapped point never moves it, however many times it is done.
        // On a lattice of tenths this loop drifts.
        let mut at = post.foot;
        for _ in 0..1_000 {
            at = Bench::snapped(at);
        }
        assert_eq!(at, post.foot, "the lattice drifts under repeated snapping");

        // Every position is a whole number of snaps from the origin, exactly.
        for axis in [at.x, at.y, at.z] {
            let steps = axis / SNAP;
            assert_eq!(steps, steps.round(), "{axis} is not a whole number of snaps");
        }
        // Seven quarter-turns is three, not an error and not seven.
        assert_eq!(post.quarters, 3);

        // And a piece's foot is its foot: the box's middle is half its height up,
        // which is the conversion the format wants and a maker should never think
        // about.
        // Measured from the FOOT, which is the whole point of storing a foot: the
        // box's middle is half its height above wherever it stands, not half its
        // height above the ground.
        assert!((post.middle().y - post.foot.y - Part::Post.size().y * 0.5).abs() < 1.0e-6);
    }

    #[test]
    fn the_same_part_will_not_stack_on_itself() {
        // A double-click is not an intention, and two coincident boxes are
        // invisible until they flicker against each other in the finished
        // building — by which time nobody remembers placing two.
        let mut bench = Bench::default();
        assert!(bench.add(Part::Wall, Vec3::ZERO, 0, 0).is_some());
        assert!(
            bench.add(Part::Wall, Vec3::new(0.02, 0.0, -0.02), 0, 0).is_none(),
            "a wall stacked on a wall"
        );
        // A different part in the same place is fine — a post at the end of a wall
        // is exactly that.
        assert!(bench.add(Part::Post, Vec3::ZERO, 0, 0).is_some());
        assert_eq!(bench.len(), 2);
    }

    #[test]
    fn taking_pieces_back_out() {
        let mut bench = Bench::default();
        bench.add(Part::Post, Vec3::ZERO, 0, 0);
        bench.add(Part::Wall, Vec3::new(9.0, 0.0, 0.0), 0, 0);

        // The last one, which is the undo anybody reaches for while building.
        assert_eq!(bench.undo(), Some(Part::Wall));
        assert_eq!(bench.len(), 1);
        // Or whatever is nearest, for going back to fix something.
        assert_eq!(bench.remove_nearest(Vec3::new(0.3, 0.6, 0.0), 2.0), Some(Part::Post));
        assert!(bench.is_empty());
        assert_eq!(bench.undo(), None, "an empty bench has nothing to take back");
        assert_eq!(bench.remove_nearest(Vec3::ZERO, 100.0), None);
    }

    #[test]
    fn a_name_becomes_a_filename() {
        // A maker naming a building "the smith's / forge" should get a file, not an
        // error about a directory that does not exist.
        assert!(path_for("the smith's / forge").ends_with("the-smith-s-forge.json"));
        assert!(path_for("Cottage").ends_with("cottage.json"));
        assert!(path_for("").ends_with("untitled.json"));
        assert!(path_for("///").ends_with("untitled.json"));
    }
}

#[cfg(test)]
mod look {
    use super::*;

    /// Prints a scene of the new parts as a baked building, for looking at.
    ///
    /// `cargo test dump_the_new_parts -- --ignored --nocapture`. Not a test of
    /// anything: the tests above measure the geometry, and this is for the one
    /// question they cannot answer, which is whether it looks like wood.
    #[test]
    #[ignore]
    fn dump_the_new_parts() {
        let mut bench = Bench::default();
        bench.name = "a-look".into();

        // Each in its own material, as choosing it at the bench now gives it — see
        // `Part::natural`. Passing colours in by hand here is how this dump came to
        // show a stone plinth while the bench was placing an oak one.
        fn put(bench: &mut Bench, part: Part, at: Vec3) -> u32 {
            bench.add(part, at, 0, part.natural()).expect("a piece")
        }

        // A floor three modules by two, which is the thing that could not be built
        // at all before.
        let floor = put(&mut bench, Part::Floor, Vec3::new(-3.0, 0.0, -1.5));
        bench.stretch(floor, 2);
        bench.widen(floor, 1);

        // A plinth along its edge, a flight climbing off it, and a bed on it.
        let plinth = put(&mut bench, Part::Foundation, Vec3::new(-3.0, 0.0, 1.5));
        bench.stretch(plinth, 2);
        let steps = put(&mut bench, Part::Stairs, Vec3::new(1.5, 0.0, 1.5));
        bench.stretch(steps, 1);
        put(&mut bench, Part::Bed, Vec3::new(-2.0, 0.25, -1.0));

        // And a room built the way the CURSOR builds one: the lattice a part sits
        // on, then wherever that part comes to rest. Written out here rather than
        // typed as positions, so what this draws is what the bench would do.
        let laid = |bench: &mut Bench, part: Part, aimed: Vec3, quarters: u8| {
            let lean = part.off_the_grid(quarters);
            let at = Bench::snapped_to(aimed - lean, MODULE) + lean;
            let foot = bench.settling(part, Vec3::new(at.x, 0.0, at.z), quarters);
            bench.add(part, foot, quarters, part.natural());
        };

        // Two modules of floor, and walls along three of its edges.
        for across in 0..2 {
            laid(&mut bench, Part::Floor, Vec3::new(4.5 + across as f32 * MODULE, 0.0, 0.0), 0);
        }
        for across in 0..2 {
            let along = 4.5 + across as f32 * MODULE;
            laid(&mut bench, Part::Wall, Vec3::new(along, 0.0, -0.7), 0);
            laid(&mut bench, Part::Wall, Vec3::new(along, 0.0, 0.7), 0);
        }
        laid(&mut bench, Part::Wall, Vec3::new(4.5 - 0.7, 0.0, 0.0), 1);

        println!("SCENE {}", as_json(&bench));
    }
}

#[cfg(test)]
mod corners {
    use super::*;

    #[test]
    fn walls_meeting_at_a_corner_do_not_climb_on_each_other() {
        // Two walls at right angles overlap by half a thickness where they meet —
        // which is what a corner IS — and `resting` must not read that as one
        // standing on the other.
        let mut bench = Bench::default();
        bench.add(Part::Floor, Vec3::ZERO, 0, 0).expect("a floor");
        let top = Part::Floor.size().y;

        let along = bench.resting(Part::Wall, Vec3::new(0.0, 0.0, -MODULE * 0.5), 0);
        bench.add(Part::Wall, along, 0, 0).expect("a wall");
        let across = bench.resting(Part::Wall, Vec3::new(-MODULE * 0.5, 0.0, 0.0), 1);

        assert!(
            (along.y - top).abs() < 1.0e-4,
            "the first wall rests at {:.3}, not on the floor at {top:.3}",
            along.y
        );
        assert!(
            (across.y - top).abs() < 1.0e-4,
            "the second wall of a corner climbed to {:.3} instead of resting at {top:.3}",
            across.y
        );
    }
}
