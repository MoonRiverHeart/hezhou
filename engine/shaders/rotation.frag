#version 450

layout(location = 0) in vec3 fragColor;
layout(location = 1) in vec3 fragNormal;
layout(location = 2) in float fragOutline;

layout(location = 0) out vec4 outColor;

void main() {
    vec3 color = fragColor;
    
    // Simple lighting
    vec3 lightDir = normalize(vec3(0.5, 1.0, 0.3));
    float diff = max(dot(normalize(fragNormal), lightDir), 0.3);
    color *= diff;
    
    if (fragOutline > 0.5) {
        // Override with outline color (orange)
        color = vec3(1.0, 0.5, 0.0);
    }
    
    outColor = vec4(color, 1.0);
}