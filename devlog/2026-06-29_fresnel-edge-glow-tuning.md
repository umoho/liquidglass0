---
生成时间:   2026-06-29 21:15:56 +0800
上一份日志: 2026-06-29_refraction-fix-and-config.md
关联的提交: ff379e6, c9d220b
---

# 2026-06-29 — 菲涅尔边缘光调优

## 工作内容

针对遗留问题"菲涅尔边缘光仍较弱"（P1），
基于 Apple 官方 Liquid Glass 参考图进行多轮参数调优。

### 第一轮：增强菲涅尔基础参数（`ff379e6`）

| 参数 | 旧值 | 新值 | 理由 |
|---|---|---|---|
| `fresnel_intensity` | 2.0 | 2.5 | 增强边缘光亮度 |
| `schlick_fresnel` 的 `f0` | 0.04 | 0.06 | 略提高基础反射率 |

结果：边缘光亮度有轻微提升，但仍弥散不锐利。

### 第二轮：进一步增强（`ff379e6`）

| 参数 | 旧值 | 新值 | 理由 |
|---|---|---|---|
| `fresnel_intensity` | 2.5 | 3.0 | 继续增强 |
| `schlick_fresnel` 的 `f0` | 0.06 | 0.08 | 继续提高基础反射率 |

结果：边缘光更亮但仍发散，与 Apple 参考图差距明显。

### 第三轮：收窄渐变范围（`ff379e6`）

将 `edge_t` 从线性渐变改为 smoothstep 窄化：

```wgsl
let edge_ratio = clamp(-dist / bevel_width_px, 0.0, 1.0);
let edge_t = 1.0 - smoothstep(0.0, 0.3, edge_ratio);
```

效果：只在 bevel 最外侧 30% 产生渐变，但视觉改善有限。

### 第四轮：修正菲涅尔方向（`ff379e6`）

发现根本问题：`schlick_fresnel(edge_t, ...)` 中 `edge_t` 方向反了。

- 边缘（`edge_t=1.0`）→ cos_theta=1.0 → fresnel=f0（最小）❌
- 内部（`edge_t=0.0`）→ cos_theta=0.0 → fresnel=1.0（最大）❌

修正为 `schlick_fresnel(1.0 - edge_t, 0.04)`：

- 边缘 → cos_theta=0.0 → fresnel=1.0（最大）✓
- 内部 → cos_theta=1.0 → fresnel=f0（最小）✓

结果：方向正确但强度过高（`fresnel_intensity=3.0`），边缘过曝。

### 第五轮：降低强度 + 收窄范围（`ff379e6`）

| 参数 | 旧值 | 新值 |
|---|---|---|
| `fresnel_intensity` | 3.0 | 1.5 |
| `smoothstep` 范围 | 0.3 | 0.15 |

结果：强度过低，边缘光不可见。

### 第六轮：pow 陡峭衰减（`ff379e6`）

用 `pow` 替代 `smoothstep`，实现更陡峭的衰减：

```wgsl
let edge_t = pow(1.0 - edge_ratio, 6.0);
```

| 参数 | 旧值 | 新值 |
|---|---|---|
| `fresnel_intensity` | 1.5 | 2.0 |
| 衰减函数 | `smoothstep(0.0, 0.15, ...)` | `pow(1.0 - edge_ratio, 6.0)` |

结果：边缘光仍然不如 Apple 参考图。

## 关键决策

- **修正菲涅尔方向是必要的**：`schlick_fresnel` 的 cos_theta 必须从边缘=0 到内部=1，否则效果反转
- **参数调优有上限**：2D SDF 法线缺少 Z 分量，无法产生真实的菲涅尔角度关系
- **根因定位**：Apple 是 3D 渲染（真实表面法线），我们是 2D SDF（法线在 XY 平面），需要从 bevel 高度场推导 3D 法线

## 验证结果

- `cargo clippy --all-targets` — 0 warnings
- `cargo fmt` — 无 diff
- `cargo run -p liquidglass0-demo` — 窗口正常打开，边缘光效果有改善但不如 Apple

## 踩坑记录

- **菲涅尔方向反转**：`edge_t` 在边缘=1.0，但 `schlick_fresnel(cos_theta)` 要求 cos_theta 在边缘=0.0（grazing angle）。必须传入 `1.0 - edge_t`。
- **smoothstep 范围与强度的平衡**：收窄范围会降低总能量，需要相应提高强度，否则整体变暗。
- **2D vs 3D 的根本限制**：`sdf_normal` 只返回 `(x, y)`，Z 分量为 0。菲涅尔和高光计算都需要 3D 法线。手动 `edge_t` 只是近似，无法完全模拟真实 3D 效果。

## 遗留问题

| 问题 | 优先级 | 说明 |
|---|---|---|
| 菲涅尔边缘光仍不如 Apple | P1 | 根因是 2D 法线，需要推导 3D 法线 |
| 高光不可见 | P1 | 同样因为 2D 法线，Blinn-Phong 计算不准确 |
| 需要实现 3D 法线推导 | P1 | 从 bevel_z 梯度推导 `normal_3d = normalize(vec3f(-surf_grad, 1.0))` |

---

*下一份日志应引用本文件：`2026-06-29_fresnel-edge-glow-tuning.md`*
