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

// WHERE THE SURFACE TURNS A CORNER, which a depth break cannot see.
//
// # Why silhouettes alone read as flat
//
// `breaks` finds where one surface ENDS and another begins, which is a silhouette.
// Photographed with the ink turned bright red, a cottage sixteen metres away had a
// line round its roof against the sky and NOTHING anywhere else: not round its
// window frames, not down its timber framing, not at the corner where two walls
// meet. Those are centimetres of depth at that range - two per cent is the floor of
// what a depth test can tell from noise - so the building came out as a flat shape
// with a coloured pattern on it, which is a large part of why it reads as generated
// rather than drawn.
//
// A corner is not a depth difference, it is an ANGLE difference, and that is what
// this asks. The normal is rebuilt from the depth buffer twice over - once from the
// two neighbours ahead, once from the two behind - and on any smooth surface those
// two agree exactly, because they are built from the same pair of vectors. Where the
// surface turns, they do not.
//
// # And why this is scale-free, which is the whole point
//
// A depth threshold has to be a share of the distance, so it grows with range and a
// near-flat kerb and a far-off cliff cannot both be judged by it. An angle does not
// care how far away it is: two walls meeting at a right angle read as a right angle
// at any distance. The terrain's own facets differ by a few degrees, which is a
// thousandth of this measure, while a building's corner is the whole of it - so one
// threshold separates architecture from ground without either being tuned.
//
// No prepass, for the reason in the module header: this reads the depth the frame
// was actually drawn with, deformation and all.
// ONE SAMPLE, not the reduced four.
//
// `reading` takes the nearest of a pixel's MSAA samples, which is what a silhouette
// wants: it decides which side of an edge the pixel belongs to. A corner is not
// asking that question - it is measuring the slope of a surface the pixel is
// wholly inside - so the extra three loads buy nothing and cost everything. With
// the crease term on the reduced read, `--flyby` put the median frame at 8.7 ms
// against 6.1 without it; this is the difference between twenty depth loads a
// pixel and five.
fn depth_at(at: vec2<i32>) -> f32 {
    return textureLoad(depth, at, 0);
}

// How square-on a surface has to be before its corners are believed.
//
// A view-space normal's z: one is facing the camera, nought is edge on.
const GRAZING: f32 = 0.22;

fn place(at: vec2<i32>, size: vec2<f32>) -> vec3<f32> {
    let raw = depth_at(at);
    if raw <= 0.0 {
        // Nothing drawn. Marked with a zero Z, which a real point never has.
        return vec3<f32>(0.0, 0.0, 0.0);
    }
    let away = ink.drawn.w / raw;
    let uv = (vec2<f32>(at) + vec2<f32>(0.5, 0.5)) / size;
    let ndc = vec2<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);
    // `deepens.y` is tan of half the camera's vertical field - see `Ink::sees`,
    // which keeps it in step with the zoom. The aspect comes from the buffer.
    let high = ink.deepens.y * away;
    return vec3<f32>(ndc.x * high * size.x / size.y, ndc.y * high, -away);
}

fn creases(at: vec2<i32>, size: vec2<f32>, step: i32) -> f32 {
    let here = place(at, size);
    let right = place(at + vec2<i32>(step, 0), size);
    let below = place(at + vec2<i32>(0, step), size);
    let left = place(at - vec2<i32>(step, 0), size);
    let above = place(at - vec2<i32>(0, step), size);
    if here.z == 0.0 || right.z == 0.0 || below.z == 0.0 || left.z == 0.0 || above.z == 0.0 {
        // A silhouette, which `breaks` is already drawing. Saying nothing here
        // keeps the two answers from doubling up into a fat dark band.
        return 0.0;
    }
    let ahead = normalize(cross(right - here, below - here));
    let behind = normalize(cross(here - left, here - above));
    let turned = 1.0 - clamp(dot(ahead, behind), -1.0, 1.0);

    // NOT WHERE THE SURFACE IS NEARLY EDGE ON.
    //
    // A normal rebuilt from depth is only as good as the depth is smooth across a
    // pixel, and on ground seen at a glancing angle one pixel spans a lot of it -
    // so the two normals disagree on a perfectly flat field and the ground comes
    // out covered in faint scratches. Photographed on a village green: thin dark
    // lines wandering over the grass, each one a terrain triangle edge caught at
    // a shallow angle.
    //
    // The z of a view-space normal says exactly this: one is facing the camera,
    // nought is edge on. Fading the corner term out as it approaches edge-on
    // costs the corners nothing, because a corner worth drawing is a corner you
    // are looking AT.
    let facing = abs(ahead.z + behind.z) * 0.5;
    return turned * smoothstep(GRAZING, GRAZING * 2.5, facing);
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

    var laid = smoothstep(ink.drawn.y, ink.drawn.y * 2.5, found);

    // AND THE CORNERS, which is everything a silhouette leaves out - see `creases`.
    // `deepens.z` is the angle it takes to count and `deepens.w` how much of the
    // line a corner earns, which is a little less than a silhouette's: an edge you
    // can see past is a stronger statement than an edge you cannot.
    if ink.deepens.w > 0.0 {
        let turned = creases(at, size, 1);
        let sharp = smoothstep(ink.deepens.z, ink.deepens.z * 2.2, turned) * ink.deepens.w;
        laid = max(laid, sharp);
    }

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
    return vec4<f32>(mix(painted.rgb, deep, much), painted.a);
}
