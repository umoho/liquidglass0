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

### 规则

- 写 dev log 前，必须使用 `date` 等只读命令获取当前系统时间，不得虚构或臆测
- 除非用户明确要求，不得修改已有的 dev log 文件

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

## CLI 调试工具

构建：`cargo build -p liquidglass0-cli --release`

所有命令输出 stdout JSON，错误输出 stderr JSON 并退出码 1。

### config — 读写参数

| 命令 | 说明 |
|------|------|
| `lg0 config get` | 读取全部参数，输出 JSON |
| `lg0 config set <K> <V>` | 修改单个参数 |
| `lg0 config set --batch` | stdin JSON 批量修改 |
| `lg0 config reset` | 恢复默认值 |

参数 key 约定：
- 标量：`optical.refractive_index`
- 数组元素：`optical.fresnel_color[0]`
- 批量 JSON：`{"optical.refractive_index": 1.33, "material.saturation": 1.5}`

### shader — 读写着色器

| 命令 | 说明 |
|------|------|
| `lg0 shader list` | 列出所有着色器名称 |
| `lg0 shader read <NAME>` | 读取源码 |
| `lg0 shader write <NAME>` | stdin 写入源码（不触发编译） |
| `lg0 shader flush <NAME>` | 从磁盘重新编译管线 |

### capture — 渲染捕获

| 命令 | 说明 |
|------|------|
| `lg0 capture <W> <H> <KIND> [--output <PATH>]` | 渲染并捕获 PNG。无 --output 写入临时目录 |

kind 取值：

| kind | 纹理 |
|------|------|
| composite | 最终合成帧 |
| displacement | 折射位移 |
| h_blur | 水平模糊 |
| v_blur | 垂直模糊 |

### 调试工作流

调参循环：

```bash
lg0 capture 512 512 composite --output /tmp/base.png  # 基线
lg0 config set optical.refractive_index 1.33
lg0 config set optical.fresnel_color[0] 0.95
lg0 config set --batch << 'JSON'
{"material.saturation": 1.5, "shadow.opacity": 0.25}
JSON
lg0 capture 512 512 composite --output /tmp/v1.png   # 对比
```

着色器调试：

```bash
lg0 shader read refract                              # 阅读当前源码
lg0 shader write refract << 'WGSL'
...修改后的 WGSL...
WGSL
lg0 shader flush refract                              # 重编译管线
lg0 capture 512 512 composite --output /tmp/v2.png    # 观察效果
```

中间纹理排查：

```bash
lg0 capture 512 512 displacement --output /tmp/disp.png
lg0 capture 512 512 h_blur --output /tmp/blur.png
```
