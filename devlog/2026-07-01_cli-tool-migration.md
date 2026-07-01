---
生成时间:   2026-07-01 22:07:43 +0800
上一份日志: 2026-07-01_mcp-schema-fix.md
关联的提交: a75a01a, c83f242
---

# 2026-07-01 — MCP 到 CLI 迁移

## 工作内容

将 `liquidglass0-mcp` crate 替换为 `liquidglass0-cli` crate，从 MCP server（JSON-RPC stdio）改为与 agent 无关的 CLI 工具（bash arg/stdin/stdout）。

- 删除 `liquidglass0-mcp`、`opencode.json`、`run-mcp.sh`
- 新建 `liquidglass0-cli`（二进制 `lg0`）
- 依赖变化：–rmcp –tokio –schemars，+clap 4.6.1 +pollster 0.4.0
- 9 个子命令：`config get/set/set --batch/reset`、`shader list/read/write/flush`、`capture`
- 删除 `MCP_DESIGN.md`，把所有 CLI 用法、key 约定、kind 取值表、REPL 工作流示例存入 AGENTS.md

## 关键决策

### 为什么弃 MCP、选 CLI

受 [What if you don't need MCP at all?](https://mariozechner.at/posts/2025-11-02-what-if-you-dont-need-mcp/) 启发：

- CLI 工具对于 agent 就是 bash 命令，不需要专门的 MCP 注册配置（`opencode.json`）或启动脚本
- 输出是纯 stdout JSON，可组合到管道、文件、shell 脚本中；MCP 需要 agent 在 context 里中转所有数据
- Token 开销：MCP 每个工具带 description + schema，7 工具消耗大量 context；CLI 只有 `--help` 一行，agent 按需查阅 AGENTS.md
- 扩展性：shader write/flush 分离、set --batch 等不需要改协议，就是加个 clap 子命令
- OpenCode 不支持 MCP，而 CLI 不依赖任何 agent 框架

### 子命令层次

`config` / `shader` / `capture` 三个顶层子命令，各自嵌套：

- `config set --batch`：替代 `set_params`，通过 heredoc/pipeline 传 JSON，与 `update_shader` stdin 风格统一
- `shader write` + `shader flush`：拆开写盘和编译两步，支持连续改多个着色器再逐一 flush
- `config reset`：不再返回默认值 JSON（无意义），改用 `config get` 验证
- `capture` 不再返回 base64，强制写文件（`--output` 或临时目录），避免二进制数据与 JSON 混杂

### 同步化 HeadlessRenderer

`HeadlessRenderer::new()` 原本是 async，依赖 tokio。改用 `pollster::block_on` 阻塞初始化，`main()` 变成普通 `fn main()`。pollster 0.4.0 比 tokio 轻量得多（0 间接依赖），对短生命周期 CLI 进程完全够用。

### 文档策略

丢弃独立设计文档 `MCP_DESIGN.md`，把所有 agent 需要的信息写入 AGENTS.md：子命令速查表、key 约定、kind 取值、完整的调参和着色器调试图示。

## 验证结果

- `cargo check -p liquidglass0-cli`：通过
- `cargo clippy --all-targets`：通过（无 warning）
- `cargo fmt --check`：通过
- 手动测试 9 个子命令全部通过：

| 命令 | 测试 |
|------|------|
| `lg0 config get` | 输出完整 JSON，含 6 个 table |
| `lg0 config set optical.refractive_index 1.33` | `{"ok": true}` |
| `lg0 config set --batch` | stdin JSON 批量，生效 |
| `lg0 config reset` | 恢复 1.3 |
| `lg0 shader list` | 输出 7 个着色器名 |
| `lg0 shader read blur_horizontal` | 返回 1180 字符源码 |
| `lg0 shader write blur_horizontal` | stdin 写盘成功 |
| `lg0 shader flush blur_horizontal` | 重编译成功 |
| `lg0 capture 512 512 composite --output /tmp/test.png` | 输出 PNG 路径 |
| `lg0 capture 256 256 displacement` | 自动写入临时目录 |

- 错误处理验证：缺少参数 exit 1、stderr `{"ok": false, "error": "..."}`
- 子 agent 自举测试：不给任何 `lg0` 用法提示，agent 仅凭 AGENTS.md 完成读取 config、列着色器、修改 refractive_index、捕获 3 种纹理（composite/displacement/h_blur）并分析结果

## 踩坑记录

- `clap` 4.6.1 的 `conflicts_with_all` 对 optional positional args 工作正常，`--batch` 与 `key`/`value` 互斥
- `capture` 无 `--output` 时的 temp path 需要拥有所有权（`owned_path` 变量）才能安全传给 `Option<&str>`，避免 borrow checker 问题
- `toml_edit` 保持 0.23 未升级（0.25.12 存在但用户决定不动），所有 API `value()`、`get_mut`、`as_array_mut` 均正常

---

*下一份日志应引用本文件：`2026-07-01_cli-tool-migration.md`*
