#version 450

layout(location = 0) in vec4 fragColor;
layout(location = 1) in vec2 fragUV;
layout(location = 2) in vec4 fragBorderRadius;
layout(location = 3) in vec2 fragPos;
layout(location = 4) in vec4 fragRect;
layout(location = 5) in float fragTextureId;

layout(binding = 1) uniform sampler2D texSampler;

layout(location = 0) out vec4 outColor;

float roundedRectSDF(vec2 p, vec2 center, vec2 halfSize, float r) {
    vec2 q = abs(p - center) - halfSize + r;
    return min(max(q.x, q.y), 0.0) + length(max(q, 0.0)) - r;
}

void main() {
    vec2 rectPos = fragRect.xy;
    vec2 rectSize = fragRect.zw;
    vec2 center = rectPos + rectSize * 0.5;
    vec2 halfSize = rectSize * 0.5;
    
    float maxRadius = max(max(fragBorderRadius.x, fragBorderRadius.y),
                          max(fragBorderRadius.z, fragBorderRadius.w));
    
    if (maxRadius > 0.0) {
        float d = roundedRectSDF(fragPos, center, halfSize, maxRadius);
        if (d > 0.0) discard;
        float alpha = 1.0 - smoothstep(-1.0, 1.0, d);
        outColor = vec4(fragColor.rgb, fragColor.a * alpha);
    } else if (fragTextureId > 0.5) {
        vec4 texColor = texture(texSampler, fragUV);
        float dist = texColor.a;
        float alpha = 1.0 - smoothstep(0.48, 0.52, dist);
        if (alpha < 0.1) discard;
        outColor = vec4(fragColor.rgb, alpha);
    } else {
        outColor = fragColor;
    }
}