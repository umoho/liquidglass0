// 玻璃材质 uniform 定义。
//
// 所有字段使用 vec4f 对齐，确保 WGSL uniform buffer 布局一致。
// 通过 naga_oil `#import glass_material` 引入。

#define_import_path glass_material

/// 玻璃面板 + 材质 + 光源的统一 uniform。
///
/// 布局：13 × vec4f = 208 bytes。
struct GlassUniforms {
    /// 面板几何：center.xy, half_size.xy。
    panel_info: vec4f,
    /// 形状 + 折射率：corner_radius, bevel_width, bevel_depth, refractive_index。
    shape_params: vec4f,
    /// 光学参数 A：chromatic_strength, fresnel_intensity, specular_intensity, specular_shininess。
    optical_a: vec4f,
    /// 菲涅尔颜色：r, g, b, _pad。
    fresnel_col: vec4f,
    /// 色调：r, g, b, tint_opacity。
    tint_col: vec4f,
    /// 材质：background_opacity, saturation, contrast, brightness。
    material: vec4f,
    /// 交互：cursor_x, cursor_y, displacement（弹簧变形）, light_count。
    interaction: vec4f,
    /// 光源 0/1 位置：l0.x, l0.y, l1.x, l1.y。
    light01_pos: vec4f,
    /// 光源 2 位置：l2.x, l2.y, _pad, _pad。
    light2_pos: vec4f,
    /// 光源 0 颜色：r, g, b, _pad。
    light0_col: vec4f,
    /// 光源 1 颜色：r, g, b, _pad。
    light1_col: vec4f,
    /// 光源 2 颜色：r, g, b, _pad。
    light2_col: vec4f,
    /// 阴影 + 厚度：thickness_multiplier, shadow_opacity, shadow_blur, shadow_offset_y。
    shadow_params: vec4f,
}
