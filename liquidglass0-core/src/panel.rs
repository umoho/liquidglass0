use glam::Vec2;

/// 玻璃面板形状定义。
///
/// 包含面板的位置、尺寸和斜面几何参数。
/// 光学/材质参数见 [`super::material::GlassMaterial`]。
///
/// # 默认值
///
/// | 字段 | 默认值 | 说明 |
/// |---|---|---|
/// | `center` | `(0, 0)` | 面板中心 |
/// | `half_size` | `(200, 150)` | 半宽/半高 |
/// | `corner_radius` | `22` | 圆角半径（像素） |
/// | `bevel_width` | `0.15` | 斜面宽度（占半径比例） |
/// | `bevel_depth` | `40` | 斜面深度（像素） |
#[derive(Debug, Clone, PartialEq)]
pub struct GlassPanel {
    /// 面板中心（像素坐标）。
    pub center: Vec2,

    /// 面板半宽/半高（像素）。
    pub half_size: Vec2,

    /// 圆角半径（像素）。
    ///
    /// 值越大，圆角越圆润。
    pub corner_radius: f32,

    /// 斜面宽度（占半径的比例）。
    ///
    /// 值越大，斜面过渡越宽。
    pub bevel_width: f32,

    /// 斜面深度（像素）。
    ///
    /// 值越大，斜面越深，折射越强。
    pub bevel_depth: f32,
}

impl Default for GlassPanel {
    fn default() -> Self {
        Self {
            center: Vec2::ZERO,
            half_size: Vec2::new(200.0, 150.0),
            corner_radius: 22.0,
            bevel_width: 0.15,
            bevel_depth: 40.0,
        }
    }
}
