---
生成时间:   2026-06-29 19:28:01 +0800
上一份日志: 2026-06-29_shader-param-tuning-and-bevel-fix.md
关联的提交: 5fd0314, fac8b6f, 1501402
---

# 2026-06-29 — 折射修正与配置文件系统

## 工作内容

从上一轮参数调优后，继续迭代渲染效果，
重点解决折射变形 X 形伪影问题，并引入 TOML 配置文件系统。

### 第一阶段：参数微调（`5fd0314`）

基于 Apple 官方 Liquid Glass 参考图对比，调整参数：

| 参数 | 旧值 | 新值 | 理由 |
|---|---|---|---|
| `refractive_index` | 1.52 | 1.3 | 降低折射强度，减少过度变形 |
| `fresnel_intensity` | 1.5 | 2.0 | 增强边缘光 |
| `blur_radius` | 12 | 20 | 增加磨砂效果 |
| specular 乘数 | 0.4 | 0.7 | 恢复部分高光 |

截图：![参数调优后](../captures/截屏2026-06-29%2016.25.30.png)

### 第二阶段：折射与菲涅尔根本性修正（`fac8b6f`）

参数调优无法解决 X 形折射伪影，定位到着色器逻辑根因。

**折射修正：基于曲率的位移**

旧公式 `offset = normal × thickness × eta` 用厚度决定位移，
在 squircle 四角法线汇聚形成 X 形。改为基于 smoothstep 导数（曲率）：

```wgsl
let t = (clamp(dist, -bevel_width_px, 0.0) / bevel_width_px) + 1.0;
let dz_dt = 6.0 * t * (1.0 - t);
let slope = dz_dt * bevel_depth / bevel_width_px;
let offset = -normal * slope * eta;  // 负号使位移向内（凸透镜聚光）
```

中心和边缘 `dz_dt=0` 无位移，中段曲率最大位移最大，消除 X 形。

**菲涅尔修正：边缘距离渐变**

旧公式基于 smoothstep 导数，在 bevel 中段形成细环。
改为基于 SDF 距离的线性渐变：

```wgsl
let edge_t = 1.0 - clamp(-dist / bevel_width_px, 0.0, 1.0);
let fresnel = schlick_fresnel(edge_t, 0.04) * fresnel_intensity;
```

边缘（`dist=0`）最强，向中心衰减，更接近 Apple 的连续边缘光。

**高光修正：移除区域限制**

移除 `bevel_mask`，让高光自然分布在整个面板上，
降低乘数至 0.3 防止过曝。

截图：![折射修正后](../captures/截屏2026-06-29%2018.55.47.png)

### 第三阶段：TOML 配置文件系统（`1501402`）

每次调参都需要重新编译 Rust，迭代效率低。
引入配置文件系统，支持运行时参数调整。

**实现方案**：
- 格式：TOML（Rust 生态标准，人类可读，支持注释）
- 依赖：`toml 1.1.2` + `serde 1.0.228`
- 配置文件：项目根目录 `config.toml`
- 缺失字段自动使用 `GlassMaterial::default()` / `GlassPanel::default()` 补全

**配置文件结构**：
```toml
[panel]      # 形状：half_size, corner_radius, bevel_width, bevel_depth
[optical]    # 光学：refractive_index, chromatic_strength, fresnel 等
[material]   # 材质：tint_opacity, background_opacity, saturation 等
[[lights]]   # 光源：position_factor（相对窗口比例）, color, intensity
```

**新增文件**：
- `liquidglass0-demo/src/config.rs` — Config 结构体 + load/to_material/to_scene
- `config.toml` — 默认配置

**修改文件**：
- `liquidglass0-demo/Cargo.toml` — 添加 toml + serde 依赖
- `liquidglass0-demo/src/main.rs` — 添加 `mod config`
- `liquidglass0-demo/src/app.rs` — 新增 `glass_config` 字段，加载配置替换硬编码

## 关键决策

- **折射用曲率而非厚度**：`thickness` 在边缘最大但曲率为 0，`slope`（smoothstep 导数）才反映真实表面倾角。Apple 的 Liquid Glass 折射是凸透镜效果，曲率决定位移。
- **菲涅尔用边缘距离而非斜率**：smoothstep 导数在边缘和中心都为 0，只在中段有值。边缘距离渐变更符合 Apple 的连续边缘光效果。
- **配置文件放项目根目录**：简单直接，`cargo run` 的工作目录就是项目根。后续可扩展为支持命令行参数指定路径。
- **不做热重载**：当前迭代频率不高，重新运行 demo 足够。热重载需要文件监听，增加复杂度。

## 验证结果

- `cargo clippy --all-targets` — 0 warnings
- `cargo fmt` — 无 diff
- `cargo run -p liquidglass0-demo` — 窗口正常打开，配置文件正确加载

## 踩坑记录

- **config 模块路径**：`app.rs` 中不能直接用 `config::Config`，需要 `use crate::config;`。Rust 模块系统要求显式声明 crate 根路径。
- **默认值填充逻辑不足 3 个光源时**：不能直接 `*d`（LightConfig 与 Light 类型不同），需要逐字段转换。
- **Apple 参考图分析**：官方 Liquid Glass 的核心特征是中心无变形 + 细微边缘光 + 柔和高光。X 形伪影的根因是折射公式错误，非参数问题。

## 遗留问题

| 问题 | 优先级 | 说明 |
|---|---|---|
| 菲涅尔边缘光仍较弱 | P1 | `fresnel_intensity=2.0` 配合边缘距离渐变，视觉上仍不如 Apple 明显 |
| 高光不可见 | P1 | `specular_total * 0.3` 可能过低，或 2D 法线用于 Blinn-Phong 不准确 |
| 磨砂效果不明显 | P2 | `blur_radius=20` 但 frost_mix 逻辑可能需要调整 |
| 无阴影 | P3 | Phase 3 工作 |
| 无交互变形 | P3 | Phase 4 工作 |
| 背景仍为棋盘格 | — | 可自行更换，非核心问题 |

---

*下一份日志应引用本文件：`2026-06-29_refraction-fix-and-config.md`*
