#version 450
layout(location = 0) in vec2 fragUV;
layout(location = 0) out vec4 outColor;
void main() {
    // 预览目标为 BGRA：按 BGR 写出，uv.y 在 Vulkan 视口下顶部为 1
    vec3 horizon = vec3(0.88, 0.74, 0.62);
    vec3 zenith = vec3(0.58, 0.28, 0.10);
    vec3 c = mix(horizon, zenith, 1.0 - clamp(fragUV.y, 0.0, 1.0));
    outColor = vec4(c, 1.0);
}
