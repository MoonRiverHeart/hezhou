#version 450
// Hezhou 0.2 骨骼蒙皮片元着色器（参考实现）
//
// 说明：skinning.vert 的 varying 输出与 rotation.vert 完全一致（location 0..4），
// 因此渲染侧**优先直接复用 rotation.frag.spv**；本文件只在需要独立蒙皮材质时使用，
// 内容与 rotation.frag 的 Blinn-Phong 回退路径保持一致，避免破坏现有 raster 行为。
//
// 编译：
//   glslc engine/shaders/skinning.frag -o engine/shaders/skinning.frag.spv

layout(location = 0) in vec3 fragColor;
layout(location = 1) in vec3 fragNormal;
layout(location = 2) in float fragOutline;
layout(location = 3) in vec3 fragPosition;
layout(location = 4) in vec2 fragUV;

layout(set = 0, binding = 0) uniform LightData {
    vec3 direction;
    vec3 color;
    float intensity;
} light;

layout(set = 1, binding = 0) uniform sampler2D texSampler;

layout(push_constant) uniform PushConstants {
    mat4 model;           // 64 bytes
    vec3 outline_color;   // 12 bytes
    float is_selected;    // 4 bytes
    vec2 viewport_size;   // 8 bytes
    float _pad0;          // 4 bytes
    float _pad1;          // 4 bytes
    vec3 camera_pos;      // 12 bytes
    float camera_yaw;     // 4 bytes
    float camera_pitch;   // 4 bytes
    float _pad2;          // 4 bytes
    float has_texture;           // 4 bytes — 1.0 if entity has texture
    float specular_strength;     // 4 bytes
    float ambient_strength;      // 4 bytes
    float shininess;             // 4 bytes
    // Total: 144 bytes
} pc;

layout(location = 0) out vec4 outColor;

void main() {
    vec3 baseColor = fragColor;

    if (pc.has_texture > 0.5) {
        baseColor = texture(texSampler, fragUV).rgb;
    }

    vec3 lightDir = normalize(-light.direction);
    vec3 norm = normalize(fragNormal);

    vec3 ambient = baseColor * pc.ambient_strength;
    float diff = max(dot(norm, lightDir), 0.0);
    vec3 diffuse = baseColor * diff * light.intensity * light.color;

    vec3 viewDir = normalize(-fragPosition);
    vec3 halfDir = normalize(lightDir + viewDir);
    float spec = pow(max(dot(norm, halfDir), 0.0), pc.shininess);
    vec3 specular = vec3(pc.specular_strength) * spec * light.intensity * light.color;

    vec3 color = ambient + diffuse + specular;

    if (fragOutline > 0.5) {
        color = pc.outline_color;
    }

    outColor = vec4(color, 1.0);
}
