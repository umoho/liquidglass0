// 最终合成 fragment shader。
//
// 全屏三角形，逐像素合成所有光学效果：
//   1. SDF 判断玻璃区域 + 阴影
//   2. 采样 displacement → 折射偏移
//   3. RGB 分离采样 → 色散
//   4. 采样模糊纹理，按球形弧面厚度混合 → 磨砂感
//   5. 从球形弧面高度场推导 3D 法线
//   6. Schlick 菲涅尔 → 边缘发光
//   7. Blinn-Phong 多光源 → 镜面高光
//   8. 色调 + 动态范围调整

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
    let light_count = u.interaction.w;

    // 阴影 + 厚度参数
    let thickness_multiplier = u.shadow_params.x;
    let shadow_opacity = u.shadow_params.y;
    let shadow_blur = u.shadow_params.z;
    let shadow_offset_y = u.shadow_params.w;
    let effective_depth = bevel_depth * thickness_multiplier;

    // 计算 SDF 距离
    let dist = sdf::squircle_sdf(pixel, center, half_size, corner_radius, 5.0);

    // SDF 归一化距离 × min_effective ≈ 像素距离，用于阴影软边换算
    let min_effective = min(half_size.x, half_size.y) - corner_radius;
    let shadow_blur_norm = shadow_blur / max(min_effective, 1.0);

    // 玻璃区域外：计算阴影，输出背景 + 阴影
    if dist >= 0.0 {
        let bg = textureSample(background_tex, tex_sampler, uv);
        let shadow_center = center + vec2f(0.0, shadow_offset_y);
        let shadow_dist = sdf::squircle_sdf(pixel, shadow_center, half_size, corner_radius, 5.0);
        let shadow_alpha = (1.0 - smoothstep(-shadow_blur_norm, 0.0, shadow_dist)) * shadow_opacity;
        let shadow_color = vec3f(0.0);
        return mix(bg, vec4f(shadow_color, 1.0), shadow_alpha);
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

    // --- 3. 磨砂：按球形弧面厚度混合清晰/模糊 ---
    let sharp = textureSample(background_tex, tex_sampler, refracted_uv);
    let blurred = textureSample(blur_tex, tex_sampler, refracted_uv);
    let bevel_t = clamp(-dist / bevel_width_px, 0.0, 1.0);
    let thickness = sdf::bevel_z_lens(dist, bevel_width_px, effective_depth);
    let frost_mix = clamp(thickness / max(effective_depth, 1.0), 0.0, 1.0);
    let frosted = mix(sharp, blurred, frost_mix);

    // --- 4. 从球形弧面高度场推导 3D 法线 ---
    let slope_norm = sdf::bevel_slope_lens_norm(bevel_t);
    let slope = slope_norm * effective_depth / bevel_width_px;
    let sdf_n = sdf::sdf_normal(pixel, center, half_size, corner_radius, 5.0);
    let surf_grad = sdf_n * slope;
    let normal_3d = normalize(vec3f(-surf_grad, 1.0));

    // 合并色散和磨砂：色散用于折射区域，磨砂用于整体
    let base_color = mix(frosted.rgb, refracted_color, 0.5);

    // --- 5. 菲涅尔：基于 3D 法线 ---
    let cos_theta = normal_3d.z;
    let f0 = 0.04;
    let fresnel = f0 + (schlick_fresnel(cos_theta, f0) - f0) * fresnel_intensity;

    // --- 6. 镜面高光 ---
    var specular_total = vec3f(0.0);
    let view_dir = vec3f(0.0, 0.0, 1.0);

    if light_count > 0.0 {
        let light_dir = normalize(vec3f(u.light01_pos.xy - pixel, 100.0));
        let half_vec = normalize(view_dir + light_dir);
        let ndh = max(dot(normal_3d, half_vec), 0.0);
        specular_total += pow(ndh, specular_shininess) * u.light0_col.xyz * specular_intensity;
    }

    if light_count > 1.0 {
        let light_dir = normalize(vec3f(u.light01_pos.zw - pixel, 100.0));
        let half_vec = normalize(view_dir + light_dir);
        let ndh = max(dot(normal_3d, half_vec), 0.0);
        specular_total += pow(ndh, specular_shininess) * u.light1_col.xyz * specular_intensity;
    }

    if light_count > 2.0 {
        let light_dir = normalize(vec3f(u.light2_pos.xy - pixel, 100.0));
        let half_vec = normalize(view_dir + light_dir);
        let ndh = max(dot(normal_3d, half_vec), 0.0);
        specular_total += pow(ndh, specular_shininess) * u.light2_col.xyz * specular_intensity;
    }

    // --- 7. 合成 ---
    var color = base_color * bg_opacity;

    // 叠加菲涅尔边缘光
    color += fresnel_color * fresnel;

    // 高光降低强度防过曝
    color += specular_total * 0.3;

    // 叠加色调
    color = mix(color, tint_color, tint_opacity);

    // 动态范围调整
    color += brightness;
    color = adjust_saturation(color, saturation);
    color = adjust_contrast(color, contrast);

    return vec4f(clamp(color, vec3f(0.0), vec3f(1.0)), 1.0);
}
