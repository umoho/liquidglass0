//! 渲染器配置。
//!
//! 集中管理纹理格式、工作组大小、blur 半径上限等常量，
//! 一处调整全局生效。

/// 渲染器配置。
pub struct RendererConfig {
    /// 输出与中间纹理的格式（默认：`Rgba8UnormSrgb`）。
    pub texture_format: wgpu::TextureFormat,
    /// 模糊水平 pass 的工作组宽度（默认：256）。
    pub blur_workgroup_width: u32,
    /// 模糊垂直 pass 的工作组高度（默认：256）。
    pub blur_workgroup_height: u32,
}

impl Default for RendererConfig {
    fn default() -> Self {
        Self {
            texture_format: wgpu::TextureFormat::Rgba8UnormSrgb,
            blur_workgroup_width: 256,
            blur_workgroup_height: 256,
        }
    }
}
