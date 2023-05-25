#version 450

layout(set = 0, binding = 0) uniform UCamData {
    mat4 view;
    mat4 projection;
} vp_uniforms;

layout(set = 1, binding = 0) uniform UModelData {
    mat4 model;
    mat4 normals;
} model_uniforms;

layout(set = 2, binding = 0) readonly buffer UVertices {
    vec4 data[]; // Alternates between position and normal
} vertices;

layout(location = 0) out vec3 out_color;
layout(location = 1) out vec3 out_normal;
layout(location = 2) out vec4 out_pos;

void main() {
    int i = gl_VertexIndex;
    vec4 position = vertices.data[i * 3];
    vec4 normal = vertices.data[i * 3 + 1];
    vec4 color = vertices.data[i * 3 + 2];

    vec4 frag_pos = vp_uniforms.projection * vp_uniforms.view * model_uniforms.model * position;
    gl_Position = frag_pos;
    out_color = vec3(color);
    out_normal = mat3(model_uniforms.normals) * vec3(normal);
    out_pos = model_uniforms.model * position;
}