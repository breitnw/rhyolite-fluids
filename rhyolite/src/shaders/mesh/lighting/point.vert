#version 450

layout(location = 0) in vec2 position;

layout(set = 0, binding = 0) uniform UCamData {
    mat4 view;
    mat4 projection;
} vp_uniforms;

layout(location = 0) out vec3 cam_pos;

void main() {
    mat4 view_i = inverse(vp_uniforms.view);
    cam_pos = vec3(view_i[3][0], view_i[3][1], view_i[3][2]);
    
    gl_Position = vec4(position, 0.0, 1.0);
}
