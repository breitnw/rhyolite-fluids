#version 450

layout(location = 0) in vec3 in_color;
layout(location = 1) in vec3 in_normal;
layout(location = 2) in vec4 in_pos;

layout(set = 1, binding = 1) uniform Specular_Data {
    float intensity;
    float shininess;
} specular;

layout(location = 0) out vec4 out_color;
layout(location = 1) out vec3 out_normal;
layout(location = 2) out vec4 out_frag_pos;
layout(location = 3) out vec2 out_specular;

void main() {
    out_color = vec4(in_color, 1.0);
    out_normal = in_normal;
    out_frag_pos = in_pos;
    out_specular = vec2(specular.intensity, specular.shininess);
}