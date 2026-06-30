//! liquidglass0 MCP 调试服务。
//!
//! 启动 headless 渲染上下文，注册 7 个 MCP 工具，
//! 通过 stdio 与 AI agent 通信。

mod capture;
mod headless;
mod server;

#[tokio::main]
async fn main() {
    let headless = headless::HeadlessRenderer::new(512, 512).await;
    server::serve(headless).await;
}
