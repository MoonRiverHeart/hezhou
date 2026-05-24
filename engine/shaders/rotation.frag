#version 450

layout(location = 0) in vec3 fragColor;
layout(location = 1) in vec3 fragNormal;
layout(location = 2) in float fragOutline;

layout(set = 0, binding = 0) uniform LightData {
    vec3 direction;
    vec3 color;
    float intensity;
} light;

layout(location = 0) out vec4 outColor;

void main() {
    vec3 color = fragColor;
    
    // Lighting from UBO — std430 layout: intensity at offset 28 (NOT 32)
    vec3 lightDir = normalize(-light.direction);
    // Ambient (additive) + Diffuse (multiplicative) — gives visible lighting
    float ambient = 0.3;
    float diff = dot(normalize(fragNormal), lightDir);
    color = color * (ambient + diff * light.intensity) * light.color;
    
    if (fragOutline > 0.5) {
        color = vec3(1.0, 0.5, 0.0);
    }
    
    outColor = vec4(color, 1.0);
}