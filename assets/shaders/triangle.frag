#version 450

layout (binding = 0, rgba8) uniform readonly image2D img;

layout(location = 0) out vec4 out_color;
layout(location = 0) in vec2 uv;

void main() {
    ivec2 img_size = imageSize(img);
    ivec2 pixel_coords = ivec2(uv * vec2(img_size));

    vec4 pixel_color = imageLoad(img, pixel_coords);
    out_color = pixel_color;
}