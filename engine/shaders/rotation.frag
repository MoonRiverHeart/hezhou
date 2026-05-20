#version 450

layout(location = 0) in vec3 fragColor;
layout(location = 1) in vec4 outlineColor;

layout(location = 0) out vec4 outColor;

void main() {
    // Use outline color if alpha > 0.01 (for transparent overlay rendering)
    if (outlineColor.a > 0.01) {
        outColor = outlineColor;
    } else {
        outColor = vec4(fragColor, 1.0);
    }
}