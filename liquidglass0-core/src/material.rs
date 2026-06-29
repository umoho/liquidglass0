use glam::Vec3;

/// 玻璃面板材质参数。
///
/// 包含光学、材质、交互、阴影四个分组的可调参数。
/// 面板形状参数见 [`super::panel::GlassPanel`]。
///
/// 默认值来自 [`LIQUID_GLASS.md`] 参数表。
#[derive(Debug, Clone, PartialEq)]
pub struct GlassMaterial {
    // ── 光学 ──
    /// 折射率，范围 1.3 ~ 1.7。
    ///
    /// 值越大，背景偏移越强。
    pub refractive_index: f32,

    /// 色散强度。
    ///
    /// 值越大，RGB 分离越明显。
    pub chromatic_strength: f32,

    /// 菲涅尔反射强度。
    ///
    /// 值越大，边缘光晕越亮。
    pub fresnel_intensity: f32,

    /// 菲涅尔颜色。
    pub fresnel_color: Vec3,

    /// 镜面高光强度。
    ///
    /// 值越大，高光越亮。
    pub specular_intensity: f32,

    /// 镜面高光锐度。
    ///
    /// 值越大，高光越锐利集中。
    pub specular_shininess: f32,

    /// 模糊半径（像素）。
    ///
    /// 值越大，磨砂模糊越强。
    pub blur_radius: f32,

    // ── 材质 ──
    /// 玻璃底色。
    pub tint_color: Vec3,

    /// 色调叠加强度，范围 0 ~ 1。
    ///
    /// 值越大，玻璃底色越明显。
    pub tint_opacity: f32,

    /// 背景透过率，范围 0 ~ 1。
    ///
    /// 值越接近 1，背景越清晰。
    pub background_opacity: f32,

    /// 饱和度，范围 0 ~ 2。
    ///
    /// 值越大，色彩越鲜艳。
    pub saturation: f32,

    /// 对比度，范围 0 ~ 2。
    ///
    /// 值越大，明暗对比越强。
    pub contrast: f32,

    /// 亮度偏移，范围 -1 ~ 1。
    ///
    /// 正值提亮整体画面，负值压暗。
    pub brightness: f32,

    // ── 交互 ──
    /// 弹簧刚度。
    ///
    /// 值越大，回弹越快、越硬。
    pub deformation_spring_k: f32,

    /// 弹簧阻尼系数。
    ///
    /// 值越大，振荡衰减越快。
    pub deformation_damping_b: f32,

    // ── 阴影 ──
    /// 阴影透明度，范围 0 ~ 1。
    ///
    /// 值越大，阴影越深。
    pub shadow_opacity: f32,

    /// 阴影模糊半径（像素）。
    ///
    /// 值越大，阴影越模糊。
    pub shadow_blur: f32,

    /// 阴影 Y 偏移（像素）。
    ///
    /// 值越大，阴影下移越多。
    pub shadow_offset_y: f32,
}

impl Default for GlassMaterial {
    fn default() -> Self {
        Self {
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
