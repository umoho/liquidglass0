---
生成时间:   2026-06-29 12:00:25 +0800
上一份日志: 2026-06-29_type-system-refactoring.md
关联的提交: 7e32104
---

# 2026-06-29 — 着色器基础设施（naga_oil）

## 工作内容

Phase 2 Part 2：引入 naga_oil 实现着色器模块化，创建公共着色器模块。

### 依赖

- 添加 `naga_oil = "0.22"`（naga 29，兼容 wgpu 29）
- wgpu 启用 `naga-ir` feature，支持 `ShaderSource::Naga` 变体

### 公共着色器模块

- `glass_material.wgsl`：
  - `#define_import_path glass_material`
  - `GlassUniforms` 结构体，12 × vec4f = 192 bytes
  - 打包 panel_info、shape_params、光学参数、材质参数、光源信息
- `sdf.wgsl`：
  - `#define_import_path sdf`
  - `squircle_sdf`：超椭圆 SDF
  - `sdf_normal`：有限差分求法线
  - `bevel_z`：斜面轮廓 Z 位移
  - 从 Rust `sdf.rs` 移植，算法一致

### NagaOilLoader

- 内部持有 `RefCell<Composer>`
- 构造时注册三个公共模块（sdf, glass_material, fullscreen_triangle）
- `load()` 方法：调用 `make_naga_module()` 解析 `#import`，返回 `ShaderSource::Naga(Cow::Owned(module))`

### ShaderLoader trait 重构

- `load_wgsl(&self, name) -> String` → `load(&self, name) -> ShaderSource<'static>`
- 直接返回 `ShaderSource`，不再强制 WGSL 字符串
- `EmbeddedLoader` 返回 `ShaderSource::Wgsl(Cow::Owned(...))`
- `NagaOilLoader` 返回 `ShaderSource::Naga(Cow::Owned(...))`
- `renderer.rs` 简化：去掉 `.into()` 和 `ShaderSource::Wgsl(...)` 包装

## 关键决策

- **RefCell 解决 &mut 问题**：`make_naga_module()` 需要 `&mut self`，但 `ShaderLoader::load()` 是 `&self`。用 `RefCell<Composer>` 实现内部可变性。
- **Composer::non_validating()**：naga_oil 0.22 没有 `new()` 方法，只有 `non_validating()`。
- **ShaderSource 改造**：改为返回 `ShaderSource<'static>` 而非 `String`，让 NagaOilLoader 能直接传递 naga Module，避免 WGSL→IR→WGSL 往返。
- **naga-ir feature**：wgpu 29 的 `ShaderSource::Naga` 变体需要启用此 feature。

## 验证结果

- `cargo clippy --all-targets` — 0 warnings
- `cargo fmt` — 无 diff
- 编译通过，无错误

## 踩坑记录

- wgpu 29 的 `ShaderSource::Naga` 藏在 `naga-ir` feature 后面，不启用会报 `no variant named Naga`。
- naga_oil 的 `Composer::new()` 不存在，文档和 README 写的 `new()` 实际对应 `non_validating()`。
- `make_naga_module()` 需要 `&mut self`，与 `ShaderLoader` 的 `&self` 签名冲突，用 RefCell 解决。

---

*下一份日志应引用本文件：`2026-06-29_shader-infrastructure-naga-oil.md`*
