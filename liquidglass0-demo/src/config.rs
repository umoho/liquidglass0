//! 运行时配置模块。
//!
//! 从 `config.toml` 加载参数，避免每次调参都需要重新编译。
//! 缺失字段自动使用默认值（与 `GlassMaterial::default()` 对齐）。

use glam::{Vec2, Vec3};
use liquidglass0_core::{GlassMaterial, GlassPanel, Light, Scene};
use serde::Deserialize;

/// 顶层配置。
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct Config {
    /// 玻璃面板形状。
    pub panel: PanelConfig,
    /// 光学参数。
    pub optical: OpticalConfig,
    /// 材质参数。
    pub material: MaterialConfig,
    /// 阴影参数。
    pub shadow: ShadowConfig,
    /// 交互参数。
    pub interaction: InteractionConfig,
    /// 光源列表（最多 3 个）。
    pub lights: Vec<LightConfig>,
}

/// 面板形状配置。
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct PanelConfig {
    /// 面板半宽/半高（像素）。
    pub half_size: [f32; 2],
    /// 圆角半径（像素）。
    pub corner_radius: f32,
    /// 斜面宽度（占半径比例）。
    pub bevel_width: f32,
    /// 斜面深度（像素）。
    pub bevel_depth: f32,
    /// 厚度乘数归一化基准尺寸（像素）。
    pub reference_size: f32,
}

/// 光学参数配置。
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct OpticalConfig {
    /// 折射率。
    pub refractive_index: f32,
    /// 色散强度。
    pub chromatic_strength: f32,
    /// 菲涅尔反射强度。
    pub fresnel_intensity: f32,
    /// 菲涅尔颜色（RGB）。
    pub fresnel_color: [f32; 3],
    /// 镜面高光强度。
    pub specular_intensity: f32,
    /// 镜面高光锐度。
    pub specular_shininess: f32,
    /// 模糊半径（像素）。
    pub blur_radius: f32,
}

/// 材质参数配置。
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct MaterialConfig {
    /// 色调叠加强度。
    pub tint_opacity: f32,
    /// 玻璃底色（RGB）。
    pub tint_color: [f32; 3],
    /// 背景透过率。
    pub background_opacity: f32,
    /// 饱和度。
    pub saturation: f32,
    /// 对比度。
    pub contrast: f32,
    /// 亮度偏移。
    pub brightness: f32,
}

/// 阴影参数配置。
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct ShadowConfig {
    /// 阴影透明度。
    pub opacity: f32,
    /// 阴影模糊半径（像素）。
    pub blur: f32,
    /// 阴影 Y 偏移（像素）。
    pub offset_y: f32,
}

/// 交互参数配置。
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct InteractionConfig {
    /// 弹簧刚度。
    pub spring_k: f32,
    /// 弹簧阻尼系数。
    pub damping_b: f32,
}

/// 光源配置。
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct LightConfig {
    /// 光源位置（相对于窗口尺寸的比例，0~1）。
    pub position_factor: [f32; 2],
    /// 光源颜色（RGB）。
    pub color: [f32; 3],
    /// 光源强度。
    pub intensity: f32,
}

// ── 默认值 ──

impl Default for PanelConfig {
    fn default() -> Self {
        Self {
            half_size: [200.0, 150.0],
            corner_radius: 28.0,
            bevel_width: 0.20,
            bevel_depth: 55.0,
            reference_size: 200.0,
        }
    }
}

impl Default for OpticalConfig {
    fn default() -> Self {
        Self {
            refractive_index: 1.3,
            chromatic_strength: 0.03,
            fresnel_intensity: 2.0,
            fresnel_color: [0.9, 0.95, 1.0],
            specular_intensity: 0.4,
            specular_shininess: 150.0,
            blur_radius: 20.0,
        }
    }
}

impl Default for MaterialConfig {
    fn default() -> Self {
        Self {
            tint_opacity: 0.08,
            tint_color: [1.0, 1.0, 1.0],
            background_opacity: 0.92,
            saturation: 1.4,
            contrast: 1.04,
            brightness: 0.08,
        }
    }
}

impl Default for ShadowConfig {
    fn default() -> Self {
        Self {
            opacity: 0.3,
            blur: 8.0,
            offset_y: 4.0,
        }
    }
}

impl Default for InteractionConfig {
    fn default() -> Self {
        Self {
            spring_k: 300.0,
            damping_b: 20.0,
        }
    }
}

impl Default for LightConfig {
    fn default() -> Self {
        Self {
            position_factor: [0.5, 0.5],
            color: [1.0, 1.0, 1.0],
            intensity: 0.8,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            panel: PanelConfig::default(),
            optical: OpticalConfig::default(),
            material: MaterialConfig::default(),
            shadow: ShadowConfig::default(),
            interaction: InteractionConfig::default(),
            lights: vec![
                LightConfig {
                    position_factor: [0.2, 0.15],
                    color: [1.0, 1.0, 1.0],
                    intensity: 0.9,
                },
                LightConfig {
                    position_factor: [0.85, 0.25],
                    color: [0.95, 0.97, 1.0],
                    intensity: 0.5,
                },
                LightConfig {
                    position_factor: [0.75, 0.8],
                    color: [1.0, 0.98, 0.95],
                    intensity: 0.3,
                },
            ],
        }
    }
}

impl Config {
    /// 从 TOML 文件加载配置。
    ///
    /// 文件不存在或解析失败时回退到全默认值。
    pub fn load(path: &str) -> Self {
        match std::fs::read_to_string(path) {
            Ok(content) => match toml::from_str::<Config>(&content) {
                Ok(config) => config,
                Err(e) => {
                    eprintln!("配置解析失败，使用默认值: {e}");
                    Self::default()
                }
            },
            Err(e) => {
                eprintln!("配置文件读取失败，使用默认值: {e}");
                Self::default()
            }
        }
    }

    /// 转换为 [`GlassMaterial`]。
    pub fn to_material(&self) -> GlassMaterial {
        GlassMaterial {
            refractive_index: self.optical.refractive_index,
            chromatic_strength: self.optical.chromatic_strength,
            fresnel_intensity: self.optical.fresnel_intensity,
            fresnel_color: Vec3::new(
                self.optical.fresnel_color[0],
                self.optical.fresnel_color[1],
                self.optical.fresnel_color[2],
            ),
            specular_intensity: self.optical.specular_intensity,
            specular_shininess: self.optical.specular_shininess,
            blur_radius: self.optical.blur_radius,
            tint_color: Vec3::new(
                self.material.tint_color[0],
                self.material.tint_color[1],
                self.material.tint_color[2],
            ),
            tint_opacity: self.material.tint_opacity,
            background_opacity: self.material.background_opacity,
            saturation: self.material.saturation,
            contrast: self.material.contrast,
            brightness: self.material.brightness,
            shadow_opacity: self.shadow.opacity,
            shadow_blur: self.shadow.blur,
            shadow_offset_y: self.shadow.offset_y,
            deformation_spring_k: self.interaction.spring_k,
            deformation_damping_b: self.interaction.damping_b,
        }
    }

    /// 转换为 [`Scene`]。
    ///
    /// `center` 设为窗口中心，`lights` 的位置由 `position_factor` × 窗口尺寸计算。
    pub fn to_scene(&self, window_size: (u32, u32)) -> Scene {
        let (w, h) = (window_size.0 as f32, window_size.1 as f32);

        let panel = GlassPanel {
            center: Vec2::new(w / 2.0, h / 2.0),
            half_size: Vec2::new(self.panel.half_size[0], self.panel.half_size[1]),
            corner_radius: self.panel.corner_radius,
            bevel_width: self.panel.bevel_width,
            bevel_depth: self.panel.bevel_depth,
            reference_size: self.panel.reference_size,
        };

        let default_lights = Config::default().lights;
        let mut lights = [Light::default(); 3];
        for (i, light_cfg) in self.lights.iter().take(3).enumerate() {
            lights[i] = Light {
                position: Vec2::new(
                    w * light_cfg.position_factor[0],
                    h * light_cfg.position_factor[1],
                ),
                color: Vec3::new(light_cfg.color[0], light_cfg.color[1], light_cfg.color[2]),
                intensity: light_cfg.intensity,
            };
        }
        // 不足 3 个光源时用默认值填充
        for i in self.lights.len()..3 {
            let d = &default_lights[i];
            lights[i] = Light {
                position: Vec2::new(w * d.position_factor[0], h * d.position_factor[1]),
                color: Vec3::new(d.color[0], d.color[1], d.color[2]),
                intensity: d.intensity,
            };
        }

        Scene { panel, lights }
    }

    /// 弹簧刚度。
    pub fn deformation_spring_k(&self) -> f32 {
        self.interaction.spring_k
    }

    /// 弹簧阻尼系数。
    pub fn deformation_damping_b(&self) -> f32 {
        self.interaction.damping_b
    }

    /// 将当前配置写入 TOML 文件。
    ///
    /// 使用 `toml_edit` 实现原地字段更新，
    /// 保留文件中已有的注释和顺序。
    pub fn save(&self, path: &str) {
        let content = std::fs::read_to_string(path).unwrap_or_default();
        let mut doc = content
            .parse::<toml_edit::DocumentMut>()
            .unwrap_or_else(|_| toml_edit::DocumentMut::new());

        Self::set_float2(&mut doc, "panel", "half_size", &self.panel.half_size);
        Self::set_float(&mut doc, "panel", "corner_radius", self.panel.corner_radius);
        Self::set_float(&mut doc, "panel", "bevel_width", self.panel.bevel_width);
        Self::set_float(&mut doc, "panel", "bevel_depth", self.panel.bevel_depth);
        Self::set_float(
            &mut doc,
            "panel",
            "reference_size",
            self.panel.reference_size,
        );

        Self::set_float(
            &mut doc,
            "optical",
            "refractive_index",
            self.optical.refractive_index,
        );
        Self::set_float(
            &mut doc,
            "optical",
            "chromatic_strength",
            self.optical.chromatic_strength,
        );
        Self::set_float(
            &mut doc,
            "optical",
            "fresnel_intensity",
            self.optical.fresnel_intensity,
        );
        Self::set_float3(
            &mut doc,
            "optical",
            "fresnel_color",
            &self.optical.fresnel_color,
        );
        Self::set_float(
            &mut doc,
            "optical",
            "specular_intensity",
            self.optical.specular_intensity,
        );
        Self::set_float(
            &mut doc,
            "optical",
            "specular_shininess",
            self.optical.specular_shininess,
        );
        Self::set_float(&mut doc, "optical", "blur_radius", self.optical.blur_radius);

        Self::set_float(
            &mut doc,
            "material",
            "tint_opacity",
            self.material.tint_opacity,
        );
        Self::set_float3(
            &mut doc,
            "material",
            "tint_color",
            &self.material.tint_color,
        );
        Self::set_float(
            &mut doc,
            "material",
            "background_opacity",
            self.material.background_opacity,
        );
        Self::set_float(&mut doc, "material", "saturation", self.material.saturation);
        Self::set_float(&mut doc, "material", "contrast", self.material.contrast);
        Self::set_float(&mut doc, "material", "brightness", self.material.brightness);

        Self::set_float(&mut doc, "shadow", "opacity", self.shadow.opacity);
        Self::set_float(&mut doc, "shadow", "blur", self.shadow.blur);
        Self::set_float(&mut doc, "shadow", "offset_y", self.shadow.offset_y);

        Self::set_float(
            &mut doc,
            "interaction",
            "spring_k",
            self.interaction.spring_k,
        );
        Self::set_float(
            &mut doc,
            "interaction",
            "damping_b",
            self.interaction.damping_b,
        );

        std::fs::write(path, doc.to_string()).unwrap_or_else(|e| {
            eprintln!("保存配置失败: {e}");
        });
    }

    fn set_float(doc: &mut toml_edit::DocumentMut, table: &str, key: &str, value: f32) {
        doc[table][key] = toml_edit::value(value as f64);
    }

    fn set_float2(doc: &mut toml_edit::DocumentMut, table: &str, key: &str, value: &[f32; 2]) {
        let arr =
            toml_edit::Array::from_iter(value.iter().map(|v| toml_edit::Value::from(*v as f64)));
        doc[table][key] = toml_edit::value(arr);
    }

    fn set_float3(doc: &mut toml_edit::DocumentMut, table: &str, key: &str, value: &[f32; 3]) {
        let arr =
            toml_edit::Array::from_iter(value.iter().map(|v| toml_edit::Value::from(*v as f64)));
        doc[table][key] = toml_edit::value(arr);
    }
}
