#version 450

layout (location = 0) out vec4 color_data_for_frag;

void main() {
    gl_PointSize = 100.0;
    gl_Position = vec4(0.0, 0.0, 0.0, 1.0);
    color_data_for_frag = vec4(1.0, 0.6, 1.0, 1.0);
}