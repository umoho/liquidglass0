//! liquidglass0 演示程序入口。
//!
//! 创建 winit 窗口和 wgpu 设备，将背景纹理送入
//! [`liquidglass0_render::GlassRenderer`] 进行高斯模糊渲染。

mod app;
mod config;

use wgpu::{ExperimentalFeatures, Trace};

use winit::event_loop::EventLoop;

/// 初始化 wgpu 并启动 winit 事件循环。
///
/// # Panics
///
/// 硬件不支持 wgpu 或窗口创建失败时 panic。
fn main() {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());

    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
    }))
    .unwrap();

    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: None,
        required_features: wgpu::Features::default(),
        required_limits: wgpu::Limits::default(),
        experimental_features: ExperimentalFeatures::disabled(),
        memory_hints: wgpu::MemoryHints::Performance,
        trace: Trace::Off,
    }))
    .unwrap();

    let app = app::App::new(instance, adapter, device, queue);

    let event_loop = EventLoop::new().unwrap();
    let mut app = app;
    event_loop.run_app(&mut app).unwrap();
}
