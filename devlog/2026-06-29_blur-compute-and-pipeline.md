---
生成时间:   2026-06-29 03:14:59 +0800
上一份日志: 2026-06-29_shader-loader-render-init.md
关联的提交: 14cf85d
---

# 2026-06-29 — blur-compute-and-pipeline

## 工作内容

完成 Phase 1 任务 5/6/7：实现分离式高斯模糊着色器和渲染管线编排。

- 实现 `blur_horizontal.wgsl`（工作组 256×1，X 轴一维高斯核卷积）
- 实现 `blur_vertical.wgsl`（工作组 1×256，Y 轴一维高斯核卷积）
- 实现 `fullscreen_triangle.wgsl`（全屏三角形顶点着色器）
- 实现 `composite.wgsl`（采样垂直模糊结果输出）
- 添加 `BlurUniforms`（16 bytes，`Pod + Zeroable`，WGSL 布局对齐）
- 完成 `GlassRenderer::new()`：管线创建、中间纹理、bind group、sampler
- 完成 `GlassRenderer::render()`：懒尺寸检测 → uniform 写入 → blur_h dispatch → blur_v dispatch → composite draw
- 引入 `bytemuck = "1"` 用于 safe 字节转换

## 关键决策

- **wgpu 29 API 适配**：`PipelineLayoutDescriptor` 的 `bind_group_layouts` 需 `Option` 包裹，`push_constant_ranges` 被 `immediate_size` 替代，`ComputePipelineDescriptor` / `RenderPipelineDescriptor` 新增 `cache` 字段，`RenderPipelineDescriptor.multiview` 重命名为 `multiview_mask`，`RenderPassColorAttachment` 新增 `depth_slice`，`RenderPassDescriptor` 新增 `multiview_mask`，`SamplerDescriptor.mipmap_filter` 类型从 `FilterMode` 变为 `MipmapFilterMode`。
- **高斯核动态计算**：sigma = max(blur_radius / 2, 0.5)，kernel_half = ceil(3σ)，每帧在 CPU 侧算好传入 uniform，shader 中只做卷积循环。
- **中间纹理格式**：`Rgba8Unorm`（storage 兼容），输出格式沿用 config 的 `Rgba8UnormSrgb`。
- **显式 bind group layout**：不依赖 wgpu 自动布局，所有绑定显式声明，为后续 naga_oil 切换打基础。

## 验证结果

- `cargo clippy --all-targets` — 零 warning
- `cargo fmt` — 格式一致

## 踩坑记录

- `include_str!` 路径修正（上一份日志已记录，本次无关）。
- wgpu 29 与之前版本的 API 差异较大，通过直接阅读 `~/.cargo/registry` 中的源码确定正确字段。

---

*下一份日志应引用本文件：`2026-06-29_blur-compute-and-pipeline.md`*
