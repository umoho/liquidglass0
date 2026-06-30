---
生成时间:   2026-07-01 02:07:05 +0800
上一份日志: 2026-06-30_phase5-scroll-debug-and-fix.md
关联的提交: a045b01
---

# 2026-07-01 — MCP 调试方案设计

## 工作内容

完成 `MCP_DESIGN.md` 设计文档（7 轮迭代审阅）。定义新 workspace member `liquidglass0-mcp` 的架构、render crate 对外接口、7 个 MCP 工具及完整数据流。

## 关键决策

### 为什么暴露 GPU 句柄而不是内嵌 dump 方法

三种方案对比：A) dump 方法内嵌在 GlassRenderer；B) Observer trait callback；C) 对外暴露中间纹理引用。

选 C。render crate 不该关心读回后的格式（PNG? 热力图? diff?），那是 mcp crate 的事。新增导出格式只改 mcp crate，不动 render——满足开闭原则。任意消费者（mcp、未来 profiler、测试套件）都可按自己的方式消费同一份句柄。

### 为什么 render 和 dump_intermediate 合并为 capture

内部流程完全一致——渲染一帧 → copy_texture_to_buffer → map_async → 解码 → PNG——唯一差异是读哪个纹理。合并为一个工具，`kind` 参数区分目标。

### 为什么中间纹理只暴露 3 个

当前管线 4 个 pass 只产 3 个中间纹理，每个对应可被隔离验证的失败模式：displacement（折射偏移）、h_blur（水平高斯核）、v_blur（最终模糊形态）。管线加新 pass 时加字段即可，目前不设计未来不需要的抽象。

### 为什么 build / clippy 不纳入 MCP

标准 shell 操作，AI agent 直接跑 `cargo build` / `cargo clippy` 即可。MCP 代理对此没有额外能力。

### 为什么不用条件编译 gate COPY_SRC

COPY_SRC 是一个 bit flag，零运行时开销。加 `#[cfg(feature = "dump")]` 引入额外编译变体，对不发布到 crates.io 的内部调试项目而言得不偿失。

### 为什么设计文档不含代码块和格式细节

开闭原则延伸至文档层面。`Rgba8Unorm`、`Rgba16Float`、Rust 方法签名都是实现细节，变化了不影响设计意图。设计文档只描述"是什么"的抽象，代码和格式正确性由编译器保证。

### 为什么新 workspace member 而非绑在 demo

MCP 的 stdio transport 与 winit 事件循环互不干扰。demo 已有 `--tune` (GPUI) 和默认 (winit) 两种模式，再加第三种会让 main.rs 的模式分发不可维护。

### 为什么参数用 key[0] 语法而非 .r/.g

`optical.fresnel_color[0]` 直接映射 TOML 数组索引，与 `toml_edit` 操作语义一致。`.r`/`.g` 需要在 MCP server 维护 key→index 映射表，增加维护负担。

### capture 为什么支持文件路径

大尺寸渲染不必经 MCP stdio 传输（性能瓶颈），文件持久化也便于人类事后翻看。path 为可选参数，不提供时回退 base64。

## 踩坑记录

- 设计文档初稿包含过多实现细节：Rust 代码块、纹理格式标注、依赖列表、macOS 平台特化。经 7 轮审阅逐步剔除，保持与 ARCHITECTURE.md 一致的抽象级别。
- 最初列了约 10 个 MCP 工具，后精简至 7 个：合并 render+dump_intermediate 为 capture，移除 build/clippy/reset_params→reset_config 改名，read_shader 合并 list_shaders 功能。

---

*下一份日志应引用本文件：`2026-07-01_mcp-design.md`*
