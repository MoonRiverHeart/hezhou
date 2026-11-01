#version 450

// 全屏三角形顶点着色器 — 0.2 后处理（Tonemap / Bloom）
// 无顶点输入，使用 gl_VertexIndex 生成覆盖整屏的两个三角形（3 顶点即可覆盖）。

layout(location = 0) out vec2 out_uv;

void main() {
    vec2 positions[3] = vec2[](
        vec2(-1.0, -1.0),
        vec2(3.0, -1.0),
        vec2(-1.0, 3.0)
    );
    vec2 uvs[3] = vec2[](
        vec2(0.0, 0.0),
        vec2(2.0, 0.0),
        vec2(0.0, 2.0)
    );
    gl_Position = vec4(positions[gl_VertexIndex], 0.0, 1.0);
    out_uv = uvs[gl_VertexIndex];
}
