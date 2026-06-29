// 最终合成 fragment shader。
//
// 全屏三角形，逐像素合成所有光学效果：
//   1. SDF 判断玻璃区域
//   2. 采样 displacement → 折射偏移
//   3. RGB 分离采样 → 色散
//   4. 采样模糊纹理，按厚度混合 → 磨砂感
//   5. Schlick 菲涅尔 → 边缘发光
//   6. Blinn-Phong 多光源 → 镜面高光
//   7. 色调 + 动态范围调整

#import glass_material
#import sdf

@group(0) @binding(0) var background_tex: texture_2d<f32>;
@group(0) @binding(1) var blur_tex: texture_2d<f32>;
@group(0) @binding(2) var displacement_tex: texture_2d<f32>;
@group(0) @binding(3) var tex_sampler: sampler;
@group(0) @binding(4) var<uniform> u: glass_material::GlassUniforms;

/// Schlick 菲涅尔近似。
fn schlick_fresnel(cos_theta: f32, f0: f32) -> f32 {
    return f0 + (1.0 - f0) * pow(1.0 - cos_theta, 5.0);
}

/// 调整饱和度。
fn adjust_saturation(color: vec3f, factor: f32) -> vec3f {
    let luma = dot(color, vec3f(0.2126, 0.7152, 0.0722));
    return mix(vec3f(luma), color, factor);
}

/// 调整对比度。
fn adjust_contrast(color: vec3f, factor: f32) -> vec3f {
    return (color - 0.5) * factor + 0.5;
}

@fragment
fn main(@location(0) uv: vec2f) -> @location(0) vec4f {
    let tex_size = vec2f(textureDimensions(background_tex));
    let pixel = uv * tex_size;

    // 从 uniform 解包参数
    let center = u.panel_info.xy;
    let half_size = u.panel_info.zw;
    let corner_radius = u.shape_params.x;
    let bevel_width_ratio = u.shape_params.y;
    let bevel_depth = u.shape_params.z;
    let bevel_width_px = bevel_width_ratio * min(half_size.x, half_size.y);
    let chromatic_strength = u.optical_a.x;
    let fresnel_intensity = u.optical_a.y;
    let specular_intensity = u.optical_a.z;
    let specular_shininess = u.optical_a.w;
    let fresnel_color = u.fresnel_col.xyz;
    let tint_color = u.tint_col.xyz;
    let tint_opacity = u.tint_col.w;
    let bg_opacity = u.material.x;
    let saturation = u.material.y;
    let contrast = u.material.z;
    let brightness = u.material.w;
    let cursor_pos = u.interaction.xy;
    let time = u.interaction.z;
    let light_count = u.interaction.w;

    // 计算 SDF 距离
    let dist = sdf::squircle_sdf(pixel, center, half_size, corner_radius, 5.0);

    // 玻璃区域外：直接输出背景
    if dist >= 0.0 {
        return textureSample(background_tex, tex_sampler, uv);
    }

    // 玻璃区域内：合成所有效果

    // --- 1. 折射偏移 ---
    let disp = textureSample(displacement_tex, tex_sampler, uv).xy;
    let refracted_uv = uv + disp / tex_size;

    // --- 2. 色散：RGB 分离采样 ---
    let chromatic_offset = disp * chromatic_strength / tex_size;
    let r = textureSample(background_tex, tex_sampler, refracted_uv - chromatic_offset * 0.98).r;
    let g = textureSample(background_tex, tex_sampler, refracted_uv).g;
    let b = textureSample(background_tex, tex_sampler, refracted_uv + chromatic_offset * 1.02).b;
    let refracted_color = vec3f(r, g, b);

    // --- 3. 磨砂：按厚度混合清晰/模糊 ---
    let sharp = textureSample(background_tex, tex_sampler, refracted_uv);
    let blurred = textureSample(blur_tex, tex_sampler, refracted_uv);
    let thickness = sdf::bevel_z(dist, bevel_width_px, bevel_depth);
    let frost_mix = clamp(thickness / bevel_depth, 0.0, 1.0);
    let frosted = mix(sharp, blurred, frost_mix);

    // 合并色散和磨砂：色散用于折射区域，磨砂用于整体
    let base_color = mix(frosted.rgb, refracted_color, 0.5);

    // --- 4. 菲涅尔：基于 bevel 斜率推导掠射角 ---
    let t = (clamp(dist, -bevel_width_px, 0.0) / bevel_width_px) + 1.0;
    let dz_dt = 6.0 * t * (1.0 - t);
    let slope = dz_dt * bevel_depth / bevel_width_px;
    let view_dot = 1.0 / sqrt(1.0 + slope * slope);
    let fresnel = schlick_fresnel(view_dot, 0.04) * fresnel_intensity;

    // --- 5. 镜面高光（仅在 bevel 区域显示） ---
    let normal = sdf::sdf_normal(pixel, center, half_size, corner_radius, 5.0);
    let bevel_mask = clamp(thickness / max(bevel_depth * 0.5, 0.001), 0.0, 1.0);
    var specular_total = vec3f(0.0);
    let view_dir = vec3f(0.0, 0.0, 1.0);

    // 光源 0
    if light_count > 0.0 {
        let light_dir = normalize(vec3f(u.light01_pos.xy - pixel, 100.0));
        let half_vec = normalize(view_dir + light_dir);
        let ndh = max(dot(vec3f(normal, 0.0), half_vec), 0.0);
        specular_total += pow(ndh, specular_shininess) * u.light0_col.xyz * specular_intensity;
    }

    // 光源 1
    if light_count > 1.0 {
        let light_dir = normalize(vec3f(u.light01_pos.zw - pixel, 100.0));
        let half_vec = normalize(view_dir + light_dir);
        let ndh = max(dot(vec3f(normal, 0.0), half_vec), 0.0);
        specular_total += pow(ndh, specular_shininess) * u.light1_col.xyz * specular_intensity;
    }

    // 光源 2
    if light_count > 2.0 {
        let light_dir = normalize(vec3f(u.light2_pos.xy - pixel, 100.0));
        let half_vec = normalize(view_dir + light_dir);
        let ndh = max(dot(vec3f(normal, 0.0), half_vec), 0.0);
        specular_total += pow(ndh, specular_shininess) * u.light2_col.xyz * specular_intensity;
    }

    // --- 6. 合成 ---
    var color = base_color * bg_opacity;

    // 叠加菲涅尔边缘光
    color += fresnel_color * fresnel;

    // 高光限制在 bevel 区域，降低强度防过曝
    color += specular_total * bevel_mask * 0.7;

    // 叠加色调
    color = mix(color, tint_color, tint_opacity);

    // 动态范围调整
    color += brightness;
    color = adjust_saturation(color, saturation);
    color = adjust_contrast(color, contrast);

    return vec4f(clamp(color, vec3f(0.0), vec3f(1.0)), 1.0);
}
