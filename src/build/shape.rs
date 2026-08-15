//! Turning a baked building's boxes into one mesh.
//!
//! A house is fifty-odd boxes and four colours. Given a mesh each that would be
//! fifty draws for one building and hundreds for a street, so they are all
//! welded into a single mesh with the colour carried per vertex — which is what
//! the terrain already does, and what the shared material is already set up for.
//!
//! # Outward-facing by construction
//!
//! Every face is emitted through [`Shell::face`], which decides its winding by
//! comparing where the face sits against the middle of the box it belongs to.
//! Getting a winding wrong is invisible until something is lit from the wrong
//! side, and there are twenty-six of them across the four shapes; deciding it
//! once, from the geometry, removes the whole class of mistake.
//!
//! # This is drawn twice, and knowingly
//!
//! Opificium draws the same four shapes from its own code. `FORMATS.md` names
//! them and says so outright: a shape is only the same shape in both because it
//! is written out twice. That is the arrangement the terrain crate exists to
//! avoid, and the honest note is that buildings have not had it done for them
//! yet — the shapes are pure geometry over vectors and would move into a shared
//! crate cleanly, the day one of these disagrees.

use bevy::prelude::*;

use crate::build::plan::{Block, Form, Plan};

/// A mesh being built up: the terrain's attribute set, plus colour.
#[derive(Default)]
pub struct Shell {
    places: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    colours: Vec<[f32; 4]>,
    indices: Vec<u32>,
}

impl Shell {
    pub fn is_empty(&self) -> bool {
        self.places.is_empty()
    }

    /// One flat face, wound so it faces away from `middle`.
    ///
    /// Takes three or four corners; a fourth of `None` is a triangle, which is
    /// what closes the ends of a gable.
    fn face(&mut self, middle: Vec3, corners: [Vec3; 4], triangle: bool, colour: [f32; 4]) {
        let used = if triangle { 3 } else { 4 };
        let mut normal = (corners[1] - corners[0]).cross(corners[2] - corners[0]);
        if normal.length_squared() < 1.0e-12 {
            // A face with no area: an end cut clean away, or a hip whose deck
            // reaches the full width. It contributes nothing and its normal is
            // undefined, so it is dropped rather than emitted as noise.
            return;
        }
        normal = normal.normalize();

        // Which way is out. The middle of a convex box is inside every one of
        // its faces, so this is decidable without knowing which face it is.
        let outward = (corners[0] - middle).dot(normal) >= 0.0;
        if !outward {
            normal = -normal;
        }

        let base = self.places.len() as u32;
        for (i, corner) in corners.iter().take(used).enumerate() {
            self.places.push(corner.to_array());
            self.normals.push(normal.to_array());
            self.uvs.push(UV[i]);
            self.colours.push(colour);
        }

        // Reversed together with the normal, so the triangles a renderer culls
        // by winding agree with the ones it lights by normal.
        if triangle {
            self.indices.extend_from_slice(&wind([base, base + 1, base + 2], outward));
        } else {
            self.indices.extend_from_slice(&wind([base, base + 1, base + 2], outward));
            self.indices.extend_from_slice(&wind([base, base + 2, base + 3], outward));
        }
    }

    fn quad(&mut self, middle: Vec3, corners: [Vec3; 4], colour: [f32; 4]) {
        self.face(middle, corners, false, colour);
    }

    fn tri(&mut self, middle: Vec3, corners: [Vec3; 3], colour: [f32; 4]) {
        self.face(middle, [corners[0], corners[1], corners[2], corners[2]], true, colour);
    }

    pub fn into_mesh(self) -> Mesh {
        let mut mesh = Mesh::new(
            bevy::render::mesh::PrimitiveTopology::TriangleList,
            // Drawn, never read back.
            bevy::asset::RenderAssetUsages::RENDER_WORLD,
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.places);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, self.colours);
        mesh.insert_indices(bevy::render::mesh::Indices::U32(self.indices));
        mesh
    }
}

/// Corner texture coordinates. Nothing samples a texture on a building yet —
/// the colour is per vertex — but the shader's layout expects the attribute.
const UV: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];

fn wind(triangle: [u32; 3], outward: bool) -> [u32; 3] {
    if outward {
        triangle
    } else {
        [triangle[0], triangle[2], triangle[1]]
    }
}

/// The whole building, welded: opaque first, then whatever lets light through.
///
/// Two shells because glass has to be drawn after everything behind it and a
/// single mesh can only be one or the other. Most buildings never fill the
/// second, and an empty shell is never given a mesh.
pub fn raise(plan: &Plan) -> (Shell, Shell) {
    let mut solid = Shell::default();
    let mut glass = Shell::default();
    for block in &plan.boxes {
        let shell = if block.is_glass() { &mut glass } else { &mut solid };
        add(shell, block);
    }
    (solid, glass)
}

/// One box, in the building's own space.
fn add(shell: &mut Shell, block: &Block) {
    let colour = block.colour.to_linear().to_f32_array();
    let half = block.size * 0.5;
    // Where the box's own middle lands, which is what decides every winding.
    let middle = block.at;
    // Local corner into building space.
    let put = |x: f32, y: f32, z: f32| block.at + block.turn * Vec3::new(x, y, z);

    match block.form {
        Form::Box => {
            box_faces(shell, middle, colour, &put, -half.x, half.x, -half.x, half.x, half);
        }
        // Runs measured along the piece's own length: how far the saw travels
        // while crossing the full height. Positive takes the top back, negative
        // the bottom — see `Form::Cut`.
        Form::Cut { low, high } => {
            let length = block.size.x;
            let (bottom_low, top_low) = if low >= 0.0 {
                (-half.x, -half.x + low * length)
            } else {
                (-half.x - low * length, -half.x)
            };
            let (bottom_high, top_high) = if high >= 0.0 {
                (half.x, half.x - high * length)
            } else {
                (half.x + high * length, half.x)
            };
            box_faces(
                shell, middle, colour, &put, bottom_low, bottom_high, top_low, top_high, half,
            );
        }
        // A gable's prism: base across X, apex over the middle, the ridge line
        // running along Z.
        Form::Wedge => {
            let peak = |z: f32| put(0.0, half.y, z);
            let foot = |x: f32, z: f32| put(x, -half.y, z);
            shell.quad(
                middle,
                [
                    foot(-half.x, -half.z),
                    foot(half.x, -half.z),
                    foot(half.x, half.z),
                    foot(-half.x, half.z),
                ],
                colour,
            );
            for side in [-1.0_f32, 1.0] {
                shell.quad(
                    middle,
                    [
                        foot(half.x * side, -half.z),
                        foot(half.x * side, half.z),
                        peak(half.z),
                        peak(-half.z),
                    ],
                    colour,
                );
            }
            for end in [-half.z, half.z] {
                shell.tri(
                    middle,
                    [foot(-half.x, end), foot(half.x, end), peak(end)],
                    colour,
                );
            }
        }
        // The same prism turned a quarter: the ridge line runs along X.
        Form::Ridge => {
            let peak = |x: f32| put(x, half.y, 0.0);
            let foot = |x: f32, z: f32| put(x, -half.y, z);
            shell.quad(
                middle,
                [
                    foot(-half.x, -half.z),
                    foot(half.x, -half.z),
                    foot(half.x, half.z),
                    foot(-half.x, half.z),
                ],
                colour,
            );
            for side in [-1.0_f32, 1.0] {
                shell.quad(
                    middle,
                    [
                        foot(-half.x, half.z * side),
                        foot(half.x, half.z * side),
                        peak(half.x),
                        peak(-half.x),
                    ],
                    colour,
                );
            }
            for end in [-half.x, half.x] {
                shell.tri(
                    middle,
                    [foot(end, -half.z), foot(end, half.z), peak(end)],
                    colour,
                );
            }
        }
        // A hip roof with a deck: the top keeps a fraction of each side.
        Form::Hip { across, along } => {
            let (deck_x, deck_z) = (half.x * across, half.z * along);
            let foot = |x: f32, z: f32| put(x, -half.y, z);
            let deck = |x: f32, z: f32| put(x, half.y, z);

            shell.quad(
                middle,
                [
                    foot(-half.x, -half.z),
                    foot(half.x, -half.z),
                    foot(half.x, half.z),
                    foot(-half.x, half.z),
                ],
                colour,
            );
            shell.quad(
                middle,
                [
                    deck(-deck_x, -deck_z),
                    deck(deck_x, -deck_z),
                    deck(deck_x, deck_z),
                    deck(-deck_x, deck_z),
                ],
                colour,
            );
            for side in [-1.0_f32, 1.0] {
                // The two faces sloping in across X, then the two along Z.
                shell.quad(
                    middle,
                    [
                        foot(half.x * side, -half.z),
                        foot(half.x * side, half.z),
                        deck(deck_x * side, deck_z),
                        deck(deck_x * side, -deck_z),
                    ],
                    colour,
                );
                shell.quad(
                    middle,
                    [
                        foot(-half.x, half.z * side),
                        foot(half.x, half.z * side),
                        deck(deck_x, deck_z * side),
                        deck(-deck_x, deck_z * side),
                    ],
                    colour,
                );
            }
        }
    }
}

/// Six faces of a box whose top and bottom edges may sit at different places
/// along X. With all four the same it is a plain cuboid; with them apart it is
/// a cut piece, and nothing else about the shape changes.
#[allow(clippy::too_many_arguments)]
fn box_faces(
    shell: &mut Shell,
    middle: Vec3,
    colour: [f32; 4],
    put: &impl Fn(f32, f32, f32) -> Vec3,
    bottom_low: f32,
    bottom_high: f32,
    top_low: f32,
    top_high: f32,
    half: Vec3,
) {
    let foot = |x: f32, z: f32| put(x, -half.y, z);
    let head = |x: f32, z: f32| put(x, half.y, z);

    // Bottom and top.
    shell.quad(
        middle,
        [
            foot(bottom_low, -half.z),
            foot(bottom_high, -half.z),
            foot(bottom_high, half.z),
            foot(bottom_low, half.z),
        ],
        colour,
    );
    shell.quad(
        middle,
        [
            head(top_low, -half.z),
            head(top_high, -half.z),
            head(top_high, half.z),
            head(top_low, half.z),
        ],
        colour,
    );

    // The two long sides, each a quadrilateral once the ends are cut.
    for side in [-1.0_f32, 1.0] {
        let z = half.z * side;
        shell.quad(
            middle,
            [
                foot(bottom_low, z),
                foot(bottom_high, z),
                head(top_high, z),
                head(top_low, z),
            ],
            colour,
        );
    }

    // The two ends. Either can be a sloped face, or nothing at all when a run
    // of 1 has taken it away entirely.
    shell.quad(
        middle,
        [
            foot(bottom_low, -half.z),
            foot(bottom_low, half.z),
            head(top_low, half.z),
            head(top_low, -half.z),
        ],
        colour,
    );
    shell.quad(
        middle,
        [
            foot(bottom_high, -half.z),
            foot(bottom_high, half.z),
            head(top_high, half.z),
            head(top_high, -half.z),
        ],
        colour,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build::plan::Plan;

    fn one_block(form: &str, size: [f32; 3]) -> Plan {
        let json = format!(
            r#"{{ "format": 2, "name": "test", "half_w": 1, "half_d": 1, "high": 1,
                  "boxes": [ {{ "at": [0,{},0], "size": {size:?}, "turn": [0,0,0,1],
                                "form": "{form}", "rgb": [200,180,160], "alpha": 1.0 }} ] }}"#,
            size[1] * 0.5
        );
        Plan::read(&json).expect("fixture should read")
    }

    /// Every position in the shell, which is all these tests need to judge a
    /// shape by.
    fn corners(plan: &Plan) -> Vec<Vec3> {
        let (solid, _) = raise(plan);
        solid.places.iter().map(|p| Vec3::from_array(*p)).collect()
    }

    fn span(points: &[Vec3]) -> (Vec3, Vec3) {
        points.iter().fold(
            (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)),
            |(low, high), p| (low.min(*p), high.max(*p)),
        )
    }

    #[test]
    fn a_box_stands_on_the_ground_and_fills_its_size() {
        let plan = one_block("box", [4.0, 2.5, 0.25]);
        let (low, high) = span(&corners(&plan));
        // `at` puts the box's MIDDLE, so a wall 2.5 m high centred at 1.25 has
        // its foot on y=0. Getting this wrong buries or floats every building.
        assert!((low.y - 0.0).abs() < 1.0e-5, "foot at {}", low.y);
        assert!((high.y - 2.5).abs() < 1.0e-5, "head at {}", high.y);
        assert!((high.x - low.x - 4.0).abs() < 1.0e-5);
        assert!((high.z - low.z - 0.25).abs() < 1.0e-5);
    }

    #[test]
    fn every_face_looks_outward() {
        // Invisible until something is lit from the wrong side, and there are
        // twenty-six windings across the four shapes. Each is checked against
        // the one thing that decides it: a face of a convex box always points
        // away from the box's middle.
        for form in ["box", "wedge", "ridge", "cut:0.2500x-0.2500", "hip:0.5000x0.6250"] {
            let plan = one_block(form, [3.0, 2.0, 4.0]);
            let (shell, _) = raise(&plan);
            let middle = plan.boxes[0].at;

            for triangle in shell.indices.chunks(3) {
                let [a, b, c] = [triangle[0], triangle[1], triangle[2]]
                    .map(|i| Vec3::from_array(shell.places[i as usize]));
                let face = (b - a).cross(c - a);
                if face.length_squared() < 1.0e-12 {
                    continue;
                }
                let out = ((a + b + c) / 3.0 - middle).dot(face);
                assert!(out > 0.0, "{form}: a face is wound inward");

                // And the stored normal must agree with the winding, or a
                // renderer culls one set of triangles and lights the other.
                let normal = Vec3::from_array(shell.normals[triangle[0] as usize]);
                assert!(
                    normal.dot(face.normalize()) > 0.9,
                    "{form}: normal and winding disagree"
                );
            }
        }
    }

    #[test]
    fn a_brace_cut_top_and_bottom_comes_out_a_parallelogram() {
        // The contract's own example, and the whole reason a run is signed: cut
        // the top at one end and the bottom at the other and the two ends come
        // out parallel, which is what a diagonal brace is.
        let plan = one_block("cut:0.2500x-0.2500", [4.0, 1.0, 0.5]);
        let points = corners(&plan);
        let (low, high) = span(&points);

        let at_height = |y: f32| {
            let row: Vec<f32> = points
                .iter()
                .filter(|p| (p.y - y).abs() < 1.0e-4)
                .map(|p| p.x)
                .collect();
            (
                row.iter().copied().fold(f32::MAX, f32::min),
                row.iter().copied().fold(f32::MIN, f32::max),
            )
        };
        let (bottom_from, bottom_to) = at_height(low.y);
        let (top_from, top_to) = at_height(high.y);

        // Same length, shifted along — a parallelogram and not a trapezium.
        assert!(
            ((top_to - top_from) - (bottom_to - bottom_from)).abs() < 1.0e-4,
            "ends are not parallel: bottom {:.3}, top {:.3}",
            bottom_to - bottom_from,
            top_to - top_from
        );
        assert!(
            (top_from - bottom_from - 1.0).abs() < 1.0e-4,
            "the top should start a quarter of 4 m along, not {:.3}",
            top_from - bottom_from
        );
    }

    #[test]
    fn a_hip_keeps_the_fraction_of_its_deck_it_was_given() {
        let plan = one_block("hip:0.5000x0.6250", [4.0, 2.0, 8.0]);
        let points = corners(&plan);
        let (low, high) = span(&points);
        let deck: Vec<Vec3> = points
            .iter()
            .filter(|p| (p.y - high.y).abs() < 1.0e-4)
            .copied()
            .collect();
        let (deck_low, deck_high) = span(&deck);

        assert!((deck_high.x - deck_low.x - 2.0).abs() < 1.0e-4, "deck width");
        assert!((deck_high.z - deck_low.z - 5.0).abs() < 1.0e-4, "deck depth");
        // And it is a roof, not a box: the foot still spans the full size.
        assert!((high.x - low.x - 4.0).abs() < 1.0e-4);
    }

    #[test]
    fn a_gable_and_a_ridge_are_the_same_prism_turned() {
        let gable = one_block("wedge", [4.0, 2.0, 8.0]);
        let ridge = one_block("ridge", [4.0, 2.0, 8.0]);

        let peak_of = |plan: &Plan| {
            let points = corners(plan);
            let (_, high) = span(&points);
            let top: Vec<Vec3> = points
                .iter()
                .filter(|p| (p.y - high.y).abs() < 1.0e-4)
                .copied()
                .collect();
            span(&top)
        };

        // A gable's ridge line runs along Z and sits over the middle in X.
        let (gable_low, gable_high) = peak_of(&gable);
        assert!(gable_low.x.abs() < 1.0e-4 && gable_high.x.abs() < 1.0e-4);
        assert!((gable_high.z - gable_low.z - 8.0).abs() < 1.0e-4);

        // A ridge cap's runs along X and sits over the middle in Z.
        let (ridge_low, ridge_high) = peak_of(&ridge);
        assert!(ridge_low.z.abs() < 1.0e-4 && ridge_high.z.abs() < 1.0e-4);
        assert!((ridge_high.x - ridge_low.x - 4.0).abs() < 1.0e-4);
    }

    #[test]
    fn glass_is_welded_separately_from_the_walls() {
        let json = r#"{ "format": 2, "name": "test", "half_w": 1, "half_d": 1, "high": 1,
          "boxes": [ { "at": [0,1,0], "size": [2,2,0.2], "form": "box", "rgb": [200,180,160] },
                     { "at": [0,1,0], "size": [1,1,0.1], "form": "box", "rgb": [180,220,255],
                       "alpha": 0.35 } ] }"#;
        let plan = Plan::read(json).unwrap();
        let (solid, glass) = raise(&plan);
        assert!(!solid.is_empty(), "the wall should be in the solid shell");
        assert!(!glass.is_empty(), "the window should be in its own");
    }
}
