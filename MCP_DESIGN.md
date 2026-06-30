# MCP 调试方案设计

## 目标

AI agent 自主调试玻璃渲染管线：修改参数和着色器 → 触发离屏渲染 → 观察最终帧与中间纹理 → 迭代。

## 架构

```
liquidglass0/
├── ...
├── liquidglass0-render/     # 对外暴露中间纹理句柄
└── liquidglass0-mcp/        # MCP server + headless 渲染
```

`liquidglass0-mcp` 依赖 `liquidglass0-render`，不重复渲染逻辑。所有纹理读回、PNG 编码、文件操作在此 crate。

## render crate 对外接口

### 中间纹理句柄

`GlassRenderer` 公开 `IntermediateTextures`，含三个字段：

- `displacement` — 折射 UV 偏移
- `h_blur` — 水平模糊
- `v_blur` — 垂直模糊

usage 含 `COPY_SRC`，支持 `copy_texture_to_buffer` 读回 CPU。

### 着色器重载

`reload_shader` 接收 WGSL 源码，解析 `#import` 并重建对应管线。

## MCP 工具

| 工具 | 输入 | 输出 |
|---|---|---|
| `get_config` | — | 完整 config 的 JSON 对象 |
| `set_param` | `key: str`, `value: f64` | `"ok"` |
| `set_params` | `params: object` | `"ok"` |
| `reset_params` | — | 默认值 JSON |
| `read_shader` | `name: str?` | 源码；省略时返回文件列表 |
| `update_shader` | `name: str`, `source: str` | `"ok"`，写文件并热更新 |
| `capture` | `width: u32`, `height: u32`, `kind: str`, `path?: str` | `path` 提供时写入文件返回路径；否则返回 base64 PNG |

### 参数 key 约定

- 标量：`"optical.refractive_index"`
- 数组元素：`"optical.fresnel_color[0]"`、`"panel.half_size[0]"`

### capture 的 kind 取值

| kind | 读回的纹理 |
|---|---|
| `"composite"` | 最终合成帧 |
| `"displacement"` | 折射位移 |
| `"h_blur"` | 水平模糊 |
| `"v_blur"` | 垂直模糊 |

## 数据流

```
AI agent
  │  MCP JSON-RPC (stdio)
  ▼
liquidglass0-mcp
  │
  ├─ get/set/reset params ──── 读写 config.toml
  ├─ read/update shader ───── 读写 shaders/*.wgsl + GlassRenderer::reload_shader
  │
  └─ capture(width, height, kind, path?)
        │
     HeadlessRenderer
        ├─ 读 config.toml → GlassMaterial + Scene → RenderInput
        ├─ GlassRenderer::render(encoder, input, &output_view)
        ├─ queue.submit()
        │
        └─ 按 kind 选目标
            ├─ composite    → output_tex
            └─ displacement/h_blur/v_blur → intermediate_textures()
              │
              ├─ copy_texture_to_buffer → staging buffer
              ├─ map_async → 解码 → PNG
              └─ path? 存在则写文件，否则 base64
```

