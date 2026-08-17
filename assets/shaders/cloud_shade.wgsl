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

#import bevy_pbr::forward_io::{VertexOutput, FragmentOutput}
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
