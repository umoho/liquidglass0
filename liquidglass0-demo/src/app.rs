//! Demo 应用状态与事件循环。
//!
//! 持有 wgpu 资源、渲染器实例和背景纹理，
//! 实现 winit [`ApplicationHandler`] 驱动窗口与渲染。

use std::sync::Arc;

use liquidglass0_core::{GlassMaterial, GlassPanel, InteractionState, Light, Scene};
use liquidglass0_render::{GlassRenderer, NagaOilLoader, RenderInput, RendererConfig};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowAttributes};

/// Demo 应用。
///
/// 持有 wgpu 设备句柄、窗口、surface、渲染器实例和背景纹理，
/// 实现 [`ApplicationHandler`] 驱动窗口与每帧渲染。
pub struct App {
    /// wgpu 设备句柄（demo 自用，surface 配置与 command encoder 创建）。
    pub device: wgpu::Device,
    /// wgpu 命令队列句柄（demo 自用，submit 与纹理上传）。
    pub queue: wgpu::Queue,

    /// wgpu 实例。
    instance: wgpu::Instance,
    /// wgpu 适配器（用于获取 surface capabilities）。
    adapter: wgpu::Adapter,
    /// 渲染器配置（surface 格式与工作组大小）。
    config: RendererConfig,

    /// winit 窗口。
    window: Option<Arc<Window>>,
    /// wgpu surface。
    surface: Option<wgpu::Surface<'static>>,
    /// 玻璃渲染器。
    renderer: Option<GlassRenderer>,
    /// 背景纹理。
    background_tex: Option<wgpu::Texture>,
    /// 背景纹理视图。
    background_view: Option<wgpu::TextureView>,

    /// 当前窗口尺寸（像素）。
    size: (u32, u32),
}

impl App {
    /// 创建应用实例。
    ///
    /// 传入已创建好的 wgpu 实例、适配器、设备和队列。
    /// 窗口、surface、渲染器在首次 [`ApplicationHandler::resumed`] 回调中延迟创建。
    pub fn new(
        instance: wgpu::Instance,
        adapter: wgpu::Adapter,
        device: wgpu::Device,
        queue: wgpu::Queue,
    ) -> Self {
        Self {
            device,
            queue,
            instance,
            adapter,
            config: RendererConfig::default(),
            window: None,
            surface: None,
            renderer: None,
            background_tex: None,
            background_view: None,
            size: (1024, 768),
        }
    }

    /// 创建窗口、surface、渲染器及背景纹理。
    ///
    /// 在首次 [`ApplicationHandler::resumed`] 时调用。
    fn init_surface(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(
                    WindowAttributes::default()
                        .with_title("liquidglass0")
                        .with_inner_size(PhysicalSize::new(self.size.0, self.size.1)),
                )
                .unwrap(),
        );

        let surface = self.instance.create_surface(window.clone()).unwrap();

        let caps = surface.get_capabilities(&self.adapter);
        let format = caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(caps.formats[0]);

        self.config.texture_format = format;

        surface.configure(
            &self.device,
            &wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format,
                width: self.size.0,
                height: self.size.1,
                present_mode: caps.present_modes[0],
                alpha_mode: caps.alpha_modes[0],
                view_formats: vec![format],
                desired_maximum_frame_latency: 2,
            },
        );

        let renderer_device = self.device.clone();
        let renderer_queue = self.queue.clone();

        self.renderer = Some(GlassRenderer::new(
            renderer_device,
            renderer_queue,
            NagaOilLoader::default(),
            self.config.clone(),
            self.size,
        ));

        let (tex, view) = Self::generate_checkerboard(&self.device, &self.queue, self.size);
        self.background_tex = Some(tex);
        self.background_view = Some(view);

        self.window = Some(window);
        self.surface = Some(surface);
    }

    /// 处理窗口大小变化。
    ///
    /// 重新配置 surface 并重建背景纹理。
    /// 宽度或高度为 0（最小化）时不处理。
    ///
    /// # 参数
    ///
    /// * `size` - 新的窗口物理尺寸（像素）。
    fn resize(&mut self, size: PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }

        self.size = (size.width, size.height);

        if let Some(ref surface) = self.surface {
            let caps = surface.get_capabilities(&self.adapter);
            surface.configure(
                &self.device,
                &wgpu::SurfaceConfiguration {
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    format: self.config.texture_format,
                    width: size.width,
                    height: size.height,
                    present_mode: caps.present_modes[0],
                    alpha_mode: caps.alpha_modes[0],
                    view_formats: vec![self.config.texture_format],
                    desired_maximum_frame_latency: 2,
                },
            );
        }

        let (tex, view) = Self::generate_checkerboard(&self.device, &self.queue, self.size);
        self.background_tex = Some(tex);
        self.background_view = Some(view);
    }

    /// 录制并提交一帧渲染命令。
    ///
    /// 获取 surface 纹理、构造 [`RenderInput`]、调用渲染器录制、
    /// 提交到 GPU 并呈现到屏幕。
    fn render(&mut self) {
        let surface = self.surface.as_ref().unwrap();

        let surface_tex = match surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(tex)
            | wgpu::CurrentSurfaceTexture::Suboptimal(tex) => tex,
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => return,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.resize(PhysicalSize::new(self.size.0, self.size.1));
                return;
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                eprintln!("surface validation error");
                return;
            }
        };

        let output_view = surface_tex
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let (w, h) = (self.size.0 as f32, self.size.1 as f32);
        let input = RenderInput {
            background: self.background_view.as_ref().unwrap(),
            size: self.size,
            interaction: InteractionState::default(),
            time: 0.0,
            material: GlassMaterial {
                // 光学参数
                refractive_index: 1.3,
                chromatic_strength: 0.03,
                fresnel_intensity: 2.0,
                specular_intensity: 0.4,
                specular_shininess: 150.0,
                blur_radius: 20.0,
                // 材质参数
                tint_opacity: 0.08,
                background_opacity: 0.92,
                saturation: 1.4,
                brightness: 0.08,
                ..Default::default()
            },
            scene: Scene {
                panel: GlassPanel {
                    center: glam::Vec2::new(w / 2.0, h / 2.0),
                    half_size: glam::Vec2::new(200.0, 150.0),
                    corner_radius: 28.0,
                    bevel_width: 0.20,
                    bevel_depth: 55.0,
                },
                lights: [
                    Light {
                        position: glam::Vec2::new(w * 0.2, h * 0.15),
                        color: glam::Vec3::ONE,
                        intensity: 0.9,
                    },
                    Light {
                        position: glam::Vec2::new(w * 0.85, h * 0.25),
                        color: glam::Vec3::new(0.95, 0.97, 1.0),
                        intensity: 0.5,
                    },
                    Light {
                        position: glam::Vec2::new(w * 0.75, h * 0.8),
                        color: glam::Vec3::new(1.0, 0.98, 0.95),
                        intensity: 0.3,
                    },
                ],
            },
        };

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        self.renderer
            .as_mut()
            .unwrap()
            .render(&mut encoder, &input, &output_view);

        self.queue.submit([encoder.finish()]);
        surface_tex.present();
    }

    /// 生成程序化棋盘格背景纹理。
    ///
    /// 以 50px 为 tile 大小，暖橙与冷蓝交替，
    /// 上传到 wgpu 纹理并返回纹理及其默认视图。
    ///
    /// # 参数
    ///
    /// * `device` - wgpu 设备。
    /// * `queue` - wgpu 队列。
    /// * `size` - 纹理尺寸（像素）。
    ///
    /// # 返回值
    ///
    /// 背景纹理及其默认视图。
    fn generate_checkerboard(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        size: (u32, u32),
    ) -> (wgpu::Texture, wgpu::TextureView) {
        const TILE: u32 = 50;

        let width = size.0;
        let height = size.1;
        let mut pixels = vec![0u8; (width * height * 4) as usize];

        for y in 0..height {
            for x in 0..width {
                let idx = ((y * width + x) * 4) as usize;
                let tile = ((x / TILE) ^ (y / TILE)) & 1 != 0;
                if tile {
                    pixels[idx] = 180;
                    pixels[idx + 1] = 120;
                    pixels[idx + 2] = 80;
                } else {
                    pixels[idx] = 70;
                    pixels[idx + 1] = 130;
                    pixels[idx + 2] = 200;
                }
                pixels[idx + 3] = 255;
            }
        }

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("background"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[wgpu::TextureFormat::Rgba8Unorm],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
    }
}

impl ApplicationHandler for App {
    /// 应用恢复时创建窗口与 surface。
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            self.init_surface(event_loop);
        }
    }

    /// 窗口事件分发。
    ///
    /// 处理关闭、缩放、重绘请求。
    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => self.resize(size),
            WindowEvent::RedrawRequested => self.render(),
            _ => {}
        }
    }

    /// 事件循环即将空闲时请求下一帧重绘。
    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(ref window) = self.window {
            window.request_redraw();
        }
    }
}
