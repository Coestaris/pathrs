#version 450

layout(binding = 0) uniform sampler2D tex_sampler;

layout(location = 0) out vec4 out_color;
layout(location = 0) in vec2 uv;

void main() {
    out_color = texture(tex_sampler, uv);
}