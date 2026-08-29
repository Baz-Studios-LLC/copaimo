//! Timing the parts of the game that can be timed honestly.
//!
//!     cargo run --release -- --measure stream
//!     cargo run --release -- --measure stream --repeats 5 --at 223,385
//!
//! # Why this exists before any optimisation
//!
//! Codex's audit opens by asking for a measurement contract, and it is right to:
//! every performance claim in this project's history that was not measured turned
//! out to be about the wrong thing. The comment saying chunk generation is "off the
//! frame budget entirely" is true of the sampling and false of the integration. The
//! comment saying cascades cost 16.7 ms of a 23.8 ms frame is the only number in the
//! codebase that was ever actually taken.
//!
//! # What this can measure, and what it deliberately cannot
//!
//! It measures PURE CPU WORK: functions that take a world and return geometry, with
//! no window, no GPU, no frame loop. `build_chunk` is exactly that by design - it
//! says so in its own docstring, because it runs on a task pool - so timing it here
//! is timing the real thing rather than a model of it.
//!
//! It cannot measure frame time, GPU passes, draw calls or the cost of handing a
//! finished mesh to Bevy. Those need a capture on real hardware with Tracy, and the
//! audit's §3 is the right shape for that work. Pretending a headless loop stands in
//! for them would be the same mistake as the comments above.
//!
//! So: this is the ruler for generation, and it is honest about its own edges.
//!
//! # Repeatable on purpose
//!
//! A fixed anchor, the real view disc, and the median of several passes rather than
//! one. The first pass is reported separately because it is the cold one - it pays
//! for whatever the terrain caches on first touch - and averaging it in would hide
//! both numbers.

use bevy::prelude::*;
use std::time::Instant;

use crate::config::{CHUNK_SIZE, VIEW_CHUNKS};
use crate::world::chunk;
use crate::world::terrain::Terrain;

/// What was asked for on the command line.
pub struct Job {
    /// Which measurement to run.
    what: String,
    /// How many passes to take the median of.
    repeats: usize,
    /// Where the viewer stands, in world metres.
    at: Vec2,
}

impl Job {
    /// Reads `--measure` and friends, if they are there.
    ///
    /// Hand-parsed like `photo::asked_for`, and for the same reason: three arguments
    /// do not justify a dependency the shipped game would carry.
    pub fn asked_for() -> Option<Job> {
        let args: Vec<String> = std::env::args().collect();
        let value = |name: &str| -> Option<String> {
            args.iter()
                .position(|a| a == name)
                .and_then(|at| args.get(at + 1))
                .cloned()
        };
        let what = value("--measure")?;
        let at = value("--at")
            .and_then(|spot| {
                let (x, z) = spot.split_once(',')?;
                Some(Vec2::new(x.trim().parse().ok()?, z.trim().parse().ok()?))
            })
            // The first city on the list, so the disc has settlements, roads and
            // coast in it rather than empty meadow.
            .unwrap_or(Vec2::new(223.0, 385.0));
        Some(Job {
            what,
            repeats: value("--repeats").and_then(|v| v.parse().ok()).unwrap_or(3),
            at,
        })
    }
}

/// Runs the measurement and prints what it found.
pub fn run(job: Job) {
    println!("measuring `{}` at {}, {}", job.what, job.at.x, job.at.y);

    let began = Instant::now();
    let terrain = Terrain::new();
    let born = began.elapsed();
    println!("  Terrain::new                     {:>9.1} ms", born.as_secs_f64() * 1000.0);

    match job.what.as_str() {
        "stream" => stream(&terrain, &job),
        other => println!("  no measurement called `{other}` - try `stream`"),
    }
}

/// Every chunk the view disc holds, nearest first, exactly as `queue_chunks` picks
/// them. Taken from the same two lines so the set cannot drift from the real one.
fn view_disc(at: Vec2) -> Vec<IVec2> {
    let centre = chunk::chunk_at(Vec3::new(at.x, 0.0, at.y));
    let radius_sq = VIEW_CHUNKS * VIEW_CHUNKS;
    let mut wanted: Vec<(i32, IVec2)> = Vec::new();
    for dz in -VIEW_CHUNKS..=VIEW_CHUNKS {
        for dx in -VIEW_CHUNKS..=VIEW_CHUNKS {
            let away = dx * dx + dz * dz;
            if away <= radius_sq {
                wanted.push((away, centre + IVec2::new(dx, dz)));
            }
        }
    }
    wanted.sort_unstable_by_key(|(away, _)| *away);
    wanted.into_iter().map(|(_, coord)| coord).collect()
}

/// What it costs to fill the view disc once: the whole of a cold start's terrain.
fn stream(terrain: &Terrain, job: &Job) {
    let disc = view_disc(job.at);
    println!(
        "  view disc                        {:>6} chunks, {:.0} m across",
        disc.len(),
        f64::from(VIEW_CHUNKS * 2 + 1) * f64::from(CHUNK_SIZE),
    );

    // WHAT SHIPS, and what of it is the ground.
    //
    // `build_chunk` is what the streaming task calls, so that is the number that
    // matters; `build_mesh` is timed beside it only to attribute the difference.
    // Timing `build_river` directly would measure a function rather than the path,
    // and would go on reporting the same cost after the call to it was removed.
    let mut passes: Vec<(f64, f64, usize)> = Vec::new();
    for _ in 0..job.repeats.max(1) {
        let mut whole = 0.0_f64;
        let mut ground = 0.0_f64;
        let mut wet = 0;
        for coord in &disc {
            let began = Instant::now();
            let (mesh, river) = chunk::build_chunk(terrain, *coord);
            whole += began.elapsed().as_secs_f64();
            wet += usize::from(river.is_some());
            std::hint::black_box((&mesh, &river));

            let began = Instant::now();
            let mesh = chunk::build_mesh(terrain, *coord);
            ground += began.elapsed().as_secs_f64();
            std::hint::black_box(&mesh);
        }
        passes.push((whole * 1000.0, ground * 1000.0, wet));
    }

    let cold = passes[0];
    println!(
        "  first pass (cold)                {:>9.1} ms build_chunk   {:>9.1} ms of it ground",
        cold.0, cold.1,
    );
    if passes.len() > 1 {
        let warm = &passes[1..];
        let middle = |mut of: Vec<f64>| -> f64 {
            of.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            of[of.len() / 2]
        };
        let whole = middle(warm.iter().map(|(w, ..)| *w).collect());
        let ground = middle(warm.iter().map(|(_, g, _)| *g).collect());
        println!(
            "  median of {} more                {:>9.1} ms build_chunk   {:>9.1} ms of it ground",
            warm.len(),
            whole,
            ground,
        );
        println!(
            "  everything that is not ground    {:>9.1} ms   ({:.0} % on top of the mesh)",
            whole - ground,
            if ground > 0.0 { (whole - ground) / ground * 100.0 } else { 0.0 },
        );
    }
    println!(
        "  chunks with water in them        {:>6} of {}   (RIVERS = {})",
        cold.2,
        disc.len(),
        crate::config::RIVERS,
    );
}
