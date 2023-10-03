use std::hash::Hash;

use bevy::{
    prelude::*,
    asset::{
        load_internal_asset,
        HandleUntyped,
    },
    core_pipeline::core_3d::Transparent3d,
    ecs::system::{
        lifetimeless::*,
        SystemParamItem,
    },
    pbr::{
        SetMeshBindGroup,
        SetMeshViewBindGroup,
    },
    reflect::TypeUuid,
    render::{
        mesh::GpuBufferInfo,
        render_asset::{
            PrepareAssetError,
            RenderAsset,
            RenderAssets,
        },
        render_phase::{
            AddRenderCommand,
            DrawFunctions,
            PhaseItem,
            RenderCommand,
            RenderCommandResult,
            RenderPhase,
            SetItemPipeline,
            TrackedRenderPass,
        },
        render_resource::*,
        renderer::RenderDevice,
        Render,
        RenderApp,
        RenderSet,
        view::ExtractedView,
    },
    utils::Hashed,
};

use crate::GaussianSplattingBundle;
use crate::gaussian::{
    Gaussian,
    GaussianCloud,
};


const GAUSSIAN_SHADER_HANDLE: HandleUntyped = HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 68294581);
const SPHERICAL_HARMONICS_SHADER_HANDLE: HandleUntyped = HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 834667312);

#[derive(Default)]
pub struct RenderPipelinePlugin;

impl Plugin for RenderPipelinePlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            GAUSSIAN_SHADER_HANDLE,
            "gaussian.wgsl",
            Shader::from_wgsl
        );

        load_internal_asset!(
            app,
            SPHERICAL_HARMONICS_SHADER_HANDLE,
            "spherical_harmonics.wgsl",
            Shader::from_wgsl
        );

        // TODO(future): pre-pass filter using output from core 3d render pipeline

        // TODO: gaussian splatting render pipeline
        // TODO: add a gaussian splatting render pass
        // TODO: add a gaussian splatting camera component
        // TODO: add a gaussian cloud sorting system

        app.sub_app_mut(RenderApp)
            .add_render_command::<Transparent3d, DrawGaussians>()
            .init_resource::<GaussianCloudPipeline>()
            .init_resource::<SpecializedRenderPipelines<GaussianCloudPipeline>>()
            .add_systems(
                Render,
                (
                    queue_gaussians.in_set(RenderSet::Queue),
                    prepare_instance_buffers.in_set(RenderSet::Prepare),
                ),
            );
    }

    fn finish(&self, app: &mut App) {
        app.sub_app_mut(RenderApp).init_resource::<GaussianCloudPipeline>();
    }
}



// see: https://github.com/bevyengine/bevy/blob/v0.11.3/examples/shader/shader_instancing.rs

pub type GaussianVertexBufferLayout = Hashed<InnerGaussianVertexBufferLayout>;
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct InnerGaussianVertexBufferLayout {
    layout: VertexBufferLayout,
}

// TODO: use point mesh pipeline instead of custom pipeline?
#[derive(Debug, Clone)]
pub struct GpuGaussianCloud {
    pub vertex_buffer: Buffer,
    pub vertex_count: u32,
    pub buffer_info: GpuBufferInfo,
    pub layout: GaussianVertexBufferLayout,
}
impl RenderAsset for GaussianCloud {
    type ExtractedAsset = GaussianCloud;
    type PreparedAsset = GpuGaussianCloud;
    type Param = SRes<RenderDevice>;

    /// clones the gaussian cloud
    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.clone()
    }

    /// converts the extracted gaussian cloud a into [`GpuGaussianCloud`].
    fn prepare_asset(
        gaussian_cloud: Self::ExtractedAsset,
        render_device: &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        let vertex_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            usage: BufferUsages::VERTEX,
            label: Some("gaussian cloud vertex buffer"),
            contents: bytemuck::cast_slice(gaussian_cloud.0.as_slice()),
        });

        // TODO: vertex layout only needs to be in one location (it is cached here)
        let layout = VertexBufferLayout {
            array_stride: std::mem::size_of::<Gaussian>() as u64,
            step_mode: VertexStepMode::Instance,
            attributes: vec![
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 0,
                    shader_location: 3,
                },
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: VertexFormat::Float32x4.size(),
                    shader_location: 4,
                },
            ],
        };

        Ok(GpuGaussianCloud {
            vertex_buffer,
            vertex_count: gaussian_cloud.0.len() as u32,
            buffer_info: GpuBufferInfo::NonIndexed,
            layout: GaussianVertexBufferLayout::new(
                InnerGaussianVertexBufferLayout {
                    layout,
                }
            )
        })
    }
}


#[allow(clippy::too_many_arguments)]
fn queue_gaussians(
    transparent_3d_draw_functions: Res<DrawFunctions<Transparent3d>>,
    custom_pipeline: Res<GaussianCloudPipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<GaussianCloudPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    gaussian_clouds: Res<RenderAssets<GaussianCloud>>,
    gaussian_splatting_bundles: Query<(Entity, &Handle<GaussianCloud>), With<GaussianSplattingBundle>>,
    mut views: Query<(&ExtractedView, &mut RenderPhase<Transparent3d>)>,
) {
    let draw_custom = transparent_3d_draw_functions.read().id::<DrawGaussians>();

    for (_view, mut transparent_phase) in &mut views {
        for (entity, gaussian_cloud_handle) in &gaussian_splatting_bundles {
            if let Some(_cloud) = gaussian_clouds.get(gaussian_cloud_handle) {
                let key = GaussianCloudPipelineKey {

                };

                let pipeline = pipelines.specialize(&pipeline_cache, &custom_pipeline, key);

                // TODO: use cached pipeline components from GpuGaussianCloud

                transparent_phase.add(Transparent3d {
                    entity,
                    draw_function: draw_custom,
                    distance: 0.0,
                    pipeline,
                });
            }
        }
    }
}


#[derive(Component)]
pub struct InstanceBuffer {
    buffer: Buffer,
    length: usize,
}

fn prepare_instance_buffers(
    mut commands: Commands,
    query: Query<(Entity, &GaussianSplattingBundle)>,
    clouds: Res<Assets<GaussianCloud>>,
    render_device: Res<RenderDevice>,
) {
    for (entity, instance_data) in &query {
        if let Some(cloud) = clouds.get(&instance_data.verticies) {
            let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("gaussian cloud data buffer"),
                contents: bytemuck::cast_slice(cloud.0.as_slice()),
                usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            });
            commands.entity(entity).insert(InstanceBuffer {
                buffer,
                length: cloud.0.len(),
            });
        }
    }
}

#[derive(Resource)]
pub struct GaussianCloudPipeline {
    shader: Handle<Shader>,
}

impl FromWorld for GaussianCloudPipeline {
    fn from_world(_world: &mut World) -> Self {
        GaussianCloudPipeline {
            shader: GAUSSIAN_SHADER_HANDLE.typed(),
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct GaussianCloudPipelineKey {

}

impl SpecializedRenderPipeline for GaussianCloudPipeline {
    type Key = GaussianCloudPipelineKey;

    fn specialize(&self, _key: Self::Key) -> RenderPipelineDescriptor {
        let shader_defs = vec!["MESH_BINDGROUP_1".into()];

        RenderPipelineDescriptor {
            label: Some("gaussian cloud pipeline".into()),
            layout: vec![],
            vertex: VertexState {
                shader: self.shader.clone(),
                shader_defs,
                entry_point: "vs_points".into(),
                buffers: vec![
                    VertexBufferLayout {
                        array_stride: std::mem::size_of::<Gaussian>() as u64,
                        step_mode: VertexStepMode::Instance,
                        attributes: vec![
                            // TODO: add all gaussian attributes
                            VertexAttribute {
                                format: VertexFormat::Float32x4,
                                offset: 0,
                                shader_location: 3,
                            },
                            VertexAttribute {
                                format: VertexFormat::Float32x4,
                                offset: VertexFormat::Float32x4.size(),
                                shader_location: 4,
                            },
                        ],
                    }
                ],
            },
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                shader_defs: vec![],
                entry_point: "fs_main".into(),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::Rgba8UnormSrgb,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            push_constant_ranges: Vec::new(),
        }
    }
}

type DrawGaussians = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMeshBindGroup<1>,
    DrawGaussianInstanced,
);

pub struct DrawGaussianInstanced;

impl<P: PhaseItem> RenderCommand<P> for DrawGaussianInstanced {
    type Param = SRes<RenderAssets<GaussianCloud>>;
    type ViewWorldQuery = ();
    type ItemWorldQuery = (Read<Handle<GaussianCloud>>, Read<InstanceBuffer>);

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: (),
        (gaussian_cloud_handle, instance_buffer): (&'w Handle<GaussianCloud>, &'w InstanceBuffer),
        gaussian_clouds: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let gpu_gaussian_cloud = match gaussian_clouds.into_inner().get(gaussian_cloud_handle) {
            Some(gpu_gaussian_cloud) => gpu_gaussian_cloud,
            None => return RenderCommandResult::Failure,
        };

        pass.set_vertex_buffer(0, gpu_gaussian_cloud.vertex_buffer.slice(..));
        pass.set_vertex_buffer(1, instance_buffer.buffer.slice(..));

        match &gpu_gaussian_cloud.buffer_info {
            GpuBufferInfo::Indexed {
                buffer,
                index_format,
                count,
            } => {
                pass.set_index_buffer(buffer.slice(..), 0, *index_format);
                pass.draw_indexed(0..*count, 0, 0..instance_buffer.length as u32);
            }
            GpuBufferInfo::NonIndexed => {
                pass.draw(0..gpu_gaussian_cloud.vertex_count, 0..instance_buffer.length as u32);
            }
        }
        RenderCommandResult::Success
    }
}
