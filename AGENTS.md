# AGENTS.md — liquidglass0 工作流程

## 提交前检查

但凡修改了 Rust 代码，提交前必须执行：

```bash
cargo clippy --all-targets
cargo fmt
```

- `cargo clippy`：检查代码质量，无 `warning` 才能提交
- `cargo fmt`：格式化代码，确保风格一致

## 提交风格

使用 Conventional Commits，**提交消息必须使用英文**：

```
<type>(<scope>): <description>

- bullet points describing specific changes
```

### type

| type | 用途 |
|---|---|
| `feat` | 新功能 |
| `fix` | 修 bug |
| `docs` | 文档 |
| `chore` | 构建、依赖、重构、清理 |

### scope

scope 标注改动所属的 crate 或关注点：

| scope | 指向 |
|---|---|
| `shader` | 着色器文件 |
| `core` | liquidglass0-core |
| `render` | liquidglass0-render（Rust 代码） |
| `demo` | liquidglass0-demo |

示例：

```
feat(shader): add separable gaussian blur compute pass
feat(render): wire up blur pipeline with input texture
fix(demo): resize surface on window resize
docs: add LIQUID_GLASS.md effect definition
```

## 代码规范

注释仅使用汉语，无须对照英文。

### Mod（模块）

```rust
//! 模块描述。
//!
//! 详细描述（可选）
//!
//! # 子模块
//!
//! - [`mod1`] - 说明
//!
//! # 主要类型
//!
//! - [`Type1`] - 说明
```

### Struct

```rust
/// 结构体描述。
///
/// 详细描述（可选）
pub struct Example {
    /// 字段说明
    pub field: Type,
}
```

### Enum

```rust
/// 枚举描述。
pub enum Example {
    /// 变体说明
    Variant1,

    /// 变体说明
    Variant2 {
        /// 字段说明
        field: Type,
    },
}
```

### Fn（函数/方法）

```rust
/// 简短描述。
///
/// 详细描述（可选）
///
/// # 参数
///
/// * `param` - 说明（单位：xxx，范围：xxx ~ xxx）
///
/// # 返回值
///
/// 说明
///
/// # Panics
///
/// 可能 panic 的条件
pub fn example(param: Type) -> ReturnType {}
```

### Const（常量）

```rust
/// 说明。
const NAME: Type = value;
```

## Dev Log

### 工作流

1. 完成代码工作后先提交（获取 commit hash）
2. 在 `devlog/` 目录下创建日志文件
3. 提交 dev log

### 文件命名

```
devlog/YYYY-MM-DD_slug-kebab-case.md
```

- 日期和 slug 之间用下划线分隔
- slug 使用 kebab-case

### 模板

```markdown
---
生成时间:   2026-06-29 14:30:00 +0800
上一份日志: 2026-06-28_slug.md
关联的提交: abc1234, def5678
---

# 2026-06-29 — slug

## 工作内容

简述本期做了什么。

## 关键决策

- 决策原因
- 替代方案及未选原因

## 验证结果

- 使用的命令和输出

## 踩坑记录

- 遇到的问题和解决方式

---

*下一份日志应引用本文件：`2026-06-29_slug.md`*
```
