use glam::{Vec2, Vec3};

/// 光源定义。
///
/// 描述一个虚拟光源的位置、颜色和强度，
/// 用于 Blinn-Phong 镜面高光计算。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Light {
    /// 光源位置（像素坐标或归一化坐标，取决于使用场景）。
    pub position: Vec2,

    /// 光源颜色（RGB，各通道范围 0 ~ 1）。
    pub color: Vec3,

    /// 光源强度。
    ///
    /// 值越大，高光越亮。
    pub intensity: f32,
}

impl Default for Light {
    fn default() -> Self {
        Self {
            position: Vec2::ZERO,
            color: Vec3::ONE,
            intensity: 1.0,
        }
    }
}
