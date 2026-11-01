#version 450

// Tonemap + Gamma + Bloom 合成（0.2）
// 输入: HDR 场景颜色（可选 Bloom 纹理），输出 LDR [0,1] 颜色。
// 渲染顺序: Game(HDR) → BloomPrefilter → 本 Pass → FXAA → UI
// 契约常量与 Rust 端 post::TonemapOperator::as_u32 一致。

#define TONEMAP_NONE 0
#define TONEMAP_REINHARD 1
#define TONEMAP_ACES 2
#define TONEMAP_FILMIC 3

layout(location = 0) in vec2 in_uv;
layout(location = 0) out vec4 out_color;

layout(set = 0, binding = 0) uniform sampler2D hdr_scene;
layout(set = 0, binding = 1) uniform sampler2D bloom_texture;

// push constant: PostTonemapPushConstant (16 bytes)
layout(push_constant) uniform PostTonemapPushConstants {
    vec4 params; // x=exposure y=gamma z=tonemap_op w=bloom_intensity
} pc;

float tonemap_curve(int op, float x) {
    x = max(x, 0.0);
    if (op == TONEMAP_REINHARD) {
        return x / (1.0 + x);
    } else if (op == TONEMAP_ACES) {
        // Narkowicz ACES 拟合
        float a = 2.51;
        float b = 0.03;
        float c = 2.43;
        float d = 0.59;
        float e = 0.14;
        return clamp((x * (a * x + b)) / (x * (c * x + d) + e), 0.0, 1.0);
    } else if (op == TONEMAP_FILMIC) {
        // Hable Uncharted2
        float a = 0.15;
        float b = 0.50;
        float c = 0.10;
        float d = 0.20;
        float e = 0.02;
        float f = 0.30;
        float curve = ((x * (a * x + c * b) + d * e) / (x * (a * x + b) + d * f)) - e / f;
        float white = 11.2;
        float w_curve = ((white * (a * white + c * b) + d * e) / (white * (a * white + b) + d * f)) - e / f;
        return clamp(curve / w_curve, 0.0, 1.0);
    }
    // TONEMAP_NONE
    return x;
}

void main() {
    vec3 hdr = texture(hdr_scene, in_uv).rgb;
    vec3 bloom = texture(bloom_texture, in_uv).rgb;
    hdr += bloom * max(pc.params.w, 0.0);

    hdr *= max(pc.params.x, 0.01);

    int op = int(pc.params.z + 0.5);
    vec3 mapped = vec3(
        tonemap_curve(op, hdr.r),
        tonemap_curve(op, hdr.g),
        tonemap_curve(op, hdr.b)
    );

    float gamma = pc.params.y;
    if (gamma > 1.0) {
        mapped = pow(mapped, vec3(1.0 / gamma));
    }

    out_color = vec4(clamp(mapped, 0.0, 1.0), 1.0);
}
