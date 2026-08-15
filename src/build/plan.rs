//! A baked building, as the bench hands it over.
//!
//! Opificium's builder draws a house as *parts* on a sixteenth-metre lattice —
//! walls, braces, a roof — and bakes them down to plain boxes with their colours
//! already looked up. `assets/buildings/<name>.json` is what comes out, and this
//! reads it. The layout is Opificium's `FORMATS.md`; this file is the game's half
//! of that contract and should be read beside it.
//!
//! # Only the finished building, for now
//!
//! A baked file carries `levels` — the original, then each upgrade — and each
//! level carries `phases`, the steps of raising it. Both are for a game that
//! shows building work happening. The top-level `boxes` and `marks` are **the
//! first level, finished**, which the format guarantees, so reading those alone
//! is a supported way to read the file and not a shortcut taken here.
//!
//! # An unknown shape refuses the building
//!
//! The bench and the game draw the four forms from separate code — they share
//! none — so a shape is only the same shape in both because it is written out
//! twice. A form this doesn't know is therefore not a box: it is a beam the two
//! programs disagree about. Refusing the file and saying which form says so;
//! quietly substituting a cuboid would put a solid block where a cut brace
//! belongs and look like a fault in the drawing.

use bevy::prelude::*;
use serde::Deserialize;

/// The shape of one box. Four of them, plus the two names `cut` replaced.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Form {
    /// The plain cuboid, which is most of everything.
    Box,
    /// A gable's prism: a triangle with its peak in the middle, its ridge line
    /// running along the box's depth.
    Wedge,
    /// The same prism turned to run lengthwise, apex up — a ridge cap.
    Ridge,
    /// A box with a face cut back at each end, at -X and at +X.
    ///
    /// The numbers are RUNS as fractions of the piece's own length: how far
    /// along it the saw travels while crossing its full height. Positive cuts
    /// the TOP face back, negative the BOTTOM — which is the whole trick, since
    /// a top cut at one end and a bottom cut at the other leaves the two ends
    /// parallel, and that is what a diagonal brace is.
    Cut { low: f32, high: f32 },
    /// A truncated pyramid: four faces sloping in from the foot to a flat top,
    /// which is a hip roof with a deck. The numbers are how much of the box's
    /// width and depth the top keeps.
    Hip { across: f32, along: f32 },
}

impl Form {
    fn read(word: &str) -> Result<Self, String> {
        // The two the bench no longer writes. Each could say only that ALL of
        // one end was gone, which is why they became a property with a number.
        match word {
            "box" => return Ok(Form::Box),
            "wedge" => return Ok(Form::Wedge),
            "ridge" => return Ok(Form::Ridge),
            "mitre" => return Ok(Form::Cut { low: 0.0, high: 1.0 }),
            "mitre-back" => return Ok(Form::Cut { low: 1.0, high: 0.0 }),
            _ => {}
        }

        if let Some(pair) = word.strip_prefix("cut:") {
            let (low, high) = two_numbers(pair, word)?;
            return Ok(Form::Cut { low, high });
        }
        if let Some(pair) = word.strip_prefix("hip:") {
            let (across, along) = two_numbers(pair, word)?;
            return Ok(Form::Hip { across, along });
        }

        Err(format!("unknown form {word:?}"))
    }
}

/// `<a>x<b>`, as both `cut:` and `hip:` spell their pair.
fn two_numbers(pair: &str, whole: &str) -> Result<(f32, f32), String> {
    // From the RIGHT, because a leading minus is legal and `x` is only ever the
    // separator once: `cut:0.2500x-0.2500` splits at the sole `x`, and rsplit
    // reaches it whether or not the first number is negative.
    let (a, b) = pair
        .rsplit_once('x')
        .ok_or_else(|| format!("form {whole:?} should read <a>x<b>"))?;
    let read = |s: &str| {
        s.parse::<f32>()
            .map_err(|_| format!("form {whole:?} has {s:?} where a number goes"))
    };
    Ok((read(a)?, read(b)?))
}

/// One box of a finished building, in the building's own space: metres, y=0 at
/// the ground it stands on, +X the front.
#[derive(Clone, Debug)]
pub struct Block {
    /// The middle of the box.
    pub at: Vec3,
    /// Full extents, not half.
    pub size: Vec3,
    pub turn: Quat,
    pub form: Form,
    /// Already looked up from the palette at the bench. The game paints exactly
    /// what it is given.
    pub colour: Color,
    /// What the box IS — `walls`, `roof`, `footings`. Carried rather than used:
    /// it is what a game showing building work would raise things in order by,
    /// and it is enough to do that without reading a level's phases at all.
    #[allow(dead_code)]
    pub stage: String,
}

impl Block {
    /// Whether this box lets light through, and so cannot go in the same mesh
    /// as the walls.
    pub fn is_glass(&self) -> bool {
        self.colour.alpha() < 0.999
    }
}

/// What a place is FOR: a door to walk in by, a sign to stand at.
#[derive(Clone, Debug)]
pub struct Mark {
    pub mark: String,
    pub at: Vec3,
    /// Which way it faces, about Y.
    pub yaw: f32,
}

/// A finished building, ready to raise.
#[derive(Clone, Debug)]
pub struct Plan {
    pub name: String,
    /// What the village raises it as — `house`, `tavern`, `watchtower`. The
    /// bench asks when a work is carried in, so it is a fact and not a guess.
    pub kind: String,
    /// The finished footprint: the plot a village clears, the obstacle while it
    /// is being raised, and the walkable shell when it is done.
    ///
    /// Nothing lays out a street yet, so nothing asks. It is read because it is
    /// the number that decides whether two buildings fit, and that question
    /// arrives the moment a site holds more than one.
    #[allow(dead_code)]
    pub half_w: f32,
    #[allow(dead_code)]
    pub half_d: f32,
    #[allow(dead_code)]
    pub high: f32,
    pub boxes: Vec<Block>,
    pub marks: Vec<Mark>,
}

impl Plan {
    /// How far the building actually reaches, corner to corner, in its own
    /// space.
    ///
    /// Every box's eight corners, turned. The cheap answer — a sphere around
    /// each box — is wrong in the way that matters: a nine-metre ridge cap's
    /// sphere hangs two hundred millimetres below the ground it sits four metres
    /// above, and a bound that buries a cottage cannot answer the question it
    /// exists for. Every form fits inside its own box, so the box's corners are
    /// the whole story.
    pub fn reach(&self) -> (Vec3, Vec3) {
        let mut low = Vec3::splat(f32::MAX);
        let mut high = Vec3::splat(f32::MIN);
        for block in &self.boxes {
            let half = block.size * 0.5;
            for corner in CORNERS {
                let at = block.at + block.turn * (half * Vec3::from_array(corner));
                low = low.min(at);
                high = high.max(at);
            }
        }
        (low, high)
    }

    /// Reads one baked building. Says why on refusal rather than logging, so the
    /// caller can name the file it came from.
    pub fn read(json: &str) -> Result<Self, String> {
        let raw: RawPlan = serde_json::from_str(json).map_err(|why| why.to_string())?;
        if raw.format > FORMAT {
            return Err(format!(
                "format {} - this game reads up to {FORMAT}",
                raw.format
            ));
        }

        let boxes = raw
            .boxes
            .iter()
            .map(Block::from_raw)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Plan {
            kind: raw.kind.unwrap_or_else(|| kind_from_name(&raw.name)),
            name: raw.name,
            half_w: raw.half_w,
            half_d: raw.half_d,
            high: raw.high,
            boxes,
            marks: raw.marks.iter().map(Mark::from_raw).collect(),
        })
    }
}

/// The eight corners of a box, as signs on its half-extents.
const CORNERS: [[f32; 3]; 8] = [
    [-1.0, -1.0, -1.0],
    [1.0, -1.0, -1.0],
    [-1.0, 1.0, -1.0],
    [1.0, 1.0, -1.0],
    [-1.0, -1.0, 1.0],
    [1.0, -1.0, 1.0],
    [-1.0, 1.0, 1.0],
    [1.0, 1.0, 1.0],
];

/// The highest baked format this reads. A file claiming a newer one is refused:
/// it may lean on something not written here yet, and a building half-understood
/// is worse than one that says it cannot be read.
const FORMAT: u32 = 2;

/// The kinds a village knows how to raise, longest first so that `longhouse`
/// wins over `house` when a name begins with it.
const KINDS: [&str; 18] = [
    "smokehouse",
    "blacksmith",
    "storehouse",
    "watchtower",
    "longhouse",
    "herbalist",
    "townhall",
    "granary",
    "sawmill",
    "smithy",
    "tavern",
    "weaver",
    "bakery",
    "shrine",
    "house",
    "well",
    "mill",
    "dock",
];

/// The older reading, for drawings baked before the bench asked what a work was:
/// the longest kind whose word begins the name.
fn kind_from_name(name: &str) -> String {
    let lower = name.to_lowercase();
    KINDS
        .iter()
        .find(|kind| lower.starts_with(*kind))
        .map_or_else(|| "house".to_string(), |kind| (*kind).to_string())
}

// --------------------------------------------------------------- as it is written

#[derive(Deserialize)]
struct RawPlan {
    #[serde(default = "one")]
    format: u32,
    name: String,
    kind: Option<String>,
    half_w: f32,
    half_d: f32,
    high: f32,
    #[serde(default)]
    boxes: Vec<RawBlock>,
    #[serde(default)]
    marks: Vec<RawMark>,
}

fn one() -> u32 {
    1
}

#[derive(Deserialize)]
struct RawBlock {
    at: [f32; 3],
    size: [f32; 3],
    /// x, y, z, w — the order glTF and every engine here writes a quaternion in.
    #[serde(default = "no_turn")]
    turn: [f32; 4],
    form: String,
    rgb: [u8; 3],
    #[serde(default = "opaque")]
    alpha: f32,
    #[serde(default)]
    stage: String,
}

fn no_turn() -> [f32; 4] {
    [0.0, 0.0, 0.0, 1.0]
}

fn opaque() -> f32 {
    1.0
}

#[derive(Deserialize)]
struct RawMark {
    mark: String,
    at: [f32; 3],
    #[serde(default)]
    yaw: f32,
}

impl Block {
    fn from_raw(raw: &RawBlock) -> Result<Self, String> {
        Ok(Block {
            at: Vec3::from_array(raw.at),
            size: Vec3::from_array(raw.size),
            // Normalised because a quaternion written to four decimal places is
            // very nearly a rotation and not quite one, and Bevy's transforms
            // will happily scale a mesh with the difference.
            turn: Quat::from_array(raw.turn).normalize(),
            form: Form::read(&raw.form)?,
            // `rgb` is sRGB, as written down; vertex colours are linear.
            colour: Color::srgba_u8(raw.rgb[0], raw.rgb[1], raw.rgb[2], to_byte(raw.alpha)),
            stage: raw.stage.clone(),
        })
    }
}

fn to_byte(alpha: f32) -> u8 {
    (alpha.clamp(0.0, 1.0) * 255.0).round() as u8
}

impl Mark {
    fn from_raw(raw: &RawMark) -> Self {
        Mark {
            mark: raw.mark.clone(),
            at: Vec3::from_array(raw.at),
            yaw: raw.yaw,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The example straight out of Opificium's `FORMATS.md`, so this is tested
    /// against the contract rather than against its own writer — which lives in
    /// another program entirely.
    const LONGHOUSE: &str = r#"{
      "format": 2, "name": "longhouse1-10people",
      "half_w": 3.65, "half_d": 6.7, "high": 5.2,
      "boxes": [ { "at": [0,1.25,0], "size": [4,2.5,0.25], "turn": [0,0,0,1],
                   "form": "box", "rgb": [110,92,70], "alpha": 1.0, "cloth": "wood",
                   "stage": "walls" } ],
      "marks": [ { "mark": "door", "at": [3.65,0.375,0.0], "yaw": 0.0 } ]
    }"#;

    #[test]
    fn the_documented_example_reads() {
        let plan = Plan::read(LONGHOUSE).expect("the contract's own example should read");
        assert_eq!(plan.name, "longhouse1-10people");
        assert_eq!(plan.half_w, 3.65);
        assert_eq!(plan.boxes.len(), 1);

        let wall = &plan.boxes[0];
        assert_eq!(wall.at, Vec3::new(0.0, 1.25, 0.0));
        assert_eq!(wall.size, Vec3::new(4.0, 2.5, 0.25));
        assert_eq!(wall.form, Form::Box);
        assert_eq!(wall.stage, "walls");
        assert!(!wall.is_glass());

        assert_eq!(plan.marks.len(), 1);
        assert_eq!(plan.marks[0].mark, "door");
    }

    #[test]
    fn a_kind_is_taken_from_the_name_when_the_card_is_absent() {
        // The older reading, and the reason the list is longest-first: a
        // longhouse must not come out a house.
        let plan = Plan::read(LONGHOUSE).unwrap();
        assert_eq!(plan.kind, "longhouse");
    }

    #[test]
    fn the_card_wins_over_the_name_when_it_is_there() {
        let json = LONGHOUSE.replace(r#""name":"#, r#""kind": "tavern", "name":"#);
        assert_eq!(Plan::read(&json).unwrap().kind, "tavern");
    }

    #[test]
    fn every_form_the_bench_can_write_is_understood() {
        for (word, wanted) in [
            ("box", Form::Box),
            ("wedge", Form::Wedge),
            ("ridge", Form::Ridge),
            // The parallelogram from the contract: top cut at one end, bottom at
            // the other, so the ends come out parallel. That is a brace.
            (
                "cut:0.2500x-0.2500",
                Form::Cut {
                    low: 0.25,
                    high: -0.25,
                },
            ),
            (
                "hip:0.5000x0.6250",
                Form::Hip {
                    across: 0.5,
                    along: 0.625,
                },
            ),
            // What `cut` replaced. Still read, so older drawings open.
            (
                "mitre",
                Form::Cut {
                    low: 0.0,
                    high: 1.0,
                },
            ),
            (
                "mitre-back",
                Form::Cut {
                    low: 1.0,
                    high: 0.0,
                },
            ),
        ] {
            assert_eq!(Form::read(word), Ok(wanted), "reading {word:?}");
        }
    }

    #[test]
    fn a_shape_this_game_does_not_know_refuses_the_building() {
        // Not a box. The two programs draw shapes from separate code, so an
        // unknown form is a beam they disagree about — and a solid cuboid where
        // a cut brace belongs would read as a fault in the drawing.
        let json = LONGHOUSE.replace(r#""form": "box""#, r#""form": "dovetail""#);
        let why = Plan::read(&json).unwrap_err();
        assert!(why.contains("dovetail"), "unhelpful reason: {why}");
    }

    #[test]
    fn a_newer_format_is_refused_rather_than_half_read() {
        let json = LONGHOUSE.replace(r#""format": 2"#, r#""format": 9"#);
        assert!(Plan::read(&json).unwrap_err().contains("format 9"));
    }

    #[test]
    fn glass_is_told_apart_by_its_alpha() {
        let json = LONGHOUSE.replace(r#""alpha": 1.0"#, r#""alpha": 0.4"#);
        assert!(Plan::read(&json).unwrap().boxes[0].is_glass());
    }
}
