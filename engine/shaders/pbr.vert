#version 450

// PBR 前向着色 — 顶点阶段（0.2）
// 布局契约由 engine/render-pipeline/tests/shader_contract.rs 校验；
// 修改字段顺序/类型必须同步 Rust 端 PbrPushConstant / PbrFrameUbo。

layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec3 inNormal;
layout(location = 2) in vec2 inUV;

// push constant: PbrPushConstant (96 bytes)
layout(push_constant) uniform PbrPushConstants {
    mat4 model;
    vec4 base_color;
    vec4 material_params;
} pc;

// 每帧 UBO: PbrFrameUbo (480 bytes, std140)
layout(std140, set = 0, binding = 0) uniform PbrFrameUbo {
    vec3 direction;
    float intensity;
    vec3 color;
    float shadow_strength;
    vec3 ambient;
    float cascade_count;
    vec4 cascade_splits;
    vec4 shadow_params;
    mat4 view;
    mat4 proj;
    mat4 light_view_proj[4];
    vec4 camera_pos;
} frame;

layout(location = 0) out vec3 outWorldPos;
layout(location = 1) out vec3 outNormal;
layout(location = 2) out vec2 outUV;

void main() {
    vec4 world = pc.model * vec4(inPosition, 1.0);
    gl_Position = frame.proj * frame.view * world;

    // 法线矩阵 = 模型矩阵左上 3x3 的逆转置（处理非统一缩放）
    mat3 normal_matrix = transpose(inverse(mat3(pc.model)));

    outWorldPos = world.xyz;
    outNormal = normalize(normal_matrix * inNormal);
    outUV = inUV;
}
