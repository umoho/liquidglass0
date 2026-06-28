---
生成时间:   2026-06-29 02:54:47 +0800
上一份日志: 2026-06-29_workspace-skeleton-core.md
关联的提交: 9972e1b, f2b1c7e
---

# 2026-06-29 — shader-loader-render-init

## 工作内容

完成 Phase 1 第四项：`liquidglass0-render` 着色器加载抽象 + wgpu 设备集成。

- 在 workspace 和 `-render` crate 引入 `wgpu = "29"`
- 定义 `ShaderLoader` trait 和 `EmbeddedLoader`（编译期 `include_str!`）
- 创建 `RendererConfig`（纹理格式、工作组大小）
- 创建 `RenderInput`（每帧数据，引用 `core` 的 `GlassParams` / `InteractionState`）
- 创建 `GlassRenderer` 骨架（`new()` / `render()`），存储 `Device` / `Queue`
- 搭建 `shaders/` 目录结构，放置 4 个占位 `.wgsl` 文件

## 关键决策

- **wgpu 版本**：调查 `naga_oil` 与 `wgpu` 的兼容性后，选用 `wgpu = "29"`（对应 `naga_oil = "0.22"`），两者通过 `naga` IR 版本绑定。
- **不引入 `bytemuck`**：当前任务只搭框架，uniform buffer 写入在后续管线任务中才需要。
- **不引入 `naga_oil`**：Phase 1 只需 `include_str!` 嵌入占位 shader。
- **`GlassRenderer` 不创建 wgpu Instance**：`new()` 接收外部已创建好的 `Device` / `Queue`，保持渲染器与窗口解耦。

## 验证结果

- `cargo clippy --all-targets` — 无 warning
- `cargo fmt` — 格式一致

## 踩坑记录

- `include_str!` 路径：文件在 `liquidglass0-render/src/shader.rs`，shader 在仓库根 `shaders/`，需用 `../../shaders/...`（两层向上），初版误写 `../../../` 导致 clippy 报文件不存在。

---

*下一份日志应引用本文件：`2026-06-29_shader-loader-render-init.md`*
