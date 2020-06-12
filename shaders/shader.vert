#version 450

layout (location = 0) in vec3 position;
layout (location = 1) in vec3 position_offset;
layout (location = 2) in vec3 color;

layout (location = 0) out vec4 color_data_for_frag;

void main() {
    gl_Position = vec4(position + position_offset, 1.0);
    color_data_for_frag = vec4(color, 1.0);
}