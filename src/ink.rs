//! The line round things.
//!
//! # Why the models' own outlines were not enough
//!
//! Every figure in `dev/art` is wrapped in an inverted hull - a copy of the mesh
//! pushed out along its normals, turned inside out and painted near-black, so back
//! face culling leaves exactly the silhouette. It is the classic stylised answer, it
//! needs nothing from the engine, and up close it is very good: photographed at three
//! metres, a shelter has a clean unbroken line all the way round it.
//!
//! It has two limits and both of them show in the same photograph. The hull is seven
//! centimetres of WORLD, so at fifty metres it is a fraction of a pixel and simply
//! is not there - a city of towers a hundred metres off has no line on it at all,
//! which is exactly where a silhouette matters most. And it belongs to a model, so
//! the terrain, the roads, the sea and the grove have no line under any circumstances:
//! a cliff against the sky reads as a colour change.
//!
//! # What this is instead
//!
//! A pass over the finished frame that finds where the DEPTH breaks - one surface
//! ending and another beginning - and puts ink there. Cost is the screen rather than
//! the scene, the width is in pixels so it is the same at every distance, and it
//! knows nothing about what a thing is, so it works on the ground, the roads and the
//! trees without any of them being told about it.
//!
//! Codex's research recommends exactly this split and it is what the production
//! literature does: screen-space ink for the environment, a hull for the player and
//! for hero assets where an artist wants to sculpt the line by hand.
//!
//! # The depth is the one the frame was actually drawn with
//!
//! Not a prepass. `CloudShade` moves grass and water in the VERTEX stage, and a
//! prepass drawn with Bevy's undeformed vertex path would put its depth where the
//! visible blade is not - a halo round every tuft, and the one trap Codex flagged as
//! most likely to bite. `ViewDepthTexture` is the depth buffer the main pass just
//! finished writing, deformation and all, so there is nothing to keep in step.
//!
//! # And why a flat field is not covered in ink
//!
//! A naive difference of depths inks every steep surface, because a road running away
//! from the camera has a large depth difference between one pixel and the next. What
//! separates a slope from an edge is that a slope is STRAIGHT: the depth at a pixel
//! is the average of its two neighbours. So each pair of opposite neighbours is asked
//! how far the middle sits from the line between them, which is nought on any flat
//! surface at any angle and large only where one surface ends and another starts.

use bevy::{
    core_pipeline::{
        core_3d::graph::{Core3d, Node3d},
        fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    },
    ecs::query::QueryItem,
    prelude::*,
    render::{
        extract_component::{
            ComponentUniforms, DynamicUniformIndex, ExtractComponent, ExtractComponentPlugin,
            UniformComponentPlugin,
        },
        render_graph::{
            NodeRunError, RenderGraphApp, RenderGraphContext, RenderLabel, ViewNode, ViewNodeRunner,
        },
        render_resource::{
            binding_types::{sampler, texture_2d, texture_depth_2d, texture_depth_2d_multisampled, uniform_buffer},
            *,
        },
        renderer::{RenderContext, RenderDevice},
        view::{Msaa, ViewDepthTexture, ViewTarget},
        RenderApp,
    },
};

const INK_SHADER: &str = "shaders/ink.wgsl";

/// How the line is drawn, and where it is allowed to appear.
///
/// Carried on the camera so it can be tuned from the game rather than from a number
/// buried in a shader - the same reason `CloudShade`'s weather rides on a uniform.
// In a module of its own for one reason: the `ShaderType` derive expands a
// per-field `check` helper that nothing calls, the dead-code lint pins its
// warning to the field spans, and no attribute ON the struct or its fields
// reaches the scope the derive puts them in. The allow is scoped to exactly
// this struct; everything else in the file still gets the lint.
mod uniform {
    #![allow(dead_code)]

    use super::*;

    #[derive(Component, Clone, Copy, ExtractComponent, ShaderType)]
    pub struct Ink {
        /// The line's colour, and how much of it is laid down.
        ///
        /// Charcoal rather than black: a line at absolute nought belongs to no lighting
        /// and reads as a hole cut in the picture. This is the same value the models'
        /// own hulls are painted, so the two kinds of line match where both appear.
        pub colour: Vec4,
        /// How wide the line is in pixels, how sharply a break has to bend before it
        /// counts, how much of the far distance is left alone, and the camera's near
        /// plane - which is what turns a depth buffer reading into metres.
        pub drawn: Vec4,
        /// How far a line darkens what it is drawn on, at most.
        ///
        /// # A line that came out WHITE at dusk
        ///
        /// The ink was mixed toward its own colour, which is charcoal - and charcoal is
        /// brighter than an unlit wall. Photographed at seven in the evening, every
        /// building in the city was drawn with a pale line round it, which is the exact
        /// opposite of the effect. It read as a glow.
        ///
        /// A line is not a colour, it is a DARKENING: whichever is darker of the
        /// charcoal and this share of what is already there. In daylight that is the
        /// charcoal and the line is a strong one; at night it is a fraction of an
        /// already dark surface, and the line stays a line.
        pub deepens: Vec4,
    }
}
pub use uniform::Ink;

/// How wide the line is, in pixels at 1080p.
///
/// One pixel is a hairline that disappears on a big screen; three is a cartoon.
/// Codex's recommendation for an environment line is 1.25 to 1.5 at this height, with
/// a hero hull - if one is kept - reading closer to two.
///
/// # It is a width now, and it was a sampling radius before
///
/// This was rounded straight into a neighbour offset, so what it actually set was how
/// far apart the samples are and the width was whatever the threshold happened to
/// produce. The shader scales it by the screen's own height and uses it to DILATE an
/// edge found at one pixel, which is a width somebody chose.
pub const INK_WIDE: f32 = 1.15;

/// How far a surface has to bend, as a share of its own depth, to be an edge.
///
/// Asked of the RECIPROCAL depth the rasteriser interpolates rather than of metres -
/// see the shader, and Codex's note on why the two are not the same question. Read
/// as: the middle of three samples has to sit this far off the line between them,
/// measured as a fraction of how deep it is. Two per cent is a step of twenty
/// centimetres at ten metres and six at three hundred, which is what a line of
/// constant weight in SCREEN space actually asks for.
const INK_BREAKS_AT: f32 = 0.02;

/// Beyond this, in metres, the line fades out.
///
/// A hill four kilometres off has a break at every fold, and inking all of them turns
/// the horizon into a scribble. It is also where a one-pixel line starts to shimmer
/// as the camera moves, which the photosensitivity rule in this project will not
/// have.
const INK_REACHES: f32 = 900.0;

/// How much of the line is actually laid down.
///
/// # Breath of the Wild, not Wind Waker
///
/// At full strength every edge in the world is a hard black rule, which is a
/// beautiful look and is not this one. What was asked for is semi-cel: the line is
/// there, it reads, and it does not become the drawing. Seven tenths of a charcoal
/// that is itself a darkening rather than a paint - see `Ink::deepens`.
const INK_STRENGTH: f32 = 0.7;

/// The near plane the ink assumes if the camera does not say.
const NEAR_ENOUGH: f32 = 0.1;

/// The most of a surface's own value a line may leave.
///
/// A quarter is a definite line without being a hole: the eye reads it as the same
/// mark in a lit street and in a dark one, because in both it is the same ratio
/// against what surrounds it.
const INK_DEEPENS: f32 = 0.25;

impl Default for Ink {
    fn default() -> Self {
        Ink {
            colour: Vec4::new(0.05, 0.055, 0.07, INK_STRENGTH),
            drawn: Vec4::new(INK_WIDE, INK_BREAKS_AT, INK_REACHES, NEAR_ENOUGH),
            deepens: Vec4::new(INK_DEEPENS, 0.0, 0.0, 0.0),
        }
    }
}

impl Ink {
    /// The same settings, told what the camera's near plane is.
    ///
    /// A depth buffer holds nothing anybody can measure with until it is divided by
    /// this. Reverse-Z, infinite far: the view-space distance is `near / depth`.
    pub fn at(near: f32) -> Ink {
        let mut ink = Ink::default();
        ink.drawn.w = near;
        ink
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct InkLabel;

#[derive(Default)]
struct InkNode;

impl ViewNode for InkNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static ViewDepthTexture,
        &'static Msaa,
        &'static Ink,
        &'static DynamicUniformIndex<Ink>,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (target, depth, msaa, _ink, which): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let drawn = world.resource::<InkPipeline>();
        let cache = world.resource::<PipelineCache>();
        let many = *msaa != Msaa::Off;
        let lit = target.main_texture_format() == ViewTarget::TEXTURE_FORMAT_HDR;
        let Some(pipeline) = cache.get_render_pipeline(drawn.drawn[usize::from(many)][usize::from(lit)])
        else {
            return Ok(());
        };
        let settings = world.resource::<ComponentUniforms<Ink>>();
        let Some(bound) = settings.uniforms().binding() else {
            return Ok(());
        };

        // Source and destination, flipped by the call - see Bevy's own post-process
        // example. Reading and writing one texture is undefined and looks it.
        let over = target.post_process_write();
        let group = render_context.render_device().create_bind_group(
            "ink_bind_group",
            &drawn.layouts[usize::from(many)],
            &BindGroupEntries::sequential((
                over.source,
                &drawn.sampler,
                depth.view(),
                bound.clone(),
            )),
        );

        let mut pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("ink_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: over.destination,
                resolve_target: None,
                ops: Operations::default(),
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_render_pipeline(pipeline);
        pass.set_bind_group(0, &group, &[which.index()]);
        pass.draw(0..3, 0..1);
        Ok(())
    }
}

/// Four pipelines, over two questions the view answers and the shader cannot.
///
/// A MULTISAMPLED depth texture is a different type from a plain one, and the main
/// camera runs `Msaa::Sample4`, so a shader declaring `texture_depth_2d` would not
/// bind against its depth buffer at all. And an HDR camera's target is
/// `Rgba16Float` where an ordinary one is `Rgba8UnormSrgb`, and a pipeline built for
/// the wrong one is refused by the pass it is set on.
///
/// Both are properties of the view rather than of the effect, so both are built and
/// the node picks. Bevy's own depth-of-field specialises on the first of them for the
/// same reason.
#[derive(Resource)]
struct InkPipeline {
    /// Indexed by whether the view is multisampled.
    layouts: [BindGroupLayout; 2],
    sampler: Sampler,
    /// Indexed by multisampled, then by whether the target is HDR.
    drawn: [[CachedRenderPipelineId; 2]; 2],
}

impl FromWorld for InkPipeline {
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>();
        let layout = |many: bool| {
            device.create_bind_group_layout(
                if many { "ink_layout_msaa" } else { "ink_layout" },
                &BindGroupLayoutEntries::sequential(
                    ShaderStages::FRAGMENT,
                    (
                        texture_2d(TextureSampleType::Float { filterable: true }),
                        sampler(SamplerBindingType::Filtering),
                        if many {
                            texture_depth_2d_multisampled()
                        } else {
                            texture_depth_2d()
                        },
                        uniform_buffer::<Ink>(true),
                    ),
                ),
            )
        };
        let layouts = [layout(false), layout(true)];
        let sampler = device.create_sampler(&SamplerDescriptor::default());
        let shader = world.load_asset(INK_SHADER);

        let mut queue = |layout: &BindGroupLayout, many: bool, lit: bool| {
            world
                .resource_mut::<PipelineCache>()
                .queue_render_pipeline(RenderPipelineDescriptor {
                    label: Some("ink_pipeline".into()),
                    layout: vec![layout.clone()],
                    vertex: fullscreen_shader_vertex_state(),
                    fragment: Some(FragmentState {
                        shader: shader.clone(),
                        shader_defs: if many {
                            vec!["MULTISAMPLED".into()]
                        } else {
                            vec![]
                        },
                        entry_point: "fragment".into(),
                        targets: vec![Some(ColorTargetState {
                            format: if lit {
                                ViewTarget::TEXTURE_FORMAT_HDR
                            } else {
                                TextureFormat::bevy_default()
                            },
                            blend: None,
                            write_mask: ColorWrites::ALL,
                        })],
                    }),
                    primitive: PrimitiveState::default(),
                    depth_stencil: None,
                    multisample: MultisampleState::default(),
                    push_constant_ranges: vec![],
                    zero_initialize_workgroup_memory: false,
                })
        };
        let drawn = [
            [queue(&layouts[0], false, false), queue(&layouts[0], false, true)],
            [queue(&layouts[1], true, false), queue(&layouts[1], true, true)],
        ];

        InkPipeline { layouts, sampler, drawn }
    }
}

/// The ink is not switched off.
///
/// # A debug line that shipped
///
/// While hunting a streak on the roads, the final blend was neutralised to
/// `much * 0.0` to prove the outline pass was not causing it. It was not - and the
/// zero stayed in, through five commits and several rounds of road photographs
/// presented as evidence, with every outline in the game switched off the whole time.
/// Codex found it by reading the shader.
///
/// This is a smoke alarm, not a proof: it reads the source and refuses a blend
/// multiplied by a literal nought, which is exactly the shape that shipped. Proving
/// the line is actually DRAWN wants rendered coverage from the shot matrix, which is
/// a bigger instrument and is tracked separately. A cheap guard that would have
/// caught the real mistake is worth more than an expensive one that was not written.
#[cfg(test)]
mod ink_is_on {
    #[test]
    fn the_outline_pass_is_not_multiplied_by_nought() {
        let shader = include_str!("../assets/shaders/ink.wgsl");
        let blend = shader
            .lines()
            .find(|line| line.contains("mix(painted.rgb"))
            .expect("the ink shader has no final blend");
        assert!(
            !blend.contains("0.0"),
            "the ink's final blend is scaled by a constant - the outline pass is off:
{blend}"
        );
    }
}

pub struct InkPlugin;

impl Plugin for InkPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ExtractComponentPlugin::<Ink>::default(),
            UniformComponentPlugin::<Ink>::default(),
        ));
        let Some(render) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render
            .add_render_graph_node::<ViewNodeRunner<InkNode>>(Core3d, InkLabel)
            // AFTER TONEMAPPING, so the line is laid on the picture the player sees
            // rather than on values that are about to be squeezed. A charcoal line
            // put down before tonemapping comes out grey.
            .add_render_graph_edges(
                Core3d,
                (Node3d::Tonemapping, InkLabel, Node3d::EndMainPassPostProcessing),
            );
    }

    fn finish(&self, app: &mut App) {
        let Some(render) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render.init_resource::<InkPipeline>();
    }
}
