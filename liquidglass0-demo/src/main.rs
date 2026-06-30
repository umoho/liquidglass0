//! liquidglass0 演示程序入口。
//!
//! 查看模式（默认）：创建 winit 窗口和 wgpu 设备渲染玻璃效果。
//! 调参模式（`--tune`）：启动 GPUI 参数面板 + 离屏预览。

mod app;
mod config;
mod tune;

/// 解析命令行参数，分发到查看/调参模式。
///
/// # Panics
///
/// 硬件不支持 wgpu 或窗口创建失败时 panic。
fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 && args[1] == "--tune" {
        let config = config::Config::load("config.toml");
        tune::run(config);
        return;
    }

    // 查看模式：原有 winit + wgpu 渲染
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
        experimental_features: wgpu::ExperimentalFeatures::disabled(),
        memory_hints: wgpu::MemoryHints::Performance,
        trace: wgpu::Trace::Off,
    }))
    .unwrap();

    let app = app::App::new(instance, adapter, device, queue);

    let event_loop = winit::event_loop::EventLoop::new().unwrap();
    let mut app = app;
    event_loop.run_app(&mut app).unwrap();
}
