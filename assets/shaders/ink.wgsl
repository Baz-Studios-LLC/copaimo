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
    // x how wide in pixels at 1080p, y how far a surface must bend to be an edge,
    // z how far away the line fades out, w the camera's near plane.
    drawn: vec4<f32>,
    // x how much of a surface's own value a line may leave.
    deepens: vec4<f32>,
};
@group(0) @binding(3) var<uniform> ink: Ink;

// THE DEPTH AS THE BUFFER HOLDS IT, reversed: bigger is nearer, and nought is
// nothing drawn at all.
//
// # Every sample, not sample nought
//
// The camera runs four samples and the colour being inked has already been resolved
// across them, so a thin branch or a subpixel roof edge can be in the picture and
// absent from sample nought - and can change which samples it covers as the camera
// moves, which is a line that crawls. The photosensitivity rule in this project will
// not have that. Reverse-Z makes the NEAREST covered surface the largest reading, so
// the samples reduce with `max`. Codex found this one.
fn reading(at: vec2<i32>) -> f32 {
#ifdef MULTISAMPLED
    return max(
        max(textureLoad(depth, at, 0), textureLoad(depth, at, 1)),
        max(textureLoad(depth, at, 2), textureLoad(depth, at, 3)),
    );
#else
    return textureLoad(depth, at, 0);
#endif
}

// HOW FAR THE MIDDLE SITS OFF THE LINE BETWEEN ITS NEIGHBOURS, as a share of the
// depth it sits at.
//
// # Why this is asked of the raw reading and not of metres
//
// A flat surface has its middle sample exactly on the line between the two either
// side of it - that is what flat means - and it is what lets a road running away from
// the camera be left alone while a real break is inked. But it is only true of the
// value the rasteriser interpolates, which is the RECIPROCAL of distance. Take the
// reciprocal back out into metres first and a perfectly planar road at a grazing
// angle has a second difference that is not nought at all, so the rule stops being
// the rule and starts being a threshold big enough to hide the error. Codex caught
// the comment and the arithmetic disagreeing.
//
// Divided by the middle's own reading, which turns it into a RELATIVE depth change:
// a metre step at ten metres is a line and the same step at three hundred is not,
// which is what a line in screen space has to mean.
fn breaks(near: f32, middle: f32, far: f32) -> f32 {
    // NOTHING DRAWN EITHER SIDE IS A SILHOUETTE, said outright.
    //
    // The first version of this took the nearer of the two neighbours, which at a
    // roof against the sky is the roof - the same reading as the middle - so it
    // answered nought and drew no line at the one place a line matters most. It only
    // looked right because this world's sky is a dome at a finite distance rather
    // than a cleared background, so the branch never ran. Codex found it by reading
    // the arithmetic instead of the photograph, which is the only way it could have
    // been found.
    if near <= 0.0 || far <= 0.0 {
        return 1.0;
    }
    return abs((near + far) * 0.5 - middle) / max(middle, 1.0e-6);
}

// The strongest break at this radius, over the two axes and the two diagonals.
//
// The diagonals are here so a roofline running at 45 degrees is as strong a line as
// one running square; without them it reads as a dotted stair.
fn around(at: vec2<i32>, middle: f32, step: i32) -> f32 {
    let across = breaks(reading(at - vec2<i32>(step, 0)), middle, reading(at + vec2<i32>(step, 0)));
    let down = breaks(reading(at - vec2<i32>(0, step)), middle, reading(at + vec2<i32>(0, step)));
    let out = vec2<i32>(step, step);
    let back = vec2<i32>(step, -step);
    let sloped = max(
        breaks(reading(at - out), middle, reading(at + out)),
        breaks(reading(at - back), middle, reading(at + back)),
    );
    // THE WORST OF THEM, not the sum. Adding makes two weak wobbles look like one
    // strong edge, which is how noise becomes ink.
    return max(max(across, down), sloped);
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let painted = textureSample(screen, screen_sampler, in.uv);
    let size = vec2<f32>(textureDimensions(depth));
    let at = vec2<i32>(in.uv * size);

    // WHAT THE SURFACE SAID ABOUT ITSELF, in the alpha nothing else reads.
    //
    // A pass over the finished frame knows pixels and not things, so this is the one
    // way a material can ask to be left alone - see `shade::CloudShade::ink`. Read
    // from the MIDDLE pixel, so a line is drawn on the object's own side: a building
    // against grass is inked and the grass beside it is not.
    if painted.a < 0.5 {
        return painted;
    }

    let middle = reading(at);
    if middle <= 0.0 {
        // Nothing was drawn here, so there is no surface to draw the edge OF. The
        // object beside it draws its own line.
        return painted;
    }
    // In metres, which is what the far fade is written in - see `Ink::drawn`.
    let away = ink.drawn.w / middle;
    if away > ink.drawn.z {
        return painted;
    }

    // WIDTH IN PIXELS, SCALED TO THE SCREEN.
    //
    // A line measured in pixels at 1080p is a thinner line on a 4K screen and a
    // fatter one on a laptop unless it is scaled by the height it is being drawn at.
    // The inner ring finds the edge crisply and the outer one thickens it, which is
    // a deliberate dilation rather than whatever width the threshold happens to
    // produce - the thing Codex pointed out the constant was not actually
    // controlling.
    let wide = max(ink.drawn.x * size.y / 1080.0, 1.0);
    let spread = max(i32(round(wide)), 1);
    var found = around(at, middle, 1);
    // Only if it is a DIFFERENT ring. At 1080p the width rounds to one pixel and the
    // outer ring is the inner one, so asking twice buys nothing and costs seventeen
    // more depth reads per pixel - times four again, because each of them reduces
    // over the samples.
    if spread > 1 {
        found = max(found, around(at, middle, spread) * 0.85);
    }

    let laid = smoothstep(ink.drawn.y, ink.drawn.y * 2.5, found);

    // FADED OUT AT THE FAR END rather than stopping at a line. A hill four kilometres
    // off has a break at every fold, and inking all of them turns the horizon into a
    // scribble that crawls as the camera moves.
    let fades = 1.0 - smoothstep(ink.drawn.z * 0.6, ink.drawn.z, away);
    let much = laid * fades * ink.colour.a;

    // A LINE IS A DARKENING, not a colour.
    //
    // Mixed toward its own charcoal, the ink came out BRIGHTER than an unlit wall:
    // photographed at dusk, every building in the city had a pale glow round it.
    // Whichever is darker of the charcoal and a quarter of what is already there
    // draws the same mark in a lit street and in a dark one. See `Ink::deepens`.
    let deep = min(painted.rgb * ink.deepens.x, ink.colour.rgb);
    return vec4<f32>(mix(painted.rgb, deep, much * 0.0), painted.a);
}
