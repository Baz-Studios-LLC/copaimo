// Cloud shadows: the same clouds that drift overhead, laid on the ground.
//
// Bevy's standard shading, unchanged, with one multiply on the end. Everything
// this game builds out of solid matter wears it — ground, grass, trunks,
// leaves, water, walls, the ranger — so a shadow crosses a hillside and its
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

/// x: 1 where this material's geometry is pushed aside, 0 where it stands still.
/// y: how far a mover pushes it, in metres.
/// z: how many of the movers below are real.
@group(2) @binding(102) var<uniform> bending: vec4<f32>;

struct Movers {
    /// xyz: where something is standing. w: how far out it parts what it stands in.
    list: array<vec4<f32>, 8>,
}
@group(2) @binding(103) var<uniform> movers: Movers;

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
    let count = i32(bending.z);
    for (var i = 0; i < count; i = i + 1) {
        let mover = movers.list[i];
        let gap = at.xz - mover.xz;
        let away = length(gap);
        if away < mover.w && away > 1e-4 {
            // Hardest against whatever is standing there and nothing at the rim,
            // squared so the edge of the disturbance is soft. A linear falloff
            // gives a moving ring you can see the edge of.
            let force = 1.0 - away / mover.w;
            push += gap / away * force * force;
        }
    }

    let swing = bending.y * along * along;
    let leaned = vec2<f32>(push.x, push.y) * swing;
    // Pushed over, and lowered by roughly what leaning over costs it in height.
    // A blade that bends sideways without dropping stretches.
    return at + vec3<f32>(leaned.x, -length(leaned) * 0.3, leaned.y);
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
#ifdef VERTEX_UVS_A
    if bending.x > 0.0 {
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

        // Darkest in the middle and soft at the rim, because a cloud's edge is a
        // thinning rather than an ending, and two hundred metres of air blurs
        // whatever is left of it.
        let reach = length(gap);
        openest = min(openest, smoothstep(disc.z * weather.y, disc.z, reach));
    }

    // The DARKEST cloud over this point, not all of them multiplied together.
    // Two clouds overlapping is still one cloud's worth of shade — stacking them
    // would put a black hole wherever the sky happened to double up.
    return 1.0 - weather.x * (1.0 - openest);
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
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
    if weather.x > 0.0 {
        let lit = sunlight_on(in.world_position.xz);
        out.color = vec4<f32>(out.color.rgb * lit, out.color.a);
    }

    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
