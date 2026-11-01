#version 450

// 方向光 Shadow Map 深度阶段 — 顶点（0.2）
// 每个 cascade 渲染一遍场景；push constant 由 Rust 端 ShadowPushConstant 提供。
// 布局契约由 engine/render-pipeline/tests/shader_contract.rs 校验。

layout(location = 0) in vec3 inPosition;

// push constant: ShadowPushConstant (128 bytes)
layout(push_constant) uniform ShadowPushConstants {
    mat4 light_view_proj;
    mat4 model;
} pc;

void main() {
    gl_Position = pc.light_view_proj * pc.model * vec4(inPosition, 1.0);
}
