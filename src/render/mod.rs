use std::hash::Hash;

use bevy::{
    prelude::*,
    asset::load_internal_asset,
    core_pipeline::core_3d::Transparent3d,
    ecs::{
        query::ROQueryItem,
        system::{
            lifetimeless::*,
            SystemParamItem,
        }
    },
    render::{
        Extract,
        extract_component::{
            ComponentUniforms,
            DynamicUniformIndex,
            UniformComponentPlugin,
        },
        globals::{
            GlobalsBuffer,
            GlobalsUniform,
        },
        render_asset::RenderAssets,
        render_phase::{
            AddRenderCommand,
            DrawFunctions,
            PhaseItem,
            PhaseItemExtraIndex,
            RenderCommand,
            RenderCommandResult,
            SetItemPipeline,
            TrackedRenderPass,
            ViewSortedRenderPhases,
        },
        render_resource::*,
        renderer::RenderDevice,
        view::{
            ExtractedView,
            RenderVisibleEntities,
            ViewUniform,
            ViewUniformOffset,
            ViewUniforms,
        },
        Render,
        RenderApp,
        RenderSet,
        sync_world::RenderEntity,
    },
};
use bevy_interleave::prelude::*;

use crate::{
    camera::GaussianCamera,
    gaussian::settings::{
        CloudSettings, DrawMode, GaussianMode, RasterizeMode
    },
    material::{
        spherical_harmonics::{
            HALF_SH_COEFF_COUNT,
            SH_COEFF_COUNT,
            SH_DEGREE,
            SH_VEC4_PLANES,
        },
        spherindrical_harmonics::{SH_4D_COEFF_COUNT, SH_4D_DEGREE_TIME},
    },
    morph::MorphPlugin,
    sort::{
        GpuSortedEntry, SortEntry, SortPlugin, SortTrigger, SortedEntriesHandle
    },
};

#[cfg(feature = "packed")]
mod packed;

#[cfg(feature = "buffer_storage")]
mod planar;

#[cfg(feature = "buffer_texture")]
mod texture;


const BINDINGS_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(675257236);
const GAUSSIAN_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(68294581);
const GAUSSIAN_2D_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(123166726);
const GAUSSIAN_3D_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(1236134564);
const GAUSSIAN_4D_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(513623421);
const HELPERS_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(134646367);
const PACKED_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(123623514);
const PLANAR_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(72345231);
const TEXTURE_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(26345735);
const TRANSFORM_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(734523534);


pub struct RenderPipelinePlugin<R: PlanarStorage> {
    _phantom: std::marker::PhantomData<R>,
}

impl<R: PlanarStorage> Default for RenderPipelinePlugin<R> {
    fn default() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<R: PlanarStorage> Plugin for RenderPipelinePlugin<R> {
    fn build(&self, app: &mut App) {
        debug!("building render pipeline plugin");

        app.add_plugins(MorphPlugin::<R>::default());

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .add_render_command::<Transparent3d, DrawGaussians<R>>()
                .init_resource::<GaussianUniformBindGroups>()
                .add_systems(ExtractSchedule, extract_gaussians::<R>)
                .add_systems(
                    Render,
                    (
                        queue_gaussian_bind_group::<R>.in_set(RenderSet::PrepareBindGroups),
                        queue_gaussian_view_bind_groups::<R>.in_set(RenderSet::PrepareBindGroups),
                        queue_gaussians::<R>.in_set(RenderSet::Queue),
                    ),
                );
        }

        // TODO: refactor common resources into a common plugin
        if app.is_plugin_added::<SortPlugin>() {
            debug!("sort plugin already added");
            return;
        }

        load_internal_asset!(
            app,
            BINDINGS_SHADER_HANDLE,
            "bindings.wgsl",
            Shader::from_wgsl
        );

        load_internal_asset!(
            app,
            GAUSSIAN_SHADER_HANDLE,
            "gaussian.wgsl",
            Shader::from_wgsl
        );

        load_internal_asset!(
            app,
            GAUSSIAN_2D_SHADER_HANDLE,
            "gaussian_2d.wgsl",
            Shader::from_wgsl
        );

        load_internal_asset!(
            app,
            GAUSSIAN_3D_SHADER_HANDLE,
            "gaussian_3d.wgsl",
            Shader::from_wgsl
        );

        load_internal_asset!(
            app,
            GAUSSIAN_4D_SHADER_HANDLE,
            "gaussian_4d.wgsl",
            Shader::from_wgsl
        );

        load_internal_asset!(
            app,
            HELPERS_SHADER_HANDLE,
            "helpers.wgsl",
            Shader::from_wgsl
        );

        load_internal_asset!(
            app,
            PACKED_SHADER_HANDLE,
            "packed.wgsl",
            Shader::from_wgsl
        );

        load_internal_asset!(
            app,
            PLANAR_SHADER_HANDLE,
            "planar.wgsl",
            Shader::from_wgsl
        );

        load_internal_asset!(
            app,
            TEXTURE_SHADER_HANDLE,
            "texture.wgsl",
            Shader::from_wgsl
        );

        load_internal_asset!(
            app,
            TRANSFORM_SHADER_HANDLE,
            "transform.wgsl",
            Shader::from_wgsl
        );

        app.add_plugins(UniformComponentPlugin::<CloudUniform>::default());
        app.add_plugins(SortPlugin);

        #[cfg(feature = "buffer_texture")]
        app.add_plugins(texture::BufferTexturePlugin);
    }

    fn finish(&self, app: &mut App) {
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<CloudPipeline<R>>()
                .init_resource::<SpecializedRenderPipelines<CloudPipeline<R>>>();
        }
    }
}


#[derive(Bundle)]
pub struct GpuCloudBundle<R: PlanarStorage> {
    pub settings: CloudSettings,
    pub settings_uniform: CloudUniform,
    pub sorted_entries: SortedEntriesHandle,
    pub cloud_handle: R::PlanarTypeHandle,
}

#[cfg(feature = "buffer_storage")]
type GpuCloudBundleQuery<R: PlanarStorage> = (
    Entity,
    &'static R::PlanarTypeHandle,
    &'static SortedEntriesHandle,
    &'static CloudSettings,
    (),
);

#[cfg(feature = "buffer_texture")]
type GpuCloudBundleQuery<R: PlanarTexture> = (
    Entity,
    &'static R::PlanarTypeHandle,
    &'static SortedEntriesHandle,
    &'static CloudSettings,
    &'static texture::GpuTextureBuffers,
);

#[allow(clippy::too_many_arguments)]
fn queue_gaussians<R: PlanarStorage>(
    gaussian_cloud_uniform: Res<ComponentUniforms<CloudUniform>>,
    transparent_3d_draw_functions: Res<DrawFunctions<Transparent3d>>,
    custom_pipeline: Res<CloudPipeline<R>>,
    mut pipelines: ResMut<SpecializedRenderPipelines<CloudPipeline<R>>>,
    pipeline_cache: Res<PipelineCache>,
    gaussian_clouds: Res<RenderAssets<R::GpuPlanarType>>,
    sorted_entries: Res<RenderAssets<GpuSortedEntry>>,
    mut transparent_render_phases: ResMut<ViewSortedRenderPhases<Transparent3d>>,
    mut views: Query<
        (
            Entity,
            &ExtractedView,
            &GaussianCamera,
            &RenderVisibleEntities,
            Option<&Msaa>,
        ),
    >,
    gaussian_splatting_bundles: Query<GpuCloudBundleQuery<R>>,
) {
    debug!("queue_gaussians");

    let warmup = views.iter().any(|(_, _, camera, _, _)| camera.warmup);
    if warmup {
        debug!("skipping gaussian cloud render during warmup");
        return;
    }

    // TODO: condition this system based on CloudBindGroup attachment
    if gaussian_cloud_uniform.buffer().is_none() {
        debug!("uniform buffer not initialized");
        return;
    };

    let draw_custom = transparent_3d_draw_functions.read().id::<DrawGaussians<R>>();

    for (
        view_entity,
        view,
        _,
        visible_entities,
        msaa,
    ) in &mut views {
        debug!("queue gaussians view");
        let Some(transparent_phase) = transparent_render_phases.get_mut(&view_entity) else {
            debug!("transparent phase not found");
            continue;
        };

        for (
            _entity,
            cloud_handle,
            sorted_entries_handle,
            settings,
            _,
        ) in &gaussian_splatting_bundles {
            debug!("queue gaussians clouds");
            if gaussian_clouds.get(cloud_handle.handle()).is_none() {
                debug!("gaussian cloud asset not found");
                return;
            }

            if sorted_entries.get(sorted_entries_handle).is_none() {
                debug!("sorted entries asset not found");
                return;
            }

            let msaa = msaa.cloned().unwrap_or_default();

            let key = CloudPipelineKey {
                aabb: settings.aabb,
                opacity_adaptive_radius: settings.opacity_adaptive_radius,
                visualize_bounding_box: settings.visualize_bounding_box,
                draw_mode: settings.draw_mode,
                gaussian_mode: settings.gaussian_mode,
                rasterize_mode: settings.rasterize_mode,
                sample_count: msaa.samples(),
                hdr: view.hdr,
            };

            let pipeline = pipelines.specialize(&pipeline_cache, &custom_pipeline, key);

            // // TODO: distance to gaussian cloud centroid
            // let rangefinder = view.rangefinder3d();

            debug!("visible entities...");
            for (
                render_entity,
                visible_entity,
            ) in visible_entities.iter::<With<R::PlanarTypeHandle>>() {
                debug!("queueing gaussian cloud entity: {:?}", render_entity);

                transparent_phase.add(Transparent3d {
                    entity: (*render_entity, *visible_entity),
                    draw_function: draw_custom,
                    distance: 0.0,
                    // distance: rangefinder
                    //     .distance_translation(&mesh_instance.transforms.transform.translation),
                    pipeline,
                    batch_range: 0..1,
                    extra_index: PhaseItemExtraIndex::NONE,
                });
            }
        }
    }
}


// TODO: pipeline trait
// TODO: support extendions /w ComputePipelineDescriptor builder
#[derive(Resource)]
pub struct CloudPipeline<R: PlanarStorage> {
    shader: Handle<Shader>,
    pub gaussian_cloud_layout: BindGroupLayout,
    pub gaussian_uniform_layout: BindGroupLayout,
    pub view_layout: BindGroupLayout,
    pub sorted_layout: BindGroupLayout,
    phantom: std::marker::PhantomData<R>,
}

impl<R: PlanarStorage> FromWorld for CloudPipeline<R> {
    fn from_world(render_world: &mut World) -> Self {
        let render_device = render_world.resource::<RenderDevice>();

        let view_layout_entries = vec![
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::all(),
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: Some(ViewUniform::min_size()),
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::all(),
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: Some(GlobalsUniform::min_size()),
                },
                count: None,
            },
        ];

        let view_layout = render_device.create_bind_group_layout(
            Some("gaussian_view_layout"),
            &view_layout_entries,
        );

        let gaussian_uniform_layout = render_device.create_bind_group_layout(
            Some("gaussian_uniform_layout"),
            &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::all(),
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: Some(CloudUniform::min_size()),
                    },
                    count: None,
                },
            ],
        );

        #[cfg(not(feature = "morph_particles"))]
        let read_only = true;
        #[cfg(feature = "morph_particles")]
        let read_only = false;

        let gaussian_cloud_layout = R::GpuPlanarType::bind_group_layout(render_device, read_only);

        #[cfg(feature = "buffer_storage")]
        let sorted_layout = render_device.create_bind_group_layout(
            Some("sorted_layout"),
            &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: true,
                        min_binding_size: BufferSize::new(std::mem::size_of::<SortEntry>() as u64),
                    },
                    count: None,
                },
            ],
        );
        #[cfg(feature = "buffer_texture")]
        let sorted_layout = texture::get_sorted_bind_group_layout(render_device);

        debug!("created cloud pipeline");

        Self {
            gaussian_cloud_layout,
            gaussian_uniform_layout,
            view_layout,
            shader: GAUSSIAN_SHADER_HANDLE,
            sorted_layout,
            phantom: std::marker::PhantomData,
        }
    }
}

// TODO: allow setting shader defines via API
// TODO: separate shader defines for each pipeline
pub struct ShaderDefines {
    pub radix_bits_per_digit: u32,
    pub radix_digit_places: u32,
    pub radix_base: u32,
    pub entries_per_invocation_a: u32,
    pub entries_per_invocation_c: u32,
    pub workgroup_invocations_a: u32,
    pub workgroup_invocations_c: u32,
    pub workgroup_entries_a: u32,
    pub workgroup_entries_c: u32,
    pub sorting_buffer_size: u32,

    pub temporal_sort_window_size: u32,
}

impl ShaderDefines {
    pub fn max_tile_count(&self, count: usize) -> u32 {
        (count as u32).div_ceil(self.workgroup_entries_c)
    }

    pub fn sorting_status_counters_buffer_size(&self, count: usize) -> usize {
        self.radix_base as usize * self.max_tile_count(count) as usize * std::mem::size_of::<u32>()
    }
}

impl Default for ShaderDefines {
    fn default() -> Self {
        let radix_bits_per_digit = 8;
        let radix_digit_places = 32 / radix_bits_per_digit;
        let radix_base = 1 << radix_bits_per_digit;
        let entries_per_invocation_a = 4;
        let entries_per_invocation_c = 4;
        let workgroup_invocations_a = radix_base * radix_digit_places;
        let workgroup_invocations_c = radix_base;
        let workgroup_entries_a = workgroup_invocations_a * entries_per_invocation_a;
        let workgroup_entries_c = workgroup_invocations_c * entries_per_invocation_c;
        let sorting_buffer_size = radix_base * radix_digit_places *
            std::mem::size_of::<u32>() as u32 + 5 * std::mem::size_of::<u32>() as u32;

        Self {
            radix_bits_per_digit,
            radix_digit_places,
            radix_base,
            entries_per_invocation_a,
            entries_per_invocation_c,
            workgroup_invocations_a,
            workgroup_invocations_c,
            workgroup_entries_a,
            workgroup_entries_c,
            sorting_buffer_size,

            temporal_sort_window_size: 16,
        }
    }
}

pub fn shader_defs(
    key: CloudPipelineKey,
) -> Vec<ShaderDefVal> {
    let defines = ShaderDefines::default();
    let mut shader_defs = vec![
        ShaderDefVal::UInt("SH_COEFF_COUNT".into(), SH_COEFF_COUNT as u32),
        ShaderDefVal::UInt("SH_4D_COEFF_COUNT".into(), SH_4D_COEFF_COUNT as u32),
        ShaderDefVal::UInt("SH_DEGREE".into(), SH_DEGREE as u32),
        ShaderDefVal::UInt("SH_DEGREE_TIME".into(), SH_4D_DEGREE_TIME as u32),
        ShaderDefVal::UInt("HALF_SH_COEFF_COUNT".into(), HALF_SH_COEFF_COUNT as u32),
        ShaderDefVal::UInt("SH_VEC4_PLANES".into(), SH_VEC4_PLANES as u32),
        ShaderDefVal::UInt("RADIX_BASE".into(), defines.radix_base),
        ShaderDefVal::UInt("RADIX_BITS_PER_DIGIT".into(), defines.radix_bits_per_digit),
        ShaderDefVal::UInt("RADIX_DIGIT_PLACES".into(), defines.radix_digit_places),
        ShaderDefVal::UInt("ENTRIES_PER_INVOCATION_A".into(), defines.entries_per_invocation_a),
        ShaderDefVal::UInt("ENTRIES_PER_INVOCATION_C".into(), defines.entries_per_invocation_c),
        ShaderDefVal::UInt("WORKGROUP_INVOCATIONS_A".into(), defines.workgroup_invocations_a),
        ShaderDefVal::UInt("WORKGROUP_INVOCATIONS_C".into(), defines.workgroup_invocations_c),
        ShaderDefVal::UInt("WORKGROUP_ENTRIES_C".into(), defines.workgroup_entries_c),

        ShaderDefVal::UInt("TEMPORAL_SORT_WINDOW_SIZE".into(), defines.temporal_sort_window_size),
    ];

    if key.aabb {
        shader_defs.push("USE_AABB".into());
    }

    if !key.aabb {
        shader_defs.push("USE_OBB".into());
    }

    if key.opacity_adaptive_radius {
        shader_defs.push("OPACITY_ADAPTIVE_RADIUS".into());
    }

    if key.visualize_bounding_box {
        shader_defs.push("VISUALIZE_BOUNDING_BOX".into());
    }

    #[cfg(feature = "morph_particles")]
    shader_defs.push("READ_WRITE_POINTS".into());

    #[cfg(feature = "packed")]
    shader_defs.push("PACKED".into());

    #[cfg(feature = "buffer_storage")]
    shader_defs.push("BUFFER_STORAGE".into());

    #[cfg(feature = "buffer_texture")]
    shader_defs.push("BUFFER_TEXTURE".into());

    // #[cfg(feature = "f16")]
    // shader_defs.push("F16".into());

    shader_defs.push("F32".into());

    #[cfg(all(feature = "packed"))]
    shader_defs.push("PACKED_F32".into());

    // #[cfg(all(feature = "f16", feature = "buffer_storage"))]
    // shader_defs.push("PLANAR_F16".into());

    #[cfg(all(feature = "buffer_storage"))]
    shader_defs.push("PLANAR_F32".into());

    // #[cfg(all(feature = "f16", feature = "buffer_texture"))]
    // shader_defs.push("PLANAR_TEXTURE_F16".into());

    #[cfg(all(feature = "buffer_texture"))]
    shader_defs.push("PLANAR_TEXTURE_F32".into());

    #[cfg(feature = "precompute_covariance_3d")]
    shader_defs.push("PRECOMPUTE_COVARIANCE_3D".into());

    #[cfg(feature = "webgl2")]
    shader_defs.push("WEBGL2".into());

    match key.gaussian_mode {
        GaussianMode::Gaussian2d => shader_defs.push("GAUSSIAN_2D".into()),
        GaussianMode::Gaussian3d => shader_defs.push("GAUSSIAN_3D".into()),
        GaussianMode::Gaussian4d => shader_defs.push("GAUSSIAN_4D".into()),
    }

    match key.gaussian_mode {
        GaussianMode::Gaussian2d | GaussianMode::Gaussian3d => {
            shader_defs.push("GAUSSIAN_3D_STRUCTURE".into());
        },
        _ => {},
    }

    match key.rasterize_mode {
        RasterizeMode::Color => shader_defs.push("RASTERIZE_COLOR".into()),
        RasterizeMode::Depth => shader_defs.push("RASTERIZE_DEPTH".into()),
        RasterizeMode::Normal => shader_defs.push("RASTERIZE_NORMAL".into()),
    }

    match key.draw_mode {
        DrawMode::All => {},
        DrawMode::Selected => shader_defs.push("DRAW_SELECTED".into()),
        DrawMode::HighlightSelected => shader_defs.push("HIGHLIGHT_SELECTED".into()),
    }

    shader_defs
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, Default)]
pub struct CloudPipelineKey {
    pub aabb: bool,
    pub visualize_bounding_box: bool,
    pub opacity_adaptive_radius: bool,
    pub draw_mode: DrawMode,
    pub gaussian_mode: GaussianMode,
    pub rasterize_mode: RasterizeMode,
    pub sample_count: u32,
    pub hdr: bool,
}

impl<R: PlanarStorage> SpecializedRenderPipeline for CloudPipeline<R> {
    type Key = CloudPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let shader_defs = shader_defs(key);

        let format = if key.hdr {
            TextureFormat::Rgba16Float
        } else {
            TextureFormat::Rgba8UnormSrgb
        };

        debug!("specializing cloud pipeline");

        RenderPipelineDescriptor {
            label: Some("gaussian cloud render pipeline".into()),
            layout: vec![
                self.view_layout.clone(),
                self.gaussian_uniform_layout.clone(),
                self.gaussian_cloud_layout.clone(),
                self.sorted_layout.clone(),
            ],
            vertex: VertexState {
                shader: self.shader.clone(),
                shader_defs: shader_defs.clone(),
                entry_point: "vs_points".into(),
                buffers: vec![],
            },
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                shader_defs,
                entry_point: "fs_main".into(),
                targets: vec![Some(ColorTargetState {
                    format,
                    blend: Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                unclipped_depth: false,
                cull_mode: None,
                conservative: false,
                polygon_mode: PolygonMode::Fill,
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: CompareFunction::GreaterEqual,
                stencil: StencilState {
                    front: StencilFaceState::IGNORE,
                    back: StencilFaceState::IGNORE,
                    read_mask: 0,
                    write_mask: 0,
                },
                bias: DepthBiasState {
                    constant: 0,
                    slope_scale: 0.0,
                    clamp: 0.0,
                },
            }),
            multisample: MultisampleState {
                count: key.sample_count,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            push_constant_ranges: Vec::new(),
            zero_initialize_workgroup_memory: true,
        }
    }
}

type DrawGaussians<R: PlanarStorage> = (
    SetItemPipeline,
    SetGaussianViewBindGroup<0>,
    SetGaussianUniformBindGroup<1>,
    DrawGaussianInstanced<R>,
);


#[derive(Component, ShaderType, Clone, Copy)]
pub struct CloudUniform {
    pub transform: Mat4,
    pub global_opacity: f32,
    pub global_scale: f32,
    pub count: u32,
    pub count_root_ceil: u32,
    pub time: f32,
    pub time_start: f32,
    pub time_stop: f32,
}

#[allow(clippy::type_complexity)]
pub fn extract_gaussians<R: PlanarStorage>(
    mut commands: Commands,
    mut prev_commands_len: Local<usize>,
    asset_server: Res<AssetServer>,
    gaussian_cloud_res: Res<RenderAssets<R::GpuPlanarType>>,
    gaussians_query: Extract<
        Query<(
            RenderEntity,
            &ViewVisibility,
            &R::PlanarTypeHandle,
            &SortedEntriesHandle,
            &CloudSettings,
            &GlobalTransform,
        )>,
    >,
) {
    let mut commands_list = Vec::with_capacity(*prev_commands_len);
    // let visible_gaussians = gaussians_query.iter().filter(|(_, vis, ..)| vis.is_visible());

    for (
        entity,
        visibility,
        cloud_handle,
        sorted_entries,
        settings,
        transform,
    ) in gaussians_query.iter() {
        debug!("extracting gaussian cloud entity: {:?}", entity);

        if !visibility.get() {
            debug!("gaussian cloud not visible");
            continue;
        }

        if let Some(load_state) = asset_server.get_load_state(cloud_handle.handle()) {
            if load_state.is_loading() {
                debug!("gaussian cloud asset loading");
                continue;
            }
        }

        if gaussian_cloud_res.get(cloud_handle.handle()).is_none() {
            debug!("gaussian cloud asset not found");
            continue;
        }

        let cloud = gaussian_cloud_res.get(cloud_handle.handle()).unwrap();

        let settings_uniform = CloudUniform {
            transform: transform.compute_matrix(),
            global_opacity: settings.global_opacity,
            global_scale: settings.global_scale,
            count: cloud.len() as u32,
            count_root_ceil: (cloud.len() as f32).sqrt().ceil() as u32,
            time: settings.time,
            time_start: settings.time_start,
            time_stop: settings.time_stop,
        };

        commands_list.push((
            entity,
            GpuCloudBundle::<R> {
                settings: settings.clone(),
                settings_uniform,
                sorted_entries: sorted_entries.clone(),
                cloud_handle: cloud_handle.clone(),
            },
        ));
    }
    *prev_commands_len = commands_list.len();
    commands.insert_or_spawn_batch(commands_list);
}


#[derive(Resource, Default)]
pub struct GaussianUniformBindGroups {
    pub base_bind_group: Option<BindGroup>,
}

#[derive(Component)]
pub struct SortBindGroup {
    pub sorted_bind_group: BindGroup,
}

#[allow(clippy::too_many_arguments)]
fn queue_gaussian_bind_group<R: PlanarStorage>(
    mut commands: Commands,
    mut groups: ResMut<GaussianUniformBindGroups>,
    gaussian_cloud_pipeline: Res<CloudPipeline<R>>,
    render_device: Res<RenderDevice>,
    gaussian_uniforms: Res<ComponentUniforms<CloudUniform>>,
    asset_server: Res<AssetServer>,
    gaussian_cloud_res: Res<RenderAssets<R::GpuPlanarType>>,
    sorted_entries_res: Res<RenderAssets<GpuSortedEntry>>,
    gaussian_clouds: Query<GpuCloudBundleQuery<R>>,
    #[cfg(feature = "buffer_texture")]
    gpu_images: Res<RenderAssets<bevy::render::texture::GpuImage>>,
) {
    let Some(model) = gaussian_uniforms.buffer() else {
        return;
    };

    // TODO: overloaded system, move to resource setup system
    groups.base_bind_group = Some(render_device.create_bind_group(
        "gaussian_uniform_bind_group",
        &gaussian_cloud_pipeline.gaussian_uniform_layout,
        &[
            BindGroupEntry {
                binding: 0,
                resource: BindingResource::Buffer(BufferBinding {
                    buffer: model,
                    offset: 0,
                    size: CloudUniform::min_size().into(),
                }),
            },
        ],
    ));

    for query in gaussian_clouds.iter() {
        let entity = query.0;
        let cloud_handle = query.1;
        let sorted_entries_handle = query.2;

        #[cfg(feature = "buffer_texture")]
        let texture_buffers = query.4;

        // TODO: add asset loading indicator (and maybe streamed loading)
        if let Some(load_state) = asset_server.get_load_state(cloud_handle.handle()) {
            if load_state.is_loading() {
                debug!("queue gaussian bind group: cloud asset loading");
                continue;
            }
        }

        if gaussian_cloud_res.get(cloud_handle.handle()).is_none() {
            debug!("queue gaussian bind group: cloud asset not found");
            continue;
        }

        if let Some(load_state) = asset_server.get_load_state(&sorted_entries_handle.0) {
            if load_state.is_loading() {
                debug!("queue gaussian bind group: sorted entries asset loading");
                continue;
            }
        }

        if sorted_entries_res.get(&sorted_entries_handle.0).is_none() {
            debug!("queue gaussian bind group: sorted entries asset not found");
            continue;
        }

        #[cfg(not(feature = "buffer_texture"))]
        let cloud = gaussian_cloud_res.get(cloud_handle.handle()).unwrap();

        let sorted_entries = sorted_entries_res.get(&sorted_entries_handle.0).unwrap();

        #[cfg(feature = "buffer_storage")]
        let sorted_bind_group = render_device.create_bind_group(
            "render_sorted_bind_group",
            &gaussian_cloud_pipeline.sorted_layout,
            &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &sorted_entries.sorted_entry_buffer,
                        offset: 0,
                        size: BufferSize::new((cloud.len() * std::mem::size_of::<SortEntry>()) as u64),
                    }),
                },
            ],
        );
        #[cfg(feature = "buffer_texture")]
        let sorted_bind_group = render_device.create_bind_group(
            Some("render_sorted_bind_group"),
            &gaussian_cloud_pipeline.sorted_layout,
            &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView( // TODO: convert to texture view array
                        &gpu_images.get(&sorted_entries.texture).unwrap().texture_view
                    ),
                },
            ],
        );

        debug!("inserting sorted bind group");

        commands.entity(entity).insert(SortBindGroup {
            sorted_bind_group,
        });
    }
}

#[derive(Component)]
pub struct GaussianViewBindGroup {
    pub value: BindGroup,
}

// TODO: move to gaussian camera module
// TODO: remove cloud pipeline dependency by separating view layout
pub fn queue_gaussian_view_bind_groups<R: PlanarStorage>(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    gaussian_cloud_pipeline: Res<CloudPipeline<R>>,
    view_uniforms: Res<ViewUniforms>,
    views: Query<
        (
            Entity,
            &ExtractedView,
        ),
        With<GaussianCamera>,
    >,
    globals_buffer: Res<GlobalsBuffer>,
) {
    if let (
        Some(view_binding),
        Some(globals),
    ) = (
        view_uniforms.uniforms.binding(),
        globals_buffer.buffer.binding(),
    ) {
        for (
            entity,
            _extracted_view,
        ) in &views {
            let layout = &gaussian_cloud_pipeline.view_layout;

            let entries = vec![
                BindGroupEntry {
                    binding: 0,
                    resource: view_binding.clone(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: globals.clone(),
                },
            ];

            let view_bind_group = render_device.create_bind_group(
                "gaussian_view_bind_group",
                layout,
                &entries,
            );

            debug!("inserting gaussian view bind group");

            commands.entity(entity).insert(GaussianViewBindGroup {
                value: view_bind_group,
            });
        }
    }
}

pub struct SetGaussianViewBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetGaussianViewBindGroup<I> {
    type Param = ();
    type ViewQuery = (
        Read<GaussianViewBindGroup>,
        Read<ViewUniformOffset>,
    );
    type ItemQuery = ();

    #[inline]
    fn render<'w>(
        _item: &P,
        (
            gaussian_view_bind_group,
            view_uniform,
        ): ROQueryItem<
            'w,
            Self::ViewQuery,
        >,
        _entity: Option<()>,
        _: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(
            I,
            &gaussian_view_bind_group.value,
            &[view_uniform.offset],
        );

        debug!("set gaussian view bind group");

        RenderCommandResult::Success
    }
}


pub struct SetGaussianUniformBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetGaussianUniformBindGroup<I> {
    type Param = SRes<GaussianUniformBindGroups>;
    type ViewQuery = ();
    type ItemQuery = Read<DynamicUniformIndex<CloudUniform>>;

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: (),
        gaussian_cloud_index: Option<ROQueryItem<'w, Self::ItemQuery>>,
        bind_groups: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let bind_groups = bind_groups.into_inner();
        let bind_group = bind_groups.base_bind_group.as_ref().expect("bind group not initialized");

        let mut set_bind_group = |indices: &[u32]| pass.set_bind_group(I, bind_group, indices);

        if gaussian_cloud_index.is_none() {
            debug!("skipping gaussian uniform bind group\n");
            return RenderCommandResult::Skip;
        }

        let gaussian_cloud_index = gaussian_cloud_index.unwrap().index();
        set_bind_group(&[gaussian_cloud_index]);

        debug!("set gaussian uniform bind group");

        RenderCommandResult::Success
    }
}

pub struct DrawGaussianInstanced<R: PlanarStorage> {
    phantom: std::marker::PhantomData<R>,
}

impl<R: PlanarStorage> Default for DrawGaussianInstanced<R> {
    fn default() -> Self {
        Self {
            phantom: std::marker::PhantomData,
        }
    }
}

impl<P: PhaseItem, R: PlanarStorage> RenderCommand<P> for DrawGaussianInstanced<R> {
    type Param = SRes<RenderAssets<R::GpuPlanarType>>;
    type ViewQuery = Read<SortTrigger>;
    type ItemQuery = (
        Read<R::PlanarTypeHandle>,
        Read<PlanarStorageBindGroup<R>>,
        Read<SortBindGroup>,
    );

    #[inline]
    fn render<'w>(
        _item: &P,
        view: &'w SortTrigger,
        entity: Option<(
            &'w R::PlanarTypeHandle,
            &'w PlanarStorageBindGroup<R>,
            &'w SortBindGroup,
        )>,
        gaussian_clouds: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        debug!("render call");

        let (
            handle,
            planar_bind_groups,
            sort_bind_groups,
        ) = entity.expect("gaussian cloud entity not found");

        let gpu_gaussian_cloud = match gaussian_clouds.into_inner().get(handle.handle()) {
            Some(gpu_gaussian_cloud) => gpu_gaussian_cloud,
            None => { debug!("gpu cloud not found"); return RenderCommandResult::Skip },
        };

        debug!("drawing indirect");

        pass.set_bind_group(
            2,
            &planar_bind_groups.bind_group,
            &[],
        );

        // TODO: align dynamic offset to `min_storage_buffer_offset_alignment`
        pass.set_bind_group(
            3,
            &sort_bind_groups.sorted_bind_group,
            &[
                view.camera_index as u32 * std::mem::size_of::<SortEntry>() as u32 * gpu_gaussian_cloud.len() as u32,
            ],
        );

        #[cfg(feature = "webgl2")]
        pass.draw(0..4, 0..gpu_gaussian_cloud.count as u32);

        #[cfg(not(feature = "webgl2"))]
        pass.draw_indirect(&gpu_gaussian_cloud.draw_indirect_buffer(), 0);

        RenderCommandResult::Success
    }
}
