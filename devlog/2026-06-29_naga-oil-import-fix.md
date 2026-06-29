---
生成时间:   2026-06-29 13:29:17 +0800
上一份日志: 2026-06-29_phase2-material-effects.md
关联的提交: 118bc58
---

# 2026-06-29 — naga_oil 导入修复

## 工作内容

修复 naga_oil 着色器导入失败的问题，demo 成功运行。

### 问题

运行 demo 时 panic：
```
编译着色器 refract 失败: Composer error: no definition in scope for identifier: GlassUniforms
```

`#import glass_material` 后直接使用 `GlassUniforms`，naga_oil 无法解析。

### 根因

naga_oil 要求导入的项必须使用模块名作为命名空间前缀。

从 naga_oil 源码确认（`parse_imports.rs` 的测试用例、`compose/mod.rs` 顶部文档注释第 25-36 行）：
- `#import my_module` 后，必须用 `my_module::item` 访问
- `#import my_module as Alias` 后，用 `Alias::item` 访问
- 这与 Bevy 的惯例一致（所有示例均使用 `#import` + `::` 前缀）

### 修复

- `refract.wgsl`：`GlassUniforms` → `glass_material::GlassUniforms`，`squircle_sdf` → `sdf::squircle_sdf` 等
- `composite.wgsl`：同上
- `shader.rs`：移除之前尝试的 `additional_imports` hack，恢复 `..Default::default()`
- `sdf.wgsl`：恢复调试时删除的注释

### 调试过程中尝试过的方法

1. 添加/移除 `#import` 后的分号 → 无效
2. 使用 `Composer::default()` 替代 `non_validating()` → 无效
3. 移除着色器文件中的注释 → 无效
4. 使用 `additional_imports` 参数显式指定导入 → 无效
5. 从公共模块注册中移除无 `#define_import_path` 的模块 → 修复了第一个 panic
6. **使用模块前缀访问导入项** → 最终修复

## 关键决策

- **不简写模块名**：不使用 `#import glass_material as GM`，保持 `glass_material::GlassUniforms` 完整前缀，与 Bevy 生态惯例一致。
- **不使用 `#from` 导入**：`#from module import item` 语法虽然存在，但 Bevy 示例中未使用，保持一致性。

## 验证结果

- `cargo clippy --all-targets` — 0 warnings
- `cargo fmt` — 无 diff
- `cargo run -p liquidglass0-demo` — 窗口正常打开，无 panic

## 踩坑记录

- naga_oil 的 `#import` 不是传统意义上的"导入到当前作用域"，而是"注册为可用模块"。访问时必须用 `模块名::` 前缀。
- `Composer::non_validating()` 和 `Composer::default()` 在导入解析上行为一致，区别只在错误报告质量。
- naga_oil 的 `#define_import_path` 支持简单名称（如 `glass_material`）和命名空间形式（如 `bevy_pbr::pbr_types`）。

---

*下一份日志应引用本文件：`2026-06-29_naga-oil-import-fix.md`*
