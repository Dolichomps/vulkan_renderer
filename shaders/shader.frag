#version 450

layout (location = 0) out vec4 theColor;
layout (location = 0) in vec4 data_from_vertex_shader;

void main() {
    theColor = data_from_vertex_shader;
}