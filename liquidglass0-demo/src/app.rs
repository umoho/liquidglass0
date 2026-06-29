//! Demo 应用状态与事件循环。
//!
//! 持有 wgpu 资源、渲染器实例和背景纹理，
//! 实现 winit [`ApplicationHandler`] 驱动窗口与渲染。

use std::sync::Arc;

use liquidglass0_core::{GlassParams, InteractionState};
use liquidglass0_render::{EmbeddedLoader, GlassRenderer, RenderInput, RendererConfig};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowAttributes};

/// Demo 应用。
pub struct App {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,

    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    config: RendererConfig,

    window: Option<Arc<Window>>,
    surface: Option<wgpu::Surface<'static>>,
    renderer: Option<GlassRenderer>,
    background_tex: Option<wgpu::Texture>,
    background_view: Option<wgpu::TextureView>,

    size: (u32, u32),
}

impl App {
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
            EmbeddedLoader,
            self.config.clone(),
            self.size,
        ));

        let (tex, view) = Self::generate_checkerboard(&self.device, &self.queue, self.size);
        self.background_tex = Some(tex);
        self.background_view = Some(view);

        self.window = Some(window);
        self.surface = Some(surface);
    }

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

        let input = RenderInput {
            background: self.background_view.as_ref().unwrap(),
            size: self.size,
            interaction: InteractionState::default(),
            time: 0.0,
            params: GlassParams::default(),
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
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            self.init_surface(event_loop);
        }
    }

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

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(ref window) = self.window {
            window.request_redraw();
        }
    }
}
