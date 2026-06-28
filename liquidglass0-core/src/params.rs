use glam::Vec3;

/// 玻璃面板材质参数。
///
/// 包含形状、光学、材质、交互、阴影五个分组的可调参数。
/// 默认值来自 [`LIQUID_GLASS.md`] 参数表。
#[derive(Debug, Clone, PartialEq)]
pub struct GlassParams {
    // ── 形状 ──
    /// 圆角半径（像素）。
    pub corner_radius: f32,

    /// 斜面宽度（占半径的比例）。
    pub bevel_width: f32,

    /// 斜面深度（像素）。
    pub bevel_depth: f32,

    // ── 光学 ──
    /// 折射率，范围 1.3 ~ 1.7。
    pub refractive_index: f32,

    /// 色散强度。
    pub chromatic_strength: f32,

    /// 菲涅尔反射强度。
    pub fresnel_intensity: f32,

    /// 菲涅尔颜色。
    pub fresnel_color: Vec3,

    /// 镜面高光强度。
    pub specular_intensity: f32,

    /// 镜面高光锐度。
    pub specular_shininess: f32,

    /// 模糊半径（像素）。
    pub blur_radius: f32,

    // ── 材质 ──
    /// 玻璃底色。
    pub tint_color: Vec3,

    /// 色调叠加强度，范围 0 ~ 1。
    pub tint_opacity: f32,

    /// 背景透过率，范围 0 ~ 1。
    pub background_opacity: f32,

    /// 饱和度，范围 0 ~ 2。
    pub saturation: f32,

    /// 对比度，范围 0 ~ 2。
    pub contrast: f32,

    /// 亮度偏移，范围 -1 ~ 1。
    pub brightness: f32,

    // ── 交互 ──
    /// 弹簧刚度。
    pub deformation_spring_k: f32,

    /// 弹簧阻尼系数。
    pub deformation_damping_b: f32,

    // ── 阴影 ──
    /// 阴影透明度，范围 0 ~ 1。
    pub shadow_opacity: f32,

    /// 阴影模糊半径（像素）。
    pub shadow_blur: f32,

    /// 阴影 Y 偏移（像素）。
    pub shadow_offset_y: f32,
}

impl Default for GlassParams {
    fn default() -> Self {
        Self {
            corner_radius: 22.0,
            bevel_width: 0.15,
            bevel_depth: 40.0,

            refractive_index: 1.5,
            chromatic_strength: 0.02,
            fresnel_intensity: 1.0,
            fresnel_color: Vec3::new(0.9, 0.95, 1.0),
            specular_intensity: 0.5,
            specular_shininess: 80.0,
            blur_radius: 2.0,

            tint_color: Vec3::ONE,
            tint_opacity: 0.15,
            background_opacity: 0.85,
            saturation: 1.8,
            contrast: 1.04,
            brightness: 0.06,

            deformation_spring_k: 300.0,
            deformation_damping_b: 20.0,

            shadow_opacity: 0.3,
            shadow_blur: 8.0,
            shadow_offset_y: 4.0,
        }
    }
}
