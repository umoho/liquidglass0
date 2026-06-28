---
生成时间:   2026-06-29 02:32:01 +0800
上一份日志: 2026-06-29_initial-project-docs.md
关联的提交: a947de7, 0364560, c37d5a1, c42e118
---

# 2026-06-29 — 工作区骨架与 core 实现

## 工作内容

完成 Phase 1 前三项任务：

1. 搭建 Rust workspace 骨架（`liquidglass0-core` / `-render` / `-demo`）
2. 配置根 `Cargo.toml` workspace，当前只声明 `glam` 统一依赖
3. 实现 `liquidglass0-core`：`GlassParams`（21 字段 + `Default`）、`InteractionState`（含 `DeformationState` 枚举）、SDF 工具（超椭圆、法线、斜面轮廓）
4. 同步更新 `ARCHITECTURE.md`，将 core 依赖列从 `—` 改为 `glam`
5. 勾选 `IMPLEMENTS.md` Phase 1 前三项

## 关键决策

- **引入 glam**：core 最初按架构文档标注为无依赖，经讨论后改为依赖 `glam`（libm only）。理由：SDF 工具大量向量运算，裸 `[f32; N]` 需手写工具函数，等于复刻 glam 子集；且 wgpu 生态以 glam 为事实标准，后续 `bytemuck` 零拷贝到 GPU 统一方便
- **暂缓 wgpu/winit**：`render` 和 `demo` 均为空骨架，`wgpu` 和 `winit` 到实际使用时再引入 workspace.dependencies，避免过早锁定版本
- **edition 2024**：当前 Rust 1.96，最新 edition 为 2024
- **LaTeX 公式**：SDF 模块注释使用 `$$ ... $$` LaTeX 格式，rustdoc 原生支持，可读性优于纯文本
- **参数行为描述**：每个标量字段注明"值越大，…"，帮助调参时快速理解效果变化方向

## 验证结果

```
cargo clippy --all-targets   # 零 warning
cargo fmt                     # 全部通过
```

三个 crate 均编译通过：

| crate | 状态 |
|---|---|
| `liquidglass0-core` | GlassParams / InteractionState / SDF 工具就位 |
| `liquidglass0-render` | 空骨架 |
| `liquidglass0-demo` | 空骨架 |

## 踩坑记录

- `[f32; N]` 裸数组方案被放弃：手写 `vec2_add` / `vec2_dot` / `vec2_length` 代码臃肿，替换为 glam 后 SDF 模块清晰一个量级
- 最初打算 workspace.dependencies 里同时声明 `wgpu` 和 `winit`，实际当前阶段用不到，提前声明不符合最小化原则

---

_下一份日志应引用本文件：`2026-06-29_workspace-skeleton-core.md`_
