---
生成时间:   2026-06-29 22:42:58 +0800
上一份日志: 2026-06-29_3d-normal-from-height-field.md
关联的提交: 6ed100f, 4d90c6b, f9e92b9
---

# 2026-06-29 — Phase 3：曲面与深度

## 工作内容

### 前置清理

- 确认 Phase 2 代码（折射、色散、菲涅尔、高光、磨砂联动）在之前已全部实现，将 `IMPLEMENTS.md` 中 Phase 2 的待办项勾选为已完成，并单独提交（`f9e92b9`）

### Phase 3 实现

实现了 Phase 3 全部 4 项功能：

1. **斜面轮廓（球形弧面凸透镜截面）** — 新增 `bevel_z_lens()` 和 `bevel_slope_lens()`，使用圆形弧面 `z = depth × (1 - sqrt(1 - t²))` 替代原来的 Hermite smoothstep，使边缘斜率最大、中心平坦，更接近真实凸透镜的截面形状
2. **动态阴影** — 在 composite shader 中直接用 SDF 计算阴影，无需额外 pass 或纹理。玻璃外像素投射软阴影（`smoothstep` + `shadow_blur` + `shadow_opacity`），玻璃内像素也叠加半透明阴影增强厚度感
3. **厚度感（thickness_multiplier）** — 新增 `thickness_multiplier` uniform，缩放 `bevel_depth`，使折射、磨砂、法线斜率同步增强
4. **尺寸自适应** — `GlassPanel` 新增 `reference_size` 字段（默认 200px），`thickness_multiplier = clamp(min(half_size) / reference_size, 1.0, 2.5)`，大面板自动模拟更厚材质

### 结构和数据流变更

- `GlassUniforms` 从 12 × vec4f（192 bytes）扩展到 13 × vec4f（208 bytes），新增 `shadow_params` 槽位
- 所有 Rust / WGSL 对应结构同步更新
- `config.toml` 新增 `reference_size` 配置项
- 保留原有的 `bevel_z()`（smoothstep）不动，新旧函数可并存

## 关键决策

- **Bevel 轮廓选型**：调研了 Apple WWDC 2025 Lensing 设计原则和社区实现后，选择球形弧面（circular arc），使其更接近物理凸透镜。Apple 的描述强调"边缘弯曲最陡、折射最强"，球形弧面比 smoothstep 更符合这一特征
- **阴影方案**：SDF 直接计算 + 预留后期升级接口。不做独立 shadow pass，避免增加额外的 compute blur 纹理和管线复杂度
- **reference_size 默认 200px**：可配置，与 demo 默认的 panel half_size 一致，使厚度乘数在默认尺寸下为 1.0

## 验证结果

```
cargo clippy --all-targets  → 零 warning
cargo fmt                    → 已格式化
cargo build                  → 编译通过
```

## 踩坑记录

- 球形弧面的导数 `dz/dt = t / sqrt(1 - t²)` 在 `t = 1`（斜面边缘）处趋向无穷大。WGSL 需用 `max(1 - t², 1e-6)` 保护分母，并在 `t >= 1` 时返回 1.0 的 clamp 值，防止除零
- Rust `f32::INFINITY` 在由 `bevel_slope_lens()` 返回时会导致 uniform 数据溢出。WGSL 侧的 `bevel_slope_lens_norm()` 已做边界保护，Rust 侧的该函数保持正确但对 WGSL 不可见（仅用于 Rust 侧参考）

---

*下一份日志应引用本文件：`2026-06-29_phase3-surface-and-depth.md`*
