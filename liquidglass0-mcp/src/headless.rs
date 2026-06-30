//! 离屏渲染上下文。
//!
//! 不依赖 winit 窗口，直接创建 wgpu 设备、GlassRenderer 和辅助纹理，
//! 组装一帧渲染并支持 capture 输出。

use std::path::Path;

use liquidglass0_core::{GlassMaterial, InteractionState, Scene};
use liquidglass0_render::{GlassRenderer, NagaOilLoader, RenderInput, RendererConfig};

use crate::capture;

/// 离屏渲染器。
pub struct HeadlessRenderer {
    /// wgpu 设备。
    device: wgpu::Device,
    /// wgpu 命令队列。
    queue: wgpu::Queue,
    /// 玻璃渲染器。
    glass: GlassRenderer,
    /// 棋盘格背景纹理。
    background_tex: wgpu::Texture,
    /// 背景纹理视图。
    background_view: wgpu::TextureView,
    /// 最终输出纹理（带 COPY_SRC，可读回）。
    output_tex: wgpu::Texture,
    /// 输出纹理视图。
    output_view: wgpu::TextureView,
    /// 当前纹理尺寸（像素）。
    size: (u32, u32),
    /// 输出纹理格式（与管线匹配）。
    output_format: wgpu::TextureFormat,
}

impl HeadlessRenderer {
    /// 创建离屏渲染器。
    ///
    /// 创建 wgpu 实例、headless adapter、设备、GlassRenderer 和辅助纹理。
    ///
    /// # Panics
    ///
    /// 无法获取 wgpu 适配器或设备时 panic。
    pub async fn new(width: u32, height: u32) -> Self {
        let instance = wgpu::Instance::default();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: None,
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
            })
            .await
            .expect("无法获取 wgpu adapter（headless 模式）");

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("无法创建 wgpu device");

        let config = RendererConfig::default();
        let out_format = config.texture_format;
        let glass = GlassRenderer::new(
            device.clone(),
            queue.clone(),
            NagaOilLoader::default(),
            config,
            (width, height),
        );

        let (bg_tex, bg_view) = Self::make_checkerboard(&device, &queue, width, height);
        let (out_tex, out_view) = Self::make_output_texture(&device, width, height, out_format);

        Self {
            device,
            queue,
            glass,
            background_tex: bg_tex,
            background_view: bg_view,
            output_tex: out_tex,
            output_view: out_view,
            size: (width, height),
            output_format: out_format,
        }
    }

    /// 改变渲染尺寸。
    ///
    /// 重建输出纹理和背景纹理。
    pub fn resize(&mut self, width: u32, height: u32) {
        self.size = (width, height);
        let (bg_tex, bg_view) = Self::make_checkerboard(&self.device, &self.queue, width, height);
        let (out_tex, out_view) =
            Self::make_output_texture(&self.device, width, height, self.output_format);
        self.background_tex = bg_tex;
        self.background_view = bg_view;
        self.output_tex = out_tex;
        self.output_view = out_view;
    }

    /// 渲染一帧。
    ///
    /// # 参数
    ///
    /// * `material` - 玻璃材质参数
    /// * `scene` - 场景配置（面板形状 + 光源）
    pub fn render(&mut self, material: &GlassMaterial, scene: &Scene) {
        let input = RenderInput {
            background: &self.background_view,
            size: self.size,
            interaction: InteractionState::default(),
            time: 0.0,
            material: material.clone(),
            scene: scene.clone(),
        };

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        self.glass.render(&mut encoder, &input, &self.output_view);
        self.queue.submit([encoder.finish()]);
    }

    /// 截取一帧到 PNG 文件或字节。
    ///
    /// # 参数
    ///
    /// * `material` - 玻璃材质
    /// * `scene` - 场景配置
    /// * `kind` - 目标纹理：`"composite"`、`"displacement"`、`"h_blur"`、`"v_blur"`
    /// * `path` - 可选输出文件路径，不提供时返回 base64
    ///
    /// # 返回值
    ///
    /// `(path_or_b64, mime_type)`
    pub fn capture(
        &mut self,
        material: &GlassMaterial,
        scene: &Scene,
        kind: &str,
        path: Option<&str>,
    ) -> Result<(String, String), String> {
        self.render(material, scene);

        let png = match kind {
            "composite" => {
                capture::dump_rgba8(&self.device, &self.queue, &self.output_tex, self.size)
            }
            "displacement" => {
                let it = self.glass.intermediate_textures();
                capture::dump_displacement(&self.device, &self.queue, it.displacement, self.size)
            }
            "h_blur" => {
                let it = self.glass.intermediate_textures();
                capture::dump_rgba8(&self.device, &self.queue, it.h_blur, self.size)
            }
            "v_blur" => {
                let it = self.glass.intermediate_textures();
                capture::dump_rgba8(&self.device, &self.queue, it.v_blur, self.size)
            }
            _ => return Err(format!("未知的 capture kind: {kind}")),
        };

        if let Some(p) = path {
            std::fs::write(p, &png).map_err(|e| format!("写入文件失败: {e}"))?;
            let abs = Path::new(p)
                .canonicalize()
                .unwrap_or_else(|_| Path::new(p).to_path_buf());
            Ok((abs.display().to_string(), "image/png".into()))
        } else {
            let b64 = base64_encode(&png);
            Ok((b64, "image/png;base64".into()))
        }
    }

    /// 热更新着色器。
    pub fn reload_shader(&mut self, name: &str, source: &str) -> Result<(), String> {
        self.glass.reload_shader(name, source)
    }

    /// 当前渲染尺寸。
    pub fn size(&self) -> (u32, u32) {
        self.size
    }

    // ── 内部方法 ──

    /// 创建棋盘格背景纹理。
    fn make_checkerboard(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        const TILE: u32 = 50;
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

    /// 创建输出纹理（Rgba8Unorm，带 COPY_SRC 可读回）。
    fn make_output_texture(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("output"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[format],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
    }
}

/// 简易 base64 编码（不引入额外依赖）。
fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0];
        let b1 = chunk.get(1).copied().unwrap_or(0);
        let b2 = chunk.get(2).copied().unwrap_or(0);
        out.push(CHARS[(b0 >> 2) as usize] as char);
        out.push(CHARS[((b0 & 0x03) << 4 | (b1 >> 4)) as usize] as char);
        if chunk.len() > 1 {
            out.push(CHARS[((b1 & 0x0f) << 2 | (b2 >> 6)) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(CHARS[(b2 & 0x3f) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}
