//! 玻璃渲染管线。
//!
//! 基于 wgpu 的 Liquid Glass 效果渲染器，对窗口无感知。

mod config;
mod input;
mod renderer;
mod shader;

pub use config::RendererConfig;
pub use input::RenderInput;
pub use renderer::GlassRenderer;
pub use shader::{EmbeddedLoader, ShaderLoader};
