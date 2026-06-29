// 超椭圆 SDF 工具函数。
//
// 从 Rust `liquidglass0-core/src/sdf.rs` 移植，
// 通过 naga_oil `#import sdf` 引入。

#define_import_path sdf

/// 有限差分步长。
const EPSILON: f32 = 1e-3;

/// 超椭圆 signed distance function。
///
/// 返回负值表示在形状内部，正值在外部。
fn squircle_sdf(p: vec2f, center: vec2f, half_size: vec2f, corner_radius: f32, n: f32) -> f32 {
    let d = p - center;
    let effective = half_size - vec2f(corner_radius);

    if effective.x <= 0.0 || effective.y <= 0.0 {
        return length(vec2f(0.0));
    }

    let nx = abs(d.x / effective.x);
    let ny = abs(d.y / effective.y);

    let r = pow(pow(nx, n) + pow(ny, n), 1.0 / n);
    return r - 1.0;
}

/// 由 SDF 梯度求表面法线。
///
/// 对 `squircle_sdf` 做有限差分，取梯度的负方向（法线指向外侧），
/// 归一化后返回。
fn sdf_normal(p: vec2f, center: vec2f, half_size: vec2f, corner_radius: f32, n: f32) -> vec2f {
    let dx = vec2f(EPSILON, 0.0);
    let dy = vec2f(0.0, EPSILON);

    let gx = squircle_sdf(p + dx, center, half_size, corner_radius, n)
           - squircle_sdf(p - dx, center, half_size, corner_radius, n);
    let gy = squircle_sdf(p + dy, center, half_size, corner_radius, n)
           - squircle_sdf(p - dy, center, half_size, corner_radius, n);

    let grad = vec2f(gx, gy);
    let len = length(grad);
    if len > 0.0 {
        return grad / len;
    }
    return vec2f(0.0);
}

/// 斜面轮廓：将 SDF 距离映射为 Z 位移。
///
/// 边缘最深（`bevel_depth`），中心为 0。
fn bevel_z(distance: f32, bevel_width: f32, bevel_depth: f32) -> f32 {
    let t = (clamp(distance, -bevel_width, 0.0) / bevel_width) + 1.0;
    let st = t * t * (3.0 - 2.0 * t);
    return st * bevel_depth;
}

/// 球形弧面轮廓（凸透镜截面）。
///
/// 使用圆形弧线替代平滑步进，边缘斜率最大、中心平坦。
fn bevel_z_lens(distance: f32, bevel_width: f32, bevel_depth: f32) -> f32 {
    let t = clamp(-distance / bevel_width, 0.0, 1.0);
    return bevel_depth * (1.0 - sqrt(1.0 - t * t));
}

/// 球形弧面轮廓的归一化斜率（不含 depth/width 缩放）。
///
/// 返回 0~1 之间的归一化斜率因子，
/// 调用方再乘以 `bevel_depth / bevel_width`。
/// 当 `t >= 1.0`（完全在斜面外）时返回 1.0 避免除零。
fn bevel_slope_lens_norm(t: f32) -> f32 {
    if t >= 1.0 {
        return 1.0;
    }
    return t / sqrt(max(1.0 - t * t, 1e-6));
}
