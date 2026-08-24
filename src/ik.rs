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
//! Thigh **42.00 cm**, calf **36.34 cm**, so a straight leg is 78.34 cm hip to ankle. The bind
//! pose sits at **99.9% of that** — `prepare_rig::KNEE_EASE` is 2 degrees each way, and 4
//! degrees of knee fold is only 0.09% off dead straight.
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
//! **A LEG MUST NOT BE ASKED TO GO STRAIGHT.** `dev/art/ik_gait.py` measured this on the
//! authoring side: a leg extended 99.3% of the way "cannot be solved: there is no bend for the
//! solver to work with and it fails to track at all", and capping it at 98% took stance slide
//! from unusable to 0.01 mm. The same cap is [`EXTENDS_AT_MOST`] here. It is not a safety
//! margin against floating point — it is that a straight two-bone chain is SINGULAR, the same
//! reason `KNEE_EASE` exists in the bind at all.
//!
//! # What it does when it cannot reach
//!
//! It clamps and the foot lands short, along the line it was asked for. Deliberately, and
//! stated because the authoring side learned it the hard way: "an unreachable target does not
//! fail loudly, it clamps, and the foot lands short". Callers that care whether the target was
//! met should compare the returned `end` against what they asked for — [`Chain::missed_by`].

use bevy::prelude::*;

/// How straight a leg may be asked to go, as a share of hip-to-ankle at full extension.
///
/// 0.98, matching `dev/art/ik_gait.py::STANCE_LEG_EXTENDS`, and the two want to stay equal:
/// the authoring side solves stance legs with this cap and the runtime corrects those same
/// poses, so a runtime that allowed straighter legs than the author did would pull a stance
/// leg past the point the clip was built at.
pub const EXTENDS_AT_MOST: f32 = 0.98;

/// How far a leg may fold, as a share of full extension — the ankle may not come closer to the
/// hip than this.
///
/// 0.27 is where a human knee stops: about 150 degrees of flexion, which on this leg's 42.00
/// and 36.34 cm puts the ankle 21.2 cm from the hip, or 27% of 78.34. Generous rather than
/// tight, because this is a rail against a nonsense target and not a pose limit.
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
    pub fn straight(&self) -> f32 {
        self.upper() + self.lower()
    }

    /// Hip to ankle as it currently stands, as a share of straight.
    ///
    /// 0.999 in this rig's bind pose, which is the measurement that shapes this whole module.
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
pub fn aim(was: Vec3, wants: Vec3) -> Quat {
    if was.length() < NO_DIRECTION || wants.length() < NO_DIRECTION {
        return Quat::IDENTITY;
    }
    Quat::from_rotation_arc(was.normalize(), wants.normalize())
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn aiming_turns_one_direction_into_another() {
        let turn = aim(Vec3::NEG_Y, Vec3::Z);
        assert!((turn * Vec3::NEG_Y).distance(Vec3::Z) < 1e-5);
        assert_eq!(aim(Vec3::ZERO, Vec3::Z), Quat::IDENTITY);
        assert_eq!(aim(Vec3::Z, Vec3::ZERO), Quat::IDENTITY);
    }

    /// The runtime cap and the authoring cap have to agree, or the runtime pulls stance legs
    /// past where the clips were built. Stated here because the authoring copy lives in a
    /// Python file no Rust test can read, so this is the reminder rather than the check.
    #[test]
    fn the_extension_cap_matches_the_authoring_side() {
        assert_eq!(
            EXTENDS_AT_MOST, 0.98,
            "dev/art/ik_gait.py::STANCE_LEG_EXTENDS is 0.98; if that moves, this moves"
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
