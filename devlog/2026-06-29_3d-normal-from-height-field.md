---
生成时间:   2026-06-29 22:14:44 +0800
上一份日志: 2026-06-29_fresnel-edge-glow-tuning.md
关联的提交: a4be4d7, b62a9bb
---

# 2026-06-29 — 从高度场推导 3D 法线

## 工作内容

针对遗留问题"菲涅尔边缘光不如 Apple"和"高光不可见"（均为 P1），
从 bevel 高度场梯度推导 3D 表面法线，替换原有的 2D 法线 + 距离启发式方案。

### 调研：Apple 的做法与开源复现

通过公开资料和第三方开源项目调研发现：

- **Apple 的 Liquid Glass** 在 iOS/macOS 上是 2D 后处理着色器效果（visionOS 才是 3D），
  和我们做的是同一类事
- 多个开源复现（temoki/lq、CrystalKit、liquid-glass-studio、Pixelux-Glass）都使用
  高度场 → 3D 法线的方式，验证了这个方向
- temoki/lq 使用圆形剖面 `sqrt(r²-x²)` 而非 smoothstep
- CrystalKit 使用 Jacobi 热扩散求解 Laplace 方程生成高度场，杜绝中轴伪影

### 实现：3D 法线推导（`a4be4d7`）

在 `composite.wgsl` 中新增从 bevel_z 梯度推导 3D 法线的计算：

```wgsl
let surf_grad = sdf_normal * slope;
let normal_3d = normalize(vec3f(-surf_grad, 1.0));
```

影响三个地方：
| 效果 | 旧方案 | 新方案 |
|---|---|---|
| 菲涅尔 | 纯距离 pow(1-edge_ratio, 6) | schlick_fresnel(normal_3d.z, 0.04) |
| 高光（光源 0） | dot(vec3f(normal, 0.0), half_vec) | dot(normal_3d, half_vec) |
| 高光（光源 1/2） | 同上 | 同上 |

与 `refract.wgsl:44-46` 所用斜率公式一致，保持一致性。

### 修复：Fresnel intensity 放大基础反射率（`b62a9bb`）

发现 `intensity` 乘在整个 Fresnel 上会导致面板平坦区整体泛白（平坦区
cos_theta=1 → schlick_fresnel=0.04 → ×intensity 后 > 0.04）。

改为只放大掠射角部分：

```wgsl
let f0 = 0.04;
let fresnel = f0 + (schlick_fresnel(cos_theta, f0) - f0) * fresnel_intensity;
```

- 平坦区（cos_theta=1）：fresnel ≡ 0.04，不受 intensity 影响
- 掠射角（cos_theta→0）：fresnel = 0.04 + 0.96 × intensity

## 关键决策

- **3D 法线 + 物理菲涅尔方向正确**：虽然当前 smoothstep bevel 导致最大斜率在
  bevel 中段而非外缘（视觉效果不够锐利），但物理模型本身正确
- **Fresnel intensity 语义变更**：从"整体亮度倍增"改为"掠射角增强量"，
  这是更物理正确的语义
- **暂缓参数调优到 Phase 5**：当前视觉差距的根因可能是：
  1. bevel 剖面（smoothstep vs 圆形）
  2. 高度场生成（Laplace 扩散 vs 一维 smoothstep）
  3. 需要更精细的 glare / 多重效果组合

## 验证结果

- `cargo clippy --all-targets` — 0 warnings
- `cargo fmt` — 无 diff
- `cargo run -p liquidglass0-demo` — 窗口正常打开

## 踩坑记录

- **Fresnel intensity 语义**：历史上在 `259db1c` 也用过斜率方案，
  但当时高光是坏的（Z=0 法线导致 X 形伪影），掩盖了 intensity 问题。
  等 `ff379e6` 切换到距离 Fresnel 后，intensity 问题变成了"强度不够/过曝"
  的二选一。
- **调参无果后发现是代码问题**：花了时间调参（`bevel_depth=90`、
  `fresnel_intensity=5.0`），但效果不理想，最终发现是 Fresnel 公式本身的
  语义问题，非参数问题。下次应先检查公式再调参。

---

*下一份日志应引用本文件：`2026-06-29_3d-normal-from-height-field.md`*
