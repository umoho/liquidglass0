//! 共享类型与工具。
//!
//! # 模块
//!
//! - [`panel`] — 玻璃面板形状定义
//! - [`material`] — 玻璃材质参数
//! - [`light`] — 光源定义
//! - [`scene`] — 场景配置
//! - [`interaction`] — 交互状态与弹簧物理
//! - [`sdf`] — 超椭圆 SDF 与斜面轮廓

pub mod interaction;
pub mod light;
pub mod material;
pub mod panel;
pub mod scene;
pub mod sdf;

pub use interaction::{DeformationState, InteractionState};
pub use light::Light;
pub use material::GlassMaterial;
pub use panel::GlassPanel;
pub use scene::Scene;
