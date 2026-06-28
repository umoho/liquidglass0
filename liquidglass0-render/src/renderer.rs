//! 玻璃渲染器。
//!
//! [`GlassRenderer`] 管理所有 wgpu 资源（管线、纹理、uniform buffer 等），
//! 对窗口无感知。`-demo` 负责创建 winit 窗口和 surface，
//! 将 `Device` / `Queue` 传入，每帧调用 `render()` 录制命令。

use crate::config::RendererConfig;
use crate::input::RenderInput;
use crate::shader::ShaderLoader;

/// 玻璃渲染器。
///
/// 持有 `Device`、`Queue` 和所有管线/纹理资源，生命周期内保持不变。
pub struct GlassRenderer {
    /// wgpu 设备。
    pub device: wgpu::Device,
    /// wgpu 命令队列。
    pub queue: wgpu::Queue,
}

impl GlassRenderer {
    /// 创建渲染器实例。
    ///
    /// 传入已有的 `Device` / `Queue`、着色器加载器和配置，
    /// 内部创建管线、bind group layout、sampler 等资源。
    ///
    /// # 参数
    ///
    /// * `device` - wgpu 设备（由调用方创建）。
    /// * `queue` - wgpu 命令队列。
    /// * `loader` - 着色器加载器实现。
    /// * `config` - 纹理格式、工作组大小等配置。
    pub fn new(
        device: wgpu::Device,
        queue: wgpu::Queue,
        loader: impl ShaderLoader,
        config: RendererConfig,
    ) -> Self {
        // Phase 1 当前任务：只存储 device / queue，管线创建留给后续任务
        let _ = loader;
        let _ = config;

        Self { device, queue }
    }

    /// 录制一帧的渲染命令到 `encoder` 中。
    ///
    /// 调用方负责在调用前 `encoder.begin`、调用后 `encoder.finish()` 并提交。
    ///
    /// # 参数
    ///
    /// * `encoder` - 已开始的命令编码器。
    /// * `input` - 当前帧的渲染输入。
    /// * `output` - 输出纹理视图（通常为 swap chain 帧）。
    pub fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        input: &RenderInput,
        output: &wgpu::TextureView,
    ) {
        let _ = encoder;
        let _ = input;
        let _ = output;
    }
}
