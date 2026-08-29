//! Paints the world as a map: the ground, and what people built on it.
//!
//! # One painting, two maps
//!
//! This started inside the terrain tool's overview, which was the only map in the
//! game. There is a second now - the one a player pulls up with M - and the two must
//! show the same world or the map is lying to somebody. So the painting lives here
//! and both ask for it.
//!
//! # Ground colour alone is a map of a world nobody lives in
//!
//! A settlement LEVELS its ground, so the biggest city on the map came out as a
//! slightly flatter patch of the same green as the country round it - which is to
//! say, as nothing at all. The map is the only way to see the whole world at once,
//! so what is on the world has to be on it: where the towns are, the roads between
//! them, and the places a bridge carries a road over water.

use bevy::prelude::*;

use crate::world::biome::surface_color;
use crate::world::terrain::Terrain;

/// Width of a painted map in pixels. The height follows the world's aspect ratio.
pub const WIDTH: u32 = 256;

pub fn dimensions(half: Vec2) -> UVec2 {
    let height = (WIDTH as f32 * half.y / half.x).round().max(1.0) as u32;
    UVec2::new(WIDTH, height)
}

pub fn paint(terrain: &Terrain, size: UVec2) -> Vec<u8> {
    let half = terrain.half();
    let mut pixels = Vec::with_capacity((size.x * size.y * 4) as usize);

    // One pixel is tens of meters, so the normal is taken over that same
    // distance — a 1 m epsilon would report slopes the map can't show.
    let epsilon = (half.x * 2.0 / size.x as f32) * 0.5;

    for py in 0..size.y {
        for px in 0..size.x {
            let x = (px as f32 / (size.x - 1) as f32 * 2.0 - 1.0) * half.x;
            let z = (py as f32 / (size.y - 1) as f32 * 2.0 - 1.0) * half.y;

            let height = terrain.height(x, z);
            let slope = 1.0 - terrain.normal(x, z, epsilon).y;
            // The same classification the terrain itself uses, so the overview
            // reads as the world rather than as a separate diagram.
            let color = surface_color(
                Vec2::new(x, z),
                height,
                slope,
                terrain.shore_character(x, z),
                terrain.worn(x, z),
                terrain.region(x, z).0,
                terrain.region(x, z).1,
                terrain.settled(x, z),
            );

            // `surface_color` returns linear; the texture is sRGB.
            let encode = |linear: f32| {
                let srgba = LinearRgba::rgb(linear, linear, linear);
                (Srgba::from(srgba).red.clamp(0.0, 1.0) * 255.0) as u8
            };
            pixels.extend_from_slice(&[
                encode(color[0]),
                encode(color[1]),
                encode(color[2]),
                255,
            ]);
        }
    }

    draw_the_works(terrain, size, &mut pixels);
    pixels
}

// ---------------------------------------------------------------- what people built
//
// Ground colour alone makes a beautiful map of a world nobody lives in. A settlement
// levels its ground, so the biggest city on the map shows as a slightly flatter patch
// of the same green as the country round it - which is to say, as nothing. The map is
// the only way to see the whole world at once, so what is ON the world has to be on
// it: where the towns are, the roads between them, and the two places a bridge
// carries a road over water.

/// A dirt road between settlements.
const MAP_ROAD: [u8; 3] = [122, 84, 44];
/// A bridge, which is masonry and reads paler than the road it carries.
const MAP_BRIDGE: [u8; 3] = [222, 218, 208];
/// A city, and the ring that keeps it legible against dark ground.
const MAP_CITY: [u8; 3] = [38, 38, 46];
/// A town.
const MAP_TOWN: [u8; 3] = [92, 72, 52];
/// The ranch, which is where the game starts and is not a settlement.
const MAP_RANCH: [u8; 3] = [214, 168, 62];
/// The pale ring drawn round every mark.
const MAP_RING: [u8; 3] = [246, 244, 238];

/// Stamps the roads, bridges and settlements over the painted ground.
fn draw_the_works(terrain: &Terrain, size: UVec2, pixels: &mut [u8]) {
    let half = terrain.half();
    let across = (size.x.max(1) - 1) as f32;
    let down = (size.y.max(1) - 1) as f32;
    // How many metres one pixel covers, which is what every radius below is in.
    let metres = half.x * 2.0 / size.x.max(1) as f32;

    let to_pixel = |at: Vec2| {
        Vec2::new(
            (at.x / half.x * 0.5 + 0.5) * across,
            (at.y / half.y * 0.5 + 0.5) * down,
        )
    };

    let mut blot = |at: Vec2, radius: f32, rgb: [u8; 3]| {
        let reach = radius.ceil() as i32;
        let middle = to_pixel(at);
        for dy in -reach..=reach {
            for dx in -reach..=reach {
                let (px, py) = (middle.x.round() as i32 + dx, middle.y.round() as i32 + dy);
                if px < 0 || py < 0 || px >= size.x as i32 || py >= size.y as i32 {
                    continue;
                }
                if (dx * dx + dy * dy) as f32 > radius * radius {
                    continue;
                }
                let at = (py as usize * size.x as usize + px as usize) * 4;
                pixels[at..at + 3].copy_from_slice(&rgb);
            }
        }
    };

    let mut draw = |from: Vec2, to: Vec2, radius: f32, rgb: [u8; 3]| {
        // Stepped in EIGHTH pixels.
        //
        // Half-pixels are not enough. A thin line blots one pixel per step, so two
        // consecutive steps that straddle a pixel boundary land either side of it
        // and leave the pixel between them unpainted - a road with holes in it, and
        // a hole exactly where a segment's middle happens to fall reads as a road
        // that was never drawn at all.
        let steps = ((from.distance(to) / metres) * 8.0).ceil().max(1.0) as usize;
        for step in 0..=steps {
            blot(from.lerp(to, step as f32 / steps as f32), radius, rgb);
        }
    };

    // Roads first, so a settlement's mark sits on top of the roads leaving it.
    for road in terrain.plan().ways() {
        draw(road.from, road.to, 0.6, MAP_ROAD);
    }
    // Then the bridges, which are drawn over the roads that lead to them and are
    // wider, because a crossing is the thing on this map worth finding.
    for bridge in terrain.plan().spans() {
        draw(bridge.from, bridge.to, 1.2, MAP_BRIDGE);
    }

    // And the places. A ring under every mark, so a dark city on dark ground and a
    // pale one on sand are both legible - the mark has to read against whatever the
    // country happens to be there.
    for site in terrain.sites() {
        let (radius, rgb) = if site.ranch {
            (2.2, MAP_RANCH)
        } else if site.city {
            (3.0, MAP_CITY)
        } else {
            (2.2, MAP_TOWN)
        };
        blot(site.at, radius + 1.2, MAP_RING);
        blot(site.at, radius, rgb);
    }
}

// ------------------------------------------------------------------ the needle
//
// Both maps draw one, so the part that is easy to get wrong is written once. That
// part is not the two nodes - it is the BEARING, which has three separate reversals
// in it and looks correct in either direction until you turn round and check.

/// How far across the heading needle's wrapper is, in pixels. The bar itself is
/// half of it, standing up from the middle.
pub const NEEDLE_SPAN: f32 = 26.0;

/// The compass bearing something is looking along, clockwise from north.
///
/// This world's `+x` is east and `+z` is south, so north is `-z` and a needle at
/// rest points up a north-up map. Rotating about Z in UI space turns `+X` toward
/// `+Y`, and UI's `+Y` is DOWN the screen - so a positive angle already reads
/// clockwise, the same sense as the bearing, and needs no minus sign.
pub fn bearing_of(forward: Vec3) -> f32 {
    forward.x.atan2(-forward.z)
}

/// Spawns a heading needle, marked with whatever lets the caller find it again.
///
/// Square and centred on nothing in particular: the caller positions it on the mark
/// it belongs to, and because it is square, rotating it turns it about that point
/// rather than swinging it around one.
pub fn needle(parent: &mut ChildSpawnerCommands, mark: impl Bundle, ink: Color) {
    parent
        .spawn((
            mark,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(NEEDLE_SPAN),
                height: Val::Px(NEEDLE_SPAN),
                margin: UiRect {
                    left: Val::Px(-NEEDLE_SPAN * 0.5),
                    top: Val::Px(-NEEDLE_SPAN * 0.5),
                    ..default()
                },
                ..default()
            },
        ))
        .with_children(|wrap| {
            // The bar itself, standing up from the middle. Pointing north at rest,
            // which is up on a map drawn north-up.
            wrap.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(50.0),
                    top: Val::Px(0.0),
                    width: Val::Px(2.0),
                    height: Val::Px(NEEDLE_SPAN * 0.5),
                    margin: UiRect {
                        left: Val::Px(-1.0),
                        ..default()
                    },
                    ..default()
                },
                BackgroundColor(ink),
            ));
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::terrain::Terrain;

    /// Roads, bridges and settlements are actually ON the map, and are the RIGHT
    /// marks.
    ///
    /// Ground colour alone paints a beautiful map of a world nobody lives in, and
    /// the failure is silent: a levelled town is a slightly flatter patch of the
    /// same green as the country round it, which reads as nothing at all.
    ///
    /// Asked of the PIXELS rather than of the drawing code: paint the world, then
    /// look at what is at each thing's own coordinates. A guard that reruns the
    /// painter cannot fail against it.
    ///
    /// The first cut of this accepted ANY settlement colour at any site and only
    /// ever looked at the middle pixel, so it would have passed with every city
    /// drawn as a town and with no ring under any of them. It now asks for the mark
    /// that place should have, and for the ring beside it.
    #[test]
    fn the_map_shows_what_people_built() {
        let terrain = Terrain::new();
        let size = dimensions(terrain.half());
        let painted = paint(&terrain, size);
        let half = terrain.half();

        let pixel = |world: Vec2| -> Vec2 {
            Vec2::new(
                (world.x / half.x * 0.5 + 0.5) * (size.x - 1) as f32,
                (world.y / half.y * 0.5 + 0.5) * (size.y - 1) as f32,
            )
        };
        let ink = |at: Vec2| -> [u8; 3] {
            let px = (at.x.round() as i64).clamp(0, size.x as i64 - 1) as usize;
            let py = (at.y.round() as i64).clamp(0, size.y as i64 - 1) as usize;
            let cell = (py * size.x as usize + px) * 4;
            [painted[cell], painted[cell + 1], painted[cell + 2]]
        };

        // Each place wears its own mark, and a ring outside it.
        for site in terrain.sites() {
            let (radius, want): (f32, [u8; 3]) = if site.ranch {
                (2.2, MAP_RANCH)
            } else if site.city {
                (3.0, MAP_CITY)
            } else {
                (2.2, MAP_TOWN)
            };
            let middle = pixel(site.at);
            let kind = if site.ranch {
                "ranch"
            } else if site.city {
                "city"
            } else {
                "town"
            };
            assert_eq!(
                ink(middle),
                want,
                "the {kind} at {:.0}, {:.0} is drawn as {:?}, not {want:?}",
                site.at.x,
                site.at.y,
                ink(middle),
            );

            // The ring sits between `radius` and `radius + 1.2` out. One pixel past
            // the fill is inside that band for both sizes of mark.
            let out = radius.round() + 1.0;
            let ringed = [
                Vec2::new(out, 0.0),
                Vec2::new(-out, 0.0),
                Vec2::new(0.0, out),
                Vec2::new(0.0, -out),
            ]
            .iter()
            .filter(|step| ink(middle + **step) == MAP_RING)
            .count();
            assert!(
                ringed >= 3,
                "the mark at {:.0}, {:.0} has a ring on only {ringed} of its four sides - without one it vanishes into dark ground",
                site.at.x,
                site.at.y,
            );
        }

        // Every bridge is drawn as masonry over the water it crosses.
        for bridge in terrain.plan().spans() {
            let middle = (bridge.from + bridge.to) * 0.5;
            assert_eq!(
                ink(pixel(middle)),
                MAP_BRIDGE,
                "the bridge at {:.0}, {:.0} is not on the map",
                middle.x,
                middle.y,
            );
        }

        // And EVERY road, bar the ones something is legitimately drawn over.
        //
        // Marks and bridges are painted after the roads, so a segment whose middle
        // lands under one is correctly hidden. Those are excluded by name rather
        // than by allowing some fraction of roads to be missing - "most of them
        // showed up" would have passed with a whole landmass unroaded.
        let roads = terrain.plan().ways();
        assert!(!roads.is_empty(), "a world with thirteen settlements has roads");
        let mut hidden = 0;
        for road in roads {
            let middle = (road.from + road.to) * 0.5;
            let under_a_mark = terrain.sites().iter().any(|site| {
                let radius = if site.city { 4.2 } else { 3.4 };
                pixel(site.at).distance(pixel(middle)) <= radius + 1.0
            });
            let under_a_bridge = terrain.plan().spans().iter().any(|bridge| {
                let run = bridge.to - bridge.from;
                let along = ((middle - bridge.from).dot(run)
                    / run.length_squared().max(1.0e-4))
                .clamp(0.0, 1.0);
                pixel(bridge.from + run * along).distance(pixel(middle)) <= 2.4
            });
            if under_a_mark || under_a_bridge {
                hidden += 1;
                continue;
            }
            // Within a pixel of where its middle lands, not exactly on it. The
            // painter walks a line by marking the nearest pixel to each step, which
            // promises the line is unbroken - not that any particular point along it
            // rounds the same way this test rounds. Asking for the exact pixel made
            // the guard fail on sub-pixel rounding rather than on a missing road.
            let on = pixel(middle);
            let drawn = [
                Vec2::ZERO,
                Vec2::new(1.0, 0.0),
                Vec2::new(-1.0, 0.0),
                Vec2::new(0.0, 1.0),
                Vec2::new(0.0, -1.0),
            ]
            .iter()
            .any(|step| ink(on + *step) == MAP_ROAD);
            assert!(
                drawn,
                "the road at {:.0}, {:.0} is not on the map and nothing is drawn over it - the map paints {:?} there",
                middle.x,
                middle.y,
                ink(on),
            );
        }
        println!(
            "{} road segments on the map, {hidden} of them under a mark or a bridge",
            roads.len() - hidden,
        );
    }
}
