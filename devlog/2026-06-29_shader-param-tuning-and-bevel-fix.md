---
生成时间:   2026-06-29 16:14:08 +0800
上一份日志: 2026-06-29_naga-oil-import-fix.md
关联的提交: 48e031c, 259db1c, 94dbdb6, c6042b9
---

# 2026-06-29 — 着色器参数调优与 bevel 修正

## 工作内容

从 demo 首次成功运行到逐步接近 Liquid Glass 效果的迭代过程，
涵盖参数调优、着色器逻辑修复和 bevel 单位修正三个阶段。

### 第一阶段：参数调优（`48e031c`）

调整 `app.rs` 中的面板形状、光学参数和光源位置：

| 类别 | 参数 | 旧值 | 新值 |
|---|---|---|---|
| 形状 | `corner_radius` | 22 | 28 |
| | `bevel_width` | 0.15 | 0.20 |
| | `bevel_depth` | 40 | 55 |
| 光学 | `refractive_index` | 1.5 | 1.52 |
| | `chromatic_strength` | 0.02 | 0.03 |
| | `fresnel_intensity` | 1.0 | 1.5 |
| | `specular_intensity` | 0.5 | 0.4 |
| | `specular_shininess` | 80 | 150 |
| | `blur_radius` | 15 | 12 |
| 材质 | `tint_opacity` | 0.15 | 0.08 |
| | `background_opacity` | 0.85 | 0.92 |
| | `saturation` | 1.8 | 1.4 |
| | `brightness` | 0.06 | 0.08 |
| 光源 | Light 0 | (0.3w, 0.2h), 0.8 | (0.2w, 0.15h), 0.9 |
| | Light 1 | (0.7w, 0.3h), 0.6 | (0.85w, 0.25h), 0.5 |
| | Light 2 | (0.5w, 0.8h), 0.4 | (0.75w, 0.8h), 0.3 |

![参数调优后](../captures/截屏2026-06-29%2013.52.21.png)

### 第二阶段：着色器逻辑修复（`259db1c`）

参数调优效果有限，X 形高光仍然明显。定位到三个着色器层面的问题：

**修复 1：菲涅尔计算方向错误**

原代码用 2D SDF 法线与 `(0,1)` 点积，无法正确表达凸面掠射角。
改为基于 `bevel_z` 的斜率推导：

```wgsl
// 旧
let view_dot = abs(dot(normal, vec2f(0.0, 1.0)));
// 新
let slope = thickness / max(bevel_width, 0.001);
let view_dot = 1.0 / sqrt(1.0 + slope * slope);
```

**修复 2：高光加法混合导致中心过曝**

原代码 `color += specular_total` 直接加法叠加，三个光源形成 X 形伪影。
改为限制在 bevel 区域并降低强度：

```wgsl
color += specular_total * bevel_mask * 0.4;
```

**修复 3：模糊纹理采样 UV 错误**

原代码 `textureSample(blur_tex, tex_sampler, uv)` 用原始 UV 采样模糊纹理，
与折射后的位置不对齐。改为 `refracted_uv`。

![着色器修复后](../captures/截屏2026-06-29%2014.39.45.png)

### 第三阶段：bevel_width 单位修正（`94dbdb6`）

修复后菲涅尔边缘光过亮、中心仍无折射效果。定位到根因：

**`bevel_width` 在 Rust 侧定义为比例（"占半径的比例"），
但着色器直接作为像素值使用。**

`bevel_width = 0.20`（比例）被当作 0.20 像素，导致 bevel 区域仅 0.2px 宽，
几乎不存在。所有基于 bevel 的效果（折射、磨砂、菲涅尔）均失效。

修复方案——在着色器中将比例转换为像素：

```wgsl
let bevel_width_ratio = u.shape_params.y;
let bevel_width_px = bevel_width_ratio * min(half_size.x, half_size.y);
```

同步修复了 `composite.wgsl` 和 `refract.wgsl`。
修正后菲涅尔斜率改用 smoothstep 导数计算：

```wgsl
let t = (clamp(dist, -bevel_width_px, 0.0) / bevel_width_px) + 1.0;
let dz_dt = 6.0 * t * (1.0 - t);
let slope = dz_dt * bevel_depth / bevel_width_px;
```

![bevel 修正后](../captures/截屏2026-06-29%2014.50.22.png)

## 关键决策

- **菲涅尔用 smoothstep 导数而非简单比值**：`thickness / bevel_width` 不是真实斜率，
  smoothstep 导数 `6t(1-t)` 才是 `bevel_z` 对距离的导数，能正确反映表面倾角。
- **高光用 bevel_mask 而非降低 specular_intensity**：保留光源对边缘的贡献，
  只在中心区域屏蔽，效果更自然。
- **保留 3 个光源**：虽然改为 2 个可减少视觉复杂度，但 3 光源配合 mask 能产生
  更丰富的边缘高光分布。

## 验证结果

- `cargo clippy --all-targets` — 0 warnings（每次提交前均通过）
- `cargo fmt` — 无 diff
- `cargo run -p liquidglass0-demo` — 窗口正常打开，4 张截图对应 4 个迭代阶段

### 截图对比

| 阶段 | 关键变化 |
|---|---|
| 基线 | X 形高光，中心过曝 |
| | ![基线](../captures/截屏2026-06-29%2013.45.45.png) |
| 参数调优后 | 效果变化不明显 |
| | ![参数调优](../captures/截屏2026-06-29%2013.52.21.png) |
| 着色器修复后 | X 形消除，菲涅尔边缘光出现，但过亮 |
| | ![着色器修复](../captures/截屏2026-06-29%2014.39.45.png) |
| bevel 修正后 | 中心清晰，折射变形可见，边缘光自然 |
| | ![bevel修正](../captures/截屏2026-06-29%2014.50.22.png) |

## 踩坑记录

- **bevel_width 单位混淆**：`panel.rs` 注释写"占半径的比例"，但 `sdf::bevel_z()`
  的 `bevel_width` 参数在 WGSL 中直接与 SDF 距离（像素）做 clamp。
  这导致比例值 0.20 被当作 0.20 像素使用。修复方式是在着色器中显式转换。
- **WGSL `let` 不可重复声明**：修复菲涅尔时曾重复定义 `thickness`，导致编译错误。
  需注意 WGSL 的 `let` 作用域为整个函数体，不能在同一作用域内二次声明。
- **X 形高光的根因是多因素叠加**：光源三角形布局 + 加法混合 + 无区域限制，
  三者共同导致。单独调整光源位置或降低强度都无法完全消除。

---

*下一份日志应引用本文件：`2026-06-29_shader-param-tuning-and-bevel-fix.md`*
