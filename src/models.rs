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
/// **These must match `dev/model_export.py`** — see
/// `the_two_gates_agree_about_what_they_allow`, which reads that file and checks.
const SMALLEST: f32 = 0.02;
const LARGEST: f32 = 60.0;

/// How far a model's base may sit off the floor before it counts as floating.
const FOOTING_SLACK: f32 = 0.02;

/// What a glTF file says about the shape inside it.
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

#[cfg(test)]
mod tests {
    use super::*;

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
