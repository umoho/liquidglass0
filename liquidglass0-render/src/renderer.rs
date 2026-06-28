//! 玻璃渲染器。
//!
//! [`GlassRenderer`] 管理所有 wgpu 资源（管线、纹理、uniform buffer 等），
//! 对窗口无感知。`-demo` 负责创建 winit 窗口和 surface，
//! 将 `Device` / `Queue` 传入，每帧调用 `render()` 录制命令。

use crate::config::RendererConfig;
use crate::input::RenderInput;
use crate::shader::ShaderLoader;

/// 传给 compute shader 的模糊参数。
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
    /// 合成 render pipeline（全屏三角形 + fragment shader）。
    composite_pipeline: wgpu::RenderPipeline,

    /// 模糊 pass 共用的 bind group layout。
    blur_bind_layout: wgpu::BindGroupLayout,
    /// 合成 pass 的 bind group layout。
    composite_bind_layout: wgpu::BindGroupLayout,

    /// 线性采样器（clamp to edge）。
    sampler: wgpu::Sampler,

    /// 模糊 uniform buffer（16 bytes）。
    blur_uniform_buf: wgpu::Buffer,
    /// CPU 侧 uniform 镜像。
    blur_uniforms: BlurUniforms,

    /// 水平模糊中间纹理。
    h_blur_tex: wgpu::Texture,
    /// 垂直模糊中间纹理。
    v_blur_tex: wgpu::Texture,
    /// 水平模糊结果视图。
    h_blur_view: wgpu::TextureView,
    /// 垂直模糊结果视图。
    v_blur_view: wgpu::TextureView,

    /// 垂直模糊 bind group（输入=h_blur_view，输出=v_blur_view）。
    blur_v_bind_group: wgpu::BindGroup,
    /// 合成 bind group（v_blur_view + sampler）。
    composite_bind_group: wgpu::BindGroup,

    /// 当前中间纹理尺寸。
    current_size: (u32, u32),
    /// 渲染器配置备份。
    _config: RendererConfig,
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
            source: wgpu::ShaderSource::Wgsl(loader.load_wgsl("blur_horizontal").into()),
        });
        let blur_v_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("blur_vertical"),
            source: wgpu::ShaderSource::Wgsl(loader.load_wgsl("blur_vertical").into()),
        });
        let vs_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("fullscreen_triangle"),
            source: wgpu::ShaderSource::Wgsl(loader.load_wgsl("fullscreen_triangle").into()),
        });
        let fs_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("composite"),
            source: wgpu::ShaderSource::Wgsl(loader.load_wgsl("composite").into()),
        });

        // --- bind group layout ---
        let blur_bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("blur_bind_layout"),
            entries: &[
                // @binding(0) 输入纹理（只读）
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
                // @binding(1) 输出纹理（storage write）
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
                // @binding(2) uniform
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

        let composite_bind_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("composite_bind_layout"),
                entries: &[
                    // @binding(0) 模糊结果纹理
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
                    // @binding(1) sampler
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
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

        // --- 中间纹理 ---
        let (h_blur_tex, v_blur_tex, h_blur_view, v_blur_view) =
            Self::create_blur_textures(&device, initial_size);

        // --- V blur bind group（输入 h_blur_view，输出 v_blur_view） ---
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

        // --- composite bind group ---
        let composite_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("composite_bind_group"),
            layout: &composite_bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&v_blur_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        Self {
            device,
            queue,
            blur_h_pipeline,
            blur_v_pipeline,
            composite_pipeline,
            blur_bind_layout,
            composite_bind_layout,
            sampler,
            blur_uniform_buf,
            blur_uniforms,
            h_blur_tex,
            v_blur_tex,
            h_blur_view,
            v_blur_view,
            blur_v_bind_group,
            composite_bind_group,
            current_size: initial_size,
            _config: config,
        }
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

        // 更新 uniform
        let sigma = (input.params.blur_radius / 2.0).max(0.5);
        let kernel_half = (3.0 * sigma).ceil() as u32;
        self.blur_uniforms = BlurUniforms {
            texture_size: [size.0, size.1],
            blur_radius: input.params.blur_radius,
            kernel_half: kernel_half.max(1),
        };
        self.queue.write_buffer(
            &self.blur_uniform_buf,
            0,
            bytemuck::bytes_of(&self.blur_uniforms),
        );

        // --- pass 1: 水平模糊 ---
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("blur_horizontal"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.blur_h_pipeline);
            pass.set_bind_group(0, &blur_h_bind_group, &[]);
            let wg_w = self._config.blur_workgroup_width;
            let dx = size.0.div_ceil(wg_w);
            pass.dispatch_workgroups(dx, size.1, 1);
        }

        // --- pass 2: 垂直模糊 ---
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("blur_vertical"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.blur_v_pipeline);
            pass.set_bind_group(0, &self.blur_v_bind_group, &[]);
            let wg_h = self._config.blur_workgroup_height;
            let dy = size.1.div_ceil(wg_h);
            pass.dispatch_workgroups(size.0, dy, 1);
        }

        // --- pass 3: 合成 ---
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

        let (h_tex, v_tex, h_view, v_view) = Self::create_blur_textures(&self.device, size);

        self.h_blur_tex = h_tex;
        self.v_blur_tex = v_tex;
        self.h_blur_view = h_view;
        self.v_blur_view = v_view;

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

        // 重建 composite bind group
        self.composite_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("composite_bind_group"),
            layout: &self.composite_bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.v_blur_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
    }

    /// 创建一对可用于 compute r/w 的中间纹理。
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
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::STORAGE_BINDING,
            view_formats: &[wgpu::TextureFormat::Rgba8Unorm],
        };

        let h_tex = device.create_texture(&desc);
        let v_tex = device.create_texture(&desc);
        let h_view = h_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let v_view = v_tex.create_view(&wgpu::TextureViewDescriptor::default());

        (h_tex, v_tex, h_view, v_view)
    }
}
