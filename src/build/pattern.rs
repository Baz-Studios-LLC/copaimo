//! Asking for something to be built, so there is something to edit.
//!
//! # A generator that hands you pieces, not a building
//!
//! The point of generating here is not to produce a finished house. It is to skip
//! the boring half of making one.
//!
//! Laying eight floor slabs and fourteen wall panels by hand takes a few minutes
//! and involves no decisions — every one of them goes exactly where the last one
//! implies. The interesting part is what you do afterwards: take a wall out for a
//! wider door, drop the roof a storey, put a lean-to on the back. So a generator
//! fills the bench with **ordinary pieces**, indistinguishable from placed ones,
//! and then gets out of the way.
//!
//! That is the whole design constraint, and it is why this is fifty lines of
//! arithmetic rather than anything clever: whatever comes out has to be editable
//! by exactly the same keys that made it, or it is a black box with a building
//! inside it.
//!
//! # Varied, and repeatable
//!
//! Everything is drawn from a seed, so asking twice gives two different buildings
//! and asking for the same seed twice gives the same one. The proportions, the
//! materials, whether there is a porch — all of it. What is NOT random is anything
//! structural: a house has a door, a roof covers its footprint, and nothing is
//! ever generated floating.

use bevy::prelude::*;
use terrain_core::Draw;

use crate::build::kit::{Bench, Part, MODULE};

/// What can be asked for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Pattern {
    /// A house: floor, walls, door, roof.
    House,
    /// A run of posts and rails.
    Fence,
    /// Tall and narrow, several storeys.
    Tower,
    /// One storey, open at the front — a market stall, a cart shed.
    Shelter,
}

impl Pattern {
    pub const ALL: [Pattern; 4] = [
        Pattern::House,
        Pattern::Fence,
        Pattern::Tower,
        Pattern::Shelter,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Pattern::House => "house",
            Pattern::Fence => "fence",
            Pattern::Tower => "tower",
            Pattern::Shelter => "shelter",
        }
    }
}

/// Fills a bench with something, replacing whatever was on it.
///
/// Clears first, deliberately. Generating a house into a half-built one leaves two
/// buildings inside each other, and a maker who wanted to keep what they had would
/// have saved it.
pub fn draw(bench: &mut Bench, what: Pattern, seed: u32) {
    bench.clear();
    let mut draw = Draw::new(seed);

    // Two materials chosen up front, so a building is made of a thing rather than
    // of six things. Timber for the frame, something else for the roof, which is
    // how nearly every real building is put together and most of why they read as
    // whole objects.
    let timber = (draw.unit() * 2.0) as usize % 2;
    let roofing = 2 + (draw.unit() * 3.0) as usize % 3;
    let stone = 4;

    match what {
        Pattern::House => house(bench, &mut draw, timber, roofing, stone),
        Pattern::Fence => fence(bench, &mut draw, timber),
        Pattern::Tower => tower(bench, &mut draw, timber, roofing, stone),
        Pattern::Shelter => shelter(bench, &mut draw, timber, roofing),
    }

    bench.name = format!("{} {seed}", what.name());
}

/// The floor slabs, and how far the building reaches.
fn floors(bench: &mut Bench, wide: u32, deep: u32, tint: usize) -> (f32, f32) {
    for x in 0..wide {
        for z in 0..deep {
            bench.add(
                Part::Floor,
                Vec3::new(x as f32 * MODULE, 0.0, z as f32 * MODULE),
                0,
                tint,
            );
        }
    }
    // The outer face of the outermost slab, either way.
    (
        (wide - 1) as f32 * MODULE + MODULE * 0.5,
        (deep - 1) as f32 * MODULE + MODULE * 0.5,
    )
}

/// Walls round a footprint at one storey, with a gap where the door goes.
///
/// `doorway` is which panel along the south face to leave out. A house walled all
/// the way round is a box; the gap is most of what makes it read as somewhere
/// people go in and out of.
fn walls(
    bench: &mut Bench,
    wide: u32,
    deep: u32,
    reach: (f32, f32),
    foot: f32,
    tint: usize,
    doorway: Option<u32>,
) {
    let (far_x, far_z) = reach;
    for x in 0..wide {
        let along = x as f32 * MODULE;
        bench.add(Part::Wall, Vec3::new(along, foot, -MODULE * 0.5), 0, tint);
        if doorway != Some(x) {
            bench.add(Part::Wall, Vec3::new(along, foot, far_z), 0, tint);
        }
    }
    for z in 0..deep {
        let along = z as f32 * MODULE;
        bench.add(Part::Wall, Vec3::new(-MODULE * 0.5, foot, along), 1, tint);
        bench.add(Part::Wall, Vec3::new(far_x, foot, along), 1, tint);
    }
}

/// A roof over a footprint, and the cap along its ridge.
fn roof(bench: &mut Bench, wide: u32, deep: u32, foot: f32, tint: usize) {
    for x in 0..wide {
        for z in 0..deep {
            bench.add(
                Part::Roof,
                Vec3::new(x as f32 * MODULE, foot, z as f32 * MODULE),
                0,
                tint,
            );
        }
    }
    // Along the ridge, which for a wedge runs the depth of each panel — so the cap
    // runs the same way, down the middle of the span.
    let middle = (wide - 1) as f32 * MODULE * 0.5;
    for z in 0..deep {
        bench.add(
            Part::Cap,
            Vec3::new(middle, foot + Part::Roof.size().y, z as f32 * MODULE),
            0,
            tint,
        );
    }
}

fn house(bench: &mut Bench, draw: &mut Draw, timber: usize, roofing: usize, stone: usize) {
    let wide = 2 + (draw.unit() * 3.0) as u32;
    let deep = 2 + (draw.unit() * 2.0) as u32;
    let storeys = 1 + (draw.unit() * 2.0) as u32;

    let reach = floors(bench, wide, deep, stone);
    let storey = Part::Wall.size().y;
    for level in 0..storeys {
        let foot = Part::Floor.size().y + level as f32 * storey;
        // A door on the ground floor only, which is the sort of thing that would
        // be funny once and wrong every time after.
        let doorway = (level == 0).then(|| (draw.unit() * wide as f32) as u32);
        walls(bench, wide, deep, reach, foot, timber, doorway);
    }
    roof(
        bench,
        wide,
        deep,
        Part::Floor.size().y + storeys as f32 * storey,
        roofing,
    );
}

fn tower(bench: &mut Bench, draw: &mut Draw, timber: usize, roofing: usize, stone: usize) {
    // Narrow by definition — a wide tower is a house. Two modules square is the
    // smallest thing with an inside.
    let side = 2;
    let storeys = 3 + (draw.unit() * 3.0) as u32;

    let reach = floors(bench, side, side, stone);
    let storey = Part::Wall.size().y;
    for level in 0..storeys {
        let foot = Part::Floor.size().y + level as f32 * storey;
        let doorway = (level == 0).then_some(0);
        walls(bench, side, side, reach, foot, stone, doorway);
        // A floor at every level, which is what makes it a tower rather than a
        // chimney.
        if level > 0 {
            for x in 0..side {
                for z in 0..side {
                    bench.add(
                        Part::Floor,
                        Vec3::new(x as f32 * MODULE, foot, z as f32 * MODULE),
                        0,
                        timber,
                    );
                }
            }
        }
    }
    roof(
        bench,
        side,
        side,
        Part::Floor.size().y + storeys as f32 * storey,
        roofing,
    );
}

fn shelter(bench: &mut Bench, draw: &mut Draw, timber: usize, roofing: usize) {
    let wide = 2 + (draw.unit() * 3.0) as u32;
    let deep = 2;

    let reach = floors(bench, wide, deep, timber);
    let foot = Part::Floor.size().y;
    // Posts at the front corners rather than a wall, which is the whole point of a
    // shelter: it is a roof you can walk under.
    for x in 0..wide {
        bench.add(
            Part::Post,
            Vec3::new(x as f32 * MODULE, foot, -MODULE * 0.5),
            0,
            timber,
        );
        bench.add(
            Part::Post,
            Vec3::new(x as f32 * MODULE, foot + Part::Post.size().y, -MODULE * 0.5),
            0,
            timber,
        );
    }
    // And a back wall, so it has a weather side.
    for x in 0..wide {
        bench.add(
            Part::Wall,
            Vec3::new(x as f32 * MODULE, foot, reach.1),
            0,
            timber,
        );
    }
    // A beam across the front to carry the roof, which is the piece that stops it
    // looking like a roof hovering over some sticks.
    for x in 0..wide {
        bench.add(
            Part::Beam,
            Vec3::new(
                x as f32 * MODULE,
                foot + Part::Wall.size().y,
                -MODULE * 0.5,
            ),
            0,
            timber,
        );
    }
    roof(bench, wide, deep, foot + Part::Wall.size().y, roofing);
}

fn fence(bench: &mut Bench, draw: &mut Draw, timber: usize) {
    let run = 4 + (draw.unit() * 6.0) as u32;
    let rails = 2 + (draw.unit() * 2.0) as u32;

    for step in 0..=run {
        let along = step as f32 * MODULE;
        bench.add(Part::Post, Vec3::new(along, 0.0, 0.0), 0, timber);
        if step == run {
            continue;
        }
        // Rails span between posts, so they sit half a module along.
        for rail in 0..rails {
            let up = 0.25 + rail as f32 * (Part::Post.size().y - 0.5) / rails.max(1) as f32;
            bench.add(
                Part::Rail,
                Vec3::new(along + MODULE * 0.5, up, 0.0),
                0,
                timber,
            );
        }
    }
}

/// Whether a tint index is one of the ones on the shelf. Guards the arithmetic
/// above, which picks by remainder and would silently wrap if the shelf shrank.
#[cfg(test)]
fn on_the_shelf(tint: usize) -> bool {
    tint < crate::build::kit::TINTS.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn everything_asked_for_comes_out_as_a_building() {
        for what in Pattern::ALL {
            for seed in [1_u32, 7, 99, 4_242] {
                let mut bench = Bench::default();
                draw(&mut bench, what, seed);

                assert!(
                    !bench.is_empty(),
                    "{} {seed} generated nothing",
                    what.name()
                );
                let plan = bench.to_plan();
                // At least one box a piece. A floor is laid as several boards, so
                // these two counts are no longer the same number.
                assert!(
                    plan.boxes.len() >= bench.len(),
                    "{} lost a piece: {} boxes from {} pieces",
                    what.name(),
                    plan.boxes.len(),
                    bench.len()
                );
                assert!(plan.high > 1.0, "{} {seed} is {:.2} m tall", what.name(), plan.high);

                // Nothing floating and nothing underground. A generator that put a
                // wall at head height would be a bug nobody could see from the
                // outside of the finished building.
                let (low, _) = plan.reach();
                assert!(
                    low.y >= -0.01,
                    "{} {seed} reaches {:.2} m below the floor",
                    what.name(),
                    low.y
                );
                for piece in bench.pieces() {
                    assert!(on_the_shelf(piece.tint), "a colour off the shelf");
                }
            }
        }
    }

    #[test]
    fn what_is_generated_can_be_edited_like_anything_else() {
        // The whole design constraint. Generating is meant to skip the boring half
        // of making a building — laying the slabs and panels that each go exactly
        // where the last one implies — and then get out of the way. If what comes
        // out cannot be taken apart with the same keys that would have made it, it
        // is a black box with a building inside it.
        let mut bench = Bench::default();
        draw(&mut bench, Pattern::House, 12);
        let was = bench.len();

        assert!(bench.undo().is_some(), "the last piece could not be taken back");
        assert!(
            bench.remove_nearest(Vec3::new(0.0, 1.0, 0.0), MODULE * 3.0).is_some(),
            "nothing near the middle could be removed"
        );
        assert_eq!(bench.len(), was - 2);

        // And more can be added to it, on the same lattice.
        assert!(
            bench.add(Part::Post, Vec3::new(-MODULE, 0.0, -MODULE), 0, 0).is_some(),
            "a generated building would not take another piece"
        );
    }

    #[test]
    fn asking_twice_gives_two_buildings_and_the_same_seed_gives_one() {
        let shape = |seed: u32| {
            let mut bench = Bench::default();
            draw(&mut bench, Pattern::House, seed);
            let plan = bench.to_plan();
            (bench.len(), format!("{:.2}x{:.2}x{:.2}", plan.half_w, plan.half_d, plan.high))
        };

        // Repeatable: the same ask twice is the same building, which is what makes
        // "that one, but wider" a thing somebody can do.
        assert_eq!(shape(5), shape(5));

        // And varied: across a spread of seeds there is more than one building.
        let spread: std::collections::HashSet<String> =
            (0..24).map(|seed| shape(seed).1).collect();
        assert!(
            spread.len() > 3,
            "twenty-four houses came out as {} different buildings",
            spread.len()
        );
    }

    #[test]
    fn a_house_has_a_way_in() {
        // A house walled the whole way round is a box. The gap is most of what
        // makes it read as somewhere people go in and out of, and it is the one
        // structural thing here that is NOT left to the seed.
        for seed in 0..12 {
            let mut bench = Bench::default();
            draw(&mut bench, Pattern::House, seed);

            // The two faces that run along X, at ground-floor height. Told apart
            // by their TURN, not by their position: the side walls are quarter
            // turned and stand at every z, so a filter on z alone counts them as
            // part of whichever face they are nearest — which is what caught this
            // test out the first time.
            let foot = Part::Floor.size().y;
            let facing: Vec<_> = bench
                .pieces()
                .iter()
                .filter(|p| p.part == Part::Wall && p.foot.y == foot && p.quarters == 0)
                .collect();
            let nearest = facing.iter().map(|p| p.foot.z).fold(f32::MAX, f32::min);
            let furthest = facing.iter().map(|p| p.foot.z).fold(f32::MIN, f32::max);
            let north = facing.iter().filter(|p| p.foot.z == nearest).count();
            let south = facing.iter().filter(|p| p.foot.z == furthest).count();
            assert_eq!(
                south + 1,
                north,
                "seed {seed}: {north} panels north, {south} south - that is not one doorway"
            );
        }
    }
}
