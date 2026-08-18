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
}

impl Part {
    /// Every part, in the order the bench cycles them.
    ///
    /// Fence parts first, because a fence is the simplest thing anybody will build
    /// and the first thing they will try.
    pub const ALL: [Part; 7] = [
        Part::Post,
        Part::Rail,
        Part::Wall,
        Part::Floor,
        Part::Beam,
        Part::Roof,
        Part::Cap,
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
        }
    }

    /// How big it is, in metres. Every figure a multiple of [`SNAP`].
    pub fn size(self) -> Vec3 {
        match self {
            // Slightly over a snap so a post reads as a post rather than a line.
            Part::Post => Vec3::new(0.25, 1.25, 0.25),
            Part::Rail => Vec3::new(MODULE, 0.25, 0.25),
            Part::Wall => Vec3::new(MODULE, 2.5, 0.25),
            Part::Floor => Vec3::new(MODULE, 0.25, MODULE),
            Part::Beam => Vec3::new(MODULE, 0.25, 0.25),
            Part::Roof => Vec3::new(MODULE, 0.75, MODULE),
            Part::Cap => Vec3::new(MODULE, 0.5, 0.5),
        }
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
            Part::Floor => "footings",
            Part::Roof | Part::Cap => "roof",
            _ => "walls",
        }
    }
}

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
}

impl Piece {
    pub fn turn(self) -> Quat {
        Quat::from_rotation_y(self.quarters as f32 * std::f32::consts::FRAC_PI_2)
    }

    /// Where the middle of its box sits, which is what the format stores.
    pub fn middle(self) -> Vec3 {
        self.foot + Vec3::Y * self.part.size().y * 0.5
    }

    /// The box or boxes this piece is drawn as.
    ///
    /// Most parts are one box. A floor is boards.
    pub fn blocks(self) -> Vec<Block> {
        let [r, g, b] = TINTS[self.tint.min(TINTS.len() - 1)].1;
        let colour = Color::srgb_u8(r, g, b);
        let size = self.part.size();

        if self.part != Part::Floor {
            return vec![Block {
                at: self.middle(),
                size,
                turn: self.turn(),
                form: self.part.form(),
                colour,
                stage: self.part.stage().into(),
            }];
        }

        // A floor, laid as boards.
        //
        // Deterministic from where the piece STANDS rather than from its id, so
        // two floors laid side by side carry on the same grain — an id is an
        // accident of the order things were placed, and a floor whose pattern
        // jumped at a seam because of that would look like two floors.
        let wide = size.x / PLANKS as f32;
        let lay = self.foot.x.round() as i32;
        let across = self.foot.z.round() as i32;
        (0..PLANKS)
            .map(|board| {
                let along = board as f32 + 0.5;
                let shade = terrain_core::forest::chance(lay, across * PLANKS as i32 + board as i32, 64);
                Block {
                    at: self.middle()
                        + self.turn()
                            * Vec3::new(0.0, 0.0, -size.z * 0.5 + wide * along),
                    // A hair short across, so the joint between boards reads as a
                    // line rather than the boards fusing into a slab again.
                    size: Vec3::new(size.x, size.y, wide - JOINT),
                    turn: self.turn(),
                    form: Form::Box,
                    colour: shaded_by(colour, 1.0 - GRAIN * 0.5 + shade * GRAIN),
                    stage: self.part.stage().into(),
                }
            })
            .collect()
    }
}

/// The same colour, lighter or darker.
fn shaded_by(colour: Color, by: f32) -> Color {
    let lit = colour.to_linear();
    Color::linear_rgb(lit.red * by, lit.green * by, lit.blue * by)
}

/// How many boards a floor is laid in, the line between them, and how far one
/// board strays from the next.
///
/// Wood is not one colour, and a floor where it is reads as printed.
const PLANKS: usize = 5;
const JOINT: f32 = 0.02;
const GRAIN: f32 = 0.30;

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
        });
        self.unsaved = true;
        Some(id)
    }

    /// Paints the nearest member, and says which it was.
    ///
    /// By its MIDDLE rather than its foot, because that is where a maker is
    /// looking when they point at a piece: a wall's foot is on the floor and its
    /// body is the thing on screen.
    pub fn paint_nearest(&mut self, to: Vec3, within: f32, tint: usize) -> Option<Part> {
        let (_, id, part) = self
            .pieces
            .iter()
            .map(|p| (p.middle().distance(to), p.id, p.part))
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
            .map(|p| (p.middle().distance(to), p.id, p.part))
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
            .map(|p| (p.middle().distance(to), p.id, p.part))
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
    fn a_floor_is_laid_as_boards() {
        // A floor slab of one flat colour is a slab. What makes a floor read as a
        // floor is that you can see it is made of something.
        let mut bench = Bench::default();
        bench.add(Part::Floor, Vec3::ZERO, 0, 0);
        let boards = bench.to_plan().boxes;
        assert!(boards.len() > 3, "a floor came out as {} boxes", boards.len());

        // Boards, not slabs: each one long and narrow.
        for board in &boards {
            assert!(
                board.size.x > board.size.z * 2.0,
                "a board {:.2} by {:.2} is a tile",
                board.size.x,
                board.size.z
            );
        }

        // They cover the module without overlapping, and they are not one colour.
        let widest = boards.iter().map(|b| b.size.z).fold(0.0_f32, f32::max);
        assert!(
            (widest * boards.len() as f32 - MODULE).abs() < 0.2,
            "{} boards of {widest:.3} do not make a {MODULE} m floor",
            boards.len()
        );
        let shades: std::collections::HashSet<u32> = boards
            .iter()
            .map(|b| (b.colour.to_linear().red * 10_000.0) as u32)
            .collect();
        assert!(shades.len() > 1, "every board is the same colour");

        // And the whole thing still sits exactly on the ground it was placed on.
        let (low, high) = bench.to_plan().reach();
        assert!(low.y.abs() < 1.0e-4, "the floor sits {:.4} m off", low.y);
        assert!(
            (high.y - Part::Floor.size().y).abs() < 1.0e-4,
            "boarding changed how thick a floor is"
        );
    }

    #[test]
    fn two_floors_side_by_side_carry_on_the_same_grain() {
        // Deterministic from where a piece STANDS, not from the order it was
        // placed in. An id is an accident of that order, and a floor whose pattern
        // jumped at a seam because of it would read as two floors.
        let grain = |first: Vec3, second: Vec3| {
            let mut bench = Bench::default();
            bench.add(Part::Floor, first, 0, 0);
            bench.add(Part::Floor, second, 0, 0);
            bench
                .to_plan()
                .boxes
                .iter()
                .map(|b| (b.colour.to_linear().red * 10_000.0) as u32)
                .collect::<Vec<_>>()
        };
        // The same two slabs, laid in the opposite order.
        let one_way = grain(Vec3::ZERO, Vec3::new(MODULE, 0.0, 0.0));
        let mut other_way = grain(Vec3::new(MODULE, 0.0, 0.0), Vec3::ZERO);
        other_way.rotate_left(PLANKS);
        assert_eq!(
            one_way, other_way,
            "the grain depends on which slab was laid first"
        );
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
