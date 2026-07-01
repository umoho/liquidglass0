//! CLI 子命令定义与实现。
//!
//! 提供 config / shader / capture 三级子命令，
//! 所有输出为 stdout JSON，错误走 stderr 并 exit(1)。

use std::io::Read;
use std::path::Path;
use std::process;

use clap::{Parser, Subcommand};
use serde_json::json;

use crate::headless::HeadlessRenderer;

/// config.toml 路径。
const CONFIG_PATH: &str = "config.toml";
/// 着色器目录。
const SHADER_DIR: &str = "shaders";

// ── CLI 定义 ──

#[derive(Parser)]
#[command(name = "lg0", about = "liquidglass0 渲染调试工具")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 读写 config.toml
    Config {
        #[command(subcommand)]
        cmd: ConfigCmd,
    },
    /// 读写着色器
    Shader {
        #[command(subcommand)]
        cmd: ShaderCmd,
    },
    /// 渲染并捕获纹理
    Capture {
        /// 输出宽度（像素）
        width: u32,
        /// 输出高度（像素）
        height: u32,
        /// 目标纹理：composite / displacement / h_blur / v_blur
        kind: String,
        /// PNG 输出路径，不指定时写入系统临时目录
        #[arg(long)]
        output: Option<String>,
    },
}

#[derive(Subcommand)]
enum ConfigCmd {
    /// 读取所有参数，输出 JSON
    Get,
    /// 修改参数
    Set {
        /// 参数 key（如 optical.refractive_index，数组用 key[0]）
        key: Option<String>,
        /// 参数 value（数字）
        value: Option<String>,
        /// 从 stdin 批量读取 JSON 键值对
        #[arg(long, conflicts_with_all = ["key", "value"])]
        batch: bool,
    },
    /// 恢复默认值
    Reset,
}

#[derive(Subcommand)]
enum ShaderCmd {
    /// 列出所有着色器名称
    List,
    /// 读取着色器源码
    Read {
        /// 着色器名称
        name: String,
    },
    /// 写入着色器源码（stdin），不刷新管线
    Write {
        /// 着色器名称
        name: String,
    },
    /// 从磁盘重编译管线
    Flush {
        /// 着色器名称
        name: String,
    },
}

// ── 入口 ──

pub fn run() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Config { cmd } => match cmd {
            ConfigCmd::Get => cmd_config_get(),
            ConfigCmd::Set { key, value, batch } => {
                if batch {
                    cmd_config_set_batch();
                } else {
                    match (key, value) {
                        (Some(k), Some(v)) => cmd_config_set(&k, &v),
                        _ => fail("需要 <KEY> <VALUE> 或 --batch"),
                    }
                }
            }
            ConfigCmd::Reset => cmd_config_reset(),
        },
        Commands::Shader { cmd } => match cmd {
            ShaderCmd::List => cmd_shader_list(),
            ShaderCmd::Read { name } => cmd_shader_read(&name),
            ShaderCmd::Write { name } => cmd_shader_write(&name),
            ShaderCmd::Flush { name } => {
                let mut h = headless();
                cmd_shader_flush(&mut h, &name);
            }
        },
        Commands::Capture {
            width,
            height,
            kind,
            output,
        } => {
            let mut h = headless();
            cmd_capture(&mut h, width, height, &kind, output.as_deref());
        }
    }
}

// ── 工具函数 ──

/// 创建 headless 渲染器（初始化 GPU 上下文）。
fn headless() -> HeadlessRenderer {
    pollster::block_on(HeadlessRenderer::new(512, 512))
}

/// 输出成功 JSON 到 stdout。
fn ok(data: serde_json::Value) {
    println!("{}", serde_json::to_string(&data).unwrap_or_default());
}

/// 输出错误 JSON 到 stderr 并退出。
fn fail(msg: &str) -> ! {
    let err = json!({"ok": false, "error": msg});
    eprintln!("{}", serde_json::to_string(&err).unwrap_or_default());
    process::exit(1);
}

// ── config 命令 ──

fn cmd_config_get() {
    let content = std::fs::read_to_string(CONFIG_PATH).unwrap_or_default();
    let val = match toml::from_str::<toml::Value>(&content) {
        Ok(doc) => {
            let json_str = serde_json::to_string_pretty(&doc).unwrap_or_default();
            serde_json::from_str(&json_str).unwrap_or(json!({}))
        }
        Err(_) => json!({}),
    };
    ok(json!({"config": val}));
}

fn cmd_config_set(key: &str, value: &str) {
    let mut doc = read_config().unwrap_or_else(|e| fail(&e));
    let val: f64 = value.parse().unwrap_or_else(|_| fail("value 需为数字"));
    apply_param(&mut doc, key, val);
    write_config(&doc).unwrap_or_else(|e| fail(&e));
    ok(json!({"ok": true}));
}

fn cmd_config_set_batch() {
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .unwrap_or_else(|e| fail(&format!("读取 stdin 失败: {e}")));
    let params: serde_json::Value =
        serde_json::from_str(&input).unwrap_or_else(|e| fail(&format!("JSON 解析失败: {e}")));
    let obj = params
        .as_object()
        .unwrap_or_else(|| fail("参数需为 JSON 对象"));

    let mut doc = read_config().unwrap_or_else(|e| fail(&e));
    for (key, value) in obj {
        let val = value
            .as_f64()
            .unwrap_or_else(|| fail(&format!("{key} 的值需为数字")));
        apply_param(&mut doc, key, val);
    }
    write_config(&doc).unwrap_or_else(|e| fail(&e));
    ok(json!({"ok": true}));
}

fn cmd_config_reset() {
    let doc = default_config_doc();
    write_config(&doc).unwrap_or_else(|e| fail(&e));
    ok(json!({"ok": true}));
}

/// 将单个 key=value 应用到 toml_edit::DocumentMut。
fn apply_param(doc: &mut toml_edit::DocumentMut, key: &str, val: f64) {
    let (table_key, field_key, index) = parse_key(key);
    let table = doc
        .get_mut(table_key)
        .unwrap_or_else(|| fail(&format!("未找到 table: {table_key}")));
    if let Some(i) = index {
        let arr = table
            .get_mut(field_key)
            .and_then(|v| v.as_array_mut())
            .unwrap_or_else(|| fail(&format!("{field_key} 不是数组")));
        arr.replace(i, toml_edit::Value::from(val));
    } else {
        table[field_key] = toml_edit::value(val);
    }
}

// ── shader 命令 ──

fn cmd_shader_list() {
    let names = list_shader_names();
    ok(json!({"names": names}));
}

fn cmd_shader_read(name: &str) {
    let path = shader_path(name);
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| fail(&format!("读取着色器 {name} 失败: {e}")));
    ok(json!({"name": name, "source": source}));
}

fn cmd_shader_write(name: &str) {
    let path = shader_path(name);
    if !path.exists() {
        fail(&format!("着色器不存在: {name}"));
    }
    let mut source = String::new();
    std::io::stdin()
        .read_to_string(&mut source)
        .unwrap_or_else(|e| fail(&format!("读取 stdin 失败: {e}")));
    std::fs::write(&path, &source).unwrap_or_else(|e| fail(&format!("写入失败: {e}")));
    ok(json!({"ok": true}));
}

fn cmd_shader_flush(headless: &mut HeadlessRenderer, name: &str) {
    let path = shader_path(name);
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| fail(&format!("读取着色器 {name} 失败: {e}")));
    headless
        .reload_shader(name, &source)
        .unwrap_or_else(|e| fail(&e));
    ok(json!({"ok": true}));
}

// ── capture 命令 ──

fn cmd_capture(
    headless: &mut HeadlessRenderer,
    width: u32,
    height: u32,
    kind: &str,
    output: Option<&str>,
) {
    let curr = headless.size();
    if width != curr.0 || height != curr.1 {
        headless.resize(width, height);
    }

    let (material, scene) = config_to_render(width, height).unwrap_or_else(|e| fail(&e));

    let owned_path;
    let path_arg = match output {
        Some(p) => Some(p),
        None => {
            let dir = std::env::temp_dir();
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            owned_path = dir
                .join(format!("lg0-{kind}-{ts}.png"))
                .display()
                .to_string();
            Some(owned_path.as_str())
        }
    };

    let (result, _mime) = headless
        .capture(&material, &scene, kind, path_arg)
        .unwrap_or_else(|e| fail(&e));

    ok(json!({"path": result, "format": "image/png"}));
}

// ── 配置读写 ──

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
) -> Result<(liquidglass0_core::GlassMaterial, liquidglass0_core::Scene), String> {
    use glam::{Vec2, Vec3};
    use liquidglass0_core::{GlassMaterial, GlassPanel, Light, Scene};

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

    let material = GlassMaterial {
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
