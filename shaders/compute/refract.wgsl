// 折射位移 compute shader。
//
// 基于 SDF 曲面法线计算每个像素的背景采样偏移量，
// 输出到 displacement_texture，供 composite shader 使用。
//
// 工作组 (16, 16)：二维分块，每个像素独立计算。

#import glass_material
#import sdf

@group(0) @binding(0) var displacement_out: texture_storage_2d<rgba16float, write>;
@group(0) @binding(1) var<uniform> u: glass_material::GlassUniforms;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let size = textureDimensions(displacement_out);
    if gid.x >= size.x || gid.y >= size.y {
        return;
    }

    // 像素坐标 → 面板中心坐标系（像素）
    let p = vec2f(f32(gid.x), f32(gid.y));

    // 从 uniform 解包参数
    let center = u.panel_info.xy;
    let half_size = u.panel_info.zw;
    let corner_radius = u.shape_params.x;
    let bevel_width_ratio = u.shape_params.y;
    let bevel_depth = u.shape_params.z;
    let refractive_index = u.shape_params.w;
    let thickness_multiplier = u.shadow_params.x;
    let displacement = u.interaction.z;
    let bevel_width_px = bevel_width_ratio * min(half_size.x, half_size.y);
    let effective_depth = bevel_depth * thickness_multiplier * (1.0 - displacement);

    // 计算 SDF 距离
    let dist = sdf::squircle_sdf(p, center, half_size, corner_radius, 5.0);

    // 玻璃区域外：无偏移
    if dist >= 0.0 {
        textureStore(displacement_out, gid.xy, vec4f(0.0, 0.0, 0.0, 1.0));
        return;
    }

    // 玻璃区域内：计算折射偏移（球形弧面轮廓）
    let normal = sdf::sdf_normal(p, center, half_size, corner_radius, 5.0);
    let t = clamp(-dist / bevel_width_px, 0.0, 1.0);
    let slope = sdf::bevel_slope_lens_norm(t);
    let slope_scaled = slope * effective_depth / bevel_width_px;

    // 基于曲率的折射：负号使位移向内（凸透镜聚光）
    let eta = 1.0 - 1.0 / refractive_index;
    let offset = -normal * slope_scaled * eta;

    textureStore(displacement_out, gid.xy, vec4f(offset, 0.0, 1.0));
}
