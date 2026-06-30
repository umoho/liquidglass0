---
生成时间:   2026-06-30 20:44:33 +0800
上一份日志: 2026-06-30_phase5-parameter-system.md
关联的提交: 0788e81, 279a6e9, 88a119c
---

# 2026-06-30 — Phase 5 参数系统调试与修复

## 工作内容

1. **修复滑动条跳跃** — 将绝对坐标跟踪改为 delta 式跟踪：
   - `on_mouse_down` 时记录 `drag_start_x`（窗口坐标系）和 `drag_start_value`
   - `on_mouse_move` 时计算 `delta_px = current_x - start_x`，转为值增量
   - 根因：`e.position.x` 是窗口绝对坐标，面板位于窗口右侧（~660px 起），直接除以轨道宽度导致 `pct > 1.0` 被 clamp 到最大值

2. **修复关闭窗口不退出** — 添加 `cx.on_window_closed(|cx| cx.quit())`

3. **尝试修复窗口前端** — 设 `WindowOptions { focus: true, .. }`，macOS 视角效果不明显

4. **滚动问题深入调试与修复**：
   - **探针诊断**：在面板 div、根 div 上加 `on_scroll_wheel` 打印，在 `render()` 中打印 `ScrollHandle.offset()`
   - **关键发现**：事件到达正常、scroll_offset **确实在更新**（证明 GPUI 内置 `paint_scroll_listener` 工作），但最大偏移仅 ~21px
   - **根因定位**：GPUI 0.2.2 的 `clamp_scroll_position` 依赖 Taffy 报告的 `content_size - bounds_size`，Taffy 在 `overflow:scroll` 时报告的差值 ≈ 0~21px，导致内置滚动无法滚到有意义的位置。这与我们的布局写法（`h_full()` vs flex stretch）无关
   - **排除过程**：尝试去掉 `h_full()` 依赖 flex stretch → 结果相同（~18px）。确认是 GPUI/Taffy bug，非用法错误
   - **尝试 git 版 GPUI**：发现 git v1.8.2 无 `runtime_shaders` feature，需要 Xcode 的 `metal` 编译器，已安装失败（macOS 版本不够新）
   - **最终方案**：实现手动滚动 — `overflow_hidden()` + `on_scroll_wheel` 处理 delta + `scroll_offset: f32` 字段 + `pt(px(scroll_offset))` 偏移内容。clamp 范围 `-800..0`

## 关键决策

- **放弃 gpui-component 的 `Scrollable` 组件**：gpui-component v0.5.1 依赖 gpui 0.2.2 API，但 crates.io gpui 0.2.2 缺少 `cx.theme()`、`when_some`、`Img::new()` 等 API（80 个编译错误），无法共存
- **放弃 git 版 GPUI**：v1.8.2 无 `runtime_shaders`，需 Xcode（系统版本不够），放弃
- **手动滚动优于修复 GPUI**：定位到 bug 在 Taffy 的 `content_size` 报告，修复需要在 Taffy/GPUI 层，不可行。手动方案 10 行代码解决

## 调参面板当前状态

| 功能 | 状态 |
|---|---|
| 25 个参数滑动条（拖拽调参） | ✅ |
| 滑动条 delta 式鼠标跟踪 | ✅ |
| 菲涅尔色 / 底色 R/G/B 色块展示（只读） | ✅ |
| 保存到 config.toml（原地字段更新） | ✅ |
| 重置为默认值 | ✅ |
| 右侧面板可滚动 | ✅ |
| 关闭窗口 = 退出应用 | ✅ |
| RGB 滑条可拖动调色 | ❌ 待实现 |

## 验证结果

```
cargo check --all-targets  ✅
cargo clippy --all-targets ✅ (0 warnings from our crate)
cargo fmt                  ✅
```

---

*下一份日志应引用本文件：`2026-06-30_phase5-scroll-debug-and-fix.md`*
