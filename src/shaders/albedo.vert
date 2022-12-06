#version 450

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;
layout(location = 2) in vec3 color;

layout(set = 0, binding = 0) uniform VP_Data {
    mat4 view;
    mat4 projection;
} vp_uniforms;

layout(set = 1, binding = 0) uniform Model_Data {
    mat4 model;
    mat4 normals;
} model_uniforms;

layout(location = 0) out vec3 out_color;
layout(location = 1) out vec3 out_normal;

void main() {
    vec4 position = vp_uniforms.projection * vp_uniforms.view * model_uniforms.model * vec4(position, 1.0);
    // position.y *= -1;
    gl_Position = position;
    out_color = color;
    out_normal = mat3(model_uniforms.normals) * normal;
}