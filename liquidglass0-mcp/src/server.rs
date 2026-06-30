//! MCP 服务端。
//!
//! 定义 7 个 MCP 工具，使用 rmcp 的 `#[tool_router]` + `#[tool_handler]` 宏。

use std::borrow::Cow;
use std::path::Path;
use std::sync::{Arc, Mutex};

use liquidglass0_core::{GlassPanel, Light, Scene};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::ErrorData as McpError;
use rmcp::model::{CallToolResult, ContentBlock, Implementation, ServerCapabilities, ServerInfo};
use rmcp::{ServerHandler, ServiceExt, tool, tool_handler, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::headless::HeadlessRenderer;

/// config.toml 路径。
const CONFIG_PATH: &str = "config.toml";
/// 着色器目录。
const SHADER_DIR: &str = "shaders";

// ── 错误辅助 ──

fn err_internal(msg: impl Into<Cow<'static, str>>) -> McpError {
    McpError::internal_error(msg, None)
}

fn err_invalid(msg: impl Into<Cow<'static, str>>) -> McpError {
    McpError::invalid_params(msg, None)
}

// ── 参数类型 ──

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct SetParamRequest {
    pub key: String,
    pub value: serde_json::Value,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct SetParamsRequest {
    pub params: serde_json::Value,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ReadShaderRequest {
    pub name: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct UpdateShaderRequest {
    pub name: String,
    pub source: String,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct CaptureRequest {
    pub width: u32,
    pub height: u32,
    pub kind: String,
    pub path: Option<String>,
}

// ── MCP 服务端 ──

/// MCP 服务端状态。
pub struct McpSrv {
    headless: Arc<Mutex<HeadlessRenderer>>,
    tool_router: rmcp::handler::server::router::tool::ToolRouter<Self>,
}

impl Clone for McpSrv {
    fn clone(&self) -> Self {
        Self {
            headless: self.headless.clone(),
            tool_router: Self::tool_router(),
        }
    }
}

impl std::fmt::Debug for McpSrv {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpSrv").finish()
    }
}

impl McpSrv {
    /// 创建服务端实例。
    pub fn new(headless: HeadlessRenderer) -> Self {
        Self {
            headless: Arc::new(Mutex::new(headless)),
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for McpSrv {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::from_build_env())
            .with_instructions("glass 调试服务")
    }
}

// ── 工具实现 ──

#[tool_router(router = tool_router)]
impl McpSrv {
    #[tool(description = "读取 config.toml 的所有参数，返回完整 JSON 对象")]
    async fn get_config(&self) -> Result<CallToolResult, McpError> {
        let content = std::fs::read_to_string(CONFIG_PATH).unwrap_or_default();
        let json = match toml::from_str::<toml::Value>(&content) {
            Ok(doc) => serde_json::to_string_pretty(&doc).unwrap_or_default(),
            Err(_) => "{}".into(),
        };
        Ok(CallToolResult::success(vec![ContentBlock::text(json)]))
    }

    #[tool(
        description = "修改 config.toml 中的一个参数。key 如 optical.refractive_index，数组元素用 key[0]"
    )]
    async fn set_param(
        &self,
        req: Parameters<SetParamRequest>,
    ) -> Result<CallToolResult, McpError> {
        let input = req.0;
        let mut doc = read_config().map_err(err_internal)?;
        let (table_key, field_key, index) = parse_key(&input.key);
        let table = doc
            .get_mut(table_key)
            .ok_or_else(|| err_invalid(format!("未找到 table: {table_key}")))?;

        if let Some(i) = index {
            let arr = table
                .get_mut(field_key)
                .and_then(|v| v.as_array_mut())
                .ok_or_else(|| err_invalid(format!("{field_key} 不是数组")))?;
            let val = input
                .value
                .as_f64()
                .ok_or_else(|| err_invalid("value 需为数字"))?;
            arr.replace(i, toml_edit::Value::from(val));
        } else {
            let val = input
                .value
                .as_f64()
                .ok_or_else(|| err_invalid("value 需为数字"))?;
            table[field_key] = toml_edit::value(val);
        }

        write_config(&doc).map_err(err_internal)?;
        Ok(CallToolResult::success(vec![ContentBlock::text("ok")]))
    }

    #[tool(description = "批量修改 config.toml 参数。params 为键值对对象")]
    async fn set_params(
        &self,
        req: Parameters<SetParamsRequest>,
    ) -> Result<CallToolResult, McpError> {
        let input = req.0;
        let mut doc = read_config().map_err(err_internal)?;
        let params = input
            .params
            .as_object()
            .ok_or_else(|| err_invalid("params 需为对象"))?;

        for (key, value) in params {
            let val = value
                .as_f64()
                .ok_or_else(|| err_invalid(format!("{key} 的值需为数字")))?;
            let (table_key, field_key, index) = parse_key(key);
            let Some(table) = doc.get_mut(table_key) else {
                continue;
            };
            if let Some(i) = index {
                if let Some(arr) = table.get_mut(field_key).and_then(|v| v.as_array_mut()) {
                    arr.replace(i, toml_edit::Value::from(val));
                }
            } else {
                table[field_key] = toml_edit::value(val);
            }
        }

        write_config(&doc).map_err(err_internal)?;
        Ok(CallToolResult::success(vec![ContentBlock::text("ok")]))
    }

    #[tool(description = "将所有参数恢复为默认值")]
    async fn reset_params(&self) -> Result<CallToolResult, McpError> {
        let default_doc = default_config_doc();
        write_config(&default_doc).map_err(err_internal)?;
        let json = serde_json::Value::String(default_doc.to_string());
        Ok(CallToolResult::success(vec![ContentBlock::text(
            serde_json::to_string_pretty(&json).unwrap_or_default(),
        )]))
    }

    #[tool(description = "读取着色器源码。省略 name 时返回可用着色器文件列表")]
    async fn read_shader(
        &self,
        req: Parameters<ReadShaderRequest>,
    ) -> Result<CallToolResult, McpError> {
        let input = req.0;
        match input.name {
            Some(name) => {
                let path = shader_path(&name);
                let source = std::fs::read_to_string(&path)
                    .map_err(|e| err_invalid(format!("读取着色器 {name} 失败: {e}")))?;
                Ok(CallToolResult::success(vec![ContentBlock::text(source)]))
            }
            None => {
                let names = list_shader_names();
                Ok(CallToolResult::success(vec![ContentBlock::text(
                    names.join("\n"),
                )]))
            }
        }
    }

    #[tool(
        description = "修改着色器源码并热更新管线。name 为着色器名，source 为含 #import 的 WGSL 源码"
    )]
    async fn update_shader(
        &self,
        req: Parameters<UpdateShaderRequest>,
    ) -> Result<CallToolResult, McpError> {
        let input = req.0;
        let path = shader_path(&input.name);
        if !path.exists() {
            return Err(err_invalid(format!("着色器不存在: {}", input.name)));
        }
        std::fs::write(&path, &input.source).map_err(|e| err_internal(format!("写入失败: {e}")))?;

        let mut h = self.headless.lock().unwrap();
        h.reload_shader(&input.name, &input.source)
            .map_err(err_internal)?;

        Ok(CallToolResult::success(vec![ContentBlock::text("ok")]))
    }

    #[tool(
        description = "渲染一帧并捕获指定纹理为 PNG。kind: composite/displacement/h_blur/v_blur。path 可选"
    )]
    async fn capture(&self, req: Parameters<CaptureRequest>) -> Result<CallToolResult, McpError> {
        let input = req.0;
        let mut h = self.headless.lock().unwrap();
        let curr = h.size();
        if input.width != curr.0 || input.height != curr.1 {
            h.resize(input.width, input.height);
        }

        let (material, scene) =
            config_to_render(input.width, input.height).map_err(err_internal)?;

        let (result, mime) = h
            .capture(&material, &scene, &input.kind, input.path.as_deref())
            .map_err(err_internal)?;

        if mime == "image/png" {
            Ok(CallToolResult::success(vec![ContentBlock::text(result)]))
        } else {
            Ok(CallToolResult::success(vec![ContentBlock::text(format!(
                "data:{mime},{result}"
            ))]))
        }
    }
}

// ── 辅助函数 ──

fn read_config() -> Result<toml_edit::DocumentMut, String> {
    let content =
        std::fs::read_to_string(CONFIG_PATH).map_err(|e| format!("读取 config.toml 失败: {e}"))?;
    content
        .parse::<toml_edit::DocumentMut>()
        .map_err(|e| format!("解析 config.toml 失败: {e}"))
}

fn write_config(doc: &toml_edit::DocumentMut) -> Result<(), String> {
    std::fs::write(CONFIG_PATH, doc.to_string()).map_err(|e| format!("写入 config.toml 失败: {e}"))
}

fn parse_key(key: &str) -> (&str, &str, Option<usize>) {
    if let Some(bracket) = key.find('[') {
        let field = &key[..bracket];
        let dot = field.rfind('.');
        let (table, field_name) = match dot {
            Some(d) => (&key[..d], &key[d + 1..bracket]),
            None => ("", field),
        };
        let idx_str = &key[bracket + 1..key.len() - 1];
        let idx = idx_str.parse::<usize>().unwrap_or(0);
        (table, field_name, Some(idx))
    } else {
        let dot = key.rfind('.');
        match dot {
            Some(d) => (&key[..d], &key[d + 1..], None),
            None => ("", key, None),
        }
    }
}

fn shader_path(name: &str) -> std::path::PathBuf {
    let dir = match name {
        "blur_horizontal" | "blur_vertical" | "refract" => "compute",
        "composite" => "fragment",
        "fullscreen_triangle" | "sdf" | "glass_material" => "common",
        _ => "",
    };
    Path::new(SHADER_DIR).join(dir).join(format!("{name}.wgsl"))
}

fn list_shader_names() -> Vec<String> {
    vec![
        "blur_horizontal".into(),
        "blur_vertical".into(),
        "refract".into(),
        "composite".into(),
        "fullscreen_triangle".into(),
        "sdf".into(),
        "glass_material".into(),
    ]
}

fn default_config_doc() -> toml_edit::DocumentMut {
    let defaults = r#"
[panel]
half_size = [200.0, 150.0]
corner_radius = 28.0
bevel_width = 0.20
bevel_depth = 55.0
reference_size = 200.0

[optical]
refractive_index = 1.3
chromatic_strength = 0.03
fresnel_intensity = 2.0
fresnel_color = [0.9, 0.95, 1.0]
specular_intensity = 0.4
specular_shininess = 150.0
blur_radius = 20.0

[material]
tint_color = [1.0, 1.0, 1.0]
tint_opacity = 0.08
background_opacity = 0.92
saturation = 1.4
contrast = 1.04
brightness = 0.08

[shadow]
opacity = 0.3
blur = 8.0
offset_y = 4.0

[interaction]
spring_k = 300.0
damping_b = 20.0

[[lights]]
position_factor = [0.2, 0.15]
color = [1.0, 1.0, 1.0]
intensity = 0.9

[[lights]]
position_factor = [0.85, 0.25]
color = [0.95, 0.97, 1.0]
intensity = 0.5

[[lights]]
position_factor = [0.75, 0.8]
color = [1.0, 0.98, 0.95]
intensity = 0.3
"#;
    defaults
        .parse::<toml_edit::DocumentMut>()
        .unwrap_or_else(|_| toml_edit::DocumentMut::new())
}

fn config_to_render(
    width: u32,
    height: u32,
) -> Result<(liquidglass0_core::GlassMaterial, Scene), String> {
    let doc = read_config()?;

    let get_f = |table: &str, key: &str| -> f32 {
        doc.get(table)
            .and_then(|t| t.get(key))
            .and_then(|v| v.as_float())
            .unwrap_or(0.0) as f32
    };

    let get_f2 = |table: &str, key: &str| -> [f32; 2] {
        let arr = doc
            .get(table)
            .and_then(|t| t.get(key))
            .and_then(|v| v.as_array());
        match arr {
            Some(a) => [
                a.get(0).and_then(|v| v.as_float()).unwrap_or(0.0) as f32,
                a.get(1).and_then(|v| v.as_float()).unwrap_or(0.0) as f32,
            ],
            None => [0.0; 2],
        }
    };

    let get_f3 = |table: &str, key: &str| -> [f32; 3] {
        let arr = doc
            .get(table)
            .and_then(|t| t.get(key))
            .and_then(|v| v.as_array());
        match arr {
            Some(a) => [
                a.get(0).and_then(|v| v.as_float()).unwrap_or(0.0) as f32,
                a.get(1).and_then(|v| v.as_float()).unwrap_or(0.0) as f32,
                a.get(2).and_then(|v| v.as_float()).unwrap_or(0.0) as f32,
            ],
            None => [0.0; 3],
        }
    };

    use glam::{Vec2, Vec3};

    let material = liquidglass0_core::GlassMaterial {
        refractive_index: get_f("optical", "refractive_index"),
        chromatic_strength: get_f("optical", "chromatic_strength"),
        fresnel_intensity: get_f("optical", "fresnel_intensity"),
        fresnel_color: Vec3::from_array(get_f3("optical", "fresnel_color")),
        specular_intensity: get_f("optical", "specular_intensity"),
        specular_shininess: get_f("optical", "specular_shininess"),
        blur_radius: get_f("optical", "blur_radius"),
        tint_color: Vec3::from_array(get_f3("material", "tint_color")),
        tint_opacity: get_f("material", "tint_opacity"),
        background_opacity: get_f("material", "background_opacity"),
        saturation: get_f("material", "saturation"),
        contrast: get_f("material", "contrast"),
        brightness: get_f("material", "brightness"),
        shadow_opacity: get_f("shadow", "opacity"),
        shadow_blur: get_f("shadow", "blur"),
        shadow_offset_y: get_f("shadow", "offset_y"),
        deformation_spring_k: get_f("interaction", "spring_k"),
        deformation_damping_b: get_f("interaction", "damping_b"),
    };

    let half_size = get_f2("panel", "half_size");
    let panel = GlassPanel {
        center: Vec2::new(width as f32 / 2.0, height as f32 / 2.0),
        half_size: Vec2::new(half_size[0], half_size[1]),
        corner_radius: get_f("panel", "corner_radius"),
        bevel_width: get_f("panel", "bevel_width"),
        bevel_depth: get_f("panel", "bevel_depth"),
        reference_size: get_f("panel", "reference_size"),
    };

    let lights_arr = doc.get("lights").and_then(|v| v.as_array_of_tables());
    let mut lights = [Light::default(); 3];
    if let Some(arr) = lights_arr {
        for (i, tbl) in arr.iter().take(3).enumerate() {
            let pf = tbl
                .get("position_factor")
                .and_then(|v| v.as_array())
                .map(|a| {
                    [
                        a.get(0).and_then(|v| v.as_float()).unwrap_or(0.5) as f32,
                        a.get(1).and_then(|v| v.as_float()).unwrap_or(0.5) as f32,
                    ]
                })
                .unwrap_or([0.5, 0.5]);
            let col = tbl
                .get("color")
                .and_then(|v| v.as_array())
                .map(|a| {
                    [
                        a.get(0).and_then(|v| v.as_float()).unwrap_or(1.0) as f32,
                        a.get(1).and_then(|v| v.as_float()).unwrap_or(1.0) as f32,
                        a.get(2).and_then(|v| v.as_float()).unwrap_or(1.0) as f32,
                    ]
                })
                .unwrap_or([1.0; 3]);
            let intensity = tbl
                .get("intensity")
                .and_then(|v| v.as_float())
                .unwrap_or(0.8) as f32;
            lights[i] = Light {
                position: Vec2::new(width as f32 * pf[0], height as f32 * pf[1]),
                color: Vec3::from_array(col),
                intensity,
            };
        }
    }

    let scene = Scene { panel, lights };
    Ok((material, scene))
}

/// 启动 MCP server（stdio transport）。
pub async fn serve(headless: HeadlessRenderer) {
    let app = McpSrv::new(headless);
    let transport = rmcp::transport::io::stdio();
    let running = app
        .serve(transport)
        .await
        .expect("MCP server 启动失败");
    running.waiting().await.ok();
}
