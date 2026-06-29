use glam::Vec2;

/// 有限差分步长。
const EPSILON: f32 = 1e-3;

/// 超椭圆 signed distance function。
///
/// 公式：
///
/// $$ \left|\frac{x}{r_x}\right|^n + \left|\frac{y}{r_y}\right|^n = 1 $$
///
/// 对应的 SDF 近似为：
///
/// $$ \operatorname{sdf}(p) = \left( \left|\frac{p_x}{r_x}\right|^n + \left|\frac{p_y}{r_y}\right|^n \right)^{\frac{1}{n}} - 1 $$
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
pub fn squircle_sdf(p: Vec2, center: Vec2, half_size: Vec2, corner_radius: f32, n: f32) -> f32 {
    let d = p - center;
    let effective = half_size - Vec2::splat(corner_radius);

    if effective.x <= 0.0 || effective.y <= 0.0 {
        return Vec2::ZERO.length();
    }

    let nx = (d.x / effective.x).abs();
    let ny = (d.y / effective.y).abs();

    let r = (nx.powf(n) + ny.powf(n)).powf(1.0 / n);
    r - 1.0
}

/// 由 SDF 梯度求表面法线。
///
/// 对 [`squircle_sdf`] 做有限差分，取梯度的负方向（法线指向外侧），
/// 归一化后返回。
pub fn sdf_normal(p: Vec2, center: Vec2, half_size: Vec2, corner_radius: f32, n: f32) -> Vec2 {
    let dx = Vec2::new(EPSILON, 0.0);
    let dy = Vec2::new(0.0, EPSILON);

    let gx = squircle_sdf(p + dx, center, half_size, corner_radius, n)
        - squircle_sdf(p - dx, center, half_size, corner_radius, n);
    let gy = squircle_sdf(p + dy, center, half_size, corner_radius, n)
        - squircle_sdf(p - dy, center, half_size, corner_radius, n);

    let grad = Vec2::new(gx, gy);
    // 归一化梯度即法线方向（指向外侧）
    //
    // $$ \mathbf{N} = \frac{\nabla \operatorname{sdf}(p)}{|\nabla \operatorname{sdf}(p)|} $$
    grad.normalize_or_zero()
}

/// 斜面轮廓：将 SDF 距离映射为 Z 位移。
///
/// 边缘最深（`bevel_depth`），中心为 0。
///
/// 公式：$$ z = \operatorname{smoothstep}\left(\frac{d}{b_w}\right) \cdot b_d $$
///
/// 其中 $b_w$ 为斜面宽度，$b_d$ 为斜面深度。
///
/// # 参数
///
/// * `distance` - SDF 距离值
/// * `bevel_width` - 斜面宽度，[`GlassParams::bevel_width`]
/// * `bevel_depth` - 斜面最大深度（像素），[`GlassParams::bevel_depth`]
pub fn bevel_z(distance: f32, bevel_width: f32, bevel_depth: f32) -> f32 {
    let t = (distance.clamp(-bevel_width, 0.0) / bevel_width) + 1.0;
    let st = t * t * (3.0 - 2.0 * t);
    st * bevel_depth
}

/// 球形弧面轮廓（凸透镜截面）。
///
/// 将 SDF 距离映射为 Z 位移。使用圆形弧线替代平滑步进，
/// 使边缘斜率最大、中心平坦，更接近真实凸透镜的横截面。
///
/// 公式：
///
/// $$ t = \max\!\left(-\frac{d}{b_w}, 0.0\right) \quad z = b_d \cdot \left(1 - \sqrt{1 - t^2}\right) $$
///
/// # 参数
///
/// * `distance` - SDF 距离值
/// * `bevel_width` - 斜面宽度（像素）
/// * `bevel_depth` - 斜面最大深度（像素）
pub fn bevel_z_lens(distance: f32, bevel_width: f32, bevel_depth: f32) -> f32 {
    let t = (-distance / bevel_width).clamp(0.0, 1.0);
    bevel_depth * (1.0 - (1.0 - t * t).sqrt())
}

/// 球形弧面轮廓的斜率（导数）。
///
/// 返回 Z 对水平距离的导数 $ \frac{dz}{dx} $，用于折射偏移计算。
///
/// 公式：
///
/// $$ \frac{dz}{dx} = \frac{b_d \cdot t}{b_w \cdot \sqrt{1 - t^2}} $$
///
/// # 参数
///
/// 同 [`bevel_z_lens`]。
pub fn bevel_slope_lens(distance: f32, bevel_width: f32, bevel_depth: f32) -> f32 {
    let t = (-distance / bevel_width).clamp(0.0, 1.0);
    if t >= 1.0 {
        return f32::INFINITY;
    }
    bevel_depth * t / (bevel_width * (1.0 - t * t).sqrt())
}
