//! An image in, a model out.
//!
//! Everything else on this bench makes a thing out of parts the bench already
//! holds. This hands a picture to somebody else's machine and keeps what comes
//! back: a GLB, mesh and materials and all, dropped into `assets/models` where the
//! placed sheet can stand it.
//!
//! # A generated mesh is not a part
//!
//! Worth saying at the top, because the temptation is to turn one into kit pieces
//! so that everything on the bench is the same kind of thing. It cannot be done
//! honestly. A part is a NAME that resolves to boxes on a lattice painted from a
//! shelf; a generated mesh is arbitrary triangles carrying their own PBR
//! materials. It cannot be painted, snapped to the lattice, or written into a
//! building's `boxes` — so it stays a FILE and is carried whole.
//!
//! # It costs money and it leaves the building
//!
//! Every firing spends credits on somebody's account and uploads the picture to a
//! third party. So it happens **on a press and never on its own**: nothing here
//! retries, polls ahead of being asked, or fires twice for one image.
//!
//! # The key is never in this repository
//!
//! `COPAIMO_3DAI_KEY`, or a file in the maker's own home. Not in `assets`, not in
//! `config.rs`, and not anywhere `git add -A` can reach — a key committed once is
//! a key that has to be rotated, and the commit that did it is usually the one
//! nobody looked at.
//!
//! # The contract
//!
//! ```text
//! POST /v1/3d-models/trellis2/generate/   { "image": "data:image/png;base64,..." }
//!   -> { "task_id": "..." }
//! GET  /v1/generation-request/<id>/status/
//!   -> PENDING | IN_PROGRESS | FINISHED | FAILED
//!   -> FINISHED: { "results": [ { "asset": "https://...", "asset_type": "3D_MODEL" } ] }
//! ```
//!
//! GLB always, one to three minutes. See <https://docs.3daistudio.com>.

use bevy::prelude::*;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::sync::Mutex;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// Where the machine lives.
const HOUSE: &str = "https://api.3daistudio.com/v1";

/// Which machine does the work. The quick one, because a bench asks questions.
const MAKER: &str = "3d-models/trellis2/generate/";

/// Where a finished model lands, so the placed sheet can name it.
const MODELS: &str = "assets/models";

/// How long to wait before giving up on a job entirely.
///
/// A quarter of an hour. A job that has not finished by then has gone wrong at the
/// far end, and waiting for ever is how a bench ends up saying WORKING overnight.
const PATIENCE: std::time::Duration = std::time::Duration::from_secs(15 * 60);

/// How many "finished but nothing attached" answers to sit through.
///
/// At five seconds apart this is a minute and a half, which is generous for
/// attaching a file to a record that already says it is complete.
const SETTLING: u32 = 18;

/// What the kiln is doing, for saying so on screen.
#[derive(Resource, Default)]
pub struct Firing {
    pub said: String,
    /// The job in flight, if there is one. One at a time, deliberately: every
    /// firing costs credits, and a key that started a second one by accident would
    /// be spending somebody's money on a double-press.
    /// The far end of the firing thread — see `start` for why it is a THREAD.
    ///
    /// In a mutex only because a resource must be `Sync` and a receiver is not;
    /// nothing ever contends for it.
    job: Option<Mutex<Receiver<Result<PathBuf, String>>>>,
}

impl Firing {
    pub fn busy(&self) -> bool {
        self.job.is_some()
    }
}

/// Reads the key, from the environment or the maker's own home.
///
/// Never from this repository. A key committed once has to be rotated, and the
/// commit that did it is usually the one nobody looked at.
fn key() -> Result<String, String> {
    if let Ok(key) = std::env::var("COPAIMO_3DAI_KEY") {
        let key = key.trim().to_string();
        if !key.is_empty() {
            return Ok(key);
        }
    }
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map_err(|_| "no home folder to look in".to_string())?;
    let path = Path::new(&home).join(".copaimo").join("3daistudio.key");
    let key = std::fs::read_to_string(&path)
        .map_err(|_| {
            format!(
                "no key. Set COPAIMO_3DAI_KEY, or put it in {}",
                path.display()
            )
        })?
        .trim()
        .to_string();
    if key.is_empty() {
        return Err(format!("{} is empty", path.display()));
    }
    Ok(key)
}

/// An image as the machine wants it: a data URI, base64, with its type named.
fn as_data_uri(path: &Path) -> Result<String, String> {
    let kind = match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .as_deref()
    {
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        other => return Err(format!("{other:?} is not a picture this can send")),
    };
    let bytes = std::fs::read(path).map_err(|why| format!("{}: {why}", path.display()))?;
    Ok(format!("data:{kind};base64,{}", base64(&bytes)))
}

/// RFC 4648 base64, written out rather than taken as a dependency.
///
/// Twenty lines against a crate, its transitive tree, and a version to keep in
/// step — for an encoding that has not changed since 2006.
fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for lot in bytes.chunks(3) {
        let packed = (lot[0] as u32) << 16
            | (lot.get(1).copied().unwrap_or(0) as u32) << 8
            | lot.get(2).copied().unwrap_or(0) as u32;
        out.push(ALPHABET[(packed >> 18 & 63) as usize] as char);
        out.push(ALPHABET[(packed >> 12 & 63) as usize] as char);
        out.push(if lot.len() > 1 {
            ALPHABET[(packed >> 6 & 63) as usize] as char
        } else {
            '='
        });
        out.push(if lot.len() > 2 {
            ALPHABET[(packed & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}

/// A name nothing else in the models folder is using.
fn a_free_name(wanted: &str) -> PathBuf {
    let folder = Path::new(MODELS);
    let first = folder.join(format!("{wanted}.glb"));
    if !first.exists() {
        return first;
    }
    // Never overwritten. A firing is paid for, and landing a second one on top of
    // the first would spend money to destroy what the money before it bought.
    for n in 2..1_000 {
        let road = folder.join(format!("{wanted}-{n}.glb"));
        if !road.exists() {
            return road;
        }
    }
    folder.join(format!("{wanted}-many.glb"))
}

/// What one poll of the job said.
enum Step {
    Waiting(String),
    /// Finished, and here is the model.
    Ready(String),
    /// Finished, and nothing attached yet.
    Settling,
}

fn how_it_goes(said: &str) -> Result<Step, String> {
    let body: serde_json::Value =
        serde_json::from_str(said).map_err(|why| format!("the kiln spoke nonsense: {why}"))?;
    let status = body
        .get("status")
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .to_ascii_uppercase();
    if status == "FAILED" {
        let why = body
            .get("error")
            .and_then(|e| e.as_str())
            .unwrap_or("no reason given");
        return Err(format!("the kiln failed: {why}"));
    }
    if status != "FINISHED" {
        return Ok(Step::Waiting(status));
    }
    let model = body
        .get("results")
        .and_then(|r| r.as_array())
        .and_then(|all| {
            all.iter()
                .filter(|one| {
                    one.get("asset_type").and_then(|t| t.as_str()) == Some("3D_MODEL")
                })
                .find_map(|one| one.get("asset").and_then(|a| a.as_str()))
        });
    match model {
        Some(url) => Ok(Step::Ready(url.to_string())),
        // Finished by its own account, with the file not yet attached. Told apart
        // from failure because it resolves itself, usually within seconds.
        None => Ok(Step::Settling),
    }
}

/// Says how a job is going, into the log.
///
/// The panel cannot be written to from the thread doing the work, and a firing
/// takes minutes — so the words go where a maker can find them if they wonder
/// whether anything is happening at all.
fn firing_says(word: &str) {
    info!("the kiln: {word}");
}

/// The whole firing, start to finish, on a background thread.
fn fire(picture: PathBuf, name: String) -> Result<PathBuf, String> {
    let key = key()?;
    let uri = as_data_uri(&picture)?;
    std::fs::create_dir_all(MODELS).map_err(|why| format!("{MODELS}: {why}"))?;

    let taken = ureq::post(format!("{HOUSE}/{MAKER}"))
        .header("Authorization", format!("Bearer {key}"))
        .send_json(serde_json::json!({ "image": uri }))
        .map_err(|why| format!("the kiln would not take it: {why}"))?
        .body_mut()
        .read_to_string()
        .map_err(|why| format!("the kiln said nothing back: {why}"))?;

    let job = serde_json::from_str::<serde_json::Value>(&taken)
        .ok()
        .and_then(|body| {
            body.get("task_id")
                .or_else(|| body.get("id"))
                .and_then(|id| id.as_str().map(str::to_string))
        })
        .ok_or_else(|| format!("the kiln named no job: {taken}"))?;

    // Now it is paid for. Poll until it is one thing or the other.
    let began = std::time::Instant::now();
    let mut settling = 0;
    let url = loop {
        if began.elapsed() > PATIENCE {
            return Err("the kiln never finished; the job may still be on your account".into());
        }
        std::thread::sleep(std::time::Duration::from_secs(5));
        let said = ureq::get(format!("{HOUSE}/generation-request/{job}/status/"))
            .header("Authorization", format!("Bearer {key}"))
            .call()
            .map_err(|why| format!("lost the kiln: {why}"))?
            .body_mut()
            .read_to_string()
            .map_err(|why| format!("lost the kiln: {why}"))?;
        match how_it_goes(&said)? {
            Step::Waiting(word) => firing_says(&word),
            Step::Ready(url) => break url,
            Step::Settling => {
                settling += 1;
                if settling > SETTLING {
                    return Err(format!(
                        "the kiln said it finished and attached nothing for {} seconds",
                        settling * 5
                    ));
                }
            }
        }
    };

    // STREAMED to the file, never gathered in memory first.
    //
    // `read_to_vec` carries a ten-megabyte cap and a textured GLB goes past it
    // without trying — which fails AFTER the model has been made and paid for, the
    // worst place to fail. Raising the cap only moves the number; a mesh has no
    // size a bench should be guessing at.
    info!("fetching {url}");
    let mut answer = ureq::get(&url)
        // A stop on the whole fetch. Without one a stalled connection leaves the
        // bench fetching for ever with a paid-for model half on the disk, and no
        // way to tell that from a slow line.
        .config()
        .timeout_global(Some(std::time::Duration::from_secs(10 * 60)))
        .build()
        .call()
        .map_err(|why| format!("could not fetch the model: {why}"))?;

    let road = a_free_name(&name);
    let mut file =
        std::fs::File::create(&road).map_err(|why| format!("{}: {why}", road.display()))?;
    let mut body = answer.body_mut().as_reader();
    let mut lot = [0_u8; 64 * 1024];
    let mut total = 0_u64;
    loop {
        let read = body
            .read(&mut lot)
            .map_err(|why| format!("the model stopped coming: {why}"))?;
        if read == 0 {
            break;
        }
        file.write_all(&lot[..read])
            .map_err(|why| format!("{}: {why}", road.display()))?;
        total += read as u64;
    }
    // A GLB starts with "glTF". Checked because the far end sending an error page
    // with a 200 on it would otherwise land as a .glb nobody can open, and the
    // firing is already paid for by this point.
    if total < 12 {
        let _ = std::fs::remove_file(&road);
        return Err(format!("the kiln sent {total} bytes, which is not a model"));
    }
    let mut magic = [0_u8; 4];
    if std::fs::File::open(&road)
        .and_then(|mut f| f.read_exact(&mut magic))
        .is_ok()
        && &magic != b"glTF"
    {
        let _ = std::fs::remove_file(&road);
        return Err("what came back is not a GLB".into());
    }

    Ok(road)
}

/// The key, and what it does.
///
/// **Only on a press.** Nothing here fires on its own, retries, or starts a second
/// job while one is running: every firing spends credits, and a tool that could
/// spend them twice for one press is a tool nobody should leave open.
pub fn ask(
    keys: Res<ButtonInput<KeyCode>>,
    reference: Res<super::reference::Reference>,
    mut firing: ResMut<Firing>,
) {
    if !keys.just_pressed(KeyCode::F5) {
        return;
    }
    start(&reference, &mut firing);
}

/// The firing itself, so the key and the panel's button do exactly one thing.
///
/// Two paths to something that spends money is two chances to spend it twice.
pub fn start(reference: &super::reference::Reference, firing: &mut Firing) {
    if firing.busy() {
        firing.said = "already firing - one at a time".into();
        return;
    }
    let Some(picture) = reference.chosen() else {
        firing.said = format!("pick a picture first with I ({})", reference.said());
        return;
    };
    let name = picture
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "model".into());

    firing.said = format!("sending {name} - costs credits, minutes");
    let picture = picture.clone();
    // A thread of its own, NOT the compute pool. `fire` blocks — sleeps between
    // polls, blocking HTTP — for up to fifteen minutes, and the compute pool is
    // where chunk meshes and the minimap are built: a firing parked on one of
    // its few threads could starve terrain meshing for the whole wait. A
    // blocking job gets a blocking thread.
    let (send, receive) = std::sync::mpsc::channel();
    let spun = std::thread::Builder::new()
        .name("kiln".into())
        .spawn(move || {
            let _ = send.send(fire(picture, name));
        });
    match spun {
        Ok(_) => firing.job = Some(Mutex::new(receive)),
        Err(why) => firing.said = format!("kiln: could not start a thread: {why}"),
    }
}

/// Picks the finished job up.
pub fn collect(mut firing: ResMut<Firing>) {
    // Read to a plain value FIRST, so the borrow of the receiver has ended by
    // the time anything writes back into the resource.
    let answer = match firing.job.as_ref() {
        // The poison error is dropped rather than kept: it carries the guard,
        // and a guard still in hand is a borrow still alive when the writes
        // below need the resource.
        Some(job) => job.lock().map(|line| line.try_recv()).map_err(|_| ()),
        None => return,
    };
    let Ok(answer) = answer else {
        firing.job = None;
        firing.said = "kiln: the firing thread poisoned its own line".into();
        return;
    };
    let done = match answer {
        Ok(done) => done,
        // Still firing.
        Err(TryRecvError::Empty) => return,
        // The thread died without answering — a panic in `fire`. An answer the
        // maker can read beats a job that says "firing" forever.
        Err(TryRecvError::Disconnected) => Err("the firing thread died without answering".into()),
    };
    firing.job = None;
    match done {
        Ok(road) => {
            let name = road.file_stem().unwrap_or_default().to_string_lossy();
            info!("the kiln sent back {}", road.display());
            firing.said = format!("got {name} - place it by that name in placed.json");
        }
        Err(why) => {
            error!("the kiln: {why}");
            firing.said = format!("kiln: {why}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_says_what_the_rfc_says() {
        // The one thing here that can be checked without spending anybody's money.
        // Written out rather than depended on, so it is worth proving it agrees
        // with the standard rather than with itself.
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foob"), "Zm9vYg==");
        assert_eq!(base64(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
        // And bytes that are not text, since a picture is not text.
        assert_eq!(base64(&[0x00, 0xff, 0x80]), "AP+A");
    }

    #[test]
    fn a_failed_job_is_told_apart_from_a_slow_one() {
        // These three answers look alike and mean completely different things:
        // wait, give up, and here it is. Reading one as another either abandons a
        // job that was paid for or waits for ever on one that failed.
        assert!(matches!(
            how_it_goes(r#"{"status":"IN_PROGRESS"}"#),
            Ok(Step::Waiting(_))
        ));
        assert!(matches!(
            how_it_goes(r#"{"status":"PENDING"}"#),
            Ok(Step::Waiting(_))
        ));
        assert!(how_it_goes(r#"{"status":"FAILED","error":"no credits"}"#)
            .is_err_and(|why| why.contains("no credits")));

        // Finished with the model attached.
        let done = r#"{"status":"FINISHED","results":[
            {"asset":"https://x/y.glb","asset_type":"3D_MODEL"}]}"#;
        assert!(matches!(how_it_goes(done), Ok(Step::Ready(url)) if url.ends_with("y.glb")));

        // Finished, with something attached that is NOT a model — a preview image,
        // say. Taking the first asset regardless would download a jpg and save it
        // as a .glb.
        let preview = r#"{"status":"FINISHED","results":[
            {"asset":"https://x/y.png","asset_type":"IMAGE"}]}"#;
        assert!(matches!(how_it_goes(preview), Ok(Step::Settling)));

        // Finished by its own account with nothing attached yet, which resolves
        // itself and must not be read as failure.
        assert!(matches!(
            how_it_goes(r#"{"status":"FINISHED","results":[]}"#),
            Ok(Step::Settling)
        ));
    }

    #[test]
    fn a_firing_never_lands_on_top_of_an_earlier_one() {
        // A firing is paid for. Landing a second one on the first would spend money
        // to destroy what the money before it bought.
        let folder = Path::new(MODELS);
        let _ = std::fs::create_dir_all(folder);
        let taken = folder.join("kiln-test-name.glb");
        let _ = std::fs::write(&taken, b"glTF");

        let next = a_free_name("kiln-test-name");
        assert_ne!(next, taken, "it would have written over a model on disk");
        assert!(next.to_string_lossy().contains("kiln-test-name-2"));

        let _ = std::fs::remove_file(&taken);
        // And with nothing in the way it takes the plain name.
        assert_eq!(a_free_name("kiln-test-name"), taken);
    }

    #[test]
    fn only_pictures_are_sent() {
        // A firing costs money before anything is checked at the far end, so what
        // cannot possibly work is refused here rather than paid for.
        assert!(as_data_uri(Path::new("a.txt")).is_err());
        assert!(as_data_uri(Path::new("a.glb")).is_err());
    }
}
