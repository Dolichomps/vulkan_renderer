#version 450

layout (location = 0) in vec3 position;
layout (location = 1) in vec3 normal;
layout (location = 2) in mat4 model_matrix;
layout (location = 6) in mat4 inverse_model_matrix;
layout (location = 10) in vec3 color;

layout (set = 0, binding = 0) uniform UniformBufferObject {
    mat4 view_matrix;
    mat4 projection_matrix;
} ubo;

layout (location = 0) out vec4 color_data_for_frag;
layout (location = 1) out vec3 out_normal;

void main() {
    gl_Position = ubo.projection_matrix * ubo.view_matrix * model_matrix * vec4(position, 1.0);
    color_data_for_frag = vec4(color, 1.0);
    out_normal = transpose(mat3(inverse_model_matrix)) * normal;
}