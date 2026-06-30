//! 玻璃渲染器。
//!
//! [`GlassRenderer`] 管理所有 wgpu 资源（管线、纹理、uniform buffer 等），
//! 对窗口无感知。`-demo` 负责创建 winit 窗口和 surface，
//! 将 `Device` / `Queue` 传入，每帧调用 `render()` 录制命令。

use crate::config::RendererConfig;
use crate::input::RenderInput;
use crate::shader::ShaderLoader;
use liquidglass0_core::GlassPanel;

/// 传给模糊 compute shader 的参数。
///
/// 16 字节，`#[repr(C)]` 确保与 WGSL 布局一致。
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct BlurUniforms {
    /// 纹理尺寸（像素）。
    texture_size: [u32; 2],
    /// 模糊半径（像素）。
    blur_radius: f32,
    /// 高斯核半宽（`ceil(3σ)`）。
    kernel_half: u32,
}

/// 玻璃面板 + 材质 + 光源的统一 uniform。
///
/// 208 字节（13 × vec4f），`#[repr(C)]` 确保与 WGSL `GlassUniforms` 布局一致。
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct GlassUniforms {
    /// center.xy, half_size.xy。
    panel_info: [f32; 4],
    /// corner_radius, bevel_width, bevel_depth, refractive_index。
    shape_params: [f32; 4],
    /// chromatic_strength, fresnel_intensity, specular_intensity, specular_shininess。
    optical_a: [f32; 4],
    /// fresnel_color.r, .g, .b, _pad。
    fresnel_col: [f32; 4],
    /// tint_color.r, .g, .b, tint_opacity。
    tint_col: [f32; 4],
    /// background_opacity, saturation, contrast, brightness。
    material: [f32; 4],
    /// cursor_x, cursor_y, time, light_count。
    interaction: [f32; 4],
    /// light0.x, light0.y, light1.x, light1.y。
    light01_pos: [f32; 4],
    /// light2.x, light2.y, _pad, _pad。
    light2_pos: [f32; 4],
    /// light0.color.r, .g, .b, _pad。
    light0_col: [f32; 4],
    /// light1.color.r, .g, .b, _pad。
    light1_col: [f32; 4],
    /// light2.color.r, .g, .b, _pad。
    light2_col: [f32; 4],
    /// thickness_multiplier, shadow_opacity, shadow_blur, shadow_offset_y。
    shadow_params: [f32; 4],
}

impl GlassUniforms {
    /// 从渲染输入构造 uniform 数据。
    fn from_input(input: &RenderInput, panel: &GlassPanel) -> Self {
        let lights = &input.scene.lights;

        // 厚度乘数：尺寸自适应
        let min_half = panel.half_size.x.min(panel.half_size.y);
        let reference = panel.reference_size.max(1.0);
        let thickness_multiplier = (min_half / reference).clamp(1.0, 2.5);

        Self {
            panel_info: [
                panel.center.x,
                panel.center.y,
                panel.half_size.x,
                panel.half_size.y,
            ],
            shape_params: [
                panel.corner_radius,
                panel.bevel_width,
                panel.bevel_depth,
                input.material.refractive_index,
            ],
            optical_a: [
                input.material.chromatic_strength,
                input.material.fresnel_intensity,
                input.material.specular_intensity,
                input.material.specular_shininess,
            ],
            fresnel_col: [
                input.material.fresnel_color.x,
                input.material.fresnel_color.y,
                input.material.fresnel_color.z,
                0.0,
            ],
            tint_col: [
                input.material.tint_color.x,
                input.material.tint_color.y,
                input.material.tint_color.z,
                input.material.tint_opacity,
            ],
            material: [
                input.material.background_opacity,
                input.material.saturation,
                input.material.contrast,
                input.material.brightness,
            ],
            interaction: [
                input.interaction.cursor_pos.x,
                input.interaction.cursor_pos.y,
                input.interaction.displacement,
                lights.len() as f32,
            ],
            light01_pos: [
                lights[0].position.x,
                lights[0].position.y,
                lights[1].position.x,
                lights[1].position.y,
            ],
            light2_pos: [lights[2].position.x, lights[2].position.y, 0.0, 0.0],
            light0_col: [lights[0].color.x, lights[0].color.y, lights[0].color.z, 0.0],
            light1_col: [lights[1].color.x, lights[1].color.y, lights[1].color.z, 0.0],
            light2_col: [lights[2].color.x, lights[2].color.y, lights[2].color.z, 0.0],
            shadow_params: [
                thickness_multiplier,
                input.material.shadow_opacity,
                input.material.shadow_blur,
                input.material.shadow_offset_y,
            ],
        }
    }
}

/// 中间纹理只读句柄集合。
///
/// 由 [`GlassRenderer::intermediate_textures`] 返回。
/// 在 `render()` 调用后，这些纹理的内容为本帧最新值。
/// 外部调试工具可通过 `copy_texture_to_buffer` 将内容读回 CPU。
pub struct IntermediateTextures<'a> {
    /// 折射位移纹理。
    pub displacement: &'a wgpu::Texture,
    /// 水平模糊中间纹理。
    pub h_blur: &'a wgpu::Texture,
    /// 垂直模糊中间纹理。
    pub v_blur: &'a wgpu::Texture,
}

/// 玻璃渲染器。
pub struct GlassRenderer {
    /// wgpu 设备。
    device: wgpu::Device,
    /// wgpu 命令队列。
    queue: wgpu::Queue,

    /// 水平模糊 compute pipeline。
    blur_h_pipeline: wgpu::ComputePipeline,
    /// 垂直模糊 compute pipeline。
    blur_v_pipeline: wgpu::ComputePipeline,
    /// 折射 compute pipeline。
    refract_pipeline: wgpu::ComputePipeline,
    /// 合成 render pipeline（全屏三角形 + fragment shader）。
    composite_pipeline: wgpu::RenderPipeline,

    /// 模糊 pass 共用的 bind group layout。
    blur_bind_layout: wgpu::BindGroupLayout,
    /// 折射 pass 的 bind group layout。
    refract_bind_layout: wgpu::BindGroupLayout,
    /// 合成 pass 的 bind group layout。
    composite_bind_layout: wgpu::BindGroupLayout,

    /// 线性采样器（clamp to edge）。
    sampler: wgpu::Sampler,

    /// 模糊 uniform buffer（16 bytes）。
    blur_uniform_buf: wgpu::Buffer,
    /// CPU 侧模糊 uniform 镜像。
    blur_uniforms: BlurUniforms,

    /// 玻璃 uniform buffer（192 bytes）。
    glass_uniform_buf: wgpu::Buffer,

    /// 水平模糊中间纹理。
    h_blur_tex: wgpu::Texture,
    /// 垂直模糊中间纹理。
    v_blur_tex: wgpu::Texture,
    /// 水平模糊结果视图。
    h_blur_view: wgpu::TextureView,
    /// 垂直模糊结果视图。
    v_blur_view: wgpu::TextureView,

    /// 位移纹理（折射偏移）。
    displacement_tex: wgpu::Texture,
    /// 位移纹理视图。
    displacement_view: wgpu::TextureView,

    /// 垂直模糊 bind group（输入=h_blur_view，输出=v_blur_view）。
    blur_v_bind_group: wgpu::BindGroup,
    /// 折射 bind group（输出=displacement_view，uniform=glass_uniform_buf）。
    refract_bind_group: wgpu::BindGroup,
    /// 合成 bind group。
    composite_bind_group: wgpu::BindGroup,

    /// 当前中间纹理尺寸。
    current_size: (u32, u32),
    /// 渲染器配置备份。
    config: RendererConfig,

    // 着色器模块句柄，用于热更新时重建管线
    /// 水平模糊着色器模块。
    blur_h_module: wgpu::ShaderModule,
    /// 垂直模糊着色器模块。
    blur_v_module: wgpu::ShaderModule,
    /// 折射着色器模块。
    refract_module: wgpu::ShaderModule,
    /// 全屏三角形顶点着色器模块。
    vs_module: wgpu::ShaderModule,
    /// 合成片段着色器模块。
    fs_module: wgpu::ShaderModule,
    /// 管线重建用的 pipeline layout。
    blur_pipeline_layout: wgpu::PipelineLayout,
    refract_pipeline_layout: wgpu::PipelineLayout,
    composite_pipeline_layout: wgpu::PipelineLayout,
    /// naga_oil composer，用于热更新时解析 `#import`。
    composer: std::cell::RefCell<naga_oil::compose::Composer>,
}

impl GlassRenderer {
    /// 创建渲染器实例。
    ///
    /// 传入已有的 `Device` / `Queue`、着色器加载器和配置，
    /// 内部创建管线、中间纹理、bind group、sampler 等资源。
    pub fn new(
        device: wgpu::Device,
        queue: wgpu::Queue,
        loader: impl ShaderLoader,
        config: RendererConfig,
        initial_size: (u32, u32),
    ) -> Self {
        // --- 加载着色器模块 ---
        let blur_h_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("blur_horizontal"),
            source: loader.load("blur_horizontal"),
        });
        let blur_v_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("blur_vertical"),
            source: loader.load("blur_vertical"),
        });
        let refract_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("refract"),
            source: loader.load("refract"),
        });
        let vs_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("fullscreen_triangle"),
            source: loader.load("fullscreen_triangle"),
        });
        let fs_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("composite"),
            source: loader.load("composite"),
        });

        // --- bind group layout ---
        let blur_bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("blur_bind_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let refract_bind_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("refract_bind_layout"),
                entries: &[
                    // @binding(0) 位移输出纹理（storage write）
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::WriteOnly,
                            format: wgpu::TextureFormat::Rgba16Float,
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    // @binding(1) 玻璃 uniform
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let composite_bind_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("composite_bind_layout"),
                entries: &[
                    // @binding(0) 背景纹理
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // @binding(1) 模糊纹理
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // @binding(2) 位移纹理
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // @binding(3) sampler
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    // @binding(4) 玻璃 uniform
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        // --- pipeline layout ---
        let blur_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("blur_pipeline_layout"),
            bind_group_layouts: &[Some(&blur_bind_layout)],
            immediate_size: 0,
        });

        let refract_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("refract_pipeline_layout"),
                bind_group_layouts: &[Some(&refract_bind_layout)],
                immediate_size: 0,
            });

        let composite_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("composite_pipeline_layout"),
                bind_group_layouts: &[Some(&composite_bind_layout)],
                immediate_size: 0,
            });

        // --- pipeline ---
        let blur_h_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("blur_horizontal"),
            layout: Some(&blur_pipeline_layout),
            module: &blur_h_module,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        let blur_v_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("blur_vertical"),
            layout: Some(&blur_pipeline_layout),
            module: &blur_v_module,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        let refract_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("refract"),
            layout: Some(&refract_pipeline_layout),
            module: &refract_module,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        let composite_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("composite"),
            layout: Some(&composite_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &vs_module,
                entry_point: Some("main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &fs_module,
                entry_point: Some("main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.texture_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        // --- sampler ---
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("linear_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });

        // --- uniform buffer ---
        let blur_uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("blur_uniforms"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let blur_uniforms = BlurUniforms {
            texture_size: [initial_size.0, initial_size.1],
            blur_radius: 0.0,
            kernel_half: 1,
        };

        let glass_uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("glass_uniforms"),
            size: std::mem::size_of::<GlassUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // --- 中间纹理 ---
        let (h_blur_tex, v_blur_tex, h_blur_view, v_blur_view) =
            Self::create_blur_textures(&device, initial_size);

        let (displacement_tex, displacement_view) =
            Self::create_displacement_texture(&device, initial_size);

        // --- V blur bind group ---
        let blur_v_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("blur_v_bind_group"),
            layout: &blur_bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&h_blur_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&v_blur_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: blur_uniform_buf.as_entire_binding(),
                },
            ],
        });

        // --- refract bind group ---
        let refract_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("refract_bind_group"),
            layout: &refract_bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&displacement_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: glass_uniform_buf.as_entire_binding(),
                },
            ],
        });

        // --- composite bind group ---
        // 需要 background_view，每帧重建
        // 这里用 v_blur_view 作为占位，render() 中会重建
        let composite_bind_group = Self::create_composite_bind_group(
            &device,
            &composite_bind_layout,
            &v_blur_view,
            &v_blur_view,
            &displacement_view,
            &sampler,
            &glass_uniform_buf,
        );

        // 热更新用的 composer
        let mut composer = naga_oil::compose::Composer::default();
        for (name, source) in &[
            ("sdf", include_str!("../../shaders/common/sdf.wgsl")),
            (
                "glass_material",
                include_str!("../../shaders/common/glass_material.wgsl"),
            ),
        ] {
            composer
                .add_composable_module(naga_oil::compose::ComposableModuleDescriptor {
                    source,
                    file_path: &format!("{name}.wgsl"),
                    ..Default::default()
                })
                .unwrap_or_else(|e| panic!("注册公共模块 {name} 失败: {e}"));
        }

        Self {
            device,
            queue,
            blur_h_pipeline,
            blur_v_pipeline,
            refract_pipeline,
            composite_pipeline,
            blur_bind_layout,
            refract_bind_layout,
            composite_bind_layout,
            sampler,
            blur_uniform_buf,
            blur_uniforms,
            glass_uniform_buf,
            h_blur_tex,
            v_blur_tex,
            h_blur_view,
            v_blur_view,
            displacement_tex,
            displacement_view,
            blur_v_bind_group,
            refract_bind_group,
            composite_bind_group,
            current_size: initial_size,
            config,
            blur_h_module,
            blur_v_module,
            refract_module,
            vs_module,
            fs_module,
            blur_pipeline_layout,
            refract_pipeline_layout,
            composite_pipeline_layout,
            composer: std::cell::RefCell::new(composer),
        }
    }

    /// 返回当前中间纹理的只读句柄。
    ///
    /// 在 `render()` 调用后纹理内容为本帧最新值。
    pub fn intermediate_textures(&self) -> IntermediateTextures<'_> {
        IntermediateTextures {
            displacement: &self.displacement_tex,
            h_blur: &self.h_blur_tex,
            v_blur: &self.v_blur_tex,
        }
    }

    /// 返回当前中间纹理尺寸。
    pub fn current_size(&self) -> (u32, u32) {
        self.current_size
    }

    /// 热更新着色器并重建对应管线。
    ///
    /// `source` 为含 `#import` 的 WGSL 源码，
    /// 内部用 naga_oil 解析公共模块依赖并重建管线。
    ///
    /// # 参数
    ///
    /// * `name` - 着色器名称（如 `"composite"`、`"refract"`）
    /// * `wgsl_source` - 新的 WGSL 源码
    ///
    /// # 错误
    ///
    /// naga 编译失败或管线重建失败时返回错误。
    pub fn reload_shader(&mut self, name: &str, wgsl_source: &str) -> Result<(), String> {
        let naga_module = self
            .composer
            .borrow_mut()
            .make_naga_module(naga_oil::compose::NagaModuleDescriptor {
                source: wgsl_source,
                file_path: &format!("{name}.wgsl"),
                ..Default::default()
            })
            .map_err(|e| format!("编译着色器 {name} 失败: {e}"))?;

        let source = wgpu::ShaderSource::Naga(std::borrow::Cow::Owned(naga_module));
        let new_module = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(name),
                source,
            });

        match name {
            "blur_horizontal" => {
                let pipeline =
                    self.device
                        .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                            label: Some("blur_horizontal"),
                            layout: Some(&self.blur_pipeline_layout),
                            module: &new_module,
                            entry_point: Some("main"),
                            compilation_options: wgpu::PipelineCompilationOptions::default(),
                            cache: None,
                        });
                self.blur_h_module = new_module;
                self.blur_h_pipeline = pipeline;
            }
            "blur_vertical" => {
                let pipeline =
                    self.device
                        .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                            label: Some("blur_vertical"),
                            layout: Some(&self.blur_pipeline_layout),
                            module: &new_module,
                            entry_point: Some("main"),
                            compilation_options: wgpu::PipelineCompilationOptions::default(),
                            cache: None,
                        });
                self.blur_v_module = new_module;
                self.blur_v_pipeline = pipeline;
            }
            "refract" => {
                let pipeline =
                    self.device
                        .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                            label: Some("refract"),
                            layout: Some(&self.refract_pipeline_layout),
                            module: &new_module,
                            entry_point: Some("main"),
                            compilation_options: wgpu::PipelineCompilationOptions::default(),
                            cache: None,
                        });
                self.refract_module = new_module;
                self.refract_pipeline = pipeline;
            }
            "composite" => {
                let pipeline =
                    self.device
                        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                            label: Some("composite"),
                            layout: Some(&self.composite_pipeline_layout),
                            vertex: wgpu::VertexState {
                                module: &self.vs_module,
                                entry_point: Some("main"),
                                compilation_options: wgpu::PipelineCompilationOptions::default(),
                                buffers: &[],
                            },
                            primitive: wgpu::PrimitiveState {
                                topology: wgpu::PrimitiveTopology::TriangleList,
                                ..Default::default()
                            },
                            depth_stencil: None,
                            multisample: wgpu::MultisampleState::default(),
                            fragment: Some(wgpu::FragmentState {
                                module: &new_module,
                                entry_point: Some("main"),
                                compilation_options: wgpu::PipelineCompilationOptions::default(),
                                targets: &[Some(wgpu::ColorTargetState {
                                    format: self.config.texture_format,
                                    blend: Some(wgpu::BlendState::REPLACE),
                                    write_mask: wgpu::ColorWrites::ALL,
                                })],
                            }),
                            multiview_mask: None,
                            cache: None,
                        });
                self.fs_module = new_module;
                self.composite_pipeline = pipeline;
            }
            "fullscreen_triangle" => {
                let pipeline =
                    self.device
                        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                            label: Some("composite"),
                            layout: Some(&self.composite_pipeline_layout),
                            vertex: wgpu::VertexState {
                                module: &new_module,
                                entry_point: Some("main"),
                                compilation_options: wgpu::PipelineCompilationOptions::default(),
                                buffers: &[],
                            },
                            primitive: wgpu::PrimitiveState {
                                topology: wgpu::PrimitiveTopology::TriangleList,
                                ..Default::default()
                            },
                            depth_stencil: None,
                            multisample: wgpu::MultisampleState::default(),
                            fragment: Some(wgpu::FragmentState {
                                module: &self.fs_module,
                                entry_point: Some("main"),
                                compilation_options: wgpu::PipelineCompilationOptions::default(),
                                targets: &[Some(wgpu::ColorTargetState {
                                    format: self.config.texture_format,
                                    blend: Some(wgpu::BlendState::REPLACE),
                                    write_mask: wgpu::ColorWrites::ALL,
                                })],
                            }),
                            multiview_mask: None,
                            cache: None,
                        });
                self.vs_module = new_module;
                self.composite_pipeline = pipeline;
            }
            _ => return Err(format!("未知着色器: {name}")),
        }

        Ok(())
    }

    /// 录制一帧的渲染命令到 `encoder` 中。
    ///
    /// 调用方负责在调用前开始 encoder、调用后 `finish()` 并提交。
    pub fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        input: &RenderInput,
        output: &wgpu::TextureView,
    ) {
        let size = input.size;

        // 尺寸变化时重建中间纹理和 bind group
        if size != self.current_size {
            self.rebuild_for_size(size);
        }

        // 每帧重建 composite bind group（background 可能变化）
        self.composite_bind_group = Self::create_composite_bind_group(
            &self.device,
            &self.composite_bind_layout,
            input.background,
            &self.v_blur_view,
            &self.displacement_view,
            &self.sampler,
            &self.glass_uniform_buf,
        );

        // 更新模糊 uniform
        let sigma = (input.material.blur_radius / 2.0).max(0.5);
        let kernel_half = (3.0 * sigma).ceil() as u32;
        self.blur_uniforms = BlurUniforms {
            texture_size: [size.0, size.1],
            blur_radius: input.material.blur_radius,
            kernel_half: kernel_half.max(1),
        };
        self.queue.write_buffer(
            &self.blur_uniform_buf,
            0,
            bytemuck::bytes_of(&self.blur_uniforms),
        );

        // 更新玻璃 uniform
        let glass_uniforms = GlassUniforms::from_input(input, &input.scene.panel);
        self.queue.write_buffer(
            &self.glass_uniform_buf,
            0,
            bytemuck::bytes_of(&glass_uniforms),
        );

        // 水平模糊 bind group（每帧创建，因为 input.background 可能变化）
        let blur_h_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("blur_h_bind_group"),
            layout: &self.blur_bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(input.background),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&self.h_blur_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.blur_uniform_buf.as_entire_binding(),
                },
            ],
        });

        // --- pass 1: 折射位移 ---
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("refract"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.refract_pipeline);
            pass.set_bind_group(0, &self.refract_bind_group, &[]);
            let wg_w = self.config.refract_workgroup_width;
            let wg_h = self.config.refract_workgroup_height;
            let dx = size.0.div_ceil(wg_w);
            let dy = size.1.div_ceil(wg_h);
            pass.dispatch_workgroups(dx, dy, 1);
        }

        // --- pass 2: 水平模糊 ---
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("blur_horizontal"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.blur_h_pipeline);
            pass.set_bind_group(0, &blur_h_bind_group, &[]);
            let wg_w = self.config.blur_workgroup_width;
            let dx = size.0.div_ceil(wg_w);
            pass.dispatch_workgroups(dx, size.1, 1);
        }

        // --- pass 3: 垂直模糊 ---
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("blur_vertical"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.blur_v_pipeline);
            pass.set_bind_group(0, &self.blur_v_bind_group, &[]);
            let wg_h = self.config.blur_workgroup_height;
            let dy = size.1.div_ceil(wg_h);
            pass.dispatch_workgroups(size.0, dy, 1);
        }

        // --- pass 4: 合成 ---
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("composite"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: output,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.composite_pipeline);
            pass.set_bind_group(0, &self.composite_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
    }

    /// 根据新尺寸重建中间纹理和相关的 bind group。
    fn rebuild_for_size(&mut self, size: (u32, u32)) {
        self.current_size = size;

        // 重建模糊纹理
        let (h_tex, v_tex, h_view, v_view) = Self::create_blur_textures(&self.device, size);
        self.h_blur_tex = h_tex;
        self.v_blur_tex = v_tex;
        self.h_blur_view = h_view;
        self.v_blur_view = v_view;

        // 重建位移纹理
        let (d_tex, d_view) = Self::create_displacement_texture(&self.device, size);
        self.displacement_tex = d_tex;
        self.displacement_view = d_view;

        // 重建 V blur bind group
        self.blur_v_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("blur_v_bind_group"),
            layout: &self.blur_bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.h_blur_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&self.v_blur_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.blur_uniform_buf.as_entire_binding(),
                },
            ],
        });

        // 重建折射 bind group
        self.refract_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("refract_bind_group"),
            layout: &self.refract_bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.displacement_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.glass_uniform_buf.as_entire_binding(),
                },
            ],
        });
    }

    /// 创建合成 pass 的 bind group。
    fn create_composite_bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        background_view: &wgpu::TextureView,
        blur_view: &wgpu::TextureView,
        displacement_view: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
        glass_uniform_buf: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("composite_bind_group"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(background_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(blur_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(displacement_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: glass_uniform_buf.as_entire_binding(),
                },
            ],
        })
    }

    /// 创建一对可用于 compute r/w 的模糊中间纹理。
    fn create_blur_textures(
        device: &wgpu::Device,
        size: (u32, u32),
    ) -> (
        wgpu::Texture,
        wgpu::Texture,
        wgpu::TextureView,
        wgpu::TextureView,
    ) {
        let desc = wgpu::TextureDescriptor {
            label: Some("blur_texture"),
            size: wgpu::Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[wgpu::TextureFormat::Rgba8Unorm],
        };

        let h_tex = device.create_texture(&desc);
        let v_tex = device.create_texture(&desc);
        let h_view = h_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let v_view = v_tex.create_view(&wgpu::TextureViewDescriptor::default());

        (h_tex, v_tex, h_view, v_view)
    }

    /// 创建位移纹理（折射偏移，Rgba16Float）。
    fn create_displacement_texture(
        device: &wgpu::Device,
        size: (u32, u32),
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("displacement_texture"),
            size: wgpu::Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
        (tex, view)
    }
}
