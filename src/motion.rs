//! Plays the warden's clips, so walking looks like walking.
//!
//! # A rig with no clip is a figure that slides
//!
//! `dev/art/people.py` authors two clips into the body — `walk` and `idle` — and a
//! glTF skin arrives in Bevy as a `SkinnedMesh` with no help needed. What does not
//! happen on its own is anything playing: the skeleton just sits in its rest pose
//! while the warden slides about the world like a chess piece.
//!
//! # Found by name, not by index
//!
//! The clips could be reached as `Animation(0)` and `Animation(1)`, and the order
//! they come out in is whatever the exporter felt like — here it was idle first,
//! which is alphabetical and not meaningful. So the `Gltf` asset is read and the
//! clips are looked up by their own names. Adding a third clip then cannot silently
//! turn walking into standing.
//!
//! # The speed follows the warden
//!
//! A walk cycle played at a fixed rate against a warden moving at a different one
//! is the skating that every game with feet has to solve. The stride covers a known
//! distance, so the clip's speed is simply how many strides a second the warden is
//! actually managing.

use bevy::prelude::*;

use crate::player::{Player, Striding};

/// Above this, in metres a second, the warden is running rather than walking.
///
/// Between the walk's own comfortable pace and the sprint: a warden pushing along
/// at the top of a walk should already be running, because a walk cycle played fast
/// enough to keep up with a sprint reads as a cartoon scurry.
// Between the two speeds, and it has to be — this was 6.5 while WALK_SPEED was 7.0,
// so the threshold sat BELOW walking pace and the walk clip never played once. Any
// value here must lie strictly between `player::WALK_SPEED` and
// `player::SPRINT_SPEED`, which `the_gait_threshold_lies_between_the_speeds` checks.
#[allow(dead_code)]
const BREAKS_INTO_A_RUN: f32 = 3.4;

/// How far one cycle of each clip carries the warden, in metres.
///
/// # Measured off the planted SOLE of the shipped clips, not derived
///
/// `dev/art/native_speed.py` (scratch, rerun after any gait rebuild) tracks the
/// lowest cluster of the deformed shoe between consecutive planted frames and takes
/// its backward world velocity - the one number that is, by definition, the speed at
/// which that clip's feet do not slide. `covers` is that velocity times the cycle's
/// duration. Three earlier derivations each got it wrong a different way (leg reach,
/// doubled swing, contact-over-stance on bones instead of the shoe), and the history
/// of those mistakes is in git if it is ever wanted; the cure was to measure the
/// thing itself.
///
/// The identity behind it: `speed = cadence x stride`, and a planted foot travels
/// backward relative to the hips at exactly the character's speed. The contact patch
/// of a ROLLING shoe also advances heel-to-toe through stance, which the sole
/// tracking sees and bone-based estimates did not - it is worth about 10% here.
///
/// Current clips, measured 2026-08-22 (toe flex, shoe weights owned by the feet,
/// stride bounded by this character's short legs and a 6 cm cap on hip drop):
/// walk 0.926 m/s native, run 2.439, sprint 4.458. The
/// running sweeps are one leg length apiece - which only fits inside the leg's reach
/// because the stance window is asymmetric (land ~35% ahead, release ~65% behind,
/// the art pipeline's LANDS_AHEAD). Measured per VERTEX of the planted sole,
/// horizontal only: a centroid whose membership shifts as the shoe rolls reads as
/// slide when nothing slid, and vertical pad lift is not slide either.
const WALK_COVERS: f32 = 0.881;
const RUN_COVERS: f32 = 1.419;

/// The moving gaits, slowest first: the word in the clip's name, how far one cycle
/// carries the warden in metres, and the speed above which the next one takes over.
///
/// # A table rather than fields, because a third clip is coming
///
/// This was two named fields and a boolean. The comment above `find_the_clips` had
/// already predicted the problem — "reaching for `Animation(0)` would quietly make the
/// warden stand still the day a third clip is added" — and the same was true of the
/// playing code: every gait meant another arm of an `if`, another pair of constants
/// threaded through, and another chance to hand one gait's distance to another gait's
/// clip.
///
/// Adding a jog between the walk and the run is now a ROW. The selection, the cadence
/// and the tests all read this table, so there is no second place to update and forget.
///
/// `covers` is measured, never derived: `dev/art/stride_measure.py` fits a line to the
/// planted foot's travel to get the stance fraction, and a cycle carries `foot travel
/// / stance fraction`. See `WALK_COVERS` for why the obvious `2 x foot swing` is
/// wrong by 45% on anything with a flight phase.
const GAITS: &[(&str, f32, f32)] = &[
    ("walk", WALK_COVERS, halfway(crate::player::WALK_SPEED, crate::player::JOG_SPEED)),
    ("run", RUN_COVERS, halfway(crate::player::JOG_SPEED, crate::player::SPRINT_SPEED)),
    ("sprint", SPRINT_COVERS, f32::INFINITY),
];

/// How many frames each clip is authored over, and at what rate Blender wrote them.
///
/// These are here so a clip's NATIVE speed can be stated rather than guessed: a clip
/// authored over `frames` at `FPS` runs one cycle in `frames / FPS` seconds, so at its
/// own natural rate it carries `covers x FPS / frames` metres a second. That is the one
/// speed at which its feet do not slide.
// Asserted in the pacing tests rather than read at runtime, which is why the non-test
// build calls them dead. They describe how the CLIPS are authored, and
// `the_declared_frame_counts_match_the_clips` compares them against the actual file -
// so they are a checked record, not a comment. `BREAKS_INTO_A_RUN` below is the same
// case and was already in it.
#[allow(dead_code)]
const FPS: f32 = 24.0;
#[allow(dead_code)]
const WALK_FRAMES: f32 = 24.0;
#[allow(dead_code)]
const RUN_FRAMES: f32 = 24.0;
#[allow(dead_code)]
const SPRINT_FRAMES: f32 = 24.0;


/// Cadence does not transfer between bodies of different size: for dynamic similarity it
/// goes as 1/sqrt(leg).
///
/// 1.019, and it was 1.065 on MISMATCHED LANDMARKS - his hip-to-ANKLE (78.35 cm) against a
/// human hip-to-FLOOR (88.9). Measured on the same landmark for both, hip joint to floor,
/// his leg is 85.2 cm on a 170.2 cm figure (50.1% of height, which is the ordinary adult
/// 50-52% band and fine for the teenager he reads as) against a human 88.5 at that height.
/// sqrt(88.5 / 85.2) = 1.019.
///
/// The error made every ceiling about 4% too generous, which is exactly the direction that
/// lets a too-fast gait pass. A ratio is only a ratio if both ends measure the same thing.
const LEGS_SHORTER_BY: f32 = 1.019;

/// The cadence band each tier lives in, in steps a minute, before the tier above takes
/// over. ONE table: the handover ceilings and the churn test both read it, because they
/// were two copies that disagreed (140/200/260 here against 90-140/150-200/220-260 in the
/// test) and a bound that exists twice is a bound that drifts.
///
/// These are ABSURDITY bounds, not realism ones, and the distinction is the whole point.
///
/// They were human bands - 90-140 walking, 150-200 running, 220-260 sprinting - and they
/// were being used as a speed GATE: each driven speed got pinned just under the ceiling its
/// band allowed, so the speeds were never chosen, they were whatever realism permitted.
/// That is what made the jog feel, in the user's words, like running through water, and no
/// amount of tuning fixed it because the tuning knob was downstream of the gate.
///
/// This is a fantasy game about collecting and raising monsters, and the standard it is
/// held to for movement is Genshin Impact, not a gait laboratory. So realism is not a
/// constraint here at all: `player::JOG_SPEED` and friends are chosen by FEEL, and these
/// bands were widened until they no longer have an opinion about them.
///
/// What they still catch, and the only reason they survive, is a broken `covers`. Cadence
/// is `speed / covers`, so if a stride measurement goes wrong - the wrong mesh measured,
/// the stance fraction misread, a clip re-authored without re-measuring - the cadence lands
/// somewhere impossible and these say so. 60 or 400 steps a minute is a bug. 300 is a
/// choice.
///
/// Scaled by `LEGS_SHORTER_BY`, which is not a realism concession - it is this character's
/// legs being genuinely shorter than the figure the numbers were written for.
const CHURNS_BETWEEN: [(f32, f32); 3] = [(60.0, 180.0), (140.0, 330.0), (200.0, 400.0)];

/// What a clip carries at its own natural rate, in metres a second.
#[allow(dead_code)]
const fn natively_carries(covers: f32, frames: f32) -> f32 {
    covers * FPS / frames
}

/// The speed at which one tier hands over to the next: halfway between the two speeds
/// they are actually driven at.
///
/// Three versions preceded this and each was wrong in a way worth keeping.
///
/// It was `natively_carries(covers, frames) * STRETCHES_TO`, which assumes `frames / FPS`
/// IS the intended cycle duration. That held while the run was sixteen frames and broke
/// the day it became twenty-four: frame count is chosen for sampling density and tempo
/// comes from the playback rate, so a longer clip is not a slower gait.
///
/// Then `design_speed * STRETCHES_TO`, which selects correctly and is still wrong: 25%
/// more speed on a fixed stride is 25% more cadence, so any tier near the top of its band
/// churns at its own ceiling.
///
/// Then `covers * churns_above / 120.0` - the speed at which CADENCE would leave a
/// believable band. That was right for as long as the selection read a measured speed that
/// could land anywhere, because then a tier really could be asked to carry every speed up
/// to its ceiling. It stopped being right in two steps. `Striding::wants` made the
/// selection read INTENT, so each tier now carries exactly one speed and a ceiling bounds
/// nothing; and the bands were dropped as a speed gate, because pinning the driven speed
/// just under a cadence ceiling is what made the jog feel slow - the speed was never a
/// choice, it was whatever a human band permitted.
///
/// So the speeds are the knobs now, chosen by feel, and this is only the line between
/// them. Halfway gives the widest margin either side, which matters because a ceiling
/// sitting a fraction under a driven speed is exactly the fragility that bit last time.
const fn halfway(slower: f32, faster: f32) -> f32 {
    (slower + faster) / 2.0
}

/// What one sprint cycle carries, measured like the others - see `WALK_COVERS`.
///
/// 2.601 m over fourteen frames is 4.46 m/s natively at 206 steps a minute. The
/// sprint sweeps the same ground per stance as the run (planted-foot travel stays
/// near one leg length at every speed - 0.99 +/- 0.08 m from 6.2 to 11.1 m/s) and
/// buys its speed with a shorter cycle and a longer flight, never by reaching
/// further. Trying to reach further is why 42 degrees of thigh swing once read as
/// the splits, and it is also why the first sprint clip came out NATIVELY SLOWER
/// than the run: it kept the run's cadence and shrank its sweep.
const SPRINT_COVERS: f32 = 1.873;

/// What to hand `set_speed` so a clip plays at the right cadence.
///
/// `set_speed` is a MULTIPLE of a clip's natural rate — one cycle over its authored
/// duration — and not a rate. So playing `speed / covers` cycles a second means
/// asking for that many multiplied by however long the clip happens to be.
///
/// Leaving the duration out is a bug that hides: the walk lasts 1.042 s, near enough
/// to one that nothing looked wrong, while the run lasts 0.708 and played 41% too
/// fast. The property that catches it is that the cadence must come out the SAME
/// whatever the clip's length, which is what the test asserts.
fn playback_rate(speed: f32, covers: f32, clip_lasts: f32) -> f32 {
    clip_lasts * speed / covers
}

/// How long one gait eases into another, in seconds.
///
/// Short: a warden who starts walking should look like they started walking, not
/// like they faded into it. Long enough that the switch is not a snap.
const BLEND: f32 = 0.18;

/// The clips, once they have been found and put in a graph.
#[derive(Resource)]
pub struct Motions {
    graph: Handle<AnimationGraph>,
    idle: AnimationNodeIndex,
    /// Every moving gait the body actually carries, slowest first.
    ///
    /// Built from `GAITS`, skipping any the file does not have — a body with only a
    /// walk still walks, and sprints by walking faster, which is wrong but is not
    /// broken and beats refusing to animate.
    gaits: Vec<Gait>,
    /// How long each gait's clip runs, in seconds.
    ///
    /// # Why a cadence needs the clip's own length
    ///
    /// `set_speed` is a MULTIPLE of a clip's natural rate, not a rate. A clip's
    /// natural rate is one cycle over its authored duration, so cycles a second is
    /// `speed / duration` — and handing it `strides_a_second` alone silently assumes
    /// every clip lasts exactly one second.
    ///
    /// The walk very nearly does, at 1.042 s. The run lasts 0.708, because it is
    /// authored over sixteen frames rather than twenty-four, so it played 41% too
    /// fast and the feet skated for it. Nothing in the cadence test could catch that,
    /// because the test checked the number being ASKED for rather than the one the
    /// player would produce.
    idle_lasts: f32,
}

/// One gait, with everything needed to play it at the right rate.
struct Gait {
    node: AnimationNodeIndex,
    /// How far one cycle carries the warden, in metres.
    covers: f32,
    /// How long the clip runs, in seconds — see `playback_rate`.
    lasts: f32,
    /// Above this speed the next gait up takes over.
    upto: f32,
}

/// The body's own glTF, held while its clips are being waited for.
#[derive(Resource)]
pub struct Waiting(Handle<Gltf>);

/// Asks for the body file, so its clips can be read out of it.
pub fn ask_for_the_clips(
    mut commands: Commands,
    assets: Res<AssetServer>,
    look: Res<crate::look::Look>,
) {
    commands.insert_resource(Waiting(assets.load(look.build.model())));
}

/// Builds the graph once the file has loaded.
///
/// By NAME: the order glTF writes its animations in is the exporter's business, and
/// reaching for `Animation(0)` would quietly make the warden stand still the day a
/// third clip is added.
pub fn find_the_clips(
    mut commands: Commands,
    waiting: Res<Waiting>,
    files: Res<Assets<Gltf>>,
    clips: Res<Assets<AnimationClip>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    let Some(file) = files.get(&waiting.0) else {
        return;
    };
    // # Found by the word in the name, not by the whole of it
    //
    // A clip authored here is called `walk`. A clip that came with a generated rig
    // is called `preset:biped:idle`. Insisting on an exact name meant the made
    // warden had no idle at all as far as the game was concerned — it warned that
    // the body carried nothing it recognised and let him slide, while the clip sat
    // right there under a longer name.
    //
    // So a clip is matched on CONTAINING its gait's word, case-insensitively. That
    // reads every convention anybody is likely to export under, and the exact name
    // is still preferred when both are present.
    let named = |gait: &str| -> Option<&Handle<AnimationClip>> {
        file.named_animations.get(gait).or_else(|| {
            file.named_animations
                .iter()
                .find(|(name, _)| name.to_lowercase().contains(gait))
                .map(|(_, clip)| clip)
        })
    };
    // Only the idle is required here. Which moving gaits exist is the `GAITS` loop's
    // business, and it warns per missing gait rather than refusing the lot.
    let Some(idle) = named("idle") else {
        warn!(
            "the body has no idle — it carries {:?}, so the warden will slide",
            file.named_animations.keys().collect::<Vec<_>>()
        );
        commands.remove_resource::<Waiting>();
        return;
    };
    // The clips themselves, for their durations. They are sub-assets of the file, so
    // they normally arrive with it — but not necessarily in the same frame, and a
    // duration of nought would divide the cadence into infinity. So this waits.
    let Some(idle_lasts) = clips.get(idle).map(AnimationClip::duration) else {
        return;
    };

    let mut graph = AnimationGraph::new();
    let standing = graph.add_clip(idle.clone(), 1.0, graph.root);
    let mut gaits = Vec::new();
    for (called, covers, upto) in GAITS {
        let Some(clip) = named(called) else {
            info!("the body has no {called} clip; the gait above it will cover for it");
            continue;
        };
        let Some(lasts) = clips.get(clip).map(AnimationClip::duration) else {
            return;
        };
        if lasts <= 0.0 {
            warn!("the {called} clip has no length, so its cadence cannot be set");
            continue;
        }
        gaits.push(Gait {
            node: graph.add_clip(clip.clone(), 1.0, graph.root),
            covers: *covers,
            lasts,
            upto: *upto,
        });
    }
    if gaits.is_empty() {
        warn!("the body carries none of the gaits in GAITS, so the warden will slide");
        commands.remove_resource::<Waiting>();
        return;
    }
    // The fastest gait always catches everything above it, whatever its row said, so
    // that a missing top tier cannot leave a speed with no clip to play.
    if let Some(top) = gaits.last_mut() {
        top.upto = f32::INFINITY;
    }
    // Say what was registered, not only what failed.
    //
    // Everything above this point logs on failure and is silent on success, so a healthy
    // run said nothing at all about its gaits and the only way to believe they loaded was
    // to note the absence of a complaint. Absence of a complaint is not evidence: a clip
    // renamed in dev/art would be reported by one info line among hundreds, and a gait
    // silently covering for a missing one below it looks identical to everything working.
    info!(
        "gaits ready: {}",
        gaits
            .iter()
            .zip(GAITS.iter().map(|(called, _, _)| *called))
            .map(|(gait, called)| format!(
                "{called} {:.3} m/cycle over {:.3} s, up to {:.2} m/s",
                gait.covers, gait.lasts, gait.upto
            ))
            .collect::<Vec<_>>()
            .join("; ")
    );
    commands.insert_resource(Motions {
        graph: graphs.add(graph),
        idle: standing,
        gaits,
        idle_lasts,
    });
    commands.remove_resource::<Waiting>();
}

/// Hands the graph to each player the scene brings in, standing still to begin.
///
/// The glTF loader puts an `AnimationPlayer` on the scene's own animation root, so
/// this waits for one to appear rather than trying to guess which entity it is.
pub fn hand_the_clips_over(
    mut commands: Commands,
    motions: Res<Motions>,
    fresh: Query<Entity, Added<AnimationPlayer>>,
) {
    for entity in &fresh {
        let mut moves = AnimationTransitions::new();
        let mut player = AnimationPlayer::default();
        moves.play(&mut player, motions.idle, std::time::Duration::ZERO);
        commands
            .entity(entity)
            .insert((AnimationGraphHandle(motions.graph.clone()), player, moves));
    }
}

/// Walks when the warden walks, stands when they stand, at their own speed.
pub fn match_the_clip_to_the_walking(
    motions: Res<Motions>,
    striding: Query<&Striding, With<Player>>,
    mut players: Query<(&mut AnimationPlayer, &mut AnimationTransitions)>,
) {
    let Ok(pace) = striding.single() else {
        return;
    };
    // Both of these read `wants`, the ASKED speed, and not the measured one. Measured
    // speed is noisy enough to sit either side of a handover ceiling on consecutive
    // frames - it counted terrain climb as ground speed until this was fixed - and every
    // crossing restarted a blend, which is what made the warden jitter while running.
    // `wants` is one of three constants, so the choice is stable by construction. The
    // measured speed still sets the RATE below, which is the thing it is actually good
    // for. Choose from intent, scale by measurement.
    let moving = pace.wants > 0.05;
    // The slowest gait whose ceiling this speed is still under. The list is ordered
    // and its last entry catches everything, so this always finds one.
    let gait = motions
        .gaits
        .iter()
        .find(|gait| pace.wants <= gait.upto)
        .unwrap_or_else(|| motions.gaits.last().expect("never empty"));

    for (mut player, mut moves) in &mut players {
        let (wanted, covers, lasts) = if moving {
            (gait.node, gait.covers, gait.lasts)
        } else {
            (motions.idle, 0.0, motions.idle_lasts)
        };
        if !player.is_playing_animation(wanted) {
            moves
                .play(&mut player, wanted, std::time::Duration::from_secs_f32(BLEND))
                .repeat();
        }
        if moving {
            // Strides a second is `speed / covers`: a clip is one stride long, so
            // that is how many of them the warden needs, and each gait carries its
            // own distance. Multiplied by the clip's LENGTH because `set_speed` is a
            // multiple of its natural rate rather than a rate — see `Motions`.
            if let Some(active) = player.animation_mut(wanted) {
                active.set_speed(playback_rate(pace.speed, covers, lasts));
            }
        }
    }
}

/// Whether the clips have been found yet.
pub fn the_clips_are_ready(motions: Option<Res<Motions>>) -> bool {
    motions.is_some()
}

/// And whether they are still being waited for.
pub fn still_waiting(waiting: Option<Res<Waiting>>) -> bool {
    waiting.is_some()
}

#[cfg(test)]
mod pacing {
    use super::*;

    /// The walk clip has to be reachable, and so does the run.
    ///
    /// The threshold sat below walking speed once, so every step the warden took
    /// played the RUN clip — at five cycles a second, which is a blur. Nothing
    /// errored and the walk clip sat in the file unused, so every judgement about
    /// how the gaits looked was made about the wrong clip.
    #[test]
    fn the_gait_threshold_lies_between_the_speeds() {
        let walk = crate::player::WALK_SPEED;
        let run = crate::player::SPRINT_SPEED;
        assert!(
            walk < BREAKS_INTO_A_RUN,
            "walking at {walk} m/s is already over the {BREAKS_INTO_A_RUN} m/s              threshold, so the walk clip can never play"
        );
        assert!(
            BREAKS_INTO_A_RUN < run,
            "the run threshold {BREAKS_INTO_A_RUN} is above the top speed {run},              so the run clip can never play"
        );
    }

    /// And the clips play at a believable cadence at those speeds.
    ///
    /// # Measured through the clip's own length, not around it
    ///
    /// This test used to assert on `speed / covers` — the number the game ASKS for —
    /// and passed while the run played 41% too fast, because `set_speed` is a
    /// multiple of a clip's natural rate and the run is authored over sixteen frames
    /// rather than twenty-four. Checking the request rather than the result is how a
    /// test agrees with the code about something they are both wrong about.
    ///
    /// So the durations come out of the FILE, and the cadence asserted here is the
    /// one the animation player will produce.
    #[test]
    fn neither_gait_plays_at_a_blur() {
        let file = std::fs::read("assets/models/person_ranger.glb")
            .expect("the ranger's own file, which the game loads");
        let model = crate::models::inspect(&file).expect("a readable GLB");
        let lasts = |gait: &str| -> f32 {
            model
                .clips
                .iter()
                .find(|(name, _)| name.to_lowercase().contains(gait))
                .map(|(_, seconds)| *seconds)
                .unwrap_or_else(|| panic!("no {gait} clip in {:?}", model.clips))
        };

        // In steps a minute, because that is the unit the evidence is in and "cycles
        // a second" hid how bad the run once was. Two steps to a cycle.
        //
        // What the player will actually produce: the rate handed to `set_speed`,
        // divided by the clip's own length, because that rate is a multiple of one
        // cycle over that length.
        let steps = |speed: f32, covers: f32, gait: &str| -> f32 {
            playback_rate(speed, covers, lasts(gait)) / lasts(gait) * 120.0
        };

        // # The debt this test used to carry is paid
        //
        // It held a ratchet at 300 steps a minute with a note explaining why: the run
        // clip kept a foot down for no frames at all, its stride was about 1.5x too
        // short for the speed it carried, and it was also being asked to carry the
        // SPRINT, because there was no sprint clip. The cadence was whatever was left
        // over — 298 — and the bound had been written loose enough to admit it.
        //
        // There is a sprint clip now, authored with two stance poses out of eight and
        // a real flight phase, and each tier plays its own clip at very nearly that
        // clip's native rate. So the bands below are the ones people actually walk and
        // run at, with no allowance for a fault.
        // Each named speed against the clip the table selects for it. A previous note
        // here said the jog rides the SPRINT clip because the run's ceiling fell below
        // the default pace - that was true of a handover derived from `covers x FPS /
        // frames`, and it is not any more: the ceiling comes from the tier's design
        // speed now, so the jog rides the run clip as its name suggests.
        //
        // # The bands are SCALED, and that is not a loosening
        //
        // Cadence does not transfer between bodies of different size. For dynamic
        // similarity it goes as 1/sqrt(leg), and this leg is 78.35 cm against a human
        // 88.9 - a factor of 1.065. A band lifted straight off human data is therefore
        // the wrong band for this character, and it is wrong in the direction that
        // makes a correct clip look like a fault.
        //
        // The sprint's was also the wrong KIND of band: 170-215 is a running cadence,
        // and sprinting is 220-260. Held to a run's band, a sprint reads as churning
        // at any speed worth having.
        for (what, speed, covers, gait, tier) in [
            ("walk", crate::player::WALK_SPEED, WALK_COVERS, "walk", 0),
            ("jog", crate::player::JOG_SPEED, RUN_COVERS, "run", 1),
            ("sprint", crate::player::SPRINT_SPEED, SPRINT_COVERS, "sprint", 2),
        ] {
            let band = (
                CHURNS_BETWEEN[tier].0 * LEGS_SHORTER_BY,
                CHURNS_BETWEEN[tier].1 * LEGS_SHORTER_BY,
            );
            let cadence = steps(speed, covers, gait);
            assert!(
                (band.0..=band.1).contains(&cadence),
                "the {what} plays at {cadence:.0} steps a minute at {speed} m/s, and \
                 {} to {} is what a person does. Fix the STRIDE rather than this \
                 bound: cadence is speed divided by how far a cycle carries them.",
                band.0,
                band.1
            );
        }
    }

    /// The gait table has to be ordered, positive, and open at the top.
    ///
    /// Three properties, each of which breaks the selection silently rather than
    /// loudly if it is violated — which is exactly why they are asserted rather than
    /// trusted. The selection takes the FIRST gait whose ceiling the speed is under,
    /// so an unordered table makes a fast gait shadow a slow one; a `covers` of nought
    /// divides the cadence into infinity; and a finite top ceiling leaves the speeds
    /// above it with no clip.
    #[test]
    fn the_gait_table_is_ordered_and_open_at_the_top() {
        assert!(!GAITS.is_empty(), "there has to be at least one moving gait");
        let mut previous = 0.0f32;
        for (called, covers, upto) in GAITS {
            assert!(
                *covers > 0.0,
                "the {called} gait covers {covers} m a cycle, and a cadence cannot be                  divided by that"
            );
            assert!(
                *upto > previous,
                "the {called} gait's ceiling {upto} is not above the one before it                  ({previous}), so it can never be chosen: the selection takes the first                  gait the speed fits under"
            );
            previous = *upto;
        }
        let (top, _, ceiling) = GAITS[GAITS.len() - 1];
        assert!(
            ceiling.is_infinite(),
            "the fastest gait is {top} and its ceiling is {ceiling}, so nothing plays              above that speed"
        );
    }

    /// Every handover sits clear of the two speeds it separates.
    ///
    /// This used to assert that a gait does not churn at its own CEILING, which was the
    /// right invariant while the selection read a measured speed: a tier could then be
    /// asked to carry anything up to its ceiling, so the ceiling was the worst case. Under
    /// `Striding::wants` each tier carries exactly ONE speed, so that version was measuring
    /// a case the game cannot produce - the same fault its own comment warned about two
    /// tests down. Cadence at the driven speeds is covered by
    /// `the_gaits_churn_like_a_person_at_the_speeds_they_are_driven`.
    ///
    /// What a ceiling does now is pick a clip from an intent, so what can go wrong is one
    /// landing on the wrong side of a speed, which silently plays a walk at sprint pace or
    /// skips a tier. Margin is demanded rather than mere ordering, because a ceiling a
    /// fraction under a driven speed is exactly what bit last time: `JOG_SPEED` 2.81
    /// against a 2.83 ceiling, close enough that a noisy measured speed crossed it every
    /// frame on a slope and restarted a blend each time.
    #[test]
    fn every_handover_separates_the_speeds_it_sits_between() {
        let driven = [
            crate::player::WALK_SPEED,
            crate::player::JOG_SPEED,
            crate::player::SPRINT_SPEED,
        ];
        for (tier, (called, _, upto)) in GAITS.iter().enumerate() {
            if upto.is_infinite() {
                continue;
            }
            let (mine, next) = (driven[tier], driven[tier + 1]);
            let clear = (next - mine) * 0.2;
            assert!(
                *upto > mine + clear && *upto < next - clear,
                "the {called} tier hands over at {upto} m/s, which is not clear of the \
                 {mine} m/s it plays at or the {next} m/s above it. A handover within a \
                 fifth of either speed is close enough to select the wrong clip."
            );
        }
    }

    /// The cadence must not depend on how long the clip was authored.
    ///
    /// This is the test that would have caught the run playing 41% too fast, and the
    /// one the old test could not be: it asserts a PROPERTY rather than a value.
    /// `speed / covers` alone passes any check of the number it produces while being
    /// wrong by exactly the clip's duration, so the only way to see the fault is to
    /// vary the duration and demand the answer stay put.
    #[test]
    fn the_cadence_does_not_care_how_long_the_clip_is() {
        let covers = WALK_COVERS;
        let speed = crate::player::WALK_SPEED;
        let wanted = speed / covers;
        for lasts in [0.25, 0.5, 0.708, 1.0, 1.042, 2.0, 7.5] {
            let plays_at = playback_rate(speed, covers, lasts) / lasts;
            assert!(
                (plays_at - wanted).abs() < 1e-4,
                "a clip authored over {lasts} s plays at {plays_at:.4} cycles a second                  where {wanted:.4} was wanted — the cadence is following the clip's                  length instead of the warden's speed"
            );
        }
    }

    /// And a clip's length is not wildly off one stride's worth of time.
    ///
    /// The cadence fix makes any duration work, which removes the pressure to author
    /// clips at a sensible length — and a clip stretched far from the speed it plays
    /// at loses its keys' spacing to the resampling. So the authored length is
    /// checked against the time a stride actually takes.
    #[test]
    /// The declared frame counts must match the clips the exporter actually wrote.
    ///
    /// `FPS` and the `*_FRAMES` constants exist to state how each clip is authored, and
    /// nothing was checking them against the file. They went stale exactly that way: the
    /// run was re-authored from sixteen frames to twenty-four in `dev/art`, and until this
    /// was noticed the game still believed sixteen - which made its native rate 2.44 m/s
    /// instead of 1.575 and put its handover ceiling above JOG_SPEED when it should have
    /// been below. A constant describing a file, with nothing comparing the two, is a
    /// comment that compiles.
    fn the_declared_frame_counts_match_the_clips() {
        let file = std::fs::read("assets/models/person_ranger.glb")
            .expect("the ranger's own file, which the game loads");
        let model = crate::models::inspect(&file).expect("a readable GLB");
        for (gait, frames) in [
            ("walk", WALK_FRAMES),
            ("run", RUN_FRAMES),
            ("sprint", SPRINT_FRAMES),
        ] {
            let lasts = model
                .clips
                .iter()
                .find(|(name, _)| name.to_lowercase().contains(gait))
                .map(|(_, seconds)| *seconds)
                .unwrap_or_else(|| panic!("no {gait} clip in {:?}", model.clips));
            // A cycle of `frames` is written with a closing seam key, so the exported
            // duration runs to `frames + 1`. Tolerance of one frame either way rather
            // than an exact match, because that off-by-one is the exporter's business and
            // not what this is guarding.
            let expected = (frames + 1.0) / FPS;
            assert!(
                (lasts - expected).abs() <= 1.0 / FPS,
                "the {gait} clip lasts {lasts:.3} s but {frames} frames at {FPS} fps                  should be {expected:.3} s. Either dev/art re-authored it and this                  constant was not updated, or the reverse - and the native rate                  {rate:.3} m/s that everything else is derived from is wrong either way",
                rate = natively_carries(
                    match gait {
                        "walk" => WALK_COVERS,
                        "run" => RUN_COVERS,
                        _ => SPRINT_COVERS,
                    },
                    frames
                ),
            );
        }
    }

    #[test]
    fn each_clip_is_authored_near_the_time_its_stride_takes() {
        let file = std::fs::read("assets/models/person_ranger.glb")
            .expect("the ranger's own file, which the game loads");
        let model = crate::models::inspect(&file).expect("a readable GLB");
        // Each clip against the speed IT is driven at. This paired the run clip with
        // SPRINT_SPEED, which made sense when walk and run were the only clips and the
        // run had to cover everything above the walk. There is a sprint clip now, so that
        // pairing was measuring a case the game no longer produces - and it only passed
        // before because the run was sixteen frames rather than twenty-four.
        for (gait, speed, covers, tier) in [
            ("walk", crate::player::WALK_SPEED, WALK_COVERS, 0),
            ("run", crate::player::JOG_SPEED, RUN_COVERS, 1),
            ("sprint", crate::player::SPRINT_SPEED, SPRINT_COVERS, 2),
        ] {
            let lasts = model
                .clips
                .iter()
                .find(|(name, _)| name.to_lowercase().contains(gait))
                .map(|(_, seconds)| *seconds)
                .unwrap_or_else(|| panic!("no {gait} clip in {:?}", model.clips));
            let a_stride_takes = covers / speed;
            let stretch = lasts / a_stride_takes;
            // The bound is DERIVED from CHURNS_BETWEEN rather than written out, because
            // `stretch` is `lasts * cadence / 120` - so the `(0.4..2.5)` that used to sit
            // here was a third copy of the cadence bound wearing a different constant
            // (2.5 x 120 = 300 steps a minute), and it refused the sprint the day the band
            // moved. Same argument as `hands_over_above`: make the guards agree by
            // construction, not by a coincidence of factors.
            //
            // What it still catches on its own is a re-authored FRAME COUNT: `lasts` moves
            // with it, so an absurd playback multiple is caught even while the cadence is
            // in band. That matters because every authored sub-motion - the arm swing, the
            // head bob, the chest twist - speeds up with the multiple.
            let allows = (
                lasts * CHURNS_BETWEEN[tier].0 * LEGS_SHORTER_BY / 120.0,
                lasts * CHURNS_BETWEEN[tier].1 * LEGS_SHORTER_BY / 120.0,
            );
            assert!(
                (allows.0..=allows.1).contains(&stretch),
                "the {gait} clip is authored over {lasts:.3} s and a stride takes                  {a_stride_takes:.3} s, so it plays at {stretch:.2}x its own rate, outside                  the {:.2}x-{:.2}x its cadence band allows",
                allows.0,
                allows.1
            );
        }
    }
}
