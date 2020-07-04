#version 450

layout (location = 0) out vec4 the_color;
layout (location = 0) in vec4 data_from_vertex_shader;
layout (location = 1) in vec3 normal;

void main() {
    vec3 direction_to_light = normalize(vec3(-1, -1, 0));
    the_color = 0.4 * (1 + max(dot(normal, direction_to_light), 0)) * data_from_vertex_shader;
}