// 最终合成 fragment shader。
//
// Phase 1：直接采样垂直模糊结果输出。
// 后续 Phase 会加入折射、色散、菲涅尔等效果。

@group(0) @binding(0) var blur_tex: texture_2d<f32>;
@group(0) @binding(1) var blur_sampler: sampler;

@fragment
fn main(@location(0) uv: vec2f) -> @location(0) vec4f {
    return textureSample(blur_tex, blur_sampler, uv);
}
