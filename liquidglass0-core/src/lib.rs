//! 共享类型与工具。
//!
//! # 模块
//!
//! - [`params`] — 玻璃面板材质参数
//! - [`interaction`] — 交互状态与弹簧物理
//! - [`sdf`] — 超椭圆 SDF 与斜面轮廓

pub mod interaction;
pub mod params;
pub mod sdf;

pub use interaction::{DeformationState, InteractionState};
pub use params::GlassParams;
