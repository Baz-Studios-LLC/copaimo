//! Two-bone inverse kinematics: given a hip, a knee and an ankle, put the ankle somewhere.
//!
//! This is the foundation two separate things need, which is why it comes first and why it is
//! a plain function over points rather than anything to do with bones or entities.
//!
//! **Foot planting.** The clips are authored against a flat floor at z=0 and the world is a
//! heightmap, so a planted foot currently meets the ground only where the ground happens to be
//! at zero. Raycast down, put the ankle on what you hit, solve the leg to reach it.
//!
//! **Stride warping.** The run leans on `motion::playback_rate` for 1.54x and the sprint for
//! 1.87x, and the standard answer is to change the STRIDE rather than the tempo — Epic scaled
//! Paragon's motion up to 60% by warping stride length against 15% by play rate. Warping a
//! stride means moving the foot targets apart and letting the legs follow, which is this.
//!
//! See `docs/animation.md`.
//!
//! # Measured on this rig, and both numbers matter
//!
//! Re-measured 2026-08-25 for the character delivered as `assets/character/*.glb`. The figures
//! before were thigh 42.00 cm and calf 36.34 cm and belonged to a rig deleted on 2026-08-24,
//! along with the `dev/art/ik_gait.py` this file used to cite for five of its constants. A reach
//! budget quoted off a skeleton that no longer exists is the same fault as a `covers` that no
//! longer describes its clip.
//!
//! `dev/art/audit_character.py::the_legs` measures these off the shipped `.glb` and refuses if
//! they have moved, so they are a checked record rather than a comment:
//!
//!     left    thigh 38.69   calf 37.64   straight 76.33   standing 76.22 cm  (99.9%)
//!     right   thigh 36.91   calf 40.59   straight 77.50   standing 77.16 cm  (99.6%)
//!
//! **The legs are not symmetric** — 1.17 cm apart in straight length, and the left is thigh-long
//! where the right is calf-long. Nothing here may assume otherwise: [`Chain`] carries each leg's
//! own segment lengths, measured from the bones, and the only shared numbers are ratios.
//!
//! Two consequences, and the first one is the whole reason this takes an argument it looks like
//! it should not need:
//!
//! **THE BEND DIRECTION CANNOT BE READ OFF THE CURRENT POSE.** With the knee 0.09% off the
//! hip-to-ankle line, the vector from that line to the knee is a rounding error pointing
//! nowhere in particular. A solver that infers "which way does this knee fold" from the pose it
//! is given will infer it from noise, and on a straight leg it will sometimes fold the knee
//! backwards. So `bends_toward` is passed in, and for a leg it is the body's forward.
//!
//! **A straight leg is fine HERE, and that is a change.** This said a leg must never be asked to
//! go straight, on a measurement from `dev/art/ik_gait.py`: a leg at 99.3% "cannot be solved:
//! there is no bend for the solver to work with and it fails to track at all", so it capped at
//! 98%. That was true of THAT solver and is not true of this one, for two reasons that are both
//! visible in [`reach`] above:
//!
//! * The singularity is in INFERRING the bend direction, and this solver does not infer it —
//!   `bends_toward` is an argument, for exactly the reason in the paragraph before this one.
//! * There is no `acos`. The knee comes from the law of cosines with `off` guarded by
//!   `.max(0.0).sqrt()`, so at full extension `off` is cleanly zero rather than a NaN.
//!
//! And the cap was costing something real. This character's clips put the stance leg at **100.0%
//! of straight** through the idle and the walk, so a 98% cap could not reach the ankle its own
//! animation had authored: the target came up 1.4 cm short every frame, the hips dropped to
//! rescue it, and the warden stood 1.4 cm lower the instant planting switched on. A test was
//! named `flat_ground_asks_for_nothing_but_the_extension_cap` and asserted that sink, which is
//! what a guard looks like once it has been taught to accept the thing it was meant to catch.
//!
//! So [`EXTENDS_AT_MOST`] is **0.999**, not 0.98 and not 1.0, and its own doc comment carries the
//! table that picked it. 1.0 exactly is stable and finite but leaves the knee exactly on the
//! hip-ankle line, with no bend to read or to test; 0.999 leaves a 1.71 cm knee offset for 0.08 cm
//! of reach, because the offset grows as the SQUARE ROOT of the reach given up. The singularity is
//! real. It is just far smaller than two percent.
//!
//! The IK corrects for TERRAIN; it does not restyle the pose. Soft knees, if they are wanted, are
//! a clip edit like `motion::move_the_arms_more` — an authored choice, not something a correction
//! layer imposes on every frame.
//!
//! # What it does when it cannot reach
//!
//! It clamps and the foot lands short, along the line it was asked for. Deliberately, and
//! stated because the authoring side learned it the hard way: "an unreachable target does not
//! fail loudly, it clamps, and the foot lands short". Callers that care whether the target was
//! met should compare the returned `end` against what they asked for — [`Chain::missed_by`].

use bevy::math::Affine3A;
use bevy::prelude::*;
use bevy::transform::TransformSystem;

use crate::player::Player;
use crate::world::terrain::TerrainSource;

/// How straight a leg may be asked to go, as a share of hip-to-ankle at full extension.
///
/// 0.999, and the third nine is doing real work. What the cap costs and what it buys, on this
/// skeleton's 38.69 + 37.64 cm left leg:
///
///     cap      knee off the hip-ankle line     ankle short of straight
///     1.000                 0.00 cm                    0.00 cm
///     0.9999                0.54 cm                    0.01 cm
///     0.999                 1.71 cm                    0.08 cm
///     0.995                 3.81 cm                    0.38 cm
///     0.98                  7.59 cm                    1.53 cm
///
/// The relationship is steeply nonlinear, which is what makes a good answer available at all: a
/// knee offset grows as the SQUARE ROOT of the reach given up. At 0.999 the knee sits 1.71 cm off
/// the line - a readable soft knee, and enough for `a_knee_folds_forward_whichever_way_the_warden
/// _faces` to have something to measure - for 0.08 cm of reach, which is under a millimetre.
///
/// 1.0 exactly does not work, and the reason is worth keeping. It does not produce a NaN: [`reach`]
/// guards its `sqrt` with `.max(0.0)` and takes the bend direction as an argument, so a fully
/// straight leg solves cleanly and stably. But `off` is then EXACTLY zero, the knee sits on the
/// hip-ankle line, and the leg has no bend to read - visually a locked knee, and untestable. The
/// singularity that justified the old 0.98 is real; it is just far smaller than 2%.
///
/// It was 0.98, inherited from `dev/art/ik_gait.py`, which no longer exists. On this character
/// that cost 1.53 cm of unauthored crouch on flat ground - the clips put the stance leg at 100.0%
/// of straight, so a 98% cap could not reach the ankle the animation had authored and the hips
/// dropped every frame to rescue it. 0.999 is 19x less.
pub const EXTENDS_AT_MOST: f32 = 0.999;

/// How far a leg may fold, as a share of full extension — the ankle may not come closer to the
/// hip than this.
///
/// 0.27 is about where a knee stops: 150 degrees of flexion, which by the cosine rule on THIS
/// skeleton's segments puts the ankle 19.78 cm from the hip on the left and 20.37 on the right -
/// 25.9% and 26.3% of straight. 0.27 sits just above both, which is the right side to be on for
/// a rail against a nonsense target rather than a pose limit.
///
/// Re-derived rather than re-used: the old note computed it off a 42.00/36.34 cm leg that no
/// longer exists and landed on the same 0.27 by luck. The deepest fold any delivered clip asks
/// for is 50% of straight, in the run, so nothing comes near this.
pub const FOLDS_AT_MOST: f32 = 0.27;

/// Below this a vector has no usable direction and asking for one gives a NaN.
const NO_DIRECTION: f32 = 1e-6;

/// A two-bone chain, as the three points that define it.
///
/// Points rather than rotations on purpose. Rotations need to know each bone's local axis
/// convention, which is a property of how the model was exported and not of the geometry; the
/// arithmetic below is the same whatever that convention is, and it is testable without a rig.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Chain {
    /// The hip.
    pub root: Vec3,
    /// The knee.
    pub joint: Vec3,
    /// The ankle — the end that is being placed.
    pub end: Vec3,
}

impl Chain {
    /// Thigh length.
    pub fn upper(&self) -> f32 {
        self.joint.distance(self.root)
    }

    /// Calf length.
    pub fn lower(&self) -> f32 {
        self.end.distance(self.joint)
    }

    /// Hip to ankle with the knee straight.
    ///
    // Used by the tests and by nothing in the shipped build, which is why it reads as dead.
    // Kept because they are how the module's own claims are checked - the extension cap, the
    // fold limit, and whether a target was actually met - and a measurement that only exists
    // inside an assertion is a measurement nobody can take by hand when something looks wrong.
    #[allow(dead_code)]
    pub fn straight(&self) -> f32 {
        self.upper() + self.lower()
    }

    /// Hip to ankle as it currently stands, as a share of straight.
    ///
    /// 0.999 in this rig's bind pose, which is the measurement that shapes this whole module.
    #[allow(dead_code)]
    pub fn extension(&self) -> f32 {
        let straight = self.straight();
        if straight < NO_DIRECTION {
            return 0.0;
        }
        self.end.distance(self.root) / straight
    }

    /// How far the ankle ended up from where it was asked to go.
    ///
    /// Non-zero means the target was out of reach and the leg clamped. Worth checking rather
    /// than assuming, because clamping is silent.
    #[allow(dead_code)]
    pub fn missed_by(&self, target: Vec3) -> f32 {
        self.end.distance(target)
    }
}

/// Solves the chain so its `end` reaches `target`, keeping both bone lengths.
///
/// `bends_toward` is a world-space hint for which way the joint folds — for a leg, the body's
/// forward. Only its component across the hip-to-target line is used, so it does not need to be
/// perpendicular to anything, and it must not be parallel to that line (see the note on the
/// module: it is passed in precisely because the pose cannot supply it).
///
/// `root` never moves. Placing the hips is the caller's business, and on this rig it matters:
/// with the sole planted, hip height IS the leg's vertical extent and nothing else.
pub fn reach(chain: Chain, target: Vec3, bends_toward: Vec3) -> Chain {
    let upper = chain.upper();
    let lower = chain.lower();
    let straight = upper + lower;
    if straight < NO_DIRECTION {
        return chain;
    }

    // Where to aim. If the target sits on the hip there is no direction to be had, so keep
    // pointing where the leg already points; if THAT is degenerate too, give up and hold the
    // pose rather than return a NaN that will spread through the transform hierarchy.
    let to_target = target - chain.root;
    let axis = if to_target.length() > NO_DIRECTION {
        to_target.normalize()
    } else {
        let held = chain.end - chain.root;
        if held.length() > NO_DIRECTION {
            held.normalize()
        } else {
            return chain;
        }
    };

    // How far along that line the ankle may actually go. Both ends are real limits, not
    // tolerances: too straight is singular, too folded is past a knee.
    let asked = to_target.length();
    let span = (straight * FOLDS_AT_MOST).max((upper - lower).abs() + NO_DIRECTION)
        ..=(straight * EXTENDS_AT_MOST);
    let reaches = asked.clamp(*span.start(), *span.end());

    // Law of cosines: how far down the axis the knee sits, and how far off it.
    let along = (reaches * reaches + upper * upper - lower * lower) / (2.0 * reaches);
    let off = (upper * upper - along * along).max(0.0).sqrt();

    // Which way it folds. The hint, with the along-axis part taken out. If the hint IS the
    // axis there is nothing left, so any consistent perpendicular will do — it will look
    // wrong, but it is finite and it does not vary frame to frame, which a normalised
    // rounding error would.
    let across = bends_toward - axis * bends_toward.dot(axis);
    let across = if across.length() > NO_DIRECTION {
        across.normalize()
    } else {
        axis.any_orthonormal_vector()
    };

    Chain {
        root: chain.root,
        joint: chain.root + axis * along + across * off,
        end: chain.root + axis * reaches,
    }
}

/// The rotation that turns a bone pointing `was` into one pointing `wants`, in world space.
///
/// Applied on the LEFT of a bone's current world rotation, so a caller does not have to know
/// which local axis the bone runs along — the export's convention drops out. Returns identity
/// rather than a NaN when either direction is degenerate.
/// The same rotation, but no further than `degrees`.
///
/// For laying a foot on the ground: a slope is something a foot follows, a cliff is not, and past
/// some angle a shoe rotated to match the ground reads as a broken ankle rather than as standing
/// on a hill.
pub fn at_most(turn: Quat, degrees: f32) -> Quat {
    let (axis, angle) = turn.to_axis_angle();
    let capped = degrees.to_radians();
    if angle <= capped || !axis.is_finite() {
        return turn;
    }
    Quat::from_axis_angle(axis, capped)
}

/// The rotation that turns a bone pointing `was` into one pointing `wants`, in world space.
pub fn aim(was: Vec3, wants: Vec3) -> Quat {
    if was.length() < NO_DIRECTION || wants.length() < NO_DIRECTION {
        return Quat::IDENTITY;
    }
    Quat::from_rotation_arc(was.normalize(), wants.normalize())
}

// ------------------------------------------------------------------ planting feet on terrain

/// The most a foot will be moved to meet the ground under it, in metres.
///
/// Past this the terrain is a cliff rather than a slope, and a leg stretched over the edge of
/// one looks far worse than a foot hanging in the air above it. The world's ground can drop
/// hundreds of metres between two samples at a coastline.
pub const GROUND_REACHES: f32 = 0.35;

/// The most the hips will drop to let a low foot reach, in metres.
///
/// 0.14 m, anchored to what the clips do on their own. The hip's own vertical travel is 1.73 cm
/// through the idle, 3.70 through the walk and 6.83 through the run, so a correction of twice the
/// largest of those is the point where a drop stops reading as part of the gait and starts
/// reading as a stance change. That is 13.7 cm, and 0.14 is 18% of this leg - inside the 10-20%
/// of leg length that hip-drop budgets are usually given.
///
/// It was 0.20 m, described as "about a quarter of this leg" on a leg that no longer exists.
/// A quarter of 76 cm is a squat, and this only ever rescues a foot the ground has moved away
/// from.
pub const HIPS_DROP_AT_MOST: f32 = 0.14;

/// How far the foot may tilt to lie along the ground, in degrees.
///
/// A foot follows a slope; it does not follow a cliff. Past this the ground under one foot is not
/// something a foot can be flat on, and a shoe rotated to match it reads as a broken ankle.
pub const FOOT_TILTS_AT_MOST: f32 = 30.0;

/// How far either side of a foot to sample the heightfield for its ground normal, in metres.
///
/// A foot's width, MEASURED. In the body's own frame - the warden faces 102.4 degrees off the
/// world axes, so an axis-aligned extent is neither width nor length - the shoe is 33.08 cm long
/// by 17.46 wide on the left and 32.59 by 18.93 on the right. Half of the wider one, rounded, is
/// the sampling radius: it spans the shoe and no more.
///
/// Tighter picks up single-vertex noise in the sculpted edits and makes the shoe twitch; wider
/// averages away the slope the foot is actually standing on. It was 0.12, "about a foot's width",
/// which was narrower than this shoe actually is.
const A_FOOT_WIDE: f32 = 0.09;

/// How fast the hip drop follows the ground, as a share closed per second.
///
/// Not instant. The ground under a foot changes discontinuously - a stair, a rock edge - and
/// snapping the hips to it reads as a twitch, where the legs alone absorbing it reads as
/// stepping. The same shape `player::SPEED_SETTLES` uses, for the same reason.
pub const HIPS_SETTLE: f32 = 12.0;

/// How far the hips must drop so the lower foot can still reach the ground.
///
/// Only ever downward. A foot that needs to go UP just bends its knee more, which a leg can
/// always do; a foot that needs to go DOWN may run out of leg, and that is what a hip drop buys.
/// Taking the minimum rather than the mean on purpose: the mean leaves the lower foot short,
/// which is the failure that reads as one leg not reaching the floor.
pub fn hips_drop_by(shifts: [f32; 2]) -> f32 {
    shifts
        .into_iter()
        .fold(0.0_f32, f32::min)
        .clamp(-HIPS_DROP_AT_MOST, 0.0)
}

/// How far a foot must move vertically to meet the ground under it.
///
/// The clips are authored with the soles on a floor at zero and the warden's transform sits on
/// the ground, so the height the clip gives an ankle is already right for flat ground at the
/// warden's feet. The whole correction is however far the ground under THAT foot differs from
/// the ground under the warden — no measured ankle-to-sole offset needed, which matters because
/// looking one up was the first instinct and it would have been a constant to keep in step with
/// the model forever.
pub fn shift_to_ground(ground: f32, feet_at: f32) -> f32 {
    (ground - feet_at).clamp(-GROUND_REACHES, GROUND_REACHES)
}

/// One leg, before and after being put on the ground.
#[derive(Clone, Copy, Debug)]
pub struct Planted {
    /// The chain as the animation left it, with the hip drop applied.
    pub was: Chain,
    /// Where the solver put it.
    pub now: Chain,
    /// How far this foot was asked to move.
    ///
    // Read by the tests, which assert on it directly rather than inferring it from where the
    // ankle ended up - the two differ whenever the leg clamps, and that difference is the thing
    // worth checking.
    #[allow(dead_code)]
    pub shift: f32,
}

/// Works out where one leg should go. The decision, with no ECS in it, so it can be tested.
///
/// Note the target is built from the ORIGINAL ankle and not the dropped one: the hips going down
/// moves the leg, and the ground stays where it is.
pub fn plant_one(
    hip: Vec3,
    knee: Vec3,
    ankle: Vec3,
    ground: f32,
    feet_at: f32,
    dropped: f32,
    forward: Vec3,
    stride: f32,
) -> Planted {
    let shift = shift_to_ground(ground, feet_at);
    let down = Vec3::Y * dropped;
    let was = Chain {
        root: hip + down,
        joint: knee + down,
        end: ankle + down,
    };
    Planted {
        was,
        now: reach(was, warped_target(hip, ankle, shift, forward, stride), forward),
        shift,
    }
}

/// Where a foot goes once the stride has been warped and the ground accounted for.
///
/// # What warping a stride actually is
///
/// The foot's offset from the hip ALONG THE LINE OF TRAVEL is scaled; everything else about it is
/// left alone. So a foot 30 cm in front of the hip goes to 37.5 cm at 1.25x and a foot behind goes
/// further behind, which widens the whole step without touching its timing. `motion.rs` then
/// divides the playback rate by the same factor, because a cycle now carries `covers x stride`.
///
/// Horizontal only. Scaling the vertical part as well would raise a swinging foot by a quarter
/// and drop a planted one through the floor, and the ground has already had its say via `shift`.
pub fn warped_target(hip: Vec3, ankle: Vec3, shift: f32, forward: Vec3, stride: f32) -> Vec3 {
    let flat = Vec3::new(forward.x, 0.0, forward.z);
    let on_the_ground = ankle + Vec3::Y * shift;
    if flat.length() < NO_DIRECTION || stride <= 1.0 {
        return on_the_ground;
    }
    let flat = flat.normalize();
    let along = (ankle - hip).dot(flat);
    on_the_ground + flat * (along * (stride - 1.0))
}

/// How far the hips must drop for a foot to reach `target` at all, or 0 if it already can.
///
/// This is the price of a wider stride and it is not a tuning number: a planted foot `ahead` of
/// the hip pins the hip to `sqrt(reach^2 - ahead^2)` above the ankle. `ik::the_reach_budget`
/// prints the exchange rate on this leg - the run's authored contact already wants about 7 cm,
/// and 1.6x would want 17, which is why `motion::STRIDE_WARPS_TO` is 1.25 and not Paragon's 1.6.
///
/// Derived rather than a constant, so the crouch is exactly what the stride asked for. A fixed
/// number would either squat when it did not need to or come up short when it did.
pub fn stride_needs_a_drop(hip: Vec3, target: Vec3, reach: f32, forward: Vec3) -> f32 {
    let flat = Vec3::new(forward.x, 0.0, forward.z);
    if flat.length() < NO_DIRECTION {
        return 0.0;
    }
    let ahead = (target - hip).dot(flat.normalize()).abs();
    if ahead >= reach {
        return -HIPS_DROP_AT_MOST;
    }
    let allowed = (reach * reach - ahead * ahead).sqrt();
    ((hip.y - target.y) - allowed).max(0.0).min(HIPS_DROP_AT_MOST) * -1.0
}

/// What was last done to one bone, so it can be undone if nothing else has written it since.
///
/// # Why this is needed at all
///
/// The correction is computed from the pose the bone is currently in. If that pose is one this
/// system already corrected, the next correction stacks on top of it, and the frame after that
/// stacks again — measured, a leg walked 57 cm out of place over 40 frames.
///
/// In the game that never happens, because every shipped clip keys every bone on every frame
/// (`animate_ranger.key` does so deliberately, and its note says why) so the authored pose is
/// restored before this runs. But that is a hidden coupling: the day something plays no clip —
/// a state that does not animate, or the frames before the clips finish loading — the legs
/// spiral. Relying on somebody else's invariant to stay correct is how the accumulating hip
/// offset nearly shipped in this same file.
///
/// So: remember what was written, and if the bone still holds exactly that, put the authored
/// pose back before solving. If it holds something else, an animation has been through and that
/// something else IS the authored pose.
#[derive(Clone, Copy, Debug, Default)]
pub struct Held {
    /// What the animation had put there, before the correction.
    authored: Quat,
    /// What was left behind after it.
    left: Quat,
}

impl Held {
    /// The pose to solve from.
    fn base(&self, now: Quat) -> Quat {
        if now.abs_diff_eq(self.left, 1e-5) {
            self.authored
        } else {
            now
        }
    }
}

/// The leg bones of one side, once they have been found in the scene.
#[derive(Clone, Copy, Debug)]
pub struct Leg {
    /// Its head is the hip.
    pub thigh: Entity,
    /// Its head is the knee.
    pub calf: Entity,
    /// Its head is the ankle — the point being placed.
    pub foot: Entity,
    /// What was last written to the thigh, the calf and the foot, in that order.
    held: [Held; 3],
}

/// Both legs, and the bone the whole body hangs from.
///
/// Found once and kept. A glTF scene arrives over several frames, so this cannot be keyed on
/// `Added` — see `look::paint_the_warden`, which asks and asks again for the same reason.
#[derive(Component, Clone, Copy, Debug)]
pub struct Legs {
    pub left: Leg,
    pub right: Leg,
    /// The scene root under the warden — the entity carrying the model's scale and turn.
    ///
    /// The body drop goes HERE and not on the `Hip` bone, which was the first instinct because
    /// the clips already key its translation. That is exactly why it is wrong: adding to a
    /// channel the animation owns only works while the animation is overwriting it, and the
    /// moment nothing drives `Hip` — no clip loaded yet, or a state that does not animate — the
    /// offset accumulates and the warden sinks a little further every frame.
    ///
    /// Nothing else writes this entity, so the drop is SET rather than added, which cannot
    /// accumulate however many times it runs.
    pub body: Entity,
    /// How far the body is currently dropped, eased toward what the ground asks for.
    pub dropped: f32,
}

/// Where a bone actually is this frame, composed by hand up its parent chain.
///
/// `GlobalTransform` is no use here. This runs between the animation writing local transforms
/// and Bevy propagating them, which is the only slot where a correction can land without a
/// frame of lag — and in that slot every `GlobalTransform` still holds last frame's answer.
/// Reading one would be a lag that shows as the feet swimming behind the body.
///
/// The walk stops at the warden, whose own transform is passed in rather than queried, because a
/// query that could hand out the warden's `Transform` mutably cannot coexist with one that reads
/// it here.
fn world_of(
    entity: Entity,
    warden: Affine3A,
    placed: &Query<(&Transform, Option<&ChildOf>), Without<Player>>,
) -> Option<Affine3A> {
    let mut chain = Vec::new();
    let mut at = entity;
    loop {
        let Ok((local, parent)) = placed.get(at) else {
            break;
        };
        chain.push(local.compute_affine());
        match parent {
            Some(above) => at = above.parent(),
            None => break,
        }
    }
    if chain.is_empty() {
        return None;
    }
    let mut world = warden;
    for local in chain.iter().rev() {
        world *= *local;
    }
    Some(world)
}

/// Finds the leg bones in the warden's scene, once it has arrived.
///
/// By NAME, and only among this warden's own descendants — the world is full of other named
/// things, and another figure's skeleton would put this one's feet on somebody else's legs. The
/// same ancestry walk `look::hang_things_on_the_head` does, for the same reason.
///
/// Asks every frame until it finds them, rather than keying on `Added`: a glTF scene is instanced
/// asynchronously and none of its entities exist at spawn.
pub fn find_the_legs(
    mut commands: Commands,
    wardens: Query<Entity, (With<Player>, Without<Legs>)>,
    named: Query<(Entity, &Name)>,
    ancestors: Query<&ChildOf>,
) {
    let Ok(warden) = wardens.single() else {
        return;
    };
    let ours = |mut at: Entity| loop {
        if at == warden {
            return true;
        }
        match ancestors.get(at) {
            Ok(above) => at = above.parent(),
            Err(_) => return false,
        }
    };
    let bone = |wanted: &str| {
        named
            .iter()
            .find(|(entity, name)| name.as_str() == wanted && ours(*entity))
            .map(|(entity, _)| entity)
    };
    let leg = |side: &str| {
        Some(Leg {
            thigh: bone(&format!("{side}_Thigh"))?,
            calf: bone(&format!("{side}_Calf"))?,
            foot: bone(&format!("{side}_Foot"))?,
            held: Default::default(),
        })
    };
    let (Some(left), Some(right)) = (leg("L"), leg("R")) else {
        return;
    };
    // The scene root: walk up from a leg until the next step would be the warden itself. Found
    // rather than assumed, because it is whatever glTF instancing put between them.
    let mut body = left.thigh;
    loop {
        let Ok(above) = ancestors.get(body) else {
            return;
        };
        if above.parent() == warden {
            break;
        }
        body = above.parent();
    }
    info!("found the warden's legs; feet will follow the ground");
    commands.entity(warden).insert(Legs {
        left,
        right,
        body,
        dropped: 0.0,
    });
}

/// Puts each foot on the ground under it, and drops the hips so the lower one can reach.
///
/// # Why a vertical shift is the whole correction
///
/// The clips are authored with the soles on a floor at zero, and the warden's own transform sits
/// on the ground. So the height the clip gives an ankle is already right for flat ground at the
/// warden's feet, and the entire correction is however far the ground under THAT foot differs
/// from the ground under the warden. No measured offset between ankle and sole is needed, which
/// is worth saying because looking one up was the first instinct and it would have been a
/// constant to keep in step with the model forever.
///
/// A SWINGING foot is shifted too, deliberately. Ground 20 cm higher under a foot in flight is
/// ground that foot has to clear, and lifting the whole trajectory is what a real leg does. Only
/// pinning a planted foot against sliding needs to know plant from swing, and that is a separate
/// thing this does not yet do.
pub fn plant_the_feet(
    time: Res<Time>,
    terrain: Option<Res<TerrainSource>>,
    mut wardens: Query<(&Transform, &mut Legs, Option<&crate::motion::Warping>), With<Player>>,
    mut placed: Query<(&mut Transform, Option<&ChildOf>), Without<Player>>,
) {
    let Some(terrain) = terrain else {
        return;
    };
    let Ok((standing, mut legs, warping)) = wardens.single_mut() else {
        return;
    };
    // 1.0 until the clips are playing, so nothing warps before there is a gait to warp.
    let stride = warping.map(|w| w.stride).unwrap_or(1.0);
    // The warden's OWN transform, not its GlobalTransform.
    //
    // This runs before Bevy propagates, so every GlobalTransform still holds last frame's
    // answer - and using one here put the whole computation a frame behind the body. At sprint
    // speed that is 10 cm of error in where the ground is, and in the very first frame of a test
    // it is the difference between standing at height 10.6 and standing at zero. The warden is
    // spawned at the top level with no parent, so its Transform IS its world transform.
    let base = standing.compute_affine();
    let feet_at = standing.translation.y;

    // Put the authored pose back before anything is measured. See `Held`: without this the
    // correction is computed from an already-corrected pose and compounds, which measured 57 cm
    // of drift over 40 frames when nothing else was writing the bones.
    for side in [legs.left, legs.right] {
        for (slot, bone) in [side.thigh, side.calf, side.foot].into_iter().enumerate() {
            if let Ok((mut local, _)) = placed.get_mut(bone) {
                local.rotation = side.held[slot].base(local.rotation);
            }
        }
    }

    // Read everything after that. Solving needs each leg's three joints in world space, and
    // writing any of them invalidates the ones below it.
    let read = |leg: &Leg| {
        let placed = &placed.as_readonly();
        let foot = world_of(leg.foot, base, placed)?;
        Some((
            world_of(leg.thigh, base, placed)?.translation.into(),
            world_of(leg.calf, base, placed)?.translation.into(),
            foot.translation.into(),
            // The foot's world ORIENTATION as the animation left it, captured before anything is
            // written. This is what the sole is aimed relative to, and taking it now is what
            // makes the aiming a plain assignment rather than a delta composed out of the two
            // turns above it - see the note where it is used.
            foot.to_scale_rotation_translation().1,
        ))
    };
    let (Some(left), Some(right)) = (read(&legs.left), read(&legs.right)) else {
        return;
    };
    // What is already applied. Everything read above includes last frame's drop, so it comes
    // back off before this frame's goes on - otherwise each frame's drop stacks on the last and
    // the warden walks into the ground.
    let held = placed
        .get(legs.body)
        .map(|(local, _)| local.translation.y)
        .unwrap_or(0.0);
    let undropped = |p: Vec3| p - Vec3::Y * held;
    let sides: [(Leg, (Vec3, Vec3, Vec3, Quat)); 2] = [
        (legs.left, (undropped(left.0), undropped(left.1), undropped(left.2), left.3)),
        (legs.right, (undropped(right.0), undropped(right.1), undropped(right.2), right.3)),
    ];

    // How far each foot has to move to meet the ground it is over.
    let grounds = sides.map(|(_, (_, _, ankle, _))| terrain.height(ankle.x, ankle.z));
    let shifts = [
        shift_to_ground(grounds[0], feet_at),
        shift_to_ground(grounds[1], feet_at),
    ];

    // Forward, from the warden's own facing — never from the pose. On a bind 99.9% extended the
    // knee's offset from the hip-to-ankle line is a rounding error, so a solver left to infer
    // the fold direction would sometimes fold the knee backwards.
    //
    // NEG_Z, and this cost a round of backwards knees in the game. `util::facing_quat` is the
    // authority and says so plainly: the rotation it produces "is applied to a model whose front
    // is -Z". Passing `Vec3::Z` put the pole BEHIND the warden, and a pole behind the knee admits
    // exactly one solution - the knee folding the wrong way. Anatomically the pole belongs in
    // front, which is what the authoring side did too: it put the pole "well
    // in FRONT of the knee, at hip height, so the only solution it admits is a knee pointing
    // forward".
    let forward = standing.rotation * Vec3::NEG_Z;

    // And what the WARPED stride needs, which is the price of the wider step: a foot further in
    // front of the hip can only be on the ground if the hip is lower. Derived per foot from its
    // own target rather than set as a constant, so the crouch is exactly what was asked for.
    let stride_wants = sides.map(|(_, (hip, knee, ankle, _))| {
        let reach = (knee.distance(hip) + ankle.distance(knee)) * EXTENDS_AT_MOST;
        let target = warped_target(hip, ankle, 0.0, forward, stride);
        stride_needs_a_drop(hip, target, reach, forward)
    });

    // Eased, not snapped: the ground under a foot changes discontinuously at a step or a rock
    // edge, and moving the hips there in one frame reads as a twitch.
    //
    // Whichever of the two asks for more, not their sum: the ground and the stride are both
    // reasons the hip must be LOWER, and satisfying the deeper one satisfies the other.
    let wants = hips_drop_by(shifts).min(stride_wants[0]).min(stride_wants[1]);
    let closes = (HIPS_SETTLE * time.delta_secs()).clamp(0.0, 1.0);
    legs.dropped += (wants - legs.dropped) * closes;
    let dropped = legs.dropped;



    for (slot, (leg, (hip, knee, ankle, sole_was))) in sides.into_iter().enumerate() {
        let put = plant_one(
            hip, knee, ankle, grounds[slot], feet_at, dropped, forward, stride,
        );
        // The thigh first, then the calf - and the calf's STARTING direction has to account
        // for the thigh having just moved, because the calf is its child and went with it.
        //
        // Using the untouched chain's calf direction here was wrong and cost a while to find:
        // the thigh turns about 10.8 degrees to bend the knee forward, which swings the calf's
        // far end 6.8 cm before the calf is touched at all, and the ankle came out 3.9 cm low
        // for it. The Blender viewer avoided this by re-reading the joint after each aim; here
        // the thigh's own turn is applied to the direction instead, which needs no re-read.
        let turns_thigh = aim(put.was.joint - put.was.root, put.now.joint - put.now.root);
        let turns_calf = aim(
            turns_thigh * (put.was.end - put.was.joint),
            put.now.end - put.now.joint,
        );
        // # The foot, which is the half of foot IK that was missing
        //
        // `_Foot` is a CHILD of `_Calf`, so every degree the knee bends carries the foot with it
        // - and the correction bends the knee on EVERY frame, because this bind stands at 99.9%
        // extension against a 98% cap. In game that read as the warden standing on his toes with
        // legs that would not bend, which is exactly what a shoe pitched toe-down by its own
        // shin looks like.
        //
        // The research said this and I built half of it: apply two-bone IK to the hip-knee-ankle
        // chain so the foot is at the right HEIGHT, *and* aim the ankle so it is aligned to the
        // ground. So the foot gets what the leg did to it undone, and is then laid along the
        // slope it is standing on.
        //
        // Measured from world +Y rather than from any axis of the bone, because the clip has the
        // sole flat on a level floor - so once the leg's rotation is undone, LEVEL is the foot's
        // own reference and the export's axis convention never enters into it.
        let slope = terrain.normal(ankle.x, ankle.z, A_FOOT_WIDE);
        let lies = at_most(aim(Vec3::Y, slope), FOOT_TILTS_AT_MOST);

        let held = [
            turn_a_bone(&mut placed, base, leg.thigh, turns_thigh),
            turn_a_bone(&mut placed, base, leg.calf, turns_calf),
            point_a_bone(&mut placed, base, leg.foot, lies * sole_was),
        ];
        let mine = if slot == 0 { &mut legs.left } else { &mut legs.right };
        for (into, got) in mine.held.iter_mut().zip(held) {
            if let Some(got) = got {
                *into = got;
            }
        }
    }

    // And drop the body. SET, not added, so it cannot accumulate.
    //
    // The scene root sits directly under the warden, whose own transform carries a turn about Y
    // and no scale - and a turn about Y leaves Y alone. So a world-vertical drop is the same
    // number in this entity's own space, with no conversion to get wrong.
    if let Ok((mut local, _)) = placed.get_mut(legs.body) {
        local.translation.y = dropped;
    }
}

/// Puts one bone at an ABSOLUTE world orientation, whatever its parents have done.
///
/// For the foot, where the wanted orientation is known outright - the sole as the animation
/// authored it, tilted onto the ground - rather than as a change from where it is now.
///
/// The first version composed a delta instead: undo the turns the thigh and calf had just
/// carried the foot through, then apply the tilt. Every part of that was measured and correct in
/// isolation - the carried rotation matched the calf's world rotation to a tenth of a degree -
/// and the sole still came out 36 degrees wrong. Assigning the orientation has no composition in
/// it to be wrong, so there is nothing left to debug: the foot ends up where it was told.
fn point_a_bone(
    placed: &mut Query<(&mut Transform, Option<&ChildOf>), Without<Player>>,
    warden: Affine3A,
    bone: Entity,
    wanted: Quat,
) -> Option<Held> {
    let parent = placed
        .get(bone)
        .ok()
        .and_then(|(_, above)| above.map(|p| p.parent()))?;
    let above = world_of(parent, warden, &placed.as_readonly())?;
    let (_, upright, _) = above.to_scale_rotation_translation();
    let (mut local, _) = placed.get_mut(bone).ok()?;
    let authored = local.rotation;
    // world = parent * local, so the local that yields `wanted` in world is parent-inverse.
    local.rotation = upright.inverse() * wanted;
    Some(Held {
        authored,
        left: local.rotation,
    })
}

/// Turns one bone so the segment it drives points where the solver put it.
///
/// Written as a delta on the bone's WORLD rotation and converted back through its parent, so
/// nothing here needs to know which local axis the bone runs along — that is a property of how
/// the model was exported, not of the geometry.
fn turn_a_bone(
    placed: &mut Query<(&mut Transform, Option<&ChildOf>), Without<Player>>,
    warden: Affine3A,
    bone: Entity,
    turn: Quat,
) -> Option<Held> {
    let parent = placed
        .get(bone)
        .ok()
        .and_then(|(_, above)| above.map(|p| p.parent()))?;
    let above = world_of(parent, warden, &placed.as_readonly())?;
    let (_, upright, _) = above.to_scale_rotation_translation();
    let (mut local, _) = placed.get_mut(bone).ok()?;
    let authored = local.rotation;
    // world = parent * local, so a world-space turn on the left becomes
    // parent_rotation-inverse * turn * parent_rotation applied to the local rotation.
    local.rotation = upright.inverse() * turn * upright * authored;
    Some(Held {
        authored,
        left: local.rotation,
    })
}

/// Feet that follow the ground.
pub struct PlantingPlugin;

impl Plugin for PlantingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, find_the_legs).add_systems(
            PostUpdate,
            // AFTER the animation has written its pose and BEFORE Bevy propagates it. Any
            // earlier and the correction is overwritten; any later and it lands a frame late,
            // which shows as the feet swimming behind the body.
            plant_the_feet
                .after(bevy::app::Animation)
                .before(TransformSystem::TransformPropagate),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hips_only_ever_drop() {
        assert_eq!(hips_drop_by([0.0, 0.0]), 0.0);
        // A foot needing to go up costs nothing: a knee can always bend further.
        assert_eq!(hips_drop_by([0.2, 0.1]), 0.0);
        // A foot needing to go down takes the hips with it.
        assert!((hips_drop_by([-0.05, 0.0]) - -0.05).abs() < 1e-6);
        // The LOWER foot decides, not the average - averaging leaves it short of the floor,
        // which is what reads as a leg not reaching.
        assert!((hips_drop_by([-0.10, 0.10]) - -0.10).abs() < 1e-6);
    }

    #[test]
    fn a_cliff_does_not_pull_the_hips_through_the_floor() {
        assert_eq!(hips_drop_by([-40.0, -40.0]), -HIPS_DROP_AT_MOST);
    }

    /// Flat ground under the foot means no shift asked for, so the only thing that moves the
    /// ankle is the extension cap - the 1.5 cm this rig's 99.9% bind costs. Worth pinning,
    /// because it is the amount the character sinks the instant planting is switched on.
    /// Flat ground asks for nothing at all, to within a fraction of a millimetre.
    ///
    /// This test used to REQUIRE a sink of 1 to 2 cm, and was named
    /// `flat_ground_asks_for_nothing_but_the_extension_cap` - a guard taught to accept the very
    /// thing it exists to catch. The sink was the 0.98 cap failing to reach an ankle the clips had
    /// authored at 100% of straight, so the hips dropped to cover it and the warden stood shorter
    /// the instant planting switched on. At 0.999 the same figure is under a millimetre.
    #[test]
    fn flat_ground_asks_for_nothing() {
        let leg = a_leg();
        let put = plant_one(leg.root, leg.joint, leg.end, 0.0, 0.0, 0.0, FORWARD, 1.0);
        assert_eq!(put.shift, 0.0);
        let sank = (put.now.end.y - leg.end.y).abs();
        assert!(
            sank * 170.0 < 0.2,
            "flat ground moved the ankle {:.2} cm, which is height taken off the warden for              standing on nothing",
            sank * 170.0
        );
    }

    #[test]
    fn ground_ten_centimetres_up_lifts_the_ankle_by_ten_centimetres() {
        let leg = a_leg();
        let up = 0.10 / 1.7;
        let put = plant_one(leg.root, leg.joint, leg.end, up, 0.0, 0.0, FORWARD, 1.0);
        assert!((put.shift - up).abs() < 1e-6);
        assert!(
            (put.now.end.y - (leg.end.y + up)).abs() < 1e-5,
            "the ankle went to {:.4} where the ground asked for {:.4}",
            put.now.end.y,
            leg.end.y + up
        );
        assert!((put.now.upper() - leg.upper()).abs() < 1e-5, "the thigh changed length");
        assert!((put.now.lower() - leg.lower()).abs() < 1e-5, "the calf changed length");
    }

    /// A coastline can drop hundreds of metres between two samples, and a leg stretched over the
    /// edge of one looks far worse than a foot hanging above it.
    #[test]
    fn a_cliff_is_not_reached_for() {
        let leg = a_leg();
        let put = plant_one(leg.root, leg.joint, leg.end, -300.0, 0.0, 0.0, FORWARD, 1.0);
        assert_eq!(put.shift, -GROUND_REACHES);
        assert!(put.now.end.is_finite());
    }

    /// Dropping the hips is what lets a foot REACH: the body goes down, the ground does not, so
    /// the leg has less distance to cover and stops clamping.
    ///
    /// This test first asserted the ankle landed in the SAME place either way, on the reasoning
    /// that the target had not moved. It failed, and it was the assertion that was wrong -
    /// which is the whole point of the drop.
    ///
    /// The numbers it used to quote came from the old 98% cap, where this leg could not even
    /// reach its own bind-pose ankle and clamped 0.92 cm short. At 0.999 it reaches that ankle,
    /// so what is being tested here is the case the drop is actually for: a target the leg
    /// genuinely cannot make, where lowering the hips shortens the distance it has to cover.
    #[test]
    fn dropping_the_hips_is_what_lets_a_foot_reach() {
        let leg = a_leg();
        let down = -0.05;
        let target = leg.end;                    // flat ground, so the target IS the bind ankle
        let plain = plant_one(leg.root, leg.joint, leg.end, 0.0, 0.0, 0.0, FORWARD, 1.0);
        let dropped = plant_one(leg.root, leg.joint, leg.end, 0.0, 0.0, down, FORWARD, 1.0);

        assert!(
            (dropped.was.root.y - (plain.was.root.y + down)).abs() < 1e-6,
            "the hip should have gone down with the body"
        );
        assert!(
            dropped.now.missed_by(target) < plain.now.missed_by(target),
            "dropped the foot missed by {:.2} cm and undropped by {:.2}; dropping the hips is \
             supposed to help it reach",
            dropped.now.missed_by(target) * 170.0,
            plain.now.missed_by(target) * 170.0
        );
        assert!(
            dropped.now.missed_by(target) * 170.0 < 0.1,
            "5 cm of hip drop should be plenty for this target, but it still missed by {:.2} cm",
            dropped.now.missed_by(target) * 170.0
        );
        // And both keep the bones honest.
        for (name, put) in [("plain", plain), ("dropped", dropped)] {
            assert!((put.now.upper() - leg.upper()).abs() < 1e-5, "{name}: thigh changed");
            assert!((put.now.lower() - leg.lower()).abs() < 1e-5, "{name}: calf changed");
        }
    }

    /// This rig's own leg, measured on the built model in Blender: thigh 42.00 cm, calf 36.34,
    /// at 170 cm per model unit. The bind pose is 99.9% extended.
    fn a_leg() -> Chain {
        let thigh = 42.00 / 170.0;
        let calf = 36.34 / 170.0;
        // Straight down, with the tiny forward knee offset the bind actually carries.
        Chain {
            root: Vec3::new(0.0, 0.85, 0.0),
            joint: Vec3::new(0.0, 0.85 - thigh, 0.004),
            end: Vec3::new(0.0, 0.85 - thigh - calf * 0.9998, 0.0),
        }
    }

    /// Forward IN THIS FIXTURE'S OWN FRAME, which is Y-up and +Z-forward because the solver is
    /// frame-agnostic and a self-contained frame makes the arithmetic readable.
    ///
    /// NOT the game's convention: a warden's front is -Z, per `util::facing_quat`. Labelled
    /// because confusing the two is what shipped backwards knees, and a bare `Vec3::Z` named
    /// `FORWARD` sitting near that code is an invitation to make the same mistake twice.
    const FORWARD: Vec3 = Vec3::Z;

    #[test]
    fn it_reaches_a_target_it_can_reach() {
        let leg = a_leg();
        // A third of the way up and a little forward: well inside the leg's range.
        let target = leg.root + Vec3::new(0.0, -leg.straight() * 0.8, 0.12);
        let got = reach(leg, target, FORWARD);
        assert!(
            got.missed_by(target) < 1e-5,
            "asked for {target:?} and got {:?}, {:.4} cm out",
            got.end,
            got.missed_by(target) * 170.0
        );
    }

    #[test]
    fn the_bones_keep_their_lengths() {
        let leg = a_leg();
        for lift in [0.0, 0.05, 0.15, 0.30] {
            for forward in [-0.20, 0.0, 0.20] {
                let target = leg.end + Vec3::new(0.0, lift, forward);
                let got = reach(leg, target, FORWARD);
                assert!(
                    (got.upper() - leg.upper()).abs() < 1e-5,
                    "the thigh changed from {:.5} to {:.5} reaching {target:?}",
                    leg.upper(),
                    got.upper()
                );
                assert!(
                    (got.lower() - leg.lower()).abs() < 1e-5,
                    "the calf changed from {:.5} to {:.5} reaching {target:?}",
                    leg.lower(),
                    got.lower()
                );
            }
        }
    }

    /// The measurement that shapes the module: a leg may not be asked to go straight, because
    /// a straight two-bone chain is singular and the authoring side measured it as untrackable
    /// at 99.3%.
    #[test]
    fn a_leg_is_never_asked_to_go_straight() {
        let leg = a_leg();
        // Straight down and further than the leg can possibly go.
        let target = leg.root + Vec3::new(0.0, -leg.straight() * 2.0, 0.0);
        let got = reach(leg, target, FORWARD);
        assert!(
            got.extension() <= EXTENDS_AT_MOST + 1e-4,
            "the leg went to {:.4} of straight, past the {EXTENDS_AT_MOST} cap",
            got.extension()
        );
        assert!(
            got.missed_by(target) > 0.0,
            "an unreachable target should be missed, not silently met"
        );
    }

    #[test]
    fn a_leg_is_never_folded_past_a_knee() {
        let leg = a_leg();
        let got = reach(leg, leg.root, FORWARD);
        assert!(
            got.extension() >= FOLDS_AT_MOST - 1e-4,
            "the ankle folded to {:.4} of straight, tighter than the {FOLDS_AT_MOST} a knee \
             allows",
            got.extension()
        );
    }

    /// The knee folds the way it is told, not the way the input pose happens to lean. On a bind
    /// pose 99.9% extended the input lean is a rounding error, so this is the property that
    /// stops a straight leg folding backwards.
    #[test]
    fn the_knee_folds_the_way_it_is_told() {
        let leg = a_leg();
        let target = leg.root + Vec3::new(0.0, -leg.straight() * 0.85, 0.0);
        for (name, toward) in [("forward", Vec3::Z), ("back", -Vec3::Z), ("out", Vec3::X)] {
            let got = reach(leg, target, toward);
            let line = (got.end - got.root).normalize();
            let offset = (got.joint - got.root) - line * (got.joint - got.root).dot(line);
            assert!(
                offset.length() > 1e-4,
                "{name}: the knee should be off the hip-to-ankle line"
            );
            assert!(
                offset.normalize().dot(toward.normalize()) > 0.99,
                "{name}: the knee went {:?}, not toward {toward:?}",
                offset.normalize()
            );
        }
    }

    /// A hint parallel to the leg has no across-component to use. It must still return
    /// something finite, and the same something every time — a normalised rounding error would
    /// jitter frame to frame, which is worse than being wrong in a fixed direction.
    #[test]
    fn a_hint_along_the_leg_does_not_produce_a_nan() {
        let leg = a_leg();
        let target = leg.root + Vec3::new(0.0, -leg.straight() * 0.85, 0.0);
        let along = (target - leg.root).normalize();
        let once = reach(leg, target, along);
        let twice = reach(leg, target, along);
        assert!(once.joint.is_finite(), "the knee came out {:?}", once.joint);
        assert!(once.end.is_finite(), "the ankle came out {:?}", once.end);
        assert_eq!(once, twice, "the same degenerate input gave two different answers");
    }

    #[test]
    fn a_target_on_the_hip_does_not_produce_a_nan() {
        let leg = a_leg();
        let got = reach(leg, leg.root, FORWARD);
        assert!(got.joint.is_finite() && got.end.is_finite());
        assert!((got.upper() - leg.upper()).abs() < 1e-5);
    }

    /// Solving for where the ankle already is should barely move it. Not exactly, because the
    /// bind is 99.9% extended and the cap pulls it back to 98% — and that difference is the
    /// point, so it is asserted rather than tolerated: 1.9% of 78.34 cm is 1.5 cm, which is how
    /// much a stance leg gets bent just by being run through the solver.
    #[test]
    fn solving_for_the_current_ankle_only_moves_it_by_the_extension_cap() {
        let leg = a_leg();
        let got = reach(leg, leg.end, FORWARD);
        let moved = got.end.distance(leg.end);
        let expected = (leg.extension() - EXTENDS_AT_MOST) * leg.straight();
        assert!(
            (moved - expected).abs() < 1e-4,
            "the ankle moved {:.3} cm where the cap alone accounts for {:.3} cm",
            moved * 170.0,
            expected * 170.0
        );
        assert!(
            moved * 170.0 < 2.0,
            "the cap moved the ankle {:.2} cm, which is enough to see",
            moved * 170.0
        );
    }

    /// Builds a warden's skeleton the shape glTF instancing gives it, and runs the real systems
    /// over it.
    ///
    /// The pure tests above check the arithmetic; this checks the plumbing, which is where a
    /// runtime IK actually fails — finding the bones, composing world transforms before
    /// propagation, and turning a solved position into a local rotation. None of that is
    /// exercised by a function taking three points.
    /// The same leg in METRES, as it stands in the world once the model is scaled up.
    ///
    /// Not `a_leg()`, which is in MODEL units at 170 cm to the unit, and building a runtime
    /// skeleton out of those numbers gives a character with 46 cm legs whose origin is 39 cm
    /// below its own ankle. That mistake made the first version of the test below fail with the
    /// ankle "35.6 cm from the ground", which is a fixture fault reported as a system fault.
    ///
    /// Measured on the rig: hip 0.501 of height, ankle 0.0418, thigh 42.00 cm and calf 36.34 at
    /// model scale. At 1.7 m tall that is a hip 85.2 cm up and an ankle 7.1 cm up — which is the
    /// number that matters here, because A PLANTED ANKLE IS NOT ON THE GROUND. It sits an
    /// ankle's height above the sole, and a test that expects otherwise is wrong about feet.
    const ANKLE_ABOVE_SOLE: f32 = 0.071;
    // This skeleton's LEFT leg, measured off the shipped .glb by
    // `dev/art/audit_character.py::the_legs`, which refuses if they drift. The right leg is
    // 36.91 + 40.59 - thigh-short and calf-long where the left is the other way round - so
    // nothing here may assume the two are the same.
    //
    // They were 0.420 and 0.363, which was the leg of a character deleted on 2026-08-24.
    const THIGH: f32 = 0.3869;
    const CALF: f32 = 0.3764;

    fn a_warden_standing_at(spot: Vec3) -> (App, Entity) {
        use crate::world::terrain::Terrain;

        let mut app = App::new();
        app.add_plugins((
            bevy::app::TaskPoolPlugin::default(),
            bevy::transform::TransformPlugin,
        ))
        .insert_resource(TerrainSource(std::sync::Arc::new(Terrain::new())))
        .insert_resource(Time::<()>::default())
        .add_systems(Update, (find_the_legs, plant_the_feet).chain());

        // In metres, sole at the origin, so the skeleton stands the way it does in the world.
        let hip_at = Vec3::Y * (ANKLE_ABOVE_SOLE + CALF + THIGH);
        let thigh_down = Vec3::NEG_Y * THIGH;
        let calf_down = Vec3::NEG_Y * CALF;

        let warden = app
            .world_mut()
            .spawn((
                Player,
                Transform::from_translation(spot),
                Visibility::default(),
            ))
            .id();
        // Player -> scene root -> Hip -> {L,R}_Thigh -> _Calf -> _Foot, each bone's translation
        // being the offset to the next joint, which is how a glTF skeleton arrives.
        let body = app
            .world_mut()
            .spawn((Name::new("Scene"), Transform::default()))
            .id();
        app.world_mut().entity_mut(body).insert(ChildOf(warden));
        let hips = app
            .world_mut()
            .spawn((Name::new("Hip"), Transform::from_translation(hip_at)))
            .id();
        app.world_mut().entity_mut(hips).insert(ChildOf(body));
        for (side, across) in [("L", -0.09_f32), ("R", 0.09)] {
            let thigh = app
                .world_mut()
                .spawn((
                    Name::new(format!("{side}_Thigh")),
                    Transform::from_translation(Vec3::new(across, 0.0, 0.0)),
                ))
                .id();
            app.world_mut().entity_mut(thigh).insert(ChildOf(hips));
            let calf = app
                .world_mut()
                .spawn((
                    Name::new(format!("{side}_Calf")),
                    Transform::from_translation(thigh_down),
                ))
                .id();
            app.world_mut().entity_mut(calf).insert(ChildOf(thigh));
            let foot = app
                .world_mut()
                .spawn((
                    Name::new(format!("{side}_Foot")),
                    Transform::from_translation(calf_down),
                ))
                .id();
            app.world_mut().entity_mut(foot).insert(ChildOf(calf));
        }
        (app, warden)
    }

    /// Runs frames with time actually passing, which anything eased needs.
    fn run_for(app: &mut App, frames: usize) {
        for _ in 0..frames {
            app.world_mut()
                .resource_mut::<Time>()
                .advance_by(std::time::Duration::from_millis(16));
            app.update();
        }
    }

    #[test]
    fn the_legs_are_found_in_a_wardens_own_skeleton() {
        let (mut app, warden) = a_warden_standing_at(Vec3::new(120.0, 0.0, -85.0));
        app.update();
        let legs = app.world().entity(warden).get::<Legs>().copied();
        assert!(legs.is_some(), "the leg bones were not found in the scene");
        let legs = legs.unwrap();
        assert_ne!(legs.left.thigh, legs.right.thigh, "both legs found the same bones");
    }

    /// The whole point, end to end: put the warden somewhere the ground is not level with their
    /// own feet, and the ankles should move toward the ground under each of them.
    #[test]
    fn the_feet_move_toward_the_ground_under_them() {
        use crate::world::terrain::Terrain;

        let terrain = Terrain::new();
        // Somewhere with real slope, so the two feet are over different heights.
        let mut spot = Vec3::new(0.0, 0.0, 0.0);
        let mut best = 0.0;
        for step in 0..60 {
            let at = Vec3::new(40.0 * step as f32, 0.0, 25.0 * step as f32);
            let slope = (terrain.height(at.x - 0.1, at.z) - terrain.height(at.x + 0.1, at.z)).abs();
            if slope > best {
                best = slope;
                spot = at;
            }
        }
        spot.y = terrain.height(spot.x, spot.z);
        assert!(best > 0.001, "found nowhere with any slope to test on");

        let (mut app, warden) = a_warden_standing_at(spot);
        // Long enough for the hip drop to settle. It is eased on purpose - the ground under a
        // foot changes discontinuously and snapping the body to it reads as a twitch - so a
        // single frame with no time elapsed leaves `dropped` at zero, and the LOWER foot cannot
        // reach because a straight leg has nothing left to extend. That is the system working;
        // the first version of this test just never let it run.
        run_for(&mut app, 40);
        let legs = *app.world().entity(warden).get::<Legs>().expect("legs");

        let ankle_of = |app: &App, leg: &Leg| {
            app.world()
                .entity(leg.foot)
                .get::<GlobalTransform>()
                .expect("the transform plugin should have propagated")
                .translation()
        };
        for (name, leg) in [("left", legs.left), ("right", legs.right)] {
            let ankle = ankle_of(&app, &leg);
            assert!(
                ankle.is_finite(),
                "{name} ankle came out {ankle:?} - a NaN here spreads through the hierarchy"
            );
            let ground = app
                .world()
                .resource::<TerrainSource>()
                .height(ankle.x, ankle.z);
            // A PLANTED ANKLE IS NOT ON THE GROUND — it sits an ankle's height above the sole.
            // So the claim is that the ankle keeps its own height above the ground UNDER IT,
            // which is what following the terrain means. Not exact: the extension cap costs
            // about 1.5 cm on this bind and a clamped foot lands short on purpose.
            let above = ankle.y - ground;
            assert!(
                (above - ANKLE_ABOVE_SOLE).abs() < 0.03,
                "{name} ankle sits {:.1} cm over its own ground where it should sit {:.1}; \
                 ankle {:.3}, ground {:.3}",
                above * 100.0,
                ANKLE_ABOVE_SOLE * 100.0,
                ankle.y,
                ground
            );
        }
    }

    /// A KNEE FOLDS FORWARD, whichever way the warden is facing.
    ///
    /// This shipped backwards. The fold direction is handed to the solver as a world-space pole,
    /// and it was `Vec3::Z` where `util::facing_quat` — the authority on which way a warden
    /// faces — says the rotation it produces "is applied to a model whose front is -Z". A pole
    /// BEHIND the knee admits exactly one solution, and it is the wrong one.
    ///
    /// Tested at four headings and against `facing_quat` itself rather than a written-down axis,
    /// because a hard-coded axis in the test is how the code's hard-coded axis went unnoticed.
    /// The property is anatomical: the knee sits on the forward side of the hip-to-ankle line,
    /// which is what a knee does.
    #[test]
    fn a_knee_folds_forward_whichever_way_the_warden_faces() {
        use crate::util::facing_quat;
        use crate::world::terrain::Terrain;

        let terrain = Terrain::new();
        let spot = Vec3::new(300.0, terrain.height(300.0, 180.0), 180.0);
        for heading in [
            Vec3::new(0.0, 0.0, -1.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(-0.6, 0.0, 0.8).normalize(),
        ] {
            let (mut app, warden) = a_warden_standing_at(spot);
            let turn = facing_quat(heading).expect("a heading off the vertical");
            app.world_mut().entity_mut(warden).insert(
                Transform::from_translation(spot).with_rotation(turn),
            );
            run_for(&mut app, 40);
            let legs = *app.world().entity(warden).get::<Legs>().expect("legs");

            for (name, leg) in [("left", legs.left), ("right", legs.right)] {
                let at = |entity: Entity| {
                    app.world()
                        .entity(entity)
                        .get::<GlobalTransform>()
                        .expect("propagated")
                        .translation()
                };
                let (hip, knee, ankle) = (at(leg.thigh), at(leg.calf), at(leg.foot));
                let line = (ankle - hip).normalize_or_zero();
                let out = (knee - hip) - line * (knee - hip).dot(line);
                assert!(
                    out.length() > 1e-4,
                    "{name} knee sits on the hip-to-ankle line, so it has not folded at all"
                );
                assert!(
                    out.normalize().dot(heading) > 0.7,
                    "{name} knee folded {:?} while the warden faces {heading:?} - a knee bends \
                     FORWARD, and this is the backwards-knee bug",
                    out.normalize()
                );
            }
        }
    }

    /// THE SOLE LIES ALONG THE GROUND, and is not pitched by the knee that bent above it.
    ///
    /// This is the half of foot IK that was missing and it showed in game: the warden stood on
    /// his toes. `_Foot` is a child of `_Calf`, the correction bends the knee on every frame
    /// because the bind is 99.9% extended against a 98% cap, and every degree of that bend
    /// carried the shoe toe-down with it.
    ///
    /// Asserted on where the sole's own up-direction ends up, against the terrain's normal. In
    /// the fixture the foot bone starts unrotated, so its world up after correction is exactly
    /// the tilt that was applied - which is what makes the claim checkable without knowing the
    /// export's axis convention.
    #[test]
    fn the_sole_lies_along_the_ground_it_stands_on() {
        use crate::world::terrain::Terrain;

        let terrain = Terrain::new();
        // The steepest spot in a sweep, so the normal is meaningfully off vertical.
        let (mut spot, mut steepest) = (Vec3::ZERO, 0.0);
        for step in 0..80 {
            let at = Vec3::new(37.0 * step as f32, 0.0, 23.0 * step as f32);
            let off = terrain.normal(at.x, at.z, A_FOOT_WIDE).dot(Vec3::Y);
            if 1.0 - off > steepest {
                steepest = 1.0 - off;
                spot = at;
            }
        }
        spot.y = terrain.height(spot.x, spot.z);
        assert!(steepest > 1e-4, "found nowhere with any slope at all");

        let (mut app, warden) = a_warden_standing_at(spot);
        run_for(&mut app, 40);
        let legs = *app.world().entity(warden).get::<Legs>().expect("legs");

        for (name, leg) in [("left", legs.left), ("right", legs.right)] {
            let placed = app
                .world()
                .entity(leg.foot)
                .get::<GlobalTransform>()
                .expect("propagated");
            let sole_up = placed.rotation() * Vec3::Y;
            let ankle = placed.translation();
            let slope = terrain.normal(ankle.x, ankle.z, A_FOOT_WIDE);
            let wanted = at_most(aim(Vec3::Y, slope), FOOT_TILTS_AT_MOST) * Vec3::Y;
            let apart = sole_up.angle_between(wanted).to_degrees();
            assert!(
                apart < 2.0,
                "{name} sole faces {sole_up:?} where the ground asks for {wanted:?}, {apart:.1} \
                 degrees apart - a knee bending above it must not pitch the shoe"
            );
        }
    }

    #[test]
    fn a_foot_follows_a_slope_but_not_a_cliff() {
        // Straight up needs no tilt at all.
        assert!(at_most(aim(Vec3::Y, Vec3::Y), FOOT_TILTS_AT_MOST).is_near_identity());
        // A wall's normal is 90 degrees off vertical; the foot goes as far as it may and stops.
        let sheer = at_most(aim(Vec3::Y, Vec3::X), FOOT_TILTS_AT_MOST);
        let (_, angle) = sheer.to_axis_angle();
        assert!(
            (angle.to_degrees() - FOOT_TILTS_AT_MOST).abs() < 1e-3,
            "a sheer face tilted the foot {:.1} degrees, past the {FOOT_TILTS_AT_MOST} cap",
            angle.to_degrees()
        );
    }

    /// The drop is SET, never added. Running many frames with nothing animating the skeleton must
    /// not walk the warden into the floor - which an `+=` on a channel the animation owns does.
    #[test]
    fn the_body_drop_does_not_accumulate() {
        let (mut app, warden) = a_warden_standing_at(Vec3::new(120.0, 0.0, -85.0));
        app.update();
        let body = app.world().entity(warden).get::<Legs>().expect("legs").body;
        let after_one = app.world().entity(body).get::<Transform>().unwrap().translation.y;
        run_for(&mut app, 120);
        let after_many = app.world().entity(body).get::<Transform>().unwrap().translation.y;
        assert!(
            after_many >= -HIPS_DROP_AT_MOST - 1e-4,
            "the body sank to {after_many:.3}, past the {HIPS_DROP_AT_MOST} cap, so the drop is \
             accumulating"
        );
        assert!(
            (after_many - after_one).abs() < HIPS_DROP_AT_MOST + 1e-4,
            "the body moved from {after_one:.4} to {after_many:.4} over 120 frames of nothing \
             changing"
        );
    }

    /// What a wider stride COSTS on this leg, printed as a table.
    ///
    ///     cargo test the_reach_budget -- --ignored --nocapture
    ///
    /// Stride warping buys speed by moving the foot targets apart instead of playing the clip
    /// faster. The question that decides how much of it is usable is not a matter of taste: a
    /// planted foot `ahead` in front of the hip pins the hip to `sqrt(reach^2 - ahead^2)` above
    /// the ankle, so every centimetre of extra stride is paid for in crouch. This prints the
    /// exchange rate, so the cap is chosen from it rather than from Paragon's 60% - which was
    /// measured on a different character.
    #[test]
    #[ignore = "prints a table rather than asserting"]
    fn the_reach_budget() {
        let straight = THIGH + CALF;
        let reach = straight * EXTENDS_AT_MOST;
        let standing = straight * 0.999; // the bind, hip above ankle
        println!(
            "leg {straight:.3} m straight, usable {reach:.3} at the {EXTENDS_AT_MOST} cap, \
             hip {standing:.3} above the ankle standing"
        );
        println!("\n  {:>10} {:>12} {:>14}", "hip drop", "foot ahead", "contact length");
        for drop in [0.0_f32, 0.02, 0.04, 0.06, 0.08, 0.10, 0.15, 0.20] {
            let hip = standing - drop;
            let ahead = (reach * reach - hip * hip).max(0.0).sqrt();
            println!(
                "  {:9.0}cm {:11.3}m {:13.3}m{}",
                drop * 100.0,
                ahead,
                ahead * 2.0,
                if ahead == 0.0 { "   <- cannot reach ahead at all" } else { "" }
            );
        }
        // What each gait actually asks for, so the two can be compared.
        println!("\n  what the clips ask, per foot, as authored:");
        for (gait, contact, covers) in
            [("walk", 0.550, 0.970), ("run", 0.578, 2.496), ("sprint", 0.625, 3.283)]
        {
            let needs = |contact: f32| {
                let ahead = contact / 2.0;
                (standing - (reach * reach - ahead * ahead).max(0.0).sqrt()).max(0.0)
            };
            println!(
                "  {gait:<7} contact {contact:.3} m needs {:.0} cm of drop; at 1.3x it is \
                 {:.3} m needing {:.0} cm; at 1.6x, {:.3} m needing {:.0} cm  (covers {covers})",
                needs(contact) * 100.0,
                contact * 1.3,
                needs(contact * 1.3) * 100.0,
                contact * 1.6,
                needs(contact * 1.6) * 100.0,
            );
        }
    }

    /// A warped stride moves the foot ALONG THE LINE OF TRAVEL and nowhere else.
    #[test]
    fn warping_widens_the_step_and_leaves_its_height_alone() {
        let leg = a_leg();
        // A foot 20 cm in front of the hip, on the ground.
        let ankle = leg.root + Vec3::new(0.0, -leg.straight() * 0.9, 0.20);
        let plain = warped_target(leg.root, ankle, 0.0, FORWARD, 1.0);
        let wider = warped_target(leg.root, ankle, 0.0, FORWARD, 1.25);
        assert_eq!(plain, ankle, "a stride of 1.0 is the stride as authored");
        assert!(
            ((wider.z - leg.root.z) - 0.25).abs() < 1e-5,
            "20 cm ahead should become 25 at 1.25x, not {:.3}",
            wider.z - leg.root.z
        );
        assert!(
            (wider.y - ankle.y).abs() < 1e-6,
            "warping must not change the foot's height - the ground has already had its say"
        );
        // And behind the hip it goes further behind, which is what widens the step rather than
        // sliding it forward.
        let behind = leg.root + Vec3::new(0.0, -leg.straight() * 0.9, -0.20);
        let back = warped_target(leg.root, behind, 0.0, FORWARD, 1.25);
        assert!((back.z - leg.root.z + 0.25).abs() < 1e-5, "the trailing foot should trail more");
    }

    /// The crouch a wider stride costs is DERIVED, not chosen, and it is the reason the cap is
    /// 1.25 rather than Paragon's 1.6.
    #[test]
    fn a_wider_stride_is_paid_for_in_crouch() {
        let leg = a_leg();
        let reach = leg.straight() * EXTENDS_AT_MOST;
        let on_the_ground = |ahead: f32| leg.root + Vec3::new(0.0, -leg.straight() * 0.999, ahead);

        // Straight under the hip the leg is at full stretch already, so even nothing ahead wants
        // a little drop - the 98% cap against a 99.9% bind.
        let square = stride_needs_a_drop(leg.root, on_the_ground(0.0), reach, FORWARD);
        assert!(square <= 0.0, "a drop is downward or nothing, never a lift");

        // The further ahead, the deeper. Monotonic, which is the property that matters.
        let mut deeper = 0.0_f32;
        for ahead in [0.10_f32, 0.20, 0.30, 0.40] {
            let drop = stride_needs_a_drop(leg.root, on_the_ground(ahead), reach, FORWARD);
            assert!(
                drop <= deeper + 1e-6,
                "{ahead:.2} m ahead asked for {drop:.3} where {:.2} m asked {deeper:.3}",
                ahead - 0.10
            );
            deeper = drop;
        }
        assert!(deeper < -0.02, "40 cm ahead should want a real crouch, not {deeper:.3}");
        // And never past the cap, whatever nonsense it is handed.
        assert_eq!(
            stride_needs_a_drop(leg.root, on_the_ground(99.0), reach, FORWARD),
            -HIPS_DROP_AT_MOST
        );
    }

    /// The split that is the entire point: the stride takes what it can and the play rate carries
    /// the rest, so `covers x stride` is the ground one cycle now covers.
    #[test]
    fn the_stride_takes_what_it_can_and_the_rate_carries_the_rest() {
        use crate::motion::{warps_the_stride, STRIDE_WARPS_TO};

        // The jog, as the game now drives it: 5.90 m/s against a clip carrying 2.496 m over
        // 1.042 s. Unwarped that asks 2.46x of the play rate.
        let (speed, covers, lasts) = (5.90_f32, 2.496_f32, 1.0417_f32);
        let unwarped = lasts * speed / covers;
        let stride = warps_the_stride(speed, covers, lasts);
        let warped = lasts * speed / (covers * stride);
        assert_eq!(stride, STRIDE_WARPS_TO, "this speed should use the whole stride budget");
        assert!(
            (warped - unwarped / STRIDE_WARPS_TO).abs() < 1e-4,
            "the rate should fall by exactly the stride: {unwarped:.3} to {warped:.3}"
        );
        assert!(warped < unwarped, "warping is supposed to slow the clip down, not speed it up");
        // In band, which is the outcome that was wanted: a 24-frame clip at this rate is an
        // effective cycle of 24/rate frames, and a run is authored over 12 to 16.
        let frames = 24.0 / warped;
        assert!(
            (12.0..=16.5).contains(&frames),
            "the effective cycle is {frames:.1} frames, outside the 12-16 a run is authored over"
        );

        // A walk already matches its clip, so there is nothing to warp and nothing to crouch for.
        assert_eq!(warps_the_stride(0.93, 0.970, lasts), 1.0);
    }

    #[test]
    fn aiming_turns_one_direction_into_another() {
        let turn = aim(Vec3::NEG_Y, Vec3::Z);
        assert!((turn * Vec3::NEG_Y).distance(Vec3::Z) < 1e-5);
        assert_eq!(aim(Vec3::ZERO, Vec3::Z), Quat::IDENTITY);
        assert_eq!(aim(Vec3::Z, Vec3::ZERO), Quat::IDENTITY);
    }

    /// How far a foot can actually be put DOWN, against how far the ground is allowed to fall.
    ///
    /// # The bind's locked legs are the real limit, and they cost more than anything else here
    ///
    /// A foot reaches down by two means: straightening the knee, and dropping the hips. This
    /// character has almost none of the first - the bind stands at 99.9% of straight, so there
    /// is 0.08 cm of extension left in the leg - which leaves the hip drop doing all of it.
    ///
    /// That is not what a bind pose is normally for. A rig meant for ground contact is bound with
    /// a real knee bend precisely so the leg has somewhere to go, and this one was not. Everything
    /// downstream inherits it: `GROUND_REACHES` promises 35 cm of correction that the leg cannot
    /// deliver, and `player::CLIMB_LIMIT` of 1.4 permits the ground to fall 43 cm over half a walk
    /// step and 155 cm over half a run step - both far past what any leg on this body can follow.
    ///
    /// Nothing breaks: `reach` clamps and the foot lands short, which is the documented behaviour
    /// and reads as a foot hanging over a drop rather than a leg tearing. But the numbers should
    /// say so out loud instead of three constants quietly disagreeing, so this asserts the parts
    /// that are true and prints the parts that are a design decision.
    #[test]
    fn a_foot_reaches_down_as_far_as_the_hips_can_drop_and_no_further() {
        let straight = THIGH + CALF;
        let standing = straight * 0.999;
        let left_in_the_leg = straight * EXTENDS_AT_MOST - standing;
        let reaches_down = left_in_the_leg + HIPS_DROP_AT_MOST;

        // Half a walk step: the distance from the warden's own footing to where a foot lands,
        // which is the span the ground can differ across. The walk clip holds two cycles and so
        // four steps, hence the quarter, and half of that again.
        let step = crate::motion::WALK_COVERS_FOR_TESTS / 4.0;
        let falls = step * 0.5 * crate::player::CLIMB_LIMIT;

        println!(
            "leg {straight:.3} m straight, standing {standing:.3}; {:.1} cm of extension left \
             plus {:.0} cm of hip drop = {:.1} cm of downward reach",
            left_in_the_leg * 100.0,
            HIPS_DROP_AT_MOST * 100.0,
            reaches_down * 100.0
        );
        println!(
            "  a walk step is {step:.3} m, so a foot lands half that from the warden's footing; \
             over that span CLIMB_LIMIT permits a fall of {:.1} cm",
            falls * 100.0
        );

        assert!(
            reaches_down > 0.05,
            "a foot can only be put {:.1} cm below where the clip left it, which is not enough \
             for any ground at all",
            reaches_down * 100.0
        );
        assert!(
            GROUND_REACHES >= reaches_down,
            "GROUND_REACHES of {GROUND_REACHES} m is less than the {reaches_down} m the leg can \
             actually cover, so the trace gives up before the leg does"
        );
        // And the one that is a design decision rather than a bug, stated where it can be seen.
        if falls > reaches_down {
            println!(
                "  NOTE the ground may fall {:.1} cm where a foot can follow it {:.1} cm, so on \
                 the steepest walkable slope a foot hangs. Fixing it means a knee-eased bind, a \
                 lower CLIMB_LIMIT, or accepting it.",
                falls * 100.0,
                reaches_down * 100.0
            );
        }
    }

    /// The cap must not fight the clips, and it must still leave a knee to read.
    ///
    /// This used to assert `EXTENDS_AT_MOST == 0.98` against a copy of the number in
    /// `dev/art/ik_gait.py` - a file that no longer exists, so the test was pinning the constant
    /// to nothing. Worse, pinning a value is not checking it: 0.98 was wrong for this character
    /// for a year of commits and a test asserting its exact value could never have said so.
    ///
    /// So this checks the two things the cap is actually FOR. The clips reach 100.0% of straight,
    /// measured by `dev/art/audit_character.py::the_legs`, so anything below about 0.998 starts
    /// taking visible height off an authored stance; and the cap has to stay under 1.0 or the
    /// knee lands exactly on the hip-ankle line with no bend at all.
    #[test]
    fn the_cap_neither_fights_the_clips_nor_locks_the_knee() {
        assert!(
            EXTENDS_AT_MOST < 1.0,
            "a cap of {EXTENDS_AT_MOST} lets the leg go dead straight, which puts the knee on              the hip-ankle line with no bend direction to read"
        );
        let straight = THIGH + CALF;
        let short = straight * (1.0 - EXTENDS_AT_MOST);
        assert!(
            short * 170.0 < 0.2,
            "the cap gives up {:.2} cm of reach, which the hips have to drop to cover - the              clips stand at 100% of straight, so that comes straight off the warden's height",
            short * 170.0
        );
        // And it must leave a knee offset big enough to see and to test.
        let reaches = straight * EXTENDS_AT_MOST;
        let along = (reaches * reaches + THIGH * THIGH - CALF * CALF) / (2.0 * reaches);
        let off = (THIGH * THIGH - along * along).max(0.0).sqrt();
        assert!(
            off * 170.0 > 1.0,
            "at a {EXTENDS_AT_MOST} cap the knee sits only {:.2} cm off the hip-ankle line,              which reads as a locked leg",
            off * 170.0
        );
    }

    /// Solves a spread of targets and prints them as JSON, for `dev/art/see_the_ik.sh` to pose
    /// the real rig with and render.
    ///
    ///     cargo test solve_a_leg_for_blender -- --ignored --nocapture
    ///
    /// The point is that there is ONE solver. Writing a second copy in Python to look at would
    /// be looking at a different solver, and agreement between two implementations of the same
    /// arithmetic proves nothing about either. So Rust computes and Blender only draws — and
    /// what it draws is measured back off the posed rig, which also tests the part that is
    /// easiest to get wrong: turning solved POSITIONS into bone rotations.
    #[test]
    #[ignore = "writes a file for the Blender viewer rather than asserting"]
    fn solve_a_leg_for_blender() {
        let leg = a_leg();
        let straight = leg.straight();
        // A spread that covers what foot planting and stride warping will actually ask for:
        // the ground rising and falling under a planted foot, and the foot reaching fore and
        // aft of the hip.
        let mut cases = Vec::new();
        for (name, lift, forward) in [
            ("flat", 0.0, 0.0),
            ("step up 10cm", 0.10 / 1.7, 0.0),
            ("step up 20cm", 0.20 / 1.7, 0.0),
            ("step down 10cm", -0.10 / 1.7, 0.0),
            ("reach forward", 0.0, straight * 0.45),
            ("reach back", 0.0, -straight * 0.45),
            ("forward and up", 0.10 / 1.7, straight * 0.30),
            ("out of reach", -straight * 0.5, 0.0),
        ] {
            let target = leg.end + Vec3::new(0.0, lift, forward);
            let got = reach(leg, target, Vec3::Z);
            // Written HIP-RELATIVE and in NAMED AXES, never as absolute coordinates. Blender
            // is Z-up and this is Y-up, and the viewer would have to convert - which is the
            // frame-of-reference trap that has cost this project more than any other single
            // mistake. Naming the axes means the viewer rebuilds each point from the rig's own
            // measured up/forward/across and no conversion exists to get backwards.
            let say = |v: Vec3| {
                let from_hip = v - leg.root;
                format!(
                    r#"{{"up": {:.6}, "forward": {:.6}, "across": {:.6}}}"#,
                    from_hip.y, from_hip.z, from_hip.x
                )
            };
            cases.push(format!(
                r#"    {{"called": "{name}",
     "target": {}, "joint": {}, "end": {},
     "missed_by": {:.6}, "extension": {:.6}}}"#,
                say(target),
                say(got.joint),
                say(got.end),
                got.missed_by(target),
                got.extension(),
            ));
        }
        println!("SOLVED_LEG_JSON_BEGIN");
        println!("{{");
        println!(r#"  "thigh": {:.6}, "calf": {:.6}, "bind_extension": {:.6},"#,
                 leg.upper(), leg.lower(), leg.extension());
        println!(r#"  "extends_at_most": {EXTENDS_AT_MOST}, "folds_at_most": {FOLDS_AT_MOST},"#);
        println!(r#"  "cases": ["#);
        println!("{}", cases.join(",\n"));
        println!("  ]");
        println!("}}");
        println!("SOLVED_LEG_JSON_END");
    }
}

