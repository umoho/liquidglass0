// 全屏三角形顶点着色器。
//
// 用 `@builtin(vertex_index)` 生成覆盖整个屏幕的两个三角形，
// 无需 vertex buffer。输出裁剪空间坐标和 UV。

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) uv: vec2f,
}

@vertex
fn main(@builtin(vertex_index) vid: u32) -> VertexOutput {
    let x = f32(i32(vid & 1u) * 2);
    let y = f32(i32(vid >> 1u) * 2);

    var out: VertexOutput;
    out.position = vec4f(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
    out.uv = vec2f(x, y);
    return out;
}
