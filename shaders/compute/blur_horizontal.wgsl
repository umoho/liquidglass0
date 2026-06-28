// 分离式高斯模糊 — 水平 pass。
//
// 工作组 (256, 1, 1)：每行一个工作组，每个线程处理一个像素。
// 沿 X 方向读入邻域像素，一维高斯核加权求和，写入输出纹理。

struct BlurParams {
    texture_size: vec2<u32>,
    blur_radius: f32,
    kernel_half: u32,
}

@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var<uniform> params: BlurParams;

@compute @workgroup_size(256, 1, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    if (gid.x >= params.texture_size.x || gid.y >= params.texture_size.y) {
        return;
    }

    let sigma = max(params.blur_radius / 2.0, 0.5);
    let two_sigma2 = 2.0 * sigma * sigma;
    let half = i32(params.kernel_half) - 1;

    var color: vec4f = vec4f(0.0);
    var weight_sum: f32 = 0.0;

    for (var i = -half; i <= half; i++) {
        let sx = i32(gid.x) + i;
        let sample_x = clamp(sx, 0, i32(params.texture_size.x) - 1);
        let weight = exp(-f32(i * i) / two_sigma2);
        color += textureLoad(input_tex, vec2i(sample_x, i32(gid.y)), 0) * weight;
        weight_sum += weight;
    }

    color /= weight_sum;
    textureStore(output_tex, vec2u(gid.x, gid.y), color);
}
