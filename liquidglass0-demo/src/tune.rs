//! 调参模式。
//!
//! 使用 GPUI 构建实时参数调节面板。左侧预览占位，右侧可滚动参数滑块。
//! 修改参数后点击 [保存] 写入 config.toml。
//! 运行 `cargo run`（查看模式）预览效果。

use gpui::*;

use crate::config::Config;

/// 右侧面板宽度（像素）。
const PANEL_WIDTH: f32 = 340.0;
/// 轨道估算宽度（面板宽度 - 两侧 padding）。
const TRACK_WIDTH: f32 = PANEL_WIDTH - 24.0;

// ── 公共入口 ──

/// 启动 GPUI 调参窗口。
pub fn run(config: Config) {
    Application::new().run(move |cx: &mut App| {
        cx.open_window(
            WindowOptions {
                focus: true,
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(1000.), px(700.)),
                    cx,
                ))),
                ..Default::default()
            },
            |_window, cx| cx.new(|cx| TuneApp::new(cx, config)),
        )
        .unwrap();

        // 关闭窗口时退出应用
        cx.on_window_closed(|cx| {
            cx.quit();
        })
        .detach();
    });
}

// ── 手动滑块数据结构 ──

/// 单个参数滑块的状态。
struct ParamSlider {
    label: SharedString,
    value: f32,
    min: f32,
    max: f32,
    step: f32,
    fmt: fn(f32) -> String,
}

impl ParamSlider {
    fn new(
        label: impl Into<SharedString>,
        min: f32,
        max: f32,
        step: f32,
        value: f32,
        fmt: fn(f32) -> String,
    ) -> Self {
        Self {
            label: label.into(),
            value: value.clamp(min, max),
            min,
            max,
            step,
            fmt,
        }
    }

    fn set_value(&mut self, v: f32) {
        self.value = v.clamp(self.min, self.max);
    }

    fn percentage(&self) -> f32 {
        (self.value - self.min) / (self.max - self.min).max(f32::EPSILON)
    }
}

// ── TuneApp ──

/// 调参面板实体。
pub struct TuneApp {
    config: Config,

    // 参数分组
    panel_params: Vec<ParamSlider>,
    optical_params: Vec<ParamSlider>,
    material_params: Vec<ParamSlider>,
    shadow_params: Vec<ParamSlider>,
    interaction_params: Vec<ParamSlider>,

    // 颜色参数
    fresnel_color: [f32; 3],
    tint_color: [f32; 3],

    // 拖动状态（delta 式跟踪）
    drag_active: bool,
    drag_slider_idx: Option<(usize, usize)>,
    /// 拖动起始时的鼠标 X 坐标（窗口坐标系）。
    drag_start_x: Option<Pixels>,
    /// 拖动起始时滑块的值。
    drag_start_value: f32,
}

impl TuneApp {
    fn new(_cx: &mut Context<Self>, config: Config) -> Self {
        let opt = |fmt: fn(f32) -> String| fmt;

        let panel_params = vec![
            ParamSlider::new("圆角半径", 0.0, 80.0, 1.0, config.panel.corner_radius, opt(|v| format!("{v:.0} px"))),
            ParamSlider::new("倒角宽度", 0.0, 0.5, 0.01, config.panel.bevel_width, opt(|v| format!("{v:.2}"))),
            ParamSlider::new("倒角深度", 0.0, 120.0, 1.0, config.panel.bevel_depth, opt(|v| format!("{v:.0} px"))),
        ];

        let optical_params = vec![
            ParamSlider::new("折射率", 1.0, 2.0, 0.01, config.optical.refractive_index, opt(|v| format!("{v:.2}"))),
            ParamSlider::new("色散强度", 0.0, 0.2, 0.001, config.optical.chromatic_strength, opt(|v| format!("{v:.3}"))),
            ParamSlider::new("菲涅尔强度", 0.0, 10.0, 0.1, config.optical.fresnel_intensity, opt(|v| format!("{v:.1}"))),
            ParamSlider::new("高光强度", 0.0, 2.0, 0.01, config.optical.specular_intensity, opt(|v| format!("{v:.2}"))),
            ParamSlider::new("高光锐度", 1.0, 500.0, 1.0, config.optical.specular_shininess, opt(|v| format!("{v:.0}"))),
            ParamSlider::new("模糊半径", 0.0, 50.0, 0.5, config.optical.blur_radius, opt(|v| format!("{v:.1} px"))),
        ];

        let material_params = vec![
            ParamSlider::new("底色强度", 0.0, 1.0, 0.01, config.material.tint_opacity, opt(|v| format!("{v:.2}"))),
            ParamSlider::new("透过率", 0.0, 1.0, 0.01, config.material.background_opacity, opt(|v| format!("{v:.2}"))),
            ParamSlider::new("饱和度", 0.0, 3.0, 0.05, config.material.saturation, opt(|v| format!("{v:.2}"))),
            ParamSlider::new("对比度", 0.0, 3.0, 0.01, config.material.contrast, opt(|v| format!("{v:.2}"))),
            ParamSlider::new("亮度", -1.0, 1.0, 0.01, config.material.brightness, opt(|v| format!("{v:+.2}"))),
        ];

        let shadow_params = vec![
            ParamSlider::new("不透明度", 0.0, 1.0, 0.01, config.shadow.opacity, opt(|v| format!("{v:.2}"))),
            ParamSlider::new("模糊", 0.0, 40.0, 0.5, config.shadow.blur, opt(|v| format!("{v:.1} px"))),
            ParamSlider::new("Y 偏移", 0.0, 30.0, 1.0, config.shadow.offset_y, opt(|v| format!("{v:.0} px"))),
        ];

        let interaction_params = vec![
            ParamSlider::new("弹簧刚度", 50.0, 600.0, 10.0, config.interaction.spring_k, opt(|v| format!("{v:.0}"))),
            ParamSlider::new("阻尼系数", 5.0, 50.0, 1.0, config.interaction.damping_b, opt(|v| format!("{v:.0}"))),
        ];

        let fresnel = config.optical.fresnel_color;
        let tint = config.material.tint_color;

        Self {
            config,
            panel_params,
            optical_params,
            material_params,
            shadow_params,
            interaction_params,
            fresnel_color: fresnel,
            tint_color: tint,
            drag_active: false,
            drag_slider_idx: None,
            drag_start_x: None,
            drag_start_value: 0.0,
        }
    }

    fn update_slider_delta(
        &mut self,
        group: usize,
        idx: usize,
        current_x: Pixels,
        cx: &mut Context<Self>,
    ) {
        let start_x = self.drag_start_x.unwrap_or(current_x);
        let start_value = self.drag_start_value;

        let params = self.get_params_group_mut(group);
        let slider = &mut params[idx];
        let range = slider.max - slider.min;
        let delta_px = current_x - start_x;
        let delta_value = (delta_px / px(TRACK_WIDTH)).clamp(-1.0, 1.0) * range;
        let raw = start_value + delta_value;
        let stepped = (raw / slider.step).round() * slider.step;
        slider.value = stepped.clamp(slider.min, slider.max);
        cx.notify();
    }

    fn get_params_group_mut(&mut self, group: usize) -> &mut Vec<ParamSlider> {
        match group {
            0 => &mut self.panel_params,
            1 => &mut self.optical_params,
            2 => &mut self.material_params,
            3 => &mut self.shadow_params,
            4 => &mut self.interaction_params,
            _ => unreachable!(),
        }
    }

    fn get_params_group(&self, group: usize) -> &Vec<ParamSlider> {
        match group {
            0 => &self.panel_params,
            1 => &self.optical_params,
            2 => &self.material_params,
            3 => &self.shadow_params,
            4 => &self.interaction_params,
            _ => unreachable!(),
        }
    }

    fn save_config(&mut self) {
        self.sync_config_from_sliders();
        self.config.save("config.toml");
    }

    fn reset_config(&mut self, cx: &mut Context<Self>) {
        let defaults = Config::default();
        self.update_sliders_from_config(&defaults);
        self.config = defaults;
        self.fresnel_color = self.config.optical.fresnel_color;
        self.tint_color = self.config.material.tint_color;
        cx.notify();
    }

    fn sync_config_from_sliders(&mut self) {
        self.config.panel.corner_radius = self.panel_params[0].value;
        self.config.panel.bevel_width = self.panel_params[1].value;
        self.config.panel.bevel_depth = self.panel_params[2].value;

        self.config.optical.refractive_index = self.optical_params[0].value;
        self.config.optical.chromatic_strength = self.optical_params[1].value;
        self.config.optical.fresnel_intensity = self.optical_params[2].value;
        self.config.optical.fresnel_color = self.fresnel_color;
        self.config.optical.specular_intensity = self.optical_params[3].value;
        self.config.optical.specular_shininess = self.optical_params[4].value;
        self.config.optical.blur_radius = self.optical_params[5].value;

        self.config.material.tint_color = self.tint_color;
        self.config.material.tint_opacity = self.material_params[0].value;
        self.config.material.background_opacity = self.material_params[1].value;
        self.config.material.saturation = self.material_params[2].value;
        self.config.material.contrast = self.material_params[3].value;
        self.config.material.brightness = self.material_params[4].value;

        self.config.shadow.opacity = self.shadow_params[0].value;
        self.config.shadow.blur = self.shadow_params[1].value;
        self.config.shadow.offset_y = self.shadow_params[2].value;

        self.config.interaction.spring_k = self.interaction_params[0].value;
        self.config.interaction.damping_b = self.interaction_params[1].value;
    }

    fn update_sliders_from_config(&mut self, config: &Config) {
        self.panel_params[0].set_value(config.panel.corner_radius);
        self.panel_params[1].set_value(config.panel.bevel_width);
        self.panel_params[2].set_value(config.panel.bevel_depth);

        self.optical_params[0].set_value(config.optical.refractive_index);
        self.optical_params[1].set_value(config.optical.chromatic_strength);
        self.optical_params[2].set_value(config.optical.fresnel_intensity);
        self.optical_params[3].set_value(config.optical.specular_intensity);
        self.optical_params[4].set_value(config.optical.specular_shininess);
        self.optical_params[5].set_value(config.optical.blur_radius);

        self.material_params[0].set_value(config.material.tint_opacity);
        self.material_params[1].set_value(config.material.background_opacity);
        self.material_params[2].set_value(config.material.saturation);
        self.material_params[3].set_value(config.material.contrast);
        self.material_params[4].set_value(config.material.brightness);

        self.shadow_params[0].set_value(config.shadow.opacity);
        self.shadow_params[1].set_value(config.shadow.blur);
        self.shadow_params[2].set_value(config.shadow.offset_y);

        self.interaction_params[0].set_value(config.interaction.spring_k);
        self.interaction_params[1].set_value(config.interaction.damping_b);
    }
}

// ── 硬编码颜色 ──

const SURFACE: Rgba = Rgba { r: 0.12, g: 0.12, b: 0.14, a: 1.0 };
const ACCENT: Rgba = Rgba { r: 0.3, g: 0.5, b: 0.9, a: 1.0 };
const ACCENT_HOVER: Rgba = Rgba { r: 0.35, g: 0.55, b: 0.95, a: 1.0 };
const THUMB_BG: Rgba = Rgba { r: 0.85, g: 0.85, b: 0.87, a: 1.0 };
const MUTED_BG: Rgba = Rgba { r: 0.15, g: 0.15, b: 0.17, a: 1.0 };
const BORDER: Rgba = Rgba { r: 0.22, g: 0.22, b: 0.25, a: 1.0 };
const TEXT_DIM: Rgba = Rgba { r: 1.0, g: 1.0, b: 1.0, a: 0.6 };
const TRACK_BG: Rgba = Rgba { r: 0.25, g: 0.25, b: 0.28, a: 1.0 };
const HOVER_BG: Rgba = Rgba { r: 0.2, g: 0.2, b: 0.22, a: 1.0 };
const PREVIEW_BORDER: Rgba = Rgba { r: 0.3, g: 0.3, b: 0.35, a: 1.0 };

// ── GPUI Render ──

impl Render for TuneApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(SURFACE)
            .text_color(gpui::white())
            .flex()
            .flex_col()
            .child(
                // 标题栏
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .px_4()
                    .py_2()
                    .gap_3()
                    .border_b_1()
                    .border_color(BORDER)
                    .child(
                        div()
                            .font_weight(FontWeight::BOLD)
                            .text_lg()
                            .child("liquidglass0 — 参数调校"),
                    )
                    .child(div().flex_1())
                    .child(
                        div()
                            .id("btn-reset")
                            .px_3()
                            .py_1()
                            .rounded_md()
                            .cursor_pointer()
                            .hover(|this| this.bg(HOVER_BG))
                            .child(SharedString::from("重置"))
                            .on_mouse_up(
                                MouseButton::Left,
                                cx.listener(|this, _: &MouseUpEvent, _w, cx| {
                                    this.reset_config(cx);
                                }),
                            ),
                    )
                    .child(
                        div()
                            .id("btn-save")
                            .px_3()
                            .py_1()
                            .rounded_md()
                            .bg(ACCENT)
                            .cursor_pointer()
                            .hover(|this| this.bg(ACCENT_HOVER))
                            .child(SharedString::from("保存"))
                            .on_mouse_up(
                                MouseButton::Left,
                                cx.listener(|this, _: &MouseUpEvent, _w, _cx| {
                                    this.save_config();
                                }),
                            ),
                    ),
            )
            .child(
                // 主内容
                div()
                    .flex_1()
                    .flex()
                    .flex_row()
                    .child(
                        // 左侧预览区域（占位）
                        div()
                            .flex_1()
                            .flex()
                            .items_center()
                            .justify_center()
                            .bg(MUTED_BG)
                            .child(
                                div()
                                    .size(px(512.0))
                                    .flex()
                                    .flex_col()
                                    .items_center()
                                    .justify_center()
                                    .gap_4()
                                    .border_1()
                                    .border_color(PREVIEW_BORDER)
                                    .child(
                                        div().text_lg().text_color(TEXT_DIM).child("玻璃效果预览"),
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(TEXT_DIM)
                                            .child("调参后点击 [保存]，"),
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(TEXT_DIM)
                                            .child("运行 `cargo run` 查看效果"),
                                    ),
                            ),
                    )
                    .child(
                        // 右侧面板
                        div()
                            .id("control-panel")
                            .w(px(PANEL_WIDTH))
                            .h_full()
                            .overflow_y_scroll()
                            .border_l_1()
                            .border_color(BORDER)
                            .child(self.render_controls(cx)),
                    ),
            )
    }
}

impl TuneApp {
    fn render_controls(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let fresnel = self.fresnel_color;
        let tint = self.tint_color;

        div().flex().flex_col().gap_1().p_3()
            .child(self.render_section("面板形状"))
            .child(self.render_slider_group(0, cx))
            .child(self.render_section("光学参数"))
            .child(self.render_slider_group(1, cx))
            .child(self.render_color_row(cx, "菲涅尔色", fresnel))
            .child(self.render_section("材质"))
            .child(self.render_color_row(cx, "底色", tint))
            .child(self.render_slider_group(2, cx))
            .child(self.render_section("阴影"))
            .child(self.render_slider_group(3, cx))
            .child(self.render_section("交互物理"))
            .child(self.render_slider_group(4, cx))
    }

    fn render_section(&self, title: &'static str) -> impl IntoElement {
        div()
            .pt_3()
            .pb_1()
            .font_weight(FontWeight::BOLD)
            .text_sm()
            .child(SharedString::from(title))
    }

    fn render_slider_group(&mut self, group: usize, cx: &mut Context<Self>) -> impl IntoElement {
        let len = self.get_params_group(group).len();
        let mut children = div().flex().flex_col().gap_1();

        for idx in 0..len {
            let slider = &self.get_params_group(group)[idx];
            let pct = slider.percentage();
            let display = (slider.fmt)(slider.value);

            let slider_div = self.render_slider_bar(group, idx, pct, &display, cx);
            children = children.child(slider_div);
        }
        children
    }

    fn render_slider_bar(
        &mut self,
        group: usize,
        idx: usize,
        pct: f32,
        display: &str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let slider = &self.get_params_group(group)[idx];
        let label = slider.label.clone();
        let display = SharedString::from(display.to_string());

        div().flex().flex_col().gap_0p5()
            .child(
                div()
                    .flex()
                    .flex_row()
                    .justify_between()
                    .child(div().text_xs().text_color(TEXT_DIM).child(label))
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::MEDIUM)
                            .child(display),
                    ),
            )
            .child(
                // 轨道
                div()
                    .h(px(20.0))
                    .w_full()
                    .flex()
                    .items_center()
                    .cursor_pointer()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, e: &MouseDownEvent, _, cx| {
                            // 记录拖动起始位置和当前值
                            this.drag_active = true;
                            this.drag_slider_idx = Some((group, idx));
                            this.drag_start_x = Some(e.position.x);
                            let params = this.get_params_group(group);
                            this.drag_start_value = params[idx].value;
                            // 立即响应点击
                            this.update_slider_delta(group, idx, e.position.x, cx);
                            cx.notify();
                        }),
                    )
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(|this, _: &MouseUpEvent, _, _cx| {
                            this.drag_active = false;
                            this.drag_slider_idx = None;
                            this.drag_start_x = None;
                        }),
                    )
                    .on_mouse_move(cx.listener(move |this, e: &MouseMoveEvent, _, cx| {
                        if this.drag_active && this.drag_slider_idx == Some((group, idx)) {
                            this.update_slider_delta(group, idx, e.position.x, cx);
                            cx.notify();
                        }
                    }))
                    .child(
                        // 实际轨道
                        div()
                            .relative()
                            .w_full()
                            .h(px(6.0))
                            .rounded_full()
                            .bg(TRACK_BG)
                            .child(
                                // 填充
                                div()
                                    .absolute()
                                    .top_0()
                                    .left_0()
                                    .h_full()
                                    .rounded_full()
                                    .bg(ACCENT)
                                    .w(relative(pct)),
                            )
                            .child(
                                // 拇指
                                div()
                                    .absolute()
                                    .top(px(-4.0))
                                    .h(px(14.0))
                                    .w(px(14.0))
                                    .rounded_full()
                                    .bg(THUMB_BG)
                                    .shadow_sm()
                                    .left(relative(pct))
                                    .ml(px(-7.0)),
                            ),
                    ),
            )
    }

    fn render_color_row(
        &self,
        cx: &mut Context<Self>,
        label: &'static str,
        color: [f32; 3],
    ) -> impl IntoElement {
        let swatch = Rgba { r: color[0], g: color[1], b: color[2], a: 1.0 };
        div().flex().flex_col().gap_1()
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .text_xs()
                            .text_color(TEXT_DIM)
                            .child(SharedString::from(label)),
                    )
                    .child(
                        div()
                            .size(px(16.0))
                            .rounded_sm()
                            .border_1()
                            .border_color(PREVIEW_BORDER)
                            .bg(swatch),
                    ),
            )
            .child(
                div().flex().flex_row().gap_1()
                    .child(div().flex_1().child(Self::render_mini_slider_static("R", color[0], cx)))
                    .child(div().flex_1().child(Self::render_mini_slider_static("G", color[1], cx)))
                    .child(div().flex_1().child(Self::render_mini_slider_static("B", color[2], cx))),
            )
    }

    fn render_mini_slider_static(
        label: &'static str,
        value: f32,
        _cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let pct = value.clamp(0.0, 1.0);
        let display = format!("{value:.2}");

        div().flex().flex_col().gap_0p5()
            .child(
                div()
                    .flex()
                    .flex_row()
                    .justify_between()
                    .child(div().text_xs().text_color(TEXT_DIM).child(SharedString::from(label)))
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::MEDIUM)
                            .child(SharedString::from(display)),
                    ),
            )
            .child(
                div().h(px(16.0)).w_full().flex().items_center().child(
                    div().relative().w_full().h(px(4.0)).rounded_full().bg(TRACK_BG)
                        .child(
                            div()
                                .absolute()
                                .top_0()
                                .left_0()
                                .h_full()
                                .rounded_full()
                                .bg(ACCENT)
                                .w(relative(pct)),
                        )
                        .child(
                            div()
                                .absolute()
                                .top(px(-3.0))
                                .h(px(10.0))
                                .w(px(10.0))
                                .rounded_full()
                                .bg(THUMB_BG)
                                .left(relative(pct))
                                .ml(px(-5.0)),
                        ),
                ),
            )
    }
}
