---
生成时间:   2026-06-29 23:03:28 +0800
上一份日志: 2026-06-29_phase3-surface-and-depth.md
关联的提交: a7eb06a, 9934649, 10f3878, 078f4e1
---

# 2026-06-29 — 阴影调试与参数暴露

## 工作内容

1. **参数暴露** — 将此前硬编码的 `fresnel_color`、`tint_color`、`shadow_opacity`、`shadow_blur`、`shadow_offset_y` 加入 `config.toml`，新增 `[shadow]` section
2. **阴影不可见 bug 修复** — `squircle_sdf` 返回归一化 SDF（中心 ≈ -1，边缘 ≈ 0），但 `smoothstep(0, 30, -dist)` 的模糊半径用像素值比较，`-dist` 最大仅约 1，导致阴影 alpha 始终趋近于 0。修复方法：将像素 blur 换算回归一化空间：`blur_norm = shadow_blur / min_effective`
3. **玻璃内部阴影移除** — 玻璃内部的阴影（像素偏移后与 shadow SDF 交叉）产生了顶部暗、底部亮的反向光照感，去掉后只保留玻璃外部投影
4. **阴影方向修正** — 最初用 `pixel + offset_y` 偏移像素坐标来查询 SDF，导致阴影出现在面板上方。修正为偏移面板中心 `center + offset_y`，使阴影正确位于面板下方

## 关键决策

- 阴影不需要响应光源方向（Phase 3 范围外，留待后续）
- 玻璃内部不叠加阴影（避免光照方向感混乱）

## 验证结果

```
cargo clippy --all-targets  → 零 warning
cargo fmt                    → 已格式化
cargo build                  → 编译通过
```

肉眼验证阴影出现在面板正下方，参数可调，方向正确。

## 踩坑记录

- **SDF 坐标空间混淆**：`squircle_sdf` 返回的是归一化距离（单位：相对于半长轴的比例，约 -1~0），不是像素距离。模糊半径、偏移距离等需要先换算到同一个空间再做 smoothstep。换算公式：`blur_norm = pixel_blur / (min(half_size) - corner_radius)`
- **pixel vs center 偏移**：SDF 是像素坐标到形状的距离查询。要整体偏移形状，必须偏移形状的 `center`/`half_size` 参数，而不是偏移像素坐标再查同一个形状。前者是形状平移，后者是像素位置平移，语义完全不同

---

*下一份日志应引用本文件：`2026-06-29_shadow-debug-and-config-exposure.md`*
