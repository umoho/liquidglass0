# liquidglass0 架构

基于 Rust + WGPU（计算着色器）的 Liquid Glass 效果实验性渲染器。
灵感来源于 Apple 在 WWDC 2025 发布的 Liquid Glass 设计语言。

## Workspace

```
liquidglass0/
├── Cargo.toml
│
├── liquidglass0-core/       # 共享类型：参数、配置、数学/SDF 工具
├── liquidglass0-render/     # WGPU 管线编排，无窗口耦合
└── liquidglass0-demo/       # 交互式 demo 二进制（winit + 鼠标输入）
```

| Crate | 依赖 | 职责 |
|---|---|---|
| `liquidglass0-core` | — | `GlassParams`、`InteractionState`、SDF 工具、数学 |
| `liquidglass0-render` | `core`、`wgpu`、`naga_oil` | 管线创建、着色器加载、逐帧渲染 |
| `liquidglass0-demo` | `render`、`winit`、`image` | 窗口生命周期、输入 → 渲染 → 呈现循环 |

## 着色器加载

通过 `ShaderLoader` trait 抽象：

```
src/shader.rs:
  pub trait ShaderLoader { fn load_wgsl(&self, name: &str) -> String; }

  EmbeddedLoader  — 编译期 include_str!（Phase 1 默认）
  NagaOilLoader   — naga_oil compose()，解析 #import（Phase 2）
```

向 `GlassRenderer::new()` 传入不同的 loader 即可切换策略，管线代码无需修改。

## 着色器模块

```
shaders/
├── compute/
│   ├── blur_horizontal.wgsl   分离式高斯模糊（水平 pass）
│   ├── blur_vertical.wgsl     分离式高斯模糊（垂直 pass）
│   └── refract.wgsl           基于 SDF 曲面法线的位移图
├── fragment/
│   └── composite.wgsl         最终合成：折射 + 色散 + 菲涅尔
└── common/
    ├── sdf.wgsl               圆角矩形 SDF、曲面法线、斜面轮廓
    ├── glass_params.wgsl      共享 uniform 布局
    └── fullscreen_triangle.wgsl 所有 fragment pass 共享的顶点着色器
```

所有导入使用 naga_oil 的 `#import` 语法，文件名不带后缀。

## 渲染管线

所有效果在一个 fragment pass 中合成，compute pass 只做数据预处理。

```
输入: background_texture
  │
  ├── 预处理 (compute): displacement
  │     输入: SDF 曲面法线 + 折射率
  │     输出: 每个像素在背景上的采样偏移量
  │
  └── 合成 (fragment): glass
        全屏三角形，逐像素:
          1. 用 displacement 偏移采样 background → 折射
          2. 分离 RGB 通道采样 → 色散
          3. 采样模糊纹理 → 磨砂感
          4. 计算菲涅尔项 → 边缘发光
          5. Blinn-Phong 镜面高光
          6. 叠加色调、亮度、饱和度调整
```

## 渲染器 API

```rust
// liquidglass0-render/src/lib.rs

pub struct GlassRenderer { /* 所有 wgpu 资源内部管理 */ }

pub struct RenderInput<'a> {
    pub background: &'a wgpu::TextureView,
    pub size: (u32, u32),
    pub interaction: InteractionState,
    pub time: f32,
    pub params: GlassParams,
}

impl GlassRenderer {
    pub fn new(
        device: wgpu::Device,
        queue: wgpu::Queue,
        loader: impl ShaderLoader,
        config: RendererConfig,
    ) -> Self;

    pub fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        input: &RenderInput,
        output: &wgpu::TextureView,
    );
}
```

`GlassRenderer` 不知道窗口的存在。`-demo` 负责创建 winit 窗口和 surface，
接收输入，将输出纹理提交给 swap chain。

## Demo 架构（`-demo`）

```
winit event_loop
 │
 ├── WindowEvent::Resized     → 重新配置 surface
 ├── WindowEvent::CursorMoved → 更新 InteractionState
 ├── WindowEvent::MouseInput  → 更新 InteractionState
 │
 └── MainEventsCleared:
       加载/生成背景纹理
       调用 GlassRenderer::render()
       提交到 swap chain
```

背景数据源由 `-demo` 决定，renderer 不关心图像来自图片、渐变还是程序化生成。

> 各阶段实现进度见 [`IMPLEMENTS.md`](./IMPLEMENTS.md)

## 工作组大小

| Pass | 大小 | 说明 |
|---|---|---|
| `blur_horizontal` | (256, 1) | 每行一个工作组，线程共享内存 |
| `blur_vertical`   | (1, 256) | 每列一个工作组 |
| `refract`         | (16, 16) | 二维分块，每个像素独立 |

常量定义在 `pipeline.rs`，一处调整全局生效。
