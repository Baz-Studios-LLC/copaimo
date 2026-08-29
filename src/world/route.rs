//! Where a road can go, and where one has to be carried over water.
//!
//! # A straight line cannot go round a lake
//!
//! Roads between settlements were laid as a straight run from one to the next with
//! a wobble applied along it, and which settlements got joined at all was decided by
//! straight-line distance. Neither of those can see water. A road ran into a lake,
//! across the bottom of it and out the far side, and the attempted fix - pulling a
//! wet waypoint back toward the straight line until it was dry - made it worse,
//! because on a road that crosses a lake the straight line is the middle of the lake.
//!
//! The fault is not in the wobble. It is that the route was never asked where the
//! land is. This asks: the world is surveyed once into a coarse grid of dry and wet
//! cells, and a road is the cheapest walk over the DRY ones. A walk that cannot
//! enter water cannot come out under it, whatever it costs to go round - and going
//! round is the point. Long is by design.
//!
//! Where there is no dry walk at all, the two shores want a bridge, and this finds
//! the narrowest place to put one. It never fills water in to make a way across:
//! that is a causeway, and a causeway is not a bridge.

use crate::config::*;
use bevy::prelude::*;
use std::collections::VecDeque;

/// How coarse the survey is, in metres.
///
/// Fine enough to find its way round a bay and to measure a crossing, coarse enough
/// that surveying the whole world is a few tens of thousands of cells rather than
/// millions. The route that comes out is smoothed afterwards, so this is the size of
/// the DECISIONS a road makes, not the size of its corners.
pub const ROUTE_CELL: f32 = 64.0;

/// How far above the sea a cell has to stand before a road may use it.
///
/// Not zero. A cell whose middle is a handful of centimetres above the water is
/// beach that the tide is over, and a road along it reads as a road in the sea.
const DRY_BY: f32 = 1.2;

/// What a slope costs, as a multiplier on the ground crossed.
///
/// A cart road would rather go a long way round than up something steep, which is
/// most of why real roads follow valleys. High enough to make that choice, not so
/// high that a road refuses to climb at all.
const SLOPE_COSTS: f32 = 34.0;

/// What being near water costs, and how near counts.
///
/// Keeps a road off the beach without forbidding it. Without this a route hugs
/// every shoreline it passes, because the coast is flat and flat is cheap - and a
/// road on a beach is one storm from being a road in the sea.
const SHORE_COSTS: f32 = 1.1;
const SHORE_WITHIN: u16 = 3;

/// The longest water a bridge may span, in metres.
///
/// A crossing longer than this is not a bridge, it is a ferry. Two landmasses that
/// are further apart than this simply are not joined by road.
pub const BRIDGE_SPANS_AT_MOST: f32 = 1_400.0;

/// Marks a cell that is under water, in the island map.
const OPEN_SEA: u16 = u16::MAX;

/// A coarse survey of what is walkable.
pub struct Land {
    /// The world corner the grid starts at.
    low: Vec2,
    wide: usize,
    high: usize,
    dry: Vec<bool>,
    height: Vec<f32>,
    /// Which landmass each cell belongs to, or `OPEN_SEA`.
    island: Vec<u16>,
    /// How many cells it is to the nearest water, saturating.
    from_water: Vec<u16>,
    /// How many separate landmasses were found.
    pub islands: u16,
}

/// One water crossing: where it leaves the land, where it arrives, and how wide.
#[derive(Clone, Copy, Debug)]
pub struct Crossing {
    pub from: Vec2,
    pub to: Vec2,
    pub span: f32,
}

impl Land {
    /// Surveys the world once.
    pub fn survey(half: Vec2, ground: &dyn Fn(Vec2) -> f32) -> Land {
        let wide = ((half.x * 2.0) / ROUTE_CELL).ceil() as usize + 1;
        let high = ((half.y * 2.0) / ROUTE_CELL).ceil() as usize + 1;
        let low = -half;

        let mut dry = vec![false; wide * high];
        let mut height = vec![0.0f32; wide * high];
        for y in 0..high {
            for x in 0..wide {
                let at = low + Vec2::new(x as f32, y as f32) * ROUTE_CELL;
                let h = ground(at);
                height[y * wide + x] = h;
                // A cell is dry only if ALL of it is.
                //
                // Sampling one height per 64 m square calls a cell dry when its
                // middle is dry, and a stream 8 m wide fits inside one with room to
                // spare. Three points of road came out under water that way. The
                // corners are asked as well, so a cell with any water in it is water
                // as far as a road is concerned.
                let wettest = [
                    Vec2::new(-0.5, -0.5),
                    Vec2::new(0.5, -0.5),
                    Vec2::new(-0.5, 0.5),
                    Vec2::new(0.5, 0.5),
                ]
                .iter()
                .map(|corner| ground(at + *corner * ROUTE_CELL))
                .fold(h, f32::min);
                dry[y * wide + x] = wettest > SEA_LEVEL + DRY_BY;
            }
        }

        let mut land = Land {
            low,
            wide,
            high,
            dry,
            height,
            island: vec![OPEN_SEA; wide * high],
            from_water: vec![0; wide * high],
            islands: 0,
        };
        land.find_the_islands();
        land.measure_the_shore();
        land
    }

    /// Floods each landmass with its own number, so two settlements can be asked
    /// whether any dry walk between them exists at all.
    fn find_the_islands(&mut self) {
        let mut next = 0u16;
        for start in 0..self.dry.len() {
            if !self.dry[start] || self.island[start] != OPEN_SEA {
                continue;
            }
            if next == OPEN_SEA {
                break;
            }
            let mine = next;
            next += 1;
            let mut queue = VecDeque::new();
            queue.push_back(start);
            self.island[start] = mine;
            while let Some(cell) = queue.pop_front() {
                for near in self.beside(cell, false) {
                    if self.dry[near] && self.island[near] == OPEN_SEA {
                        self.island[near] = mine;
                        queue.push_back(near);
                    }
                }
            }
        }
        self.islands = next;
    }

    /// How far every dry cell is from water, in cells, by flooding out from the sea.
    fn measure_the_shore(&mut self) {
        let mut queue = VecDeque::new();
        for cell in 0..self.dry.len() {
            if !self.dry[cell] {
                self.from_water[cell] = 0;
                queue.push_back(cell);
            } else {
                self.from_water[cell] = u16::MAX;
            }
        }
        while let Some(cell) = queue.pop_front() {
            let out = self.from_water[cell].saturating_add(1);
            for near in self.beside(cell, false) {
                if self.from_water[near] > out {
                    self.from_water[near] = out;
                    queue.push_back(near);
                }
            }
        }
    }

    /// The cells next to this one: four-way, or eight-way when `corners`.
    fn beside(&self, cell: usize, corners: bool) -> Vec<usize> {
        let (x, y) = (cell % self.wide, cell / self.wide);
        let mut out = Vec::with_capacity(8);
        let steps: &[(i32, i32)] = if corners {
            &[
                (1, 0),
                (-1, 0),
                (0, 1),
                (0, -1),
                (1, 1),
                (1, -1),
                (-1, 1),
                (-1, -1),
            ]
        } else {
            &[(1, 0), (-1, 0), (0, 1), (0, -1)]
        };
        for (dx, dy) in steps {
            let (nx, ny) = (x as i32 + dx, y as i32 + dy);
            if nx < 0 || ny < 0 || nx >= self.wide as i32 || ny >= self.high as i32 {
                continue;
            }
            out.push(ny as usize * self.wide + nx as usize);
        }
        out
    }

    /// The middle of a cell, in world coordinates.
    fn middle(&self, cell: usize) -> Vec2 {
        self.low + Vec2::new((cell % self.wide) as f32, (cell / self.wide) as f32) * ROUTE_CELL
    }

    /// The cell a point falls in, clamped to the survey.
    fn cell_at(&self, at: Vec2) -> usize {
        let on = (at - self.low) / ROUTE_CELL;
        let x = (on.x.round() as i32).clamp(0, self.wide as i32 - 1) as usize;
        let y = (on.y.round() as i32).clamp(0, self.high as i32 - 1) as usize;
        y * self.wide + x
    }

    /// The nearest dry cell to a point, searched outward.
    ///
    /// A settlement placed by hand can sit on ground the survey calls wet - a coarse
    /// grid samples one height per cell, and a town beside a river or on a low spit
    /// lands on the wrong side of that. Its road still has to start somewhere.
    fn dry_near(&self, at: Vec2) -> Option<usize> {
        let start = self.cell_at(at);
        if self.dry[start] {
            return Some(start);
        }
        let (sx, sy) = ((start % self.wide) as i32, (start / self.wide) as i32);
        for ring in 1..24i32 {
            let mut best: Option<(usize, f32)> = None;
            for dy in -ring..=ring {
                for dx in -ring..=ring {
                    if dx.abs() != ring && dy.abs() != ring {
                        continue;
                    }
                    let (x, y) = (sx + dx, sy + dy);
                    if x < 0 || y < 0 || x >= self.wide as i32 || y >= self.high as i32 {
                        continue;
                    }
                    let cell = y as usize * self.wide + x as usize;
                    if !self.dry[cell] {
                        continue;
                    }
                    let away = self.middle(cell).distance(at);
                    if best.is_none_or(|(_, was)| away < was) {
                        best = Some((cell, away));
                    }
                }
            }
            if let Some((cell, _)) = best {
                return Some(cell);
            }
        }
        None
    }

    /// Which landmass a point stands on.
    pub fn island_at(&self, at: Vec2) -> Option<u16> {
        self.dry_near(at).map(|cell| self.island[cell])
    }

    /// What it costs to step into a cell, per metre travelled.
    fn footing(&self, from: usize, to: usize, run: f32) -> f32 {
        let rise = (self.height[to] - self.height[from]).abs();
        let shy = if self.from_water[to] <= SHORE_WITHIN {
            SHORE_COSTS * (SHORE_WITHIN + 1 - self.from_water[to]) as f32
                / (SHORE_WITHIN + 1) as f32
        } else {
            0.0
        };
        run * (1.0 + SLOPE_COSTS * (rise / run) + shy)
    }

    /// Every cost from one place, worked out once.
    ///
    /// A network of thirteen settlements wants the cost between every pair, and a
    /// walk that stops at the first destination has to be rerun for the second. This
    /// runs to exhaustion instead, so one walk per settlement answers for all of
    /// them - thirteen walks rather than a hundred and sixty-nine.
    pub fn walk_from(&self, from: Vec2) -> Option<Reach> {
        let start = self.dry_near(from)?;
        let mut cost = vec![f32::MAX; self.dry.len()];
        let mut came = vec![usize::MAX; self.dry.len()];
        let mut queue: std::collections::BinaryHeap<Step> = std::collections::BinaryHeap::new();
        cost[start] = 0.0;
        queue.push(Step { cost: 0.0, cell: start });

        while let Some(Step { cost: sofar, cell }) = queue.pop() {
            if sofar > cost[cell] {
                continue;
            }
            let here = self.middle(cell);
            for near in self.beside(cell, true) {
                if !self.dry[near] {
                    continue;
                }
                let run = here.distance(self.middle(near));
                let then = sofar + self.footing(cell, near, run);
                if then < cost[near] {
                    cost[near] = then;
                    came[near] = cell;
                    queue.push(Step { cost: then, cell: near });
                }
            }
        }
        Some(Reach { start, cost, came, at: from })
    }

    /// The cheapest walk from one point to another that never leaves dry land.
    ///
    /// `None` when there is no such walk - which is not a failure but an answer: it
    /// means the two are on different landmasses and want a bridge.
    pub fn route(&self, from: Vec2, to: Vec2) -> Option<Vec<Vec2>> {
        self.walk_from(from)?.route_to(self, to)
    }

    /// Every cell of one landmass that stands on its coast.
    fn coast_of(&self, island: u16) -> Vec<usize> {
        (0..self.dry.len())
            .filter(|cell| self.island[*cell] == island && self.from_water[*cell] == 1)
            .collect()
    }

    /// Whether the straight line between two shores is water the whole way across.
    ///
    /// A bridge is a span over open water. Two coasts can be close together and
    /// still have a headland between them, and a "crossing" that clips dry land
    /// halfway is not one span - it is two bridges and a road nobody built.
    fn open_between(&self, from: Vec2, to: Vec2) -> bool {
        let span = from.distance(to);
        let steps = (span / (ROUTE_CELL * 0.5)).ceil().max(2.0) as usize;
        // The ends are the shores themselves, and so is everything within a cell of
        // them - `cell_at` rounds to the nearest cell middle, so a sample 32 m off a
        // shore is still IN the shore's cell. Skipping only the endpoints rejected
        // every crossing there was and left two landmasses off the map with no
        // bridge and no complaint.
        let skirt = ROUTE_CELL / span.max(1.0);
        (1..steps)
            .map(|step| step as f32 / steps as f32)
            .filter(|along| *along > skirt && *along < 1.0 - skirt)
            .all(|along| !self.dry[self.cell_at(from.lerp(to, along))])
    }

    /// The narrowest water between two landmasses, if they are close enough to bridge.
    ///
    /// # Eight directions is not a search
    ///
    /// This used to march out from each coast cell along the eight compass bearings,
    /// which finds a crossing only where the water happens to lie square to the
    /// grid. On this world that returned spans of 704 m and 1216 m - not because the
    /// channels are that wide, but because those were the narrowest gaps that
    /// happened to be axis-aligned.
    ///
    /// Every pair of coast cells is measured instead, and the line between them has
    /// to be open water the whole way. What comes back is the actual narrowest place
    /// to put a bridge.
    pub fn crossing(&self, one: u16, two: u16) -> Option<Crossing> {
        let here = self.coast_of(one);
        let there = self.coast_of(two);
        let mut best: Option<Crossing> = None;
        for near in &here {
            let from = self.middle(*near);
            for far in &there {
                let to = self.middle(*far);
                let span = from.distance(to);
                if span > BRIDGE_SPANS_AT_MOST {
                    continue;
                }
                if best.is_some_and(|had| span >= had.span) {
                    continue;
                }
                if !self.open_between(from, to) {
                    continue;
                }
                best = Some(Crossing { from, to, span });
            }
        }
        best
    }
}

/// Everywhere reachable on foot from one place, and what it costs to get there.
pub struct Reach {
    start: usize,
    at: Vec2,
    cost: Vec<f32>,
    came: Vec<usize>,
}

impl Reach {
    /// What the cheapest dry walk to a place costs, if there is one.
    pub fn cost_to(&self, land: &Land, to: Vec2) -> Option<f32> {
        let finish = land.dry_near(to)?;
        (self.cost[finish] < f32::MAX).then_some(self.cost[finish])
    }

    /// That walk, as points from here to there.
    pub fn route_to(&self, land: &Land, to: Vec2) -> Option<Vec<Vec2>> {
        let finish = land.dry_near(to)?;
        if self.cost[finish] == f32::MAX {
            return None;
        }
        let mut walk = vec![to];
        let mut cell = finish;
        while cell != self.start {
            walk.push(land.middle(cell));
            cell = *self.came.get(cell)?;
            if cell == usize::MAX {
                return None;
            }
        }
        walk.push(self.at);
        walk.reverse();
        Some(walk)
    }
}

/// One entry in the walk's queue, ordered cheapest-first.
#[derive(PartialEq)]
struct Step {
    cost: f32,
    cell: usize,
}

impl Eq for Step {}

impl Ord for Step {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Reversed, because `BinaryHeap` is a max-heap and this wants the cheapest.
        other.cost.total_cmp(&self.cost)
    }
}

impl PartialOrd for Step {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A world cut in half by a channel, with a narrow neck at one end.
    fn split_world(at: Vec2) -> f32 {
        // Water in a band across the middle, except for a gap the road could use if
        // it went round - which is exactly what a route is supposed to find.
        if at.y.abs() < 200.0 && at.x < 1_500.0 {
            -10.0
        } else {
            30.0
        }
    }

    #[test]
    fn a_route_goes_round_the_water_rather_than_through_it() {
        let half = Vec2::new(2_000.0, 2_000.0);
        let land = Land::survey(half, &split_world);
        let walk = land
            .route(Vec2::new(-1_000.0, -1_000.0), Vec2::new(-1_000.0, 1_000.0))
            .expect("both ends are on one landmass, joined round the end of the channel");

        // Every step of it is on dry ground. This is the whole claim.
        for point in &walk {
            assert!(
                split_world(*point) > SEA_LEVEL,
                "a route ran through water at {point:?}",
            );
        }
        // And it is much longer than the straight line, because it went round.
        let far: f32 = walk.windows(2).map(|p| p[0].distance(p[1])).sum();
        assert!(
            far > 2_000.0 * 1.5,
            "a route that goes round the end of a channel cannot be nearly straight - it ran {far:.0} m",
        );
    }

    #[test]
    fn two_landmasses_report_a_crossing_rather_than_a_route() {
        // Water all the way across: no walk exists, and a bridge is the answer.
        let moat = |at: Vec2| if at.y.abs() < 160.0 { -10.0 } else { 30.0 };
        let half = Vec2::new(1_500.0, 1_500.0);
        let land = Land::survey(half, &moat);

        let north = Vec2::new(0.0, 800.0);
        let south = Vec2::new(0.0, -800.0);
        assert!(
            land.route(south, north).is_none(),
            "there is no dry walk across a moat, and pretending there is one is how a road ends up under water",
        );

        let (a, b) = (
            land.island_at(south).expect("south is land"),
            land.island_at(north).expect("north is land"),
        );
        assert_ne!(a, b, "a moat makes two landmasses");
        let crossing = land.crossing(a, b).expect("a moat this narrow can be bridged");
        assert!(
            crossing.span < 500.0,
            "the crossing found was {:.0} m, which is not the narrow neck",
            crossing.span,
        );
        // Both ends stand on dry ground - a bridge lands on shores.
        assert!(moat(crossing.from) > SEA_LEVEL && moat(crossing.to) > SEA_LEVEL);
    }
}
