// The line round things. See `src/ink.rs` for why this is a pass over the finished
// frame rather than a shell round every model.

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var screen: texture_2d<f32>;
@group(0) @binding(1) var screen_sampler: sampler;

#ifdef MULTISAMPLED
@group(0) @binding(2) var depth: texture_depth_multisampled_2d;
#else
@group(0) @binding(2) var depth: texture_depth_2d;
#endif

struct Ink {
    // The line's colour, and how much of it is laid down.
    colour: vec4<f32>,
    // x how wide in pixels, y how far a surface must bend to be an edge,
    // z how far away the line fades out, w the camera's near plane.
    drawn: vec4<f32>,
    // x how much of a surface's own value a line may leave.
    deepens: vec4<f32>,
};
@group(0) @binding(3) var<uniform> ink: Ink;

// HOW FAR AWAY, IN METRES.
//
// The depth buffer holds a reversed, infinite-far projection, so nothing in it is a
// distance until it is divided into the near plane. Nought means the sky, which has
// no surface and gets no line.
fn metres(at: vec2<i32>) -> f32 {
#ifdef MULTISAMPLED
    let raw = textureLoad(depth, at, 0);
#else
    let raw = textureLoad(depth, at, 0);
#endif
    if raw <= 0.0 {
        // The sky. Held at a distance rather than at infinity so a silhouette against
        // it is still a break of a definite size.
        return 1.0e6;
    }
    return ink.drawn.w / raw;
}

// HOW FAR THE MIDDLE SITS OFF THE LINE BETWEEN ITS NEIGHBOURS.
//
// A flat surface at any angle whatever has its middle sample exactly on the line
// between the two either side of it - that is what flat means. So this is nought on a
// road running away from the camera, nought on a hillside, and large only where one
// surface stops and another starts. Comparing the plain difference instead inks every
// steep thing in the world, which is the usual way this effect goes wrong.
fn breaks(near: f32, middle: f32, far: f32) -> f32 {
    // A neighbour on the sky is a silhouette, and there is no line to sit off.
    if near > 1.0e5 || far > 1.0e5 {
        return abs(min(near, far) - middle);
    }
    return abs((near + far) * 0.5 - middle);
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let painted = textureSample(screen, screen_sampler, in.uv);
    let size = vec2<f32>(textureDimensions(depth));
    let at = vec2<i32>(in.uv * size);
    let step = max(i32(round(ink.drawn.x)), 1);

    let middle = metres(at);
    if middle > ink.drawn.z {
        // Too far to ink. A horizon full of folds is a scribble, and a line this thin
        // at that distance crawls as the camera moves.
        return painted;
    }

    // ACROSS AND DOWN, each as a pair either side of the middle.
    let left = metres(at - vec2<i32>(step, 0));
    let right = metres(at + vec2<i32>(step, 0));
    let up = metres(at - vec2<i32>(0, step));
    let down = metres(at + vec2<i32>(0, step));
    let edge = max(breaks(left, middle, right), breaks(up, middle, down));

    // AND ON THE DIAGONALS, so a corner running at 45 degrees is as strong a line as
    // one running square. Without them a roofline reads as a dotted stair.
    let out = vec2<i32>(step, step);
    let back = vec2<i32>(step, -step);
    let sloped = max(
        breaks(metres(at - out), middle, metres(at + out)),
        breaks(metres(at - back), middle, metres(at + back)),
    );

    // THE WORST OF THEM, not the sum. Adding makes two weak wobbles look like one
    // strong edge, which is how noise becomes ink.
    let found = max(edge, sloped);

    // SCALED BY DISTANCE. A ten-centimetre step is a wall at arm's length and nothing
    // at three hundred metres, and a line that ignores that is a line that thickens
    // as you walk away from it.
    let wants = ink.drawn.y * middle;
    let laid = smoothstep(wants, wants * 2.5, found);

    // AND FADED OUT AT THE FAR END rather than stopping at a line.
    let fades = 1.0 - smoothstep(ink.drawn.z * 0.6, ink.drawn.z, middle);
    let much = laid * fades * ink.colour.a;

    // A LINE IS A DARKENING, not a colour.
    //
    // Mixed toward its own charcoal, the ink came out BRIGHTER than an unlit wall:
    // photographed at dusk, every building in the city had a pale glow round it.
    // Whichever is darker of the charcoal and a quarter of what is already there
    // draws the same mark in a lit street and in a dark one. See `Ink::deepens`.
    let deep = min(painted.rgb * ink.deepens.x, ink.colour.rgb);
    return vec4<f32>(mix(painted.rgb, deep, much), painted.a);
}
