# 实现状态

## 里程碑

当前项目分 5 个阶段推进，每个阶段有明确的可验证目标。

| 阶段 | 目标 |
|---|---|
| **1** | 基础玻璃面板：圆角 + 折射 + 模糊，窗口可见 |
| **2** | 材质丰富：色散、菲涅尔、高光 |
| **3** | 曲面与深度：bevel、阴影、厚度自适应 |
| **4** | 交互：弹簧变形、悬停高光、滚动抬起 |
| **5** | 打磨：参数系统、多玻璃叠加、性能调优 |

> 当前阶段：Phase 1

---

## Phase 1：基础玻璃面板

- [x] Workspace 骨架：三个 crate 就位
- [x] Cargo.toml workspace 配置
- [x] liquidglass0-core：GlassParams、InteractionState、SDF 工具
- [x] liquidglass0-render：ShaderLoader + EmbeddedLoader + wgpu 设备初始化
- [ ] 着色器：blur_horizontal（compute）
- [ ] 着色器：blur_vertical（compute）
- [ ] 管线编排：blur → composite 串联
- [ ] liquidglass0-demo：winit 窗口 + 背景图渲染
- [ ] 第一个可验证效果：窗口里看到经过高斯模糊的玻璃面板

## Phase 2：材质丰富

- [ ] 折射（refraction）：基于 SDF 法线的背景位移采样
- [ ] 色散（chromatic aberration）：RGB 分离偏移
- [ ] 菲涅尔边缘光（Fresnel）：Schlick 近似
- [ ] 镜面高光（specular）：多光源 Blinn-Phong
- [ ] 磨砂联动（blur）：模糊半径随厚度变化

## Phase 3：曲面与深度

- [ ] 斜面轮廓（bevel）：凸透镜截面
- [ ] 动态阴影
- [ ] 厚度感（thickness multiplier）
- [ ] 尺寸自适应

## Phase 4：交互

- [ ] 鼠标悬停：高光跟随
- [ ] 点击：弹簧式表面变形
- [ ] 拖拽释放：阻尼回弹
- [ ] 抬起/滚动自适应

## Phase 5：打磨

- [ ] 参数系统：开放可调参数
- [ ] 多玻璃叠加
- [ ] 性能调优
- [ ] 跨平台验证
