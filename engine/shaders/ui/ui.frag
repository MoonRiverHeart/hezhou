#version 450

layout(location = 0) in vec4 frag_color;
layout(location = 1) in vec2 frag_uv;

layout(location = 0) out vec4 out_color;

layout(binding = 0) uniform sampler2D ui_texture;

void main() {
    vec4 tex_color = texture(ui_texture, frag_uv);
    vec4 final_color = frag_color * tex_color;
    
    out_color = final_color;
}