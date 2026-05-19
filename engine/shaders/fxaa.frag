#version 450

layout(location = 0) in vec2 in_uv;
layout(location = 0) out vec4 out_color;

layout(set = 0, binding = 0) uniform sampler2D input_texture;

layout(push_constant) uniform PushConstants {
    vec2 resolution;
} pc;

void main() {
    vec2 texel_size = 1.0 / pc.resolution;
    
    vec3 rgb_center = texture(input_texture, in_uv).rgb;
    vec3 rgb_n = texture(input_texture, in_uv + vec2(0.0, -texel_size.y)).rgb;
    vec3 rgb_s = texture(input_texture, in_uv + vec2(0.0, texel_size.y)).rgb;
    vec3 rgb_e = texture(input_texture, in_uv + vec2(texel_size.x, 0.0)).rgb;
    vec3 rgb_w = texture(input_texture, in_uv + vec2(-texel_size.x, 0.0)).rgb;
    
    vec3 luma_weights = vec3(0.299, 0.587, 0.114);
    float luma_center = dot(luma_weights, rgb_center);
    float luma_n = dot(luma_weights, rgb_n);
    float luma_s = dot(luma_weights, rgb_s);
    float luma_e = dot(luma_weights, rgb_e);
    float luma_w = dot(luma_weights, rgb_w);
    
    float luma_min = min(luma_center, min(min(luma_n, luma_w), min(luma_s, luma_e)));
    float luma_max = max(luma_center, max(max(luma_n, luma_w), max(luma_s, luma_e)));
    float luma_range = luma_max - luma_min;
    
    float edge_threshold = 0.0312;
    float edge_threshold_min = 0.0625;
    
    if (luma_range < edge_threshold_min) {
        out_color = vec4(rgb_center, 1.0);
        return;
    }
    
    float blend_factor = max(0.0, (luma_range - edge_threshold_min) / edge_threshold);
    blend_factor = min(blend_factor, 1.0);
    
    vec3 rgb_avg = (rgb_n + rgb_s + rgb_e + rgb_w + rgb_center) / 5.0;
    vec3 rgb_blended = mix(rgb_center, rgb_avg, blend_factor);
    
    out_color = vec4(rgb_blended, 1.0);
}