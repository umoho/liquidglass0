---
生成时间:   2026-06-29 12:46:10 +0800
上一份日志: 2026-06-29_shader-infrastructure-naga-oil.md
关联的提交: 11b9632
---

# 2026-06-29 — Phase 2 材质效果实现

## 工作内容

Phase 2 Part 3：实现折射、色散、磨砂、菲涅尔、镜面高光五大材质效果。

### 折射 compute shader（refract.wgsl）

- 基于 SDF 法线计算每个像素的背景采样偏移量
- 偏移公式：`offset = normal × thickness × (1 - 1/n)`
- 输出 displacement_texture（Rgba16Float），供 composite 使用
- 工作组 (16, 16)

### 合成 fragment shader（composite.wgsl）

完整 7 步合成流程：
1. SDF 判断玻璃区域，区域外直接输出 background
2. 采样 displacement → 折射偏移
3. RGB 分离采样 → 色散（R×0.98, G×1.00, B×1.02）
4. 采样模糊纹理，按厚度（bevel_z）混合 → 磨砂感
5. Schlick 菲涅尔 → 边缘发光
6. Blinn-Phong 多光源 → 镜面高光
7. 色调 + 亮度/饱和度/对比度调整

### 渲染器（renderer.rs）

- 新增 `GlassUniforms` 结构体（192 bytes = 12 × vec4f）
  - 打包 panel_info、shape_params、光学参数、材质参数、光源信息
  - `from_input()` 方法从 RenderInput + GlassPanel 构造
- 新增 displacement_texture（Rgba16Float）
- 新增 glass_uniform_buf
- 新增 refract compute pipeline + bind group
- 更新 composite bind group：5 个 binding（background + blur + displacement + sampler + uniform）
- 管线流程：refract → blur_h → blur_v → composite

### 配置（config.rs）

- 新增 refract_workgroup_width/height（默认 16）

### Demo（app.rs）

- 切换到 NagaOilLoader
- 面板居中（200×150）
- 3 个默认光源（不同位置和颜色）

## 关键决策

- **磨砂联动采用合成阶段混合（方案 A）**：固定半径模糊整张背景，然后在 composite shader 中按 bevel_z 混合清晰/模糊。这是最简单的实现方式。Apple 的 Liquid Glass 可能采用了更精确的逐像素可变半径模糊（方案 B），效果更好但实现复杂度高。Phase 2 先用方案 A 验证管线，后续可升级。
- **位移纹理格式选择 Rgba16Float**：Rg16Float 足够（只需 XY 偏移），但 Rgba16Float 兼容性更好，额外通道开销可忽略。
- **composite bind group 每帧重建**：因为 background 纹理可能每帧变化，composite bind group 在 render() 中每帧创建。

## 验证结果

- `cargo clippy --all-targets` — 0 warnings
- `cargo fmt` — 无 diff
- 编译通过，无错误

## 踩坑记录

- demo crate 缺少 glam 依赖，导致 `glam::Vec2/Vec3` 无法使用。添加 `glam = { workspace = true }` 解决。
- renderer.rs 中 `GlassUniforms::from_input()` 需要访问 `input.scene.lights`，而 lights 是 `[Light; 3]`，直接用索引访问即可。

---

*下一份日志应引用本文件：`2026-06-29_phase2-material-effects.md`*
