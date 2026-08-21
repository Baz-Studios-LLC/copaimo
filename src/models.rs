//! Checks a model file against the conventions the game needs of it.
//!
//! # A bad model does not error, it just looks wrong
//!
//! Every fault this catches loads perfectly well. A figure exported in
//! centimetres arrives a hundred times too big and fills the sky; one whose
//! origin is at its waist arrives half-buried in the ground; one exported Z-up
//! arrives on its back. Nothing in Bevy objects to any of it, so the report is
//! always a person saying the game looks strange, which is the most expensive
//! kind of bug report to act on.
//!
//! So the rules are written down and asked of every model in the game. They are
//! asked TWICE, at two different moments: `dev/model_export.py` asks them the
//! instant a model is exported, when the fix is a keypress in Blender, and this
//! asks them of whatever is actually in `assets/models/`, which also covers
//! anything dropped in by hand. The two sets of numbers agreeing is itself
//! tested below — this project has been bitten repeatedly by one question with
//! two answers, and a gate that has drifted from its twin is exactly that shape.
//!
//! Compiled only under `cfg(test)`: it is a gate on the content, not something
//! the running game needs.

/// What a model may measure along its longest axis, in metres.
///
/// Test-only, with everything else in the gate. The running game does not check a
/// model against the conventions — it reads geometry out of one. The checking is
/// `cargo test`'s job and `dev/model_export.py`'s, and compiling it into a
/// player's build earned seven dead-code warnings the moment the reader moved to
/// runtime.
///
/// **These must match `dev/model_export.py`** — see
/// `the_two_gates_agree_about_what_they_allow`, which reads that file and checks.
#[cfg(test)]
const SMALLEST: f32 = 0.02;
#[cfg(test)]
const LARGEST: f32 = 60.0;

/// How far a model's base may sit off the floor before it counts as floating.
#[cfg(test)]
const FOOTING_SLACK: f32 = 0.02;

/// What a glTF file says about the shape inside it.
#[cfg(test)]
#[derive(Debug)]
pub struct Model {
    /// Corner to corner, in the game's own axes: X across, Y up, Z forward.
    pub low: [f32; 3],
    pub high: [f32; 3],
    pub triangles: usize,
    /// Each mesh in the file, by name, with how many primitives it carries.
    ///
    /// Names matter: a tree is read out of its file by looking for the meshes
    /// called `wood` and `leaves`, because bark and foliage wear different
    /// materials. A renamed object is a silent failure — the game finds nothing
    /// and quietly keeps the shape it grew for itself.
    pub meshes: Vec<(String, usize)>,
}

#[cfg(test)]
impl Model {
    fn size(&self) -> [f32; 3] {
        [
            self.high[0] - self.low[0],
            self.high[1] - self.low[1],
            self.high[2] - self.low[2],
        ]
    }
}

/// Reads a GLB's header and reports the shape it describes.
///
/// The container by hand rather than through a glTF crate, because the whole
/// question here is about the numbers in the header — every mesh's POSITION
/// accessor carries its own `min` and `max`, which is the model's extent without
/// decoding a single vertex.
#[cfg(test)]
pub fn inspect(bytes: &[u8]) -> Result<Model, String> {
    if bytes.len() < 12 || &bytes[0..4] != b"glTF" {
        return Err("not a GLB file".into());
    }
    let word = |at: usize| -> u32 {
        u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
    };
    let version = word(4);
    if version != 2 {
        return Err(format!("glTF version {version}, and this reads version 2"));
    }
    let total = (word(8) as usize).min(bytes.len());

    // The chunks: a JSON one, and usually a binary one this does not need.
    let mut at = 12;
    let mut json = None;
    while at + 8 <= total {
        let long = word(at) as usize;
        let kind = &bytes[at + 4..at + 8];
        let from = at + 8;
        let upto = from.checked_add(long).ok_or("a chunk longer than the file")?;
        if upto > total {
            return Err("truncated: a chunk runs past the end of the file".into());
        }
        if kind == b"JSON" {
            json = Some(&bytes[from..upto]);
        }
        at = upto + (4 - long % 4) % 4;
    }
    let json = json.ok_or("no JSON chunk: a GLB must carry one")?;
    let tree: serde_json::Value =
        serde_json::from_slice(json).map_err(|why| format!("the JSON will not parse: {why}"))?;

    let accessors = tree["accessors"]
        .as_array()
        .ok_or("no accessors, so nothing says how big this is")?;
    let meshes = tree["meshes"].as_array().ok_or("no meshes in the file")?;

    let mut low = [f32::MAX; 3];
    let mut high = [f32::MIN; 3];
    let mut triangles = 0;
    let mut seen = false;
    let mut named = Vec::new();
    for mesh in meshes {
        let Some(parts) = mesh["primitives"].as_array() else {
            continue;
        };
        named.push((
            mesh["name"].as_str().unwrap_or_default().to_string(),
            parts.len(),
        ));
        for part in parts {
            let Some(which) = part["attributes"]["POSITION"].as_u64() else {
                continue;
            };
            let Some(accessor) = accessors.get(which as usize) else {
                return Err("a primitive points at an accessor that is not there".into());
            };
            let corner = |field: &str| -> Option<[f32; 3]> {
                let list = accessor[field].as_array()?;
                let mut out = [0.0; 3];
                for (axis, slot) in out.iter_mut().enumerate() {
                    *slot = list.get(axis)?.as_f64()? as f32;
                }
                Some(out)
            };
            let (Some(least), Some(most)) = (corner("min"), corner("max")) else {
                return Err("a mesh gives no min/max for its positions".into());
            };
            for axis in 0..3 {
                if !least[axis].is_finite() || !most[axis].is_finite() {
                    return Err("a mesh's extent is not a finite number".into());
                }
                low[axis] = low[axis].min(least[axis]);
                high[axis] = high[axis].max(most[axis]);
            }
            seen = true;
            // Indexed or not: the count is of indices when there are any and of
            // positions when there are not.
            let count = part["indices"]
                .as_u64()
                .and_then(|which| accessors.get(which as usize))
                .or(Some(accessor))
                .and_then(|it| it["count"].as_u64())
                .unwrap_or(0);
            triangles += (count / 3) as usize;
        }
    }
    if !seen {
        return Err("nothing in the file has any geometry".into());
    }
    Ok(Model {
        low,
        high,
        triangles,
        meshes: named,
    })
}

/// Everything wrong with a model, in words a person can act on.
///
/// Empty means it is fit to go in the game.
#[cfg(test)]
pub fn faults(model: &Model) -> Vec<String> {
    let mut faults = Vec::new();
    let size = model.size();
    let biggest = size.iter().copied().fold(f32::MIN, f32::max);

    if biggest > LARGEST {
        faults.push(format!(
            "it measures {biggest:.1} m — over {LARGEST:.0} m, so this is almost \
             certainly a scale mistake rather than a very large thing"
        ));
    }
    if biggest < SMALLEST {
        faults.push(format!(
            "it measures {biggest:.3} m — under {SMALLEST} m, which is a scale \
             mistake rather than a very small thing"
        ));
    }

    // Standing on its own origin, so placing it at a terrain height puts it ON
    // the ground. This also catches the model that was exported Z-up: its height
    // then lies along Z and its Y spans the thing's depth, half of it under the
    // floor.
    if model.low[1] < -FOOTING_SLACK {
        let upright = size[1] >= size[0] && size[1] >= size[2];
        let why = if upright {
            "put its base on Z=0 in Blender"
        } else {
            "it is also wider and deeper than it is tall, which is what a model \
             exported Z-UP looks like — export with Y-up"
        };
        faults.push(format!(
            "its base sits {:.2} m below the floor, so it imports half-buried: {why}",
            model.low[1]
        ));
    }
    if model.low[1] > FOOTING_SLACK {
        faults.push(format!(
            "its base floats {:.2} m over the floor, so it imports hovering — put \
             its base on Z=0 in Blender",
            model.low[1]
        ));
    }
    faults
}

// ---------------------------------------------------------- reading the geometry

/// Reads one named mesh out of a GLB as geometry the world can stamp.
///
/// # Why this exists when Bevy has a glTF loader
///
/// Bevy's loader is asynchronous and hands back a `Mesh` for the GPU. Neither
/// suits the things this is for. Rocks, bushes and ground cover are not drawn as
/// objects: a chunk's worth of them is WELDED into one mesh on a background
/// thread, because fifty separate little objects per chunk would be fifty draw
/// calls paid for again in every shadow cascade. That welding happens off the main
/// thread with no access to Bevy's assets, and it happens the moment a chunk
/// streams in — so the shapes have to be in hand, as plain data, before any of it
/// starts.
///
/// The same reasoning the heightmap already follows: the world's own data is read
/// directly and synchronously, because everything downstream needs an answer now
/// rather than in a frame or two.
///
/// Trees are the exception and go the other way — one mesh per variety, planted as
/// objects, so they use Bevy's loader. See `world::authored`.
pub fn read_geometry(bytes: &[u8], want: &str) -> Result<terrain_core::Geometry, String> {
    let (tree, bin) = split(bytes)?;

    let meshes = tree["meshes"].as_array().ok_or("no meshes in the file")?;
    let mesh = meshes
        .iter()
        .find(|mesh| mesh["name"].as_str() == Some(want))
        .ok_or_else(|| {
            let had: Vec<&str> = meshes
                .iter()
                .filter_map(|mesh| mesh["name"].as_str())
                .collect();
            format!("no mesh named `{want}`; the file has {had:?}")
        })?;

    let mut out = terrain_core::Geometry::default();
    for part in mesh["primitives"]
        .as_array()
        .ok_or("a mesh with no primitives")?
    {
        // Anything other than triangles would be drawn wrong rather than refused,
        // and a wrongly drawn rock is harder to notice than a missing one.
        let mode = part["mode"].as_u64().unwrap_or(4);
        if mode != 4 {
            return Err(format!("`{want}` is drawn in mode {mode}, and only triangles (4) are read"));
        }
        let attributes = &part["attributes"];
        let places: Vec<[f32; 3]> = triples(&tree, bin, attributes["POSITION"].as_u64())?
            .ok_or("a primitive with no POSITION")?;
        let count = places.len();

        let normals = triples(&tree, bin, attributes["NORMAL"].as_u64())?
            .unwrap_or_else(|| vec![[0.0, 1.0, 0.0]; count]);
        let uvs = pairs(&tree, bin, attributes["TEXCOORD_0"].as_u64())?
            .unwrap_or_else(|| vec![[0.0, 0.0]; count]);
        // Colour is optional and both widths are legal, so this takes whichever
        // the exporter chose. Absent, the geometry simply carries none — which is
        // what a tree wants and what a rock must not have.
        let colours = quads(&tree, bin, attributes["COLOR_0"].as_u64())?;

        if normals.len() != count || uvs.len() != count {
            return Err(format!(
                "`{want}` has {count} positions but {} normals and {} uvs",
                normals.len(),
                uvs.len()
            ));
        }

        // Welded onto whatever came before, so a mesh in several primitives still
        // reads as one shape.
        let base = out.places.len() as u32;
        match part["indices"].as_u64() {
            Some(which) => {
                for index in whole_numbers(&tree, bin, which)? {
                    if index as usize >= count {
                        return Err(format!("`{want}` indexes vertex {index} of {count}"));
                    }
                    out.indices.push(base + index);
                }
            }
            // Unindexed: the vertices are already in drawing order.
            None => out.indices.extend((0..count as u32).map(|step| base + step)),
        }
        out.places.extend(places);
        out.normals.extend(normals);
        out.uvs.extend(uvs);
        if let Some(colours) = colours {
            if colours.len() != count {
                return Err(format!("`{want}` has {count} positions and {} colours", colours.len()));
            }
            out.colours.extend(colours);
        }
    }
    if out.places.is_empty() {
        return Err(format!("`{want}` has no vertices"));
    }
    // All of it or none: a mesh half-coloured would be drawn with the missing half
    // black, which reads as a hole.
    if !out.colours.is_empty() && out.colours.len() != out.places.len() {
        return Err(format!(
            "`{want}` carries colour for {} of its {} vertices",
            out.colours.len(),
            out.places.len()
        ));
    }
    Ok(out)
}

/// The JSON and the binary blob of a GLB, checked apart.
fn split(bytes: &[u8]) -> Result<(serde_json::Value, &[u8]), String> {
    if bytes.len() < 12 || &bytes[0..4] != b"glTF" {
        return Err("not a GLB file".into());
    }
    let word = |at: usize| -> u32 {
        u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
    };
    if word(4) != 2 {
        return Err(format!("glTF version {}, and this reads version 2", word(4)));
    }
    let total = (word(8) as usize).min(bytes.len());
    let mut at = 12;
    let (mut json, mut bin) = (None, None);
    while at + 8 <= total {
        let long = word(at) as usize;
        let kind = &bytes[at + 4..at + 8];
        let from = at + 8;
        let upto = from.checked_add(long).ok_or("a chunk longer than the file")?;
        if upto > total {
            return Err("truncated: a chunk runs past the end of the file".into());
        }
        match kind {
            b"JSON" => json = Some(&bytes[from..upto]),
            b"BIN\0" => bin = Some(&bytes[from..upto]),
            _ => {}
        }
        at = upto + (4 - long % 4) % 4;
    }
    let json = json.ok_or("no JSON chunk: a GLB must carry one")?;
    let tree: serde_json::Value =
        serde_json::from_slice(json).map_err(|why| format!("the JSON will not parse: {why}"))?;
    Ok((tree, bin.unwrap_or(&[])))
}

/// Where one accessor's bytes are, and how they are laid out.
struct Run {
    from: usize,
    stride: usize,
    count: usize,
    kind: u64,
    normalised: bool,
}

fn locate(tree: &serde_json::Value, which: u64, wide: usize) -> Result<Run, String> {
    let accessor = tree["accessors"]
        .get(which as usize)
        .ok_or_else(|| format!("no accessor {which}"))?;
    let kind = accessor["componentType"]
        .as_u64()
        .ok_or("an accessor with no componentType")?;
    let size = match kind {
        5120 | 5121 => 1,
        5122 | 5123 => 2,
        5125 | 5126 => 4,
        other => return Err(format!("component type {other} is not one this reads")),
    };
    let count = accessor["count"].as_u64().unwrap_or(0) as usize;
    let packed = size * wide;

    let Some(view) = accessor["bufferView"].as_u64() else {
        // A legal accessor with no view is all zeroes, and nothing here wants one.
        return Err("an accessor with no bufferView".into());
    };
    let view = tree["bufferViews"]
        .get(view as usize)
        .ok_or_else(|| format!("no bufferView {view}"))?;
    if view["buffer"].as_u64().unwrap_or(0) != 0 {
        return Err("a bufferView pointing outside the GLB's own binary chunk".into());
    }
    let stride = view["byteStride"].as_u64().unwrap_or(packed as u64) as usize;
    if stride < packed {
        return Err(format!("a byteStride of {stride} cannot hold {packed} bytes"));
    }
    let from = view["byteOffset"].as_u64().unwrap_or(0) as usize
        + accessor["byteOffset"].as_u64().unwrap_or(0) as usize;
    Ok(Run {
        from,
        stride,
        count,
        kind,
        normalised: accessor["normalized"].as_bool().unwrap_or(false),
    })
}

/// One component, whatever width it was stored at, as the float it means.
fn component(bin: &[u8], at: usize, kind: u64, normalised: bool) -> Result<f32, String> {
    let need = |size: usize| -> Result<&[u8], String> {
        bin.get(at..at + size)
            .ok_or_else(|| "the binary chunk is shorter than its accessors claim".to_string())
    };
    Ok(match kind {
        5126 => f32::from_le_bytes(need(4)?.try_into().unwrap()),
        5125 => {
            let raw = u32::from_le_bytes(need(4)?.try_into().unwrap());
            if normalised { raw as f32 / u32::MAX as f32 } else { raw as f32 }
        }
        5123 => {
            let raw = u16::from_le_bytes(need(2)?.try_into().unwrap());
            if normalised { raw as f32 / u16::MAX as f32 } else { raw as f32 }
        }
        5121 => {
            let raw = need(1)?[0];
            if normalised { raw as f32 / u8::MAX as f32 } else { raw as f32 }
        }
        5122 => {
            let raw = i16::from_le_bytes(need(2)?.try_into().unwrap());
            if normalised { (raw as f32 / 32767.0).max(-1.0) } else { raw as f32 }
        }
        5120 => {
            let raw = need(1)?[0] as i8;
            if normalised { (raw as f32 / 127.0).max(-1.0) } else { raw as f32 }
        }
        other => return Err(format!("component type {other} is not one this reads")),
    })
}

fn floats<const N: usize>(
    tree: &serde_json::Value,
    bin: &[u8],
    which: Option<u64>,
    wide: usize,
    fill: f32,
) -> Result<Option<Vec<[f32; N]>>, String> {
    let Some(which) = which else {
        return Ok(None);
    };
    let run = locate(tree, which, wide)?;
    let size = match run.kind {
        5120 | 5121 => 1,
        5122 | 5123 => 2,
        _ => 4,
    };
    let mut out = Vec::with_capacity(run.count);
    for step in 0..run.count {
        let mut one = [fill; N];
        for lane in 0..wide.min(N) {
            one[lane] = component(bin, run.from + step * run.stride + lane * size, run.kind, run.normalised)?;
        }
        out.push(one);
    }
    Ok(Some(out))
}

fn triples(tree: &serde_json::Value, bin: &[u8], which: Option<u64>) -> Result<Option<Vec<[f32; 3]>>, String> {
    floats::<3>(tree, bin, which, 3, 0.0)
}

fn pairs(tree: &serde_json::Value, bin: &[u8], which: Option<u64>) -> Result<Option<Vec<[f32; 2]>>, String> {
    floats::<2>(tree, bin, which, 2, 0.0)
}

/// Colour, which glTF allows as RGB or RGBA. Missing alpha means opaque.
fn quads(tree: &serde_json::Value, bin: &[u8], which: Option<u64>) -> Result<Option<Vec<[f32; 4]>>, String> {
    let Some(index) = which else {
        return Ok(None);
    };
    let wide = match tree["accessors"][index as usize]["type"].as_str() {
        Some("VEC4") => 4,
        Some("VEC3") => 3,
        other => return Err(format!("COLOR_0 is {other:?}, and only VEC3 or VEC4 are read")),
    };
    floats::<4>(tree, bin, Some(index), wide, 1.0)
}

fn whole_numbers(tree: &serde_json::Value, bin: &[u8], which: u64) -> Result<Vec<u32>, String> {
    let run = locate(tree, which, 1)?;
    let size = match run.kind {
        5121 => 1,
        5123 => 2,
        5125 => 4,
        other => return Err(format!("indices of component type {other} are not read")),
    };
    let mut out = Vec::with_capacity(run.count);
    for step in 0..run.count {
        let at = run.from + step * run.stride;
        out.push(match size {
            1 => bin.get(at).copied().ok_or("indices run past the binary chunk")? as u32,
            2 => u16::from_le_bytes(
                bin.get(at..at + 2)
                    .ok_or("indices run past the binary chunk")?
                    .try_into()
                    .unwrap(),
            ) as u32,
            _ => u32::from_le_bytes(
                bin.get(at..at + 4)
                    .ok_or("indices run past the binary chunk")?
                    .try_into()
                    .unwrap(),
            ),
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A real Blender export decodes to geometry that agrees with its own header.
    ///
    /// # Checked against the file, not against a number I typed
    ///
    /// A decoder is exactly the kind of code that can be confidently wrong: read
    /// the stride wrong, or the component type, and out come plausible vertices in
    /// the wrong places. So the decoded result is checked against something the
    /// file states INDEPENDENTLY of the bytes this reads — every POSITION accessor
    /// carries its own `min` and `max`, written by the exporter. Decode the
    /// positions, take their extent, and the two have to agree.
    ///
    /// It runs on whatever real models are in the game, so it gains coverage as
    /// models are added rather than needing new fixtures.
    #[test]
    fn a_real_export_decodes_to_what_its_own_header_claims() {
        let folder = std::path::Path::new("assets/models");
        let Ok(entries) = std::fs::read_dir(folder) else {
            println!("no assets/models folder yet");
            return;
        };
        let mut checked = 0;
        for entry in entries.flatten() {
            let road = entry.path();
            if road.extension().and_then(|end| end.to_str()) != Some("glb") {
                continue;
            }
            let name = road.file_name().unwrap_or_default().to_string_lossy().to_string();
            let bytes = std::fs::read(&road).expect("a model that is there should read");
            let model = inspect(&bytes).unwrap_or_else(|why| panic!("{name}: {why}"));

            for (mesh, _) in &model.meshes {
                let shape = read_geometry(&bytes, mesh)
                    .unwrap_or_else(|why| panic!("{name} `{mesh}`: {why}"));
                assert!(!shape.places.is_empty(), "{name} `{mesh}` decoded to nothing");
                assert_eq!(
                    shape.normals.len(),
                    shape.places.len(),
                    "{name} `{mesh}`: a normal per vertex or none at all"
                );
                assert!(
                    shape.indices.len() % 3 == 0,
                    "{name} `{mesh}` has {} indices, which is not whole triangles",
                    shape.indices.len()
                );
                assert!(
                    shape.indices.iter().all(|at| (*at as usize) < shape.places.len()),
                    "{name} `{mesh}` indexes a vertex it does not have"
                );

                // Colour, where there is any. Blender writes COLOR_0 as
                // NORMALISED integers, so a reader that took the raw value would
                // hand back 0..65535 and every rock in the world would draw at
                // full white. The range is the assertion that catches it.
                if !shape.colours.is_empty() {
                    assert_eq!(
                        shape.colours.len(),
                        shape.places.len(),
                        "{name} `{mesh}`: a colour per vertex or none at all"
                    );
                    for colour in &shape.colours {
                        for (lane, part) in colour.iter().enumerate() {
                            assert!(
                                (0.0..=1.0).contains(part),
                                "{name} `{mesh}` has {part} in lane {lane} — colour                                  is being read raw rather than normalised"
                            );
                        }
                    }
                    // And it is not one flat colour: these are painted with the
                    // light baked in, darker at the foot, which is most of what
                    // makes an untextured rock read as a rock.
                    let lightest = shape.colours.iter().map(|c| c[0]).fold(f32::MIN, f32::max);
                    let darkest = shape.colours.iter().map(|c| c[0]).fold(f32::MAX, f32::min);
                    assert!(
                        lightest - darkest > 0.02,
                        "{name} `{mesh}` is one flat colour — the baked shading is gone"
                    );
                }
                checked += 1;
            }

            // The whole file's extent, decoded, against the extent its accessors
            // declare. This is the assertion that catches a misread stride.
            let mut low = [f32::MAX; 3];
            let mut high = [f32::MIN; 3];
            for (mesh, _) in &model.meshes {
                for place in read_geometry(&bytes, mesh).expect("decodes").places {
                    for axis in 0..3 {
                        low[axis] = low[axis].min(place[axis]);
                        high[axis] = high[axis].max(place[axis]);
                    }
                }
            }
            for axis in 0..3 {
                assert!(
                    (low[axis] - model.low[axis]).abs() < 0.001
                        && (high[axis] - model.high[axis]).abs() < 0.001,
                    "{name} axis {axis}: decoded {:.3}..{:.3} but the header says \
                     {:.3}..{:.3} — the bytes are being read wrong",
                    low[axis],
                    high[axis],
                    model.low[axis],
                    model.high[axis]
                );
            }
        }
        assert!(checked > 0, "no meshes were decoded, so this proved nothing");
        println!("{checked} meshes decoded and agreed with their own headers");
    }

    /// Rubbish is refused with a reason rather than decoded into nonsense.
    #[test]
    fn geometry_that_cannot_be_read_is_refused_with_a_reason() {
        assert!(read_geometry(b"not a model", "wood")
            .unwrap_err()
            .contains("not a GLB"));

        // A real file, asked for a mesh it does not have: the reason names what it
        // DOES have, because that is the whole question when a rename breaks this.
        let road = std::path::Path::new("assets/models/tree_oak.glb");
        if let Ok(bytes) = std::fs::read(road) {
            let why = read_geometry(&bytes, "trunk").unwrap_err();
            assert!(why.contains("no mesh named `trunk`"), "unhelpful: {why}");
            assert!(why.contains("wood"), "the reason does not say what is there: {why}");
        }
    }

    /// A GLB carrying nothing but the numbers this checks.
    ///
    /// Built here rather than committed as a binary fixture: what is under test is
    /// the reading of an extent and the judging of it, and a hand-built header says
    /// exactly what extent it means. A pair of `.glb` files in the repository would
    /// say it too, less legibly, and would have to be regenerated by hand every
    /// time a rule moved.
    fn glb(low: [f32; 3], high: [f32; 3], triangles: usize) -> Vec<u8> {
        let json = format!(
            concat!(
                r#"{{"asset":{{"version":"2.0"}},"#,
                r#""accessors":[{{"type":"VEC3","componentType":5126,"count":{count},"#,
                r#""min":[{lx},{ly},{lz}],"max":[{hx},{hy},{hz}]}},"#,
                r#"{{"type":"SCALAR","componentType":5125,"count":{count}}}],"#,
                r#""meshes":[{{"name":"body","primitives":[{{"attributes":{{"POSITION":0}},"#,
                r#""indices":1}}]}}]}}"#
            ),
            count = triangles * 3,
            lx = low[0],
            ly = low[1],
            lz = low[2],
            hx = high[0],
            hy = high[1],
            hz = high[2],
        );
        wrap(json.as_bytes())
    }

    /// The GLB container around a JSON payload: magic, version, length, one chunk.
    fn wrap(json: &[u8]) -> Vec<u8> {
        let mut body = json.to_vec();
        while !body.len().is_multiple_of(4) {
            body.push(b' ');
        }
        let mut out = Vec::new();
        out.extend_from_slice(b"glTF");
        out.extend_from_slice(&2u32.to_le_bytes());
        out.extend_from_slice(&((12 + 8 + body.len()) as u32).to_le_bytes());
        out.extend_from_slice(&(body.len() as u32).to_le_bytes());
        out.extend_from_slice(b"JSON");
        out.extend_from_slice(&body);
        out
    }

    /// A warden-sized figure standing on its own origin passes.
    #[test]
    fn a_model_built_to_the_conventions_is_accepted() {
        let bytes = glb([-0.25, 0.0, -0.3], [0.25, 1.8, 0.3], 400);
        let model = inspect(&bytes).expect("a hand-built GLB should read");
        assert_eq!(model.triangles, 400);
        assert!((model.high[1] - 1.8).abs() < 0.001, "the height read wrong");
        assert_eq!(model.meshes, vec![("body".to_string(), 1)]);
        let faults = faults(&model);
        assert!(faults.is_empty(), "a good model was refused: {faults:?}");
    }

    /// The centimetres mistake: everything a hundred times too big.
    ///
    /// This is the fixture that caught the FIRST version of these bounds. The cap
    /// was 200 m, a 1.8 m figure built in centimetres is 180 m, and it sailed
    /// straight through the check meant to stop exactly that.
    #[test]
    fn a_model_a_hundred_times_too_big_is_refused() {
        let model = inspect(&glb([-25.0, 0.0, -30.0], [25.0, 180.0, 30.0], 400)).expect("reads");
        let faults = faults(&model);
        assert!(
            faults.iter().any(|why| why.contains("scale mistake")),
            "180 m passed as a reasonable size: {faults:?}"
        );
    }

    /// Its origin at the waist, or a stray offset: it imports hovering.
    #[test]
    fn a_model_that_floats_over_the_floor_is_refused() {
        let model = inspect(&glb([-0.25, 1.0, -0.3], [0.25, 2.8, 0.3], 400)).expect("reads");
        let faults = faults(&model);
        assert!(
            faults.iter().any(|why| why.contains("hovering")),
            "a model a metre off the ground passed: {faults:?}"
        );
    }

    /// Exported Z-up: it arrives on its back, and the fault says so.
    ///
    /// The shape of it is what gives it away. A 1.8 m figure exported Z-up has its
    /// height along Z, and its Y then spans the figure's DEPTH with half of that
    /// under the floor — so it is both sunk and wider than it is tall, which
    /// nothing built correctly ever is.
    #[test]
    fn a_model_exported_z_up_is_refused_and_told_why() {
        let model = inspect(&glb([-0.25, -0.3, 0.0], [0.25, 0.3, 1.8], 400)).expect("reads");
        let faults = faults(&model);
        assert!(
            faults.iter().any(|why| why.contains("Z-UP")),
            "a model on its back passed, or was refused for the wrong reason: {faults:?}"
        );
    }

    /// Rubbish is refused with a reason rather than panicking.
    #[test]
    fn something_that_is_not_a_model_is_refused_with_a_reason() {
        assert!(inspect(b"not a model at all")
            .unwrap_err()
            .contains("not a GLB"));
        assert!(inspect(&[]).unwrap_err().contains("not a GLB"));

        let mut short = glb([0.0; 3], [1.0, 1.0, 1.0], 2);
        short.truncate(30);
        let why = inspect(&short).unwrap_err();
        assert!(!why.is_empty(), "a truncated file gave an empty reason");

        // A GLB whose header is fine and which says nothing about any size.
        let empty = wrap(br#"{"asset":{"version":"2.0"},"meshes":[],"accessors":[]}"#);
        assert!(
            inspect(&empty).is_err(),
            "a file with no geometry in it was accepted"
        );
    }

    /// Every model actually in the game keeps the conventions.
    ///
    /// The gate on real content. There are no models yet — everything the game
    /// draws is built from primitives in code — so today this checks nothing and
    /// says so. It starts working the moment the first `.glb` lands, which is the
    /// point: nobody has to remember to come back and write it.
    #[test]
    fn every_model_in_the_game_keeps_the_conventions() {
        let Ok(entries) = std::fs::read_dir("assets/models") else {
            println!("no assets/models folder yet");
            return;
        };
        let mut checked = 0;
        for entry in entries.flatten() {
            let road = entry.path();
            if road.extension().and_then(|end| end.to_str()) != Some("glb") {
                continue;
            }
            let name = road.file_name().unwrap_or_default().to_string_lossy().to_string();
            let bytes = std::fs::read(&road).expect("a model that is there should read");
            let model =
                inspect(&bytes).unwrap_or_else(|why| panic!("{name} is not a model: {why}"));
            let faults = faults(&model);
            assert!(faults.is_empty(), "{name}: {}", faults.join("; "));
            checked += 1;
        }
        println!("{checked} model(s) in assets/models keep the conventions");
    }

    /// The two gates allow the same things.
    ///
    /// `dev/model_export.py` checks a model as it is exported, and this checks
    /// whatever is in the folder — and they agree only because somebody wrote the
    /// same three numbers twice. That is the shape of fault this project keeps
    /// meeting: one question with two answers, drifting apart quietly. So the
    /// agreement is tested rather than trusted. Move a bound in one place and this
    /// fails, naming the other.
    #[test]
    fn the_two_gates_agree_about_what_they_allow() {
        let script = std::fs::read_to_string("dev/model_export.py")
            .expect("dev/model_export.py should be beside the crate");
        for (name, ours) in [
            ("SMALLEST", SMALLEST),
            ("LARGEST", LARGEST),
            ("FOOTING_SLACK", FOOTING_SLACK),
        ] {
            let prefix = format!("{name} = ");
            let line = script
                .lines()
                .find(|line| line.starts_with(&prefix))
                .unwrap_or_else(|| panic!("{name} is not set in dev/model_export.py at all"));
            let said: f32 = line
                .split('=')
                .nth(1)
                .and_then(|rest| rest.trim().parse().ok())
                .unwrap_or_else(|| panic!("cannot read {name} from {line:?}"));
            assert!(
                (said - ours).abs() < 1e-6,
                "the export script allows {name} = {said} and models.rs allows {ours} — \
                 the two gates have drifted apart"
            );
        }
    }
}
