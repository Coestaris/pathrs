#version 450

layout(location = 0) out vec4 out_color;
layout(location = 0) in vec2 uv;

void main() {
    out_color = vec4(uv, 0.0, 1.0);
}