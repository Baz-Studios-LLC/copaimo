//! The terrain sculpting tool — a separate mode, entered from the main menu.
//!
//! A brush follows the crosshair across the ground and raises, lowers, smooths,
//! flattens, cuts paths through or roughens it. Edits go to the hand-edit layer
//! (`world/edit.rs`) as offsets on top of generated terrain, so sculpted
//! geography survives re-rolling the noise or swapping the map image.
//!
//! Affected chunks re-mesh live as you paint, reusing the same background task
//! machinery that streaming uses — the old mesh stays on screen until the new
//! one is ready, so the ground never blinks out from under you.
//!
//! **Where the brush itself lives.** Not here — in `terrain-core`, which
//! Opificium's terrain bench runs too. This module is the *mode*: aiming, the
//! gestures, the panel, and telling chunks to mesh again. That split is why the
//! tool could come back into the game at all, and it is how the studios do it:
//! the editor is built on top of the runtime, and the shaping code exists once.
//!
//! **Reuse.** This module plus `world/edit.rs` is the tool. Everything it needs
//! from the host project is narrow and listed here, so pointing it at another
//! world is a matter of supplying these rather than untangling it:
//!
//! * a heightfield to raycast and read — `Terrain::height` / `base_height`
//! * an offset grid to write — `Terrain::edits`
//! * a painted forest layer — `Terrain::woods`
//! * a way to invalidate meshes over a rectangle — `invalidate_area` below
//! * a camera to aim from — any entity with a `GlobalTransform`

mod minimap;
pub mod ui;

use bevy::prelude::*;

use crate::camera::{CameraMode, MainCamera};
use crate::config::{CHUNK_SIZE, EDIT_CELL};
use crate::states::AppState;
use crate::world::edit::{Brushing, Patch, Stamp};
use crate::world::chunk::Chunk;
use crate::world::stream::{plant_chunk, spawn_chunk_mesh, ChunkMap, Grove, PendingChunk};
use crate::world::terrain::{Terrain, TerrainSource};

/// How far the brush can reach from the camera, in meters.
const REACH: f32 = 800.0;
/// Ray march step. Small enough not to tunnel through a ridge, large enough
/// that a full-reach miss is a few hundred samples rather than thousands.
const MARCH_STEP: f32 = 1.5;

pub const MIN_RADIUS: f32 = 4.0;
pub const MAX_RADIUS: f32 = 500.0;
/// Radius changes by a proportion per wheel notch rather than a fixed number of
/// meters, so it feels the same whether you're shaping a mound or a range.
const RADIUS_STEP: f32 = 1.15;

/// How long unsaved work sits before it writes itself.
///
/// A crash, a stray Alt+F4, or simply forgetting is the one failure this tool
/// can inflict that cannot be undone, and an afternoon of shaping is a real
/// afternoon. Two minutes is short enough to lose nothing worth mourning and
/// long enough never to interrupt a stroke.
const AUTOSAVE_AFTER: f32 = 120.0;

/// How fast PATH wears the ground bare, against how fast it grades.
///
/// Higher than the grading rate on purpose: a road should LOOK like a road
/// within a second of holding the brush on it, where levelling the bumps out
/// underneath is allowed to take longer. Tied to the same stroke either way, so
/// one press takes back both.
const SURFACING_RATE: f32 = 2.5;

pub const MIN_STRENGTH: f32 = 2.0;
pub const MAX_STRENGTH: f32 = 150.0;
const STRENGTH_STEP: f32 = 1.25;

#[derive(Resource)]
pub struct Brush {
    pub radius: f32,
    /// What strength means depends on the tool — meters per second to the ones
    /// that push, a blend fraction to the ones that level, a settling pace to
    /// erosion. `Brushing::rate` is the one place that decides which.
    pub strength: f32,
    pub how: Brushing,
    /// Where the brush is currently pointed, if it's on the ground at all.
    pub hit: Option<Vec3>,
    /// Height captured when a levelling stroke began, so the whole stroke
    /// levels to one plane instead of chasing the ground as it moves.
    flatten_target: f32,
    /// Whether a stroke is currently open, for undo grouping.
    stroking: bool,
    /// Which layer the open stroke was started on. The tool can change
    /// mid-drag, so closing goes by this rather than by what is selected now.
    strokes: Layer,
    /// The first end of a ramp, once it has been clicked. A ramp is laid
    /// between two points rather than dragged, so it needs somewhere to
    /// remember the first one.
    pub ramp_from: Option<Vec3>,
    /// Which layer each stroke went to, oldest first.
    ///
    /// The ground and the woods keep their own histories, and neither can know
    /// about the other. Undo means "take back the last thing I did", so
    /// something has to remember what that was — otherwise pressing it after
    /// planting silently reaches past the wood and takes back a hillside.
    order: Vec<Layer>,
    /// Which country the BIOME brush lays. Chosen the way a colour is, by
    /// pressing the tool's own key again.
    laying: terrain_core::region::Country,
}

/// Which of the two painted layers a stroke touched.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Layer {
    Ground,
    Woods,
    /// A road: PATH grades the ground AND wears it bare, so one gesture writes
    /// two layers and one press has to take back both.
    ///
    /// Its own variant rather than a flag on `Ground`, because the two stacks
    /// have to stay in step: opening a surface group for every RAISE stroke
    /// would leave undo popping a road that was never laid.
    Road,
    /// Which country the ground is in. Its own layer for the same reason the
    /// woods are: it is a separate file with a separate history.
    Country,
}

/// As deep as either layer's own history. Beyond this the layer has forgotten
/// the stroke anyway, so remembering that it happened would only mislead.
const ORDER_DEPTH: usize = 64;

impl Default for Brush {
    fn default() -> Self {
        Self {
            radius: 40.0,
            strength: 25.0,
            how: Brushing::Raise,
            hit: None,
            flatten_target: 0.0,
            stroking: false,
            strokes: Layer::Ground,
            ramp_from: None,
            order: Vec::new(),
            laying: terrain_core::region::Country::Desert,
        }
    }
}

impl Brush {
    /// Remembers that a stroke landed, so undo knows where to look.
    fn stroked(&mut self, layer: Layer) {
        self.order.push(layer);
        if self.order.len() > ORDER_DEPTH {
            self.order.remove(0);
        }
    }
}

/// Whether the maker is holding the cursor free to reach the panels.
///
/// Sculpting captures the cursor — it has to, the brush aims down the view ray
/// and mouse-look needs the pointer out of the way. But an 8 km world cannot be
/// crossed by flying and hoping, and the overview on the right is the only thing
/// that knows where anything is. So ALT lets go: the pointer comes back, the
/// view stops turning, and the brush stops painting until it is released.
///
/// A modifier rather than a mode, because reaching for the map is a moment
/// inside the work and not a change of what you are doing.
#[derive(Resource, Default, Deref)]
pub struct CursorFree(pub bool);

/// What the maker has picked up and is carrying about, if anything.
///
/// # Carried, not dragged
///
/// This tool has no pointer to drag with — it aims down the view ray and the
/// crosshair IS the cursor — so moving something is picking it up and putting it
/// down again: `G` takes hold of what the ring is over, the thing follows the
/// crosshair, and `G` sets it down. It is also the gesture that needs no second
/// hand, which matters when the other one is on the fly keys.
///
/// # Only the DRAWN thing moves until it is set down
///
/// While something is carried, the sheet is left alone and the raised entity's own
/// transform is moved instead. Writing the sheet every frame would be correct and
/// unusable: every placed thing in the world is despawned and raised again whenever
/// the sheet changes, so carrying one house would rebuild a whole street sixty times
/// a second.
///
/// Setting it down writes once, which raises everything once. Cancelling writes
/// nothing and touches the sheet, so the truth on file puts the drawn thing back.
#[derive(Resource, Default)]
pub struct Carrying(pub Option<u32>);

/// Seconds of unsaved work, and whether leaving has already been questioned.
#[derive(Resource, Default)]
pub struct Keeping {
    unsaved_for: f32,
    /// Set when ESC was pressed with work outstanding. A second press leaves.
    asked: bool,
}

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Carrying>()
            .init_resource::<Brush>()
            .init_resource::<CursorFree>()
            .init_resource::<Keeping>()
            .add_plugins((ui::EditorUiPlugin, minimap::MinimapPlugin))
            .add_systems(OnEnter(AppState::Editing), enter_editor)
            .add_systems(
                Update,
                (
                    hold_to_reach,
                    aim_brush,
                    adjust_brush,
                    paint,
                    lay_ramp,
                    place_things,
                    // After the placing, so a thing picked up this frame follows the
                    // crosshair from the frame it is picked up on.
                    carry_things,
                    history,
                    save_edits,
                    keep_the_work,
                    draw_brush,
                )
                    .chain()
                    .run_if(in_state(AppState::Editing)),
            );
    }
}

/// ALT frees the pointer for as long as it is held.
fn hold_to_reach(
    keys: Res<ButtonInput<KeyCode>>,
    mut free: ResMut<CursorFree>,
    mut windows: Query<&mut Window, With<bevy::window::PrimaryWindow>>,
) {
    let wanted = keys.any_pressed([KeyCode::AltLeft, KeyCode::AltRight]);
    if wanted == free.0 {
        return;
    }
    free.0 = wanted;

    let Some(mut window) = windows.iter_mut().next() else {
        return;
    };
    window.cursor_options.grab_mode = if wanted {
        bevy::window::CursorGrabMode::None
    } else {
        bevy::window::CursorGrabMode::Confined
    };
    window.cursor_options.visible = wanted;
}

fn enter_editor(mut camera: ResMut<CameraMode>, mut free: ResMut<CursorFree>) {
    // Sculpting from the follow camera means aiming past your own warden at
    // whatever happens to be in front of them. Free-fly is what the tool wants.
    *camera = CameraMode::Fly;
    // And nobody arrives holding ALT. Left true from a previous visit, this
    // would disagree with the freshly captured cursor and `hold_to_reach`'s
    // early-out would keep the two apart until the next ALT press.
    free.0 = false;
}

/// Marches the camera's view ray until it goes under the ground, then binary
/// searches the last step for the crossing. Cheaper and simpler than colliding
/// against chunk meshes, and it works on terrain that hasn't been meshed yet.
fn raycast_terrain(terrain: &Terrain, origin: Vec3, direction: Vec3) -> Option<Vec3> {
    let mut previous = origin;
    let mut travelled = 0.0;

    while travelled < REACH {
        travelled += MARCH_STEP;
        let point = origin + direction * travelled;

        if point.y <= terrain.height(point.x, point.z) {
            // `low` is always above ground and `high` always below, so the
            // surface is bracketed and this converges on it.
            let (mut low, mut high) = (previous, point);
            for _ in 0..16 {
                let middle = (low + high) * 0.5;
                if middle.y <= terrain.height(middle.x, middle.z) {
                    high = middle;
                } else {
                    low = middle;
                }
            }
            return Some(high);
        }
        previous = point;
    }

    None
}

fn aim_brush(
    terrain: Res<TerrainSource>,
    cameras: Query<&GlobalTransform, With<MainCamera>>,
    mut brush: ResMut<Brush>,
) {
    let Some(camera) = cameras.iter().next() else {
        return;
    };
    brush.hit = raycast_terrain(&terrain.0, camera.translation(), camera.forward().as_vec3());
}

/// Which key selects each tool, in the order [`Brushing::ALL`] lists them.
///
/// **One table, read by the input and by the panel both.** It was two: the keys
/// here, and the panel numbering its rows `(index + 1) % 10` — which gave the
/// eleventh tool the label `1`, the same key as the first. A maker reading the
/// panel was told something untrue about the tool they had just added, and nothing
/// in either place could notice.
pub const TOOL_KEYS: [KeyCode; 11] = [
    KeyCode::Digit1,
    KeyCode::Digit2,
    KeyCode::Digit3,
    KeyCode::Digit4,
    KeyCode::Digit5,
    KeyCode::Digit6,
    KeyCode::Digit7,
    KeyCode::Digit8,
    KeyCode::Digit9,
    // Reverting sits on 0, past the nine that make things.
    KeyCode::Digit0,
    // And the biome brush on B, because the digits are full and because it is the
    // one tool that paints a decision about a whole region rather than shaping a
    // patch of ground.
    KeyCode::KeyB,
];

/// What to print on a tool's keycap.
pub fn key_for(how: Brushing) -> &'static str {
    let at = Brushing::ALL.iter().position(|one| *one == how);
    match at.and_then(|at| TOOL_KEYS.get(at)) {
        Some(KeyCode::Digit1) => "1",
        Some(KeyCode::Digit2) => "2",
        Some(KeyCode::Digit3) => "3",
        Some(KeyCode::Digit4) => "4",
        Some(KeyCode::Digit5) => "5",
        Some(KeyCode::Digit6) => "6",
        Some(KeyCode::Digit7) => "7",
        Some(KeyCode::Digit8) => "8",
        Some(KeyCode::Digit9) => "9",
        Some(KeyCode::Digit0) => "0",
        Some(KeyCode::KeyB) => "B",
        // A tool with no key is a row you press, which is a perfectly good tool.
        _ => "",
    }
}

fn adjust_brush(
    keys: Res<ButtonInput<KeyCode>>,
    scroll: Res<bevy::input::mouse::AccumulatedMouseScroll>,
    free: Res<CursorFree>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    panels: Query<(&ComputedNode, &GlobalTransform), With<crate::tools::widget::Scrolls>>,
    mut brush: ResMut<Brush>,
) {
    let notches = crate::util::wheel_notches(&scroll);
    // With ALT held and the pointer on the panel, the wheel is the panel's
    // scroll and must not also resize the brush behind it. Only then: while
    // sculpting the cursor is hidden but still MOVES with the mouse, so a bare
    // over-panel test would silently kill brush-resize whenever the invisible
    // pointer happened to be resting there.
    let reaching = free.0 && crate::tools::widget::pointer_on_a_panel(&windows, &panels);
    if notches != 0.0 && !reaching {
        brush.radius = (brush.radius * RADIUS_STEP.powf(notches)).clamp(MIN_RADIUS, MAX_RADIUS);
    }

    if keys.just_pressed(KeyCode::BracketRight) {
        brush.strength = (brush.strength * STRENGTH_STEP).min(MAX_STRENGTH);
    }
    if keys.just_pressed(KeyCode::BracketLeft) {
        brush.strength = (brush.strength / STRENGTH_STEP).max(MIN_STRENGTH);
    }

    for (key, how) in TOOL_KEYS.iter().zip(Brushing::ALL) {
        if keys.just_pressed(*key) {
            // Pressing B again cycles which country it lays, rather than
            // spending three more keys on three more tools. One brush, and the
            // country is chosen the way a colour is.
            if how.is_countrying() && brush.how == how {
                let all = terrain_core::region::Country::ALL;
                let next = all
                    .iter()
                    .position(|c| *c == brush.laying)
                    .map_or(0, |at| (at + 1) % all.len());
                brush.laying = all[next];
            }
            brush.how = how;
            // Half a ramp with a different tool selected would lay itself from
            // wherever it was left the next time the tool came back around.
            brush.ramp_from = None;
        }
    }
}

fn paint(
    mut commands: Commands,
    time: Res<Time>,
    buttons: Res<ButtonInput<MouseButton>>,
    terrain: Res<TerrainSource>,
    chunks: Res<ChunkMap>,
    busy: Query<(), With<PendingChunk>>,
    grove: Option<Res<Grove>>,
    standing: Query<Option<&Children>, With<Chunk>>,
    free: Res<CursorFree>,
    mut brush: ResMut<Brush>,
) {
    // The pointer is out reaching for a panel, not aimed at the ground.
    if free.0 {
        return;
    }
    // Laid between two clicked points, not dragged. `lay_ramp` has it.
    if brush.how.is_two_point() {
        return;
    }

    // Right button inverts the stroke, so raising and lowering — and planting
    // and clearing — are one gesture rather than a mode switch.
    let inverted = buttons.pressed(MouseButton::Right);
    let painting = buttons.pressed(MouseButton::Left) || inverted;

    // Open and close the undo group around the whole drag, so a stroke lasting
    // two hundred frames undoes in one step rather than two hundred. Whichever
    // layer the tool writes to keeps its own, and `brush.order` remembers which
    // it was so undo can find it again.
    let layer = if brush.how.is_countrying() {
        Layer::Country
    } else if brush.how.is_planting() {
        Layer::Woods
    } else if brush.how.is_surfacing() || brush.how.is_reverting() {
        // Reverting takes back both halves of a road. Putting the ground back
        // and leaving the dirt on top of it has not put anything back.
        Layer::Road
    } else {
        Layer::Ground
    };

    // Changing tool mid-drag moves the stroke to the other layer. Closing the
    // old one and opening a new one keeps both undoable; without it the rest of
    // the drag wrote to a layer with no group open and could not be taken back.
    if painting && brush.stroking && layer != brush.strokes {
        close_stroke(&terrain, brush.strokes);
        let closed = brush.strokes;
        brush.stroked(closed);
        brush.stroking = false;
    }

    if painting && !brush.stroking {
        match layer {
            Layer::Ground => {
                if let Ok(mut edits) = terrain.edits().write() {
                    edits.begin_stroke();
                }
            }
            Layer::Woods => {
                if let Ok(mut woods) = terrain.woods().write() {
                    woods.begin_stroke();
                }
            }
            Layer::Country => {
                if let Ok(mut countries) = terrain.countries().write() {
                    countries.begin_stroke();
                }
            }
            Layer::Road => {
                if let Ok(mut edits) = terrain.edits().write() {
                    edits.begin_stroke();
                }
                if let Ok(mut worn) = terrain.surface().write() {
                    worn.begin_stroke();
                }
            }
        }
        brush.stroking = true;
        brush.strokes = layer;
        brush.flatten_target = brush.hit.map_or(0.0, |hit| hit.y);
    } else if !painting && brush.stroking {
        // Closed against the layer the stroke OPENED on, not the one selected
        // now — the tool can be changed mid-drag, and closing the wrong layer
        // would leave a group open forever and lose the drag.
        close_stroke(&terrain, brush.strokes);
        let closed = brush.strokes;
        brush.stroked(closed);
        brush.stroking = false;
    }

    let Some(hit) = brush.hit.filter(|_| painting) else {
        return;
    };

    let how = match (brush.how, inverted) {
        (Brushing::Raise, true) => Brushing::Lower,
        (Brushing::Lower, true) => Brushing::Raise,
        (how, _) => how,
    };
    let amount = how.rate(brush.strength, time.delta_secs());
    let at = Vec2::new(hit.x, hit.z);

    // Planting touches the woods and never the ground. A brush that moved earth
    // as well would dig a hole every time somebody grew a stand of trees.
    let patch = if how.is_countrying() {
        let Ok(mut countries) = terrain.countries().write() else {
            return;
        };
        // Zero, stamped — NOT faded.
        //
        // The right button gets back to the world's own regions, and for every
        // other layer that means fading: a bias is a quantity, and easing it
        // toward nothing is exactly right.
        //
        // A country cannot be faded, because a mark is a NAME and not an amount.
        // Fading three toward nothing takes it through two and one on the way,
        // and two and one are other countries — so a snowfield being cleared
        // would read as desert, then as grassland, then as nothing. Stamping zero
        // means a cell only ever holds a country or no opinion at all, which is
        // the invariant the whole layer rests on.
        let mark = if inverted { 0.0 } else { brush.laying.mark() };
        countries.stamp(at, brush.radius, mark)
    } else if how.is_planting() {
        let Ok(mut woods) = terrain.woods().write() else {
            return;
        };
        // Negative bias clears, and zero leaves the ground's own answer alone —
        // which is why the right button thins a wood rather than paving it.
        woods.paint(at, brush.radius, if inverted { -amount } else { amount })
    } else {
        let Ok(mut edits) = terrain.edits().write() else {
            return;
        };
        // Reads the generator directly, never back through the edit layer —
        // that would deadlock against the write lock held right here.
        let under = |p: Vec2| terrain.base_height(p.x, p.y);
        edits.apply(&Stamp {
            centre: at,
            radius: brush.radius,
            how,
            amount,
            target: brush.flatten_target,
            under: &under,
        })
    };

    // A graded strip of grass is a lawn. What makes it a road is that it is
    // WORN, so PATH lays surface over exactly the ground it just graded — flat
    // across the bed and quick at the shoulders, the same profile it cuts.
    if how.is_surfacing() {
        if let Ok(mut worn) = terrain.surface().write() {
            let bare = if inverted { -amount } else { amount };
            worn.paint_with(at, brush.radius, bare * SURFACING_RATE, |away, radius| {
                crate::util::smoothstep(radius, radius * 0.7, away)
            });
        }
    }
    // And reverting takes it off again — faded to nothing rather than painted
    // green, so what is left is the biome's own answer and not another opinion.
    if how.is_reverting() {
        if let Ok(mut worn) = terrain.surface().write() {
            worn.fade(at, brush.radius, amount);
        }
    }

    // A country decides the ground's COLOUR, what grows on it and what is
    // littered across it, so a stroke has to rebuild the meshes and the woods
    // both — it is the one brush that changes every layer at once without
    // touching the heightfield.
    if how.is_countrying() {
        invalidate_area(&mut commands, &terrain, &chunks, &busy, patch);
        regrow_area(
            &mut commands,
            &terrain,
            &chunks,
            grove.as_deref(),
            &standing,
            patch,
        );
    } else if how.is_planting() {
        regrow_area(
            &mut commands,
            &terrain,
            &chunks,
            grove.as_deref(),
            &standing,
            patch,
        );
    } else {
        invalidate_area(&mut commands, &terrain, &chunks, &busy, patch);
    }
}

/// Closes an undo group on whichever layer it was opened on.
fn close_stroke(terrain: &TerrainSource, layer: Layer) {
    match layer {
        Layer::Ground => {
            if let Ok(mut edits) = terrain.edits().write() {
                edits.end_stroke();
            }
        }
        Layer::Woods => {
            if let Ok(mut woods) = terrain.woods().write() {
                woods.end_stroke();
            }
        }
        Layer::Country => {
            if let Ok(mut countries) = terrain.countries().write() {
                countries.end_stroke();
            }
        }
        Layer::Road => {
            // Both, together — a road is one thing to whoever laid it.
            if let Ok(mut edits) = terrain.edits().write() {
                edits.end_stroke();
            }
            if let Ok(mut worn) = terrain.surface().write() {
                worn.end_stroke();
            }
        }
    }
}

/// The ramp: click once for the foot, once for the head, and a graded run is
/// laid between them in a single stroke.
///
/// A gesture of its own because the levelling brushes pull toward ONE height,
/// which is right for a town square and useless for a road up a hillside. The
/// right button cancels a half-placed one.
fn lay_ramp(
    mut commands: Commands,
    buttons: Res<ButtonInput<MouseButton>>,
    terrain: Res<TerrainSource>,
    chunks: Res<ChunkMap>,
    busy: Query<(), With<PendingChunk>>,
    free: Res<CursorFree>,
    mut brush: ResMut<Brush>,
    mut toast: ResMut<ui::Toast>,
) {
    if free.0 || !brush.how.is_two_point() {
        return;
    }

    if buttons.just_pressed(MouseButton::Right) && brush.ramp_from.take().is_some() {
        toast.show("Ramp cancelled");
        return;
    }
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }
    let Some(hit) = brush.hit else {
        return;
    };

    let Some(from) = brush.ramp_from.take() else {
        brush.ramp_from = Some(hit);
        toast.show("Ramp: now click the far end");
        return;
    };

    let patch = {
        let Ok(mut edits) = terrain.edits().write() else {
            return;
        };
        let under = |p: Vec2| terrain.base_height(p.x, p.y);
        // Laid inside a stroke of its own so one press takes the whole run back,
        // the same as a drag with any other tool.
        edits.begin_stroke();
        let patch = edits.ramp(from, hit, brush.radius, &under);
        edits.end_stroke();
        patch
    };

    brush.stroked(Layer::Ground);
    toast.show(format!("Ramp laid, {:.0} m", from.distance(hit)));
    invalidate_area(&mut commands, &terrain, &chunks, &busy, patch);
}

fn history(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    terrain: Res<TerrainSource>,
    chunks: Res<ChunkMap>,
    busy: Query<(), With<PendingChunk>>,
    mut brush: ResMut<Brush>,
    mut toast: ResMut<ui::Toast>,
) {
    let control = keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    if !control {
        return;
    }
    let shift = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);

    // Ctrl+Z undoes; Ctrl+Y and Ctrl+Shift+Z both redo, since both conventions
    // are in common use and neither costs anything to support.
    let redo = keys.just_pressed(KeyCode::KeyY) || (shift && keys.just_pressed(KeyCode::KeyZ));
    let undo = !shift && keys.just_pressed(KeyCode::KeyZ);
    if !undo && !redo {
        return;
    }

    // Which layer to reach into. Undoing takes the last stroke off the end;
    // redoing puts one back, so it goes to whichever layer would be next.
    let layer = if undo {
        // Read, don't take. If the layer turns out to have nothing left — its
        // own history is shallower than this record — the record is stale and
        // consuming it would silently shift every earlier undo onto the wrong
        // layer. It is dropped below, once the undo is known to have happened.
        brush.order.last().copied()
    } else {
        let ground = terrain.edits().read().is_ok_and(|edits| edits.can_redo());
        let woods = terrain.woods().read().is_ok_and(|woods| woods.can_redo());
        let country = terrain.countries().read().is_ok_and(|them| them.can_redo());
        let wear = terrain.surface().read().is_ok_and(|worn| worn.can_redo());
        // Every layer that keeps a history, or a stroke on the missing one can
        // never be redone: the biome brush and the road's worn surface were left
        // out of this list, so Ctrl+Y answered "Nothing to redo" while their
        // histories sat there holding exactly that. A road is ground AND wear
        // together — when both are redoable it is a road stroke, and redoing
        // only the grading would put back half a road.
        //
        // Ties are ambiguous after undoing across layers, and the ground wins
        // them because it is what most of the work is.
        if ground && wear {
            Some(Layer::Road)
        } else if ground {
            Some(Layer::Ground)
        } else if woods {
            Some(Layer::Woods)
        } else if country {
            Some(Layer::Country)
        } else if wear {
            Some(Layer::Road)
        } else {
            None
        }
    };

    let patch = match layer {
        Some(Layer::Ground) => terrain.edits().write().ok().and_then(|mut edits| {
            if undo {
                edits.undo()
            } else {
                edits.redo()
            }
        }),
        Some(Layer::Woods) => terrain.woods().write().ok().and_then(|mut woods| {
            if undo {
                woods.undo()
            } else {
                woods.redo()
            }
        }),
        Some(Layer::Country) => terrain.countries().write().ok().and_then(|mut them| {
            if undo {
                them.undo()
            } else {
                them.redo()
            }
        }),
        Some(Layer::Road) => {
            let ground = terrain.edits().write().ok().and_then(|mut edits| {
                if undo {
                    edits.undo()
                } else {
                    edits.redo()
                }
            });
            let wear = terrain.surface().write().ok().and_then(|mut worn| {
                if undo {
                    worn.undo()
                } else {
                    worn.redo()
                }
            });
            // Whichever of the two moved, and both if both did.
            match (ground, wear) {
                (Some(a), Some(b)) => Some((a.0.min(b.0), a.1.max(b.1))),
                (only, None) | (None, only) => only,
            }
        }
        None => None,
    };

    match patch {
        Some(patch) => {
            match (undo, layer) {
                (true, _) => {
                    brush.order.pop();
                }
                (false, Some(layer)) => brush.stroked(layer),
                (false, None) => {}
            }
            toast.show(match (undo, layer) {
                // Named, because taking back a hillside and taking back a wood
                // look nothing alike and the wood may be behind you.
                (true, Some(Layer::Woods)) => "Planting undone",
                (true, Some(Layer::Country)) => "Biome undone",
                (true, _) => "Undone",
                (false, Some(Layer::Woods)) => "Planting redone",
                (false, Some(Layer::Country)) => "Biome redone",
                (false, _) => "Redone",
            });
            invalidate_area(&mut commands, &terrain, &chunks, &busy, patch);
        }
        // Say so rather than doing nothing — a dead shortcut and an empty
        // history look identical otherwise.
        None => toast.show(if undo {
            "Nothing to undo"
        } else {
            "Nothing to redo"
        }),
    }
}

/// Queues a re-mesh of every loaded chunk overlapping a world-space rectangle.
///
/// The edit layer is sampled bilinearly, so a change influences one cell beyond
/// its own bounds — hence the margin, without which chunk seams would drift
/// apart along the edge of a stroke.
fn invalidate_area(
    commands: &mut Commands,
    terrain: &TerrainSource,
    chunks: &ChunkMap,
    busy: &Query<(), With<PendingChunk>>,
    patch: Patch,
) {
    for (entity, coord) in chunks_over(chunks, patch) {
        // Already rebuilding: skip rather than queue a second task. This is what
        // throttles painting — a chunk rebuilds as fast as it can and no faster,
        // however many frames the stroke lasts.
        if busy.contains(entity) {
            continue;
        }
        spawn_chunk_mesh(commands, entity, terrain, coord);
    }
}

/// Regrows the wood over a rectangle, leaving the ground alone.
///
/// Planting does not move earth, so sending it through `invalidate_area` meant
/// rebuilding a whole chunk mesh — a hundred thousand terrain samples — to show
/// one tree. Worse, it went through the same one-rebuild-at-a-time throttle the
/// brush uses, so a wide stroke found most of its chunks busy and dropped them
/// entirely. Trees appeared slowly, or not at all.
fn regrow_area(
    commands: &mut Commands,
    terrain: &TerrainSource,
    chunks: &ChunkMap,
    grove: Option<&Grove>,
    standing: &Query<Option<&Children>, With<Chunk>>,
    patch: Patch,
) {
    let Some(grove) = grove else {
        return;
    };
    for (entity, coord) in chunks_over(chunks, patch) {
        // Clear what is standing before growing what stands there now. The
        // clearing moved out of `plant_chunk` so that a chunk being re-meshed
        // sheds its water as well as its wood, so each caller does its own.
        if let Some(wood) = standing.get(entity).ok().flatten() {
            for old in wood.iter() {
                commands.entity(old).despawn();
            }
        }
        plant_chunk(commands, entity, coord, terrain, grove);
    }
}

/// The loaded chunks a world-space rectangle touches.
///
/// The edit layer is sampled bilinearly, so a change influences one cell beyond
/// its own bounds — hence the margin, without which chunk seams would drift
/// apart along the edge of a stroke.
fn chunks_over(chunks: &ChunkMap, patch: Patch) -> Vec<(Entity, IVec2)> {
    let (min, max) = patch;
    let low = ((min - EDIT_CELL) / CHUNK_SIZE).floor().as_ivec2();
    let high = ((max + EDIT_CELL) / CHUNK_SIZE).floor().as_ivec2();

    let mut touched = Vec::new();
    for z in low.y..=high.y {
        for x in low.x..=high.x {
            let coord = IVec2::new(x, z);
            if let Some(&entity) = chunks.loaded.get(&coord) {
                touched.push((entity, coord));
            }
        }
    }
    touched
}

fn draw_brush(mut gizmos: Gizmos, terrain: Res<TerrainSource>, brush: Res<Brush>) {
    let Some(hit) = brush.hit else {
        return;
    };

    // Rings sampled at ground height rather than a flat circle, so on a slope
    // the brush wraps the terrain and you can see exactly what a stroke covers.
    const SEGMENTS: usize = 72;
    let colour = crate::tools::theme::tool_color(brush.how);

    let point_at = |index: usize, radius: f32| {
        let angle = index as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
        let x = hit.x + angle.cos() * radius;
        let z = hit.z + angle.sin() * radius;
        Vec3::new(x, terrain.height(x, z) + 0.4, z)
    };

    let mut ring = |radius: f32, colour: Color| {
        let mut previous = point_at(0, radius);
        for index in 1..=SEGMENTS {
            let next = point_at(index, radius);
            gizmos.line(previous, next, colour);
            previous = next;
        }
    };

    // The rim, and then where the brush is still pulling hard.
    //
    // A single outline says where a stroke STOPS and nothing about what it does
    // in between — and every tool here fades from the middle out, so the edge is
    // the one part that barely moves. Rings drawn at fixed fractions of the
    // brush's own falloff make the shape of it visible: bunched near the rim for
    // a soft dish, out at the shoulder for a road's flat bed.
    ring(brush.radius, colour);
    for strength in FALLOFF_RINGS {
        let radius = falloff_radius(brush.how, brush.radius, strength);
        if radius <= 0.5 {
            continue;
        }
        // Dimmer the weaker the brush is there, so the gradient reads as one.
        ring(radius, colour.with_alpha(0.18 + strength * 0.42));
    }

    // A short mast at the centre, so the brush is findable when the ring falls
    // out of view behind a rise.
    gizmos.line(hit, hit + Vec3::Y * 3.0, colour);

    // Half a ramp is invisible otherwise: the first click lands somewhere behind
    // you and there is nothing on screen saying a run is waiting on its far end.
    if let Some(from) = brush.ramp_from {
        gizmos.line(from, from + Vec3::Y * 6.0, colour);
        gizmos.line(from, hit, colour.with_alpha(0.7));
    }
}

/// Where the brush is still pulling this hard, as fractions of full strength.
const FALLOFF_RINGS: [f32; 3] = [0.25, 0.5, 0.75];

/// The radius at which a tool's falloff has fallen to `strength`.
///
/// Solved by bisection rather than by inverting each curve. There are two curves
/// today and inverting them by hand would be two more places to keep in step
/// with `Brushing::falloff` — which is private, and rightly so. Sixteen steps
/// over a monotonic curve is exact to well under a pixel and runs once a frame.
fn falloff_radius(how: Brushing, radius: f32, strength: f32) -> f32 {
    let (mut near, mut far) = (0.0, radius);
    for _ in 0..16 {
        let middle = (near + far) * 0.5;
        if how.strength_at(middle, radius) > strength {
            near = middle;
        } else {
            far = middle;
        }
    }
    (near + far) * 0.5
}

fn save_edits(
    keys: Res<ButtonInput<KeyCode>>,
    terrain: Res<TerrainSource>,
    mut placed: ResMut<crate::world::placed::Standing>,
    mut keeping: ResMut<Keeping>,
    mut toast: ResMut<ui::Toast>,
) {
    let pressed_save = keys.just_pressed(KeyCode::KeyS)
        && keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    if !pressed_save {
        return;
    }
    keeping.unsaved_for = 0.0;
    keeping.asked = false;

    // Ground and woods together, in one keystroke. They were separate once and
    // planting quietly failed to survive a restart; a maker who has just spent
    // an afternoon on a hillside should not have to know there are two files.
    let ground = {
        let Ok(mut edits) = terrain.edits().write() else {
            return;
        };
        crate::world::edit::save(&mut edits).map(|()| edits.sculpted_cells())
    };
    let woods = {
        let Ok(mut painted) = terrain.woods().write() else {
            return;
        };
        crate::world::forest::save(&mut painted).map(|()| painted.painted_cells())
    };
    let worn = {
        let Ok(mut painted) = terrain.surface().write() else {
            return;
        };
        crate::world::surface::save(&mut painted).map(|()| painted.painted_cells())
    };
    let countries = {
        let Ok(mut painted) = terrain.countries().write() else {
            return;
        };
        crate::world::country::save(&mut painted).map(|()| painted.painted_cells())
    };
    let built = crate::world::placed::save(&mut placed).map(|()| placed.len());

    match (ground, woods, worn, countries, built) {
        (Ok(cells), Ok(planted), Ok(laid), Ok(marked), Ok(stood)) => {
            info!(
                "saved {cells} sculpted, {planted} planted, {laid} surfaced, {marked} biome, {stood} placed"
            );
            toast.show(format!(
                "Saved {cells} sculpted, {planted} planted, {laid} surfaced, {marked} biome, {stood} placed"
            ));
        }
        // Said separately, because which one failed decides what was lost.
        (.., Err(why)) => {
            error!("could not save what is placed: {why}");
            toast.show("Placed things not saved - see log");
        }
        (Err(why), ..) => {
            error!("could not save the sculpted ground: {why}");
            toast.show("Ground not saved - see log");
        }
        (_, Err(why), ..) => {
            error!("could not save the planted woods: {why}");
            toast.show("Woods not saved - see log");
        }
        (_, _, Err(why), ..) => {
            error!("could not save the worn surface: {why}");
            toast.show("Surface not saved - see log");
        }
        (_, _, _, Err(why), _) => {
            error!("could not save the painted biomes: {why}");
            toast.show("Biomes not saved - see log");
        }
    }
}

/// Whether any layer is holding work that is not on disk.
fn anything_outstanding(terrain: &TerrainSource, placed: &crate::world::placed::Standing) -> bool {
    let ground = terrain.edits().read().is_ok_and(|edits| edits.unsaved);
    let woods = terrain.woods().read().is_ok_and(|woods| woods.unsaved);
    let worn = terrain.surface().read().is_ok_and(|worn| worn.unsaved);
    let marked = terrain.countries().read().is_ok_and(|them| them.unsaved);
    ground || woods || worn || marked || placed.unsaved
}

/// Writes every layer, and says what it wrote.
fn write_everything(
    terrain: &TerrainSource,
    placed: &mut crate::world::placed::Standing,
) -> Result<(usize, usize, usize, usize, usize), String> {
    let cells = {
        let mut edits = terrain.edits().write().map_err(|_| "ground locked".to_string())?;
        crate::world::edit::save(&mut edits).map_err(|why| why.to_string())?;
        edits.sculpted_cells()
    };
    let planted = {
        let mut woods = terrain.woods().write().map_err(|_| "woods locked".to_string())?;
        crate::world::forest::save(&mut woods).map_err(|why| why.to_string())?;
        woods.painted_cells()
    };
    let laid = {
        let mut worn = terrain.surface().write().map_err(|_| "surface locked".to_string())?;
        crate::world::surface::save(&mut worn).map_err(|why| why.to_string())?;
        worn.painted_cells()
    };
    let marked = {
        let mut them = terrain
            .countries()
            .write()
            .map_err(|_| "biomes locked".to_string())?;
        crate::world::country::save(&mut them).map_err(|why| why.to_string())?;
        them.painted_cells()
    };
    let stood = {
        crate::world::placed::save(placed).map_err(|why| why.to_string())?;
        placed.len()
    };
    Ok((cells, planted, laid, marked, stood))
}

/// Keeps the work: writes it on its own after a while, and refuses to let ESC
/// throw it away without saying so.
///
/// The one failure this tool can inflict that no undo reaches is losing the
/// afternoon — to a crash, to a stray keypress, or to walking away. Neither half
/// of this is clever and both are the difference between a tool people trust and
/// one they back up by hand.
fn keep_the_work(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    terrain: Res<TerrainSource>,
    mut placed: ResMut<crate::world::placed::Standing>,
    mut carrying: ResMut<Carrying>,
    mut keeping: ResMut<Keeping>,
    mut next: ResMut<NextState<AppState>>,
    mut toast: ResMut<ui::Toast>,
) {
    let outstanding = anything_outstanding(&terrain, &placed);
    if !outstanding {
        keeping.unsaved_for = 0.0;
        keeping.asked = false;
    } else {
        keeping.unsaved_for += time.delta_secs();
    }

    // ESC with work outstanding asks once. A second press leaves anyway —
    // it is the maker's world and their decision, and a dialog that cannot be
    // dismissed is worse than a lost afternoon.
    if keys.just_pressed(KeyCode::Escape) {
        // Something in hand is put back before anything else is considered. Leaving
        // the tool with a house held over the wrong hill would write it there on the
        // next save, and ESC is the key everybody presses to mean "no".
        if let Some(id) = carrying.0.take() {
            let what = placed.get(id).map(|t| t.kind.clone()).unwrap_or_default();
            // Nothing was written while it was carried, so touching the sheet is
            // enough: everything is raised again from what is actually on file, and
            // the thing goes back where it was.
            let _ = placed.as_mut();
            toast.show(format!("Put the {what} back"));
            return;
        }
        if outstanding && !keeping.asked {
            keeping.asked = true;
            toast.show("Unsaved - Ctrl+S to keep it, Esc again to leave");
        } else {
            next.set(AppState::Menu);
        }
        return;
    }

    if !outstanding || keeping.unsaved_for < AUTOSAVE_AFTER {
        return;
    }
    keeping.unsaved_for = 0.0;
    match write_everything(&terrain, &mut placed) {
        Ok((cells, planted, laid, marked, stood)) => {
            info!(
                "kept the work: {cells} sculpted, {planted} planted, {laid} surfaced, {marked} biome, {stood} placed"
            );
            toast.show("Kept the work");
        }
        Err(why) => {
            error!("could not keep the work: {why}");
            toast.show("Autosave failed - see log");
        }
    }
}

/// Puts a thing in the world, and takes one back out.
///
/// # The smallest honest end of the placed sheet
///
/// A file format nothing can write to is not a feature, it is a plan. This is the
/// least that makes [`crate::world::placed`] real: `P` stands the next thing in
/// the catalogue where the brush is pointing, `Delete` takes back whichever is
/// nearest, and both are written with everything else on save.
///
/// It is deliberately not the workbench. There is no gizmo, no snapping and no
/// picking a piece from a shelf — that is the next job, and it can be built on
/// this without any of this changing.
fn place_things(
    keys: Res<ButtonInput<KeyCode>>,
    catalogue: Res<crate::build::Catalogue>,
    brush: Res<Brush>,
    free: Res<CursorFree>,
    mut placed: ResMut<crate::world::placed::Standing>,
    mut carrying: ResMut<Carrying>,
    mut choosing: Local<usize>,
    mut toast: ResMut<ui::Toast>,
) {
    if free.0 {
        return;
    }
    let Some(hit) = brush.hit else {
        return;
    };
    let at = Vec2::new(hit.x, hit.z);

    if keys.just_pressed(KeyCode::Delete) || keys.just_pressed(KeyCode::Backspace) {
        // What is in hand goes first, wherever the ring happens to be: somebody
        // holding a thing and pressing delete means THIS one.
        if let Some(id) = carrying.0.take() {
            let what = placed.get(id).map(|t| t.kind.clone()).unwrap_or_default();
            placed.remove(id);
            toast.show(format!("Took away the {what}"));
            return;
        }
        // Otherwise within the brush, so what gets taken away is what the ring is
        // over — the same rule every other tool here follows.
        match placed.nearest(at, brush.radius) {
            Some(id) => {
                let what = placed.get(id).map(|t| t.kind.clone()).unwrap_or_default();
                placed.remove(id);
                toast.show(format!("Took away the {what}"));
            }
            None => toast.show("Nothing of yours under the brush"),
        }
        return;
    }

    // Picking something up, and putting it down again.
    if keys.just_pressed(KeyCode::KeyG) {
        match carrying.0.take() {
            // Setting it down: the sheet is written once, here, which is what
            // raises everything again in its new place.
            Some(id) => {
                let what = placed.get(id).map(|t| t.kind.clone()).unwrap_or_default();
                if let Some(thing) = placed.get_mut(id) {
                    thing.at = at;
                }
                toast.show(format!("Set the {what} down"));
            }
            None => match placed.nearest(at, brush.radius) {
                Some(id) => {
                    let what = placed.get(id).map(|t| t.kind.clone()).unwrap_or_default();
                    carrying.0 = Some(id);
                    toast.show(format!("Carrying the {what} - G to set it down"));
                }
                None => toast.show("Nothing of yours under the brush"),
            },
        }
        return;
    }

    // Turning what is already down. R goes one way, SHIFT+R the other.
    //
    // Quarters, like the kit's own turns: a building three degrees off its street
    // is a mistake that reads as one and takes a while to find. The sheet stores
    // radians, so anything that genuinely wants a finer angle — a boulder — can
    // still hold one; nothing here has asked yet.
    if keys.just_pressed(KeyCode::KeyR) {
        let widdershins = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
        let by = std::f32::consts::FRAC_PI_2 * if widdershins { -1.0 } else { 1.0 };
        match placed.nearest(at, brush.radius) {
            Some(id) => {
                let what = placed.get(id).map(|t| t.kind.clone()).unwrap_or_default();
                let facing = placed.turn(id, by).unwrap_or(0.0);
                // Said in quarters rather than in radians, which is how it was
                // asked for and not how it is stored.
                let quarter = (facing / std::f32::consts::FRAC_PI_2).round() as i32 % 4;
                let way = ["north", "east", "south", "west"][quarter.rem_euclid(4) as usize];
                toast.show(format!("The {what} faces {way}"));
            }
            None => toast.show("Nothing of yours under the brush"),
        }
        return;
    }

    if !keys.just_pressed(KeyCode::KeyP) {
        return;
    }
    if catalogue.0.is_empty() {
        // Said rather than ignored. An empty buildings folder is the ordinary
        // state of this world right now, and a key that silently does nothing
        // reads as a broken key.
        toast.show("Nothing in the catalogue to place");
        return;
    }

    // Cycling through the catalogue on repeated presses, so a second press puts
    // down something different rather than the same house twice.
    let plan = &catalogue.0[*choosing % catalogue.0.len()];
    *choosing += 1;
    // Facing north to begin with, and R turns it from there. Guessing at placement
    // — at the camera, say — would be a decision somebody has to undo rather than
    // one they asked for; a known starting point they can turn in one keypress is
    // not the same thing as being stuck with it.
    placed.add(plan.name.clone(), at, 0.0, 1.0);
    toast.show(format!("Placed a {}", plan.name));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::placed::Standing;

    /// The placing tool, with everything it reads.
    ///
    /// A real app running the real system, because what went wrong in this codebase
    /// has twice been the wiring rather than the arithmetic — a key that nothing
    /// listened for, a guard that fired at the wrong moment.
    fn placing_app() -> App {
        let mut app = App::new();
        app.init_resource::<Brush>()
            .init_resource::<Carrying>()
            .init_resource::<CursorFree>()
            .init_resource::<Standing>()
            .init_resource::<crate::build::Catalogue>()
            .init_resource::<ui::Toast>()
            .init_resource::<ButtonInput<KeyCode>>()
            .add_systems(Update, place_things);
        // Pointed at the ground, as the brush is whenever it is over the world.
        app.world_mut().resource_mut::<Brush>().hit = Some(Vec3::new(10.0, 0.0, -4.0));
        app
    }

    fn press(app: &mut App, keys: &[KeyCode]) {
        {
            let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            for key in keys {
                input.press(*key);
            }
        }
        app.update();
        let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        for key in keys {
            input.release(*key);
        }
        input.clear();
    }

    fn facing(app: &App, id: u32) -> f32 {
        app.world().resource::<Standing>().get(id).expect("still there").turn
    }

    #[test]
    fn r_turns_what_is_under_the_brush_and_shift_r_turns_it_back() {
        let mut app = placing_app();
        let id = app
            .world_mut()
            .resource_mut::<Standing>()
            .add("cottage", Vec2::new(10.0, -4.0), 0.0, 1.0);

        let quarter = std::f32::consts::FRAC_PI_2;
        press(&mut app, &[KeyCode::KeyR]);
        assert!(
            (facing(&app, id) - quarter).abs() < 1.0e-4,
            "R left it facing {}",
            facing(&app, id)
        );

        press(&mut app, &[KeyCode::ShiftLeft, KeyCode::KeyR]);
        assert!(
            facing(&app, id).abs() < 1.0e-4 || (facing(&app, id) - std::f32::consts::TAU).abs() < 1.0e-4,
            "Shift+R left it facing {} instead of back where it started",
            facing(&app, id)
        );
    }

    #[test]
    fn turning_reaches_what_the_ring_is_over_and_nothing_else() {
        // The same rule every other tool here follows: what the brush is over.
        let mut app = placing_app();
        let (near, far) = {
            let mut standing = app.world_mut().resource_mut::<Standing>();
            let near = standing.add("cottage", Vec2::new(10.0, -4.0), 0.0, 1.0);
            // Well outside the default brush radius.
            let far = standing.add("barn", Vec2::new(600.0, 600.0), 0.0, 1.0);
            (near, far)
        };

        press(&mut app, &[KeyCode::KeyR]);
        assert!(facing(&app, near) > 0.0, "the near cottage did not turn");
        assert_eq!(facing(&app, far), 0.0, "a thing across the map turned too");

        // And with the brush off the ground entirely, nothing turns.
        app.world_mut().resource_mut::<Brush>().hit = None;
        let was = facing(&app, near);
        press(&mut app, &[KeyCode::KeyR]);
        assert_eq!(facing(&app, near), was, "it turned with the brush pointing at nothing");
    }

    #[test]
    fn turning_holds_still_while_the_pointer_is_free() {
        // ALT lets go of the cursor to reach the panels, and every gesture here is
        // off while it is held — a key that acted anyway would fire while somebody
        // was clicking a row.
        let mut app = placing_app();
        let id = app
            .world_mut()
            .resource_mut::<Standing>()
            .add("cottage", Vec2::new(10.0, -4.0), 0.0, 1.0);
        app.world_mut().resource_mut::<CursorFree>().0 = true;

        press(&mut app, &[KeyCode::KeyR]);
        assert_eq!(facing(&app, id), 0.0, "it turned while the pointer was free");
    }
}

/// Carries whatever is in hand along under the crosshair.
///
/// Moves the DRAWN thing and not the sheet — see [`Carrying`] for why. On the ground
/// under the cursor plus its own lift, so a thing carried over a hill rides up it
/// rather than sinking into it.
fn carry_things(
    carrying: Res<Carrying>,
    brush: Res<Brush>,
    terrain: Res<TerrainSource>,
    placed: Res<crate::world::placed::Standing>,
    mut raised: Query<(&crate::build::FromSheet, &mut Transform)>,
) {
    let (Some(id), Some(hit)) = (carrying.0, brush.hit) else {
        return;
    };
    let Some(thing) = placed.get(id) else {
        return;
    };
    // The DRAWN height, so it rides the ground as the maker has sculpted it rather
    // than the ground the noise would have given.
    let ground = terrain.0.drawn_height(hit.x, hit.z);
    for (from, mut stance) in &mut raised {
        if from.0 == id {
            stance.translation = Vec3::new(hit.x, ground + thing.lift, hit.z);
        }
    }
}

#[cfg(test)]
mod carrying {
    use super::*;
    use crate::build::FromSheet;
    use crate::world::placed::Standing;

    /// The placing tool AND the carrying, because picking a thing up and having it
    /// follow the crosshair are two systems and a bug would live between them.
    fn carrying_app() -> App {
        let mut app = App::new();
        app.add_plugins(bevy::state::app::StatesPlugin)
            .init_state::<AppState>()
            .init_resource::<Brush>()
            .init_resource::<Carrying>()
            .init_resource::<CursorFree>()
            .init_resource::<Standing>()
            .init_resource::<crate::build::Catalogue>()
            .init_resource::<Keeping>()
            .init_resource::<ui::Toast>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<Time>()
            .insert_resource(TerrainSource(std::sync::Arc::new(Terrain::new())))
            .add_systems(Update, (place_things, carry_things, keep_the_work).chain());
        app
    }

    fn press(app: &mut App, keys: &[KeyCode]) {
        {
            let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            for key in keys {
                input.press(*key);
            }
        }
        app.update();
        let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        for key in keys {
            input.release(*key);
        }
        input.clear();
    }

    /// Points the brush at a spot on the ground.
    fn aim(app: &mut App, at: Vec2) {
        app.world_mut().resource_mut::<Brush>().hit = Some(Vec3::new(at.x, 0.0, at.y));
    }

    fn sheet_at(app: &App, id: u32) -> Vec2 {
        app.world()
            .resource::<Standing>()
            .get(id)
            .expect("still there")
            .at
    }

    #[test]
    fn a_thing_is_picked_up_carried_and_set_down_somewhere_else() {
        let mut app = carrying_app();
        let was = Vec2::new(10.0, -4.0);
        let id = app
            .world_mut()
            .resource_mut::<Standing>()
            .add("cottage", was, 0.0, 1.0);
        // The drawn thing, as `raise_the_placed` spawns it.
        let drawn = app
            .world_mut()
            .spawn((FromSheet(id), Transform::from_xyz(was.x, 0.0, was.y)))
            .id();

        aim(&mut app, was);
        press(&mut app, &[KeyCode::KeyG]);
        assert_eq!(
            app.world().resource::<Carrying>().0,
            Some(id),
            "G did not pick it up"
        );

        // Carried: the DRAWN thing follows and the sheet is left alone, or every
        // placed thing in the world would be rebuilt sixty times a second.
        let there = Vec2::new(90.0, 30.0);
        aim(&mut app, there);
        app.update();
        let stance = *app.world().get::<Transform>(drawn).expect("still drawn");
        assert!(
            (stance.translation.x - there.x).abs() < 0.01
                && (stance.translation.z - there.y).abs() < 0.01,
            "the carried thing did not follow the crosshair: {:?}",
            stance.translation
        );
        assert_eq!(
            sheet_at(&app, id),
            was,
            "the sheet was written while carrying"
        );

        // Set down: the sheet is written once, here.
        press(&mut app, &[KeyCode::KeyG]);
        assert_eq!(app.world().resource::<Carrying>().0, None);
        assert_eq!(sheet_at(&app, id), there, "setting it down did not move it");
        assert!(app.world().resource::<Standing>().unsaved);
    }

    #[test]
    fn escape_puts_a_carried_thing_back_rather_than_leaving_the_tool() {
        // ESC is the key everybody presses to mean "no", and leaving the tool with a
        // house held over the wrong hill would write it there at the next save.
        let mut app = carrying_app();
        let was = Vec2::new(10.0, -4.0);
        let id = app
            .world_mut()
            .resource_mut::<Standing>()
            .add("cottage", was, 0.0, 1.0);

        aim(&mut app, was);
        press(&mut app, &[KeyCode::KeyG]);
        aim(&mut app, Vec2::new(200.0, 200.0));
        app.update();

        press(&mut app, &[KeyCode::Escape]);
        assert_eq!(
            app.world().resource::<Carrying>().0,
            None,
            "ESC did not put it down"
        );
        assert_eq!(sheet_at(&app, id), was, "ESC moved it anyway");
        // Nothing ASKED to leave, which is the honest question here: state
        // transitions are applied before Update, so a request made in this frame is
        // still pending — and the harness starts in the default state anyway, so
        // reading the current one would prove nothing either way.
        assert!(
            matches!(
                *app.world().resource::<NextState<AppState>>(),
                NextState::Unchanged
            ),
            "ESC asked to leave the tool while something was in hand"
        );

        // And once it is put down, ESC goes back to meaning what it meant: the
        // first press asks about the unsaved sheet, the second leaves. Three
        // presses, three different answers, in the order a maker would want them.
        press(&mut app, &[KeyCode::Escape]);
        assert!(
            matches!(
                *app.world().resource::<NextState<AppState>>(),
                NextState::Unchanged
            ),
            "ESC left with the sheet unsaved and unasked about"
        );
        press(&mut app, &[KeyCode::Escape]);
        assert!(
            !matches!(
                *app.world().resource::<NextState<AppState>>(),
                NextState::Unchanged
            ),
            "ESC never leaves the tool at all now"
        );
    }

    #[test]
    fn nothing_under_the_brush_picks_nothing_up() {
        let mut app = carrying_app();
        app.world_mut()
            .resource_mut::<Standing>()
            .add("cottage", Vec2::new(10.0, -4.0), 0.0, 1.0);

        // Well outside the brush.
        aim(&mut app, Vec2::new(900.0, 900.0));
        press(&mut app, &[KeyCode::KeyG]);
        assert_eq!(app.world().resource::<Carrying>().0, None);

        // And with the pointer free — ALT held to reach the panels — G holds still.
        aim(&mut app, Vec2::new(10.0, -4.0));
        app.world_mut().resource_mut::<CursorFree>().0 = true;
        press(&mut app, &[KeyCode::KeyG]);
        assert_eq!(
            app.world().resource::<Carrying>().0,
            None,
            "G fired while reaching for a panel"
        );
    }

    #[test]
    fn delete_takes_away_what_is_in_hand() {
        // Somebody holding a thing and pressing delete means THIS one, wherever the
        // ring happens to be pointing.
        let mut app = carrying_app();
        let (held, other) = {
            let mut standing = app.world_mut().resource_mut::<Standing>();
            let held = standing.add("cottage", Vec2::new(10.0, -4.0), 0.0, 1.0);
            let other = standing.add("barn", Vec2::new(12.0, -4.0), 0.0, 1.0);
            (held, other)
        };

        aim(&mut app, Vec2::new(10.0, -4.0));
        press(&mut app, &[KeyCode::KeyG]);
        assert_eq!(app.world().resource::<Carrying>().0, Some(held));

        // Pointing at the OTHER one, but holding the first.
        aim(&mut app, Vec2::new(12.0, -4.0));
        press(&mut app, &[KeyCode::Delete]);
        assert!(
            app.world().resource::<Standing>().get(held).is_none(),
            "the wrong one went"
        );
        assert!(
            app.world().resource::<Standing>().get(other).is_some(),
            "the barn went too"
        );
        assert_eq!(app.world().resource::<Carrying>().0, None);
    }
}
