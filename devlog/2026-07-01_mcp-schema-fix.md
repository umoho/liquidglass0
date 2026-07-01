---
生成时间:   2026-07-01 15:16:53 +0800
上一份日志: 2026-07-01_mcp-implementation.md
关联的提交: c723c3e
---

# 2026-07-01 — MCP Schema 兼容修复

## 工作内容

修复 MCP server 在 OpenCode 中 "Failed to get tools" 的问题。

## 根因

`rmcp` + `schemars` 生成的 `tools/list` 响应中，`serde_json::Value` 类型字段被序列化为 JSON Schema 布尔 `true`。TypeScript MCP SDK (`@modelcontextprotocol/sdk` v1.29.0) 的 Zod v4 校验拒绝布尔值作为 property schema，只接受对象形式，导致 `listTools` 返回 ZodError：

```
path ["tools", 4, "inputSchema", "properties", "value"] - "Invalid input"
path ["tools", 5, "inputSchema", "properties", "params"] - "Invalid input"
```

## 解决方案

新增 `JsonValue` 包装类型，覆写 `schemars::JsonSchema` 返回空对象 schema `{}`，而非布尔 `true`。JSON Schema 中 `{}` 和 `true` 语义等价，但 Zod v4 接受前者。

修改范围：
- `SetParamRequest.value`: `serde_json::Value` → `JsonValue`
- `SetParamsRequest.params`: `serde_json::Value` → `JsonValue`
- 三处 `.0` 解包：`set_param` ×2、`set_params` ×1

## 验证结果

- `cargo clippy --all-targets`：通过
- `cargo fmt --check`：通过
- TypeScript SDK `listTools`：7 工具全部成功
- TypeScript SDK `callTool`：`get_config`、`read_shader` 成功
- `opencode mcp list`：`connected`（修复前 `failed`）

## 踩坑记录

- `schemars` 1.2.1 的 `schema` 模块和 `gen` 模块均为私有，`JsonSchema` trait 使用 `schemars::Schema`（公开 re-export）和 `schemars::SchemaGenerator`
- `Schema::from(serde_json::Value)` 不存在，需用 `Schema::from(serde_json::Map::new())` 构造空对象 schema
- 修复后 `schemars` 自动生成 `$defs`/`$ref` 结构（未内联），TypeScript SDK 完全支持

---

*下一份日志应引用本文件：`2026-07-01_mcp-schema-fix.md`*
