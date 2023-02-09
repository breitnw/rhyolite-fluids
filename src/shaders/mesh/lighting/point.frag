#version 450

layout(location = 0) in vec3 cam_pos;

// Unlike binding, the value of input_attachment_index depends on the order the attachments are given in the 
// renderpass, not in the descriptor set.
layout(input_attachment_index = 0, set = 1, binding = 0) uniform subpassInput u_color;
layout(input_attachment_index = 1, set = 1, binding = 1) uniform subpassInput u_normals;
layout(input_attachment_index = 2, set = 1, binding = 2) uniform subpassInput u_frag_pos;
layout(input_attachment_index = 3, set = 1, binding = 3) uniform subpassInput u_specular;

layout(set = 1, binding = 4) uniform Point_Light_Data {
    vec4 position;
    vec3 color;
    float intensity;
} light;

layout(location = 0) out vec4 f_color;

// Phong shading
void main() {
    vec3 frag_pos = subpassLoad(u_frag_pos).xyz;

    vec3 light_dir = light.position.xyz - frag_pos;
    float dist_squared = pow(length(light_dir), 2);
    light_dir = normalize(light_dir);

    vec3 normal = normalize(subpassLoad(u_normals).xyz);

    float specular_intensity = subpassLoad(u_specular).x;
    float specular_shininess = subpassLoad(u_specular).y;

    float lambertian = max(dot(normal, light_dir), 0.0);
    float specular = 0.0;

    if (lambertian > 0.0) {
        vec3 view_dir = normalize(cam_pos - frag_pos);
        vec3 reflect_dir = reflect(-light_dir, normal);
        float specAngle = max(dot(reflect_dir, view_dir), 0.0);
        specular = pow(specAngle, specular_shininess);
    }

    vec3 light_color = (lambertian * light.color + specular * light.color) * light.intensity / dist_squared;

    f_color = vec4(subpassLoad(u_color).rgb * light_color, 1.0);
}
