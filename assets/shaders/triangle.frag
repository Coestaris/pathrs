#version 450

layout (set=0, binding = 0, rgba32f) uniform readonly image2D img;

layout(location = 0) out vec4 out_color;
layout(location = 0) in vec2 uv;

void main() {
    ivec2 img_size = imageSize(img);
    ivec2 pixel_coords = ivec2(uv * vec2(img_size));

    vec4 pixel_color = imageLoad(img, pixel_coords);

    // Simple gamma correction
    pixel_color.rgb = pow(pixel_color.rgb, vec3(1.0 / 2.2));

    out_color = pixel_color;
}