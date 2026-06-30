---
生成时间:   2026-06-30 17:31:46 +0800
上一份日志: 2026-06-30_phase4-interaction-debug.md
关联的提交: 23a1847, e9f4982
---

# 2026-06-30 — Phase 5 参数系统实现

## 工作内容

1. **方案调研** — 评估 GPUI 作为参数面板 UI 框架的可行性：
   - GPUI 是 Zed 编辑器的 GPU 加速 UI 框架（Apache-2.0）
   - 发现 [gpui-component]（Longbridge，11.9k stars，Apache-2.0），提供 60+ 组件含 Slider、ColorPicker
   - Licenses 验证：gpui / gpui_platform / gpui-component 均为 Apache-2.0，与项目 license 一致

2. **架构设计** — 经过多轮讨论最终确定：
   - ~~双窗口方案~~ → 模式分离方案：`cargo run` 查看模式（winit），`cargo run -- --tune` 调参模式（GPUI）
   - ~~gpui-component Slider~~ → 手动实现滑动条（因 crates.io gpui 0.2.2 与 gpui-component v0.5.1 存在 API 不兼容）
   - ~~实时 wgpu 预览~~ → 静态占位（需完整 Xcode 的 `metal` 编译器，当前仅 Command Line Tools）
   - 参数面板布局：左侧预览占位 + 右侧 340px 可滚动参数区

3. **代码实现**（`liquidglass0-demo` crate 内）：
   - `Cargo.toml`：添加 `gpui`（crates.io 0.2.2 + `runtime_shaders` feature）、`toml_edit`（0.23）
   - `config.rs`：新增 `Config::save()` 方法，用 `toml_edit` 实现原地 TOML 字段更新（保留注释和结构）
   - `main.rs`：解析 `--tune` 命令行参数，分发到 `tune::run()` 或原有 `app::App`
   - `tune.rs`（新文件，~640 行）：
     - `ParamSlider` 结构体：存储 label / value / min / max / step / 格式化函数
     - `TuneApp` 实体：持有 5 组参数（面板形状 / 光学 / 材质 / 阴影 / 交互物理）共 25 个滑动条
     - 颜色参数：`fresnel_color` / `tint_color` 各自 R/G/B 色块预览（只读）
     - 手动实现滑动条轨道 + 拇指的拖拽交互（`on_mouse_down` / `on_mouse_up` / `on_mouse_move`）
     - 保存按钮：将滑块值同步到 `Config`，调用 `Config::save()` 写回 `config.toml`
     - 重置按钮：恢复 `Config::default()` 值到所有滑块

4. **环境配置** — 解决编译问题：
   - gpui 0.2.2 build.rs 需要 macOS SDK 头文件（`dispatch/dispatch.h`），Command Line Tools 不提供默认的 SDK 搜索路径
   - 通过 `runtime_shaders` feature 绕过 Metal 着色器预编译（避免需要完整 Xcode 的 `metal` 工具）
   - 创建 `.cargo/config.toml` 持久化 `SDKROOT` 环境变量

## 关键决策

- **放弃 gpui-component**：gpui-component v0.5.1 依赖 gpui 0.2.2 API，但 crates.io 版 gpui 0.2.2 缺少 `cx.theme()`、`when_some`、`overflow_y_scroll`、`Img::new()` 等 API。git 版（Zed v1.8.2）则与 gpui-component 有 80 个 API 不兼容错误。手动实现滑动条仅 ~400 行，比解决版本冲突更快。
- **放弃实时预览**：wgpu 需要离屏渲染 → CPU 读回 → PNG 编码 → GPUI Image 显示，但 crates.io gpui 0.2.2 的 Image/Img API 完全不同于 git 版（无 `Image::new()` 构造函数），且无 timer 支持（`AsyncApp` 的 `update` API 签名与 `Context::spawn` 闭包不兼容），在无完整 Xcode 的条件下不值得强求。
- **模式分离而非双窗口**：避免了 GPUI 和 winit 争抢 macOS 主线程的线程模型冲突。

## 验证结果

```
cargo check --all-targets  ✅
cargo clippy --all-targets ✅ (1 clippy suggestion applied)
cargo fmt                  ✅
```

## 已知问题（待修复）

- [ ] 滑动条一点就跳到最大值：`e.position.x` 是窗口绝对坐标，未减除轨道偏移量
- [ ] 关闭窗口不退出：GPUI 0.2.2 默认 hide 而非 quit，需 `cx.on_window_closed()`
- [ ] 窗口不在最前：`WindowOptions` 缺 `focus: true`
- [ ] 侧边面板不能滚动：需配合 `ScrollHandle` + `track_scroll()`

---

*下一份日志应引用本文件：`2026-06-30_phase5-parameter-system.md`*
