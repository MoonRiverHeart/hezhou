#version 450

layout(location = 0) in vec2 in_position;
layout(location = 1) in vec4 in_color;
layout(location = 2) in vec2 in_uv;

layout(location = 0) out vec4 frag_color;
layout(location = 1) out vec2 frag_uv;

layout(push_constant) uniform PushConstants {
    vec2 screen_size;
    vec2 offset;
} pc;

void main() {
    vec2 normalized_pos = (in_position + pc.offset) / pc.screen_size;
    vec2 clip_pos = normalized_pos * 2.0 - 1.0;
    
    gl_Position = vec4(clip_pos.x, clip_pos.y, 0.0, 1.0);
    frag_color = in_color;
    frag_uv = in_uv;
}