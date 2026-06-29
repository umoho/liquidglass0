use super::light::Light;
use super::panel::GlassPanel;

/// 场景配置。
///
/// 包含玻璃面板的形状定义和场景中的光源列表，
/// 属于场景级别的配置，与材质参数（[`super::material::GlassMaterial`]）分离。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Scene {
    /// 玻璃面板形状定义。
    pub panel: GlassPanel,

    /// 场景中的光源（最多 3 个）。
    pub lights: [Light; 3],
}
