#version 450

layout(location = 0) in vec2 position;

layout(set = 0, binding = 0) uniform VP_Data {
    mat4 view;
    mat4 projection;
} vp_uniforms;

layout(location = 0) out vec2 uv;

void main() {
    float f = vp_uniforms.projection[1][1];
    uv = vec2(position.x * f / vp_uniforms.projection[0][0], position.y);
    gl_Position = vec4(position, 0.0, 1.0);
}
