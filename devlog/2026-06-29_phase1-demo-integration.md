---
生成时间:   2026-06-29 10:24:13 +0800
上一份日志: 2026-06-29_blur-compute-and-pipeline.md
关联的提交: de45507, ba7db6a, 5f4ed29, 6555c42
---

# 2026-06-29 — Phase 1 收尾

## 工作内容

完成 Phase 1 最后两项任务：liquidglass0-demo 集成 winit 窗口与渲染管线，
端到端验证高斯模糊玻璃效果。

### demo crate 搭建
- 添加 winit 0.30、wgpu 29、pollster 0.4、liquidglass0-core/render 依赖
- 拆分为 `main.rs`（wgpu 初始化 + event loop 入口）和 `app.rs`（App struct + ApplicationHandler）
- `App` 持有 device/queue 两份 clone（一份给 renderer、一份给 demo 侧 surface configure / submit）
- `GlassRenderer` 零改动，device/queue 通过 clone 在外部保留副本

### 背景纹理
- 程序化生成 RGBA8 棋盘格（50px tile，暖橙/冷蓝交替）
- 通过 `queue.write_texture()` 上传，适配 wgpu 29 重命名后的 `TexelCopyTextureInfo` / `TexelCopyBufferLayout`

### wgpu 29 API 适配
- `InstanceDescriptor::new_without_display_handle()` 替代已移除的 `Default`
- `DeviceDescriptor` 新增 `experimental_features` 和 `trace` 字段
- `request_device()` 移除第二个参数 `trace_dir`
- `Surface::get_current_texture()` 返回 `CurrentSurfaceTexture` 枚举替代 `Result`
- `ImageCopyTexture` → `TexelCopyTextureInfo`，`ImageDataLayout` → `TexelCopyBufferLayout`

### 模糊效果验证
- 默认 `blur_radius = 2.0` 几乎不可见（sigma=1，仅 ±3px）
- 调至 25.0（sigma=12.5，核覆盖 ±38px）效果一目了然
- 这是验证级调参，`GlassParams::default()` 保持 2.0 不变

### 文档注释
- 按 AGENTS.md 规范补齐 `App` 字段、所有方法及 trait 实现的 doc 注释（汉语）

## 关键决策

- **device/queue 双份 clone**：renderer 内部持有一份，demo 侧保留原始句柄用于 surface configure 和 submit。避免了在 renderer 上加存取器方法，保持其"对窗口无感知"架构。
- **winit 0.30 而非 0.31**：0.31 尚在 beta，API 大改（Window 变 trait、事件重命名）。锁 0.30.13 稳定版。
- **程序化背景而非 image crate**：Phase 1 只需验证管线正确性，程序化棋盘格无需外部图片依赖，减少 crate 数量和编译时间。

## 验证结果

- `cargo clippy --all-targets` — 0 warnings
- `cargo fmt` — 无 diff
- `cargo build -p liquidglass0-demo` — 编译通过
- `cargo run -p liquidglass0-demo` — 窗口正常打开，可见棋盘格经高斯模糊后的柔和渐变效果

## 踩坑记录

- wgpu 29 相比 22 有多处 API 重命名（`ImageCopyTexture` → `TexelCopyTextureInfo` 等），初次编译报 13 个 error，逐一查源码修复
- `SurfaceConfiguration` 变为泛型 `SurfaceConfiguration<V>`，`view_formats` 字段类型为泛型参数，传 `vec![format]` 编译器可自动推断

---

*下一份日志应引用本文件：`2026-06-29_phase1-demo-integration.md`*
