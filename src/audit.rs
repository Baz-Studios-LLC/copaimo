//! What is standing where nothing should be, asked of the assembled world.
//!
//! # Why this cannot be a test
//!
//! Where a boulder or a tree ends up is decided by a world-wide lattice, the biome
//! under it, and a pool of models loaded from disk at run time. A unit test can ask
//! the lattice and it cannot ask the pool, so it can prove that the ARITHMETIC of
//! placement is right and cannot see a rock sitting in the middle of a city street -
//! which is the only form the question is ever actually asked in.
//!
//! So it is asked of the running game, of the same resources the warden's own
//! collision reads, through the same `standing_near` the warden calls. What this
//! reports is what a player would walk into.

use bevy::prelude::*;
use std::io::Write;

/// How far apart the audit samples a street, in metres.
///
/// Half the warden's width, so nothing they could be stopped by fits between two
/// samples.
const LOOKS_EVERY: f32 = 0.16;

#[derive(Resource, Default)]
pub struct Auditing {
    /// Frames still to wait for the pool and the grove to finish loading.
    pub settling: u32,
}

/// Whether this is in the way, and the nearest miss either way.
///
/// One place decides it, so the report and the running tally cannot disagree about
/// what "on the street" means - and the nearest miss is kept whether or not anything
/// was found, because an audit that only ever prints "nothing" is indistinguishable
/// from an audit that is not looking. It is the ruler's own reading.
fn seen(found: &mut Vec<InTheWay>, nearest: &mut Option<InTheWay>, thing: InTheWay) {
    let clear = thing.across - thing.reach - thing.carriage;
    if nearest.as_ref().is_none_or(|had| {
        clear < had.across - had.reach - had.carriage
    }) {
        *nearest = Some(InTheWay { what: thing.what.clone(), ..thing });
    }
    if clear > 0.0 {
        return;
    }
    if found.iter().any(|had| had.at.distance(thing.at) < 0.5) {
        return;
    }
    found.push(thing);
}

/// One thing standing where it should not be.
struct InTheWay {
    what: String,
    at: Vec2,
    /// How far from the middle of the street, and how wide the street's own
    /// carriageway is there.
    across: f32,
    carriage: f32,
    reach: f32,
    settlement: Vec2,
    city: bool,
    solid: bool,
}

pub fn asked_for() -> bool {
    std::env::args().any(|arg| arg == "--audit")
}

/// Walks every street of every settlement and asks what is standing on it.
pub fn audit_the_streets(
    mut auditing: ResMut<Auditing>,
    terrain: Option<Res<crate::world::terrain::TerrainSource>>,
    grove: Option<Res<crate::world::stream::Grove>>,
    props: Option<Res<crate::world::prop::PropPool>>,
    mut quit: EventWriter<AppExit>,
) {
    let (Some(terrain), Some(grove), Some(props)) = (terrain, grove, props) else {
        return;
    };
    if auditing.settling > 0 {
        auditing.settling -= 1;
        return;
    }

    let plan = terrain.0.plan();
    let mut found: Vec<InTheWay> = Vec::new();
    let mut streets = 0;
    let mut metres = 0.0_f32;
    // Held between samples rather than allocated at each one - the same reason
    // `move_player` keeps it as a `Local`.
    let mut nearest: Option<InTheWay> = None;

    for (key, site) in plan.sites().iter().enumerate() {
        if site.ranch {
            continue;
        }
        let layout = crate::world::town::lay_the_site_out(plan, key, site);

        for street in &layout.streets {
            streets += 1;
            let along = street.to - street.from;
            let run = along.length();
            metres += run;
            let steps = (run / LOOKS_EVERY).ceil().max(1.0) as usize;

            for step in 0..=steps {
                let on = street.from + along * step as f32 / steps as f32;
                // THE SECTION AT THIS POINT, from the one place that owns it. A
                // street is not a constant width and an audit that assumed one would
                // report the verge as the carriageway.
                let cut = crate::world::town::RoadSection::at(
                    street.wide,
                    street.wide,
                    crate::world::town::paved_here(plan, on),
                    on,
                );

                // EVERY prop, not only the solid ones. `standing_near` filters to
                // what stops a warden, which is the right question for collision and
                // the wrong one here: a bush growing out of a cobbled street looks
                // exactly as wrong as a boulder does, and you walk through it.
                for strewn in crate::world::prop::litter_in(
                    &terrain.0,
                    &props.0,
                    on - Vec2::splat(crate::player::LOOKS_AHEAD),
                    on + Vec2::splat(crate::player::LOOKS_AHEAD),
                ) {
                    seen(
                        &mut found,
                        &mut nearest,
                        InTheWay {
                            what: format!("{:?}", strewn.kind),
                            at: strewn.at,
                            across: strewn.at.distance(on),
                            carriage: cut.carriage,
                            reach: strewn.reach,
                            settlement: site.at,
                            city: site.city,
                            solid: crate::world::prop::is_solid(strewn.kind),
                        },
                    );
                }

                for tree in terrain
                    .0
                    .trees_in(on - Vec2::splat(crate::player::LOOKS_AHEAD), on + Vec2::splat(crate::player::LOOKS_AHEAD))
                {
                    let reach = grove
                        .trees
                        .get(tree.variety)
                        .map(|variety| variety.trunk * tree.scale)
                        .unwrap_or(0.2);
                    let at = Vec2::new(tree.at.x, tree.at.z);
                    seen(
                        &mut found,
                        &mut nearest,
                        InTheWay {
                            what: "a tree".into(),
                            at,
                            across: at.distance(on),
                            carriage: cut.carriage,
                            reach,
                            settlement: site.at,
                            city: site.city,
                            solid: true,
                        },
                    );
                }
            }
        }

        // AND THE BUILDINGS, through the rule that is supposed to keep them off.
        //
        // `off_this_street` is what placement asks. An audit with its own idea of
        // "on the street" would be testing its own arithmetic; asking the real
        // predicate means anything found here is something that never went through
        // the rule, which is the useful thing to be told.
        // EVERY ROAD NEAR THE SETTLEMENT, not only the ones it laid itself.
        //
        // `clear_of_streets` is handed `layout.streets`, which is what the town
        // planned. The roads BETWEEN settlements are planned somewhere else entirely
        // and they do not stop at a town's edge - an approach runs in and keeps
        // going - so if that is where the buildings in the road are, no rule has ever
        // looked at it.
        let mut roads: Vec<crate::world::town::Street> = layout.streets.clone();
        let country = roads.len();
        for way in plan.ways() {
            if way.from.distance(site.at) < site.radius + 120.0
                || way.to.distance(site.at) < site.radius + 120.0
            {
                roads.push(crate::world::town::Street {
                    from: way.from,
                    to: way.to,
                    wide: crate::config::ROAD_WIDE,
                });
            }
        }

        // The same paving the town's own placement rules were given.
        let made = f32::from(u8::from(site.city));
        for plot in &layout.plots {
            for (which, street) in roads.iter().enumerate() {
                if crate::world::town::off_this_street(street, plot.at, plot.facing, plot.what, made) {
                    continue;
                }
                let whose = if which < country { "street" } else { "COUNTRY ROAD" };
                let on = street.nearest_point(plot.at);
                seen(
                    &mut found,
                    &mut nearest,
                    InTheWay {
                        what: format!("{:?} in a {whose}", plot.what),
                        at: plot.at,
                        across: plot.at.distance(on),
                        carriage: crate::world::town::RoadSection::widest_half(
                            street.wide,
                            street.wide,
                            made,
                        ),
                        // Already decided by the rule above; nought so `seen` keeps it.
                        reach: 0.0,
                        settlement: site.at,
                        city: site.city,
                        solid: true,
                    },
                );
            }
        }
    }

    write_the_report(&found, &nearest, streets, metres);
    if found.is_empty() {
        let miss = nearest.as_ref().map_or("nothing was looked at".into(), |one| {
            format!(
                "the nearest thing to a street was {} at ({:.0}, {:.0}), clear by {:.2} m",
                one.what,
                one.at.x,
                one.at.y,
                one.across - one.reach - one.carriage
            )
        });
        info!("{streets} streets, {metres:.0} m: nothing standing in any of them — {miss}");
    } else {
        warn!(
            "{} things standing in {streets} streets of {:.0} m",
            found.len(),
            metres
        );
        for one in found.iter().take(12) {
            warn!(
                "  {} at ({:.0}, {:.0}) — {:.2} m from the middle of a {:.2} m {}",
                one.what,
                one.at.x,
                one.at.y,
                one.across,
                one.carriage * 2.0,
                if one.city { "city street" } else { "village lane" },
            );
        }
    }
    quit.write(AppExit::Success);
}

fn write_the_report(
    found: &[InTheWay],
    nearest: &Option<InTheWay>,
    streets: usize,
    metres: f32,
) {
    let _ = std::fs::create_dir_all("dev/evidence");
    let Ok(mut file) = std::fs::File::create("dev/evidence/obstructions.md") else {
        return;
    };
    let _ = writeln!(
        file,
        "# What is standing in the streets\n\n\
         Asked of the assembled world, through the same `standing_near` the warden's \
         own collision reads. Anything here is something a player walks into.\n\n\
         Seed {}. {streets} streets, {metres:.0} m of carriageway, {} obstructions.\n",
        crate::config::WORLD_SEED,
        found.len()
    );
    if let Some(one) = nearest {
        let _ = writeln!(
            file,
            "Nearest miss: {} at ({:.0}, {:.0}), clear of the carriageway by {:.2} m.              Printed whether or not anything was found, because an audit that only              ever says \"nothing\" cannot be told from one that is not looking.
",
            one.what,
            one.at.x,
            one.at.y,
            one.across - one.reach - one.carriage
        );
    }
    if found.is_empty() {
        return;
    }
    let _ = writeln!(
        file,
        "| where | settlement | across | carriageway | reach |\n|---|---|---|---|---|"
    );
    for one in found {
        let _ = writeln!(
            file,
            "| ({:.0}, {:.0}) | {} at ({:.0}, {:.0}) | {:.2} m | {:.2} m | {:.2} m |",
            one.at.x,
            one.at.y,
            if one.city { "city" } else { "village" },
            one.settlement.x,
            one.settlement.y,
            one.across,
            one.carriage * 2.0,
            one.reach,
        );
    }
}

pub struct AuditPlugin;

impl Plugin for AuditPlugin {
    fn build(&self, app: &mut App) {
        if !asked_for() {
            return;
        }
        app.insert_resource(Auditing { settling: 180 })
            .add_systems(
                Startup,
                |mut next: ResMut<NextState<crate::states::AppState>>| {
                    next.set(crate::states::AppState::Playing);
                },
            )
            .add_systems(
                Update,
                audit_the_streets.run_if(in_state(crate::states::AppState::Playing)),
            );
    }
}
