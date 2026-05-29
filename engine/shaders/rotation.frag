#version 450

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
    float specular_strength;     // 4 bytes — 高光强度 (默认0.3)
    float ambient_strength;      // 4 bytes — 环境光强度 (默认0.15)
    float shininess;             // 4 bytes — 高光指数 (默认32.0)
    // Total: 144 bytes
} pc;

layout(location = 0) out vec4 outColor;

void main() {
    vec3 baseColor = fragColor;
    
    // 纹理采样 — 如果has_texture>0.5则使用纹理颜色替代面着色
    if (pc.has_texture > 0.5) {
        baseColor = texture(texSampler, fragUV).rgb;
    }
    
    vec3 lightDir = normalize(-light.direction);
    vec3 norm = normalize(fragNormal);
    
    float ambientStrength = pc.ambient_strength;
    vec3 ambient = baseColor * ambientStrength;
    
    float diff = max(dot(norm, lightDir), 0.0);
    vec3 diffuse = baseColor * diff * light.intensity * light.color;
    
    vec3 viewDir = normalize(-fragPosition);
    vec3 halfDir = normalize(lightDir + viewDir);
    float spec = pow(max(dot(norm, halfDir), 0.0), pc.shininess);
    vec3 specular = vec3(pc.specular_strength) * spec * light.intensity * light.color;
    
    vec3 color = ambient + diffuse + specular;
    
    if (fragOutline > 0.5) {
        color = vec3(1.0, 0.5, 0.0);
    }
    
    outColor = vec4(color, 1.0);
}