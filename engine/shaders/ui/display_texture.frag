#version 450

layout(location = 0) in vec4 frag_color;
layout(location = 1) in vec2 frag_uv;

layout(location = 0) out vec4 out_color;

layout(set = 0, binding = 0) uniform sampler2D input_texture;

void main() {
    vec4 tex_color = texture(input_texture, frag_uv);
    out_color = vec4(tex_color.rgb * frag_color.rgb, tex_color.a * frag_color.a);
}