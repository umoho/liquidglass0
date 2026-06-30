---
生成时间:   2026-07-01 03:13:10 +0800
上一份日志: 2026-07-01_mcp-design.md
关联的提交: 2b2b500, cd42f59, a19b41a, 2be2add, 03f9689, 5c9e02b
---

# 2026-07-01 — MCP 实现与集成

## 工作内容

完成 `liquidglass0-mcp` 的 Rust 实现，修复 3 个运行时 bug，配置 OpenCode 集成并通过 Python 脚本完成端到端测试。

## 实现

### render crate 改动

- 中间纹理 usage 加 `COPY_SRC`，公开 `IntermediateTextures` 句柄
- 新增 `reload_shader()` — 接收 WGSL 源码，naga_oil 解析 `#import` 后重建对应管线。缓存 5 个 `ShaderModule` 句柄和 `Composer`
- `current_size()` 公开当前纹理尺寸

### mcp crate（7 个 MCP 工具）

| 工具 | 实现方式 |
|---|---|
| `get_config` | `toml::from_str` → JSON |
| `set_param` / `set_params` | `toml_edit` in-place 修改 `config.toml`，支持 `key[0]` 数组语法 |
| `reset_params` | 写入硬编码默认 TOML |
| `read_shader` / `update_shader` | 文件级读写 `shaders/*.wgsl`；`update_shader` 调用 `GlassRenderer::reload_shader` 热更新 |
| `capture` | headless 渲染 → `copy_texture_to_buffer` → `map_async` → PNG（`composite` 用 Rgba8Unorm 直出，`displacement` 用半精度浮点解码为方向热力图） |

### HeadlessRenderer

- `compatible_surface: None` 请求 headless adapter
- 复用 demo 的棋盘格背景生成逻辑
- 输出纹理格式对齐管线默认 `Rgba8UnormSrgb`
- 支持 `capture` 写入文件或返回 base64

## 踩坑记录

### [1] RunningService 被提前 drop

`rmcp::ServiceExt::serve()` 返回 `RunningService` 即结束——后台任务在 `RunningService` 被 drop 时取消。修复：调用 `running.waiting().await` 阻塞直到服务退出。症状：进程收到 initialize 后立即 exit(0)。

### [2] 管线格式与输出纹理不匹配

`RendererConfig::default()` 默认 `Rgba8UnormSrgb`，但 headless 输出纹理写成 `Rgba8Unorm`，导致 `set_pipeline` 校验失败。修复：存储 `output_format` 字段，`new()` / `resize()` 统一使用。

### [3] wgpu 29 API 迁移

- `Maintain::Wait` → `PollType::Wait { submission_index, timeout }`
- `ImageCopyTexture` → `TexelCopyTextureInfo`
- `ImageCopyBuffer` + `ImageDataLayout` → `TexelCopyBufferInfo` + `TexelCopyBufferLayout`
- `request_device()` 不再接受 trace 参数

### [4] rmcp 2.0 API

- `Service` 替代旧版 `Server`；`serve_server()` → `ServiceExt::serve()`
- `#[tool_router]` + `#[tool_handler]` 替代单一 `#[tool]` 宏
- `CallToolResult::success(Vec<ContentBlock>)` 而非 `Vec<Content>`
- `ServerInfo` 为非穷举结构，需用 `ServerInfo::new(capabilities).with_*()`
- `ErrorData::internal_error()` / `invalid_params()` 需第二个参数 `Option<serde_json::Value>`

### [5] OpenCode MCP 连接失败

Python 脚本验证 MCP server 正常工作（7 工具、capture 产出 PNG、热更新成功），但 OpenCode 报 `Failed to get tools`。日志无 spawn 记录。已尝试：`timeout: 30000`、`bash` 显式启动、`enabled: true`、`cwd: "."`——均未解决。根因待查。

## 验证结果

```
capture(composite)        ✅ 9827 字节 PNG 文件
capture(displacement)     ✅ 1912 字节热力图 PNG
set_param + re-capture    ✅ 修改折射率 → 重渲染成功
read_shader(composite)    ✅ 6831 字符
update_shader (热更新)    ✅ 无重编译
reset_params              ✅ 恢复默认
cargo clippy --all-targets ✅ 零 warning
```

---

*下一份日志应引用本文件：`2026-07-01_mcp-implementation.md`*
