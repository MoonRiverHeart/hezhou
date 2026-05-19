#version 450

layout(push_constant) uniform PushConstants {
    float rotation;
} pc;

vec2 positions[3] = vec2[](
    vec2(0.0, -0.667),   // 顶部 (向下偏移使重心在origin)
    vec2(-0.5, 0.333),   // 左下
    vec2(0.5, 0.333)     // 右下
);

vec3 colors[3] = vec3[](
    vec3(1.0, 0.0, 0.0),
    vec3(0.0, 1.0, 0.0),
    vec3(0.0, 0.0, 1.0)
);

layout(location = 0) out vec3 fragColor;

void main() {
    float angle = pc.rotation;
    float cosA = cos(angle);
    float sinA = sin(angle);
    
    vec2 rotated = vec2(
        positions[gl_VertexIndex].x * cosA - positions[gl_VertexIndex].y * sinA,
        positions[gl_VertexIndex].x * sinA + positions[gl_VertexIndex].y * cosA
    );
    
    gl_Position = vec4(rotated, 0.0, 1.0);
    fragColor = colors[gl_VertexIndex];
}