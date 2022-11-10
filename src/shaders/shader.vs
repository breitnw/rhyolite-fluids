#version 450

layout(location = 0) in vec3 position;
layout(location = 1) in vec3 color;

layout(location = 0) out vec3 out_color;

layout(set = 0, binding = 0) uniform MVP_Data {
    mat4 model;
    mat4 view;
    mat4 projection;
} uniforms;

void main() {
    mat4 modelview = uniforms.view * uniforms.model;
    gl_Position = uniforms.projection * modelview * vec4(position, 1.0);
    out_color = color;
}