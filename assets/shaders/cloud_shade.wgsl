// Cloud shadows: the same clouds that drift overhead, laid on the ground.
//
// Bevy's standard shading, unchanged, with one multiply on the end. Everything
// this game builds out of solid matter wears it — ground, grass, trunks,
// leaves, water, walls, the warden — so a shadow crosses a hillside and its
// trees together instead of stopping at their feet.
//
// # It is the actual clouds
//
// The usual way to do this is scrolling noise, and it looks fine until somebody
// stands in a patch of shade, looks up, and finds clear sky. Here the shadows
// come from the cloud list itself: one soft disc per cloud, at the point the
// sun's own line through that cloud strikes the ground. Look up from a shadow
// and the cloud casting it is overhead.
//
// The list never changes while the game runs. Clouds drift at a fixed speed, so
// the drift is a multiply by the clock rather than something the CPU has to
// send down every frame, and the only thing that is ever rewritten is the sun's
// slant — a few times a minute, as it climbs.

#import bevy_pbr::forward_io::{Vertex, VertexOutput, FragmentOutput}
#import bevy_pbr::{mesh_bindings::mesh, mesh_functions, skinning}
#import bevy_pbr::view_transformations::position_world_to_clip
#import bevy_pbr::mesh_view_bindings::globals
#import bevy_pbr::pbr_fragment::pbr_input_from_standard_material
#import bevy_pbr::pbr_functions::{alpha_discard, apply_pbr_lighting, main_pass_post_lighting_processing}

/// x: how dark a shadow gets, 0 for none at all.
/// y: where a shadow's soft edge starts, as a share of its radius.
/// z: how far apart the tiles of sky repeat, in metres.
/// w: how many of the discs below are real.
@group(2) @binding(100) var<uniform> weather: vec4<f32>;

struct Discs {
    /// xy: where the shadow sat when the world began. z: its radius.
    /// w: how fast it slides east, in metres a second.
    list: array<vec4<f32>, 32>,
}
@group(2) @binding(101) var<uniform> discs: Discs;

/// x: what this material's geometry DOES — 0 stands still, 1 is grass pushed
///    aside by movers, [`SEA`] is the sea carried by the swell. It also says which
///    surface this IS, which is what keeps cloud shadows off the water.
/// y: how far a mover pushes it, in metres (grass only).
/// z: how many of the movers below are real (grass only).
@group(2) @binding(102) var<uniform> bending: vec4<f32>;

struct Movers {
    /// For grass — xyz: where something is standing. w: how far out it parts
    /// what it stands in.
    ///
    /// For the sea the same slots carry the swell instead: [0] and [1] are
    /// (height, length, period, -) of the two layers, [2] is (tide height,
    /// tide period, sea level, -). Numbers, not a second opinion: they are fed
    /// from the same Rust constants the CPU's `sea_height` reads, so only the
    /// formula itself exists twice — see `sea_surface_at`.
    list: array<vec4<f32>, 8>,
}
@group(2) @binding(103) var<uniform> movers: Movers;
// How a paved surface is laid: x how much one stone differs from the next, y how
// wide its joint is as a share of the stone, z how dark the joint goes. Zero on
// everything that is not a road.
@group(2) @binding(104) var<uniform> paving: vec4<f32>;

const TAU: f32 = 6.28318530718;

/// What `bending.x` holds for the sea. Named, because two different questions are
/// asked of it — how this surface MOVES and what it IS — and a bare 2.0 in the
/// fragment stage would look like a mistake.
const SEA: f32 = 2.0;

/// How high the sea stands at a point, right now.
///
/// # The mirror of `water::sea_height`, and why it moved here
///
/// The sea's vertices used to be walked on the CPU every frame — twenty-six
/// thousand of them, marking the mesh asset modified and re-uploading more than
/// a megabyte of attributes per frame to describe motion a vertex shader gets
/// for free. The mesh is static now and the swell happens here, from the same
/// constants, against the same clock.
///
/// **These two functions must agree** — the CPU one still answers gameplay
/// (where the waterline is, how deep a wade is). Both are three sines: two
/// layers of swell at their own angles, and the tide. Change one, change both.
fn sea_surface_at(ground: vec2<f32>) -> f32 {
    let tide = sin(globals.time / movers.list[2].y * TAU) * movers.list[2].x;
    var swell = 0.0;
    for (var i = 0; i < 2; i = i + 1) {
        let layer = movers.list[i];
        // Each layer runs at its own angle so they interfere rather than
        // marching in step — waves in lockstep read as corrugated iron.
        let angle = f32(i) * 2.1;
        let along = ground.x * cos(angle) + ground.y * sin(angle);
        let phase = along / layer.y - globals.time / layer.z;
        swell += sin(phase * TAU) * layer.x;
    }
    return movers.list[2].z + tide + swell;
}

/// How much of a mover's reach is "underfoot" rather than "beside".
const UNDERFOOT: f32 = 0.55;
/// How far a blade is pressed down by something standing right on it, as a share
/// of how far it would be pushed aside.
const TROD: f32 = 0.8;

/// Where a point ends up once whatever is standing near it has pushed it aside.
///
/// `along` is how far up its own blade the vertex sits, which the mesh carries in
/// its U coordinate. It has to, because the vertex cannot work it out: its height
/// is the GROUND's height plus the blade's, and only the second part may move.
/// Squared, so a foot stays planted and the bend is a curve rather than a shear.
fn parted(at: vec3<f32>, along: f32) -> vec3<f32> {
    if along <= 0.0 {
        return at;
    }

    var push = vec2<f32>(0.0, 0.0);
    var press = 0.0;
    let count = i32(bending.z);

    for (var i = 0; i < count; i = i + 1) {
        let mover = movers.list[i];
        let gap = at.xz - mover.xz;
        let away = length(gap);
        if away >= mover.w || away <= 1e-4 {
            continue;
        }

        // Softly in at the rim, so the edge of the disturbance is not a ring you
        // can watch travel over the field.
        let near = 1.0 - away / mover.w;
        let strength = near * near;

        // # The snap, and what it actually was
        //
        // The push points away from whatever is standing there — which means it
        // points the OPPOSITE way on either side of it. Walk over a blade and its
        // lean reverses through a hundred and eighty degrees in the width of a
        // boot: not a bend, a flick. And right underfoot the direction is
        // whatever the arithmetic makes of a gap of nearly nothing, which is
        // noise.
        //
        // So the sideways push is faded out toward the middle and a downward
        // press fades in to replace it. What is directly under something is
        // TRODDEN, not shoved aside, and that is both what happens and the shape
        // that has no discontinuity in it — the reversal now happens where the
        // push is already nothing.
        let aside = smoothstep(0.0, mover.w * UNDERFOOT, away);
        push += gap / away * strength * aside;
        press += strength * (1.0 - aside);
    }

    // Squared along the blade, so the foot stays planted and the whole length
    // curves rather than shearing over.
    let swing = bending.y * along * along;
    let leaned = push * swing;
    // Pushed over, and dropped by what leaning costs it in height plus whatever
    // is standing on it. A blade that bends sideways without dropping stretches.
    let dropped = length(leaned) * 0.3 + press * swing * TROD;
    return at + vec3<f32>(leaned.x, -dropped, leaned.y);
}

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    // Bevy's own vertex stage, with one thing added.
    //
    // There is no hook for "displace and then do what you were going to do", so
    // this is the standard path copied and one call inserted — the alternative
    // being a second material type for grass alone, which would have meant the
    // cloud shading written out twice.
    //
    // Everything that is not grass takes the `bending.x` branch straight past it,
    // and a uniform branch is free: every fragment of a draw agrees on it.
    var out: VertexOutput;

    let mesh_world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);

#ifdef SKINNED
    var world_from_local = skinning::skin_model(
        vertex.joint_indices,
        vertex.joint_weights,
        vertex.instance_index
    );
#else
    var world_from_local = mesh_world_from_local;
#endif

#ifdef VERTEX_NORMALS
#ifdef SKINNED
    out.world_normal = skinning::skin_normals(world_from_local, vertex.normal);
#else
    out.world_normal = mesh_functions::mesh_normal_local_to_world(
        vertex.normal,
        vertex.instance_index
    );
#endif
#endif

#ifdef VERTEX_POSITIONS
    out.world_position = mesh_functions::mesh_position_local_to_world(
        world_from_local,
        vec4<f32>(vertex.position, 1.0)
    );
    // What kind of thing this is decides how it moves. A uniform branch is
    // free: every vertex of a draw agrees on it.
    if bending.x > SEA - 0.5 {
        // The sea: the flat plane is lifted to wherever the surface stands.
        out.world_position.y = sea_surface_at(out.world_position.xz);
    }
#ifdef VERTEX_UVS_A
    else if bending.x > 0.0 {
        out.world_position = vec4<f32>(
            parted(out.world_position.xyz, vertex.uv.x),
            out.world_position.w
        );
    }
#endif
    out.position = position_world_to_clip(out.world_position.xyz);
#endif

#ifdef VERTEX_UVS_A
    out.uv = vertex.uv;
#endif
#ifdef VERTEX_UVS_B
    out.uv_b = vertex.uv_b;
#endif

#ifdef VERTEX_TANGENTS
    out.world_tangent = mesh_functions::mesh_tangent_local_to_world(
        world_from_local,
        vertex.tangent,
        vertex.instance_index
    );
#endif

#ifdef VERTEX_COLORS
    out.color = vertex.color;
#endif

#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    out.instance_index = vertex.instance_index;
#endif

#ifdef VISIBILITY_RANGE_DITHER
    out.visibility_range_dither = mesh_functions::get_visibility_range_dither_level(
        vertex.instance_index, mesh_world_from_local[3]);
#endif

    return out;
}

/// How much light reaches a point on the ground, 0 dark to 1 open sky.
fn sunlight_on(ground: vec2<f32>) -> f32 {
    let spread = weather.z;
    let count = i32(weather.w);
    var openest = 1.0;

    for (var i = 0; i < count; i = i + 1) {
        let disc = discs.list[i];
        let centre = vec2<f32>(disc.x + disc.w * globals.time, disc.y);

        // The nearest copy of this cloud, not the one it was born as.
        //
        // The sky is a tile of `spread` metres repeated in every direction — that
        // is exactly what the drift does when it wraps a cloud back around the
        // viewer — so the copy that shades a point is whichever one it is nearest
        // to. Rounding the gap away in whole tiles finds it in two instructions,
        // where checking the nine neighbouring tiles would cost nine times the
        // work to reach the same answer.
        var gap = ground - centre;
        gap = gap - spread * round(gap / spread);

        // AND IT IS NOT A CIRCLE.
        //
        // # Painted circles on the grass
        //
        // A shadow was a disc: solid out to half its radius, then a smooth rim.
        // Over a wood or a hillside that passes, because there is other detail for
        // the eye to hold. Over the flat levelled ground of a city there is nothing
        // else at all, and what it reads as is a row of grey circles somebody
        // painted on the lawn - which is what was asked about, in those words.
        //
        // A cloud has no edge and it also has no radius. The reach is pushed in and
        // out around the shadow, twice over at two rates, so its outline is a
        // wandering blob and its middle is unevenly lit rather than flat. It is the
        // cheapest thing that stops a circle being a circle: two hashes of the
        // direction, no texture, no extra samples.
        let reach = length(gap);
        let turn = atan2(gap.y, gap.x);
        let seed = disc.x + disc.y;
        let wander = sin(turn * 3.0 + seed) * 0.17
            + sin(turn * 7.0 - seed * 2.3) * 0.09
            + sin(turn * 13.0 + seed * 0.7) * 0.05;
        // The middle broken up too, so a big cloud is not a flat grey plate.
        let dappled = 1.0 + sin(gap.x * 0.011 + seed) * sin(gap.y * 0.013 - seed) * 0.10;
        let edge = disc.z * (1.0 + wander) * dappled;
        openest = min(openest, smoothstep(edge * weather.y, edge, reach));
    }

    // The DARKEST cloud over this point, not all of them multiplied together.
    // Two clouds overlapping is still one cloud's worth of shade — stacking them
    // would put a black hole wherever the sky happened to double up.
    return 1.0 - weather.x * (1.0 - openest);
}

/// How many steps the light is pushed into, and how hard the step edges are.
///
/// # Almost, but not quite, cel shaded
///
/// True cel shading quantises light into flat steps with hard edges, and it is a
/// strong look that fights everything else here: a world of soft rolling ground and
/// twenty greens of foliage goes to poster paint the moment you posterise it.
///
/// This is the "almost". The lighting is pulled TOWARD steps rather than snapped to
/// them: each band edge is a smoothstep a few percent wide rather than a cliff, and
/// the result is mixed back over the original at less than full strength. Surfaces
/// read as banded - a wall has a lit face, a turning face and a shaded face, and you
/// can see where each begins - while a hillside still rolls.
///
/// Bands few enough to read: four is a lit side, a half-lit side, a shaded side and
/// a dark side, which is what a stylised building wants and no more.
const BANDS: f32 = 4.0;
const BAND_EDGE: f32 = 0.055;
const BAND_STRENGTH: f32 = 0.72;

/// Pulls a lit colour toward flat bands without snapping it to them.
fn banded(colour: vec3<f32>) -> vec3<f32> {
    // Banded on BRIGHTNESS, not per channel. Per channel pulls each of red, green
    // and blue across its own step edge at a different moment, which shifts hue in
    // the middle of a smooth surface - a grey wall picks up a green face. Working on
    // the luminance and scaling the original colour by the ratio keeps the hue.
    let lit = dot(colour, vec3<f32>(0.2126, 0.7152, 0.0722));
    if lit <= 0.0001 {
        return colour;
    }

    let scaled = lit * BANDS;
    let step_below = floor(scaled);
    let across = scaled - step_below;
    // The soft edge: nearly flat through the middle of a band, quick across its rim.
    let eased = smoothstep(0.5 - BAND_EDGE, 0.5 + BAND_EDGE, across);
    let stepped = (step_below + eased) / BANDS;

    let pulled = colour * (stepped / lit);
    return mix(colour, pulled, BAND_STRENGTH);
}



/// One number from a cell, so every stone gets its own tone.
fn one_of(cell: vec2<f32>) -> f32 {
    return fract(sin(dot(cell, vec2<f32>(127.1, 311.7))) * 43758.545);
}

/// A courseway of stones laid in world space: which stone, and how far into it.
///
/// # A running bond, because a grid is not a pavement
///
/// Rows are laid first and every other one is shifted half a stone along. Without
/// that offset the joints line up in both directions and the surface reads as graph
/// paper - the single thing that separates a drawn pavement from a real one is that
/// its cross joints are broken.
///
/// Returns the stone's own number and its distance to the nearest joint, nought at
/// the joint and a half in the middle of the stone.
fn laid_in(at: vec2<f32>, size: f32) -> vec2<f32> {
    let down = at.y / size;
    let row = floor(down);
    // Half a stone across on alternate courses.
    let along = at.x / size + 0.5 * (row - 2.0 * floor(row * 0.5));
    let cell = vec2<f32>(floor(along), row);
    let inside = vec2<f32>(fract(along), fract(down));
    let joint = min(min(inside.x, 1.0 - inside.x), min(inside.y, 1.0 - inside.y));
    return vec2<f32>(one_of(cell), joint);
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    // THE STONES, BEFORE THE LIGHT.
    //
    // Laid into the base colour rather than over the lit result, so a cobble is
    // shaded by the same sun and the same near-cel ramp as the surface it belongs
    // to - painted on afterwards it would read as a texture printed over the road
    // rather than as the road being made of pieces.
    //
    // The size comes from the VERTEX, so the carriageway can be cobbled and the
    // footway flagged on one continuous ribbon; the strength comes from the
    // material, which is zero on everything that is not a road. See `PAVING_STONE`.
#ifdef VERTEX_COLORS
    let stone = in.color.a * 2.0;
    // HOW PAVED this point is, which is a separate fact from how big the stones are.
    // The pattern fades in with it; the stones keep their size the whole way. Fading
    // by shrinking - which is what multiplying the size by this did - turns a city's
    // whole approach into a band of ever-finer crawling gravel.
    let made = clamp(in.uv.y, 0.0, 1.0);
    if paving.x > 0.0 && stone > 0.02 && made > 0.01 {
        let laid = laid_in(in.world_position.xz, stone);
        // Each stone its own tone, and a line of shadow where they meet.
        let joint = smoothstep(0.0, paving.y, laid.y);
        let tone = (1.0 + (laid.x - 0.5) * paving.x * made)
            * mix(1.0 - paving.z * made, 1.0, joint);
        pbr_input.material.base_color = vec4<f32>(
            pbr_input.material.base_color.rgb * tone,
            pbr_input.material.base_color.a,
        );
    }
#endif

    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);

    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);

    // Before the post-lighting pass, not after: whatever that does to distance
    // and tone should be done to a shaded hillside as well, or a far-off shadow
    // would sit on top of the view instead of in it.
    //
    // Skipped outright at dawn and dusk and through the night. A sun near the
    // horizon throws a cloud's shadow kilometres sideways, which is true and
    // useless, and by then the light is too flat for anyone to read a shadow on
    // the ground anyway.
    //
    // And never on the SEA. A cloud's shadow on water is a real thing and it does
    // not read as one here: open water is a broad flat blue, so a soft dark disc
    // laid on it has nothing to sit on and comes out as a stain. Half the view from
    // any coast is sea, which made the whole sky's weather look like dirt on the
    // lens. It is the one surface in the world that goes without.
    if weather.x > 0.0 && bending.x < SEA - 0.5 {
        let lit = sunlight_on(in.world_position.xz);
        out.color = vec4<f32>(out.color.rgb * lit, out.color.a);
    }

    // The near-cel pass, before the post-lighting one so fog and tone-mapping act
    // on a banded surface rather than banding a fogged one - otherwise the steps
    // march about as the view moves, which is the one thing that would make this
    // read as a bug rather than as a style.
    out.color = vec4<f32>(banded(out.color.rgb), out.color.a);

    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
