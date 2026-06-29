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
///
/// # 参数
///
/// * `p` - 待测点坐标
/// * `center` - 面板中心
/// * `half_size` - 面板半宽/半高
/// * `corner_radius` - 圆角半径（像素）
/// * `n` - 超椭圆指数，范围 4 ~ 6
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
