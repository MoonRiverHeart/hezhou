#version 450

// Bloom 亮度提取 + 4-tap 降采样（0.2）
// 输入 HDR 场景，输出超过阈值的亮部（供后续 mip 降采样/上采样模糊）。
// 契约参数与 Rust 端 post::BloomPrefilterPushConstant 一致。

layout(location = 0) in vec2 in_uv;
layout(location = 0) out vec4 out_color;

layout(set = 0, binding = 0) uniform sampler2D source_texture;

// push constant: BloomPrefilterPushConstant (16 bytes)
layout(push_constant) uniform BloomPrefilterPushConstants {
    vec4 params; // x=threshold y=soft_knee z=texel_x w=texel_y
} pc;

// 感知亮度（Rec.709）
float luminance(vec3 c) {
    return dot(c, vec3(0.2126, 0.7152, 0.0722));
}

// 软膝阈值: 超过 threshold 的亮度按 soft_knee 平滑过渡
vec3 prefilter(vec3 color) {
    float luma = luminance(color);
    if (luma <= 0.0) {
        return vec3(0.0);
    }
    float knee = max(pc.params.x * pc.params.y, 1e-6);
    float soft = clamp(luma - pc.params.x + knee, 0.0, 2.0 * knee);
    float contribution = max(soft * soft / (4.0 * knee + 1e-6), luma - pc.params.x);
    return color * (contribution / luma);
}

void main() {
    vec2 uv = in_uv;
    vec2 texel = pc.params.zw;
    // 4-tap 降采样（2x2 盒式），同时做亮度提取
    vec3 c = prefilter(texture(source_texture, uv + vec2(-texel.x, -texel.y)).rgb)
           + prefilter(texture(source_texture, uv + vec2(texel.x, -texel.y)).rgb)
           + prefilter(texture(source_texture, uv + vec2(-texel.x, texel.y)).rgb)
           + prefilter(texture(source_texture, uv + vec2(texel.x, texel.y)).rgb);
    out_color = vec4(c * 0.25, 1.0);
}
