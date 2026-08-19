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

/// Whether the pointer is free to point at things — which, in this tool, it is
/// unless somebody is deliberately looking around.
///
/// # A tool you point at things with
///
/// This was the other way up: the cursor was captured for mouse-look and ALT let
/// go of it so the panels could be reached. Every row in the panel was therefore
/// a thing you could only click while holding a modifier, which is not a menu —
/// it is a keyboard tool with a picture of a menu beside it. Asked for three
/// times, and each time I moved the rows around instead of the rule.
///
/// So the tool points by default: the pointer is visible, the panel is clickable,
/// and the brush aims wherever it is aimed. **ALT is now the one that looks
/// around** — held, the pointer goes away and the mouse turns the view, which is
/// the moment inside the work rather than the resting state of it.
#[derive(Resource, Deref)]
pub struct CursorFree(pub bool);

impl Default for CursorFree {
    fn default() -> Self {
        // Pointing, not looking.
        Self(true)
    }
}

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

/// A row in the panel was pressed.
///
/// An event rather than a resource because a press is a MOMENT: a resource would
/// have to be cleared by whoever read it, and two readers would race over who got
/// there first.
#[derive(Event, Clone, Copy)]
pub struct Asked(pub Act);

/// The mouth of a tunnel a maker has started but not finished.
///
/// A bore takes two points, so it takes two clicks: the first is remembered here
/// and the second lays the tunnel. The same shape as the ramp tool, and for the
/// same reason — a thing defined by where it starts and where it ends cannot be
/// painted, because painting only ever knows where the brush is now.
#[derive(Resource, Default)]
pub struct Boring(pub Option<Vec2>);

/// What a press in the panel's ACTIONS does.
///
/// # Not brushes, which is why they are not in the palette
///
/// A brush is dragged over ground and does its work wherever it passes. None of
/// these are: placing a building, picking one up, turning it, and boring a tunnel
/// all happen at a MOMENT, and two of them need two moments to say what they mean.
/// Sitting them in the palette would mean a maker selects PLACE and then wonders
/// why dragging does nothing.
///
/// They were keyboard-only, which is worse — a key nobody has been told about is a
/// tool that does not exist as far as anyone can tell. Every one of them has a row
/// to press now, with its key printed on it, which is what the whole panel is for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Act {
    /// Place the next building from the catalogue where the ring is.
    Place,
    /// Pick up what the ring is over, or set down what is in hand.
    Carry,
    /// Turn what the ring is over a quarter.
    Turn,
    /// Take away what the ring is over.
    Remove,
    /// Start a tunnel, or finish the one already started.
    Bore,
    /// Take away the nearest tunnel.
    Unbore,
}

impl Act {
    pub const ALL: [Act; 6] = [
        Act::Place,
        Act::Carry,
        Act::Turn,
        Act::Remove,
        Act::Bore,
        Act::Unbore,
    ];

    /// What the row says, and the key that does the same thing.
    ///
    /// One table, read by the panel and by the keyboard both — the same
    /// arrangement `TOOL_KEYS` keeps, and for the same reason: a panel numbering
    /// its own rows and an input holding its own keys drift apart, and the panel
    /// then tells a maker something untrue.
    pub fn says(self) -> (&'static str, &'static str) {
        match self {
            Act::Place => ("P", "PLACE A BUILDING"),
            Act::Carry => ("G", "PICK UP / PUT DOWN"),
            Act::Turn => ("R", "TURN A QUARTER"),
            Act::Remove => ("Del", "TAKE IT AWAY"),
            Act::Bore => ("T", "BORE A TUNNEL"),
            Act::Unbore => ("Sh-T", "FILL A TUNNEL IN"),
        }
    }

    pub fn key(self) -> KeyCode {
        match self {
            Act::Place => KeyCode::KeyP,
            Act::Carry => KeyCode::KeyG,
            Act::Turn => KeyCode::KeyR,
            Act::Remove => KeyCode::Delete,
            // T for tunnel. It was B, which the BIOME brush already had — so one
            // press picked up the biome brush AND started a tunnel, which is
            // exactly the collision a panel full of printed keycaps is supposed to
            // make impossible to ship. Both tables are read by the panel, and the
            // panel drew them both, and nothing compared them to each other.
            Act::Bore | Act::Unbore => KeyCode::KeyT,
        }
    }
}

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
        app.add_event::<Asked>()
            .init_resource::<Carrying>()
            .init_resource::<Boring>()
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
                    bore_tunnels,
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
/// Whether a click belongs to the world rather than to the panel.
///
/// # It used to ask the wrong question
///
/// Every tool here guarded on "is the pointer free", which was a workable PROXY
/// while the pointer was captured except when reaching for a panel: free meant
/// reaching. Now that the tool points by default, free means nothing of the sort —
/// it means ordinary use — so the guards had to start asking what they always
/// meant. Which is this: the pointer is somewhere over the world, and not over the
/// shelf of rows on the left.
pub fn aiming_at_the_world(
    free: &CursorFree,
    windows: &Query<&Window, With<bevy::window::PrimaryWindow>>,
    panels: &Query<(&ComputedNode, &GlobalTransform), With<crate::tools::widget::Scrolls>>,
) -> bool {
    // Nothing acts while ALT has hold of the view. A hand swinging the camera is
    // not a hand placing a building, and the aim is sweeping across the country
    // while it happens — so one rule rather than a different answer per tool:
    // point at things to do them, hold ALT to look, and the two never overlap.
    if !free.0 {
        return false;
    }
    !crate::tools::widget::pointer_on_a_panel(windows, panels)
}

/// ALT takes hold of the view for as long as it is held.
///
/// The inverse of what this used to do — see [`CursorFree`]. Released, the pointer
/// is a pointer.
fn hold_to_reach(
    keys: Res<ButtonInput<KeyCode>>,
    mut free: ResMut<CursorFree>,
    mut windows: Query<&mut Window, With<bevy::window::PrimaryWindow>>,
) {
    let looking = keys.any_pressed([KeyCode::AltLeft, KeyCode::AltRight]);
    if looking != free.0 {
        return;
    }
    free.0 = !looking;

    let Some(mut window) = windows.iter_mut().next() else {
        return;
    };
    window.cursor_options.grab_mode = if looking {
        bevy::window::CursorGrabMode::Confined
    } else {
        bevy::window::CursorGrabMode::None
    };
    window.cursor_options.visible = !looking;
}

fn enter_editor(mut camera: ResMut<CameraMode>, mut free: ResMut<CursorFree>) {
    // Sculpting from the follow camera means aiming past your own warden at
    // whatever happens to be in front of them. Free-fly is what the tool wants.
    *camera = CameraMode::Fly;
    // Pointing, not looking — nobody arrives holding ALT. Left disagreeing with
    // the window's own cursor state, `hold_to_reach`'s early-out would keep the
    // two apart until the next ALT press.
    free.0 = true;
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

/// Aims the brush: at the pointer when there is one, down the view otherwise.
///
/// A tool you point things at has to paint where you are pointing. It used to aim
/// straight down the middle of the screen always, which is right for a captured
/// cursor and wrong the moment there is a cursor to aim with — the crosshair and
/// the pointer would be in two different places, both claiming to be the brush.
fn aim_brush(
    terrain: Res<TerrainSource>,
    free: Res<CursorFree>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut brush: ResMut<Brush>,
) {
    let Some((camera, eye)) = cameras.iter().next() else {
        return;
    };

    // Where the pointer is, if it is a pointer. Looking around, there is nothing
    // to aim with and the middle of the view is the honest answer.
    let ray = free
        .0
        .then(|| windows.iter().next().and_then(Window::cursor_position))
        .flatten()
        .and_then(|at| camera.viewport_to_world(eye, at).ok());

    brush.hit = match ray {
        Some(ray) => raycast_terrain(&terrain.0, ray.origin, *ray.direction),
        None => raycast_terrain(&terrain.0, eye.translation(), eye.forward().as_vec3()),
    };
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
    // The wheel over the shelf is the shelf's scroll and must not also resize the
    // brush behind it.
    if notches != 0.0 && aiming_at_the_world(&free, &windows, &panels) {
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
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    panels: Query<(&ComputedNode, &GlobalTransform), With<crate::tools::widget::Scrolls>>,
    mut brush: ResMut<Brush>,
) {
    // A click on the shelf is for the shelf, not for the ground behind it.
    if !aiming_at_the_world(&free, &windows, &panels) {
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
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    panels: Query<(&ComputedNode, &GlobalTransform), With<crate::tools::widget::Scrolls>>,
    mut brush: ResMut<Brush>,
    mut toast: ResMut<ui::Toast>,
) {
    if !aiming_at_the_world(&free, &windows, &panels) || !brush.how.is_two_point() {
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
    // The tunnels go with everything else, in the same keystroke. A maker who
    // has just bored one should not have to learn there is a sixth file.
    let dug = {
        let Ok(mut bores) = terrain.bores().write() else {
            return;
        };
        crate::world::bores::save(&mut bores)
            .map(|()| bores.len())
            .map_err(|why| why.to_string())
    };

    match (ground, woods, worn, countries, built) {
        (Ok(cells), Ok(planted), Ok(laid), Ok(marked), Ok(stood)) => {
            let bored = dug.clone().unwrap_or_default();
            info!(
                "saved {cells} sculpted, {planted} planted, {laid} surfaced,                  {marked} biome, {stood} placed, {bored} bored"
            );
            toast.show(format!(
                "Saved {cells} sculpted, {planted} planted, {laid} surfaced,                  {marked} biome, {stood} placed, {bored} bored"
            ));
            if let Err(why) = &dug {
                error!("could not save the tunnels: {why}");
                toast.show("Tunnels not saved - see log");
            }
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
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    panels: Query<(&ComputedNode, &GlobalTransform), With<crate::tools::widget::Scrolls>>,
    mut placed: ResMut<crate::world::placed::Standing>,
    mut carrying: ResMut<Carrying>,
    mut asked: EventReader<Asked>,
    mut choosing: Local<usize>,
    mut toast: ResMut<ui::Toast>,
) {
    // A row pressed in the panel and the key it prints mean the same thing, so
    // they arrive by the same door.
    let mut pressed: Vec<Act> = asked.read().map(|ask| ask.0).collect();
    {
        for act in [Act::Place, Act::Carry, Act::Turn, Act::Remove] {
            let shift = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
            // Shift+B is the bore's own business, and Shift+R turns the other way
            // rather than being a different act.
            if act != Act::Remove && shift && act == Act::Place {
                continue;
            }
            if keys.just_pressed(act.key()) && !pressed.contains(&act) {
                pressed.push(act);
            }
        }
        if keys.just_pressed(KeyCode::Backspace) && !pressed.contains(&Act::Remove) {
            pressed.push(Act::Remove);
        }
    }
    if !aiming_at_the_world(&free, &windows, &panels) {
        return;
    }
    let Some(hit) = brush.hit else {
        return;
    };
    let at = Vec2::new(hit.x, hit.z);

    if pressed.contains(&Act::Remove) {
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
    if pressed.contains(&Act::Carry) {
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
    if pressed.contains(&Act::Turn) {
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

    if !pressed.contains(&Act::Place) {
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
            .add_event::<Asked>()
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
    fn turning_holds_still_while_the_view_is_being_turned() {
        // `CursorFree` used to mean "reaching for a panel", so everything guarded
        // on it and this test asserted that nothing acted while it was set. It
        // means the opposite now — pointing, which is the tool's resting state —
        // so what it guards is the other half: while ALT has hold of the view
        // there is no pointer aimed at anything, and R must not turn whatever the
        // brush was last over.
        let mut app = placing_app();
        let id = app
            .world_mut()
            .resource_mut::<Standing>()
            .add("cottage", Vec2::new(10.0, -4.0), 0.0, 1.0);

        app.world_mut().resource_mut::<CursorFree>().0 = false;
        press(&mut app, &[KeyCode::KeyR]);
        assert_eq!(
            facing(&app, id),
            0.0,
            "it turned while the view was being turned"
        );

        // And with the pointer back, it turns.
        app.world_mut().resource_mut::<CursorFree>().0 = true;
        press(&mut app, &[KeyCode::KeyR]);
        assert!(
            facing(&app, id) != 0.0,
            "it would not turn with the pointer aimed at it"
        );
    }
}

/// Bores a tunnel, in two clicks.
///
/// The first says where it starts, the second where it comes out, and the tunnel
/// is cut between them through whatever happens to be in the way. It only ever
/// cuts DOWN — run one over open ground and nothing happens, because there was no
/// hill to get through.
pub fn bore_tunnels(
    keys: Res<ButtonInput<KeyCode>>,
    brush: Res<Brush>,
    free: Res<CursorFree>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    panels: Query<(&ComputedNode, &GlobalTransform), With<crate::tools::widget::Scrolls>>,
    terrain: Res<TerrainSource>,
    chunks: Res<ChunkMap>,
    busy: Query<(), With<PendingChunk>>,
    mut boring: ResMut<Boring>,
    mut asked: EventReader<Asked>,
    mut commands: Commands,
    mut toast: ResMut<ui::Toast>,
) {
    let shift = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    // From the key, or from the row in the panel — one path, so the two cannot
    // come to mean different things.
    let mut wanted = if keys.just_pressed(Act::Bore.key()) {
        Some(if shift { Act::Unbore } else { Act::Bore })
    } else {
        None
    };
    for ask in asked.read() {
        if matches!(ask.0, Act::Bore | Act::Unbore) {
            wanted = Some(ask.0);
        }
    }
    let Some(act) = wanted else {
        return;
    };
    if !aiming_at_the_world(&free, &windows, &panels) {
        return;
    }
    let Some(hit) = brush.hit else {
        return;
    };
    let at = Vec2::new(hit.x, hit.z);

    if act == Act::Unbore {
        let Ok(mut bores) = terrain.0.bores().write() else {
            return;
        };
        // A tunnel is long, so what counts as near it is generous.
        if bores.remove_nearest(at, crate::world::bores::SPAN * 12.0) {
            drop(bores);
            toast.show("Tunnel filled in");
            redraw_around(&mut commands, &terrain, &chunks, &busy, at, at);
        } else {
            toast.show("No tunnel near the ring");
        }
        return;
    }

    let Some(from) = boring.0.take() else {
        boring.0 = Some(at);
        toast.show("Tunnel started - aim at the far side and press again");
        return;
    };
    if from.distance(at) < crate::world::bores::SPAN {
        toast.show("Too short to be a tunnel - aim further off");
        boring.0 = Some(from);
        return;
    }

    // Two points AIMED at a hill, not two mouths surveyed. The ends are walked in
    // to the first standable ground, so overshooting into water — the normal way
    // anybody aims — gives a tunnel that comes out at the shore rather than an
    // error. The floors are read from the ground as it is NOW, before this bore
    // cuts it; see `bores` for why they cannot be asked for later.
    let ground = |at: Vec2| terrain.0.unbored(at.x, at.y);
    let Some(bore) = crate::world::bores::Bore::aimed(from, at, ground) else {
        toast.show("No dry ground along that aim");
        boring.0 = Some(from);
        return;
    };
    // The one thing trimming cannot fix. The start is KEPT, so a better second
    // press is one press away.
    if let Err(why) = bore.makes_sense(ground) {
        toast.show(why);
        boring.0 = Some(from);
        return;
    }
    let Ok(mut bores) = terrain.0.bores().write() else {
        return;
    };
    bores.add(bore);
    drop(bores);
    toast.show(format!("Tunnel bored, {:.0} m", from.distance(at)));
    redraw_around(&mut commands, &terrain, &chunks, &busy, from, at);
}

/// Rebuilds the ground a tunnel runs under.
fn redraw_around(
    commands: &mut Commands,
    terrain: &TerrainSource,
    chunks: &ChunkMap,
    busy: &Query<(), With<PendingChunk>>,
    from: Vec2,
    to: Vec2,
) {
    let edge = crate::world::bores::SPAN * 2.0;
    let low = from.min(to) - Vec2::splat(edge);
    let high = from.max(to) + Vec2::splat(edge);
    invalidate_area(commands, terrain, chunks, busy, (low, high));
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
            .add_event::<Asked>()
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

        // And while ALT has hold of the view, nothing acts: a hand swinging the
        // camera is not a hand picking things up.
        aim(&mut app, Vec2::new(10.0, -4.0));
        app.world_mut().resource_mut::<CursorFree>().0 = false;
        press(&mut app, &[KeyCode::KeyG]);
        assert_eq!(
            app.world().resource::<Carrying>().0,
            None,
            "G fired while the view was being turned"
        );

        // Pointer back, and it picks up.
        app.world_mut().resource_mut::<CursorFree>().0 = true;
        press(&mut app, &[KeyCode::KeyG]);
        assert!(
            app.world().resource::<Carrying>().0.is_some(),
            "G would not pick up with the pointer aimed at it"
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

#[cfg(test)]
mod boring {
    use super::*;
    use crate::world::bores::Bores;

    /// The tool, driven the way a maker drives it: two presses on the ground.
    fn bench() -> App {
        let mut app = App::new();
        app.add_plugins(bevy::state::app::StatesPlugin)
            .init_state::<AppState>()
            .init_resource::<Brush>()
            .init_resource::<Boring>()
            .init_resource::<CursorFree>()
            .init_resource::<ui::Toast>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ChunkMap>()
            .add_event::<Asked>()
            .insert_resource(TerrainSource(std::sync::Arc::new(Terrain::new())))
            .add_systems(Update, bore_tunnels);
        app
    }

    fn press_at(app: &mut App, at: Vec2, shift: bool) {
        app.world_mut().resource_mut::<Brush>().hit = Some(Vec3::new(at.x, 0.0, at.y));
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            if shift {
                keys.press(KeyCode::ShiftLeft);
            }
            keys.press(Act::Bore.key());
        }
        app.update();
        // Released as well as cleared. `clear` only forgets what was pressed THIS
        // frame; the key itself stays down, and pressing an already-down key is
        // not a new press — so without this the second press never happened and
        // the tool looked broken when it was the test holding the key.
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(Act::Bore.key());
        keys.release(KeyCode::ShiftLeft);
        keys.clear();
    }

    fn bores(app: &App) -> usize {
        let terrain = app.world().resource::<TerrainSource>();
        let count = terrain.0.bores().read().unwrap().len();
        count
    }

    #[test]
    fn two_presses_bore_a_tunnel_through_a_mountain() {
        let mut app = bench();
        // Through the pass's mountain, which is the one piece of high ground this
        // test can count on being there. Aimed generously past it at both ends —
        // the lining trims itself to the rock.
        let middle = crate::world::pass::AT;
        let (sin, cos) = crate::world::pass::HEADING.sin_cos();
        let along = Vec2::new(cos, sin);
        // Aimed symmetrically and generously, clean past the mountain on both
        // sides — which on this one overshoots into the sea to the east. That is
        // the normal way to aim, so the far mouth walks itself back to the shore
        // rather than the bore being refused.
        let from = middle - along * 700.0;
        let to = middle + along * 700.0;

        let was = bores(&app);
        press_at(&mut app, from, false);
        assert_eq!(bores(&app), was, "one press laid a tunnel on its own");
        assert!(
            app.world().resource::<Boring>().0.is_some(),
            "the first press did not remember where it was"
        );

        press_at(&mut app, to, false);
        assert_eq!(bores(&app), was + 1, "two presses did not lay a tunnel");

        // The hill KEEPS ITS SKIN: the ground over the tunnel is untouched, which
        // is the whole point of the rework. What opens is the two mouths.
        let terrain = app.world().resource::<TerrainSource>();
        let over = terrain.0.height(middle.x, middle.y);
        assert!(
            over > 150.0,
            "the mountain over the tunnel came down to {over:.0} m"
        );
        // And it can be WALKED. A single-point question cannot answer this: which
        // ground claims a walker depends on where they already are, and their
        // height comes down with the floor as they go — so the walk is the query.
        let mut standing = terrain.0.height(from.x, from.y);
        let mut rock_overhead = 0.0_f32;
        for step in 0..=800 {
            let at = from.lerp(to, step as f32 / 800.0);
            standing = terrain.0.walk_floor(at.x, at.y, standing);
            rock_overhead = rock_overhead.max(terrain.0.height(at.x, at.y) - standing);
        }
        assert!(
            rock_overhead > 100.0,
            "the walk only ever had {rock_overhead:.0} m of rock overhead"
        );

        // Shift takes it out again.
        press_at(&mut app, middle, true);
        assert_eq!(bores(&app), was, "the tunnel would not fill back in");
    }

    #[test]
    fn a_bore_over_open_ground_is_refused() {
        // A tunnel is a hole through something. Across a plain there is nothing to
        // make one in, and the first build of this tool laid a mesh in the open
        // twice before anybody said so — it says so itself now.
        let mut app = bench();
        let from = Vec2::new(crate::config::RANCH_AT.0, crate::config::RANCH_AT.1);
        let to = from + Vec2::new(220.0, 0.0);

        let sample = |app: &App, at: Vec2| {
            app.world().resource::<TerrainSource>().0.height(at.x, at.y)
        };
        let before: Vec<f32> = (0..9)
            .map(|step| sample(&app, from.lerp(to, step as f32 / 8.0)))
            .collect();

        press_at(&mut app, from, false);
        press_at(&mut app, to, false);
        assert_eq!(bores(&app), 0, "a tunnel across a plain was laid anyway");
        assert!(
            app.world().resource::<Boring>().0.is_some(),
            "the refused start was thrown away rather than kept for a better aim"
        );

        for (step, was) in before.iter().enumerate() {
            let at = from.lerp(to, step as f32 / 8.0);
            let now = sample(&app, at);
            assert!(
                (was - now).abs() < 0.01,
                "the ground moved {:.2} m at step {step} for a tunnel that was refused",
                was - now
            );
        }
    }

    #[test]
    fn a_tunnel_needs_two_ends_that_are_not_the_same_place() {
        let mut app = bench();
        let at = crate::world::pass::AT;
        press_at(&mut app, at, false);
        press_at(&mut app, at + Vec2::new(2.0, 0.0), false);
        assert_eq!(bores(&app), 0, "a tunnel two metres long was laid");
        assert!(
            app.world().resource::<Boring>().0.is_some(),
            "the start was thrown away rather than kept for a better second press"
        );
    }
}


#[cfg(test)]
mod keycaps {
    use super::*;

    /// No key may mean two things in this tool.
    ///
    /// # B was the biome brush AND the bore
    ///
    /// Two tables of keys — `TOOL_KEYS` for the palette and `Act::key` for the
    /// actions — each correct on its own, each printed faithfully on its own rows,
    /// and nothing anywhere compared one to the other. So one press picked up the
    /// biome brush and started a tunnel at the same time, and the panel showed a
    /// `B` on both rows without a hint that anything was wrong.
    ///
    /// The panel exists so a maker can SEE the keys. That only helps if the keys
    /// are true, and truth across two tables is exactly the thing a person cannot
    /// check by reading.
    #[test]
    fn no_key_is_bound_to_two_things() {
        let mut taken: Vec<(KeyCode, String)> = Vec::new();

        for (key, how) in TOOL_KEYS.iter().zip(Brushing::ALL) {
            taken.push((*key, format!("the {} brush", how.name())));
        }
        for act in Act::ALL {
            // The bore and the unbore share a key on purpose: shift tells them
            // apart, the way shift tells the two turns apart.
            if act == Act::Unbore {
                continue;
            }
            taken.push((act.key(), act.says().1.to_string()));
        }

        for (index, (key, what)) in taken.iter().enumerate() {
            if let Some((_, other)) = taken[index + 1..].iter().find(|(k, _)| k == key) {
                panic!("{key:?} is bound to both {what} and {other}");
            }
        }
    }

    /// And every row prints the key that actually does it.
    ///
    /// The other half of the same fault: a panel that prints a keycap nobody
    /// pressed is worse than no keycap at all, because it is believed.
    #[test]
    fn every_row_prints_the_key_that_works() {
        for act in Act::ALL {
            let (printed, says) = act.says();
            let key = act.key();
            let expected = match key {
                KeyCode::KeyP => "P",
                KeyCode::KeyG => "G",
                KeyCode::KeyR => "R",
                KeyCode::KeyT => "T",
                KeyCode::Delete => "Del",
                other => panic!("{says} is on {other:?}, which this test cannot spell"),
            };
            // Shift-prefixed rows say so, and the rest are the key itself.
            let bare = printed.trim_start_matches("Sh-");
            assert_eq!(
                bare, expected,
                "the {says} row prints {printed:?} and is on {key:?}"
            );
        }
    }
}
