//! 每帧渲染输入。

use liquidglass0_core::{GlassMaterial, InteractionState, Scene};

/// 传给 `GlassRenderer::render()` 的单帧数据。
pub struct RenderInput<'a> {
    /// 背景纹理视图。
    pub background: &'a wgpu::TextureView,
    /// 输出纹理尺寸（像素）。
    pub size: (u32, u32),
    /// 当前交互状态。
    pub interaction: InteractionState,
    /// 帧时间（秒）。
    pub time: f32,
    /// 玻璃材质参数。
    pub material: GlassMaterial,
    /// 场景配置（面板形状 + 光源）。
    pub scene: Scene,
}
