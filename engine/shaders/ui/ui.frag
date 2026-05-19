#version 450

layout(location = 0) in vec4 frag_color;
layout(location = 1) in vec2 frag_uv;

layout(location = 0) out vec4 out_color;

layout(set = 0, binding = 0) uniform sampler2D font_texture;

layout(push_constant) uniform PushConstants {
    vec2 screen_size;
    vec2 offset;
    float px_range;
    bool enable_msdf;
} pc;

void main() {
    if (frag_uv.x == 0.0 && frag_uv.y == 0.0) {
        out_color = frag_color;
    } else {
        vec4 texture_color = texture(font_texture, frag_uv);
        
        if (pc.enable_msdf) {
            // MSDF text rendering (alpha only from red channel)
            float dist = texture_color.r;
            float px_range = pc.px_range;
            if (px_range <= 0.0) {
                px_range = 4.0;
            }
            float d = (dist - 0.5) * px_range;
            float alpha = smoothstep(-1.0, 1.0, d);
            out_color = vec4(frag_color.rgb, frag_color.a * alpha);
        } else {
            // RGB texture display (use full RGBA from texture)
            out_color = vec4(texture_color.rgb * frag_color.rgb, texture_color.a * frag_color.a);
        }
    }
}